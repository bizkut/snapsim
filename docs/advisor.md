## Advisor and Python binding

`snapsim::advise::advise_json(position_json, options_json)` takes a position in the `Position` JSON shape (validated by `Position::from_json`). Options are `rollouts`, `seed`, `max_plans`, `time_limit` (seconds or null), `approximate`, `playout` (`spread`, `heuristic`, `margin` or experimental `smart_opponent`), `workers` and `plays_only`. It computes coverage, sampling priors and status itself; strict searches return unsupported positions without running rollouts. Results keep `"engine": "simulator-rollout"`. Each plan lists its steps in `plays` in staging order. With `plays_only` a step is `{hand_index, card, location}`, as in the reference. Otherwise every step has a `kind`: `play` (with `hand_index`), or `move` and `activate`, which name the board card by `from` (its location) and `board_index` (its index in that location's `mine` list); `location` is a move's destination.

The optional `position_rollouts` (unsigned 32-bit, default 0) independently estimates the current position by sampling hidden information and letting the configured playout policy choose for both seats. When samples complete, `position` contains `win`, `draw`, `loss`, `samples`, and `approximate` (the latter reflects incomplete coverage). This estimates policy-vs-policy outcomes, not an optimal-play guarantee. Its random stream is separate from plan search; without a time limit enabling it does not change plan results. With `time_limit`, plan search receives 70% of the budget and position sampling runs afterward within the remaining shared deadline; fewer samples may complete and the position is omitted if none complete. Cancellation and strict unsupported coverage likewise can leave it absent. The same option passes through `snapsim_py.advise`.

An opt-in `opponent.weights` object in the position supplies a finite positive weight for every candidate card ID. Hidden cards are drawn without replacement with probability proportional to those weights; seen and known cards remain excluded or fixed as usual. Missing or invalid candidate weights are errors. Without this object the existing Recruit and uniform priors are unchanged.

`snapsim compare --a-opponent-prior prior.json` (or `--b-opponent-prior`) accepts a generic JSON object `{"base":{"CardId":1.0,...},"groups":[{"card_ids":["CardId",...],"weight":2.0}]}`. The base must include every candidate card with a positive finite weight. A group's positive weight is added once to each of its cards if any of its cards has been revealed by that opponent. Groups and base weights may come from any source; the file contains no implicit recording access. Omitting the option preserves the original compare behavior.

The opt-in `best_reply` option (`true`, or `{"plans": N, "turns": 1|2}`; default off, so advice is unchanged) replaces the playout policy's opponent on the final turn with a best reply. In every sampled hidden state the opponent stages the best of a bounded set of its own plans against the plan being evaluated (for the `position` estimate, against the policy's plan for us), ranking win over draw over loss, and the estimates average over samples; it is not a worst case across hidden states. The candidate set is the policy's own plan, which stays and wins ties, plus up to `plans - 1` (default 32) maximal play plans of the sampled hand (plans that cannot take another card), each preceded by the policy's moves and activations. When more maximal plans exist a uniform subset is evaluated, and enumeration stops after 20,000 search nodes; `best_reply` in the result reports samples, evaluated plans, how often either cap applied, how often the reply changed the sample's result and rejected candidates. `turns: 2` also covers the turn before the last, with the policy playing the final turn after the reply. The opponent knows our plan and its own sampled hand but not more, which makes it stronger than a real opponent, and each sample costs up to `plans` playouts, so fewer samples fit in `time_limit`. Scripted seats (autoplay, Daredevil's ordered staging) fall back to the ordinary playout, and the stake scenarios of `stake_rollouts` keep the ordinary opponent. The option passes through `snapsim_py.advise`, `snapsim position --best-reply [--best-reply-plans N --best-reply-turns 1|2]` and `snapsim compare --a-best-reply/--b-best-reply`.

`optional_moves` defaults to false. When enabled, best reply also enumerates bounded combinations of keeping each movable card in place or moving it to a legal destination before enumerating maximal plays, with at most 64 move prefixes and a 20,000-node play budget per prefix. The policy used by both plan and position rollouts stages its plays before considering optional moves, and retains only moves that improve its lane-margin score. This is a heuristic, not optimal movement; in particular, it does not consider a move that would make a currently illegal play possible. `plays_only` still disables moves and activations.

The experimental `playout: "smart_opponent"` is opt-in. In the final two turns
it chooses between spread and the optional-move margin heuristic with equal
probability for the sampled opponent; our own playout remains spread. Earlier
turns use spread for both seats. This is a cheap heuristic mixture, not a
one-ply search or a guarantee of finding a winning reply. Held-out evaluation
did not establish better board or selected-plan calibration; it is not
recommended for live use.

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

