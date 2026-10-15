"""Append the steps of a transport run (stepNN-word.json, NN >= start) to a chain file, checking digest continuity.

usage: extchain.py <chain dir> <run dir> <start step>
"""
import json
import sys
from pathlib import Path

cdir, rdir, start = Path(sys.argv[1]), Path(sys.argv[2]), int(sys.argv[3])
chain = json.loads((cdir / 'chain.json').read_text())
last = json.loads((cdir / chain['steps'][-1]['word']).read_text())['output_digest']
for f in sorted(rdir.glob('step*-word.json')):
    n = int(f.name[4:6])
    if n < start:
        continue
    w = json.loads(f.read_text())
    if w['input_digest'] != last:
        raise SystemExit(f'{f.name}: input digest does not continue the chain')
    pcs = sorted(int(p) for p in w['pieces'])
    label = f"s{n}-block{w['block']}-{len(pcs)}at{pcs[0]}"
    (cdir / label).mkdir(exist_ok=True)
    (cdir / label / 'word-full.json').write_text(json.dumps(
        {'word': w['word'], 'input_digest': w['input_digest'], 'output_digest': w['output_digest'],
         'block_index': w['block'], 'template': w['template'], 'm': w['m']}) + '\n')
    (cdir / label / 'target.json').write_text(json.dumps({'pieces': pcs}) + '\n')
    chain['steps'].append({'label': label, 'word': f'{label}/word-full.json', 'target': f'{label}/target.json'})
    last = w['output_digest']
    print(label, len(w['word']))
(cdir / 'chain.json').write_text(json.dumps(chain, indent=1) + '\n')
print('steps', len(chain['steps']))
