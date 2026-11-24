//! Random-pool card rules traced to the 59.15.1 client.
use super::CardDef;
use crate::game::{Event, Game};

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        reveal: Some(agent_coulson),
        ..CardDef::vanilla("AgentCoulson")
    },
    CardDef {
        on_event: Some(ezekiel_sims),
        ..CardDef::vanilla("EzekielSims")
    },
    CardDef {
        reveal: Some(gambit),
        ..CardDef::vanilla("Gambit")
    },
    CardDef {
        reveal: Some(gwenpool),
        ..CardDef::vanilla("Gwenpool")
    },
    CardDef {
        move_limit: 1,
        on_move: Some(nocturne),
        ..CardDef::vanilla("Nocturne")
    },
    CardDef {
        reveal: Some(phastos),
        ..CardDef::vanilla("Phastos")
    },
];

// AddCards (0x7e616cc) samples both costs before creating either card.
// GetRandomCardDefIds is approximated by the registered client-eligible pool.
fn agent_coulson(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let four = game.random_card(|row| row.cost == 4);
    let five = game.random_card(|row| row.cost == 5);
    for id in [four, five].into_iter().flatten() {
        if game.players[owner].hand.len() < 7 {
            let target = game.create_card(id, owner);
            game.add_hand(owner, target, false);
        }
    }
}

// ContingentTriggerAction::Execute (0x7d24190) runs DestroyAction only
// when DiscardCardsAction (0x7d24be0) succeeds. Both targets come from
// RandomSelector::Select (0x7d28478) with count 1, which returns the whole
// pool without a Game::Roll when it holds at most one card.
fn gambit(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let count = game.players[owner].hand.len();
    if count == 0 {
        return;
    }
    let choice = if count > 1 {
        game.rng.borrow_mut().randbelow(count)
    } else {
        0
    };
    let discarded = game.players[owner].hand[choice];
    if !game.discard_card(owner, discarded) {
        return;
    }
    let mut targets = game.revealed_cards_in_resolution_order();
    targets.retain(|&target| game.cards[target].owner != owner);
    if !targets.is_empty() {
        let choice = if targets.len() > 1 {
            game.rng.borrow_mut().randbelow(targets.len())
        } else {
            0
        };
        game.destroy_card(targets[choice]);
    }
}

// BuffAndMove (0x7ea587c) observes an allied card resolved from a
// play staged at this lane, then compares its current Power to Ezekiel's.
fn ezekiel_sims(game: &mut Game<'_>, card: usize, event: Event) {
    let Event::CardResolved { card: played } = event else {
        return;
    };
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    if !game.played_cards_at(owner, lane).any(|id| id == played)
        || game.cards[played].power() <= game.cards[card].power()
    {
        return;
    }
    game.add_power(card, 2, owner);
    // GetRandomLocationToMoveCard (0x7db2f50) with fallback to every other
    // location; the move then fails when none was legal.
    if let Some(destination) = game.random_location_to_move_card(card, lane, true) {
        game.move_card(card, destination);
    }
}

// ReduceCostOrAddPower (0x7f14da0): each deck card independently gets
// -1 Cost or +2 Power; cards without Power always receive the Cost change,
// while zero-Cost characters always receive Power.
fn phastos(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let deck = game.players[owner].deck.clone();
    for target in deck {
        let has_power = game.is_character(target);
        let cost = game.cards[target].cost + game.cards[target].ongoing_cost;
        if !has_power || (cost > 0 && game.rng.borrow_mut().randbelow(2) == 0) {
            game.add_cost(target, -1, owner);
        } else {
            game.add_power(target, 2, owner);
        }
    }
}

// ReplaceLocation (0x7f0fb84) morphs Nocturne's destination after any move
// to Game::GetRandomLocationDefId (0x7db248c): one rarity-weighted roll that
// excludes only the current location's definition (not other board
// locations). The location pool is restricted to implemented locations.
fn nocturne(game: &mut Game<'_>, card: usize, _from: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let current = game.lanes[lane].def.id;
    if let Some(id) = game.random_location_def_id(Some(current)) {
        game.replace_location(lane, id);
    }
}

