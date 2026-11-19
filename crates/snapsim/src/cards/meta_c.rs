//! Meta-deck cards, batch C: Activate and Move; rules from the 59.15.1 client.
use super::CardDef;
use crate::game::{EffectLifetime, Event, Game, PendingEffect, PlayerTrigger};

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        on_activate: Some(karima),
        ..CardDef::vanilla("KarimaShapandar")
    },
    CardDef {
        on_activate: Some(kid_omega),
        ..CardDef::vanilla("KidOmega")
    },
    CardDef {
        on_activate: Some(king_eitri),
        ..CardDef::vanilla("KingEitri")
    },
    CardDef {
        reveal: Some(zombie_reveal),
        on_activate: Some(zombie_activate),
        ..CardDef::vanilla("ZombieMisterFantastic")
    },
    CardDef {
        on_move: Some(spider_move),
        move_limit: 2,
        ..CardDef::vanilla("SpiderManBrandNewDay")
    },
    CardDef {
        on_event: Some(jeff_event),
        move_limit: 1,
        ..CardDef::vanilla("JeffTheBabyDolphin")
    },
    CardDef {
        on_activate: Some(symbiote),
        ..CardDef::vanilla("SymbioteSpiderMan")
    },
];
pub(crate) static TOKENS: &[&str] = &[];
fn symbiote(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let mut lowest = i32::MAX;
    let mut tied = [0; 4];
    let mut count = 0;
    for &id in &game.lanes[lane].cards {
        if id == card || game.cards[id].owner != owner || !game.cards[id].revealed {
            continue;
        }
        let cost = game.card_cost(id);
        if cost < lowest {
            lowest = cost;
            count = 0;
        }
        if cost == lowest {
            tied[count] = id;
            count += 1;
        }
    }
    if count == 0 {
        return;
    }
    // MergeLowestCostIntoSelf (0x7cd1b78) rolls even for a single lowest-cost ally.
    let index = game.rng.borrow_mut().randbelow(count);
    let target = tied[index];
    if game.merge_card(card, target) {
        game.copy_text(card, target, false);
        game.clone_external_abilities(card, target);
        game.trigger_on_reveal(card);
    }
}

fn karima(game: &mut Game<'_>, card: usize) {
    if game.cards[card].lane.is_none() {
        return;
    }
    let owner = game.cards[card].owner;
    // GetAllRevealedCards (0x7dac98c) sorts by latest resolution order.
    let mut candidates = game.revealed_cards_in_resolution_order();
    // GetHighestPowerCards (predicate 0x7dc6ecc) excludes cards without Power.
    candidates.retain(|&id| game.cards[id].owner != owner && game.is_character(id));
    let Some(highest) = candidates.iter().map(|&id| game.cards[id].power()).max() else {
        return;
    };
    let ties = candidates
        .iter()
        .filter(|&&id| game.cards[id].power() == highest)
        .count();
    // AfflictCard (0x7edab44); Game::Roll (0x7dab62c) also rolls for one candidate.
    let pick = game.rng.borrow_mut().randbelow(ties);
    let target = candidates
        .into_iter()
        .filter(|&id| game.cards[id].power() == highest)
        .nth(pick)
        .expect("selected a counted target");
    game.add_power(target, -7, owner);
}

fn kid_omega(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    // AddDestroyTrigger (0x7ede1b4) creates a player-owned trigger for this turn.
    game.add_player_effect(
        owner,
        PendingEffect {
            source: card,
            lane: game.cards[card].lane,
            created_turn: game.turn,
            trigger: PlayerTrigger::CardPlayed,
            lifetime: EffectLifetime::ThisTurn,
            state: Box::new(()),
            apply: kid_destroy,
        },
    );
}

fn kid_destroy(game: &mut Game<'_>, effect: &mut PendingEffect, target: Option<usize>) -> bool {
    let Some(target) = target else {
        return false;
    };
    if game.turn != effect.created_turn {
        return false;
    }
    // DestroyCard (0x7ede2f4) snapshots current power before attempting destruction.
    let power = game.cards[target].power();
    let owner = game.cards[target].owner;
    if game.destroy_card(target) {
        game.add_power(effect.source, power, owner);
    }
    true
}

