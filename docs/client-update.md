# Updating SnapSim for a new MARVEL SNAP client

This is a procedure for bringing the simulator's rules, catalog and checks to a new Android client build. The current rules were derived from **59.15.1**; every RVA in this repository refers to that build's `libil2cpp.so`. Everything under "Procedure" is version-independent. Everything under "59.15.1 examples" is a finding about that one build: it is an example of what to look for and how it was recorded, not a rule that can be assumed for the next build.

Nothing here is a runtime observation of the client unless it says so. The audits are static (instruction reading) plus replay checks against recorded matches. Statements that the evidence does not establish are marked **[INFERENCE]**.

Do not commit client binaries, metadata files, class dumps, decompiled code, or recordings. Method names and RVAs in comments and docs are citations of evidence, not game code. Recordings and generated reports may contain private match data and stay local.

## Contents

1. [Overview](#overview)
2. [Step 1: capture the client](#step-1-capture-the-client)
3. [Step 2: diff the old and new builds](#step-2-diff-the-old-and-new-builds)
4. [Step 3: regenerate derived files](#step-3-regenerate-derived-files)
5. [Step 4: refresh the catalog](#step-4-refresh-the-catalog)
6. [Step 5: re-verify behavior](#step-5-re-verify-behavior)
7. [Step 6: replay check, parity gates and fuzzing](#step-6-replay-check-parity-gates-and-fuzzing)
8. [Release checklist](#release-checklist)
9. [59.15.1 examples](#59151-examples)

## Overview

| Step | Input | Output |
|---|---|---|
| 1. Capture | APK splits of the new build | `libil2cpp.so`, `global-metadata.dat`, and the derived `rules/` corpus (`metadata.json`, `defs.json`, `carddefs.json`, `pools.json`) plus a C#-style class dump |
| 2. Diff | old and new `rules/metadata.json` | list of renamed, removed or re-signed methods; RVA map for every RVA cited here |
| 3. Regenerate | the new corpus | `data/client_cards.json`, `data/past_years_pool.{json,rs}`, `cards/no_text.rs`, `cards/ongoing_priorities.rs`, `move_priority.rs` |
| 4. Catalog | the public website catalog and the client definitions | `data/cards.json`, `data/locations.json` |
| 5. Re-verify | cited RVAs and per-area hook inventories | audit ledgers, fixes with tests, parity divergence lines |
| 6. Check | local recordings | `snapsim-replaycheck`, `parity/gates.sh`, fuzzing |

Order matters: derived files (step 3) and the catalog (step 4) change which cards and locations exist, so do them before auditing behavior (step 5). Run the gates (step 6) after every batch of fixes, not only at the end.

## Step 1: capture the client

The tooling that produced the 59.15.1 corpus lives outside this repository; this repository's scripts only consume its outputs. For 59.15.1 it was:

| Output | Tool (59.15.1) |
|---|---|
| `rules/metadata.json` (method names, signatures, RVAs) | Il2CppInspectorRedux 2026.2, run statically over `libil2cpp.so` + `global-metadata.dat` |
| `rules/defs.json`, `carddefs.json`, `pools.json` (runtime definitions) | a read-only Frida script built on [frida-il2cpp-bridge](https://github.com/vfsfitvnm/frida-il2cpp-bridge) 0.14.0 with Frida 17.26.4, attached to the running client on a menu screen; it uses static definition getters and reflection only |
| C#-style class dump (`dump/<package>/<version>/…`) | captured from the running client the same day; the producing tool and command were not recorded (the installed frida-il2cpp-bridge 0.14.0 declarations expose no dump API), so record them next time |
| pseudocode | Ghidra 12.1.4 headless, used as navigation only |

frida-il2cpp-bridge reads the runtime's own type and definition tables; it does not decompile native code. Behaviour evidence comes from `objdump` of `libil2cpp.so` (see the evidence rules). After a game update, verify the pinned bridge against the new client before trusting a capture: confirm it attaches and that the captured definitions and fields agree with the new `metadata.json`. Use whatever equivalent tooling you have, but produce the same files.

1. Obtain the base APK and the `arm64-v8a` split from the owned device. The two artifacts are `lib/arm64-v8a/libil2cpp.so` and `assets/bin/Data/Managed/Metadata/global-metadata.dat`. Record the SHA-256 of both, the ELF GNU build ID, the IL2CPP metadata version, and the application version.
2. Generate `rules/metadata.json` with an IL2CPP metadata inspector run over both files. The scripts and this guide rely on the `addressMap.methodDefinitions[]` records, each with `virtualAddress`, `name`, `signature`, `dotNetSignature` and `group` (assembly, namespace and class). For 59.15.1 the `virtualAddress` of a method equals the RVA used in `objdump` addresses throughout this repository (for example `Card::IsMovableByPlayer` is `0x7cb0cac` in both). Confirm that equality again on the new build before trusting any lookup (**[INFERENCE]** that it holds across builds).
3. Capture the runtime definition lists read-only, from a device on a menu screen: `rules/defs.json` (the behavior map, `CubeGame.CardBehaviorDefList.DefMap`: abilities, trigger types and delegate targets with method and RVA), `rules/carddefs.json` (`CubeDef.CardDefList.Defs`: name, cost, power, type, attributes, `EnabledDate`, `ExcludedFromInGameRandomPool`), and `rules/pools.json` (Scarlet Witch Queen of Chaos's static inclusion and exclusion lists). Capture static definition getters only; do not call getters that fill caches or change state.
4. Generate a C#-style class dump of the client. Two scripts read `CubeDef.cs` from it for the static `CardDef`/`LocationDef` ID field offsets.
5. Import `libil2cpp.so` into a decompiler **without** auto-analysis, and treat its output as navigation only (see the evidence rules below). Convert RVAs to the decompiler's addresses using the image base; for 59.15.1 the base was `0x100000`.

The scripts expect one client directory (`CLIENT_DIR` below) laid out as:

```text
CLIENT_DIR/libil2cpp.so
CLIENT_DIR/global-metadata.dat
CLIENT_DIR/rules/{metadata,defs,carddefs,pools}.json
CLIENT_DIR/dump/<package>/<version>/SecondDinner/CubeDef.cs
```

## Step 2: diff the old and new builds

Every behavior rule in the code cites a method and RVA. After an update RVAs shift even when behavior does not, so a stale RVA in a comment is not evidence. Do this before touching behavior.

Run the review checklist before making changes:

```sh
python3 parity/client_diff.py OLD_CLIENT_DIR NEW_CLIENT_DIR --objdump objdump
python3 parity/client_diff.py OLD_CLIENT_DIR NEW_CLIENT_DIR --json > client-diff.json
```

`--objdump` (or the `OBJDUMP` environment variable) selects an ARM64-capable objdump. The tool collects cited method-start RVAs with `git grep`, resolves each old method by exact group, name and signature, and flags missing or ambiguous new methods. For mapped methods it compares instruction streams bounded by the next metadata method start, masking relocation-sensitive addresses. It compares card and behavior definitions by ID, ignoring delegate RVAs but not their method names or other fields. When both captures contain `rules/locationdefs.json` it compares those definitions too; the standard 59.15.1 capture does not contain a location definition list, so locations require a separate manual audit. If only one side has that file, the missing capture requires review.

Every finding starts with verdict `review` and follows the audit ledger fields below; the tool exits 1 when findings need review, 2 on an input failure, and 0 when none are detected. An unchanged normalized stream is **not** proof of unchanged behavior: relocations and literal contents are not fully checked, and method boundaries are inferred. Check constants, callees and indirect calls manually. This checklist does not regenerate files; run the generators in step 3 after reviewing their version-specific assumptions.

## Step 3: regenerate derived files

Run each generator against the new corpus, then review the diff. The scripts hard-code some RVAs and the version in a path, so each needs a manual check (last column) before its output can be trusted.

| Output | Command | Hard-coded for 59.15.1, update first |
|---|---|---|
| `data/client_cards.json` | `python3 parity/generate_client_cards.py CLIENT_DIR/rules/carddefs.json` | none |
| `data/past_years_pool.json`, `data/past_years_pool.rs` | `python3 parity/generate_past_years_pool.py CLIENT_DIR START_UTC` (`START_UTC` is an ISO-8601 time with an explicit UTC offset) | the `59.15.1` path component of the `CubeDef.cs` dump; the instruction range `0x7f1f508`-`0x7f21ed8` of `RamaTutBehavior::GetPastYearsCards` (`0x7f1f418`); the resolve predicate at `0x7f22580` named in its docstring |
| `crates/snapsim/src/cards/no_text.rs` | `python3 parity/generate_no_text.py CLIENT_DIR/rules > crates/snapsim/src/cards/no_text.rs` | the RVAs in the generated header comment (`Card::HasNoText`, `GetCardBehaviorDef`) |
| `crates/snapsim/src/cards/ongoing_priorities.rs` | `python3 parity/generate_ongoing_priorities.py CLIENT_DIR/rules > crates/snapsim/src/cards/ongoing_priorities.rs` (destination taken from the generated file's header; **[INFERENCE]** that this is the intended redirect) | the priority names mapped in the script (`Default`, `Init`, `ConditionalCard`, `ConditionalLocation`, `PostConditional`); re-read `AbilitySystem::UpdateOngoingStats` and its ordering passes as cited in [card-technicalities.md](card-technicalities.md) |
| `crates/snapsim/src/move_priority.rs` | `python3 parity/generate_move_priority.py CLIENT_DIR/libil2cpp.so CLIENT_DIR/dump/<package>/<version>/SecondDinner/CubeDef.cs > crates/snapsim/src/move_priority.rs` | the objdump range of `MoveSystem::.cctor` (`0x7c9deb0`-`0x7c9e4d8`), the sub-ranges for the card (`0x7c9e020`-`0x7c9e30c`) and location (`0x7c9e380`-`0x7c9e3b0`) tables, the dictionary-add callee `0x6413d4c`, and the expected entry counts (27 cards, 2 locations) |

`generate_move_priority.py` fails with an error instead of guessing when an entry cannot be resolved or the counts differ. A failure there means the initializer changed shape: read the new initializer, do not edit the counts to make it pass.

`parity/reference-pool.json` and the CPython fixtures (`parity/generate_rng_fixtures.py`) belong to the frozen Python reference and do not change with the client.

## Step 4: refresh the catalog

`data/cards.json` and `data/locations.json` are the public-website catalog (names, printed stats, printed text, `status`, `source`, rarity). A scraper outside this repository produces them. `data/client_cards.json` supplements them with client-only definitions; website entries win, except that `factions` and `faction_cards` come from the client.

1. Refresh the website catalog with the scraper, keep the file shape (`carddefid`, `name`, `cost`, `power`, `ability`, `status`, `source`, `type`, `random_pool_excluded` for cards; `carddefid`, `name`, `ability`, `status`, `rarity`, `difficulty` for locations), then regenerate `data/client_cards.json` (step 3).
2. Compare the catalog with the client definitions in `carddefs.json`: IDs only on one side, changed cost or power, changed `ExcludedFromInGameRandomPool`.
3. **Check `status` against reality.** The website labels are a lagging signal. In 59.15.1 data, Jean Grey School was listed `unreleased` and unregistered while a recorded live ranked match contained it, and Crimson Citadel was also `unreleased`. Both were corrected to `released` and registered with client-derived rules. For every entry still marked `unreleased`, ask whether it appears in a recording, in the reader's catalog of the installed client, or in a live game; if so treat the label as wrong, and verify the behavior (step 5) before changing it.
4. Know what `status` does here. `released` drives coverage counts (`snapsim coverage` reports released characters and locations that are registered) and the fuzzing pool. Random-card pools are built from registration, not from `status`: `cards::deck_eligible_ids()` is every registered non-token card. Registering an unreleased card therefore also puts it into simulated random pools. Register unreleased content deliberately.
5. Run `snapsim coverage` and update the coverage paragraph in `README.md` (registered card and location counts, released counts) and the affected rows in [coverage-audit.md](coverage-audit.md). The audit's own text says its summary counts describe an earlier baseline; update its "current status" section, not the historical tables.
6. For each new card or location follow [Adding content](architecture.md#adding-content): a module, a descriptor in the module's slice, behavior taken from the client method with RVA comments, focused boundary tests including random-call order for random effects. Never invent a stand-in rule.

## Step 5: re-verify behavior

### Evidence rules

These are the rules the existing audits ([activate.md](activate.md), [move.md](move.md)) were written under.

1. **`objdump` of `libil2cpp.so` is authoritative.** Decompiler output is a map. Use an ARM64-capable `objdump -d --no-show-raw-insn --start-address=START --stop-address=END`.
2. **Decompilers follow tail branches into unrelated bodies.** A function's end is only inferred from the next dumped RVA, and an unconditional `b` out of the range makes the decompiler merge another method into the output. `Game::GatherOptions` (`0x7da8e90`) produced a very large decompilation with merged control flow, and the option-generation path there was only partly established. So for every handler first confirm entry control flow in the disassembly: read from the metadata RVA to the next dumped RVA, check that the direct calls agree with the rule you are citing, and check that no unconditional tail branch leaves the range. Say exactly this when you record a match. It confirms entry control flow, not every indirect call or predicate.
3. **Indirect calls stay open.** An interface or virtual dispatch (`blr x8`) that cannot be resolved from the disassembly makes the verdict **unverified** with the address named. Examples in the 59.15.1 audit: `0x7c956a8` (inside `AbilitySystem::ActivateTriggerAbility`) and `0x7cbc388` (inside `Card::Merge`).
4. **Delegates and predicates are separate methods.** A `Where`/`Any` predicate is a compiler-generated `<...>b__N_M` method with its own RVA; read it. `defs.json` records delegate targets with method and RVA, which is the shortest way to find them.
5. **Constants come from the instructions or the constructor.** Turn numbers, thresholds and amounts live in `.cctor` and constructor bodies or in static data (`adrp`/`ldr` from rodata). Read them there; do not copy them from card text. When a value was only read as an instruction operand and not confirmed in metadata, say so (Stark Industries' trigger turn was read from disassembly and not confirmed in metadata).
6. **Cite RVAs in code comments** next to the rule, as `Class::Method 0xRVA`, one comment per rule, on the code that implements it. Mark inferences with `[INFERENCE]` in the same comment.
7. **Static is not runtime.** A static match means "consistent with the instructions I read". Confirmation that behavior held in play comes only from the replay check against recordings (step 6), and "not checkable" is not "correct".

### The audit ledger format

Run one audit per area, each as a ledger with one entry per simulator hook or rule. The area list for 59.15.1 was: play, move and activate legality (published as [activate.md](activate.md) and [move.md](move.md), with the same row shape), and random effects for cards and locations (**in progress**: the per-area ledgers for it are still being written and fixes from it are landing; this guide records the format, not those results).

To build the inventory, search the code for the area's call sites (for example every hook that uses `game.rng`, every `can_move_card`/`can_play_internal` use, every activate handler) and list each as an entry. A hook-site inventory over-reports (test functions, helpers and no-RNG hooks appear in it), so every entry gets a verdict.

Each entry records:

| Field | Content |
|---|---|
| Entry | card or location ID, or helper name, and the simulator function that implements it |
| Client method(s) | `Class::Method` with RVAs, including the predicates and helpers on the path |
| Evidence | what was read: the instruction range, the calls that matched, the callees and constants checked |
| Verdict | one of the values below |
| Reason | for `fixed`: what changed and the test name; for `unverified`: the exact reason, such as an unresolved `blr`, an undecoded generic body or a constant not found |

Verdict values:

* **match**: the instruction ranges were read, calls agree with the simulator, no tail branch leaves the range. State the scope ("rolls once, over revealed allies, in `GetAllRevealedCards` order").
* **fixed**: the simulator differed from the client and was changed. Name the regression test and whether the old behavior is retained under `reference_rules_enabled()`.
* **unverified**: comparison could not be completed. Always give the reason. An honest `unverified` is a result; do not convert it to `match` because the code "looks right".
* **n/a** (no RNG or no relevant client behavior; false positive of the inventory) and **test** (a test function, not a rule): keep them so entry counts reconcile.

Report counts per ledger and state their scope ("these rows only, not every interaction"). The activate audit does this: "12 full-match, 6 fixed, 2 unverified" for its first 20 definitions. Raw dumps and working ledgers stay in a local work folder; only the published audit files under `docs/` are committed.

### Fixing a divergence

1. Write the client rule, with the RVA comment.
2. If the rule exists in the frozen Python reference, keep the reference behavior reachable behind `cards::reference_rules_enabled()` (a process-global switch: `cards::use_reference_rules()` or `SNAPSIM_REFERENCE_RULES=1`), either in `cards/reference_rules.rs` or inside the function. The parity gates run with it on so unchanged content stays exact, and live play never sets it. Rules for content that has no reference counterpart do not need a reference branch.
3. Add a divergence line to [Python reference (frozen)](parity.md#python-reference-frozen): what the client does, the method and RVA, and what the reference did. If a seeded full-game parity run now differs, list the game in `parity/divergences.json` with its first-divergence path and a reason; `snapsim-parity` fails on any unlisted divergence and on a listed one that no longer occurs.
4. Add a test that fails on the old behavior. For RNG changes, clone the generator before the call and assert that exactly the client's draws were consumed (count and range), and that the result indexes the client's candidate order, not lane order. Cover the edge branches the audit found (empty pool, pool no larger than the count, full hand, fallback roll). Tests cannot exercise reference mode because the switch is process-global; the parity gates cover it.
5. Re-run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, then the gates.

### Systems with their own checks

* **Stakes.** `Game::UpdateCubeValueNextTurn`, `CheckIncreaseStakesForNextTurn`, the end-of-game stake behavior and the concession checks are cited in [architecture](architecture.md#architecture) with RVAs; re-read them for formula changes. Mephisto's payout has no client rule found, so stake advice declines when he may be in play; recheck whether the new build adds one.
* **Provenance and hidden legality.** `hidden_legality.rs` classifies cards and locations whose hooks might read hidden state. When the reader or recordings change (new cards in decks, new exported fields), review new visible-hook cards and keep drift tests with each review. The reader-side update is documented in that project's own update guide; SnapSim only consumes its exports.
* **Ongoing ordering.** The pass order and conditional-retry behavior in [card-technicalities.md](card-technicalities.md) cite `AbilitySystem::UpdateOngoingStats` and its helpers. Re-read them because the generated priority table feeds that order.

## Step 6: replay check, parity gates and fuzzing

### Replay check

```sh
cargo build --release -p snapsim --bin snapsim-replaycheck
target/release/snapsim-replaycheck RECORDING.jsonl [...]
target/release/snapsim-replaycheck --coverage-json report.json RECORDING.jsonl [...]
```

Each turn is **consistent**, **inconsistent** or **not checkable**. Not checkable is unproven, not correct. When a recorded turn contains play and move entries on the same history line their relative order is unknown, so the checker enumerates the orders: every combination of tied (move, play) pairs, with a cap of 3 ties (2^3 = 8 orders). A turn is consistent if any order matches, and the detail says "matched 1 of N orders (mask ...)". Above the cap a turn stays not checkable and the message names the tie count. With the new build, rerun on every recording and investigate each inconsistent turn by reading the closest trial before touching any rule; the enumeration found a real mismatch in Stark Industries on 59.15.1 that a fixed order had hidden.

`parity/gates.sh` runs the replay check twice: against recordings that contain no new content (compared with the frozen reference, so rules unchanged by an update must still match it), and as `replay-client` against every recording with the reference pool and reference rules switched off. The `replay-client` gate allows **no** inconsistent turn.

### Parity gates

Set `SNAPSIM_REFERENCE`, `SNAPSIM_PYTHON`, `SNAPSIM_RUNS`, `SNAPSIM_SNAPSHOTS` and `SNAPSIM_ORACLES` as in [Environment](interfaces.md#environment), then:

```sh
export PATH="$HOME/.cargo/bin:$PATH"
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
parity/gates.sh        # must print ALL GATES PASSED
```

`gates.sh` builds into its own target directory, checks all full-game, position, advisor, coverage, validation, SNAP-adapter, deck-code, deck-builder and CLI parity, then the two replay gates. It stops at the first failure. Judge it by exit status and the final line, not through a pipeline. It reports the replay result as consistent, inconsistent and not checkable counts.

The gates prove that unchanged content still behaves as the frozen reference did and that the client rules are consistent with the recordings. They do not prove that a changed rule is correct; step 5 does. Do not claim parity beyond implemented coverage: copy effects at some locations do not preserve original card state, and parity there is unproven (see [parity](parity.md)).

### Fuzzing

Parity gates and recordings exercise a few thousand real situations. A new build changes interactions between copy, reveal and random effects, which finds hangs and illegal states that no recording contains. The harness approach used on 59.15.1:

* **Decks.** Take `cards::deck_eligible_ids()` (via `snapsim_py.deck_eligible_ids()`), **filtered to released catalog entries**, so unreleased registered cards do not enter. Build random 12-card decks, and in about half of them force three cards from a hand-picked list of copy/reveal-heavy cards (Rogue Scion of Division, Morph, Moonstone, Super-Skrull, Odin, Wave, Mysterio, Arnim Zola, Mister Sinister, Hood, Daken, Copycat, Kang, Galacta, High Evolutionary). Update the list from the new build's copy-capable cards.
* **Games.** Play many seeded games through the vectorised step environment (`snapsim_py.VecEnv`) with random legal actions drawn from the mask, a fixed step cap (2000 in the 59.15.1 harness), and one seed per game derived from the batch seed. Treat three outcomes as findings: an action the mask offered but the engine rejected; a game that does not terminate within the cap; a panic or crash.
* **Isolation.** Run each batch in its own child process with a **hard timeout** (kill the child, do not wait). A hang in one game must not stall the run, and a timed-out batch is itself a finding: rerun its games one per child with the same seeds to find the one that hangs. The batch prints its decks on the first line so a failure can be replayed exactly.
* **Triage.** A finding is reproduced from `(batch seed, game index)`, minimized to a position, fixed to client behavior (not by special-casing), and kept as a regression test. A real example: a reveal chain from Rogue Scion of Division expanded exponentially under copy effects and hung advice; the fix is under the Rogue Scion item below.

Keep fuzz scripts and outputs in a local work folder. They are not part of the repository.

## Release checklist

1. Client artifacts captured; SHA-256, build ID, metadata version and app version recorded.
2. Step 2 diff done; every cited RVA re-resolved or re-audited; stale RVA comments updated.
3. Derived files regenerated (step 3) with hard-coded ranges updated; diffs reviewed.
4. Catalog refreshed (step 4); `status` of anything seen in recordings or the live client checked; README coverage line and coverage audit updated.
5. Per-area audit ledgers written for the areas that changed, each entry `match`/`fixed`/`unverified` with reason; unverified rows listed in the docs, not hidden.
6. Every fix has a client-RVA comment, a test that fails on the old behavior, `reference_rules_enabled()` retention when the reference covers it, and a parity divergence line.
7. `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` clean.
8. `parity/gates.sh` prints ALL GATES PASSED, with zero inconsistent turns in `replay-client`.
9. Fuzzing run on the new build with no open findings, or each open finding written down.
10. `snapsim_py` rebuilt with `crates/snapsim-py/build-module.sh` and the `snapsim`/`snapsim-replaycheck` binaries rebuilt, into a separate target directory if a consumer has the module loaded.
11. Docs updated: README (coverage paragraph and version mentions), parity documentation (divergence lines), this guide where the procedure changed, audits. The version string in generator docstrings and generated-file headers now names the new build.
12. The repository is public: scan the docs and README for personal paths (user names, absolute home or project directories) and for private recording file names; none may appear.
13. Commit after the combined checks pass, naming the checks and the replay counts.

## 59.15.1 examples

Each item cites the client method and RVA it was read from. They are findings for 59.15.1 only. The random-effects audit that applies these across card and location code is still **in progress**; rows below are the client conventions it works from, not a list of what is fixed.

### Random draws and selection

* `Game::Roll` (`0x7dab62c`) is the integer draw: `Roll(min, max)`. `Roll(0, n)` selects one of `n` items; Stark Industries indexes an `int[n]` array with it, so `max` is exclusive. Percent rolls are integers too: Danger Room (`0x7d4d318`) rolls `Roll(0, 100)` and destroys the card when the result is below 25.
* `Game::Shuffle<T>` (`0x4b12f30`) and `Deck::Shuffle` (`0x7cc0ac0`) are a **forward** Fisher-Yates: for each index `i` from the start, swap with `Roll(i, n)`. A backward pass has the same distribution but a different draw order, which changes seeded outcomes and replay matching; any simulator loop of the form `for i in (1..n).rev()` over a shuffle is therefore an audit candidate.
* `RandomSelector::Select` (`0x7d28478`) returns the whole pool **without rolling** when the pool has at most `count` items; otherwise it rolls `Roll(0, remaining)` and removes the pick, per pick. By contrast `Zone::GetRandomCard` (`0x7e06e70`) tail-calls `Game::GetRandomCard` (`0x7dad84c`), which rolls even for a one-card pool and rolls only if the list is non-empty. Check which helper a card uses; they differ on a one-item pool.
* `Game::GetRandomLowestPowerCard` (`0x7dad998`) and `GetRandomHighestPowerCard` (`0x7dae060`) order by power, take the tied prefix, then call `GetRandomCard`.
* `Game::GetRandomCardDefIds` (`0x7db1dd8`) rolls over `GetCardDefPool`; with unique picks it removes each pick. The client pool is the full client set, so the simulator's pool (registered cards) differs for unreleased entries. Rama-Tut uses a hard-coded whitelist of IDs in `GetPastYearsCards` (`0x7f1f418`); the step 3 generator reads the whitelist from that method's instructions and filters with `EnabledDate`, `ExcludedFromInGameRandomPool` and a resolve-trigger predicate, but how the client then draws from it (replacement, order) was not decoded. Kang also rolls `GetRandomVariantExcludingDefault` (`0x7ed9478`) per pick for an art variant; the simulator has no equivalent, so only the card choice matches.
* **Random locations.** `Game::GetRandomLocationToSpawnCard` (`0x7db2b64`) and `GetRandomLocationToMoveCard` (`0x7db2f50`) roll over the legal locations other than the excluded one. When none is legal and the call's `fallbackToAny` flag is set, they roll over **every** other location (the move variant's filter is at `0x7dcae40`, its fallback at `0x7dcaefc`). With no candidates and no fallback they return null without rolling. Whether the flag is set is per caller (Scarlet Spider's `CreateCopy`, `0x7f2f4c4`, enables it), so read each call site. Callers must still check that the card can be added at the result.
* `Game::GetRandomLocationDefId` (`0x7db248c`) builds a weighted table (`GetRandomLocationTable` `0x7db1edc`) from `LocationRarityDefList::.cctor` (`0x7ac3c60`), which stores weights 200, 125, 75 and 15 for rarity IDs 1 to 4 (constants read from rodata); `FindWeight` is `0x7c67e94`. The mapping of IDs 1 to 4 to common, uncommon, rare and ultra rare is **[INFERENCE]** (monotonic with the four names). The pool excludes only the morphing location's own definition and the game's starting-location definitions (the final `Except` at `0x7db2484`), not every location on the board. The cumulative order inside the generic `WeightedTable` (`0x60708f0`, `0x6070938`) was not decoded, so the distribution is exact but the roll-to-location mapping is not. `GameConfigDef.LocationRarityOverrides` (`0x75123d8`) is server data and is not modelled.
* `Game::DiscardCard` (`0x7dabe68`) checks the card is in the Hand zone before anything else and returns false otherwise, for example for a card an earlier discard trigger already moved.
* Player order in hooks that visit both players is `Game::GetOrderedPlayersForResolution` (`0x7daa454`); the initiative holder (`PlayerWithInitiative`, set at `0x7dcfbc8`) determines who goes first in some effects. That the initiative holder is visited first is **[INFERENCE]**; hooks that loop over seats 0 and 1 do not model it and are listed as unverified in the location ledger. Kluh's `ReduceCostsForBothPlayers` (`0x7ee0950`) processes the owner, then the opponent.
* Stark Industries (`StarkIndustriesBehavior::BuffCards`, `0x7d82ee8`): eight independent `Game::Roll(0, n)` picks with replacement over `Location::GetAllRevealedCards` (`0x7ddea48`), each pick adding one to a per-card counter, then `Card::AddPowerPermanently` per card in list order. There is no even `8 / n` base share. A card can get zero or more than three. Whether `Roll(0, 0)` is guarded was not determined; the simulator keeps an early return.

### Ability depth and reentrancy

* `Ability::CanActivate` (`0x7c8a6e0`) caps the global active-ability stack at 32. Player and location triggers count too. `CheckReentrant` (`0x7ca61e8`) blocks re-entry of a non-reentrant instance, not nesting of distinct instances. The simulator shares one `ability_call_depth` across card frames and generic trigger frames and stops at 32.
* Rogue Scion of Division (`StealTextFromOpponentHand`, `0x7f2b228`) reactivates even identical text, so a reveal chain could expand exponentially (this hung advice on a recorded position). The simulator cuts the cycle once every highest-cost target already has the same text removed; the client repeats that no-op until the depth limit, and the cutoff skips those redundant random draws, which can change later random outcomes. This is a known approximation, listed in [architecture](architecture.md#adding-content).

### Legality

* **Play.** `Card::IsPlayableByPlayer` (`0x7cb091c`) and `IsPlayableByCard` (`0x7cb0a28`); `Game::GatherOptions` (`0x7da8e90`) reports a hand card even when it has no legal destination, so the simulator's legal-action list is a subset of the client's option list. `Location::HasEmptyCardSlot` (`0x7ddb4c0`) calls `GetNumberOfEmptyCardSlots` (`0x7ddb4d8`), which uses the physical maximum of four slots when the ignore-play-restrictions flag is passed.
* **Move.** `Card::IsMovableByPlayer` (`0x7cb0cac`), `Location::CanMoveCard` (`0x7ddba50`), `MoveSystem::FindMovableByPlayerSource` (`0x7c9d9ec`): filter components to unrestricted or matching destinations, sort by ascending source priority then component creation time, choose the first, and consume only that source's stack. The priority table is built by `MoveSystem::.cctor` (`0x7c9deb0`) and read by `GetSourcePriority` (`0x7c9e67c`); the full table and per-grant verdicts are in [move.md](move.md). Priority 1: Cloak, Hellion, Vision, Sam Wilson, Phoenix Force, SP//dr, Miek. Priority 2: Madame Web, Web Sling, Space Shot, Cloak of Levitation Trinket. Priority 3: Nightcrawler, Jeff, Nocturne, Spider-Punk, Spider-Man 2099, Moveable Augment. Unknown keys sort last. Regenerate it with `parity/generate_move_priority.py` after an update and re-read both methods; equal-priority ties that depend on creation timestamps are reported as unknown by the simulator, not guessed.
* **Activate.** `Card::CanActivate` (`0x7cb9a24`), `HasActivateAbility` (`0x7cb36b8`, `includeHidden=false`), `StageForActivate` (`0x7cb9678`), `CanResolveForActivate` (`0x7cb9d4c`); the full audit with per-card verdicts is [activate.md](activate.md).

### Catalog status

Jean Grey School (turn-5 movement grant, `0x7d5b11c`, `0x7d5b888`) was in a live ranked match while the catalog said `unreleased`; it and Crimson Citadel were registered and marked released. Isle of Shhhhh (`0x7d5a3e8`) and Pool's School (`0x7d735a0`) were still unreleased and unregistered in the movement audit (check again with the new catalog).

### Known open items on 59.15.1

* Random-effects audit: in progress, see the ledger format above; Sersi's `TransformCard` per-target pool, and the initiative-order visiting of players in location hooks, were unverified when last recorded.
* `WeightedTable` roll-to-location mapping, `LocationRarityOverrides`, Kang's variant roll, and Rama-Tut's unrolled whitelist body (see above).
* Unresolved indirect calls named in the activate audit (`0x7c956a8`, `0x7cbc388`, and the modifier insertion calls at `0x7cb3f64` and `0x7cb3f80`).
