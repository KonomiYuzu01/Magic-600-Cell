"""Composable closure V W V^-1 at the current state, with a pre-construction certificate (reads only the pose table).

Target-list version of segment 3's closure2.py (only the choice of T changed): T is the piece list of a target file
(a block of equal Mov), and every other off piece is protected. W = P Q P^-1 restores block T. Before any twist is applied the script records, at the state S:
- the state: digest, off pieces by block, holes, the exact poses of the protected off pieces (all off pieces outside T);
- the SAFE-ALL menu rebuilt at S (buffer.Alphabet): grips, letters, blocked grips and why, orbits; optionally the
  inheritance of a reference menu (every reference SAFE-ALL grip is SAFE-ALL at S);
- the ghost premises of W: T ends on K, the protected pieces return to their exact contents, Bad(W), T's landing;
  Bad(P^-1) and T's position after P Q against Good(P^-1) (the allowed landing of the inverse part);
- the vacancy premises for the target Bad(W): N1, N2 and the per-orbit demand, then the peeling certificate
  (thresholds per orbit, all routes reached) and the sufficient condition of Theorem B (holes >= thresholds).
Then it constructs V (empty when Bad(W) holds no lattice piece; else generic_prepare from the certificate, and the
two-pass greedy of vacate2.py if the certificate fails), replays
V, P, Q, P^-1, V^-1 on the simulator (every twist checked admissible), writes each part with its input and output
digests, and compares the end with S: T restored, collateral, protected exact poses, and, for the lattice pieces of S,
lattice kept (on K at the end) and exact pose kept (same K pose) counted apart. Times of each phase are recorded.

--free <json {pieces}> (tclose2): pieces outside T that W may move (the other frames of a multi-frame block): they are
not protected; the ghost premise asks only that they stay admissible, and the end record reports them apart
(free_on_K, free_still_off, free_pose_changed). The step is then a partial step of the block's restoration.

Usage: python -I tclose2.py <theory dir> <E-chain scripts dir> <pose table> <target json {pieces}> <W json {'P','Q'}>
       <out dir>
       [--ref-safe <json list of grips>] [--greedy] [--force] [--greedy-seconds N] [--skip-bad <json list>]
Stops before the menu when the ghost premises fail (T not all on K, or a protected piece not exact), unless --force.
The greedy vacancy search stops after N seconds (default 600; then no V). --skip-bad names the digests of Bad(W)
sets already found not vacatable at this state (sha256 of the sorted chamber list as JSON); such a W stops after the
ghost premises with result 'known no V'. pre.json records the digest of Bad(W) as badW_digest.
Writes the end pose table as <out dir>/end-table.json. P may be empty (then W = Q).
"""
import hashlib
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
import buffer as B  # noqa: E402
import conjclose as CC  # noqa: E402
import conjprep as CP  # noqa: E402
import ghostlib as GL  # noqa: E402
import vacate as VA  # noqa: E402
sys.path.insert(0, str(Path(__file__).resolve().parent))
import vacate2 as VA2  # noqa: E402
from sim.kplus import matrix_json, to_tuple  # noqa: E402

T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