// GwenpoolBehavior::AddPower (0x7eba9bc): shuffle the character hand
// snapshot before each of three selections, and add two Power each time.
fn gwenpool(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let mut targets: Vec<_> = game.players[owner]
        .hand
        .iter()
        .copied()
        .filter(|&target| game.is_character(target))
        .collect();
    if targets.is_empty() {
        return;
    }
    for _ in 0..3 {
        // Game::Shuffle (0x4b12f30): forward pass, one Roll(i, len) per index.
        for i in 0..targets.len().saturating_sub(1) {
            let j = i + game.rng.borrow_mut().randbelow(targets.len() - i);
            targets.swap(i, j);
        }
        game.add_power(targets[0], 2, owner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    #[test]
    fn gwenpool_buffs_a_single_character_three_times() {
        let mut rng = PythonRandom::seed(1);
        let deck = [
            "Wasp",
            "MistyKnight",
            "Hulk",
            "Cyclops",
            "Shocker",
            "TheThing",
            "Abomination",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
            "Quicksilver",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        game.players[0].hand.clear();
        let source = game.create_card("Gwenpool", 0);
        let target = game.create_card("Hulk", 0);
        game.add_hand(0, target, false);
        let before = game.cards[target].power();
        gwenpool(&mut game, source);
        assert_eq!(game.cards[target].power(), before + 6);
    }

    #[test]
    fn coulson_adds_one_four_and_one_five_cost() {
        let mut rng = PythonRandom::seed(1);
        let deck = [
            "Wasp",
            "MistyKnight",
            "Hulk",
            "Cyclops",
            "Shocker",
            "TheThing",
            "Abomination",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
            "Quicksilver",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        game.players[0].hand.clear();
        let source = game.create_card("AgentCoulson", 0);
        agent_coulson(&mut game, source);
        assert_eq!(game.players[0].hand.len(), 2);
        assert_eq!(game.printed_cost(game.players[0].hand[0]), 4);
        assert_eq!(game.printed_cost(game.players[0].hand[1]), 5);
    }

    #[test]
    fn phastos_buffs_zero_cost_and_modifies_every_deck_card() {
        let mut rng = PythonRandom::seed(2);
        let deck = [
            "Wasp",
            "MistyKnight",
            "Hulk",
            "Cyclops",
            "Shocker",
            "TheThing",
            "Abomination",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
            "Quicksilver",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Phastos", 0);
        let zero = game.create_card("Wasp", 0);
        let positive = game.create_card("Hulk", 0);
        game.players[0].deck = vec![zero, positive];
        let zero_power = game.cards[zero].power();
        let power = game.cards[positive].power();
        let cost = game.cards[positive].cost;
        phastos(&mut game, source);
        assert_eq!(game.cards[zero].power(), zero_power + 2);
        assert!(
            (game.cards[positive].cost == cost - 1 && game.cards[positive].power() == power)
                || (game.cards[positive].cost == cost && game.cards[positive].power() == power + 2)
        );
    }

    #[test]
    fn ezekiel_buffs_and_moves_after_stronger_ally_resolves() {
        let mut rng = PythonRandom::seed(2);
        let deck = [
            "Wasp",
            "MistyKnight",
            "Hulk",
            "Cyclops",
            "Shocker",
            "TheThing",
            "Abomination",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
            "Quicksilver",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("EzekielSims", 0);
        let played = game.create_card("Hulk", 0);
        game.cards[played].turn_played = game.turn.max(1);
        game.cards[source].lane = Some(1);
        game.cards[source].revealed = true;
        game.lanes[1].cards.push(source);
        game.played[0].push((played, 1));
        let before = game.cards[source].power();
        ezekiel_sims(&mut game, source, Event::CardResolved { card: played });
        assert_eq!(game.cards[source].power(), before + 2);
        assert_ne!(game.cards[source].lane, Some(1));
    }

    #[test]
    fn gambit_requires_discard_before_destroying() {
        let mut rng = PythonRandom::seed(2);
        let deck = [
            "Wasp",
            "MistyKnight",
            "Hulk",
            "Cyclops",
            "Shocker",
            "TheThing",
            "Abomination",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
            "Quicksilver",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        game.players[0].hand.clear();
        let source = game.create_card("Gambit", 0);
        let enemy = game.create_card("Hulk", 1);
        game.cards[enemy].lane = Some(1);
        game.cards[enemy].revealed = true;
        game.lanes[1].cards.push(enemy);
        gambit(&mut game, source);
        assert_eq!(game.cards[enemy].lane, Some(1));
        let discard = game.create_card("Wasp", 0);
        game.add_hand(0, discard, false);
        gambit(&mut game, source);
        assert_eq!(game.cards[enemy].lane, None);
    }

    #[test]
    fn nocturne_changes_destination_not_origin() {
        let mut rng = PythonRandom::seed(3);
        let deck = [
            "Wasp",
            "MistyKnight",
            "Hulk",
            "Cyclops",
            "Shocker",
            "TheThing",
            "Abomination",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
            "Quicksilver",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        // The test pool is the board itself; GetRandomLocationDefId (0x7db248c) would
        // otherwise exclude the starting locations.
        game.starting_location_ids.clear();
        let card = game.create_card("Nocturne", 0);
        game.cards[card].lane = Some(1);
        game.cards[card].revealed = true;
        game.lanes[1].cards.push(card);
        let (first, second) = (game.lanes[0].def.id, game.lanes[1].def.id);
        nocturne(&mut game, card, 0);
        assert_eq!(game.lanes[0].def.id, first);
        assert_ne!(game.lanes[1].def.id, second);
    }

    fn client_deck() -> [String; 12] {
        [
            "Wasp",
            "MistyKnight",
            "Hulk",
            "Cyclops",
            "Shocker",
            "TheThing",
            "Abomination",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
            "Quicksilver",
        ]
        .map(str::to_owned)
    }

    // RandomSelector::Select (0x7d28478) returns a pool of at most `count`
    // cards without a Game::Roll.
    #[test]
    fn gambit_rolls_nothing_for_single_card_pools() {
        let mut rng = PythonRandom::seed(4);
        let deck = client_deck();
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        game.players[0].hand.clear();
        let source = game.create_card("Gambit", 0);
        let enemy = game.create_card("Hulk", 1);
        game.cards[enemy].lane = Some(1);
        game.cards[enemy].revealed = true;
        game.lanes[1].cards.push(enemy);
        let discard = game.create_card("Wasp", 0);
        game.add_hand(0, discard, false);
        let mut expected = game.rng.borrow().clone_rng();
        gambit(&mut game, source);
        assert_eq!(game.cards[enemy].lane, None);
        assert_eq!(
            game.rng.borrow_mut().randbelow(1_000_000),
            expected.randbelow(1_000_000)
        );
    }

    // GetRandomLocationDefId (0x7db248c) excludes only the replaced
    // location's definition, so another board location's definition can
    // be chosen.
    #[test]
    fn nocturne_may_pick_a_definition_already_on_the_board() {
        let deck = client_deck();
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut rng = PythonRandom::seed(5);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        // The test pool is the board itself; GetRandomLocationDefId (0x7db248c) would
        // otherwise exclude the starting locations.
        game.starting_location_ids.clear();
        let card = game.create_card("Nocturne", 0);
        game.cards[card].lane = Some(1);
        game.cards[card].revealed = true;
        game.lanes[1].cards.push(card);
        let before = game.lanes[1].def.id;
        // The pool is exactly the three board locations, so the only legal
        // result is one of the other two board definitions.
        nocturne(&mut game, card, 0);
        assert_ne!(game.lanes[1].def.id, before);
        assert!(
            [0, 2]
                .iter()
                .any(|&lane| game.lanes[lane].def.id == game.lanes[1].def.id)
        );
    }
}
