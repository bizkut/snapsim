import base64
import json
import random
import unittest
from pathlib import Path

import numpy as np
import snapsim_py as sim

ROOT = Path(__file__).resolve().parents[2]


def decks():
    ids = [x['carddefid'] for x in json.loads((ROOT / 'data/cards.json').read_text())]
    compact = {''.join(c for c in name if c not in 'aeiouy') + format(len(name), 'X'): name for name in ids}
    codes = [line for line in (ROOT / 'decks/snapcomplete-2026-11.txt').read_text().splitlines() if line and not line.startswith('#')]
    return [[compact[x] for x in base64.b64decode(code).decode().split(',')] for code in codes]


def buffers(n):
    return (np.zeros((n, 2), np.uint8), np.zeros((n, 2, sim.ENV_OBS_SIZE), np.int32),
            np.zeros((n, 2, sim.ENV_ACTION_COUNT), np.uint8), np.zeros((n, 2), np.int64),
            np.zeros(n, np.uint8), np.zeros((n, 2), np.uint8))


def play(ds, seeds, threads):
    n = len(ds)
    env = sim.VecEnv([[d, d] for d in ds], seeds, threads=threads)
    due, obs, mask, rewards, terminal, rejected = buffers(n)
    rng = [random.Random(seed) for seed in seeds]
    phase = np.zeros((n, 3), np.int32)
    traces = [[] for _ in ds]
    done = [False] * n
    for _ in range(1000):
        env.observe(phase, due, obs, mask)
        actions = np.full((n, 2), sim.ENV_NOOP, np.int64)
        for i in range(n):
            if done[i]:
                continue
            traces[i].append((phase[i].tobytes(), due[i].tobytes(), obs[i].tobytes(), mask[i].tobytes()))
            for seat in range(2):
                if due[i, seat]:
                    # Snap and retreat are exercised by engine tests; prefer actual full games here.
                    choices = np.flatnonzero(mask[i, seat, :70])
                    assert len(choices)
                    actions[i, seat] = int(rng[i].choice(choices))
        reasons = env.step(actions, rewards, terminal, rejected)
        assert not rejected.any(), reasons
        for i in range(n):
            if not done[i]:
                traces[i].append((tuple(actions[i]), tuple(rewards[i]), int(terminal[i])))
                done[i] = bool(terminal[i])
        if all(done):
            return traces
    raise AssertionError('games did not terminate')


class VectorEnvTests(unittest.TestCase):
    def test_sampled_position_masks_are_diagnostic(self):
        position = {
            "turn": 1, "energy": 1,
            "hand": [{"id": "AntMan", "cost": 1, "power": 1}],
            "deck": [], "deck_count": 0,
            "locations": [{"id": "Mojoworld", "mine": [], "theirs": []},
                          {"id": None, "mine": [], "theirs": []},
                          {"id": None, "mine": [], "theirs": []}],
            "opponent": {"hand": 3, "deck": 9},
        }
        result = json.loads(sim.sample_position_masks(
            json.dumps(position), 1, 1, 8, True, (False, False)))
        self.assertEqual(len(result["masks"]), 8)
        self.assertTrue(all(len(mask) == sim.ENV_ACTION_COUNT for mask in result["masks"]))
        self.assertTrue(all(mask[69] for mask in result["masks"]))
        self.assertIn("coverage", result)

    def test_hidden_mask_gate_rejects_missing_provenance(self):
        reasons = sim.snapshot_hidden_mask_gate([], [], [])
        self.assertTrue(any("unknown" in reason for reason in reasons))

    def test_shapes_and_rejections(self):
        d = decks()[0]
        env = sim.VecEnv([[d, d]], [5])
        due, obs, mask, rewards, terminal, rejected = buffers(1)
        phase = np.zeros((1, 3), np.int32)
        env.observe(phase, due, obs, mask)
        self.assertEqual(due.tolist(), [[1, 1]])
        self.assertTrue((obs[0, :, 0] > 0).all())
        self.assertTrue(mask[0, :, 69].all())
        with self.assertRaises(ValueError):
            env.observe(phase, due, obs[:, :, :-1], mask)
        reasons = env.step(np.full((1, 2), -1, np.int64), rewards, terminal, rejected)
        self.assertTrue(rejected.all())
        self.assertTrue(all(reasons[0]))
        env.reset([0], [5])
        with self.assertRaises(ValueError):
            env.reset([0, 0], [5, 6])

    def test_all_decks_and_thread_determinism(self):
        ds = decks()
        seeds = list(range(100, 100 + len(ds)))
        one = play(ds, seeds, 1)
        self.assertEqual(one, play(ds, seeds, 8))
        for i in range(len(ds)):
            self.assertEqual(one[i], play([ds[i]], [seeds[i]], 1)[0])

    def test_opponent_hidden_deck_not_encoded(self):
        ds = decks()
        env = sim.VecEnv([[ds[0], ds[1]], [ds[0], ds[2]]], [42, 42])
        due, obs, mask, *_ = buffers(2)
        phase = np.zeros((2, 3), np.int32)
        env.observe(phase, due, obs, mask)
        np.testing.assert_array_equal(obs[0, 0], obs[1, 0])
        np.testing.assert_array_equal(mask[0, 0], mask[1, 0])
        self.assertFalse(np.array_equal(obs[0, 1], obs[1, 1]))


if __name__ == '__main__':
    unittest.main()
