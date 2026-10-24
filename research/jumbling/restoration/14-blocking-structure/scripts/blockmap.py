# Stage 14 (blocking structure), tool 1: the exact blocking map of one state.
#
# Input: a pose table in the format of buffer.export_pose_table (the runner's start and step tables). For every piece
# its left tree height and branch (trees.py), and for every piece and every grip whether its posed region lies inside
# the cap (1), outside it (0) or straddles the cut (2). The label table is r22d82_unlock_graph.py's all_labels, exact,
# run over the grips in forked workers. From the table:
# - per grip: its class, its straddlers by height and by branch (own class or other), clean or blocked;
# - per clean grip: what its cap holds (lattice, own-class and other-class pieces by height) and whether it is pure
#   (no piece of another branch at height 2 or more inside, so that no menu twist of it raises a height above 2);
# - the matrix M[c, d] of pieces inside cap c that straddle cut d, and from it, exactly, the effect of every retained
#   twist (c, k), k in A4_c, at every clean grip on every straddler set (Lemma R of the stage README):
#   S_d after = (S_d minus the inside of c) plus k(pieces inside c that straddle k^-1(d));
# - per blocked grip: the clean grips whose caps hold all its straddlers, whether every straddler has a clean holder,
#   and whether one retained twist frees it;
# - per pose group of height 4 or more: its own-branch caps that hold it whole, clean, pure and blocked, with the
#   straddler counts of the blocked ones.
# Checks: no lattice piece straddles any cut; no piece of height 2 straddles a cut of its own branch's class (card 3,
# Theorem B); the table reproduces its digest. Exact geometry; research only, headless Linux; no Windows, DirectX,
# input or performance claim.
#
# Run from the repository root:
#   python3 -I -B research/jumbling/restoration/14-blocking-structure/scripts/blockmap.py <table.json> <out.json>
#       [--labels <cache.npz>] [--workers N] [--name NAME]
# --labels reads the label table from the cache when it exists and its digest matches, and writes it otherwise.
import argparse
import json
import os
import sys
import time
from pathlib import Path

import numpy as np

REPO = Path.cwd()
sys.path[:0] = [str(REPO / 'research/jumbling/theory'), str(REPO / 'research/jumbling'),
                str(REPO / 'research/jumbling/fixtures')]
import restore as R  # noqa: E402
import sim  # noqa: E402
import trees as T  # noqa: E402
from sim.kernel import Evaluator, pullback_form  # noqa: E402
from sim.kplus import matrix_from_json  # noqa: E402
from sim.state import Pose  # noqa: E402

T0 = time.time()
C = sim.get_context()
M = R.menu('I_a')
KP = C.kplus
CLS = np.array([T.branch(tuple(n)) for n in C.data.N], np.int8)


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)


def load_table(path):
    doc = json.loads(Path(path).read_text())
    st = sim.State(C, filtered=False, menu=M)
    st._poses, st._lookup = [Pose(0, None, ('K', 0))], {('K', 0): 0}
    remap = np.array([st._intern_matrix(matrix_from_json(m)) for m in doc['poses']], np.int32)
    st.pose_id = remap[np.asarray(doc['pose_id'], np.int64)]
    st._compact()
    st.journal, st._anchor_cache = [], {}
    assert st.digest() == doc['digest'], 'pose table does not reproduce its digest'
    return st


def tree_info(st):
    """Left height and branch of every piece (right height 0 is checked)."""
    hs = np.zeros(len(st._poses), np.int16)
    br = np.full(len(st._poses), -1, np.int8)
    for i, p in enumerate(st._poses):
        if p.kidx >= 0:
            continue
        lf, rf = T.factors(p.matrix)
        assert T.dist(rf) == 0
        hs[i], br[i] = T.dist(lf), T.branch(lf)
    return hs[st.pose_id], br[st.pose_id]


def _label_caps(st, caps, lp, k, groups):
    out = np.zeros((len(st.pose_id), len(caps)), np.uint8)
    inv = KP.inv[k[lp]]
    for j, c in enumerate(caps):
        out[lp, j] = C.data.in_sig(lp, KP.perms[inv, c])
        for pid, anchors in groups:
            ev = Evaluator(pullback_form(st._pose_matrix(pid), C.data.N[c]))
            for a, sub in anchors:
                lab = ev.classify(C.regions.cap_vform(a))[0]
                assert lab != 'uncertain'
                if lab != 'straddle':
                    if lab == 'in':
                        out[sub, j] = 1
                    continue
                for p in sub.tolist():
                    lab = ev.classify(C.regions.vform(p))[0]
                    assert lab != 'uncertain'
                    out[p, j] = {'in': 1, 'out': 0, 'straddle': 2}[lab]
    return out


