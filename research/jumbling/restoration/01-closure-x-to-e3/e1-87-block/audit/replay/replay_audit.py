"""Independent replay audit of E1 -> E2 (label 'replay'). Own loop over R.apply; no replay_check/prefixrec/closure2.

Usage (each chunk is one process; chunks hand over through exported, digest-checked pose tables in OUT):
    python3 -I -B replay_audit.py A        # E1 snapshot, word checks, V (prefixes 0..590)
    python3 -I -B replay_audit.py B        # P, Q, Pinv (prefixes 590..620)
    python3 -I -B replay_audit.py C        # Vinv (prefixes 620..1210), E2 comparison, holes
    python3 -I -B replay_audit.py summary  # stage table and comparison with the report's claims

Per prefix (= number of twists applied) the row records: digest, total off, E1-lattice pieces off (collateral),
E1-off pieces on K (restored), E1-off pieces still off whose exact pose differs from E1 (to_tuple comparison),
B' pieces whose exact pose differs from E1, E1-lattice pieces not at their E1 K pose, and for the twist that led to
the prefix: the pieces whose exact pose changed (canonical pose keys), split by T, B' and E1-off/E1-lattice, and the
simulator's own journal 'moved' count.
"""
import hashlib
import json
import sys
import time
from pathlib import Path

sys.dont_write_bytecode = True
S = Path('<work>')
TH = S / 'wt-e3/research/jumbling/theory'
C0 = S / 'v26/E2/c0'
OUT = S / 'v26/audit/replay'
PKG = S / 'pkg26/Claude-E-closure-X1-20261012'
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))

import numpy as np  # noqa: E402

import blocks as BL  # noqa: E402
from sim.kplus import matrix_from_json, matrix_json, to_tuple  # noqa: E402

O = BL.O
restore = BL.restore
CTX = BL.CL.ctx
E1_DIGEST = '903997457de665c2846e00ba62b71c3d3ccb7df79c78391f5a1b9f1757afc4e8'
E2_DIGEST = '19e404d438fd929b0c6480c35374aa5147b361831806774ff65ca0f4739bc5ff'
BP = [16245, 16249, 32088]
PARTS = ['V', 'P', 'Q', 'Pinv', 'Vinv']
REPORT_HOLES_E2 = [24905, 89251, 89252]
T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


def sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def load_part(name):
    d = json.loads((C0 / 'parts' / f'{name}.json').read_text())
    return [(int(c), int(i)) for c, i in d['word']], d['input_digest'], d['output_digest']


def offsets():
    off, n = {}, 0
    for name in PARTS:
        off[name] = n
        n += len(load_part(name)[0])
    return off, n


class E1:
    """Snapshot of E1 from E1-table.json, loaded in its own restorer."""

    def __init__(self):
        R1, _ = O.start(str(C0 / 'E1-table.json'))
        s1 = R1.st
        self.digest = s1.digest()
        self.K = s1._kpose()[s1.pose_id].copy()
        self.off_mask = self.K < 0
        self.lat_mask = ~self.off_mask
        self.off_idx = np.flatnonzero(self.off_mask)
        self.poses = {int(p): to_tuple(s1.pose(int(p))) for p in self.off_idx.tolist()}
        self.T = np.setdiff1d(self.off_idx, np.array(BP, np.int64))
        self.T_mask = np.zeros(len(self.K), bool)
        self.T_mask[self.T] = True
        self.st = s1


KEYS = {}


def key_ids(st):
    """Canonical per-piece pose ids: equal ids <=> equal exact matrices (K+ poses are keyed by K index, others by
    the exact matrix in lowest terms; the simulator interns every K+ matrix by its K index)."""
    return np.array([KEYS.setdefault(p.key, len(KEYS)) for p in st._poses], np.int64)[st.pose_id]