def peel2(tr, budget_s=1200.0):
    """Transport.peel(maxg=1) (same candidates, same selection rule), extended when it stops: a lazy search over
    depth-2 commutators [h, g1 g2] (h at a cap avoiding the target, g1, g2 letters touching the target, g1 or g2
    at a cap holding a still unpeeled target chamber) that accepts the first candidate whose new target support is
    one point with preimage outside the target. Accepted candidates are ordinary peeling candidates, so the pointwise
    check of Transport.peel applies unchanged at the end. (maxg=2 over all touching letters does not fit in memory.)"""
    G, T, L = tr.G, tr.T, tr.G.letters
    touchT = np.flatnonzero(~tr.avoidT).tolist()
    hcaps = np.array(sorted({L[n][0] for n in np.flatnonzero(tr.avoidT)}), np.int64)
    words, masks = [], []

    def add(word, indices):
        mask = 0
        for j in indices:
            mask |= 1 << int(j)
        if mask:
            words.append(word)
            masks.append(mask)

    for n in touchT:
        c, _, _, ys = L[n]
        inside = np.flatnonzero(G.home[T, c])
        img = ys[np.searchsorted(G.capx[c], T[inside])]
        add([n], inside[img != T[inside]])
    for n in touchT:
        g = [n]
        P = G.images(G.inverse(g), T)
        inside = G.home[np.ix_(P, hcaps)]
        for ci in np.flatnonzero(inside.any(axis=0)).tolist():
            c = int(hcaps[ci])
            js = np.flatnonzero(inside[:, ci])
            xs = P[js]
            for hn in G.by_grip[c]:
                img = L[hn][3][np.searchsorted(G.capx[c], xs)]
                moved = js[img != xs]
                if len(moved):
                    add([int(G.inv[hn])] + G.inverse(g) + [hn] + g, moved)
    log(peeling_candidates_depth1=len(words))
    order = np.argsort([len(w) for w in words], kind='stable').tolist()
    late, chosen, bad = 0, [], set()
    t_end = time.time() + budget_s
    stats = {'depth2_scanned': 0, 'depth2_accepted': 0}

    def try_word(word, mask):
        new = mask & ~late
        if not new or new & (new - 1):
            return None
        t = int(T[new.bit_length() - 1])
        p = G.perm(word)
        y = int(np.flatnonzero(p == t)[0])
        if tr.inT[y]:
            return None
        return t, y, new

    def depth2():
        rest = [j for j in range(len(T)) if not (late >> j) & 1]
        rcaps = {c for c in tr.safe.tolist() if G.home[T[rest], c].any()}
        touchR = [n for n in touchT if L[n][0] in rcaps]
        pairs = [(a, b) for a in touchR for b in touchT] + [(a, b) for a in touchT for b in touchR]
        log(peel_stuck=len(chosen), rest=len(rest), touchR=len(touchR), depth2_pairs=len(pairs))
        seen = set()
        for a, b in pairs:
            if (a, b) in seen or b == G.inv[a]:
                continue
            seen.add((a, b))
            if time.time() > t_end:
                return None
            stats['depth2_scanned'] += 1
            g = [a, b]
            P = G.images(G.inverse(g), T)
            inside = G.home[np.ix_(P, hcaps)]
            for ci in np.flatnonzero(inside.any(axis=0)).tolist():
                c = int(hcaps[ci])
                js = np.flatnonzero(inside[:, ci])
                if not any(not (late >> int(j)) & 1 for j in js):
                    continue
                xs = P[js]
                for hn in G.by_grip[c]:
                    img = L[hn][3][np.searchsorted(G.capx[c], xs)]
                    moved = js[img != xs]
                    if not len(moved):
                        continue
                    mask = 0
                    for j in moved:
                        mask |= 1 << int(j)
                    word = [int(G.inv[hn])] + G.inverse(g) + [hn] + g
                    r = try_word(word, mask)
                    if r is not None:
                        return word, mask, r
        return None

    while len(chosen) < len(T):
        for e in order:
            new = masks[e] & ~late
            if not new or new & (new - 1) or (e, new) in bad:
                continue
            t = int(T[new.bit_length() - 1])
            p = G.perm(words[e])
            y = int(np.flatnonzero(p == t)[0])
            if tr.inT[y]:
                bad.add((e, new))
                continue
            late |= new
            chosen.append((t, words[e], y))
            break
        else:
            found = depth2()
            if found is None:
                raise ValueError(f'peeling stopped at {len(chosen)} of {len(T)} target chambers '
                                 f'(depth-2 targeted search: {stats})')
            word, mask, (t, y, new) = found
            words.append(word)
            masks.append(mask)
            order.append(len(words) - 1)
            late |= new
            chosen.append((t, word, y))
            stats['depth2_accepted'] += 1
            log(depth2_accept=t, word_len=len(word), chosen=len(chosen), **stats)
    chosen.reverse()
    protected = []
    for t, w, y in chosen:
        p = G.perm(w)
        if p[t] == t or p[y] != t or not np.array_equal(p[protected], protected):
            raise AssertionError('peeling word does not satisfy its pointwise constraints')
        protected.append(t)
    log(peel_done=len(chosen), **stats)
    return chosen, stats


