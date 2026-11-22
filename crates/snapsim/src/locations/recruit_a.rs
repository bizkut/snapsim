use super::{LocationDef, location};
use crate::game::Game;

pub const ASGARD: LocationDef = LocationDef {
    on_turn_end: Some(asgard),
    ..location("Asgard", 0, 0, 1, false)
};
pub const ATLANTIS: LocationDef = LocationDef {
    ongoing: Some(atlantis),
    ..location("Atlantis", 0, 0, 1, false)
};
pub const AVENGERS_COMPOUND: LocationDef = LocationDef {
    allows_play_elsewhere: Some(not_turn_five),
    ..location("AvengersCompound", 0, 0, 1, false)
};
pub const BAXTER_BUILDING: LocationDef = LocationDef {
    ongoing: Some(baxter),
    ..location("BaxterBuilding", 0, 0, 1, false)
};
pub const CASTLE_BLACKSTONE: LocationDef = LocationDef {
    energy_bonus: Some(castle),
    ..location("CastleBlackstone", 0, 0, 1, false)
};
pub const CENTRAL_PARK: LocationDef = LocationDef {
    on_reveal: Some(central_park),
    ..location("CentralPark", 0, 0, 1, false)
};
pub const CLONING_VATS: LocationDef = LocationDef {
    after_card_play: Some(cloning_vats),
    ..location("CloningVats", 0, 0, 1, false)
};
pub const BAR_SINISTER: LocationDef = LocationDef {
    after_card_play: Some(bar_sinister),
    ..location("BarSinister", 0, 0, 1, false)
};
pub const DAILY_BUGLE: LocationDef = LocationDef {
    on_reveal: Some(daily_bugle),
    ..location("DailyBugle", 0, 0, 1, false)
};
pub const MACHINEWORLD: LocationDef = LocationDef {
    after_card_play: Some(machineworld),
    ..location("Machineworld", 0, 0, 1, false)
};
pub const CRIMSON_COSMOS: LocationDef = LocationDef {
    allows_play: Some(crimson),
    ..location("CrimsonCosmos", 0, 0, 1, false)
};
pub const DANGER_ROOM: LocationDef = LocationDef {
    after_card_play: Some(danger),
    ..location("DangerRoom", 0, 0, 1, false)
};
pub const EBONSHIRE: LocationDef = LocationDef {
    on_reveal: Some(ebonshire),
    ..location("Ebonshire", 0, 0, 1, false)
};
pub const ETERNITY_RANGE: LocationDef = LocationDef {
    on_turn_end: Some(eternity),
    ..location("EternityRange", 0, 0, 1, false)
};
pub const FISK_TOWER: LocationDef = LocationDef {
    on_card_moved: Some(fisk),
    ..location("FiskTower", 0, 0, 1, false)
};
pub const FLOODED: LocationDef = LocationDef {
    allows_play: Some(flooded),
    ..location("Flooded", 0, 0, 1, false)
};
pub const FOGWELLS_GYM: LocationDef = location("FogwellsGym", 0, 0, 1, false);
pub const FRONTIER_OUTPOST: LocationDef = LocationDef {
    on_reveal: Some(frontier),
    ..location("FrontierOutpost", 0, 0, 1, false)
};
pub const GENOSHA: LocationDef = LocationDef {
    on_turn_end: Some(genosha),
    ..location("Genosha", 0, 0, 1, false)
};
pub const GRAND_CENTRAL: LocationDef = LocationDef {
    on_turn_end: Some(grand_central),
    ..location("GrandCentral", 0, 0, 1, false)
};
pub const HELLFIRE_CLUB: LocationDef = LocationDef {
    allows_play: Some(hellfire),
    ..location("HellfireClub", 0, 0, 1, false)
};
pub const HELLS_KITCHEN: LocationDef = LocationDef {
    on_reveal: Some(kitchen),
    ..location("HellsKitchen", 0, 0, 1, false)
};
pub const KUN_LUN: LocationDef = LocationDef {
    on_card_moved: Some(kun_lun),
    ..location("KunLun", 0, 0, 1, false)
};
pub const KYLN: LocationDef = LocationDef {
    allows_play: Some(kyln),
    ..location("Kyln", 0, 0, 1, false)
};
pub const LAKE_HELLAS: LocationDef = LocationDef {
    ongoing: Some(lake),
    ..location("LakeHellas", 0, 0, 1, false)
};
pub const MADRIPOOR: LocationDef = LocationDef {
    on_turn_end: Some(madripoor),
    ..location("Madripoor", 0, 0, 1, false)
};
pub const MINIATURIZED_LAB: LocationDef = LocationDef {
    allows_addition: Some(lab),
    ..location("MiniaturizedLab", 0, 0, 1, false)
};
pub const MINDSCAPE: LocationDef = LocationDef {
    on_turn_start: Some(mindscape),
    ..location("Mindscape", 0, 0, 1, false)
};
pub const DREAM_DIMENSION: LocationDef = LocationDef {
    modify_cost: Some(dream),
    ..location("DreamDimension", 0, 0, 1, false)
};
pub const ELYSIUM: LocationDef = LocationDef {
    modify_cost: Some(elysium),
    ..location("Elysium", 0, 0, 1, false)
};
pub const FLOODING: LocationDef = LocationDef {
    on_turn_end: Some(flooding),
    ..location("Flooding", 0, 0, 1, false)
};
pub const LOS_DIABLOS: LocationDef = LocationDef {
    on_turn_end: Some(los_diablos),
    ..location("LosDiablos", 0, 0, 1, false)
};

