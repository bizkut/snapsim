use super::{LocationDef, location};
use crate::game::Game;

pub(crate) const BROOKLYN_BRIDGE: LocationDef = LocationDef {
    allows_play: Some(allows_play),
    ..location("BrooklynBridge", 0, 0, 1, false)
};

// BrooklynBridgeBehavior::DisableCardPlay 0x7d41bd4 queries the player's
// prior-turn staged plays at this location, including entities that left.
fn allows_play(
    game: &Game<'_>,
    lane: usize,
    _card: usize,
    _destination: usize,
    seat: usize,
) -> bool {
    let Some(previous) = game.turn.checked_sub(1) else {
        return true;
    };
    !game.players[seat]
        .staged_lane_history
        .contains(&(previous, lane))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    #[test]
    fn prior_staged_lane_blocks_only_that_player_and_next_turn() {
        let mut rng = PythonRandom::seed(15);
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
        .map(str::to_owned)
        .to_vec();
        let locations = ["BrooklynBridge", "Ruins", "Xandar"]
            .map(str::to_owned)
            .to_vec();
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        game.turn = 3;
        game.players[0].staged_lane_history.push((2, 0));
        assert!(!allows_play(&game, 0, 0, 0, 0));
        assert!(allows_play(&game, 0, 0, 0, 1));
        assert!(allows_play(&game, 1, 0, 1, 0));
        assert!(allows_play(&game, 2, 0, 2, 0));
        game.turn = 4;
        assert!(allows_play(&game, 0, 0, 0, 0));
    }
}
