//! Hand affliction rules traced from the 59.15.1 client.
use super::CardDef;
use crate::game::{Card, CardState, Event, Game};

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        reveal: Some(scorpion),
        ..CardDef::vanilla("Scorpion")
    },
    CardDef {
        reveal: Some(cassandra_nova),
        ..CardDef::vanilla("CassandraNova")
    },
    CardDef {
        move_limit: 1,
        on_move: Some(spider_man_2099),
        ..CardDef::vanilla("SpiderMan2099")
    },
    CardDef {
        reveal: Some(kraglin),
        ..CardDef::vanilla("Kraglin")
    },
    CardDef {
        reveal: Some(dragon_of_the_moon),
        ..CardDef::vanilla("DragonOfTheMoon")
    },
    CardDef {
        reveal: Some(mole_man),
        ..CardDef::vanilla("MoleMan")
    },
    CardDef {
        reveal: Some(red_hulk_fractured_frontier),
        ..CardDef::vanilla("RedHulkFracturedFrontier")
    },
    CardDef {
        on_activate: Some(bullseye),
        ..CardDef::vanilla("Bullseye")
    },
    CardDef {
        move_limit: 1,
        on_event: Some(rocket_and_groot),
        ..CardDef::vanilla("RocketAndGroot")
    },
    CardDef {
        move_limit: 1,
        on_event: Some(spider_punk),
        ..CardDef::vanilla("SpiderPunk")
    },
    CardDef {
        on_activate: Some(fantomex),
        ..CardDef::vanilla("Fantomex")
    },
    CardDef {
        initial_state: Some(selene_state),
        objective_update: Some(selene_update),
        objective_freeze: Some(selene_freeze),
        on_event: Some(selene_complete),
        ..CardDef::vanilla("SeleneHorseman")
    },
];

// ScorpionBehavior::ReducePower (0x7f31df8), ThisCardResolvedTrigger:
// apply ReducedPower=1 to each card in the opposing hand.
fn scorpion(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let targets = game.players[1 - owner].hand.clone();
    for target in targets {
        game.add_power(target, -1, owner);
    }
}

// CassandraNovaBehavior::DrainFromDeck (0x7e7f0f4), ThisCardResolvedTrigger.
fn cassandra_nova(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let targets = game.players[1 - owner].deck.clone();
    game.steal_power_many(&targets, card, 1, owner);
}

// KraglinBehavior::BanishForReward (0x7ee20d0), ThisCardResolvedTrigger.
fn kraglin(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let Some(top) = game.top_card(owner) else {
        return;
    };
    let cost = game.card_cost(top);
    if game.banish_card(top) {
        if cost >= 4 {
            game.schedule_energy(owner, game.turn.saturating_add(1), 2);
        } else {
            game.add_power(card, 4, owner);
        }
    }
}

// DrainFromHand 0x7e92288; selectors 0x7e928e4 and 0x7e92950
// require power-bearing cards with even current Cost.
fn dragon_of_the_moon(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game.players[1 - owner]
        .hand
        .iter()
        .chain(game.players[owner].hand.iter())
        .copied()
        .filter(|&id| game.is_character(id) && game.card_cost(id) % 2 == 0)
        .collect();
    game.steal_power_many(&targets, card, 1, owner);
}

// MoleManBehavior::BanishAndCreateCards 0x7efe1d8 snapshots DefCost,
// banishes the top card, then sets each created Rock's base Power.
pub(crate) fn mole_man(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let Some(top) = game.top_card(owner) else {
        return;
    };
    let cost = game.printed_cost(top);
    if !game.banish_card(top) {
        return;
    }
    for _ in 0..2 {
        if game.players[owner].hand.len() >= 7 {
            break;
        }
        let rock = game.create_card("Rock", owner);
        game.add_hand(owner, rock, false);
        game.set_base_power(rock, cost);
    }
}

