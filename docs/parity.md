## Parity and checks

`Position::from_json` validates the Python position format, with one deliberate
difference: integers must also fit the destination Rust field (`u8` turns,
`i32` power and energy, and `usize` counts). Python integers are unbounded.
The validation parity gate compares in-range cases to Python and requires
out-of-range cases to be rejected separately. Generate the two JSONL inputs
using the [position](live-state.md#position-materialization-parity) and [advisor](advisor.md) recipes, then run from an empty working directory:

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
  * Daredevil's turn-five response validates and stages after the opponent's committed actions (`DaredevilBehavior::ForceOpponentToStageFirst`, `0x7e87de0`), so changes such as moved cards affect play restrictions; reference rules retain simultaneous independent staging.
  * Scarlet Witch replaces her location with one drawn rarity-weighted (common 200, uncommon 125, rare 75, ultra rare 15; `LocationRarityDefList::.cctor`, `0x7ac3c60`) from the pool, excluding only that location's own definition (`ScarletWitchBehavior::MorphLocation`, `0x7f2ff08`; `Game::GetRandomLocationDefId`, `0x7db248c`); reference rules draw uniformly from the pool minus every location in play. Reality Stone, Omniversal Presence and Nico's fifth spell use the same client draw (not in the reference).
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
  * Ironheart filters revealed allies to cards with a power attribute, shuffles them in resolution order and buffs the first two (`IronheartBehavior::BuffCards`, `0x7ece770`, predicate `0x7eceee0`, `Game::Shuffle` `0x4b12f30`); reference rules sample two indices as CPython's `sample` does without that filter.
  * Stark Island, Sakaar, Zenn-La, Monster Island, Shadowland, Savage Land, Central Park and Ebonshire process players in initiative order (`GetOrderedPlayersForResolution`, `0x7daa454`); Central Park also creates all of one player's Squirrels before the other's. Reference rules keep seat order and Central Park's lane-first ordering.
  * Korg shuffles the Rock into the whole opponent deck (`KorgBehavior::ShuffleRocks`, `0x7ee1d1c`, `Deck::Shuffle` `0x7cc0ac0`); reference rules insert it at one random position.
  * Mantis and White Queen roll their random pick before the full-hand check, so a full hand still consumes the roll (`MantisBehavior::CopyCard`, `0x7ef1ee8`; `WhiteQueenBehavior::CreateACopy`, `0x7cf1064`); reference rules return early.
  * Wolverine's regeneration and White Tiger's Tiger roll `GetRandomLocationToSpawnCard` (`0x7db2b64`) with the fallback to all other locations (`WolverineBehavior::SpawnAtRandomLocation`, `0x7cf4c78`; `WhiteTigerBehavior::CreateToken`, `0x7cf1df4`); reference rules never roll when no location can take the card.
  * Zombie Blade steals from a character in the opponent's hand only (`ZombieBladeBehavior::StealToHorde`, `0x7cf9fdc`, predicate `0x7cfa878`); reference rules pick from the whole hand.
  * Stark Industries makes eight independent picks with replacement over its revealed cards and gives each picked card +1 Power per pick, in card order (`StarkIndustriesBehavior::BuffCards`, `0x7d82ee8`, `Game::Roll` `0x7dab62c`); the reference gives every card `8 / n` and draws the remainder without replacement. Reference rules retain that split.
  * A card whose text was removed no longer reacts to events.
  * Start-of-turn, game-start, draw and Empower events also reach on-event hooks of cards in hand and deck.
  * `Game::top_card` is the literal top card (`Deck::GetTopCard`), not the next card `draw` would take.
  * A resolving card is marked revealed and Ongoing effects are refreshed before its On Reveal and the play observers run (`Card::ResolveForPlay`, `0x7cb9d90`, calls `SetRevealed` at `0x7cba010`). This includes location spawns via `ResolveForSpawn` (`0x7cbaae8`): e.g. a resolving Carnage receives its destroyed Nova's buff. Reference rules retain the earlier post-On-Reveal flip.
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

