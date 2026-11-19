#!/usr/bin/env python3
"""Generate a reproducible Python-oracle validation corpus from parity outputs and SNAP recordings."""
import argparse
import copy
import json
import random
from pathlib import Path

from position import Position
from record_advisor import recorded_positions


def outside_rust_range(value):
    def check(v, low, high):
        return type(v) is int and not low <= v <= high

    if not isinstance(value, dict):
        return False
    fields = [(value, 'turn', 0, 255), (value, 'total_turns', 0, 255),
              (value, 'energy', -(2**31), 2**31-1), (value, 'pending_power', -(2**31), 2**31-1),
              (value, 'deck_count', 0, 2**64-1)]
    opp = value.get('opponent')
    if isinstance(opp, dict):
        fields += [(opp, k, lo, hi) for k, lo, hi in [
            ('hand', 0, 2**64-1), ('deck', 0, 2**64-1), ('energy', -(2**31), 2**31-1),
            ('pending_power', -(2**31), 2**31-1)]]
    cards = value.get('hand')
    cards = list(cards) if isinstance(cards, list) else []
    lanes = value.get('locations')
    if isinstance(lanes, list):
        for lane in lanes:
            if isinstance(lane, dict):
                fields += [(lane, 'theirs_face_down', 0, 2**64-1), (lane, 'revealed_turn', 0, 255)]
                for key in ('mine', 'theirs'):
                    if isinstance(lane.get(key), list):
                        cards.extend(lane[key])
    for card in cards:
        if isinstance(card, dict):
            fields += [(card, k, lo, hi) for k, lo, hi in [
                ('cost', -(2**31), 2**31-1), ('power', -(2**31), 2**31-1), ('played_turn', 0, 255)]]
    return any(check(obj.get(key), lo, hi) for obj, key, lo, hi in fields)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--corpus', type=Path, action='append', required=True,
                        help='position_record.py or record_advisor.py JSONL output; supply both')
    parser.add_argument('--runs', type=Path, required=True)
    args = parser.parse_args()
    positions = []
    for path in args.corpus:
        for line in path.read_text().splitlines():
            record = json.loads(line)
            if 'position' in record:
                positions.append(record['position'])
    positions.extend(position.to_dict() for position, _ in recorded_positions(args.runs))
    if not positions:
        raise ValueError('no positions found')
    rng = random.Random(527)
    values = [None, True, False, 1.5, -1, 0, 1, 7, 8, 255, 256, 2**31, 2**64, -(2**70),
              '', 'bogus', [], {}, [None], ['MissingCard'], [True, False], [False]*8]
    paths = [('turn',), ('total_turns',), ('energy',), ('hand',), ('deck',), ('locations',),
             ('priority',), ('mode',), ('pending_power',), ('deck_count',), ('opponent',),
             ('opponent','hand'), ('opponent','deck'), ('opponent','energy'), ('opponent','seen'),
             ('opponent','deck_list'), ('opponent','pool'), ('opponent','pending_power'), ('opponent','prior'),
             ('locations',0,'id'), ('locations',0,'mine'), ('locations',0,'theirs'),
             ('locations',0,'theirs_face_down'), ('locations',0,'revealed_turn'), ('locations',0,'ottos_lab_spent')]
    mutations = []
    for base in positions:
        mutations.append(base)
    for i in range(4000):
        case = copy.deepcopy(rng.choice(positions))
        path = rng.choice(paths)
        parent = case
        try:
            for key in path[:-1]:
                parent = parent[key]
            if i % 11 == 0:
                parent.pop(path[-1], None)
            elif i % 13 == 0:
                parent['extra_validation_key'] = {'nested': [True, 42]}
            else:
                parent[path[-1]] = copy.deepcopy(rng.choice(values))
            if i % 5 == 0 and isinstance(case.get('hand'), list):
                case['hand'] = [{'id': rng.choice(['Hulk', 'NoSuchCard', '', 5]),
                                 rng.choice(['cost','power','played_turn','revealed']): rng.choice(values)}]
        except (KeyError, IndexError, TypeError, AttributeError):
            case = rng.choice(values)
        mutations.append(case)
    counts = [0, 0, 0]
    with args.output.open('w') as output:
        for case in mutations:
            try:
                Position.from_dict(case)
                accepted = True
            except (ValueError, TypeError, KeyError):
                accepted = False
            out_of_range = outside_rust_range(case)
            counts[2 if out_of_range else int(accepted)] += 1
            output.write(json.dumps({'position': case, 'accepted': accepted, 'out_of_range': out_of_range}, separators=(',', ':')) + '\n')
    print(f'in-range rejected {counts[0]}, accepted {counts[1]}; out-of-range {counts[2]}')


if __name__ == '__main__':
    main()
