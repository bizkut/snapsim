//! Client 59.15.1 card rules, batch I1.
use super::CardDef;
use crate::game::{EffectLifetime, Event, Game, PendingEffect, PlayerTrigger};

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        reveal: Some(nakia),
        ..CardDef::vanilla("Nakia")
    },
    CardDef {
        reveal: Some(hobgoblin),
        ..CardDef::vanilla("Hobgoblin")
    },
    CardDef {
        end_turn: Some(attuma),
        ..CardDef::vanilla("Attuma")
    },
    CardDef {
        on_event: Some(namorita),
        ..CardDef::vanilla("Namorita")
    },
    CardDef {
        on_event: Some(topaz),
        ..CardDef::vanilla("Topaz")
    },
    CardDef {
        reveal: Some(brood),
        ..CardDef::vanilla("Brood")
    },
    CardDef::vanilla("Broodling"),
    CardDef {
        reveal: Some(stick),
        ..CardDef::vanilla("Stick")
    },
    CardDef {
        reveal: Some(wave),
        ..CardDef::vanilla("Wave")
    },
];

pub(crate) static TOKENS: &[&str] = &["Broodling"];

// Nakia's TriggerAbilityDef::ExecuteTriggerAction (0x7e0f6e4) uses
// AddPowerTriggerAction(+1) and CardsInHandSelector.
fn nakia(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let targets = game.players[owner].hand.clone();
    for target in targets {
        game.add_power(target, 1, owner);
    }
}

// HobgoblinBehavior::SwitchSide (0x7ec5584) changes ownership, not location.
fn hobgoblin(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let other = 1 - game.cards[card].owner;
    if game.has_space(lane, other, 0) {
        game.cards[card].owner = other;
    }
}

// AttumaBehavior::DestroyWeakerCard (0x7e6c730) chooses one random
// revealed ally of lower power, not necessarily the weakest.
fn attuma(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let power = game.cards[card].power();
    let targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| {
            id != card
                && game.cards[id].owner == owner
                && game.cards[id].revealed
                && game.cards[id].power() < power
        })
        .collect();
    if !targets.is_empty() {
        let index = game.rng.borrow_mut().randbelow(targets.len());
        game.destroy_card(targets[index]);
    }
}

// NamoritaBehavior::ReduceDrawnCardCost (0x7f0a68c) counts all allies,
// including face-down cards, and permanently reduces the drawn card by one.
fn namorita(game: &mut Game<'_>, card: usize, event: Event) {
    let Event::CardDrawn { card: drawn, seat } = event else {
        return;
    };
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    if seat == owner
        && game.lanes[lane]
            .cards
            .iter()
            .filter(|&&id| game.cards[id].owner == owner)
            .count()
            == 1
    {
        game.add_cost(drawn, -1, owner);
    }
}

// TopazBehavior::MoveToMiddle (0x7cdff9c) checks where the card was played.
fn topaz(game: &mut Game<'_>, card: usize, event: Event) {
    let Event::CardResolved { card: target } = event else {
        return;
    };
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let played_lane = game.played[owner]
        .iter()
        .find_map(|&(id, lane)| (id == target).then_some(lane));
    if lane != 1 && game.cards[target].owner == owner && played_lane == Some(lane) {
        game.move_card(target, 1);
    }
}

// BroodBehavior::AddBroodlings (0x7e76ad8) captures
// Card::GetPowerWithModifiersOnly (0x7cb0fc0), excluding ongoing/location power.
fn brood(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let power = game.cards[card].permanent;
    for _ in 0..2 {
        if !game.has_space(lane, owner, 0) {
            break;
        }
        let token = game.create_card("Broodling", owner);
        game.set_power(token, power, owner);
        if !game.add_card(token, lane, owner, true) {
            break;
        }
    }
}

// StickBehavior::AddBuffCardTriggerToPlayer (0x7cc684c), BuffCard
// (0x7cc6938): the player trigger reads Stick's current power on the next play.
fn stick(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    game.add_player_effect(
        owner,
        PendingEffect {
            source: card,
            lane: game.cards[card].lane,
            created_turn: game.turn,
            trigger: PlayerTrigger::BeforeCardRevealed,
            lifetime: EffectLifetime::NextCard,
            state: Box::new(()),
            apply: stick_buff,
        },
    );
}

fn stick_buff(game: &mut Game<'_>, effect: &mut PendingEffect, target: Option<usize>) -> bool {
    let Some(target) = target else {
        return false;
    };
    let power = game.cards[effect.source].power();
    game.add_power(target, power, game.cards[target].owner);
    true
}

