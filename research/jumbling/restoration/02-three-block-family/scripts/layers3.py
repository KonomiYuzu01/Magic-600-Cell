"""Certificate V(X, sigma) for the three-block iterated-conjugation template, from a pose table, the model and sigma.

Template sigma = (R; x_0 .. x_{m-1}; F): P_0 = R, P_{k+1} = P_k x_k P_k^-1, W = P_m F P_m^-1, |W| = 2^(m+2) - 1.

Word-only chamber ghost of a word w: the lattice content of every chamber is followed exactly (fastghost).
  KV(w): contents that straddle a cut at some twist. Off(w): alive and off K at the end. Good(w): alive and on K, with
  the injective end map pi_w. Bad(w) = KV(w) | Off(w). Chambers outside every cap of the template are fixed.
Frame ghost of the off block B' (its exact contents) with the exact pose of each piece, multiplied by every twist
that moves it.

Per level k, with T_k = Bad(P_k^-1) (the bubble) and L_k = chambers of B' after P_k:
  K0  KV(P_k) = KV(P_k^-1) = {}; B' alive through P_k and on K at L_k; L_k inside T_k;
      Good(P_k^-1) = pi_k(Good(P_k)) and pi_{P_k^-1} = pi_k^-1.
  R0  (k = 0) R moves all of B' and B' lands on K.
  S1  x_k in K+.  S2  x_k neither moves nor cuts a parked content (a content of Off(P_k) after P_k).
  S3  L_k inside cap(x_k) and x_k(L_k) disjoint from T_k.
  Update formulas (pi = pi_{P_k}, x = chamber map of x_k), compared with the direct ghosts of level k+1:
      Off(P_{k+1}) = pi^-1(x^-1(T_k) - T_k), T_{k+1} = pi^-1(x(T_k) - T_k), L_{k+1} = pi^-1(x(L_k)),
      pi_{k+1} = identity on Off(P_k) and pi^-1 x pi on the rest of Good(P_k).
Finishing letter F at level m: F1 in K+; F2 no parked content moved or cut; F3 F(L_m) disjoint from T_m;
  F4 F(T_m - L_m) inside T_m. Predicted: KV(W) = {}, Bad(W) = Off(W) = pi^-1(F^-1(T_m) - T_m) with 3 chambers (D),
  B' lands on K at pi^-1(F(L_m)); compared with the direct ghost of W.
Vacancy at X (SAFE-ALL alphabet of the off geometry, buffer.Alphabet): N1, N2 for D, Jordan certificates for the
orbits that meet D, the peeling certificate and the clearing word V (predicted holes only, nothing applied).

--search adds, per level, every finishing letter (F1-F4) with its D, and every pure letter y (Lemma Z: y in K+,
cap(y) misses L_k, no parked content moved or cut, y(T_k) = T_k) with the chambers that Z = P_k y P_k^-1 moves.

Nothing here applies a twist to the simulator state.

Usage: python -I layers3.py <theory dir> <v26 dir> <pose table> <template json> <out json> [--search] [--no-vacancy]
       template json: {"R": [c, i], "x": [[c, i], ...], "F": [c, i] or null}
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
V26 = Path(sys.argv[2]).resolve()
for _p in (str(V26), str(TH.parent), str(TH)):
    if _p not in sys.path:
        sys.path.insert(0, _p)
import buffer as B  # noqa: E402
import conjprep as CP  # noqa: E402
import fastghost as FG  # noqa: E402
import ghostlib as GL  # noqa: E402
from exact import matmul  # noqa: E402
from sim.kplus import matrix_json, to_tuple  # noqa: E402

kp, reg, data = FG.kp, FG.reg, FG.data
NP = len(data.sigbits)
IDK = FG.IDK
T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


def kidx(m):
    return int(kp.index_of_matrix(to_tuple(m)))


def ids(mask):
    return np.flatnonzero(mask).tolist()


def level_words(M, R, xs):
    P = [[tuple(R)]]
    for x in xs:
        P.append(P[-1] + [tuple(x)] + CP.inverse_word(M, P[-1]))
    return P


class Chambers:
    """Word-only chamber ghost of a word over the chamber set U (every other chamber is fixed by every letter)."""

    def __init__(self, C, word, U):
        g = FG.Ghost(C.F, np.zeros(len(U), np.int64), U)
        for c, i in word:
            g.apply(c, C.E(c, i), (c, i))
        self.g, self.U = g, U
        offm = g.alive & (g.dom != 0)
        good = g.alive & (g.dom == 0)
        self.kv = np.zeros(NP, bool)
        self.kv[U[~g.alive]] = True
        self.off = np.zeros(NP, bool)
        self.off[U[offm]] = True
        self.pi = np.arange(NP)
        self.pi[U[~good]] = -1
        self.pi[U[good]] = g.q[good]
        self.parked_at = np.flatnonzero(offm)
        gm = self.pi >= 0
        self.injective = bool(len(np.unique(self.pi[gm])) == int(gm.sum()))

    @property
    def bad(self):
        return self.kv | self.off

    def inverse_map(self):
        ip = np.full(NP, -1, np.int64)
        gm = np.flatnonzero(self.pi >= 0)
        ip[self.pi[gm]] = gm
        return ip

    def parked(self):
        j = self.parked_at
        return self.g.dom[j], self.g.q[j]


def sides(C, dom, q, c):
    """Exact side of contents (frame dom, chamber q) against cut c: 1 in, 0 out, -1 straddle."""
    out = np.zeros(len(q), np.int64)
    for d in np.unique(dom).tolist():
        m = dom == d
        e = C.F.pole(d, c)
        if e >= 0:
            out[m] = np.where(data.in_sig(q[m], np.full(int(m.sum()), e, np.int64)), 1, 0)
        else:
            out[m] = C.F.sides(d, c, q[m])
    return out


def block_run(C, word, dom, q, poses):
    """Follow the block's contents and exact poses through a word."""
    dom, q, poses = dom.copy(), q.copy(), list(poses)
    moved = []
    for s, (c, i) in enumerate(word):
        sd = sides(C, dom, q, c)
        if (sd < 0).any():
            return {'alive': False, 'died_at': s}
        ins = np.flatnonzero(sd == 1)
        if len(ins):
            E = C.E(c, i)
            for j in ins.tolist():
                d2, k = C.F.mul((c, i), E, int(dom[j]))
                dom[j] = d2
                q[j] = FG.chamber_image(k, np.array([q[j]]))[0]
                poses[j] = matmul(E, poses[j])
            moved.append([s, len(ins)])
    ks = [kidx(P) for P in poses]
    return {'alive': True, 'on_K': bool((dom == 0).all()), 'dom': dom.tolist(), 'chambers': q.tolist(), 'kplus': ks,
            'rigid': all(to_tuple(P) == to_tuple(poses[0]) for P in poses), 'moved_at': moved, 'poses': poses}


