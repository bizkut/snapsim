//! Sinister Six's player-owned draw and post-resolution abilities.
use crate::game::{Event, Game};

#[derive(Clone, Default)]
pub struct SinisterState {
    pub source: usize,
    pub marked: Vec<usize>,
    pub played: Vec<usize>,
}

// AddAbilitiesToPlayer 0x7f3e48c: the state belongs to the player, not the card.
pub(crate) fn start(game: &mut Game<'_>, source: usize) {
    let owner = game.cards[source].owner;
    game.players[owner].sinister_six.push(SinisterState {
        source,
        ..SinisterState::default()
    });
}

// MarkCardsEvil 0x7f3e5b0 and CheckEvilPlayed 0x7f3ebfc.
pub(crate) fn sinister_event(game: &mut Game<'_>, event: Event) {
    match event {
        Event::CardDrawn { card, seat } => {
            if !game.is_character(card) || game.cards[card].owner != seat {
                return;
            }
            for state in &mut game.players[seat].sinister_six {
                if state.marked.len() < 6
                    && state.source != card
                    && game.cards[state.source].owner == seat
                    && !state.marked.contains(&card)
                {
                    state.marked.push(card);
                }
            }
        }
        Event::StagedCardResolved { card, seat, lane } => {
            let mut completions = Vec::new();
            for state in &mut game.players[seat].sinister_six {
                if state.played.len() < 6
                    && game.cards[state.source].owner == seat
                    && state.marked.contains(&card)
                    && !state.played.contains(&card)
                {
                    state.played.push(card);
                    if state.played.len() == 6 {
                        completions.push((state.source, state.played.clone()));
                    }
                }
            }
            for (source, played) in completions {
                let on_board = game.cards[source].lane.is_some() && game.cards[source].revealed;
                let in_hand = game.players[seat].hand.contains(&source);
                let in_deck = game.players[seat].deck.contains(&source);
                if !on_board && !in_hand && !in_deck {
                    continue;
                }
                if !game.spawns_allowed()
                    || game.lane_locked(lane)
                    || (game.location_abilities_active(lane)
                        && game.lanes[lane]
                            .def
                            .allows_addition
                            .is_some_and(|test| !test(game, lane, source, lane, seat)))
                {
                    continue;
                }
                let absorbed: Vec<_> = played
                    .into_iter()
                    .filter(|id| {
                        !game
                            .players
                            .iter()
                            .any(|player| player.set_aside.contains(id))
                    })
                    .collect();
                game.merge_cards(source, &absorbed);
                if on_board {
                    game.move_card(source, lane);
                } else if in_hand {
                    game.move_hand_card_to_location(source, lane);
                } else {
                    game.move_deck_card_to_location(source, lane);
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    #[test]
    fn draw_tags_survive_source_leaving_play() {
        let mut rng = PythonRandom::seed(4);
        let mut game = super::super::test_game(&mut rng);
        let source = game.create_card("SinisterSix", 0);
        game.cards[source].lane = Some(0);
        game.lanes[0].cards.push(source);
        start(&mut game, source);
        game.lanes[0].cards.retain(|&id| id != source);
        game.cards[source].lane = None;
        game.players[0].graveyard.push((source, Some(0)));
        let drawn = game.create_card("Wasp", 0);
        sinister_event(
            &mut game,
            Event::CardDrawn {
                card: drawn,
                seat: 0,
            },
        );
        assert_eq!(game.players[0].sinister_six[0].marked, [drawn]);
    }

    #[test]
    fn six_distinct_tagged_resolutions_merge_as_one_batch() {
        let mut rng = PythonRandom::seed(5);
        let mut game = super::super::test_game(&mut rng);
        let source = game.create_card("SinisterSix", 0);
        game.players[0].deck.push(source);
        start(&mut game, source);
        let mut tagged = Vec::new();
        for index in 0..6 {
            let card = game.create_card("Wasp", 0);
            sinister_event(&mut game, Event::CardDrawn { card, seat: 0 });
            let lane = index % 3;
            game.cards[card].revealed = true;
            game.cards[card].turn_played = game.turn;
            game.cards[card].lane = Some(lane);
            game.lanes[lane].cards.push(card);
            game.played[0].push((card, lane));
            sinister_event(
                &mut game,
                Event::StagedCardResolved {
                    card,
                    seat: 0,
                    lane,
                },
            );
            tagged.push(card);
        }
        assert_eq!(game.cards[source].lane, Some(2));
        assert!(
            tagged
                .iter()
                .all(|&id| game.cards[id].merged_to == Some(source))
        );
        assert_eq!(game.players[0].sinister_six[0].played.len(), 6);
    }

    #[test]
    fn game_start_installs_effect_before_opening_draw() {
        let mut rng = PythonRandom::seed(6);
        let deck = [
            "SinisterSix",
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
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        for player in &game.players {
            assert_eq!(player.sinister_six.len(), 1);
            let state = &player.sinister_six[0];
            let expected = player
                .draws
                .iter()
                .filter(|&&(_, id)| id != state.source)
                .count();
            assert_eq!(state.marked.len(), expected);
        }
    }
}
