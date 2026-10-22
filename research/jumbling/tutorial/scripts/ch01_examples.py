"""Chapter 1 of the jumbling tutorial: the exact facts and examples it quotes (headless research).

Read-only. It imports the simulator and the theory modules, computes, and writes one JSON record.
Nothing here searches or plays a rule of the runner except section E, which takes the first bulk step
at the start of seed 7001 with the runner's own bulk order.

Run from the repository root (about three minutes on a cloud machine):
    python3 -I -B research/jumbling/tutorial/scripts/ch01_examples.py --out <new file> --cache <new dir>

Sections of the record:
  A menu      the 60 items of I_a: identity, orders, retained and jumble items, inverse table at all 600
              caps, the pole fixed at all 600 caps, the ordered list's digest and the frames' digest
  B geometry  chambers per cap, cut cells, the exhaustive check of every signature-hash lookup, NEAR
  C seed7001  the start of seed 7001 (depth 10): digest, off, heights, domains, N, survey, cap 72
  D gamma     N and every N_c at that start again, with a different representative for each domain
  E bulk0     the first bulk step at that start, rules in the runner's bulk order
  F seed7101  the start of seed 7101 (depth 40), and the state before the 17th step of its census run
"""
import argparse
import hashlib
import json
import sys
import time
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--out', required=True, type=Path)
parser.add_argument('--cache', required=True, type=Path)
parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[4])
args = parser.parse_args()
ROOT = args.root.resolve()
J = ROOT / 'research/jumbling'
sys.path[:0] = [str(J / 'theory'), str(J), str(J / 'fixtures'),
                str(J / 'restoration/13-tutorial-runner/scripts')]
import numpy as np  # noqa: E402
import restore  # noqa: E402
import rim  # noqa: E402
import seams  # noqa: E402
import trees  # noqa: E402
import runner  # noqa: E402  (only its constants are read)
from exact import matmul, matvec, transpose  # noqa: E402
from sim.kplus import matrix_json, to_tuple  # noqa: E402

args.cache.mkdir(parents=True, exist_ok=True)
seams.CACHE = args.cache / 'home_centroids.npy'   # never the default cache under the repository
T0 = time.time()
ctx = restore.ctx
kp, data = ctx.kplus, ctx.data
M = restore.menu('I_a')
I4 = restore.I4
out = {'format': 'tutorial-ch01-examples/1'}


def sha(doc):
    return hashlib.sha256(json.dumps(doc, separators=(',', ':')).encode()).hexdigest()


def order(m):
    p = m
    for k in range(1, 61):
        if to_tuple(p) == I4:
            return k
        p = matmul(p, m)
    return None


def heights(st):
    pid, counts = np.unique(st.pose_id, return_counts=True)
    hist = {}
    for p, n in zip(pid.tolist(), counts.tolist()):
        if st._poses[p].kidx < 0:
            h = trees.dist(trees.factors(st._pose_matrix(p))[0])
            hist[h] = hist.get(h, 0) + n
    return {str(h): hist[h] for h in sorted(hist)}


# ---------------------------------------------------------------- A: the menu I_a
items = [m for _, m in M.items]
lookup = {to_tuple(m): i for i, m in enumerate(items)}
inverse = [lookup[to_tuple(transpose(m))] for m in items]
orders = {}
for i, m in enumerate(items):
    orders.setdefault(order(m), []).append(i)
jumble = restore.jumble_items(M)
same_inverse, fixes_pole = True, True
for c in range(600):
    els = [restore.element(M, c, i) for i in range(len(items))]
    at = {to_tuple(e): i for i, e in enumerate(els)}
    same_inverse &= [at[to_tuple(transpose(e))] for e in els] == inverse
    fixes_pole &= all(matvec(e, data.N[c]) == list(data.N[c]) for e in els)
