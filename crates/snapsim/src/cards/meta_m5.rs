//! Card rules traced from the 59.15.1 client.
use super::CardDef;
use crate::game::{Event, Game};

pub(crate) static TOKENS: &[&str] = &["BannerHulk"];

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        end_turn: Some(bruce_banner),
        end_turn_trigger: true,
        ..CardDef::vanilla("BruceBanner")
    },
    CardDef {
        reveal: Some(shanna),
        ..CardDef::vanilla("Shanna")
    },
    CardDef {
        on_discard: Some(x23_discard),
        on_destroy: Some(x23_destroy),
        ..CardDef::vanilla("X23")
    },
    CardDef {
        on_discard: Some(helicarrier),
        ..CardDef::vanilla("Helicarrier")
    },
    CardDef {
        on_activate: Some(spider_ham),
        ..CardDef::vanilla("SpiderHam")
    },
    CardDef {
        on_event: Some(sub_mariner),
        ..CardDef::vanilla("SubMariner")
    },
    CardDef {
        on_activate: Some(boomerang_activate),
        on_event: Some(boomerang_returned),
        ..CardDef::vanilla("Boomerang")
    },
    CardDef {
        reveal: Some(doctor_octopus_fractured),
        ..CardDef::vanilla("DoctorOctopusFracturedFrontier")
    },
    CardDef {
        reveal: Some(sersi),
        ..CardDef::vanilla("Sersi")
    },
    CardDef::vanilla("BannerHulk"),
];

// HulkOut 0x7e76ff8 rolls [0, 100) only with unspent energy; a roll
// below 33 morphs the card into BannerHulk.
fn bruce_banner(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if game.players[owner].energy != 0 && game.rng.borrow_mut().randbelow(100) < 33 {
        game.transform_card(card, "BannerHulk");
    }
}

// AddCards 0x7f39be8 selects three distinct one-cost characters before
// attempting one creation per lane; the random pool is implemented cards only.
fn shanna(game: &mut Game<'_>, card: usize) {
    let mut selected = [None; 3];
    for index in 0..3 {
        selected[index] = game.random_card(|row| {
            row.cost == 1
                && row.card_type == "Character"
                && !selected[..index].contains(&Some(row.carddefid.as_str()))
        });
    }
    let owner = game.cards[card].owner;
    for (lane, id) in selected.into_iter().enumerate() {
        if let Some(id) = id {
            game.add_created(id, lane, owner);
        }
    }
}

// AddCards 0x7ebf8d0 selects distinct random definitions for every free
// hand slot before creating them. The random pool contains implemented cards.
fn helicarrier(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    game.push_discard(owner, card);
    let count = 7usize.saturating_sub(game.players[owner].hand.len());
    let mut selected = [None; 7];
    for index in 0..count {
        selected[index] =
            game.random_card(|row| !selected[..index].contains(&Some(row.carddefid.as_str())));
    }
    for id in selected.into_iter().flatten() {
        if game.players[owner].hand.len() == 7 {
            break;
        }
        let created = game.create_card(id, owner);
        game.add_hand(owner, created, false);
    }
}

// ReplaceText 0x7f4af20 chooses a random opposing character in hand;
// predicate 0x7f4b538 tests character type, then Pig text replaces its text.
fn spider_ham(game: &mut Game<'_>, card: usize) {
    let opponent = 1 - game.cards[card].owner;
    let mut candidates = [0; 7];
    let mut count = 0;
    for &target in &game.players[opponent].hand {
        if game.is_character(target) {
            candidates[count] = target;
            count += 1;
        }
    }
    if count != 0 {
        let target = candidates[game.rng.borrow_mut().randbelow(count)];
        game.remove_text(target);
    }
}

