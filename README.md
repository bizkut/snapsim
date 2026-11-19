# snapsim

The MARVEL SNAP simulator behind Snapper-Assist: rules for every implemented card (84 deck cards and 10 tokens) and location (84), the rollout advisor, the SNAP observation adapter, the deck builder and the replay checker, with the `snapsim` command line and the `snapsim_py` Python module. It began as an exact port of the Python simulator (`Marvel-snap-simulator`). Before Python was retired, every part matched it on the gates below; the one deliberate difference is that position validation rejects integers too large for their Rust fields, which Python accepted. See "Python reference (frozen)" below.

## Command line

Build with `cargo build --release -p snapsim-cli`; run `target/release/snapsim --help` or add `--help` to any subcommand for the complete argument list.

* `deckbuild --collection FILE` searches owned cards. Optional inputs are `--decks FILE`, `--selected FILE`, or `--base-deck FILE_OR_IDS --try-cards IDS`. Search controls are `--count`, `--time-budget`, `--workers`, `--seed`, and `--confirm-rollouts`. `--progress` emits JSON progress and result lines, `--output FILE` saves the result, and Ctrl-C returns the best available result with status `cancelled`.
* `position [--file FILE]` reads a position (stdin by default). `--snap` accepts a SNAP observation; supply its starting deck with `--snap-deck FILE` or `--deck-code CODE`. Search controls are `--mode recruit`, `--rollouts`, `--seed`, `--time-limit`, `--max-plans`, and `--approximate`.
* `coverage` prints supported card and location IDs.
* `compare --mode recruit` or `compare --deck-file FILE` compares advisors in paired seats. Options include `--hero-deck`, `--deck-prior`, `--games`, `--seed`, `--workers`, `--rollouts`, `--time-limit`, and the per-side `--a-rollouts`, `--a-playout`, `--a-prior` and corresponding `--b-*` flags.
* `benchmark --mode recruit` or `benchmark --deck-file FILE` compares controllers. It accepts `--games`, `--seed`, `--rollouts`, and `--opponent`.
* The game-log replay checker is the separate binary `snapsim-replaycheck FILE...` (`cargo build --release -p snapsim --bin snapsim-replaycheck`). It replays recorded turns and reports each as consistent, inconsistent or not checkable; use it to validate rule changes against real matches.

## Architecture

`snapsim` uses arena indices for cards and board lanes. `Game` owns the turn flow, player zones, priority, event dispatch and Ongoing recomputation; `policy` contains the heuristic, spread and margin planners. The game borrows a `GameRng`, so hooks can use `game.rng.borrow_mut()` and consume the same stream as the Python reference. `PythonRandom` implements CPython's integer seeding and sampling; its fixtures come from CPython 3.12.16. `data` exposes typed copies of the reference catalogs.

`cards::registry()` (one descriptor slice per card-group module, listed in `cards::GROUPS`) and `locations::REGISTRY` contain rule descriptors. Card modules provide callbacks; see `cards/sword_master.rs` for a random effect and `cards/representative.rs` for reveal, ongoing, play and end-turn examples. Location modules provide callbacks; see `locations/turns.rs` for an end-turn rule. Each descriptor can provide initial per-instance state, and hooks can access the game's RNG. `Event` also exposes turn, reveal, play, addition, movement, destruction, discard, location reveal, energy and turn-count events. Hooks must preserve the Python event and RNG call order.

The engine exposes location replacement, delayed reveal scheduling, location cost hooks, ability removal, On Reveal and Ongoing repetition, movement, off-board placement, hand swaps and Horde creation. Lasting card costs are stored on cards. `Game::card_cost` mirrors Python's `energy_cost`: revealed locations modify the cost of hand cards only, in lane order, while board and deck cards keep their lasting cost. `Card::has_ongoing`, `can_reduce_power` and `reduce_power` support ability and immunity checks. `Game::copy_card` clones `CardState`, resets temporary board state and invokes the descriptor's optional `clear_reveal_state` hook. `cards::deck_eligible_ids()` matches the reference's sorted `deck_eligible_ids()`, which Agent 13 draws from. Registry membership alone is not a parity proof.

## Adding content

