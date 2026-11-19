"""Emit frame-by-frame SNAP adapter and watch oracle JSONL from recordings."""
import argparse
import json
from pathlib import Path

from snap_adapter import DeckWatch, LocationWatch, position_from_snap


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("files", nargs="+", type=Path)
    args = parser.parse_args()
    for path in args.files:
        if path.name.endswith(".log.jsonl"):
            continue
        sidecar = path.with_suffix(".deck.json")
        decks = json.loads(sidecar.read_text()).get("decks", []) if sidecar.exists() else []
        deck = decks[0].get("card_ids", []) if len(decks) == 1 else None
        location_watch, deck_watch = LocationWatch(), DeckWatch()
        with path.open() as source:
            for line_number, line in enumerate(source, 1):
                if not line.strip():
                    continue
                row = json.loads(line)
                observation = row.get("envelope", {}).get("observation") if "envelope" in row else row.get("observation", row)
                if not isinstance(observation, dict):
                    continue
                turns = location_watch.update(observation)
                added = deck_watch.update(observation, deck=deck)
                try:
                    position = position_from_snap(observation, deck=deck, deck_added=added, revealed_turns=turns)
                    result = {"position": position}
                except (ValueError, KeyError, TypeError, IndexError) as exc:
                    result = {"error": type(exc).__name__}
                print(json.dumps({"file": str(path), "line": line_number, "observation": observation, "deck": deck, "turns": turns, "added": added, "result": result}, separators=(",", ":")))


if __name__ == "__main__":
    main()
