//! Card rules traced from the 59.15.1 client.
use super::CardDef;
use crate::game::Game;

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        destroy_guard: Some(caiera),
        ..CardDef::vanilla("Caiera")
    },
    CardDef {
        in_hand_ongoing: Some(miles),
        ..CardDef::vanilla("MilesMorales")
    },
    CardDef {
        reveal: Some(thor),
        ..CardDef::vanilla("Thor")
    },
    CardDef {
        reveal: Some(mjolnir),
        ..CardDef::vanilla("Mjolnir")
    },
    CardDef {
        reveal: Some(luna_snow),
        ..CardDef::vanilla("LunaSnow")
    },
    CardDef {
        start_turn: Some(ice_cube),
        ..CardDef::vanilla("IceCube")
    },
    CardDef {
        reveal: Some(infinity_ultron),
        ..CardDef::vanilla("InfinityUltron")
    },
    CardDef {
        reveal_repetitions: Some(ultron_mind),
        ..CardDef::vanilla("UltronMindStone")
    },
    CardDef {
        reveal: Some(ultron_power),
        ..CardDef::vanilla("UltronPowerStone")
    },
    CardDef {
        reveal: Some(ultron_reality),
        ..CardDef::vanilla("UltronRealityStone")
    },
    CardDef {
        reveal: Some(ultron_soul),
        ..CardDef::vanilla("UltronSoulStone")
    },
    CardDef {
        reveal: Some(ultron_space),
        ..CardDef::vanilla("UltronSpaceStone")
    },
    CardDef {
        reveal: Some(ultron_time),
        ..CardDef::vanilla("UltronTimeStone")
    },
];
pub(crate) static TOKENS: &[&str] = &[
    "Mjolnir",
    "IceCube",
    "UltronMindStone",
    "UltronPowerStone",
    "UltronRealityStone",
    "UltronSoulStone",
    "UltronSpaceStone",
    "UltronTimeStone",
];

// AddStones 0x7ecb844 rolls an index, creates that stone, and removes it
// from the remaining list before the second roll.
fn infinity_ultron(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let mut stones = [
        "UltronPowerStone",
        "UltronRealityStone",
        "UltronTimeStone",
        "UltronSpaceStone",
        "UltronSoulStone",
        "UltronMindStone",
    ]
    .to_vec();
    for _ in 0..2 {
        let index = game.rng.borrow_mut().randbelow(stones.len());
        let stone = game.create_card(stones.remove(index), owner);
        if !game.add_hand(owner, stone, false) {
            game.cards.pop();
        }
    }
}

// DoubleResolveAbility 0x7ce45b8.
fn ultron_mind(game: &Game<'_>, source: usize, target: usize) -> u8 {
    u8::from(source != target && game.cards[source].owner == game.cards[target].owner)
}

fn ultrons(game: &Game<'_>, owner: usize) -> Vec<usize> {
    game.lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&id| {
            game.cards[id].revealed
                && game.cards[id].owner == owner
                && game.cards[id].def.id == "InfinityUltron"
        })
        .collect()
}

// BuffInfinityUltron 0x7ce4b90 doubles every target.
fn ultron_power(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    for target in ultrons(game, owner) {
        game.multiply_power(target, 2, owner);
    }
}

// AddDrones 0x7ce51e4 creates a Drone even if no Ultron exists; otherwise
// chooses one target and sets the Drone's power to its current power.
fn ultron_reality(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let drone = game.create_card("Drone", owner);
    if !game.add_card(drone, lane, owner, true) {
        game.cards.pop();
        return;
    }
    let targets: Vec<_> = game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&id| game.cards[id].owner == owner && game.cards[id].def.id == "InfinityUltron")
        .collect();
    if !targets.is_empty() {
        let target = targets[game.rng.borrow_mut().randbelow(targets.len())];
        game.set_power(drone, game.cards[target].power(), owner);
    }
}

// BuffCards 0x7ce5718 and selector 0x7ce6128 exclude the source.
fn ultron_soul(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    for lane in 0..3 {
        if game.has_space(lane, owner, 0) {
            continue;
        }
        let targets: Vec<_> = game.lanes[lane]
            .cards
            .iter()
            .copied()
            .filter(|&id| id != card && game.cards[id].owner == owner && game.cards[id].revealed)
            .collect();
        if !targets.is_empty() {
            let target = targets[game.rng.borrow_mut().randbelow(targets.len())];
            game.add_power(target, 3, owner);
        }
    }
}

