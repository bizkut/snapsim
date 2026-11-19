#!/bin/bash
# Every Python-parity gate on the current tree. Fails closed: the first failing gate stops the run.
# Oracle inputs come from the recorders in the README: /tmp/snapsim-full/{spread,margin}.jsonl,
# /tmp/snapsim-api/{position,advisor}.jsonl, /tmp/snapsim-validation.jsonl and
# /tmp/snapsim-snap-oracle.jsonl. Builds go to target-gates, never the live target/release.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
REF=${SNAPSIM_REFERENCE:-$SNAPSIM_REFERENCE}
PY=${SNAPSIM_PYTHON:-$SNAPSIM_PYTHON}
RUNS=${SNAPSIM_RUNS:-$SNAPSIM_RUNS}
SNAPS=${SNAPSIM_SNAPSHOTS:-$SNAPSIM_SNAPSHOTS}
export PATH="$HOME/.cargo/bin:$PATH" CARGO_BUILD_JOBS=4 CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$ROOT/target-gates}
export SNAPSIM_REFERENCE_POOL=1
T=$CARGO_TARGET_DIR/release
cd "$ROOT"
cargo build --release --quiet --bins
cargo build --release --quiet -p snapsim-cli
crates/snapsim-py/build-module.sh > /dev/null
export SNAPSIM_PY_PATH=$T/snapsim_py.so SNAPSIM_BIN=$T/snapsim
gate() { local name=$1; shift; echo "== $name"; "$@"; }
WORK=$(mktemp -d); cd "$WORK"
gate game-spread $T/snapsim-parity $ROOT/parity/corpus-full-spread.json /tmp/snapsim-full/spread.jsonl
gate game-margin $T/snapsim-parity $ROOT/parity/corpus-full-margin.json /tmp/snapsim-full/margin.jsonl
gate position $T/snapsim-position-parity /tmp/snapsim-api/position.jsonl
gate advisor $T/snapsim-advisor-parity /tmp/snapsim-api/advisor.jsonl
gate coverage env PYTHONPATH=$REF $PY $ROOT/parity/coverage_parity.py $T/snapsim-coverage-parity /tmp/snapsim-api/position.jsonl /tmp/snapsim-api/advisor.jsonl
gate validation $T/snapsim-validation-parity /tmp/snapsim-validation.jsonl
gate snap $T/snapsim-snap-parity /tmp/snapsim-snap-oracle.jsonl
gate snap-binding env PYTHONPATH=$REF $PY $ROOT/parity/snap_binding.py $T/snapsim_py.so /tmp/snapsim-snap-oracle.jsonl
gate deckcodes env PYTHONPATH=$REF $PY $ROOT/parity/compare_deckcodes.py $T/snapsim-deckcodes-parity
gate compare-harness env PYTHONPATH=$REF $PY $ROOT/parity/compare_harness.py $T/snapsim-compare-parity
gate deckbuild env PYTHONPATH=$REF $PY $ROOT/parity/deckbuild_compare.py
gate cli-compare env PYTHONPATH=$REF $PY $ROOT/parity/cli_compare.py
gate cli-deckbuild env PYTHONPATH=$REF $PY $ROOT/parity/cli_deckbuild.py
gate cli-snap env PYTHONPATH=$REF $PY $ROOT/parity/cli_snap.py
files=$(ls $RUNS/*.jsonl $SNAPS/*.jsonl | grep -v '\.log\.jsonl$')
# Cards with rules here but not in the frozen reference: Rust coverage minus Python coverage.
beyond=$(PYTHONPATH=$REF $PY -c "
import json, subprocess
rust = set(json.loads(subprocess.check_output(['$T/snapsim', 'coverage']))['cards'])
py = set(json.loads(subprocess.check_output(['$PY', '$REF/advise.py', 'coverage']))['cards'])
print('|'.join(sorted(rust - py)))")
compared=""; new_content=""
for f in $files; do
    if grep -qE "\"card_id\": ?\"($beyond)\"" "$f"; then new_content+=" $f"; else compared+=" $f"; fi
done
echo "replay: $(echo $compared | wc -w | tr -d ' ') recordings compared; new content in:$new_content"
gate replay $PY $ROOT/parity/replay_compare.py --python $PY --reference $REF --rust $T/snapsim-replaycheck $compared
gate replay-new-content $T/snapsim-replaycheck $new_content
echo "ALL GATES PASSED"
