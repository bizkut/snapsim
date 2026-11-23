//! Location rules from the 59.15.1 client.
use super::{LocationDef, location};
use crate::game::{Event, Game};

pub const SOKOVIA: LocationDef = LocationDef {
    on_reveal: Some(sokovia),
    ..location("Sokovia", 0, 0, 1, false)
};
pub const THE_HUB: LocationDef = LocationDef {
    on_reveal: Some(the_hub),
    ..location("TheHub", 0, 0, 1, false)
};
pub const GREAT_PORTAL: LocationDef = LocationDef {
    on_reveal: Some(great_portal),
    ..location("GreatPortal", 0, 0, 1, false)
};
pub const MILANO: LocationDef = LocationDef {
    on_reveal: Some(milano),
    ..location("Milano", 0, 0, 1, false)
};
pub const SUBTERRANEA: LocationDef = LocationDef {
    on_reveal: Some(subterranea),
    ..location("Subterranea", 0, 0, 1, false)
};
pub const WEIRDWORLD: LocationDef = LocationDef {
    on_reveal: Some(weirdworld),
    ..location("Weirdworld", 0, 0, 1, false)
};
pub const LECHUGUILLA: LocationDef = LocationDef {
    on_event: Some(lechuguilla),
    ..location("Lechuguilla", 0, 0, 1, false)
};
pub const LUKES_BAR: LocationDef = LocationDef {
    on_event: Some(lukes_bar),
    ..location("LukesBar", 0, 0, 1, false)
};
pub const UTNAPISHTIMS_ARK: LocationDef = LocationDef {
    on_turn_end: Some(utnapishtims_ark),
    ..location("UtnapishtimsArk", 0, 0, 1, false)
};
pub const SPACE_THRONE: LocationDef = location("SpaceThrone", 0, 0, 1, false);
pub const DEEP_SPACE: LocationDef = LocationDef {
    disables_on_reveal: true,
    disables_ongoing: true,
    blocks_activate: true,
    ..location("DeepSpace", 0, 0, 1, false)
};
pub const STARLIGHT_CITADEL: LocationDef = LocationDef {
    on_turn_end: Some(starlight_citadel),
    ..location("StarlightCitadel", 0, 0, 1, false)
};
pub const PANOPTICHRON: LocationDef = LocationDef {
    ongoing: Some(panoptichron),
    ..location("Panoptichron", 0, 0, 1, false)
};
pub const GREAT_WEB: LocationDef = LocationDef {
    on_turn_end: Some(great_web),
    ..location("GreatWeb", 0, 0, 1, false)
};
// InverseWinner 0x7d3f594 and ForceTie 0x7d431e8 set lane winner rules.
pub const BAR_WITH_NO_NAME: LocationDef = location("BarWithNoName", 0, 0, 1, false);
pub const CANCUN: LocationDef = location("Cancun", 0, 0, 1, false);
pub const NOOR_DIMENSION: LocationDef = LocationDef {
    on_event: Some(noor_dimension),
    ..location("NoorDimension", 0, 0, 1, false)
};
pub const PIZZA_PLACE: LocationDef = LocationDef {
    on_reveal: Some(pizza_place),
    ..location("PizzaPlace", 0, 0, 1, false)
};
pub const TACO_TRUCK: LocationDef = LocationDef {
    on_reveal: Some(taco_truck),
    ..location("TacoTruck", 0, 0, 1, false)
};
pub const DESTROYED_MANSION: LocationDef = LocationDef {
    on_reveal: Some(destroyed_mansion),
    ..location("DestroyedMansion", 0, 0, 1, false)
};
pub const VIBRANIUM_MINES: LocationDef = LocationDef {
    on_event: Some(vibranium_mines),
    ..location("VibraniumMines", 0, 0, 1, false)
};
pub const GAMMA_LAB: LocationDef = LocationDef {
    on_turn_end: Some(gamma_lab),
    ..location("GammaLab", 0, 0, 1, false)
};
pub const BLACK_VORTEX: LocationDef = LocationDef {
    on_event: Some(black_vortex),
    initial_state: Some(|| Box::new([false; 2])),
    ..location("BlackVortex", 0, 0, 1, false)
};
pub const ASGARD_BESIEGED: LocationDef = LocationDef {
    on_turn_end: Some(asgard_besieged),
    ..location("AsgardBesieged", 0, 0, 1, false)
};
pub const HOTEL_INFERNO: LocationDef = LocationDef {
    on_event: Some(hotel_inferno),
    ..location("HotelInferno", 0, 0, 1, false)
};
pub const KRAKOA: LocationDef = LocationDef {
    on_event: Some(krakoa),
    initial_state: Some(|| Box::new([false; 2])),
    ..location("Krakoa", 0, 0, 1, false)
};
pub const THE_PEAK: LocationDef = LocationDef {
    on_reveal: Some(the_peak),
    ..location("ThePeak", 0, 0, 1, false)
};
pub const TRISKELION: LocationDef = LocationDef {
    on_reveal: Some(triskelion),
    ..location("Triskelion", 0, 0, 1, false)
};
pub const TARNAX: LocationDef = LocationDef {
    on_event: Some(tarnax),
    ..location("Tarnax", 0, 0, 1, false)
};
pub const CELESTIAL_BURIAL_GROUND: LocationDef = LocationDef {
    on_reveal: Some(celestial_burial_ground),
    ..location("CelestialBurialGround", 0, 0, 1, false)
};
pub const AUNT_MAYS: LocationDef = LocationDef {
    on_event: Some(aunt_mays),
    initial_state: Some(|| Box::new([false; 2])),
    ..location("AuntMays", 0, 0, 1, false)
};
pub const SACRED_TIMELINE: LocationDef = LocationDef {
    on_reveal: Some(sacred_timeline),
    on_turn_start: Some(sacred_timeline),
    on_event: Some(sacred_timeline_event),
    initial_state: Some(|| Box::new(false)),
    ..location("SacredTimeline", 0, 0, 1, false)
};
pub const ASTRAL_PLANE: LocationDef = LocationDef {
    on_turn_end: Some(astral_plane),
    ..location("AstralPlane", 0, 0, 1, false)
};
pub const CASTLE_ZEMO: LocationDef = LocationDef {
    on_event: Some(castle_zemo),
    initial_state: Some(|| Box::new([false; 2])),
    ..location("CastleZemo", 0, 0, 1, false)
};
pub const KVCH: LocationDef = LocationDef {
    on_reveal: Some(kvch),
    ..location("Kvch", 0, 0, 1, false)
};

