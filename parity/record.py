#!/usr/bin/env python3
"""Record seeded games against the Python reference, from a restricted corpus."""
import argparse
import json
import random
import sys
from pathlib import Path

from advisor import MarginPlayer, SpreadPlayer
from factories import resolve_deck
from game import Game
from position import quiet
from turn import Turn
from winconditions import WinConditions


def record(seed, config):
    deals = random.Random(seed)
    deck_ids = [deals.sample(config['cards'], 12) for _ in range(2)]
    decks = tuple(resolve_deck(tuple(deck)) for deck in deck_ids)
    policy = {'spread': SpreadPlayer, 'margin': MarginPlayer}[config['policy']]()
    random.seed(seed)
    game = Game(decks=decks, policies=(policy, policy), location_ids=config['locations'])
    result = {'seed': seed, 'decks': deck_ids,
              'locations': [location.card_def_id for location in game.locations], 'turns': []}
    while True:
        turn = game.turn
        turn.start_turn()
        first = turn.select_reveal_priority()
        plans = [turn._plan_plays(seat, game.policies[seat]) for seat in range(2)]
        plays = [[{'card': card.card_def_id, 'lane': lane} for card, lane in plan] for plan in plans]
        turn.commit_plays(plans)
        turn.reveal_plays(first)
        turn.end_of_turn()
        result['turns'].append({'turn': turn.turn_id, 'priority': first, 'plays': plays,
                                'powers': [[loc.calculate_total_power(seat) for seat in range(2)] for loc in game.locations]})
        if turn.turn_id >= game.total_turns:
            break
        game.turn = Turn(turn.turn_id + 1, game)
    game.turn.finish_reveals()
    result['winner'] = WinConditions(game.locations, game.players).determine_winner()
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('config', type=Path)
    parser.add_argument('--games', type=int, default=1000)
    parser.add_argument('--seed', type=int, default=0)
    args = parser.parse_args()
    config = json.loads(args.config.read_text())
    with quiet():
        for seed in range(args.seed, args.seed + args.games):
            print(json.dumps(record(seed, config), separators=(',', ':')), file=sys.stdout)


if __name__ == '__main__':
    main()