def prefix_row(st, prefix, part, idx, letter, e1, prev_ids, journal_moved):
    K = st._kpose()[st.pose_id]
    off = K < 0
    coll_mask = off & e1.lat_mask
    still = e1.off_idx[off[e1.off_idx]]
    changed = [int(p) for p in still.tolist() if to_tuple(st.pose(int(p))) != e1.poses[int(p)]]
    bp_changed = [p for p in BP if (not off[p]) or to_tuple(st.pose(p)) != e1.poses[p]]
    ids = key_ids(st)
    row = {'prefix': prefix, 'part': part, 'index_in_part': idx, 'letter': letter, 'digest': st.digest(),
           'off': int(off.sum()), 'collateral': int(coll_mask.sum()),
           'restored': int((~off & e1.off_mask).sum()), 'still_off_changed': len(changed),
           'still_off_changed_in_T': int(e1.T_mask[changed].sum()) if changed else 0,
           'bp_changed': len(bp_changed),
           'e1_lattice_displaced': int(((K != e1.K) & e1.lat_mask).sum())}
    if prev_ids is not None:
        mv = ids != prev_ids
        row.update({'moved': int(mv.sum()), 'journal_moved': journal_moved, 'moved_T': int(mv[e1.T].sum()),
                    'moved_bp': int(mv[BP].sum()), 'moved_e1_lattice': int((mv & e1.lat_mask).sum()),
                    'moved_offnow_e1_lattice': int((mv & coll_mask).sum())})
    return row, ids, off, coll_mask


def export_table(st, path):
    poses = [matrix_json(st._pose_matrix(i)) for i in range(len(st._poses))]
    doc = {'poses': poses, 'pose_id': st.pose_id.astype(int).tolist(), 'digest': st.digest()}
    Path(path).write_text(json.dumps(doc) + '\n')
    return doc['digest']


def word_checks(M):
    """Words: parts vs report letter lists, inverse relations by exact matrices, word-full = concatenation."""
    I4 = restore.I4
    kp = CTX.kplus
    w = {n: load_part(n) for n in PARTS}
    el = {}

    def E(c, i):
        if (c, i) not in el:
            el[(c, i)] = restore.element(M, c, i)
        return el[(c, i)]

    from exact import matmul

    def inv_ok(a, b):
        """Is word b the exact inverse of word a, letter by letter in reverse order?"""
        if len(a) != len(b):
            return False, -1
        for k, ((c1, i1), (c2, i2)) in enumerate(zip(a, reversed(b))):
            if c1 != c2 or to_tuple(matmul(E(c2, i2), E(c1, i1))) != I4:
                return False, k
        return True, None

    rep_P = [(115, 26), (190, 3), (115, 58), (190, 9), (317, 48), (190, 3), (115, 26), (190, 9), (115, 58), (317, 17)]
    rep_Q = [(346, 10), (314, 1), (115, 26), (314, 1), (115, 58), (346, 10), (115, 26), (314, 1), (115, 58), (314, 1)]
    rep_Pinv = [(317, 48), (115, 26), (190, 3), (115, 58), (190, 9), (317, 17), (190, 3), (115, 26), (190, 9),
                (115, 58)]
    full = json.loads((C0 / 'word-full.json').read_text())
    concat = [x for n in PARTS for x in w[n][0]]
    letters = sorted({x for n in PARTS for x in w[n][0]})
    identity_letters = [x for x in letters if E(*x) == I4]
    named = {'X': (115, 26), 'Xinv': (115, 58), 'Y': (190, 3), 'Yinv': (190, 9), 'Z': (317, 48), 'Zinv': (317, 17),
             'A': (346, 10), 'B': (314, 1)}
    kmember = {k: int(kp.index_of_matrix(E(*v)) >= 0) for k, v in named.items()}
    invol = {k: to_tuple(matmul(E(*named[k]), E(*named[k]))) == I4 for k in ('A', 'B', 'X')}
    pairs = {k: to_tuple(matmul(E(*named[k + 'inv']), E(*named[k]))) == I4 for k in ('X', 'Y', 'Z')}
    res = {'lengths': {n: len(w[n][0]) for n in PARTS},
           'P_equals_report': w['P'][0] == rep_P, 'Q_equals_report': w['Q'][0] == rep_Q,
           'Pinv_equals_report': w['Pinv'][0] == rep_Pinv,
           'Pinv_is_inverse_of_P': inv_ok(w['P'][0], w['Pinv'][0]),
           'Vinv_is_inverse_of_V': inv_ok(w['V'][0], w['Vinv'][0]),
           'word_full_is_concatenation': [tuple(x) for x in full['word']] == concat,
           'word_full_digests': [full['input_digest'] == E1_DIGEST, full['output_digest'] == E2_DIGEST],
           'distinct_letters': len(letters), 'identity_letters': identity_letters,
           'K_membership_of_named_letters': kmember, 'involutions': invol, 'inverse_pairs': pairs,
           'digest_chain_of_parts': [w['V'][1] == E1_DIGEST] + [w[PARTS[k]][2] == w[PARTS[k + 1]][1]
                                                                for k in range(4)] + [w['Vinv'][2] == E2_DIGEST]}
    return res


