# SnapSim

SnapSim is an unofficial MARVEL SNAP rules engine, rollout advisor, deck builder,
and replay checker. Its Rust library, CLI and Python binding implement behavior
from the Android 59.15.1 client, supported by cited client methods and recorded
matches. The original Python simulator is frozen and retained as a parity oracle.

The registry contains 696 card IDs and 187 location IDs (`snapsim coverage`, November 24, 2026),
including every released card and location with an ID in the November 23, 2026 catalog. Registration
is not a guarantee that every restored position is exact: strict advice declines
when required state is unavailable. See the [coverage audit](docs/coverage-audit.md)
and [parity notes](docs/parity.md) for evidence, limitations and divergences.

## Requirements

- Rust 1.98.1 (pinned in `rust-toolchain.toml`).
- Python 3.9 or later for the optional Python binding; NumPy for `VecEnv`.

## Build

```sh
cargo build --release -p snapsim-cli
cargo build --release -p snapsim --bin snapsim-replaycheck
```

## Quick start

```sh
# List supported content.
target/release/snapsim coverage

# Search for a deck using an owned collection.
target/release/snapsim deckbuild --collection collection.json

# Advise on a position read from a file.
target/release/snapsim position --file position.json

# Compare advisors in paired seats.
target/release/snapsim compare --mode recruit

# Check recorded turns against the simulator.
target/release/snapsim-replaycheck recording.jsonl
```

Run `target/release/snapsim --help` or add `--help` to a subcommand for options.
See the [command-line reference](docs/cli.md) for all commands and replay-checker
limitations.

## Python binding

Build the `snapsim_py` extension with `crates/snapsim-py/build-module.sh`.
It exposes advice, SNAP observation conversion, coverage, deck building and
vectorized environments. See [advisor and Python binding](docs/advisor.md) and
[interfaces](docs/interfaces.md) for the API and array formats.

## Documentation

- [Interfaces, trajectories and environment](docs/interfaces.md): Rust and Python environments, recording conversion and path variables.
- [Command-line reference](docs/cli.md): commands, options and replay-checker details.
- [Architecture and content](docs/architecture.md): engine internals, adding rules, approximations and registry.
- [Advisor and Python binding](docs/advisor.md): advice options, results and Python API.
- [Live state](docs/live-state.md): SNAP observation mapping and position materialization parity.
- [Parity and checks](docs/parity.md): validation gates and the frozen Python reference's divergences.
- [Card and engine technicalities](docs/card-technicalities.md): timing, mechanics and client evidence.
- [Coverage audit](docs/coverage-audit.md): per-card and per-location status and restored-position limits.
- [Client update guide](docs/client-update.md): updating rules and evidence for a new client build.
- [Activate audit](docs/activate.md): Activate legality and client evidence.
- [Move audit](docs/move.md): movement legality, ordering and client evidence.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT), at your option. Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

## Disclaimer

SnapSim is an unofficial fan project. It is not affiliated with, endorsed by or sponsored by Marvel, Second Dinner or Nuverse. MARVEL SNAP and all related names, card names, card text, artwork and game data are trademarks and property of their respective owners. The card and location catalogs in `data/` (card text gathered from marvelsnapzone.com, and rule data derived from the game client) are included for interoperability and research; the licenses above cover SnapSim's own code and documentation, not that third-party material. Client method names and addresses in comments and docs are citations of evidence, not game code; the repository contains no game binaries, dumps or decompiled source.
