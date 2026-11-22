//! Meta-deck cards, batch A1, based on the 59.15.1 client.
use super::CardDef;
use crate::game::{EffectLifetime, Game, PendingEffect, PlayerTrigger};

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        reveal: Some(zabu),
        ..CardDef::vanilla("Zabu")
    },
    CardDef {
        reveal: Some(surge),
        on_play: Some(surge_played),
        initial_state: Some(surge_state),
        ..CardDef::vanilla("Surge")
    },
    CardDef {
        on_play: Some(captain_britain),
        ..CardDef::vanilla("CaptainBritain")
    },
    CardDef {
        end_turn: Some(fantastic),
        end_turn_trigger: true,
        ..CardDef::vanilla("MrFantasticFirstSteps")
    },
    CardDef {
        reveal: Some(arthur),
        ..CardDef::vanilla("ArthurPendragon")
    },
    CardDef {
        ongoing: Some(excalibur),
        ..CardDef::vanilla("Excalibur")
    },
    CardDef {
        on_play: Some(werewolf),
        ..CardDef::vanilla("WerewolfByNight")
    },
    CardDef {
        end_turn: Some(adam_warlock),
        end_turn_trigger: true,
        ..CardDef::vanilla("AdamWarlock")
    },
    CardDef {
        on_play: Some(echo),
        ..CardDef::vanilla("Echo")
    },
    CardDef {
        ongoing_ability: true,
        reveal_repetitions: Some(wong),
        ..CardDef::vanilla("Wong")
    },
    CardDef {
        reveal: Some(shuri),
        ..CardDef::vanilla("Shuri")
    },
    CardDef {
        reveal: Some(jubilee),
        ..CardDef::vanilla("Jubilee")
    },
];

pub(crate) static TOKENS: &[&str] = &["Excalibur"];

fn gain_power(game: &mut Game<'_>, card: usize, amount: i32, source: usize) {
    game.add_power(card, amount, source);
}

fn multiply_power(game: &mut Game<'_>, card: usize, factor: i32, source: usize) {
    game.multiply_power(card, factor, source);
}

fn change_cost(game: &mut Game<'_>, card: usize, amount: i32, source: usize) {
    game.add_cost(card, amount, source);
}

fn ongoing_power(game: &mut Game<'_>, card: usize, amount: i32, source: usize) {
    game.add_ongoing_power(card, amount, source);
}

fn zabu(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    // ZabuBehavior::ReduceCostInDeck (0x7cf8ac8) compares the card's base cost.
    for index in 0..game.players[owner].deck.len() {
        let target = game.players[owner].deck[index];
        if game.printed_cost(target) == 4 {
            change_cost(game, target, -1, owner);
        }
    }
}

fn adam_warlock(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    if game.cards[card].abilities_removed {
        return;
    }
    let owner = game.cards[card].owner;
    // AdamWarlockBehavior::DrawIfWinning (0x7e5f75c): a tie is not a win.
    if game.lane_winner(lane) == Some(owner) {
        game.draw(owner);
    } else {
        gain_power(game, card, 1, owner);
    }
}

fn echo(game: &mut Game<'_>, card: usize, played: usize) {
    let owner = game.cards[played].owner;
    let played_lane = game.played[owner]
        .iter()
        .find_map(|&(id, lane)| (id == played).then_some(lane))
        .or(game.cards[played].lane);
    // RemoveAbilitiesFromOngoingCard (0x7e96694) checks the staged location.
    if !game.cards[card].abilities_removed
        && game.cards[card].owner != owner
        && game.cards[card].lane.is_some()
        && game.cards[card].lane == played_lane
        && game.cards[played].has_ongoing()
    {
        game.cards[played].abilities_removed = true;
    }
}

fn wong(game: &Game<'_>, card: usize, target: usize) -> u8 {
    u8::from(game.cards[card].owner == game.cards[target].owner)
}

fn jubilee(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    // JubileeBehavior::PlayCard (0x7ed6114) selects Deck::GetTopCard, not a draw.
    let Some(&target) = game.players[owner].deck.first() else {
        return;
    };
    if game.can_add(target, lane, owner) {
        game.players[owner].deck.remove(0);
        game.add_card(target, lane, owner, true);
    }
}

