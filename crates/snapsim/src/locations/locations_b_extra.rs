use super::{CostUpdate, LocationDef, location};
use crate::game::Game;

pub const ONSLAUGHTS_CITADEL: LocationDef = LocationDef {
    ongoing_repeats: 2,
    ..location("OnslaughtsCitadel", 0, 0, 1, false)
};

pub const TITAN: LocationDef = LocationDef {
    modify_cost: Some(titan),
    ..location("Titan", 0, 0, 1, false)
};
pub const SANDBAR: LocationDef = LocationDef {
    modify_cost: Some(sandbar),
    ..location("TheSandbar", 0, 0, 1, false)
};
pub const STARK_ISLAND: LocationDef = LocationDef {
    on_turn_end: Some(stark_island),
    ..location("StarkIsland", 0, 0, 1, false)
};
pub const ICE_BOX: LocationDef = LocationDef {
    on_reveal: Some(ice_box),
    ..location("TheIceBox", 0, 0, 1, false)
};
pub const DEADLANDS: LocationDef = LocationDef {
    on_turn_end: Some(deadlands),
    ..location("TheDeadlands", 0, 0, 1, false)
};
pub const SURVIVORS_CAMP: LocationDef = LocationDef {
    on_turn_end: Some(survivors_camp),
    ..location("SurvivorsCamp", 0, 0, 1, false)
};
pub const UNDEAD_METROPOLIS: LocationDef = LocationDef {
    after_card_play: Some(undead_metropolis),
    ..location("UndeadMetropolis", 0, 0, 1, false)
};
pub const OSCORP_TOWER: LocationDef = LocationDef {
    on_turn_end: Some(oscorp_tower),
    ..location("OscorpTower", 0, 0, 1, false)
};
pub const MIRROR_DIMENSION: LocationDef = LocationDef {
    on_turn_start: Some(mirror_dimension),
    ..location("MirrorDimension", 0, 0, 1, false)
};
pub const ZENN_LA: LocationDef = LocationDef {
    on_reveal: Some(zenn_la),
    ..location("ZennLa", 0, 0, 1, false)
};
pub const SAKAAR: LocationDef = LocationDef {
    on_turn_end: Some(sakaar),
    ..location("Sakaar", 0, 0, 1, false)
};
pub const OTTOS_LAB: LocationDef = LocationDef {
    after_card_play: Some(ottos_lab),
    initial_state: Some(|| Box::new([false; 2])),
    ..location("OttosLab", 0, 0, 1, false)
};
pub const STRANGE_ACADEMY: LocationDef = LocationDef {
    on_turn_end: Some(strange_academy),
    ..location("StrangeAcademy", 0, 0, 1, false)
};
pub const QUANTUM_TUNNEL: LocationDef = LocationDef {
    after_card_play: Some(quantum_tunnel),
    ..location("QuantumTunnel", 0, 0, 1, false)
};
pub const SINISTER_LONDON: LocationDef = LocationDef {
    after_card_play: Some(sinister_london),
    ..location("SinisterLondon", 0, 0, 1, false)
};
pub const WESTVIEW: LocationDef = LocationDef {
    on_turn_start: Some(westview),
    ..location("Westview", 0, 0, 1, false)
};
pub const WHITE_PALACE: LocationDef = LocationDef {
    on_reveal: Some(white_palace),
    ..location("WhitePalace", 0, 0, 1, false)
};

