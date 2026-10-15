"""Collect the written steps of one or more driver runs into a chain for verify_u.py.

Each run log (run.jsonl of cbrun.py, cbrun2.py or cbrun3.py) lists its tries; the successful ones, in step order, are
the steps (when a batch holds several successes, the one whose end digest is the next state's digest; at the last step
of a run, the first success in the log). For each step the tclose2 output directory holds word-full.json (the word V W V^-1 with its input and
output digests) and pre.json (the unit T, the protected off pieces and the blocks at the start of the step); the free
set is every off piece at the start that is neither in T nor protected. The runs must follow each other: the first
input digest of a run equals the last output digest of the run before.

usage: mkchain_u.py <out dir> <name> <start table (copied to the out dir)> <run.jsonl>...
Writes <out dir>/chain.json, stepNNN-word.json, stepNNN-unit.json, stepNNN-free.json and the start table.
"""
import json
import shutil
import sys
from pathlib import Path

out, name, start = Path(sys.argv[1]), sys.argv[2], Path(sys.argv[3])
out.mkdir(parents=True, exist_ok=True)
shutil.copyfile(start, out / 'start-table.json')
steps, last = [], None
for log in sys.argv[4:]:
    lp = Path(log)
    rows = [json.loads(x) for x in lp.read_text().splitlines() if x.strip()]
    # a batch may hold several successes; the step taken is the one whose end is the next state of the run
    nxt = {r['step']: r['digest'] for r in rows if r.get('event') == 'state' and 'digest' in r}
    ok = {}
    for t in rows:
        if t.get('event') != 'try' or not t.get('ok'):
            continue
        k = t['step']
        if k + 1 in nxt and (t.get('end') or {}).get('digest') not in (None, nxt[k + 1]):
            continue
        ok.setdefault(k, t)
    for k in sorted(ok):
        t = ok[k]
        et = Path(t['end_table'])
        d = et.parent if et.is_absolute() else (lp.parent.parent.parent / et).parent
        if not d.exists():
            d = lp.parent / et.parent.name
        w = json.loads((d / 'word-full.json').read_text())
        pre = json.loads((d / 'pre.json').read_text())
        if last is not None and w['input_digest'] != last:
            raise SystemExit(f'step {len(steps) + 1} does not start where the previous step ended ({d})')
        last = w['output_digest']
        T = sorted(pre['target_block'])
        off = set(p for b in pre['blocks'] for p in b)
        F = sorted(off - set(T) - set(pre['protected_off']))
        n = len(steps) + 1
        (out / f'step{n:03d}-word.json').write_text(json.dumps(w) + '\n')
        (out / f'step{n:03d}-unit.json').write_text(json.dumps({'pieces': T}) + '\n')
        (out / f'step{n:03d}-free.json').write_text(json.dumps({'pieces': F}) + '\n')
        steps.append({'label': f'{lp.parent.name}-k{t["step"]:02d}-u{t.get("unit", t.get("block"))}', 'word': f'step{n:03d}-word.json',
                      'unit': f'step{n:03d}-unit.json', 'free': f'step{n:03d}-free.json' if F else None,
                      'size': len(T), 'free_size': len(F), 'length': len(w['word'])})
first = json.loads((out / 'step001-word.json').read_text())['input_digest'] if steps else None
(out / 'chain.json').write_text(json.dumps({'name': name, 'start': 'start-table.json', 'start_digest': first,
                                            'steps': steps}, indent=1) + '\n')
print(json.dumps({'name': name, 'steps': len(steps), 'start_digest': first, 'end_digest': last}))