// ModifyOwnPower 0x7f24420 counts all opposing lane occupants, including face down.
fn red_hulk_fractured_frontier(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let count = game.lanes[lane]
        .cards
        .iter()
        .filter(|&&id| game.cards[id].owner != owner)
        .count() as i32;
    let amount = if game.is_quickdraw_active(card) {
        count
    } else {
        -count
    };
    if amount != 0 {
        game.add_power(card, amount, owner);
    }
}

// DiscardHand 0x7e78874 snapshots current-cost <=1 cards, discards each,
// shuffles all revealed enemies and afflicts one per successful discard.
// Game::DiscardCard 0x7dabe68 fails for targets no longer in hand.
fn bullseye(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let hand: Vec<_> = game.players[owner]
        .hand
        .iter()
        .copied()
        .filter(|&id| game.card_cost(id) <= 1)
        .collect();
    if hand.is_empty() {
        return;
    }
    let discarded = hand
        .into_iter()
        .filter(|&id| game.discard_card(owner, id))
        .count();
    let mut targets = game.revealed_cards_in_resolution_order();
    targets.retain(|&id| game.cards[id].owner != owner);
    for index in 0..targets.len().saturating_sub(1) {
        let other = index + game.rng.borrow_mut().randbelow(targets.len() - index);
        targets.swap(index, other);
    }
    for target in targets.into_iter().take(discarded) {
        game.add_power(target, -2, owner);
    }
}

// StealPower 0x7f28f98: opponent's resolved card must have been played
// at this lane, even if its reveal moved it elsewhere.
fn rocket_and_groot(game: &mut Game<'_>, card: usize, event: Event) {
    let Event::CardResolved { card: target } = event else {
        return;
    };
    let owner = game.cards[card].owner;
    if let Some(lane) = game.cards[card].lane
        && game.played_cards_at(1 - owner, lane).any(|id| id == target)
    {
        game.steal_power(target, card, 1, owner);
    }
}

// PostCardMovedTrigger StealPowerFromEnemyWithHighestPower 0x7cc1dcc;
// GetHighestPowerCardForPlayer 0x7ddefdc rolls uniformly among highest ties.
fn spider_punk(game: &mut Game<'_>, card: usize, event: Event) {
    let Event::CardMoved { card: moved, .. } = event else {
        return;
    };
    if moved != card {
        return;
    }
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let mut targets = [0; 4];
    let mut count = 0;
    let mut best = i32::MIN;
    for &id in &game.lanes[lane].cards {
        if game.cards[id].owner == owner || !game.cards[id].revealed {
            continue;
        }
        let power = game.cards[id].power();
        if power > best {
            best = power;
            count = 0;
        }
        if power == best {
            targets[count] = id;
            count += 1;
        }
    }
    if count > 0 {
        let index = game.rng.borrow_mut().randbelow(count);
        game.steal_power(targets[index], card, 2, owner);
    }
}

// AfflictFrontRowCards 0x7ea7ee4 checks owner discard/destruction counts.
fn fantomex(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if game.players[owner].discard_history.is_empty()
        || game.players[owner].destruction_history.is_empty()
    {
        return;
    }
    for target in fantomex_targets(game, owner) {
        game.add_power(target, -2, owner);
    }
}

// GetLocations 0x7daed18 and Location::GetAllRevealedFrontRowCards 0x7ddf184
// enumerate locations, then the opponent's card slots within each location.
fn fantomex_targets(game: &Game<'_>, owner: usize) -> Vec<usize> {
    let mut candidates: Vec<_> = if crate::cards::reference_rules_enabled() {
        game.revealed_cards_in_resolution_order()
    } else {
        game.lanes
            .iter()
            .flat_map(|lane| lane.cards.iter().copied())
            .collect()
    };
    candidates.retain(|&id| {
        game.cards[id].owner != owner && game.cards[id].revealed && game.is_front_row(id)
    });
    candidates
}

#[derive(Clone, Default)]
struct SeleneObjective {
    progress: usize,
    ready: bool,
    completed: bool,
}

fn selene_state() -> Box<dyn CardState> {
    Box::new(SeleneObjective::default())
}