1. Add a module under `crates/snapsim/src/cards/` or `locations/`, define a descriptor with the card or location ID and the necessary hooks, and list it in that module's descriptor slice (cards; add a new module's slice to `cards::GROUPS`) or in `locations::REGISTRY`. Tokens also go in `cards::NON_OWNABLE`. Use the catalog's current printed stats; do not invent stand-in rules.
2. Add focused boundary tests. For a random effect, test seeds and the order of random calls. New rules have no Python counterpart: check them against real matches that contain the card or location with `snapsim-replaycheck`.
3. Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` and the Python parity gates below. Gates for unchanged content must still pass. If the needed hook or state transition is unavailable, extend the core before claiming the rule is supported.

## Advisor and Python binding

`snapsim::advise::advise_json(position_json, options_json)` takes a position in the `Position` JSON shape (validated by `Position::from_json`). Options are `rollouts`, `seed`, `max_plans`, `time_limit` (seconds or null), `approximate`, `playout` (`spread`, `heuristic` or `margin`) and `workers`. It computes coverage, sampling priors and status itself; strict searches return unsupported positions without running rollouts. Results keep `"engine": "simulator-rollout"`.

The `snapsim_py` module exposes `advise(position_json, options_json, cancel=None)`, `position_from_snap(observation_json, options_json)` (raising `snapsim_py.NotPlanningError` for frames that are not a planning point), `coverage`, `supported`, `deck_eligible_ids`, `recruit_card_pool`, `recruit_card_weights` and `deckbuild(args_json, progress=None, cancel=None)`. Values are JSON strings. It is built for the stable Python ABI (abi3, Python 3.9 or newer), so one build loads in any of those interpreters. Build it without installing into any virtual environment:

```sh
export PATH="$HOME/.cargo/bin:$PATH" CARGO_BUILD_JOBS=6
crates/snapsim-py/build-module.sh
```

It writes `snapsim_py.so` next to the release build (`target/release/`, or `$CARGO_TARGET_DIR/release/`). Snapper-Assist loads `target/release/snapsim_py.so` from this repository by default, or the file or directory named by `SNAPSIM_PY_PATH`. Generate and compare advisor cases from real recordings and SearchAI games with:

```sh
export PATH="$HOME/.cargo/bin:$PATH" CARGO_BUILD_JOBS=6
ROOT="$PWD"
cd /tmp
PYTHONPATH=$SNAPSIM_REFERENCE \
  $SNAPSIM_PYTHON "$ROOT/parity/record_advisor.py" \
  /tmp/snapsim-advisor.jsonl --runs $SNAPSIM_RUNS
cargo run --manifest-path "$ROOT/Cargo.toml" --release --bin snapsim-advisor-parity -- /tmp/snapsim-advisor.jsonl
```

The generator includes approximate positions, preserves watcher state across recording frames, and writes separate records for spread and margin. The gate matched 449 positions per policy (360 seeded SearchAI decisions and 89 real recorded positions), with 24 rollouts per position and one worker. The Python parity test also compares one and four workers.

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
PYTHONPATH=$SNAPSIM_REFERENCE \
  $SNAPSIM_PYTHON \
  parity/position_record.py --games 1 --seeds 20 \
  --recordings $SNAPSIM_RUNS \
  > /tmp/snapsim-position-cases.jsonl
cargo run --release -p snapsim --bin snapsim-position-parity -- \
  /tmp/snapsim-position-cases.jsonl
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
using the position and advisor recipes above, then run from a temporary cwd:

```sh
ROOT="$PWD"
cd "$(mktemp -d)"
PYTHONPATH=$SNAPSIM_REFERENCE \
  $SNAPSIM_PYTHON \
  "$ROOT/parity/validation_record.py" /tmp/snapsim-validation.jsonl \
  --corpus /tmp/snapsim-position-cases.jsonl --corpus /tmp/snapsim-advisor.jsonl \
  --runs $SNAPSIM_RUNS
cargo run --manifest-path "$ROOT/Cargo.toml" --bin snapsim-validation-parity -- \
  /tmp/snapsim-validation.jsonl
