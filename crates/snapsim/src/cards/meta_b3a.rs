//! Meta-deck cards, batch B3a: play and draw tracking, in-hand effects, random-card pools; rules from the 59.15.1 client.
use super::CardDef;
use crate::game::{EffectLifetime, Event, Game, PendingEffect, PlayerTrigger};

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        in_hand_ongoing: Some(sasquatch),
        ..CardDef::vanilla("Sasquatch")
    },
    CardDef {
        reveal: Some(shou_lao),
        ..CardDef::vanilla("ShouLao")
    },
    CardDef {
        blocks_spawns: true,
        ..CardDef::vanilla("Stardust")
    },
    CardDef {
        cost_guard: Some(mobius),
        ..CardDef::vanilla("MobiusMMobius")
    },
    CardDef {
        reveal: Some(kluh),
        ..CardDef::vanilla("Kluh")
    },
    CardDef {
        reveal: Some(maria_hill),
        ..CardDef::vanilla("MariaHill")
    },
    CardDef {
        reveal: Some(valentina),
        ..CardDef::vanilla("Valentina")
    },
    CardDef {
        reveal: Some(iron_patriot),
        ..CardDef::vanilla("IronPatriot")
    },
    CardDef {
        on_event: Some(jubilee),
        ..CardDef::vanilla("JubileeSilverSurfer")
    },
    CardDef {
        reveal: Some(daredevil),
        on_event: Some(daredevil_draw),
        ..CardDef::vanilla("ShadowlandsDaredevil")
    },
];

pub(crate) static TOKENS: &[&str] = &[];

fn sasquatch(game: &mut Game<'_>, card: usize) {
    let seat = game.cards[card].owner;
    // ReduceCost (0x7f2e920): count all cards staged on the preceding turn.
    let count = game.players[seat]
        .staged_history
        .iter()
        .filter(|play| play.turn.checked_add(1) == Some(game.turn))
        .count() as i32;
    game.add_ongoing_cost(card, -count, seat);
}

fn shou_lao(game: &mut Game<'_>, card: usize) {
    let seat = game.cards[card].owner;
    // GrantPower (0x7f3aba8): find the last character, not simply the last play.
    let target = game.players[seat]
        .play_history
        .iter()
        .rev()
        .find_map(|&(_, id)| game.is_character(id).then_some(id));
    let Some(target) = target.filter(|&id| game.cards[id].lane.is_some()) else {
        return;
    };
    // GetBuffCount (0x7f3aaa0) counts turns with at least two staged cards.
    let count = (1..=game.turn)
        .filter(|&turn| {
            game.players[seat]
                .staged_history
                .iter()
                .filter(|play| play.turn == turn)
                .count()
                >= 2
        })
        .count() as i32;
    game.add_power(target, 2 * count, seat);
}

fn mobius(game: &Game<'_>, guard: usize, target: usize, old: i32, new: i32) -> bool {
    // LockCardCosts (0x7efc3b4) applies only to the players' hands and decks.
    let owner = game.cards[target].owner;
    if !game.players[owner].hand.contains(&target) && !game.players[owner].deck.contains(&target) {
        return true;
    }
    let allied = game.cards[guard].owner == game.cards[target].owner;
    if allied { new <= old } else { new >= old }
}

fn kluh(game: &mut Game<'_>, card: usize) {
    // GetDistinctCostCards (0x7ee0b90) groups revealed cards by printed cost;
    // ApplyCostReductions (0x7ee0d98) rolls once per cost, excluding zero-cost hand cards.
    // ReduceCostsForBothPlayers (0x7ee0950) handles the owner, then the opponent.
    let owner = game.cards[card].owner;
    for seat in [owner, 1 - owner] {
        let mut costs = Vec::new();
        for lane in &game.lanes {
            for &id in &lane.cards {
                if game.cards[id].owner == seat && (game.cards[id].revealed || id == card) {
                    let cost = game.printed_cost(id);
                    if !costs.contains(&cost) {
                        costs.push(cost);
                    }
                }
            }
        }
        for _ in 0..costs.len() {
            let candidates: Vec<_> = game.players[seat]
                .hand
                .iter()
                .copied()
                .filter(|&id| game.cards[id].cost + game.cards[id].ongoing_cost > 0)
                .collect();
            if candidates.is_empty() {
                break;
            }
            let index = game.rng.borrow_mut().randbelow(candidates.len());
            game.add_cost(candidates[index], -1, game.cards[card].owner);
        }
    }
}

