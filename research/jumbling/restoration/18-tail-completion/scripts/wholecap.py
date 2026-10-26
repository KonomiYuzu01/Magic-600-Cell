"""Stage 18 (tail completion): the exact whole-cap test of COV-33 with SAFE preparation (stage 12 PROOFS-B.md, COV-12,
COV-32 and COV-33) at any pose table, with every open cap resolved by the inside-mask-empty flow.

COV-33's computation (scripts/round22/cov33_whole_cap_737.py) read one table and resolved one cap (91, item 14). This
version reads the table from the command line and, in part B, tries every cap that part A leaves open, item by item in
part A's order, until one item passes. The tests are COV-33's, unchanged:
- eligibility: an unstraddled cap whose off pieces are all of one frame domain and a jumble item that lands them all;
- part A, per item: the unprepared inventory at the actual middle (deficit 0 and global Jordan certificates on every
  middle orbit that meets the landing chambers: PASS); otherwise the optimistic middle group, built from the
  mandatory displaced regions (the occupied chambers of the cap in source SAFE orbits without a hole), excludes every
  preparation when it fixes an occupied landing chamber or its upper hole counts fall short (NO_PREP), else OPEN_PREP;
- part B, per open item: the inside mask is empty (every hole moved out of the cap), so the middle geometry is the
  whole cap displaced and exact; an integer flow from the source SAFE orbits (their holes) to the middle demand orbits
  (the landing chambers), with capacities |source orbit ∩ middle orbit outside the cap|, chooses the prepared hole
  set; global Jordan certificates on the partially filled source orbits and on the middle demand orbits, each
  checked again, prove that the preparation X and the vacating word V′ exist (COV-10, Corollary A1). Then
  X·W·V′·W⁻¹·V′⁻¹ is a strict′ step that lands every off piece of the cap (COV-12, COV-32).
Nothing here builds X or V′ as words. Research only, headless; exact geometry. Writes nothing; prints one line per
result (ELIGIBILITY, CAP_RESULT, SUMMARY, PREP_RESULT).
Usage (from the repository root): OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u
<this script> <pose table (.json or .json.gz)> [<unit files ...>]
"""
import json
import resource
import sys
import time
from collections import Counter, deque
from math import lcm
from pathlib import Path

resource.setrlimit(resource.RLIMIT_CPU, (3400, 3420))
sys.path.insert(0, str(Path('research/jumbling/restoration/12-low-height/scripts').resolve()))
import screen as s  # noqa: E402
import numpy as np  # noqa: E402
from exact import matmul  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402
from sim.regions import transport_rows  # noqa: E402
from sim.kernel import sign5_vec, KAPPA_FORM  # noqa: E402

M = s.restore.menu('I_a')
ctx = s.CL.ctx
kp, reg = ctx.kplus, ctx.regions


