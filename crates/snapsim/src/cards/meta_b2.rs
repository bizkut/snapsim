//! Meta-deck cards, batch B2: Skills and skill synergies; rules from the 59.15.1 client.
use super::CardDef;
use crate::game::Game;

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        reveal: Some(deafening_chord),
        ..CardDef::vanilla("DeafeningChord")
    },
    CardDef {
        reveal: Some(majestic_wingbeat),
        ..CardDef::vanilla("MajesticWingbeat")
    },
    CardDef {
        reveal: Some(the_hunger),
        ..CardDef::vanilla("TheHunger")
    },
    CardDef {
        reveal: Some(fastball_special),
        ..CardDef::vanilla("FastballSpecial")
    },
    CardDef {
        reveal: Some(adamantium_infusion),
        ..CardDef::vanilla("AdamantiumInfusion")
    },
    CardDef {
        on_play: Some(nicholas_scratch),
        ..CardDef::vanilla("NicholasScratch")
    },
    CardDef {
        end_turn: Some(vampire_killmonger),
        ..CardDef::vanilla("VampireKillmonger")
    },
    CardDef {
        reveal: Some(vampiric_spear),
        ..CardDef::vanilla("VampiricSpear")
    },
    CardDef {
        on_play: Some(zombie_scarlet_witch),
        ..CardDef::vanilla("ZombieScarletWitch")
    },
];

pub(crate) static TOKENS: &[&str] = &["VampiricSpear"];

fn deafening_chord(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    // SilenceAndAfflictCard predicate 0x7d01c8c uses printed cost and characters.
    let targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| {
            game.cards[id].owner != owner
                && game.cards[id].revealed
                && game.is_character(id)
                && game.printed_cost(id) <= 3
        })
        .collect();
    if !targets.is_empty() {
        let target = targets[game.rng.borrow_mut().randbelow(targets.len())];
        game.cards[target].abilities_removed = true;
    }
}

fn majestic_wingbeat(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if let Some(&target) = game.players[owner].hand.first() {
        // BuffAndDiscount 0x7d046c4 compares the card's printed cost.
        game.add_power(target, 3, owner);
        if game.printed_cost(target) >= 5 {
            game.add_cost(target, -1, owner);
        }
    }
}

fn the_hunger(game: &mut Game<'_>, card: usize) {
    if let Some(lane) = game.cards[card].lane {
        game.horde(lane, game.cards[card].owner, 3);
    }
}

fn fastball_special(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let allies: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| {
            id != card
                && game.cards[id].owner == owner
                && game.cards[id].revealed
                && game.is_character(id)
        })
        .collect();
    let Some(max) = allies.iter().map(|&id| game.cards[id].power()).max() else {
        return;
    };
    let highest: Vec<_> = allies
        .into_iter()
        .filter(|&id| game.cards[id].power() == max)
        .collect();
    let victim = highest[game.rng.borrow_mut().randbelow(highest.len())];
    // DestroyToAfflict 0x7d024b8 captures power and enemy candidates before destruction.
    let enemies: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| game.cards[id].owner != owner && game.cards[id].revealed)
        .collect();
    if game.destroy_card(victim) && !enemies.is_empty() && max > 0 {
        let enemy = enemies[game.rng.borrow_mut().randbelow(enemies.len())];
        game.add_power(enemy, -max, owner);
    }
}

fn adamantium_infusion(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    // ResurrectCard 0x7cffaac snapshots and sorts the graveyard before banishing
    // the source, but reads the first candidate's power after the banish.
    let mut targets: Vec<_> = game.players[owner]
        .graveyard
        .iter()
        .filter_map(|&(id, _)| game.is_character(id).then_some(id))
        .collect();
    if targets.is_empty() {
        return;
    }
    targets.sort_by_key(|&id| std::cmp::Reverse(game.cards[id].power()));
    game.banish_card(card);
    let power = game.cards[targets[0]].power();
    let count = targets
        .iter()
        .take_while(|&&id| game.cards[id].power() == power)
        .count();
    let target = targets[game.rng.borrow_mut().randbelow(count)];
    game.revive_card(target, lane, 2, owner);
}

