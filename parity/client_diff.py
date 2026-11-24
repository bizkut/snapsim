#!/usr/bin/env python3
"""Create a review ledger from two locally captured client corpora."""
import argparse
import bisect
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
HEX = re.compile(r"0x[0-9a-f]{6,8}")
INSN = re.compile(r"^\s*[0-9a-fA-F]+:\s+([a-zA-Z][\w.]*)\s*(.*?)\s*$")


def cited_rvas():
    proc = subprocess.run(['git', 'grep', '-ohE', r'0x[0-9a-f]{6,8}', '--',
                           'crates', 'docs', 'README.md', 'parity'], cwd=ROOT,
                          text=True, capture_output=True)
    if proc.returncode not in (0, 1):
        raise RuntimeError(f'git grep failed: {proc.stderr.strip()}')
    return {int(x, 16) for x in HEX.findall(proc.stdout)}


def load_json(path):
    with path.open(encoding='utf-8') as f:
        return json.load(f)


def metadata(client):
    rows = load_json(client / 'rules' / 'metadata.json')['addressMap']['methodDefinitions']
    by_address, by_key = {}, {}
    for row in rows:
        address = int(row['virtualAddress'], 16)
        key = (row['group'], row['name'], row['signature'])
        by_address.setdefault(address, []).append(row)
        by_key.setdefault(key, []).append(row)
    return by_address, by_key, sorted(by_address)


def normalize_disassembly(text):
    """Mask relocation-sensitive operands, but retain ordinary immediates and offsets.

    PC-relative branch/ADR targets and literal loads are masked. An offset based on
    a register last written by ADRP is also masked; other memory offsets are not.
    This is a heuristic, not proof of equivalence (especially for literal contents).
    """
    result, page_regs = [], set()
    for line in text.splitlines():
        match = INSN.match(line)
        if not match:
            continue
        op, args = match.groups()
        op = op.lower()
        args = re.sub(r'\s*<[^>]*>', '', args).strip().lower()
        if op in ('adr', 'adrp'):
            args = re.sub(r',\s*(?:0x)?[0-9a-f]+(?:\s*//.*)?$', ', <address>', args)
        elif op in ('b', 'bl', 'cbz', 'cbnz', 'tbz', 'tbnz') or op.startswith('b.'):
            args = re.sub(r'(?:0x)?[0-9a-f]+(?:\s*//.*)?$', '<target>', args)
        elif op.startswith(('ldr', 'ldrsw', 'prfm')):
            args = re.sub(r',\s*(?:0x)?[0-9a-f]+(?:\s*//.*)?$', ', <literal>', args)
        for reg in page_regs:
            args = re.sub(r'(\[' + reg + r',\s*)#(?:0x[0-9a-f]+|[0-9]+)(\])', r'\1<offset>\2', args)
        args = re.sub(r'\s*//.*$', '', args)
        result.append(op + ' ' + re.sub(r'\s+', ' ', args))
        dest = re.match(r'(x\d+|w\d+)\s*,', args)
        if dest and not op.startswith(('st', 'cmp', 'cmn', 'tst', 'cb', 'tb', 'b.')):
            register = 'x' + dest.group(1)[1:]
            page_regs.discard(register)
            if op == 'adrp':
                page_regs.add(register)
    return result


def disassemble(objdump, binary, start, end):
    command = [objdump, '-d', '--no-show-raw-insn', f'--start-address={start:#x}',
               f'--stop-address={end:#x}', str(binary)]
    p = subprocess.run(command, text=True, capture_output=True)
    if p.returncode:
        raise RuntimeError(f'objdump failed: {p.stderr.strip()}')
    instructions = normalize_disassembly(p.stdout)
    if not instructions:
        raise RuntimeError(f'objdump returned no instructions at {start:#x}')
    return instructions


def ledger(entry, reason, methods='', evidence=''):
    return {'entry': entry, 'client_methods': methods, 'evidence': evidence,
            'verdict': 'review', 'reason': reason}


def next_start(starts, address):
    i = bisect.bisect_right(starts, address)
    return starts[i] if i < len(starts) else None


