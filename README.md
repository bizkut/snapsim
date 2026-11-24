# SnapSim

The MARVEL SNAP simulator behind Snapper-Assist: card and location rules, the rollout advisor, the SNAP observation adapter, the deck builder and the replay checker, with the `snapsim` command line and the `snapsim_py` Python module. It began as an exact port of the Python simulator (`Marvel-snap-simulator`), whose 84 deck cards, 10 tokens and 84 locations it still matches on the gates below; everything added since comes from the game client's own rule definitions (Android 59.15.1), and every card in the nine SnapComplete decks of November 2026 (`decks/snapcomplete-2026-11.txt`) is supported. Position validation also rejects integers too large for their Rust fields, which Python accepted. See "Python reference (frozen)" below.

Coverage (`snapsim coverage`, November 23, 2026): the registry holds 698 card IDs (including tokens, skills, spells and created cards) and 187 location IDs. Every released character in the catalog (560, including the 61 team and Champion variants) and every released location with an ID (184) is registered with client-derived rules; some unreleased catalog entries are not. Registration does not mean every restored mid-game position is exact: when a position depends on game state the readers do not export, strict advice declines and names that state (see the coverage audit). Mephisto's in-match power and play follow the client, but his extra winnings are not modelled (no client rule found), so stake estimates decline when he may be in play.

Documentation:

* [Card and engine technicalities](docs/card-technicalities.md): timing, mechanics, client evidence and restored-position limits.
* [Coverage audit](docs/coverage-audit.md): per-card and per-location status with client method addresses, approximations and the unexported state behind each restored-position gate.

## Interfaces

The simulator has no HTTP server or web API. Use it as:

* the Rust library (`crates/snapsim`);
* the `snapsim` command line and the separate tools in `crates/snapsim/src/bin` (for example `snapsim-replaycheck`);
* the Python module `snapsim_py` (see "Advisor and Python binding"), which Snapper-Assist loads in-process. Snapper-Assist hosts the web page and its own HTTP API.

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

Write work files (benchmarks, replay reports, recorded cases) under a persistent folder such as `~/.cache/snapsim-work`, not `/tmp`. The examples below use `WORK="${WORK:-$HOME/.cache/snapsim-work}"`.

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

## Command line

Build with `cargo build --release -p snapsim-cli`; run `target/release/snapsim --help` or add `--help` to any subcommand for the complete argument list.

* `deckbuild --collection FILE` searches owned cards. Optional inputs are `--decks FILE`, `--selected FILE`, or `--base-deck FILE_OR_IDS --try-cards IDS`. Search controls are `--count`, `--time-budget`, `--workers`, `--seed`, and `--confirm-rollouts`. `--progress` emits JSON progress and result lines, `--output FILE` saves the result, and Ctrl-C returns the best available result with status `cancelled`.
  `deckbuild --collection FILE --complete ID1,ID2,... [--decks FILE]` keeps the 1–11 distinct supported cards you specify and searches for the best owned cards to bring the deck to 12. The ranked completions (confirmed ones first) include a pasteable `deck_code` and list the added cards in `changes.in`. Their `ci95` is a Wilson interval over the games played, so a short run where every game was won still shows its uncertainty. `--complete` cannot be combined with `--selected`, `--base-deck`, or `--try-cards`.
* `position [--file FILE]` reads a position (stdin by default). `--snap` accepts a SNAP observation; supply its starting deck with `--snap-deck FILE` or `--deck-code CODE`. Search controls are `--mode recruit`, `--rollouts`, `--seed`, `--time-limit`, `--max-plans`, and `--approximate`.
* `coverage` prints supported card and location IDs.
* `compare --mode recruit` or `compare --deck-file FILE` compares advisors in paired seats. Options include `--hero-deck`, `--deck-prior`, `--games`, `--seed`, `--workers`, `--rollouts`, `--time-limit`, and the per-side `--a-rollouts`, `--a-playout`, `--a-prior` and corresponding `--b-*` flags.
* `benchmark --mode recruit` or `benchmark --deck-file FILE` compares controllers. It accepts `--games`, `--seed`, `--rollouts`, and `--opponent`.
* Plans include the client's player actions: playing a hand card, moving a movable card and activating an Activate ability. `--plays-only` (on `position`, `compare`, `benchmark` and `deckbuild`) restricts plans to plays, as the frozen reference did; the parity gates use it.
* The game-log replay checker is the separate binary `snapsim-replaycheck FILE...` (`cargo build --release -p snapsim --bin snapsim-replaycheck`). It replays recorded turns and reports each as consistent, inconsistent or not checkable; use it to validate rule changes against real matches. A turn with a previously revealed Nico Minoru or Maverick and a subsequent play by their owner is not checkable because their pending trigger is not recorded. A turn with Nico resolving before another play by her owner is also not checkable because her prepared spell is hidden. An opponent play after Ta Lo has potentially buffed their hidden hand is not checkable because the affected card and its power are not recorded.
  Pass `--coverage-json PATH` before the recordings to write per-released-card and per-location evidence. Consistent hook counts identify dispatches in a simulation that matched the observed turn, not proof that each hook produced a visible effect. Inconsistent counts use the closest trial and are not verified. Recorded stat-update source IDs are separate corroborating evidence; presence and recovered plays do not establish that an ability fired. Uncheckable turns have unknown simulated hooks, so the report separates observed update sources and presence-based reasons rather than attributing executions to them. Some dispatched hooks can evaluate a condition and do nothing. Local recordings and generated reports may contain private match data.
  To avoid attributing sampled hidden cards to the recording, hook totals include only definitions visible on the board/location or named by an observed new stat update during the turn; this conservative filter can omit genuine hand/deck and copied-ability hooks. Stat-update evidence requires the same visible entity and complete ledgers both before and after the turn. Recovered plays require enough recorded staging information; missing observations are not treated as zero plays.
  Collapsed Mine turns with visible Rocks are not checkable because recordings do not fully expose moves and activations needed to determine whether a player skipped. Vormir's per-player consumed triggers are likewise not present in live positions; coverage marks Vormir and Collapsed Mine state approximate.
  Time Theater's per-player draw-use state and Quantum Realm's historical base-power assignments may be absent from observations. When the missing state can affect a turn, replay checking marks it not checkable and strict advice reports the missing state; approximate advice remains available.
  The Abbey and The Raft have a shared one-shot location trigger whose consumed state is not exported. Previously revealed copies therefore decline strict advice and replay checking rather than assuming it is unused.
  Nebula's installed pre-end trigger and The Thing (First Steps)'s one-time fill-trigger consumption are not exported by the readers. Earlier revealed copies therefore have uncertain state in isolated positions; their Power alone does not establish those flags.
  Card augment attachments are not exported; a changed augment-sourced stat ledger makes the replay turn not checkable. A visible augment ledger also makes strict advice report uncertain card augment state.
  Wave and Stick install player-owned effects that are not exported. Strict advice reports uncertain state when a visible card or recorded play could have left one active; cards still in hand or deck do not cause a decline.
  For an opponent's revealed play, replay checking uses recorded stat-ledger updates proven to predate the turn to reconstruct hidden pre-play Power. When a relevant update's timing cannot be established, the turn is not checkable rather than guessing that Power.
  For Cave of the Dragon, the checker uses uniquely observed created-card identities and, when visible in separate frames, their reveal order; it still consumes the normal random draws. Frozen-reference checks do not use this evidence.