pub(crate) fn restore_selene(card: &mut Card, progress: usize, completed: bool) {
    if let Some(state) = card
        .ability_state_mut()
        .and_then(|state| state.as_any_mut().downcast_mut::<SeleneObjective>())
    {
        state.progress = progress;
        state.ready = false;
        state.completed = completed;
    }
}

// CalculateObjectiveProgress 0x7f34908 counts revealed and staged allies
// for which Card::HasNoText 0x7cb3768 returns true.
fn selene_update(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let progress = game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&id| {
            let target = &game.cards[id];
            target.owner == owner
                && (target.revealed
                    || (game.is_wait_for_staging
                        && target.turn_played == game.turn
                        && game.turn > 0))
                && target.has_no_text() == Some(true)
        })
        .count()
        .min(3);
    let state = game.cards[card]
        .ability_state_mut()
        .unwrap()
        .as_any_mut()
        .downcast_mut::<SeleneObjective>()
        .unwrap();
    if !state.completed && !state.ready {
        state.progress = progress;
    }
}

fn selene_freeze(game: &mut Game<'_>, card: usize) {
    let state = game.cards[card]
        .ability_state_mut()
        .unwrap()
        .as_any_mut()
        .downcast_mut::<SeleneObjective>()
        .unwrap();
    if !state.completed && state.progress >= 3 {
        state.ready = true;
    }
}

// StealPower 0x7f34c14 rolls a revealed power-bearing enemy here, stealing 5.
fn selene_complete(game: &mut Game<'_>, card: usize, event: Event) {
    if !matches!(event, Event::AfterTurnEnd) || game.cards[card].abilities_removed {
        return;
    }
    let state = game.cards[card]
        .ability_state_mut()
        .unwrap()
        .as_any_mut()
        .downcast_mut::<SeleneObjective>()
        .unwrap();
    if state.completed || !state.ready {
        return;
    }
    state.completed = true;
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let mut targets = [0; 4];
    let mut count = 0;
    for &target in &game.lanes[lane].cards {
        if game.cards[target].owner != owner
            && game.cards[target].revealed
            && game.is_character(target)
        {
            targets[count] = target;
            count += 1;
        }
    }
    if count > 0 {
        let index = game.rng.borrow_mut().randbelow(count);
        game.steal_power(targets[index], card, 5, owner);
    }
}

