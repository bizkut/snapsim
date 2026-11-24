//! Rules requiring original-deck entity provenance.
use super::CardDef;
use crate::game::{Event, Game, Stat, StatOperation};

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        reveal: Some(pixie),
        ..CardDef::vanilla("Pixie")
    },
    CardDef {
        on_event: Some(thanos),
        ..CardDef::vanilla("Thanos")
    },
    CardDef {
        late_ongoing: Some(ajax),
        ..CardDef::vanilla("Ajax")
    },
    CardDef {
        on_activate: Some(scarlet_spider),
        ..CardDef::vanilla("ScarletSpider")
    },
    CardDef::vanilla("ScarletSpiderClone"),
];
pub(crate) const TOKENS: &[&str] = &["ScarletSpiderClone"];
/// Card and location definitions with implemented random pools admitting Pixie.
pub(crate) const RANDOM_CHARACTER_CREATORS: &[&str] = &[
    "Agent13",
    "Arishem",
    "Askanison",
    "MariaHill",
    "Polymorph",
    "Helicarrier",
    "DoctorOctopusFracturedFrontier",
    "Sersi",
    "RedShift",
    "TheHub",
    "Triskelion",
    "Tarnax",
    "CelestialBurialGround",
    "XMansion",
];

/// Deck tracker membership is only decisive if an ID occurs in exactly one
/// source. Duplicate IDs across sources cannot be attributed by identity alone.
pub(crate) fn deck_origins(
    remaining: &[serde_json::Value],
    starting: Option<&[serde_json::Value]>,
    added: Option<&[serde_json::Value]>,
) -> Vec<Option<bool>> {
    let Some(starting) = starting else {
        return vec![None; remaining.len()];
    };
    let added = added.unwrap_or_default();
    remaining
        .iter()
        .map(|id| match (starting.contains(id), added.contains(id)) {
            (true, false) => Some(false),
            (false, true) => Some(true),
            _ => None,
        })
        .collect()
}

// AjaxBehavior::GetPowerChange 0x7e638f4 counts each revealed card once
// if power reduction is allowed and either ledger contains a negative Add.
// The client predicates are 0x7e64064 and 0x7e640ec.
fn ajax(game: &mut Game<'_>, card: usize) -> bool {
    if game.cards[card].lane.is_none() {
        return false;
    }
    let count = game
        .lanes
        .iter()
        .flat_map(|lane| &lane.cards)
        .filter(|&&id| {
            let target = &game.cards[id];
            target.revealed
                && game.can_reduce_card_power(id)
                && (target
                    .power_modifiers
                    .iter()
                    .any(|m| m.operation == StatOperation::Add && m.value < 0)
                    || target.stat_updates.iter().any(|u| {
                        u.stat == Stat::Power && u.operation == StatOperation::Add && u.value < 0
                    }))
        })
        .count() as i32;
    game.add_ongoing_power(card, count, game.cards[card].owner);
    true
}

// CreateCopy 0x7f2f4c4 uses GetPowerWithModifiersOnly 0x7cb0fc0, not
// current ongoing/location power, and sets it on a new token in another lane.
fn scarlet_spider(game: &mut Game<'_>, card: usize) {
    let Some(origin) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let mut power = game.cards[card]
        .base_power_override
        .unwrap_or_else(|| game.printed_power(card));
    let can_reduce = game.can_reduce_card_power(card);
    for modifier in &game.cards[card].power_modifiers {
        let next = match modifier.operation {
            StatOperation::Add => power.saturating_add(modifier.value),
            StatOperation::Multiply => power.saturating_mul(modifier.value),
            StatOperation::Set => modifier.value,
        };
        if next >= power || can_reduce {
            power = next;
        }
    }
    let copy = game.create_card("ScarletSpiderClone", owner);
    let mut lanes = [0; 2];
    let mut count = 0;
    for lane in 0..3 {
        if lane != origin && game.can_add(copy, lane, owner) {
            lanes[count] = lane;
            count += 1;
        }
    }
    // GetRandomLocationToSpawnCard (0x7db2b64) falls back to all other
    // locations when none can spawn, still consuming a random roll.
    if count == 0 {
        let _ = game.rng.borrow_mut().randbelow(2);
        game.cards.pop();
        return;
    }
    let lane = lanes[game.rng.borrow_mut().randbelow(count)];
    game.cards[copy].permanent = power;
    game.cards[copy]
        .power_modifiers
        .push(crate::game::CardPowerModifier {
            operation: StatOperation::Set,
            value: power,
            source: Some(owner),
        });
    game.add_card(copy, lane, owner, true);
}