_HOME = []


def HOME():
    """Home signature matrix: HOME()[z, c] is True when chamber z lies in cap c."""
    if not _HOME:
        _HOME.append(np.asarray(data.sig_matrix(np.arange(NP))))
    return _HOME[0]


def letter_map(C, c, i):
    """Chamber map of a K+ letter (identity outside its cap), or None when the letter is not in K+."""
    k = kidx(C.E(c, i))
    if k < 0:
        return None, k
    p = np.arange(NP)
    xs = np.flatnonzero(GL.cap(c))
    p[xs] = FG.chamber_image(k, xs)
    return p, k


def kplus_letters(C, grips):
    out = []
    for c in grips:
        for i in range(len(C.M.items)):
            k = kidx(C.E(c, i))
            if k >= 0 and k != IDK:
                out.append((c, i))
    return out


def check_level_update(pred, gP, gI, L):
    """Compare the predicted level k+1 sets with the direct ghosts of P_{k+1} and P_{k+1}^-1."""
    return {'off_P': bool(np.array_equal(pred['off_P'], gP.off)),
            'T': bool(np.array_equal(pred['T'], gI.bad)),
            'L': bool(sorted(pred['L']) == sorted(L)),
            'pi': bool(np.array_equal(pred['pi'], gP.pi))}