fn captain_britain(game: &mut Game<'_>, card: usize, played: usize) {
    let owner = game.cards[card].owner;
    if game.cards[card].abilities_removed
        || game.cards[played].owner != owner
        || game.cards[played].turn_played != game.turn
        || game.printed_cost(played) < 4
    {
        return;
    }
    gain_power(game, played, 1, owner);
    // AddPowerAndReduceHandCost (0x7e7c9a0) prefers printed-cost-positive
    // cards; printed-zero cards are eligible only when none are positive.
    let hand = &game.players[owner].hand;
    let positive = hand.iter().filter(|&&id| game.printed_cost(id) > 0).count();
    let count = if positive > 0 {
        positive
    } else {
        hand.iter()
            .filter(|&&id| game.printed_cost(id) == 0)
            .count()
    };
    if count == 0 {
        return;
    }
    // Client Game::Roll (0x7dab62c): one uniform selection from the chosen list.
    let choice = game.rng.borrow_mut().randbelow(count);
    let target = hand
        .iter()
        .copied()
        .filter(|&id| {
            let cost = game.printed_cost(id);
            if positive > 0 { cost > 0 } else { cost == 0 }
        })
        .nth(choice)
        .expect("selected a counted hand card");
    change_cost(game, target, -1, owner);
}

fn fantastic(game: &mut Game<'_>, card: usize) {
    if game.cards[card].abilities_removed || game.cards[card].lane.is_none() {
        return;
    }
    let owner = game.cards[card].owner;
    let len = game.players[owner].hand.len();
    if len == 0 {
        return;
    }
    // BuffHand (0x7f04808) rolls the target first; ApplyBuff (0x7f0495c)
    // only rolls the buff type when the target has power and positive cost.
    // Client Game::Roll (0x7dab62c) is called for each of these selections.
    let target = game.players[owner].hand[game.rng.borrow_mut().randbelow(len)];
    let reduce = !game.is_character(target)
        || (game.cards[target].cost > 0 && game.rng.borrow_mut().randbelow(2) == 0);
    if reduce {
        change_cost(game, target, -1, owner);
    } else {
        gain_power(game, target, 3, owner);
    }
}

fn surge_state() -> Box<dyn crate::game::CardState> {
    Box::new(None::<usize>)
}

fn surge(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let Some(&target) = game.players[owner].deck.first() else {
        return;
    };
    if let Some(state) = game.cards[card]
        .state
        .as_deref_mut()
        .and_then(|state| state.as_any_mut().downcast_mut::<Option<usize>>())
    {
        *state = Some(target);
    }
    change_cost(game, target, -1, owner);
    gain_power(game, target, 1, owner);
}

fn surge_played(game: &mut Game<'_>, card: usize, played: usize) {
    if game.cards[card].abilities_removed || game.cards[card].owner != game.cards[played].owner {
        return;
    }
    let target = game.cards[card]
        .state
        .as_deref()
        .and_then(|state| state.as_any().downcast_ref::<Option<usize>>())
        .copied()
        .flatten();
    if target == Some(played) {
        game.trigger_on_reveal(card);
    }
}

fn shuri(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    game.add_player_effect(
        owner,
        PendingEffect {
            source: card,
            lane: Some(lane),
            created_turn: game.turn,
            trigger: PlayerTrigger::BeforeCardRevealed,
            lifetime: EffectLifetime::NextCard,
            state: Box::new(()),
            apply: shuri_apply,
        },
    );
}

fn shuri_apply(game: &mut Game<'_>, effect: &mut PendingEffect, card: Option<usize>) -> bool {
    if let Some(card) = card {
        let owner = game.cards[card].owner;
        let lane = game.played[owner]
            .iter()
            .find_map(|&(id, lane)| (id == card).then_some(lane))
            .or(game.cards[card].lane);
        // BuffCard (0x7f3b600) compares the staged slot and skips cards without power.
        if lane == effect.lane && game.is_character(card) {
            multiply_power(game, card, 2, owner);
        }
    }
    true
}

fn arthur(game: &mut Game<'_>, card: usize) {
    if let Some(lane) = game.cards[card].lane {
        game.add_created("Excalibur", lane, game.cards[card].owner);
    }
}

fn excalibur(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    for lane in 0..3 {
        for index in 0..game.lanes[lane].cards.len() {
            let target = game.lanes[lane].cards[index];
            if game.cards[target].revealed
                && game.cards[target].owner == owner
                && game.printed_cost(target) >= 4
            {
                ongoing_power(game, target, 2, owner);
            }
        }
    }
}

