"""3E pipeline on one input pose table (menu I_a): seed table + conflict graph, current state only.

Reads the input pose table (checked against its digest), the fixed model and the seed library (single words from
solved; never the input's history). Steps:
 1. Off pieces, depth-one test and position class (e3lib.canon).
 2. Candidates: for each off piece p, each seed q of p's class and each m in K+ with m(R_q) = R_p: the restoration
    word w = m W_q^-1 m^-1 (letterwise, exact), its grip set and its needed vacancy m(h_q) (Theorem C).
 3. Cap status of every off piece for every grip a candidate uses (float on exact vertices, every status that a
    reported fact depends on confirmed by sim.kernel.Evaluator).
 4. Sequential restoration (Theorem C, sinks of the conflict digraph first): repeatedly take a piece with a candidate
    whose grips leave every other remaining off piece outside their caps and whose needed vacancy is vacant or can be
    made vacant by a SAFE word (retained letters at grips whose caps hold no off piece, breadth-first). With --exec the
    preparation and the word are applied on the exact simulator and checked: every twist admissible, p on the lattice,
    every other off piece at its exact pose, every lattice piece still on the lattice.
 5. For what remains, a blocking certificate per piece: not depth one / no seed in the library for its class /
    every candidate conflicts (the pieces and one exactly confirmed grip each) / no preparation (needed vacancy outside
    the SAFE orbit of every vacancy of its orbit). The conflict digraph of the chosen candidates and its cycles.
usage: e3pipe.py <research/jumbling> <classes.json> <arrays.npz> <seeds.json> <pose table json> <out json>
                 [--exec] [--name NAME] [--max-steps N] [--kseeds kseeds.json]
k-seeds (kseeds.py): words from solved ending with k off pieces. A k-seed matches when one m in K+ maps each of its
off regions onto the region of a distinct off piece of the input (exact: K_kp m K_kq^-1 lies in K+ ∩ gamma K+ gamma^-1
and its sigma maps s_q to s_p, for every pair); its word m W^-1 m^-1 then restores those pieces together (Lemma R),
provided the m-images of its k holes are vacant and every other off piece is outside every cap of its grips. Blocks
are tried only when no single makes progress."""
import argparse
import json
import sys
import time
from collections import Counter, defaultdict
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from e3lib import REGNAMES, Lib  # noqa: E402

ap = argparse.ArgumentParser()
ap.add_argument('J'), ap.add_argument('classes'), ap.add_argument('arrays'), ap.add_argument('seeds')
ap.add_argument('table'), ap.add_argument('out')
ap.add_argument('--exec', action='store_true'), ap.add_argument('--name', default=None)
ap.add_argument('--max-steps', type=int, default=10 ** 6)
ap.add_argument('--kseeds', default=None), ap.add_argument('--save-end', default=None)
a = ap.parse_args()
t0 = time.time()
L = Lib(a.J, a.classes, a.arrays)
sim, d, reg, kp = L.sim, L.d, L.reg, L.kp
from sim.kplus import matrix_from_json  # noqa: E402
from sim.state import Pose  # noqa: E402
from sim.kplus import matrix_json  # noqa: E402

TOL = 1e-7
NF = np.array(d.NF, float)
KAP = float(d.KAPPA)
repV = [np.array([[float(v) for v in vert] for vert in vs]) for vs in reg.rep_vertices]


def load_table(path):
    doc = json.loads(Path(path).read_text())
    st = sim.State(L.ctx, menu=L.M)
    st._poses, st._lookup = [Pose(0, None, ('K', 0))], {('K', 0): 0}
    ids = np.array([st._intern_matrix(matrix_from_json(m)) for m in doc['poses']], np.int32)
    st.pose_id = ids[np.asarray(doc['pose_id'], np.int64)]
    st._compact()
    st.journal, st._anchor_cache = [], {}
    assert st.digest() == doc['digest'], 'pose table does not reproduce its digest'
    return st, doc['digest']


st, digest0 = load_table(a.table)
name = a.name or Path(a.table).stem
seeds = json.load(open(a.seeds))
by_class = defaultdict(list)
for q in seeds:
    by_class[q['class']].append(q)
TW = {}


