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
pub const ROUND_TABLE: LocationDef = LocationDef {
    modify_cost: Some(round_table),
    ..location("RoundTable", 0, 0, 1, false)
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
// RoundTableBehavior::ReduceCardCost (0x7d79a20) checks DefCost, not
// current cost, for both hands and applies AddCost(-1). The definition
// constructor (0x7a70590) supplies the threshold 4 and reduction 1.
fn round_table(game: &Game<'_>, _: usize, card: usize, _: i32) -> Option<CostUpdate> {
    (game.printed_cost(card) >= 4).then_some(CostUpdate::Add(-1))
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
// StarkIslandBehavior::ReduceCardCostInEachHand 0x7d83498 calls ordered
// players at 0x7d83620 (Game::GetOrderedPlayersForResolution 0x7daa454):
// a non-empty hand is copied and shuffled (Game::Shuffle 0x4b12f30), then cards whose Cost is 0
// or that cannot have their cost reduced (predicate 0x7d83e88) are split off as fallback targets.
// When reducible cards remain, Game::Roll(0, n) picks one; otherwise the fallback list is
// stably ordered by descending Cost (0x7d83f10) and Game::Roll picks from all of it, so a roll
// is consumed even though a 0-cost card cannot get cheaper. The reference chose one reducible
// card without the shuffle and without the fallback roll.
fn stark_island(game: &mut Game<'_>, _: usize) {
    if crate::cards::reference_rules_enabled() {
        stark_island_reference(game);
        return;
    }
    let first = game.reveal_priority.unwrap_or(0);
    for seat in [first, 1 - first] {
        let mut hand = game.players[seat].hand.clone();
        if hand.is_empty() {
            continue;
        }
        super::client_shuffle(game, &mut hand);
        let (mut reducible, mut fallback): (Vec<_>, Vec<_>) =
            hand.into_iter().partition(|&id| game.card_cost(id) > 0);
        if !reducible.is_empty() {
            let index = game.rng.borrow_mut().randbelow(reducible.len());
            let card = reducible.swap_remove(index);
            game.set_cost(card, (game.cards[card].cost - 1).max(0), seat);
        } else {
            fallback.sort_by_key(|&id| std::cmp::Reverse(game.card_cost(id)));
            let index = game.rng.borrow_mut().randbelow(fallback.len());
            let card = fallback[index];
            game.set_cost(card, (game.cards[card].cost - 1).max(0), seat);
        }
    }
}
fn stark_island_reference(game: &mut Game<'_>) {
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
// TheIceBoxBehavior::FreezeHands @ 0x7d8a800, per player in resolution order: for a non-empty
// hand, Where(Cost < 6) (lambda 0x7d8b058) then Game::Roll(0, n) picks the card that gets
// AddCostPermanently(+1). When every card costs 6 or more, Zone::GetRandomCard (0x7e06e70 ->
// Game::GetRandomCard 0x7dad84c) rolls over the whole hand instead. The reference picked
// any hand card regardless of cost.
fn ice_box(game: &mut Game<'_>, _: usize) {
    for seat in 0..2 {
        let hand = game.players[seat].hand.clone();
        if hand.is_empty() {
            continue;
        }
        let mut pool: Vec<_> = hand.clone();
        if !crate::cards::reference_rules_enabled() {
            let cheap: Vec<_> = hand
                .iter()
                .copied()
                .filter(|&id| game.card_cost(id) < 6)
                .collect();
            if !cheap.is_empty() {
                pool = cheap;
            }
        }
        let index = game.rng.borrow_mut().randbelow(pool.len());
        game.add_cost(pool[index], 1, seat);
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
    // ZennLaBehavior::RecruitThreeCostCard (0x7d9bcec) calls the ordered
    // player helper at 0x7d9bdd8; predicate 0x7d9c414 uses DefCost.
    let first = game.reveal_priority.unwrap_or(0);
    for seat in [first, 1 - first] {
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
// SakaarBehavior::PlayCards 0x7d7aab8 calls ordered players at 0x7d7ab74.
fn sakaar(game: &mut Game<'_>, lane: usize) {
    if game.lanes[lane].revealed_turn == Some(game.turn) {
        let first = if crate::cards::reference_rules_enabled() {
            0
        } else {
            game.reveal_priority.unwrap_or(0)
        };
        for seat in [first, 1 - first] {
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
    // OttosLabBehavior::PullEnemyCardFromHand (0x7d6f2f4) calls Zone::GetRandomCard
    // (0x7e06e70) on the opponent's hand and then Location::StageForPlay without a prior
    // space test, so a full side still consumes the roll. The reference skipped the roll.
    if !crate::cards::reference_rules_enabled() || game.has_space(lane, opponent, 0) {
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
// QuantumTunnelBehavior::SwapCard @ 0x7d76a84: Zone::GetRandomCard (0x7e06e70) rolls over the
// deck, Game::MoveCardToZone moves the played card into the deck and the picked card onto the
// board, and a successful swap ends with Deck::Shuffle (0x7cc0ac0 -> Game::Shuffle
// 0x4b12f30). The reference left the played card in the picked card's deck slot.
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
    } else if !crate::cards::reference_rules_enabled() {
        game.shuffle_deck(seat);
    }
}
// SinisterLondonBehavior::CreateCopy @ 0x7d7fe8c calls Game::GetRandomLocationToSpawnCard
// (0x7db2b64) with fallbackToAnyLocation = true, then Game::CreateCopy there. With no other
// location that can take the copy it still rolls over every other location and the copy fails
// to enter. The reference returned without rolling.
fn sinister_london(game: &mut Game<'_>, lane: usize, card: usize) {
    let seat = game.cards[card].owner;
    let copy = game.copy_card(card, seat);
    if crate::cards::reference_rules_enabled() {
        let targets: Vec<_> = (0..3)
            .filter(|&target| target != lane && game.can_add(copy, target, seat))
            .collect();
        if targets.is_empty() {
            game.cards.pop();
            return;
        }
        let index = game.rng.borrow_mut().randbelow(targets.len());
        game.add_card(copy, targets[index], seat, true);
        return;
    }
    let target = game.random_location_to_spawn_card(copy, seat, Some(lane), true);
    if !target.is_some_and(|target| game.add_card(copy, target, seat, true)) {
        game.cards.pop();
    }
}
// WestviewBehavior::Morph @ 0x7d97e70 calls Game::GetRandomLocationDefId(this location)
// (0x7db248c): one rarity-weighted roll over the location pool that skips only this
// location's own definition (Game::random_location_def_id); locations already on the board
// stay eligible. The reference drew uniformly among definitions not on the board.
fn westview(game: &mut Game<'_>, lane: usize) {
    if game.turn != 4 {
        return;
    }
    if !crate::cards::reference_rules_enabled() {
        if let Some(id) = game.random_location_def_id(Some(game.lanes[lane].def.id)) {
            game.replace_location(lane, id);
        }
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
// WhitePalaceBehavior::AddHighestCostCard @ 0x7d98d98 orders both hands by descending Cost up
// front; g__CopyEnemyHighestCostCard (0x7d990c8) takes the leading equal-Cost run (TakeWhile),
// rolls Game::Roll(0, n) and calls Game::CreateCopy into the owner's hand. No hand-size test
// precedes the roll, so an owner with a full hand still consumes it. The reference skipped
// the roll when the owner's hand was full.
fn white_palace(game: &mut Game<'_>, _: usize) {
    let mut sources = [None; 2];
    for (seat, source) in sources.iter_mut().enumerate() {
        let hand = &game.players[1 - seat].hand;
        let full = game.players[seat].hand.len() >= 7;
        if hand.is_empty() || (full && crate::cards::reference_rules_enabled()) {
            continue;
        }
        let highest = hand.iter().map(|&card| game.card_cost(card)).max().unwrap();
        let choices: Vec<_> = hand
            .iter()
            .copied()
            .filter(|&card| game.card_cost(card) == highest)
            .collect();
        let picked = choices[game.rng.borrow_mut().randbelow(choices.len())];
        if !full {
            *source = Some(picked);
        }
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

    fn rng_game(rng: &mut PythonRandom) -> Game<'_> {
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
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        Game::new([&deck, &deck], &locations, rng).unwrap()
    }
    #[test]
    fn round_table_uses_printed_cost_for_both_hands_and_stacks() {
        let mut rng = PythonRandom::seed(1);
        let mut game = rng_game(&mut rng);
        for lane in &mut game.lanes {
            lane.def = crate::locations::get("Ruins").unwrap();
            lane.revealed = true;
        }
        game.lanes[0].def = crate::locations::get("RoundTable").unwrap();
        game.lanes[1].def = crate::locations::get("Elysium").unwrap();
        for player in &mut game.players {
            player.hand.clear();
            player.deck.clear();
        }
        let four = game.create_card("TheThing", 0);
        let three = game.create_card("Cyclops", 0);
        let other = game.create_card("Hulk", 1);
        let deck = game.create_card("Hulk", 0);
        game.cards[four].cost = 1;
        game.cards[three].cost = 5;
        game.players[0].hand.extend([four, three]);
        game.players[1].hand.push(other);
        game.players[0].deck.push(deck);
        game.ongoing();
        assert_eq!(game.card_cost(four), 0);
        assert_eq!(game.card_cost(three), 4);
        assert_eq!(game.card_cost(other), 4);
        assert_eq!(game.card_cost(deck), 6);
        game.lanes[0].revealed = false;
        game.ongoing();
        assert_eq!(game.card_cost(other), 5);
    }

    // StarkIslandBehavior::ReduceCardCostInEachHand (0x7d83498) shuffles a non-empty hand
    // (n - 1 rolls) before its pick, so a 2-card hand consumes two rolls per player.
    #[test]
    fn stark_island_shuffles_hand_before_picking() {
        let mut rng = PythonRandom::seed(3);
        let mut game = rng_game(&mut rng);
        for seat in 0..2 {
            game.players[seat].hand.truncate(2);
            while game.players[seat].hand.len() < 2 {
                let card = game.create_card("Hulk", seat);
                game.add_hand(seat, card, false);
            }
            for &card in &game.players[seat].hand.clone() {
                game.cards[card].cost = 3;
            }
        }
        let mut expected = game.rng.borrow().clone_rng();
        for _ in 0..2 {
            expected.randbelow(2);
            expected.randbelow(2);
        }
        super::stark_island(&mut game, 0);
        assert_eq!(
            game.rng.borrow_mut().getrandbits(32),
            expected.getrandbits(32)
        );
    }

    #[test]
    fn stark_island_picks_in_initiative_order() {
        let mut rng = PythonRandom::seed(31);
        let mut game = rng_game(&mut rng);
        game.reveal_priority = Some(1);
        for seat in 0..2 {
            game.players[seat].hand.clear();
            for _ in 0..3 {
                let card = game.create_card("Hulk", seat);
                game.cards[card].cost = 3;
                game.add_hand(seat, card, false);
            }
        }
        let mut expected = game.rng.borrow().clone_rng();
        let mut picked = [0; 2];
        for seat in [1, 0] {
            let mut hand = game.players[seat].hand.clone();
            for i in 0..hand.len() - 1 {
                let j = i + expected.randbelow(hand.len() - i);
                hand.swap(i, j);
            }
            picked[seat] = hand[expected.randbelow(hand.len())];
        }
        super::stark_island(&mut game, 0);
        for card in picked {
            assert_eq!(game.card_cost(card), 2);
        }
    }

    #[test]
    fn zenn_la_recruits_in_initiative_order() {
        let mut rng = PythonRandom::seed(27);
        let mut game = rng_game(&mut rng);
        game.reveal_priority = Some(1);
        for seat in 0..2 {
            game.players[seat].deck.clear();
            let card = game.create_card("Bishop", seat);
            game.push_deck(seat, card);
        }
        super::zenn_la(&mut game, 0);
        assert_eq!(game.cards[game.lanes[0].cards[0]].owner, 1);
        assert_eq!(game.cards[game.lanes[0].cards[1]].owner, 0);
    }
    #[test]
    fn sakaar_rolls_hands_in_initiative_order() {
        let mut rng = PythonRandom::seed(27);
        let mut game = rng_game(&mut rng);
        game.reveal_priority = Some(1);
        game.lanes[0].revealed_turn = Some(game.turn);
        for seat in 0..2 {
            game.players[seat].hand.clear();
            for _ in 0..3 {
                let card = game.create_card("Wasp", seat);
                game.add_hand(seat, card, false);
            }
        }
        let mut control = game.rng.borrow().clone_rng();
        let selected = [
            game.players[1].hand[control.randbelow(3)],
            game.players[0].hand[control.randbelow(3)],
        ];
        super::sakaar(&mut game, 0);
        assert!(selected.iter().all(|id| game.lanes[0].cards.contains(id)));
    }
    // TheIceBoxBehavior::FreezeHands (0x7d8a800) only picks among cards with Cost < 6.
    #[test]
    fn ice_box_prefers_cards_costing_under_six() {
        let mut rng = PythonRandom::seed(5);
        let mut game = rng_game(&mut rng);
        game.players[0].hand.clear();
        game.players[1].hand.clear();
        let big = game.create_card("Hulk", 0);
        game.cards[big].cost = 6;
        game.add_hand(0, big, false);
        let small = game.create_card("Hulk", 0);
        game.cards[small].cost = 2;
        game.add_hand(0, small, false);
        for _ in 0..5 {
            let before = game.card_cost(small);
            super::ice_box(&mut game, 0);
            assert_eq!(game.card_cost(small), before + 1);
            game.cards[small].cost = 2;
        }
        assert_eq!(game.card_cost(big), 6);
    }

    // PullEnemyCardFromHand (0x7d6f2f4) rolls without a prior space test.
    #[test]
    fn ottos_lab_rolls_even_when_the_enemy_side_is_full() {
        use crate::locations::test_support::{assert_same_stream, fill, set_hand};
        let mut rng = PythonRandom::seed(6);
        let mut game = super::super::test_support::rng_game(&mut rng);
        fill(&mut game, 1, 0, 4);
        set_hand(&mut game, 1, 3);
        let played = fill(&mut game, 0, 1, 1)[0];
        game.lanes[0].state = Some(Box::new([false; 2]));
        let mut expected = game.rng.borrow().clone_rng();
        expected.randbelow(3);
        super::ottos_lab(&mut game, 0, played);
        assert_eq!(game.players[1].hand.len(), 3);
        assert_same_stream(&game, &mut *expected);
    }

    // AddHighestCostCard (0x7d98d98): an owner with a full hand still consumes its roll.
    #[test]
    fn white_palace_rolls_when_own_hand_is_full() {
        use crate::locations::test_support::{assert_same_stream, set_hand};
        let mut rng = PythonRandom::seed(7);
        let mut game = super::super::test_support::rng_game(&mut rng);
        set_hand(&mut game, 0, 7);
        set_hand(&mut game, 1, 2);
        let mut expected = game.rng.borrow().clone_rng();
        expected.randbelow(2);
        expected.randbelow(7);
        super::white_palace(&mut game, 0);
        assert_eq!(game.players[0].hand.len(), 7);
        assert_eq!(game.players[1].hand.len(), 3);
        assert_same_stream(&game, &mut *expected);
    }

    // CreateCopy (0x7d7fe8c) uses GetRandomLocationToSpawnCard with fallbackToAnyLocation.
    #[test]
    fn sinister_london_rolls_when_no_other_location_can_take_the_copy() {
        use crate::locations::test_support::{assert_same_stream, fill};
        let mut rng = PythonRandom::seed(8);
        let mut game = super::super::test_support::rng_game(&mut rng);
        let card = fill(&mut game, 0, 0, 1)[0];
        fill(&mut game, 0, 1, 4);
        fill(&mut game, 0, 2, 4);
        let count = game.cards.len();
        let mut expected = game.rng.borrow().clone_rng();
        expected.randbelow(2);
        super::sinister_london(&mut game, 0, card);
        assert_eq!(game.cards.len(), count);
        assert_same_stream(&game, &mut *expected);
    }

    // Horde creation (LocationCreateNewZombie 0x7e0feac) rolls over the other locations
    // even when none can take the new Horde.
    #[test]
    fn horde_rolls_when_no_location_can_take_the_new_horde() {
        use crate::locations::test_support::{assert_same_stream, fill};
        let mut rng = PythonRandom::seed(9);
        let mut game = super::super::test_support::rng_game(&mut rng);
        fill(&mut game, 0, 1, 4);
        fill(&mut game, 0, 2, 4);
        let mut expected = game.rng.borrow().clone_rng();
        expected.randbelow(2);
        assert_eq!(game.horde(0, 0, 1), None);
        assert_same_stream(&game, &mut *expected);
    }

    // Morph (0x7d97e70): GetRandomLocationDefId(this) is one rarity-weighted roll that skips
    // only Westview's own definition; definitions already on the board stay eligible.
    #[test]
    fn westview_uses_the_weighted_location_pick() {
        use crate::locations::test_support::{assert_same_stream, rng_game_in};
        const POOL: [&str; 6] = ["Ruins", "Xandar", "Wakanda", "Atlantis", "Asgard", "Kyln"];
        let mut rng = PythonRandom::seed(15);
        let mut game = rng_game_in(&mut rng, &POOL);
        let mut oracle_rng = PythonRandom::seed(15);
        let oracle = rng_game_in(&mut oracle_rng, &POOL);
        let expected_id = oracle.random_location_def_id(Some("Westview")).unwrap();
        game.lanes[0].def = crate::locations::get("Westview").unwrap();
        game.turn = 4;
        let mut expected = oracle.rng.borrow().clone_rng();
        super::westview(&mut game, 0);
        assert_eq!(game.lanes[0].def.id, expected_id);
        assert_same_stream(&game, &mut *expected);
    }
}