def method_diff(old, new, rvas, objdump):
    oa, _, osorted = metadata(old)
    _, nk, nsorted = metadata(new)
    old_bin, new_bin = old / 'libil2cpp.so', new / 'libil2cpp.so'
    # Equal binary bytes and ranges imply equal disassembly, regardless of path.
    def digest(path):
        h = hashlib.sha256()
        with path.open('rb') as f:
            for block in iter(lambda: f.read(1024 * 1024), b''):
                h.update(block)
        return h.digest()
    same_binary = digest(old_bin) == digest(new_bin)
    items = []
    for address in sorted(rvas & oa.keys()):
        for row in oa[address]:
            key = (row['group'], row['name'], row['signature'])
            label = f'{key[0]}::{key[1]}'
            matches = nk.get(key, [])
            if len(matches) != 1:
                reason = 'missing, renamed or re-signed method' if not matches else 'ambiguous new method identity'
                items.append(ledger(label, reason, f'old {address:#x}', key[2]))
                continue
            target = int(matches[0]['virtualAddress'], 16)
            oe, ne = next_start(osorted, address), next_start(nsorted, target)
            methods = f'old {address:#x}; new {target:#x}'
            if oe is None or ne is None:
                items.append(ledger(label, 'no next method start to bound disassembly', methods))
                continue
            if same_binary and (address, oe) == (target, ne):
                continue
            try:
                a = disassemble(objdump, old_bin, address, oe)
                b = disassemble(objdump, new_bin, target, ne)
            except (OSError, RuntimeError) as exc:
                items.append(ledger(label, f'disassembly unavailable: {exc}', methods))
                continue
            if a != b:
                items.append(ledger(label, 'normalized instruction stream changed', methods,
                                    f'old [{address:#x}, {oe:#x}); new [{target:#x}, {ne:#x})'))
    return items


def without_rvas(value):
    if isinstance(value, dict):
        return {k: without_rvas(v) for k, v in value.items() if k != 'rva'}
    if isinstance(value, list):
        return [without_rvas(v) for v in value]
    return value


def definition_diff(old, new):
    items = []
    # locationdefs is optional: many captures, including 59.15.1, contain no
    # location definition list. Never interpret its absence as an empty list.
    for filename in ('carddefs.json', 'defs.json', 'locationdefs.json'):
        a, b = old / 'rules' / filename, new / 'rules' / filename
        if filename == 'locationdefs.json' and not a.exists() and not b.exists():
            continue
        if not a.exists() or not b.exists():
            items.append(ledger(filename, 'definition capture missing on one side'))
            continue
        left, right = load_json(a), load_json(b)
        if not isinstance(left, dict) or not isinstance(right, dict):
            raise ValueError(f'{filename}: expected object keyed by definition ID')
        for key in sorted(left.keys() | right.keys()):
            if key not in left or key not in right:
                items.append(ledger(key, f'{filename}: ' + ('added' if key not in left else 'removed')))
                continue
            x, y = without_rvas(left[key]), without_rvas(right[key])
            if x != y:
                fields = sorted(k for k in x.keys() | y.keys()
                                if k not in x or k not in y or x[k] != y[k])
                details = []
                for field in fields:
                    before = json.dumps(x[field], ensure_ascii=False) if field in x else '<missing>'
                    after = json.dumps(y[field], ensure_ascii=False) if field in y else '<missing>'
                    if len(before) + len(after) <= 200:
                        details.append(f'{field}: {before} -> {after}')
                items.append(ledger(key, f'{filename}: changed ' + ', '.join(fields),
                                    evidence='; '.join(details)))
    return items


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('old', type=Path, help='old client directory')
    parser.add_argument('new', type=Path, help='new client directory')
    parser.add_argument('--objdump', default=os.environ.get('OBJDUMP', 'objdump'), help='ARM64-capable objdump')
    parser.add_argument('--json', action='store_true', help='print JSON ledger instead of checklist')
    args = parser.parse_args(argv)
    try:
        items = method_diff(args.old, args.new, cited_rvas(), args.objdump)
        items.extend(definition_diff(args.old, args.new))
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError, RuntimeError) as exc:
        print(f'client_diff: {exc}', file=sys.stderr)
        return 2
    limitations = ['Location definitions are only compared when rules/locationdefs.json is captured; '
                   'the standard 59.15.1 corpus does not include it.',
                   'Normalized instruction comparison masks relocation-sensitive operands, not literal data; '
                   'matching streams do not prove matching behavior.']
    if args.json:
        print(json.dumps({'work_list': items, 'limitations': limitations}, indent=2))
    else:
        print('Client update review checklist')
        for item in items:
            print(f"- [ ] {item['entry']} | Client method(s): {item['client_methods'] or '—'} | "
                  f"Evidence: {item['evidence'] or 'definition diff'} | Verdict: review | Reason: {item['reason']}")
        if not items:
            print('- No differences requiring review detected.')
        for limitation in limitations:
            print(f'Note: {limitation}')
    return int(bool(items))


if __name__ == '__main__':
    sys.exit(main())
