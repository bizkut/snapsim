"""Compare Python and Rust replaycheck output on files separately and together."""

import argparse
import os
from pathlib import Path
import subprocess
import sys
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--python", required=True, help="Python interpreter for the reference")
    parser.add_argument("--reference", required=True, type=Path, help="Python simulator checkout")
    parser.add_argument("--rust", required=True, type=Path, help="snapsim-replaycheck executable")
    parser.add_argument("--seeds", type=int, default=128)
    parser.add_argument("files", nargs="+", type=Path)
    args = parser.parse_args()
    reference = args.reference.resolve()
    rust = args.rust.resolve()
    files = [str(path.resolve()) for path in args.files]
    env = os.environ.copy()
    env["PYTHONPATH"] = str(reference)
    with tempfile.TemporaryDirectory(prefix="snapsim-replay-") as cwd:
        for selection in [[path] for path in files] + [files]:
            python = subprocess.run(
                [args.python, str(reference / "replaycheck.py"), "--seeds", str(args.seeds), *selection],
                cwd=cwd, env=env, capture_output=True, check=False,
            )
            native = subprocess.run(
                [str(rust), "--seeds", str(args.seeds), *selection],
                cwd=cwd, env=env, capture_output=True, check=False,
            )
            if (python.returncode, python.stdout, python.stderr) != (
                native.returncode, native.stdout, native.stderr
            ):
                print(f"Replay mismatch: {selection}", file=sys.stderr)
                print(f"Python exit {python.returncode}:\n{python.stdout.decode(errors='replace')}{python.stderr.decode(errors='replace')}", file=sys.stderr)
                print(f"Rust exit {native.returncode}:\n{native.stdout.decode(errors='replace')}{native.stderr.decode(errors='replace')}", file=sys.stderr)
                return 1
    print(f"Matched {len(files)} recordings individually and together")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