fn king_eitri(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let count = game.players[owner]
        .deck
        .iter()
        .filter(|&&id| game.cards[id].created)
        .count();
    if count == 0 {
        return;
    }
    // DrawCreatedCard (0x7edf338) rolls before Player::DrawCard checks the hand.
    // Game::Roll (0x7dab62c) is called even when there is only one candidate.
    let pick = game.rng.borrow_mut().randbelow(count);
    let slot = game.players[owner]
        .deck
        .iter()
        .enumerate()
        .filter(|&(_, &id)| game.cards[id].created)
        .nth(pick)
        .map(|(slot, _)| slot)
        .expect("selected a counted created card");
    if game.players[owner].hand.len() < 7 {
        let target = game.players[owner].deck.remove(slot);
        game.add_hand(owner, target, true);
    }
}

fn zombie_reveal(game: &mut Game<'_>, card: usize) {
    if let Some(lane) = game.cards[card].lane {
        let owner = game.cards[card].owner;
        game.horde(lane, owner, 2);
    }
}

fn zombie_activate(game: &mut Game<'_>, card: usize) {
    if game.cards[card].lane.is_none() {
        return;
    }
    let owner = game.cards[card].owner;
    let mut targets = game.revealed_cards_in_resolution_order();
    targets.retain(|&id| game.cards[id].owner == owner && game.cards[id].created);
    // BuffCreated (0x7cfcc38) shuffles the entire list even when it has at most four.
    // Game::Shuffle (0x4b12f30) uses forward Fisher–Yates and Game::Roll.
    for i in 0..targets.len().saturating_sub(1) {
        let j = i + game.rng.borrow_mut().randbelow(targets.len() - i);
        targets.swap(i, j);
    }
    for target in targets.into_iter().take(4) {
        game.add_power(target, 1, owner);
    }
}

fn spider_move(game: &mut Game<'_>, card: usize, _from: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    if game.cards[card].abilities_removed {
        return;
    }
    let owner = game.cards[card].owner;
    let count = game.lanes[lane]
        .cards
        .iter()
        .filter(|&&id| id != card && game.cards[id].owner == owner)
        .count();
    if count == 0 {
        return;
    }
    // BuffCard (0x7f4cd0c) includes face-down allies; Game::Roll (0x7dab62c).
    let pick = game.rng.borrow_mut().randbelow(count);
    let target = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| id != card && game.cards[id].owner == owner)
        .nth(pick)
        .expect("selected a counted ally");
    game.add_power(target, 1, owner);
}