def tw(l):
    l = tuple(l)
    if l not in TW:
        TW[l] = sim.Twist(L.ctx, l[0], L.RS.element(L.M, l[0], l[1]), 'menu', {'menu': L.M.name})
    return TW[l]


def off_pieces():
    return np.flatnonzero(st._kpose()[st.pose_id] < 0)


def region_vertices(p):
    X = L.fm(st.pose(int(p)))
    home = repV[L.orb[p]] @ L.Kf[L.tr[p]].T
    return home @ X.T


def float_status(V, grips):
    vals = V @ NF[grips].T - KAP
    ins = (vals > -TOL).all(0)
    out = (vals < TOL).all(0)
    return np.where(ins & ~out, 1, np.where(out & ~ins, 0, 2))  # 0 out, 1 in, 2 straddle/unclear


def exact_out(p, c):
    return L.status(st, int(p), int(c)) == 'out'


# ---------------------------------------------------------------- 1. off pieces
O0 = off_pieces()
info = {}
pcache = {}
for p in O0.tolist():
    pid = int(st.pose_id[p])
    if pid not in pcache:
        pcache[pid] = L.canon(p, L.fm(st._pose_matrix(pid)))   # (k, r, s) for this p; k, r depend on the pose only
        pcache[pid] = None if pcache[pid] is None else pcache[pid][:2]
    c = pcache[pid]
    if c is None:
        info[p] = {'piece': p, 'orbit': int(L.orb[p]), 'depth_one': False}
        continue
    k, rr = c
    s = int(L.act(rr, [p])[0])
    row = {'piece': p, 'orbit': int(L.orb[p]), 'depth_one': True, 'k': int(k), 'r': int(rr)}
    row.update(L.describe(s))
    info[p] = row
print(name, 'off', len(O0), 'depth one', sum(r['depth_one'] for r in info.values()), '%.0fs' % (time.time() - t0),
      flush=True)
# ---------------------------------------------------------------- 2. candidates
cands = defaultdict(list)
for p, r in info.items():
    if not r['depth_one']:
        continue
    for q in by_class.get(r['class'], []):
        for m in L.conjugators(q, r['k'], r['position']):
            grips = sorted({int(L.perms[m, c]) for c in q['grips']})
            need = int(L.act(m, [q['hole']])[0])
            cands[p].append({'seed': q['name'], 'm': int(m), 'length': q['length'], 'grips': grips, 'need': need})
print('candidates', sum(len(v) for v in cands.values()), 'for', len(cands), 'pieces', '%.0fs' % (time.time() - t0),
      flush=True)
TSIG = {int(t_): int(s_) for t_, s_ in L.Spairs.tolist()}
kseeds = json.load(open(a.kseeds)) if a.kseeds else []
kcands = []
for ks in kseeds:
    q1 = ks['off'][0]
    for p, r in info.items():
        if not r['depth_one'] or r['class'] != q1['class']:
            continue
        for m in L.conjugators({'s': q1['s'], 'k': q1['k']}, r['k'], r['position']):
            assign = [p]
            for qi in ks['off'][1:]:
                hit = None
                for p2, r2 in info.items():
                    if p2 in assign or not r2['depth_one'] or r2['class'] != qi['class']:
                        continue
                    sg = TSIG.get(L.comp(r2['k'], L.comp(m, L.inv[qi['k']])))
                    if sg is not None and int(L.act(sg, [qi['s']])[0]) == r2['position']:
                        hit = p2
                        break
                if hit is None:
                    break
                assign.append(hit)
            if len(assign) == ks['k_off']:
                kcands.append({'kseed': ks['name'], 'm': int(m), 'pieces': assign, 'length': ks['length'],
                               'grips': sorted({int(L.perms[m, c]) for c in ks['grips']}),
                               'needs': [int(x) for x in L.act(m, ks['holes'])]})
print('k-seed candidates', len(kcands), [(c['kseed'][:8], c['pieces']) for c in kcands][:10], flush=True)
# ---------------------------------------------------------------- 3. float cap status
allgrips = np.arange(600)
FS = {}
BIG = len(O0) > 5000
if BIG:
    print('more than 5000 off pieces: classification only, no conflict analysis', flush=True)
    cands = {}
    kcands = []
for p in ([] if BIG else O0.tolist()):
    FS[p] = float_status(region_vertices(p), allgrips)
