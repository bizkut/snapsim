import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import client_diff


class ClientDiffTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.old = Path(self.tmp.name) / 'old'
        self.new = Path(self.tmp.name) / 'new'
        self.row = {'virtualAddress': '0x100000', 'group': 'Assembly/Class',
                    'name': 'method', 'signature': 'void method()'}
        self.end = {'virtualAddress': '0x100010', 'group': 'Assembly/Class',
                    'name': 'next', 'signature': 'void next()'}
        for path in (self.old, self.new):
            (path / 'rules').mkdir(parents=True)
            (path / 'libil2cpp.so').write_bytes(b'old' if path == self.old else b'new')
            self.save(path, 'metadata.json', {'addressMap': {'methodDefinitions': [self.row, self.end]}})
            self.save(path, 'carddefs.json', {'Card': {'Cost': 1, 'Power': 2}})
            self.save(path, 'defs.json', {'Card': {'Abilities': [{'method': 'A', 'rva': '0x100000'}]}})

    def save(self, path, name, value):
        (path / 'rules' / name).write_text(json.dumps(value))

    def test_renamed_method(self):
        renamed = dict(self.row, name='renamed')
        self.save(self.new, 'metadata.json', {'addressMap': {'methodDefinitions': [renamed, self.end]}})
        items = client_diff.method_diff(self.old, self.new, {0x100000}, 'objdump')
        self.assertIn('missing, renamed', items[0]['reason'])
        self.assertEqual(items[0]['verdict'], 'review')

    def test_changed_body(self):
        with patch.object(client_diff, 'disassemble', side_effect=[['mov w0, #1'], ['mov w0, #2']]):
            items = client_diff.method_diff(self.old, self.new, {0x100000}, 'objdump')
        self.assertIn('instruction stream changed', items[0]['reason'])

    def test_card_cost_and_delegate(self):
        self.save(self.new, 'carddefs.json', {'Card': {'Cost': 2, 'Power': 2}})
        self.save(self.new, 'defs.json', {'Card': {'Abilities': [{'method': 'B', 'rva': '0x200000'}]}})
        reasons = [x['reason'] for x in client_diff.definition_diff(self.old, self.new)]
        self.assertTrue(any('Cost' in r for r in reasons))
        self.assertTrue(any('Abilities' in r for r in reasons))

    def test_relocation_does_not_change_body(self):
        a = ' 100000: adrp x2, 0x120000\n 100004: ldr x3, [x2, #0x150]\n 100008: bl 0x102000 <target>\n'
        b = ' 200000: adrp x2, 0x240000\n 200004: ldr x3, [x2, #0x1a0]\n 200008: bl 0x204000 <target>\n'
        self.assertEqual(client_diff.normalize_disassembly(a), client_diff.normalize_disassembly(b))
        self.assertNotEqual(client_diff.normalize_disassembly(' 10: mov w0, #1'),
                            client_diff.normalize_disassembly(' 10: mov w0, #2'))

    def test_self_diff_and_ambiguous(self):
        self.assertEqual(client_diff.method_diff(self.old, self.old, {0x100000}, 'objdump'), [])
        self.assertEqual(client_diff.definition_diff(self.old, self.old), [])
        self.save(self.new, 'metadata.json', {'addressMap': {'methodDefinitions': [self.row, self.row, self.end]}})
        items = client_diff.method_diff(self.old, self.new, {0x100000}, 'objdump')
        self.assertIn('ambiguous', items[0]['reason'])

    def test_rva_only_change_is_ignored(self):
        self.save(self.new, 'defs.json', {'Card': {'Abilities': [{'method': 'A', 'rva': '0x200000'}]}})
        self.assertEqual(client_diff.definition_diff(self.old, self.new), [])


if __name__ == '__main__':
    unittest.main()