def predict_next(gP, T, L, xmap, kv_free):
    """Level k+1 from level k under S1-S3 (pi = pi_{P_k}, xmap = chamber map of x_k)."""
    pi = gP.pi
    ip = gP.inverse_map()
    xT = np.zeros(NP, bool)
    xT[xmap[np.flatnonzero(T)]] = True
    pre = np.zeros(NP, bool)  # x^-1(T)
    pre[np.flatnonzero(T[xmap])] = True
    off_next = np.zeros(NP, bool)
    off_next[ip[np.flatnonzero(pre & ~T)]] = True
    T_next = np.zeros(NP, bool)
    T_next[ip[np.flatnonzero(xT & ~T)]] = True
    L_next = ip[xmap[np.asarray(L)]].tolist()
    pi_next = np.full(NP, -1, np.int64)
    good = np.flatnonzero(pi >= 0)
    y2 = xmap[pi[good]]
    keep = ~T[y2]
    pi_next[good[keep]] = ip[y2[keep]]
    pi_next[gP.off] = np.flatnonzero(gP.off)
    return {'off_P': off_next, 'T': T_next, 'L': L_next, 'pi': pi_next,
            'beta': int((xT & ~T).sum()) - len(L), 'valid': kv_free}


def certify(C, sigma, search=False, frozen=None, log_rows=True):
    M, st = C.M, C.st
    Bp = np.asarray(C.off, np.int64)
    poses0 = [st.pose(int(p)) for p in Bp]
    dom0, q0 = C.contents(Bp)
    R, xs, F = tuple(sigma['R']), [tuple(x) for x in sigma['x']], sigma.get('F')
    F = tuple(F) if F is not None else None
    m = len(xs)
    grips = sorted({R[0]} | {x[0] for x in xs} | ({F[0]} if F else set()))
    U = np.flatnonzero(np.logical_or.reduce([GL.cap(c) for c in grips]))
    cert = {'input_digest': st.digest(), 'template': {'R': list(R), 'x': [list(x) for x in xs],
                                                      'F': list(F) if F else None, 'm': m},
            'block': {'pieces': Bp.tolist(), 'count': int(len(Bp)),
                      'one_block': bool(len(C.bl.blocks) == 1 and sorted(map(int, C.bl.blocks[0])) == Bp.tolist()),
                      'rigid': all(to_tuple(P) == to_tuple(poses0[0]) for P in poses0),
                      'pose': matrix_json(poses0[0]), 'frames': dom0.tolist(), 'contents': q0.tolist()},
            'universe': {'grips': grips, 'chambers': int(len(U))}, 'levels': [], 'failures': []}
    fail = cert['failures']
    if cert['block']['count'] != 3 or not cert['block']['one_block'] or not cert['block']['rigid']:
        fail.append({'where': 'input', 'condition': 'off set is one rigid 3-block'})
    words = level_words(M, R, xs)
    pred = None
    search_rows = []
    for k, Pk in enumerate(words):
        t = time.time()
        Pinv = CP.inverse_word(M, Pk)
        gP, gI = Chambers(C, Pk, U), Chambers(C, Pinv, U)
        T = gI.bad
        blk = block_run(C, Pk, dom0, q0, poses0)
        L = blk['chambers'] if blk['alive'] and blk['on_K'] else []
        if L:
            blk['pose_matches_chambers'] = all(kk >= 0 for kk in blk['kplus']) and all(
                int(FG.chamber_image(kk, np.array([p]))[0]) == int(qq)
                for kk, p, qq in zip(blk['kplus'], Bp.tolist(), L))
        goodI = gI.pi >= 0
        img = np.zeros(NP, bool)
        img[gP.pi[gP.pi >= 0]] = True
        ipP = gP.inverse_map()
        row = {'k': k, 'P_length': len(Pk), 'KV_P': int(gP.kv.sum()), 'KV_Pinv': int(gI.kv.sum()),
               'Off_P': int(gP.off.sum()), 'T': int(T.sum()), 'T_chambers': ids(T) if T.sum() <= 64 else None,
               'pi_injective': gP.injective and gI.injective,
               'inverse_consistent': bool(np.array_equal(goodI, img)
                                          and np.array_equal(gI.pi[goodI], ipP[goodI])),
               'block': {x: blk.get(x) for x in ('alive', 'on_K', 'chambers', 'kplus', 'rigid',
                                                 'pose_matches_chambers', 'died_at')},
               'L_in_T': bool(L and T[L].all())}
        if k > 0 and pred is not None:
            row['update_formulas'] = check_level_update(pred, gP, gI, L)
            row['T_subset_of_previous_T'] = bool(not (T & ~prevT).any())
            row['beta_previous'] = pred['beta']
        prevT = T
        k0 = row['KV_P'] == 0 and row['KV_Pinv'] == 0 and blk['alive'] and blk.get('on_K') and row['L_in_T'] \
            and row['pi_injective'] and row['inverse_consistent'] and blk.get('pose_matches_chambers')
        row['K0'] = bool(k0)
        if not k0:
            fail.append({'where': f'level {k}', 'condition': 'K0', 'detail': {x: row[x] for x in (
                'KV_P', 'KV_Pinv', 'L_in_T', 'pi_injective', 'inverse_consistent')}, 'block': row['block']})
        if k == 0:
            r0 = blk['alive'] and blk['moved_at'] == [[0, len(Bp)]] and blk['on_K']
            row['R0'] = bool(r0)
            if not r0:
                fail.append({'where': 'level 0', 'condition': 'R0', 'block': row['block']})
        if 'update_formulas' in row and not all(row['update_formulas'].values()):
            fail.append({'where': f'level {k}', 'condition': 'update formulas', 'detail': row['update_formulas']})
        pd, pq = gP.parked()
        if search and k0:
            search_rows.append(level_search(C, k, gP, T, L, pd, pq, frozen))
        if k < m:
            c, i = xs[k]
            xmap, kx = letter_map(C, c, i)
            s1 = xmap is not None and kx != IDK
            sd = sides(C, pd, pq, c)
            s2 = bool((sd == 0).all())
            s3 = bool(s1 and L and GL.cap(c)[L].all() and not T[xmap[L]].any())
            row['x'] = {'letter': [c, i], 'S1': bool(s1), 'S2': s2, 'parked_in_cap': int((sd == 1).sum()),
                        'parked_cut': int((sd < 0).sum()), 'S3': s3}
            if s1:
                row['x']['moved_out_of_T'] = int((T & ~T[xmap]).sum())
                row['x']['moved_into_T'] = int((~T & T[xmap]).sum())
            for name, ok in (('S1', s1), ('S2', s2), ('S3', s3)):
                if not ok:
                    fail.append({'where': f'level {k}', 'condition': name, 'letter': [c, i]})
            pred = predict_next(gP, T, L, xmap, k0) if (s1 and s2 and s3 and k0) else None
        else:
            row['level_seconds'] = round(time.time() - t, 1)
            cert['levels'].append(row)
            if F is None:
                break
            c, i = F
            fmap, kf = letter_map(C, c, i)
            f1 = fmap is not None and kf != IDK
            sd = sides(C, pd, pq, c)
            f2 = bool((sd == 0).all())
            f3 = bool(f1 and L and not T[fmap[L]].any())
            TL = T.copy()
            TL[L] = False
            f4 = bool(f1 and T[fmap[np.flatnonzero(TL)]].all())
            fin = {'letter': [c, i], 'F1': bool(f1), 'F2': f2, 'F3': f3, 'F4': f4,
                   'parked_in_cap': int((sd == 1).sum()), 'parked_cut': int((sd < 0).sum())}
            for name, ok in (('F1', f1), ('F2', f2), ('F3', f3), ('F4', f4)):
                if not ok:
                    fail.append({'where': 'finish', 'condition': name, 'letter': [c, i]})
            W = Pk + [F] + Pinv
            gW = Chambers(C, W, U)
            bw = block_run(C, W, dom0, q0, poses0)
            fin['W_length'] = len(W)
            fin['KV_W'] = int(gW.kv.sum())
            fin['Bad_W'] = ids(gW.bad) if gW.bad.sum() <= 64 else int(gW.bad.sum())
            fin['block_end'] = {x: bw.get(x) for x in ('alive', 'on_K', 'chambers', 'kplus', 'rigid', 'died_at')}
            if f1:
                ip = gP.inverse_map()
                pre = T[fmap] & ~T
                D_pred = sorted(ip[np.flatnonzero(pre)].tolist())
                land_pred = ip[fmap[np.asarray(L)]].tolist() if L else []
                fin['D_predicted'] = D_pred
                fin['landing_predicted'] = land_pred
                fin['prediction_matches_ghost'] = bool(
                    fin['KV_W'] == 0 and isinstance(fin['Bad_W'], list) and fin['Bad_W'] == D_pred
                    and bw['alive'] and bw.get('on_K') and sorted(bw['chambers']) == sorted(land_pred))
                if bw['alive'] and bw.get('on_K'):
                    fin['block_end']['pose_matches_chambers'] = all(kk >= 0 for kk in bw['kplus']) and all(
                        int(FG.chamber_image(kk, np.array([p]))[0]) == int(qq)
                        for kk, p, qq in zip(bw['kplus'], Bp.tolist(), bw['chambers']))
                    fin['block_end']['pose'] = matrix_json(bw['poses'][0]) if bw['rigid'] else None
            ok_w = fin.get('prediction_matches_ghost') and len(fin.get('D_predicted', [])) == 3
            fin['W_ok'] = bool(ok_w)
            if not ok_w:
                fail.append({'where': 'finish', 'condition': 'W prediction', 'detail': {
                    x: fin.get(x) for x in ('KV_W', 'Bad_W', 'D_predicted', 'landing_predicted')}})
            cert['finish'] = fin
            cert['W'] = [list(x) for x in W]
            if log_rows:
                log(finish={x: fin[x] for x in fin if x not in ('block_end',)})
            break
        row['level_seconds'] = round(time.time() - t, 1)
        cert['levels'].append(row)
        if log_rows:
            log(level=k, T=row['T'], Off_P=row['Off_P'], K0=row['K0'], L=L, x=row.get('x'),
                upd=row.get('update_formulas'), sec_level=row['level_seconds'])
    if search:
        cert['search'] = search_rows
    cert['template_ok'] = not fail
    return cert


