import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import numpy as np

from test_trajectory_extract import frame
from trajectory_convert import _matches, convert, remask


class ConverterTests(unittest.TestCase):
    def test_heartbeat_does_not_split_match(self):
        first, second = frame(1), frame(2)
        rows = [{"envelope": {"observation": first}},
                {"heartbeat": {"stream_id": "s", "sequence": 1}},
                {"envelope": {"observation": second}}]
        self.assertEqual([len(segment) for segment in _matches(rows)], [2])

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
            rows = [{"envelope": {"stream_id": "s", "sequence": 1, "observation": before}},
                    {"t": 1, "heartbeat": {"stream_id": "s", "sequence": 1,
                                               "sample_count": 5}},
                    {"envelope": {"stream_id": "s", "sequence": 2, "observation": after}}]
            path.write_text("".join(json.dumps(row) + "\n" for row in rows))
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

    def test_remask_counts_ledger_reasons(self):
        import copy
        parent = Path.home() / ".cache/snapsim-work/test-datasets"
        parent.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=parent) as work:
            work = Path(work)
            before = frame()
            before["game"]["provenance"] = {
                "schema_version": 2, "entities": [],
                "ledger": {"complete": False, "match_start_seen": False,
                           "reasons": ["missing_match_start"], "items": [
                               {"attribution": "ambiguous", "reasons": ["multiple_sources"]},
                               {"attribution": "exact", "reasons": []}]},
            }
            after = copy.deepcopy(before)
            card = after["game"]["players"][0]["zones"]["hand"]["cards"].pop()
            after["game"]["players"][0]["zones"]["hand"]["count"] = 0
            after["game"]["locations"][2]["cards"].append(card)
            path = work / "recording.jsonl"
            path.write_text("".join(json.dumps({"envelope": {"stream_id": "s", "sequence": i,
                                                              "observation": obs}}) + "\n"
                                    for i, obs in enumerate([before, after], 1)))
            convert([path], work / "out")
            reasons = remask([path], work / "out")["gate_reasons"]
            self.assertEqual(reasons["provenance_ledger_missing_match_start"], 1)
            self.assertEqual(reasons["provenance_ledger_attribution_ambiguous"], 1)
            self.assertEqual(reasons["provenance_ledger_item_multiple_sources"], 1)

    def test_player_cost_gate_clears_only_for_complete_exact_swan(self):
        import copy
        import snapsim_py as sim
        provenance = {"schema_version": 2, "entities_complete": True, "entities": [{
            "kind": "player", "entity_id": 1, "card_id": None, "lane_index": None,
            "abilities": {"complete": True, "reasons": [], "items": [{
                "ability_def_id": "BlackSwanOngoing", "owner_def_type": "Card",
                "owner_def_id": "BlackSwan", "index": 4, "origin_entity_id": 42,
                "created_turn": 2, "lifetime": 2, "active": True}]},
            "movable_components": {"complete": True, "reasons": [], "items": []}}],
            "ledger": {"complete": False, "complete_for_legality": True,
                       "legality_reasons": [], "match_start_seen": True, "reasons": [],
                       "items": [{"effect_key": "swan", "kind": "ability",
                                  "target_entity_id": 1, "creator_entity_id": 42,
                                  "creator_def_id": "BlackSwan", "created_turn": 2,
                                  "expiry_turn": None, "attribution": "exact", "reasons": [],
                                  "active": True, "legality_relevant": True,
                                  "ability_def_id": "BlackSwanOngoing", "ability_index": 4}]}}
        position = {"turn": 3, "energy": 3, "hand": [], "deck": [],
                    "locations": [{"id": None, "mine": [], "theirs": []}] * 3,
                    "opponent": {"hand": 0, "deck": 0}, "player_entity_ids": [1, 2],
                    "provenance": provenance}
        def gate():
            return sim.snapshot_hidden_mask_gate([], [], [], json.dumps(provenance), json.dumps(position))
        self.assertNotIn("player cost effects are unknown", gate())
        bad = copy.deepcopy(provenance)
        bad["ledger"]["complete_for_legality"] = False
        position["provenance"] = bad
        reasons = sim.snapshot_hidden_mask_gate([], [], [], json.dumps(bad), json.dumps(position))
        self.assertIn("player cost effects are unknown", reasons)

    def test_intrinsic_move_gate_requires_complete_spent_history(self):
        import snapsim_py as sim
        ability = "NightcrawlerMovableOngoing"
        provenance = {"schema_version": 2, "entities_complete": True, "entities": [{
            "kind": "card", "entity_id": 42, "card_id": "Nightcrawler", "lane_index": 0,
            "abilities": {"complete": True, "reasons": [], "items": [{
                "ability_def_id": ability, "owner_def_type": "Card",
                "owner_def_id": "Nightcrawler", "index": 4}]},
            "movable_components": {"complete": True, "reasons": [], "items": [{
                "source_entity_id": 42, "owner_def_type": "Card", "owner_def_id": "Nightcrawler",
                "ability_index": 4, "created_ticks": "123", "destination_entity_id": 0}]}}],
            "ledger": {"complete": True, "complete_for_legality": True, "match_start_seen": True,
                       "reasons": [], "items": [{"effect_key": "move", "kind": "move_grant",
                           "target_entity_id": 42, "creator_entity_id": 42,
                           "creator_def_id": "Nightcrawler", "created_turn": 1,
                           "expiry_turn": None, "attribution": "exact", "reasons": [],
                           "active": True, "ability_def_id": ability, "ability_index": 4}]}}
        action = {"owner": "local", "turn": 1, "kind": "play", "location_slot": 0,
                  "card_entity_id": 42, "card_id": "Nightcrawler", "resolved": True,
                  "turn_resolved": 1, "resolution_order": 1}
        position = {"turn": 2, "energy": 2, "hand": [], "deck": [],
                    "locations": [{"id": "Ruins", "mine": [{"id": "Nightcrawler",
                        "entity_id": 42, "revealed": True}], "theirs": []},
                        {"id": "Ruins", "mine": [], "theirs": []},
                        {"id": "Ruins", "mine": [], "theirs": []}],
                    "opponent": {"hand": 0, "deck": 0}, "provenance": provenance,
                    "history": {"actions": [action], "action_completeness":
                        {"local": {"1": True, "2": True}, "opponent": {}}, "own_draws": None}}
        def gate():
            return sim.snapshot_hidden_mask_gate([], [], [], json.dumps(provenance), json.dumps(position))
        self.assertNotIn("move source provenance is unknown", gate())
        del position["history"]["action_completeness"]["local"]["2"]
        self.assertIn("move source provenance is unknown", gate())


if __name__ == "__main__":
    unittest.main()
