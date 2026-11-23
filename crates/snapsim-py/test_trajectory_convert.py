import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import numpy as np

from test_trajectory_extract import frame
from trajectory_convert import convert, remask


class ConverterTests(unittest.TestCase):
    def test_shard_mask_is_explicitly_invalid(self):
        parent = Path.home() / ".cache/snapsim-work/test-datasets"
        parent.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=parent) as work:
            work = Path(work)
            before = frame()
            for player in before["game"]["players"]:
                player["snap"] = {"requested_turns": []}
            import copy
            after = copy.deepcopy(before)
            card = after["game"]["players"][0]["zones"]["hand"]["cards"].pop()
            after["game"]["players"][0]["zones"]["hand"]["count"] = 0
            after["game"]["locations"][2]["cards"].append(card)
            path = work / "recording.jsonl"
            path.write_text("".join(json.dumps({"envelope": {"stream_id": "s", "sequence": i,
                                                              "observation": obs}}) + "\n"
                                    for i, obs in enumerate([before, after], 1)))
            report = convert([path], work / "out")
            self.assertEqual(report["counts"]["decisions"], 1)
            with np.load(work / "out/trajectory-000001.npz", allow_pickle=False) as shard:
                self.assertEqual(shard["action"].tolist(), [2])
                self.assertFalse(shard["mask_valid"][0])
                self.assertEqual(shard["mask_reason"][0], "hidden_dependent")
                self.assertEqual(shard["snap_flags_basis"][0], "fields")
                self.assertEqual(shard["source_sequence"][0], 1)
                self.assertTrue(shard["snap_flags_known"][0])
            masked = remask([path], work / "out")
            self.assertEqual(masked["counts"]["blocker_effect_provenance"], 1)
            with np.load(work / "out/trajectory-000001.npz", allow_pickle=False) as shard:
                self.assertFalse(shard["mask_valid"][0])
                self.assertEqual(shard["mask_pool"][0], "full")
                self.assertIn("effect_provenance", shard["mask_blockers"][0])

    def test_unknown_snap_flags_do_not_block_non_stake_mask(self):
        import copy
        import snapsim_py as sim

        parent = Path.home() / ".cache/snapsim-work/test-datasets"
        parent.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=parent) as work:
            work = Path(work)
            before = frame()
            after = copy.deepcopy(before)
            card = after["game"]["players"][0]["zones"]["hand"]["cards"].pop()
            after["game"]["players"][0]["zones"]["hand"]["count"] = 0
            after["game"]["locations"][2]["cards"].append(card)
            path = work / "recording.jsonl"
            path.write_text("".join(json.dumps({"envelope": {"stream_id": "s", "sequence": i,
                                                              "observation": obs}}) + "\n"
                                    for i, obs in enumerate([before, after], 1)))
            convert([path], work / "out")
            mask = [0] * 73
            mask[2] = mask[69] = mask[70] = 1
            result = json.dumps({"coverage": {}, "masks": [mask] * 8})
            with patch.object(sim, "snapshot_hidden_mask_gate", return_value=[]), \
                 patch.object(sim, "position_from_snap", return_value="{}"), \
                 patch.object(sim, "sample_position_masks", return_value=result):
                report = remask([path], work / "out")
            self.assertEqual(report["counts"]["valid"], 1)
            with np.load(work / "out/trajectory-000001.npz", allow_pickle=False) as shard:
                self.assertFalse(shard["snap_flags_known"][0])
                self.assertTrue(shard["mask_valid"][0])
                self.assertFalse(shard["stake_mask_valid"][0])
                self.assertEqual(shard["legal_mask"][0, 2], 1)
                self.assertEqual(shard["legal_mask"][0, 69], 1)
                self.assertFalse(shard["legal_mask"][0, 70:].any())


if __name__ == "__main__":
    unittest.main()
