#!/usr/bin/env python3
"""Single-process, seeded reference workloads; run from a temporary cwd."""
import argparse
import json
import random
import time
from pathlib import Path

from advisor import MarginPlayer, SpreadPlayer, advise
from factories import resolve_deck
from game import Game
from position import (Position, quiet, recruit_card_pool, recruit_card_weights,
                      recruit_location_pool, sample_deck)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--games', type=int, default=500)
    parser.add_argument('--rollouts', type=int, default=1000)
    parser.add_argument('--seed', type=int, default=20261119)
    args = parser.parse_args()
    pool, weights = recruit_card_pool(), recruit_card_weights()
    locations = recruit_location_pool()
    for name, policy in [('spread', SpreadPlayer()), ('margin', MarginPlayer())]:
        start = time.perf_counter()
        with quiet():
            for seed in range(args.seed, args.seed + args.games):
                deals = random.Random(seed)
                decks = tuple(resolve_deck(tuple(sample_deck(deals, pool, weights=weights))) for _ in range(2))
                random.seed(seed)
                Game(decks=decks, policies=(policy, policy), location_ids=locations).play_game(record=False)
        seconds = time.perf_counter() - start
        print(json.dumps({'workload': name, 'count': args.games, 'seconds': seconds, 'per_second': args.games / seconds}), flush=True)
    position = Position.from_dict(json.loads(Path(__file__).with_name('turn3.json').read_text()))
    start = time.perf_counter()
    result = advise(position, rollouts=args.rollouts, seed=args.seed, workers=1)
    seconds = time.perf_counter() - start
    print(json.dumps({'workload': 'advise', 'count': result['rollouts'], 'status': result['status'], 'unsupported': result['unsupported'], 'seconds': seconds, 'per_second': result['rollouts'] / seconds}), flush=True)


if __name__ == '__main__':
    main()