safe_grips_float = lambda O: np.flatnonzero(np.all([FS[p] == 0 for p in O], axis=0)) if len(O) else allgrips


def conflicts(cand, O, p):
    """Remaining off pieces (other than p) not outside some cap of the candidate's grips (float)."""
    g = np.array(cand['grips'])
    return [int(x) for x in O if x != p and (FS[x][g] != 0).any()]


def confirm_clean(cand, O, p):
    """Exact: every remaining off piece other than p is outside every cap of the candidate's grips."""
    for x in O:
        if x == p:
            continue
        for c in cand['grips']:
            if not exact_out(x, c):
                return False, (int(x), int(c), L.status(st, int(x), int(c)))
    return True, None


def witness(x, cand):
    for c in cand['grips']:
        s = L.status(st, int(x), int(c))
        if s != 'out':
            return [int(c), s]
    return None


# ---------------------------------------------------------------- preparation (SAFE BFS)
RET = {}


def retained_letters(c):
    if c not in RET:
        RET[c] = [(int(c), i, L.kidx_letter(int(c), i)) for i in L.RI]
    return RET[c]


def prep_word(need, O, max_depth=12):
    """Shortest SAFE word moving some vacancy of need's orbit to need (None if impossible within the depth)."""
    vac = L.vacancies(st)
    o = int(L.orb[need])
    vo = vac[L.orb[vac] == o]
    if need in set(vo.tolist()):
        return [], 0
    safe = [int(c) for c in safe_grips_float(O)]
    parent = {int(z): None for z in vo.tolist()}
    frontier = np.array(sorted(parent), np.int64)
    for depth in range(max_depth):
        if not len(frontier):
            return None, depth
        nxt = []
        for c in safe:
            inc = frontier[L.in_cap_chambers(c, frontier)]
            if not len(inc):
                continue
            for (cc, i, k) in retained_letters(c):
                img = L.act(k, inc)
                for z, z2 in zip(inc.tolist(), img.tolist()):
                    if z2 not in parent:
                        parent[z2] = (z, (cc, i))
                        nxt.append(z2)
                        if z2 == need:
                            path, x = [], z2
                            while parent[x] is not None:
                                x, l = parent[x]
                                path.append(l)
                            return path[::-1], depth + 1
        frontier = np.array(sorted(set(nxt)), np.int64)
    return None, max_depth


def word_of(cand):
    q = next(s for s in seeds if s['name'] == cand['seed'])
    inv = [L.inv_letter(tuple(l)) for l in reversed(q['word'])]
    return [L.conj_letter(cand['m'], l) for l in inv]


def prep_multi(needs, O, max_depth=12):
    """SAFE word placing a vacancy of the right orbit at every chamber of needs, one after another; grips whose caps
    hold an already placed chamber are not used again, so placed vacancies stay. None if a step fails."""
    vac = set(L.vacancies(st).tolist())
    safe = [int(c) for c in safe_grips_float(O)]
    word, placed = [], []
    for need in needs:
        if need in vac:
            placed.append(need)
            continue
        o = int(L.orb[need])
        grips = [c for c in safe if not (placed and L.in_cap_chambers(c, placed).any())]
        starts = [z for z in vac if int(L.orb[z]) == o and z not in placed]
        parent = {z: None for z in starts}
        frontier = np.array(sorted(parent), np.int64)
        path = None
        for depth in range(max_depth):
            if not len(frontier) or path is not None:
                break
            nxt = []
            for c in grips:
                inc = frontier[L.in_cap_chambers(c, frontier)]
                if not len(inc):
                    continue
                for (cc, i, k) in retained_letters(c):
                    for z, z2 in zip(inc.tolist(), L.act(k, inc).tolist()):
                        if z2 not in parent:
                            parent[z2] = (z, (cc, i))
                            nxt.append(z2)
                            if z2 == need:
                                path, x = [], z2
                                while parent[x] is not None:
                                    x, l = parent[x]
                                    path.append(l)
                                path = path[::-1]
                                break
                    if path is not None:
                        break
                if path is not None:
                    break
            frontier = np.array(sorted(set(nxt)), np.int64)
        if path is None:
            return None
        for (cc, i) in path:                       # move every vacancy in the cap, as the letter does
            k = L.kidx_letter(cc, i)
            vs = np.array(sorted(vac), np.int64)
            inc = L.in_cap_chambers(cc, vs)
            vac = set(vs[~inc].tolist()) | set(L.act(k, vs[inc]).tolist())
        assert need in vac and all(x in vac for x in placed)
        word += path
        placed.append(need)
    return word


