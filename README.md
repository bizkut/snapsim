# snapsim

Rust port of the Python reference simulator. Every card (84 deck cards and 10 tokens) and every location (84) the reference implements is registered. The full-content parity gate below matches the reference game for game.

## Architecture

`snapsim` uses arena indices for cards and board lanes. `Game` owns the turn flow, player zones, priority, event dispatch and Ongoing recomputation; `policy` contains the heuristic, spread and margin planners. The game borrows a `GameRng`, so hooks can use `game.rng.borrow_mut()` and consume the same stream as the Python reference. `PythonRandom` implements CPython's integer seeding and sampling; its fixtures come from CPython 3.12.16. `data` exposes typed copies of the reference catalogs.

`cards::REGISTRY` and `locations::REGISTRY` contain rule descriptors. Card modules provide callbacks; see `cards/sword_master.rs` for a random effect and `cards/representative.rs` for reveal, ongoing, play and end-turn examples. Location modules provide callbacks; see `locations/turns.rs` for an end-turn rule. Each descriptor can provide initial per-instance state, and hooks can access the game's RNG. `Event` also exposes turn, reveal, play, addition, movement, destruction, discard, location reveal, energy and turn-count events. Hooks must preserve the Python event and RNG call order.

The engine exposes location replacement, delayed reveal scheduling, location cost hooks, ability removal, On Reveal and Ongoing repetition, movement, off-board placement, hand swaps and Horde creation. Lasting card costs are stored on cards. `Game::card_cost` mirrors Python's `energy_cost`: revealed locations modify the cost of hand cards only, in lane order, while board and deck cards keep their lasting cost. `Card::has_ongoing`, `can_reduce_power` and `reduce_power` support ability and immunity checks. `Game::copy_card` clones `CardState`, resets temporary board state and invokes the descriptor's optional `clear_reveal_state` hook. `cards::deck_eligible_ids()` matches the reference's sorted `deck_eligible_ids()`, which Agent 13 draws from. Registry membership alone is not a parity proof.

## Adding content

1. Add a module under `crates/snapsim/src/cards/` or `locations/`, define a descriptor with the Python card/location ID and the necessary hooks, and register it in the corresponding `REGISTRY`. Use the catalog's current printed stats; do not invent stand-in rules.
2. Add focused boundary tests and a differential Python parity case for the rule. For a random effect, test seeds and the order of random calls. Add the card or location to `parity/corpus-full-{spread,margin}.json`.
3. Run the full parity gate below for both policies, `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`. If the needed hook or state transition is unavailable, extend the core before claiming the rule is supported.

## Advisor and Python binding

`snapsim::advise::advise_json(position_json, options_json)` accepts the JSON from Python `Position.to_dict()`. Options contain `rollouts`, `seed`, `max_plans`, `time_limit` (seconds or null), `playout` (`spread`, `heuristic`, or `margin`), and `workers`. The Python position layer must also supply `unsupported`, `priors`, and `status`: it remains the single source of truth for coverage and prior descriptions. Missing fields are errors. The Python layer blocks strict unsupported positions before calling Rust; approximate searches pass through. Results retain `"engine": "simulator-rollout"`.

Build the optional Python extension without installing into any virtual environment:

```sh
export PATH="$HOME/.cargo/bin:$PATH" CARGO_BUILD_JOBS=6
crates/snapsim-py/build-module.sh
```

It writes `target/release/snapsim_py.so`; the advice worktree loads that path by default, or the file or directory specified by `SNAPSIM_PY_PATH`. Generate and compare advisor cases from real recordings and SearchAI games with:

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
stand-ins; coverage decisions remain the Python position layer's responsibility.

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