## Architecture

`snapsim` uses arena indices for cards and board lanes. `Game` owns the turn flow, player zones, priority, event dispatch and Ongoing recomputation; `policy` contains the heuristic, spread and margin planners. The game borrows a `GameRng`, so hooks can use `game.rng.borrow_mut()` and consume the same stream as the Python reference. `PythonRandom` implements CPython's integer seeding and sampling; its fixtures come from CPython 3.12.16. `data` exposes typed copies of the reference catalogs.

The Rust card catalog supplements `data/cards.json` with client-only definitions from
`data/client_cards.json`; website entries take precedence. Regenerate the supplement with
`python3 parity/generate_client_cards.py /path/to/59.15.1/rules/carddefs.json`.
This does not modify the frozen Python reference catalog.

`cards::registry()` (one descriptor slice per card-group module, listed in `cards::GROUPS`) and `locations::REGISTRY` contain rule descriptors. Card modules provide callbacks; see `cards/sword_master.rs` for a random effect and `cards/representative.rs` for reveal, ongoing, play and end-turn examples. Location modules provide callbacks; see `locations/turns.rs` for an end-turn rule. Each descriptor can provide initial per-instance state, and hooks can access the game's RNG. `Event` also exposes turn, reveal, play, addition, movement, destruction, discard, location reveal, energy and turn-count events. Hooks must preserve the Python event and RNG call order.
Cube accounting is opt-in. `snapsim compare --stakes` compares two stake policies
using equal `--rollouts` per seat for both play and visible-position win estimates.
`--a-snap-threshold` and `--b-snap-threshold` snap at or above the estimate;
omitting a threshold never snaps. `--a-retreat-threshold` and
`--b-retreat-threshold` retreat below it. The corresponding
`--{a,b}-{snap,retreat}-probability` options gate those decisions; the remaining
probability stays. This independent-action response model is a stand-in, not a
calibrated opponent. Results report mean net cubes and a duplicate-seat paired
confidence interval. The estimate is simulated against the advisor's playout
model and is not calibrated against live opponents.

Modes supply stakes with `--stake-initial`, `--stake-initial-next`,
`--stake-maximum`, `--stake-final-multiplier`, and `--stake-prevent-raise`.
The defaults are a configurable comparison scenario, not automatic league
detection. The recorded 20261121-232715-01d945f5 match shows 1/1 → 1/2 → 2/2
on turn 4, then 2/2 → 2/4 → 4/4 on the final turn; sampling alone does not
identify the boundary. Bounded instructions in `Game::UpdateCubeValueNextTurn`
(0x7db0df8–0x7db0e2c) compute `min(maximum, current × player multiplier ×
end-game multiplier × game multiplier)`. `Game::CheckIncreaseStakesForNextTurn`
(0x7db0fc4) promotes player and game multipliers, while
`CheckEndOfGameIncreaseStakesGameBehavior::CheckIncreaseStakesForEndGame`
(0x7dccd14) uses a configured end multiplier and caps the result. A supplied
initial `next` is a snapshot value; subsequent projections use the formula.
The concession request checks the displayed current stake, and result creation
copies that current stake (client 0x7e50204 and 0x7dd4324). The last player to
raise cannot concede on their own snap turn (0x7dd1bf4). Deferred concessions
are checked before reveal and stake promotion (0x7e09570); both players
conceding is a tie (0x7dd3a6c). Simultaneously sampled requests are ordered
randomly. Comparisons still label retreat and draw semantics provisional.

