"""Compare a heap-peeling run with the scramble's history: for each step, the history twists at its grip, and whether
its letter is the exact inverse of one of them, or the inverse of a product of consecutive twists at that grip.

usage: histmatch.py <research/jumbling> <E-chain scripts> <any small table> <run dir> <history json> <out json>
history json: {"history": [[grip, item], ...]} (restore.scramble) or a W-J prefix given as "wj:<n>"
"""
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / 'gf'))
import glib  # noqa: E402

J, SC, TABLE, RUN, HIST, OUT = sys.argv[1:7]
glib.setup(J + '/theory', SC)
W = glib.World(TABLE, '-')
if HIST.startswith('wj:'):
    sys.path.insert(0, J + '/fixtures')
    import wj  # noqa: E402
    fx, _ = wj.load(W.GL.BL.CL.ctx, 'I_a')
    hist = [(int(r['grip']), int(r['params']['label'].split('[')[1].rstrip(']')))
            for r in fx['journal']['records'][:int(HIST[3:])]]
else:
    hist = [tuple(map(int, x)) for x in json.loads(Path(HIST).read_text())['history']]
at = {}
for n, (c, i) in enumerate(hist, 1):
    at.setdefault(c, []).append((n, i))
rows = []
for line in (Path(RUN) / 'run.jsonl').read_text().splitlines():
    x = json.loads(line)
    c, X = x['word'][-1]  # the jumble letter (a carrier step starts with a K+ letter)
    recs = at.get(c, [])
    exact = [n for n, i in recs if W.inv(c, i) == X]
    # products of consecutive twists at the same grip (re-jumbles): inverse of E_last ... E_first
    prod = []
    for a in range(len(recs)):
        g = W.E(c, recs[a][1])
        for b in range(a + 1, len(recs)):
            g = W.matmul(W.E(c, recs[b][1]), g)
            t = W.to_tuple(W.transpose(g))
            if W.to_tuple(W.E(c, X)) == t:
                prod.append([recs[a][0], recs[b][0]])
    rows.append({'step': x['step'], 'kind': x['kind'], 'cap': c, 'letter': X, 'gain': x['gain'],
                 'carrier': x['word'][0] if len(x['word']) > 1 else None,
                 'history_twists_at_grip': [n for n, _ in recs], 'exact_inverse_of': exact,
                 'inverse_of_product': prod})
summ = {'steps': len(rows), 'at_history_grip': sum(1 for r in rows if r['history_twists_at_grip']),
        'exact_inverse': sum(1 for r in rows if r['exact_inverse_of'] or r['inverse_of_product']),
        'history_twists': len(hist), 'history_grips': len(at)}
Path(OUT).write_text(json.dumps({'summary': summ, 'steps': rows}, indent=1) + '\n')
print(json.dumps(summ))
