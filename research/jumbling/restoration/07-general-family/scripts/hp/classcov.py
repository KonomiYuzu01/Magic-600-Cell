"""Per-class coverage of a heap-peeling run on the I_a@40 fixture, by the block classes of the v1 census.

Classes (at the start state): frozen (no mover); one mover or at least two; one frame or several (distinct frame
domains among the block's pieces, exact). For each class: blocks, pieces, pieces on the lattice at the end of the run,
the steps that restored them, and how many of its pieces were ever sent off again (collateral of a later step).

usage: classcov.py <research/jumbling> <E-chain scripts> <start table> <blocks json> <run dir> <out json>
"""
import collections
import json
import sys
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / 'gf'))
import glib  # noqa: E402

J, SC, TABLE, BLOCKS, RUN, OUT = sys.argv[1:7]
glib.setup(J + '/theory', SC)
W = glib.World(TABLE, '-')
blocks = json.loads(Path(BLOCKS).read_text())['blocks']
chain = json.loads((Path(RUN) / 'chain.json').read_text())
NP = W.NP
on = W.dom == 0
last_restored = np.full(NP, -1, np.int64)
times_off_again = np.zeros(NP, np.int64)
for k, step in enumerate(chain['steps'], 1):
    sets = json.loads((Path(RUN) / step['sets']).read_text())
    r, c = np.asarray(sets['restored'], np.int64), np.asarray(sets['collateral'], np.int64)
    on[r] = True
    on[c] = False
    last_restored[r] = k
    times_off_again[c] += 1
rows = collections.OrderedDict()
per_block = []
for j, b in enumerate(blocks):
    P = np.asarray(b['pieces'], np.int64)
    nf = len(set(W.dom[P].tolist()))
    mv = len(b['movers'])
    cls = 'frozen' if b['frozen'] or mv == 0 else ('one mover' if mv == 1 else 'two or more movers')
    cls += ', ' + ('one frame' if nf == 1 else 'several frames')
    steps = sorted(set(last_restored[P][on[P]].tolist()))
    per_block.append({'block': j, 'size': int(len(P)), 'class': cls, 'frames': nf, 'movers': mv,
                      'on_at_end': int(on[P].sum()), 'steps': steps, 'pieces_sent_off_again': int((times_off_again[P] > 0).sum())})
    r = rows.setdefault(cls, {'blocks': 0, 'pieces': 0, 'on_at_end': 0, 'blocks_complete': 0, 'steps': set(),
                              'pieces_sent_off_again': 0})
    r['blocks'] += 1
    r['pieces'] += int(len(P))
    r['on_at_end'] += int(on[P].sum())
    r['blocks_complete'] += int(on[P].all())
    r['steps'].update(steps)
    r['pieces_sent_off_again'] += per_block[-1]['pieces_sent_off_again']
summary = {k: dict(v, steps=len(v['steps'])) for k, v in rows.items()}
tot = {'blocks': len(blocks), 'pieces': int(sum(len(b['pieces']) for b in blocks)), 'on_at_end': int(sum(
    r['on_at_end'] for r in summary.values())), 'run_steps': len(chain['steps'])}
Path(OUT).write_text(json.dumps({'run': Path(RUN).name, 'total': tot, 'classes': summary, 'blocks': per_block},
                                indent=1) + '\n')
print(json.dumps({'total': tot, 'classes': summary}, indent=1))