def gc1(G, T):
    """Copy of conjprep.generic_certificate with the peeling restricted to letters and depth-1 commutators
    (tr.peel(maxg=1)); maxg=2 crashed with exit 139 on this target. Everything else is unchanged."""
    G.orbits()
    T = np.asarray(sorted(set(int(x) for x in T)), np.int64)
    fixed = T[G.orbit_size[G.orbit[T]] == 1]
    movable = T[G.orbit_size[G.orbit[T]] > 1]
    tr = B.Transport(G, movable)
    chosen, pstats = peel2(tr)
    macros = []
    for _, w, _ in chosen:
        p = G.perm(w)
        macros.append((w, p, tr.support(p)))
    free, protected, rows = np.ones(len(G.letters), bool), [], []
    for j, (t, w, y) in enumerate(chosen):
        tree, extra = B.close_routes(tr, t, y, protected, free, macros[j:])
        metrics = B._route_metrics(G, t, protected, w, tree)
        rows.append(dict(t=t, orbit=int(G.orbit[t]), y=y, word=G.pairs(w), **metrics,
                         conjugate_edges=[dict(e, word=G.pairs(e['word'])) for e in extra]))
        protected.append(t)
        free &= ~G.home[t, tr.capsof]
    doc = {'format': 'generic-target-1', 'off_geometry_fingerprint': G.fingerprint, 'source_digest': G.state_digest,
           'menu': G.M.name, 'target_chambers': T.tolist(), 'fixed': fixed.tolist(), 'peeled': len(chosen),
           'of': len(movable), 'rows': rows, **B._totals(G, T, rows), 'peel': 'maxg1+targeted-depth2', 'peel_stats': pstats}
    return doc


def gp_forced(G, cert, T, holes=None):
    """Copy of conjprep.generic_prepare without the threshold gate (the deficits are still returned): the routing
    greedy runs even when the certificate thresholds exceed the holes; it raises when no reached hole is left, and its
    hole prediction is the exact SAFE-ALL permutation. N1 (fixed chambers vacant) is still required."""
    T = np.asarray(cert['target_chambers'], np.int64)
    holes = B._holes_mask(G, holes)
    cond = B.condition(G, T, None, holes)
    thr = {int(o): v for o, v in cert['thresholds'].items()}
    counts = np.bincount(G.orbit[np.flatnonzero(holes)], minlength=len(G.orbit_size))
    deficits = {o: v - int(counts[o]) for o, v in thr.items() if counts[o] < v}
    if not cond['fixed_vacant']:
        return None, cond, deficits
    tr, macros, extras = B._decoded_routes(G, cert)
    protected, free, word, steps = [], np.ones(len(G.letters), bool), [], []
    for j, row in enumerate(cert['rows']):
        t, y = row['t'], row['y']
        if not holes[t]:
            tree = B.weighted_tree(tr, y, free, macros[j:], extras[j])
            candidates = holes & (tree[0] >= 0)
            candidates[protected], candidates[t] = False, False
            points = np.flatnonzero(candidates)
            if not len(points):
                raise ValueError(f'no reached hole is available for target {t}')
            z = int(points[np.argmin(tree[0][points])])
            path = B.tree_path(tr, tree, macros[j:], z, extras[j])
            route = G.inverse(path)
            if len(path) != tree[0][z] or G.image(route, z) != y:
                raise AssertionError('reconstructed route does not match its shortest path')
            step = route + macros[j][0]
            p = G.perm(step)
            predicted = np.zeros(G.NP, bool)
            predicted[p[holes]] = True
            holes = predicted
            word.extend(step)
            steps.append({'row': j, 'target': t, 'hole': z, 'route_length': len(route), 'length': len(step)})
        if not holes[t] or not holes[protected].all():
            raise AssertionError('greedy step did not preserve the cleared target prefix')
        protected.append(t)
        free &= ~G.home[t, tr.capsof]
    if not holes[T].all():
        raise AssertionError('preparation did not clear the target')
    return {'word': G.pairs(word), 'predicted_holes': np.flatnonzero(holes).tolist(), 'length': len(word),
            'filled': len(steps), 'steps': steps}, cond, deficits