fn random_to_hand(
    game: &mut Game<'_>,
    card: usize,
    min: i32,
    max: Option<i32>,
    iron: bool,
) -> Option<usize> {
    let seat = game.cards[card].owner;
    if game.players[seat].hand.len() >= 7 {
        return None;
    }
    // GetRandomCardDefIds (0x7db1dd8) makes one roll after filtering.
    // IronPatriot::.cctor (0x7ed023c) excludes these three definitions.
    let name = game.random_card(|row| {
        row.cost >= min
            && max.is_none_or(|max| row.cost <= max)
            && (!iron
                || !matches!(
                    row.carddefid.as_str(),
                    "AgathaHarkness" | "BlackCat" | "MrNegative"
                ))
    })?;
    let target = game.create_card(name, seat);
    game.add_hand(seat, target, false).then_some(target)
}

fn maria_hill(game: &mut Game<'_>, card: usize) {
    // AddCard predicate (0x7ef2e38): printed cost equals two.
    random_to_hand(game, card, 2, Some(2), false);
}

fn valentina(game: &mut Game<'_>, card: usize) {
    // AddCard (0x7ce79b4): printed cost six; modify the created card.
    if let Some(target) = random_to_hand(game, card, 6, Some(6), false) {
        let seat = game.cards[card].owner;
        game.add_cost(target, -2, seat);
        game.add_power(target, -2, seat);
    }
}

fn iron_patriot(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let Some(target) = random_to_hand(game, card, 4, None, true) else {
        return;
    };
    let seat = game.cards[card].owner;
    game.add_player_effect(
        seat,
        PendingEffect {
            source: card,
            lane: Some(lane),
            created_turn: game.turn,
            trigger: PlayerTrigger::EndTurn,
            lifetime: EffectLifetime::UntilTriggered,
            state: Box::new(target),
            apply: iron_discount,
        },
    );
}

fn iron_discount(game: &mut Game<'_>, effect: &mut PendingEffect, _: Option<usize>) -> bool {
    if game.turn <= effect.created_turn {
        return false;
    }
    // ReduceCardCost (0x7ecfd94) checks next turn's winner, even if the target left hand.
    let seat = game.cards[effect.source].owner;
    let lane = effect.lane.unwrap();
    if game.lane_winner(lane) == Some(seat) {
        let target = *effect.state.as_any().downcast_ref::<usize>().unwrap();
        game.add_cost(target, -3, seat);
    }
    true
}

fn jubilee(game: &mut Game<'_>, card: usize, event: Event) {
    // BuffCardDrawn (0x7ed6498): must be in play, owned draw, and positive power.
    if let Event::CardDrawn { card: target, seat } = event
        && game.cards[card].lane.is_some()
        && !game.cards[card].abilities_removed
        && game.cards[card].owner == seat
        && game.cards[card].power() > 0
    {
        game.add_power(target, game.cards[card].power(), seat);
    }
}

fn daredevil(game: &mut Game<'_>, card: usize) {
    let seat = game.cards[card].owner;
    // ShuffleDemons (0x7f38068) inserts all three then shuffles the entire deck.
    for _ in 0..3 {
        let demon = game.create_card("Demon", seat);
        game.push_deck(seat, demon);
    }
    // Deck::Shuffle (0x7cc0ac0) is Game::Shuffle's forward pass.
    game.shuffle_deck(seat);
}

