# Original overview

The MARVEL SNAP simulator behind Snapper-Assist: card and location rules, the rollout advisor, the SNAP observation adapter, the deck builder and the replay checker, with the `snapsim` command line and the `snapsim_py` Python module. It began as an exact port of the Python simulator (`Marvel-snap-simulator`), whose 84 deck cards, 10 tokens and 84 locations it still matches on the [parity gates](parity.md#python-reference-frozen); everything added since comes from the game client's own rule definitions (Android 59.15.1), and every card in the nine SnapComplete decks of November 2026 (`decks/snapcomplete-2026-11.txt`) is supported. Position validation also rejects integers too large for their Rust fields, which Python accepted. See [Python reference (frozen)](parity.md#python-reference-frozen).

Coverage (`snapsim coverage`, November 23, 2026): the registry holds 698 card IDs (including tokens, skills, spells and created cards) and 187 location IDs. Every released character in the catalog (560, including the 61 team and Champion variants) and every released location with an ID (184) is registered with client-derived rules; some unreleased catalog entries are not. Registration does not mean every restored mid-game position is exact: when a position depends on game state the readers do not export, strict advice declines and names that state (see the coverage audit). Mephisto's in-match power and play follow the client, but his extra winnings are not modelled (no client rule found), so stake estimates decline when he may be in play.

## Interfaces

The simulator has no HTTP server or web API. Use it as:

* the Rust library (`crates/snapsim`);
* the `snapsim` command line and the separate tools in `crates/snapsim/src/bin` (for example `snapsim-replaycheck`);
* the Python module `snapsim_py` (see [Advisor and Python binding](advisor.md)), which Snapper-Assist loads in-process. Snapper-Assist hosts the web page and its own HTTP API.

The Rust `env::Env` exposes seeded two-seat games with action-at-a-time staging:
`reset`, `legal_mask`, `step`, and seat-relative `observation`. Action IDs 0–20
play a hand slot to a lane, 21–56 move a board slot to a lane, 57–68 activate a
board slot, and 69–72 end the turn, snap, retreat now, or retreat later. Board slots are four per
lane. `phase` returns the phase kind (including the ordered index and count),
seats due, their private observations and masks;
`step_phase` atomically submits one action for each due seat. Normal staging is
simultaneous; Daredevil queues ordered phases, revealing completed staged cards
only to the next acting seat. Two Daredevils can produce four phases. Terminal
steps return signed cube results.

The action space omits staging undo. The client allows undo of its most recent
staged play, move, or activation (`Card::IsStageUndoable`, `0x7cb0e74`);
the same final staging sequence can instead be submitted directly. The lower-level
game API additionally supports returning the latest staged play to hand.

Stake requests are buffered until the phase commits: unlike the client, a snap
cannot be observed and answered mid-phase. Simultaneous requests use seeded
ordering from the compare model; this random ordering is not proven client
behavior. Observations otherwise exclude opposing hand/deck identities, hidden
cards, unrevealed locations, and the random stream.
This also omits opposing deck cards explicitly revealed by effects such as
Magus (`MagusBehavior::CopyForUnspentEnergy`, `0x7eef1d4`): the simulator does
not track per-player visibility of deck cards.

`snapsim_py.VecEnv(decks, seeds, recruit=False, threads=1)` runs one game per
pair of 12-card ID lists in `decks`, using independent integer seeds and a
private Rayon pool. `reset(indices, seeds)` replaces selected games; indices
must be distinct. `observe(phase, due, obs, masks)` writes caller-owned NumPy
arrays: `int32[N,3]`, `uint8[N,2]`, `int32[N,2,4096]`, and `uint8[N,2,73]`,
respectively. Phase rows are `[kind, index, total]`, where kind is 0 for
simultaneous, 1 for ordered (zero-based index and phase count), and 2 for
terminal; index and total are zero otherwise. A
non-due seat has all-zero observation and mask. The first observation integer
is the used payload length; the remaining unused integers are zero. Payload:
turn, total turns, energy, own deck count, opponent hand and deck counts,
current and next cubes, own and opponent snapped flags, own ended flag; then
three location strings, hand count and cards, then for each lane the opposing
face-down count and own and opponent board lists, then staged-opponent count and
entries. Each list starts with its count; a card is ID string, cost, power;
a staged entry is ID string, lane. Strings are UTF-8 byte length followed by
one integer per byte; empty location strings denote unrevealed locations.
Oversized observations raise rather than truncate.

`step(actions, rewards, terminal, rejected)` takes `int64[N,2]` action IDs
or `ENV_NOOP` (-1) for seats not due, and writes signed cube results to
`int64[N,2]`, terminal flags to `uint8[N]`, and rejection flags to
`uint8[N,2]`. Its return value contains per-seat rejection reasons or `None`;
a rejected environment remains unchanged. Terminal games remain terminal
until reset. `ENV_ACTION_COUNT` and `ENV_OBS_SIZE` expose the array dimensions.
Stepping releases the GIL and parallelises across games; seed trajectories do
not depend on the thread count. NumPy is required at runtime; the extension
retains its Python 3.9 stable ABI.
When movement-source ordering is unknown, only the ambiguous move is omitted
from the mask. `VecEnv.omitted_tie_moves()` reports cumulative omitted mask
entries per environment since reset; repeated observations count again.

`Env::from_sampled_position` constructs a planning view from a position and
explicit stakes, sampling unobserved cards and locations. It does not restore
ordered phases or pending actions. The Python
`sample_position_masks(position_json, current, next, maximum, can_raise, snapped)`
returns coverage and eight diagnostic seat-zero masks, not recorded data or
validated legality labels. Agreement across samples is not proof of invariance:
callers must independently establish phase and stake state, reconstruction
coverage and the absence of hidden-state reads in every applicable legality
hook before using a mask for training.
Restored positions with potentially conflicting equal-priority movement sources
are marked `move_source_tie_unknown`: the reader does not export
`MovableByPlayerComponent.CreatedTimestamp`, which the client uses to break ties.

### Visible-recording trajectories

`crates/snapsim-py/trajectory_convert.py` and its `convert(paths, output_dir,
press_dir=None)` function accept SNAP observation JSONL paths and optional
matching press logs. Install NumPy in the Python environment. The Snapper-Assist
checkout also provides `scripts/convert_trajectories.py`. For example:

```sh
python crates/snapsim-py/trajectory_convert.py --output-dir "$WORK/datasets/trajectories" "$RECORDING"
```

Pass only observation recordings, not press logs, as positional inputs; a
matching `<recording-stem>.log.jsonl` is loaded separately if present. Each
`trajectory-NNNNNN.npz` holds one match segment, and `report.json` describes
counts and skipped reasons. The output is private data and must not be
committed. Only the recording player's actions are extracted; opponent actions
are observable only when revealed. Sparse captures cannot establish every
input, ordering, activation, retreat, or match result, so ambiguous cases are
skipped rather than guessed.

Shard schema (one row per extracted action):

* `observation`: `int32[N,4096]`, the VecEnv seat-relative encoding described
  above; it contains no opponent hidden identities.
* `action`: `int16[N]`, the Env action ID. `turn`: `int16[N]`.
* `legal_mask`: `uint8[N,73]`. Its bits 0–69 are usable only where
  `mask_valid` is true; bits 70–72 only where `stake_mask_valid` is true.
  `mask_reason` records why a mask was rejected, and `mask_basis` names the
  proof for an accepted mask (`static_visible_only`).
  `mask_blockers` is a comma-separated set of independently failing gates:
  `opponent_hidden`, `copied_or_attached`, `effect_provenance`, `visible_hook`,
  and `location_hook`.
  After `--remask`, `mask_pool` records `recruit` or `full`; the report lists
  hidden-card blockers. Incomplete history falls back to the full pool.
* `snap_flags_known` identifies whether both encoded snap flags are known.
  Unknown flags are neutral zeros, not observed false values; consumers must
  use this boolean. `snap_flags_basis` is `fields`, `no_raise_proof`,
  `press_log`, `mode_rule`, or `unknown`. `mode_rule` follows
  `compare::stake_can_raise`, whose Recruit setting was motivated by seven
  recordings (702 frames) without a projected raise; this evidence is a
  mode-model assumption, not an independently complete client proof.
* `match_outcome` and `turn_outcome` are signed `int8`; `cube_result` is
  signed `int16`. Their corresponding `_valid` boolean arrays distinguish
  unknown from zero. A turn result is currently available only for an
  observed, non-conceded final-turn result.
* `source_recording`, `source_stream`, `source_sequence`, and `source_line`
  locate the original frame for a future mask-only pass.

The initial conversion marks every mask invalid. Remasking separately checks
opponent-hidden sources and recorded-seat/public effects. SNAP does not export
complete copied/attached abilities or persistent card/player effects, and
visible-card and location hooks may be unreviewed. Proven complete provenance
for those effects, together with reviewed hooks and coverage, would permit
admission. Do not treat a zero-filled invalid mask as a legal-action label. Once
the Python extension is built, `--remask` reopens the supplied recordings and
re-evaluates only mask admission and legality; it does not regenerate actions.
An accepted mask requires the static hidden-state gate, restored-position
coverage, and eight matching diagnostic samples. The sample agreement is a
consistency check, not the proof. Stake masks remain separately invalid when
stake provenance is insufficient.
Schema-2 provenance may include a card `zone`: `board` and `hand` are active
holders, while `graveyard`, `banished`, and `trinkets` provide identities
without contributing active-effect inventories. An absent zone preserves
compatibility with older observations.

```python
import numpy as np
with np.load("trajectory-000001.npz", allow_pickle=False) as shard:
    usable = shard["mask_valid"]
    observations = shard["observation"][usable]
    actions = shard["action"][usable]
    masks = shard["legal_mask"][usable]
```

Write work files (benchmarks, replay reports, recorded cases) under a persistent folder such as `~/.cache/snapsim-work`, not `/tmp`. The examples in this documentation use `WORK="${WORK:-$HOME/.cache/snapsim-work}"`.

## Environment

Set these variables to local paths before running the examples or parity gates that need them:

| Variable | Purpose |
|---|---|
| `SNAPSIM_REFERENCE` | Frozen Python simulator checkout; also set `PYTHONPATH="$SNAPSIM_REFERENCE"` for reference scripts |
| `SNAPSIM_PYTHON` | Python interpreter with the reference dependencies installed |
| `SNAPSIM_RUNS` | Snapper-Assist recordings directory |
| `SNAPSIM_SNAPSHOTS` | SNAP snapshots directory |
| `SNAPSIM_FIXTURES` | SNAP observation fixtures directory |
| `SNAPSIM_ORACLES` | Directory for generated parity oracle files |

The recorded-deck case in `parity/compare_harness.py` is optional; pass `--recorded-deck FILE` to include it.