```

The gate matches 1,000 seeded full games each for spread and margin on `parity/corpus-full-*.json`: all 84 deck cards and all 84 locations, with decks drawn uniformly. It compares initial decks and locations, every turn's plans, reveal priority and lane powers, and the final winner. Merging the earlier per-group corpora exposed one cross-group bug: location cost modifiers were applied to board cards (Elektra at Elysium). `tests/costs.rs` covers it. `parity/corpus-{spread,margin}.json` is the original 25-card representative corpus that the first benchmark used.

```sh
export PATH="$HOME/.cargo/bin:$PATH" CARGO_TARGET_DIR="$PWD/target"
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release --bins
ROOT="$PWD"
mkdir -p /tmp/snapsim-bench
for policy in spread margin; do
  (cd /tmp/snapsim-bench &&
    PYTHONPATH=$SNAPSIM_REFERENCE \
    $SNAPSIM_PYTHON \
      "$ROOT/parity/record.py" "$ROOT/parity/corpus-full-$policy.json" \
      --games 1000 --seed 20261201 > "/tmp/snapsim-$policy.jsonl")
  "$ROOT/target/release/snapsim-parity" "$ROOT/parity/corpus-full-$policy.json" \
    "/tmp/snapsim-$policy.jsonl"
done
```

`snapsim-parity` independently samples decks from each recorded seed, replays the game and reports the first differing JSON field. Measured full-content and advisor-rollout throughput are in `BENCHMARKS.md`.

## Python reference (frozen)

The Python simulator (`Marvel-snap-simulator`, branch `stronger-advice` in the `Marvel-snap-simulator-advice` worktree) is retired and receives no rule changes. It remains the oracle for the gates below. They prove that unchanged content still behaves exactly as it did there, and are expected to differ only where a rule is deliberately changed or added here. The scripts need that checkout on `PYTHONPATH` and a Python with `loguru` (Snapper-Assist's `.venv`). Run them from a temporary directory. The table abbreviates paths: `parity/...` means `$ROOT/parity/...` and a bare binary means `$ROOT/target/release/<binary>`, where `ROOT` is this repository. Judge each gate by its exit status and success line, not through a pipeline that can hide a failure.

| Gate | Command (oracle → Rust check) | Result on the integrated build |
|---|---|---|
| Full games | `parity/record.py` → `snapsim-parity`, per policy | 1,000 spread and 1,000 margin games identical |
| Position building | `parity/position_record.py` → `snapsim-position-parity` | 2,400 position–seed pairs |
| Advisor | `parity/record_advisor.py` → `snapsim-advisor-parity` | 898 records (449 per playout), 0 mismatches |
| Coverage, priors, strict advice | `parity/coverage_parity.py snapsim-coverage-parity POSITIONS ADVISOR` | 3,307 positions, plus 9 approximate edge positions |
| Position validation | `parity/validation_record.py` → `snapsim-validation-parity` | 4,118 accepted and 2,932 rejected alike; 337 out-of-range rejected (see above) |
| SNAP adapter and watches | `parity/snap_record.py RECORDINGS` → `snapsim-snap-parity`; `parity/snap_binding.py MODULE ORACLE` | 1,047 recorded frames (190 positions, 857 errors), core and Python module |
| Deck codes and deck files | `parity/compare_deckcodes.py snapsim-deckcodes-parity` | 40/40 cases |
| Compare and benchmark harness | `parity/compare_harness.py snapsim-compare-parity` | 100 Recruit deal pairs, 9 harness configurations |
| Deck builder | `SNAPSIM_PY_PATH=MODULE parity/deckbuild_compare.py`; again with `SNAPSIM_COMPLETE_ONLY=1` | 12 cancellation cut-offs and 1 complete run identical |
| Command line | `SNAPSIM_BIN=snapsim parity/cli_compare.py`, `cli_deckbuild.py`, `cli_snap.py` | 7 + 1 + 2 cases |
| Replay checker | `parity/replay_compare.py --python PY --reference REF --rust snapsim-replaycheck RECORDINGS` | 12 recordings byte-identical, one at a time and together; the six Recruit matches give 26 consistent, 0 inconsistent, 5 not checkable |

The Python suite of the frozen checkout (360 tests) also passes against this module.

Refresh data and regenerate RNG fixtures:

```sh
python3 parity/refresh_data.py /path/to/Marvel-snap-simulator-advice
$SNAPSIM_PYTHON parity/generate_rng_fixtures.py
cargo test --test random
```

## Registry

Compare the registry with the reference worktree's implemented rules:

```sh
cargo build --release --bin snapsim-inventory
ROOT="$PWD"
cd /tmp/snapsim-bench
PYTHONPATH=$SNAPSIM_REFERENCE \
  $SNAPSIM_PYTHON "$ROOT/parity/inventory.py"
```

As of this commit it reports 94 cards and 84 locations implemented, none remaining.
