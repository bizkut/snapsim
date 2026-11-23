//! Meta-deck cards, batch E1: Scarlet Witch Queen of Chaos and her curated skills; rules from
//! the 59.15.1 client.
use super::CardDef;
use crate::game::Game;

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        reveal: Some(flames),
        ..CardDef::vanilla("FlamesOfTheFaltine")
    },
    CardDef {
        reveal: Some(winds),
        ..CardDef::vanilla("Spell01Agamotto")
    },
    CardDef {
        reveal: Some(remote_mines),
        ..CardDef::vanilla("RemoteMines")
    },
    CardDef {
        reveal: Some(mole_skill),
        ..CardDef::vanilla("Skill01MoleManChampion")
    },
    CardDef {
        reveal: Some(copy_highest),
        ..CardDef::vanilla("Spell03Agamotto")
    },
    CardDef {
        reveal: Some(spell_energy),
        ..CardDef::vanilla("Spell04Agamotto")
    },
    CardDef {
        reveal: Some(vibro),
        ..CardDef::vanilla("VibroShockGauntlets")
    },
    CardDef {
        reveal: Some(polymorph),
        ..CardDef::vanilla("Polymorph")
    },
    CardDef {
        reveal: Some(tao),
        ..CardDef::vanilla("TaoMandala")
    },
    CardDef {
        reveal: Some(omniversal),
        ..CardDef::vanilla("OmniversalPresence")
    },
    CardDef {
        reveal: Some(flight),
        ..CardDef::vanilla("FlightHarness")
    },
    CardDef {
        reveal: Some(once_and_future),
        ..CardDef::vanilla("OnceAndFuture")
    },
    CardDef {
        reveal: Some(captain_skill),
        ..CardDef::vanilla("Skill01CaptainAmericaChampion")
    },
    CardDef {
        start_turn: Some(queen),
        initial_state: Some(queen_state),
        ..CardDef::vanilla("ScarletWitchQueenOfChaos")
    },
];

pub(crate) static TOKENS: &[&str] = &[
    "Spell01Agamotto",
    "Spell03Agamotto",
    "Spell04Agamotto",
    "RemoteMines",
    "VibroShockGauntlets",
    "FlightHarness",
    "OmniversalPresence",
    "Polymorph",
    "OnceAndFuture",
    "FlamesOfTheFaltine",
    "TaoMandala",
    "Skill01CaptainAmericaChampion",
    "Skill01MoleManChampion",
];

fn revealed_cards(game: &Game<'_>, card: usize, owner: usize) -> Vec<usize> {
    game.cards[card].lane.map_or_else(Vec::new, |lane| {
        game.lanes[lane]
            .cards
            .iter()
            .copied()
            .filter(|&id| game.cards[id].owner == owner && game.cards[id].revealed)
            .collect()
    })
}

fn pick_random(game: &Game<'_>, candidates: &[usize]) -> Option<usize> {
    if candidates.is_empty() {
        None
    } else {
        Some(candidates[game.rng.borrow_mut().randbelow(candidates.len())])
    }
}

fn extreme(game: &Game<'_>, card: usize, highest: bool) -> Option<usize> {
    let owner = game.cards[card].owner;
    let candidates: Vec<_> = revealed_cards(game, card, owner)
        .into_iter()
        .filter(|&id| id != card && game.is_character(id))
        .collect();
    let value = if highest {
        candidates.iter().map(|&id| game.cards[id].power()).max()
    } else {
        candidates.iter().map(|&id| game.cards[id].power()).min()
    }?;
    let tied: Vec<_> = candidates
        .into_iter()
        .filter(|&id| game.cards[id].power() == value)
        .collect();
    // Location::GetHighest/LowestPowerCardForPlayer (0x7ddefdc/0x7ddf014)
    // rolls among tied cards even for a singleton.
    pick_random(game, &tied)
}