fn daredevil_draw(game: &mut Game<'_>, card: usize, event: Event) {
    // BuffSelf (0x7f381fc) reads the drawn card's current power.
    if let Event::CardDrawn { card: target, seat } = event
        && game.cards[card].lane.is_some()
        && !game.cards[card].abilities_removed
        && game.cards[card].owner == seat
        && game.cards[target].power() >= 6
    {
        game.add_power(card, 2, seat);
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
        let names: Vec<String> = ["Ruins", "Xandar", "Wakanda"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        let mut game = Game::new([&deck, &deck], &names, rng).unwrap();
        for lane in &mut game.lanes {
            lane.def = locations::get("Ruins").unwrap();
            lane.revealed = true;
        }
        game.turn = 3;
        game
    }

    fn place(game: &mut Game<'_>, name: &str, seat: usize, revealed: bool) -> usize {
        let id = game.create_card(name, seat);
        game.cards[id].lane = Some(0);
        game.cards[id].revealed = revealed;
        game.lanes[0].cards.push(id);
        id
    }

    fn stage(game: &mut Game<'_>, turn: u8, card: usize) {
        let printed_cost = game.printed_cost(card);
        game.players[0]
            .staged_history
            .push(crate::game::StagedPlay {
                turn,
                card,
                def_id: game.cards[card].def.id,
                printed_cost,
                created: false,
                resolved: true,
            });
    }

    #[test]
    fn sasquatch_counts_only_last_turn() {
        let mut rng = PythonRandom::seed(1);
        let mut g = game(&mut rng);
        let s = g.create_card("Sasquatch", 0);
        g.players[0].hand.push(s);
        for turn in [1, 2, 2, 3] {
            stage(&mut g, turn, s);
        }
        g.ongoing();
        assert_eq!(g.cards[s].ongoing_cost, -2);
    }

    #[test]
    fn shou_lao_counts_turns_and_requires_target_in_play() {
        let mut rng = PythonRandom::seed(2);
        let mut g = game(&mut rng);
        let s = place(&mut g, "ShouLao", 0, true);
        let target = place(&mut g, "Hulk", 0, true);
        g.players[0].play_history = vec![(1, target), (1, target), (2, target)];
        for turn in [1, 1, 2] {
            stage(&mut g, turn, target);
        }
        let power = g.cards[target].power();
        shou_lao(&mut g, s);
        assert_eq!(g.cards[target].power(), power + 2);
        g.cards[target].lane = None;
        shou_lao(&mut g, s);
        assert_eq!(g.cards[target].power(), power + 2);
    }

    #[test]
    fn stardust_stops_spawning_only_while_active() {
        let mut rng = PythonRandom::seed(3);
        let mut g = game(&mut rng);
        let s = place(&mut g, "Stardust", 0, true);
        assert!(!g.spawns_allowed());
        g.cards[s].abilities_removed = true;
        assert!(g.spawns_allowed());
    }

    #[test]
    fn mobius_blocks_only_disfavored_directions() {
        let mut rng = PythonRandom::seed(4);
        let mut g = game(&mut rng);
        let m = place(&mut g, "MobiusMMobius", 0, true);
        let a = g.create_card("Hulk", 0);
        let b = g.create_card("Hulk", 1);
        g.players[0].hand.push(a);
        g.players[1].deck.push(b);
        assert!(!mobius(&g, m, a, 5, 6));
        assert!(mobius(&g, m, a, 6, 5));
        assert!(!mobius(&g, m, b, 6, 5));
        assert!(mobius(&g, m, b, 5, 6));
    }

    #[test]
    fn kluh_rolls_per_distinct_printed_cost() {
        let mut rng = PythonRandom::seed(5);
        let mut g = game(&mut rng);
        let k = place(&mut g, "Kluh", 0, true);
        place(&mut g, "Hulk", 0, true);
        place(&mut g, "Hulk", 0, false);
        let target = g.create_card("Hulk", 0);
        g.players[0].hand = vec![target];
        kluh(&mut g, k);
        assert_eq!(g.cards[target].cost, 4);
    }

    #[test]
    fn kluh_rolls_for_its_owner_before_the_opponent() {
        fn setup(rng: &mut PythonRandom) -> (Game<'_>, usize, Vec<usize>, Vec<usize>) {
            let mut g = game(rng);
            let k = place(&mut g, "Kluh", 1, true);
            place(&mut g, "Hulk", 0, true);
            let own: Vec<_> = (0..2).map(|_| g.create_card("Hulk", 1)).collect();
            let other: Vec<_> = (0..3).map(|_| g.create_card("Hulk", 0)).collect();
            g.players[1].hand = own.clone();
            g.players[0].hand = other.clone();
            (g, k, own, other)
        }
        let mut rng = PythonRandom::seed(21);
        let (mut g, k, own, other) = setup(&mut rng);
        kluh(&mut g, k);
        let mut expected_rng = PythonRandom::seed(21);
        let (e, _, _, _) = setup(&mut expected_rng);
        let first = e.rng.borrow_mut().randbelow(2);
        let second = e.rng.borrow_mut().randbelow(3);
        for (i, &id) in own.iter().enumerate() {
            assert_eq!(g.cards[id].cost, if i == first { 5 } else { 6 });
        }
        for (i, &id) in other.iter().enumerate() {
            assert_eq!(g.cards[id].cost, if i == second { 5 } else { 6 });
        }
    }

    #[test]
    fn daredevil_shuffles_with_the_forward_pass() {
        let mut rng = PythonRandom::seed(10);
        let mut g = game(&mut rng);
        let d = place(&mut g, "ShadowlandsDaredevil", 0, true);
        let mut expected_rng = PythonRandom::seed(10);
        let mut e = game(&mut expected_rng);
        let ed = place(&mut e, "ShadowlandsDaredevil", 0, true);
        for _ in 0..3 {
            let demon = e.create_card("Demon", 0);
            e.push_deck(0, demon);
        }
        for i in 0..e.players[0].deck.len() - 1 {
            let j = i + e.rng.borrow_mut().randbelow(e.players[0].deck.len() - i);
            e.swap_deck(0, i, j);
        }
        let _ = ed;
        daredevil(&mut g, d);
        assert_eq!(g.players[0].deck, e.players[0].deck);
    }

    #[test]
    fn maria_hill_adds_two_cost_and_respects_full_hand() {
        let mut rng = PythonRandom::seed(6);
        let mut g = game(&mut rng);
        let m = place(&mut g, "MariaHill", 0, true);
        g.players[0].hand.clear();
        maria_hill(&mut g, m);
        assert_eq!(g.printed_cost(g.players[0].hand[0]), 2);
        let id = g.players[0].hand[0];
        g.players[0].hand = vec![id; 7];
        maria_hill(&mut g, m);
        assert_eq!(g.players[0].hand.len(), 7);
    }

    #[test]
    fn valentina_modifies_generated_six_cost() {
        let mut rng = PythonRandom::seed(7);
        let mut g = game(&mut rng);
        let v = place(&mut g, "Valentina", 0, true);
        g.players[0].hand.clear();
        valentina(&mut g, v);
        let target = g.players[0].hand[0];
        assert_eq!(g.printed_cost(target), 6);
        assert_eq!(g.cards[target].cost, 4);
        assert!(
            g.cards[target]
                .stat_updates
                .iter()
                .any(|u| u.stat == crate::game::Stat::Power && u.value == -2)
        );
    }

    #[test]
    fn iron_patriot_discount_checks_next_turn_and_winner() {
        let mut rng = PythonRandom::seed(8);
        let mut g = game(&mut rng);
        let p = place(&mut g, "IronPatriot", 0, true);
        g.players[0].hand.clear();
        iron_patriot(&mut g, p);
        let target = g.players[0].hand[0];
        assert!(g.printed_cost(target) >= 4);
        let mut effect = g.players[0].pending_effects.pop().unwrap();
        assert!(!iron_discount(&mut g, &mut effect, None));
        g.turn += 1;
        let before = g.cards[target].cost;
        assert!(iron_discount(&mut g, &mut effect, None));
        assert_eq!(g.cards[target].cost, before - 3);
    }

    #[test]
    fn jubilee_only_buffs_friendly_draw() {
        let mut rng = PythonRandom::seed(9);
        let mut g = game(&mut rng);
        let j = place(&mut g, "JubileeSilverSurfer", 0, true);
        let target = g.create_card("Hulk", 0);
        let original = g.cards[target].power();
        jubilee(
            &mut g,
            j,
            Event::CardDrawn {
                card: target,
                seat: 1,
            },
        );
        assert_eq!(g.cards[target].power(), original);
        jubilee(
            &mut g,
            j,
            Event::CardDrawn {
                card: target,
                seat: 0,
            },
        );
        assert_eq!(g.cards[target].power(), original + g.cards[j].power());
    }

    #[test]
    fn daredevil_shuffles_three_demons_and_checks_draw_power() {
        let mut rng = PythonRandom::seed(10);
        let mut g = game(&mut rng);
        let d = place(&mut g, "ShadowlandsDaredevil", 0, true);
        let old = g.players[0].deck.len();
        daredevil(&mut g, d);
        assert_eq!(g.players[0].deck.len(), old + 3);
        assert_eq!(
            g.players[0]
                .deck
                .iter()
                .filter(|&&id| g.cards[id].def.id == "Demon")
                .count(),
            3
        );
        let low = g.create_card("Wasp", 0);
        let high = g.create_card("Hulk", 0);
        let power = g.cards[d].power();
        daredevil_draw(&mut g, d, Event::CardDrawn { card: low, seat: 0 });
        daredevil_draw(
            &mut g,
            d,
            Event::CardDrawn {
                card: high,
                seat: 1,
            },
        );
        assert_eq!(g.cards[d].power(), power);
        daredevil_draw(
            &mut g,
            d,
            Event::CardDrawn {
                card: high,
                seat: 0,
            },
        );
        assert_eq!(g.cards[d].power(), power + 2);
    }
}