The JSON advisor accepts `stake_rollouts` (default 0), alongside
`position_rollouts`, and opt-in SNAP adapter option `include_stakes`. It reports
separate scenarios for observed or assumed local snap eligibility and six
opponent threshold policies. Values use simulated play and stake decisions for
both seats; they are policy sensitivity estimates, not fitted probabilities.
`stake_policy_rollouts` controls the minimum chosen-plan samples behind each
seat's stake decision (default 32). Search first shares this budget across
candidates using the normal advisor, then samples its chosen plan until that
minimum is met. The result includes histograms of candidate-plan counts and
chosen-plan samples; they describe the simulated policy, not calibration.
The simulator assumes an eight-cube cap if unobserved, standard final doubling,
and opponent eligibility if unknown; it omits scenarios whose displayed
current/next values cannot uniquely establish the pending multiplier. The outer
deadline and cancellation also stop nested stake-policy searches. An interrupted
scenario is reported as `not_enough_time` rather than a partial estimate; work
already inside an individual game operation can still finish after the deadline.
Mephisto's extra winnings are not modelled: no rule for them was found in the client's behaviour map or result builders, so the payout is presumably applied server-side (unverified):
stake advice declines when he is observed in play or exceeds 5% of sampled
end states. Otherwise it reports unknown sample counts and estimates from known
outcomes. This does not prevent ordinary board advice.


The engine exposes location replacement, delayed reveal scheduling, location cost hooks, ability removal, On Reveal and Ongoing repetition, movement, off-board placement, hand swaps and Horde creation. Lasting card costs are stored on cards. `Game::card_cost` mirrors Python's `energy_cost`: revealed locations modify the cost of hand cards only, in lane order, while board and deck cards keep their lasting cost. `Card::has_ongoing`, `can_reduce_power` and `reduce_power` support ability and immunity checks. `Game::copy_card` clones `CardState`, resets temporary board state and invokes the descriptor's optional `clear_reveal_state` hook. `cards::deck_eligible_ids()` matches the reference's sorted `deck_eligible_ids()`, which Agent 13 draws from. Registry membership alone is not a parity proof.

Copied text, merging and attached abilities follow the client. Copying text (`Card::CopyText`) loads the copied definition with fresh state and repeats its On Reveal under reveal-doubling effects; Nico Minoru's prepared spell is her current text, and her preparation trigger is tracked separately. Abilities one card attaches to another (Hawkeye's and Jessica Jones's end-of-turn buff, Web Sling's move-power trigger) are instances on that card with their turn, location and origin: an instance installed by the card's own text is replaced with that text, while one cloned from another card stays. A merge (Symbiote Spider-Man) adds the absorbed card's power from modifiers, copies its text and clones its attached abilities except those the receiver already has by default (`CloneExternalAbilities`).

When Stryfe uses a card's On Reveal, the borrowed reveal has its own temporary context; a card copied during that reveal retains Stryfe's persistent state rather than the borrowed context.

## Adding content