// AddInfinityStones 0x7cd5e3c creates the configured tokens in order, then
// shuffles the owner's deck once before the opening hand is drawn.
pub(crate) fn thanos(game: &mut Game<'_>, card: usize, event: Event) {
    if !matches!(event, Event::GameStart) {
        return;
    }
    let seat = game.cards[card].owner;
    for id in [
        "SpaceStone",
        "RealityStone",
        "TimeStone",
        "MindStone",
        "PowerStone",
        "SoulStone",
    ] {
        let stone = game.create_card(id, seat);
        game.cards[stone].created_by_card_id = Some(game.cards[card].def.id.to_owned());
        game.cards[stone].created_by_player = Some(seat);
        game.finish_creation_zone(stone, false, seat);
        game.push_deck(seat, stone);
    }
    game.shuffle_deck(seat);
}

// ShuffleCardCosts 0x7f15370 filters the current deck with StartedInDeck
// (predicate 0x7f159c4), shuffles their DefCost values (0x7f159b0), and sets
// each cost permanently. Original entities remain original after moving or morphing.
fn pixie(game: &mut Game<'_>, card: usize) {
    let seat = game.cards[card].owner;
    let original = game.players[seat].starting_deck_instances.as_ref();
    let targets: Vec<_> = game.players[seat]
        .deck
        .iter()
        .copied()
        .filter(|&id| original.map_or(!game.cards[id].created, |entities| entities.contains(&id)))
        .collect();
    let mut costs: Vec<_> = targets.iter().map(|&id| game.printed_cost(id)).collect();
    // Game::Shuffle<T> (0x4b12f30) rolls Roll(i, count) for i in 0..count - 1.
    for i in 0..costs.len().saturating_sub(1) {
        let j = i + game.rng.borrow_mut().randbelow(costs.len() - i);
        costs.swap(i, j);
    }
    for (target, cost) in targets.into_iter().zip(costs) {
        game.set_cost(target, cost, card);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    #[test]
    fn ajax_counts_afflicted_cards_once_across_both_ledgers() {
        let mut rng = PythonRandom::seed(23);
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
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let ajax_id = game.create_card("Ajax", 0);
        let target = game.create_card("Hulk", 1);
        assert!(game.add_card(ajax_id, 0, 0, true));
        assert!(game.add_card(target, 1, 1, true));
        game.add_power(target, -2, 0);
        game.cards[ajax_id].ongoing = 0;
        ajax(&mut game, ajax_id);
        assert_eq!(game.cards[ajax_id].ongoing, 1);
    }

    #[test]
    fn ajax_counts_location_affliction_after_luke_init() {
        let mut rng = PythonRandom::seed(24);
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
        let locations = ["NegativeZone", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        game.lanes[0].def = crate::locations::get("NegativeZone").unwrap();
        game.lanes[1].def = crate::locations::get("Ruins").unwrap();
        game.lanes[2].def = crate::locations::get("Ruins").unwrap();
        for lane in &mut game.lanes {
            lane.revealed = true;
        }
        let ajax_id = game.create_card("Ajax", 0);
        let target = game.create_card("Hulk", 1);
        assert!(game.add_card(ajax_id, 1, 0, false));
        assert!(game.add_card(target, 0, 1, false));
        game.ongoing();
        assert_eq!(game.cards[ajax_id].ongoing, 1);
        let luke = game.create_card("LukeCage", 1);
        assert!(game.add_card(luke, 2, 1, false));
        game.ongoing();
        assert_eq!(game.cards[ajax_id].ongoing, 0);
        assert_eq!(game.cards[target].power(), game.printed_power(target));
    }

    #[test]
    fn scarlet_spider_copies_modifiers_not_ongoing_power() {
        let mut rng = PythonRandom::seed(22);
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
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let spider = game.create_card("ScarletSpider", 0);
        assert!(game.add_card(spider, 0, 0, true));
        game.add_power(spider, -2, 0);
        game.cards[spider].ongoing = 10;
        scarlet_spider(&mut game, spider);
        let clone = game
            .cards
            .iter()
            .find(|card| card.def.id == "ScarletSpiderClone")
            .unwrap();
        assert_eq!(clone.permanent, 3);
        assert_ne!(clone.lane, Some(0));
    }

    #[test]
    fn scarlet_spider_rolls_fallback_when_spawns_are_blocked() {
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
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut rng = PythonRandom::seed(23);
        let mut control_rng = PythonRandom::seed(23);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let control = Game::new([&deck, &deck], &locations, &mut control_rng).unwrap();
        let spider = game.create_card("ScarletSpider", 0);
        let stardust = game.create_card("Stardust", 0);
        game.add_card(spider, 0, 0, true);
        game.add_card(stardust, 0, 0, true);
        assert!(!game.spawns_allowed());
        control.rng.borrow_mut().randbelow(2);
        scarlet_spider(&mut game, spider);
        assert!(
            !game
                .cards
                .iter()
                .any(|card| card.def.id == "ScarletSpiderClone")
        );
        assert_eq!(
            game.rng.borrow_mut().randbelow(10000),
            control.rng.borrow_mut().randbelow(10000)
        );
    }

    #[test]
    fn deck_tracker_origin_rejects_ambiguous_duplicate_ids() {
        use serde_json::json;
        let starting = vec![json!("Wasp"), json!("Hulk")];
        let added = vec![json!("Rock"), json!("Hulk")];
        let remaining = vec![json!("Wasp"), json!("Rock"), json!("Hulk"), json!("Other")];
        assert_eq!(
            deck_origins(&remaining, Some(&starting), Some(&added)),
            vec![Some(false), Some(true), None, None]
        );
        assert_eq!(deck_origins(&remaining, None, None), vec![None; 4]);
    }

    #[test]
    fn random_card_calls_have_reviewed_pixie_pool_classification() {
        // Each random_card call is reviewed against Pixie's 2-Cost Character
        // definition. Helpers with multiple callers list every eligible owner.
        let admits = [
            ("random_to_hand", &["MariaHill"][..]),
            ("arishem_start", &["Arishem"][..]),
            ("askanison_activate", &["Askanison"][..]),
            ("polymorph", &["Polymorph"][..]),
            ("helicarrier", &["Helicarrier"][..]),
            (
                "doctor_octopus_fractured",
                &["DoctorOctopusFracturedFrontier"][..],
            ),
            ("sersi", &["Sersi"][..]),
            ("red_shift", &["RedShift"][..]),
            ("the_hub", &["TheHub"][..]),
            ("triskelion", &["Triskelion"][..]),
            ("tarnax", &["Tarnax"][..]),
            ("celestial_burial_ground", &["CelestialBurialGround"][..]),
            ("x_mansion", &["XMansion"][..]),
        ];
        let excludes = [
            ("nick_fury", "six-cost only"),
            ("agent_coulson", "four- and five-cost only"),
            ("shanna", "one-cost only"),
            ("sub_mariner", "one-cost only"),
            ("camp_lehigh", "three-cost only"),
            ("great_portal", "at least ten power"),
            ("black_vortex", "six-cost only"),
            ("valentina", "Thunderbolts faction excludes Pixie"),
            ("scarlet_witch", "Brotherhood faction excludes Pixie"),
            ("kvch", "activate cards only"),
            ("cave_of_the_dragon", "at least five cost"),
            ("first_to_fill", "six-cost only"),
            (
                "kang",
                "three-cost or greater text sources; Pixie costs two",
            ),
            ("hulkling", "six-cost text sources; Pixie costs two"),
            (
                "nightmare",
                "copies text onto an existing hand card; the Pixie template never enters play",
            ),
        ];
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        for dir in ["cards", "locations"] {
            for entry in std::fs::read_dir(root.join(dir)).unwrap() {
                let path = entry.unwrap().path();
                if path.extension().is_none_or(|ext| ext != "rs") {
                    continue;
                }
                let text = std::fs::read_to_string(&path).unwrap();
                let mut function = "";
                for line in text.split("#[cfg(test)]").next().unwrap().lines() {
                    let line = line.trim();
                    if let Some((_, rest)) = line.split_once("fn ") {
                        function = rest.split('(').next().unwrap_or("");
                    }
                    if !line.contains(".random_card(") {
                        continue;
                    }
                    if let Some((_, ids)) = admits.iter().find(|(name, _)| *name == function) {
                        assert!(ids.iter().all(|id| RANDOM_CHARACTER_CREATORS.contains(id)));
                    } else {
                        assert!(
                            excludes
                                .iter()
                                .any(|(name, reason)| *name == function && !reason.is_empty()),
                            "unreviewed random-card pool in {}: {function}",
                            path.display()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn thanos_creates_six_distinct_stones_before_opening_draw() {
        let mut rng = PythonRandom::seed(8);
        let deck = [
            "Thanos",
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
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        for player in &game.players {
            let stones: Vec<_> = player
                .hand
                .iter()
                .chain(&player.deck)
                .filter(|&&id| game.cards[id].created_by_card_id.as_deref() == Some("Thanos"))
                .collect();
            assert_eq!(stones.len(), 6);
            assert!(stones.iter().all(|&&id| game.cards[id].created));
        }
    }

    #[test]
    fn pixie_shuffles_costs_with_the_forward_game_shuffle() {
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
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut rng = PythonRandom::seed(61);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let mut control = game.rng.borrow().clone_rng();
        let source = game.create_card("Pixie", 0);
        let targets = game.players[0].deck.clone();
        let mut costs: Vec<_> = targets.iter().map(|&id| game.printed_cost(id)).collect();
        pixie(&mut game, source);
        // Game::Shuffle<T> (0x4b12f30): Roll(i, count) for i in 0..count - 1.
        for i in 0..costs.len() - 1 {
            let j = i + control.randbelow(costs.len() - i);
            costs.swap(i, j);
        }
        let after: Vec<_> = targets.iter().map(|&id| game.card_cost(id)).collect();
        assert_eq!(after, costs);
        assert_eq!(
            game.rng.borrow_mut().randbelow(10000),
            control.randbelow(10000)
        );
    }

    #[test]
    fn pixie_shuffles_only_original_entities_still_in_deck() {
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
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let source = game.create_card("Pixie", 0);
        let transformed = game.players[0].deck[0];
        assert!(game.transform_card(transformed, "Hulk"));
        let created = game.copy_card(transformed, 0);
        game.players[0].deck.push(created);
        let original = game.players[0].starting_deck_instances.as_ref().unwrap();
        let targets: Vec<_> = game.players[0]
            .deck
            .iter()
            .copied()
            .filter(|id| original.contains(id))
            .collect();
        assert!(targets.contains(&transformed));
        assert!(!targets.contains(&created));
        let mut before: Vec<_> = targets.iter().map(|&id| game.printed_cost(id)).collect();
        before.sort_unstable();
        game.set_cost(transformed, 99, source);
        let created_cost = game.cards[created].cost;
        pixie(&mut game, source);
        let mut after: Vec<_> = targets.iter().map(|&id| game.cards[id].cost).collect();
        after.sort_unstable();
        assert_eq!(before, after);
        assert_eq!(game.cards[created].cost, created_cost);
        // Restored positions use per-card creation flags when entity membership
        // cannot be reconstructed.
        game.players[0].starting_deck_instances = None;
        pixie(&mut game, source);
        let mut restored: Vec<_> = targets.iter().map(|&id| game.cards[id].cost).collect();
        restored.sort_unstable();
        assert_eq!(before, restored);
        assert_eq!(game.cards[created].cost, created_cost);
    }

    #[test]
    fn pixie_search_projection_retains_original_entities() {
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
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let created = game.create_card("Rock", 0);
        game.players[0].deck.push(created);
        let position = crate::compare::search_position(&game, 0, None, None, None);
        let originals: Vec<_> = game.players[0]
            .deck
            .iter()
            .filter(|&&id| !game.cards[id].created)
            .map(|&id| game.cards[id].def.id)
            .collect();
        let mut rng = PythonRandom::seed(6);
        let mut restored = crate::position::build_game(&position, &mut rng).unwrap();
        let targets: Vec<_> = restored.players[0]
            .deck
            .iter()
            .copied()
            .filter(|&id| !restored.cards[id].created)
            .collect();
        let mut restored_ids: Vec<_> = targets
            .iter()
            .map(|&id| restored.cards[id].def.id)
            .collect();
        let mut originals = originals;
        originals.sort_unstable();
        restored_ids.sort_unstable();
        assert_eq!(originals, restored_ids);
        let created = restored.players[0]
            .deck
            .iter()
            .copied()
            .find(|&id| restored.cards[id].created)
            .unwrap();
        let cost = restored.cards[created].cost;
        let source = restored.create_card("Pixie", 0);
        pixie(&mut restored, source);
        assert_eq!(restored.cards[created].cost, cost);
    }
}
