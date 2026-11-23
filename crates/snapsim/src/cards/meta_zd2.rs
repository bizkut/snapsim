//! Card rules traced from the 59.15.1 client.
use super::CardDef;
use crate::{
    game::{CardState, Event, Game},
    zones::CardZone,
};

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        on_activate: Some(sparky),
        ..CardDef::vanilla("Sparky")
    },
    CardDef {
        initial_state: Some(objective_state),
        objective_update: Some(objective_update),
        objective_freeze: Some(objective_freeze),
        on_event: Some(objective_event),
        ..CardDef::vanilla("VampireDeaconFrost")
    },
    CardDef {
        reveal: Some(crimson_behemoth),
        ..CardDef::vanilla("CrimsonBehemoth")
    },
    CardDef {
        initial_state: Some(objective_state),
        objective_update: Some(objective_update),
        objective_freeze: Some(objective_freeze),
        on_event: Some(objective_event),
        ..CardDef::vanilla("PunisherWarMachine")
    },
];

// SparkyBehavior::BuffAndMoveCard 0x7f44ec8 buffs before attempting the move,
// even if the last moved card has since left the board or the lane is full.
fn sparky(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let Some(target) = game.players[owner].last_moved_card else {
        return;
    };
    let Some(CardZone::Location(lane)) = game.cards[target].previous_zone else {
        return;
    };
    game.add_power(target, 2, owner);
    game.move_card_including_staged(target, lane);
}

pub(crate) static TOKENS: &[&str] = &["CrimsonBehemoth"];

#[derive(Clone, Default)]
struct Objective {
    progress: usize,
    ready: bool,
    completed: bool,
    marked_lane: Option<usize>,
}

fn objective_state() -> Box<dyn CardState> {
    Box::new(Objective::default())
}

fn objective_update(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let ability = game.cards[card].default_ability().id;
    let progress =
        if ability == "VampireDeaconFrost" {
            // CalculateObjectiveProgress 0x7ce96c0 reads the player-wide dictionary,
            // including steals made before this card was created or played.
            game.players[owner].stolen_power_recipients.len().min(3)
        } else {
            let marked = game.cards[card]
                .ability_state()
                .and_then(|s| s.as_any().downcast_ref::<Objective>())
                .and_then(|s| s.marked_lane);
            usize::from(marked.is_some_and(|lane| {
                game.cards[card].revealed && game.lane_winner(lane) == Some(owner)
            }))
        };
    if let Some(state) = game.cards[card]
        .ability_state_mut()
        .and_then(|s| s.as_any_mut().downcast_mut::<Objective>())
        && !state.completed
        && !state.ready
    {
        state.progress = progress;
    }
}

fn objective_freeze(game: &mut Game<'_>, card: usize) {
    let goal = if game.cards[card].default_ability().id == "VampireDeaconFrost" {
        3
    } else {
        1
    };
    if let Some(state) = game.cards[card]
        .ability_state_mut()
        .and_then(|s| s.as_any_mut().downcast_mut::<Objective>())
        && !state.completed
        && state.progress >= goal
    {
        state.ready = true;
    }
}

fn objective_event(game: &mut Game<'_>, card: usize, event: Event) {
    let ability = game.cards[card].default_ability().id;
    if ability == "PunisherWarMachine"
        && matches!(event, Event::CardDrawn { card: drawn, .. } if drawn == card)
    {
        // MarkLocation 0x7f1cda8 shuffles all currently empty owned locations.
        let owner = game.cards[card].owner;
        let mut lanes: Vec<_> = (0..3)
            .filter(|&lane| {
                !game.lanes[lane].destroyed
                    && !game.lanes[lane]
                        .cards
                        .iter()
                        .any(|&id| game.cards[id].owner == owner)
            })
            .collect();
        for i in (1..lanes.len()).rev() {
            let j = game.rng.borrow_mut().randbelow(i + 1);
            lanes.swap(i, j);
        }
        if let Some(&lane) = lanes.first()
            && let Some(state) = game.cards[card]
                .ability_state_mut()
                .and_then(|s| s.as_any_mut().downcast_mut::<Objective>())
        {
            state.marked_lane = Some(lane);
        }
        return;
    }
    if !matches!(event, Event::AfterTurnEnd) {
        return;
    }
    let Some(state) = game.cards[card]
        .ability_state_mut()
        .and_then(|s| s.as_any_mut().downcast_mut::<Objective>())
    else {
        return;
    };
    if state.completed || !state.ready {
        return;
    }
    state.completed = true;
    if ability == "VampireDeaconFrost" {
        // Morph 0x7ce977c preserves the entity and its modifiers.
        game.transform_card(card, "CrimsonBehemoth");
    } else {
        // AddMaxEnergy 0x7f1ccd0 rechecks the marked location at execution.
        let owner = game.cards[card].owner;
        let lane = game.cards[card]
            .ability_state()
            .and_then(|s| s.as_any().downcast_ref::<Objective>())
            .and_then(|s| s.marked_lane);
        if lane
            .is_some_and(|lane| game.cards[card].revealed && game.lane_winner(lane) == Some(owner))
        {
            game.add_permanent_energy(owner, 1);
        }
    }
}

