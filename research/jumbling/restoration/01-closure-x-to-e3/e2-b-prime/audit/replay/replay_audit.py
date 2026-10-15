"""Independent replay audit of segment 3 (E2 -> E3) and of the type-S word, with my own loop.

Usage: python3 -I -B replay_audit.py W3|S

Only the simulator's state, its `R.apply` (admissibility oracle) and its reference classifier
`classify_per_piece` are used from the research code. Pose keys, exact products, digests and
table comparisons are computed here with my own integer arithmetic in Q(sqrt5). Nothing from
replay_check.py, prefixrec.py, closure2.py or seg3check.py is called.
"""
import hashlib
import json
import sys
import time
from math import gcd
from pathlib import Path

sys.dont_write_bytecode = True
SP = Path('<work>')
TH = SP / 'wt-e3/research/jumbling/theory'
AUD = SP / 'v26/B4/audit/replay'
B4 = SP / 'v26/B4'
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
import numpy as np  # noqa: E402
import blocks as BL  # noqa: E402
import restore  # noqa: E402
from sim.kplus import matrix_json  # noqa: E402

O = BL.O
T0 = time.time()
BP = [16245, 16249, 32088]
E2_TABLE = B4 / 'E2-table.json'
E3_TABLE = B4 / 'c3/E3-table.json'
E2_DIGEST = '19e404d438fd929b0c6480c35374aa5147b361831806774ff65ca0f4739bc5ff'
E2_SHA = '649576e69ede65a6203dfbebd3b879a68e40250abaf18a0f08ad86084834d893'
E3_DIGEST = 'fb199d9997a240c69ce91b39953d2434939f094f183f566067ca1779fe10c441'
E3_SHA = '53cec2e37f3b858924c216d45d99f1563f0f1e57d1d8a78fa39946c1c965d889'
S_DIGEST = '3ba8e6c2e1f84d9f89a1e7b9cc408ed60e33e589524f57856344f00e5bba839a'
S5 = 5 ** 0.5
KF = (1452 + 484 * S5) / 125      # alpha |n|^2, the cut offset in the units of the pole vectors


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


