# snapsim

Rust port of the Python reference simulator. **Coverage is restricted:** the fixed corpus below has differential evidence, not the full Recruit pool. The rollout advisor and complete rules reference are not yet ported. Unknown registered content is rejected rather than treated as a vanilla card or location.

## Architecture

`snapsim` uses arena indices for cards and board lanes. `Game` owns the turn flow, player zones, priority, event dispatch and Ongoing recomputation; `policy` contains the heuristic, spread and margin planners. The game borrows a `GameRng`, so hooks can use `game.rng.borrow_mut()` and consume the same stream as the Python reference. `PythonRandom` implements CPython's integer seeding and sampling; its fixtures come from CPython 3.12.16. `data` exposes typed copies of the reference catalogs.

`cards::REGISTRY` and `locations::REGISTRY` contain rule descriptors. Card modules provide callbacks; see `cards/sword_master.rs` for a random effect and `cards/representative.rs` for reveal, ongoing, play and end-turn examples. Location modules provide callbacks; see `locations/turns.rs` for an end-turn rule. Each descriptor can provide initial per-instance state, and hooks can access the game's RNG. `Event` also exposes turn, reveal, play, addition, movement, destruction, discard, location reveal, energy and turn-count events. Hooks must preserve the Python event and RNG call order.

This interface does **not** yet establish that every remaining reference rule can be ported without core changes: e.g. delayed reveals, replacement locations, cost modifiers, ability removal and arbitrary ongoing/reveal repetition are not implemented. Tokens other than Drone have no generator in the fixed corpus; Kraven's movement hook has a Rust unit test but no reference-game movement parity yet. The current deck constructor also does not enforce the reference's non-ownable-token restriction. Do not interpret registry membership as a complete per-rule parity proof.

## Adding content

1. Add a module under `crates/snapsim/src/cards/` or `locations/`, define a descriptor with the Python card/location ID and the necessary hooks, and register it in the corresponding `REGISTRY`. Use the catalog's current printed stats; do not invent stand-in rules.
2. Add focused boundary tests and a differential Python parity case for the rule. For a random effect, test seeds and the order of random calls. Add only proven content to the fixed corpus.
3. Run the full parity gate below for both policies, `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`. If the needed hook or state transition is unavailable, extend the core before claiming the rule is supported.

## Parity and checks

The current gate matched 1,000 seeded full games each for spread and margin, using the 25 deck cards and 11 locations in the corpus, including the random Sword Master and Muir Island's turn-end hook. This compares initial decks and locations, every turn's plans, reveal priority and lane powers, and the final winner. It is not full-Recruit parity.

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
      "$ROOT/parity/record.py" "$ROOT/parity/corpus-$policy.json" \
      --games 1000 --seed 20261119 > "/tmp/snapsim-$policy.jsonl")
  "$ROOT/target/release/snapsim-parity" "$ROOT/parity/corpus-$policy.json" \
    "/tmp/snapsim-$policy.jsonl"
done
```

`snapsim-parity` independently samples decks from each recorded seed, replays the game and reports the first differing JSON field. For benchmarks and measured representative-corpus ratios, see `BENCHMARKS.md`; full-Recruit and advisor-throughput comparisons are not measured.

Refresh data and regenerate RNG fixtures:

```sh
python3 parity/refresh_data.py /path/to/Marvel-snap-simulator-advice
$SNAPSIM_PYTHON parity/generate_rng_fixtures.py
cargo test --test random
```

## Registry and remaining reference content

The following snapshot compares the registry against the reference worktree's implemented rules, not against the entire data catalog. Regenerate it after changes:

```sh
cargo build --release --bin snapsim-inventory
ROOT="$PWD"
cd /tmp/snapsim-bench
PYTHONPATH=$SNAPSIM_REFERENCE \
  $SNAPSIM_PYTHON "$ROOT/parity/inventory.py"
```

**Registered cards (35):** Abomination, Angela, Bishop, Blade, BlueMarvel, Carnage, Cosmo, Cyclops, Demon, Drone, Hawkeye, Hulk, IronMan, JessicaJones, KaZar, Klaw, Kraven, Medusa, MistyKnight, Monster, MrFantastic, Ninja, Quicksilver, Raptor, Rock, Sentinel, Shocker, SinisterClone, Squirrel, SwordMaster, TheThing, TigerSpirit, Ultron, Wasp, ZombieHorde.

**Remaining reference cards (59):** Agent13, AmericaChavez, Angel, AntMan, Apocalypse, Armor, BlackPanther, Cable, CaptainAmerica, Colossus, Deathlok, DevilDinosaur, DoctorStrange, Domino, Elektra, Enchantress, Forge, Gamora, Groot, Heimdall, Hood, Hulkbuster, Iceman, IronFist, Ironheart, JoaquinTorres, Korg, LadySif, Lizard, Magik, Mantis, MoonGirl, Morph, MrSinister, MultipleMan, Namor, Nightcrawler, Nova, Odin, Onslaught, ProfessorX, Punisher, RocketRaccoon, ScarletWitch, Spectrum, SpiderWoman, SquirrelGirl, Starlord, StrongGuy, TheCollector, Uatu, WhiteQueen, WhiteTiger, Wolfsbane, Wolverine, Yellowjacket, Yondu, ZombieBlade, ZombieGiantMan.

**Registered locations (11):** KamarTaj, MuirIsland, Necrosha, NegativeZone, Nidavellir, ProjectPegasus, Ruins, SewerSystem, TinkerersWorkshop, Wakanda, Xandar.

**Remaining reference locations (73):** Asgard, Atlantis, AvengersCompound, BarSinister, BaxterBuilding, CastleBlackstone, CentralPark, CloningVats, CrimsonCosmos, DailyBugle, DangerRoom, DreamDimension, Ebonshire, Elysium, EternityRange, FiskTower, Flooded, Flooding, FogwellsGym, FrontierOutpost, Genosha, GrandCentral, HellfireClub, HellsKitchen, KunLun, Kyln, LakeHellas, Lemuria, Limbo, LosDiablos, Machineworld, Madripoor, Mindscape, MiniaturizedLab, MirrorDimension, Mojoworld, MonsterIsland, MonsterMetropolis, MoshPit, Murderworld, NovaRoma, Olympia, OnslaughtsCitadel, OrchisForge, OscorpTower, OttosLab, Proscenium, QuantumTunnel, ReedsLab, RocketPad, Sakaar, SavageLand, Shadowland, ShurisLab, SinisterLondon, StarkIndustries, StarkIsland, StarkTower, StrangeAcademy, SurvivorsCamp, TheBigHouse, TheDeadlands, TheIceBox, TheSandbar, Titan, UndeadMetropolis, Vault, WakandanEmbassy, WakandanThroneRoom, WashingtonDC, Westview, WhitePalace, ZennLa.
