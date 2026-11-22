"""Check CLI outputs against the Python reference at fixed budgets."""
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
REFERENCE = Path('$SNAPSIM_REFERENCE')
BINARY = Path(os.environ.get('SNAPSIM_BIN', ROOT / 'target-RustDeckbuild/release/snapsim'))


def invoke(cmd):
    return json.loads(subprocess.check_output(cmd, text=True))


def main():
    cases = [
        ['coverage'],
        ['position', '--file', str(ROOT / 'parity/turn3.json'), '--rollouts', '2', '--seed', '17'],
        ['position', '--file', str(ROOT / 'parity/turn3.json'), '--rollouts', '4', '--seed', '3', '--max-plans', '2'],
        ['compare', '--mode', 'recruit', '--games', '1', '--workers', '1', '--rollouts', '1', '--seed', '11'],
        ['compare', '--mode', 'recruit', '--games', '1', '--workers', '1', '--rollouts', '2', '--seed', '19', '--deck-prior', 'weighted'],
        ['benchmark', '--mode', 'recruit', '--games', '1', '--rollouts', '1', '--seed', '11'],
        ['benchmark', '--mode', 'recruit', '--games', '1', '--rollouts', '2', '--seed', '19', '--opponent', 'spread'],
    ]
    added = []
    for args in cases:
        expected = invoke([sys.executable, str(REFERENCE / 'advise.py'), *args])
        actual = invoke([str(BINARY), *args, *([] if args[0] == 'coverage' else ['--plays-only'])])
        if args[0] == 'coverage':
            # Rules added after the reference froze are new content: every reference card and
            # location must still be listed, and nothing else may differ.
            missing = sorted(set(expected['cards']) - set(actual['cards']))
            assert not missing, (args, 'reference cards no longer supported', missing)
            missing = sorted(set(expected['locations']) - set(actual['locations']))
            assert not missing, (args, 'reference locations no longer supported', missing)
            added = sorted(set(actual['cards']) - set(expected['cards']))
            added_locations = sorted(set(actual['locations']) - set(expected['locations']))
            actual['cards'] = expected['cards']
            actual['locations'] = expected['locations']
        if args[0] == 'compare':
            expected.pop('elapsed_seconds', None)
            actual.pop('elapsed_seconds', None)
            for result in (expected, actual):
                for side in ('a', 'b'):
                    result['config'][side].pop('engine', None)
        assert expected == actual, (args, expected, actual)
    print(f'CLI parity: {len(cases)} cases passed ({len(added)} cards and '
          f'{len(added_locations)} locations beyond the reference)')


if __name__ == '__main__':
    main()
