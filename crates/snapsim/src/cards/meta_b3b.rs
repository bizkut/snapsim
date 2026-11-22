//! Meta-deck cards, batch B3b: Quickdraw, copied text, rows, game start and end, snapping, spells; rules from the 59.15.1 client.
use super::CardDef;
use crate::game::{EffectLifetime, Event, Game, PendingEffect, PlayerTrigger};

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        reveal: Some(psylocke),
        ..CardDef::vanilla("PsylockeFracturedFrontier")
    },
    CardDef {
        reveal: Some(fin_fang_foom),
        ..CardDef::vanilla("FinFangFoom")
    },
    CardDef {
        reveal: Some(jane),
        ..CardDef::vanilla("JaneFosterFracturedFrontier")
    },
    CardDef {
        on_event: Some(captain_marvel),
        ..CardDef::vanilla("CaptainMarvel")
    },
    CardDef {
        reveal: Some(iron_lad),
        ..CardDef::vanilla("IronLad")
    },
    CardDef {
        reveal: Some(zombie_reveal),
        on_event: Some(zombie_event),
        initial_state: Some(zombie_state),
        ..CardDef::vanilla("ZombieGalacti")
    },
    CardDef {
        end_turn: Some(hydra_bob),
        end_turn_trigger: true,
        ..CardDef::vanilla("HydraBob")
    },
    CardDef::vanilla("NicoMinoru"),
    CardDef {
        reveal: Some(spell01),
        ..CardDef::vanilla("Spell01NicoMinoru")
    },
    CardDef {
        reveal: Some(spell02),
        ..CardDef::vanilla("Spell02NicoMinoru")
    },
    CardDef {
        reveal: Some(spell03),
        ..CardDef::vanilla("Spell03NicoMinoru")
    },
    CardDef {
        reveal: Some(spell04),
        ..CardDef::vanilla("Spell04NicoMinoru")
    },
    CardDef {
        reveal: Some(spell05),
        ..CardDef::vanilla("Spell05NicoMinoru")
    },
    CardDef {
        reveal: Some(spell06),
        ..CardDef::vanilla("Spell06NicoMinoru")
    },
    CardDef {
        reveal: Some(spell07),
        ..CardDef::vanilla("Spell07NicoMinoru")
    },
];

pub(crate) static TOKENS: &[&str] = &[
    "Spell01NicoMinoru",
    "Spell02NicoMinoru",
    "Spell03NicoMinoru",
    "Spell04NicoMinoru",
    "Spell05NicoMinoru",
    "Spell06NicoMinoru",
    "Spell07NicoMinoru",
];

fn quickdraw(game: &Game<'_>, card: usize) -> bool {
    game.is_quickdraw_active(card)
}

/// MoveToBestLocation (0x7e7d6cc) simulates each legal destination on a clone.
fn captain_marvel(game: &mut Game<'_>, card: usize, event: Event) {
    if !matches!(event, Event::GameEnd) {
        return;
    }
    let Some(origin) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let winner = game.winner();
    if winner == owner as i32 {
        return;
    }
    let score = |game: &Game<'_>| {
        game.powers()
            .iter()
            .filter(|power| power[owner] > power[1 - owner])
            .count()
    };
    let mut best_win: Option<(usize, usize)> = None;
    let mut first_tie = (winner == 2).then_some(origin);
    let mut best_loss = (origin, score(game));
    for lane in 0..3 {
        if lane == origin {
            continue;
        }
        let mut rng = game.fork_rng();
        let mut simulation = game.fork(rng.as_mut());
        if !simulation.move_card(card, lane) {
            continue;
        }
        let result = simulation.winner();
        let points = score(&simulation);
        if result == owner as i32 {
            if best_win.is_none_or(|(_, best)| points > best) {
                best_win = Some((lane, points));
            }
        } else if result == 2 {
            if first_tie.is_none() {
                first_tie = Some(lane);
            }
        } else if points > best_loss.1 {
            best_loss = (lane, points);
        }
    }
    let destination = best_win
        .map(|(lane, _)| lane)
        .or(first_tie)
        .unwrap_or(best_loss.0);
    if destination != origin {
        game.move_card(card, destination);
    }
}

