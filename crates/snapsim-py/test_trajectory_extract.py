import copy
import unittest

from trajectory_extract import Unavailable, action_between, encode_observation, extract_decisions, snap_flags_from_history


def frame(turn=1):
    hand = {"entity_id": 10, "owner_entity_id": 1, "zone_position": 0,
            "card_id": "AntMan", "cost": 1, "power": 1, "revealed": False}
    return {"source": {"terminal": False}, "game": {
        "turn": turn, "total_turns": 6, "cube_value": 1, "cube_value_next_turn": 1,
        "stakes": {"max": 8}, "players": [
            {"slot": "local", "entity_id": 1, "energy": 1, "ended_turn": 0,
             "zones": {"hand": {"count": 1, "cards": [hand]}, "deck": {"count": 8}}},
            {"slot": "opponent", "entity_id": 2,
             "zones": {"hand": {"count": 4}, "deck": {"count": 8}}}],
        "locations": [{"entity_id": 20 + i, "location_id": "RevealOn2", "cards": []}
                      for i in range(3)],
        "legal_actions": [{"entity_id": 10, "valid": True, "target_entity_ids": [20, 21, 22]}]}}


class ExtractionTests(unittest.TestCase):
    def test_hidden_identity_never_enters_payload(self):
        before = frame()
        other = copy.deepcopy(before)
        hidden = {"entity_id": 50, "owner_entity_id": 2, "zone_position": 0,
                  "revealed": False, "card_id": "SecretA", "cost": 99, "power": 99}
        before["game"]["locations"][0]["cards"].append(hidden)
        other["game"]["locations"][0]["cards"].append({**hidden, "card_id": "SecretB"})
        self.assertEqual(encode_observation(before, (False, False)),
                         encode_observation(other, (False, False)))

    def test_play_and_ambiguous_end(self):
        before = frame()
        after = copy.deepcopy(before)
        card = after["game"]["players"][0]["zones"]["hand"]["cards"].pop()
        after["game"]["players"][0]["zones"]["hand"]["count"] = 0
        after["game"]["locations"][2]["cards"].append(card)
        self.assertEqual(action_between(before, after), 2)
        after["game"]["players"][0]["ended_turn"] = 1
        with self.assertRaisesRegex(Unavailable, "action_order_ambiguous"):
            action_between(before, after)

    def test_unknown_snap_flags_are_explicit(self):
        before = frame()
        after = copy.deepcopy(before)
        card = after["game"]["players"][0]["zones"]["hand"]["cards"].pop()
        after["game"]["players"][0]["zones"]["hand"]["count"] = 0
        after["game"]["locations"][2]["cards"].append(card)
        decisions, _ = extract_decisions([before, after], [1, 2], ["s", "s"])
        self.assertEqual(len(decisions), 1)
        self.assertFalse(decisions[0]["snap_flags_known"])
        self.assertEqual(decisions[0]["snap_flags_basis"], "unknown")
        self.assertEqual(decisions[0]["observation"][9:11], [0, 0])

    def test_snap_proof_requires_cap_and_contiguous_sequence(self):
        frames = [frame(0), frame(1), frame(2)]
        flags = snap_flags_from_history(frames, [1, 2, 3], ["s"] * 3)
        self.assertEqual(flags[1], ((False, False), "no_raise_proof"))
        flags = snap_flags_from_history(frames, [1, 3, 4], ["s"] * 3)
        self.assertEqual(flags[1], (None, "observation_snap_unknown"))
        frames[1]["game"]["cube_value_next_turn"] = 8
        flags = snap_flags_from_history(frames, [1, 2, 3], ["s"] * 3)
        self.assertEqual(flags[1], (None, "observation_snap_unknown"))

    def test_confirmed_snap_requires_opponent_flag_and_timestamp(self):
        frames = [frame(0), frame(1), frame(2)]
        for observation in frames:
            observation["game"]["players"][1]["snap"] = {"requested_turns": []}
        logs = [{"t": 2.5, "play": {"action": "snap", "revision": "s:2",
                  "result": {"status": "tapped", "message": "Snap confirmed by the game."}}}]
        flags = snap_flags_from_history(frames, [1, 2, 3], ["s"] * 3, [1.0, 2.0, 3.0], logs)
        self.assertEqual(flags[1], ((False, False), "no_raise_proof"))
        self.assertEqual(flags[2], ((True, False), "press_log"))


if __name__ == "__main__":
    unittest.main()
