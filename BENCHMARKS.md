# Benchmarks

## Full-content corpus comparison

Single run, same machine and conditions as below (shared; another project's training process may have been using about five cores). The corpus `parity/corpus-full-{spread,margin}.json` holds every deck card (84) and every location (84) the Python reference implements. Decks are drawn uniformly from those cards, not with the weighted Recruit prior. The parity gate matched 1,000 seeded games per policy on this corpus before these runs. Python and Rust ran back to back, one process each.

| Policy | Games | Python seconds | Python games/s | Rust seconds | Rust games/s | Measured Rust/Python |
|---|---:|---:|---:|---:|---:|---:|
| Spread | 5,000 | 4.549169 | 1,099.10 | 0.209396 | 23,878.25 | 21.73× |
| Margin | 5,000 | 4.802480 | 1,041.13 | 0.176696 | 28,297.22 | 27.18× |

Load averages (1, 5, 15 minutes), immediately before each measurement: Python spread 5.58/7.07/7.55; Rust spread 5.58/7.07/7.55; Python margin 5.77/7.08/7.55; Rust margin 5.87/7.08/7.55. The commands are the ones below with `corpus-full-$policy.json`. Advisor rollouts are still unmeasured in Rust: the rollout loop is not ported.

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

The full-Recruit and advisor-rollout **Rust** workloads remain unmeasured: their rules and rollout loop are not implemented. The Python-only numbers below are not comparable to the representative Rust workloads.

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

No Rust comparisons have been measured for these full-Recruit or advisor workloads.
