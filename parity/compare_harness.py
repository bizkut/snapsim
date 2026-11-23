"""Compare match-harness JSON with the Python oracle.

Run from a temporary cwd with PYTHONPATH pointing to the reference:
    python /path/to/parity/compare_harness.py /path/to/snapsim-compare-parity
"""
import argparse
import json
import random
from pathlib import Path
import subprocess
import sys

import advise
from position import recruit_card_pool, recruit_card_weights, sample_deck


def normalized(value):
    value = dict(value)
    value.pop('elapsed_seconds', None)
    if 'config' in value:
        value['config'] = dict(value['config'])
        for side in ('a', 'b'):
            value['config'][side] = dict(value['config'][side])
            value['config'][side].pop('engine', None)
    return value


def flags(args):
    result = []
    for name, value in args.items():
        if value is not None:
            result.extend(('--' + name.replace('_', '-'), str(value)))
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('binary')
    parser.add_argument('--games', type=int, default=20)
    parser.add_argument('--recorded-deck', type=Path, help='optional recorded selected-deck JSON')
    options = parser.parse_args()
    reference = Path(advise.__file__).parent
    hero = ','.join(recruit_card_pool()[:12])
    deck_file = str(reference / 'examples/starter-decks.txt')
    common = {'games': options.games, 'seed': 214, 'workers': 1}
    cases = [
        {'mode':'recruit', 'rollouts':2},
        {'mode':'recruit', 'deck_prior':'weighted', 'rollouts':2},
        {'mode':'recruit', 'hero_deck':hero, 'deck_prior':'weighted', 'rollouts':2},
        {'deck_file':deck_file, 'rollouts':2},
        {'mode':'recruit', 'a_playout':'margin', 'b_playout':'spread', 'rollouts':4},
        {'mode':'recruit', 'a_rollouts':2, 'b_rollouts':4, 'rollouts':2, 'b_prior':'weighted'},
    ]
    if options.recorded_deck:
        cases.append({'mode':'recruit', 'hero_deck':str(options.recorded_deck), 'rollouts':2})
    tests = [('compare', {**common, **case}) for case in cases]
    tests.extend(('benchmark', {'mode':'recruit', 'games':2, 'seed':214, 'rollouts':2, 'opponent':opponent})
                 for opponent in ('spread', 'heuristic'))
    rust = subprocess.Popen([options.binary], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
    for weighted in (False, True):
        for seed in range(50):
            rng = random.Random(seed)
            weights = recruit_card_weights() if weighted else None
            expected = [sample_deck(rng, recruit_card_pool(), weights=weights) for _ in range(2)]
            rust.stdin.write(json.dumps({'command':'deal', 'args':{'seed':seed, 'weighted':weighted}}) + '\n')
            rust.stdin.flush()
            actual = json.loads(rust.stdout.readline())
            assert actual == {'value': expected}, (seed, weighted, expected, actual)
    print('Recruit deals: 100/100 pairs matched', flush=True)
    for command, args in tests:
        oracle = subprocess.run([sys.executable, str(reference / 'advise.py'), command, *flags(args)],
                                text=True, capture_output=True, check=True)
        expected = normalized(json.loads(oracle.stdout))
        rust.stdin.write(json.dumps({'command':command, 'args':args}) + '\n')
        rust.stdin.flush()
        result = json.loads(rust.stdout.readline())
        if 'error' in result:
            raise AssertionError((command, args, result))
        actual = normalized(result['value'])
        assert actual == expected, (command, args, expected, actual)
        print(f'{command}: matched {args["games"]} seed pairs: {args}', flush=True)
    rust.stdin.close()
    if rust.wait():
        raise SystemExit('Rust parity executable failed')
    print(f'matched {len(tests)} harness configurations')


if __name__ == '__main__':
    main()
