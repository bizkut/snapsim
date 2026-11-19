"""Compare the Python and Rust deck builders, including cancellation boundaries."""
import importlib.util
import json
import os
from pathlib import Path

import deckbuild
from factories import deck_eligible_ids


def main():
    path = os.environ['SNAPSIM_PY_PATH']
    if Path(path).is_dir():
        path = str(Path(path) / 'snapsim_py.so')
    spec = importlib.util.spec_from_file_location('snapsim_py', path)
    rust = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(rust)
    owned = sorted(deck_eligible_ids())[:13]
    base = owned[:12]
    run = Path('$SNAPSIM_RUNS')
    real_owned = json.loads((run / 'collection.json').read_text())
    real_saved = json.loads((run / 'decks.json').read_text())
    selected = json.loads((run / '20261119-044912-7d8396d1.deck.json').read_text())
    cases = [
        ({'owned': owned, 'seed': 42, 'workers': 1, 'confirm_rollouts': 1}, False),
        ({'owned': owned, 'base': base, 'cards': [owned[12]], 'seed': 42, 'workers': 1, 'confirm_rollouts': 1}, True),
        ({'owned': real_owned, 'saved': real_saved, 'baseline': selected, 'seed': 3, 'workers': 1, 'confirm_rollouts': 1}, False),
    ]
    if os.environ.get('SNAPSIM_PARTNERS_ONLY'):
        cases = [cases[1]]
    if os.environ.get('SNAPSIM_NEW_PARTNER_ONLY'):
        cases = [({'owned': sorted(deck_eligible_ids())[:14], 'base': base, 'cards': [owned[12]], 'seed': 42, 'workers': 1, 'confirm_rollouts': 1}, True)]
    total = 0
    for args, trying in ([] if os.environ.get('SNAPSIM_COMPLETE_ONLY') or os.environ.get('SNAPSIM_WORKERS_ONLY') or os.environ.get('SNAPSIM_TRY_COMPLETE_ONLY') else cases):
        for cutoff in ((193,) if os.environ.get('SNAPSIM_NEW_PARTNER_ONLY') else (193, 200) if os.environ.get('SNAPSIM_PARTNERS_ONLY') else (0, 1, 2, 4)):
            py_events, rs_events = [], []
            opts = dict(args)
            if trying:
                result_py = deckbuild.try_cards(opts.pop('base'), opts.pop('cards'), **opts, progress=py_events.append, cancel=lambda: len(py_events) >= cutoff)
            else:
                result_py = deckbuild.build_decks(**opts, progress=py_events.append, cancel=lambda: len(py_events) >= cutoff)
            result_rs = json.loads(rust.deckbuild(json.dumps(args), rs_events.append, lambda: len(rs_events) >= cutoff))
            for result in (result_py, result_rs):
                result.pop('elapsed_seconds', None)
                result['config'].pop('engine', None)
            for event in py_events + rs_events:
                event.pop('elapsed', None)
            assert result_py == result_rs, (trying, cutoff, result_py, result_rs)
            assert py_events == rs_events, (trying, cutoff, py_events, rs_events)
            total += 1
    if os.environ.get('SNAPSIM_COMPLETE') or os.environ.get('SNAPSIM_COMPLETE_ONLY'):
        opts = {'owned': base, 'seed': 42, 'workers': 1, 'confirm_rollouts': 1}
        expected = deckbuild.build_decks(**opts)
        actual = json.loads(rust.deckbuild(json.dumps(opts)))
        for result in (expected, actual):
            result.pop('elapsed_seconds', None)
            result['config'].pop('engine', None)
        assert expected == actual, (expected, actual)
        total += 1
    if os.environ.get('SNAPSIM_TRY_COMPLETE_ONLY'):
        opts = {'owned': owned, 'seed': 42, 'workers': 1, 'confirm_rollouts': 1}
        expected = deckbuild.try_cards(base, [owned[12]], **opts)
        actual = json.loads(rust.deckbuild(json.dumps(dict(opts, base=base, cards=[owned[12]]))))
        for result in (expected, actual):
            result.pop('elapsed_seconds', None)
            result['config'].pop('engine', None)
        assert expected == actual, (expected, actual)
        total += 1
    if os.environ.get('SNAPSIM_WORKERS_ONLY'):
        results = []
        for workers in (1, 2):
            events = []
            opts = {'owned': owned, 'seed': 42, 'workers': workers, 'confirm_rollouts': 1}
            result = json.loads(rust.deckbuild(json.dumps(opts), events.append, lambda: len(events) >= 4))
            result.pop('elapsed_seconds', None)
            result['config'].pop('workers', None)
            for event in events:
                event.pop('elapsed', None)
            results.append((result, events))
        assert results[0] == results[1], results
        total += 1
    print(f'deckbuild parity: {total} cancellation cases passed')


if __name__ == '__main__':
    main()
