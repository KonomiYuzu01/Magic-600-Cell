"""Skeptic check for finding F3 (inputs of the E1 -> E2 construction). Read-only on all inputs.

Checks, from the raw files only (no simulator):
1. X-table vs E1-table: the pieces whose exact pose differs; the 87-block and B' equal entry by entry.
2. Provenance of Q2: q87-ownerlike.json[0], the X Q search (g-X-b0.jsonl), the lens classification (triples-b0.jsonl),
   the parking row named by w87-c0.json, and the E1 rerun (E1q/q-E1-b0.jsonl).
3. The revised report's rerun claims: counts, ghost flags, X subset of E1 with equal |Bad|, new candidates and grip 190,
   minimum |Bad| at both states.
4. X-safe-list.json vs resources-X.json safe grips; straddling grips at X vs E1 (pre.json).
"""
import json
import sys
from pathlib import Path

V26 = Path(sys.argv[1])
PKG = Path(sys.argv[2])
out = {}


def load_table(p):
    d = json.loads(Path(p).read_text())
    return d['poses'], d['pose_id'], d.get('digest')


Xp, Xid, Xd = load_table(PKG / 'input' / 'X-table.json')
Ep, Eid, Ed = load_table(V26 / 'E2' / 'c0' / 'E1-table.json')
pre = json.loads((V26 / 'E2' / 'c0' / 'pre.json').read_text())
T = pre['target_block']
Bp = [16245, 16249, 32088]
diff = [p for p in range(len(Xid)) if Xp[Xid[p]] != Ep[Eid[p]]]
out['tables'] = {'X_digest': Xd, 'E1_digest': Ed, 'pieces': len(Xid), 'X_to_E1_pose_changed': len(diff),
                 'T_size': len(T), 'T_equal_X_E1': all(Xp[Xid[p]] == Ep[Eid[p]] for p in T),
                 'Bp_equal_X_E1': all(Xp[Xid[p]] == Ep[Eid[p]] for p in Bp),
                 'changed_intersects_T': len(set(diff) & set(T)), 'changed_intersects_Bp': len(set(diff) & set(Bp))}
resX = json.loads((V26 / 'E2' / 'resources-X.json').read_text())
b17 = [b['pieces'] for b in resX['blocks'] if b['size'] == 17]
out['tables']['changed_equals_X_17block'] = bool(b17) and sorted(diff) == sorted(b17[0])

Q2 = [[346, 10], [314, 1], [115, 26], [314, 1], [115, 58], [346, 10], [115, 26], [314, 1], [115, 58], [314, 1]]
w = json.loads((V26 / 'E2' / 'w87-c0.json').read_text())
qown = json.loads((V26 / 'q87-ownerlike.json').read_text())
src_file, src_line = w['source'].split(':')
park_rows = (V26 / src_file).read_text().splitlines()
park_row = json.loads(park_rows[int(src_line)])
out['provenance'] = {'w87_c0_Q_is_Q2': w['Q'] == Q2, 'q87_ownerlike_len': len(qown),
                     'q87_ownerlike_index_of_Q2': [i for i, q in enumerate(qown) if q == Q2],
                     'w87_c0_source': w['source'], 'park_row_P_equals_w87_c0_P': park_row['P'] == w['P'],
                     'park_row': {k: v for k, v in park_row.items() if k != 'P'}}
plog = (V26 / 'park-E1-87-0.log').read_text().splitlines()
out['provenance']['park_E1_87_0_log_first_two'] = [json.loads(plog[0]), json.loads(plog[1])]


def rows(p):
    for line in open(p):
        line = line.strip()
        if line:
            yield json.loads(line)


def key(q):
    return tuple(tuple(x) for x in q)


Xrows = {key(r['Q']): r for r in rows(V26 / 'g-X-b0.jsonl')}
Erows = {key(r['Q']): r for r in rows(V26 / 'E1q' / 'q-E1-b0.jsonl')}
glog = open(V26 / 'g-X-b0.log').readline()
elog = open(V26 / 'E1q' / 'dcomm2.log').readline()
tri = list(rows(V26 / 'triples-b0.jsonl'))
owner_shape = [r for r in tri if r['invB'] and r['invA'] and r['c_in'] and r['pe_in'] and not r['e_in']]
own_keys = {key(r['Q']) for r in owner_shape}
out['provenance'].update({
    'g_X_b0_log_line1': json.loads(glog), 'E1q_dcomm2_log_line1': json.loads(elog),
    'Q2_in_X_search': key(Q2) in Xrows, 'Q2_bad_at_X': Xrows.get(key(Q2), {}).get('bad'),
    'Q2_in_E1_rerun': key(Q2) in Erows, 'Q2_bad_at_E1': Erows.get(key(Q2), {}).get('bad'),
    'triples_rows': len(tri), 'owner_shape_rows': len(owner_shape),
    'owner_shape_distinct_ec': len({(r['e'], r['c']) for r in owner_shape}),
    'q87_ownerlike_all_owner_shape': all(key(q) in own_keys for q in qown),
    'q87_ownerlike_all_in_X_search': all(key(q) in Xrows for q in qown),
    'q87_ownerlike_distinct_ec': len({(q[1][0], q[2][0]) for q in qown}),
})

newE = [k for k in Erows if k not in Xrows]
grips_of = lambda r: {r['e'], r['c'], r['a']}  # noqa: E731
out['rerun'] = {
    'X_count': len(Xrows), 'E1_count': len(Erows),
    'E1_all_T_on_87': all(r['T_on'] == 87 and r['T_dead'] == 0 for r in Erows.values()),
    'E1_all_prot_exact': all(r['prot_dead'] == 0 and r['prot_moved'] == 0 for r in Erows.values()),
    'X_all_T_on_87_prot_exact': all(r['T_on'] == 87 and r['T_dead'] == 0 and r['prot_dead'] == 0
                                    and r['prot_moved'] == 0 for r in Xrows.values()),
    'X_subset_E1': all(k in Erows for k in Xrows),
    'X_subset_same_bad': all(Erows[k]['bad'] == r['bad'] for k, r in Xrows.items() if k in Erows),
    'new_count': len(newE),
    'new_all_use_190': all(190 in grips_of(Erows[k]) for k in newE),
    'new_roles_190': sorted({tuple(sorted(n for n, v in (('e', Erows[k]['e']), ('c', Erows[k]['c']),
                                                          ('a', Erows[k]['a'])) if v == 190)) for k in newE}),
    'X_any_uses_190': any(190 in grips_of(r) for r in Xrows.values()),
    'min_bad_X': min(r['bad'] for r in Xrows.values()), 'min_bad_E1': min(r['bad'] for r in Erows.values()),
    'min_bad_new': min((Erows[k]['bad'] for k in newE), default=None),
}

xsafe = json.loads((V26 / 'E2' / 'X-safe-list.json').read_text())
out['ref_safe'] = {'X_safe_list_len': len(xsafe), 'equals_resources_X_safe': sorted(xsafe) == sorted(resX['safe_all']['grips']),
                   'resources_X_digest': resX['digest'], 'X_table_digest': Xd,
                   'pre_reference_safe_grips': pre['menu'].get('reference_safe_grips'),
                   'pre_safe_here_not_in_reference': pre['menu'].get('safe_here_not_in_reference'),
                   'X_straddling_grips': resX['straddling_grips'],
                   'E1_straddled_blocked': sorted(int(k) for k, v in pre['menu']['blocked'].items() if v['straddled'])}
print(json.dumps(out, indent=1))
