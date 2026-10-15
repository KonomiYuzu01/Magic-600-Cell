"""Summarise the audit-hook traces of the four constructors and compare their re-run outputs with the delivered files.

Usage: python3 -I -B trace_summary.py <out json>
"""
import importlib.util
import json
import os
import struct
import sys
from pathlib import Path

SP = Path('<work>')
B4 = SP / 'v26' / 'B4'
AUD = B4 / 'audit' / 'words'
TR = AUD / 'trace'
SYS_PREFIXES = ('/usr/lib/python3', '/usr/local/lib/python3', '/usr/lib/python3/dist-packages', '/etc/', '/dev/',
                '/proc/', '/sys/')
TIMING = {'times', 'seconds', 'level_sec', 'sec'}


def strip(x):
    if isinstance(x, dict):
        return {k: strip(v) for k, v in x.items() if k not in TIMING}
    if isinstance(x, list):
        return [strip(v) for v in x]
    return x


def jl(p):
    return json.loads(Path(p).read_text())


def pyc_check(pyc):
    """(flags, valid against its source) for a timestamp-based pyc of this interpreter."""
    if not Path(pyc).exists():
        return {'pyc': pyc, 'exists': False, 'timestamp_valid': True, 'note': 'open attempted, no cache; source compiled'}
    raw = Path(pyc).read_bytes()[:16]
    magic, flags = raw[:4], struct.unpack('<I', raw[4:8])[0]
    if magic != importlib.util.MAGIC_NUMBER:
        return {'pyc': pyc, 'magic_ok': False}
    name = Path(pyc).name.split('.')[0] + '.py'
    src = Path(pyc).parent.parent / name
    out = {'pyc': pyc, 'magic_ok': True, 'flags': flags, 'source': str(src), 'source_exists': src.exists()}
    if flags == 0 and src.exists():
        mtime, size = struct.unpack('<II', raw[8:16])
        stt = src.stat()
        out['timestamp_valid'] = (mtime == (int(stt.st_mtime) & 0xFFFFFFFF)) and (size == (stt.st_size & 0xFFFFFFFF))
    return out


def summary(name):
    d = jl(TR / f'{name}.trace.json')
    data_reads, writes, code, other, listdirs = [], [], [], [], []
    for e in d['events']:
        if e['event'] == 'import':
            continue
        if e['event'] == 'open':
            p = e['real'] or e['target']
            if p.startswith(SYS_PREFIXES):
                continue
            w = any(ch in e['mode'] for ch in 'wax+')
            if w:
                writes.append(p)
            elif p.endswith(('.py', '.pyc')):
                code.append(p)
            else:
                data_reads.append(p)
        elif e['event'] == 'os.listdir':
            t = e['target']
            if not any(s in t for s in SYS_PREFIXES):
                listdirs.append(t)
        else:
            other.append([e['event'], e['target'], e['count']])
    mods = {k: v for k, v in d['modules'].items() if not v.startswith(SYS_PREFIXES)}
    pycs = [pyc_check(p) for p in code if p.endswith('.pyc')]
    return {'script': d['script'], 'argv': d['argv'], 'status': d['status'], 'data_reads': sorted(set(data_reads)),
            'writes': sorted(set(writes)), 'code_files': sorted(set(code)), 'other_events': other,
            'listdirs': sorted(set(listdirs)), 'non_system_modules': mods,
            'pyc_all_valid_timestamp': all(x.get('timestamp_valid') for x in pycs), 'pyc_checks': pycs}


def main():
    out = {'traces': {n: summary(n) for n in ('singleP', 'shrink', 'constructS', 'closure2')}}
    cmp = {}
    # singleP
    cmp['singleP_out_equals_B3_singleP.json'] = jl(TR / 'singleP-out.json') == jl(SP / 'v26' / 'B3' / 'singleP.json')
    # shrink
    new = [json.loads(x) for x in (TR / 'shrink.jsonl').read_text().splitlines() if x.strip()]
    old = [json.loads(x) for x in (B4 / 'shrink-237-13.jsonl').read_text().splitlines() if x.strip()]
    cmp['shrink_jsonl_equal_apart_from_timing'] = strip(new) == strip(old)
    cmp['shrink_levels'] = len(new)
    # constructS
    o, n = B4 / 'cS-P4-162-1', TR / 'cS'
    for f in ('pre.json', 'certificate-Q0.json', 'word-full.json', 'result.json'):
        cmp[f'constructS_{f}_equal_apart_from_timing'] = strip(jl(o / f)) == strip(jl(n / f))
    for f in sorted(os.listdir(o / 'parts')):
        cmp[f'constructS_parts/{f}_identical'] = jl(o / 'parts' / f) == jl(n / 'parts' / f)
    cmp['constructS_parts_same_names'] = sorted(os.listdir(o / 'parts')) == sorted(os.listdir(n / 'parts'))
    # closure2
    o, n = B4 / 'c3', TR / 'c3'
    for f in ('pre.json', 'certificate.json', 'word-full.json', 'result.json'):
        cmp[f'closure2_{f}_equal_apart_from_timing'] = strip(jl(o / f)) == strip(jl(n / f))
    for f in sorted(os.listdir(o / 'parts')):
        cmp[f'closure2_parts/{f}_identical'] = jl(o / 'parts' / f) == jl(n / 'parts' / f)
    cmp['closure2_parts_same_names'] = sorted(os.listdir(o / 'parts')) == sorted(os.listdir(n / 'parts'))
    out['comparisons'] = cmp
    Path(sys.argv[1]).write_text(json.dumps(out, indent=1) + '\n')
    for k, v in out['traces'].items():
        print('==', k, v['status'])
        print('  data reads:', v['data_reads'])
        print('  writes    :', v['writes'])
        print('  other     :', v['other_events'])
        print('  listdirs  :', v['listdirs'])
        print('  pyc valid :', v['pyc_all_valid_timestamp'], len(v['pyc_checks']))
        print('  modules   :', sorted((m, p) for m, p in v['non_system_modules'].items()
                                     if 'jumbling' not in p or m in ('__main__',)))
    print(json.dumps(cmp, indent=1))


if __name__ == '__main__':
    main()