fn jeff_event(game: &mut Game<'_>, card: usize, event: Event) {
    let Event::BeforeCardRevealed { card: target } = event else {
        return;
    };
    // BuffCard (0x7ed33b8) compares the current zones, not the staged play slot.
    if game.cards[card].lane.is_some()
        && game.cards[card].lane == game.cards[target].lane
        && game.cards[card].owner == game.cards[target].owner
    {
        game.add_power(target, 1, game.cards[card].owner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{locations, random::PythonRandom};

    fn game(rng: &mut PythonRandom) -> Game<'_> {
        let deck: Vec<String> = [
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
        .into_iter()
        .map(str::to_owned)
        .collect();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, rng).unwrap();
        for lane in &mut game.lanes {
            lane.def = locations::get("Ruins").unwrap();
            lane.revealed = true;
        }
        game
    }

    fn place(game: &mut Game<'_>, id: &str, owner: usize, lane: usize, revealed: bool) -> usize {
        let card = game.create_card(id, owner);
        game.cards[card].lane = Some(lane);
        game.cards[card].revealed = revealed;
        game.lanes[lane].cards.push(card);
        card
    }

    #[test]
    fn karima_afflicts_only_a_highest_revealed_enemy() {
        let mut rng = PythonRandom::seed(1);
        let mut game = game(&mut rng);
        let source = place(&mut game, "KarimaShapandar", 0, 0, true);
        let weak = place(&mut game, "Wasp", 1, 0, true);
        let high = place(&mut game, "Hulk", 1, 1, true);
        let hidden = place(&mut game, "Hulk", 1, 2, false);
        let power = game.cards[high].power();
        karima(&mut game, source);
        assert_eq!(game.cards[high].power(), power - 7);
        assert_eq!(game.cards[weak].permanent, 1);
        assert_eq!(game.cards[hidden].permanent, power);
    }

    #[test]
    fn karima_ties_can_select_either_enemy() {
        let mut seen = [false; 2];
        for seed in 0..64 {
            let mut rng = PythonRandom::seed(seed);
            let mut game = game(&mut rng);
            let source = place(&mut game, "KarimaShapandar", 0, 0, true);
            let targets = [
                place(&mut game, "Hulk", 1, 0, true),
                place(&mut game, "Hulk", 1, 1, true),
            ];
            let power = game.cards[targets[0]].power();
            karima(&mut game, source);
            for i in 0..2 {
                seen[i] |= game.cards[targets[i]].power() == power - 7;
            }
        }
        assert_eq!(seen, [true; 2]);
    }

    #[test]
    fn kid_omega_consumes_next_play_and_uses_its_pre_destroy_power() {
        let mut rng = PythonRandom::seed(2);
        let mut game = game(&mut rng);
        let source = place(&mut game, "KidOmega", 0, 0, true);
        let target = place(&mut game, "Hulk", 0, 1, false);
        game.cards[target].turn_played = game.turn;
        let power = game.cards[target].power();
        let old = game.cards[source].power();
        kid_omega(&mut game, source);
        game.reveal(target);
        assert!(game.cards[target].lane.is_none());
        assert_eq!(game.cards[source].power(), old + power);
        assert!(game.players[0].pending_effects.is_empty());
    }

    #[test]
    fn kid_omega_expires_at_end_of_activation_turn() {
        let mut rng = PythonRandom::seed(7);
        let mut game = game(&mut rng);
        let source = place(&mut game, "KidOmega", 0, 0, true);
        kid_omega(&mut game, source);
        game.end_turn_effects();
        assert!(game.players[0].pending_effects.is_empty());
    }

    #[test]
    fn eitri_draws_created_only_and_keeps_full_hand_intact() {
        let mut rng = PythonRandom::seed(3);
        let mut game = game(&mut rng);
        let source = place(&mut game, "KingEitri", 0, 0, true);
        let target = game.create_card("Hulk", 0);
        game.players[0].deck.push(target);
        king_eitri(&mut game, source);
        assert!(game.players[0].hand.contains(&target));
        assert!(!game.players[0].deck.contains(&target));
        let blocked = game.create_card("Cyclops", 0);
        game.players[0].deck.push(blocked);
        while game.players[0].hand.len() < 7 {
            let filler = game.create_card("Wasp", 0);
            game.add_hand(0, filler, false);
        }
        king_eitri(&mut game, source);
        assert!(game.players[0].deck.contains(&blocked));
    }

    #[test]
    fn eitri_can_choose_each_created_card() {
        let mut seen = [false; 2];
        for seed in 0..64 {
            let mut rng = PythonRandom::seed(seed);
            let mut game = game(&mut rng);
            let source = place(&mut game, "KingEitri", 0, 0, true);
            let cards = [game.create_card("Hulk", 0), game.create_card("Cyclops", 0)];
            game.players[0].deck.extend(cards);
            king_eitri(&mut game, source);
            for i in 0..2 {
                seen[i] |= game.players[0].hand.contains(&cards[i]);
            }
        }
        assert_eq!(seen, [true; 2]);
    }

    #[test]
    fn zombie_horde_and_created_only_activation() {
        let mut rng = PythonRandom::seed(4);
        let mut game = game(&mut rng);
        let source = place(&mut game, "ZombieMisterFantastic", 0, 0, true);
        game.cards[source].created = false;
        zombie_reveal(&mut game, source);
        let horde = game
            .lanes
            .iter()
            .flat_map(|lane| lane.cards.iter())
            .copied()
            .find(|&id| game.cards[id].def.id == "ZombieHorde")
            .unwrap();
        let ally = place(&mut game, "Hulk", 0, 0, true);
        game.cards[ally].created = false;
        let hidden = place(&mut game, "Hulk", 0, 2, false);
        let ally_power = game.cards[ally].power();
        let hidden_power = game.cards[hidden].power();
        let before = game.cards[horde].power();
        zombie_activate(&mut game, source);
        assert_eq!(game.cards[horde].power(), before + 1);
        assert_eq!(game.cards[ally].power(), ally_power);
        assert_eq!(game.cards[hidden].power(), hidden_power);
    }

    #[test]
    fn zombie_randomly_buffs_four_distinct_created_cards() {
        let mut rng = PythonRandom::seed(9);
        let mut game = game(&mut rng);
        let source = place(&mut game, "ZombieMisterFantastic", 0, 0, true);
        game.cards[source].created = false;
        let cards = [
            place(&mut game, "Hulk", 0, 0, true),
            place(&mut game, "Hulk", 0, 0, true),
            place(&mut game, "Hulk", 0, 1, true),
            place(&mut game, "Hulk", 0, 1, true),
            place(&mut game, "Hulk", 0, 2, true),
        ];
        let before = game.cards[cards[0]].power();
        zombie_activate(&mut game, source);
        assert_eq!(
            cards
                .iter()
                .filter(|&&id| game.cards[id].power() == before + 1)
                .count(),
            4
        );
        assert_eq!(
            cards
                .iter()
                .filter(|&&id| game.cards[id].power() == before)
                .count(),
            1
        );
    }

    #[test]
    fn spider_moves_twice_and_buffs_face_down_allies() {
        let mut rng = PythonRandom::seed(5);
        let mut game = game(&mut rng);
        let source = place(&mut game, "SpiderManBrandNewDay", 0, 0, true);
        let target = place(&mut game, "Hulk", 0, 1, false);
        let power = game.cards[target].power();
        assert!(game.move_player_card(source, 1, 0));
        assert_eq!(game.cards[target].power(), power + 1);
        assert!(game.move_player_card(source, 2, 0));
        assert!(!game.move_player_card(source, 0, 0));
    }

    #[test]
    fn jeff_buffs_ally_before_reveal_but_not_enemy_or_other_lane() {
        let mut rng = PythonRandom::seed(6);
        let mut game = game(&mut rng);
        let source = place(&mut game, "JeffTheBabyDolphin", 0, 0, true);
        let target = place(&mut game, "Hawkeye", 0, 0, false);
        let enemy = place(&mut game, "Hawkeye", 1, 0, false);
        let elsewhere = place(&mut game, "Hawkeye", 0, 1, false);
        let power = game.cards[target].power();
        jeff_event(&mut game, source, Event::BeforeCardRevealed { card: enemy });
        jeff_event(
            &mut game,
            source,
            Event::BeforeCardRevealed { card: elsewhere },
        );
        jeff_event(
            &mut game,
            source,
            Event::BeforeCardRevealed { card: target },
        );
        assert_eq!(game.cards[target].power(), power + 1);
        assert_eq!(game.cards[enemy].power(), power);
        assert_eq!(game.cards[elsewhere].power(), power);
    }

    #[test]
    fn symbiote_copies_nico_spell_and_preparation_context() {
        let mut rng = PythonRandom::seed(17);
        let mut game = game(&mut rng);
        let nico = game.create_card("NicoMinoru", 0);
        game.players[0].hand.push(nico);
        game.prepare_nico(nico, 0);
        let spell = game.cards[nico].ability_def().id;
        game.players[0].hand.retain(|&id| id != nico);
        game.cards[nico].lane = Some(0);
        game.cards[nico].revealed = true;
        game.lanes[0].cards.push(nico);
        let sym = place(&mut game, "SymbioteSpiderMan", 0, 0, true);
        symbiote(&mut game, sym);
        assert_eq!(game.cards[sym].ability_def().id, spell);
        assert_eq!(game.cards[nico].merged_to, Some(sym));
        assert_eq!(game.players[0].pending_effects.len(), 1);
        assert!(matches!(game.cards[sym].attached_abilities[0].kind,
            crate::game::AttachedAbilityKind::NicoPrepare { used } if used.count_ones() == 1));
        assert!(game.take_from_board(sym));
        game.players[0].hand.push(sym);
        game.prepare_nico(sym, 0);
        assert_ne!(game.cards[sym].ability_def().id, spell);
    }

    #[test]
    fn symbiote_clones_watches_and_adds_copied_reveal() {
        for name in ["Hawkeye", "JessicaJones"] {
            let mut rng = PythonRandom::seed(19);
            let mut game = game(&mut rng);
            let target = place(&mut game, name, 0, 0, true);
            game.trigger_on_reveal(target);
            let sym = place(&mut game, "SymbioteSpiderMan", 0, 0, true);
            symbiote(&mut game, sym);
            assert_eq!(
                game.cards[sym].watch_contexts(),
                vec![(game.turn + 1, 0); 2]
            );
            game.turn += 1;
            if name == "Hawkeye" {
                let play = place(&mut game, "Wasp", 0, 0, false);
                game.cards[play].turn_played = game.turn;
                game.played[0].push((play, 0));
            }
            let before = game.cards[sym].power();
            game.resolve_attached_watches(sym);
            assert_eq!(
                game.cards[sym].power(),
                before + if name == "Hawkeye" { 8 } else { 10 }
            );
            assert!(game.cards[sym].watch_contexts().is_empty());
        }
    }

    #[test]
    fn symbiote_copies_removed_text_and_modifier_only_power() {
        let mut rng = PythonRandom::seed(21);
        let mut game = game(&mut rng);
        let target = place(&mut game, "Hawkeye", 0, 0, true);
        game.add_power(target, 3, 0);
        game.cards[target].ongoing = 5;
        game.cards[target].location_power = 2;
        game.cards[target].abilities_removed = true;
        let sym = place(&mut game, "SymbioteSpiderMan", 0, 0, true);
        let before = game.cards[sym].permanent;
        let permanent = game.cards[target].permanent;
        symbiote(&mut game, sym);
        assert_eq!(game.cards[sym].permanent, before + permanent);
        assert!(game.cards[sym].abilities_removed);
        assert!(game.cards[sym].watch_contexts().is_empty());
    }

    #[test]
    fn symbiote_clones_staged_web_sling_trigger() {
        use crate::game::{Action, MoveGrant};
        fn power(game: &mut Game<'_>, source: usize, target: usize) {
            game.add_power(target, 1, game.cards[source].owner);
        }
        let mut rng = PythonRandom::seed(20);
        let mut game = game(&mut rng);
        let skill = game.create_card("WebSling", 0);
        let target = place(&mut game, "Nightcrawler", 0, 0, true);
        let sym = place(&mut game, "SymbioteSpiderMan", 0, 1, true);
        game.grant_move(MoveGrant {
            seat: 0,
            source: skill,
            card: Some(target),
            lane: None,
            start_turn: game.turn,
            end_turn: game.turn,
            remaining: 1,
            on_move: Some(power),
        });
        game.stage_action(
            0,
            Action::Move {
                card: target,
                lane: 1,
            },
        )
        .unwrap();
        symbiote(&mut game, sym);
        let before = game.cards[sym].permanent;
        assert!(game.move_card(sym, 2));
        assert_eq!(game.cards[sym].permanent, before + 1);
    }

    #[test]
    fn external_clone_filters_default_id_and_replaced_owner() {
        let mut rng = PythonRandom::seed(22);
        let mut game = game(&mut rng);
        let source = game.create_card("NicoMinoru", 0);
        let receiver = game.create_card("NicoMinoru", 0);
        game.clone_external_abilities(receiver, source);
        assert_eq!(game.cards[receiver].attached_abilities.len(), 1);
        let other = game.create_card("SymbioteSpiderMan", 0);
        game.cards[source].replaced_card_def_id = Some("NicoMinoru");
        game.clone_external_abilities(other, source);
        assert!(game.cards[other].attached_abilities.is_empty());
    }

    #[test]
    fn copied_text_removes_self_origin_watch_but_keeps_cloned_watch() {
        let mut rng = PythonRandom::seed(27);
        let mut game = game(&mut rng);
        let source = place(&mut game, "Hawkeye", 0, 0, true);
        game.trigger_on_reveal(source);
        let receiver = place(&mut game, "SymbioteSpiderMan", 0, 0, true);
        symbiote(&mut game, receiver);
        assert_eq!(game.cards[receiver].watch_contexts().len(), 2);
        let replacement = game.create_card("Medusa", 0);
        game.copy_text(receiver, replacement, false);
        assert_eq!(
            game.cards[receiver].watch_contexts(),
            vec![(game.turn + 1, 0)]
        );
        assert_eq!(
            game.cards[receiver].attached_abilities[0].origin,
            Some(source)
        );
    }

    #[test]
    fn clone_duplicate_id_only_blocks_receiver_default_ability() {
        let mut rng = PythonRandom::seed(28);
        let mut game = game(&mut rng);
        let source = place(&mut game, "Hawkeye", 0, 0, true);
        game.trigger_on_reveal(source);
        let receiver = place(&mut game, "SymbioteSpiderMan", 0, 0, true);
        game.clone_external_abilities(receiver, source);
        game.clone_external_abilities(receiver, source);
        assert_eq!(game.cards[receiver].watch_contexts().len(), 2);
        assert_ne!(
            game.cards[receiver].attached_abilities[0].index,
            game.cards[receiver].attached_abilities[1].index
        );
        assert_ne!(
            game.cards[receiver].attached_abilities[0].index,
            game.cards[source].attached_abilities[0].index
        );
        game.cards[receiver].attach_watch(receiver, game.turn + 1, 0, true, 4);
        game.clone_external_abilities(receiver, source);
        assert_eq!(game.cards[receiver].watch_contexts().len(), 3);
    }
}
