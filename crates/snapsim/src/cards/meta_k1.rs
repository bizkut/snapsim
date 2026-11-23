//! Card rules from the 59.15.1 client.
use super::CardDef;
use crate::game::{Event, Game};

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        on_event: Some(black_cat),
        end_turn_trigger: true,
        ..CardDef::vanilla("BlackCat")
    },
    CardDef {
        on_move_here: Some(kingpin),
        ..CardDef::vanilla("Kingpin")
    },
    CardDef {
        reveal: Some(gilgamesh),
        ..CardDef::vanilla("Gilgamesh")
    },
    CardDef {
        ongoing: Some(starbrand),
        ongoing_ability: true,
        ..CardDef::vanilla("Starbrand")
    },
    CardDef {
        reveal: Some(psylocke),
        ..CardDef::vanilla("Psylocke")
    },
    CardDef {
        on_activate: Some(danger),
        ..CardDef::vanilla("Danger")
    },
    CardDef {
        reveal: Some(colleen_wing),
        ..CardDef::vanilla("ColleenWing")
    },
    CardDef {
        end_turn: Some(moondragon),
        end_turn_trigger: true,
        ..CardDef::vanilla("Moondragon")
    },
    CardDef {
        ongoing_ability: true,
        ..CardDef::vanilla("HowardTheDuck")
    },
    CardDef {
        reveal: Some(zombie_nova),
        on_destroy: Some(zombie_nova),
        ..CardDef::vanilla("ZombieNova")
    },
    CardDef {
        reveal: Some(debrii),
        ..CardDef::vanilla("Debrii")
    },
    CardDef {
        reveal: Some(dr_doom),
        ..CardDef::vanilla("DrDoom")
    },
    CardDef {
        reveal: Some(rhino),
        ..CardDef::vanilla("Rhino")
    },
    CardDef::vanilla("DoomBot"),
    CardDef {
        reveal: Some(hazmat),
        ..CardDef::vanilla("Hazmat")
    },
    CardDef {
        ongoing: Some(orka),
        ongoing_ability: true,
        ..CardDef::vanilla("Orka")
    },
    CardDef {
        end_turn: Some(sunspot),
        end_turn_trigger: true,
        ..CardDef::vanilla("Sunspot")
    },
    CardDef {
        on_move: Some(vulture),
        ..CardDef::vanilla("Vulture")
    },
];

pub(crate) static TOKENS: &[&str] = &["DoomBot"];

// BlackCatBehavior::DiscardSelf (0x7e727f8) fires at end of turn
// only while this card is in its owner's hand.
fn black_cat(game: &mut Game<'_>, card: usize, event: Event) {
    let owner = game.cards[card].owner;
    if matches!(event, Event::TurnEnd) && game.players[owner].hand.contains(&card) {
        game.discard_card(owner, card);
    }
}

// KingpinBehavior::ReducePower (0x7edf858): an enemy moved to
// Kingpin's location receives -2 permanent power.
fn kingpin(game: &mut Game<'_>, card: usize, target: usize) {
    if game.cards[target].owner != game.cards[card].owner && game.cards[target].revealed {
        game.add_power(target, -2, game.cards[card].owner);
    }
}

// GilgameshBehavior::BuffThisCard (0x7eb6850), WithIncreasedPower
// (0x7d29874): count other revealed allies above their printed power.
fn gilgamesh(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let count = game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&target| {
            target != card
                && game.cards[target].owner == owner
                && game.cards[target].revealed
                && game.has_increased_power(target)
        })
        .count() as i32;
    if count > 0 {
        game.add_power(card, count, owner);
    }
}

// StarbrandBehavior::AddPowerToOpponent (0x7cc2f84) adds 3 to
// the opponent's score at each other location.
fn starbrand(game: &mut Game<'_>, card: usize) {
    let Some(source) = game.cards[card].lane else {
        return;
    };
    let opponent = 1 - game.cards[card].owner;
    for lane in 0..3 {
        if lane != source {
            game.add_lane_bonus(lane, opponent, 3);
        }
    }
}

// PsylockeBehavior::AddEnergyTriggerToPlayer (0x7f1b9c4) installs
// a player-owned trigger granting one energy on the following turn.
fn psylocke(game: &mut Game<'_>, card: usize) {
    game.schedule_energy(game.cards[card].owner, game.turn.saturating_add(1), 1);
}

// DangerBehavior::DestroyCard (0x7e87438), predicate 0x7e87cd0:
// randomly choose a revealed enemy character here with strictly less power.
fn danger(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let power = game.cards[card].power();
    let targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&target| {
            game.cards[target].owner != owner
                && game.cards[target].revealed
                && game.is_character(target)
                && game.cards[target].power() < power
        })
        .collect();
    if !targets.is_empty() {
        let target = targets[game.rng.borrow_mut().randbelow(targets.len())];
        game.destroy_card(target);
    }
}

