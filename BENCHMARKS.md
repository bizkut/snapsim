# Benchmarks

## Advisor rollout comparison

Single back-to-back run on the same shared Apple M4 Pro machine, CPython 3.12 and Rust release build. Each search used `parity/turn3.json`, spread playout, seed 42, a warmup, and a fixed 2,000-rollout budget. The timed searches used a 10,000,000-rollout cap and a one-second limit. Results include position setup and Python binding overhead. The 1-second searches may screen a different number of plans from the fixed-budget searches.

| Workers | Engine | Fixed seconds | Fixed rollouts/s | Rollouts in 1 second | Actual seconds |
|---:|---|---:|---:|---:|---:|
| 1 | Python | 0.6151 | 3,252 | 3,262 | 1.0003 |
| 1 | Rust | 0.0745 | 26,837 | 27,979 | 1.0002 |
| 4 | Python | 0.1833 | 10,914 | 11,383 | 1.0006 |
| 4 | Rust | 0.0259 | 77,351 | 88,165 | 1.0002 |

Fixed-budget throughput was 8.25× higher with one worker and 7.09× with four. Load averages (1/5/15 minutes) before Python 1, Rust 1, Python 4 and Rust 4 respectively were 5.35/5.21/5.76, 5.35/5.21/5.76, 5.16/5.17/5.75 and 5.16/5.17/5.75. After the last run they were 5.16/5.17/5.75. These are single-run measurements on a shared machine, not win-rate results.

```sh
export PATH="$HOME/.cargo/bin:$PATH" CARGO_BUILD_JOBS=6
crates/snapsim-py/build-module.sh
ROOT="$PWD"
cd /tmp
PYTHONPATH=$SNAPSIM_REFERENCE \
  $SNAPSIM_PYTHON \
  "$ROOT/parity/bench_advisor.py" --rollouts 2000
```

## Full-content corpus comparison

Single run, same machine and conditions as below (shared; another project's training process may have been using about five cores). The corpus `parity/corpus-full-{spread,margin}.json` holds every deck card (84) and every location (84) the Python reference implements. Decks are drawn uniformly from those cards, not with the weighted Recruit prior. The parity gate matched 1,000 seeded games per policy on this corpus before these runs. Python and Rust ran back to back, one process each.

| Policy | Games | Python seconds | Python games/s | Rust seconds | Rust games/s | Measured Rust/Python |
|---|---:|---:|---:|---:|---:|---:|
| Spread | 5,000 | 4.549169 | 1,099.10 | 0.209396 | 23,878.25 | 21.73× |
| Margin | 5,000 | 4.802480 | 1,041.13 | 0.176696 | 28,297.22 | 27.18× |

Load averages (1, 5, 15 minutes), immediately before each measurement: Python spread 5.58/7.07/7.55; Rust spread 5.58/7.07/7.55; Python margin 5.77/7.08/7.55; Rust margin 5.87/7.08/7.55. The commands are the ones below with `corpus-full-$policy.json`.

## Representative-corpus comparison

Single run on Apple M4 Pro, 24 GiB RAM, macOS Darwin 27.2.0 arm64, CPython 3.12.16, Rust 1.98.1, release profile. The machine was shared; another project's training process may have been using about five cores. Each measurement used one process, with Python and Rust run back to back for each policy. The fixed corpus is in `parity/corpus-{spread,margin}.json`; it is **not** the full Recruit pool. For each seed, both implementations sample the same two decks, seed the game separately, sample the same location pool and run the named policy in both seats. Timings include sampling, setup and complete games. Rust also constructs turn traces while Python does not, so the measured Rust workload includes that extra work. No warmup or statistical isolation was applied.

| Policy | Games | Python seconds | Python games/s | Rust seconds | Rust games/s | Measured Rust/Python |
|---|---:|---:|---:|---:|---:|---:|
| Spread | 5,000 | 3.788306 | 1,319.85 | 0.150738 | 33,170.10 | 25.13× |
| Margin | 5,000 | 4.027489 | 1,241.47 | 0.127564 | 39,195.92 | 31.57× |

Load averages (1, 5, 15 minutes), immediately before each measurement: Python spread 4.53/6.24/7.53; Rust spread 4.33/6.17/7.49; Python margin 4.33/6.17/7.49; Rust margin 4.62/6.20/7.50. These are single-run throughput ratios, not claims about all content or systems.

```sh
export PATH="$HOME/.cargo/bin:$PATH" CARGO_TARGET_DIR="$PWD/target"
cargo build --release --bins
ROOT="$PWD"
mkdir -p /tmp/snapsim-bench
cd /tmp/snapsim-bench
for policy in spread margin; do
  uptime
  PYTHONPATH=$SNAPSIM_REFERENCE \
    $SNAPSIM_PYTHON \
    "$ROOT/parity/corpus_bench.py" "$ROOT/parity/corpus-$policy.json" \
    --games 5000 --seed 20261119
  uptime
  "$ROOT/target/release/snapsim-bench" "$ROOT/parity/corpus-$policy.json" 5000 20261119
done
```

The full-Recruit game workload remains unmeasured in Rust; the advisor comparison above is a separate workload. The Python-only numbers below are not comparable to the representative Rust full-game workloads.

## Provisional Python-only baseline

**Provisional:** This run preceded the request to measure both implementations back to back under the same load. It must not be used for a speed ratio.

Measured on Apple M4 Pro, 24 GiB RAM, macOS Darwin 27.2.0 arm64. The machine is shared; load averages immediately before measurement were 8.33, 8.91, 9.54. Results are a single run, not a controlled isolation or a statistical estimate. One Python process was used. Games include weighted Recruit deck sampling, setup and play; each game uses the seed both for its two deck samples and the independently seeded global game RNG. Both seats use the named policy. The advisor workload uses the fixed turn-3 position in `parity/turn3.json`, strict advice, spread playout, and `workers=1`.

```sh
mkdir -p /tmp/snapsim-bench
cd /tmp/snapsim-bench
PYTHONPATH=$SNAPSIM_REFERENCE \
  $SNAPSIM_PYTHON \
  ./parity/python_bench.py \
  --games 500 --rollouts 1000 --seed 20261119
```

| Workload | Count | Python seconds | Python/s |
|---|---:|---:|---:|
| Spread full games | 500 | 0.424142 | 1178.85 |
| Margin full games | 500 | 0.446800 | 1119.07 |
| Turn-3 advisor rollouts | 1000 | 0.312598 | 3199.00 |

No Rust full-Recruit game comparison has been measured for these Python-only runs.