1. Add a module under `crates/snapsim/src/cards/` or `locations/`, define a descriptor with the card or location ID and the necessary hooks, and list it in that module's descriptor slice (cards; add a new module's `CARDS` and `TOKENS` slices to `cards::GROUPS` and `cards::TOKEN_GROUPS`) or in `locations::REGISTRY`. Tokens, which decks may not contain, go in the module's `TOKENS`. Use the catalog's current printed stats; do not invent stand-in rules.
2. Take behavior from the Android client (59.15.1), not from card text: the card's ability definitions and the methods they run, including which candidates a random choice draws from and how many rolls it makes (using `game.rng`). Cite the client method and address in a comment. Never commit client binaries, dumps or decompiled code.
3. Add focused boundary tests. For a random effect, test seeds and the order of random calls. New rules have no Python counterpart: check them against real matches that contain the card or location with `snapsim-replaycheck`.
4. Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` and `parity/gates.sh`. Gates for unchanged content must still pass. If the needed hook or state transition is unavailable, extend the core before claiming the rule is supported.

Known approximations of client rules:

* Random choices use the client's candidate set and number and order of rolls, but draw from `game.rng`; the client's own random stream is not reproduced.
* Rogue Scion of Division's repeated self-copy is stopped once every highest-cost target already has the same text removed. The client repeats this no-op up to its 32-ability depth limit; the cutoff avoids exponential expansion but omits redundant random rolls, which can change subsequent random outcomes.
* Effects that create random cards or locations draw only from implemented ones; the client draws from its full pools. Every released card and location is implemented, so this only matters for unreleased entries in the client's pools. Scarlet Witch Queen of Chaos's own pool is complete (13 curated and 11 collectible skills), so she matches the client's skill choice. Rama-Tut's pool is the client's hard-coded 107-card list (`data/past_years_pool.rs`).
* Nico Minoru's location spell and Omniversal Presence pick locations uniformly, while the client weights the pick (`GetRandomLocationDefId`). Merging of power from the same source, the order of conditional Ongoing effects that depend on each other, and gaps left by cards that cannot shift are not modelled.
* Visual-only abilities the client attaches (Tao Mandala, Zombie Galacti) are not modelled. `ReplacedCardDefId` follows the client's copy and clear paths, but no implemented card replaces another, so it is always empty.
* Live positions use the client's `LatestResolutionOrder` when the reader supplies it, and otherwise derive the order from played turn, priority and slot (see "Live state from SNAP observations"). None of the newly exported live fields has been checked against a live match yet.

## Advisor and Python binding

`snapsim::advise::advise_json(position_json, options_json)` takes a position in the `Position` JSON shape (validated by `Position::from_json`). Options are `rollouts`, `seed`, `max_plans`, `time_limit` (seconds or null), `approximate`, `playout` (`spread`, `heuristic` or `margin`), `workers` and `plays_only`. It computes coverage, sampling priors and status itself; strict searches return unsupported positions without running rollouts. Results keep `"engine": "simulator-rollout"`. Each plan lists its steps in `plays` in staging order. With `plays_only` a step is `{hand_index, card, location}`, as in the reference. Otherwise every step has a `kind`: `play` (with `hand_index`), or `move` and `activate`, which name the board card by `from` (its location) and `board_index` (its index in that location's `mine` list); `location` is a move's destination.

The optional `position_rollouts` (unsigned 32-bit, default 0) independently estimates the current position by sampling hidden information and letting the configured playout policy choose for both seats. When samples complete, `position` contains `win`, `draw`, `loss`, `samples`, and `approximate` (the latter reflects incomplete coverage). This estimates policy-vs-policy outcomes, not an optimal-play guarantee. Its random stream is separate from plan search; without a time limit enabling it does not change plan results. With `time_limit`, plan search receives 70% of the budget and position sampling runs afterward within the remaining shared deadline; fewer samples may complete and the position is omitted if none complete. Cancellation and strict unsupported coverage likewise can leave it absent. The same option passes through `snapsim_py.advise`.

The opt-in `best_reply` option (`true`, or `{"plans": N, "turns": 1|2}`; default off, so advice is unchanged) replaces the playout policy's opponent on the final turn with a best reply. In every sampled hidden state the opponent stages the best of a bounded set of its own plans against the plan being evaluated (for the `position` estimate, against the policy's plan for us), ranking win over draw over loss, and the estimates average over samples; it is not a worst case across hidden states. The candidate set is the policy's own plan, which stays and wins ties, plus up to `plans - 1` (default 32) maximal play plans of the sampled hand (plans that cannot take another card), each preceded by the policy's moves and activations. When more maximal plans exist a uniform subset is evaluated, and enumeration stops after 20,000 search nodes; `best_reply` in the result reports samples, evaluated plans, how often either cap applied, how often the reply changed the sample's result and rejected candidates. `turns: 2` also covers the turn before the last, with the policy playing the final turn after the reply. The opponent knows our plan and its own sampled hand but not more, which makes it stronger than a real opponent, and each sample costs up to `plans` playouts, so fewer samples fit in `time_limit`. Scripted seats (autoplay, Daredevil's ordered staging) fall back to the ordinary playout, and the stake scenarios of `stake_rollouts` keep the ordinary opponent. The option passes through `snapsim_py.advise`, `snapsim position --best-reply [--best-reply-plans N --best-reply-turns 1|2]` and `snapsim compare --a-best-reply/--b-best-reply`.

Red Onslaught's default and virtual copied generated abilities are fixed for a single advice request, including its position and stake rollouts; simulated stake continuations preserve those generated abilities from their game state. When the identities are not available from the observation, coverage reports them as unknown and the advice is approximate. Detected attached Red Onslaught abilities that cannot be reconstructed are declined even in approximate mode.

The `snapsim_py` module exposes `advise(position_json, options_json, cancel=None)`, `position_from_snap(observation_json, options_json)` (raising `snapsim_py.NotPlanningError` for frames that are not a planning point), `coverage`, `supported`, `deck_eligible_ids`, `recruit_card_pool`, `recruit_card_weights` and `deckbuild(args_json, progress=None, cancel=None)`. Values are JSON strings. It is built for the stable Python ABI (abi3, Python 3.9 or newer), so one build loads in any of those interpreters. Build it without installing into any virtual environment:

```sh
export PATH="$HOME/.cargo/bin:$PATH" CARGO_BUILD_JOBS=6
crates/snapsim-py/build-module.sh
```

It writes `snapsim_py.so` next to the release build (`target/release/`, or `$CARGO_TARGET_DIR/release/`). Snapper-Assist loads `target/release/snapsim_py.so` from this repository by default, or the file or directory named by `SNAPSIM_PY_PATH`. Generate and compare advisor cases from real recordings and SearchAI games with:

```sh
export PATH="$HOME/.cargo/bin:$PATH" CARGO_BUILD_JOBS=6
ROOT="$PWD"
WORK="${WORK:-$HOME/.cache/snapsim-work}"; mkdir -p "$WORK"; cd "$WORK"
PYTHONPATH="$SNAPSIM_REFERENCE" \
  "$SNAPSIM_PYTHON" "$ROOT/parity/record_advisor.py" \
  "$WORK/snapsim-advisor.jsonl" --runs "$SNAPSIM_RUNS"