// MoveCard 0x7ce6174 chooses an Ultron destination then a tied lowest-power
// revealed enemy at this lane.
fn ultron_space(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let mut enemies: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| game.cards[id].owner != owner && game.cards[id].revealed)
        .collect();
    if enemies.is_empty() {
        return;
    }
    let destinations: Vec<_> = ultrons(game, owner)
        .into_iter()
        .filter(|&id| game.cards[id].lane != Some(lane))
        .collect();
    if destinations.is_empty() {
        return;
    }
    let destination = destinations[game.rng.borrow_mut().randbelow(destinations.len())];
    let min = enemies
        .iter()
        .map(|&id| game.cards[id].power())
        .min()
        .unwrap();
    enemies.retain(|&id| game.cards[id].power() == min);
    let target = enemies[game.rng.borrow_mut().randbelow(enemies.len())];
    game.move_card(target, game.cards[destination].lane.unwrap());
}

// HandIntoPlay 0x7ce6ac4 picks a random hand card and moves it here.
fn ultron_time(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    if game.players[owner].hand.is_empty() {
        return;
    }
    let index = game
        .rng
        .borrow_mut()
        .randbelow(game.players[owner].hand.len());
    let target = game.players[owner].hand[index];
    if game.can_add(target, lane, owner) {
        game.players[owner].hand.remove(index);
        game.add_card(target, lane, owner, true);
    }
}

// ProtectCardsByCost (0x7e79584) tests revealed allies' DefCost.
fn caiera(game: &Game<'_>, source: usize, target: usize) -> bool {
    !(game.cards[source].owner == game.cards[target].owner
        && game.cards[target].revealed
        && matches!(game.printed_cost(target), 1 | 6))
}

// ReduceCost (0x7ef8abc) assigns cost one if any card moved last turn.
fn miles(game: &mut Game<'_>, card: usize) {
    if let Some(turn) = game.turn.checked_sub(1)
        && (game.has_moved_on_turn(0, turn) || game.has_moved_on_turn(1, turn))
    {
        game.set_ongoing_cost(card, 1, game.cards[card].owner);
    }
}

// AddMjolnir (0x7cdcc10) creates the token and shuffles the deck.
fn thor(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let token = game.create_card("Mjolnir", owner);
    game.players[owner].deck.push(token);
    game.shuffle_deck(owner);
}

// BuffThor (0x7efba30), GetTargets (0x7efb8e8): revealed allied Thor.
fn mjolnir(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&id| {
            game.cards[id].owner == owner
                && game.cards[id].revealed
                && game.cards[id].def.id == "Thor"
        })
        .collect();
    for id in targets {
        game.add_power(id, 7, owner);
    }
}

// AddIceCube (0x7eeb75c): each side gets its own Ice Cube here.
fn luna_snow(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    game.add_created("IceCube", lane, owner);
    game.add_created("IceCube", lane, 1 - owner);
}

