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
2. Take behavior from the Android client (59.15.1), not from card text: the card's ability definitions and the methods they run, including which candidates a random choice draws from and how many rolls it makes (using `game.rng`). Cite the client method and address in a comment. Never commit client binaries, dumps or decompiled code. For a new client build see [docs/client-update.md](client-update.md).
3. Add focused boundary tests. For a random effect, test seeds and the order of random calls. New rules have no Python counterpart: check them against real matches that contain the card or location with `snapsim-replaycheck`.
4. Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` and `parity/gates.sh`. Gates for unchanged content must still pass. If the needed hook or state transition is unavailable, extend the core before claiming the rule is supported.

Known approximations of client rules:

* Random choices use the client's candidate set and number and order of rolls, but draw from `game.rng`; the client's own random stream is not reproduced.
* Rogue Scion of Division's repeated self-copy is stopped once every highest-cost target already has the same text removed. The client repeats this no-op up to its 32-ability depth limit; the cutoff avoids exponential expansion but omits redundant random rolls, which can change subsequent random outcomes.
* Effects that create random cards or locations draw only from implemented ones; the client draws from its full pools. Every released card and location is implemented, so this only matters for unreleased entries in the client's pools. Scarlet Witch Queen of Chaos's own pool is complete (13 curated and 11 collectible skills), so she matches the client's skill choice. Rama-Tut's pool is the client's hard-coded 107-card list (`data/past_years_pool.rs`).
* Nico Minoru's location spell and Omniversal Presence pick locations uniformly, while the client weights the pick (`GetRandomLocationDefId`). Merging of power from the same source, the order of conditional Ongoing effects that depend on each other, and gaps left by cards that cannot shift are not modelled.
* Visual-only abilities the client attaches (Tao Mandala, Zombie Galacti) are not modelled. `ReplacedCardDefId` follows the client's copy and clear paths, but no implemented card replaces another, so it is always empty.
* Live positions use the client's `LatestResolutionOrder` when the reader supplies it, and otherwise derive the order from played turn, priority and slot (see [Live state from SNAP observations](live-state.md)). None of the newly exported live fields has been checked against a live match yet.

## Registry

Compare the registry with the reference worktree's implemented rules:

```sh
cargo build --release --bin snapsim-inventory
ROOT="$PWD"
WORK="${WORK:-$HOME/.cache/snapsim-work}"; mkdir -p "$WORK/bench"; cd "$WORK/bench"
PYTHONPATH="$SNAPSIM_REFERENCE" \
  "$SNAPSIM_PYTHON" "$ROOT/parity/inventory.py"
```

It lists every card and location the reference implemented and confirms none is missing here; everything else in the registry comes from the client (current totals in the [overview](interfaces.md#original-overview)). Set `CARGO_TARGET_DIR` if the inventory binary was built elsewhere.

