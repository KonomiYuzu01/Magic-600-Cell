"""Compare the fixture-free constructor outputs (nofix/) with the delivered files, ignoring timing fields.

Usage: python3 -I -B nofix_compare.py <out json>
"""
import json
import os
import sys
from pathlib import Path

SP = Path('<work>')
B4 = SP / 'v26' / 'B4'
NF = B4 / 'audit' / 'words' / 'nofix'
TIMING = {'times', 'seconds', 'level_sec', 'sec'}


def strip(x):
    if isinstance(x, dict):
        return {k: strip(v) for k, v in x.items() if k not in TIMING}
    if isinstance(x, list):
        return [strip(v) for v in x]
    return x


def jl(p):
    return json.loads(Path(p).read_text())


cmp = {}
for n in ('closure2', 'constructS', 'shrink', 'singleP'):
    t = jl(NF / f'{n}.nofix.json')
    cmp[f'{n}_status'] = t['status']
    cmp[f'{n}_fixture_opened'] = t['fixture_opened']
    cmp[f'{n}_data_opens'] = sorted({os.path.basename(r['path']) + ':' + r['mode'] for r in t['data_opens']})
cmp['singleP_equal'] = jl(NF / 'singleP-out.json') == jl(SP / 'v26' / 'B3' / 'singleP.json')
new = [json.loads(x) for x in (NF / 'shrink.jsonl').read_text().splitlines() if x.strip()]
old = [json.loads(x) for x in (B4 / 'shrink-237-13.jsonl').read_text().splitlines() if x.strip()]
cmp['shrink_equal_apart_from_timing'] = strip(new) == strip(old)
for label, o, n, files in (('constructS', B4 / 'cS-P4-162-1', NF / 'cS', ('pre.json', 'certificate-Q0.json', 'word-full.json', 'result.json')),
                           ('closure2', B4 / 'c3', NF / 'c3', ('pre.json', 'certificate.json', 'word-full.json', 'result.json'))):
    for f in files:
        cmp[f'{label}_{f}_equal_apart_from_timing'] = strip(jl(o / f)) == strip(jl(n / f))
    cmp[f'{label}_parts_identical'] = (sorted(os.listdir(o / 'parts')) == sorted(os.listdir(n / 'parts'))
                                       and all(jl(o / 'parts' / f) == jl(n / 'parts' / f) for f in os.listdir(o / 'parts')))
Path(sys.argv[1]).write_text(json.dumps(cmp, indent=1) + '\n')
print(json.dumps(cmp, indent=1))
