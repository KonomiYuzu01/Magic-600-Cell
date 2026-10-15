"""Chain from tclose2.py output directories (one step each, in order), for verify_u.py: the same files as mkchain_u.py
writes for driver runs. The free set of a step is every off piece at its start that is neither in T nor protected.

usage: mkchain1.py <out dir> <name> <start table (copied)> <tclose2 out dir>...
"""
import json
import shutil
import sys
from pathlib import Path

out, name, start = Path(sys.argv[1]), sys.argv[2], Path(sys.argv[3])
out.mkdir(parents=True, exist_ok=True)
shutil.copyfile(start, out / 'start-table.json')
steps, last = [], None
for n, dd in enumerate(sys.argv[4:], start=1):
    d = Path(dd)
    w = json.loads((d / 'word-full.json').read_text())
    pre = json.loads((d / 'pre.json').read_text())
    if last is not None and w['input_digest'] != last:
        raise SystemExit(f'step {n} does not start where the previous step ended ({d})')
    last = w['output_digest']
    T = sorted(pre['target_block'])
    off = set(p for b in pre['blocks'] for p in b)
    F = sorted(off - set(T) - set(pre['protected_off']))
    (out / f'step{n:03d}-word.json').write_text(json.dumps(w) + '\n')
    (out / f'step{n:03d}-unit.json').write_text(json.dumps({'pieces': T}) + '\n')
    (out / f'step{n:03d}-free.json').write_text(json.dumps({'pieces': F}) + '\n')
    steps.append({'label': d.name, 'word': f'step{n:03d}-word.json', 'unit': f'step{n:03d}-unit.json',
                  'free': f'step{n:03d}-free.json' if F else None, 'size': len(T), 'free_size': len(F),
                  'length': len(w['word'])})
first = json.loads((out / 'step001-word.json').read_text())['input_digest'] if steps else None
(out / 'chain.json').write_text(json.dumps({'name': name, 'start': 'start-table.json', 'start_digest': first,
                                            'steps': steps}, indent=1) + '\n')
print(json.dumps({'name': name, 'steps': len(steps), 'start_digest': first, 'end_digest': last}))