fn werewolf(game: &mut Game<'_>, card: usize, played: usize) {
    let owner = game.cards[card].owner;
    if game.cards[card].abilities_removed
        || game.cards[played].owner != owner
        || game.cards[played].abilities_removed
        || game.cards[played].ability_def().reveal.is_none()
    {
        return;
    }
    // MoveAndBuff (0x7cf08f4) uses the staged slot, even if the card moved on reveal.
    let destination = game.played[owner]
        .iter()
        .find_map(|&(id, lane)| (id == played).then_some(lane))
        .or(game.cards[played].lane);
    if let Some(lane) = destination
        && game.cards[card].lane != Some(lane)
        && game.move_card(card, lane)
    {
        gain_power(game, card, 2, owner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::{Position, build_game};
    use crate::random::PythonRandom;
    use serde_json::json;

    fn position(mine: serde_json::Value, theirs: serde_json::Value) -> Position {
        serde_json::from_value(json!({
            "turn": 3, "energy": 3, "hand": [], "deck": ["Hulk"],
            "locations": [
                {"id": null, "mine": mine, "theirs": theirs},
                {"id": null, "mine": [], "theirs": []},
                {"id": null, "mine": [], "theirs": []}
            ],
            "opponent": {"hand": 0, "deck": 0}
        }))
        .unwrap()
    }

    #[test]
    fn adam_wins_draws_and_ties_gain_power() {
        let p = position(json!([{"id":"AdamWarlock"}]), json!([]));
        let mut rng = PythonRandom::seed(1);
        let mut game = build_game(&p, &mut rng).unwrap();
        let adam = game.lanes[0].cards[0];
        adam_warlock(&mut game, adam);
        assert_eq!(game.cards[adam].power(), 1);
        assert_eq!(game.players[0].hand.len(), 0);
        adam_warlock(&mut game, adam);
        assert_eq!(game.players[0].hand.len(), 1);
        game.cards[adam].abilities_removed = true;
        adam_warlock(&mut game, adam);
        assert_eq!(game.cards[adam].power(), 1);
    }

    #[test]
    fn zabu_only_reduces_printed_four_cost_deck_cards() {
        let mut p = position(json!([{"id":"Zabu"}]), json!([]));
        p.deck = vec!["JessicaJones".into(), "Hulk".into()];
        let mut rng = PythonRandom::seed(2);
        let mut game = build_game(&p, &mut rng).unwrap();
        let zabu_id = game.lanes[0].cards[0];
        let target = *game.players[0]
            .deck
            .iter()
            .find(|&&id| game.cards[id].def.id == "JessicaJones")
            .unwrap();
        zabu(&mut game, zabu_id);
        zabu(&mut game, zabu_id);
        assert_eq!(game.cards[target].cost, 2);
        let hulk = *game.players[0]
            .deck
            .iter()
            .find(|&&id| game.cards[id].def.id == "Hulk")
            .unwrap();
        assert_eq!(game.cards[hulk].cost, 6);
    }

    #[test]
    fn echo_only_removes_enemy_ongoing_here() {
        let p = position(
            json!([{"id":"Echo"}, {"id":"IronMan"}]),
            json!([{"id":"IronMan"}]),
        );
        let mut rng = PythonRandom::seed(3);
        let mut game = build_game(&p, &mut rng).unwrap();
        let echo_id = game.lanes[0].cards[0];
        let ally = game.lanes[0].cards[1];
        let enemy = game.lanes[0].cards[2];
        echo(&mut game, echo_id, ally);
        assert!(!game.cards[ally].abilities_removed);
        echo(&mut game, echo_id, enemy);
        assert!(game.cards[enemy].abilities_removed);
    }

    #[test]
    fn wong_repeats_only_allied_reveals() {
        let p = position(
            json!([{"id":"Wong"}, {"id":"Hawkeye"}]),
            json!([{"id":"Hawkeye"}]),
        );
        let mut rng = PythonRandom::seed(4);
        let game = build_game(&p, &mut rng).unwrap();
        assert_eq!(
            wong(&game, game.lanes[0].cards[0], game.lanes[0].cards[1]),
            1
        );
        assert_eq!(
            wong(&game, game.lanes[0].cards[0], game.lanes[0].cards[2]),
            0
        );
    }

    #[test]
    fn jubilee_adds_top_card_but_not_to_full_lane() {
        let p = position(json!([{"id":"Jubilee"}]), json!([]));
        let mut rng = PythonRandom::seed(5);
        let mut game = build_game(&p, &mut rng).unwrap();
        let jubilee_id = game.lanes[0].cards[0];
        let target = game.players[0].deck[0];
        jubilee(&mut game, jubilee_id);
        assert_eq!(game.cards[target].lane, Some(0));
        assert!(game.players[0].deck.is_empty());

        let p = position(
            json!([{"id":"Jubilee"}, {"id":"Hawkeye"}, {"id":"Cyclops"}, {"id":"Shocker"}]),
            json!([]),
        );
        let mut rng = PythonRandom::seed(5);
        let mut game = build_game(&p, &mut rng).unwrap();
        let jubilee_id = game.lanes[0].cards[0];
        let target = game.players[0].deck[0];
        jubilee(&mut game, jubilee_id);
        assert_eq!(game.players[0].deck, vec![target]);
        assert_eq!(game.cards[target].lane, None);
    }
    #[test]
    fn surge_tracks_the_buffed_card() {
        let mut p = position(json!([{"id":"Surge"}]), json!([]));
        p.deck = vec!["Hulk".into(), "Cyclops".into()];
        let mut rng = PythonRandom::seed(6);
        let mut game = build_game(&p, &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        let target = game.players[0].deck[0];
        let other = game.players[0].deck[1];
        let power = game.cards[target].power();
        let cost = game.cards[target].cost;
        surge(&mut game, source);
        assert_eq!(game.cards[target].power(), power + 1);
        assert_eq!(game.cards[target].cost, cost - 1);
        surge_played(&mut game, source, other);
        assert_eq!(game.cards[target].power(), power + 1);
        game.players[0].deck.remove(0);
        let next_power = game.cards[other].power();
        surge_played(&mut game, source, target);
        assert_eq!(game.cards[other].power(), next_power + 1);
    }

    #[test]
    fn captain_britain_uses_printed_cost_and_prefers_positive_cost_cards() {
        let mut p = position(
            json!([{"id":"CaptainBritain"}, {"id":"JessicaJones"}]),
            json!([]),
        );
        p.hand = serde_json::from_value(json!([{"id":"Hulk"}, {"id":"Wasp"}])).unwrap();
        let mut rng = PythonRandom::seed(7);
        let mut game = build_game(&p, &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        let played = game.lanes[0].cards[1];
        game.cards[played].turn_played = game.turn;
        let target = game.players[0].hand[0];
        let old = game.cards[played].power();
        captain_britain(&mut game, source, played);
        assert_eq!(game.cards[played].power(), old + 1);
        assert_eq!(game.cards[target].cost, 5);
    }

    #[test]
    fn fantastic_zero_cost_character_gets_power() {
        let mut p = position(json!([{"id":"MrFantasticFirstSteps"}]), json!([]));
        p.hand = serde_json::from_value(json!([{"id":"Wasp"}])).unwrap();
        let mut rng = PythonRandom::seed(8);
        let mut game = build_game(&p, &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        let target = game.players[0].hand[0];
        let old = game.cards[target].power();
        fantastic(&mut game, source);
        assert_eq!(game.cards[target].power(), old + 3);
    }

    #[test]
    fn random_hand_targets_reach_every_eligible_candidate() {
        let mut captain = position(
            json!([{"id":"CaptainBritain"}, {"id":"JessicaJones"}]),
            json!([]),
        );
        captain.deck.clear();
        captain.hand =
            serde_json::from_value(json!([{"id":"Hulk"}, {"id":"Cyclops"}, {"id":"Wasp"}]))
                .unwrap();
        let mut fantastic_position = position(json!([{"id":"MrFantasticFirstSteps"}]), json!([]));
        fantastic_position.deck.clear();
        fantastic_position.hand =
            serde_json::from_value(json!([{"id":"Hulk"}, {"id":"Cyclops"}])).unwrap();
        let mut captain_seen = [false; 2];
        let mut fantastic_seen = [false; 2];
        let mut fantastic_modes = [false; 2];
        for seed in 0..64 {
            let mut rng = PythonRandom::seed(seed);
            let mut game = build_game(&captain, &mut rng).unwrap();
            let source = game.lanes[0].cards[0];
            let played = game.lanes[0].cards[1];
            game.cards[played].turn_played = game.turn;
            let hand = game.players[0].hand.clone();
            let costs: Vec<_> = hand.iter().map(|&id| game.cards[id].cost).collect();
            captain_britain(&mut game, source, played);
            for i in 0..2 {
                captain_seen[i] |= game.cards[hand[i]].cost == costs[i] - 1;
            }
            assert_eq!(game.cards[hand[2]].cost, costs[2]);

            let mut rng = PythonRandom::seed(seed);
            let mut game = build_game(&fantastic_position, &mut rng).unwrap();
            let source = game.lanes[0].cards[0];
            let hand = game.players[0].hand.clone();
            let before: Vec<_> = hand
                .iter()
                .map(|&id| (game.cards[id].cost, game.cards[id].power()))
                .collect();
            fantastic(&mut game, source);
            for i in 0..2 {
                let reduced = game.cards[hand[i]].cost < before[i].0;
                let boosted = game.cards[hand[i]].power() > before[i].1;
                fantastic_seen[i] |= reduced || boosted;
                fantastic_modes[0] |= reduced;
                fantastic_modes[1] |= boosted;
            }
        }
        assert_eq!(captain_seen, [true; 2]);
        assert_eq!(fantastic_seen, [true; 2]);
        assert_eq!(fantastic_modes, [true; 2]);
    }

    #[test]
    fn arthur_creates_excalibur_and_it_buffs_only_large_allies() {
        let p = position(
            json!([{"id":"ArthurPendragon"}, {"id":"Hawkeye"}]),
            json!([{"id":"Hulk"}]),
        );
        let mut rng = PythonRandom::seed(9);
        let mut game = build_game(&p, &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        let small = game.lanes[0].cards[1];
        let enemy = game.lanes[0].cards[2];
        arthur(&mut game, source);
        assert!(
            game.lanes[0]
                .cards
                .iter()
                .any(|&id| game.cards[id].def.id == "Excalibur")
        );
        assert_eq!(game.cards[source].ongoing, 2);
        assert_eq!(game.cards[small].ongoing, 0);
        assert_eq!(game.cards[enemy].ongoing, 0);
    }

    #[test]
    fn werewolf_moves_only_for_another_lane_and_reveal_card() {
        let p = position(
            json!([{"id":"WerewolfByNight"}, {"id":"Hawkeye"}]),
            json!([]),
        );
        let mut rng = PythonRandom::seed(10);
        let mut game = build_game(&p, &mut rng).unwrap();
        let wolf = game.lanes[0].cards[0];
        let played = game.lanes[0].cards[1];
        let power = game.cards[wolf].power();
        werewolf(&mut game, wolf, played);
        assert_eq!(game.cards[wolf].power(), power);
        game.played[0].push((played, 1));
        werewolf(&mut game, wolf, played);
        assert_eq!(game.cards[wolf].lane, Some(1));
        assert_eq!(game.cards[wolf].power(), power + 2);
    }
    #[test]
    fn shuri_effect_survives_source_destruction_and_is_consumed() {
        let p = position(
            json!([{"id":"Shuri"}, {"id":"Hawkeye", "revealed":false, "played_turn":3}]),
            json!([]),
        );
        let mut rng = PythonRandom::seed(11);
        let mut game = build_game(&p, &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        let target = game.lanes[0].cards[1];
        game.played[0].push((target, 0));
        let power = game.cards[target].permanent;
        shuri(&mut game, source);
        assert!(game.destroy_card(source));
        game.reveal(target);
        assert_eq!(game.cards[target].permanent, power * 2);
        assert!(game.players[0].pending_effects.is_empty());

        // The next card played at another location is not doubled, and still uses up the effect.
        let p: Position = serde_json::from_value(json!({
            "turn": 3, "energy": 3, "hand": [], "deck": ["Hulk"],
            "locations": [
                {"id": null, "mine": [{"id":"Shuri"}], "theirs": []},
                {"id": null, "mine": [{"id":"Hawkeye", "revealed":false, "played_turn":3}], "theirs": []},
                {"id": null, "mine": [], "theirs": []}
            ],
            "opponent": {"hand": 0, "deck": 0}
        }))
        .unwrap();
        let mut rng = PythonRandom::seed(12);
        let mut game = build_game(&p, &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        let target = game.lanes[1].cards[0];
        game.played[0].push((target, 1));
        let power = game.cards[target].permanent;
        shuri(&mut game, source);
        game.reveal(target);
        assert_eq!(game.cards[target].permanent, power);
        assert!(game.players[0].pending_effects.is_empty());
    }
}
