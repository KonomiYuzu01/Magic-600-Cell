"""Cap restoration run (S-type steps): repeatedly find an unstraddled cap with no lattice piece inside whose off
contents are all sent to K+ by one letter at its grip, and apply that letter on the simulator.

Lemma S (cap restoration): if no off region straddles cap c, no lattice piece is inside cap c, and the letter X at c
sends every off content inside cap c to K+, then the twist (c, X) is admissible, puts every off piece inside cap c on
the lattice, and changes no other piece (pieces outside the cap do not move; there is no lattice piece inside).

The scan is exact: float ranges only select the contents that get an exact side test. Each step is executed on the
simulator and checked there (restored set, collateral 0, every other off pose unchanged by exact pose code), and the
virtual contents are checked against the simulator's off set after every step.

usage: srun.py <research/jumbling> <E-chain scripts> <table> <out dir> [--blocks <blocks json>] [--max-steps N]
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import glib  # noqa: E402

J, SC, TABLE, OUT = sys.argv[1:5]
a = sys.argv[5:]
BLOCKS = a[a.index('--blocks') + 1] if '--blocks' in a else None
MAXS = int(a[a.index('--max-steps') + 1]) if '--max-steps' in a else 10 ** 6
OUT = Path(OUT)
OUT.mkdir(parents=True, exist_ok=True)
glib.setup(J + '/theory', SC)
import conjclose as CC  # noqa: E402

W = glib.World(TABLE, BLOCKS)
R, st = W.R, W.st
codes = CC.Codes()
_letters = {}


def letters_for(c, doms):
    key = (c, tuple(doms))
    if key not in _letters:
        out = []
        for Xi in range(W.n_items):
            X = W.E(c, Xi)
            if all(W.F.mul(W.to_tuple(X), X, d)[0] == 0 for d in doms):
                out.append(Xi)
        _letters[key] = out
    return _letters[key]


def cap_row(vs, c, offv=None):
    off = vs.off
    occ = vs.occ()
    nocc = int((W.GL.cap(c) & occ).sum())
    if not len(off):
        return {'cap': c, 'occupied': nocc, 'inside': np.zeros(0, np.int64), 'straddlers': 0, 'letters': []}
    offv = vs.off_vertices() if offv is None else offv
    lo, hi = W.cap_range(offv, [c])
    near = np.flatnonzero(hi[:, 0] >= -1e-6)
    s = W.sides(vs.dom[off[near]], vs.q[off[near]], c) if len(near) else np.zeros(0, np.int64)
    inside = off[near[s == 1]]
    strad = int((s < 0).sum())
    lets = letters_for(c, sorted(set(vs.dom[inside].tolist()))) if (strad == 0 and len(inside)) else []
    return {'cap': c, 'occupied': nocc, 'inside': inside, 'straddlers': strad, 'letters': lets}


def scan(vs):
    offv = vs.off_vertices()
    return [cap_row(vs, c, offv) for c in range(600)]


def execute(vs, c, X, target, step):
    """Apply (c, X) on the simulator; check; return the new virtual state."""
    t0 = time.time()
    c_s = codes.of(st)
    d_in = st.digest()
    R.apply(int(c), int(X))
    c_e = codes.of(st)
    d_out = st.digest()
    off_s, off_e = c_s < 0, c_e < 0
    inT = np.zeros(len(c_s), bool)
    inT[target] = True
    prot = off_s & ~inT
    vs2 = vs.apply([(int(c), int(X))])
    row = {'step': step, 'cap': int(c), 'letter': int(X), 'target': int(len(target)),
           'blocks': sorted(set(W.block_of[target].tolist())),
           'off_before': int(off_s.sum()), 'off_after': int(off_e.sum()),
           'target_on': int((inT & ~off_e).sum()), 'collateral': int((~off_s & off_e).sum()),
           'protected': int(prot.sum()), 'protected_changed': int((prot & (c_e != c_s)).sum()),
           'virtual_matches': bool(vs2 is not None and np.array_equal(np.flatnonzero(vs2.dom != 0),
                                                                     np.flatnonzero(off_e))),
           'input_digest': d_in, 'output_digest': d_out, 'sec': round(time.time() - t0, 1)}
    row['ok'] = (row['target_on'] == row['target'] and row['collateral'] == 0 and row['protected_changed'] == 0
                 and row['virtual_matches'])
    (OUT / f'step{step:03d}-word.json').write_text(json.dumps(
        {'word': [[int(c), int(X)]], 'input_digest': d_in, 'output_digest': d_out, 'kind': 'S', 'cap': int(c)}) + '\n')
    (OUT / f'step{step:03d}-target.json').write_text(json.dumps({'pieces': [int(p) for p in target]}) + '\n')
    with open(OUT / 'run.jsonl', 'a') as f:
        f.write(json.dumps(row) + '\n')
    glib.log(event='step', **{k: v for k, v in row.items() if k not in ('input_digest', 'output_digest')})
    if not row['ok']:
        raise SystemExit('step check failed')
    return vs2


vs = glib.VState(W)
steps = []
start_digest = st.digest()
step = 0
while step < MAXS:
    t0 = time.time()
    rows = scan(vs)
    cands = [r for r in rows if r['straddlers'] == 0 and r['occupied'] == 0 and len(r['inside']) and r['letters']]
    glib.log(event='scan', off=int(len(vs.off)), s_caps=len(cands), pieces=int(sum(len(r['inside']) for r in cands)),
             same_frame_occupied=sum(1 for r in rows if r['straddlers'] == 0 and r['occupied'] > 0
                                     and len(r['inside']) and r['letters']),
             sec_scan=round(time.time() - t0, 1))
    if not cands:
        break
    done_any = False
    for r in sorted(cands, key=lambda r: -len(r['inside'])):
        r2 = cap_row(vs, r['cap'])  # re-check on the current state
        if not (r2['straddlers'] == 0 and r2['occupied'] == 0 and len(r2['inside']) and r2['letters']):
            continue
        step += 1
        vs = execute(vs, r2['cap'], r2['letters'][0], r2['inside'], step)
        steps.append({'label': f'S{step:03d}-cap{r2["cap"]}', 'word': f'step{step:03d}-word.json',
                      'target': f'step{step:03d}-target.json'})
        done_any = True
        if step >= MAXS:
            break
    if not done_any:
        break
start_rel = Path(TABLE).resolve()
(OUT / 'chain.json').write_text(json.dumps({'name': OUT.name, 'start': str(start_rel), 'start_digest': start_digest,
                                            'steps': steps}, indent=1) + '\n')
glib.log(event='end', steps=step, off=int(len(vs.off)))