fn nicholas_scratch(game: &mut Game<'_>, card: usize, played: usize) {
    // BuffSelf 0x7f0bc98 tests the resolved card's type and player, not its location.
    if game.cards[card].lane.is_some()
        && !game.cards[card].abilities_removed
        && game.cards[card].owner == game.cards[played].owner
        && game.is_skill(played)
    {
        game.add_power(card, 1, game.cards[card].owner);
    }
}

fn vampire_killmonger(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if game.cards[card].lane.is_none()
        || game.cards[card].abilities_removed
        || !game.players[owner]
            .energy_history
            .iter()
            .any(|&(turn, _, all)| turn == game.turn && all)
        || game.players[owner].hand.len() >= 7
    {
        return;
    }
    let spear = game.create_card("VampiricSpear", owner);
    game.add_hand(owner, spear, false);
}

fn vampiric_spear(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let killmongers: Vec<_> = game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&id| {
            game.cards[id].owner == owner
                && game.cards[id].revealed
                && game.cards[id].def.id == "VampireKillmonger"
        })
        .collect();
    let targets: Vec<_> = game.players[1 - owner]
        .hand
        .iter()
        .copied()
        .filter(|&id| game.is_character(id))
        .collect();
    if killmongers.is_empty() || targets.is_empty() {
        return;
    }
    // StealFromOpponentHand 0x7d190bc rolls once and every Killmonger steals from that card.
    let target = targets[game.rng.borrow_mut().randbelow(targets.len())];
    for killmonger in killmongers {
        game.steal_power(target, killmonger, 2, owner);
    }
}

