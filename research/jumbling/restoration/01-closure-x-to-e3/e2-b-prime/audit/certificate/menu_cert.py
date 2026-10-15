"""Audit 'certificate', items 3 to 6: SAFE-ALL menu, orbit partition, N1/N2, peeling certificate and V3, at E2.

Independent parts (not using buffer.Alphabet's results):
- SAFE-ALL grips from the simulator's reference classifier (classify_per_piece) at E2;
- letters: menu elements of each SAFE-ALL grip that lie in K+ and are not the identity;
- each letter's chamber permutation from the signature table alone (a chamber is identified by its set of caps; the
  image of x under h has the cap set h(caps(x)), looked up by a random 64-bit hash of cap sets);
- the orbit partition by breadth-first search on the letter graph;
- the certificate rows re-validated with these permutations and a breadth-first route search.
Then buffer.Alphabet / conjprep.generic_certificate / generic_prepare are rerun and compared, and V3 is replayed
letter by letter with a SAFE-ALL test at every prefix.

Usage: python3 -I menu_cert.py <theory dir> <bad.json from bad.py> <out json>
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
S = Path('<work>')
B4 = S / 'v26/B4'
PKG2 = S / 'pkg27/Claude-E-closure-E1E2-20261012'
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(S / 'v26'))
import buffer as B  # noqa: E402
import conjprep as CP  # noqa: E402
from blocks import CL, O  # noqa: E402
import restore  # noqa: E402
from exact import identity  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

T0 = time.time()
BPRIME = [16245, 16249, 32088]


def log(*a):
    print(round(time.time() - T0, 1), *a, file=sys.stderr, flush=True)


def occupancy(st):
    reg, kp = CL.ctx.regions, CL.ctx.kplus
    K = st._kpose()[st.pose_id]
    lat = np.flatnonzero(K >= 0)
    ch = reg.piece_of[reg.orbit_of[lat], kp.compose_vec(K[lat], reg.transport[lat])]
    occ = np.full(len(K), -1, np.int64)
    occ[ch] = lat
    return occ, K


def safe_test(st, c):
    """(straddled, off pieces inside) for grip c with the reference per-piece classifier."""
    cl = st.classify_per_piece(int(c), need_inside=True, stop_at_straddle=False)
    lat = st.lattice_flags()
    return cl.straddle is not None, int((~lat[np.asarray(cl.inside, np.int64)]).sum())


def components(NP, src, dst):
    s = np.concatenate([src, dst]).astype(np.int64)
    d = np.concatenate([dst, src]).astype(np.int64)
    key = np.unique(s * NP + d)
    s, d = key // NP, key % NP
    start = np.searchsorted(s, np.arange(NP + 1))
    comp = np.full(NP, -1, np.int64)
    n = 0
    for v in range(NP):
        if comp[v] >= 0:
            continue
        comp[v] = n
        frontier = np.array([v], np.int64)
        while len(frontier):
            a, b = start[frontier], start[frontier + 1]
            lens = b - a
            tot = int(lens.sum())
            if not tot:
                break
            offs = np.concatenate([[0], np.cumsum(lens)[:-1]])
            idx = np.repeat(a - offs, lens) + np.arange(tot)
            nb = np.unique(d[idx])
            nb = nb[comp[nb] < 0]
            comp[nb] = n
            frontier = nb
        n += 1
    return comp, n


def bfs(NP, root, src, dst):
    """Unit-weight distances from root along directed edges src -> dst."""
    order = np.argsort(src, kind='stable')
    s, d = src[order], dst[order]
    start = np.searchsorted(s, np.arange(NP + 1))
    dist = np.full(NP, -1, np.int64)
    dist[root] = 0
    frontier = np.array([root], np.int64)
    k = 0
    while len(frontier):
        a, b = start[frontier], start[frontier + 1]
        lens = b - a
        tot = int(lens.sum())
        if not tot:
            break
        offs = np.concatenate([[0], np.cumsum(lens)[:-1]])
        idx = np.repeat(a - offs, lens) + np.arange(tot)
        nb = np.unique(d[idx])
        nb = nb[dist[nb] < 0]
        k += 1
        dist[nb] = k
        frontier = nb
    return dist


def main():
    out = Path(sys.argv[3])
    badj = json.loads(Path(sys.argv[2]).read_text())
    badW = np.array(badj['badW'], np.int64)
    table = B4 / 'E2-table.json'
    R, M = O.start(str(table))
    st = R.st
    NP = len(st.pose_id)
    kp, data = CL.ctx.kplus, CL.ctx.data
    res = {'E2_digest': st.digest(), 'target_badW_from_bad_json': badW.tolist()}
    occ, K = occupancy(st)
    holes = np.flatnonzero(occ < 0)
    off = np.flatnonzero(K < 0)
    res['holes'] = holes.tolist()
    res['off'] = off.tolist()
    # ---- independent SAFE-ALL grips (reference classifier)
    t = time.time()
    safe, blocked = [], {}
    for c in range(600):
        strad, offin = safe_test(st, c)
        if not strad and not offin:
            safe.append(c)
        else:
            blocked[c] = {'straddled': strad, 'off_inside': offin}
    res['mine_safe_count'] = len(safe)
    res['mine_blocked'] = {str(k): v for k, v in blocked.items()}
    res['seconds_safe'] = round(time.time() - t, 1)
    log('safe', len(safe), sorted(blocked))
    # ---- independent letters and their chamber permutations from cap sets
    home = data.sig_matrix(np.arange(NP))
    rows, cols = np.nonzero(home)
    deg = np.bincount(rows, minlength=NP)
    maxdeg = int(deg.max())
    SIG = np.full((NP, maxdeg), 600, np.int64)
    pos_in_row = np.arange(len(rows)) - np.searchsorted(rows, rows)
    SIG[rows, pos_in_row] = cols
    rng = np.random.default_rng(20261012)
    w = rng.integers(1, 2 ** 63, size=600, dtype=np.uint64) * np.uint64(2) + np.uint64(1)
    wext = np.append(w, np.uint64(0))
    hsh = wext[SIG].sum(axis=1, dtype=np.uint64)
    res['cap_set_hash_unique'] = bool(len(np.unique(hsh)) == NP)
    res['caps_per_chamber_min_max'] = [int(deg.min()), maxdeg]
    hord = np.argsort(hsh)
    hs = hsh[hord]
    kid = kp.index_of_matrix(to_tuple(identity(4)))
    letters = []  # (c, i, k, xs, ys)
    t = time.time()
    for c in safe:
        xs = np.flatnonzero(home[:, c])
        for i in range(len(M.items)):
            k = kp.index_of_matrix(restore.element(M, c, i))
            if k < 0 or k == kid:
                continue
            if int(kp.perms[k][c]) != c:
                raise AssertionError('a K+ letter does not fix its pole')
            we = np.append(w[kp.perms[k].astype(np.int64)], np.uint64(0))
            hy = we[SIG[xs]].sum(axis=1, dtype=np.uint64)
            p = np.searchsorted(hs, hy)
            p = np.minimum(p, NP - 1)
            if not np.array_equal(hs[p], hy):
                raise AssertionError(f'letter ({c}, {i}): an image cap set is not a chamber')
            ys = hord[p]
            if not np.array_equal(np.sort(ys), xs):
                raise AssertionError(f'letter ({c}, {i}) does not permute its cap')
            letters.append((c, i, k, xs, ys))
    res['mine_letter_count'] = len(letters)
    res['mine_letters_per_grip'] = sorted({sum(1 for l in letters if l[0] == c) for c in safe})
    res['seconds_letters'] = round(time.time() - t, 1)
    log('letters', len(letters))
    # ---- orbit partition by BFS
    t = time.time()
    src = np.concatenate([l[3] for l in letters])
    dst = np.concatenate([l[4] for l in letters])
    keep = src != dst
    comp, ncomp = components(NP, src[keep], dst[keep])
    size = np.bincount(comp, minlength=ncomp)
    res['mine_orbits'] = int(ncomp)
    res['mine_singletons'] = int((size == 1).sum())
    res['mine_largest_orbits'] = sorted(size.tolist())[-8:]
    res['seconds_orbits'] = round(time.time() - t, 1)
    log('orbits', ncomp, int((size == 1).sum()))
    # ---- N1, N2 for the target Bad(W)
    tgt_orbits = sorted(set(comp[badW].tolist()))
    hole_counts = np.bincount(comp[holes], minlength=ncomp)
    tgt_counts = np.bincount(comp[badW], minlength=ncomp)
    fixed_t = badW[size[comp[badW]] == 1]
    res['N1_fixed_targets'] = fixed_t.tolist()
    res['N1_fixed_occupied_targets'] = [int(z) for z in fixed_t if occ[z] >= 0]
    res['N1'] = not res['N1_fixed_occupied_targets']
    res['N2_table'] = [{'orbit_least_chamber': int(np.flatnonzero(comp == o)[0]), 'size': int(size[o]),
                        'targets': int(tgt_counts[o]), 'holes': int(hole_counts[o]),
                        'target_chambers': [int(z) for z in badW if comp[z] == o],
                        'hole_chambers': [int(z) for z in holes if comp[z] == o]} for o in tgt_orbits]
    res['N2'] = all(r['holes'] >= r['targets'] for r in res['N2_table'])
    res['holes_outside_target_orbits'] = [int(z) for z in holes if comp[z] not in tgt_orbits]
    # ---- the producing code path: buffer.Alphabet and its orbits
    t = time.time()
    G = B.Alphabet(R, M)
    G.orbits()
    res['alphabet'] = {'safe_count': len(G.safe), 'letters': len(G.letters), 'orbits': int(len(G.orbit_size)),
                       'singletons': int((G.orbit_size == 1).sum()),
                       'blocked': {str(k): v for k, v in G.blocked.items()}}
    res['alphabet_safe_equals_mine'] = list(map(int, G.safe)) == safe
    res['alphabet_blocked_equals_mine'] = {str(k): v for k, v in G.blocked.items()} == res['mine_blocked']
    mine_by = {(l[0], l[1]): l for l in letters}
    same_letters = len(G.letters) == len(letters) and all(
        (c, i) in mine_by and mine_by[(c, i)][2] == k and np.array_equal(mine_by[(c, i)][4], ys)
        for c, i, k, ys in G.letters)
    res['alphabet_letters_and_permutations_equal_mine'] = bool(same_letters)
    # same partition: bijection between my components and G's orbits
    pairs = np.unique(comp.astype(np.int64) * (len(G.orbit_size) + 1) + G.orbit)
    res['alphabet_partition_equals_mine'] = bool(len(pairs) == ncomp == len(G.orbit_size))
    res['seconds_alphabet'] = round(time.time() - t, 1)
    cond = B.condition(G, badW)
    res['buffer_condition'] = {'N1': cond['fixed_vacant'], 'N2': cond['necessary'], 'by_orbit': cond['by_orbit']}
    # ---- compare with pre.json and the segment-2 resource row
    pre = json.loads((B4 / 'c3/pre.json').read_text())
    rs2 = json.loads((PKG2 / 'resources/resources-E2.json').read_text())
    res['pre_menu_matches'] = {
        'safe_grips': pre['menu']['safe_grips'] == safe, 'safe_grip_count': pre['menu']['safe_grip_count'] == len(safe),
        'letters': pre['menu']['letters'] == len(letters), 'orbits': pre['menu']['orbits'] == ncomp,
        'singletons': pre['menu']['singleton_orbits'] == res['mine_singletons'],
        'blocked': pre['menu']['blocked'] == res['mine_blocked'], 'holes': pre['menu']['holes'] == len(holes)}
    res['pre_ghost_badW_matches'] = pre['ghost']['badW'] == badW.tolist()
    res['pre_holes_match'] = pre['holes'] == holes.tolist()
    res['pre_vacancy'] = pre['vacancy']
    res['pre_vacancy_rows_match_mine'] = sorted((r['size'], r['target'], r['holes']) for r in pre['vacancy']['by_orbit']) \
        == sorted((r['size'], r['targets'], r['holes']) for r in res['N2_table'])
    res['seg2_resources_E2'] = {'digest_is_E2': rs2['digest'] == res['E2_digest'],
                                'grips_equal': rs2['safe_all']['grips'] == safe,
                                'grip_count': rs2['safe_all']['grip_count'], 'letters': rs2['safe_all']['letters'],
                                'orbits': rs2['safe_all']['orbits'],
                                'singletons': rs2['safe_all']['singleton_orbits'],
                                'blocked_equal': rs2['safe_all']['blocked'] == res['mine_blocked'],
                                'holes_equal': rs2['holes'] == holes.tolist()}
    # ---- peeling certificate: rerun the producing function and compare with the delivered file
    t = time.time()
    cert = CP.generic_certificate(G, badW)
    cert = json.loads(json.dumps(cert))
    delivered = json.loads((B4 / 'c3/certificate.json').read_text())
    res['certificate_rerun_equals_delivered'] = cert == delivered
    if cert != delivered:
        res['certificate_differences'] = {k: [cert.get(k), delivered.get(k)] for k in set(cert) | set(delivered)
                                          if cert.get(k) != delivered.get(k)}
    res['seconds_certificate'] = round(time.time() - t, 1)
    # ---- independent validation of the delivered certificate rows with my permutations
    idx = {(l[0], l[1]): n for n, l in enumerate(letters)}

    def perm_of(word):
        q = np.arange(NP)
        for c, i in word:
            l = letters[idx[(int(c), int(i))]]
            # chronological source -> destination: q maps each start point to its current point
            m = np.arange(NP)
            m[l[3]] = l[4]
            q = m[q]
        return q

    T = badW
    inT = np.zeros(NP, bool)
    inT[T] = True
    movable = T[size[comp[T]] > 1]
    rows_check = []
    protected = []
    capsof = np.array([l[0] for l in letters], np.int64)
    ok_all = sorted(r['t'] for r in delivered['rows']) == sorted(movable.tolist()) \
        and delivered['peeled'] == len(movable) == delivered['of'] and delivered['fixed'] == fixed_t.tolist() \
        and delivered['target_chambers'] == T.tolist() and delivered['source_digest'] == res['E2_digest']
    res['certificate_header_ok'] = bool(ok_all)
    macros = []
    for r in delivered['rows']:
        word = [tuple(x) for x in r['word']]
        if any((c, i) not in idx for c, i in word):
            raise AssertionError('a certificate word letter is not SAFE-ALL at E2')
        macros.append((word, perm_of(word)))
    for j, r in enumerate(delivered['rows']):
        t_j, y_j = int(r['t']), int(r['y'])
        word, p = macros[j]
        o = comp[t_j]
        chk = {'t': t_j, 'y': y_j, 'E': word}
        chk['E_maps_y_to_t'] = bool(p[y_j] == t_j)
        chk['E_moves_t'] = bool(p[t_j] != t_j)
        chk['E_fixes_P'] = bool(np.array_equal(p[protected], protected)) if protected else True
        chk['y_not_in_T'] = bool(not inT[y_j])
        chk['orbit_size'] = int(size[o])
        chk['conjugate_edges'] = len(r['conjugate_edges'])
        # route graph: free letters (caps avoiding P_j) and macros E_k (k >= j) both ways
        free = np.ones(len(letters), bool)
        for q in protected:
            free &= ~home[q, capsof]
        s_list, d_list = [], []
        for n in np.flatnonzero(free).tolist():
            s_list.append(letters[n][3])
            d_list.append(letters[n][4])
        mac_fix = True
        for k in range(j, len(macros)):
            wk, pk = macros[k]
            if len(wk) != 1:
                raise AssertionError('macro of length > 1: unit-weight BFS does not apply')
            sup = np.flatnonzero(pk != np.arange(NP))
            s_list += [sup, pk[sup]]
            d_list += [pk[sup], sup]
            if protected and not np.array_equal(pk[protected], protected):
                mac_fix = False
        chk['macros_fix_P'] = mac_fix
        src_r, dst_r = np.concatenate(s_list), np.concatenate(d_list)
        sel = comp[src_r] == o
        dist = bfs(NP, y_j, src_r[sel], dst_r[sel])
        pts = np.flatnonzero(comp == o)
        inP = np.isin(pts, protected)
        cand = pts[~inP]
        unreached = cand[(dist[cand] < 0) & (cand != t_j)]
        ecc = int(dist[cand].max())
        u = len(unreached)
        mine = {'E_length': len(word), 'ecc': ecc, 'orbit_minus_P': int(len(cand)),
                'reached': int((dist[cand] >= 0).sum()), 'u_j': u,
                'holes_needed': int(inP.sum()) + 1 + u, 'bound': len(word) + ecc}
        chk['mine'] = mine
        chk['row_values_equal_mine'] = all(r[k] == v for k, v in mine.items()) and r['unreached'] == unreached.tolist() \
            and r['orbit'] == int(G.orbit[t_j])
        rows_check.append(chk)
        protected.append(t_j)
    res['certificate_rows_check'] = rows_check
    thr = {}
    for chk in rows_check:
        o = int(comp[chk['t']])
        thr[o] = max(thr.get(o, 0), chk['mine']['holes_needed'])
    res['mine_thresholds'] = {str(int(np.flatnonzero(comp == o)[0])): v for o, v in thr.items()}
    res['mine_thresholds_equal_targets'] = all(v == int(tgt_counts[o]) for o, v in thr.items())
    res['mine_all_reached'] = all(chk['mine']['u_j'] == 0 for chk in rows_check)
    res['mine_family_bound'] = sum(chk['mine']['bound'] for chk in rows_check)
    res['mine_sufficient'] = all(int(hole_counts[o]) >= v for o, v in thr.items())
    res['delivered_thresholds_family_bound'] = [delivered['thresholds'], delivered['family_bound'],
                                                delivered['all_reached'], delivered['thresholds_equal_targets']]
    res['pre_certificate'] = pre['certificate']
    # ---- V3: rerun generic_prepare, compare, predict holes with my permutations, replay with SAFE-ALL tests
    prep, _, deficits = CP.generic_prepare(G, delivered, badW)
    Vfile = [tuple(x) for x in json.loads((B4 / 'c3/parts/V.json').read_text())['word']]
    res['generic_prepare_word'] = prep['word'] if prep else None
    res['generic_prepare_steps'] = prep['steps'] if prep else None
    res['generic_prepare_equals_V_file'] = prep is not None and [tuple(x) for x in prep['word']] == Vfile
    pv = perm_of(Vfile)
    pred = np.sort(pv[holes])
    res['mine_predicted_holes_after_V'] = pred.tolist()
    res['mine_predicted_holes_equal_badW'] = bool(np.array_equal(pred, badW))
    bpose0 = {p: to_tuple(st.pose(p)) for p in BPRIME}
    per = []
    for n, (c, i) in enumerate(Vfile):
        strad, offin = safe_test(st, c)
        k = kp.index_of_matrix(restore.element(M, c, i))
        per.append({'n': n, 'letter': [c, i], 'straddled': strad, 'off_inside': offin, 'in_K': bool(k >= 0),
                    'identity': bool(k == kid)})
        R.apply(c, i)
    occ_v, K_v = occupancy(st)
    res['V_prefix_checks'] = per
    res['V_all_letters_safe_all'] = all(not x['straddled'] and not x['off_inside'] and x['in_K'] and not x['identity']
                                        for x in per)
    res['after_V'] = {'digest': st.digest(), 'holes': np.flatnonzero(occ_v < 0).tolist(),
                      'badW_vacant': int((occ_v[badW] < 0).sum()),
                      'off_set_unchanged': bool(np.array_equal(np.flatnonzero(K_v < 0), off)),
                      'Bprime_exact_pose_unchanged': all(to_tuple(st.pose(p)) == bpose0[p] for p in BPRIME)}
    res['delivered_result_after_V'] = json.loads((B4 / 'c3/result.json').read_text())['after_V']
    res['seconds'] = round(time.time() - T0, 1)
    out.write_text(json.dumps(res, indent=1) + '\n')
    log('done')


if __name__ == '__main__':
    main()