def im(m):
    d = lcm(*(v.d for row in m for v in row))
    return (np.array([[v.a * (d // v.d) for v in row] for row in m], np.int64),
            np.array([[v.b * (d // v.d) for v in row] for row in m], np.int64), d)


def labels(mi, y):
    """Per cap: 1 if the region of chamber y under the exact matrix mi is inside, 0 outside, 2 straddling."""
    e, v = transport_rows(mi, reg.vform(y))
    v = np.array(v, np.int64)
    aa = 125 * (v[:, :4] @ s.CL.data.N1.T + 5 * v[:, 4:] @ s.CL.data.N2.T) - 2 * e * KAPPA_FORM[0]
    bb = 125 * (v[:, 4:] @ s.CL.data.N1.T + v[:, :4] @ s.CL.data.N2.T) - 2 * e * KAPPA_FORM[1]
    assert max(int(abs(aa).max()), int(abs(bb).max())) < 2 ** 29
    ss = sign5_vec(aa, bb)
    return np.where((ss >= 0).all(axis=0), 1, np.where((ss <= 0).all(axis=0), 0, 2))


def cpu():
    return round(time.process_time(), 1)


st, M, _ = s.load('table', sys.argv[1])
unit_of = {}
for f in sys.argv[2:]:
    ps = json.loads(Path(f).read_text())['pieces']
    for p in ps:
        unit_of[int(p)] = min(ps)
off = s.B.off_pieces(st)
D = s.rim.Domains(st)
mi = {int(pid): im(st._pose_matrix(int(pid))) for pid in np.unique(st.pose_id[off])}
ls = np.array([labels(mi[int(st.pose_id[p])], int(p)) for p in off])


def units_in(u):
    return sorted({unit_of.get(int(p), -1) for p in u})


ct, eligible = Counter(), []
for c in range(600):
    if (ls[:, c] == 2).any():
        ct['straddled'] += 1
        continue
    u = off[ls[:, c] == 1]
    if not len(u):
        ct['empty'] += 1
        continue
    if len(set(D.dom[u])) != 1:
        ct['mixed'] += 1
        continue
    pose = st.pose(int(u[0]))
    items = [i for i in range(12, 60)
             if kp.index_of_matrix(to_tuple(matmul(s.restore.element(M, c, i), pose))) >= 0]
    if items:
        ct['eligible'] += 1
        eligible.append([c, len(u), items])
    else:
        ct['no_menu'] += 1
print('ELIGIBILITY', st.digest(), 'off', len(off), dict(ct),
      [[c, n, units_in(off[ls[:, c] == 1]), items] for c, n, items in eligible], 'CPU', cpu(), flush=True)

s.B.log = lambda **kw: None
home = s.CL.data.sig_matrix(np.arange(len(reg.orbit_of)))
typ = np.asarray(reg.orbit_of)
NP = len(home)
allcaps = [np.flatnonzero(home[:, c]) for c in range(600)]
two = {}
for c in range(600):
    rows = []
    for k in kp.gen_idx[2 * c:2 * c + 2]:
        xs = allcaps[c]
        ys = reg.piece_of[typ[xs], kp.compose_vec(np.full(len(xs), k), reg.transport[xs])]
        i = next(i for i in range(1, 12) if kp.index_of_matrix(s.restore.element(M, c, i)) == k)
        rows.append((c, i, int(k), ys))
    two[c] = rows


def group(safe):
    g = s.B.Alphabet.__new__(s.B.Alphabet)
    g.NP, g.home, g.type = NP, home, typ
    g.safe = list(map(int, safe))
    g.letters = [l for c in g.safe for l in two[c]]
    g.capx = {c: allcaps[c] for c in g.safe}
    g.index = {(l[0], l[1]): j for j, l in enumerate(g.letters)}
    g.orbits()
    return g


def landing_chambers(g, u):
    dst = []
    for p in u:
        k = kp.index_of_matrix(to_tuple(matmul(g, st.pose(int(p)))))
        assert k >= 0
        dst.append(int(reg.piece_of[typ[p], kp.compose(k, int(reg.transport[p]))]))
    return np.array(dst)


G0 = group(np.flatnonzero((ls == 0).all(axis=0)))
H = s.B.hole_set(st)
o0, z0 = G0.orbits()
hc = np.bincount(o0[H], minlength=len(z0))
possible = hc[o0] > 0
print('SOURCE', 'safe_grips', len(G0.safe), 'holes', len(H), 'orbits', len(z0), 'singletons', int((z0 == 1).sum()),
      'orbits_with_holes', int((hc > 0).sum()), 'CPU', cpu(), flush=True)

# Part A: COV-33's actual middles and preparation-independent exclusions.
totals, open_items = Counter(), []
for c, nu, items in eligible:
    cap = allcaps[c]
    u = off[ls[:, c] == 1]
    otherbad = (ls[ls[:, c] != 1] != 0).any(axis=0)
    qs = cap[~np.isin(cap, H)]
    mandatory = ~possible[qs]
    rows, cap_pass, opened = [], False, []
    for i in items:
        g = s.restore.element(M, c, i)
        mg = im(g)
        qbad, qmust = np.zeros(600, bool), np.zeros(600, bool)
        for q, must in zip(qs, mandatory):
            bad = labels(mg, int(q)) != 0
            qbad |= bad
            if must:
                qmust |= bad
        dst = landing_chambers(g, u)
        G1 = group(np.flatnonzero(~(otherbad | qbad)))
        o1, z1 = G1.orbits()
        have = np.bincount(o1[H[~home[H, c]]], minlength=len(z1))
        need = np.bincount(o1[dst], minlength=len(z1))
        deficit = int(np.maximum(need - have, 0).sum())
        fixed = int((z1[o1[dst]] == 1).sum())
        if deficit == 0:
            cert = s.B.jordan(G1, dst, seed=29)
            if not cert['missing'] and not cert['uncertifiable_below_8']:
                for o, v in cert['certificates'].items():
                    assert s.B.check_jordan(G1, v['word'], int(o), v['prime'])['verified']
                rows.append([i, fixed, deficit, 'PASS', len(cert['certificates'])])
                cap_pass = True
                break
        GM = group(np.flatnonzero(~(otherbad | qmust)))
        om, zm = GM.orbits()
        mustfixed, upperfail = int((zm[om[dst]] == 1).sum()), 0
        for oid in set(om[dst]):
            delta = np.flatnonzero((om == oid) & (~home[:, c]))
            counts = np.bincount(o0[delta], minlength=len(z0))
            upper, req = int(np.minimum(hc, counts).sum()), int((om[dst] == oid).sum())
            if upper < req:
                upperfail += req - upper
        verdict = 'NO_PREP' if mustfixed or upperfail else 'OPEN_PREP'
        if verdict == 'OPEN_PREP':
            opened.append(i)
        rows.append([i, fixed, deficit, verdict, mustfixed, upperfail])
    status = 'PASS' if cap_pass else ('OPEN_PREP' if opened else 'NO_PREP')
    totals[status] += 1
    if status == 'OPEN_PREP':
        open_items.append((c, opened))
    print('CAP_RESULT', c, status, 'units', units_in(u), 'pieces', len(u), rows, 'CPU', cpu(), flush=True)
print('SUMMARY', dict(totals), 'CPU', cpu(), flush=True)


def flow(caps_src, caps_mid, want_mid):
    """Max integer flow source orbit -> middle orbit; returns the value and the chosen chambers per source orbit."""
    na, nj = len(caps_src), len(want_mid)
    sink = na + nj + 1
    cap = np.zeros((sink + 1, sink + 1), np.int64)
    for a, h in enumerate(caps_src):
        cap[0, 1 + a] = h
        for b in range(nj):
            cap[1 + a, 1 + na + b] = caps_mid[a][b]
    for b, w in enumerate(want_mid):
        cap[1 + na + b, sink] = w
    res, value = cap.copy(), 0
    while True:
        prev = np.full(sink + 1, -1)
        prev[0] = 0
        queue = deque([0])
        while queue and prev[sink] < 0:
            a = queue.popleft()
            for b in np.flatnonzero(res[a] > 0):
                if prev[b] < 0:
                    prev[b] = a
                    queue.append(int(b))
        if prev[sink] < 0:
            break
        v, d = sink, 10 ** 9
        while v:
            a = prev[v]
            d = min(d, int(res[a, v]))
            v = a
        v = sink
        while v:
            a = prev[v]
            res[a, v] -= d
            res[v, a] += d
            v = a
        value += d
    used = cap - res
    return value, [[int(used[1 + a, 1 + na + b]) for b in range(nj)] for a in range(na)]


# Part B: every open cap with the inside mask empty, item by item until one passes.
for c, opened in open_items:
    cap = allcaps[c]
    u = off[ls[:, c] == 1]
    otherbad = (ls[ls[:, c] != 1] != 0).any(axis=0)
    for i in opened:
        g = s.restore.element(M, c, i)
        mg = im(g)
        bad = otherbad.copy()
        for q in cap:
            bad |= labels(mg, int(q)) != 0
        G1 = group(np.flatnonzero(~bad))
        o1, z1 = G1.orbits()
        dst = landing_chambers(g, u)
        fixed = int((z1[o1[dst]] == 1).sum())
        ais = np.flatnonzero(hc)
        js = np.unique(o1[dst])
        want = np.bincount(o1[dst], minlength=len(z1))
        outs = [np.flatnonzero((o0 == a) & (~home[:, c])) for a in ais]
        src_short = sum(max(0, int(hc[a]) - len(xs)) for a, xs in zip(ais, outs))
        if fixed or src_short:
            print('PREP_RESULT', c, i, 'FAIL', 'fixed', fixed, 'source_capacity_deficit', src_short, 'CPU', cpu(),
                  flush=True)
            continue
        value, used = flow([int(hc[a]) for a in ais], [[int((o1[xs] == j).sum()) for j in js] for xs in outs],
                           [int(want[j]) for j in js])
        if value < len(dst):
            print('PREP_RESULT', c, i, 'FAIL', 'flow', value, 'demand', len(dst), 'CPU', cpu(), flush=True)
            continue
        J = []
        for a, (ai, xs) in enumerate(zip(ais, outs)):
            chosen = []
            for b, j in enumerate(js):
                chosen.extend(xs[o1[xs] == j][:used[a][b]].tolist())
            chosen.extend(np.setdiff1d(xs, np.array(chosen, dtype=int))[:int(hc[ai]) - len(chosen)].tolist())
            J += chosen
        J = np.array(J)
        assert len(J) == len(H) and not home[J, c].any()
        assert (np.bincount(o1[J], minlength=len(z1)) >= want).all()
        missing = {}
        for side, GG, targets in [('source', G0, np.array([int(np.flatnonzero(o0 == a)[0]) for a in ais
                                                           if 0 < hc[a] < z0[a]], dtype=int)),
                                  ('middle', G1, dst)]:
            cert = s.B.jordan(GG, targets, seed=29)
            for o, v in cert['certificates'].items():
                assert s.B.check_jordan(GG, v['word'], int(o), v['prime'])['verified']
            missing[side] = [len(cert['certificates']), cert['missing'], cert['uncertifiable_below_8']]
        ok = all(not m[1] and not m[2] for m in missing.values())
        print('PREP_RESULT', c, i, 'PASS' if ok else 'OPEN', 'units', units_in(u), 'landed', len(u), 'flow', value,
              'source_orbits', len(ais), 'middle_orbits', len(js), 'certificates', missing, 'CPU', cpu(), flush=True)
        if ok:
            break
print('DONE', 'CPU', cpu(), flush=True)
