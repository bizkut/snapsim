use numpy::{
    PyReadonlyArray2, PyReadwriteArray1, PyReadwriteArray2, PyReadwriteArray3,
    PyUntypedArrayMethods,
};
use pyo3::{exceptions::PyValueError, prelude::*};
use rayon::prelude::*;
use snapsim::env::{ACTION_COUNT, Env, Observation, PhaseKind};
use std::sync::Mutex;

const OBS_SIZE: usize = 4096;
const NOOP: i64 = -1;

fn error(message: impl ToString) -> PyErr {
    PyValueError::new_err(message.to_string())
}

// A length-prefixed UTF-8 string avoids hashes and gives an exact, reversible ID.
fn text(out: &mut Vec<i32>, value: &str) {
    out.push(value.len() as i32);
    out.extend(value.bytes().map(i32::from));
}
fn observation(obs: &Observation) -> Vec<i32> {
    let mut v = vec![
        obs.turn.into(),
        obs.total_turns.into(),
        obs.energy,
        obs.deck_count as i32,
        obs.opponent_hand_count as i32,
        obs.opponent_deck_count as i32,
        obs.current_cubes as i32,
        obs.next_cubes as i32,
        obs.snapped[0] as i32,
        obs.snapped[1] as i32,
        obs.ended as i32,
    ];
    for loc in obs.locations {
        text(&mut v, loc.unwrap_or(""));
    }
    v.push(obs.hand.len() as i32);
    for card in &obs.hand {
        text(&mut v, card.id);
        v.extend([card.cost, card.power]);
    }
    for lane in 0..3 {
        v.push(obs.face_down_opponent[lane] as i32);
        for side in 0..2 {
            v.push(obs.board[lane][side].len() as i32);
            for card in &obs.board[lane][side] {
                text(&mut v, card.id);
                v.extend([card.cost, card.power]);
            }
        }
    }
    v.push(obs.opponent_staged.len() as i32);
    for action in &obs.opponent_staged {
        text(&mut v, action.card);
        v.push(action.lane as i32);
    }
    v
}

#[pyclass]
pub struct VecEnv {
    envs: Mutex<Vec<Env>>,
    decks: Vec<[Vec<String>; 2]>,
    recruit: bool,
    pool: rayon::ThreadPool,
}

