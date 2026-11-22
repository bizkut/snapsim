//! Meta-deck cards, batch H3: rules from the 59.15.1 client.
use super::CardDef;
use crate::game::Game;

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        ongoing_ability: true,
        grants_move_from: Some(madame_web),
        ..CardDef::vanilla("MadameWeb")
    },
    CardDef {
        in_hand_ongoing: Some(mockingbird),
        ..CardDef::vanilla("Mockingbird")
    },
    CardDef {
        initial_state: Some(|| Box::new(None::<u8>)),
        start_turn: Some(nebula_enable),
        pre_end_turn: Some(nebula),
        ..CardDef::vanilla("Nebula")
    },
    CardDef {
        initial_state: Some(|| Box::new(false)),
        end_turn: Some(thing),
        ..CardDef::vanilla("TheThingFirstSteps")
    },
    CardDef {
        reveal: Some(vampire_enchantress),
        ..CardDef::vanilla("VampireEnchantress")
    },
];

// MadameWebBehavior::SelectCardsAtSameLocation (0x7eebb74) and
// NoCardsMovedThisTurn (0x7eebda8).
fn madame_web(game: &Game<'_>, source: usize, target: usize) -> bool {
    let owner = game.cards[source].owner;
    let Some(lane) = game.cards[source].lane else {
        return false;
    };
    source != target
        && game.cards[target].owner == owner
        && game.players[owner].move_from_turn[lane] != Some(game.turn)
}

// MockingbirdBehavior::DiscountSelf (0x7efcc0c): revealed created allies only.
fn mockingbird(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let count = game
        .lanes
        .iter()
        .flat_map(|lane| &lane.cards)
        .filter(|&&id| {
            game.cards[id].owner == owner && game.cards[id].revealed && game.cards[id].created
        })
        .count() as i32;
    game.add_ongoing_cost(card, -count, owner);
}

// NebulaBehavior::EnableAbility (0x7f0abd0) installs a pre-end trigger.
fn nebula_enable(game: &mut Game<'_>, card: usize) {
    if let Some(state) = game.cards[card]
        .state
        .as_mut()
        .and_then(|s| s.as_any_mut().downcast_mut::<Option<u8>>())
    {
        *state = Some(game.turn);
    }
}

// NebulaBehavior::BuffItselfIfNoOpponentPlays (0x7f0ad44).
fn nebula(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let enabled = game.cards[card]
        .state
        .as_ref()
        .and_then(|s| s.as_any().downcast_ref::<Option<u8>>())
        .copied()
        .flatten();
    if enabled == Some(game.turn) && !game.played_at(1 - owner, lane) {
        game.add_power(card, 2, owner);
    }
}

// TheThingFirstStepsBehavior::GainPowerAndCheckDestroyCard (0x7cdb8ac):
// the once-only flag is consumed even if no enemy qualifies.
fn thing(game: &mut Game<'_>, card: usize) {
    if game.cards[card].abilities_removed {
        return;
    }
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    game.add_power(card, 2, owner);
    if game.has_space(lane, owner, 0) {
        return;
    }
    let used = game.cards[card]
        .state
        .as_mut()
        .and_then(|s| s.as_any_mut().downcast_mut::<bool>())
        .unwrap();
    if *used {
        return;
    }
    *used = true;
    game.ongoing();
    let power = game.cards[card].power();
    let targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| {
            game.cards[id].owner != owner
                && game.cards[id].revealed
                && game.cards[id].power() < power
        })
        .collect();
    if !targets.is_empty() {
        let index = game.rng.borrow_mut().randbelow(targets.len());
        game.destroy_card(targets[index]);
    }
}

// VampireEnchantressBehavior::StealForEachDistinctCost (0x7ce9f38);
// its selector (0x7cea690) reads each revealed ally's printed cost.
fn vampire_enchantress(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let mut costs = Vec::new();
    for id in game.lanes.iter().flat_map(|l| l.cards.iter().copied()) {
        if game.cards[id].owner == owner && (game.cards[id].revealed || id == card) {
            let cost = game.printed_cost(id);
            if !costs.contains(&cost) {
                costs.push(cost);
            }
        }
    }
    let enemies: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| game.cards[id].owner != owner && game.cards[id].revealed)
        .collect();
    if enemies.is_empty() {
        return;
    }
    let mut amounts = Vec::<(usize, i32)>::new();
    for _ in costs {
        let target = enemies[game.rng.borrow_mut().randbelow(enemies.len())];
        if let Some((_, amount)) = amounts.iter_mut().find(|(id, _)| *id == target) {
            *amount += 1;
        } else {
            amounts.push((target, 1));
        }
    }
    game.steal_power_weighted(&amounts, card, owner);
}