fn flames(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let enemies = revealed_cards(game, card, 1 - owner);
    // AfflictCards (0x7d02d58) rolls over all revealed enemies.
    if let Some(target) = pick_random(game, &enemies) {
        game.add_power(target, -3, owner);
    }
}

fn winds(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let enemies = revealed_cards(game, card, 1 - owner);
    if let Some(target) = pick_random(game, &enemies) {
        game.add_power(target, -5, owner);
        // AfflictAndMoveRight (0x7d121ec) uses the target's current lane.
        if let Some(lane) = game.cards[target].lane
            && lane < 2
        {
            game.move_card(target, lane + 1);
        }
    }
}

fn remote_mines(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let mut candidates: Vec<_> = revealed_cards(game, card, 1 - owner)
        .into_iter()
        .filter(|&id| game.is_character(id))
        .collect();
    let mut targets = Vec::new();
    // AfflictHighestPower (0x7d06814) chooses each highest-power tie
    // independently, removes it, and only then applies both afflictions.
    for _ in 0..2 {
        let Some(power) = candidates.iter().map(|&id| game.cards[id].power()).max() else {
            break;
        };
        let tied: Vec<_> = candidates
            .iter()
            .copied()
            .filter(|&id| game.cards[id].power() == power)
            .collect();
        let target = pick_random(game, &tied).unwrap();
        targets.push(target);
        candidates.retain(|&id| id != target);
    }
    for target in targets {
        game.add_power(target, -2, owner);
    }
}

fn mole_skill(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game.players[owner]
        .hand
        .iter()
        .copied()
        .filter(|&id| game.cards[id].created)
        .collect();
    // BuffForHavingCreatedCards (0x7d0d45c) reduces skills, buffs
    // zero-cost characters, and rolls only for positive-cost characters.
    for target in targets {
        if !game.is_character(target) {
            game.add_cost(target, -1, owner);
        } else if game.card_cost(target) <= 0 {
            game.add_power(target, 3, owner);
        } else {
            let roll = game.rng.borrow_mut().randbelow(2);
            if roll == 0 {
                game.add_cost(target, -1, owner);
            } else {
                game.add_power(target, 3, owner);
            }
        }
    }
}

fn copy_highest(game: &mut Game<'_>, card: usize) {
    let Some(source) = extreme(game, card, true) else {
        return;
    };
    let owner = game.cards[card].owner;
    let targets: Vec<_> = revealed_cards(game, card, owner)
        .into_iter()
        .filter(|&id| id != card && id != source)
        .collect();
    // TransformCardsToHighestPower (0x7d12e40) selects the source once.
    for target in targets {
        game.transform_into_copy(target, source);
    }
}

fn spell_energy(game: &mut Game<'_>, card: usize) {
    // AddEnergyTriggerToPlayer (0x7d13994) survives the skill leaving play.
    game.schedule_energy(game.cards[card].owner, game.turn + 1, 4);
}

fn vibro(game: &mut Game<'_>, card: usize) {
    let Some(target) = extreme(game, card, true) else {
        return;
    };
    let owner = game.cards[card].owner;
    // BuffAndAddEnergyTrigger (0x7d198a4) schedules energy only if a target exists.
    game.schedule_energy(owner, game.turn + 1, 1);
    game.add_power(target, 2, owner);
}