def kword_of(kc):
    ks = next(x for x in kseeds if x['name'] == kc['kseed'])
    inv = [L.inv_letter(tuple(l)) for l in reversed(ks['word'])]
    return [L.conj_letter(kc['m'], l) for l in inv]


# ---------------------------------------------------------------- 4. sequential restoration
O = [int(p) for p in O0.tolist()]
steps, total = [], 0
while len(steps) < a.max_steps:
    progress = False
    order = sorted(cands, key=lambda p: min((c['length'] for c in cands[p]), default=10 ** 9))
    for p in order:
        if p not in O:
            continue
        for cand in sorted(cands[p], key=lambda c: (len(conflicts(c, O, p)), c['length'])):
            if conflicts(cand, O, p):
                break
            ok, bad = confirm_clean(cand, O, p)
            if not ok:
                continue
            pw, _ = prep_word(cand['need'], O)
            if pw is None:
                continue
            w = word_of(cand)
            rec = {'piece': p, 'class': info[p]['class'], 'registry_orbit': info[p]['registry_orbit'],
                   'seed': cand['seed'], 'm': cand['m'], 'prep_length': len(pw), 'word_length': len(w),
                   'need': cand['need'], 'off_before': len(O)}
            if a.exec:
                # every SAFE grip used by the preparation is confirmed exactly
                for (c, i) in pw:
                    assert all(exact_out(x, c) for x in O), 'SAFE grip not exact'
                before = {x: st.pose(x) for x in O if x != p}
                lat0 = st._kpose()[st.pose_id] >= 0
                for n, l in enumerate(list(pw) + list(w)):
                    r_ = st.apply(tw(l), journal=False)
                    if not r_.applied:
                        raise SystemExit(f'step at piece {p}: twist {n} not admissible')
                lat = st._kpose()[st.pose_id] >= 0
                assert lat[p], 'target not restored'
                assert all(st.pose(x) == before[x] for x in before), 'another off pose changed'
                assert lat[lat0].all(), 'a lattice piece went off'
                rec['end_digest'] = st.digest()
                rec['prep'] = [list(x) for x in pw]
                rec['word'] = [list(x) for x in w]
                L.clear_status_cache()
            steps.append(rec)
            total += len(pw) + len(w)
            O.remove(p)
            print('restored', {k_: rec[k_] for k_ in rec if k_ not in ('prep', 'word')}, '%.0fs' % (time.time() - t0),
                  flush=True)
            progress = True
            break
        if progress:
            break
    if not progress:
        for kc in sorted(kcands, key=lambda c: c['length']):
            if not all(x in O for x in kc['pieces']):
                continue
            others = [x for x in O if x not in kc['pieces']]
            g = np.array(kc['grips'])
            if any((FS[x][g] != 0).any() for x in others):
                continue
            if not all(exact_out(x, c) for x in others for c in kc['grips']):
                continue
            pw = prep_multi(kc['needs'], O)
            if pw is None:
                continue
            w = kword_of(kc)
            rec = {'block': kc['pieces'], 'kseed': kc['kseed'], 'm': kc['m'], 'prep_length': len(pw),
                   'word_length': len(w), 'needs': kc['needs'], 'off_before': len(O)}
            if a.exec:
                for (c, i) in pw:
                    assert all(exact_out(x, c) for x in O), 'SAFE grip not exact'
                before = {x: st.pose(x) for x in others}
                lat0 = st._kpose()[st.pose_id] >= 0
                for n, l in enumerate(list(pw) + list(w)):
                    if not st.apply(tw(l), journal=False).applied:
                        raise SystemExit(f'block {kc["pieces"]}: twist {n} not admissible')
                lat = st._kpose()[st.pose_id] >= 0
                assert all(lat[x] for x in kc['pieces']), 'block not restored'
                assert all(st.pose(x) == before[x] for x in before), 'another off pose changed'
                assert lat[lat0].all(), 'a lattice piece went off'
                rec['end_digest'] = st.digest()
                rec['prep'] = [list(x) for x in pw]
                rec['word'] = [list(x) for x in w]
                L.clear_status_cache()
            steps.append(rec)
            total += len(pw) + len(w)
            for x in kc['pieces']:
                O.remove(x)
            print('restored block', {k_: rec[k_] for k_ in rec if k_ not in ('prep', 'word')}, '%.0fs' % (time.time() - t0),
                  flush=True)
            progress = True
            break
    if not progress or not O:
        break