#[pymethods]
impl VecEnv {
    #[new]
    #[pyo3(signature = (decks, seeds, recruit=false, threads=1))]
    fn new(
        decks: Vec<[Vec<String>; 2]>,
        seeds: Vec<i64>,
        recruit: bool,
        threads: usize,
    ) -> PyResult<Self> {
        if decks.is_empty() || decks.len() != seeds.len() || threads == 0 {
            return Err(error(
                "decks and seeds must have equal nonzero length; threads must be positive",
            ));
        }
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .map_err(error)?;
        let envs = decks
            .iter()
            .zip(seeds)
            .map(|(deck, seed)| {
                Env::reset(seed.into(), [&deck[0], &deck[1]], recruit).map_err(error)
            })
            .collect::<PyResult<_>>()?;
        Ok(Self {
            envs: Mutex::new(envs),
            decks,
            recruit,
            pool,
        })
    }
    fn __len__(&self) -> usize {
        self.decks.len()
    }
    fn reset(&mut self, indices: Vec<usize>, seeds: Vec<i64>) -> PyResult<()> {
        if indices.len() != seeds.len() {
            return Err(error("indices and seeds must have equal length"));
        }
        let envs = self.envs.get_mut().map_err(error)?;
        let mut seen = vec![false; envs.len()];
        let mut replacements = Vec::with_capacity(indices.len());
        for (&index, &seed) in indices.iter().zip(&seeds) {
            if index >= seen.len() || seen[index] {
                return Err(error("indices must be distinct and in range"));
            }
            seen[index] = true;
            let deck = &self.decks[index];
            replacements
                .push(Env::reset(seed.into(), [&deck[0], &deck[1]], self.recruit).map_err(error)?);
        }
        for (index, env) in indices.into_iter().zip(replacements) {
            envs[index] = env;
        }
        Ok(())
    }
    fn observe(
        &self,
        mut phase_info: PyReadwriteArray2<'_, i32>,
        mut due: PyReadwriteArray2<'_, u8>,
        mut obs: PyReadwriteArray3<'_, i32>,
        mut masks: PyReadwriteArray3<'_, u8>,
    ) -> PyResult<()> {
        let envs = self.envs.lock().map_err(error)?;
        let n = envs.len();
        if phase_info.shape() != [n, 3]
            || due.shape() != [n, 2]
            || obs.shape() != [n, 2, OBS_SIZE]
            || masks.shape() != [n, 2, ACTION_COUNT]
        {
            return Err(error("invalid output shape"));
        }
        let mut phase_info = phase_info.as_array_mut();
        let mut due = due.as_array_mut();
        let mut obs = obs.as_array_mut();
        let mut masks = masks.as_array_mut();
        phase_info.fill(0);
        due.fill(0);
        obs.fill(0);
        masks.fill(0);
        for (i, env) in envs.iter().enumerate() {
            let phase = env.phase().map_err(error)?;
            match phase.kind {
                PhaseKind::Simultaneous => phase_info[[i, 0]] = 0,
                PhaseKind::Ordered { index, total } => {
                    phase_info[[i, 0]] = 1;
                    phase_info[[i, 1]] = index as i32;
                    phase_info[[i, 2]] = total as i32;
                }
                PhaseKind::Terminal => phase_info[[i, 0]] = 2,
            }
            for seat in 0..2 {
                if let Some(view) = &phase.observations[seat] {
                    let encoded = observation(view);
                    if encoded.len() + 1 > OBS_SIZE {
                        return Err(error("observation exceeds fixed buffer"));
                    }
                    due[[i, seat]] = 1;
                    obs[[i, seat, 0]] = encoded.len() as i32;
                    for (j, value) in encoded.into_iter().enumerate() {
                        obs[[i, seat, j + 1]] = value;
                    }
                    for (j, legal) in phase.masks[seat].iter().enumerate() {
                        masks[[i, seat, j]] = *legal as u8;
                    }
                }
            }
        }
        Ok(())
    }
    fn step(
        &mut self,
        py: Python<'_>,
        actions: PyReadonlyArray2<'_, i64>,
        mut rewards: PyReadwriteArray2<'_, i64>,
        mut terminal: PyReadwriteArray1<'_, u8>,
        mut rejected: PyReadwriteArray2<'_, u8>,
    ) -> PyResult<Vec<[Option<String>; 2]>> {
        let envs = self.envs.get_mut().map_err(error)?;
        let n = envs.len();
        if actions.shape() != [n, 2]
            || rewards.shape() != [n, 2]
            || terminal.shape() != [n]
            || rejected.shape() != [n, 2]
        {
            return Err(error("invalid array shape"));
        }
        let a = actions.as_array();
        let mut inputs = Vec::with_capacity(n);
        for i in 0..n {
            let mut pair = [None; 2];
            for seat in 0..2 {
                let id = a[[i, seat]];
                if id != NOOP {
                    pair[seat] = Some(usize::try_from(id).map_err(|_| error("invalid action"))?);
                }
            }
            inputs.push(pair);
        }
        let results: Vec<_> = py.allow_threads(|| {
            self.pool.install(|| {
                envs.par_iter_mut()
                    .zip(inputs)
                    .map(|(env, actions)| env.step_phase(actions))
                    .collect()
            })
        });
        let mut rewards = rewards.as_array_mut();
        let mut terminal = terminal.as_array_mut();
        let mut rejected = rejected.as_array_mut();
        rewards.fill(0);
        terminal.fill(0);
        rejected.fill(0);
        let mut reasons = Vec::with_capacity(n);
        for (i, result) in results.into_iter().enumerate() {
            if let Some(cubes) = result.outcome {
                terminal[i] = 1;
                rewards[[i, 0]] = cubes[0];
                rewards[[i, 1]] = cubes[1];
            }
            for seat in 0..2 {
                rejected[[i, seat]] = result.rejected[seat].is_some() as u8;
            }
            reasons.push(result.rejected);
        }
        Ok(reasons)
    }
}

pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<VecEnv>()?;
    module.add("ENV_OBS_SIZE", OBS_SIZE)?;
    module.add("ENV_ACTION_COUNT", ACTION_COUNT)?;
    module.add("ENV_NOOP", NOOP)
}