// DiscardCard 0x7d80884 rolls a random card from each player's hand.
fn sokovia(game: &mut Game<'_>, _: usize) {
    let first = game.reveal_priority.unwrap_or(0);
    for seat in [first, 1 - first] {
        let count = game.players[seat].hand.len();
        if count != 0 {
            let index = game.rng.borrow_mut().randbelow(count);
            let card = game.players[seat].hand[index];
            game.discard_card(seat, card);
        }
    }
}

// AddRandomCard 0x7d8a1e4 uses the client random-card pool. The
// simulator samples only implemented cards, so its pool is approximate.
fn the_hub(game: &mut Game<'_>, _: usize) {
    let first = game.reveal_priority.unwrap_or(0);
    for seat in [first, 1 - first] {
        if let Some(id) = game.random_card(|_| true) {
            let card = game.create_card(id, seat);
            game.add_hand(seat, card, false);
        }
    }
}

// AddRandomHighPowerCard 0x7d56b84, predicate 0x7d573e8; constructor
// 0x7d57014 excludes Agatha Harkness. The implemented pool is approximate.
fn great_portal(game: &mut Game<'_>, _: usize) {
    let first = game.reveal_priority.unwrap_or(0);
    for seat in [first, 1 - first] {
        if let Some(id) =
            game.random_card(|row| row.power >= 10 && row.carddefid != "AgathaHarkness")
        {
            let card = game.create_card(id, seat);
            game.add_hand(seat, card, false);
        }
    }
}

// AddGuardian 0x7d64fec rolls independently for each player from the
// six-card list in LocationDefList::CreateMilano 0x7a69f98.
fn milano(game: &mut Game<'_>, _: usize) {
    const CREW: [&str; 6] = [
        "Mantis",
        "RocketRaccoon",
        "Starlord",
        "Groot",
        "Drax",
        "Gamora",
    ];
    let first = game.reveal_priority.unwrap_or(0);
    for seat in [first, 1 - first] {
        let index = game.rng.borrow_mut().randbelow(CREW.len());
        let card = game.create_card(CREW[index], seat);
        game.add_hand(seat, card, false);
    }
}

// ShuffleRocks 0x7d84dbc creates the configured four Rocks in each
// deck and shuffles each deck once.
fn subterranea(game: &mut Game<'_>, _: usize) {
    let first = game.reveal_priority.unwrap_or(0);
    for seat in [first, 1 - first] {
        for _ in 0..4 {
            let card = game.create_card("Rock", seat);
            game.cards[card].created_by_location_id = Some("Subterranea".into());
            game.push_deck(seat, card);
        }
        game.shuffle_deck(seat);
    }
}

// DrawCard 0x7d97880 draws the configured two cards for each player
// from the opponent's deck. Hidden deck contents are sampled.
fn weirdworld(game: &mut Game<'_>, _: usize) {
    let first = game.reveal_priority.unwrap_or(0);
    for seat in [first, 1 - first] {
        for _ in 0..2 {
            game.draw_from_deck(seat, 1 - seat);
        }
    }
}

// ShuffleRocks 0x7d618d0 checks IsCardPlayedHereByPlayer 0x7de1ea8
// after resolution, creates three Rocks in the player's deck, then shuffles.
fn lechuguilla(game: &mut Game<'_>, lane: usize, event: Event) {
    if let Event::CardResolved { card } = event
        && let Some(seat) = (0..2).find(|&seat| {
            game.played_cards_at(seat, lane)
                .any(|played| played == card)
        })
    {
        for _ in 0..3 {
            let rock = game.create_card("Rock", seat);
            game.cards[rock].created_by_location_id = Some("Lechuguilla".into());
            game.push_deck(seat, rock);
        }
        game.shuffle_deck(seat);
    }
}

// AddDjinn 0x7d6cea4 checks the staged play's location.
fn noor_dimension(game: &mut Game<'_>, lane: usize, event: Event) {
    if let Event::CardResolved { card } = event
        && let Some(seat) = (0..2).find(|&seat| {
            game.played_cards_at(seat, lane)
                .any(|played| played == card)
        })
    {
        let token = game.create_card("Djinn", seat);
        game.add_hand(seat, token, false);
    }
}

// AddSlicesToHand 0x7d72890 creates two Pizza Slices per player.
fn pizza_place(game: &mut Game<'_>, _: usize) {
    let first = game.reveal_priority.unwrap_or(0);
    for seat in [first, 1 - first] {
        for _ in 0..2 {
            let token = game.create_card("PizzaSlice", seat);
            game.add_hand(seat, token, false);
        }
    }
}

// AddChimichanga 0x7d86a50 creates one token per player.
fn taco_truck(game: &mut Game<'_>, _: usize) {
    let first = game.reveal_priority.unwrap_or(0);
    for seat in [first, 1 - first] {
        let token = game.create_card("Chimichanga", seat);
        game.add_hand(seat, token, false);
    }
}

