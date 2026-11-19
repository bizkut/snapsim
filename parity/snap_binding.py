"""Compare the installed Rust Python binding against a snap_record.py corpus."""
import argparse
import importlib.util
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("module", type=Path, help="path to the snapsim_py extension")
    parser.add_argument("corpus", type=Path)
    args = parser.parse_args()
    spec = importlib.util.spec_from_file_location("snapsim_py", args.module)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    count = 0
    with args.corpus.open() as source:
        for line in source:
            case = json.loads(line)
            options = {"deck": case["deck"], "deck_added": case["added"], "revealed_turns": case["turns"], "plays_only": True}
            try:
                position = json.loads(module.position_from_snap(json.dumps(case["observation"]), json.dumps(options)))
                actual = {"position": position}
            except module.NotPlanningError:
                actual = {"error": "NotPlanningError"}
            except ValueError:
                actual = {"error": "ValueError"}
            expected = case["result"]
            # The binding contract converts malformed input to ValueError even where
            # the reference adapter itself raises TypeError.
            if expected.get("error") in ("TypeError", "KeyError", "IndexError"):
                expected = {"error": "ValueError"}
            if actual != expected:
                raise AssertionError(f"{case['file']}:{case['line']}: expected {expected!r}, got {actual!r}")
            count += 1
    print(f"Matched Python binding on {count} frames")


if __name__ == "__main__":
    main()