cargo run --manifest-path "$ROOT/Cargo.toml" --release --bin snapsim-advisor-parity -- "$WORK/snapsim-advisor.jsonl"
```

The generator includes approximate positions, preserves watcher state across recording frames, and writes separate records for spread and margin. The gate matched 449 positions per policy (360 seeded SearchAI decisions and 89 real recorded positions), with 24 rollouts per position and one worker. The Python parity test also compares one and four workers.

## Live state from SNAP observations

`position_from_snap` maps the reader's per-card fields (Snapper-Assist `SNAP/README.md`) onto engine state:

| Reader field | Engine state |
|---|---|
| `latest_resolution_order` | resolution order; when absent, a deterministic order from played turn, priority, lane and slot |
| `turns_drawn` | `drawn_turn` and the Quickdraw extension flag: the last entry `Card.IsQuickdrawActive` accepts this turn, else the last entry; also the player's draw history |
| `activated`, `moves_per_turn` | activation flag, move history and moves used |
| `created`, `created_by_*`, `merged_to_entity_id` | card provenance; a merge target resolves to its card when it is visible |
| `stat_updates` | the Power/Cost ledger, with the source player mapped to a seat |
| graveyard and banished cards | both players' zones; a graveyard card's death lane stays unknown |

Stat updates are history, not deltas: the client's power and cost getters already report totals, and the client does not label which rows come from Ongoing effects. A row is removed only when it matches a contribution the engine recomputes from the same named active card or revealed location (same stat, operation, value and source). Rows from faction, augment or league updaters, and rows whose provenance is missing or ambiguous, stay historical. The reader does not report the order of different cards drawn on the same turn or where a graveyard card died.

Optional per-player `default_energy` and `max_energy` fields are forwarded when
present; the default is separate from the persistent maximum and current energy.
The simulator tracks each player's lifetime peak current energy as energy changes;
schema v2 does not export that historical peak, so Cyclops X-Men positions fail closed.
The reader does not guarantee either field. Public graveyard cards retain
`zone_position` order when every entry has a position. During simulation, the
combined discard/destroy graveyard preserves insertion order and removes cards
when they leave; restored discarded-vs-destroyed provenance may still be unknown.

When the reader omits destroyed locations, `position_from_snap` accepts
`lane_layout: {"complete":true,"lanes":[entity_id,null,entity_id]}` in its
options. The three entries are in original lane order, and null denotes a
destroyed lane. Supply it only when all three lane entity IDs were observed
before destruction; incomplete or inconsistent layouts are rejected. Without a
complete layout, frames with fewer than three locations remain unavailable.

For coverage of pending energy from a replaced Feast or Altar of Death,
`position_from_snap` accepts `previous_turn_locations` either in `options` or
in `observation.game` (options take precedence), for example
`{"complete":true,"ids":[["Feast","Westview"],[],["Ruins"]]}`. `ids` has
exactly three arrays in lane order, listing every identity that lane held at any
point during the previous turn; an empty array denotes no revealed identity.
Set `complete` only when the entire previous turn is known, including
locations replaced before the current observation. This is external
history, not a field currently guaranteed by the reader. When it is absent,
coverage fails closed from turn 2; known prior Feast or Altar of Death also
fails closed because the pending player energy grants are not exported.
The same `previous_turn_locations` object is accepted in `advise` options,
including through `snapsim_py.advise`, and overrides the position field.
Missing evidence yields a coverage reason: approximate mode continues with an
approximate result, while exact mode declines.

## Position materialization parity

`position::Position` deserializes Python's `Position.to_dict()` shape.
`build_game` samples hidden cards and locations with the supplied `PythonRandom`,
restores visible state and pending reveals, and leaves the RNG at the playout
state. `state_json` provides a canonical dump for differential testing. Known
catalog cards without rules and unknown locations use inert approximate
stand-ins; the Rust coverage layer reports unsupported content and state.

Generate positions from both seats of seeded SearchAI games, hand-made edge
cases, and recorded SNAP observations (using `LocationWatch` and `DeckWatch`):

```sh
export PATH="$HOME/.cargo/bin:$PATH"
WORK="${WORK:-$HOME/.cache/snapsim-work}"; mkdir -p "$WORK"
PYTHONPATH="$SNAPSIM_REFERENCE" \
  "$SNAPSIM_PYTHON" \
  parity/position_record.py --games 1 --seeds 20 \
  --recordings "$SNAPSIM_RUNS" \
  > "$WORK/snapsim-position-cases.jsonl"
cargo run --release -p snapsim --bin snapsim-position-parity -- \
  "$WORK/snapsim-position-cases.jsonl"
```

The generator reports unique position counts and rejected positions on stderr;
the comparison reports the first differing state field. The recording corpus is
local and may vary between runs.

## Parity and checks

`Position::from_json` validates the Python position format, with one deliberate
difference: integers must also fit the destination Rust field (`u8` turns,
`i32` power and energy, and `usize` counts). Python integers are unbounded.
The validation parity gate compares in-range cases to Python and requires
out-of-range cases to be rejected separately. Generate the two JSONL inputs
using the position and advisor recipes above, then run from an empty working directory:

```sh
ROOT="$PWD"
WORK="${WORK:-$HOME/.cache/snapsim-work}"; mkdir -p "$WORK/validation"; cd "$WORK/validation"
PYTHONPATH="$SNAPSIM_REFERENCE" \
  "$SNAPSIM_PYTHON" \
  "$ROOT/parity/validation_record.py" "$WORK/snapsim-validation.jsonl" \
  --corpus "$WORK/snapsim-position-cases.jsonl" --corpus "$WORK/snapsim-advisor.jsonl" \
  --runs "$SNAPSIM_RUNS"
cargo run --manifest-path "$ROOT/Cargo.toml" --bin snapsim-validation-parity -- \
  "$WORK/snapsim-validation.jsonl"
```

The gate matches 1,000 seeded full games each for spread and margin on `parity/corpus-full-*.json`: all 84 deck cards and all 84 locations, with decks drawn uniformly. It compares initial decks and locations, every turn's plans, reveal priority and lane powers, and the final winner. Merging the earlier per-group corpora exposed one cross-group bug: location cost modifiers were applied to board cards (Elektra at Elysium). `tests/costs.rs` covers it. `parity/corpus-{spread,margin}.json` is the original 25-card representative corpus that the first benchmark used.

```sh
export PATH="$HOME/.cargo/bin:$PATH" CARGO_TARGET_DIR="$PWD/target"
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release --bins
ROOT="$PWD"
WORK="${WORK:-$HOME/.cache/snapsim-work}"; mkdir -p "$WORK/bench"
for policy in spread margin; do
  (cd "$WORK/bench" &&
    PYTHONPATH="$SNAPSIM_REFERENCE" \
    "$SNAPSIM_PYTHON" \
      "$ROOT/parity/record.py" "$ROOT/parity/corpus-full-$policy.json" \
      --games 1000 --seed 20261201 > "$WORK/snapsim-$policy.jsonl")
  "$ROOT/target/release/snapsim-parity" "$ROOT/parity/corpus-full-$policy.json" \
    "$WORK/snapsim-$policy.jsonl"