// AddRubble 0x7d4eb08 creates a Rock here and Vibranium in hand.
fn destroyed_mansion(game: &mut Game<'_>, lane: usize) {
    let first = game.reveal_priority.unwrap_or(0);
    for seat in [first, 1 - first] {
        game.add_created("Rock", lane, seat);
        let token = game.create_card("Vibranium", seat);
        game.add_hand(seat, token, false);
    }
}

// ShuffleVibraniumChunks 0x7d93528 checks the staged play's location,
// creates three Vibranium and then shuffles that player's deck.
fn vibranium_mines(game: &mut Game<'_>, lane: usize, event: Event) {
    if let Event::CardResolved { card } = event
        && let Some(seat) = (0..2).find(|&seat| {
            game.played_cards_at(seat, lane)
                .any(|played| played == card)
        })
    {
        for _ in 0..3 {
            let token = game.create_card("Vibranium", seat);
            game.cards[token].created_by_location_id = Some("VibraniumMines".into());
            game.push_deck(seat, token);
        }
        game.shuffle_deck(seat);
    }
}

// BounceCard 0x7d63a5c moves a card played here back to the staged
// player's hand after resolution, provided it is still in a location.
fn lukes_bar(game: &mut Game<'_>, lane: usize, event: Event) {
    if let Event::CardResolved { card } = event
        && game.cards[card].lane.is_some()
        && let Some(seat) = (0..2).find(|&seat| {
            game.played_cards_at(seat, lane)
                .any(|played| played == card)
        })
    {
        game.return_board_card_to_seat_hand(card, seat);
    }
}

// MorphToFlooded 0x7d90018 visits every other location after turn five.
fn utnapishtims_ark(game: &mut Game<'_>, lane: usize) {
    if game.turn == 5 {
        for other in 0..3 {
            if other != lane {
                game.replace_location(other, "Flooded");
            }
        }
    }
}

// ShuffleLocations 0x7d8450c calls Game::ShuffleLocations after turn four.
fn starlight_citadel(game: &mut Game<'_>, _: usize) {
    if game.turn == 4 {
        game.shuffle_locations();
    }
}

// BuffCards 0x7d70bc0 uses IsCreated predicate 0x7d712a8.
fn panoptichron(game: &mut Game<'_>, lane: usize) {
    for index in 0..game.lanes[lane].cards.len() {
        let card = game.lanes[lane].cards[index];
        if game.cards[card].revealed && game.cards[card].created {
            game.add_location_power(card, 2);
        }
    }
}

// MoveCard 0x7d57450 collects revealed cards elsewhere for players with
// room here, chooses one uniformly, then attempts the forced move.
fn great_web(game: &mut Game<'_>, lane: usize) {
    let mut candidates = [0; 24];
    let mut count = 0;
    let first = game.reveal_priority.unwrap_or(0);
    for seat in [first, 1 - first] {
        if !game.has_space(lane, seat, 0) {
            continue;
        }
        for source in 0..3 {
            if source == lane {
                continue;
            }
            for &card in &game.lanes[source].cards {
                if game.cards[card].owner == seat && game.cards[card].revealed {
                    candidates[count] = card;
                    count += 1;
                }
            }
        }
    }
    if count != 0 {
        let choice = game.rng.borrow_mut().randbelow(count);
        game.move_card(candidates[choice], lane);
    }
}

// Hulkify 0x7d5555c morphs each revealed card here without modifiers
// after turn three.
fn gamma_lab(game: &mut Game<'_>, lane: usize) {
    if game.turn != 3 {
        return;
    }
    let mut cards = [0; 8];
    let mut count = 0;
    for &card in &game.lanes[lane].cards {
        if game.cards[card].revealed {
            cards[count] = card;
            count += 1;
        }
    }
    for card in cards[..count].iter().copied() {
        game.transform_card_without_modifiers(card, "Hulk");
    }
}

// TransformCardInto6Cost 0x7d407d4 uses a separate consumed flag for
// each staged player; candidate predicate 0x7d4126c checks printed cost
// and character type. The random pool is limited to implemented cards.
fn black_vortex(game: &mut Game<'_>, lane: usize, event: Event) {
    let Event::CardResolved { card } = event else {
        return;
    };
    let Some(seat) = (0..2).find(|&seat| {
        game.played_cards_at(seat, lane)
            .any(|played| played == card)
    }) else {
        return;
    };
    // A hook dispatched in the same pass can run after this location was
    // replaced; act only while its initialised consumed-flag state is present.
    let Some(spent) = game.lanes[lane]
        .state
        .as_deref()
        .and_then(|state| state.as_any().downcast_ref::<[bool; 2]>())
    else {
        return;
    };
    if spent[seat] {
        return;
    }
    if let Some(id) = game.random_card(|row| row.cost == 6 && row.card_type == "Character") {
        // Mark before transforming: the transformed card's reveal can move this
        // location (Quake swaps whole lanes, carrying the flags) or replace it
        // (e.g. with Kyln, dropping them), and the lane index is stale afterwards.
        if let Some(spent) = game.lanes[lane]
            .state
            .as_deref_mut()
            .and_then(|state| state.as_any_mut().downcast_mut::<[bool; 2]>())
        {
            spent[seat] = true;
        }
        game.transform_card_without_modifiers(card, id);
    }
}

// AddDestroyer 0x7d3bffc creates Destroyer for the location winner on turn 3.
fn asgard_besieged(game: &mut Game<'_>, lane: usize) {
    if game.turn == 3
        && let Some(seat) = game.lane_winner(lane)
    {
        game.add_created("Destroyer", lane, seat);
    }
}

// BanishTopCardOfDeck 0x7d59b04 acts on local plays during the reveal turn.
fn hotel_inferno(game: &mut Game<'_>, lane: usize, event: Event) {
    if game.lanes[lane].revealed_turn != Some(game.turn) {
        return;
    }
    if let Event::CardResolved { card } = event
        && let Some(seat) = (0..2).find(|&seat| {
            game.played_cards_at(seat, lane)
                .any(|played| played == card)
        })
        && let Some(&top) = game.players[1 - seat].deck.first()
    {
        game.banish_card(top);
    }
}

