"""Independent exact check of chamber-ghost verdicts (audit lemma-h).

Follows the exact vertices of chosen regions through a word with no frames, poles, signature bits or caches from
fastghost: a region R (vertices v) with accumulated motion h is inside cap c when every vertex satisfies
n_c . (h v) >= KAPPA, outside when every vertex satisfies <= KAPPA, and straddles otherwise (it dies there). A float
screen with margin 1e-9 decides clear vertices; every vertex inside the margin is decided by exact Q(sqrt5) signs.
Galois conjugates of the orthogonal matrices involved are orthogonal, so float conversions keep absolute error near
1e-15. At the end a region is good when it never straddled and h (times the start pose, for a piece) lies in K+.

The same contents are then run through ghostlib/fastghost and compared one by one (alive, on K, end chamber).

Usage: python3 -I minighost.py <theory dir> <v26 dir> <table> <word json> <start> <end> <out json>
       [--chambers <json list>] [--sample N] [--pieces <json list>] [--seed S]
<start>:<end> slices the word; --chambers adds chosen chambers; --sample adds N random chambers of the union of caps;
--pieces adds real pieces with their exact poses read from the table.
"""
import json
import random
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(TH.parent / 'fixtures'))
sys.path.insert(0, str(Path(sys.argv[2]).resolve()))
import fastghost as FG  # noqa: E402  (only for the comparison)
import ghostlib as GL  # noqa: E402  (only for the comparison)
import restore  # noqa: E402
import seams  # noqa: E402
import sim  # noqa: E402
from exact import Q5, ZERO, identity, matmul, matvec  # noqa: E402
from sim.kplus import matrix_from_json, to_tuple  # noqa: E402

ctx = sim.get_context()
kp, reg, data = ctx.kplus, ctx.regions, ctx.data
KAPPA = data.KAPPA
KAPPA_F = float(KAPPA)
NF = [np.array([float(x) for x in data.N[c]]) for c in range(600)]
T0 = time.time()
EPS = 1e-9


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


def fmat(m):
    return np.array([[float(x) for x in row] for row in m])


def exact_side(h, c, verts, idx):
    """Exact signs of n_c . (h v) - KAPPA for the vertices verts[idx]."""
    n = data.N[c]
    w = [sum((h[r][k] * n[r] for r in range(4)), ZERO) for k in range(4)]  # h^T n
    out = []
    for j in idx:
        v = verts[j]
        s = sum((w[k] * v[k] for k in range(4)), ZERO) - KAPPA
        out.append(s.sign())
    return out


class Content:
    def __init__(self, verts):
        self.v = [list(v) for v in verts]
        self.vf = np.array([[float(x) for x in v] for v in verts])
        self.h = [list(r) for r in identity(4)]
        self.hf = np.eye(4)
        self.alive = True
        self.died_at = -1
        self.moves = 0
        self.exact_calls = 0

    def side(self, c):
        """1 inside, 0 outside, -1 straddle."""
        s = self.vf @ (self.hf.T @ NF[c]) - KAPPA_F
        pos, neg = s > EPS, s < -EPS
        if pos.any() and neg.any():
            return -1
        amb = np.flatnonzero(~(pos | neg))
        signs = set()
        if len(amb):
            self.exact_calls += 1
            signs = set(exact_side(self.h, c, self.v, amb.tolist()))
        has_pos = pos.any() or 1 in signs
        has_neg = neg.any() or -1 in signs
        if has_pos and has_neg:
            return -1
        if has_pos:
            return 1
        if has_neg:
            return 0
        raise AssertionError('a full-dimensional region with every vertex on a cut')

    def move(self, E):
        self.h = matmul(E, self.h)
        self.hf = fmat(self.h)
        self.moves += 1