def level_search(C, k, gP, T, L, pd, pq, frozen):
    """Every finishing letter (F1-F4) and every pure letter (Lemma Z) at level k."""
    t = time.time()
    ip = gP.inverse_map()
    Lm = np.zeros(NP, bool)
    Lm[L] = True
    fins, pure = [], []
    grips_L = np.flatnonzero(HOME()[L].all(axis=0)).tolist()
    clear = {}

    def untouched(c):
        if c not in clear:
            clear[c] = bool((sides(C, pd, pq, c) == 0).all())
        return clear[c]

    for c, i in kplus_letters(C, grips_L):
        fmap, _ = letter_map(C, c, i)
        if T[fmap[L]].any():
            continue
        TL = T & ~Lm
        if not T[fmap[np.flatnonzero(TL)]].all():
            continue
        if not untouched(c):
            continue
        D = sorted(ip[np.flatnonzero(T[fmap] & ~T)].tolist())
        land = ip[fmap[np.asarray(L)]].tolist()
        fins.append({'F': [c, i], 'D': D, 'landing': land, 'involution_landing': sorted(land) == D})
    if frozen:
        targets = {int(z): int(gP.pi[z]) for z in frozen}
        ys = [y for y in targets.values() if y >= 0]
        H = HOME()
        grips = [c for c in range(600) if not H[L, c].any() and len(ys) and H[ys, c].any()]
        for c, i in kplus_letters(C, grips):
            ymap, _ = letter_map(C, c, i)
            if not np.array_equal(T[ymap], T):
                continue
            if not untouched(c):
                continue
            moved = {z: int(ip[ymap[y]]) for z, y in targets.items() if y >= 0 and ymap[y] != y}
            pure.append({'y': [c, i], 'frozen_images': moved,
                         'moves': int((ymap != np.arange(NP)).sum())})
    out = {'k': k, 'finishing': fins, 'pure': pure, 'seconds': round(time.time() - t, 1)}
    log(search_level=k, finishing=len(fins), pure=len(pure), sec_level=out['seconds'])
    return out