def run_chunk(name):
    plan = {'A': (['V'], C0 / 'E1-table.json', E1_DIGEST),
            'B': (['P', 'Q', 'Pinv'], OUT / 'state-after-V.json', None),
            'C': (['Vinv'], OUT / 'state-after-Pinv.json', None)}
    parts, table, want = plan[name]
    if want is None:
        prev = json.loads((OUT / f'chunk-{chr(ord(name) - 1)}.json').read_text())
        want = prev['end_digest']
    e1 = E1()
    log(e1_digest=e1.digest, e1_off=len(e1.off_idx), T=len(e1.T))
    R, M = O.start(str(table))
    st = R.st
    start_digest = st.digest()
    res = {'chunk': name, 'table': str(table), 'table_sha256': sha256(table), 'start_digest': start_digest,
           'start_digest_ok': start_digest == want, 'errors': []}
    if name == 'A':
        res['e1'] = {'digest': e1.digest, 'digest_ok': e1.digest == E1_DIGEST, 'off': len(e1.off_idx),
                     'off_pieces': e1.off_idx.tolist(), 'T_size': len(e1.T), 'T': e1.T.tolist(),
                     'bp_off': all(bool(e1.off_mask[p]) for p in BP),
                     'bp_poses': {p: matrix_json(e1.poses[p]) for p in BP},
                     'bp_share_one_pose': len({e1.poses[p] for p in BP}) == 1,
                     'T_distinct_poses': len({e1.poses[int(p)] for p in e1.T.tolist()}),
                     'lattice': int(e1.lat_mask.sum()),
                     'sha256': {'c0/E1-table.json': sha256(C0 / 'E1-table.json'),
                                'v26/E1-table.json': sha256(S / 'v26/E1-table.json'),
                                'pkg D17/end-table.json': sha256(PKG / 'D17/end-table.json')
                                if (PKG / 'D17/end-table.json').exists() else None}}
        pre = json.loads((C0 / 'pre.json').read_text())
        res['e1']['pre_json_target_block_equal'] = sorted(pre.get('target_block', [])) == e1.T.tolist()
        res['e1']['pre_json_protected_equal'] = sorted(pre.get('protected_off', [])) == BP
        res['words'] = word_checks(M)
        log(words=res['words'])
    off0, _ = offsets()
    rows = []
    prefix = off0[parts[0]]
    row, ids, _, _ = prefix_row(st, prefix, None, None, None, e1, None, None)
    rows.append(row)
    stage_sets = {}
    for pn in parts:
        word, din, dout = load_part(pn)
        if st.digest() != din:
            res['errors'].append(f'{pn}: input digest differs from the replayed state')
        for k, (c, i) in enumerate(word):
            try:
                R.apply(c, i)
            except RuntimeError as exc:
                res['errors'].append(f'inadmissible twist at prefix {prefix + 1} ({pn}[{k}] = {(c, i)}): {exc}')
                log(error=res['errors'][-1])
                break
            prefix += 1
            row, ids2, off, coll_mask = prefix_row(st, prefix, pn, k, [c, i], e1, ids,
                                                   int(st.journal[-1]['moved']))
            ids = ids2
            rows.append(row)
            if prefix % 50 == 0:
                log(prefix=prefix, off=row['off'], coll=row['collateral'])
        else:
            d = st.digest()
            if d != dout:
                res['errors'].append(f'{pn}: output digest {d} differs from part file {dout}')
            K = st._kpose()[st.pose_id]
            coll = np.flatnonzero((K < 0) & e1.lat_mask)
            stage_sets[pn] = {'collateral_pieces': coll.tolist(),
                              'collateral_poses': {int(p): matrix_json(st.pose(int(p))) for p in coll.tolist()}}
            log(part=pn, digest=d, ok=d == dout, off=int((K < 0).sum()), coll=len(coll))
            continue
        break
    res['rows'] = rows
    res['end_digest'] = st.digest()
    if name in ('A', 'B') and not res['errors']:
        path = OUT / ('state-after-V.json' if name == 'A' else 'state-after-Pinv.json')
        res['export'] = {'path': str(path), 'digest': export_table(st, path)}
    if name == 'B' and 'P' in stage_sets and 'Q' in stage_sets:
        a, b = stage_sets['P'], stage_sets['Q']
        res['parked'] = {'after_P': len(a['collateral_pieces']), 'after_Q': len(b['collateral_pieces']),
                         'same_set': a['collateral_pieces'] == b['collateral_pieces'],
                         'same_exact_poses': a['collateral_poses'] == b['collateral_poses'],
                         'pieces': a['collateral_pieces']}
    if name == 'C' and not res['errors']:
        res['final'] = final_checks(st, e1)
    (OUT / f'chunk-{name}.json').write_text(json.dumps(res) + '\n')
    log(chunk=name, rows=len(rows), errors=res['errors'], end=res['end_digest'])