def main():
    a = sys.argv[1:]
    table, wfile, s0, s1, outp = a[2], a[3], int(a[4]), int(a[5]), Path(a[6])
    opt = {a[k]: a[k + 1] for k in range(7, len(a) - 1, 2)}
    word = [(int(c), int(i)) for c, i in json.loads(Path(wfile).read_text())['word']][s0:s1]
    tdoc = json.loads(Path(table).read_text())
    _, M = seams._fixture_state('I_a', 40)
    grips = sorted({c for c, _ in word})
    capmask = np.zeros(len(data.sigbits), bool)
    for c in grips:
        capmask |= data.in_sig(np.arange(len(data.sigbits)), np.full(len(data.sigbits), c, np.int64))
    union = np.flatnonzero(capmask)
    chambers = [int(x) for x in json.loads(opt['--chambers'])] if '--chambers' in opt else []
    if '--sample' in opt:
        rng = random.Random(int(opt.get('--seed', '1')))
        pool = sorted(set(union.tolist()) - set(chambers))
        chambers += sorted(rng.sample(pool, int(opt['--sample'])))
    pieces = [int(x) for x in json.loads(opt['--pieces'])] if '--pieces' in opt else []
    contents, kinds = [], []
    for z in chambers:
        contents.append(Content([list(v) for v in reg.vertices(z)]))
        kinds.append(('chamber', z))
    starts = {}
    for p in pieces:
        m = matrix_from_json(tdoc['poses'][tdoc['pose_id'][p]])
        starts[p] = m
        contents.append(Content([matvec(m, v) for v in reg.vertices(p)]))
        kinds.append(('piece', p))
    # convention check: a chamber with c in its signature lies in n_c . x >= KAPPA, else in <= KAPPA
    conv_bad = 0
    for (kind, z), ct in zip(kinds, contents):
        if kind != 'chamber':
            continue
        for c in grips:
            sd = ct.side(c)
            if sd != (1 if data.in_sig(np.array([z]), np.array([c]))[0] else 0):
                conv_bad += 1
    log(contents=len(contents), chambers=len(chambers), pieces=len(pieces), twists=len(word), convention_bad=conv_bad)
    els, efs = {}, {}
    t = time.time()
    for n, (c, i) in enumerate(word):
        if (c, i) not in els:
            els[(c, i)] = restore.element(M, c, i)
        E = els[(c, i)]
        for ct in contents:
            if not ct.alive:
                continue
            sd = ct.side(c)
            if sd < 0:
                ct.alive, ct.died_at = False, n
            elif sd == 1:
                ct.move(E)
        if (n + 1) % 200 == 0:
            log(twist=n + 1, alive=sum(ct.alive for ct in contents))
    mine_seconds = round(time.time() - t, 1)
    rows = []
    for (kind, z), ct in zip(kinds, contents):
        H = ct.h if kind == 'chamber' else matmul(ct.h, starts[z])
        k = kp.index_of_matrix(to_tuple(H)) if ct.alive else -1
        end = int(reg.piece_of[reg.orbit_of[z], kp.compose(k, int(reg.transport[z]))]) if k >= 0 else None
        ident = ct.alive and to_tuple(ct.h) == to_tuple(identity(4))
        rows.append({'kind': kind, 'id': z, 'alive': ct.alive, 'died_at': ct.died_at, 'on_K': k >= 0,
                     'end_chamber': end, 'h_identity': ident, 'moves': ct.moves, 'exact_calls': ct.exact_calls})
    log(mine_seconds=mine_seconds)
    # the same contents through ghostlib / fastghost
    t = time.time()
    C = GL.Ctx(table)
    pd, pq = C.contents(np.array(pieces, np.int64)) if pieces else (np.zeros(0, np.int64), np.zeros(0, np.int64))
    g = FG.Ghost(C.F, np.concatenate([np.zeros(len(chambers), np.int64), pd]),
                 np.concatenate([np.array(chambers, np.int64), pq]))
    for c, i in word:
        g.apply(c, C.E(c, i), (c, i))
    ghost_seconds = round(time.time() - t, 1)
    agree = {'alive': 0, 'on_K': 0, 'end_chamber': 0, 'died_at': 0}
    diffs = []
    for j, r in enumerate(rows):
        ga = bool(g.alive[j])
        gk = ga and int(g.dom[j]) == 0
        gend = int(g.q[j]) if gk else None
        r['ghost'] = {'alive': ga, 'on_K': gk, 'end_chamber': gend, 'died_at': int(g.died_at[j]),
                      'domain': int(g.dom[j]), 'q': int(g.q[j])}
        ok = {'alive': ga == r['alive'], 'on_K': gk == r['on_K'], 'end_chamber': gend == r['end_chamber'],
              'died_at': int(g.died_at[j]) == r['died_at']}
        for key, v in ok.items():
            agree[key] += int(v)
        if not all(ok.values()):
            diffs.append(r)
    out = {'table': table, 'word': wfile, 'slice': [s0, s1], 'twists': len(word), 'contents': len(rows),
           'chambers': len(chambers), 'pieces': len(pieces), 'convention_mismatches': conv_bad,
           'agree': agree, 'disagreements': diffs[:50], 'disagreement_count': len(diffs),
           'mine_bad_chambers': sorted(r['id'] for r in rows if r['kind'] == 'chamber' and not (r['alive'] and r['on_K'])),
           'mine_seconds': mine_seconds, 'ghost_seconds': ghost_seconds, 'rows': rows,
           'seconds': round(time.time() - T0, 1)}
    outp.write_text(json.dumps(out, indent=1) + '\n')
    log(agree=agree, disagreements=len(diffs), bad=out['mine_bad_chambers'][:20], done=str(outp))


if __name__ == '__main__':
    main()
