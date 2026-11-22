# snapsim

The MARVEL SNAP simulator behind Snapper-Assist: rules for every implemented card (158 deck cards and 34 tokens, skills and spells) and location (84), the rollout advisor, the SNAP observation adapter, the deck builder and the replay checker, with the `snapsim` command line and the `snapsim_py` Python module. It began as an exact port of the Python simulator (`Marvel-snap-simulator`), whose 84 deck cards, 10 tokens and 84 locations it still matches on the gates below; the cards added since come from the game client's own rule definitions, and every card in the nine SnapComplete decks of November 2026 (`decks/snapcomplete-2026-11.txt`) is supported. Position validation also rejects integers too large for their Rust fields, which Python accepted. See "Python reference (frozen)" below.

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
  Collapsed Mine turns with visible Rocks are not checkable because recordings do not fully expose moves and activations needed to determine whether a player skipped. Vormir's per-player consumed triggers are likewise not present in live positions; coverage marks Vormir and Collapsed Mine state approximate.
  Time Theater's per-player draw-use state and Quantum Realm's historical base-power assignments may be absent from observations. When the missing state can affect a turn, replay checking marks it not checkable and strict advice reports the missing state; approximate advice remains available.
  The Abbey and The Raft have a shared one-shot location trigger whose consumed state is not exported. Previously revealed copies therefore decline strict advice and replay checking rather than assuming it is unused.
  Nebula's installed pre-end trigger and The Thing (First Steps)'s one-time fill-trigger consumption are not exported by the readers. Earlier revealed copies therefore have uncertain state in isolated positions; their Power alone does not establish those flags.
  Card augment attachments are not exported; a changed augment-sourced stat ledger makes the replay turn not checkable. A visible augment ledger also makes strict advice report uncertain card augment state.
  For an opponent's revealed play, replay checking uses recorded stat-ledger updates proven to predate the turn to reconstruct hidden pre-play Power. When a relevant update's timing cannot be established, the turn is not checkable rather than guessing that Power.
  For Cave of the Dragon, the checker uses uniquely observed created-card identities and, when visible in separate frames, their reveal order; it still consumes the normal random draws. Frozen-reference checks do not use this evidence.

## Architecture

`snapsim` uses arena indices for cards and board lanes. `Game` owns the turn flow, player zones, priority, event dispatch and Ongoing recomputation; `policy` contains the heuristic, spread and margin planners. The game borrows a `GameRng`, so hooks can use `game.rng.borrow_mut()` and consume the same stream as the Python reference. `PythonRandom` implements CPython's integer seeding and sampling; its fixtures come from CPython 3.12.16. `data` exposes typed copies of the reference catalogs.

`cards::registry()` (one descriptor slice per card-group module, listed in `cards::GROUPS`) and `locations::REGISTRY` contain rule descriptors. Card modules provide callbacks; see `cards/sword_master.rs` for a random effect and `cards/representative.rs` for reveal, ongoing, play and end-turn examples. Location modules provide callbacks; see `locations/turns.rs` for an end-turn rule. Each descriptor can provide initial per-instance state, and hooks can access the game's RNG. `Event` also exposes turn, reveal, play, addition, movement, destruction, discard, location reveal, energy and turn-count events. Hooks must preserve the Python event and RNG call order.

The engine exposes location replacement, delayed reveal scheduling, location cost hooks, ability removal, On Reveal and Ongoing repetition, movement, off-board placement, hand swaps and Horde creation. Lasting card costs are stored on cards. `Game::card_cost` mirrors Python's `energy_cost`: revealed locations modify the cost of hand cards only, in lane order, while board and deck cards keep their lasting cost. `Card::has_ongoing`, `can_reduce_power` and `reduce_power` support ability and immunity checks. `Game::copy_card` clones `CardState`, resets temporary board state and invokes the descriptor's optional `clear_reveal_state` hook. `cards::deck_eligible_ids()` matches the reference's sorted `deck_eligible_ids()`, which Agent 13 draws from. Registry membership alone is not a parity proof.

Copied text, merging and attached abilities follow the client. Copying text (`Card::CopyText`) loads the copied definition with fresh state and repeats its On Reveal under reveal-doubling effects; Nico Minoru's prepared spell is her current text, and her preparation trigger is tracked separately. Abilities one card attaches to another (Hawkeye's and Jessica Jones's end-of-turn buff, Web Sling's move-power trigger) are instances on that card with their turn, location and origin: an instance installed by the card's own text is replaced with that text, while one cloned from another card stays. A merge (Symbiote Spider-Man) adds the absorbed card's power from modifiers, copies its text and clones its attached abilities except those the receiver already has by default (`CloneExternalAbilities`).

## Adding content

