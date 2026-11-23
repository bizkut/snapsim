//! Action-at-a-time, two-seat environment for independent self-play games.
use crate::game::{Action, Game, Stakes};
use crate::random::PythonRandom;
use std::cell::Cell;

/// Play: 0..21 (hand slot × lane); move: 21..57 (board slot × lane);
/// activate: 57..69 (board slot); end, snap, retreat and retreat later: 69..73.
pub const ACTION_COUNT: usize = 73;
pub const END_TURN: usize = 69;
pub const SNAP: usize = 70;
pub const RETREAT: usize = 71;
pub const RETREAT_LATER: usize = 72;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisibleCard {
    pub id: &'static str,
    pub cost: i32,
    pub power: i32,
}

/// An opponent card staged during a completed ordered phase.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisibleAction {
    pub card: &'static str,
    pub lane: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observation {
    pub turn: u8,
    pub total_turns: u8,
    pub energy: i32,
    pub hand: Vec<VisibleCard>,
    pub deck_count: usize,
    pub opponent_hand_count: usize,
    pub opponent_deck_count: usize,
    pub locations: [Option<&'static str>; 3],
    /// Own cards and revealed opposing cards, in lane order.
    pub board: [[Vec<VisibleCard>; 2]; 3],
    pub face_down_opponent: [usize; 3],
    pub opponent_staged: Vec<VisibleAction>,
    pub current_cubes: u32,
    pub next_cubes: u32,
    pub snapped: [bool; 2],
    pub ended: bool,
    pub outcome: Option<[i64; 2]>,
}

/// Each seat stages on an independent private fork until both end the turn.
pub struct Env {
    game: Game<'static>,
    views: [Game<'static>; 2],
    plans: [Vec<Action>; 2],
    staging_actions: Vec<usize>,
    staged_counts: [usize; 2],
    ended: [bool; 2],
    priority: usize,
    outcome: Option<[i64; 2]>,
    staging_queue: Vec<usize>,
    queue_index: usize,
    visible_actions: [usize; 2],
    pending_stakes: [Option<StakeRequest>; 2],
    omitted_tie_moves: Cell<u64>,
    move_source_provenance_unknown: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StakeRequest {
    Snap,
    Retreat,
    RetreatLater,
}

/// The current turn's simultaneous phase, ordered staging phase, or terminal state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhaseKind {
    Simultaneous,
    /// Zero-based phase index and total number of ordered phases.
    Ordered {
        index: usize,
        total: usize,
    },
    Terminal,
}

/// Seats due and their private observations and masks for the current phase.
pub struct Phase {
    pub kind: PhaseKind,
    pub due: [bool; 2],
    pub observations: [Option<Observation>; 2],
    pub masks: [[bool; ACTION_COUNT]; 2],
}

/// A rejected phase leaves the environment unchanged.
pub struct PhaseStep {
    pub outcome: Option<[i64; 2]>,
    pub rejected: [Option<String>; 2],
}

impl Env {
    /// Construct a sampled local-seat planning view from an observed position.
    ///
    /// This is not an exact restore: hidden cards and locations are sampled,
    /// ordered staging is not reconstructed, and pending actions are absent.
    /// Callers must establish phase, stakes and coverage independently and
    /// prove that legality does not depend on hidden data before using a mask.
    pub fn from_sampled_position(
        position: &crate::position::Position,
        stakes: Stakes,
        seed: i128,
    ) -> Result<Self, String> {
        let mut rng = PythonRandom::seed(seed);
        let mut game = crate::position::build_game(position, &mut rng)?.into_owned();
        game.stakes = Some(stakes);
        let priority = game.action_priority(None);
        let views = [game.fork_owned(), game.fork_owned()];
        Ok(Self {
            game,
            views,
            plans: Default::default(),
            staging_actions: Vec::new(),
            staged_counts: [0; 2],
            ended: [false; 2],
            priority,
            staging_queue: Vec::new(),
            queue_index: 0,
            visible_actions: [0; 2],
            pending_stakes: [None; 2],
            omitted_tie_moves: Cell::new(0),
            move_source_provenance_unknown: crate::coverage::move_source_provenance_unknown(
                position,
            ),
            outcome: None,
        })
    }

    pub fn reset(seed: i128, decks: [&[String]; 2], recruit: bool) -> Result<Self, String> {
        let mut rng = PythonRandom::seed(seed);
        let locations = crate::position::all_locations(recruit);
        let mut game = Game::new(decks, &locations, &mut rng)?.into_owned();
        game.stakes = Some(Stakes {
            current: 1,
            next: 1,
            maximum: 8,
            can_raise: true,
            player_multiplier: [1; 2],
            end_game_multiplier: 1,
            game_multiplier: 1,
            snapped: [false; 2],
            last_raiser: None,
            pending_retreat: [false; 2],
        });
        game.staging_order = Some(Vec::new());
        game.begin_turn();
        let priority = game.action_priority(None);
        let staging_queue = std::mem::take(game.staging_order.as_mut().expect("staging recorder"));
        let views = [game.fork_owned(), game.fork_owned()];
        Ok(Self {
            game,
            views,
            plans: Default::default(),
            staging_actions: Vec::new(),
            staged_counts: [0; 2],
            ended: [false; 2],
            priority,
            staging_queue,
            queue_index: 0,
            visible_actions: [0; 2],
            pending_stakes: [None; 2],
            omitted_tie_moves: Cell::new(0),
            move_source_provenance_unknown: false,
            outcome: None,
        })
    }

    pub fn seats_due(&self) -> [bool; 2] {
        if self.outcome.is_some() {
            return [false; 2];
        }
        if let Some(&seat) = self.staging_queue.get(self.queue_index) {
            return [seat == 0, seat == 1];
        }
        [!self.ended[0], !self.ended[1]]
    }

    pub fn phase(&self) -> Result<Phase, String> {
        let due = self.seats_due();
        let mut observations = [None, None];
        let mut masks = [[false; ACTION_COUNT]; 2];
        for seat in 0..2 {
            if due[seat] {
                observations[seat] = Some(self.observation(seat)?);
                masks[seat] = self.legal_mask(seat)?;
            }
        }
        Ok(Phase {
            kind: if self.outcome.is_some() {
                PhaseKind::Terminal
            } else if self.staging_queue.is_empty() {
                PhaseKind::Simultaneous
            } else {
                PhaseKind::Ordered {
                    index: self.queue_index,
                    total: self.staging_queue.len(),
                }
            },
            due,
            observations,
            masks,
        })
    }

    /// Submit one action for every seat due, all against this phase's views.
    /// If any action is rejected, no action in the submission is committed.
    pub fn step_phase(&mut self, actions: [Option<usize>; 2]) -> PhaseStep {
        let due = self.seats_due();
        let mut rejected: [Option<String>; 2] = [None, None];
        for seat in 0..2 {
            if due[seat] != actions[seat].is_some() {
                rejected[seat] = Some("action does not match seats due".into());
            }
        }
        if rejected.iter().any(Option::is_some) {
            return PhaseStep {
                outcome: self.outcome,
                rejected,
            };
        }
        let mut next = self.snapshot();
        for (seat, action) in actions.into_iter().enumerate() {
            if let Some(action) = action
                && let Err(error) = next.step_inner(seat, action)
            {
                let target = if error.contains("seat 0") {
                    0
                } else if error.contains("seat 1") {
                    1
                } else {
                    seat
                };
                rejected[target] = Some(error);
                return PhaseStep {
                    outcome: self.outcome,
                    rejected,
                };
            }
        }
        let outcome = next.outcome;
        *self = next;
        PhaseStep { outcome, rejected }
    }

    fn snapshot(&self) -> Self {
        Self {
            game: self.game.fork_owned(),
            views: [self.views[0].fork_owned(), self.views[1].fork_owned()],
            plans: self.plans.clone(),
            staging_actions: self.staging_actions.clone(),
            staged_counts: self.staged_counts,
            ended: self.ended,
            priority: self.priority,
            outcome: self.outcome,
            staging_queue: self.staging_queue.clone(),
            queue_index: self.queue_index,
            visible_actions: self.visible_actions,
            pending_stakes: self.pending_stakes,
            omitted_tie_moves: Cell::new(self.omitted_tie_moves.get()),
            move_source_provenance_unknown: self.move_source_provenance_unknown,
        }
    }

    fn view(&self, seat: usize) -> Result<&Game<'static>, String> {
        Ok(&self.views[seat])
    }

    fn board_card(game: &Game<'_>, seat: usize, slot: usize) -> Option<usize> {
        let lane = slot / 4;
        game.lanes
            .get(lane)?
            .cards
            .iter()
            .copied()
            .filter(|&id| game.cards[id].owner == seat)
            .nth(slot % 4)
    }

    fn action(game: &Game<'_>, seat: usize, id: usize) -> Option<Action> {
        match id {
            0..21 => Some(Action::Play {
                card: *game.players[seat].hand.get(id / 3)?,
                lane: id % 3,
            }),
            21..57 => {
                let n = id - 21;
                Some(Action::Move {
                    card: Self::board_card(game, seat, n / 3)?,
                    lane: n % 3,
                })
            }
            57..69 => Some(Action::Activate {
                card: Self::board_card(game, seat, id - 57)?,
            }),
            _ => None,
        }
    }

    pub fn legal_mask(&self, seat: usize) -> Result<[bool; ACTION_COUNT], String> {
        if seat >= 2 {
            return Err("invalid seat".into());
        }
        if self.move_source_provenance_unknown {
            return Err("move_source_provenance_unknown".into());
        }
        let view = self.view(seat)?;
        let mask = self.mask_for_view(seat, view);
        if !crate::cards::reference_rules_enabled()
            && self.seats_due()[seat]
            && view.autoplay_controller(seat).is_none()
            && !self
                .game
                .stakes
                .as_ref()
                .expect("environment stakes")
                .pending_retreat[seat]
            && !matches!(
                self.pending_stakes[seat],
                Some(StakeRequest::Retreat | StakeRequest::RetreatLater)
            )
        {
            let omitted = (21..57)
                .filter(|&id| {
                    !mask[id]
                        && matches!(
                            Self::action(view, seat, id),
                            Some(Action::Move { card, lane })
                                if view.can_player_move_without_tie_check(card, lane, seat)
                                    && view.move_source_tie_unknown(card, lane, seat)
                        )
                })
                .count() as u64;
            self.omitted_tie_moves
                .set(self.omitted_tie_moves.get().saturating_add(omitted));
        }
        Ok(mask)
    }

    /// Number of otherwise legal move actions omitted from requested masks
    /// because their source creation order was unknown. Repeated mask requests
    /// count again; resetting the environment resets the counter.
    pub fn omitted_tie_moves(&self) -> u64 {
        self.omitted_tie_moves.get()
    }

    /// Definition ID of the selected source in this seat's latest staged move.
    pub fn selected_move_source(&self, seat: usize) -> Option<&'static str> {
        self.views.get(seat).and_then(|view| view.last_move_source)
    }

    fn mask_for_view(&self, seat: usize, view: &Game<'_>) -> [bool; ACTION_COUNT] {
        let mut mask = [false; ACTION_COUNT];
        if !self.seats_due()[seat] {
            return mask;
        }
        if self
            .game
            .stakes
            .as_ref()
            .expect("environment stakes")
            .pending_retreat[seat]
            || matches!(
                self.pending_stakes[seat],
                Some(StakeRequest::Retreat | StakeRequest::RetreatLater)
            )
        {
            mask[END_TURN] = true;
            return mask;
        }
        if view.autoplay_controller(seat).is_none() {
            for (id, enabled) in mask.iter_mut().enumerate().take(69) {
                *enabled = match Self::action(view, seat, id) {
                    Some(Action::Play { card, lane }) => {
                        view.can_play(card, lane, seat, view.players[seat].energy, 0)
                    }
                    Some(Action::Move { card, lane }) => view.can_player_move(card, lane, seat),
                    Some(Action::Activate { card }) => view.can_activate(card, seat),
                    _ => false,
                };
            }
        }
        mask[END_TURN] = true;
        let stakes = self.game.stakes.as_ref().expect("environment stakes");
        mask[SNAP] = self.pending_stakes[seat].is_none()
            && stakes.can_raise
            && !stakes.snapped[seat]
            && stakes.next < stakes.maximum;
        mask[RETREAT] =
            self.pending_stakes[seat].is_none() && self.game.can_retreat_seat(seat, false);
        mask[RETREAT_LATER] = mask[RETREAT];
        mask
    }

    pub fn step(&mut self, seat: usize, id: usize) -> Result<Option<[i64; 2]>, String> {
        if seat < 2 && id == END_TURN && (!self.staging_queue.is_empty() || self.ended[1 - seat]) {
            let mut next = self.snapshot();
            let result = next.step_inner(seat, id)?;
            *self = next;
            Ok(result)
        } else {
            self.step_inner(seat, id)
        }
    }

    fn step_inner(&mut self, seat: usize, id: usize) -> Result<Option<[i64; 2]>, String> {
        if seat >= 2 || id >= ACTION_COUNT {
            return Err("illegal action".into());
        }
        let view = self.view(seat)?;
        if !self.mask_for_view(seat, view)[id] {
            return Err("illegal action".into());
        }
        match id {
            END_TURN => {
                if !matches!(
                    self.pending_stakes[seat],
                    Some(StakeRequest::Retreat | StakeRequest::RetreatLater)
                ) && !self
                    .game
                    .stakes
                    .as_ref()
                    .expect("environment stakes")
                    .pending_retreat[seat]
                    && let Some(controller) = view.autoplay_controller(seat)
                {
                    let plan = {
                        let mut rng = self.game.rng.borrow_mut();
                        view.autoplay_plan_with_rng(seat, controller, &mut **rng)
                    };
                    for &action in &plan {
                        self.views[seat].stage_action(seat, action)?;
                        if !self.staging_queue.is_empty() {
                            self.staging_actions.push(seat);
                        }
                    }
                    self.plans[seat].extend(plan);
                }
                if self.staging_queue.is_empty() {
                    self.ended[seat] = true;
                    if self.ended == [true; 2] {
                        self.commit_phase(true)?;
                    }
                } else {
                    self.queue_index += 1;
                    let final_phase = self.queue_index == self.staging_queue.len();
                    if !final_phase {
                        self.visible_actions[seat] = self.plans[seat].len();
                    }
                    self.commit_phase(final_phase)?;
                }
            }
            SNAP => self.pending_stakes[seat] = Some(StakeRequest::Snap),
            RETREAT => self.pending_stakes[seat] = Some(StakeRequest::Retreat),
            RETREAT_LATER => self.pending_stakes[seat] = Some(StakeRequest::RetreatLater),
            _ => {
                let action = Self::action(view, seat, id).ok_or("invalid action")?;
                self.views[seat].stage_action(seat, action)?;
                if !self.staging_queue.is_empty() {
                    self.staging_actions.push(seat);
                }
                self.plans[seat].push(action);
            }
        }
        Ok(self.outcome)
    }

    fn commit_phase(&mut self, final_phase: bool) -> Result<(), String> {
        let requests = std::mem::take(&mut self.pending_stakes);
        let mut deferred = self
            .game
            .stakes
            .as_ref()
            .expect("environment stakes")
            .pending_retreat;
        let first =
            if requests.iter().all(Option::is_some) && self.game.rng.borrow_mut().random() >= 0.5 {
                1
            } else {
                0
            };
        if requests == [Some(StakeRequest::Retreat); 2] {
            self.outcome = Some([0, 0]);
            return Ok(());
        }
        for seat in [first, 1 - first] {
            match requests[seat] {
                Some(StakeRequest::Snap) => {
                    self.game
                        .stakes
                        .as_mut()
                        .expect("environment stakes")
                        .snap(seat)
                        .map_err(|error| format!("seat {seat}: {error}"))?;
                    self.game.players[seat].snapped_turns.push(self.game.turn);
                    if deferred[1 - seat] {
                        self.outcome = self
                            .game
                            .stakes
                            .as_ref()
                            .expect("environment stakes")
                            .deferred_retreat(deferred);
                        return Ok(());
                    }
                }
                Some(StakeRequest::Retreat) => {
                    if !self.game.can_retreat_seat(seat, false) {
                        return Err(format!("seat {seat}: retreat unavailable"));
                    }
                    deferred[seat] = true;
                    self.outcome = self
                        .game
                        .stakes
                        .as_ref()
                        .expect("environment stakes")
                        .deferred_retreat(deferred);
                    return Ok(());
                }
                Some(StakeRequest::RetreatLater) => {
                    if !self.game.can_retreat_seat(seat, false) {
                        return Err(format!("seat {seat}: retreat unavailable"));
                    }
                    deferred[seat] = true;
                }
                None => {}
            }
        }
        self.game
            .stakes
            .as_mut()
            .expect("environment stakes")
            .pending_retreat = deferred;
        for view in &mut self.views {
            view.stakes = self.game.stakes.clone();
            for seat in 0..2 {
                view.players[seat]
                    .snapped_turns
                    .clone_from(&self.game.players[seat].snapped_turns);
            }
        }
        if final_phase
            && let Some(outcome) = self
                .game
                .stakes
                .as_ref()
                .expect("environment stakes")
                .deferred_retreat(deferred)
        {
            self.outcome = Some(outcome);
            return Ok(());
        }
        if !self.staging_queue.is_empty() {
            if !crate::cards::reference_rules_enabled() {
                self.game.reveal_priority = Some(self.priority);
            }
            for seat in self.staging_actions.drain(..) {
                let index = self.staged_counts[seat];
                self.game
                    .stage_action(seat, self.plans[seat][index])
                    .map_err(|error| format!("seat {seat}: {error}"))?;
                self.staged_counts[seat] += 1;
            }
            if !final_phase {
                let next = self.staging_queue[self.queue_index];
                self.views[next] = self.game.fork_owned();
            }
        }
        if final_phase {
            if self.staging_queue.is_empty() {
                self.game
                    .resolve_action_plans(std::mem::take(&mut self.plans), self.priority)?;
            } else {
                self.game
                    .resolve_already_staged(std::mem::take(&mut self.plans), self.priority)?;
            }
            self.staged_counts = [0; 2];
            if self.game.turn > self.game.total_turns {
                let winner = self.game.winner();
                let stakes = self.game.stakes.as_mut().expect("environment stakes");
                stakes.set_end_game_multiplier(2);
                stakes.finish_game();
                self.outcome = Some(stakes.showdown((winner < 2).then_some(winner as usize)));
            } else {
                self.ended = [false; 2];
                self.game.begin_turn();
                self.priority = self.game.action_priority(None);
                self.staging_queue =
                    std::mem::take(self.game.staging_order.as_mut().expect("staging recorder"));
                self.views = [self.game.fork_owned(), self.game.fork_owned()];
                self.queue_index = 0;
                self.visible_actions = [0; 2];
            }
        }
        Ok(())
    }

    pub fn observation(&self, seat: usize) -> Result<Observation, String> {
        if seat >= 2 {
            return Err("invalid seat".into());
        }
        if self.outcome.is_none() && !self.seats_due()[seat] {
            return Err("seat is not due in this phase".into());
        }
        let view = self.view(seat)?;
        let other = 1 - seat;
        let card = |id: usize| VisibleCard {
            id: view.cards[id].def.id,
            cost: view.card_cost(id),
            power: view.cards[id].power(),
        };
        let board = std::array::from_fn(|lane| {
            std::array::from_fn(|side| {
                let owner = if side == 0 { seat } else { other };
                view.lanes[lane]
                    .cards
                    .iter()
                    .copied()
                    .filter(|&id| {
                        view.cards[id].owner == owner && (owner == seat || view.cards[id].revealed)
                    })
                    .map(card)
                    .collect()
            })
        });
        let face_down_opponent = std::array::from_fn(|lane| {
            view.lanes[lane]
                .cards
                .iter()
                .filter(|&&id| view.cards[id].owner == other && !view.cards[id].revealed)
                .count()
        });
        let opponent_staged = if self.visible_actions[other] != 0 {
            let opponent_view = self.view(other)?;
            self.plans[other]
                .iter()
                .take(self.visible_actions[other])
                .filter_map(|action| match *action {
                    Action::Play { card, lane } | Action::Move { card, lane } => {
                        Some(VisibleAction {
                            card: opponent_view.cards[card].def.id,
                            lane,
                        })
                    }
                    Action::Activate { .. } | Action::Return { .. } => None,
                })
                .collect()
        } else {
            Vec::new()
        };
        let stakes = self.game.stakes.as_ref().expect("environment stakes");
        Ok(Observation {
            turn: view.turn,
            total_turns: view.total_turns,
            energy: view.players[seat].energy,
            hand: view.players[seat].hand.iter().copied().map(card).collect(),
            deck_count: view.players[seat].deck.len(),
            opponent_hand_count: view.players[other].hand.len(),
            opponent_deck_count: view.players[other].deck.len(),
            locations: std::array::from_fn(|lane| {
                view.lanes[lane].revealed.then_some(view.lanes[lane].def.id)
            }),
            board,
            face_down_opponent,
            opponent_staged,
            current_cubes: stakes.current,
            next_cubes: stakes.next,
            snapped: [stakes.snapped[seat], stakes.snapped[other]],
            ended: self.ended[seat],
            outcome: self.outcome,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deckcodes;

    fn decks() -> Vec<Vec<String>> {
        let (decks, errors) = deckcodes::parse_deck_file(
            include_str!("../../../decks/snapcomplete-2026-11.txt"),
            "decks",
        );
        assert!(errors.is_empty(), "{errors:?}");
        let mut decks: Vec<_> = decks.into_iter().map(|(_, deck)| deck).collect();
        let pool = crate::position::recruit_cards();
        let mut rng = PythonRandom::seed(42);
        decks.push(
            rng.sample_indices(pool.len(), 12)
                .into_iter()
                .map(|i| pool[i].to_string())
                .collect(),
        );
        decks
    }

    #[test]
    fn self_play_never_inherits_restored_move_provenance_gate() {
        let decks = decks();
        let mut env = Env::reset(17, [&decks[0], &decks[1]], false).unwrap();
        assert!(!env.move_source_provenance_unknown);
        let card = env.views[0].create_card("Copycat", 0);
        env.views[0].add_card(card, 0, 0, false);
        assert!(env.legal_mask(0).is_ok());
        assert!(env.legal_mask(1).is_ok());
        let snapshot = env.snapshot();
        assert!(!snapshot.move_source_provenance_unknown);
        assert!(snapshot.legal_mask(0).is_ok());
    }

    #[test]
    fn restored_position_has_local_planning_mask() {
        let position: crate::position::Position = serde_json::from_value(serde_json::json!({
            "turn": 1, "energy": 1,
            "hand": [{"id": "AntMan", "cost": 1, "power": 1}],
            "deck": [], "deck_count": 0,
            "locations": [
                {"id": "Mojoworld", "mine": [], "theirs": []},
                {"id": null, "mine": [], "theirs": []},
                {"id": null, "mine": [], "theirs": []}
            ],
            "opponent": {"hand": 0, "deck": 0}
        }))
        .unwrap();
        let stakes = Stakes {
            current: 1,
            next: 1,
            maximum: 8,
            can_raise: true,
            player_multiplier: [1; 2],
            end_game_multiplier: 1,
            game_multiplier: 1,
            snapped: [false; 2],
            last_raiser: None,
            pending_retreat: [false; 2],
        };
        let env = Env::from_sampled_position(&position, stakes, 1).unwrap();
        assert!(env.seats_due()[0]);
        assert_eq!(env.observation(0).unwrap().hand[0].id, "AntMan");
        assert!(env.legal_mask(0).unwrap()[END_TURN]);
    }

    #[test]
    fn restored_unknown_copied_move_source_invalidates_mask() {
        let position: crate::position::Position = serde_json::from_value(serde_json::json!({
            "turn": 3, "energy": 3, "hand": [], "deck": [], "deck_count": 0,
            "locations": [
                {"id": "Mojoworld", "mine": [{"id": "Copycat", "revealed": true, "move_targets": [1]}], "theirs": []},
                {"id": null, "mine": [], "theirs": []},
                {"id": null, "mine": [], "theirs": []}
            ],
            "opponent": {"hand": 0, "deck": 0}
        })).unwrap();
        let stakes = Stakes {
            current: 1,
            next: 1,
            maximum: 8,
            can_raise: true,
            player_multiplier: [1; 2],
            end_game_multiplier: 1,
            game_multiplier: 1,
            snapped: [false; 2],
            last_raiser: None,
            pending_retreat: [false; 2],
        };
        let env = Env::from_sampled_position(&position, stakes, 1).unwrap();
        assert_eq!(
            env.legal_mask(0),
            Err("move_source_provenance_unknown".into())
        );
    }

    #[test]
    fn unknown_move_source_order_omits_only_the_ambiguous_move() {
        let position: crate::position::Position = serde_json::from_value(serde_json::json!({
            "turn": 6, "energy": 6, "hand": [], "deck": [], "deck_count": 0,
            "locations": [
                {"id": "Mojoworld", "mine": [{"id": "Vision", "revealed": true}], "theirs": []},
                {"id": "NewYork", "mine": [], "theirs": []},
                {"id": "Mojoworld", "mine": [], "theirs": []}
            ],
            "opponent": {"hand": 0, "deck": 0}
        }))
        .unwrap();
        let stakes = Stakes {
            current: 1,
            next: 1,
            maximum: 8,
            can_raise: true,
            player_multiplier: [1; 2],
            end_game_multiplier: 1,
            game_multiplier: 1,
            snapped: [false; 2],
            last_raiser: None,
            pending_retreat: [false; 2],
        };
        let env = Env::from_sampled_position(&position, stakes, 1).unwrap();
        let mask = env.legal_mask(0).unwrap();
        assert!(!mask[22], "move into New York has ambiguous sources");
        assert!(mask[23], "other destination remains legal");
        assert!(mask[END_TURN]);
        assert_eq!(env.omitted_tie_moves(), 1);
        env.legal_mask(0).unwrap();
        assert_eq!(env.omitted_tie_moves(), 2);
        assert_eq!(env.snapshot().omitted_tie_moves(), 2);
    }

    fn trace(deck: &[String], seed: i128, check_mask: bool, recruit: bool) -> Vec<String> {
        let mut env = Env::reset(seed, [deck, deck], recruit).unwrap();
        let mut choice = PythonRandom::seed(seed + 10000);
        let mut trace = Vec::new();
        let mut seat = 0;
        loop {
            if !env.seats_due()[seat] {
                seat = 1 - seat;
            }
            let mask = env.legal_mask(seat).unwrap();
            trace.push(format!("{:?}:{mask:?}", env.observation(seat).unwrap()));
            if check_mask {
                for (id, enabled) in mask.iter().enumerate() {
                    let mut probe = env.snapshot();
                    assert_eq!(
                        probe.step(seat, id).is_ok(),
                        *enabled,
                        "seat {seat}, action {id}"
                    );
                }
            }
            if check_mask {
                let view = env.view(seat).unwrap();
                if view.autoplay_controller(seat).is_none() {
                    for (id, enabled) in mask.iter().enumerate().take(69) {
                        let legal = Env::action(view, seat, id).is_some_and(|action| {
                            let mut probe = view.fork_owned();
                            probe.stage_action(seat, action).is_ok()
                        });
                        assert_eq!(*enabled, legal, "seat {seat}, action {id}");
                    }
                }
            }
            let choices: Vec<_> = (0..=END_TURN).filter(|&i| mask[i]).collect();
            let id = choices[choice.randbelow(choices.len())];
            let done = env.step(seat, id).unwrap().is_some();
            trace.push(id.to_string());
            if done {
                break;
            }
            if !env.seats_due()[seat] {
                seat = 1 - seat;
            }
        }
        trace
    }

    #[test]
    fn mask_matches_staging_across_decks() {
        for (i, deck) in decks().iter().enumerate() {
            for seed in 0..4 {
                trace(deck, seed, true, i == 9);
            }
        }
    }

    #[test]
    fn snaps_retreats_and_invalid_actions() {
        let decks = decks();
        let deck = &decks[0];
        let mut env = Env::reset(7, [deck, deck], false).unwrap();
        let before = env.observation(0).unwrap();
        assert!(env.step(2, END_TURN).is_err());
        assert!(env.step(0, ACTION_COUNT).is_err());
        assert_eq!(before, env.observation(0).unwrap());
        assert_eq!(env.step(0, SNAP).unwrap(), None);
        assert!(!env.legal_mask(0).unwrap()[SNAP]);
        assert!(!env.legal_mask(0).unwrap()[RETREAT]);
        assert_eq!(env.step(1, RETREAT).unwrap(), None);
        assert_eq!(env.step(0, END_TURN).unwrap(), None);
        assert_eq!(env.step(1, END_TURN).unwrap(), Some([1, -1]));
        assert!(env.legal_mask(0).unwrap().iter().all(|v| !v));
    }

    #[test]
    fn observations_do_not_expose_hidden_state() {
        let decks = decks();
        let mut env = Env::reset(17, [&decks[0], &decks[1]], false).unwrap();
        let before = env.observation(0).unwrap();
        let hand = env.game.players[1].hand[0];
        let deck = env.game.players[1].deck[0];
        let def = env.game.cards[hand].def;
        env.game.cards[hand].def = env.game.cards[deck].def;
        env.game.cards[deck].def = def;
        let hidden = env
            .game
            .lanes
            .iter()
            .position(|lane| !lane.revealed)
            .unwrap();
        env.game.lanes[hidden].def = env
            .game
            .location_pool
            .iter()
            .copied()
            .find(|def| def.id != env.game.lanes[hidden].def.id)
            .unwrap();
        env.views = [env.game.fork_owned(), env.game.fork_owned()];
        assert_eq!(before, env.observation(0).unwrap());
    }

    #[test]
    fn ego_controls_staging() {
        let decks = decks();
        let deck = &decks[1];
        let mut env = Env::reset(2, [deck, deck], false).unwrap();
        env.game.lanes[0].def = crate::locations::get("Ego").unwrap();
        env.game.lanes[0].revealed = true;
        env.views = [env.game.fork_owned(), env.game.fork_owned()];
        assert!(env.game.autoplay_controller(0).is_some());
        assert!(env.legal_mask(0).unwrap()[..69].iter().all(|v| !v));
        assert!(env.legal_mask(0).unwrap()[END_TURN]);
        assert_eq!(env.step(0, END_TURN).unwrap(), None);
        for action in &env.plans[0] {
            assert!(matches!(
                action,
                Action::Play { .. } | Action::Move { .. } | Action::Activate { .. }
            ));
        }
        env.step(1, END_TURN).unwrap();
        assert!(env.game.turn >= 2);
    }

    #[test]
    fn opponent_staging_is_private_until_resolution() {
        let decks = decks();
        let deck = &decks[1];
        let mut env = (0..100)
            .map(|seed| Env::reset(seed, [deck, deck], false).unwrap())
            .find(|env| env.legal_mask(1).unwrap()[..21].iter().any(|&v| v))
            .unwrap();
        let before = env.observation(0).unwrap();
        let mask = env.legal_mask(0).unwrap();
        let action = env.legal_mask(1).unwrap()[..21]
            .iter()
            .position(|&v| v)
            .unwrap();
        env.step(1, action).unwrap();
        assert_eq!(before, env.observation(0).unwrap());
        assert_eq!(mask, env.legal_mask(0).unwrap());
    }

    #[test]
    fn daredevil_sees_only_opponent_staging_on_turn_five() {
        let decks = decks();
        let deck = &decks[1];
        let mut env = Env::reset(3, [deck, deck], false).unwrap();
        env.game.turn = 5;
        env.game.players[0].energy = 5;
        env.game.players[1].energy = 5;
        let daredevil = env.game.create_card("Daredevil", 0);
        env.game.cards[daredevil].lane = Some(0);
        env.game.cards[daredevil].revealed = true;
        env.game.push_location(0, daredevil);
        env.game.start_turn();
        env.staging_queue = std::mem::take(env.game.staging_order.as_mut().unwrap());
        env.views = [env.game.fork_owned(), env.game.fork_owned()];
        assert_eq!(env.seats_due(), [false, true]);
        assert_eq!(
            env.phase().unwrap().kind,
            PhaseKind::Ordered { index: 0, total: 2 }
        );
        assert!(env.observation(0).is_err());
        assert!(env.step(0, END_TURN).is_err());
        let rejected = env.step_phase([Some(END_TURN), None]);
        assert!(rejected.rejected.iter().all(Option::is_some));
        assert_eq!(env.seats_due(), [false, true]);
        let action = env.legal_mask(1).unwrap()[..21]
            .iter()
            .position(|&v| v)
            .unwrap();
        let expected = match Env::action(env.view(1).unwrap(), 1, action).unwrap() {
            Action::Play { card, lane } => VisibleAction {
                card: env.game.cards[card].def.id,
                lane,
            },
            _ => unreachable!(),
        };
        env.step(1, action).unwrap();
        assert!(env.observation(0).is_err());
        env.step(1, END_TURN).unwrap();
        assert_eq!(env.seats_due(), [true, false]);
        assert_eq!(env.observation(0).unwrap().opponent_staged, [expected]);
        let action = env.legal_mask(0).unwrap()[..21]
            .iter()
            .position(|&v| v)
            .unwrap();
        env.step(0, action).unwrap();
        assert!(env.observation(1).is_err());
        env.step(0, END_TURN).unwrap();
        assert!(env.observation(1).unwrap().opponent_staged.is_empty());
    }
    #[test]
    fn ordered_phases_carry_stage_rng_and_mutations_into_resolution() {
        fn random_stage(game: &mut Game<'_>, _: usize, event: crate::game::Event) {
            if let crate::game::Event::CardStaged { card, .. } = event {
                let roll = game.rng.borrow_mut().randbelow(100) as i32 + 1;
                game.cards[card].permanent += roll;
            }
        }
        static OBSERVER: crate::cards::CardDef = crate::cards::CardDef {
            on_event: Some(random_stage),
            ..crate::cards::CardDef::vanilla("Wasp")
        };
        static VANILLA: crate::cards::CardDef = crate::cards::CardDef::vanilla("Wasp");
        let decks = decks();
        let deck = &decks[1];
        for first in 0..2 {
            let mut env = Env::reset(3, [deck, deck], false).unwrap();
            for lane in &mut env.game.lanes {
                lane.def = crate::locations::get("Ruins").unwrap();
                lane.revealed = true;
            }
            let observer = env.game.create_card("Wasp", 0);
            env.game.cards[observer].lane = Some(2);
            env.game.cards[observer].revealed = true;
            env.game.cards[observer].override_ability = Some(&OBSERVER);
            env.game.push_location(2, observer);
            env.views = [env.game.fork_owned(), env.game.fork_owned()];
            let actions: [usize; 2] = std::array::from_fn(|seat| {
                env.legal_mask(seat).unwrap()[..21]
                    .iter()
                    .position(|&v| v)
                    .unwrap()
            });
            let cards: [usize; 2] = std::array::from_fn(|seat| {
                match Env::action(&env.game, seat, actions[seat]).unwrap() {
                    Action::Play { card, .. } => card,
                    _ => unreachable!(),
                }
            });
            for card in cards {
                env.game.cards[card].override_ability = Some(&VANILLA);
            }
            env.views = [env.game.fork_owned(), env.game.fork_owned()];
            env.staging_queue = vec![first, 1 - first];
            let mut powers = [0; 2];
            for seat in [first, 1 - first] {
                env.step(seat, actions[seat]).unwrap();
                powers[seat] = env.views[seat].cards[cards[seat]].permanent;
                if seat != first {
                    assert_eq!(env.observation(seat).unwrap().opponent_staged.len(), 1);
                    assert_eq!(env.views[seat].cards[cards[first]].permanent, powers[first]);
                }
                env.step(seat, END_TURN).unwrap();
            }
            for seat in 0..2 {
                assert_eq!(env.game.cards[cards[seat]].permanent, powers[seat]);
            }
        }
    }

    #[test]
    fn daredevil_queue_records_trigger_time_not_end_of_turn_scan() {
        fn silence(game: &mut Game<'_>, _: usize) {
            for card in &mut game.cards {
                if card.default_ability().id == "Daredevil" {
                    card.abilities_removed = true;
                }
            }
        }
        static SILENCER: crate::cards::CardDef = crate::cards::CardDef {
            start_turn: Some(silence),
            ..crate::cards::CardDef::vanilla("Wasp")
        };
        let decks = decks();
        let deck = &decks[1];
        for (silencer_order, daredevil_order, expected) in [(1, 2, vec![]), (2, 1, vec![1, 0])] {
            let mut env = Env::reset(3, [deck, deck], false).unwrap();
            env.game.turn = 5;
            env.game.current_location = 3;
            let daredevil = env.game.create_card("Daredevil", 0);
            env.game.cards[daredevil].lane = Some(0);
            env.game.cards[daredevil].revealed = true;
            env.game.cards[daredevil].latest_resolution_order = daredevil_order;
            env.game.push_location(0, daredevil);
            let silencer = env.game.create_card("Wasp", 0);
            env.game.cards[silencer].lane = Some(1);
            env.game.cards[silencer].revealed = true;
            env.game.cards[silencer].override_ability = Some(&SILENCER);
            env.game.cards[silencer].latest_resolution_order = silencer_order;
            env.game.push_location(1, silencer);
            env.game.start_turn();
            assert_eq!(env.game.staging_order.as_ref().unwrap(), &expected);
        }
    }

    #[test]
    fn two_daredevils_follow_resolution_order_and_repeat_phases() {
        let decks = decks();
        let deck = &decks[1];
        let mut env = Env::reset(3, [deck, deck], false).unwrap();
        env.game.turn = 5;
        env.game.players[0].energy = 5;
        env.game.players[1].energy = 5;
        for (seat, lane, order) in [(0, 0, 9), (1, 1, 2)] {
            let card = env.game.create_card("Daredevil", seat);
            env.game.cards[card].lane = Some(lane);
            env.game.cards[card].revealed = true;
            env.game.cards[card].latest_resolution_order = order;
            env.game.push_location(lane, card);
        }
        env.game.start_turn();
        env.staging_queue = std::mem::take(env.game.staging_order.as_mut().unwrap());
        env.views = [env.game.fork_owned(), env.game.fork_owned()];
        assert_eq!(env.staging_queue, [0, 1, 1, 0]);
        assert!(env.observation(1).is_err());
        let play = env.legal_mask(0).unwrap()[..21]
            .iter()
            .position(|&v| v)
            .unwrap();
        env.step(0, play).unwrap();
        env.step(0, END_TURN).unwrap();
        assert_eq!(env.observation(1).unwrap().opponent_staged.len(), 1);
        let play = env.legal_mask(1).unwrap()[..21]
            .iter()
            .position(|&v| v)
            .unwrap();
        env.step(1, play).unwrap();
        env.step(1, END_TURN).unwrap();
        assert_eq!(env.seats_due(), [false, true]);
        assert_eq!(env.observation(1).unwrap().opponent_staged.len(), 1);
        env.step(1, END_TURN).unwrap();
        assert_eq!(env.seats_due(), [true, false]);
        assert_eq!(env.observation(0).unwrap().opponent_staged.len(), 1);
        env.step(0, END_TURN).unwrap();
        assert!(env.game.turn > 5);
    }

    #[test]
    fn phase_policy_uses_only_due_seats() {
        let decks = decks();
        let deck = &decks[1];
        let mut env = Env::reset(9, [deck, deck], false).unwrap();
        assert_eq!(env.phase().unwrap().kind, PhaseKind::Simultaneous);
        loop {
            let phase = env.phase().unwrap();
            let mut actions = [None; 2];
            for (seat, action) in actions.iter_mut().enumerate() {
                assert_eq!(phase.observations[seat].is_some(), phase.due[seat]);
                if phase.due[seat] {
                    assert!(phase.masks[seat][END_TURN]);
                    *action = Some(END_TURN);
                }
            }
            let result = env.step_phase(actions);
            assert_eq!(result.rejected, [None, None]);
            if result.outcome.is_some() {
                break;
            }
        }
        assert_eq!(env.phase().unwrap().kind, PhaseKind::Terminal);
    }

    #[test]
    fn simultaneous_stakes_are_buffered_and_seeded() {
        let decks = decks();
        let deck = &decks[1];
        let mut a = Env::reset(3, [deck, deck], false).unwrap();
        let mut b = Env::reset(3, [deck, deck], false).unwrap();
        a.step(0, SNAP).unwrap();
        a.step(1, SNAP).unwrap();
        b.step(1, SNAP).unwrap();
        b.step(0, SNAP).unwrap();
        assert_eq!(a.observation(0).unwrap().next_cubes, 1);
        for seat in 0..2 {
            a.step(seat, END_TURN).unwrap();
            b.step(seat, END_TURN).unwrap();
        }
        assert_eq!(a.observation(0).unwrap(), b.observation(0).unwrap());
        assert_eq!(a.observation(0).unwrap().current_cubes, 4);
        let mut env = Env::reset(3, [deck, deck], false).unwrap();
        let result = env.step_phase([Some(SNAP), Some(RETREAT)]);
        assert_eq!(result.rejected, [None, None]);
        let result = env.step_phase([Some(END_TURN), Some(END_TURN)]);
        assert_eq!(result.rejected, [None, None]);
        assert_eq!(result.outcome, Some([1, -1]));
        let mut env = Env::reset(3, [deck, deck], false).unwrap();
        assert_eq!(
            env.step_phase([Some(RETREAT_LATER), Some(RETREAT_LATER)])
                .rejected,
            [None, None]
        );
        assert_eq!(
            env.step_phase([Some(END_TURN), Some(END_TURN)]).outcome,
            Some([0, 0])
        );
        let mut env = Env::reset(3, [deck, deck], false).unwrap();
        assert_eq!(
            env.step_phase([Some(RETREAT), Some(RETREAT)]).rejected,
            [None, None]
        );
        assert_eq!(
            env.step_phase([Some(END_TURN), Some(END_TURN)]).outcome,
            Some([0, 0])
        );
    }

    #[test]
    fn snap_and_card_calls_are_order_independent_for_hydra_bob() {
        let decks = decks();
        let deck = &decks[1];
        let mut a = Env::reset(3, [deck, deck], false).unwrap();
        for lane in &mut a.game.lanes {
            lane.def = crate::locations::get("Ruins").unwrap();
            lane.revealed = true;
        }
        let bob = a.game.create_card("HydraBob", 0);
        a.game.cards[bob].lane = Some(0);
        a.game.cards[bob].revealed = true;
        a.game.push_location(0, bob);
        a.views = [a.game.fork_owned(), a.game.fork_owned()];
        let action = a.legal_mask(1).unwrap()[..21]
            .iter()
            .position(|&v| v)
            .unwrap();
        let mut b = a.snapshot();
        a.step(0, SNAP).unwrap();
        a.step(1, action).unwrap();
        b.step(1, action).unwrap();
        b.step(0, SNAP).unwrap();
        for seat in 0..2 {
            a.step(seat, END_TURN).unwrap();
            b.step(seat, END_TURN).unwrap();
        }
        assert_ne!(a.game.cards[bob].lane, Some(0));
        assert_eq!(a.observation(0).unwrap(), b.observation(0).unwrap());
    }

    #[test]
    fn seat_order_preserves_private_views_and_resolution() {
        let decks = decks();
        let deck = &decks[1];
        let seed = (0..100)
            .find(|&seed| {
                let env = Env::reset(seed, [deck, deck], false).unwrap();
                (0..2).all(|seat| env.legal_mask(seat).unwrap()[..21].iter().any(|&v| v))
                    && env.game.daredevil_viewer().is_none()
            })
            .unwrap();
        let mut a = Env::reset(seed, [deck, deck], false).unwrap();
        let mut b = Env::reset(seed, [deck, deck], false).unwrap();
        let initial = [a.observation(0).unwrap(), a.observation(1).unwrap()];
        let actions = std::array::from_fn::<_, 2, _>(|seat| {
            a.legal_mask(seat).unwrap()[..21]
                .iter()
                .position(|&v| v)
                .unwrap()
        });
        a.step(0, actions[0]).unwrap();
        b.step(1, actions[1]).unwrap();
        assert_eq!(a.observation(1).unwrap(), initial[1]);
        assert_eq!(b.observation(0).unwrap(), initial[0]);
        a.step(1, actions[1]).unwrap();
        b.step(0, actions[0]).unwrap();
        for seat in 0..2 {
            assert_eq!(a.observation(seat).unwrap(), b.observation(seat).unwrap());
        }
        a.step(0, END_TURN).unwrap();
        b.step(1, END_TURN).unwrap();
        assert_eq!(a.step(1, END_TURN).unwrap(), b.step(0, END_TURN).unwrap());
        for seat in 0..2 {
            assert_eq!(a.observation(seat).unwrap(), b.observation(seat).unwrap());
        }
    }

    #[test]
    fn seeded_trajectories_ignore_worker_count() {
        fn send<T: Send>() {}
        send::<Game<'static>>();
        send::<Env>();
        let deck = &decks()[3];
        let reference: Vec<_> = (0..8).map(|seed| trace(deck, seed, false, false)).collect();
        for workers in [1, 4, 8] {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(workers)
                .build()
                .unwrap();
            let actual = pool.install(|| {
                use rayon::prelude::*;
                (0..8)
                    .into_par_iter()
                    .map(|seed| trace(deck, seed, false, false))
                    .collect::<Vec<_>>()
            });
            assert_eq!(reference, actual);
        }
    }
}