fn psylocke(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let active = quickdraw(game, card);
    // ReduceHandCardCosts predicate (0x7f1c3d8) compares the original cost.
    let targets: Vec<_> = game.players[owner]
        .hand
        .iter()
        .copied()
        .filter(|&other| other != card && (active || game.printed_cost(other) >= 5))
        .collect();
    for target in targets {
        game.add_cost(target, -1, owner);
    }
}

fn fin_fang_foom(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    // GainFrontRowPower (0x7ea8d98) rolls among revealed enemy characters.
    let targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&other| {
            game.cards[other].owner != owner
                && game.cards[other].revealed
                && game.is_front_row(other)
                && game.is_character(other)
        })
        .collect();
    if !targets.is_empty() {
        let index = game.rng.borrow_mut().randbelow(targets.len());
        let power = game.cards[targets[index]].power();
        game.add_power(card, power, owner);
    }
}

fn hydra_bob(game: &mut Game<'_>, card: usize) {
    if !game.snapped_this_turn() {
        return;
    }
    let Some(source) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let mut targets: Vec<_> = (0..3)
        .filter(|&lane| {
            lane != source
                && game.has_space(lane, owner, 0)
                && (!game.lanes[lane].revealed
                    || game.lanes[lane]
                        .def
                        .allows_addition
                        .is_none_or(|test| test(game, lane, card, lane, owner)))
        })
        .collect();
    // GetRandomLocationToMoveCard (0x7db2f50) falls back to every other
    // location when none is movable; the subsequent move may still fail.
    if targets.is_empty() {
        targets = (0..3).filter(|&lane| lane != source).collect();
    }
    let index = game.rng.borrow_mut().randbelow(targets.len());
    game.move_card(card, targets[index]);
}

fn iron_lad(game: &mut Game<'_>, card: usize) {
    if let Some(top) = game.top_card(game.cards[card].owner) {
        game.copy_text(card, top, true);
    }
}

impl Game<'_> {
    pub(crate) fn prepare_nico(&mut self, card: usize, index: usize) {
        let owner = self.cards[card].owner;
        let in_hand = self.players[owner].hand.contains(&card);
        let in_deck = self.players[owner].deck.contains(&card);
        let crate::game::AttachedAbilityKind::NicoPrepare { mut used } =
            self.cards[card].attached_abilities[index].kind
        else {
            return;
        };
        // PrepareNewSpell (0x7f0c9e8) retains the trigger and its context
        // after loading the selected spell as the card's current text.
        if !in_hand && (!in_deck || used != 0) {
            return;
        }
        if used == 0x7f {
            used = 0;
        }
        let count = 7 - used.count_ones() as usize;
        let pick = self.rng.borrow_mut().randbelow(count);
        let spell = (0..7)
            .filter(|&i| used & (1 << i) == 0)
            .nth(pick)
            .expect("selected an available spell") as u8
            + 1;
        used |= 1 << (spell - 1);
        self.cards[card].attached_abilities[index].kind =
            crate::game::AttachedAbilityKind::NicoPrepare { used };
        if self.cards[card].attached_abilities[index].origin.is_none() {
            self.cards[card].attached_abilities[index].origin = Some(card);
        }
        let ids = [
            "Spell01NicoMinoru",
            "Spell02NicoMinoru",
            "Spell03NicoMinoru",
            "Spell04NicoMinoru",
            "Spell05NicoMinoru",
            "Spell06NicoMinoru",
            "Spell07NicoMinoru",
        ];
        let def = super::get(ids[usize::from(spell - 1)]).expect("registered Nico spell");
        self.cards[card].override_ability = Some(def);
        self.cards[card].state = def.initial_state.map(|init| init());
    }
}