// SpiderMan2099Behavior::AfflictEnemyAtLocation (0x7f4b568), ThisCardMovedTrigger.
fn spider_man_2099(game: &mut Game<'_>, card: usize, _from: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let mut targets = [0; 4];
    let mut count = 0;
    for &id in &game.lanes[lane].cards {
        if game.cards[id].owner != owner && game.cards[id].revealed {
            targets[count] = id;
            count += 1;
        }
    }
    if count > 0 {
        let index = game.rng.borrow_mut().randbelow(count);
        game.add_power(targets[index], -3, owner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    #[test]
    fn scorpion_afflicts_only_opposing_hand() {
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
        let own: Vec<_> = game.players[0]
            .hand
            .iter()
            .map(|&id| (id, game.cards[id].power()))
            .collect();
        let enemy: Vec<_> = game.players[1]
            .hand
            .iter()
            .map(|&id| (id, game.cards[id].power()))
            .collect();
        let card = game.create_card("Scorpion", 0);
        scorpion(&mut game, card);
        assert!(!enemy.is_empty());
        for (id, power) in own {
            assert_eq!(game.cards[id].power(), power);
        }
        for (id, power) in enemy {
            assert_eq!(game.cards[id].power(), power - 1);
        }
    }

    #[test]
    fn cassandra_drains_opposing_deck() {
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
        let targets: Vec<_> = game.players[1]
            .deck
            .iter()
            .map(|&id| (id, game.cards[id].power()))
            .collect();
        let card = game.create_card("CassandraNova", 0);
        let power = game.cards[card].power();
        cassandra_nova(&mut game, card);
        assert!(!targets.is_empty());
        assert_eq!(game.cards[card].power(), power + targets.len() as i32);
        for (id, before) in targets {
            assert_eq!(game.cards[id].power(), before - 1);
        }
    }

    #[test]
    fn spider_man_2099_afflicts_revealed_enemy_on_move() {
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
        let card = game.create_card("SpiderMan2099", 0);
        let enemy = game.create_card("Hulk", 1);
        game.add_card(card, 1, 0, true);
        game.add_card(enemy, 1, 1, true);
        let before = game.cards[enemy].power();
        spider_man_2099(&mut game, card, 0);
        assert_eq!(game.cards[enemy].power(), before - 3);
    }

    #[test]
    fn kraglin_banishes_top_and_rewards_low_cost() {
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
        let target = game.create_card("Wasp", 0);
        game.players[0].deck.insert(0, target);
        let card = game.create_card("Kraglin", 0);
        let before = game.cards[card].power();
        kraglin(&mut game, card);
        assert!(game.players[0].banished.contains(&target));
        assert_eq!(game.cards[card].power(), before + 4);
    }

    #[test]
    fn dragon_drains_even_cost_from_both_hands() {
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
        let targets: Vec<_> = game
            .players
            .iter()
            .flat_map(|p| p.hand.iter())
            .copied()
            .map(|id| (id, game.cards[id].power(), game.card_cost(id) % 2 == 0))
            .collect();
        let card = game.create_card("DragonOfTheMoon", 0);
        let before = game.cards[card].power();
        dragon_of_the_moon(&mut game, card);
        let count = targets.iter().filter(|&&(_, _, even)| even).count();
        assert_eq!(game.cards[card].power(), before + count as i32);
        for (id, power, even) in targets {
            assert_eq!(game.cards[id].power(), power - i32::from(even));
        }
    }

    #[test]
    fn mole_man_uses_banished_printed_cost_for_rocks() {
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
        let top = game.create_card("Hulk", 0);
        game.players[0].deck.insert(0, top);
        game.set_cost(top, 1, 0);
        let card = game.create_card("MoleMan", 0);
        mole_man(&mut game, card);
        assert!(game.players[0].banished.contains(&top));
        let rocks: Vec<_> = game.players[0]
            .hand
            .iter()
            .copied()
            .filter(|&id| game.cards[id].def.id == "Rock")
            .collect();
        assert_eq!(rocks.len(), 2);
        assert!(rocks.into_iter().all(|id| game.cards[id].power() == 6));
    }

    #[test]
    fn red_hulk_counts_unrevealed_enemies_and_quickdraw() {
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
        let card = game.create_card("RedHulkFracturedFrontier", 0);
        let enemy = game.create_card("Hulk", 1);
        game.add_card(card, 0, 0, true);
        game.add_card(enemy, 0, 1, true);
        game.cards[enemy].revealed = false;
        let power = game.cards[card].power();
        red_hulk_fractured_frontier(&mut game, card);
        assert_eq!(game.cards[card].power(), power - 1);
        game.cards[card].drawn_turn = Some(game.turn);
        red_hulk_fractured_frontier(&mut game, card);
        assert_eq!(game.cards[card].power(), power);
    }

    #[test]
    fn bullseye_discards_low_cost_and_afflicts_distinct_enemies() {
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
        game.players[0].hand.clear();
        let wasp = game.create_card("Wasp", 0);
        game.add_hand(0, wasp, false);
        let card = game.create_card("Bullseye", 0);
        let enemy = game.create_card("Hulk", 1);
        game.add_card(card, 0, 0, true);
        game.add_card(enemy, 0, 1, true);
        let power = game.cards[enemy].power();
        bullseye(&mut game, card);
        assert!(game.players[0].discard.contains(&wasp));
        assert_eq!(game.cards[enemy].power(), power - 2);
    }
    #[test]
    fn bullseye_skips_target_discarded_by_an_earlier_discard_trigger() {
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
        game.players[0].hand.clear();
        let cat = game.create_card("ZombieBlackCat", 0);
        let swarm = game.create_card("Swarm", 0);
        game.cards[cat].cost = 1;
        game.add_hand(0, cat, false);
        game.add_hand(0, swarm, false);
        let card = game.create_card("Bullseye", 0);
        game.add_card(card, 0, 0, true);
        let enemies = [game.create_card("Hulk", 1), game.create_card("Hulk", 1)];
        for &enemy in &enemies {
            game.add_card(enemy, 0, 1, true);
        }
        let total_before: i32 = enemies.iter().map(|&id| game.cards[id].power()).sum();
        bullseye(&mut game, card);
        let total_after: i32 = enemies.iter().map(|&id| game.cards[id].power()).sum();
        assert_eq!(game.players[0].discard_history.len(), 2);
        assert_eq!(total_after, total_before - 2);
    }

    #[test]
    fn rocket_and_groot_ignore_unplayed_resolutions() {
        let mut rng = PythonRandom::seed(10);
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
        let card = game.create_card("RocketAndGroot", 0);
        let enemy = game.create_card("Hulk", 1);
        game.add_card(card, 0, 0, true);
        game.add_card(enemy, 0, 1, true);
        let before = game.cards[card].power();
        rocket_and_groot(&mut game, card, Event::CardResolved { card: enemy });
        assert_eq!(game.cards[card].power(), before);
        game.cards[enemy].turn_played = game.turn.max(1);
        game.played[1].push((enemy, 0));
        rocket_and_groot(&mut game, card, Event::CardResolved { card: enemy });
        assert_eq!(game.cards[card].power(), before + 1);
    }

    #[test]
    fn spider_punk_steals_from_highest_revealed_enemy() {
        let mut rng = PythonRandom::seed(11);
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
        let card = game.create_card("SpiderPunk", 0);
        let enemy = game.create_card("Hulk", 1);
        game.add_card(card, 1, 0, true);
        game.add_card(enemy, 1, 1, true);
        let before = game.cards[card].power();
        spider_punk(
            &mut game,
            card,
            Event::CardMoved {
                card,
                from: 0,
                to: 1,
            },
        );
        assert_eq!(game.cards[card].power(), before + 2);
    }

    #[test]
    fn fantomex_requires_both_history_kinds() {
        let mut rng = PythonRandom::seed(12);
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
        let card = game.create_card("Fantomex", 0);
        let enemy = game.create_card("Hulk", 1);
        game.add_card(card, 0, 0, true);
        game.add_card(enemy, 0, 1, true);
        let right = game.create_card("Hulk", 1);
        game.add_card(right, 2, 1, true);
        game.cards[enemy].latest_resolution_order = 10;
        game.cards[right].latest_resolution_order = 1;
        assert_eq!(fantomex_targets(&game, 0), vec![enemy, right]);
        let power = game.cards[enemy].power();
        fantomex(&mut game, card);
        assert_eq!(game.cards[enemy].power(), power);
        let item = crate::game::CardZoneEvent {
            turn: 0,
            card,
            power: 0,
            lane: Some(0),
        };
        game.players[0].discard_history.push(item);
        game.players[0].destruction_history.push(item);
        assert!(game.is_front_row(enemy));
        fantomex(&mut game, card);
        assert_eq!(game.cards[enemy].power(), power - 2);
    }

    #[test]
    fn selene_completes_for_three_vanilla_allies() {
        let mut rng = PythonRandom::seed(13);
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
        let card = game.create_card("SeleneHorseman", 0);
        game.add_card(card, 0, 0, true);
        for name in ["Hulk", "Wasp", "Cyclops"] {
            let ally = game.create_card(name, 0);
            game.add_card(ally, 1, 0, true);
        }
        let enemy = game.create_card("Hulk", 1);
        game.add_card(enemy, 0, 1, true);
        let before = game.cards[card].power();
        selene_update(&mut game, card);
        selene_freeze(&mut game, card);
        selene_complete(&mut game, card, Event::AfterTurnEnd);
        assert_eq!(game.cards[card].power(), before + 5);
    }
}