pub(crate) static TOKENS: &[&str] = &[];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{locations, random::PythonRandom};

    fn game(rng: &mut PythonRandom) -> Game<'_> {
        let deck = [
            "Abomination",
            "Cyclops",
            "Hulk",
            "MistyKnight",
            "Shocker",
            "TheThing",
            "Wasp",
            "Quicksilver",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
        ]
        .map(str::to_owned);
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, rng).unwrap();
        for lane in &mut game.lanes {
            lane.def = locations::get("Ruins").unwrap();
            lane.revealed = true;
        }
        game.turn = 2;
        game
    }

    fn place(game: &mut Game<'_>, id: &str, owner: usize, lane: usize) -> usize {
        let card = game.create_card(id, owner);
        game.cards[card].lane = Some(lane);
        game.cards[card].revealed = true;
        game.lanes[lane].cards.push(card);
        card
    }

    #[test]
    fn web_grants_one_move_per_lane_per_turn_and_is_suppressed() {
        let mut rng = PythonRandom::seed(1);
        let mut game = game(&mut rng);
        let web = place(&mut game, "MadameWeb", 0, 0);
        let first = place(&mut game, "Hulk", 0, 0);
        let second = place(&mut game, "Wasp", 0, 0);
        assert!(!game.can_player_move(web, 1, 0));
        assert!(game.can_player_move(first, 1, 0));
        assert!(game.move_player_card(first, 1, 0));
        assert!(!game.can_player_move(second, 1, 0));
        game.turn += 1;
        assert!(game.can_player_move(second, 1, 0));
        game.lanes[0].def = locations::get("IsleOfSilence").unwrap();
        assert!(!game.can_player_move(second, 1, 0));
    }

    #[test]
    fn mockingbird_counts_only_revealed_created_allies() {
        let mut rng = PythonRandom::seed(1);
        let mut game = game(&mut rng);
        let bird = game.create_card("Mockingbird", 0);
        game.players[0].hand.push(bird);
        let created = place(&mut game, "Wasp", 0, 0);
        game.cards[created].created = true;
        let hidden = place(&mut game, "Hulk", 0, 1);
        game.cards[hidden].created = true;
        game.cards[hidden].revealed = false;
        let enemy = place(&mut game, "Wasp", 1, 0);
        game.cards[enemy].created = true;
        game.ongoing();
        assert_eq!(game.card_cost(bird), 5);
    }

    #[test]
    fn nebula_requires_start_trigger_and_no_opponent_play() {
        let mut rng = PythonRandom::seed(1);
        let mut game = game(&mut rng);
        let neb = place(&mut game, "Nebula", 0, 0);
        nebula(&mut game, neb);
        assert_eq!(game.cards[neb].power(), 1);
        nebula_enable(&mut game, neb);
        nebula(&mut game, neb);
        assert_eq!(game.cards[neb].power(), 3);
        let enemy = place(&mut game, "Wasp", 1, 1);
        game.cards[enemy].turn_played = game.turn;
        game.played[1].push((enemy, 0));
        nebula(&mut game, neb);
        assert_eq!(game.cards[neb].power(), 3);
    }

    #[test]
    fn thing_consumes_full_lane_trigger_even_without_target() {
        let mut rng = PythonRandom::seed(1);
        let mut game = game(&mut rng);
        let card = place(&mut game, "TheThingFirstSteps", 0, 0);
        for _ in 0..3 {
            place(&mut game, "Wasp", 0, 0);
        }
        thing(&mut game, card);
        let enemy = place(&mut game, "Wasp", 1, 0);
        thing(&mut game, card);
        assert_eq!(game.cards[enemy].lane, Some(0));
        assert_eq!(game.cards[card].power(), 10);
    }

    #[test]
    fn vampire_counts_distinct_printed_costs_and_steals_from_revealed_only() {
        let mut rng = PythonRandom::seed(1);
        let mut game = game(&mut rng);
        let vampire = place(&mut game, "VampireEnchantress", 0, 0);
        // A created card's reveal hook runs before the engine flips its flag.
        game.cards[vampire].revealed = false;
        let ally = place(&mut game, "Wasp", 0, 1);
        place(&mut game, "Wasp", 0, 2);
        game.cards[ally].cost = 5;
        let enemy = place(&mut game, "Hulk", 1, 0);
        vampire_enchantress(&mut game, vampire);
        assert_eq!(game.cards[vampire].power(), 9);
        assert_eq!(game.cards[enemy].power(), 12);
    }
}