fn polymorph(game: &mut Game<'_>, card: usize) {
    let Some(target) = extreme(game, card, false) else {
        return;
    };
    let cost = game.printed_cost(target) + 1;
    // TransformCard (0x7d05c8c) uses the full collectible pool in the client;
    // random_card approximates it with implemented cards. An empty pool does nothing.
    if let Some(random) = game.random_card(|row| row.card_type == "Character" && row.cost == cost) {
        // A replay can provide a uniquely observed morph, without changing the
        // random draw or the outcome when a different ally was selected.
        let id = game.cards[card]
            .ability_state()
            .and_then(|state| state.as_any().downcast_ref::<(usize, &'static str)>())
            .filter(|(observed_target, _)| *observed_target == target)
            .map_or(random, |(_, id)| *id);
        game.transform_card_without_modifiers(target, id);
    }
}

fn tao(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let allies = revealed_cards(game, card, owner);
    let candidates: Vec<_> = allies
        .iter()
        .copied()
        .filter(|&id| id != card && game.cards[id].has_ongoing(false))
        .collect();
    let Some(source) = pick_random(game, &candidates) else {
        return;
    };
    // TransformText (0x7d16550) copies text without activating On Reveal.
    for target in allies {
        if target != source && game.is_character(target) {
            game.copy_text(target, source, false);
        }
    }
}

fn omniversal(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    // GetRandomLocationDefId (0x7db248c) is weighted in the client; use the
    // simulator's implemented location pool and exclude the current definition.
    let choices: Vec<_> = game
        .location_pool
        .iter()
        .filter(|def| def.id != game.lanes[lane].def.id)
        .map(|def| def.id)
        .collect();
    if !choices.is_empty() {
        let index = game.rng.borrow_mut().randbelow(choices.len());
        game.replace_location(lane, choices[index]);
    }
    // MorphLocationAndBuffMerlin (0x7d04ac0) searches all revealed locations.
    let targets: Vec<_> = game
        .lanes
        .iter()
        .flat_map(|l| l.cards.iter().copied())
        .filter(|&id| {
            game.cards[id].owner == owner
                && game.cards[id].revealed
                && game.cards[id].def.id == "Merlin"
        })
        .collect();
    for target in targets {
        game.add_power(target, 1, owner);
    }
}

fn flight(game: &mut Game<'_>, card: usize) {
    let Some(target) = extreme(game, card, true) else {
        return;
    };
    let owner = game.cards[card].owner;
    game.add_power(target, 1, owner);
    // BuffAndMove (0x7d031a0) checks the skill's ability to move, not the ally's.
    if let Some(lane) = game.cards[card].lane
        && lane < 2
        && game.can_move_card(card, lane + 1)
    {
        game.move_card(target, lane + 1);
    }
}

fn once_and_future(game: &mut Game<'_>, card: usize) {
    let Some(target) = extreme(game, card, false) else {
        return;
    };
    let owner = game.cards[card].owner;
    // ShuffleCardForEnergy (0x7d0516c) schedules energy only after a successful move.
    if game.move_card_to_deck(target) {
        game.shuffle_deck(owner);
        game.schedule_energy(owner, game.turn + 1, 2);
    }
}

fn captain_skill(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let candidates: Vec<_> = if game.has_space(lane, owner, 0) {
        game.players[owner]
            .deck
            .iter()
            .copied()
            .filter(|&id| game.cards[id].has_ongoing(false))
            .collect()
    } else {
        Vec::new()
    };
    // SwapCard (0x7d087b4) creates a fresh skill if no target, but creates
    // a copy and shuffles when there is a target, even if the move is blocked.
    if let Some(target) = pick_random(game, &candidates) {
        if game.move_deck_card_to_location(target, lane) {
            game.add_power(target, 3, owner);
        }
        let copy = game.copy_card(card, owner);
        game.push_deck(owner, copy);
        game.shuffle_deck(owner);
    } else {
        let copy = game.create_card("Skill01CaptainAmericaChampion", owner);
        game.push_deck(owner, copy);
    }
}

// AddSkillToHand (0x7f309d4), CardDefList::InGameRandomlyObtainableDefs
// (0x782132c), and the 59.15.1 carddefs series start dates at 2026-11-20.
// Update this explicit collectible pool when further skill releases become eligible.
const GENERIC_SKILLS: &[&str] = &[
    "AstralProjection",
    "DeafeningChord",
    "FastballSpecial",
    "KnightsVow",
    "MajesticWingbeat",
    "Sandstorm",
    "SpiderSense",
    "TechnoOrganicVirus",
    "TheHunger",
    "TheInversion",
    "WebSling",
];
const CURATED_SKILLS: &[&str] = &[
    "Spell01Agamotto",
    "Spell03Agamotto",
    "Spell04Agamotto",
    "RemoteMines",
    "VibroShockGauntlets",
    "FlightHarness",
    "OmniversalPresence",
    "Polymorph",
    "OnceAndFuture",
    "FlamesOfTheFaltine",
    "TaoMandala",
    "Skill01CaptainAmericaChampion",
    "Skill01MoleManChampion",
];

fn queen_state() -> Box<dyn crate::game::CardState> {
    Box::new(None::<usize>)
}

fn queen(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if game.players[owner].hand.len() >= 7 {
        return;
    }
    let previous = game.cards[card]
        .ability_state()
        .and_then(|s| s.as_any().downcast_ref::<Option<usize>>())
        .copied()
        .flatten()
        .map(|id| game.cards[id].def.id);
    // Roll(0,10) first; previous generated definition is excluded from either pool.
    let generic = game.rng.borrow_mut().randbelow(10) < 4;
    let pool = if generic {
        GENERIC_SKILLS
    } else {
        CURATED_SKILLS
    };
    let choices: Vec<_> = pool
        .iter()
        .copied()
        .filter(|&id| Some(id) != previous)
        .collect();
    if choices.is_empty() {
        return;
    }
    let index = game.rng.borrow_mut().randbelow(choices.len());
    let skill = game.create_card(choices[index], owner);
    if game.add_hand(owner, skill, false) {
        game.set_temporary_hand_cost(skill, game.turn, 0);
        game.cards[card].set_ability_state(Some(Box::new(Some(skill))));
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
    fn affliction_skills_select_revealed_enemies() {
        let p = position(
            json!([{"id":"FlamesOfTheFaltine"}, {"id":"Spell01Agamotto"}]),
            json!([{"id":"Hulk"}]),
        );
        let mut rng = PythonRandom::seed(1);
        let mut game = build_game(&p, &mut rng).unwrap();
        let flame = game.lanes[0].cards[0];
        let wind = game.lanes[0].cards[1];
        let enemy = game.lanes[0].cards[2];
        let power = game.cards[enemy].power();
        flames(&mut game, flame);
        assert_eq!(game.cards[enemy].power(), power - 3);
        winds(&mut game, wind);
        assert_eq!(game.cards[enemy].power(), power - 8);
        assert_eq!(game.cards[enemy].lane, Some(1));
    }

    #[test]
    fn mines_afflict_two_highest_without_touching_lowest() {
        let p = position(
            json!([{"id":"RemoteMines"}]),
            json!([{"id":"Hulk"}, {"id":"IronMan"}, {"id":"AntMan"}]),
        );
        let mut rng = PythonRandom::seed(2);
        let mut game = build_game(&p, &mut rng).unwrap();
        let ids = game.lanes[0].cards.clone();
        let before: Vec<_> = ids.iter().map(|&id| game.cards[id].power()).collect();
        remote_mines(&mut game, ids[0]);
        assert_eq!(game.cards[ids[1]].power(), before[1] - 2);
        assert_eq!(game.cards[ids[2]].power(), before[2]);
        assert_eq!(game.cards[ids[3]].power(), before[3] - 2);
    }

    #[test]
    fn spells_copy_highest_and_polymorph_one_cost_higher() {
        let p = position(
            json!([{"id":"Spell03Agamotto"}, {"id":"AntMan"}, {"id":"Hulk"}]),
            json!([]),
        );
        let mut rng = PythonRandom::seed(3);
        let mut game = build_game(&p, &mut rng).unwrap();
        let ids = game.lanes[0].cards.clone();
        copy_highest(&mut game, ids[0]);
        assert_eq!(game.cards[ids[1]].def.id, "Hulk");
        let p = position(json!([{"id":"Polymorph"}, {"id":"AntMan"}]), json!([]));
        let mut rng = PythonRandom::seed(4);
        let mut game = build_game(&p, &mut rng).unwrap();
        let ids = game.lanes[0].cards.clone();
        polymorph(&mut game, ids[0]);
        assert_eq!(game.printed_cost(ids[1]), 2);
    }

    #[test]
    fn polymorph_observed_choice_preserves_random_stream() {
        let p = position(json!([{"id":"Polymorph"}, {"id":"AntMan"}]), json!([]));
        let mut observed_rng = PythonRandom::seed(4);
        let mut ordinary_rng = PythonRandom::seed(4);
        let mut observed = build_game(&p, &mut observed_rng).unwrap();
        let mut ordinary = build_game(&p, &mut ordinary_rng).unwrap();
        let ids = observed.lanes[0].cards.clone();
        let ordinary_ids = ordinary.lanes[0].cards.clone();
        observed.cards[ids[0]].state = Some(Box::new((ids[1], "Medusa")));
        polymorph(&mut observed, ids[0]);
        polymorph(&mut ordinary, ordinary_ids[0]);
        assert_eq!(observed.cards[ids[1]].def.id, "Medusa");
        assert_eq!(
            observed.rng.borrow_mut().randbelow(10000),
            ordinary.rng.borrow_mut().randbelow(10000)
        );
    }

    #[test]
    fn energy_and_highest_buff_require_targets_as_in_client() {
        let mut p = position(
            json!([{"id":"VibroShockGauntlets"}, {"id":"Hulk"}]),
            json!([]),
        );
        // Turn advancement must not depend on sampled hidden locations.
        for (lane, id) in p.locations.iter_mut().zip(["Ruins", "Wakanda", "KamarTaj"]) {
            lane.id = Some(id.into());
        }
        let mut rng = PythonRandom::seed(5);
        let mut game = build_game(&p, &mut rng).unwrap();
        let ids = game.lanes[0].cards.clone();
        let power = game.cards[ids[1]].power();
        vibro(&mut game, ids[0]);
        assert_eq!(game.cards[ids[1]].power(), power + 2);
        spell_energy(&mut game, ids[0]);
        game.turn += 1;
        game.start_turn();
        assert_eq!(game.players[0].energy, 9);
    }

    #[test]
    fn flight_and_tao_target_allies() {
        let p = position(json!([{"id":"FlightHarness"}, {"id":"Hulk"}]), json!([]));
        let mut rng = PythonRandom::seed(6);
        let mut game = build_game(&p, &mut rng).unwrap();
        let ids = game.lanes[0].cards.clone();
        let power = game.cards[ids[1]].power();
        flight(&mut game, ids[0]);
        assert_eq!(game.cards[ids[1]].power(), power + 1);
        assert_eq!(game.cards[ids[1]].lane, Some(1));
        let p = position(
            json!([{"id":"TaoMandala"}, {"id":"IronMan"}, {"id":"Hulk"}]),
            json!([]),
        );
        let mut rng = PythonRandom::seed(7);
        let mut game = build_game(&p, &mut rng).unwrap();
        let ids = game.lanes[0].cards.clone();
        tao(&mut game, ids[0]);
        assert!(game.cards[ids[2]].has_ongoing(false));
    }

    #[test]
    fn mole_buffs_created_zero_cost_character() {
        let mut p = position(json!([{"id":"Skill01MoleManChampion"}]), json!([]));
        p.hand = serde_json::from_value(json!([{"id":"AntMan", "cost":0}])).unwrap();
        let mut rng = PythonRandom::seed(8);
        let mut game = build_game(&p, &mut rng).unwrap();
        let target = game.players[0].hand[0];
        game.cards[target].created = true;
        let power = game.cards[target].power();
        let source = game.lanes[0].cards[0];
        mole_skill(&mut game, source);
        assert_eq!(game.cards[target].power(), power + 3);
    }

    #[test]
    fn omniversal_changes_location() {
        let p = position(json!([{"id":"OmniversalPresence"}]), json!([]));
        let mut rng = PythonRandom::seed(9);
        let mut game = build_game(&p, &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        let before = game.lanes[0].def.id;
        omniversal(&mut game, source);
        assert_ne!(game.lanes[0].def.id, before);
    }

    #[test]
    fn once_and_future_moves_lowest_and_adds_energy() {
        let mut p = position(
            json!([{"id":"OnceAndFuture"}, {"id":"AntMan"}, {"id":"Hulk"}]),
            json!([]),
        );
        // Keep random hidden-location effects out of this energy-specific test.
        for (lane, id) in p.locations.iter_mut().zip(["Ruins", "Wakanda", "KamarTaj"]) {
            lane.id = Some(id.into());
        }
        let mut rng = PythonRandom::seed(10);
        let mut game = build_game(&p, &mut rng).unwrap();
        let ids = game.lanes[0].cards.clone();
        assert_eq!(game.players[0].energy, 3);
        assert_eq!(game.cards[ids[1]].def.id, "AntMan");
        assert!(game.cards[ids[1]].power() < game.cards[ids[2]].power());
        once_and_future(&mut game, ids[0]);
        assert_eq!(game.players[0].energy, 3);
        assert!(game.players[0].deck.contains(&ids[1]));
        assert_eq!(game.cards[ids[2]].lane, Some(0));
        game.turn += 1;
        game.start_turn();
        assert_eq!(game.players[0].energy, 6);
    }

    #[test]
    fn captain_skill_moves_ongoing_and_shuffles_copy() {
        let mut p = position(json!([{"id":"Skill01CaptainAmericaChampion"}]), json!([]));
        p.deck = vec!["IronMan".into()];
        let mut rng = PythonRandom::seed(11);
        let mut game = build_game(&p, &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        let target = game.players[0].deck[0];
        let power = game.cards[target].power();
        captain_skill(&mut game, source);
        assert_eq!(game.cards[target].lane, Some(0));
        assert_eq!(game.cards[target].power(), power + 3);
        assert_eq!(game.players[0].deck.len(), 1);
        assert_eq!(
            game.cards[game.players[0].deck[0]].def.id,
            "Skill01CaptainAmericaChampion"
        );
    }

    #[test]
    fn queen_generates_zero_cost_skill_without_repeating_previous() {
        let mut p = position(json!([{"id":"ScarletWitchQueenOfChaos"}]), json!([]));
        // Do not couple turn-dependent behavior to the hidden-location pool.
        for lane in &mut p.locations {
            lane.id = Some("Ruins".into());
        }
        let mut rng = PythonRandom::seed(12);
        let mut game = build_game(&p, &mut rng).unwrap();
        // Start-of-turn triggers run before WaitForStaging (0x7e4a2dc).
        game.is_wait_for_staging = false;
        let source = game.lanes[0].cards[0];
        queen(&mut game, source);
        game.ongoing();
        let first = game.players[0].hand[0];
        assert_eq!(game.card_cost(first), 0);
        game.turn += 1;
        queen(&mut game, source);
        game.ongoing();
        let second = game.players[0].hand[1];
        assert_ne!(game.cards[first].def.id, game.cards[second].def.id);
        assert_eq!(game.card_cost(second), 0);
        assert!(
            GENERIC_SKILLS
                .iter()
                .chain(CURATED_SKILLS)
                .all(|id| super::super::get(id).is_some())
        );
        while game.players[0].hand.len() < 7 {
            let filler = game.create_card("Hulk", 0);
            game.add_hand(0, filler, false);
        }
        let count = game.cards.len();
        queen(&mut game, source);
        assert_eq!(game.cards.len(), count);
    }
}