def vacancy(C, D, jordan=True, holes=None, G=None):
    """N1, N2, Jordan certificates and the peeling certificate for the target D at the alphabet of the off geometry."""
    if G is None:
        G = B.Alphabet(C.R, C.M)
    G.orbits()
    D = np.asarray(sorted(D), np.int64)
    hm = B._holes_mask(G, holes)
    H = np.flatnonzero(hm)
    typ = np.asarray(reg.orbit_of)
    out = {'holes': H.tolist(), 'hole_types': typ[H].tolist(), 'hole_orbits': G.orbit[H].tolist(),
           'hole_orbit_sizes': G.orbit_size[G.orbit[H]].tolist(), 'D': D.tolist(), 'D_types': typ[D].tolist(),
           'D_orbits': G.orbit[D].tolist(), 'D_orbit_sizes': G.orbit_size[G.orbit[D]].tolist(),
           'safe_grips': len(G.safe), 'letters': len(G.letters), 'orbits': int(len(G.orbit_size)),
           'singletons': int((G.orbit_size == 1).sum())}
    cond = B.condition(G, D, None, hm)
    out['N1'], out['N2'] = cond['fixed_vacant'], cond['necessary']
    out['fixed_occupied'], out['deficits'], out['by_orbit'] = cond['fixed_occupied'], cond['necessary_deficits'], \
        cond['by_orbit']
    if jordan:
        jr = B.jordan(G, D)
        out['jordan'] = {'certified': {o: {x: v for x, v in c.items() if x != 'word'} | {'word_length': len(c['word'])}
                                       for o, c in jr['certificates'].items()},
                         'missing': jr['missing'], 'below_8': jr['uncertifiable_below_8']}
        out['jordan_words'] = {o: c['word'] for o, c in jr['certificates'].items()}
        for o, c in jr['certificates'].items():
            chk = B.check_jordan(G, c['word'], int(o), c['prime'])
            out['jordan']['certified'][o]['rechecked'] = bool(chk['verified'])
    V = None
    if cond['necessary']:
        try:
            pc = CP.generic_certificate(G, D)
            out['peeling'] = {x: pc[x] for x in ('peeled', 'of', 'fixed', 'thresholds', 'all_reached',
                                                 'thresholds_equal_targets', 'family_bound')}
            prep, _, deficits = CP.generic_prepare(G, pc, D, hm)
            out['peeling']['deficits'] = {str(x): v for x, v in deficits.items()}
            if prep is not None:
                V = [tuple(x) for x in prep['word']]
                out['V'] = {'length': prep['length'], 'filled': prep['filled'], 'steps': prep['steps'],
                            'predicted_holes': prep['predicted_holes'],
                            'predicted_holes_equal_D': sorted(prep['predicted_holes']) == D.tolist()}
        except (ValueError, AssertionError) as exc:
            out['peeling_error'] = str(exc)[:400]
    return out, V, G