1. Add a module under `crates/snapsim/src/cards/` or `locations/`, define a descriptor with the card or location ID and the necessary hooks, and list it in that module's descriptor slice (cards; add a new module's `CARDS` and `TOKENS` slices to `cards::GROUPS` and `cards::TOKEN_GROUPS`) or in `locations::REGISTRY`. Tokens, which decks may not contain, go in the module's `TOKENS`. Use the catalog's current printed stats; do not invent stand-in rules.
2. Take behavior from the Android client (59.15.1), not from card text: the card's ability definitions and the methods they run, including which candidates a random choice draws from and how many rolls it makes (using `game.rng`). Cite the client method and address in a comment. Never commit client binaries, dumps or decompiled code.
3. Add focused boundary tests. For a random effect, test seeds and the order of random calls. New rules have no Python counterpart: check them against real matches that contain the card or location with `snapsim-replaycheck`.
4. Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` and `parity/gates.sh`. Gates for unchanged content must still pass. If the needed hook or state transition is unavailable, extend the core before claiming the rule is supported.

Known approximations of client rules:

* Random choices use the client's candidate set and number and order of rolls, but draw from `game.rng`; the client's own random stream is not reproduced.
* Effects that create random cards or locations draw only from implemented ones; the client draws from its full pools. Scarlet Witch Queen of Chaos's own pool is complete (13 curated and 11 collectible skills), so she matches the client's skill choice.
* Nico Minoru's location spell and Omniversal Presence pick locations uniformly, while the client weights the pick (`GetRandomLocationDefId`). Merging of power from the same source, the order of conditional Ongoing effects that depend on each other, and gaps left by cards that cannot shift are not modelled.
* Visual-only abilities the client attaches (Tao Mandala, Zombie Galacti) are not modelled. `ReplacedCardDefId` follows the client's copy and clear paths, but no implemented card replaces another, so it is always empty.
* Live positions use the client's `LatestResolutionOrder` when the reader supplies it, and otherwise derive the order from played turn, priority and slot (see "Live state from SNAP observations"). None of the newly exported live fields has been checked against a live match yet.

## Advisor and Python binding

`snapsim::advise::advise_json(position_json, options_json)` takes a position in the `Position` JSON shape (validated by `Position::from_json`). Options are `rollouts`, `seed`, `max_plans`, `time_limit` (seconds or null), `approximate`, `playout` (`spread`, `heuristic` or `margin`), `workers` and `plays_only`. It computes coverage, sampling priors and status itself; strict searches return unsupported positions without running rollouts. Results keep `"engine": "simulator-rollout"`. Each plan lists its steps in `plays` in staging order. With `plays_only` a step is `{hand_index, card, location}`, as in the reference. Otherwise every step has a `kind`: `play` (with `hand_index`), or `move` and `activate`, which name the board card by `from` (its location) and `board_index` (its index in that location's `mine` list); `location` is a move's destination.

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

Registering new cards changes `cards::deck_eligible_ids()`, which Agent 13 draws from and position sampling uses. Gates therefore pin it to `parity/reference-pool.json`, the frozen reference's 84 deck cards: the parity binaries call `cards::use_reference_pool()` themselves, and the CLI, `snapsim-replaycheck` and the Python module do the same under `SNAPSIM_REFERENCE_POOL=1`. Export that variable for every Python-side gate script.

The same pin restricts random location pools to `parity/reference-locations.json`, the reference's 84 locations, so locations added since do not change its draws. Gate scripts treat cards and locations beyond the reference as new content: coverage must still list every reference entry, and recordings containing new content are excluded from the Python replay comparison (the client replay gate still checks them).

Two kinds of record are reported rather than compared:

* New content: a position, case or recording that contains a card with rules here but not in the reference (`cards::beyond_reference`). The reference played such cards as unsupported stand-ins.
* Deliberate changes to existing rules, made to match the 59.15.1 client. A game that diverges because of an engine change is listed with its exact first-divergence path and values in `parity/divergences.json`; `snapsim-parity` fails on any other divergence and on a listed one that no longer occurs. A changed card keeps the reference's version in `cards/reference_rules.rs`, which the gates switch to with `cards::use_reference_rules()` or `SNAPSIM_REFERENCE_RULES=1`; live play never does. The `replay-client` gate then checks the client rules, these changes included, against every real recording. The changes:
  * Horde grows every owned Zombie Horde (`ZombieHordeAbilityDef::GrowZombieHorde`), not just the first; it changes one spread game. A new Horde is placed before it grows, so the growth triggers in play.
  * Wolfsbane counts every other card its owner has at its location, face-down ones included (`WolfsbaneBehavior::GetTargets` over `Location::GetAllCards`); the reference counted revealed cards only. Reference rules: `cards/reference_rules.rs`.
  * Zenn-La recruits the first card in deck order with printed cost 3 and reveals it (`ZennLaBehavior::RecruitThreeCostCard`, predicate `0x7d9c414`); the reference checks effective cost and skips On Reveal. Reference rules: `cards/reference_rules.rs`.
  * A card whose text was removed no longer reacts to events.
  * Start-of-turn, game-start, draw and Empower events also reach on-event hooks of cards in hand and deck.
  * `Game::top_card` is the literal top card (`Deck::GetTopCard`), not the next card `draw` would take.
  * A resolving card is marked revealed and Ongoing effects are refreshed before its On Reveal and the play observers run (`Card::ResolveForPlay`).
  * Plans may move and activate cards; the parity gates plan with plays only.

`parity/gates.sh` runs every gate in order and stops at the first failure (inputs: the recorder outputs listed in its header).

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
| Client rules on real matches | `snapsim-replaycheck RECORDINGS` without the reference variables | all 49 Snapper-Assist recordings plus the SNAP snapshots (2026-11-22, 229 cards and 120 locations): 185 consistent, 0 inconsistent, 95 not checkable turns |

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

As of this commit it reports 192 cards (the reference's 94 plus 98 from the client) and 84 locations implemented, none of the reference's remaining. Set `CARGO_TARGET_DIR` if the inventory binary was built elsewhere.