// BuffTopCard 0x7d5e418 consumes each player's first local play even if
// their deck is empty, then adds that play's current power to its top card.
fn krakoa(game: &mut Game<'_>, lane: usize, event: Event) {
    let Event::CardResolved { card } = event else {
        return;
    };
    let Some(seat) = (0..2).find(|&seat| {
        game.played_cards_at(seat, lane)
            .any(|played| played == card)
    }) else {
        return;
    };
    let spent = game.lanes[lane]
        .state
        .as_deref_mut()
        .unwrap()
        .as_any_mut()
        .downcast_mut::<[bool; 2]>()
        .unwrap();
    if spent[seat] {
        return;
    }
    spent[seat] = true;
    if let Some(&top) = game.players[seat].deck.first() {
        game.add_power(top, game.cards[card].power(), seat);
    }
}

// SwapCards 0x7d8b7cc exchanges the leftmost hand cards simultaneously,
// even when one player's hand is empty or full.
fn the_peak(game: &mut Game<'_>, _: usize) {
    game.exchange_top_hand_cards();
}

// FillWithRandomCards 0x7d8ee08 requests the vacant hand slots in one
// GetRandomCardDefIds call with preventDuplicates=true for each player.
// The pool is limited to implemented cards.
fn triskelion(game: &mut Game<'_>, _: usize) {
    let first = game.reveal_priority.unwrap_or(0);
    for seat in [first, 1 - first] {
        let count = 7_usize.saturating_sub(game.players[seat].hand.len());
        let mut chosen = [None; 7];
        for index in 0..count {
            let id = game.random_card(|row| {
                !chosen[..index]
                    .iter()
                    .flatten()
                    .any(|id| *id == row.carddefid)
            });
            let Some(id) = id else { break };
            chosen[index] = Some(id);
        }
        for id in chosen.into_iter().flatten() {
            let card = game.create_card(id, seat);
            game.add_hand(seat, card, false);
        }
    }
}

// TransformCard 0x7d87eac uses DefCost, excludes the original definition
// and morphs without modifiers. It falls back to the original if the pool
// has no candidate; random candidates are limited to implemented cards.
fn tarnax(game: &mut Game<'_>, lane: usize, event: Event) {
    let Event::CardResolved { card } = event else {
        return;
    };
    if !game.is_character(card)
        || !(0..2).any(|seat| {
            game.played_cards_at(seat, lane)
                .any(|played| played == card)
        })
    {
        return;
    }
    let original = game.cards[card].def.id;
    let cost = game.printed_cost(card);
    let id = game
        .random_card(|row| {
            row.cost == cost && row.card_type == "Character" && row.carddefid != original
        })
        .unwrap_or(original);
    game.transform_card_without_modifiers(card, id);
}

// DiscardAndAddCardToHand 0x7d45fdc discards one random card per player,
// then creates a different same-DefCost card; fallback is the original.
// Random candidates are limited to implemented cards.
fn celestial_burial_ground(game: &mut Game<'_>, _: usize) {
    let first = game.reveal_priority.unwrap_or(0);
    for seat in [first, 1 - first] {
        let count = game.players[seat].hand.len();
        if count == 0 {
            continue;
        }
        let index = game.rng.borrow_mut().randbelow(count);
        let card = game.players[seat].hand[index];
        let cost = game.printed_cost(card);
        let original = game.cards[card].def.id;
        if game.discard_card(seat, card) {
            let id = game
                .random_card(|row| row.cost == cost && row.carddefid != original)
                .unwrap_or(original);
            let replacement = game.create_card(id, seat);
            game.add_hand(seat, replacement, false);
        }
    }
}

// MoveCard 0x7d3e664 consumes the player's first resolved local play,
// chooses a random valid destination, grants +3, then attempts the move.
fn aunt_mays(game: &mut Game<'_>, lane: usize, event: Event) {
    let Event::CardResolved { card } = event else {
        return;
    };
    let Some(seat) = (0..2).find(|&seat| {
        game.played_cards_at(seat, lane)
            .any(|played| played == card)
    }) else {
        return;
    };
    let spent = game.lanes[lane]
        .state
        .as_deref_mut()
        .unwrap()
        .as_any_mut()
        .downcast_mut::<[bool; 2]>()
        .unwrap();
    if spent[seat] {
        return;
    }
    spent[seat] = true;
    let destination = game.random_location_to_move_card(card, lane, false);
    game.add_power(card, 3, seat);
    if let Some(destination) = destination {
        game.move_card(card, destination);
    }
}

// TriggerGiveStartingHand 0x7d79e08 checks the shared
// AlreadyGaveStartingHand context after visiting both filled sides.
fn sacred_timeline(game: &mut Game<'_>, lane: usize) {
    if *game.lanes[lane]
        .state
        .as_deref()
        .unwrap()
        .as_any()
        .downcast_ref::<bool>()
        .unwrap()
    {
        return;
    }
    let full = std::array::from_fn::<_, 2, _>(|seat| !game.has_space(lane, seat, 0));
    if !full.iter().any(|&value| value) {
        return;
    }
    *game.lanes[lane]
        .state
        .as_deref_mut()
        .unwrap()
        .as_any_mut()
        .downcast_mut::<bool>()
        .unwrap() = true;
    let first = game.reveal_priority.unwrap_or(0);
    for seat in [first, 1 - first] {
        if full[seat] {
            let count = game.players[seat]
                .starting_hand
                .as_ref()
                .map_or(0, Vec::len);
            for index in 0..count {
                game.copy_starting_hand_card(seat, index);
            }
        }
    }
}

