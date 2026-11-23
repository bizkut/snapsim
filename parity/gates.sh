#!/bin/bash
# Every Python-parity gate on the current tree. Fails closed: the first failing gate stops the run.
# Oracle inputs are generated as documented in the README under SNAPSIM_ORACLES.
# Builds go to target-gates, never the live target/release.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
REF=${SNAPSIM_REFERENCE:?Set SNAPSIM_REFERENCE to the frozen Python checkout}
PY=${SNAPSIM_PYTHON:?Set SNAPSIM_PYTHON to the Python interpreter with reference dependencies}
RUNS=${SNAPSIM_RUNS:?Set SNAPSIM_RUNS to the recordings directory}
SNAPS=${SNAPSIM_SNAPSHOTS:?Set SNAPSIM_SNAPSHOTS to the snapshots directory}
ORACLES=${SNAPSIM_ORACLES:?Set SNAPSIM_ORACLES to the oracle directory}
export PATH="$HOME/.cargo/bin:$PATH" CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-4} CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$ROOT/target-gates}
export SNAPSIM_REFERENCE_POOL=1 SNAPSIM_REFERENCE_RULES=1
T=$CARGO_TARGET_DIR/release
cd "$ROOT"
cargo build --release --quiet --bins
cargo build --release --quiet -p snapsim-cli
crates/snapsim-py/build-module.sh > /dev/null
export SNAPSIM_PY_PATH=$T/snapsim_py.so SNAPSIM_BIN=$T/snapsim
gate() { local name=$1; shift; echo "== $name"; "$@"; }
mkdir -p "$HOME/.cache/snapsim-work"
WORK=$(mktemp -d "$HOME/.cache/snapsim-work/gates.XXXXXXXX"); cd "$WORK"
gate game-spread "$T/snapsim-parity" "$ROOT/parity/corpus-full-spread.json" "$ORACLES/snapsim-full/spread.jsonl"
gate game-margin "$T/snapsim-parity" "$ROOT/parity/corpus-full-margin.json" "$ORACLES/snapsim-full/margin.jsonl"
gate position "$T/snapsim-position-parity" "$ORACLES/snapsim-api/position.jsonl"
gate advisor "$T/snapsim-advisor-parity" "$ORACLES/snapsim-api/advisor.jsonl"
gate coverage env PYTHONPATH="$REF" "$PY" "$ROOT/parity/coverage_parity.py" "$T/snapsim-coverage-parity" "$ORACLES/snapsim-api/position.jsonl" "$ORACLES/snapsim-api/advisor.jsonl"
gate validation "$T/snapsim-validation-parity" "$ORACLES/snapsim-validation.jsonl"
gate snap "$T/snapsim-snap-parity" "$ORACLES/snapsim-snap-oracle.jsonl"
gate snap-binding env PYTHONPATH="$REF" "$PY" "$ROOT/parity/snap_binding.py" "$T/snapsim_py.so" "$ORACLES/snapsim-snap-oracle.jsonl"
gate deckcodes env PYTHONPATH=$REF $PY $ROOT/parity/compare_deckcodes.py $T/snapsim-deckcodes-parity
deck_files=("$RUNS"/*.deck.json)
[[ -f "${deck_files[0]}" ]] || { echo "SNAPSIM_RUNS must contain a selected-deck .deck.json file" >&2; exit 1; }
gate compare-harness env PYTHONPATH="$REF" "$PY" "$ROOT/parity/compare_harness.py" "$T/snapsim-compare-parity" --recorded-deck "${deck_files[0]}"
gate deckbuild env PYTHONPATH=$REF $PY $ROOT/parity/deckbuild_compare.py
gate cli-compare env PYTHONPATH=$REF $PY $ROOT/parity/cli_compare.py
gate cli-deckbuild env PYTHONPATH=$REF $PY $ROOT/parity/cli_deckbuild.py
gate cli-snap env PYTHONPATH=$REF $PY $ROOT/parity/cli_snap.py
files=$(ls $RUNS/*.jsonl $SNAPS/*.jsonl | grep -v '\.log\.jsonl$')
# Cards and locations with rules here but not in the frozen reference: Rust minus Python coverage.
beyond_for() { PYTHONPATH=$REF $PY -c "
import json, subprocess
rust = set(json.loads(subprocess.check_output(['$T/snapsim', 'coverage']))['$1'])
py = set(json.loads(subprocess.check_output(['$PY', '$REF/advise.py', 'coverage']))['$1'])
print('|'.join(sorted(rust - py)) or '^$')"; }
beyond=$(beyond_for cards); beyond_locations=$(beyond_for locations)
compared=""; new_content=""; schema_v2=""
for f in $files; do
    # The frozen reference reads only schema v1 observations; v2 recordings are checked by replay-client.
    if grep -qE '"schema_version": ?2' "$f"; then
        schema_v2+=" $f"
    elif grep -qE "\"card_id\": ?\"($beyond)\"|\"location_id\": ?\"($beyond_locations)\"" "$f"; then
        new_content+=" $f"
    else
        compared+=" $f"
    fi
done
echo "replay: schema v2 (client gate only):$schema_v2"
echo "replay: $(echo $compared | wc -w | tr -d ' ') recordings compared; new content in:$new_content"
gate replay $PY $ROOT/parity/replay_compare.py --python $PY --reference $REF --rust $T/snapsim-replaycheck $compared
# The client rules themselves, deliberate changes included, against every real recording:
# no reference pin, no reference rules, and no inconsistent turn allowed.
gate replay-client env -u SNAPSIM_REFERENCE_POOL -u SNAPSIM_REFERENCE_RULES $T/snapsim-replaycheck $files
echo "ALL GATES PASSED"