fn install_spell(game: &mut Game<'_>, card: usize, spell: u8) {
    let owner = game.cards[card].owner;
    // Each AddNextCardTriggerToPlayer installs an independent PostCardResolved trigger.
    game.add_player_effect(
        owner,
        PendingEffect {
            source: card,
            lane: game.cards[card].lane,
            created_turn: game.turn,
            trigger: PlayerTrigger::CardPlayed,
            lifetime: EffectLifetime::UntilTriggered,
            state: Box::new((spell, owner)),
            apply: apply_spell,
        },
    );
}
fn spell01(g: &mut Game<'_>, c: usize) {
    install_spell(g, c, 1);
}
fn spell02(g: &mut Game<'_>, c: usize) {
    install_spell(g, c, 2);
}
fn spell03(g: &mut Game<'_>, c: usize) {
    install_spell(g, c, 3);
}
fn spell04(g: &mut Game<'_>, c: usize) {
    install_spell(g, c, 4);
}
fn spell05(g: &mut Game<'_>, c: usize) {
    install_spell(g, c, 5);
}
fn spell06(g: &mut Game<'_>, c: usize) {
    install_spell(g, c, 6);
}
fn spell07(g: &mut Game<'_>, c: usize) {
    install_spell(g, c, 7);
}

fn apply_spell(game: &mut Game<'_>, effect: &mut PendingEffect, target: Option<usize>) -> bool {
    let Some(target) = target else { return false };
    // Spell01's PostCardResolved predicate excludes its source on every turn.
    if target == effect.source {
        return false;
    }
    let &(spell, owner) = effect.state.as_any().downcast_ref::<(u8, usize)>().unwrap();
    match spell {
        1 => {
            game.transform_card_without_modifiers(target, "Demon");
        }
        2 => {
            // Spell02 (0x7f47624): draw only after successful destruction.
            if game.cards[target].lane.is_some() && game.destroy_card(target) {
                game.draw(owner);
                game.draw(owner);
            }
        }
        3 => {
            if let Some(lane) = game.cards[target].lane
                && lane < 2
            {
                game.move_card(target, lane + 1);
            }
        }
        4 => {
            game.add_power(target, 2, owner);
        }
        5 => {
            // Client's weighted GetRandomLocationDefId (0x7db248c) is approximated
            // by the simulator's implemented location pool, without weights.
            if let Some(lane) = game.cards[target].lane.or(game.cards[target].last_lane) {
                let choices: Vec<_> = game
                    .location_pool
                    .iter()
                    .filter(|location| location.id != game.lanes[lane].def.id)
                    .map(|location| location.id)
                    .collect();
                if !choices.is_empty() {
                    let index = game.rng.borrow_mut().randbelow(choices.len());
                    game.replace_location(lane, choices[index]);
                }
            }
        }
        6 => {
            if game.copy_card_to_hand(target, owner).is_some() {
                game.notify_hand_added(owner);
            }
        }
        7 => {
            if game.cards[effect.source].lane.is_some() {
                game.multiply_power(effect.source, 2, owner);
            }
        }
        _ => unreachable!(),
    }
    true
}

fn zombie_state() -> Box<dyn crate::game::CardState> {
    Box::new(None::<(usize, Vec<usize>)>)
}

fn zombie_event(game: &mut Game<'_>, card: usize, event: Event) {
    if !matches!(event, Event::GameStart) {
        return;
    }
    let owner = game.cards[card].owner;
    let mut candidates: Vec<_> = game.players[owner]
        .deck
        .iter()
        .copied()
        .filter(|&other| other != card && game.is_character(other))
        .collect();
    let mut selected = Vec::new();
    // ChooseDeckCards (0x7cfb1b4) chooses randomly without player input.
    for _ in 0..6 {
        if candidates.is_empty() {
            break;
        }
        let index = game.rng.borrow_mut().randbelow(candidates.len());
        selected.push(candidates.remove(index));
    }
    game.cards[card].state = Some(Box::new(Some((card, selected))));
}

