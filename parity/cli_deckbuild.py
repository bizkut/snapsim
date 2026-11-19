"""Check deckbuild JSON-lines and interrupt handling against the Python CLI."""
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
REFERENCE = Path('$SNAPSIM_REFERENCE/deckbuild.py')
BINARY = Path(os.environ.get('SNAPSIM_BIN', ROOT / 'target-RustDeckbuild/release/snapsim'))
CARDS = ['AntMan', 'Hawkeye', 'SquirrelGirl', 'Nightcrawler', 'Angela', 'Sentinel', 'Bishop', 'Wolfsbane', 'JessicaJones', 'KaZar', 'IronMan', 'Ultron']


def run(command):
    proc = subprocess.Popen(command, text=True, stdout=subprocess.PIPE)
    events = []
    for line in proc.stdout:
        item = json.loads(line)
        events.append(item)
        if item.get('type') == 'progress':
            proc.send_signal(signal.SIGINT)
            break
    for line in proc.stdout:
        events.append(json.loads(line))
    assert proc.wait() == 0
    assert events[-1]['type'] == 'result' and events[-1]['status'] == 'cancelled'
    for event in events:
        event.pop('elapsed', None)
        event.pop('elapsed_seconds', None)
    return events


def main():
    with tempfile.TemporaryDirectory() as directory:
        collection = Path(directory) / 'collection.json'
        collection.write_text(json.dumps({'cards': CARDS}))
        args = ['--collection', str(collection), '--workers', '1', '--seed', '0', '--confirm-rollouts', '1', '--progress']
        expected = run([sys.executable, str(REFERENCE), *args, '--engine', 'rust'])
        actual = run([str(BINARY), 'deckbuild', *args])
        assert expected == actual, (expected, actual)
        print('CLI deckbuild: 1 interrupt/progress case passed')


if __name__ == '__main__':
    main()