// ColleenWingBehavior::DiscardLowestCostCard (0x7e822e0): sort by current
// cost, then roll among all cards tied for the minimum.
fn colleen_wing(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let hand = &game.players[owner].hand;
    let Some(minimum) = hand.iter().map(|&target| game.card_cost(target)).min() else {
        return;
    };
    let targets: Vec<_> = hand
        .iter()
        .copied()
        .filter(|&target| game.card_cost(target) == minimum)
        .collect();
    let target = targets[game.rng.borrow_mut().randbelow(targets.len())];
    game.discard_card(owner, target);
}

// MoondragonBehavior::BuffSelf (0x7efef94) checks the number of cards
// staged for play this turn and current energy at end of turn.
fn moondragon(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if game.players[owner].energy == 0
        && game.players[owner]
            .staged_history
            .iter()
            .filter(|p| p.turn == game.turn)
            .count()
            == 1
    {
        game.add_power(card, 2, owner);
    }
}

// ZombieNovaBehavior::GrowHorde (0x7cfd56c) uses the current or previous
// location on reveal or destruction, respectively.
fn zombie_nova(game: &mut Game<'_>, card: usize) {
    if let Some(lane) = game.cards[card].lane.or(game.cards[card].last_lane) {
        game.horde(lane, game.cards[card].owner, 3);
    }
}

// DebriiBehavior::AddRocks (0x7e8a6ac) creates Rocks for both players
// at each other location; DrDoomBehavior::AddDoombot (0x7e93cf0)
// creates one allied DoomBot at each other location.
fn debrii(game: &mut Game<'_>, card: usize) {
    let Some(source) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    for lane in 0..3 {
        if lane != source {
            for seat in [owner, 1 - owner] {
                let token = game.create_card("Rock", seat);
                game.cards[token].created_by_card_id = Some("Debrii".to_owned());
                game.cards[token].created_by_player = Some(owner);
                game.add_card(token, lane, seat, true);
            }
        }
    }
}

pub(crate) fn dr_doom(game: &mut Game<'_>, card: usize) {
    let Some(source) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    for lane in 0..3 {
        if lane != source {
            let token = game.create_card("DoomBot", owner);
            game.cards[token].created_by_card_id = Some("DrDoom".to_owned());
            game.cards[token].created_by_player = Some(owner);
            game.add_card(token, lane, owner, true);
        }
    }
}

// RhinoBehavior::AddRock (0x7f28be8) creates an allied Rock here.
fn rhino(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let token = game.create_card("Rock", owner);
    game.cards[token].created_by_card_id = Some("Rhino".to_owned());
    game.cards[token].created_by_player = Some(owner);
    game.add_card(token, lane, owner, true);
}

// HazmatBehavior::SubtractPower (0x7ebbfd4): all other revealed cards.
fn hazmat(game: &mut Game<'_>, card: usize) {
    let targets: Vec<_> = game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&other| other != card && game.cards[other].revealed)
        .collect();
    for target in targets {
        game.add_power(target, -1, game.cards[card].owner);
    }
}

// OrkaBehavior::BuffSelf (0x7f11d0c): only allied card at this location.
fn orka(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    if game.lanes[lane]
        .cards
        .iter()
        .filter(|&&other| game.cards[other].owner == owner)
        .count()
        == 1
    {
        game.add_ongoing_power(card, 7, owner);
    }
}

// SunspotBehavior::BuffSelf (0x7ccb3d4): current unspent energy.
fn sunspot(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let energy = game.players[owner].energy;
    if energy > 0 {
        game.add_power(card, energy, owner);
    }
}