def holes_formula(st):
    reg, kp = CTX.regions, CTX.kplus
    K = st._kpose()[st.pose_id]
    lat = np.flatnonzero(K >= 0)
    ch = reg.piece_of[reg.orbit_of[lat], kp.compose_vec(K[lat], reg.transport[lat])]
    occ = np.full(len(K), -1, np.int64)
    occ[ch] = lat
    return np.flatnonzero(occ < 0), int(len(np.unique(ch)) == len(lat))


def holes_signature(st):
    """Second path: chamber of a lattice piece p with K pose k = the chamber whose signature is perms[k][sig(p)]."""
    data, kp = CTX.data, CTX.kplus
    mo, mv = data.mask_offsets, data.mask_values
    NP = len(st.pose_id)
    sig_index = {tuple(mv[mo[p]:mo[p + 1]].tolist()): p for p in range(NP)}
    perms = kp.perms.astype(np.int64)
    K = st._kpose()[st.pose_id]
    lat = np.flatnonzero(K >= 0)
    hit = np.zeros(NP, np.int64)
    missing = 0
    for p, k in zip(lat.tolist(), K[lat].tolist()):
        img = tuple(sorted(perms[k][mv[mo[p]:mo[p + 1]]].tolist()))
        c = sig_index.get(img)
        if c is None:
            missing += 1
            continue
        hit[c] += 1
    return np.flatnonzero(hit == 0), int((hit > 1).sum()), missing


