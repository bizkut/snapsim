#!/usr/bin/env python3
"""List registered and remaining reference rules; run with reference PYTHONPATH."""
import json
import subprocess
from pathlib import Path

from factories import implemented_cards
from location import implemented_location_ids

root = Path(__file__).resolve().parents[1]
registered = json.loads(subprocess.check_output([root / 'target/release/snapsim-inventory'], text=True))
reference = {'cards': set(implemented_cards()), 'locations': set(implemented_location_ids())}
for kind in ('cards', 'locations'):
    installed = set(registered[kind])
    remaining = sorted(reference[kind] - installed)
    print(f'{kind} implemented ({len(installed)}): ' + ', '.join(sorted(installed)))
    print(f'{kind} remaining ({len(remaining)}): ' + ', '.join(remaining))