done
```

`snapsim-parity` independently samples decks from each recorded seed, replays the game and reports the first differing JSON field. Measured full-content and advisor-rollout throughput are in `BENCHMARKS.md`.

## Python reference (frozen)

The Python simulator (`Marvel-snap-simulator`, branch `stronger-advice` in the `Marvel-snap-simulator-advice` worktree) is retired and receives no rule changes. It remains the oracle for the gates below. They prove that unchanged content still behaves exactly as it did there, and are expected to differ only where a rule is deliberately changed or added here. The scripts need that checkout on `PYTHONPATH` and a Python with `loguru` (Snapper-Assist's `.venv`). Run them from a temporary directory. The table abbreviates paths: `parity/...` means `$ROOT/parity/...` and a bare binary means `$ROOT/target/release/<binary>`, where `ROOT` is this repository. Judge each gate by its exit status and success line, not through a pipeline that can hide a failure.

Registering new cards changes `cards::deck_eligible_ids()`, which Agent 13 draws from and position sampling uses. Gates therefore pin it to `parity/reference-pool.json`, the frozen reference's 84 deck cards: the parity binaries call `cards::use_reference_pool()` themselves, and the CLI, `snapsim-replaycheck` and the Python module do the same under `SNAPSIM_REFERENCE_POOL=1`. Export that variable for every Python-side gate script.

The same pin restricts random location pools to `parity/reference-locations.json`, the reference's 84 locations, so locations added since do not change its draws. Gate scripts treat cards and locations beyond the reference as new content: coverage must still list every reference entry, and recordings containing new content are excluded from the Python replay comparison (the client replay gate still checks them).

Two kinds of record are reported rather than compared:

* New content: a position, case or recording that contains a card with rules here but not in the reference (`cards::beyond_reference`). The reference played such cards as unsupported stand-ins.
* Deliberate changes to existing rules, made to match the 59.15.1 client. A game that diverges because of an engine change is listed with its exact first-divergence path and values in `parity/divergences.json`; `snapsim-parity` fails on any other divergence and on a listed one that no longer occurs. A changed card keeps the reference's version in `cards/reference_rules.rs`, which the gates switch to with `cards::use_reference_rules()` or `SNAPSIM_REFERENCE_RULES=1`; live play never does. The `replay-client` gate then checks the client rules, these changes included, against every real recording. The changes:
  * Fantasticar selects visible end-of-turn triggers, including independently attached triggers after default text removal (`FantasticarBehavior::GetTargetCard` predicate `0x7ea7c8c`); reference rules retain the earlier hidden-trigger and removed-text checks.
  * Chamber does not record a zero-power modifier when no cards qualify (`ChamberBehavior::BuffPower`, `0x7e7fb14`); reference rules retain the earlier zero update.
  * Fantomex afflicts front-row enemies in location and slot order (`FantomexBehavior::AfflictFrontRowCards`, `0x7ea7ee4`); reference rules retain global resolution order.
  * Arana and Kid Omega reuse their pending next-play trigger and replace its source context on repeated activation (`NextCardPlayedAbilityDef::AddAbilityToPlayer`, `0x7e0cfa4`); reference rules retain independent pending effects.
  * Askanison rolls a fallback other location even when none can spawn (`Game::GetRandomLocationToSpawnCard`, `0x7db2b64`); reference rules return without the fallback roll.
  * Colossus's movement immunity requires its Ongoing ability to be active (`ColossusBehavior::SetCantBeDestroyedOrMovedOrWeakened`, `0x7e834e8`); reference rules retain the unconditional movement guard.
  * Player movement consumes only its client-priority source (`MoveSystem::FindMovableByPlayerSource`, `0x7c9d9ec`); reference rules retain the earlier all-sources consumption.
  * Intrinsic hidden Ongoing movement is suppressed where hidden Ongoing abilities are disabled (`MovableOngoingAbilityDef::.ctor`, `0x7e0c8e4`); reference rules retain the earlier unconditional movement permission.
  * Jeff's hidden ignore-restrictions ability is inactive when hidden Ongoing effects are suppressed or Jeff is staged; while active it permits moving into Space Throne up to its four physical slots (`Location::CanMoveCard`, `0x7ddba50`; `IgnoreMoveRestrictionsAction::ExecuteOnTarget`, `0x7d2324c`). Reference rules retain the previous override and capacity checks.
  * During staging, the latest staged move can return to its original location before capacity or movement restrictions are checked (`Location::CanMoveCard`, `0x7ddba50`); reference rules retain the previous check.
  * Strange Academy moves only revealed cards, and it and Spider Island use full movement legality followed by the client's random-location fallback (`StrangeAcademyBehavior::MoveAllCards`, `0x7d84854`; `Game::GetRandomLocationToMoveCard`, `0x7db2f50`). Reference rules retain their previous enumeration and destination selection.
  * Ongoing stat effects use the 59.15.1 client's Init → modifiers → Default → ConditionalCard → ConditionalLocation → PostConditional ordering, with player effects before locations and cards. Staging preserves non-staging contributions; conditional abilities that fail are retried after later successes. Reference rules retain the frozen ordering.
  * Revealed card trigger observers, including start-of-turn and post-start hooks, run in latest-resolution order (`Game::GetAllCardsForActivatingTriggerAbility`, `0x7dacfcc`); reference rules retain board order.
  * Morph completion and copied-text activation run only the selected default text's resolve ability (`Card::ActivateTextResolveAbility`, `0x7cbb424`), not foreign attached resolves. Reference rules retain the earlier all-resolves dispatch.
  * Repetition effects do not repeat hidden ability instances; normal resolution still executes them once. Reference rules retain the earlier repetition behavior.
  * Nested ability dispatch uses the client's 32-active-ability limit (`Ability::CanActivate`, `0x7c8a6e0`).
  * Corvus Glaive attempts two discards from a shuffled hand snapshot and counts both attempts for its +1 Max Energy even when the first target's discard trigger (Zombie Black Cat) already discarded the second (`DiscardCardsToGainEnergy`, `0x7e83c18`). `discard_card` returns false for any card not in the hand, as `Game::DiscardCard` (`0x7dabe68`) does, so hand snapshots (Corvus Glaive, MODOK, Bullseye) need no guard; Bullseye counts only successful discards. Neither Corvus nor Zombie Black Cat is in the frozen reference, which never discarded a non-hand card.
  * Activate staging checks visible activation abilities and the card definition's activation cost, rather than the attached ability's cost or its target availability (`Card::HasActivateAbility`, `0x7cb36b8`; `Card::CanActivate`, `0x7cb9a24`). Reference rules retain the previous checks.
  * A pre-draw interceptor can run with a full hand before the draw's destination check, and an empty deck does not trigger it (`Player::DrawCard`, `0x7de7abc`; `TimeTheaterBehavior::ReplaceDraw`, `0x7d8d750`). Reference rules retain the previous full-hand and empty-deck ordering.
  * Cards that ignore play restrictions can use up to four physical slots at Space Throne, rather than its normal one-slot limit (`Card::IsPlayableByCard`, `0x7cb0a28`; `Location::GetNumberOfEmptyCardSlots`, `0x7ddb4d8`). Reference rules retain the previous capacity check.
  * Each ongoing additive cost update clamps to zero before subsequent cost effects (`Card::AddCost`, `0x7cb69e4` and `0x7cb6a2c`); reference rules retain unbounded intermediate values.
  * Location cost effects emit updates only for their client-selected targets, preserving bounded Add versus Set behavior (`TheSandbarBehavior::IncreaseCardCost`, `0x7d8c410`); reference rules retain the previous numerical calculation.
  * Horde grows every owned Zombie Horde (`ZombieHordeAbilityDef::GrowZombieHorde`), not just the first; it changes one spread game. A new Horde is placed before it grows, so the growth triggers in play.
  * Sinister London copies a staged Skill after it resolves even though the Skill has already been banished (`SinisterLondonBehavior::CreateCopy`, `0x7d7fe8c`). Reference rules retain the earlier lane-based behavior.
  * Wolfsbane counts every other card its owner has at its location, face-down ones included (`WolfsbaneBehavior::GetTargets` over `Location::GetAllCards`); the reference counted revealed cards only. Reference rules: `cards/reference_rules.rs`.
  * Yondu banishes a randomly chosen lowest-current-cost card from the opponent's deck (`YonduBehavior::BanishLowestCostInDeck`, `0x7cf8068`); the reference drops it without recording a zone transition. Reference rules: `cards/reference_rules.rs`.
  * Zenn-La recruits the first card in deck order with printed cost 3 and reveals it (`ZennLaBehavior::RecruitThreeCostCard`, predicate `0x7d9c414`); the reference checks effective cost and skips On Reveal. Reference rules: `cards/reference_rules.rs`.
  * Shuri's Lab doubles a card played there at PostCardResolved, even if it moved away; cards played elsewhere and moved into the Lab are not doubled (`ShurisLabBehavior::DoublePower`). Reference rules retain the original current-location behavior.
  * A card whose text was removed no longer reacts to events.
  * Start-of-turn, game-start, draw and Empower events also reach on-event hooks of cards in hand and deck.
  * `Game::top_card` is the literal top card (`Deck::GetTopCard`), not the next card `draw` would take.
  * A resolving card is marked revealed and Ongoing effects are refreshed before its On Reveal and the play observers run (`Card::ResolveForPlay`).
  * Plans may move and activate cards; the parity gates plan with plays only.

`parity/gates.sh` runs every gate in order and stops at the first failure (inputs: the recorder outputs listed in its header).

The gate reads oracle files from `$SNAPSIM_ORACLES`.
To regenerate the historical six-run corpus (the recording directory has grown since
the original run), execute from this repository:

```sh
ROOT="$PWD"
O="$SNAPSIM_ORACLES"
RUNS="$SNAPSIM_RUNS"
SNAPS="$SNAPSIM_SNAPSHOTS"
FIXTURES="$SNAPSIM_FIXTURES"
PY="$SNAPSIM_PYTHON"
export PYTHONPATH="$SNAPSIM_REFERENCE"
export SNAPSIM_REFERENCE_POOL=1
mkdir -p "$O/snapsim-full" "$O/snapsim-api" "$O/reference-runs"
for f in "$RUNS"/20261118*.jsonl "$RUNS"/20261119*.jsonl \
         "$RUNS"/20261118*.deck.json "$RUNS"/20261119*.deck.json; do
  [ ! -e "$f" ] || ln -sf "$f" "$O/reference-runs/$(basename "$f")"