def final_checks(st, e1):
    out = {}
    K = st._kpose()[st.pose_id]
    off = K < 0
    out['digest'] = st.digest()
    out['digest_ok'] = out['digest'] == E2_DIGEST
    out['off'] = int(off.sum())
    out['off_pieces'] = np.flatnonzero(off).tolist()
    out['T_restored'] = int((~off[e1.T]).sum())
    out['collateral'] = int((off & e1.lat_mask).sum())
    out['e1_lattice_on_K'] = int((~off & e1.lat_mask).sum())
    out['e1_lattice_same_K_pose'] = int(((K == e1.K) & e1.lat_mask).sum())
    out['e1_lattice'] = int(e1.lat_mask.sum())
    out['bp_exact_same'] = {p: bool(off[p] and to_tuple(st.pose(p)) == e1.poses[p]) for p in BP}
    # exact pose changes E1 -> E2 over all pieces (E1 pose: K index on K, exact matrix off K)
    changed = []
    for p in range(len(K)):
        if e1.lat_mask[p]:
            if K[p] != e1.K[p]:
                changed.append(p)
        elif off[p]:
            if to_tuple(st.pose(p)) != e1.poses[p]:
                changed.append(p)
        else:
            changed.append(p)
    out['exact_changed_E1_E2'] = len(changed)
    out['exact_changed_is_T'] = changed == e1.T.tolist()
    # entry by entry against E2-table.json
    doc2 = json.loads((C0 / 'E2-table.json').read_text())
    out['E2_table_digest_field_ok'] = doc2.get('digest') == E2_DIGEST
    out['E2_table_pieces'] = len(doc2['pose_id'])
    kmap = {}
    theirs = np.array([kmap.setdefault(json.dumps(matrix_json(matrix_from_json(m))), len(kmap))
                       for m in doc2['poses']], np.int64)[np.asarray(doc2['pose_id'], np.int64)]
    mine = np.array([kmap.setdefault(json.dumps(matrix_json(st._pose_matrix(i))), len(kmap))
                     for i in range(len(st._poses))], np.int64)[st.pose_id]
    mism = np.flatnonzero(theirs != mine)
    out['E2_table_entry_mismatches'] = int(len(mism))
    out['E2_table_first_mismatches'] = mism[:10].tolist()
    # holes at E2 and at E1, two paths each
    h1, inj1 = holes_formula(st)
    h2, dup2, miss2 = holes_signature(st)
    out['holes_E2_formula'] = h1.tolist()
    out['holes_E2_formula_injective'] = inj1
    out['holes_E2_signature'] = h2.tolist()
    out['holes_E2_signature_dups_missing'] = [dup2, miss2]
    out['holes_E2_equal_report'] = h1.tolist() == REPORT_HOLES_E2 and h2.tolist() == REPORT_HOLES_E2
    g1, ginj = holes_formula(e1.st)
    g2, gdup, gmiss = holes_signature(e1.st)
    pre = json.loads((C0 / 'pre.json').read_text())
    out['holes_E1'] = {'count_formula': int(len(g1)), 'count_signature': int(len(g2)),
                       'paths_equal': g1.tolist() == g2.tolist(), 'injective': ginj, 'dups_missing': [gdup, gmiss],
                       'equal_pre_json': g1.tolist() == sorted(pre.get('holes', []))}
    log(final={k: v for k, v in out.items() if not isinstance(v, list)})
    return out


