"""Independent check of |Bad(W)| with the simulator's exact classifier, including the chambers W cannot carry.

Bad(W) is the set of chambers whose lattice ghost W takes off the lattice or cannot carry (the ghost straddles the
cap of a later letter); it depends on W alone. Start from a state with every piece on the lattice, so every chamber
holds a lattice piece, and apply the letters of W one by one with the simulator's own exact classification
(`sim.state.State.classify`, restated here to list every straddling piece instead of stopping at the first): the
pieces inside the cap are rotated (`State._commit`), a piece that straddles is marked dead and frozen. At the end
|Bad(W)| = dead pieces + live pieces off the lattice. If no piece dies this is the plain simulator replay of W. The
state is restored from a saved copy after each word and its digest is checked.
Uses the simulator only (no ghost library of the search scripts).

usage: ghostsim.py <research/jumbling> <echain dir> <all-lattice start table> <word file to reach the lattice | ->
       <scored jsonl(.gz) with 'Q' and 'badQ'> <out jsonl> [--max-bad N] [--sample K] [--seed S] [--all]
       [--types t1,t2]
--types: also count Bad(W) by type (regions.orbit_of; at an all-lattice state the piece in a chamber has the
chamber's type), for the check of Corollary D''.
"""
import gzip
import json
import random
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
J = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(J / 'theory'))
sys.path.insert(0, str(J))
sys.path.insert(0, str(Path(sys.argv[2]).resolve()))
import orient as O  # noqa: E402
import sim  # noqa: E402
from sim.kernel import Evaluator, pullback_form  # noqa: E402
from sim.state import Classification  # noqa: E402

a = sys.argv[1:]
table, wfile, scored, out = a[2], a[3], a[4], Path(a[5])
MAXB = int(a[a.index('--max-bad') + 1]) if '--max-bad' in a else 400
K = int(a[a.index('--sample') + 1]) if '--sample' in a else 100
SEED = int(a[a.index('--seed') + 1]) if '--seed' in a else 7
t0 = time.time()
R, M = O.start(table, [] if wfile == '-' else [wfile])
st = R.st
ctx = st.ctx
data, kp, reg = ctx.data, ctx.kplus, ctx.regions
assert st.off_lattice_count() == 0, 'the start is not on the lattice'
d0 = st.digest()
TYPES = [int(x) for x in a[a.index('--types') + 1].split(',')] if '--types' in a else []
TYPE = np.asarray(reg.orbit_of, np.int64)


def ghost_letter(c, i, dead):
    """One letter in the ghost semantics: rotate the pieces inside cap c, mark every live straddler dead."""
    cl = Classification(int(c))
    Kp = st._kpose()[st.pose_id]
    lat = Kp >= 0
    lp = np.nonzero(lat)[0]
    parts = [lp[data.in_sig(lp, kp.perms[kp.inv[Kp[lp]], int(c)])]]
    off = np.nonzero(~lat & ~dead)[0]
    newly = []
    for pid, members in (st._groups(off) if off.size else []):
        ev = Evaluator(pullback_form(st._pose_matrix(pid), data.N[c]))
        for anchor, sub in st._anchors(pid, members):
            r = st._eval(ev, reg.cap_vform(anchor), lambda q=anchor: reg.cap_frows(q), cl, None)
            if r[0] == 'uncertain':
                raise RuntimeError('uncertain classification')
            if r[0] != 'straddle':
                if r[0] == 'in':
                    parts.append(sub)
                continue
            ins = []
            for p in sub.tolist():
                r = st._eval(ev, reg.vform(p), lambda q=p: reg.frows(q), cl, None)
                if r[0] == 'uncertain':
                    raise RuntimeError('uncertain classification')
                if r[0] == 'straddle':
                    newly.append(p)
                elif r[0] == 'in':
                    ins.append(p)
            if ins:
                parts.append(np.array(ins, np.int64))
    if newly:
        dead[np.array(newly, np.int64)] = True
    inside = np.sort(np.concatenate(parts)) if parts else np.zeros(0, np.int64)
    st._commit(sim.Twist(ctx, int(c), R.g(c, i), 'menu', {'menu': M.name}), inside, False)
    return len(newly)


op = gzip.open if scored.endswith('.gz') else open
rows = [json.loads(x) for x in op(scored, 'rt') if x.strip()]
if '--all' in a:
    todo = rows
    nlow = len(rows)
else:
    low = [r for r in rows if r['badQ'] <= MAXB]
    rest = [r for r in rows if r['badQ'] > MAXB]
    random.Random(SEED).shuffle(rest)
    todo = low + rest[:K]
    nlow = len(low)
print(json.dumps({'rows': len(rows), 'checked': len(todo), 'low': nlow, 'max_bad': MAXB,
                  'load_s': round(time.time() - t0, 1)}), flush=True)
res = []
with out.open('w') as fh:
    for n, r in enumerate(todo):
        saved = (st.pose_id.copy(), list(st._poses), dict(st._lookup))
        dead = np.zeros(len(st.pose_id), bool)
        died = [ghost_letter(int(c), int(i), dead) for c, i in r['Q']]
        badm = dead | ~st.lattice_flags()
        live_off = int((badm & ~dead).sum())
        sim_bad = int(dead.sum()) + live_off
        by_type = {str(t): int((badm & (TYPE == t)).sum()) for t in TYPES}
        st.pose_id, st._poses, st._lookup = saved
        st._anchor_cache = {}
        back = st.digest() == d0
        row = {'n': n, 'e': r['e'], 'B': r['B'], 'c': r['c'], 'X': r['X'], 'a': r['a'], 'A': r['A'],
               'badQ': r['badQ'], 'sim_bad': sim_bad, 'dead': int(dead.sum()), 'live_off': live_off,
               'died_per_letter': died, 'by_type': by_type, 'match': sim_bad == r['badQ'], 'back': back}
        fh.write(json.dumps(row) + '\n')
        fh.flush()
        res.append(row)
        if not back:
            raise SystemExit(f'state not returned after row {n}')
mm = [x for x in res if not x['match']]
print(json.dumps({'checked': len(res), 'match': len(res) - len(mm), 'mismatch': len(mm),
                  'with_dead': sum(1 for x in res if x['dead']), 'min_sim_bad': min((x['sim_bad'] for x in res), default=None),
                  'min_badQ_checked': min((x['badQ'] for x in res), default=None), 'seconds': round(time.time() - t0, 1)}),
      flush=True)
