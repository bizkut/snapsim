"""Time an interrupted fixed-seed screen across all three front ends."""
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
REFERENCE = Path('$SNAPSIM_REFERENCE')
BINARY = Path(os.environ.get('SNAPSIM_BIN', ROOT / 'target-RustDeckbuild/release/snapsim'))
CARDS = ['AntMan', 'Hawkeye', 'SquirrelGirl', 'Nightcrawler', 'Angela', 'Sentinel', 'Bishop', 'Wolfsbane', 'JessicaJones', 'KaZar', 'IronMan', 'Ultron', 'Medusa']


def run(command):
    started = time.monotonic()
    proc = subprocess.Popen(command, stdout=subprocess.PIPE, text=True)
    lines = 0
    for line in proc.stdout:
        event = json.loads(line)
        if event.get('type') == 'progress':
            lines += 1
            if lines == 2:
                proc.send_signal(signal.SIGINT)
        if event.get('type') == 'result':
            result = event
    if proc.wait() != 0:
        raise RuntimeError(command)
    return time.monotonic() - started, result['games_played']


def main():
    with tempfile.TemporaryDirectory() as directory:
        collection = Path(directory) / 'collection.json'
        collection.write_text(json.dumps({'cards': CARDS}))
        args = ['--collection', str(collection), '--workers', '1', '--seed', '42', '--confirm-rollouts', '1', '--progress']
        for name, cmd in [('Python', [sys.executable, str(REFERENCE / 'deckbuild.py'), *args, '--engine', 'python']), ('Python/Rust', [sys.executable, str(REFERENCE / 'deckbuild.py'), *args, '--engine', 'rust']), ('Rust CLI', [str(BINARY), 'deckbuild', *args])]:
            print(name, 'load', os.getloadavg(), flush=True)
            seconds, games = run(cmd)
            print(f'{name}: {seconds:.3f}s, {games} games', flush=True)
        compare = ['compare', '--mode', 'recruit', '--games', '3', '--workers', '1', '--rollouts', '4', '--seed', '42']
        for name, cmd in [('Python compare', [sys.executable, str(REFERENCE / 'advise.py'), *compare]), ('Rust compare', [str(BINARY), *compare])]:
            print(name, 'load', os.getloadavg(), flush=True)
            started = time.monotonic()
            subprocess.run(cmd, check=True, stdout=subprocess.DEVNULL)
            print(f'{name}: {time.monotonic() - started:.3f}s, 6 games', flush=True)


if __name__ == '__main__':
    main()