out['A_menu'] = {
    'items': len(items),
    'item0_is_identity': to_tuple(items[0]) == I4,
    'orders': {str(k): v for k, v in sorted(orders.items())},
    'retained': [i for i in range(len(items)) if i not in jumble],
    'jumble': jumble,
    'inverse': inverse,
    'self_inverse': [i for i in range(len(items)) if inverse[i] == i],
    'inverse_table_same_at_all_600_caps': bool(same_inverse),
    'every_item_fixes_its_pole_at_all_600_caps': bool(fixes_pole),
    'menu_identity_sorted': M.identity,
    'ordered_items_sha256': sha([matrix_json(m) for m in items]),
    'frame_kplus_index': [int(x) for x in kp.frame_idx],
    'frames_sha256': sha([matrix_json(M.frame(c)) for c in range(600)]),
    'base_items': [matrix_json(m) for m in items],
}
print('A', round(time.time() - T0), flush=True)

# ---------------------------------------------------------------- B: geometry and the signature hash
bits = np.unpackbits(data.sigbits, axis=1)[:, :600].astype(bool)
per_cap = bits.sum(axis=0)
lo, up = rim.cell_pairs()
bad_hits, hits_plus, hits_minus, core_hits = 0, 0, 0, 0
for e in range(600):
    member = bits[:, e]
    for add, q in ((True, np.flatnonzero(~member)), (False, np.flatnonzero(member))):
        r = rim.partner(q, np.full(len(q), e), add=add)
        expected = bits[q].copy()
        expected[:, e] = add
        core = r == rim.CORE
        found = (r >= 0) & ~core
        bad_hits += int((bits[r[found]] != expected[found]).any(axis=1).sum())
        bad_hits += int(expected[core].any(axis=1).sum())       # CORE only for the signature {e}
        if add:
            hits_plus += int(found.sum())
        else:
            hits_minus += int(found.sum())
            core_hits += int(core.sum())
near = [list(map(int, x)) for x in restore.NEAR]
out['B_geometry'] = {
    'chambers': int(rim.NP),
    'chambers_per_cap_min_max': [int(per_cap.min()), int(per_cap.max())],
    'cut_cells': int(len(lo)), 'cut_cells_against_core': int((lo == rim.CORE).sum()),
    'hash_lookups_checked': {'add': hits_plus, 'remove': hits_minus, 'core': core_hits,
                             'false_hits': bad_hits},
    'near_sizes': sorted({len(x) for x in near}),
    'near_contains_self_and_sorted': all(c in x and x == sorted(x) for c, x in enumerate(near)),
}
print('B', round(time.time() - T0), flush=True)

# ---------------------------------------------------------------- C: the start of seed 7001
st, _ = restore.scramble(M, 10, 7001)
D = rim.Domains(st)
pose_ids = np.unique(st.pose_id).tolist()
first_pose, poses_by_domain = {}, {}
for pid in pose_ids:
    d = int(D.dom[np.flatnonzero(st.pose_id == pid)[0]])
    first_pose.setdefault(d, pid)
    poses_by_domain.setdefault(str(d), []).append([pid, int((st.pose_id == pid).sum())])
R = seams.SeamRestorer(st, M)
sv = R.survey()
now72, after72 = sv[72]
closing72 = [i for i, v in after72.items() if v == 0]
coset = sorted(lookup[to_tuple(matmul(items[r], items[18]))] for r in out['A_menu']['retained'])
out['C_seed7001'] = {
    'digest': st.digest(), 'off': int(st.off_lattice_count()), 'heights': heights(st),
    'domains': len(D.reps), 'domain_sizes_in_domain_order': [len(m) for m in D.members],
    'representative_pose_id': [first_pose[d] for d in range(len(D.reps))],
    'poses_by_domain': poses_by_domain,      # [pose id, pieces] in pose-table order
    'N': int(R.misalignment()),
    'survey_in_cap_order': {str(c): v[0] for c, v in sv.items()},
    'grip_N': int(R.grip_misalignment()),
    'cap72_closing_items': closing72,
    'cap72_right_coset_of_item18': coset,
    'cap72_balance': list(R.balance(72)),
    'balance_in_cap_order': {str(c): list(R.balance(c)) for c in sv},
    'face_caps': R.face_caps(), 'cover_tops': R.cover_tops(),
}
print('C', round(time.time() - T0), flush=True)

