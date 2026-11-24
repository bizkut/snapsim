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

