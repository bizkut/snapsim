#!/usr/bin/env python3
"""Generate differential position cases from SearchAI, SNAP frames and edge cases."""
import argparse
import json
import random
import sys
from collections import Counter
from pathlib import Path

import advisor
from advisor import SearchAI, SpreadPlayer
from factories import deck_eligible_ids, resolve_deck
from game import Game
from position import Position, build_game, quiet, recruit_card_pool, recruit_location_pool
from snap_adapter import DeckWatch, LocationWatch, position_from_snap
from turn import Turn


def state(game):
    def card(c):
        watches = [[turn, lane] for turn, lanes in getattr(c, '_watched_lanes', {}).items() for lane in lanes]
        return {'id': c.card_def_id, 'owner': c.owner_id, 'cost': c.energy_cost, 'power': c.power,
                'revealed': c.revealed, 'turn_played': c.turn_played, 'watches': watches}
    def plays(groups):
        return [[[c.card_def_id, lane] for c, lane in groups[i]] for i in range(2)]
    return {'turn': game.turn.turn_id, 'total_turns': game.total_turns, 'priority': game.turn.reveal_priority,
            'players': [{'energy': p.energy, 'pending_power': p.next_play_power,
                         'hand': [card(c) for c in p.hand], 'deck': [c.card_def_id for c in p.deck]}
                        for p in game.players],
            'locations': [{'id': l.card_def_id, 'revealed': l.revealed, 'revealed_turn': l.revealed_turn,
                           'spent': getattr(l, 'spent', None), 'cards': [card(c) for c in l.cards]}
                          for l in game.locations],
            'pending_reveals': plays(game.turn._pending_reveals), 'played': plays(game.turn._plays)}


def edges():
    def lane(id=None, mine=None, theirs=None, **extra):
        return dict(id=id, mine=mine or [], theirs=theirs or [], **extra)
    base = dict(turn=3, energy=3, hand=[{'id': 'Hulk', 'cost': 6}], deck=['Hawkeye', 'Cyclops', 'IronMan'],
                locations=[lane('KamarTaj', [{'id': 'Hawkeye', 'played_turn': 2}]), lane('OttosLab', ottos_lab_spent=[True, False]), lane()],
                opponent=dict(hand=2, deck=3, seen=[]), priority='me', mode='recruit')
    yield base
    yield dict(base, turn=4, locations=[lane('Lemuria', revealed_turn=3, theirs_face_down=2), lane('Xandar'), lane('OttosLab', ottos_lab_spent=[False, True])])
    yield dict(base, deck_count=5, locations=[lane('CosmoFake'), lane(), lane()])
    yield dict(base, deck_count=1, opponent=dict(hand=2, deck=2, seen=['Hulk'], pool=['Hulk', 'Cyclops']))
    yield dict(base, mode=None, opponent=dict(hand=2, deck=3, deck_list=['Hulk', 'Cyclops', 'IronMan', 'Hawkeye', 'Medusa', 'Cosmo']), locations=[lane('KamarTaj', [{'id': 'Hawkeye', 'played_turn': 2}], [{'id': 'Cosmo', 'played_turn': 1}]), lane(), lane()])
    yield dict(base, hand=[{'id': 'AbsorbingMan', 'cost': 4, 'power': 3}])
    yield dict(base, opponent=dict(hand=0, deck=0))


def search_positions(games):
    captured = []
    current = [None, None]
    original = advisor.advise
    spread = SpreadPlayer()
    def capture(position, **kwargs):
        captured.append(position.to_dict())
        turn, seat = current
        plays = spread.plan_turn(turn, seat)
        hand = turn.game.players[seat].hand
        return {'plans': [{'plays': [{'hand_index': hand.index(c), 'location': lane} for c, lane in plays]}]}
    advisor.advise = capture
    try:
        for mode in ('recruit', None):
            pool = list(recruit_card_pool()) if mode else list(deck_eligible_ids())
            for seed in range(games):
                rng = random.Random(seed)
                decks = tuple(resolve_deck(tuple(rng.sample(pool, 12))) for _ in range(2))
                random.seed(seed)
                game = Game(decks=decks, location_ids=recruit_location_pool() if mode else None)
                controller = SearchAI(mode=mode)
                while True:
                    turn = game.turn
                    turn.start_turn()
                    first = turn.select_reveal_priority()
                    plans = []
                    for seat in range(2):
                        current[:] = [turn, seat]
                        plans.append(controller.plan_turn(turn, seat))
                    turn.commit_plays(plans); turn.reveal_plays(first); turn.end_of_turn()
                    if turn.turn_id >= game.total_turns: break
                    game.turn = Turn(turn.turn_id + 1, game)
    finally:
        advisor.advise = original
    return captured


def recordings(directory):
    for path in sorted(Path(directory).glob('*.jsonl')):
        loc = LocationWatch(); deck = DeckWatch()
        with path.open() as stream:
            for line in stream:
                try:
                    record = json.loads(line); obs = record.get('envelope', {}).get('observation')
                    if not obs: continue
                    turns = loc.update(obs); added = deck.update(obs)
                    yield position_from_snap(obs, revealed_turns=turns, deck_added=added)
                except (ValueError, KeyError, TypeError):
                    continue


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--seeds', type=int, default=100)
    parser.add_argument('--games', type=int, default=2)
    parser.add_argument('--recordings')
    args = parser.parse_args()
    with quiet():
        positions = [('edge', raw) for raw in edges()]
        positions.extend(('search', raw) for raw in search_positions(args.games))
        if args.recordings:
            positions.extend(('recording', raw) for raw in recordings(args.recordings))
        seen = set()
        counts = Counter()
        rejected = Counter()
        for source, raw in positions:
            try:
                p = Position.from_dict(raw)
                key = json.dumps(p.to_dict(), sort_keys=True)
                if key in seen:
                    continue
                # Check materialization before writing any cases for this position.
                build_game(p, random.Random(0))
                seen.add(key)
                counts[source] += 1
                for seed in range(args.seeds):
                    game = build_game(p, random.Random(seed))
                    print(json.dumps({'position': p.to_dict(), 'seed': seed, 'state': state(game)}, separators=(',', ':')))
            except ValueError as error:
                rejected[(source, str(error))] += 1
        print(f'positions by source: {dict(counts)}; pairs: {sum(counts.values()) * args.seeds}', file=sys.stderr)
        for (source, reason), count in rejected.items():
            print(f'rejected {count} {source} positions: {reason}', file=sys.stderr)

if __name__ == '__main__': main()