fn zombie_reveal(game: &mut Game<'_>, card: usize) {
    let Some((chooser, chosen)) = game.cards[card]
        .state
        .as_ref()
        .and_then(|s| s.as_any().downcast_ref::<Option<(usize, Vec<usize>)>>())
        .and_then(Option::as_ref)
    else {
        return;
    };
    // SetChosenCardPower (0x7cfb7f8) requires GalactiCardKey to identify
    // the revealing entity; morphed copies retain the original chooser.
    if *chooser != card {
        return;
    }
    let chosen = chosen.clone();
    for target in chosen {
        game.transform_into_copy(target, card);
    }
}

fn jane(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let rounds = 1 + usize::from(quickdraw(game, card));
    // PickAndModifyCharacters (0x7ed1900) repeats the entire random selection.
    for _ in 0..rounds {
        let mut targets: Vec<_> = game
            .lanes
            .iter()
            .flat_map(|lane| lane.cards.iter().copied())
            .filter(|&id| id != card && game.cards[id].revealed && game.is_character(id))
            .collect();
        // Game::Shuffle (0x4b12f30) uses forward Fisher–Yates.
        for i in 0..targets.len().saturating_sub(1) {
            let j = i + game.rng.borrow_mut().randbelow(targets.len() - i);
            targets.swap(i, j);
        }
        for target in targets.into_iter().take(4) {
            let amount = if game.cards[target].owner == owner {
                1
            } else {
                -1
            };
            game.add_power(target, amount, owner);
        }
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
    fn captain_marvel_simulates_moves_and_stays_when_already_winning() {
        let mut p = position(
            json!([{"id":"CaptainMarvel", "power":6}]),
            json!([{"id":"Hulk", "power":7}]),
        );
        p.locations[1] = serde_json::from_value(json!({
            "id":"Ruins", "mine":[{"id":"Wasp", "power":1}],
            "theirs":[{"id":"Shocker", "power":6}]
        }))
        .unwrap();
        p.locations[2] = serde_json::from_value(json!({
            "id":"Ruins", "mine":[{"id":"Hulk", "power":14}],
            "theirs":[{"id":"MistyKnight", "power":3}]
        }))
        .unwrap();
        let mut rng = PythonRandom::seed(19);
        let mut game = build_game(&p, &mut rng).unwrap();
        let captain = game.lanes[0].cards[0];
        assert_eq!(game.winner(), 1);
        captain_marvel(&mut game, captain, Event::GameEnd);
        assert_eq!(game.cards[captain].lane, Some(1));
        assert_eq!(game.winner(), 0);
        captain_marvel(&mut game, captain, Event::GameEnd);
        assert_eq!(game.cards[captain].lane, Some(1));
    }

    #[test]
    fn spells_install_independent_post_play_effects() {
        let hooks: [(u8, super::super::CardHook); 7] = [
            (1, spell01),
            (2, spell02),
            (3, spell03),
            (4, spell04),
            (5, spell05),
            (6, spell06),
            (7, spell07),
        ];
        for (spell, hook) in hooks {
            let mut p = position(json!([{"id":"AntMan"}, {"id":"Hulk"}]), json!([]));
            p.deck = vec!["IronMan".into(), "Medusa".into()];
            let mut rng = PythonRandom::seed(spell.into());
            let mut game = build_game(&p, &mut rng).unwrap();
            let source = game.lanes[0].cards[0];
            let target = game.lanes[0].cards[1];
            let source_power = game.cards[source].power();
            let target_power = game.cards[target].power();
            let location = game.lanes[0].def.id;
            hook(&mut game, source);
            let mut effect = game.players[0].pending_effects.remove(0);
            assert_eq!(effect.trigger, PlayerTrigger::CardPlayed);
            assert!(!(effect.apply)(&mut game, &mut effect, Some(source)));
            assert!((effect.apply)(&mut game, &mut effect, Some(target)));
            match spell {
                1 => assert_eq!(game.cards[target].def.id, "Demon"),
                2 => {
                    assert!(game.cards[target].lane.is_none());
                    assert_eq!(game.players[0].hand.len(), 2);
                }
                3 => assert_eq!(game.cards[target].lane, Some(1)),
                4 => assert_eq!(game.cards[target].power(), target_power + 2),
                5 => assert_ne!(game.lanes[0].def.id, location),
                6 => assert_eq!(game.players[0].hand.len(), 1),
                7 => assert_eq!(game.cards[source].power(), source_power * 2),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn nico_cycles_without_repeating_until_all_spells_are_used() {
        let mut p = position(json!([]), json!([]));
        p.hand = serde_json::from_value(json!([{"id":"NicoMinoru"}])).unwrap();
        let mut rng = PythonRandom::seed(8);
        let mut game = build_game(&p, &mut rng).unwrap();
        let nico = game.players[0].hand[0];
        let mut seen = 0u8;
        for _ in 0..7 {
            game.prepare_nico(nico, 0);
            let spell = game.cards[nico].ability_def().id;
            let bit = 1 << (spell.as_bytes()[6] - b'1');
            assert_eq!(seen & bit, 0);
            seen |= bit;
            let crate::game::AttachedAbilityKind::NicoPrepare { used } =
                game.cards[nico].attached_abilities[0].kind
            else {
                panic!("missing prepare trigger")
            };
            assert_eq!(used, seen);
        }
        game.prepare_nico(nico, 0);
        let crate::game::AttachedAbilityKind::NicoPrepare { used } =
            game.cards[nico].attached_abilities[0].kind
        else {
            panic!("missing prepare trigger")
        };
        assert_eq!(used.count_ones(), 1);
    }

    #[test]
    fn psylocke_uses_printed_cost_unless_quickdrawn() {
        let mut p = position(json!([{"id":"PsylockeFracturedFrontier"}]), json!([]));
        p.hand = serde_json::from_value(json!([{"id":"Hulk"}, {"id":"AntMan"}])).unwrap();
        let mut rng = PythonRandom::seed(1);
        let mut game = build_game(&p, &mut rng).unwrap();
        let psy = game.lanes[0].cards[0];
        let hulk = game.players[0].hand[0];
        let ant = game.players[0].hand[1];
        psylocke(&mut game, psy);
        assert_eq!(game.cards[hulk].cost, 5);
        assert_eq!(game.cards[ant].cost, 1);
        game.cards[psy].drawn_turn = Some(game.turn);
        psylocke(&mut game, psy);
        assert_eq!(game.cards[hulk].cost, 4);
        assert_eq!(game.cards[ant].cost, 0);
    }

    #[test]
    fn fin_fang_foom_only_uses_revealed_enemy_front_row() {
        let p = position(
            json!([{"id":"FinFangFoom"}]),
            json!([{"id":"Hulk"}, {"id":"AntMan"}, {"id":"IronMan"}]),
        );
        let mut rng = PythonRandom::seed(2);
        let mut game = build_game(&p, &mut rng).unwrap();
        let fin = game.lanes[0].cards[0];
        let enemies: Vec<_> = game.lanes[0]
            .cards
            .iter()
            .copied()
            .filter(|&id| game.cards[id].owner == 1)
            .collect();
        game.cards[enemies[1]].revealed = false;
        let expected = game.cards[fin].power() + game.cards[enemies[0]].power();
        fin_fang_foom(&mut game, fin);
        assert_eq!(game.cards[fin].power(), expected);
        game.cards[enemies[0]].revealed = false;
        fin_fang_foom(&mut game, fin);
        assert_eq!(game.cards[fin].power(), expected);
    }

    #[test]
    fn iron_lad_copies_and_activates_top_card_without_consuming_it() {
        let mut p = position(json!([{"id":"IronLad"}]), json!([]));
        p.deck = vec!["Medusa".into()];
        let mut rng = PythonRandom::seed(3);
        let mut game = build_game(&p, &mut rng).unwrap();
        let lad = game.lanes[0].cards[0];
        let top = game.top_card(0);
        iron_lad(&mut game, lad);
        assert_eq!(game.top_card(0), top);
        assert_eq!(game.cards[lad].power(), 7);
    }

    #[test]
    fn zombie_random_choice_excludes_itself_and_transforms_selected() {
        let mut p = position(json!([{"id":"ZombieGalacti"}]), json!([]));
        p.deck = vec!["Hulk".into(), "AntMan".into()];
        let mut rng = PythonRandom::seed(4);
        let mut game = build_game(&p, &mut rng).unwrap();
        let zombie = game.lanes[0].cards[0];
        zombie_event(&mut game, zombie, Event::GameStart);
        let chosen = game.cards[zombie]
            .state
            .as_ref()
            .unwrap()
            .as_any()
            .downcast_ref::<Option<(usize, Vec<usize>)>>()
            .unwrap()
            .as_ref()
            .unwrap()
            .1
            .clone();
        assert_eq!(chosen.len(), 2);
        zombie_reveal(&mut game, zombie);
        assert!(
            chosen
                .iter()
                .all(|&id| game.cards[id].def.id == "ZombieGalacti")
        );
    }

    #[test]
    fn zombie_copies_on_board_do_not_repeat_original_choice() {
        let p = position(
            json!([
                {"id":"ZombieGalacti"},
                {"id":"Hulk"},
                {"id":"AntMan"},
                {"id":"IronMan"}
            ]),
            json!([]),
        );
        let mut rng = PythonRandom::seed(42);
        let mut game = build_game(&p, &mut rng).unwrap();
        let zombie = game.lanes[0].cards[0];
        let chosen = game.lanes[0].cards[1..].to_vec();
        game.cards[zombie].state = Some(Box::new(Some((zombie, chosen.clone()))));
        zombie_reveal(&mut game, zombie);
        assert!(
            chosen
                .iter()
                .all(|&id| { game.cards[id].revealed && game.cards[id].def.id == "ZombieGalacti" })
        );
    }

    #[test]
    fn hydra_moves_only_on_a_snap_this_turn() {
        let p = position(json!([{"id":"HydraBob"}]), json!([]));
        let mut rng = PythonRandom::seed(5);
        let mut game = build_game(&p, &mut rng).unwrap();
        let bob = game.lanes[0].cards[0];
        hydra_bob(&mut game, bob);
        assert_eq!(game.cards[bob].lane, Some(0));
        game.players[1].snapped_turns.push(game.turn);
        hydra_bob(&mut game, bob);
        assert_ne!(game.cards[bob].lane, Some(0));
    }

    #[test]
    fn jane_quickdraw_repeats_on_both_sides() {
        let p = position(
            json!([{"id":"JaneFosterFracturedFrontier"}, {"id":"Hulk"}]),
            json!([{"id":"AntMan"}]),
        );
        let mut rng = PythonRandom::seed(6);
        let mut game = build_game(&p, &mut rng).unwrap();
        let jane_id = game.lanes[0].cards[0];
        let ally = game.lanes[0].cards[1];
        let enemy = game.lanes[0].cards[2];
        let ally_power = game.cards[ally].power();
        let enemy_power = game.cards[enemy].power();
        game.cards[jane_id].drawn_turn = Some(game.turn);
        jane(&mut game, jane_id);
        assert_eq!(game.cards[ally].power(), ally_power + 2);
        assert_eq!(game.cards[enemy].power(), enemy_power - 2);
        game.cards[jane_id].drawn_turn = Some(game.turn - 1);
        jane(&mut game, jane_id);
        assert_eq!(game.cards[ally].power(), ally_power + 3);
        assert_eq!(game.cards[enemy].power(), enemy_power - 3);
        game.cards[jane_id].draw_extends_quickdraw = true;
        jane(&mut game, jane_id);
        assert_eq!(game.cards[ally].power(), ally_power + 5);
        assert_eq!(game.cards[enemy].power(), enemy_power - 5);
    }

    #[test]
    fn iron_lad_copies_prepared_nico_spell_not_prepare_context() {
        let p = position(json!([{"id":"IronLad"}]), json!([]));
        let mut rng = PythonRandom::seed(23);
        let mut game = build_game(&p, &mut rng).unwrap();
        let lad = game.lanes[0].cards[0];
        let nico = game.create_card("NicoMinoru", 0);
        game.players[0].hand.push(nico);
        game.prepare_nico(nico, 0);
        let spell = game.cards[nico].ability_def().id;
        game.players[0].hand.retain(|&id| id != nico);
        game.players[0].deck.insert(0, nico);
        iron_lad(&mut game, lad);
        assert_eq!(game.cards[lad].ability_def().id, spell);
        assert!(game.cards[lad].attached_abilities.is_empty());
        assert_eq!(game.players[0].pending_effects.len(), 1);
    }

    #[test]
    fn iron_lad_gets_fresh_surge_context() {
        let p = position(json!([{"id":"IronLad"}]), json!([]));
        let mut rng = PythonRandom::seed(24);
        let mut game = build_game(&p, &mut rng).unwrap();
        let lad = game.lanes[0].cards[0];
        let surge = game.create_card("Surge", 0);
        game.cards[surge].state = Some(Box::new(Some(usize::MAX)));
        game.players[0].deck.insert(0, surge);
        iron_lad(&mut game, lad);
        let state = game.cards[lad]
            .state
            .as_ref()
            .unwrap()
            .as_any()
            .downcast_ref::<Option<usize>>()
            .unwrap();
        assert_eq!(*state, Some(surge));
    }

    #[test]
    fn copied_resolve_repeats_at_kamar_taj() {
        let mut p = position(json!([{"id":"IronLad"}]), json!([]));
        p.locations[0].id = Some("KamarTaj".into());
        let mut rng = PythonRandom::seed(25);
        let mut game = build_game(&p, &mut rng).unwrap();
        let lad = game.lanes[0].cards[0];
        // Use a 1-cost Hawkeye to observe the number of copied resolve activations.
        let hawkeye = game.create_card("Hawkeye", 0);
        game.players[0].deck.insert(0, hawkeye);
        iron_lad(&mut game, lad);
        assert_eq!(game.cards[lad].watch_contexts().len(), 2);
    }

    #[test]
    fn iron_lad_copies_completed_objective_with_fresh_progress() {
        let p = position(
            json!([{"id":"IronLad"}, {"id":"PolarisHorseman"}]),
            json!([{"id":"Hulk"}]),
        );
        let mut rng = PythonRandom::seed(26);
        let mut game = build_game(&p, &mut rng).unwrap();
        let lad = game.lanes[0].cards[0];
        let polaris = game.lanes[0].cards[1];
        let enemy = game.lanes[0].cards[2];
        game.add_power(enemy, -5, 0);
        game.ongoing();
        game.players[0].hand.clear();
        let first = game.create_card("Wasp", 0);
        game.players[0].hand.push(first);
        game.end_turn_effects();
        assert_eq!(game.cards[first].lane, Some(0));
        assert!(game.take_from_board(polaris));
        game.players[0].deck.insert(0, polaris);
        let second = game.create_card("Wasp", 0);
        game.players[0].hand.push(second);
        iron_lad(&mut game, lad);
        game.end_turn_effects();
        assert_eq!(game.cards[second].lane, Some(0));
    }
}