def summary():
    ch = {n: json.loads((OUT / f'chunk-{n}.json').read_text()) for n in 'ABC'}
    rows = {}
    dup_mismatch = []
    for n in 'ABC':
        for r in ch[n]['rows']:
            p = r['prefix']
            if p in rows:
                a, b = rows[p], r
                keys = ('digest', 'off', 'collateral', 'restored', 'still_off_changed', 'bp_changed',
                        'e1_lattice_displaced')
                if any(a[k] != b[k] for k in keys):
                    dup_mismatch.append(p)
                continue
            rows[p] = r
    prefixes = sorted(rows)
    off0, total = offsets()
    out = {'errors': {n: ch[n]['errors'] for n in 'ABC'}, 'prefix_count': len(prefixes),
           'prefixes_contiguous': prefixes == list(range(total + 1)), 'handover_mismatch': dup_mismatch,
           'chunk_start_digest_ok': {n: ch[n]['start_digest_ok'] for n in 'ABC'},
           'exports': {n: ch[n].get('export') for n in 'AB'}}
    # stage table
    bounds = {}
    for name in PARTS:
        a = off0[name]
        b = a + len(load_part(name)[0])
        bounds[name] = (a, b)
    stages = {}
    for name, (a, b) in bounds.items():
        rs = [rows[p] for p in range(a, b + 1)]
        e = rows[b]
        stages[name] = {'end_prefix': b, 'digest': e['digest'], 'off': e['off'], 'restored': e['restored'],
                        'collateral': e['collateral'], 'still_off_changed': e['still_off_changed'],
                        'off_range_incl_start': [min(r['off'] for r in rs), max(r['off'] for r in rs)],
                        'off_range_excl_start': [min(r['off'] for r in rs[1:]), max(r['off'] for r in rs[1:])],
                        'max_collateral': max(r['collateral'] for r in rs),
                        'letters_moving_T': sum(1 for r in rs[1:] if r.get('moved_T', 0) > 0),
                        'letters_moving_parked': sum(1 for r in rs[1:] if r.get('moved_offnow_e1_lattice', 0) > 0),
                        'moved_vs_journal_mismatch': [r['prefix'] for r in rs[1:]
                                                      if r.get('moved') != r.get('journal_moved')]}
    out['stages'] = stages
    rep = {'V': ('d4ff7ac5', 'ae9bcab3', 90, (90, 90), 0, 0, 0),
           'P': ('d25915b3', 'b396385932', 682, (90, 5802), 0, 592, 0),
           'Q': ('bd543100', '0e1d56c9fd', 595, (595, 3495), 87, 592, 0),
           'Pinv': ('51d4c560', '238d704d5d9f8', 3, (3, 5802), 87, 0, 0),
           'Vinv': ('19e404d4', '4739bc5ff', 3, (3, 3), 87, 0, 0)}
    cmp = {}
    for name, (h, t, off, rng, rest, coll, chg) in rep.items():
        s = stages[name]
        cmp[name] = {'digest': s['digest'].startswith(h) and s['digest'].endswith(t), 'off': s['off'] == off,
                     'range': tuple(s['off_range_incl_start']) == rng, 'restored': s['restored'] == rest,
                     'collateral': s['collateral'] == coll, 'still_off_changed': s['still_off_changed'] == chg}
    out['stage_vs_report'] = cmp
    coll_pref = [p for p in prefixes if rows[p]['collateral'] > 0]
    chg_pref = [p for p in prefixes if rows[p]['still_off_changed'] > 0]
    out['collateral_prefixes'] = [min(coll_pref), max(coll_pref)] if coll_pref else None
    out['collateral_prefixes_contiguous'] = coll_pref == list(range(min(coll_pref), max(coll_pref) + 1)) \
        if coll_pref else None
    out['collateral_max'] = max(rows[p]['collateral'] for p in prefixes)
    out['collateral_max_at'] = [p for p in prefixes if rows[p]['collateral'] == out['collateral_max']]
    out['still_off_changed_prefixes'] = chg_pref
    out['still_off_changed_count'] = len(chg_pref)
    out['still_off_changed_max'] = max(rows[p]['still_off_changed'] for p in prefixes)
    out['still_off_changed_all_in_T'] = all(rows[p]['still_off_changed'] == rows[p]['still_off_changed_in_T']
                                            for p in prefixes)
    out['bp_changed_any'] = [p for p in prefixes if rows[p]['bp_changed']]
    out['bp_moved_any'] = [p for p in prefixes if rows[p].get('moved_bp', 0)]
    out['e1_lattice_displaced_max'] = max(rows[p]['e1_lattice_displaced'] for p in prefixes)
    out['parked'] = ch['B'].get('parked', {})
    out['parked'].pop('pieces', None)
    out['final'] = ch['C'].get('final')
    if out['final']:
        out['final'] = {k: v for k, v in out['final'].items() if k not in ('off_pieces',)}
    out['words'] = ch['A'].get('words')
    out['e1'] = {k: v for k, v in ch['A']['e1'].items() if k not in ('off_pieces', 'T', 'bp_poses')}
    # consistency with prefixes.jsonl (claimed records; not used as evidence)
    theirs = [json.loads(x) for x in (C0 / 'prefixes.jsonl').read_text().splitlines() if x.strip()]
    diffs = []
    for r in theirs:
        p = r['prefix']
        m = rows.get(p)
        if m is None:
            diffs.append((p, 'missing'))
            continue
        for k_t, k_m in (('digest', 'digest'), ('off', 'off'), ('restored', 'restored'),
                         ('collateral', 'collateral'), ('pose_changed', 'still_off_changed')):
            if r.get(k_t) != m[k_m]:
                diffs.append((p, k_t, r.get(k_t), m[k_m]))
        if r.get('letter') is not None and r['letter'] != m['letter']:
            diffs.append((p, 'letter', r['letter'], m['letter']))
    out['prefixes_jsonl'] = {'rows': len(theirs), 'diffs': len(diffs), 'first_diffs': diffs[:20]}
    (OUT / 'summary.json').write_text(json.dumps(out, indent=1) + '\n')
    print(json.dumps(out, indent=1))


if __name__ == '__main__':
    if sys.argv[1] == 'summary':
        summary()
    else:
        run_chunk(sys.argv[1])