// PutCardBack 0x7e8d144 puts the leftmost hand card on top of the deck,
// creates a random character in hand, then gives both +3 Power.
fn doctor_octopus_fractured(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if game.players[owner].hand.is_empty() {
        return;
    }
    let target = game.take_hand(owner, 0);
    game.put_deck(owner, 0, target);
    let created = game
        .random_card(|row| row.card_type == "Character")
        .map(|id| {
            let created = game.create_card(id, owner);
            game.add_hand(owner, created, false);
            created
        });
    game.add_power(target, 3, card);
    if let Some(created) = created {
        game.add_power(created, 3, card);
    }
}

// TransformCard 0x7f37018 snapshots the other revealed allied characters,
// then morphs each into a random character costing one more; the client
// static exclusion list contains Sersi and Morph.
fn sersi(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&target| {
            target != card
                && game.cards[target].owner == owner
                && game.cards[target].revealed
                && game.is_character(target)
        })
        .collect();
    for target in targets {
        let cost = game.card_cost(target) + 1;
        if let Some(id) = game.random_card(|row| {
            row.cost == cost
                && row.card_type == "Character"
                && row.carddefid != "Sersi"
                && row.carddefid != "Morph"
        }) {
            game.transform_card_without_modifiers(target, id);
        }
    }
}

// MoveToHand 0x7e765e4; StealPower 0x7e75fcc responds to this card's
// board-to-hand transition, rolls four times with replacement over opposing
// hand characters, then steals the grouped amounts in first-selected order.
fn boomerang_activate(game: &mut Game<'_>, card: usize) {
    game.return_board_card_to_hand(card);
}

fn boomerang_returned(game: &mut Game<'_>, card: usize, event: Event) {
    if !matches!(event, Event::CardReturnedToHand { card: target } if target == card) {
        return;
    }
    let opponent = 1 - game.cards[card].owner;
    let mut targets = [0; 7];
    let mut len = 0;
    for &target in &game.players[opponent].hand {
        if game.is_character(target) {
            targets[len] = target;
            len += 1;
        }
    }
    if len == 0 {
        return;
    }
    let mut picks = [(0, 0); 4];
    let mut used = 0;
    for _ in 0..4 {
        let target = targets[game.rng.borrow_mut().randbelow(len)];
        if let Some((_, amount)) = picks[..used].iter_mut().find(|(id, _)| *id == target) {
            *amount += 1;
        } else {
            picks[used] = (target, 1);
            used += 1;
        }
    }
    for &(target, amount) in &picks[..used] {
        game.steal_power(target, card, amount, card);
    }
}

// AddCardAndBoost 0x7ccad68 reacts to its own positive permanent Power
// increase in hand/deck and creates a random one-cost card with that increase.
fn sub_mariner(game: &mut Game<'_>, card: usize, event: Event) {
    let Event::Empower {
        card: target,
        amount,
        ..
    } = event
    else {
        return;
    };
    let owner = game.cards[card].owner;
    if target != card
        || amount <= 0
        || !(game.players[owner].hand.contains(&card) || game.players[owner].deck.contains(&card))
        || game.players[owner].hand.len() >= 7
    {
        return;
    }
    if let Some(id) = game.random_card(|row| {
        row.cost == 1 && row.card_type == "Character" && row.carddefid != "SubMariner"
    }) {
        let created = game.create_card(id, owner);
        game.add_hand(owner, created, false);
        game.add_power(created, amount, card);
    }
}

// SpawnAtRandomLocation 0x7cf6368 resurrects at a random spawnable
// location and schedules one energy for the next turn, even if none exists.
fn x23_discard(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    game.push_discard(owner, card);
    x23_destroy(game, card);
}

