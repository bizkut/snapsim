use super::{LocationDef, location};
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

fn titan(game: &Game<'_>, _: usize, card: usize, cost: i32) -> i32 {
    cost - i32::from(game.cards[card].cost == 6)
}
fn sandbar(game: &Game<'_>, _: usize, card: usize, cost: i32) -> i32 {
    cost + i32::from(game.cards[card].created && cost < 6)
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
        let p = game.powers()[lane];
        for seat in 0..2 {
            if p[seat] > p[1 - seat] {
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
            .map(|i| game.lanes[i].def.id)
            .collect();
        if !choices.is_empty() {
            let index = game.rng.borrow_mut().randbelow(choices.len());
            game.replace_location(lane, choices[index]);
        }
    }
}
fn zenn_la(game: &mut Game<'_>, lane: usize) {
    for seat in 0..2 {
        let slot = game.players[seat]
            .deck
            .iter()
            .position(|&id| game.card_cost(id) == 3);
        if let Some(slot) = slot {
            let id = game.players[seat].deck[slot];
            if game.add_card(id, lane, seat, false) {
                game.players[seat].deck.remove(slot);
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
            game.players[seat].hand.remove(slot);
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
