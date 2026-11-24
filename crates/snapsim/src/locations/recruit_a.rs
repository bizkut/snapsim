use super::{CostUpdate, LocationDef, location};
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
    conditional_ongoing: Some(baxter),
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
        let card = game.take_deck(seat, slot);
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
// ActionReturns2: Baxter's action succeeds even when there is no winner.
fn baxter(game: &mut Game<'_>, lane: usize) -> bool {
    if let Some(seat) = leader(game, lane) {
        for other in 0..3 {
            if other != lane {
                game.add_lane_bonus(other, seat, 4);
            }
        }
    }
    true
}
fn castle(game: &Game<'_>, lane: usize, seat: usize) -> i32 {
    i32::from(leader(game, lane) == Some(seat))
}
// CentralParkBehavior::AddSquirrel 0x7d471c4 calls ordered players at
// 0x7d472d0 and enumerates locations inside each player's iteration.
fn central_park(game: &mut Game<'_>, _: usize) {
    if crate::cards::reference_rules_enabled() {
        for lane in 0..3 {
            for seat in 0..2 {
                game.add_created("Squirrel", lane, seat);
            }
        }
    } else {
        let first = game.reveal_priority.unwrap_or(0);
        for seat in [first, 1 - first] {
            for lane in 0..3 {
                game.add_created("Squirrel", lane, seat);
            }
        }
    }
}
fn crimson(game: &Game<'_>, _: usize, card: usize, _: usize, _: usize) -> bool {
    game.card_cost(card) > 3
}
// DangerRoomBehavior::DestroyCard @ 0x7d4d318: Game::Roll(0, 100) below PercentChance (25)
// destroys the card. The reference drew a CPython float below 0.25, the same probability
// from a different RNG consumption.
fn danger(game: &mut Game<'_>, _: usize, card: usize) {
    let destroy = if crate::cards::reference_rules_enabled() {
        game.rng.borrow_mut().random() < 0.25
    } else {
        game.rng.borrow_mut().randbelow(100) < 25
    };
    if destroy {
        game.destroy_card(card);
    }
}
// EbonshireBehavior::DrawFromBottom 0x7d50098 calls ordered players at
// 0x7d50144 before drawing each player's card.
fn ebonshire(game: &mut Game<'_>, _: usize) {
    let first = if crate::cards::reference_rules_enabled() {
        0
    } else {
        game.reveal_priority.unwrap_or(0)
    };
    for seat in [first, 1 - first] {
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
// GenoshaBehavior::DestroySmallCards @ 0x7d55abc takes Location::GetAllRevealedCards for each
// player; with at least two it orders them by descending DefCost (key 0x7d564a4, the printed
// cost), reads the first, and destroys every card whose DefCost differs. Equal-cost cards all
// survive and no random roll is made. The reference kept one random card of the highest
// current cost and destroyed the rest, hidden cards included.
fn genosha(game: &mut Game<'_>, lane: usize) {
    if game.turn != 5 {
        return;
    }
    if crate::cards::reference_rules_enabled() {
        genosha_reference(game, lane);
        return;
    }
    for seat in 0..2 {
        let cards: Vec<_> = game.lanes[lane]
            .cards
            .iter()
            .copied()
            .filter(|&id| game.cards[id].owner == seat && game.cards[id].revealed)
            .collect();
        if cards.len() < 2 {
            continue;
        }
        let highest = cards.iter().map(|&id| game.printed_cost(id)).max().unwrap();
        for id in cards {
            if game.printed_cost(id) != highest {
                game.destroy_card(id);
            }
        }
    }
}
fn genosha_reference(game: &mut Game<'_>, lane: usize) {
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
fn dream(game: &Game<'_>, _: usize, _: usize, _: i32) -> Option<CostUpdate> {
    (game.turn == 5).then_some(CostUpdate::Add(1))
}
fn elysium(_: &Game<'_>, _: usize, _: usize, _: i32) -> Option<CostUpdate> {
    Some(CostUpdate::Add(-1))
}
// FloodingBehavior::MorphToFlooded (0x7d53b14) waits until after the reveal
// turn; the frozen reference morphs immediately.
fn flooding(game: &mut Game<'_>, lane: usize) {
    if crate::cards::reference_rules_enabled() || game.lanes[lane].revealed_turn != Some(game.turn)
    {
        game.replace_location(lane, "Flooded");
    }
}
// LosDiablosBehavior::SpawnRuins @ 0x7d63384 rolls Game::Roll(0, n) over Game::GetLocations
// filtered by `location != this` (predicate 0x7d6396c) before Location::MorphIntoNewLocation,
// so only the other two locations can be ruined. The reference chose among all three.
fn los_diablos(game: &mut Game<'_>, lane: usize) {
    if game.turn == 3 {
        let target = if crate::cards::reference_rules_enabled() {
            game.rng.borrow_mut().randbelow(3)
        } else {
            let others: Vec<_> = (0..3).filter(|&other| other != lane).collect();
            others[game.rng.borrow_mut().randbelow(others.len())]
        };
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
            game.take_hand(seat, slot);
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
// DailyBugleBehavior::AddRandomCard @ 0x7d4cf74 calls Zone::GetRandomCard (0x7e06e70) on each
// player's opponent hand and only then Game::CreateCopy into each hand; there is no earlier
// hand-size test, so a full hand still consumes its roll. The reference skipped that roll.
fn daily_bugle(game: &mut Game<'_>, _: usize) {
    let mut sources = [None; 2];
    for (seat, source) in sources.iter_mut().enumerate() {
        let hand = &game.players[1 - seat].hand;
        let full = game.players[seat].hand.len() >= 7;
        if !hand.is_empty() && !(full && crate::cards::reference_rules_enabled()) {
            let card = hand[game.rng.borrow_mut().randbelow(hand.len())];
            if !full {
                *source = Some(card);
            }
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

#[cfg(test)]
mod tests {
    use crate::game::Game;
    use crate::random::PythonRandom;

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
        let locations = ["Genosha", "LosDiablos", "Xandar"].map(str::to_owned);
        Game::new([&deck, &deck], &locations, rng).unwrap()
    }

    // GenoshaBehavior::DestroySmallCards (0x7d55abc): equal highest DefCost cards all
    // survive and no roll is made.
    #[test]
    fn genosha_keeps_every_card_of_the_highest_printed_cost() {
        let mut rng = PythonRandom::seed(2);
        let mut game = rng_game(&mut rng);
        let a = game.create_card("Hulk", 0);
        let b = game.create_card("Hulk", 0);
        let c = game.create_card("Wasp", 0);
        for id in [a, b, c] {
            assert!(game.add_card(id, 0, 0, false));
            game.cards[id].revealed = true;
        }
        game.turn = 5;
        let mut expected = game.rng.borrow().clone_rng();
        super::genosha(&mut game, 0);
        assert_eq!(game.cards[a].lane, Some(0));
        assert_eq!(game.cards[b].lane, Some(0));
        assert_ne!(game.cards[c].lane, Some(0));
        assert_eq!(
            game.rng.borrow_mut().getrandbits(32),
            expected.getrandbits(32)
        );
    }

    // LosDiablosBehavior::SpawnRuins (0x7d63384) never ruins itself.
    #[test]
    fn los_diablos_ruins_only_other_locations() {
        for seed in 0..20 {
            let mut rng = PythonRandom::seed(seed);
            let mut game = rng_game(&mut rng);
            game.turn = 3;
            game.lanes[1].def = crate::locations::get("LosDiablos").unwrap();
            super::los_diablos(&mut game, 1);
            assert_eq!(game.lanes[1].def.id, "LosDiablos");
        }
    }

    // AddRandomCard (0x7d4cf74): a full hand still consumes its roll.
    #[test]
    fn daily_bugle_rolls_when_own_hand_is_full() {
        use crate::locations::test_support::{assert_same_stream, rng_game, set_hand};
        let mut rng = PythonRandom::seed(10);
        let mut game = rng_game(&mut rng);
        set_hand(&mut game, 0, 7);
        set_hand(&mut game, 1, 2);
        let mut expected = game.rng.borrow().clone_rng();
        expected.randbelow(2);
        expected.randbelow(7);
        super::daily_bugle(&mut game, 0);
        assert_eq!(game.players[0].hand.len(), 7);
        assert_eq!(game.players[1].hand.len(), 3);
        assert_same_stream(&game, &mut *expected);
    }

    // DestroyCard (0x7d4d318): Game::Roll(0, 100) below 25 destroys the card.
    #[test]
    fn danger_room_rolls_zero_to_ninety_nine_against_twenty_five() {
        use crate::locations::test_support::{assert_same_stream, fill, rng_game};
        for seed in 0..12 {
            let mut rng = PythonRandom::seed(seed);
            let mut game = rng_game(&mut rng);
            let card = fill(&mut game, 0, 0, 1)[0];
            let mut expected = game.rng.borrow().clone_rng();
            let roll = expected.randbelow(100);
            super::danger(&mut game, 0, card);
            assert_eq!(game.cards[card].lane.is_none(), roll < 25);
            assert_same_stream(&game, &mut *expected);
        }
    }
    #[test]
    fn central_park_creates_all_initiative_squirrels_first() {
        let mut rng = PythonRandom::seed(11);
        let mut game = rng_game(&mut rng);
        game.reveal_priority = Some(1);
        super::central_park(&mut game, 0);
        let one: [usize; 3] = std::array::from_fn(|lane| game.lanes[lane].cards[0]);
        assert!(one.iter().all(|&id| game.cards[id].owner == 1));
        assert!(one[0] < one[1] && one[1] < one[2]);
        assert!(one[2] < game.lanes[0].cards[1]);
    }

    #[test]
    fn ebonshire_draws_in_initiative_order() {
        use crate::cards::CardDef;
        use crate::game::Event;
        fn record(game: &mut Game<'_>, _: usize, event: Event) {
            if let Event::CardDrawn { seat, .. } = event {
                game.players[0].energy = game.players[0].energy * 10 + seat as i32 + 1;
            }
        }
        static OBSERVER: CardDef = CardDef {
            on_event: Some(record),
            ..CardDef::vanilla("DrawOrderObserver")
        };
        let mut rng = PythonRandom::seed(11);
        let mut game = rng_game(&mut rng);
        game.reveal_priority = Some(1);
        let observer = game.create_card("Wasp", 0);
        game.add_card(observer, 0, 0, true);
        game.cards[observer].override_ability = Some(&OBSERVER);
        game.players[0].energy = 0;
        super::ebonshire(&mut game, 0);
        assert_eq!(game.players[0].energy, 21);
    }
}
