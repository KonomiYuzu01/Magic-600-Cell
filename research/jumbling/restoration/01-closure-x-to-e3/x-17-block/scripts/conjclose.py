"""Conjugate closure of a window P Q P^-1 [tail] of a word, read piece class by piece class.

The window is word[s:e] replayed from the current-only pose table of prefix s. It is split as P (|P| twists), Q
(|Q| twists), Pbar (|P| twists, checked to be the exact mirror inverse of P: same grip, product the identity) and a
tail (the rest, possibly empty). Classes are fixed from the replay: at s, off pieces split into the target
(on the lattice at the end) and the protected pieces (off at the end; their exact poses are compared); lattice pieces
of s split, after P, into C (taken off by P) and L (still on the lattice after P). For each twist the script records
how many pieces of each class its cap selects, and the lattice flags before it. For Q it records the net-moved set
M (exact pose after Q differs from the exact pose after P) per class.

Closure facts checked on the replay (they hold for any legal P Q Pbar; the script verifies them, it does not assume
them):
- (F1) every piece outside M has, after Pbar, its exact pose of s;
- (F2) for every twist of Pbar, the pieces outside M that its cap selects are exactly the pieces outside M that the
  mirror twist of P selected;
- (F3) the landing poses after Q of the pieces of M determine their Pbar trajectories alone: a ghost replay of Pbar
  that holds only the pieces of M at their poses after Q (every other piece fixed on the lattice) is clean for each
  of them and ends at their real poses after Pbar.
With ``--chambers`` it also replays Pbar on the 177,120 home chambers alone (ghost pieces at the identity, every
straddle recorded per chamber): a chamber is good when its trajectory is clean and ends in K. This set depends on P
only. The script compares it with the occupancy after P and after Q.

Usage: python -I conjclose.py <theory dir> <pose table of prefix s> <word file> <s> <|P|> <|Q|> <e> <out>
       [--chambers]
Evidence: exact simulator replay and exact ghost replays; no native host evidence.
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
import buffer as B  # noqa: E402
import sim  # noqa: E402
from sim.kernel import Evaluator, pullback_form  # noqa: E402

CL, O, restore = B.CL, B.O, B.restore
T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)


def classify3(st, e, alive=None):
    """Exact (inside, straddle) piece arrays for cut e; the same grouping and anchor caps as State.classify,
    but every straddling piece is returned. With ``alive`` only those off pieces are evaluated."""
    data, kp, reg = st.ctx.data, st.ctx.kplus, st.ctx.regions
    K = st._kpose()[st.pose_id]
    lat = K >= 0
    lp = np.flatnonzero(lat)
    parts = [lp[data.in_sig(lp, kp.perms[kp.inv[K[lp]], int(e)])]]
    strad = []
    off = np.flatnonzero(~lat)
    if alive is not None:
        off = off[alive[off]]
    for pid, members in (st._groups(off) if off.size else []):
        ev = Evaluator(pullback_form(st._pose_matrix(pid), data.N[e]))
        for anchor, sub in st._anchors(pid, members):
            r = ev.classify(reg.cap_vform(anchor))
            if r[0] == 'in':
                parts.append(sub)
                continue
            if r[0] == 'out':
                continue
            for p in sub.tolist():
                r = ev.classify(reg.vform(p))
                if r[0] == 'in':
                    parts.append(np.array([p], np.int64))
                elif r[0] == 'straddle':
                    strad.append(p)
    inside = np.sort(np.concatenate(parts)) if parts else np.zeros(0, np.int64)
    return inside, np.array(sorted(strad), np.int64)


class Codes:
    """Global exact pose codes: K poses k -> k, off poses -> -(1 + n) for the n-th distinct matrix seen."""

    def __init__(self):
        self.off = {}

    def of(self, st):
        table = np.empty(len(st._poses), np.int64)
        for i, pose in enumerate(st._poses):
            if pose.kidx >= 0:
                table[i] = pose.kidx
            else:
                table[i] = -1 - self.off.setdefault(pose.key, len(self.off))
        return table[st.pose_id]


def twist(c, i, M):
    return sim.Twist(CL.ctx, int(c), restore.element(M, c, i), 'menu', {'menu': M.name})


def ghost_run(M, word, poses, pieces, st_src):
    """Replay ``word`` on ghost copies of ``pieces`` at their poses in ``st_src`` (others fixed at the identity).

    Returns (end pose codes source -> matrices, clean mask, first unclean twist per piece, ghost state)."""
    g = sim.State(CL.ctx, menu=M)
    pieces = np.asarray(pieces, np.int64)
    for pid in np.unique(st_src.pose_id[pieces]).tolist():
        sel = pieces[st_src.pose_id[pieces] == pid]
        k = st_src._poses[pid].kidx
        g.pose_id[sel] = g._intern_k(k) if k >= 0 else g._intern_matrix(st_src._poses[pid].matrix)
    alive = np.zeros(len(g.pose_id), bool)
    alive[pieces] = True
    first_bad = np.full(len(g.pose_id), -1, np.int64)
    for j, (c, i) in enumerate(word):
        inside, strad = classify3(g, c, alive)
        if len(strad):
            first_bad[strad] = j
            alive[strad] = False
            g.pose_id[strad] = 0
        inside = inside[alive[inside]]
        if len(inside):
            tw = twist(c, i, M)
            uniq, inv = np.unique(g.pose_id[inside], return_inverse=True)
            new = np.array([g._compose(tw, int(u)) for u in uniq], np.int32)
            g.pose_id[inside] = new[inv]
        if j % 50 == 49:
            log(ghost_twist=j + 1, of=len(word), alive=int(alive[pieces].sum()))
    return alive, first_bad, g


def main():
    a = sys.argv[1:]
    table, wordfile = a[1], a[2]
    s, p, q, e = (int(x) for x in a[3:7])
    out = Path(a[7])
    chambers = '--chambers' in a[8:]
    pre = int(a[a.index('--pre') + 1]) if '--pre' in a else 0
    word = [(int(c), int(i)) for c, i in json.loads(Path(wordfile).read_text())['word']]
    seg = word[s + pre:e]
    if len(seg) < 2 * p + q:
        raise ValueError('window shorter than P Q Pbar')
    P, Q, Pb, tail = seg[:p], seg[p:p + q], seg[p + q:2 * p + q], seg[2 * p + q:]
    R, M = O.start(table)
    st = R.st
    if st.digest() != json.loads(Path(table).read_text())['digest']:
        raise AssertionError('table digest mismatch')
    for c, i in word[s:s + pre]:
        R.apply(c, i)
    d_s = st.digest()
    for j in range(p):
        x, y = P[p - 1 - j], Pb[j]
        prod = restore.matmul(restore.element(M, *x), restore.element(M, *y))
        if x[0] != y[0] or not all(prod[r][c] == (1 if r == c else 0) for r in range(4) for c in range(4)):
            raise AssertionError(f'Pbar twist {j} is not the mirror inverse of P twist {p - 1 - j}')
    kp = CL.ctx.kplus
    kidx = {w: int(kp.index_of_matrix(restore.element(M, *w))) for w in set(seg)}
    NP = len(st.pose_id)
    codes = Codes()
    reg = CL.ctx.regions
    ORB, TRN, POF = np.asarray(reg.orbit_of), np.asarray(reg.transport), reg.piece_of

    def chamber_of(code_arr):
        out_ = np.full(NP, -1, np.int64)
        on = np.flatnonzero(code_arr >= 0)
        out_[on] = POF[ORB[on], kp.compose_vec(code_arr[on], TRN[on])]
        return out_

    cap = {}
    commit = st._commit

    def capture(tw, inside, journal):
        cap['inside'] = np.asarray(inside, np.int64).copy()
        return commit(tw, inside, journal)

    st._commit = capture
    snap = {'s': codes.of(st)}
    dig = {'s': d_s}
    rows, insides = [], []
    lat = snap['s'] >= 0
    stages = [('P', P), ('Q', Q), ('Pbar', Pb), ('tail', tail)]
    done = 0
    for name, w in stages:
        for j, (c, i) in enumerate(w):
            R.apply(c, i)
            ins = cap['inside']
            before_on = lat[ins]
            now = codes.of(st)
            lat_new = now >= 0
            rows.append({'stage': name, 'j': j, 'grip': c, 'menu': i, 'k': kidx[(c, i)] if kidx[(c, i)] >= 0 else None,
                         'moved': len(ins), 'moved_on_before': int(before_on.sum()),
                         'moved_off_before': int((~before_on).sum()),
                         'went_off': int((before_on & ~lat_new[ins]).sum()),
                         'came_on': int((~before_on & lat_new[ins]).sum()), 'off_after': int((~lat_new).sum())})
            insides.append(ins.astype(np.int32))
            lat = lat_new
            done += 1
            if done % 100 == 0:
                log(replayed=done, of=len(seg), stage=name, off=int((~lat).sum()))
        snap[name] = codes.of(st)
        dig[name] = st.digest()
    end = snap['tail']
    off_s = snap['s'] < 0
    on_e = end >= 0
    T = off_s & on_e
    Prot = off_s & ~on_e
    coll = ~off_s & ~on_e
    sP, sQ, sPb = snap['P'], snap['Q'], snap['Pbar']
    C = ~off_s & (sP < 0)
    Lc = ~off_s & (sP >= 0)
    classes = {'T': T, 'Prot': Prot, 'C': C, 'L': Lc}
    Mv = sQ != sP
    res = {'window': {'s': s, 'pre': pre, 'e': e, 'P': len(P), 'Q': len(Q), 'Pbar': len(Pb), 'tail': len(tail)},
           'words': {'P': P, 'Q': Q, 'Pbar': Pb, 'tail': tail},
           'digests': dig, 'mirror_inverse_checked': True,
           'at_s': {'off': int(off_s.sum()), 'target': int(T.sum()), 'protected': int(Prot.sum()),
                    'protected_exact_at_end': bool((end[Prot] == snap['s'][Prot]).all()),
                    'lattice_of_s_off_at_end': int(coll.sum())},
           'after_P': {'off': int((sP < 0).sum()), 'target_on': int((T & (sP >= 0)).sum()),
                       'protected_on': int((Prot & (sP >= 0)).sum()), 'C': int(C.sum()), 'L': int(Lc.sum()),
                       'protected_moved_by_P': int((Prot & (sP != snap['s'])).sum()),
                       'target_moved_by_P': int((T & (sP != snap['s'])).sum())}}
    # per-twist class selections
    for r, ins in zip(rows, insides):
        r['by_class'] = {k: int(v[ins].sum()) for k, v in classes.items()}
        r['moved_of_M'] = int(Mv[ins].sum()) if r['stage'] in ('Pbar', 'tail') else None
    res['twists'] = rows
    res['Q'] = {'net_moved': {k: int((Mv & v).sum()) for k, v in classes.items()}, 'net_moved_total': int(Mv.sum()),
                'M_on_after_Q': int((Mv & (sQ >= 0)).sum()), 'M_off_after_Q': int((Mv & (sQ < 0)).sum()),
                'twists_outside_K': sum(1 for r in rows if r['stage'] == 'Q' and r['k'] is None),
                'twists_moving_off_pieces': sum(1 for r in rows if r['stage'] == 'Q' and r['moved_off_before']),
                'max_off_inside_Q': max((r['off_after'] for r in rows if r['stage'] == 'Q'), default=None)}
    # F1: pieces outside M back at their exact poses of s after Pbar
    U = ~Mv
    res['F1_unmoved_back_exact'] = bool((sPb[U] == snap['s'][U]).all())
    res['F1_unmoved_count'] = int(U.sum())
    # F2: mirror selections outside M
    nP, nQ = len(P), len(Q)
    f2 = []
    for j in range(nP):
        a_ins = insides[nP + nQ + j].astype(np.int64)
        b_ins = insides[nP - 1 - j].astype(np.int64)
        au, bu = a_ins[U[a_ins]], b_ins[U[b_ins]]
        f2.append({'j': j, 'equal_outside_M': bool(np.array_equal(au, bu)), 'Pbar_selects_M': int(Mv[a_ins].sum()),
                   'P_mirror_selected_M': int(Mv[b_ins].sum())})
    res['F2_mirror_selection'] = {'all_equal_outside_M': all(x['equal_outside_M'] for x in f2), 'rows': f2}
    # F3: ghost replay of Pbar from landing poses of M only
    Mi = np.flatnonzero(Mv)
    log(stage='ghost_landing', pieces=len(Mi))
    # rebuild the state after Q to read landing matrices: replay from table again is costly; use the stored codes
    # instead by re-creating poses from the code table.
    inv_off = {v: k for k, v in codes.off.items()}
    src = sim.State(CL.ctx, menu=M)
    for code in np.unique(sQ[Mi]).tolist():
        sel = Mi[sQ[Mi] == code]
        src.pose_id[sel] = src._intern_k(code) if code >= 0 else src._intern_matrix(inv_off[-1 - code][1])
    alive, first_bad, g = ghost_run(M, Pb, None, Mi, src)
    gend = codes.of(g)
    res['F3_landing'] = {'M': len(Mi), 'clean': int(alive[Mi].sum()),
                         'ghost_end_equals_real': bool((gend[Mi] == sPb[Mi]).all()),
                         'end_in_K': int((sPb[Mi] >= 0).sum()),
                         'end_in_K_by_class': {k: int(((sPb >= 0) & Mv & v).sum()) for k, v in classes.items()},
                         'end_off_by_class': {k: int(((sPb < 0) & Mv & v).sum()) for k, v in classes.items()}}
    # places: where the moved pieces end, against the occupancy at s
    ch_s, ch_end = chamber_of(snap['s']), chamber_of(sPb)
    occ_s = np.full(NP, -1, np.int64)
    on_s = np.flatnonzero(ch_s >= 0)
    occ_s[ch_s[on_s]] = on_s
    land = ch_end[Mi]
    land = land[land >= 0]
    held = occ_s[land]
    res['places'] = {'M_ending_in_a_chamber': len(land), 'chamber_was_a_hole_at_s': int((held < 0).sum()),
                     'chamber_held_by_M_piece_at_s': int(((held >= 0) & Mv[np.maximum(held, 0)]).sum()),
                     'chamber_held_by_unmoved_piece_at_s': int(((held >= 0) & ~Mv[np.maximum(held, 0)]).sum())}
    if chambers:
        log(stage='chamber_ghost', pieces=NP)
        sol = sim.State(CL.ctx, menu=M)
        allp = np.arange(NP)
        alive_c, bad_at, gc = ghost_run(M, Pb, None, allp, sol)
        gcode = codes.of(gc)
        good = alive_c & (gcode >= 0)
        occ = {}
        for name in ('s', 'P', 'Q'):
            chs = chamber_of(snap[name])
            occ[name] = np.zeros(NP, bool)
            occ[name][chs[chs >= 0]] = True
        chP = chamber_of(sP)
        tP = chP[T & (sP >= 0)]
        res['chambers'] = {'good': int(good.sum()), 'bad': int((~good).sum()),
                           'bad_unclean': int((~alive_c).sum()), 'bad_clean_ends_off': int((alive_c & (gcode < 0)).sum()),
                           'occupied_bad_after_P': int((occ['P'] & ~good).sum()),
                           'occupied_bad_after_P_equals_target_on_chambers': bool(
                               np.array_equal(np.sort(np.flatnonzero(occ['P'] & ~good)), np.sort(chP[(sP >= 0) & off_s]))),
                           'off_of_s_on_after_P': int(((sP >= 0) & off_s).sum()), 'target_on_after_P': len(tP),
                           'holes_after_P': int((~occ['P']).sum()), 'bad_holes_after_P': int((~occ['P'] & ~good).sum()),
                           'occupied_bad_after_Q': int((occ['Q'] & ~good).sum()),
                           'holes_after_Q': int((~occ['Q']).sum())}
        np.save(out.with_suffix('.good.npy'), good)
    res['seconds'] = round(time.time() - T0)
    out.write_text(json.dumps(res, indent=1) + '\n')
    log(done=str(out))


if __name__ == '__main__':
    main()
