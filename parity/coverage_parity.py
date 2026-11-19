#!/usr/bin/env python3
"""Compare Rust coverage, priors and strict unsupported advice on recorded positions."""
import argparse
import copy
import json
import subprocess

import advisor
from position import Position, coverage, sampling_priors, quiet
from position_record import edges


def extra_edges():
    base = next(edges())
    def lane(id=None, mine=None, theirs=None, **kwargs):
        return dict(id=id, mine=mine or [], theirs=theirs or [], **kwargs)
    variants = [
        dict(hand=[{'id':'AbsorbingMan'}], locations=[lane('UnknownLocation'), lane(), lane()]),
        dict(locations=[lane(theirs_face_down=2), lane(), lane()]),
        dict(turn=4, locations=[lane('Lemuria', revealed_turn=3, theirs_face_down=2), lane(), lane()]),
        dict(locations=[lane('OttosLab'), lane(), lane()]),
        dict(deck_count=8, opponent=dict(hand=7, deck=10, pool=['Hulk'])),
        dict(locations=[lane(mine=[{'id':'Hawkeye','played_turn':2}, {'id':'Cosmo','played_turn':2}]), lane(), lane()]),
        dict(locations=[lane(mine=[{'id':'Hawkeye','played_turn':2}, {'id':'JoaquinTorres','played_turn':1}, {'id':'Onslaught','played_turn':2}]), lane(), lane()]),
        dict(locations=[lane(mine=[{'id':'JessicaJones','played_turn':2}], theirs=[{'id':'Cosmo'}]), lane(), lane()]),
        dict(locations=[lane(mine=[{'id':'Hulk', 'revealed':False}]), lane(), lane()]),
    ]
    for changes in variants:
        raw = copy.deepcopy(base); raw.update(changes); yield raw


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary')
    parser.add_argument('corpora', nargs='+')
    args = parser.parse_args()
    positions = [Position.from_dict(raw) for raw in extra_edges()]
    for path in args.corpora:
        with open(path) as records:
            positions.extend(Position.from_dict(json.loads(line)['position']) for line in records)
    payload = ''.join(json.dumps(p.to_dict(), separators=(',', ':')) + '\n' for p in positions)
    result = subprocess.run([args.binary], input=payload, text=True, capture_output=True, check=True)
    actual = [json.loads(line) for line in result.stdout.splitlines()]
    assert len(actual) == len(positions), (len(actual), len(positions))
    with quiet():
        for index, (position, got) in enumerate(zip(positions, actual)):
            cov = coverage(position)
            strict = advisor.advise(position, rollouts=1, max_plans=1, workers=1) if any(cov.values()) else None
            approximate = advisor.advise(position, rollouts=1, max_plans=1, workers=1, approximate=True) if index < 9 else None
            expected = dict(coverage=cov, priors=sampling_priors(position), strict=strict, approximate=approximate)
            if got != expected:
                raise AssertionError(f'position {index}: {position.to_dict()}\nexpected {expected}\nactual {got}')
    print(f'{len(positions)} positions matched coverage, priors and strict advice; 9 edge positions matched approximate advice')

if __name__ == '__main__':
    main()