fn titan(game: &Game<'_>, _: usize, card: usize, _: i32) -> Option<CostUpdate> {
    (game.cards[card].cost == 6).then_some(CostUpdate::Add(-1))
}
// TheSandbarBehavior::IncreaseCardCost (0x7d8c410) targets only created
// cards and uses AddCostWithMax(+1, max(current, 6)).
fn sandbar(game: &Game<'_>, _: usize, card: usize, cost: i32) -> Option<CostUpdate> {
    game.cards[card].created.then_some(if cost < 6 {
        CostUpdate::Add(1)
    } else {
        CostUpdate::Set(cost)
    })
}
fn stark_island(game: &mut Game<'_>, _: usize) {
    for seat in 0..2 {
        let candidates: Vec<_> = game.players[seat]
            .hand
            .iter()
            .copied()
            .filter(|&id| game.card_cost(id) > 0)
            .collect();
        if !candidates.is_empty() {
            let index = game.rng.borrow_mut().randbelow(candidates.len());
            let card = candidates[index];
            game.set_cost(card, (game.cards[card].cost - 1).max(0), seat);
        }
    }
}
fn ice_box(game: &mut Game<'_>, _: usize) {
    for seat in 0..2 {
        let len = game.players[seat].hand.len();
        if len != 0 {
            let index = game.rng.borrow_mut().randbelow(len);
            let card = game.players[seat].hand[index];
            game.add_cost(card, 1, seat);
        }
    }
}
fn deadlands(game: &mut Game<'_>, lane: usize) {
    for seat in 0..2 {
        game.horde(lane, seat, 1);
    }
}
fn survivors_camp(game: &mut Game<'_>, lane: usize) {
    if game.turn == 4 {
        let winner = game.lane_winner(lane);
        for seat in 0..2 {
            if winner == Some(seat) {
                game.draw(seat);
            } else {
                game.horde(lane, seat, 2);
            }
        }
    }
}
fn undead_metropolis(game: &mut Game<'_>, lane: usize, card: usize) {
    game.horde(lane, game.cards[card].owner, 1);
}
fn oscorp_tower(game: &mut Game<'_>, lane: usize) {
    if game.turn == 3 {
        for &card in &game.lanes[lane].cards {
            game.cards[card].owner = 1 - game.cards[card].owner;
        }
    }
}
fn mirror_dimension(game: &mut Game<'_>, lane: usize) {
    if game.turn == 4 {
        let choices: Vec<_> = (0..3)
            .filter(|&i| i != lane && game.lanes[i].revealed)
            .collect();
        if !choices.is_empty() {
            let index = game.rng.borrow_mut().randbelow(choices.len());
            // Copy the definition itself: a location without rules is an unregistered stand-in.
            let source = &game.lanes[choices[index]];
            let (def, observed_id) = (source.def, source.observed_id.clone());
            game.replace_location_def(lane, def, observed_id);
        }
    }
}
fn zenn_la(game: &mut Game<'_>, lane: usize) {
    if crate::cards::reference_rules_enabled() {
        crate::cards::reference_rules::zenn_la(game, lane);
        return;
    }
    // ZennLaBehavior::RecruitThreeCostCard (0x7d9bcec) takes the first matching
    // deck card; its predicate (0x7d9c414) compares DefCost, not modified cost.
    for seat in 0..2 {
        let slot = game.players[seat]
            .deck
            .iter()
            .position(|&id| game.printed_cost(id) == 3);
        if let Some(slot) = slot {
            let id = game.players[seat].deck[slot];
            if game.can_add(id, lane, seat) {
                game.take_deck(seat, slot);
                game.add_card(id, lane, seat, true);
            }
        }
    }
}
fn put_from_hand(game: &mut Game<'_>, lane: usize, seat: usize, reveal: bool) {
    let len = game.players[seat].hand.len();
    if len > 0 {
        let slot = game.rng.borrow_mut().randbelow(len);
        let card = game.players[seat].hand[slot];
        if game.can_add(card, lane, seat) {
            game.take_hand(seat, slot);
            game.add_card(card, lane, seat, reveal);
        }
    }
}
fn sakaar(game: &mut Game<'_>, lane: usize) {
    if game.lanes[lane].revealed_turn == Some(game.turn) {
        for seat in 0..2 {
            put_from_hand(game, lane, seat, false);
        }
    }
}
fn ottos_lab(game: &mut Game<'_>, lane: usize, card: usize) {
    let owner = game.cards[card].owner;
    let spent = game.lanes[lane]
        .state
        .as_deref_mut()
        .unwrap()
        .as_any_mut()
        .downcast_mut::<[bool; 2]>()
        .unwrap();
    if spent[owner] {
        return;
    }
    spent[owner] = true;
    let opponent = 1 - owner;
    if game.has_space(lane, opponent, 0) {
        put_from_hand(game, lane, opponent, true);
    }
}
fn strange_academy(game: &mut Game<'_>, lane: usize) {
    if game.turn != 5 {
        return;
    }
    for card in game.lanes[lane].cards.clone() {
        // StrangeAcademyBehavior::MoveAllCards (0x7d84854) enumerates
        // revealed cards only; hidden cards must not consume a random roll.
        if !crate::cards::reference_rules_enabled() && !game.cards[card].revealed {
            continue;
        }
        if !crate::cards::reference_rules_enabled() {
            if let Some(target) = game.random_location_to_move_card(card, lane, true) {
                game.move_card(card, target);
            }
            continue;
        }
        let seat = game.cards[card].owner;
        let targets: Vec<_> = (0..3)
            .filter(|&target| {
                target != lane
                    && game.has_space(target, seat, 0)
                    && (!game.lanes[target].revealed
                        || game.lanes[target]
                            .def
                            .allows_addition
                            .is_none_or(|test| test(game, target, card, target, seat)))
            })
            .collect();
        if !targets.is_empty() {
            let index = game.rng.borrow_mut().randbelow(targets.len());
            game.move_card(card, targets[index]);
        }
    }
}
fn quantum_tunnel(game: &mut Game<'_>, lane: usize, card: usize) {
    let seat = game.cards[card].owner;
    let len = game.players[seat].deck.len();
    if len == 0 {
        return;
    }
    let slot = game.rng.borrow_mut().randbelow(len);
    let replacement = game.players[seat].deck[slot];
    if !game.take_from_board(card) {
        return;
    }
    game.players[seat].deck[slot] = card;
    if !game.add_card(replacement, lane, seat, false) {
        game.players[seat].deck[slot] = replacement;
        game.add_card(card, lane, seat, false);
    }
}
fn sinister_london(game: &mut Game<'_>, lane: usize, card: usize) {
    let seat = game.cards[card].owner;
    let copy = game.copy_card(card, seat);
    let targets: Vec<_> = (0..3)
        .filter(|&target| target != lane && game.can_add(copy, target, seat))
        .collect();
    if targets.is_empty() {
        game.cards.pop();
        return;
    }
    let index = game.rng.borrow_mut().randbelow(targets.len());
    game.add_card(copy, targets[index], seat, true);
}
fn westview(game: &mut Game<'_>, lane: usize) {
    if game.turn != 4 {
        return;
    }
    let candidates: Vec<_> = game
        .location_pool
        .iter()
        .filter(|def| def.id != "Westview" && !game.lanes.iter().any(|lane| lane.def.id == def.id))
        .map(|def| def.id)
        .collect();
    if !candidates.is_empty() {
        let index = game.rng.borrow_mut().randbelow(candidates.len());
        game.replace_location(lane, candidates[index]);
    }
}
fn white_palace(game: &mut Game<'_>, _: usize) {
    let mut sources = [None; 2];
    for (seat, source) in sources.iter_mut().enumerate() {
        let hand = &game.players[1 - seat].hand;
        if game.players[seat].hand.len() >= 7 || hand.is_empty() {
            continue;
        }
        let highest = hand.iter().map(|&card| game.card_cost(card)).max().unwrap();
        let choices: Vec<_> = hand
            .iter()
            .copied()
            .filter(|&card| game.card_cost(card) == highest)
            .collect();
        *source = Some(choices[game.rng.borrow_mut().randbelow(choices.len())]);
    }
    for (seat, source) in sources.into_iter().enumerate() {
        if let Some(source) = source {
            let copy = game.copy_card(source, seat);
            game.add_hand(seat, copy, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::game::Game;
    use crate::position::UNKNOWN_LOCATION;
    use crate::random::PythonRandom;

    #[test]
    fn strange_academy_skips_hidden_cards_without_a_random_roll() {
        let deck: Vec<String> = [
            "Hulk",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
            "Wasp",
            "Cyclops",
            "Shocker",
            "TheThing",
            "MistyKnight",
            "Abomination",
            "Quicksilver",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        let locations = ["StrangeAcademy", "Ruins", "Xandar"].map(str::to_owned);
        let mut rng = PythonRandom::seed(4);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let card = game.create_card("Hawkeye", 0);
        game.add_card(card, 0, 0, false);
        game.cards[card].revealed = false;
        game.turn = 5;
        let mut expected = game.rng.borrow().clone_rng();
        super::strange_academy(&mut game, 0);
        assert_eq!(game.cards[card].lane, Some(0));
        assert_eq!(
            game.rng.borrow_mut().getrandbits(32),
            expected.getrandbits(32)
        );
    }

    #[test]
    fn random_move_destination_filters_legality_and_falls_back() {
        let deck: Vec<String> = [
            "Hulk",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
            "Wasp",
            "Cyclops",
            "Shocker",
            "TheThing",
            "MistyKnight",
            "Abomination",
            "Quicksilver",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        let locations = ["Ruins", "SpaceThrone", "Xandar"].map(str::to_owned);
        let mut rng = PythonRandom::seed(4);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        game.lanes[1].revealed = true;
        let occupant = game.create_card("Hawkeye", 0);
        game.add_card(occupant, 1, 0, false);
        let card = game.create_card("Hawkeye", 0);
        game.add_card(card, 0, 0, false);
        assert_eq!(game.random_location_to_move_card(card, 0, true), Some(2));

        let colossus = game.create_card("Colossus", 0);
        game.add_card(colossus, 0, 0, false);
        let mut expected = game.rng.borrow().clone_rng();
        assert_eq!(game.random_location_to_move_card(colossus, 0, false), None);
        let choice = expected.randbelow(2);
        assert_eq!(
            game.random_location_to_move_card(colossus, 0, true),
            Some([1, 2][choice])
        );
        assert_eq!(
            game.rng.borrow_mut().getrandbits(32),
            expected.getrandbits(32)
        );
    }

    #[test]
    fn zenn_la_reveals_first_printed_three_cost_card() {
        let deck: Vec<String> = [
            "Hawkeye",
            "Wolfsbane",
            "Bishop",
            "Nightcrawler",
            "AntMan",
            "Angela",
            "JessicaJones",
            "KaZar",
            "IronMan",
            "BlueMarvel",
            "Ultron",
            "SquirrelGirl",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        let locations: Vec<String> = ["ZennLa", "Ruins", "Xandar"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        let mut rng = PythonRandom::seed(1);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        game.players[0].deck.clear();
        game.players[1].deck.clear();
        game.lanes[0].def = crate::locations::get("ZennLa").unwrap();
        game.lanes[0].revealed = true;
        game.turn = 2;
        let hawkeye = game.cards.len();
        game.cards
            .push(crate::position::make_card("Hawkeye", 0).unwrap());
        game.add_card(hawkeye, 0, 0, false);
        let nightcrawler = game.cards.len();
        game.cards
            .push(crate::position::make_card("Nightcrawler", 0).unwrap());
        game.cards[nightcrawler].cost = 3;
        let wolfsbane = game.cards.len();
        game.cards
            .push(crate::position::make_card("Wolfsbane", 0).unwrap());
        game.cards[wolfsbane].cost = 2;
        let bishop = game.cards.len();
        game.cards
            .push(crate::position::make_card("Bishop", 0).unwrap());
        game.players[0].deck = vec![nightcrawler, wolfsbane, bishop];

        super::zenn_la(&mut game, 0);

        assert_eq!(game.players[0].deck, vec![nightcrawler, bishop]);
        assert!(game.cards[wolfsbane].revealed);
        assert_eq!(game.cards[wolfsbane].power(), 4);
    }

    #[test]
    fn mirror_dimension_copies_a_location_without_rules() {
        let deck: Vec<String> = ["Hulk", "Medusa", "Sentinel", "IronMan", "Hawkeye", "Wasp"]
            .into_iter()
            .chain([
                "Cyclops",
                "Shocker",
                "TheThing",
                "MistyKnight",
                "Abomination",
                "Quicksilver",
            ])
            .map(str::to_owned)
            .collect();
        let locations: Vec<String> = ["MirrorDimension", "Ruins", "Xandar"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        let mut rng = PythonRandom::seed(1);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        game.lanes[0].def = crate::locations::get("MirrorDimension").unwrap();
        game.lanes[0].revealed = true;
        game.lanes[1].def = &UNKNOWN_LOCATION;
        game.lanes[1].observed_id = Some("Utopia".into());
        game.lanes[1].revealed = true;
        game.lanes[2].revealed = false;
        game.turn = 4;
        super::mirror_dimension(&mut game, 0);
        assert!(std::ptr::eq(game.lanes[0].def, &UNKNOWN_LOCATION));
        assert_eq!(game.lanes[0].observed_id.as_deref(), Some("Utopia"));
        assert!(game.lanes[0].revealed);
    }
}
