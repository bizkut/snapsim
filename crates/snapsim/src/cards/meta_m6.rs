//! Card rules traced from the 59.15.1 client.
use super::CardDef;
use crate::game::Game;

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        reveal: Some(nick_fury),
        ..CardDef::vanilla("NickFury")
    },
    CardDef {
        reveal: Some(red_shift),
        ..CardDef::vanilla("RedShift")
    },
    CardDef {
        on_discard: Some(marrow_discard),
        on_destroy: Some(marrow),
        ..CardDef::vanilla("Marrow")
    },
];

// NickFuryBehavior::AddCards (0x7f0c2fc) selects three distinct six-Cost
// cards before adding them, excluding Agatha Harkness. The client pool is
// approximated by implemented cards.
fn nick_fury(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let mut selected = [None; 3];
    for index in 0..selected.len() {
        selected[index] = game.random_card(|row| {
            row.cost == 6
                && row.carddefid != "AgathaHarkness"
                && !selected[..index].contains(&Some(row.carddefid.as_str()))
        });
    }
    for id in selected.into_iter().flatten() {
        if game.players[owner].hand.len() >= 7 {
            break;
        }
        let created = game.create_card(id, owner);
        game.add_hand(owner, created, false);
    }
}
// DiscardAndReplace 0x7f27000 discards the rightmost card, then samples
// a different definition with its printed Cost (predicate 0x7f27574).
// The client pool is approximated by implemented cards; if empty, it creates
// the discarded definition instead, as the client does.
fn red_shift(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let Some(&target) = game.players[owner].hand.last() else {
        return;
    };
    if !game.discard_card(owner, target) || game.players[owner].hand.len() >= 7 {
        return;
    }
    let cost = game.printed_cost(target);
    let discarded_id = game.cards[target].def.id;
    let id = game
        .random_card(|row| {
            row.cost == cost && row.carddefid != "RedShift" && row.carddefid != discarded_id
        })
        .unwrap_or(discarded_id);
    let created = game.create_card(id, owner);
    game.add_hand(owner, created, false);
}
// AfflictCards 0x7ef2ea4 calls GetAllRevealedCards for the opponent,
// rolls seven times with replacement, aggregates by target, then applies power.
fn marrow(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game
        .revealed_cards_in_resolution_order()
        .into_iter()
        .filter(|&id| game.cards[id].owner != owner)
        .collect();
    if targets.is_empty() {
        return;
    }
    let mut counts = vec![0; targets.len()];
    for _ in 0..7 {
        let index = game.rng.borrow_mut().randbelow(targets.len());
        counts[index] -= 1;
    }
    for (target, amount) in targets.into_iter().zip(counts) {
        if amount != 0 {
            game.add_power(target, amount, owner);
        }
    }
}

fn marrow_discard(game: &mut Game<'_>, card: usize) {
    game.players[game.cards[card].owner].discard.push(card);
    marrow(game, card);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    #[test]
    fn nick_fury_adds_distinct_six_cost_cards() {
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
        let card = game.create_card("NickFury", 0);
        game.players[0].hand.clear();
        nick_fury(&mut game, card);
        let hand = &game.players[0].hand;
        assert_eq!(hand.len(), 3);
        assert!(hand.iter().all(|&id| game.card_cost(id) == 6));
        let mut ids: Vec<_> = hand.iter().map(|&id| game.cards[id].def.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 3);
    }
    #[test]
    fn red_shift_discards_rightmost_and_replaces_at_same_cost() {
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
        let source = game.create_card("RedShift", 0);
        let left = game.create_card("Hulk", 0);
        let right = game.create_card("Cyclops", 0);
        game.cards[right].cost = 1;
        game.players[0].hand.clear();
        game.players[0].hand.extend([left, right]);
        red_shift(&mut game, source);
        assert!(game.players[0].discard.contains(&right));
        assert_eq!(game.players[0].hand[0], left);
        let replacement = game.players[0].hand[1];
        assert_eq!(game.card_cost(replacement), 3);
        assert_ne!(game.cards[replacement].def.id, "Cyclops");
    }
    #[test]
    fn marrow_discard_afflicts_only_revealed_enemies() {
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
        let source = game.create_card("Marrow", 0);
        let enemy = game.create_card("Hulk", 1);
        let hidden = game.create_card("Cyclops", 1);
        for id in [enemy, hidden] {
            game.cards[id].lane = Some(0);
            game.lanes[0].cards.push(id);
        }
        game.cards[enemy].revealed = true;
        let before = game.cards[enemy].power();
        game.players[0].hand.clear();
        game.players[0].hand.push(source);
        game.discard_card(0, source);
        assert_eq!(game.cards[enemy].power(), before - 7);
        assert!(game.players[0].discard.contains(&source));
    }
}
