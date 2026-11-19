#!/usr/bin/env python3
"""Capture SearchAI decisions and SNAP observations with Python reference results.

Run with PYTHONPATH pointing at the advice worktree, from any directory.
"""
import argparse
import json
import random
from pathlib import Path

import advisor
from factories import resolve_deck
from game import Game
from position import Position, quiet, recruit_card_pool, recruit_location_pool
from snap_adapter import DeckWatch, LocationWatch, NotPlanningError, position_from_snap


def recorded_positions(directory):
    seen = set()
    for path in sorted(directory.glob('*.jsonl')):
        if path.name.endswith('.log.jsonl'):
            continue
        deck_path = path.with_suffix('.deck.json')
        deck = json.loads(deck_path.read_text())['decks'][0]['card_ids'] if deck_path.exists() else None
        locations, additions = LocationWatch(), DeckWatch()
        for line in path.read_text().splitlines():
            frame = json.loads(line).get('envelope', {}).get('observation')
            if not frame:
                continue
            turns = locations.update(frame)
            added = additions.update(frame, deck=deck)
            try:
                raw = position_from_snap(frame, deck=deck, mode='recruit', revealed_turns=turns, deck_added=added)
                position = Position.from_dict(raw)
            except (NotPlanningError, ValueError, KeyError, TypeError):
                continue
            key = json.dumps(position.to_dict(), sort_keys=True)
            if key not in seen:
                seen.add(key)
                yield position, str(path.name)


def simulated_positions(count):
    original = advisor.advise
    captured = []
    def capture(position, **kwargs):
        captured.append((position, 'SearchAI'))
        return original(position, **kwargs)
    advisor.advise = capture
    try:
        for seed in range(count):
            deal = random.Random(seed)
            pool = recruit_card_pool()
            decks = tuple(resolve_deck(tuple(deal.sample(pool, 12))) for _ in range(2))
            random.seed(seed)
            search = advisor.SearchAI(rollouts=1, seed=seed, mode='recruit', approximate=True, workers=1)
            Game(decks=decks, policies=(search, advisor.SpreadPlayer()),
                 location_ids=recruit_location_pool()).play_game(record=False)
    finally:
        advisor.advise = original
    return captured


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--runs', type=Path, required=True)
    parser.add_argument('--games', type=int, default=60)
    parser.add_argument('--rollouts', type=int, default=24)
    parser.add_argument('--seed', type=int, default=23)
    args = parser.parse_args()
    with quiet():
        positions = list(recorded_positions(args.runs)) + simulated_positions(args.games)
        counts = {}
        with args.output.open('w') as output:
            for index, (position, source) in enumerate(positions):
                counts[source] = counts.get(source, 0) + 1
                for name in ('spread', 'margin'):
                    kwargs = dict(rollouts=args.rollouts, seed=args.seed + index, max_plans=5,
                                  time_limit=None, approximate=True, workers=1,
                                  playout=advisor.playout_policy(name))
                    expected = advisor.advise(position, **kwargs)
                    options = dict(rollouts=kwargs['rollouts'], seed=kwargs['seed'], max_plans=5,
                                   time_limit=None, approximate=True, workers=1, playout=name)
                    print(json.dumps(dict(source=source, position=position.to_dict(), options=options,
                                          expected=expected), separators=(',', ':')), file=output)
        print(json.dumps({'positions': len(positions), 'sources': counts}))


if __name__ == '__main__':
    main()