fn leader(game: &Game<'_>, lane: usize) -> Option<usize> {
    game.lane_winner(lane)
}
fn draw_slot(game: &mut Game<'_>, seat: usize, slot: Option<usize>) {
    if game.players[seat].hand.len() < 7
        && let Some(slot) = slot
    {
        let card = game.players[seat].deck.remove(slot);
        game.add_hand(seat, card, true);
    }
}
fn asgard(game: &mut Game<'_>, lane: usize) {
    if game.turn == 4
        && let Some(seat) = leader(game, lane)
    {
        for _ in 0..2 {
            let slot = game.players[seat].deck.iter().position(|&id| {
                game.cards[id]
                    .def
                    .draw_turn
                    .is_none_or(|turn| turn <= game.turn)
            });
            draw_slot(game, seat, slot);
        }
    }
}
fn atlantis(game: &mut Game<'_>, lane: usize) {
    for seat in 0..2 {
        let mut cards = game.lanes[lane]
            .cards
            .iter()
            .copied()
            .filter(|&id| game.cards[id].owner == seat);
        if let Some(id) = cards.next()
            && cards.next().is_none()
            && game.cards[id].revealed
        {
            game.add_location_power(id, 5);
        }
    }
}
fn baxter(game: &mut Game<'_>, lane: usize) {
    if let Some(seat) = leader(game, lane) {
        for other in 0..3 {
            if other != lane {
                game.lanes[other].bonus[seat] += 4;
            }
        }
    }
}
fn castle(game: &Game<'_>, lane: usize, seat: usize) -> i32 {
    i32::from(leader(game, lane) == Some(seat))
}
fn central_park(game: &mut Game<'_>, _: usize) {
    for lane in 0..3 {
        for seat in 0..2 {
            game.add_created("Squirrel", lane, seat);
        }
    }
}
fn crimson(game: &Game<'_>, _: usize, card: usize, _: usize, _: usize) -> bool {
    game.card_cost(card) > 3
}
fn danger(game: &mut Game<'_>, _: usize, card: usize) {
    let destroy = game.rng.borrow_mut().random() < 0.25;
    if destroy {
        game.destroy_card(card);
    }
}
fn ebonshire(game: &mut Game<'_>, _: usize) {
    for seat in 0..2 {
        let slot = game.players[seat].deck.iter().rposition(|&id| {
            game.cards[id]
                .def
                .draw_turn
                .is_none_or(|turn| turn <= game.turn)
        });
        draw_slot(game, seat, slot);
    }
}
fn eternity(game: &mut Game<'_>, lane: usize) {
    if game.turn == 3
        && let Some(seat) = leader(game, lane)
    {
        game.add_created("Rock", lane, 1 - seat);
    }
}
fn fisk(game: &mut Game<'_>, _: usize, card: usize, _: usize) {
    game.add_power(card, -4, game.cards[card].owner);
}
fn flooded(_: &Game<'_>, _: usize, _: usize, _: usize, _: usize) -> bool {
    false
}
fn matching(game: &mut Game<'_>, cost: i32) {
    for seat in 0..2 {
        let slot = game.players[seat]
            .deck
            .iter()
            .position(|&id| game.cards[id].cost == cost);
        if let Some(slot) = slot
            && game.cards[game.players[seat].deck[slot]]
                .def
                .draw_turn
                .is_none_or(|turn| turn <= game.turn)
        {
            draw_slot(game, seat, Some(slot));
        }
    }
}
fn frontier(game: &mut Game<'_>, _: usize) {
    matching(game, 3);
}
fn kitchen(game: &mut Game<'_>, _: usize) {
    matching(game, 1);
}
fn genosha(game: &mut Game<'_>, lane: usize) {
    if game.turn != 5 {
        return;
    }
    for seat in 0..2 {
        let cards: Vec<_> = game.lanes[lane]
            .cards
            .iter()
            .copied()
            .filter(|&id| game.cards[id].owner == seat)
            .collect();
        if let Some(highest) = cards.iter().map(|&id| game.cards[id].cost).max() {
            let tied: Vec<_> = cards
                .iter()
                .copied()
                .filter(|&id| game.cards[id].cost == highest)
                .collect();
            let survivor = tied[game.rng.borrow_mut().randbelow(tied.len())];
            for id in cards {
                if id != survivor {
                    game.destroy_card(id);
                }
            }
        }
    }
}
fn hellfire(game: &Game<'_>, _: usize, card: usize, _: usize, _: usize) -> bool {
    game.card_cost(card) != 1
}
fn kun_lun(game: &mut Game<'_>, _: usize, card: usize, _: usize) {
    game.add_power(card, 2, game.cards[card].owner);
}
fn kyln(game: &Game<'_>, _: usize, _: usize, _: usize, _: usize) -> bool {
    game.turn <= 4
}
fn lab(game: &Game<'_>, _: usize, _: usize, _: usize, _: usize) -> bool {
    !(3..=5).contains(&game.turn)
}
fn lake(game: &mut Game<'_>, lane: usize) {
    for index in 0..game.lanes[lane].cards.len() {
        let id = game.lanes[lane].cards[index];
        if game.cards[id].revealed && game.cards[id].cost == 1 {
            game.add_location_power(id, 2);
        }
    }
}
fn madripoor(game: &mut Game<'_>, lane: usize) {
    if let Some(highest) = game.lanes[lane]
        .cards
        .iter()
        .map(|&id| game.cards[id].cost)
        .max()
    {
        for index in 0..game.lanes[lane].cards.len() {
            let id = game.lanes[lane].cards[index];
            if game.cards[id].cost == highest {
                game.add_power(id, 2, game.cards[id].owner);
            }
        }
    }
}
fn not_turn_five(game: &Game<'_>, _: usize, _: usize, _: usize, _: usize) -> bool {
    game.turn != 5
}
fn dream(game: &Game<'_>, _: usize, _: usize, cost: i32) -> i32 {
    cost + i32::from(game.turn == 5)
}
fn elysium(_: &Game<'_>, _: usize, _: usize, cost: i32) -> i32 {
    cost - 1
}
// FloodingBehavior::MorphToFlooded (0x7d53b14) waits until after the reveal
// turn; the frozen reference morphs immediately.
fn flooding(game: &mut Game<'_>, lane: usize) {
    if crate::cards::reference_rules_enabled() || game.lanes[lane].revealed_turn != Some(game.turn)
    {
        game.replace_location(lane, "Flooded");
    }
}
fn los_diablos(game: &mut Game<'_>, _: usize) {
    if game.turn == 3 {
        let target = game.rng.borrow_mut().randbelow(3);
        game.replace_location(target, "Ruins");
    }
}
fn grand_central(game: &mut Game<'_>, lane: usize) {
    if game.turn != 5 {
        return;
    }
    for seat in 0..2 {
        let len = game.players[seat].hand.len();
        if len == 0 {
            continue;
        }
        let slot = game.rng.borrow_mut().randbelow(len);
        let card = game.players[seat].hand[slot];
        if game.can_add(card, lane, seat) {
            game.players[seat].hand.remove(slot);
            game.add_card(card, lane, seat, false);
        }
    }
}
fn copy_to_hand(game: &mut Game<'_>, card: usize, seat: usize) {
    if game.players[seat].hand.len() < 7 {
        let copy = game.copy_card(card, seat);
        game.add_hand(seat, copy, false);
    }
}
fn cloning_vats(game: &mut Game<'_>, _: usize, card: usize) {
    copy_to_hand(game, card, game.cards[card].owner);
}
fn machineworld(game: &mut Game<'_>, _: usize, card: usize) {
    copy_to_hand(game, card, 1 - game.cards[card].owner);
}
fn bar_sinister(game: &mut Game<'_>, lane: usize, card: usize) {
    let seat = game.cards[card].owner;
    while game.has_space(lane, seat, 0) {
        let copy = game.copy_card(card, seat);
        if !game.add_card(copy, lane, seat, false) {
            break;
        }
    }
}
fn daily_bugle(game: &mut Game<'_>, _: usize) {
    let mut sources = [None; 2];
    for (seat, source) in sources.iter_mut().enumerate() {
        let hand = &game.players[1 - seat].hand;
        if !hand.is_empty() && game.players[seat].hand.len() < 7 {
            *source = Some(hand[game.rng.borrow_mut().randbelow(hand.len())]);
        }
    }
    for (seat, source) in sources.into_iter().enumerate() {
        if let Some(card) = source {
            copy_to_hand(game, card, seat);
        }
    }
}

fn mindscape(game: &mut Game<'_>, _: usize) {
    if game.turn == 6 {
        game.swap_hands();
    }
}
