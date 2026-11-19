# snapsim

This repository is an incomplete Rust port of the Python simulator. The copied catalogs are reference data, not a registry of implemented rules. No cards, locations, game engine, rollout advisor, Rust parity runner, or Rust benchmark are implemented yet; consequently there is no full-game parity or Rust speed measurement.

The `snapsim` library currently provides typed catalog records and `GameRng`, including a CPython-compatible integer-seeded `PythonRandom`. The latter supports state snapshots and CPython's random, getrandbits (up to 128 bits), randbelow, randrange, randint, choice, shuffle, and sample-index operations. `seed_words` accepts arbitrarily wide integer seeds. Rules should consume `GameRng` rather than depending on a particular generator.

Refresh the catalogs from the read-only reference worktree:

```sh
python3 parity/refresh_data.py /path/to/Marvel-snap-simulator-advice
```

Regenerate and check the RNG fixtures (generated with CPython 3.12.16):

```sh
$SNAPSIM_PYTHON parity/generate_rng_fixtures.py
export PATH="$HOME/.cargo/bin:$PATH" CARGO_TARGET_DIR="$PWD/target"
cargo test
```

`parity/record.py` emits Python game traces from a JSON config containing `cards`, `locations`, and `policy` (`spread` or `margin`). Supply only implemented content when using it for parity. The Rust replay and comparison side is not yet implemented. See `BENCHMARKS.md` for the provisional Python baseline and reproducible command.
