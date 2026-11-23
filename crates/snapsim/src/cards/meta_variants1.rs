//! Standalone client definitions omitted by the stage-1 audit.
use super::CardDef;
use crate::game::Game;

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        reveal: Some(random),
        ..CardDef::vanilla("Random")
    },
    CardDef {
        reveal: Some(toad),
        ..CardDef::vanilla("Toad")
    },
    // There is no Mephisto entry in the client CardBehaviorDefList.DefMap.
    // Winnings are not an in-match board effect and require a separate stake gate.
    CardDef::vanilla("Mephisto"),
];

// RandomAlexBehavior::GainStatsOfRandomCardInDeck 0x7f2273c.
fn random(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let deck = &game.players[owner].deck;
    if !deck.is_empty() {
        let target = deck[game.rng.borrow_mut().randbelow(deck.len())];
        let power = game.cards[target].power();
        game.set_power(card, power, owner);
    }
}

// ToadBehavior::MoveCard 0x7cdf2e8 reads LastCardPlayed and only moves
// that exact card when still in play and at a different location.
fn toad(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    if let Some(&(_, target)) = game.players[owner].play_history.last()
        && game.cards[target].lane.is_some_and(|other| other != lane)
    {
        game.move_card(target, lane);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    fn setup<'a>(rng: &'a mut PythonRandom) -> Game<'a> {
        let deck = [
            "Abomination",
            "Hulk",
            "Cyclops",
            "Wasp",
            "Quicksilver",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
            "MistyKnight",
            "Shocker",
            "TheThing",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        Game::new([&deck, &deck], &lanes, rng).unwrap()
    }

    #[test]
    fn random_copies_current_power_and_empty_deck_is_noop() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        let card = game.create_card("Random", 0);
        let target = game.create_card("Hulk", 0);
        game.players[0].deck.clear();
        game.set_power(target, 7, 0);
        game.players[0].deck.push(target);
        random(&mut game, card);
        assert_eq!(game.cards[card].power(), 7);
        game.players[0].deck.clear();
        random(&mut game, card);
        assert_eq!(game.cards[card].power(), 7);
    }

    #[test]
    fn toad_moves_last_played_card_only() {
        let mut rng = PythonRandom::seed(2);
        let mut game = setup(&mut rng);
        let card = game.create_card("Toad", 0);
        let target = game.create_card("Hulk", 0);
        for (id, lane) in [(card, 1), (target, 0)] {
            game.cards[id].lane = Some(lane);
            game.cards[id].revealed = true;
            game.lanes[lane].cards.push(id);
        }
        game.players[0].play_history.push((1, target));
        toad(&mut game, card);
        assert_eq!(game.cards[target].lane, Some(1));
    }

    #[test]
    fn mephisto_has_no_client_board_ability() {
        let def = CARDS.iter().find(|def| def.id == "Mephisto").unwrap();
        assert!(def.reveal.is_none());
        assert!(def.ongoing.is_none());
        assert!(def.on_event.is_none());
    }
}