fn sacred_timeline_event(game: &mut Game<'_>, lane: usize, event: Event) {
    if matches!(
        event,
        Event::BeforeCardRevealed { .. }
            | Event::CardResolved { .. }
            | Event::BeforeCardMoved { .. }
    ) {
        sacred_timeline(game, lane);
    }
}

// AddCopies 0x7d3ca74 selects one random hand card per player at the
// end of the reveal turn, creates its copy here, then sets its Power to 0.
fn astral_plane(game: &mut Game<'_>, lane: usize) {
    if game.lanes[lane].revealed_turn != Some(game.turn) {
        return;
    }
    let first = game.reveal_priority.unwrap_or(0);
    for seat in [first, 1 - first] {
        let count = game.players[seat].hand.len();
        if count == 0 {
            continue;
        }
        let index = game.rng.borrow_mut().randbelow(count);
        let original = game.players[seat].hand[index];
        let copy = game.copy_card(original, seat);
        if game.add_card_deferred(copy, lane, seat) {
            game.set_power(copy, 0, seat);
        }
    }
    game.resolve_deferred_cards();
}

// FirstPlaySwapsSides 0x7d44b5c consumes each staging player's first
// local play, then changes the card's owner if it remains at a location.
fn castle_zemo(game: &mut Game<'_>, lane: usize, event: Event) {
    let Event::CardResolved { card } = event else {
        return;
    };
    let Some(seat) = (0..2).find(|&seat| {
        game.played_cards_at(seat, lane)
            .any(|played| played == card)
    }) else {
        return;
    };
    let spent = game.lanes[lane]
        .state
        .as_deref_mut()
        .unwrap()
        .as_any_mut()
        .downcast_mut::<[bool; 2]>()
        .unwrap();
    if spent[seat] {
        return;
    }
    spent[seat] = true;
    if game.cards[card].lane.is_some() {
        game.change_card_owner(card);
    }
}