def main():
    a = sys.argv[1:]
    table, tfile, out = a[2], a[3], Path(a[4])
    sigma = json.loads(Path(tfile).read_text())
    C = GL.Ctx(table)
    frozen, G0 = None, None
    if '--search' in a:
        G0 = B.Alphabet(C.R, C.M)
        G0.orbits()
        typ = np.asarray(reg.orbit_of)
        want = set(typ[np.asarray(C.off)].tolist())
        frozen = [int(z) for z in np.flatnonzero(G0.orbit_size[G0.orbit] == 1) if int(typ[z]) in want]
        log(frozen_of_block_types=frozen)
    cert = certify(C, sigma, search='--search' in a, frozen=frozen)
    if frozen is not None:
        cert['frozen_chambers_of_block_types'] = frozen
    if cert.get('finish', {}).get('W_ok') and '--no-vacancy' not in a:
        t = time.time()
        vac, V, G = vacancy(C, cert['finish']['D_predicted'], G=G0)
        vac['seconds'] = round(time.time() - t, 1)
        cert['vacancy'] = vac
        if V is not None:
            Vinv = CP.inverse_word(C.M, V)
            cert['U'] = {'V': [list(x) for x in V], 'W_length': len(cert['W']), 'length': 2 * len(V) + len(cert['W'])}
            cert['U']['word'] = [list(x) for x in V] + cert['W'] + [list(x) for x in Vinv]
        log(vacancy={x: vac[x] for x in ('N1', 'N2', 'hole_orbits', 'D_orbits')}, V=len(V) if V else None,
            jordan=vac.get('jordan', {}).get('certified'))
    cert['seconds'] = round(time.time() - T0, 1)
    out.write_text(json.dumps(cert, indent=1) + '\n')
    log(done=str(out), template_ok=cert['template_ok'], failures=cert['failures'][:3])


if __name__ == '__main__':
    main()
