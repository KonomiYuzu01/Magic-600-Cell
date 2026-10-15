"""Transport run: restore blocks of one state with block words carried from other states by K+ (Lemma TT).

Inputs: a target state (pose table), its block list (blocks.py output) with the census shape keys, and template
sources: blocks of other states, each with candidate words {P, Q} whose ghost put that source block on K. For every
target block that has the shape of a source block, the exact K+ element m that maps the source regions onto the
target regions is found (float centroids, then exact region equality, as transport.py) and every source word is
carried letter by letter (e3lib.Lib.conj_letter). Then, repeatedly:
1. score every carried word at the current state by the exact frame ghost of all off pieces and of the chambers of
   its caps: T on K at the end, other off pieces alive and at their exact contents, Bad(W) and how many of its
   chambers hold lattice pieces;
2. take a clean word (T on K, no other off piece killed or moved, Bad(W) vacant: then V is empty and Lemma V applies
   with V = 1) of smallest Bad(W), replay it on the simulator (every twist checked admissible), and check the end:
   the block on the lattice, collateral 0, every other off piece at its exact pose;
3. after a step, the clean words found for other blocks are checked again at the new state (one ghost each) and
   replayed while clean; when none is left, every carried word is scored again; the run stops when a full scoring
   pass finds no clean word.
For every block that is not restored the run records why: no shape match, the carried words' grips blocked in the
state (a straddling off piece), the blocks whose pieces they disturb, or Bad(W) occupied.

usage: fxrun.py <research/jumbling> <E-chain scripts> <e3lib dir> <classes json> <arrays npz> <table> <blocks json>
       <census json> <census state name> <templates json> <out dir> [--max-words N] [--start-step K]
Blocks with a piece already on the lattice at the start are skipped (for continuing from a later table).
templates json: [{"shape": key prefix, "table": source table, "block": source target json, "rows": [jsonl files],
                  "max": N}]
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
J, SC, LIBDIR, CLS, ARR, TABLE, BLOCKS, CENSUS, CNAME, TEMPL, OUT = sys.argv[1:12]
a = sys.argv[12:]
MAXW = int(a[a.index('--max-words') + 1]) if '--max-words' in a else 40
STEP0 = int(a[a.index('--start-step') + 1]) if '--start-step' in a else 0
TH = Path(J).resolve() / 'theory'
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(Path(SC).resolve()))
sys.path.insert(0, str(Path(LIBDIR).resolve()))
import buffer as BU  # noqa: E402
import conjclose as CC  # noqa: E402
import conjprep as CP  # noqa: E402
import fastghost as FG  # noqa: E402
import ghostlib as GL  # noqa: E402
import rim  # noqa: E402
from e3lib import Lib  # noqa: E402
from exact import matvec  # noqa: E402
from sim.kplus import matrix_from_json, to_tuple  # noqa: E402

T0 = time.time()
OUT = Path(OUT)
OUT.mkdir(parents=True, exist_ok=True)
LOG = OUT / 'run.jsonl'


def log(**kw):
    kw['sec'] = round(time.time() - T0)
    with LOG.open('a') as fh:
        fh.write(json.dumps(kw) + '\n')
    print(json.dumps(kw), flush=True)


L = Lib(J, CLS, ARR)
reg, kp = L.reg, L.kp
repVf = [np.array([[float(x) for x in v] for v in vs]) for vs in reg.rep_vertices]


def load_poses(table, pieces):
    tab = json.load(open(table))
    return {p: matrix_from_json(tab['poses'][tab['pose_id'][p]]) for p in pieces}


def centroids(ps, mats):
    out = []
    for p in ps:
        V = repVf[L.orb[p]] @ L.Kf[L.tr[p]].T @ L.fm(mats[p]).T
        out.append(V.mean(0))
    return np.array(out)


def exact_region(p, X, pre=None):
    home = [matvec(kp.matrix(int(L.tr[p])), v) for v in reg.rep_vertices[int(L.orb[p])]]
    pts = [matvec(X, w) for w in home]
    if pre is not None:
        pts = [matvec(pre, w) for w in pts]
    return frozenset(tuple(repr(x) for x in w) for w in pts)


def find_m(sps, smat, tps, tmat):
    """K+ elements m (exactly confirmed) with m(source regions) = target regions, bijectively."""
    if len(sps) != len(tps):
        return []
    Cs, Ct = centroids(sps, smat), centroids(tps, tmat)
    tkey = {tuple(np.round(c, 5).tolist()): j for j, c in enumerate(Ct)}
    res = []
    for m in range(7200):
        img = Cs @ L.Kf[m].T
        match = [tkey.get(tuple(np.round(c, 5).tolist())) for c in img]
        if all(x is not None for x in match) and len(set(match)) == len(match):
            Km = kp.matrix(m)
            if all(exact_region(p, smat[p], Km) == exact_region(tps[j], tmat[tps[j]]) for p, j in zip(sps, match)):
                res.append(m)
    return res


# target state
R, M = GL.O.start(TABLE)
st = R.st
F = FG.Frames()
codes = CC.Codes()
_el = {}


def E(c, i):
    if (c, i) not in _el:
        _el[(c, i)] = GL.restore.element(M, c, i)
    return _el[(c, i)]


def contents_all():
    D = rim.Domains(st)
    dmap = {j: F.domain_of(rep) for j, rep in enumerate(D.reps)}
    off = np.flatnonzero(st._kpose()[st.pose_id] < 0)
    dom = np.zeros(len(off), np.int64)
    q = np.zeros(len(off), np.int64)
    for n, p in enumerate(off.tolist()):
        d2, k = dmap[int(D.dom[p])]
        dom[n] = d2
        q[n] = int(FG.chamber_image(k, np.array([D.cham[p]]))[0])
    lat = np.flatnonzero(D.dom == 0)
    occ = np.zeros(GL.NP, bool)
    occ[D.cham[lat]] = True
    return off, dom, q, occ


blocks = json.load(open(BLOCKS))['blocks']
census = json.load(open(CENSUS))['states'][CNAME]['blocks']
assert len(blocks) == len(census)
templates = json.load(open(TEMPL))
tposes = load_poses(TABLE, [p for b in blocks for p in b['pieces']])
targets = []   # (block index, pieces, [(template index, m, words)])
for bi, (b, cb) in enumerate(zip(blocks, census)):
    pcs = sorted(int(p) for p in b['pieces'])
    srcs = []
    for ti, t in enumerate(templates):
        if not cb['shape'].startswith(t['shape']):
            continue
        sps = sorted(int(p) for p in json.load(open(t['block']))['pieces'])
        ms = find_m(sps, load_poses(t['table'], sps), pcs, {p: tposes[p] for p in pcs})
        if not ms:
            continue
        words, seen = [], set()
        for f in t['rows']:
            for line in open(f):
                r = json.loads(line)
                if r.get('T_on') != len(sps) or r.get('T_dead', 0):
                    continue
                key = json.dumps([r.get('P', []), r['Q']])
                if key in seen:
                    continue
                seen.add(key)
                words.append({'P': r.get('P', []), 'Q': r['Q']})
        words = words[:t.get('max', MAXW)]
        for m in ms:
            carried = [{'P': [list(L.conj_letter(m, (int(c), int(i)))) for c, i in w['P']],
                        'Q': [list(L.conj_letter(m, (int(c), int(i)))) for c, i in w['Q']]} for w in words]
            srcs.append((ti, m, carried))
    targets.append((bi, pcs, srcs))
log(event='targets', with_template=[bi for bi, _, s in targets if s],
    words=sum(len(c) for _, _, s in targets for _, _, c in s))
piece_block = {p: bi for bi, b in enumerate(blocks) for p in b['pieces']}


def score(W, pcs, S):
    off, dom, q, occ, pos = S
    tix = np.array([pos[p] for p in pcs], np.int64)
    mask = np.ones(len(off), bool)
    mask[tix] = False
    grips = sorted({c for c, _ in W})
    blocked = [c for c in grips if R.inside(c) is None]
    U = np.flatnonzero(np.logical_or.reduce([GL.cap(c) for c in grips]))
    g = FG.Ghost(F, np.concatenate([dom, np.zeros(len(U), np.int64)]), np.concatenate([q, U]))
    for c, i in W:
        g.apply(c, E(c, i), (c, i))
    n = len(off)
    T_on = int((g.alive[tix] & (g.dom[tix] == 0)).sum())
    bad_p = (~g.alive[:n] | (g.dom[:n] != dom) | (g.q[:n] != q)) & mask
    lat = slice(n, None)
    badW = U[~g.alive[lat] | (g.dom[lat] != 0)]
    occb = int(occ[badW].sum())
    conf = sorted({piece_block.get(int(p), -1) for p in off[bad_p].tolist()})
    row = {'T_on': T_on, 'T': len(pcs), 'blocked': blocked, 'disturbed': int(bad_p.sum()), 'conflict_blocks': conf,
           'badW': int(len(badW)), 'badW_occupied': occb}
    return row, (T_on == len(pcs) and not bad_p.any() and occb == 0)


def state():
    off, dom, q, occ = contents_all()
    return off, dom, q, occ, {int(p): n for n, p in enumerate(off.tolist())}


def replay(bi, pcs, W, row, step):
    c_s = codes.of(st)
    d_s = st.digest()
    poses_s = {int(p): to_tuple(st.pose(int(p))) for p in np.flatnonzero(c_s < 0).tolist()}
    for c, i in W:
        R.apply(c, i)
    c_e = codes.of(st)
    inT = np.zeros(len(c_s), bool)
    inT[pcs] = True
    off_s, off_e = c_s < 0, c_e < 0
    end = {'digest': st.digest(), 'off': int(off_e.sum()), 'T_restored': int((inT & ~off_e).sum()),
           'collateral': int((~off_s & off_e).sum()),
           'protected_exact_pose_changed': sum(1 for p, mm in poses_s.items() if not inT[p]
                                               and (not off_e[p] or to_tuple(st.pose(p)) != mm)),
           'lattice_of_S_on_K_at_end': int((~off_s & ~off_e).sum()), 'lattice_of_S': int((~off_s).sum())}
    ok = end['T_restored'] == len(pcs) and end['collateral'] == 0 and end['protected_exact_pose_changed'] == 0
    (OUT / f'step{step:02d}-word.json').write_text(json.dumps({'word': [list(x) for x in W], 'input_digest': d_s,
                                                               'output_digest': end['digest'], 'block': bi,
                                                               'pieces': pcs, 'template': row['template'],
                                                               'm': row['m']}) + '\n')
    log(event='step', step=step, block=bi, size=len(pcs), template=row['template'], m=row['m'], length=len(W),
        badW=row['badW'], input_digest=d_s, end=end, ok=ok)
    return ok


restored = set()
step = STEP0
S = state()
targets = [t for t in targets if all(p in S[4] for p in t[1]) or log(event='skip', block=t[0], reason='not all off')]
while True:
    # full scoring pass
    clean, why = {}, {}
    for bi, pcs, srcs in targets:
        if bi in restored or not srcs:
            continue
        reasons = []
        for ti, m, carried in srcs:
            for w in carried:
                W = [tuple(x) for x in w['P']] + [tuple(x) for x in w['Q']] + CP.inverse_word(M, [tuple(x) for x in w['P']])
                row, ok = score(W, pcs, S)
                row.update(template=ti, m=m)
                reasons.append(row)
                if ok:
                    clean.setdefault(bi, []).append((row['badW'], W, row))
        why[bi] = sorted(reasons, key=lambda r: (len(r['blocked']), r['disturbed'], r['badW_occupied']))[:3]
    log(event='pass', step=step, clean_blocks=sorted(clean), remaining=len([t for t in targets if t[0] not in restored]))
    if not clean:
        log(event='stop', step=step, restored=sorted(restored), remaining={str(k): v for k, v in why.items()})
        break
    pcs_of = {bi: pcs for bi, pcs, _ in targets}
    progressed = True
    stop = False
    while progressed:
        progressed = False
        for bi in sorted(clean, key=lambda b: min(x[0] for x in clean[b])):
            if bi in restored:
                continue
            for _, W, row in sorted(clean[bi], key=lambda x: x[0]):
                row2, ok = score(W, pcs_of[bi], S)
                if not ok:
                    continue
                if not replay(bi, pcs_of[bi], W, dict(row, **row2), step):
                    stop = True
                    break
                restored.add(bi)
                step += 1
                BU.export_pose_table(st, str(OUT / f'step{step:02d}-table.json'))
                S = state()
                progressed = True
                break
            if stop or progressed:
                break
        if stop:
            break
    if stop:
        break