// StealFrontRowPower 0x7e84d08 and predicate 0x7e85258.
fn crimson_behemoth(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| {
            game.cards[id].owner != owner
                && game.cards[id].revealed
                && game.is_character(id)
                && game.is_front_row(id)
                && game.cards[id].power() >= 0
        })
        .collect();
    if !targets.is_empty() {
        let target = targets[game.rng.borrow_mut().randbelow(targets.len())];
        game.steal_power(target, card, game.cards[target].power(), owner);
    }
}

pub(crate) fn swap_punisher_marked_locations(game: &mut Game<'_>, a: usize, b: usize) {
    for card in &mut game.cards {
        if let Some(state) = card
            .ability_state_mut()
            .and_then(|s| s.as_any_mut().downcast_mut::<Objective>())
        {
            state.marked_lane = state.marked_lane.map(|lane| {
                if lane == a {
                    b
                } else if lane == b {
                    a
                } else {
                    lane
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    fn setup(rng: &mut PythonRandom) -> Game<'_> {
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
        Game::new([&deck, &deck], &lanes, rng).unwrap()
    }

    #[test]
    fn sparky_buffs_and_returns_last_moved_card() {
        let mut rng = PythonRandom::seed(10);
        let mut game = setup(&mut rng);
        let sparky_card = game.create_card("Sparky", 0);
        let target = game.create_card("Hulk", 0);
        game.add_card(sparky_card, 2, 0, false);
        game.add_card(target, 0, 0, false);
        assert!(game.move_card(target, 1));
        let power = game.cards[target].permanent;
        sparky(&mut game, sparky_card);
        assert_eq!(game.cards[target].permanent, power + 2);
        assert_eq!(game.cards[target].lane, Some(0));
    }

    #[test]
    fn sparky_buffs_even_when_return_is_blocked() {
        let mut rng = PythonRandom::seed(11);
        let mut game = setup(&mut rng);
        let sparky_card = game.create_card("Sparky", 0);
        let target = game.create_card("Hulk", 0);
        game.add_card(sparky_card, 2, 0, false);
        game.add_card(target, 0, 0, false);
        assert!(game.move_card(target, 1));
        for _ in 0..4 {
            let filler = game.create_card("Wasp", 0);
            assert!(game.add_card(filler, 0, 0, false));
        }
        let power = game.cards[target].permanent;
        sparky(&mut game, sparky_card);
        assert_eq!(game.cards[target].permanent, power + 2);
        assert_eq!(game.cards[target].lane, Some(1));
    }

    #[test]
    fn sparky_ignores_non_location_previous_zone() {
        let mut rng = PythonRandom::seed(12);
        let mut game = setup(&mut rng);
        let sparky_card = game.create_card("Sparky", 0);
        let target = game.create_card("Hulk", 0);
        game.players[0].last_moved_card = Some(target);
        game.push_hand(0, target);
        game.take_hand(0, game.players[0].hand.len() - 1);
        game.push_location(0, target);
        game.cards[target].lane = Some(0);
        let power = game.cards[target].permanent;
        sparky(&mut game, sparky_card);
        assert_eq!(game.cards[target].permanent, power);
        assert_eq!(game.cards[target].lane, Some(0));
    }
    #[test]
    fn vampire_counts_distinct_stealers_before_play_and_transforms_once() {
        let mut rng = PythonRandom::seed(13);
        let mut game = setup(&mut rng);
        let victim = game.create_card("Hulk", 1);
        game.add_card(victim, 0, 1, false);
        let recipients: Vec<_> = (0..3).map(|_| game.create_card("Wasp", 0)).collect();
        for &id in &recipients {
            game.add_card(id, 0, 0, false);
            game.steal_power(victim, id, 1, 0);
        }
        game.steal_power(victim, recipients[0], 1, 0);
        assert_eq!(game.players[0].stolen_power_recipients.len(), 3);
        let vampire = game.create_card("VampireDeaconFrost", 0);
        game.add_card(vampire, 1, 0, false);
        objective_update(&mut game, vampire);
        objective_freeze(&mut game, vampire);
        objective_event(&mut game, vampire, Event::AfterTurnEnd);
        assert_eq!(game.cards[vampire].def.id, "CrimsonBehemoth");
    }

    #[test]
    fn steal_ledger_ignores_noop_and_survives_zone_changes() {
        let mut rng = PythonRandom::seed(14);
        let mut game = setup(&mut rng);
        let victim = game.create_card("Hulk", 1);
        let recipient = game.create_card("Wasp", 0);
        game.add_card(victim, 0, 1, false);
        game.add_card(recipient, 0, 0, false);
        game.steal_power(victim, recipient, 0, 0);
        assert!(game.players[0].stolen_power_recipients.is_empty());
        game.steal_power(victim, recipient, 2, 0);
        game.steal_power(victim, recipient, 1, 0);
        assert!(game.take_from_board(recipient));
        assert!(game.add_hand(0, recipient, false));
        assert_eq!(game.players[0].stolen_power_recipients, [recipient]);
        assert!(game.players[1].stolen_power_recipients.is_empty());
    }

    #[test]
    fn behemoth_only_steals_from_revealed_nonnegative_front_row() {
        let mut rng = PythonRandom::seed(15);
        let mut game = setup(&mut rng);
        let behemoth = game.create_card("CrimsonBehemoth", 0);
        let victim = game.create_card("Hulk", 1);
        game.add_card(behemoth, 0, 0, false);
        game.add_card(victim, 0, 1, false);
        let power = game.cards[victim].power();
        crimson_behemoth(&mut game, behemoth);
        assert_eq!(game.cards[victim].power(), 0);
        assert_eq!(game.cards[behemoth].permanent, 9 + power);
        assert_eq!(game.players[0].stolen_power_recipients, [behemoth]);
    }

    #[test]
    fn punisher_marks_empty_location_and_grants_energy_when_won() {
        let mut rng = PythonRandom::seed(16);
        let mut game = setup(&mut rng);
        let source = game.create_card("PunisherWarMachine", 0);
        assert!(game.add_hand(0, source, true));
        let marked = game.cards[source]
            .ability_state()
            .unwrap()
            .as_any()
            .downcast_ref::<Objective>()
            .unwrap()
            .marked_lane
            .unwrap();
        let index = game.players[0]
            .hand
            .iter()
            .position(|&id| id == source)
            .unwrap();
        game.take_hand(0, index);
        game.add_card(source, marked, 0, false);
        objective_update(&mut game, source);
        objective_freeze(&mut game, source);
        objective_event(&mut game, source, Event::AfterTurnEnd);
        objective_event(&mut game, source, Event::AfterTurnEnd);
        assert_eq!(game.players[0].permanent_energy, 1);
    }

    #[test]
    fn punisher_marker_follows_location_shuffle() {
        let mut rng = PythonRandom::seed(17);
        let mut game = setup(&mut rng);
        let source = game.create_card("PunisherWarMachine", 0);
        objective_event(
            &mut game,
            source,
            Event::CardDrawn {
                card: source,
                seat: 0,
            },
        );
        let before = game.cards[source]
            .ability_state()
            .unwrap()
            .as_any()
            .downcast_ref::<Objective>()
            .unwrap()
            .marked_lane
            .unwrap();
        let id = game.lanes[before].def.id;
        game.shuffle_locations();
        let after = game.cards[source]
            .ability_state()
            .unwrap()
            .as_any()
            .downcast_ref::<Objective>()
            .unwrap()
            .marked_lane
            .unwrap();
        assert_eq!(game.lanes[after].def.id, id);
    }
}
