#!/usr/bin/env python3
"""Measure complete games from the same restricted corpus as Rust."""
import argparse
import json
import random
import time
from pathlib import Path

from advisor import MarginPlayer, SpreadPlayer
from factories import resolve_deck
from game import Game
from position import quiet

parser = argparse.ArgumentParser()
parser.add_argument('config', type=Path)
parser.add_argument('--games', type=int, default=5000)
parser.add_argument('--seed', type=int, default=20261119)
args = parser.parse_args()
config = json.loads(args.config.read_text())
policy = {'spread': SpreadPlayer, 'margin': MarginPlayer}[config['policy']]()
start = time.perf_counter()
with quiet():
    for seed in range(args.seed, args.seed + args.games):
        deals = random.Random(seed)
        decks = tuple(resolve_deck(tuple(deals.sample(config['cards'], 12))) for _ in range(2))
        random.seed(seed)
        Game(decks=decks, policies=(policy, policy), location_ids=config['locations']).play_game(record=False)
seconds = time.perf_counter() - start
print(json.dumps({'engine': 'python', 'policy': config['policy'], 'games': args.games, 'seed': args.seed, 'seconds': seconds, 'games_per_second': args.games / seconds}))