def all_labels(st, workers=1):
    """n x 600 table: 0 outside the cap, 1 inside, 2 straddles the cut (exact)."""
    k = st._kpose()[st.pose_id]
    lp = np.flatnonzero(k >= 0)
    op = np.flatnonzero(k < 0)
    groups = [(pid, st._anchors(pid, mem)) for pid, mem in st._groups(op)]
    parts = [list(range(w, 600, workers)) for w in range(workers)]
    if workers == 1:
        res = [_label_caps(st, parts[0], lp, k, groups)]
    else:
        pipes = []
        for part in parts:
            r, w = os.pipe()
            pid = os.fork()
            if pid == 0:
                os.close(r)
                arr = _label_caps(st, part, lp, k, groups)
                with os.fdopen(w, 'wb') as f:
                    np.save(f, arr)
                os._exit(0)
            os.close(w)
            pipes.append((pid, r))
        res = []
        for pid, r in pipes:
            with os.fdopen(r, 'rb') as f:
                res.append(np.load(f))
            _, status = os.waitpid(pid, 0)
            assert status == 0, 'label worker failed'
    out = np.zeros((len(st.pose_id), 600), np.uint8)
    for part, arr in zip(parts, res):
        out[:, part] = arr
    return out


def labels_for(st, cache, workers):
    digest = st.digest()
    if cache and Path(cache).exists():
        z = np.load(cache)
        if str(z['digest']) == digest:
            return z['labels']
    lab = all_labels(st, workers)
    if cache:
        np.savez_compressed(cache, labels=lab, digest=np.array(digest))
    return lab


def by_height_branch(mask_rows, h, b, heights):
    """counts[H][B][cap] over rows of mask_rows (n x 600 bool) for each height H and branch B (-1 for lattice)."""
    out = {}
    for H in heights:
        for B in ([-1] if H == 0 else range(5)):
            sel = (h == H) & (b == B)
            if sel.any():
                out[(int(H), int(B))] = mask_rows[sel].sum(0).astype(np.int64)
    return out