done
mkdir -p "$HOME/.cache/snapsim-work"; cd "$(mktemp -d "$HOME/.cache/snapsim-work/oracle.XXXXXXXX")"
for policy in spread margin; do
  "$PY" "$ROOT/parity/record.py" "$ROOT/parity/corpus-full-$policy.json" \
    --games 1000 --seed 20261201 > "$O/snapsim-full/$policy.jsonl"
done
"$PY" "$ROOT/parity/position_record.py" --games 1 --seeds 20 \
  --recordings "$O/reference-runs" > "$O/snapsim-api/position.jsonl"
"$PY" "$ROOT/parity/record_advisor.py" "$O/snapsim-api/advisor.jsonl" \
  --runs "$O/reference-runs"
"$PY" "$ROOT/parity/validation_record.py" "$O/snapsim-validation.jsonl" \
  --corpus "$O/snapsim-api/position.jsonl" --corpus "$O/snapsim-api/advisor.jsonl" \
  --runs "$O/reference-runs"
"$PY" "$ROOT/parity/snap_record.py" "$O/reference-runs"/*.jsonl "$SNAPS"/*.jsonl \
  "$FIXTURES/match-1.jsonl" "$FIXTURES/match-2.jsonl" > "$O/snapsim-snap-oracle.jsonl"
```

The advisor corpus has 898 records, of which the gate compares 834 and identifies
64 as new content. The SNAP corpus has 1,047 frames (190 positions, 857 errors).

| Gate | Command (oracle → Rust check) | Result on the integrated build |
|---|---|---|
| Full games | `parity/record.py` → `snapsim-parity`, per policy | 1,000 margin games identical; 999 spread games identical and 1 known divergence |
| Position building | `parity/position_record.py` → `snapsim-position-parity` | 2,400 position–seed pairs |
| Advisor | `parity/record_advisor.py` → `snapsim-advisor-parity` | 834 records, 0 mismatches; 64 new-content records |
| Coverage, priors, strict advice | `parity/coverage_parity.py snapsim-coverage-parity POSITIONS ADVISOR` | 2,603 positions, whose 9 edge positions also match approximate advice; 704 new-content positions |
| Position validation | `parity/validation_record.py` → `snapsim-validation-parity` | 4,118 accepted and 2,932 rejected alike; 337 out-of-range rejected (see above) |
| SNAP adapter and watches | `parity/snap_record.py RECORDINGS` → `snapsim-snap-parity`; `parity/snap_binding.py MODULE ORACLE` | 1,047 recorded frames (190 positions, 857 errors), core and Python module |
| Deck codes and deck files | `SNAPSIM_BIN=snapsim parity/compare_deckcodes.py snapsim-deckcodes-parity` | 31/31 cases; 9 new-content cases |
| Compare and benchmark harness | `parity/compare_harness.py snapsim-compare-parity` | 100 Recruit deal pairs, 9 harness configurations |
| Deck builder | `SNAPSIM_PY_PATH=MODULE parity/deckbuild_compare.py`; again with `SNAPSIM_COMPLETE_ONLY=1` | 12 cancellation cut-offs and 1 complete run identical |
| Command line | `SNAPSIM_BIN=snapsim parity/cli_compare.py`, `cli_deckbuild.py`, `cli_snap.py` | 7 + 1 + 2 cases |
| Replay checker | `parity/replay_compare.py --python PY --reference REF --rust snapsim-replaycheck RECORDINGS` | 10 recordings byte-identical, one at a time and together |
| Client rules on real matches | `snapsim-replaycheck RECORDINGS` without the reference variables | private frozen corpus (60 Snapper-Assist recordings and 4 SNAP snapshots from 2026-11-23, not distributed; checksums kept with it) at `cab9770713`: 249 consistent (3 order-enumerated), 0 inconsistent, 99 not checkable turns; at `a70e5cc681` it was 243 / 0 / 105, and every changed turn is attributed to a newly implemented card or a named missing-state reason |

The Python suite of the frozen checkout (360 tests) also passes against this module.

Refresh data and regenerate RNG fixtures:

```sh
python3 parity/refresh_data.py "$SNAPSIM_REFERENCE"
"$SNAPSIM_PYTHON" parity/generate_rng_fixtures.py
cargo test --test random
```

## Registry

Compare the registry with the reference worktree's implemented rules:

```sh
cargo build --release --bin snapsim-inventory
ROOT="$PWD"
WORK="${WORK:-$HOME/.cache/snapsim-work}"; mkdir -p "$WORK/bench"; cd "$WORK/bench"
PYTHONPATH="$SNAPSIM_REFERENCE" \
  "$SNAPSIM_PYTHON" "$ROOT/parity/inventory.py"
```

It lists every card and location the reference implemented and confirms none is missing here; everything else in the registry comes from the client (current totals under "Coverage" above). Set `CARGO_TARGET_DIR` if the inventory binary was built elsewhere.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT), at your option. Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

## Disclaimer

SnapSim is an unofficial fan project. It is not affiliated with, endorsed by or sponsored by Marvel, Second Dinner or Nuverse. MARVEL SNAP and all related names, card names, card text, artwork and game data are trademarks and property of their respective owners. The card and location catalogs in `data/` (card text gathered from marvelsnapzone.com, and rule data derived from the game client) are included for interoperability and research; the licenses above cover SnapSim's own code and documentation, not that third-party material. Client method names and addresses in comments and docs are citations of evidence, not game code; the repository contains no game binaries, dumps or decompiled source.
