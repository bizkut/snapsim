#!/usr/bin/env python3
"""Generate missing client card definitions without changing the website catalog."""
import argparse
import json
from pathlib import Path

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument('carddefs', type=Path, help='client rules/carddefs.json')
args = parser.parse_args()
client = json.loads(args.carddefs.read_text())
website = {card['carddefid'] for card in json.loads((root / 'data/cards.json').read_text())}
records = []
for card_id, card in sorted(client.items()):
    if card_id in website or not all(key in card for key in ('Name', 'Cost', 'Power', 'Type')):
        continue
    records.append({
        'carddefid': card_id,
        'name': card['Name'],
        'cost': card['Cost'],
        'power': card['Power'],
        'ability': card.get('Description') or '',
        'status': 'client',
        'source': 'client',
        'type': card['Type'],
        'random_pool_excluded': bool(card.get('ExcludedFromInGameRandomPool', False)),
    })
(root / 'data/client_cards.json').write_text(json.dumps(records, indent=2, ensure_ascii=False) + '\n')
