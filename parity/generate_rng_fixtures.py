#!/usr/bin/env python3
"""Regenerate the CPython random fixtures using the reference Python interpreter."""
import json
import random
from pathlib import Path


def fixture(seed):
    rng = random.Random(seed)
    bits = [0, 1, 5, 31, 32, 33, 53, 64, 96, 128]
    result = {'seed': str(seed), 'bits': [[n, str(rng.getrandbits(n))] for n in bits]}
    result['random'] = [rng.random() for _ in range(12)]
    result['randbelow'] = [[n, rng._randbelow(n)] for n in [1, 2, 3, 7, 8, 11, 31, 32, 1000]]
    result['range'] = [[a, b, step, rng.randrange(a, b, step)] for a, b, step in [(0, 3, 1), (-20, 19, 2), (100, -20, -7), (-1, 1, 1)]]
    result['randint'] = [rng.randint(-100, 100) for _ in range(10)]
    result['choice'] = [rng.choice(list(range(n))) for n in [1, 2, 3, 4, 17, 128]]
    result['shuffle'] = []
    for n in [0, 1, 2, 10, 50]:
        values = list(range(n))
        rng.shuffle(values)
        result['shuffle'].append(values)
    result['sample'] = [[n, k, rng.sample(range(n), k)] for n, k in [(0, 0), (1, 1), (10, 3), (100, 3), (100, 10), (100, 90)]]
    state = rng.getstate()
    result['state'] = list(state[1])
    result['after_state'] = [rng.getrandbits(32) for _ in range(8)]
    rng.setstate(state)
    assert result['after_state'] == [rng.getrandbits(32) for _ in range(8)]
    return result


if __name__ == '__main__':
    path = Path(__file__).with_name('rng_fixtures.json')
    path.write_text(json.dumps([fixture(seed) for seed in [0, 1, -1, 42, 20261119, 2**100 + 12345]], indent=2) + '\n')
