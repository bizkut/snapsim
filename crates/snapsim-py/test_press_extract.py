import unittest

from test_trajectory_extract import frame
from trajectory_extract import press_action, Unavailable


class PressTests(unittest.TestCase):
    def test_map_confirmed_play_and_reject_failed(self):
        observation = frame()
        self.assertEqual(press_action(observation, {"card": 10, "lane": 2, "result": {"status": "staged"}}), 2)
        with self.assertRaisesRegex(Unavailable, "press_not_confirmed"):
            press_action(observation, {"card": 10, "lane": 2, "result": {"status": "failed"}})

    def test_end_and_snap(self):
        observation = frame()
        self.assertEqual(press_action(observation, {"action": "end_turn", "result": {"status": "ended"}}), 69)
        self.assertEqual(press_action(observation, {"action": "snap", "result": {"status": "tapped", "message": "Snap confirmed by the game."}}), 70)


if __name__ == "__main__":
    unittest.main()
