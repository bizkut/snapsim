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
def attribute(card, group, key):
    attributes = card.get('Attributes') or {}
    for entry in attributes.get(group) or []:
        if entry['key'] == key:
            return [item for row in entry['value'] for item in row]
    return []


def factions(card):
    result = attribute(card, 'Factions', 'Faction_Affiliation')
    base = attribute(card, 'Cards', 'Card_Base')
    if not result and base:
        result = attribute(client.get(base[0], {}), 'Factions', 'Faction_Affiliation')
    return result


records = []
for card_id, card in sorted(client.items()):
    if not all(key in card for key in ('Name', 'Cost', 'Power', 'Type')):
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
        'factions': factions(card),
        'faction_cards': attribute(card, 'Cards', 'Card_Faction'),
    })
(root / 'data/client_cards.json').write_text(json.dumps(records, indent=2, ensure_ascii=False) + '\n')