// WaveBehavior::AddSetCostAbilityToPlayer (0x7ceedac): both players'
// cards cost at most four through the end of the next turn.
fn wave(game: &mut Game<'_>, _card: usize) {
    for player in &mut game.players {
        player.wave_until = Some(game.turn.saturating_add(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{locations, random::PythonRandom};

    fn game(rng: &mut PythonRandom) -> Game<'_> {
        let deck = [
            "Abomination",
            "Cyclops",
            "Hulk",
            "MistyKnight",
            "Shocker",
            "TheThing",
            "Wasp",
            "Quicksilver",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
        ]
        .map(str::to_owned);
        let names = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &names, rng).unwrap();
        for lane in &mut game.lanes {
            lane.def = locations::get("Ruins").unwrap();
            lane.revealed = true;
        }
        game.turn = 2;
        game
    }

    fn place(game: &mut Game<'_>, id: &str, owner: usize, lane: usize) -> usize {
        let card = game.create_card(id, owner);
        game.cards[card].lane = Some(lane);
        game.cards[card].revealed = true;
        game.lanes[lane].cards.push(card);
        card
    }

    #[test]
    fn wave_caps_both_hands_through_next_turn() {
        let mut rng = PythonRandom::seed(1);
        let mut game = game(&mut rng);
        let source = place(&mut game, "Wave", 0, 0);
        let own = game.create_card("Hulk", 0);
        let opposing = game.create_card("Hulk", 1);
        let cheap = game.create_card("Wasp", 0);
        game.players[0].hand.extend([own, cheap]);
        game.players[1].hand.push(opposing);
        wave(&mut game, source);
        assert_eq!(game.card_cost(own), 4);
        assert_eq!(game.card_cost(opposing), 4);
        assert_eq!(game.card_cost(cheap), 0);
        game.turn += 1;
        assert_eq!(game.card_cost(own), 4);
        game.turn += 1;
        assert_eq!(game.card_cost(own), 6);
    }

    #[test]
    fn stick_uses_source_power_for_next_card() {
        let mut rng = PythonRandom::seed(1);
        let mut game = game(&mut rng);
        let source = place(&mut game, "Stick", 0, 0);
        stick(&mut game, source);
        game.add_power(source, 2, 0);
        let target = place(&mut game, "Hulk", 0, 0);
        let before = game.cards[target].power();
        let mut effect = game.players[0].pending_effects.pop().unwrap();
        assert!(stick_buff(&mut game, &mut effect, Some(target)));
        assert_eq!(game.cards[target].power(), before + 5);
    }

    #[test]
    fn restored_player_effects_fail_closed_but_hand_is_clean() {
        use crate::{
            coverage,
            position::{CardRef, Position},
        };
        let mut position = Position {
            turn: 4,
            ..Position::default()
        };
        for id in ["Stick", "Wave"] {
            position.hand.push(CardRef {
                id: id.into(),
                ..CardRef::default()
            });
        }
        let clean = coverage::coverage(&position);
        assert!(!clean["state"].to_string().contains("effect may be active"));
        position.locations[0].mine = position.hand.clone();
        for card in &mut position.locations[0].mine {
            card.revealed = true;
        }
        let blocked = coverage::coverage(&position);
        assert!(
            blocked["state"]
                .to_string()
                .contains("Stick effect may be active")
        );
        assert!(
            blocked["state"]
                .to_string()
                .contains("Wave effect may be active")
        );
    }

    #[test]
    fn wave_history_expires_only_after_the_following_turn() {
        use crate::{
            coverage,
            position::{CardRef, Position},
        };
        let mut position = Position {
            turn: 4,
            ..Position::default()
        };
        position.locations[0].mine.push(CardRef {
            id: "Wave".into(),
            revealed: true,
            entity_id: Some(10),
            latest_resolution_order: Some(1),
            ..CardRef::default()
        });
        position.history = Some(
            serde_json::from_value(serde_json::json!({
                "actions": [{"owner":"local", "turn":3, "kind":"play", "location_slot":0,
                    "card_entity_id":10, "card_id":"Wave", "resolved":true,
                    "turn_resolved":3, "resolution_order":1}]
            }))
            .unwrap(),
        );
        assert!(
            coverage::coverage(&position)["state"]
                .to_string()
                .contains("Wave effect may be active")
        );
        position.turn = 5;
        assert!(
            !coverage::coverage(&position)["state"]
                .to_string()
                .contains("Wave effect may be active")
        );
    }

    #[test]
    fn stick_complete_history_can_prove_consumption() {
        use crate::{
            coverage,
            position::{CardRef, Position},
        };
        let mut position = Position {
            turn: 4,
            ..Position::default()
        };
        position.locations[0].mine.push(CardRef {
            id: "Stick".into(),
            revealed: true,
            entity_id: Some(10),
            latest_resolution_order: Some(1),
            ..CardRef::default()
        });
        position.locations[0].mine.push(CardRef {
            id: "Wasp".into(),
            revealed: true,
            entity_id: Some(11),
            ..CardRef::default()
        });
        position.history = Some(serde_json::from_value(serde_json::json!({
            "actions": [
                {"owner":"local", "turn":2, "kind":"play", "location_slot":0,
                 "card_entity_id":10, "card_id":"Stick", "resolved":true, "turn_resolved":2, "resolution_order":1},
                {"owner":"local", "turn":3, "kind":"play", "location_slot":0,
                 "card_entity_id":11, "card_id":"Wasp", "resolved":true, "turn_resolved":3, "resolution_order":2}
            ],
            "action_completeness":{"local":{"1":true,"2":true,"3":true},"opponent":{}}
        })).unwrap());
        for lane in &mut position.locations {
            lane.id = Some("Ruins".into());
        }
        assert!(
            !coverage::coverage(&position)["state"]
                .to_string()
                .contains("Stick effect may be active")
        );
        position
            .history
            .as_mut()
            .unwrap()
            .actions
            .as_mut()
            .unwrap()
            .pop();
        assert!(
            coverage::coverage(&position)["state"]
                .to_string()
                .contains("Stick effect may be active")
        );
    }

    #[test]
    fn brood_copies_permanent_power_and_respects_space() {
        let mut rng = PythonRandom::seed(1);
        let mut game = game(&mut rng);
        let source = place(&mut game, "Brood", 0, 0);
        game.add_power(source, 3, 0);
        game.cards[source].ongoing = 4;
        place(&mut game, "Wasp", 0, 0);
        brood(&mut game, source);
        let tokens: Vec<_> = game.lanes[0]
            .cards
            .iter()
            .copied()
            .filter(|&id| game.cards[id].def.id == "Broodling")
            .collect();
        assert_eq!(tokens.len(), 2);
        assert!(tokens.iter().all(|&id| game.cards[id].permanent == 5));
        brood(&mut game, source);
        assert_eq!(game.lanes[0].cards.len(), 4);
    }

    #[test]
    fn attuma_destroys_only_revealed_weaker_ally() {
        let mut rng = PythonRandom::seed(1);
        let mut game = game(&mut rng);
        let source = place(&mut game, "Attuma", 0, 0);
        let weak = place(&mut game, "Wasp", 0, 0);
        let hidden = place(&mut game, "Wasp", 0, 0);
        game.cards[hidden].revealed = false;
        let enemy = place(&mut game, "Wasp", 1, 0);
        attuma(&mut game, source);
        assert!(game.cards[weak].lane.is_none());
        assert_eq!(game.cards[hidden].lane, Some(0));
        assert_eq!(game.cards[enemy].lane, Some(0));
    }

    #[test]
    fn namorita_requires_being_alone_and_own_draw() {
        let mut rng = PythonRandom::seed(1);
        let mut game = game(&mut rng);
        let source = place(&mut game, "Namorita", 0, 0);
        let target = game.create_card("Hulk", 0);
        let before = game.cards[target].cost;
        namorita(
            &mut game,
            source,
            Event::CardDrawn {
                card: target,
                seat: 1,
            },
        );
        assert_eq!(game.cards[target].cost, before);
        namorita(
            &mut game,
            source,
            Event::CardDrawn {
                card: target,
                seat: 0,
            },
        );
        assert_eq!(game.cards[target].cost, before - 1);
        let ally = place(&mut game, "Wasp", 0, 0);
        game.cards[ally].revealed = false;
        namorita(
            &mut game,
            source,
            Event::CardDrawn {
                card: target,
                seat: 0,
            },
        );
        assert_eq!(game.cards[target].cost, before - 1);
    }

    #[test]
    fn topaz_moves_only_own_cards_played_at_her_lane() {
        let mut rng = PythonRandom::seed(1);
        let mut game = game(&mut rng);
        let source = place(&mut game, "Topaz", 0, 0);
        let target = place(&mut game, "Wasp", 0, 0);
        game.played[0].push((target, 2));
        topaz(&mut game, source, Event::CardResolved { card: target });
        assert_eq!(game.cards[target].lane, Some(0));
        game.played[0].clear();
        game.played[0].push((target, 0));
        topaz(&mut game, source, Event::CardResolved { card: target });
        assert_eq!(game.cards[target].lane, Some(1));
    }

    #[test]
    fn nakia_only_buffs_own_hand() {
        let mut rng = PythonRandom::seed(1);
        let mut game = game(&mut rng);
        let source = game.create_card("Nakia", 0);
        let ally = game.create_card("Hulk", 0);
        let enemy = game.create_card("Hulk", 1);
        game.players[0].hand.push(ally);
        game.players[1].hand.push(enemy);
        let before = game.cards[ally].power();
        nakia(&mut game, source);
        assert_eq!(game.cards[ally].power(), before + 1);
        assert_eq!(game.cards[enemy].power(), before);
    }

    #[test]
    fn hobgoblin_switches_only_with_space() {
        let mut rng = PythonRandom::seed(1);
        let mut game = game(&mut rng);
        let source = game.create_card("Hobgoblin", 0);
        assert!(game.add_card(source, 0, 0, false));
        hobgoblin(&mut game, source);
        assert_eq!(game.cards[source].owner, 1);
        for _ in 0..4 {
            let card = game.create_card("Wasp", 0);
            assert!(game.add_card(card, 0, 0, false));
        }
        hobgoblin(&mut game, source);
        assert_eq!(game.cards[source].owner, 1);
    }
}
