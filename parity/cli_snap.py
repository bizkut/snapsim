"""Compare CLI SNAP observation handling with a recorded selected deck."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

from deckcodes import encode_deck
from snap_adapter import position_from_snap

ROOT = Path(__file__).resolve().parents[1]
REFERENCE = Path(os.environ.get('SNAPSIM_REFERENCE') or sys.exit('Set SNAPSIM_REFERENCE to the frozen Python checkout'))
RUN = Path(os.environ.get('SNAPSIM_RUNS') or sys.exit('Set SNAPSIM_RUNS to the recordings directory'))
BINARY = Path(os.environ.get('SNAPSIM_BIN', ROOT / 'target-RustDeckbuild/release/snapsim'))


def main():
    deck_path = next((path for path in sorted(RUN.glob('*.deck.json'))
                      if path.with_name(path.name.removesuffix('.deck.json') + '.jsonl').is_file()), None)
    if deck_path is None:
        raise SystemExit('SNAPSIM_RUNS must contain a recording and its matching .deck.json')
    deck = json.loads(deck_path.read_text())['decks'][0]['card_ids']
    with deck_path.with_name(deck_path.name.removesuffix('.deck.json') + '.jsonl').open() as stream:
        for line in stream:
            row = json.loads(line)
            obs = row.get('envelope', {}).get('observation') if 'envelope' in row else row.get('observation', row)
            try:
                position_from_snap(obs, deck=deck)
                break
            except (ValueError, KeyError, TypeError, IndexError):
                pass
        else:
            raise RuntimeError('no planning observation')
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / 'observation.json'
        path.write_text(json.dumps(obs))
        for choice in (['--snap-deck', str(deck_path)], ['--deck-code', encode_deck(deck)]):
            args = ['position', '--snap', '--file', str(path), '--rollouts', '2', '--seed', '31', *choice]
            expected = json.loads(subprocess.check_output([sys.executable, str(REFERENCE / 'advise.py'), *args], text=True))
            actual = json.loads(subprocess.check_output([str(BINARY), *args, '--plays-only'], text=True))
            assert expected == actual, (args, expected, actual)
    print('CLI SNAP: 2 selected-deck cases passed')


if __name__ == '__main__':
    main()