fn zombie_scarlet_witch(game: &mut Game<'_>, card: usize, played: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    // PlayToHorde 0x7cfdd28 checks the played location, not its current zone.
    let played_lane = game.played[owner]
        .iter()
        .find_map(|&(id, lane)| (id == played).then_some(lane))
        .or(game.cards[played].lane);
    if !game.cards[card].abilities_removed
        && game.cards[played].owner == owner
        && played_lane == Some(lane)
    {
        game.horde(lane, owner, 2);
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
            "turn": 3, "energy": 3, "hand": [], "deck": [],
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
    fn played_skill_notifies_scratch_before_banishment() {
        let p = position(
            json!([{"id":"NicholasScratch"},{"id":"TheHunger"}]),
            json!([]),
        );
        let mut rng = PythonRandom::seed(8);
        let mut game = build_game(&p, &mut rng).unwrap();
        let scratch = game.lanes[0].cards[0];
        let skill = game.lanes[0].cards[1];
        game.cards[skill].revealed = false;
        game.cards[skill].turn_played = game.turn;
        game.reveal(skill);
        assert_eq!(game.cards[scratch].power(), 4);
        assert!(game.players[0].banished.contains(&skill));
    }

    #[test]
    fn chord_silences_only_eligible_enemy() {
        let p = position(
            json!([{"id":"DeafeningChord"}]),
            json!([{"id":"Hawkeye"},{"id":"Hulk"}]),
        );
        let mut rng = PythonRandom::seed(1);
        let mut game = build_game(&p, &mut rng).unwrap();
        let chord = game.lanes[0].cards[0];
        let hawkeye = game.lanes[0].cards[1];
        let hulk = game.lanes[0].cards[2];
        deafening_chord(&mut game, chord);
        assert!(game.cards[hawkeye].abilities_removed);
        assert!(!game.cards[hulk].abilities_removed);
    }

    #[test]
    fn wingbeat_targets_leftmost_and_uses_printed_cost() {
        let p = position(json!([{"id":"MajesticWingbeat"}]), json!([]));
        let mut rng = PythonRandom::seed(2);
        let mut game = build_game(&p, &mut rng).unwrap();
        let skill = game.lanes[0].cards[0];
        let hulk = game.create_card("Hulk", 0);
        let hawkeye = game.create_card("Hawkeye", 0);
        game.add_hand(0, hulk, false);
        game.add_hand(0, hawkeye, false);
        game.add_cost(hulk, -3, 0);
        majestic_wingbeat(&mut game, skill);
        assert_eq!(game.cards[hulk].power(), 17);
        assert_eq!(game.cards[hulk].cost, 2);
        assert_eq!(game.cards[hawkeye].power(), 1);
    }

    #[test]
    fn hunger_and_witch_grow_horde() {
        let p = position(
            json!([{"id":"TheHunger"},{"id":"ZombieScarletWitch"},{"id":"Hawkeye"}]),
            json!([]),
        );
        let mut rng = PythonRandom::seed(3);
        let mut game = build_game(&p, &mut rng).unwrap();
        let skill = game.lanes[0].cards[0];
        let witch = game.lanes[0].cards[1];
        let played = game.lanes[0].cards[2];
        the_hunger(&mut game, skill);
        let horde = game
            .lanes
            .iter()
            .flat_map(|lane| lane.cards.iter())
            .copied()
            .find(|&id| game.cards[id].def.id == "ZombieHorde")
            .unwrap();
        let before = game.cards[horde].power();
        zombie_scarlet_witch(&mut game, witch, played);
        assert_eq!(game.cards[horde].power(), before + 2);
        game.cards[witch].abilities_removed = true;
        zombie_scarlet_witch(&mut game, witch, played);
        assert_eq!(game.cards[horde].power(), before + 2);
    }

    #[test]
    fn fastball_destroys_highest_even_without_an_enemy() {
        let p = position(
            json!([{"id":"FastballSpecial"},{"id":"Hawkeye"},{"id":"Hulk"}]),
            json!([]),
        );
        let mut rng = PythonRandom::seed(4);
        let mut game = build_game(&p, &mut rng).unwrap();
        let skill = game.lanes[0].cards[0];
        let hulk = game.lanes[0].cards[2];
        fastball_special(&mut game, skill);
        assert!(game.players[0].graveyard.iter().any(|&(id, _)| id == hulk));
    }

    #[test]
    fn fastball_nonpositive_power_does_not_buff_enemy() {
        let p = position(
            json!([{"id":"FastballSpecial"},{"id":"Hawkeye"}]),
            json!([{"id":"Hulk"}]),
        );
        let mut rng = PythonRandom::seed(9);
        let mut game = build_game(&p, &mut rng).unwrap();
        let skill = game.lanes[0].cards[0];
        let victim = game.lanes[0].cards[1];
        let enemy = game.lanes[0].cards[2];
        game.set_power(victim, -2, 1);
        fastball_special(&mut game, skill);
        assert_eq!(game.cards[enemy].power(), 14);
        assert!(
            game.players[0]
                .graveyard
                .iter()
                .any(|&(id, _)| id == victim)
        );
    }

    #[test]
    fn infusion_revives_strongest_from_another_lane() {
        let p = position(
            json!([{"id":"AdamantiumInfusion"},{"id":"Hawkeye"},{"id":"Hulk"}]),
            json!([]),
        );
        let mut rng = PythonRandom::seed(10);
        let mut game = build_game(&p, &mut rng).unwrap();
        let infusion = game.lanes[0].cards[0];
        let hawkeye = game.lanes[0].cards[1];
        let hulk = game.lanes[0].cards[2];
        assert!(game.destroy_card(hawkeye));
        assert!(game.destroy_card(hulk));
        game.players[0]
            .graveyard
            .iter_mut()
            .find(|entry| entry.0 == hulk)
            .unwrap()
            .1 = Some(1);
        adamantium_infusion(&mut game, infusion);
        assert!(game.players[0].banished.contains(&infusion));
        assert_eq!(game.cards[hulk].lane, Some(0));
        assert_eq!(game.cards[hulk].power(), 28);
        assert!(
            game.players[0]
                .graveyard
                .iter()
                .any(|&(id, _)| id == hawkeye)
        );
    }

    #[test]
    fn copied_infusion_banishes_character_and_uses_post_banish_power() {
        let p = position(
            json!([{"id":"IronLad"},{"id":"Hulk"},{"id":"Hawkeye"}]),
            json!([]),
        );
        let mut rng = PythonRandom::seed(11);
        let mut game = build_game(&p, &mut rng).unwrap();
        let lad = game.lanes[0].cards[0];
        let hulk = game.lanes[0].cards[1];
        let hawkeye = game.lanes[0].cards[2];
        assert!(game.destroy_card(hulk));
        assert!(game.destroy_card(hawkeye));
        let infusion = game.create_card("AdamantiumInfusion", 0);
        assert!(game.copy_text(lad, infusion, false));
        // Banish recomputes ongoing power even on graveyard cards.
        game.cards[hulk].ongoing = 5;
        adamantium_infusion(&mut game, lad);
        assert!(game.players[0].banished.contains(&lad));
        assert_eq!(game.cards[hulk].lane, Some(0));
        assert_eq!(game.cards[hulk].power(), 28);
    }

    #[test]
    fn copied_infusion_with_no_destroyed_card_does_not_banish_source() {
        let p = position(json!([{"id":"IronLad"}]), json!([]));
        let mut rng = PythonRandom::seed(12);
        let mut game = build_game(&p, &mut rng).unwrap();
        let lad = game.lanes[0].cards[0];
        let infusion = game.create_card("AdamantiumInfusion", 0);
        assert!(game.copy_text(lad, infusion, false));
        adamantium_infusion(&mut game, lad);
        assert_eq!(game.cards[lad].lane, Some(0));
        assert!(!game.players[0].banished.contains(&lad));
    }

    #[test]
    fn scratch_counts_allied_skills_only() {
        let p = position(
            json!([{"id":"NicholasScratch"},{"id":"TheHunger"},{"id":"Hawkeye"}]),
            json!([]),
        );
        let mut rng = PythonRandom::seed(5);
        let mut game = build_game(&p, &mut rng).unwrap();
        let scratch = game.lanes[0].cards[0];
        let skill = game.lanes[0].cards[1];
        let character = game.lanes[0].cards[2];
        nicholas_scratch(&mut game, scratch, character);
        assert_eq!(game.cards[scratch].power(), 3);
        nicholas_scratch(&mut game, scratch, skill);
        assert_eq!(game.cards[scratch].power(), 4);
    }

    #[test]
    fn killmonger_only_creates_spear_after_spending_all() {
        let p = position(json!([{"id":"VampireKillmonger"}]), json!([]));
        let mut rng = PythonRandom::seed(6);
        let mut game = build_game(&p, &mut rng).unwrap();
        let killmonger = game.lanes[0].cards[0];
        game.players[0].energy_history.push((game.turn, 2, false));
        vampire_killmonger(&mut game, killmonger);
        assert!(game.players[0].hand.is_empty());
        game.players[0].energy_history.clear();
        game.players[0].energy_history.push((game.turn, 3, true));
        vampire_killmonger(&mut game, killmonger);
        assert_eq!(game.cards[game.players[0].hand[0]].def.id, "VampiricSpear");
    }

    #[test]
    fn spear_steals_from_one_enemy_for_each_killmonger() {
        let p = position(
            json!([{"id":"VampiricSpear"},{"id":"VampireKillmonger"},{"id":"VampireKillmonger"}]),
            json!([]),
        );
        let mut rng = PythonRandom::seed(7);
        let mut game = build_game(&p, &mut rng).unwrap();
        let spear = game.lanes[0].cards[0];
        let a = game.lanes[0].cards[1];
        let b = game.lanes[0].cards[2];
        let target = game.create_card("Hulk", 1);
        game.add_hand(1, target, false);
        vampiric_spear(&mut game, spear);
        assert_eq!(game.cards[a].power(), 4);
        assert_eq!(game.cards[b].power(), 4);
        assert_eq!(game.cards[target].power(), 10);
    }
}
