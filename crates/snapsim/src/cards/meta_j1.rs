//! Card rules from the 59.15.1 client.
use super::CardDef;
use crate::game::{Event, Game};

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        can_play: Some(giganto_can_play),
        ..CardDef::vanilla("Giganto")
    },
    CardDef {
        on_event: Some(sebastian_shaw),
        ..CardDef::vanilla("SebastianShaw")
    },
];

// GigantoBehavior::CanPlay (0x7eb6610) compares the location index with zero.
fn giganto_can_play(_game: &Game<'_>, _card: usize, lane: usize, _seat: usize) -> bool {
    lane == 0
}

// SebastianShawBehavior::AdditionalBuffOnBuff (0x7f337a0) checks the
// recipient and positive permanent increase, then adds its AddedPower (2)
// with the post-increase trigger disabled.
fn sebastian_shaw(game: &mut Game<'_>, card: usize, event: Event) {
    if matches!(event, Event::Empower { card: target, amount, .. } if target == card && amount > 0)
    {
        game.add_power_silent(card, 2, card);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{locations, random::PythonRandom};

    #[test]
    fn giganto_only_plays_left_unless_ability_removed() {
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
        for lane in &mut game.lanes {
            lane.def = locations::get("Ruins").unwrap();
            lane.revealed = true;
        }
        let card = game.create_card("Giganto", 0);
        assert!(game.can_play(card, 0, 0, 6, 0));
        assert!(!game.can_play(card, 1, 0, 6, 0));
        assert!(!game.can_play(card, 2, 0, 6, 0));
        game.cards[card].abilities_removed = true;
        assert!(game.can_play(card, 1, 0, 6, 0));
    }

    #[test]
    fn sebastian_shaw_gains_two_for_each_permanent_increase() {
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
        let card = game.create_card("SebastianShaw", 0);
        game.players[0].hand.push(card);
        game.add_power(card, 1, card);
        assert_eq!(game.cards[card].permanent, 8);
        game.add_power(card, -1, card);
        assert_eq!(game.cards[card].permanent, 7);
        game.set_power(card, 10, card);
        assert_eq!(game.cards[card].permanent, 12);
        game.cards[card].abilities_removed = true;
        game.add_power(card, 1, card);
        assert_eq!(game.cards[card].permanent, 13);
    }

    #[test]
    fn infinity_stones_fail_closed_without_known_deck() {
        use crate::{coverage, position::Position};
        let mut position = Position {
            turn: 2,
            ..Position::default()
        };
        position.opponent.seen.push("TimeStone".into());
        assert!(
            coverage::coverage(&position)["state"]
                .to_string()
                .contains("post-shuffle deck composition unknown")
        );
    }
}