fn x23_destroy(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let mut lanes = [0; 3];
    let mut count = 0;
    for lane in 0..3 {
        if game.can_add(card, lane, owner) {
            lanes[count] = lane;
            count += 1;
        }
    }
    if count != 0 {
        let lane = lanes[game.rng.borrow_mut().randbelow(count)];
        game.resurrect_from_zone(card, lane, owner);
    }
    game.schedule_energy(owner, game.turn.saturating_add(1), 1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    #[test]
    fn bruce_banner_requires_unspent_energy() {
        let mut rng = PythonRandom::seed(1);
        let mut roll_rng = PythonRandom::seed(1);
        let deck = [
            "Hulk",
            "Wasp",
            "Cyclops",
            "Shocker",
            "Medusa",
            "IronMan",
            "Hawkeye",
            "Sentinel",
            "TheThing",
            "MistyKnight",
            "Abomination",
            "Quicksilver",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let card = game.create_card("BruceBanner", 0);
        game.players[0].energy = 0;
        bruce_banner(&mut game, card);
        assert_eq!(game.cards[card].def.id, "BruceBanner");
        game.players[0].energy = 1;
        *game.rng.borrow_mut() = crate::game::GameRandom::Borrowed(&mut roll_rng);
        bruce_banner(&mut game, card);
        assert_eq!(game.cards[card].def.id, "BannerHulk");
    }

    #[test]
    fn shanna_creates_distinct_one_cost_characters_at_each_lane() {
        let mut rng = PythonRandom::seed(2);
        let deck = [
            "Hulk",
            "Wasp",
            "Cyclops",
            "Shocker",
            "Medusa",
            "IronMan",
            "Hawkeye",
            "Sentinel",
            "TheThing",
            "MistyKnight",
            "Abomination",
            "Quicksilver",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let card = game.create_card("Shanna", 0);
        shanna(&mut game, card);
        let ids: Vec<_> = game
            .lanes
            .iter()
            .map(|lane| game.cards[lane.cards[0]].def.id)
            .collect();
        assert_eq!(ids.len(), 3);
        assert_ne!(ids[0], ids[1]);
        assert_ne!(ids[0], ids[2]);
        assert_ne!(ids[1], ids[2]);
        for lane in &game.lanes {
            let created = lane.cards[0];
            assert_eq!(game.card_cost(created), 1);
            assert!(game.is_character(created));
        }
    }

    #[test]
    fn x23_discard_resurrects_and_schedules_energy() {
        let mut rng = PythonRandom::seed(3);
        let deck = [
            "Hulk",
            "Wasp",
            "Cyclops",
            "Shocker",
            "Medusa",
            "IronMan",
            "Hawkeye",
            "Sentinel",
            "TheThing",
            "MistyKnight",
            "Abomination",
            "Quicksilver",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let card = game.create_card("X23", 0);
        game.players[0].hand.push(card);
        game.discard_card(0, card);
        assert!(game.cards[card].lane.is_some());
        assert!(!game.players[0].discard.contains(&card));
        assert_eq!(game.players[0].scheduled_energy, vec![(game.turn + 1, 1)]);
    }

    #[test]
    fn helicarrier_fills_hand_with_distinct_cards() {
        let mut rng = PythonRandom::seed(4);
        let deck = [
            "Hulk",
            "Wasp",
            "Cyclops",
            "Shocker",
            "Medusa",
            "IronMan",
            "Hawkeye",
            "Sentinel",
            "TheThing",
            "MistyKnight",
            "Abomination",
            "Quicksilver",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let card = game.create_card("Helicarrier", 0);
        game.players[0].hand.clear();
        game.players[0].hand.push(card);
        game.discard_card(0, card);
        assert_eq!(game.players[0].hand.len(), 7);
        let mut ids: Vec<_> = game.players[0]
            .hand
            .iter()
            .map(|&id| game.cards[id].def.id)
            .collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 7);
        assert!(game.players[0].discard.contains(&card));
    }

    #[test]
    fn spider_ham_removes_text_from_opponent_character() {
        let mut rng = PythonRandom::seed(5);
        let deck = [
            "Hulk",
            "Wasp",
            "Cyclops",
            "Shocker",
            "Medusa",
            "IronMan",
            "Hawkeye",
            "Sentinel",
            "TheThing",
            "MistyKnight",
            "Abomination",
            "Quicksilver",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("SpiderHam", 0);
        let target = game.create_card("IronMan", 1);
        game.players[1].hand.clear();
        game.players[1].hand.push(target);
        spider_ham(&mut game, source);
        assert!(game.cards[target].abilities_removed);
        assert_eq!(game.cards[target].def.id, "IronMan");
    }

    #[test]
    fn sub_mariner_creates_buffed_card_after_empower_in_deck() {
        let mut rng = PythonRandom::seed(6);
        let deck = [
            "Hulk",
            "Wasp",
            "Cyclops",
            "Shocker",
            "Medusa",
            "IronMan",
            "Hawkeye",
            "Sentinel",
            "TheThing",
            "MistyKnight",
            "Abomination",
            "Quicksilver",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("SubMariner", 0);
        game.players[0].hand.clear();
        game.players[0].deck.push(source);
        game.add_power(source, 4, source);
        assert_eq!(game.players[0].hand.len(), 1);
        let created = game.players[0].hand[0];
        assert_ne!(game.cards[created].def.id, "SubMariner");
        assert_eq!(game.card_cost(created), 1);
        let base = crate::data::cards()
            .into_iter()
            .find(|row| row.carddefid == game.cards[created].def.id)
            .unwrap()
            .power;
        assert_eq!(game.cards[created].permanent, base + 4);
    }

    #[test]
    fn boomerang_bounces_and_steals_four_from_opposing_hand() {
        let mut rng = PythonRandom::seed(7);
        let deck = [
            "Hulk",
            "Wasp",
            "Cyclops",
            "Shocker",
            "Medusa",
            "IronMan",
            "Hawkeye",
            "Sentinel",
            "TheThing",
            "MistyKnight",
            "Abomination",
            "Quicksilver",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Boomerang", 0);
        let target = game.create_card("Hulk", 1);
        let original = game.cards[target].permanent;
        game.players[0].hand.clear();
        game.players[1].hand.clear();
        game.players[1].hand.push(target);
        game.add_card(source, 0, 0, true);
        boomerang_activate(&mut game, source);
        assert!(game.players[0].hand.contains(&source));
        assert_eq!(game.cards[source].permanent, 6);
        assert_eq!(game.cards[target].permanent, original - 4);
    }

    #[test]
    fn doctor_octopus_puts_leftmost_on_top_and_buffs_both() {
        let mut rng = PythonRandom::seed(8);
        let deck = [
            "Hulk",
            "Wasp",
            "Cyclops",
            "Shocker",
            "Medusa",
            "IronMan",
            "Hawkeye",
            "Sentinel",
            "TheThing",
            "MistyKnight",
            "Abomination",
            "Quicksilver",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("DoctorOctopusFracturedFrontier", 0);
        let target = game.create_card("Hulk", 0);
        let original = game.cards[target].permanent;
        game.players[0].hand.clear();
        game.players[0].hand.push(target);
        doctor_octopus_fractured(&mut game, source);
        assert_eq!(game.players[0].deck[0], target);
        assert_eq!(game.cards[target].permanent, original + 3);
        let created = game.players[0].hand[0];
        let base = crate::data::cards()
            .into_iter()
            .find(|row| row.carddefid == game.cards[created].def.id)
            .unwrap()
            .power;
        assert_eq!(game.cards[created].permanent, base + 3);
    }

    #[test]
    fn sersi_transforms_other_revealed_allies_one_cost_higher() {
        let mut rng = PythonRandom::seed(9);
        let deck = [
            "Hulk",
            "Wasp",
            "Cyclops",
            "Shocker",
            "Medusa",
            "IronMan",
            "Hawkeye",
            "Sentinel",
            "TheThing",
            "MistyKnight",
            "Abomination",
            "Quicksilver",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Sersi", 0);
        let target = game.create_card("Wasp", 0);
        game.add_card(source, 0, 0, true);
        game.add_card(target, 0, 0, true);
        sersi(&mut game, source);
        assert_eq!(game.cards[source].def.id, "Sersi");
        assert_eq!(game.card_cost(target), 1);
        assert_ne!(game.cards[target].def.id, "Wasp");
    }
}