# ---------------------------------------------------------------- 5. blocking certificate
block = []
edges = {}
for p in O:
    r = info[p]
    if not r['depth_one']:
        block.append({'piece': p, 'reason': 'not depth one'})
        continue
    if BIG:
        block.append({'piece': p, 'reason': 'depth one, not analysed (more than 5000 off pieces)', 'class': r['class'],
                      'registry_orbit': r['registry_orbit'], 'has_seed': r['class'] in by_class})
        continue
    if not cands.get(p):
        block.append({'piece': p, 'reason': 'no seed for its class', 'class': r['class'],
                      'registry_orbit': r['registry_orbit']})
        continue
    rows = []
    for cand in cands[p]:
        cf = conflicts(cand, O, p)
        row = {'seed': cand['seed'], 'm': cand['m'], 'conflicts': cf}
        if cf:
            row['witness'] = {str(x): witness(x, cand) for x in cf[:5]}
        else:
            pw, dep = prep_word(cand['need'], O)
            row['preparation'] = None if pw is None else len(pw)
        rows.append(row)
    best = min(cands[p], key=lambda c: (len(conflicts(c, O, p)), c['length']))
    edges[p] = conflicts(best, O, p)
    block.append({'piece': p, 'reason': 'conflict or preparation', 'class': r['class'],
                  'registry_orbit': r['registry_orbit'], 'candidates': rows})
# cycles of the chosen-candidate digraph (strongly connected components of size > 1)
idx = {}
low, stack, onst, sccs, cnt = {}, [], set(), [], [0]


def strong(v):
    idx[v] = low[v] = cnt[0]
    cnt[0] += 1
    stack.append(v)
    onst.add(v)
    for w_ in edges.get(v, []):
        if w_ not in edges:
            continue
        if w_ not in idx:
            strong(w_)
            low[v] = min(low[v], low[w_])
        elif w_ in onst:
            low[v] = min(low[v], idx[w_])
    if low[v] == idx[v]:
        comp = []
        while True:
            x = stack.pop()
            onst.discard(x)
            comp.append(x)
            if x == v:
                break
        if len(comp) > 1:
            sccs.append(sorted(comp))


sys.setrecursionlimit(100000)
for v in edges:
    if v not in idx:
        strong(v)
summary = {
    'input': name, 'input_digest': digest0, 'executed': a.exec, 'off_start': len(O0),
    'depth_one': sum(r['depth_one'] for r in info.values()),
    'by_registry_orbit': dict(Counter(r.get('registry_orbit', 'deep') for r in info.values())),
    'classes_present': len({r['class'] for r in info.values() if r['depth_one']}),
    'classes_with_seed': len({r['class'] for r in info.values() if r['depth_one'] and r['class'] in by_class}),
    'pieces_with_seed': sum(1 for r in info.values() if r['depth_one'] and r['class'] in by_class),
    'restored': len(O0) - len(O), 'steps': len(steps), 'block_steps': sum(1 for x in steps if 'block' in x),
    'remaining': len(O), 'total_letters': total, 'kseed_candidates': len(kcands),
    'end_digest': st.digest() if a.exec else None,
    'blocking': Counter(b['reason'] for b in block), 'conflict_cycles': sccs,
    'seconds': round(time.time() - t0, 1)}
out = {'summary': summary, 'pieces': list(info.values()), 'steps': steps, 'blocking': block,
       'conflict_edges': {str(k): v for k, v in edges.items()}}
json.dump(out, open(a.out, 'w'), indent=1, default=int)
if a.exec and a.save_end:
    Path(a.save_end).write_text(json.dumps({'poses': [matrix_json(st._pose_matrix(i)) for i in range(len(st._poses))],
                                            'pose_id': st.pose_id.tolist(), 'digest': st.digest(), 'journal': []}) + '\n')
print(json.dumps(summary, default=int))