def main():
    a = sys.argv[1:]
    table, tfile, wfile, out = a[2], a[3], a[4], Path(a[5])
    out.mkdir(parents=True, exist_ok=True)
    ref_safe = json.loads(Path(a[a.index('--ref-safe') + 1]).read_text()) if '--ref-safe' in a else None
    times = {}
    t = time.time()
    C = GL.Ctx(table)
    R, M, st = C.R, C.M, C.st
    Wd = json.loads(Path(wfile).read_text())
    P = [(int(c), int(i)) for c, i in Wd['P']]
    Q = [(int(c), int(i)) for c, i in Wd['Q']]
    Pinv = CP.inverse_word(M, P)
    W = P + Q + Pinv
    T = np.asarray(sorted(json.loads(Path(tfile).read_text())['pieces']), np.int64)
    assert np.isin(T, C.off).all(), 'target pieces must be off the lattice'
    FREE = np.asarray(sorted(json.loads(Path(a[a.index('--free') + 1]).read_text())['pieces']), np.int64)         if '--free' in a else np.zeros(0, np.int64)
    prot = np.setdiff1d(np.setdiff1d(C.off, T), FREE)
    d_s = st.digest()
    holes0 = np.flatnonzero(~C.occ)
    pre = {'state_digest': d_s, 'off': int(len(C.off)), 'blocks': [sorted(map(int, b)) for b in C.bl.blocks],
           'target_block': T.tolist(), 'protected_off': prot.tolist(),
           'protected_poses': {int(p): matrix_json(st.pose(int(p))) for p in prot.tolist()},
           'holes': holes0.tolist(), 'W': {'P': [list(x) for x in P], 'Q': [list(x) for x in Q],
                                          'P_inverse': [list(x) for x in Pinv]}}
    # ghost premises of W
    g, n, U = C.ghost(W, np.concatenate([T, prot, FREE]))
    nT = len(T)
    nF = len(FREE)
    free_alive = bool(g.alive[n - nF:n].all())
    n = n - nF
    d0, q0 = C.contents(prot)
    lat = slice(n + nF, None)
    badW = np.sort(U[~g.alive[lat] | (g.dom[lat] != 0)])
    gpq, _, _ = C.ghost(P + Q, T, np.zeros(0, np.int64))
    badPinv = C.bad(Pinv)[0] if Pinv else np.zeros(0, np.int64)
    on_pq = gpq.alive[:nT] & (gpq.dom[:nT] == 0)
    pre['ghost'] = {'T_on_lattice_at_end': int((g.alive[:nT] & (g.dom[:nT] == 0)).sum()), 'T_size': nT,
                    'protected_exact_at_end': bool(g.alive[nT:n].all() and np.array_equal(g.dom[nT:n], d0)
                                                   and np.array_equal(g.q[nT:n], q0)),
                    'free_alive': free_alive, 'free': int(nF),
                    'badW': badW.tolist(), 'badW_size': int(len(badW)),
                    'badW_occupied': int(C.occ[badW].sum()), 'T_landing_chambers': sorted(g.q[:nT].tolist()),
                    'T_landing_in_badW': int(np.isin(g.q[:nT], badW).sum()),
                    'badPinv_size': int(len(badPinv)),
                    'T_after_PQ_on_lattice': int(on_pq.sum()),
                    'T_after_PQ_in_badPinv': int(np.isin(gpq.q[:nT][on_pq], badPinv).sum())}
    pre['ghost']['badW_digest'] = hashlib.sha256(json.dumps(badW.tolist()).encode()).hexdigest()
    times['ghost_premises'] = round(time.time() - t, 1)
    log(ghost={k: v for k, v in pre['ghost'].items() if not isinstance(v, list)})
    if (pre['ghost']['T_on_lattice_at_end'] != nT or not pre['ghost']['protected_exact_at_end']
            or not free_alive) and '--force' not in a:
        (out / 'pre.json').write_text(json.dumps(pre, indent=1) + '\n')
        (out / 'result.json').write_text(json.dumps({'result': 'ghost premises fail', 'times': times}, indent=1) + '\n')
        log(result='ghost premises fail')
        return
    skip = set(json.loads(Path(a[a.index('--skip-bad') + 1]).read_text())) if '--skip-bad' in a else set()
    if pre['ghost']['badW_digest'] in skip:
        (out / 'pre.json').write_text(json.dumps(pre, indent=1) + '\n')
        (out / 'result.json').write_text(json.dumps({'result': 'known no V', 'times': times}, indent=1) + '\n')
        log(result='known no V')
        return
    # SAFE-ALL menu rebuilt at S
    t = time.time()
    G = B.Alphabet(R, M)
    G.orbits()
    pre['menu'] = {'safe_grips': list(map(int, G.safe)), 'safe_grip_count': len(G.safe), 'letters': len(G.letters),
                   'blocked': {str(k): v for k, v in G.blocked.items()}, 'orbits': int(len(G.orbit_size)),
                   'singleton_orbits': int((G.orbit_size == 1).sum()), 'holes': int(len(G.holes))}
    if ref_safe is not None:
        ref = set(int(x) for x in ref_safe)
        pre['menu']['reference_safe_grips'] = len(ref)
        pre['menu']['reference_safe_not_safe_here'] = sorted(ref - set(G.safe))
        pre['menu']['safe_here_not_in_reference'] = sorted(set(G.safe) - ref)
    cond = B.condition(G, badW)
    pre['vacancy'] = {'N1': cond['fixed_vacant'], 'fixed_occupied': cond['fixed_occupied'],
                      'N2': cond['necessary'], 'deficits': cond['necessary_deficits'], 'by_orbit': cond['by_orbit']}
    times['menu_and_condition'] = round(time.time() - t, 1)
    log(menu={k: v for k, v in pre['menu'].items() if not isinstance(v, (list, dict))},
        N1=cond['fixed_vacant'], N2=cond['necessary'])
    # peeling certificate (Theorem B) and its sufficient condition; no V is needed when Bad(W) holds no lattice piece
    V, source = None, None
    t = time.time()
    if pre['ghost']['badW_occupied'] == 0:
        V, source = [], 'empty (Bad(W) vacant)'
    if V is None and cond['necessary'] and '--greedy' not in a:
        try:
            cert = json.loads(Path(a[a.index('--cert') + 1]).read_text()) if '--cert' in a else gc1(G, badW)
            counts = np.bincount(G.orbit[G.holes], minlength=len(G.orbit_size))
            suff = {o: {'threshold': int(v), 'holes': int(counts[int(o)])} for o, v in cert['thresholds'].items()}
            pre['certificate'] = {'peeled': cert['peeled'], 'of': cert['of'], 'fixed': cert['fixed'],
                                  'thresholds': suff, 'thresholds_equal_targets': cert['thresholds_equal_targets'],
                                  'all_reached': cert['all_reached'], 'family_bound': cert['family_bound'],
                                  'sufficient': all(x['holes'] >= x['threshold'] for x in suff.values())}
            (out / 'certificate.json').write_text(json.dumps(cert) + '\n')
            times['certificate'] = round(time.time() - t, 1)
            log(certificate={k: v for k, v in pre['certificate'].items() if k != 'thresholds'})
            t = time.time()
            prep, _, deficits = gp_forced(G, cert, badW)
            pre['certificate']['prepare_deficits'] = {str(k): v for k, v in deficits.items()}
            if prep is not None:
                V, source = [tuple(x) for x in prep['word']], 'certificate'
            times['prepare'] = round(time.time() - t, 1)
        except Exception as exc:  # noqa: BLE001
            pre['certificate_error'] = str(exc)[:400]
            log(certificate_error=pre['certificate_error'])
    if V is None and cond['necessary']:
        t = time.time()
        mask = np.zeros(G.NP, bool)
        mask[badW] = True
        va = VA2.Vacate2(G, mask, G.holes)
        gsec = float(a[a.index('--greedy-seconds') + 1]) if '--greedy-seconds' in a else 600.0
        deadline = time.time() + gsec
        gain0 = va.gain

        def gain(word):
            if time.time() > deadline:
                raise TimeoutError
            return gain0(word)
        va.gain = gain
        timed_out = False
        try:
            word, steps, ok = va.run()
        except TimeoutError:
            word, steps, ok, timed_out = [], [], False, True
        pre['greedy'] = {'ok': ok, 'steps': len(steps), 'length': len(word), 'left': int((va.occ & mask).sum()),
                         'timed_out': timed_out, 'seconds_limit': gsec}
        times['greedy'] = round(time.time() - t, 1)
        if ok:
            V, source = [tuple(x) for x in G.pairs(word)], 'greedy'
    (out / 'pre.json').write_text(json.dumps(pre, indent=1) + '\n')
    if V is None:
        (out / 'result.json').write_text(json.dumps({'result': 'no V', 'times': times}, indent=1) + '\n')
        log(result='no V', times=times)
        return
    log(V=len(V), source=source)
    # replay
    t = time.time()
    codes = CC.Codes()
    c_s = codes.of(st)
    poses_s = {}
    for p in np.flatnonzero(c_s < 0).tolist():
        poses_s[p] = to_tuple(st.pose(int(p)))
    parts = [('V', V), ('P', P), ('Q', Q), ('Pinv', Pinv), ('Vinv', CP.inverse_word(M, V))]
    rows = []
    (out / 'parts').mkdir(exist_ok=True)
    for name, part in parts:
        d_in = st.digest()
        for c, i in part:
            R.apply(c, i)
        d_out = st.digest()
        c_now = codes.of(st)
        if name == 'V':
            occ_v = np.ones(G.NP, bool)
            occ_v[B.hole_set(st)] = False
            after_v = {'occupied_badW': int(occ_v[badW].sum()),
                       'off_set_unchanged': bool(np.array_equal(c_now < 0, c_s < 0)),
                       'off_exact_poses_unchanged': all(to_tuple(st.pose(p)) == m for p, m in poses_s.items())}
        (out / 'parts' / f'{name}.json').write_text(json.dumps({'word': [list(x) for x in part], 'input_digest': d_in,
                                                                'output_digest': d_out}) + '\n')
        rows.append({'part': name, 'twists': len(part), 'input_digest': d_in, 'output_digest': d_out,
                     'off_end': int((c_now < 0).sum())})
        log(part=name, twists=len(part), off=rows[-1]['off_end'])
    times['replay'] = round(time.time() - t, 1)
    c_e = codes.of(st)
    inT = np.zeros(len(c_s), bool)
    inT[T] = True
    inF = np.zeros(len(c_s), bool)
    inF[FREE] = True
    off_s, off_e = c_s < 0, c_e < 0
    lat_s = ~off_s
    end = {'digest': st.digest(), 'off': int(off_e.sum()), 'T_restored': int((inT & ~off_e).sum()),
           'collateral': int((lat_s & off_e).sum()),
           'protected_still_off': int((off_s & ~inT & ~inF & off_e).sum()),
           'protected_exact_pose_changed': sum(1 for p, m in poses_s.items() if not inT[p] and not inF[p]
                                               and (not off_e[p] or to_tuple(st.pose(int(p))) != m)),
           'free_on_K': int((inF & ~off_e).sum()), 'free_still_off': int((inF & off_e).sum()),
           'free_pose_changed': sum(1 for p in FREE.tolist() if to_tuple(st.pose(int(p))) != poses_s[p]),
           'lattice_of_S_on_K_at_end': int((lat_s & ~off_e).sum()), 'lattice_of_S': int(lat_s.sum()),
           'lattice_of_S_same_exact_pose': int((lat_s & (c_e == c_s)).sum())}
    full = [list(x) for _, part in parts for x in part]
    (out / 'word-full.json').write_text(json.dumps({'word': full, 'input_digest': d_s,
                                                    'output_digest': end['digest']}) + '\n')
    res = {'input_digest': d_s, 'V_source': source, 'after_V': after_v, 'parts': rows, 'end': end,
           'length': len(full), 'times': times, 'seconds': round(time.time() - T0, 1)}
    (out / 'result.json').write_text(json.dumps(res, indent=1) + '\n')
    B.export_pose_table(st, str(out / 'end-table.json'))
    log(end=end, length=len(full), times=times)


if __name__ == '__main__':
    main()
