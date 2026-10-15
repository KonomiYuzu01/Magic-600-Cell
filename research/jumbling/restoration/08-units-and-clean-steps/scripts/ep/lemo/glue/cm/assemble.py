"""Assemble a one-step replay chain for a carrier-move closure (PROOFS-U Section 6.1).

The step word is the path of apply.json followed by the word of word-full.json (V W V^-1), from the start table, with
unit A and the moved carrier B as the free set. The start digest is the state digest of apply.json
(checked again by verify_u.py on load); verify_u.py replays the chain from the start table alone.

usage: assemble.py <start table> <apply.json> <word-full.json> <unit A json> <free B json> <job> <out dir>
"""
import json
import shutil
import sys
from pathlib import Path

start, ap, wf, unit, free, job, out = sys.argv[1:8]
a = json.loads(Path(ap).read_text())
w = json.loads(Path(wf).read_text())
assert a['output_digest'] == w['input_digest'], 'word-full must start where the path ends'
o = Path(out)
o.mkdir(parents=True, exist_ok=False)
shutil.copyfile(start, o / 'start-table.json')
shutil.copyfile(unit, o / 'step001-unit.json')
shutil.copyfile(free, o / 'step001-free.json')
(o / 'step001-word.json').write_text(json.dumps({'word': a['word'] + w['word'], 'input_digest': a['input_digest'],
                                                 'output_digest': w['output_digest']}) + '\n')
doc = {'name': f'carrier-move-{job}', 'start': 'start-table.json',
       'start_digest': a['input_digest'],
       'steps': [{'label': f'cm-{job}', 'word': 'step001-word.json', 'unit': 'step001-unit.json',
                  'free': 'step001-free.json'}]}
(o / 'chain.json').write_text(json.dumps(doc) + '\n')
print(json.dumps({'job': job, 'twists': len(a['word']) + len(w['word'])}))
