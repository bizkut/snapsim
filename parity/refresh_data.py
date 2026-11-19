#!/usr/bin/env python3
"""Copy reference catalogs without modifying the reference worktree."""
import argparse
import shutil
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('reference', type=Path, help='path to the Python simulator worktree')
args = parser.parse_args()
destination = Path(__file__).resolve().parents[1] / 'data'
for name in ('cards.json', 'locations.json'):
    shutil.copyfile(args.reference / 'data' / name, destination / name)
