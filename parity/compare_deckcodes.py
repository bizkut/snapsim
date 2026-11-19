"""Compare clipboard imports and exports with the Python reference.

Run from a temporary cwd with PYTHONPATH pointing to the reference:
    python /path/to/parity/compare_deckcodes.py /path/to/snapsim-deckcodes-parity
"""
import base64
import io
import json
import os
from pathlib import Path
import re
import runpy
import subprocess
import sys

import deckcodes
import deckinputs
from unittest.mock import patch


def b64(text):
    return base64.b64encode(text.encode()).decode()


def main():
    reference = Path(deckcodes.__file__).parent
    examples = runpy.run_path(str(reference / 'tests/test_deckcodes.py'))
    compact = examples['CREATED_POWER_CODE']
    legacy = examples['LEGACY_CODE']
    clipboard = examples['CLIPBOARD_TEXT']
    ids = list(examples['CREATED_POWER_IDS'])
    tokens = base64.b64decode(compact).decode().split(',')
    cases = [
        {'operation': 'parse', 'text': text} for text in (
            compact, compact.rstrip('='), legacy, clipboard,
            ' # comment\n' + compact[:24] + ' \n' + compact[24:],
            '# comment\r' + compact, '# comment\u2028' + compact,
            '', '# no code', '!', 'AA', 'AA===', '////', b64(''),
            b64('{'), b64('{}'), b64('{"Cards":[]}'),
            b64('{"Cards":[],"Cards":[]}'),
            b64('{"Cards":[{"CardDefId":"Yondu","x":1,"x":2}]}'),
            b64(json.dumps({'Cards': [{'CardDefId': x} for x in ids]})),
            b64(json.dumps({'Cards': [{'CardDefId': x} for x in ids[:-1]]})),
            b64(json.dumps({'Cards': [{'CardDefId': x} for x in ids[:-1] + ['NotACard']]})),
            b64(json.dumps({'Cards': [{'CardDefId': x} for x in ids[:-1] + [ids[0]]]})),
            b64(','.join(tokens[:-1] + ['notacard'])),
            b64(','.join(tokens[:-1] + ["bad'card"])),
            b64(','.join(tokens[:-1] + ['café'])),
            b64(','.join(tokens[:-1] + ['bad"\'card'])),
            b64(json.dumps({'Cards': [{'CardDefId': 'AcidArrow'}] +
                                      [{'CardDefId': x} for x in examples['LEGACY_IDS'][1:]]})),
            b64(','.join(tokens[:-1] + [tokens[0]])),
        )
    ]
    cases.extend({'operation': 'encode', 'ids': deck} for deck in (
        ids, list(examples['LEGACY_IDS']), ids[:-1], ids[:-1] + [ids[0]],
        ids[:-1] + ['NotACard'],
    ))
    cases.extend({'operation': 'file', 'text': text, 'label': 'stdin'} for text in (
        '', '# comment\n\n', compact + ' # unsupported cards\n' + legacy,
        '# comment\ninvalid\n' + compact, (reference / 'examples/starter-decks.txt').read_text(),
        '# header\r\n' + legacy + '\r# more\u2028invalid',
    ))
    expected = []
    for case in cases:
        try:
            if case['operation'] == 'file':
                with patch('sys.stdin', io.StringIO(case['text'])):
                    decks, errors = deckinputs.load_decks([Path('-')])
                result = {'decks': [[label, list(deck)] for label, deck, _ in decks], 'errors': errors}
            elif case['operation'] == 'parse':
                result = deckcodes.parse_deck(case['text'])
            else:
                result = deckcodes.encode_deck(case['ids'])
            expected.append({'value': list(result) if isinstance(result, tuple) else result})
        except ValueError as error:
            expected.append({'error': str(error)})
    process = subprocess.run([sys.argv[1]], input=''.join(json.dumps(c) + '\n' for c in cases),
                             text=True, capture_output=True, check=True)
    actual = [json.loads(line) for line in process.stdout.splitlines()]
    assert len(actual) == len(expected), (len(actual), len(expected), process.stderr)
    # Cards with rules here but not in the frozen reference are new content: the reference
    # rejects them as unsupported, so cases whose reference output names one are not compared.
    rust = json.loads(subprocess.check_output([os.environ['SNAPSIM_BIN'], 'coverage']))['cards']
    python = json.loads(subprocess.check_output([sys.executable, str(reference / 'advise.py'), 'coverage']))['cards']
    beyond = sorted(set(rust) - set(python))
    pattern = re.compile(r'\b(' + '|'.join(map(re.escape, beyond)) + r')\b') if beyond else None
    skipped = 0
    for index, (want, got) in enumerate(zip(expected, actual)):
        if pattern and pattern.search(json.dumps(want)):
            skipped += 1
            continue
        assert want == got, (index, cases[index], want, got)
    compared = len(cases) - skipped
    print(f'deck codes: {compared}/{compared} cases matched; {skipped} cases with cards beyond the reference not compared')


if __name__ == '__main__':
    main()