# ------------------------------------------------------------------ own exact arithmetic in Q(sqrt5)
def norm_entry(v):
    a, b, d = (int(x) for x in v)
    if d == 0:
        raise ValueError('zero denominator')
    if d < 0:
        a, b, d = -a, -b, -d
    g = gcd(gcd(abs(a), abs(b)), d)
    return [a // g, b // g, d // g]


def norm_key(mj):
    rows = [[norm_entry(x) for x in row] for row in mj]
    if len(rows) != 4 or any(len(r) != 4 for r in rows):
        raise ValueError('not 4 x 4')
    return json.dumps(rows, separators=(',', ':'))


def cd_of_key(key):
    """(A, B, D) with matrix = (A + B sqrt5) / D, integer entries, common denominator D."""
    rows = json.loads(key)
    D = 1
    for row in rows:
        for _, _, d in row:
            D = D * d // gcd(D, d)
    A = [[a * (D // d) for a, _, d in row] for row in rows]
    B = [[b * (D // d) for _, b, d in row] for row in rows]
    return A, B, D


def cd_mul(X, Y):
    A1, B1, D1 = X
    A2, B2, D2 = Y
    A = [[sum(A1[i][k] * A2[k][j] + 5 * B1[i][k] * B2[k][j] for k in range(4)) for j in range(4)] for i in range(4)]
    B = [[sum(A1[i][k] * B2[k][j] + B1[i][k] * A2[k][j] for k in range(4)) for j in range(4)] for i in range(4)]
    return A, B, D1 * D2


def cd_eq(X, Y):
    A1, B1, D1 = X
    A2, B2, D2 = Y
    return all(A1[i][j] * D2 == A2[i][j] * D1 and B1[i][j] * D2 == B2[i][j] * D1 for i in range(4) for j in range(4))


def cd_float(X):
    A, B, D = X
    return np.array([[(A[i][j] + B[i][j] * S5) / D for j in range(4)] for i in range(4)])


I4 = ([[int(i == j) for j in range(4)] for i in range(4)], [[0] * 4 for _ in range(4)], 1)
IDENT_KEY = norm_key([[[int(i == j), 0, 1] for j in range(4)] for i in range(4)])


class Keys:
    """Global numbering of exact pose matrices by their normalised JSON key."""

    def __init__(self):
        self.code, self.keys, self.cd, self.fl = {}, [], [], []
        self._bypose = {}
        self._farr = None

    def of_key(self, key):
        k = self.code.get(key)
        if k is None:
            k = len(self.keys)
            self.code[key] = k
            self.keys.append(key)
            cd = cd_of_key(key)
            self.cd.append(cd)
            self.fl.append(cd_float(cd))
            self._farr = None
        return k

    def codes(self, st):
        arr = np.empty(len(st._poses), np.int64)
        for u, p in enumerate(st._poses):
            k = self._bypose.get(p.key)
            if k is None:
                k = self.of_key(norm_key(matrix_json(st._pose_matrix(u))))
                self._bypose[p.key] = k
            arr[u] = k
        return arr[np.asarray(st.pose_id, np.int64)]

    def farr(self):
        if self._farr is None:
            self._farr = np.array(self.fl)
        return self._farr


def table_codes(path, keys):
    raw = Path(path).read_bytes()
    doc = json.loads(raw)
    pk = np.array([keys.of_key(norm_key(m)) for m in doc['poses']], np.int64)
    pid = np.asarray(doc['pose_id'], np.int64)
    if len(pid) != 177120:
        raise ValueError('table size')
    return pk[pid], doc, hashlib.sha256(raw).hexdigest()


def own_digest(codes, keys):
    """The simulator's digest definition, recomputed from my own pose keys."""
    ident = keys.code.get(IDENT_KEY, -1)
    order = np.argsort(codes, kind='stable')
    sc = codes[order]
    cuts = np.flatnonzero(np.diff(sc)) + 1
    items = []
    for grp in np.split(order, cuts):
        c = int(codes[grp[0]])
        if c == ident:
            continue
        items.append((json.dumps(json.loads(keys.keys[c])), np.sort(grp).astype(np.int32).tobytes()))
    items.sort()
    h = hashlib.sha256()
    for key, b in items:
        h.update(key.encode())
        h.update(b)
    return h.hexdigest()


def kpos_of(keys, codes, kplus_codes):
    """True where the exact pose is one of the 7200 K+ matrices (my own key comparison)."""
    return kplus_codes[codes]


# ------------------------------------------------------------------ replay loop
def replay(R, keys, parts, ref, C, NF, isK, out_rows, letter_cd, memo):
    """Apply parts in order with R.apply; per twist: admissibility, reference classifier, exact
    composition, float centroid side test and the metrics. Returns (codes, stage records, failures)."""
    st = R.st
    codes0, lat0, off0 = ref['codes0'], ref['lat0'], ref['off0']
    prev = keys.codes(st)
    prefix = 0
    stages, failures = [], []
    coll_hist = []
    for name, path in parts:
        d = json.loads(Path(path).read_text())
        word = [tuple(x) for x in d['word']]
        dig_in = st.digest()
        rec_part = {'part': name, 'file': str(path.relative_to(SP)), 'twists': len(word),
                    'input_digest_file': d.get('input_digest'), 'input_digest_state': dig_in,
                    'input_digest_ok': dig_in == d.get('input_digest')}
        if not rec_part['input_digest_ok']:
            failures.append(f'{name}: input digest differs')
        for j, (c, i) in enumerate(word):
            c, i = int(c), int(i)
            # reference classifier (exact, per piece, unfiltered) before the twist
            cl = st.classify_per_piece(c, need_inside=True, stop_at_straddle=False)
            ref_straddle = cl.straddle is not None
            ref_unc = int(cl.uncertain)
            ref_inside = np.asarray(cl.inside, np.int64)
            # float centroid side test before the twist
            X = np.einsum('nij,nj->ni', keys.farr()[prev], C)
            s = X @ NF[c] - KF
            prevK = isK[prev]
            try:
                R.apply(c, i)
            except RuntimeError as e:
                failures.append(f'prefix {prefix + 1} ({c},{i}) in {name}: {e}')
                out_rows.append({'prefix': prefix + 1, 'part': name, 'index': j, 'letter': [c, i],
                                 'admissible': False, 'error': str(e)})
                return prev, stages, failures, coll_hist
            prefix += 1
            moved = int(st.journal[-1]['moved'])
            cur = keys.codes(st)
            changed = np.flatnonzero(cur != prev)
            # exact composition: new pose = g . old pose for every changed piece
            exact_ok = True
            gcd_ = letter_cd(c, i)
            pairs = np.unique(np.stack([prev[changed], cur[changed]], 1), axis=0) if len(changed) else np.zeros((0, 2), np.int64)
            for a, b in pairs.tolist():
                mk = (a, c, i)
                if mk in memo:
                    if memo[mk] != b:
                        exact_ok = False
                    continue
                ok = cd_eq(cd_mul(gcd_, keys.cd[a]), keys.cd[b])
                if ok:
                    memo[mk] = b
                else:
                    exact_ok = False
            inside = np.zeros(len(cur), bool)
            inside[changed] = True
            fmin_in = float(s[inside].min()) if inside.any() else None
            fmax_out = float(s[~inside].max())
            float_ok = (fmin_in is None or fmin_in > 0) and fmax_out < 0
            ref_eq = bool(len(ref_inside) == len(changed) and np.array_equal(np.sort(ref_inside), changed))
            Kc = isK[cur]
            off = ~Kc
            coll = np.flatnonzero(lat0 & off)
            bp_on = int(Kc[BP].sum())
            offE2_still_off = np.flatnonzero(off0 & off)
            offE2_pose_diff = int((cur[offE2_still_off] != codes0[offE2_still_off]).sum())
            exact_e2 = int((lat0 & (cur == codes0)).sum())
            dig = st.digest()
            row = {'prefix': prefix, 'part': name, 'index': j, 'letter': [c, i], 'admissible': True,
                   'moved_journal': moved, 'changed': int(len(changed)), 'moved_eq_changed': moved == len(changed),
                   'ref_straddle': ref_straddle, 'ref_uncertain': ref_unc, 'ref_inside_eq_changed': ref_eq,
                   'exact_composition_ok': exact_ok, 'float_side_ok': bool(float_ok),
                   'float_min_inside': fmin_in, 'float_max_outside': fmax_out,
                   'off_pieces_moved': int((~prevK[changed]).sum()),
                   'off': int(off.sum()), 'collateral': int(len(coll)),
                   'collateral_ids': coll.tolist() if len(coll) <= 20 else None,
                   'Bprime_on_K': bp_on, 'E2off_still_off': int(len(offE2_still_off)),
                   'E2off_still_off_pose_changed': offE2_pose_diff,
                   'E2_lattice_exact_pose': exact_e2, 'digest': dig}
            for flag in ('moved_eq_changed', 'ref_inside_eq_changed', 'exact_composition_ok', 'float_side_ok'):
                if not row[flag]:
                    failures.append(f'prefix {prefix}: {flag} false')
            if ref_straddle or ref_unc:
                failures.append(f'prefix {prefix}: reference classifier straddle={ref_straddle} uncertain={ref_unc}')
            out_rows.append(row)
            coll_hist.append((prefix, int(len(coll))))
            prev = cur
        dig_out = st.digest()
        rec_part.update({'output_digest_file': d.get('output_digest'), 'output_digest_state': dig_out,
                         'output_digest_ok': dig_out == d.get('output_digest')})
        if not rec_part['output_digest_ok']:
            failures.append(f'{name}: output digest differs')
        Kc = isK[prev]
        coll = np.flatnonzero(lat0 & ~Kc)
        rec_part.update({'prefix_end': prefix, 'end_digest': dig_out, 'end_digest_own': own_digest(prev, keys),
                         'off_end': int((~Kc).sum()), 'off_ids': np.flatnonzero(~Kc).tolist() if (~Kc).sum() <= 20 else None,
                         'Bprime_on_K': int(Kc[BP].sum()),
                         'collateral_vs_E2': int(len(coll)), 'collateral_ids': coll.tolist() if len(coll) <= 50 else None,
                         'E2_lattice_on_K': int((lat0 & Kc).sum()),
                         'E2_lattice_exact_pose': int((lat0 & (prev == codes0)).sum()),
                         'E2off_exact_pose_kept': int((off0 & (prev == codes0)).sum())})
        if rec_part['end_digest_own'] != dig_out:
            failures.append(f'{name}: own digest differs from simulator digest')
        stages.append(rec_part)
        log(part=name, prefix=prefix, off=rec_part['off_end'], coll=rec_part['collateral_vs_E2'],
            exact=rec_part['E2_lattice_exact_pose'], digest=dig_out[:16])
    return prev, stages, failures, coll_hist


def main():
    mode = sys.argv[1]
    keys = Keys()
    res = {'mode': mode}
    # own parse of the E2 table; sha256
    t_codes, t_doc, t_sha = table_codes(E2_TABLE, keys)
    res['E2_table_sha256'] = t_sha
    res['E2_table_sha256_ok'] = t_sha == E2_SHA
    res['E2_table_digest_field'] = t_doc['digest']
    # K+ codes: the 7200 exact K+ matrices of the simulator, keyed by my normalisation
    R, M = O.start(str(E2_TABLE))
    st = R.st
    kp = st.ctx.kplus
    kplus_keys = set()
    for k in range(len(kp.perms)):
        kplus_keys.add(keys.of_key(norm_key(matrix_json(kp.matrix(k)))))
    res['kplus_distinct_matrices'] = len(kplus_keys)
    res['kplus_index0_is_identity'] = norm_key(matrix_json(kp.matrix(0))) == IDENT_KEY

    def isK_arr():
        a = np.zeros(len(keys.keys) + 200000, bool)
        a[list(kplus_keys)] = True
        return a

    res['E2_digest_sim'] = st.digest()
    res['E2_digest_ok'] = res['E2_digest_sim'] == E2_DIGEST
    codes0 = keys.codes(st)
    res['E2_load_equals_own_parse'] = bool(np.array_equal(codes0, t_codes))
    res['E2_digest_own'] = own_digest(codes0, keys)
    isK = isK_arr()
    off0 = ~isK[codes0]
    simK = st._kpose()[st.pose_id] >= 0
    res['E2_offflag_own_equals_sim'] = bool(np.array_equal(~off0, simK))
    lat0 = ~off0
    res['E2_off_set'] = np.flatnonzero(off0).tolist()
    res['E2_off_set_is_Bprime'] = res['E2_off_set'] == BP
    res['E2_distinct_poses'] = int(len(np.unique(codes0)))
    res['E2_Bprime_pose_keys'] = sorted({keys.keys[int(codes0[p])] for p in BP})
    res['E2_lattice_pieces'] = int(lat0.sum())
    # save my E2 pose record
    np.save(AUD / f'E2-codes-{mode}.npy', codes0.astype(np.int32))
    (AUD / f'E2-keys-{mode}.json').write_text(json.dumps(
        {'keys': [keys.keys[k] for k in range(len(keys.keys))],
         'note': 'codes index into keys; key = normalised exact matrix [a,b,d] = (a+b sqrt5)/d'}))
    res['E2_codes_npy_sha256'] = hashlib.sha256((AUD / f'E2-codes-{mode}.npy').read_bytes()).hexdigest()
    log(stage='E2 loaded', digest_ok=res['E2_digest_ok'], off=res['E2_off_set'], same=res['E2_load_equals_own_parse'])

    # letters
    lcache = {}

    def letter_cd(c, i):
        if (c, i) not in lcache:
            lcache[(c, i)] = cd_of_key(norm_key(matrix_json(R.g(c, i))))
        return lcache[(c, i)]

    def inverse_index(c, i):
        g = letter_cd(c, i)
        hits = [j for j in range(R.n_items) if cd_eq(cd_mul(g, letter_cd(c, j)), I4)]
        return hits

    def is_kplus_letter(c, i):
        return keys.of_key(norm_key(matrix_json(R.g(c, i)))) in kplus_keys

    if mode == 'W3':
        parts_dir = B4 / 'c3/parts'
        parts = [(n, parts_dir / f'{n}.json') for n in ('V', 'P', 'Q', 'Pinv', 'Vinv')]
        words = {n: [tuple(x) for x in json.loads(p.read_text())['word']] for n, p in parts}
        full = json.loads((B4 / 'c3/word-full.json').read_text())
        res['word_full_equals_parts'] = [tuple(x) for x in full['word']] == sum((words[n] for n, _ in parts), [])
        res['word_full_digests'] = [full['input_digest'] == E2_DIGEST, full['output_digest'] == E3_DIGEST]
        res['part_lengths'] = {n: len(w) for n, w in words.items()}
        p4 = [tuple(x) for x in json.loads((B4 / 'P4.json').read_text())]
        w3 = json.loads((B4 / 'w3-P4-162-1.json').read_text())
        res['P_equals_P4json_and_w3P'] = words['P'] == p4 == [tuple(x) for x in w3['P']]
        res['Q_equals_w3Q_and_F'] = words['Q'] == [tuple(x) for x in w3['Q']] == [(162, 1)]
        # inverse letters by exact matrices
        inv = {}
        for (c, i) in sorted(set(sum(words.values(), []))):
            h = inverse_index(c, i)
            inv[(c, i)] = h
        res['inverse_index_unique'] = all(len(h) == 1 for h in inv.values())
        res['inverse_table'] = {f'{c},{i}': h for (c, i), h in inv.items()}

        def mirror(w):
            return [(c, inv[(c, i)][0]) for c, i in reversed(w)]
        res['Pinv_is_mirror_inverse_of_P'] = words['Pinv'] == mirror(words['P'])
        res['Vinv_is_mirror_inverse_of_V'] = words['Vinv'] == mirror(words['V'])
        # iterated conjugation from R and x_k
        Rl, xs = (237, 13), [(91, 6), (154, 7), (133, 1), (183, 6)]
        for x in xs:
            if x not in inv:
                inv[x] = inverse_index(*x)
        Pk = [Rl]
        lengths = [1]
        for x in xs:
            Pk = Pk + [x] + mirror(Pk)
            lengths.append(len(Pk))
        res['iterated_conjugation_lengths'] = lengths
        res['P_equals_iterated_conjugation'] = Pk == words['P']
        res['R_inverse_is_237_47'] = inv[(237, 13)] == [47]
        res['x0_inverse_91_7'] = inv[(91, 6)] == [7]
        res['x1_inverse_154_6'] = inv[(154, 7)] == [6]
        res['kplus_letters'] = {f'{c},{i}': is_kplus_letter(c, i) for (c, i) in sorted(set(sum(words.values(), [])))}
        expected = {'V': ('98d58699', 3, 0, 0, 158120), 'P': ('1509fd07', 10, 3, 10, 155218),
                    'Q': ('6edd6cbc', 10, 3, 10, 154037), 'Pinv': ('79abe255', 0, 3, 0, 155986),
                    'Vinv': ('fb199d99', 0, 3, 0, 174026)}
    else:
        D = B4 / 'cS-P4-162-1'
        parts = [(n, D / 'parts' / f'{n}.json') for n in ('P', 'Q0', 'F', 'Pinv')]
        words = {n: [tuple(x) for x in json.loads(p.read_text())['word']] for n, p in parts}
        full = json.loads((D / 'word-full.json').read_text())
        res['word_full_len'] = len(full['word'])
        res['word_full_equals_parts'] = [tuple(x) for x in full['word']] == sum((words[n] for n, _ in parts), [])
        res['part_lengths'] = {n: len(w) for n, w in words.items()}
        c3w = {n: [tuple(x) for x in json.loads((B4 / 'c3/parts' / f'{n}.json').read_text())['word']]
               for n in ('V', 'P', 'Q', 'Pinv')}
        res['S_P_equals_P4'] = words['P'] == c3w['P']
        res['S_Pinv_equals_P4inv'] = words['Pinv'] == c3w['Pinv']
        res['S_F_equals_162_1'] = words['F'] == [(162, 1)] == c3w['Q']
        res['S_Q0_equals_V3'] = words['Q0'] == c3w['V']
        res['word_full_digests'] = [full['input_digest'] == E2_DIGEST, full['output_digest'] == S_DIGEST]
        pre = json.loads((D / 'pre.json').read_text())
        cert = json.loads((D / 'certificate-Q0.json').read_text())
        rj = json.loads((D / 'result.json').read_text())
        res['pre_consistent'] = {'state_digest': pre['state_digest'] == E2_DIGEST,
                                 'P': [tuple(x) for x in pre['P']] == words['P'],
                                 'F': [tuple(x) for x in pre['F']] == words['F'],
                                 'after_P_digest_is_P_output': pre['after_P_digest'] == json.loads(parts[0][1].read_text())['output_digest'],
                                 'certificate_source_is_after_P': cert['source_digest'] == pre['after_P_digest']}
        chain = []
        for (n1, p1), (n2, p2) in zip(parts, parts[1:]):
            chain.append(json.loads(p1.read_text())['output_digest'] == json.loads(p2.read_text())['input_digest'])
        res['parts_digest_chain'] = chain
        res['result_parts_match_files'] = [
            (rp['input_digest'], rp['output_digest']) == (json.loads(p.read_text())['input_digest'], json.loads(p.read_text())['output_digest'])
            for rp, (_, p) in zip(rj['parts'], parts)]
        expected = None
        res['result_json'] = {'parts': [{k: rp[k] for k in ('part', 'twists', 'off_end', 'lattice_of_start_off')} for rp in rj['parts']],
                              'end': rj['end'], 'length': rj['length']}

    # float geometry for the side test
    C = np.load(SP / 'wt-e3/work/jumbling-cache/home_centroids.npy')
    NF = np.asarray(restore._NF, float)
    ref = {'codes0': codes0, 'lat0': lat0, 'off0': off0}
    rows, memo = [], {}
    # isK must cover codes created during the replay: extend lazily via a view of the K+ set
    isK_big = isK_arr()
    end_codes, stages, failures, coll_hist = replay(R, keys, parts, ref, C, NF, isK_big, rows, letter_cd, memo)
    if len(keys.keys) > len(isK_big):
        failures.append('isK array too short')
    res['twists_applied'] = sum(1 for r in rows if r.get('admissible'))
    res['all_admissible'] = all(r.get('admissible') for r in rows)
    res['stages'] = stages
    res['failures'] = failures
    res['max_E2off_still_off_pose_changed'] = max((r.get('E2off_still_off_pose_changed', 0) for r in rows), default=0)
    res['max_off_pieces_moved_by_letter'] = {f"{r['prefix']}": r['off_pieces_moved'] for r in rows if r.get('off_pieces_moved')}
    pos = [p for p, n in coll_hist if n > 0]
    res['collateral_positive_prefixes'] = {'count': len(pos), 'min': min(pos) if pos else None, 'max': max(pos) if pos else None,
                                           'contiguous': pos == list(range(pos[0], pos[-1] + 1)) if pos else None}
    res['collateral_max'] = max((n for _, n in coll_hist), default=0)
    res['collateral_max_at'] = [p for p, n in coll_hist if n == res['collateral_max']]
    res['float_margin_min_inside'] = min((r['float_min_inside'] for r in rows if r.get('float_min_inside') is not None), default=None)
    res['float_margin_max_outside'] = max((r['float_max_outside'] for r in rows if r.get('admissible')), default=None)
    Kend = isK_big[end_codes]
    res['end'] = {'digest': stages[-1]['end_digest'] if stages else None, 'off': int((~Kend).sum()),
                  'Bprime_on_K': int(Kend[BP].sum()), 'collateral': int((lat0 & ~Kend).sum()),
                  'E2_lattice_on_K': int((lat0 & Kend).sum()),
                  'E2_lattice_exact_pose': int((lat0 & (end_codes == codes0)).sum()),
                  'changed_vs_E2': int((end_codes != codes0).sum()),
                  'changed_vs_E2_includes_Bprime': bool((end_codes[BP] != codes0[BP]).all()),
                  'E2_lattice_changed_exact_pose': int((lat0 & (end_codes != codes0)).sum())}
    if expected is not None:
        cmp = {}
        for sr in stages:
            e = expected[sr['part']]
            got = (sr['end_digest'][:8], sr['off_end'], sr['Bprime_on_K'], sr['collateral_vs_E2'], sr['E2_lattice_exact_pose'])
            cmp[sr['part']] = {'expected': list(e), 'got': list(got), 'match': tuple(got) == e}
        res['stage_table_vs_report'] = cmp
        tenset = [32877, 48853, 53907, 55426, 64529, 72188, 78289, 105000, 129283, 164054]
        res['collateral_set_after_P_and_F'] = [sr['collateral_ids'] for sr in stages if sr['part'] in ('P', 'Q')]
        res['collateral_set_after_P_and_F_match'] = all(sr['collateral_ids'] == tenset for sr in stages if sr['part'] in ('P', 'Q'))
        fq = [r for r in rows if r.get('part') == 'Q']
        # compare the end state with E3-table.json entry by entry (own parse)
        e3_codes, e3_doc, e3_sha = table_codes(E3_TABLE, keys)
        res['E3_table_sha256'] = e3_sha
        res['E3_table_sha256_ok'] = e3_sha == E3_SHA
        res['E3_table_digest_field_ok'] = e3_doc['digest'] == E3_DIGEST
        res['E3_table_own_digest'] = own_digest(e3_codes, keys)
        res['E3_table_own_digest_ok'] = res['E3_table_own_digest'] == E3_DIGEST
        diff = np.flatnonzero(e3_codes != end_codes)
        res['E3_entrywise_differences'] = int(len(diff))
        res['E3_entrywise_first_diffs'] = diff[:20].tolist()
        res['E3_all_on_K_own'] = bool(isK_big[e3_codes].all())
        # extra: other tables of the chain, own parse
        extra = {}
        for nm, pth in (('E1', SP / 'v26/E2/c0/E1-table.json'),
                        ('X', SP / 'pkg26/Claude-E-closure-X1-20261012/input/X-table.json'),
                        ('seg2_end_E2', SP / 'v26/E2/c0/E2-table.json'),
                        ('pkg27_D87_E2', SP / 'pkg27/Claude-E-closure-E1E2-20261012/D87/E2-table.json')):
            try:
                cc, dd, sh = table_codes(pth, keys)
                extra[nm] = {'sha256': sh, 'digest_field': dd['digest'], 'own_digest_ok': own_digest(cc, keys) == dd['digest'],
                             'changed_vs_E3': int((cc != end_codes).sum()),
                             'off': int((~isK_big[cc]).sum()) if len(keys.keys) <= len(isK_big) else None}
            except Exception as exc:  # noqa: BLE001
                extra[nm] = {'error': repr(exc)}
        res['extra_tables'] = extra
        # F letter's moved set against the parked set before it
        res['F_row'] = {k: fq[0][k] for k in ('prefix', 'moved_journal', 'off_pieces_moved', 'collateral', 'collateral_ids')} if fq else None
    if mode == 'S':
        e = res['end']
        res['S_end_check'] = {'digest': e['digest'] == S_DIGEST, 'off0': e['off'] == 0, 'Bprime3': e['Bprime_on_K'] == 3,
                              'coll0': e['collateral'] == 0, 'exact156104': e['E2_lattice_exact_pose'] == 156104,
                              'twists71': res['twists_applied'] == 71}
    res['seconds'] = round(time.time() - T0, 1)
    (AUD / f'replay-{mode}.json').write_text(json.dumps(res, indent=1) + '\n')
    with open(AUD / f'prefixes-{mode}.jsonl', 'w') as fh:
        for r in rows:
            fh.write(json.dumps(r) + '\n')
    summary = {k: v for k, v in res.items() if not isinstance(v, (list, dict)) or k in ('end', 'failures', 'collateral_positive_prefixes')}
    print(json.dumps(summary))


if __name__ == '__main__':
    main()
