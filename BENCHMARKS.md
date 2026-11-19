# Benchmarks

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

No Rust comparisons have been measured yet.