// GetRandomActivate 0x7d5f1e4 samples up to two distinct Activate cards
// before creating them; if only one candidate exists both sides use it.
// The candidate pool is restricted to implemented cards.
fn kvch(game: &mut Game<'_>, lane: usize) {
    let is_activate = |row: &crate::data::CardData| {
        crate::cards::get(&row.carddefid).is_some_and(|def| def.on_activate.is_some())
    };
    let Some(first) = game.random_card(is_activate) else {
        return;
    };
    let second = game
        .random_card(|row| is_activate(row) && row.carddefid != first)
        .unwrap_or(first);
    for (seat, id) in [first, second].into_iter().enumerate() {
        if game.has_space(lane, seat, 0) {
            let card = game.create_card(id, seat);
            game.add_card_deferred(card, lane, seat);
        }
    }
    game.resolve_deferred_cards();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    fn deck() -> [String; 12] {
        [
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
        .map(str::to_owned)
    }

    #[test]
    fn sokovia_discards_one_from_each_nonempty_hand() {
        let mut rng = PythonRandom::seed(1);
        let deck = deck();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        for player in &mut game.players {
            player.hand.clear();
        }
        for seat in 0..2 {
            let card = game.create_card("Hawkeye", seat);
            game.add_hand(seat, card, false);
        }
        sokovia(&mut game, 0);
        for player in &game.players {
            assert!(player.hand.is_empty());
            assert_eq!(player.discard.len(), 1);
        }
    }

    #[test]
    fn hub_adds_one_card_to_each_hand() {
        let mut rng = PythonRandom::seed(2);
        let deck = deck();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        for player in &mut game.players {
            player.hand.clear();
        }
        the_hub(&mut game, 0);
        assert_eq!(game.players[0].hand.len(), 1);
        assert_eq!(game.players[1].hand.len(), 1);
    }

    #[test]
    fn portal_adds_only_high_power_cards() {
        let mut rng = PythonRandom::seed(3);
        let deck = deck();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        for player in &mut game.players {
            player.hand.clear();
        }
        great_portal(&mut game, 0);
        for player in &game.players {
            assert_eq!(player.hand.len(), 1);
            let card = &game.cards[player.hand[0]];
            assert!(card.power() >= 10);
            assert_ne!(card.def.id, "AgathaHarkness");
        }
    }

    #[test]
    fn milano_adds_a_guardian_to_each_hand() {
        let mut rng = PythonRandom::seed(4);
        let deck = deck();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        for player in &mut game.players {
            player.hand.clear();
        }
        milano(&mut game, 0);
        for player in &game.players {
            assert_eq!(player.hand.len(), 1);
            assert!(
                [
                    "Mantis",
                    "RocketRaccoon",
                    "Starlord",
                    "Groot",
                    "Drax",
                    "Gamora"
                ]
                .contains(&game.cards[player.hand[0]].def.id)
            );
        }
    }

    #[test]
    fn subterranea_shuffles_four_rocks_into_each_deck() {
        let mut rng = PythonRandom::seed(5);
        let deck = deck();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let before = [game.players[0].deck.len(), game.players[1].deck.len()];
        subterranea(&mut game, 0);
        for (seat, count) in before.into_iter().enumerate() {
            assert_eq!(game.players[seat].deck.len(), count + 4);
            assert_eq!(
                game.players[seat]
                    .deck
                    .iter()
                    .filter(|&&card| game.cards[card].def.id == "Rock")
                    .count(),
                4
            );
        }
    }

    #[test]
    fn weirdworld_draws_two_from_each_opposing_deck() {
        let mut rng = PythonRandom::seed(6);
        let a = deck();
        let b = deck();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&a, &b], &locations, &mut rng).unwrap();
        for player in &mut game.players {
            player.hand.clear();
        }
        for (seat, id) in ["Hawkeye", "Hulk"].into_iter().enumerate() {
            game.players[seat].deck.clear();
            for _ in 0..2 {
                let card = game.create_card(id, seat);
                game.players[seat].deck.push(card);
            }
        }
        weirdworld(&mut game, 0);
        for (seat, expected) in ["Hulk", "Hawkeye"].into_iter().enumerate() {
            assert_eq!(game.players[seat].hand.len(), 2);
            assert!(
                game.players[seat]
                    .hand
                    .iter()
                    .all(|&card| game.cards[card].def.id == expected
                        && game.cards[card].owner == seat)
            );
        }
    }

    #[test]
    fn lechuguilla_adds_three_rocks_after_play_here() {
        let mut rng = PythonRandom::seed(7);
        let deck = deck();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let card = game.create_card("Hawkeye", 0);
        game.cards[card].turn_played = 1;
        game.played[0].push((card, 0));
        let before = game.players[0].deck.len();
        lechuguilla(&mut game, 0, Event::CardResolved { card });
        assert_eq!(game.players[0].deck.len(), before + 3);
        assert_eq!(
            game.players[0]
                .deck
                .iter()
                .filter(|&&card| game.cards[card].def.id == "Rock")
                .count(),
            3
        );
    }

    #[test]
    fn lukes_bar_returns_resolved_play_to_hand() {
        let mut rng = PythonRandom::seed(8);
        let deck = deck();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let card = game.create_card("Hawkeye", 0);
        assert!(game.add_card(card, 0, 0, false));
        game.cards[card].turn_played = 1;
        game.played[0].push((card, 0));
        lukes_bar(&mut game, 0, Event::CardResolved { card });
        assert!(game.players[0].hand.contains(&card));
        assert!(game.cards[card].lane.is_none());
    }

    #[test]
    fn ark_floods_other_locations_after_turn_five() {
        let mut rng = PythonRandom::seed(9);
        let deck = deck();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let original = game.lanes.each_ref().map(|lane| lane.def.id);
        game.turn = 4;
        utnapishtims_ark(&mut game, 1);
        assert_eq!(game.lanes[0].def.id, original[0]);
        game.turn = 5;
        utnapishtims_ark(&mut game, 1);
        assert_eq!(game.lanes[0].def.id, "Flooded");
        assert_eq!(game.lanes[1].def.id, original[1]);
        assert_eq!(game.lanes[2].def.id, "Flooded");
    }

    #[test]
    fn space_throne_limits_each_side_to_one() {
        let mut rng = PythonRandom::seed(10);
        let deck = deck();
        let locations = ["SpaceThrone", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        game.lanes[0].def = &SPACE_THRONE;
        game.lanes[0].revealed = true;
        let card = game.create_card("Hawkeye", 0);
        assert!(game.add_card(card, 0, 0, false));
        assert!(!game.has_space(0, 0, 0));
        assert!(game.has_space(0, 1, 0));
    }

    #[test]
    fn unrestricted_play_uses_physical_slots_at_space_throne() {
        let mut rng = PythonRandom::seed(10);
        let deck = deck();
        let locations = ["SpaceThrone", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        game.lanes[0].def = &SPACE_THRONE;
        game.lanes[0].revealed = true;
        let occupant = game.create_card("Hawkeye", 0);
        game.cards[occupant].created = true;
        assert!(game.add_card(occupant, 0, 0, false));
        let jeff = game.create_card("JeffTheBabyLandShark", 0);
        game.players[0].hand.push(jeff);
        assert!(!game.can_play(jeff, 0, 0, 0, 0));
        assert!(game.can_play(jeff, 0, 0, 10, 0));
        assert!(!game.can_play(jeff, 0, 0, 10, 3));

        let normal = game.create_card("Wasp", 0);
        game.players[0].hand.push(normal);
        assert!(!game.can_play(normal, 0, 0, 10, 0));
        let war_machine = game.create_card("WarMachine", 0);
        assert!(game.add_card(war_machine, 1, 0, true));
        assert!(game.can_play(normal, 0, 0, 10, 0));
        game.players[0].energy = 10;
        assert!(
            game.stage_action(
                0,
                crate::game::Action::Play {
                    card: normal,
                    lane: 0
                }
            )
            .is_ok()
        );
        assert_eq!(game.lanes[0].cards.len(), 2);
        game.lanes[0].destroyed = true;
        assert!(!game.can_play(jeff, 0, 0, 10, 0));
    }

    #[test]
    fn deep_space_disables_card_trigger_and_ongoing() {
        let mut rng = PythonRandom::seed(11);
        let deck = deck();
        let locations = ["DeepSpace", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        game.lanes[0].def = &DEEP_SPACE;
        game.lanes[0].revealed = true;
        let card = game.create_card("IronMan", 0);
        assert!(game.add_card(card, 0, 0, false));
        game.cards[card].revealed = true;
        assert!(!game.ongoing_active(card));
    }

    #[test]
    fn starlight_moves_every_location_after_turn_four() {
        let mut rng = PythonRandom::seed(12);
        let deck = deck();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let original = game.lanes.each_ref().map(|lane| lane.def.id);
        game.turn = 4;
        starlight_citadel(&mut game, 0);
        for (index, original) in original.into_iter().enumerate() {
            assert_ne!(game.lanes[index].def.id, original);
        }
    }

    #[test]
    fn panoptichron_buffs_only_created_revealed_cards() {
        let mut rng = PythonRandom::seed(13);
        let deck = deck();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let created = game.create_card("Hawkeye", 0);
        let original = game.create_card("Hawkeye", 1);
        game.cards[original].created = false;
        assert!(game.add_card(created, 0, 0, false));
        assert!(game.add_card(original, 0, 1, false));
        game.cards[created].revealed = true;
        game.cards[original].revealed = true;
        panoptichron(&mut game, 0);
        assert_eq!(game.cards[created].location_power, 2);
        assert_eq!(game.cards[original].location_power, 0);
    }

    #[test]
    fn great_web_moves_a_revealed_card_from_another_location() {
        let mut rng = PythonRandom::seed(14);
        let deck = deck();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let card = game.create_card("Hawkeye", 0);
        assert!(game.add_card(card, 1, 0, false));
        game.cards[card].revealed = true;
        great_web(&mut game, 0);
        assert_eq!(game.cards[card].lane, Some(0));
    }

    #[test]
    fn bar_with_no_name_awards_fewer_points() {
        let mut rng = PythonRandom::seed(15);
        let deck = deck();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        game.lanes[0].def = &BAR_WITH_NO_NAME;
        game.lanes[0].revealed = true;
        game.lanes[0].bonus = [3, 1];
        assert_eq!(game.lane_winner(0), Some(1));
    }

    #[test]
    fn cancun_forces_tie_despite_power_difference() {
        let mut rng = PythonRandom::seed(16);
        let deck = deck();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        game.lanes[0].def = &CANCUN;
        game.lanes[0].revealed = true;
        game.lanes[0].bonus = [3, 1];
        assert_eq!(game.lane_winner(0), None);
    }

    #[test]
    fn noor_adds_djinn_after_local_play() {
        let mut rng = PythonRandom::seed(17);
        let deck = deck();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let card = game.create_card("Hawkeye", 0);
        game.cards[card].turn_played = 1;
        game.played[0].push((card, 0));
        noor_dimension(&mut game, 0, Event::CardResolved { card });
        assert!(
            game.players[0]
                .hand
                .iter()
                .any(|&id| game.cards[id].def.id == "Djinn")
        );
    }

    #[test]
    fn pizza_adds_two_slices_per_player() {
        let mut rng = PythonRandom::seed(18);
        let deck = deck();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        pizza_place(&mut game, 0);
        for player in &game.players {
            assert_eq!(
                player
                    .hand
                    .iter()
                    .filter(|&&id| game.cards[id].def.id == "PizzaSlice")
                    .count(),
                2
            );
        }
    }

    #[test]
    fn taco_adds_chimichanga_per_player() {
        let mut rng = PythonRandom::seed(19);
        let deck = deck();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        taco_truck(&mut game, 0);
        for player in &game.players {
            assert!(
                player
                    .hand
                    .iter()
                    .any(|&id| game.cards[id].def.id == "Chimichanga")
            );
        }
    }

    #[test]
    fn destroyed_mansion_adds_rock_and_vibranium() {
        let mut rng = PythonRandom::seed(20);
        let deck = deck();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        destroyed_mansion(&mut game, 0);
        assert_eq!(
            game.lanes[0]
                .cards
                .iter()
                .filter(|&&id| game.cards[id].def.id == "Rock")
                .count(),
            2
        );
        for player in &game.players {
            assert!(
                player
                    .hand
                    .iter()
                    .any(|&id| game.cards[id].def.id == "Vibranium")
            );
        }
    }

    #[test]
    fn vibranium_mines_shuffles_three_chunks() {
        let mut rng = PythonRandom::seed(21);
        let deck = deck();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let card = game.create_card("Hawkeye", 0);
        game.cards[card].turn_played = 1;
        game.played[0].push((card, 0));
        vibranium_mines(&mut game, 0, Event::CardResolved { card });
        assert_eq!(
            game.players[0]
                .deck
                .iter()
                .filter(|&&id| game.cards[id].def.id == "Vibranium")
                .count(),
            3
        );
    }

    #[test]
    fn gamma_lab_morphs_revealed_cards_after_turn_three() {
        let mut rng = PythonRandom::seed(22);
        let deck = deck();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let card = game.create_card("Hawkeye", 0);
        assert!(game.add_card(card, 0, 0, false));
        game.cards[card].revealed = true;
        game.turn = 3;
        gamma_lab(&mut game, 0);
        assert_eq!(game.cards[card].def.id, "Hulk");
    }

    #[test]
    fn vortex_transforms_only_first_play_per_player() {
        let mut rng = PythonRandom::seed(23);
        let deck = deck();
        let locations = ["BlackVortex", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        game.lanes[0].state = Some(Box::new([false; 2]));
        let first = game.create_card("Hawkeye", 0);
        game.cards[first].turn_played = 1;
        game.played[0].push((first, 0));
        black_vortex(&mut game, 0, Event::CardResolved { card: first });
        assert_eq!(game.cards[first].cost, 6);
        let second = game.create_card("Hawkeye", 0);
        game.cards[second].turn_played = 1;
        game.played[0].push((second, 0));
        black_vortex(&mut game, 0, Event::CardResolved { card: second });
        assert_eq!(game.cards[second].def.id, "Hawkeye");
    }

    #[test]
    fn asgard_besieged_summons_destroyer_for_winner() {
        let mut rng = PythonRandom::seed(24);
        let deck = deck();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        game.lanes[0].bonus = [2, 0];
        game.turn = 3;
        asgard_besieged(&mut game, 0);
        assert!(
            game.lanes[0]
                .cards
                .iter()
                .any(|&id| game.cards[id].def.id == "Destroyer" && game.cards[id].owner == 0)
        );
    }

    #[test]
    fn hotel_banishes_opponent_top_card_in_reveal_turn() {
        let mut rng = PythonRandom::seed(25);
        let deck = deck();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let card = game.create_card("Hawkeye", 0);
        game.turn = 2;
        game.lanes[0].revealed_turn = Some(2);
        game.cards[card].turn_played = 2;
        game.played[0].push((card, 0));
        let top = game.players[1].deck[0];
        hotel_inferno(&mut game, 0, Event::CardResolved { card });
        assert!(game.players[1].banished.contains(&top));
    }

    #[test]
    fn krakoa_buffs_top_deck_card_once_per_player() {
        let mut rng = PythonRandom::seed(26);
        let deck = deck();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        game.lanes[0].state = Some(Box::new([false; 2]));
        let card = game.create_card("Hawkeye", 0);
        game.cards[card].turn_played = 1;
        game.played[0].push((card, 0));
        let top = game.players[0].deck[0];
        let before = game.cards[top].power();
        krakoa(&mut game, 0, Event::CardResolved { card });
        krakoa(&mut game, 0, Event::CardResolved { card });
        assert_eq!(game.cards[top].power(), before + game.cards[card].power());
    }

    #[test]
    fn peak_exchanges_leftmost_hand_cards() {
        let mut rng = PythonRandom::seed(27);
        let deck = deck();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let a = game.players[0].hand[0];
        let b = game.players[1].hand[0];
        the_peak(&mut game, 0);
        assert!(game.players[0].hand.contains(&b));
        assert!(game.players[1].hand.contains(&a));
        assert_eq!(game.cards[a].owner, 1);
        assert_eq!(game.cards[b].owner, 0);
    }

    #[test]
    fn triskelion_fills_each_hand_with_distinct_generated_cards() {
        let mut rng = PythonRandom::seed(28);
        let deck = deck();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let before = [game.players[0].hand.len(), game.players[1].hand.len()];
        triskelion(&mut game, 0);
        for (seat, initial) in before.into_iter().enumerate() {
            assert_eq!(game.players[seat].hand.len(), 7);
            let generated = &game.players[seat].hand[initial..];
            for (index, &card) in generated.iter().enumerate() {
                assert!(
                    generated[..index]
                        .iter()
                        .all(|&prior| game.cards[prior].def.id != game.cards[card].def.id)
                );
            }
        }
    }

    #[test]
    fn tarnax_morphs_played_character_using_printed_cost() {
        let mut rng = PythonRandom::seed(29);
        let deck = deck();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let card = game.create_card("Hawkeye", 0);
        game.cards[card].cost = 6;
        game.cards[card].turn_played = 1;
        game.played[0].push((card, 0));
        tarnax(&mut game, 0, Event::CardResolved { card });
        assert_eq!(game.printed_cost(card), 1);
        assert_ne!(game.cards[card].def.id, "Hawkeye");
    }

    #[test]
    fn burial_replaces_discarded_card_with_same_printed_cost() {
        let mut rng = PythonRandom::seed(30);
        let deck = deck();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        for player in &mut game.players {
            player.hand.clear();
        }
        for seat in 0..2 {
            let card = game.create_card("Hawkeye", seat);
            game.add_hand(seat, card, false);
        }
        celestial_burial_ground(&mut game, 0);
        for seat in 0..2 {
            assert_eq!(game.players[seat].discard.len(), 1);
            assert_eq!(game.players[seat].hand.len(), 1);
            assert_eq!(game.printed_cost(game.players[seat].hand[0]), 1);
        }
    }

    #[test]
    fn aunt_mays_buffs_then_moves_first_play() {
        let mut rng = PythonRandom::seed(31);
        let deck = deck();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        game.lanes[0].state = Some(Box::new([false; 2]));
        let card = game.create_card("Hawkeye", 0);
        assert!(game.add_card(card, 0, 0, false));
        game.cards[card].revealed = true;
        game.cards[card].turn_played = 1;
        game.played[0].push((card, 0));
        let power = game.cards[card].power();
        aunt_mays(&mut game, 0, Event::CardResolved { card });
        assert_eq!(game.cards[card].power(), power + 3);
        assert_ne!(game.cards[card].lane, Some(0));
    }

    #[test]
    fn sacred_timeline_copies_opening_hand_once_when_full() {
        let mut rng = PythonRandom::seed(32);
        let deck = deck();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        game.lanes[0].state = Some(Box::new(false));
        game.players[0].hand.clear();
        for _ in 0..4 {
            let card = game.create_card("Hawkeye", 0);
            assert!(game.add_card(card, 0, 0, false));
        }
        sacred_timeline(&mut game, 0);
        assert_eq!(game.players[0].hand.len(), 3);
        sacred_timeline(&mut game, 0);
        assert_eq!(game.players[0].hand.len(), 3);
    }

    #[test]
    fn astral_plane_copies_hand_cards_at_zero_power() {
        let mut rng = PythonRandom::seed(33);
        let deck = deck();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        game.turn = 2;
        game.lanes[0].revealed_turn = Some(2);
        for player in &mut game.players {
            player.hand.clear();
        }
        for seat in 0..2 {
            let card = game.create_card("Hulk", seat);
            game.add_hand(seat, card, false);
        }
        astral_plane(&mut game, 0);
        assert_eq!(game.lanes[0].cards.len(), 2);
        for &card in &game.lanes[0].cards {
            assert_eq!(game.cards[card].def.id, "Hulk");
            assert_eq!(game.cards[card].power(), 0);
        }
    }

    #[test]
    fn castle_zemo_changes_first_play_owner() {
        let mut rng = PythonRandom::seed(34);
        let deck = deck();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        game.lanes[0].state = Some(Box::new([false; 2]));
        let card = game.create_card("Hawkeye", 0);
        assert!(game.add_card(card, 0, 0, false));
        game.cards[card].turn_played = 1;
        game.played[0].push((card, 0));
        castle_zemo(&mut game, 0, Event::CardResolved { card });
        assert_eq!(game.cards[card].owner, 1);
        castle_zemo(&mut game, 0, Event::CardResolved { card });
        assert_eq!(game.cards[card].owner, 1);
    }

    #[test]
    fn kvch_creates_activate_cards_for_both_sides() {
        let mut rng = PythonRandom::seed(35);
        let deck = deck();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        kvch(&mut game, 0);
        assert_eq!(game.lanes[0].cards.len(), 2);
        for &card in &game.lanes[0].cards {
            assert!(game.cards[card].def.on_activate.is_some());
        }
    }
}