# ---------------------------------------------------------------- D: another representative per domain
alt = rim.Domains(st)
for d in range(1, len(alt.reps)):
    pids = [p for p in pose_ids if int(D.dom[np.flatnonzero(st.pose_id == p)[0]]) == d]
    rep = st._pose_matrix(pids[-1])                          # the last pose of the domain, not the first
    alt.reps[d] = rep
    for p in pids:
        k = kp.index_of_matrix(to_tuple(matmul(transpose(rep), st._pose_matrix(p))))
        assert k >= 0
        mem = np.flatnonzero(st.pose_id == p)
        alt.cham[mem] = rim.piece_of[rim.orbit_of[mem], kp.compose_vec(np.full(len(mem), k), rim.transport[mem])]
alt._pole, alt._coset = {}, {}
same_nc, admissible = True, 0
for c in range(600):
    ins = R.inside(c)
    if ins is None:
        continue
    admissible += 1
    mask = np.zeros(rim.NP, bool)
    mask[ins] = True
    rs = D.rim_sets(c, mask)
    rs2 = alt.rim_sets(c, mask)
    same_nc &= D.misalignment(c, mask, rs=rs) == alt.misalignment(c, mask, rs=rs2)
    if c == 72:
        same_after72 = all(D.misalignment(c, mask, (c, i), R.g(c, i), rs) ==
                           alt.misalignment(c, mask, (c, i), R.g(c, i), rs2) for i in range(len(items)))
cover = R.cover_graph()
R._dom = alt
cover_alt = R.cover_graph()
out['D_representatives'] = {
    'representative_pose_id': [0] + [int(max(p for p in pose_ids
                                             if int(D.dom[np.flatnonzero(st.pose_id == p)[0]]) == d))
                                     for d in range(1, len(D.reps))],
    'N_same': int(alt.total_misalignment()) == out['C_seed7001']['N'],
    'admissible_caps': admissible, 'every_N_c_same': bool(same_nc), 'cap72_scores_same': bool(same_after72),
    'cover_graph_same': {k: sorted(v) for k, v in cover.items()} == {k: sorted(v) for k, v in cover_alt.items()},
}
R._dom = None
print('D', round(time.time() - T0), flush=True)

# ---------------------------------------------------------------- E: the first bulk step at that start
order_ = tuple(runner.BULK_ORDER)
B = seams.SeamRestorer(st, M, order=order_)
methods = {'seam': B.seam, 'cover': B.cover, 'close': B.level1, 'lock': B.level2, 'improve': B.improve,
           'shift': B.shift, 'conj': B.conj}
n_before = int(B.misalignment())
for rule in order_:
    if methods[rule]() is not None:
        break
out['E_bulk0'] = {'bulk_order': list(order_), 'rule': rule, 'word': [list(x) for x in B.moves],
                  'N_before': n_before, 'N_after': int(B.misalignment()),
                  'off_after': int(st.off_lattice_count()), 'digest_after': st.digest()}
print('E', round(time.time() - T0), flush=True)

# ---------------------------------------------------------------- F: seed 7101
record = json.loads((J / 'restoration/11-scramble-census/runs/round1/ia-k40-s7101.json').read_text())
st, hist = restore.scramble(M, 40, 7101)
S = seams.SeamRestorer(st, M)
start = {'digest': st.digest(), 'off': int(st.off_lattice_count()), 'heights': heights(st),
         'N': int(S.misalignment()), 'history_matches_record': [list(x) for x in hist] == record['history']}
for c, i in record['moves'][:16]:
    S.apply(c, i)
sv = S.survey()
cov = S.cover_graph()
out['F_seed7101'] = {
    'start': start,
    'before_step_16': {'digest': st.digest(), 'off': int(st.off_lattice_count()), 'N': int(S.misalignment()),
                       'misaligned_caps': len(sv), 'face_caps': S.face_caps(),
                       'cover_graph': {str(c): sorted(v) for c, v in cov.items() if v},
                       'cover_tops': S.cover_tops(), 'record_move_16': record['moves'][16]},
}
out['seconds'] = round(time.time() - T0, 1)
args.out.write_text(json.dumps(out, indent=1) + '\n')
print('done', out['seconds'], flush=True)
