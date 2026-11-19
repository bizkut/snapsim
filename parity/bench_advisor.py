#!/usr/bin/env python3
"""Back-to-back fixed-budget and one-second advisor measurements."""
import argparse
import json
import os
from pathlib import Path
from time import perf_counter

import advisor
from position import Position, quiet


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--rollouts', type=int, default=2000)
    parser.add_argument('--position', type=Path, default=Path(__file__).with_name('turn3.json'))
    args = parser.parse_args()
    position = Position.from_dict(json.loads(args.position.read_text()))
    rows = []
    with quiet():
        for workers in (1, 4):
            for engine in ('python', 'rust'):
                advisor.advise(position, rollouts=16, seed=42, workers=workers, engine=engine)
                start_load = os.getloadavg()
                start = perf_counter()
                fixed = advisor.advise(position, rollouts=args.rollouts, seed=42, workers=workers, engine=engine)
                elapsed = perf_counter() - start
                start_one = perf_counter()
                limited = advisor.advise(position, rollouts=10_000_000, seed=42, workers=workers,
                                         engine=engine, time_limit=1.0)
                one_elapsed = perf_counter() - start_one
                rows.append(dict(engine=engine, workers=workers, fixed_rollouts=fixed['rollouts'],
                                 fixed_seconds=elapsed, rollouts_per_second=fixed['rollouts']/elapsed,
                                 one_second_rollouts=limited['rollouts'], one_second_elapsed=one_elapsed,
                                 load_before=start_load, load_after=os.getloadavg()))
    print(json.dumps(rows, indent=2))


if __name__ == '__main__':
    main()