def analyse(st, lab, h, b, name):
    n = len(h)
    off = h > 0
    heights = sorted(set(h.tolist()))
    S = lab == 2
    I = lab == 1
    assert not S[~off].any(), 'a lattice piece straddles a cut'
    nS = S.sum(0).astype(np.int64)
    clean = nS == 0
    strad = by_height_branch(S, h, b, heights)
    inside = by_height_branch(I, h, b, heights)
    for (H, B), v in strad.items():
        if H == 2:
            assert not v[CLS == B].any(), 'a height-2 piece straddles a cut of its own class'
    own_s = np.zeros(600, np.int64)
    other_s = np.zeros(600, np.int64)
    strad_h = {}
    for (H, B), v in strad.items():
        own = np.where(CLS == B, v, 0)
        own_s += own
        other_s += v - own
        strad_h[H] = strad_h.get(H, 0) + v
    other_in = np.zeros(600, np.int64)
    for (H, B), v in inside.items():
        if H >= 2:
            other_in += np.where(CLS == B, 0, v)
    pure = clean & (other_in == 0)

    # M[c, d]: pieces inside cap c that straddle cut d.
    A = I[off].astype(np.float32)
    B_ = S[off].astype(np.float32)
    Mx = np.rint(A.T @ B_).astype(np.int64)
    assert (Mx.diagonal() == 0).all() and (Mx[clean] <= nS).all()

    # Retained twists at clean grips.
    ret = []
    unlock_by = {}
    for c in np.flatnonzero(clean).tolist():
        for k in KP.a4(c):
            if k == 0:
                continue
            pre = KP.perms[int(KP.inv[k])].astype(np.int64)
            new = nS - Mx[c] + Mx[c, pre]
            un = np.flatnonzero((nS > 0) & (new == 0))
            nb = np.flatnonzero((nS == 0) & (new > 0))
            dec = int(((new < nS)).sum())
            inc = int(((new > nS)).sum())
            ret.append(dict(cap=c, k=int(k), unlocked=un.tolist(), newly_blocked=nb.tolist(),
                            caps_fewer=dec, caps_more=inc, total_after=int(new.sum())))
            for d in un.tolist():
                unlock_by.setdefault(d, []).append([c, int(k)])

    # Blocked grips: covers by one clean cap, clean holders of every straddler.
    has_clean_holder = (I & clean[None, :]).any(1)
    blocked = np.flatnonzero(~clean)
    cover = {}
    reach = {}
    for d in blocked.tolist():
        cov = np.flatnonzero(clean & (Mx[:, d] == nS[d]))
        cover[d] = cov.tolist()
        reach[d] = bool(has_clean_holder[S[:, d]].all())

    # High pose groups.
    high = []
    pid = st.pose_id
    for g in np.unique(pid[h >= 4]).tolist():
        mem = np.flatnonzero(pid == g)
        H, Bg = int(h[mem[0]]), int(b[mem[0]])
        whole = I[mem].all(0)
        own = np.flatnonzero(whole & (CLS == Bg))
        high.append(dict(pose=int(g), size=int(len(mem)), height=H, branch=Bg,
                         own_holders=len(own), own_clean=int(clean[own].sum()), own_pure=int(pure[own].sum()),
                         blocked_straddlers=sorted(int(nS[c]) for c in own if not clean[c])[:12],
                         other_holders_clean=int((whole & (CLS != Bg) & clean).sum())))

    hist = {}
    for H, B in zip(h.tolist(), b.tolist()):
        hist[f'{H}:{B}'] = hist.get(f'{H}:{B}', 0) + 1
    per_class = []
    for q in range(5):
        cq = CLS == q
        per_class.append(dict(cls=q, clean=int(clean[cq].sum()), pure=int(pure[cq].sum()),
                              straddlers_own_branch_high=int(own_s[cq].sum()),
                              straddlers_other_branch=int(other_s[cq].sum())))
    caps = []
    for c in range(600):
        row = dict(cap=c, cls=int(CLS[c]), straddlers=int(nS[c]), own_branch=int(own_s[c]),
                   other_branch=int(other_s[c]), by_height={str(H): int(v[c]) for H, v in strad_h.items() if v[c]})
        if clean[c]:
            ins = {}
            for (H, B), v in inside.items():
                key = 'lattice' if H == 0 else f'h{H}-{"own" if B == CLS[c] else "other"}'
                ins[key] = ins.get(key, 0) + int(v[c])
            row['inside'] = ins
            row['pure'] = bool(pure[c])
        else:
            row['clean_covers'] = cover[c][:20]
            row['n_clean_covers'] = len(cover[c])
            row['every_straddler_has_clean_holder'] = reach[c]
            row['freed_by_one_retained_twist'] = unlock_by.get(c, [])[:20]
        caps.append(row)
    best = sorted(ret, key=lambda r: (-(len(r['unlocked']) - len(r['newly_blocked'])), r['total_after']))[:30]
    summary = dict(
        name=name, digest=st.digest(), off=int(off.sum()),
        heights={str(H): int((h == H).sum()) for H in heights},
        height_branch=hist,
        clean=int(clean.sum()), pure=int(pure.sum()), blocked=int((~clean).sum()),
        straddle_pairs=int(nS.sum()),
        straddle_pairs_by_height={str(H): int(v.sum()) for H, v in strad_h.items()},
        straddle_pairs_own_branch=int(own_s.sum()), straddle_pairs_other_branch=int(other_s.sum()),
        straddlers_per_blocked_cap=np.percentile(nS[~clean], [0, 10, 25, 50, 75, 90, 100]).tolist() if (~clean).any() else [],
        per_class=per_class,
        blocked_with_one_clean_cover=int(sum(1 for d in cover if cover[d])),
        blocked_every_straddler_clean_held=int(sum(reach.values())),
        blocked_freed_by_one_retained_twist=len(unlock_by),
        retained_twists=len(ret),
        retained_twists_net_positive=int(sum(1 for r in ret if len(r['unlocked']) > len(r['newly_blocked']))),
        best_retained=best,
        high_groups=len(high),
        high_groups_with_clean_own_holder=int(sum(1 for g in high if g['own_clean'])),
        high_groups_with_pure_own_holder=int(sum(1 for g in high if g['own_pure'])),
        high_pieces_with_pure_own_holder=int(sum(g['size'] for g in high if g['own_pure'])),
    )
    return summary, caps, high


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument('table')
    ap.add_argument('out')
    ap.add_argument('--labels')
    ap.add_argument('--workers', type=int, default=1)
    ap.add_argument('--name', default=None)
    a = ap.parse_args(argv)
    st = load_table(a.table)
    log(event='loaded', digest=st.digest(), off=int(st.off_lattice_count()))
    h, b = tree_info(st)
    log(event='trees', heights={str(H): int((h == H).sum()) for H in sorted(set(h.tolist()))})
    lab = labels_for(st, a.labels, a.workers)
    log(event='labels')
    summary, caps, high = analyse(st, lab, h, b, a.name or Path(a.table).name)
    Path(a.out).write_text(json.dumps(dict(summary=summary, caps=caps, high=high), indent=1))
    log(event='done', **{k: summary[k] for k in ('clean', 'pure', 'blocked', 'straddle_pairs',
                                                 'blocked_freed_by_one_retained_twist', 'high_groups',
                                                 'high_groups_with_pure_own_holder')})


if __name__ == '__main__':
    main()