// VultureBehavior::BuffSelf (0x7ced15c): on each move while in play.
fn vulture(game: &mut Game<'_>, card: usize, _from: usize) {
    game.add_power(card, 4, game.cards[card].owner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{locations, random::PythonRandom};

    fn setup<'a>(rng: &'a mut PythonRandom) -> Game<'a> {
        let deck = [
            "Wasp",
            "MistyKnight",
            "Hulk",
            "Cyclops",
            "Shocker",
            "TheThing",
            "Abomination",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
            "Quicksilver",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, rng).unwrap();
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
    fn hazmat_afflicts_only_other_revealed_cards() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let h = place(&mut g, "Hazmat", 0, 0, true);
        let a = place(&mut g, "Hulk", 0, 1, true);
        let b = place(&mut g, "Hulk", 1, 2, false);
        let before = (
            g.cards[h].permanent,
            g.cards[a].permanent,
            g.cards[b].permanent,
        );
        hazmat(&mut g, h);
        assert_eq!(
            (
                g.cards[h].permanent,
                g.cards[a].permanent,
                g.cards[b].permanent
            ),
            (before.0, before.1 - 1, before.2)
        );
    }
    #[test]
    fn orka_counts_facedown_allies() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let o = place(&mut g, "Orka", 0, 0, true);
        g.ongoing();
        assert_eq!(g.cards[o].ongoing, 7);
        place(&mut g, "Hulk", 0, 0, false);
        g.ongoing();
        assert_eq!(g.cards[o].ongoing, 0);
    }
    #[test]
    fn sunspot_uses_current_energy() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let s = place(&mut g, "Sunspot", 0, 0, true);
        g.players[0].energy = 3;
        sunspot(&mut g, s);
        assert_eq!(g.cards[s].permanent, 3);
        g.players[0].energy = 0;
        sunspot(&mut g, s);
        assert_eq!(g.cards[s].permanent, 3);
    }
    #[test]
    fn vulture_gains_power_per_move() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let v = place(&mut g, "Vulture", 0, 0, true);
        let before = g.cards[v].permanent;
        assert!(g.move_card(v, 1));
        assert!(g.move_card(v, 2));
        assert_eq!(g.cards[v].permanent, before + 8);
    }
    #[test]
    fn debrii_creates_both_sides_at_other_lanes() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let c = place(&mut g, "Debrii", 0, 1, true);
        debrii(&mut g, c);
        assert_eq!(g.lanes[0].cards.len(), 2);
        assert_eq!(g.lanes[1].cards.len(), 1);
        assert_eq!(g.lanes[2].cards.len(), 2);
        assert!(
            g.lanes[0]
                .cards
                .iter()
                .all(|&i| g.cards[i].def.id == "Rock")
        );
    }
    #[test]
    fn doom_creates_only_allied_bots_elsewhere() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let c = place(&mut g, "DrDoom", 0, 1, true);
        dr_doom(&mut g, c);
        for lane in [0, 2] {
            let i = g.lanes[lane].cards[0];
            assert_eq!((g.cards[i].def.id, g.cards[i].owner), ("DoomBot", 0));
        }
        assert_eq!(g.lanes[1].cards.len(), 1);
    }
    #[test]
    fn rhino_creates_rock_here_unless_full() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let c = place(&mut g, "Rhino", 0, 1, true);
        rhino(&mut g, c);
        assert_eq!(g.cards[g.lanes[1].cards[1]].def.id, "Rock");
        place(&mut g, "Hulk", 0, 1, true);
        place(&mut g, "Hulk", 0, 1, true);
        rhino(&mut g, c);
        assert_eq!(g.lanes[1].cards.len(), 4);
    }
    #[test]
    fn howard_is_ongoing_without_changing_game_state() {
        let def = super::super::get("HowardTheDuck").unwrap();
        assert!(def.ongoing_ability);
        assert!(def.ongoing.is_none());
    }
    #[test]
    fn zombie_nova_grows_horde_on_reveal_and_destroy() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let c = place(&mut g, "ZombieNova", 0, 1, true);
        zombie_nova(&mut g, c);
        let horde = g
            .lanes
            .iter()
            .flat_map(|lane| &lane.cards)
            .copied()
            .find(|&i| g.cards[i].def.id == "ZombieHorde")
            .unwrap();
        let power = g.cards[horde].permanent;
        assert!(g.destroy_card(c));
        assert_eq!(g.cards[horde].permanent, power + 3);
    }
    #[test]
    fn moondragon_requires_one_play_and_zero_energy() {
        use crate::game::StagedPlay;
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let c = place(&mut g, "Moondragon", 0, 1, true);
        let before = g.cards[c].permanent;
        g.players[0].energy = 0;
        moondragon(&mut g, c);
        assert_eq!(g.cards[c].permanent, before);
        g.players[0].staged_history.push(StagedPlay {
            turn: g.turn,
            card: c,
            def_id: "Moondragon",
            printed_cost: 2,
            created: false,
            resolved: true,
        });
        g.players[0].energy = 1;
        moondragon(&mut g, c);
        assert_eq!(g.cards[c].permanent, before);
        g.players[0].energy = 0;
        moondragon(&mut g, c);
        assert_eq!(g.cards[c].permanent, before + 2);
    }
    #[test]
    fn colleen_discards_only_a_lowest_current_cost_card() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let c = place(&mut g, "ColleenWing", 0, 0, true);
        let expensive = g.create_card("Hulk", 0);
        let cheap = g.create_card("Wasp", 0);
        g.players[0].hand = vec![expensive, cheap];
        colleen_wing(&mut g, c);
        assert_eq!(g.players[0].hand, vec![expensive]);
        assert!(g.players[0].discard.contains(&cheap));
    }
    #[test]
    fn danger_requires_strictly_lower_power() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let c = place(&mut g, "Danger", 0, 0, true);
        let target = place(&mut g, "Wasp", 1, 0, true);
        let equal = place(&mut g, "Hulk", 1, 0, true);
        g.cards[equal].permanent = g.cards[c].power();
        danger(&mut g, c);
        assert!(g.cards[target].lane.is_none());
        assert_eq!(g.cards[equal].lane, Some(0));
    }
    #[test]
    fn psylocke_schedules_next_turn_energy_even_after_removal() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let c = place(&mut g, "Psylocke", 0, 0, true);
        psylocke(&mut g, c);
        assert_eq!(g.players[0].scheduled_energy, vec![(g.turn + 1, 1)]);
        assert!(g.destroy_card(c));
        assert_eq!(g.players[0].scheduled_energy, vec![(g.turn + 1, 1)]);
    }
    #[test]
    fn starbrand_scores_only_other_lanes_for_opponent() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let c = place(&mut g, "Starbrand", 0, 1, true);
        g.ongoing();
        assert_eq!(
            [
                g.lanes[0].bonus[1],
                g.lanes[1].bonus[1],
                g.lanes[2].bonus[1]
            ],
            [3, 0, 3]
        );
        g.cards[c].abilities_removed = true;
        g.ongoing();
        assert_eq!(g.lanes[0].bonus[1], 0);
    }
    #[test]
    fn gilgamesh_counts_only_other_increased_revealed_allies() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let c = place(&mut g, "Gilgamesh", 0, 0, true);
        let a = place(&mut g, "Wasp", 0, 1, true);
        let hidden = place(&mut g, "Wasp", 0, 2, false);
        g.add_power(a, 1, 0);
        g.add_power(hidden, 1, 0);
        let before = g.cards[c].permanent;
        gilgamesh(&mut g, c);
        assert_eq!(g.cards[c].permanent, before + 1);
    }
    #[test]
    fn kingpin_afflicts_enemy_but_not_friendly_move() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        place(&mut g, "Kingpin", 0, 1, true);
        let enemy = place(&mut g, "Hulk", 1, 0, true);
        let ally = place(&mut g, "Hulk", 0, 0, true);
        let before_enemy = g.cards[enemy].permanent;
        let before_ally = g.cards[ally].permanent;
        assert!(g.move_card(enemy, 1));
        assert!(g.move_card(ally, 1));
        assert_eq!(g.cards[enemy].permanent, before_enemy - 2);
        assert_eq!(g.cards[ally].permanent, before_ally);
    }
    #[test]
    fn coverage_gates_missing_cobra_and_darkhawk_inputs() {
        use crate::{coverage, position::Position};
        let mut position = Position {
            turn: 2,
            ..Position::default()
        };
        position.opponent.seen.push("Darkhawk".into());
        assert!(
            coverage::coverage(&position)["state"]
                .to_string()
                .contains("Darkhawk opponent deck count unknown")
        );
        position.deck_count = Some(0);
        assert!(
            !coverage::coverage(&position)["state"]
                .to_string()
                .contains("Darkhawk opponent deck count unknown")
        );
        position
            .hand
            .push(serde_json::from_value(serde_json::json!({"id":"Cobra"})).unwrap());
        position.locations[0].theirs.push(
            serde_json::from_value(serde_json::json!({"id":"Hulk","revealed":true})).unwrap(),
        );
        assert!(
            coverage::coverage(&position)["state"]
                .to_string()
                .contains("Cobra target created provenance unknown")
        );
        position.locations[0].theirs[0].created = Some(false);
        assert!(
            !coverage::coverage(&position)["state"]
                .to_string()
                .contains("Cobra target created provenance unknown")
        );
    }
    #[test]
    fn black_cat_discards_from_hand_not_deck_or_board() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let hand = g.create_card("BlackCat", 0);
        let deck = g.create_card("BlackCat", 0);
        let board = place(&mut g, "BlackCat", 0, 0, true);
        g.players[0].hand.push(hand);
        g.players[0].deck.push(deck);
        g.end_turn_effects();
        assert!(g.players[0].discard.contains(&hand));
        assert!(g.players[0].deck.contains(&deck));
        assert_eq!(g.cards[board].lane, Some(0));
    }
}
