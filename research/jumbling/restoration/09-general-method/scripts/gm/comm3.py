"""Commutator refinement of block words at a state (menu I_a), read from the pose table and the words only.

For a word Q whose exact frame ghost brings the target block T onto the lattice and keeps every other off piece at
its exact content, and a K+ letter k at a grip whose cap holds T's landing chambers after Q, the commutator
    [Q, k] = Q, k, Q^-1, k^-1
brings T onto the lattice as well (T moves with k out of the support of Q^-1, then back), and its Bad set is only
what k moves inside the support of Q. Necessary condition used as a filter: k^-1 maps T's landing into chambers that
are holes, Bad(Q) chambers or the landing itself (whatever k brings into the landing meets Q^-1 on T's path).
Every candidate passing the filter is scored by the exact frame ghost of T, of every other off piece and of every
chamber of the word's caps: T on K, protected exact, Bad([Q, k]) and its occupancy at the state.

Each output row carries the full word W = Q, k, Q^-1, k^-1, so a row can serve as input again (nested refinement
[[Q, k], k2]). With --rescore the input words are scored again at this state and only those with T on K and the
protected pieces exact are refined (rows from a search at an earlier state). --max-k N keeps the N best refinements
per input word (by occupied Bad, then Bad); default all.

--free <json {pieces}> (comm3): pieces outside T that the word may move (the other frames of a multi-frame block):
they are not protected; a word is kept only if every free piece stays admissible (its ghost alive). Rows add free_dead
and free_on (free pieces that end on K).

Every row carries 'deficit': the sum over piece types t of max(0, occupied Bad chambers of type t - holes of type t
outside Bad). A lattice-preserving vacating word moves lattice pieces only into holes of their
own type, so a word with deficit > 0 cannot be closed by Lemma V or V'. --emit-base also writes each rescored input
word itself (k = null, 'base': true) with its deficit.

usage: comm3.py <theory dir> <E-chain scripts dir> <pose table> <target json {pieces}> <q.jsonl> <out jsonl>
       [--max-q N] [--rescore] [--max-k N] [--free <json>] [--emit-base]
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(Path(sys.argv[2]).resolve()))
import conjprep as CP  # noqa: E402
import fastghost as FG  # noqa: E402
import ghostlib as GL  # noqa: E402
import restore  # noqa: E402
import rim  # noqa: E402
from exact import identity  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)


class Lite(GL.Ctx):
    def __init__(self, table):  # noqa: D401 - ghostlib.Ctx without the block computation
        self.R, self.M = GL.O.start(table)
        self.st = self.R.st
        self.D = rim.Domains(self.st)
        self.F = FG.Frames()
        self.dmap = {j: self.F.domain_of(rep) for j, rep in enumerate(self.D.reps)}
        lat = np.flatnonzero(self.D.dom == 0)
        self.occ = np.zeros(GL.NP, bool)
        self.occ[self.D.cham[lat]] = True
        self.off = np.flatnonzero(self.st._kpose()[self.st.pose_id] < 0)
        self._el = {}


table, tfile, qfile, out = sys.argv[3:7]
a = sys.argv[7:]
MAXQ = int(a[a.index('--max-q') + 1]) if '--max-q' in a else 10 ** 9
MAXK = int(a[a.index('--max-k') + 1]) if '--max-k' in a else 10 ** 9
RESCORE = '--rescore' in a
C = Lite(table)
M = C.M
kp = GL.BL.CL.ctx.kplus
KID = kp.index_of_matrix(to_tuple(identity(4)))
T = np.asarray(sorted(json.loads(Path(tfile).read_text())['pieces']), np.int64)
FREE = np.asarray(sorted(json.loads(Path(a[a.index('--free') + 1]).read_text())['pieces']), np.int64) \
    if '--free' in a else np.zeros(0, np.int64)
prot = np.setdiff1d(np.setdiff1d(C.off, T), FREE)
nF = len(FREE)
d0, q0 = C.contents(prot)
holes = ~C.occ
ORB = np.asarray(GL.data.orbit_id, np.int64) + 1  # 36 K+ chamber orbits; data labels them -1..34
NORB = int(ORB.max()) + 1
nT = len(T)
_caps, _kl = {}, {}


def capm(c):
    if c not in _caps:
        _caps[c] = GL.cap(c)
    return _caps[c]


def kletters(c):
    """Non-identity K+ letters at grip c: (letter, K+ index, inverse letter)."""
    if c not in _kl:
        out_ = []
        for i in range(len(M.items)):
            k = kp.index_of_matrix(restore.element(M, c, i))
            if k >= 0 and k != KID:
                j = CP.inverse_word(M, [(c, i)])[0][1]
                out_.append((i, int(k), int(j)))
        _kl[c] = out_
    return _kl[c]


def score(W):
    g, n, U = C.ghost(W, np.concatenate([T, FREE, prot]))
    m = nT + nF
    pa = g.alive[m:n]
    mv = pa & ((g.dom[m:n] != d0) | (g.q[m:n] != q0))
    lat = slice(n, None)
    bad = U[~g.alive[lat] | (g.dom[lat] != 0)]
    fa = g.alive[nT:m]
    ton = g.alive[:nT] & (g.dom[:nT] == 0)
    occb = bad[C.occ[bad]]
    freeh = holes.copy()
    freeh[bad] = False
    need = np.bincount(ORB[occb], minlength=NORB)
    have = np.bincount(ORB[np.flatnonzero(freeh)], minlength=NORB)
    return {'T_on': int(ton.sum()), 'prot_dead': int((~pa).sum()),
            'prot_moved': int(mv.sum()), 'bad': int(len(bad)), 'bad_occupied': int(len(occb)),
            'deficit': int(np.maximum(need - have, 0).sum()),
            'free_dead': int((~fa).sum()), 'free_on': int((fa & (g.dom[nT:m] == 0)).sum())}, g, bad


rows = [json.loads(x) for x in Path(qfile).read_text().splitlines() if x.strip()]
for r in rows:
    if 'W' in r:
        r['Q'] = r['W']
if RESCORE:
    seen, good = set(), []
    for r in rows:
        key = json.dumps(r['Q'])
        if key in seen or r.get('T_on') != nT:
            continue
        seen.add(key)
        s, _, _ = score([(int(c), int(i)) for c, i in r['Q']])
        if s['T_on'] == nT and s['prot_dead'] == 0 and s['prot_moved'] == 0 and s['free_dead'] == 0:
            good.append(dict(r, **s))
        if len(good) >= MAXQ:
            break
else:
    good = [r for r in rows if r['T_on'] == nT and r['prot_dead'] == 0 and r['prot_moved'] == 0
            and r.get('free_dead', 0) == 0][:MAXQ]
log(state_off=int(len(C.off)), T=nT, holes=int(holes.sum()), q_rows=len(rows), q_good=len(good))
best = None
tried = passed = 0
with open(out, 'w') as fh:
    for nq, r in enumerate(good):
        Q = [(int(c), int(i)) for c, i in r['Q']]
        Qi = CP.inverse_word(M, Q)
        s0, g, badQ = score(Q)
        land = g.q[:nT]
        allowed = holes.copy()
        allowed[badQ] = True
        allowed[land] = True
        grips = [c for c in range(600) if capm(c)[land].all()]
        mine = []
        for c2 in grips:
            for i, k, j in kletters(c2):
                tried += 1
                kinv = int(kp.inv[k])
                pre = FG.chamber_image(kinv, land)
                if not allowed[pre].all():
                    continue
                passed += 1
                W = Q + [(c2, i)] + Qi + [(c2, j)]
                s, _, bad = score(W)
                row = {'q': nq, 'Q': r['Q'], 'k': [c2, i], 'badQ': s0['bad'], 'badQ_occupied': s0['bad_occupied'], **s,
                       'W': [list(x) for x in W]}
                ok = s['T_on'] == nT and s['prot_dead'] == 0 and s['prot_moved'] == 0 and s['free_dead'] == 0
                if '--diag' in a:
                    log(diag=[nq, c2, i], **s)
                if ok:
                    mine.append(row)
                if ok and (best is None or (s['bad_occupied'], s['bad']) < (best['bad_occupied'], best['bad'])):
                    best = row
                    log(event='best', q=nq, k=[c2, i], bad=s['bad'], bad_occupied=s['bad_occupied'],
                        badQ=s0['bad'], badQ_occupied=s0['bad_occupied'])
        mine.sort(key=lambda x: (x['deficit'], x['bad_occupied'], x['bad']))
        for row in mine[:MAXK]:
            fh.write(json.dumps(row) + '\n')
        if '--emit-base' in a:
            fh.write(json.dumps({'q': nq, 'Q': r['Q'], 'k': None, 'badQ': s0['bad'],
                                 'badQ_occupied': s0['bad_occupied'], **s0, 'W': [list(x) for x in Q],
                                 'base': True}) + '\n')
        if nq % 20 == 0:
            log(q=nq, of=len(good), tried=tried, passed=passed,
                best=None if best is None else [best['bad'], best['bad_occupied']])
log(event='end', tried=tried, passed=passed, best=best)