// AddExtraEnergy (0x7ec9b74): grant energy first, then destroy if full.
fn ice_cube(game: &mut Game<'_>, card: usize) {
    if game.cards[card].abilities_removed {
        return;
    }
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    game.set_energy(owner, game.players[owner].energy + 1);
    if !game.has_space(lane, owner, 0) {
        game.destroy_card(card);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{locations, random::PythonRandom};
    fn game<'a>(rng: &'a mut PythonRandom) -> Game<'a> {
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
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut g = Game::new([&deck, &deck], &lanes, rng).unwrap();
        for lane in &mut g.lanes {
            lane.def = locations::get("Ruins").unwrap();
            lane.revealed = true;
        }
        g
    }
    fn place(g: &mut Game<'_>, name: &str, owner: usize, lane: usize) -> usize {
        let id = g.create_card(name, owner);
        g.cards[id].lane = Some(lane);
        g.cards[id].revealed = true;
        g.lanes[lane].cards.push(id);
        id
    }
    #[test]
    fn reality_stone_copies_unrevealed_ultron_power() {
        let mut rng = PythonRandom::seed(4);
        let mut g = game(&mut rng);
        let stone = place(&mut g, "UltronRealityStone", 0, 0);
        let ultron = place(&mut g, "InfinityUltron", 0, 1);
        g.cards[ultron].revealed = false;
        ultron_reality(&mut g, stone);
        let drone = g.lanes[0]
            .cards
            .iter()
            .copied()
            .find(|&id| g.cards[id].def.id == "Drone")
            .unwrap();
        assert_eq!(g.cards[drone].power(), g.cards[ultron].power());
    }
    #[test]
    fn time_stone_rolls_before_full_lane_rejects() {
        let mut rng = PythonRandom::seed(7);
        let mut g = game(&mut rng);
        let stone = place(&mut g, "UltronTimeStone", 0, 0);
        for _ in 0..3 {
            place(&mut g, "Wasp", 0, 0);
        }
        let mut expected = g.rng.borrow().clone_rng();
        expected.randbelow(g.players[0].hand.len());
        ultron_time(&mut g, stone);
        assert_eq!(
            g.rng.borrow_mut().randbelow(1_000_000),
            expected.randbelow(1_000_000)
        );
    }
    #[test]
    fn infinity_ultron_creates_two_distinct_stones() {
        let mut rng = PythonRandom::seed(1);
        let mut g = game(&mut rng);
        let ultron = place(&mut g, "InfinityUltron", 0, 0);
        let before = g.players[0].hand.len();
        infinity_ultron(&mut g, ultron);
        let hand = &g.players[0].hand[before..];
        assert_eq!(hand.len(), 2);
        assert_ne!(g.cards[hand[0]].def.id, g.cards[hand[1]].def.id);
        let power = g.cards[ultron].power();
        let stone = hand[0];
        ultron_power(&mut g, stone);
        assert_eq!(g.cards[ultron].power(), power * 2);
    }
    #[test]
    fn caiera_guards_allied_printed_costs() {
        let mut rng = PythonRandom::seed(1);
        let mut g = game(&mut rng);
        let c = place(&mut g, "Caiera", 0, 0);
        let a = place(&mut g, "Hulk", 0, 1);
        let b = place(&mut g, "Hulk", 1, 1);
        g.cards[a].cost = 2;
        assert!(!caiera(&g, c, a));
        assert!(caiera(&g, c, b));
        assert!(!g.destroy_card(a));
        assert!(g.destroy_card(b));
        g.cards[c].abilities_removed = true;
        assert!(g.destroy_card(a));
    }
    #[test]
    fn miles_discounts_after_either_side_moves() {
        let mut rng = PythonRandom::seed(1);
        let mut g = game(&mut rng);
        g.turn = 4;
        let moved = place(&mut g, "Hulk", 1, 0);
        g.cards[moved].move_turns.push(3);
        let card = g.create_card("MilesMorales", 0);
        miles(&mut g, card);
        assert_eq!(g.cards[card].cost + g.cards[card].ongoing_cost, 1);
    }
    #[test]
    fn thor_shuffles_and_hammer_buffs() {
        let mut rng = PythonRandom::seed(1);
        let mut g = game(&mut rng);
        let t = place(&mut g, "Thor", 0, 0);
        let old = g.cards[t].power();
        thor(&mut g, t);
        let hammer = *g.players[0]
            .deck
            .iter()
            .find(|&&id| g.cards[id].def.id == "Mjolnir")
            .unwrap();
        mjolnir(&mut g, hammer);
        assert_eq!(g.cards[t].power(), old + 7);
    }
    #[test]
    fn luna_creates_two_energy_cubes() {
        let mut rng = PythonRandom::seed(1);
        let mut g = game(&mut rng);
        let l = place(&mut g, "LunaSnow", 0, 1);
        luna_snow(&mut g, l);
        let cubes: Vec<_> = g.lanes[1]
            .cards
            .iter()
            .copied()
            .filter(|&id| g.cards[id].def.id == "IceCube")
            .collect();
        assert_eq!(cubes.len(), 2);
        let e = g.players[0].energy;
        ice_cube(&mut g, cubes[0]);
        assert_eq!(g.players[0].energy, e + 1);
    }
}
