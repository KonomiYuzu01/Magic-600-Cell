"""Acceptance run for the J1 general simulator.

Run from the repository root (four worker processes):

    python research/jumbling/sim/accept.py

Reads assets/model.npz and assets/primitives.npz read-only and writes
research/jumbling/sim/acceptance.json. The criteria are the "later simulator" acceptance of
research/jumbling/state-contract.md section 6, as listed for J1 in
docs/progress/1.0/jumbling-plan.md:
  1. labelled sticker and frame agreement with rotperms, move_src and move_dst for all 1,200
     retained generators;
  2. independently certified blocked and unblocked cases, at least the witness E2 and E3 sets;
  3. conservative handling of uncertain contacts;
  4. rejection without state change;
  5. grouped and filtered classification against the individual exact reference;
  6. exact replay, inverse round trips and undo of seeded mixed sequences;
  7. same-cap checkpoint excursions and global K+ checkpoint obstructions;
  8. inverse/conjugacy closure and frame-independent menu transport;
  9. the exact infinite-order negative control, plus A1 input-map and A2 certificate checks.
Every classification is exact in Q(sqrt5). Floating point appears only in the float
cross-check of section 4c, in the optional A2 filter (whose float decisions are compared with
exact signs) and in displayed angles and h values.
"""
import json
import hashlib
import itertools
import math
import os
import platform
import random
import sys
import time
from collections import Counter
from multiprocessing import Pool
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
import sim  # noqa: E402
import witness as W  # noqa: E402
from exact import ONE, ZERO, Q5, dot, matmul, matvec, transpose  # noqa: E402
from sim.kernel import Evaluator, filtered_signs, int_form, lcm, pullback_form  # noqa: E402
from sim.kplus import generator_word_matrix, matrix_from_json, matrix_json, q5_from_json, to_tuple  # noqa: E402
from sim.model import CELL_SLOTS, CONTRACT_REVISION, NP, NS  # noqa: E402

OUT = HERE / 'acceptance.json'
WORKERS = 4
SEED = 20261009
PRIM = None          # retained slot moves (src, dst, offsets), loaded before the pools fork
INDEP = None         # shared input of the independent recomputation workers
COMPARE_STATE = None
MENU_CONTROLS = None


def timed(fn, *a):
    t = time.time()
    r = fn(*a)
    r['runtime_s'] = round(time.time() - t, 1)
    print(f'  {fn.__name__}: {r["runtime_s"]} s', flush=True)
    return r


def prim_moves(k):
    src, dst, off = PRIM
    return src[off[k]:off[k + 1]], dst[off[k]:off[k + 1]]


def ref_labels(word):
    """Labels after a word of signed 1-based primitive ids, replayed on the retained slot moves
    exactly as core.Model.word_net does (a[d] = a[s]; an inverse swaps source and target)."""
    a = np.arange(NS, dtype=np.int64)
    for mid in word:
        s, d = prim_moves(abs(mid) - 1)
        if mid > 0:
            a[d] = a[s]
        else:
            a[s] = a[d]
    return a


# ---------------------------------------------------------------------------------------------
# 0. build and certification of the exact model data

def section_build(ctx):
    data, kp, reg = ctx.data, ctx.kplus, ctx.regions
    res = {'kplus': kp.verify_all()}
    res['kplus'].update({'retained_generators_in_kplus': int((kp.gen_idx >= 0).sum()),
                         'retained_frames_in_kplus': int((kp.frame_idx >= 0).sum()),
                         'stabiliser_of_pole_0': len(kp.stab0)})
    res['orbits'] = {k: reg.report[k] for k in ('kplus_orbits', 'orbit_sizes', 'g_class_in_one_kplus_orbit',
                                                  'kplus_orbit_g_classes')}
    res['orbits']['note'] = ('Each K+-orbit of pieces is exactly one retained orbit class (orbit_id); the '
                             '600 fixed cell centres (orbit_id -1) form one K+-orbit. So K+-orbits are '
                             'unions of G-orbits.')
    res['regions'] = {k: reg.report[k] for k in ('cap_region_vertices', 'cap_candidate_poles', 'cap_near_facets',
                                                   'far_facet_max_patch_dim_in_cap', 'rep_full_dimensional',
                                                   'rep_host_patches_match_retained', 'rep_vertex_counts')}
    res['regions'].update(reg.certify())
    # retained frames: exact K+ elements; float frames agree
    ferr = max(float(np.abs(data.frames[c] - np.array([[float(x) for x in r] for r in kp.matrix(int(kp.frame_idx[c]))])).max())
               for c in range(600))
    fpole = all(int(kp.perms[int(kp.frame_idx[c])][0]) == c for c in range(600))
    berr = max(float(np.abs(data.base_twists[t] - np.array([[float(x) for x in r] for r in kp.matrix(int(kp.gen_idx[t]))])).max())
               for t in range(2))
    res['frames'] = {'frames_exact_in_kplus': 600, 'frame_c_maps_pole_0_to_c': fpole,
                     'max_float_frame_deviation': ferr, 'frame_0_is_identity': int(kp.frame_idx[0]) == 0,
                     'base_twists_max_float_deviation_from_exact_H0_T0': berr}
    # slot layout covariance: slot 433 f + j holds the frame-F_f image of base region j
    j = np.arange(NS) % CELL_SLOTS
    f = np.arange(NS) // CELL_SLOTS
    base_piece = data.slot_piece[j]
    img = reg.piece_of[reg.orbit_of[base_piece], kp.compose_vec(kp.frame_idx[f], reg.transport[base_piece])]
    res['frames']['slot_layout_frame_covariant'] = f'{int((img == data.slot_piece).sum())}/{NS}'
    return res


# ---------------------------------------------------------------------------------------------
# 1. all 1,200 retained generators: labelled stickers and frames

def section_generators(ctx):
    data, kp, reg = ctx.data, ctx.kplus, ctx.regions
    perms = kp.perms.astype(np.int64)
    mo, mv = data.mask_offsets, data.mask_values
    fails = Counter()
    for k in range(1200):
        c, t = divmod(k, 2)
        gi = int(kp.gen_idx[k])
        # (a) exact matrix, pole images, cap stabiliser, order
        if not kp.pole_images_ok(gi):
            fails['pole_images'] += 1
        if perms[gi][c] != c:
            fails['fixes_pole'] += 1
        g2 = kp.compose(gi, gi)
        if not ((g2 == 0) if t == 0 else (g2 != 0 and kp.compose(gi, g2) == 0)):
            fails['order'] += 1
        # (b) frame conjugation of the base twists: r_k = F_c r_t F_c^-1
        fc = int(kp.frame_idx[c])
        if kp.compose(fc, kp.compose(int(kp.gen_idx[t]), int(kp.inv[fc]))) != gi:
            fails['frame_conjugation'] += 1
        # (c) piece map: inside set and transported signatures
        ms, md = data.move_src[k], data.move_dst[k]
        cap = data.cap_members(c)
        if not np.array_equal(np.sort(ms), cap):
            fails['inside_set'] += 1
        lens = mo[ms + 1] - mo[ms]
        ok = True
        for ln in np.unique(lens):
            sel = lens == ln
            a = mv[mo[ms[sel]][:, None] + np.arange(ln)]
            b = mv[mo[md[sel]][:, None] + np.arange(ln)]
            ok &= bool(np.array_equal(np.sort(perms[gi][a], axis=1), b))
        if not ok:
            fails['signature_transport'] += 1
        dst_geo = reg.piece_of[reg.orbit_of[ms], kp.compose_vec(np.full(len(ms), gi), reg.transport[ms])]
        if not np.array_equal(dst_geo, md):
            fails['piece_map'] += 1
        # (d) sticker map against the retained slot moves
        where = np.zeros(NP, bool)
        where[cap] = True
        S = np.nonzero(where[data.slot_piece])[0]
        A = data.slot_piece[S]
        piece_map = np.arange(NP)
        piece_map[ms] = md
        f = S // CELL_SLOTS
        f2 = perms[gi][f]
        dest = data.slot_of(piece_map[A], f2)
        moved = dest != S
        ps, pd = prim_moves(k)
        if not (np.array_equal(S[moved], ps) and np.array_equal(dest[moved], pd)):
            fails['sticker_map'] += 1
        # (e) frames: dest region index = L(src region index), L = F_f'^-1 r_k F_f in A4_0
        L = kp.compose_vec(kp.inv[kp.frame_idx[f2]], kp.compose_vec(np.full(len(S), gi), kp.frame_idx[f]))
        pos = ctx.stab0_pos[L]
        if (pos < 0).any() or not np.array_equal(ctx.base_region_perm[pos, S % CELL_SLOTS], dest % CELL_SLOTS):
            fails['frame_map'] += 1
    combinatorial = {'generators': 1200, 'failures': dict(fails), 'all_pass': not fails,
                     'checks': ['exact matrix maps all 600 poles as rotperms[k] (integer exact)',
                                'fixes n_c; order 2 for H_c, 3 for T_c',
                                'r_k = F_c r_t F_c^-1 with the retained frames and base twists',
                                'inside set from solved = move_src[k] = pieces whose signature contains c',
                                'perm_k(signature(src)) = signature(dst) for all 3,097 moves (region transport proof)',
                                'sticker (A, f) -> slot(dst(A), perm_k(f)) equals every primitives.npz slot move, '
                                'and the stickers it fixes are exactly those absent from primitives.npz',
                                'dest region index = L(src region index) with L = F_f\'^-1 r_k F_f in A4_0']}
    with Pool(WORKERS) as pool:
        rows = pool.map(_gen_worker, range(1200), chunksize=25)
        a4rows = pool.map(_a4_worker, range(600), chunksize=10)
    keys = ['admissible', 'moved_3097', 'signature_certificates', 'labels_equal_primitives', 'frames_agree',
            'checkpoint', 'undo_restores_bytes', 'inverse_labels_equal_primitives']
    state_level = {k: f'{sum(r[i + 1] for r in rows)}/1200' for i, k in enumerate(keys)}
    a4 = {'caps': 600, 'nonidentity_elements_admissible_from_solved_and_moving_3097': f'{sum(r[1] for r in a4rows)}/6600',
          'word_matrix_equal': f'{sum(r[2] for r in a4rows)}/6600',
          'undo_restores_bytes': f'{sum(r[3] for r in a4rows)}/600'}
    solved = sim.State(ctx).lattice_stickers()
    projection = {'solved_bijection': bool(np.array_equal(solved['slot'], np.arange(NS))
                                          and np.array_equal(solved['labels'], np.arange(NS))),
                  'solved_orientations_in_a4': bool(((0 <= solved['orientation']) & (solved['orientation'] < 12)).all()),
                  'generator_bijections': f'{sum(r[9] for r in rows)}/1200',
                  'generator_orientations_in_a4': f'{sum(r[10] for r in rows)}/1200',
                  'centre_pose_changes_with_fixed_labels': f'{sum(r[11] for r in rows)}/1200',
                  'centre_stickers_per_configuration': len(data.centre_slots),
                  'centre_stickers_checked': len(data.centre_slots) * (1 + len(rows)),
                  'labelled_stickers_per_configuration': NS}
    return {'combinatorial_all_1200': combinatorial, 'state_api_all_1200': state_level,
            'retained_a4_twists_from_solved': a4,
            'projection_bijection_and_centres': projection,
            'proof_note': ('Labelled agreement is proved combinatorially: each generator matrix is an exact K+ '
                           'element whose pole permutation is rotperms[k]; a K+ element maps the chamber of '
                           'signature M onto the chamber of signature r(M) and each host facet patch onto the patch '
                           'on the image facet, so perm_k(signature(src)) = signature(dst) proves that src lands '
                           'exactly on dst, and the sticker and frame identities follow from the exact facet map '
                           'and the frame-covariant slot layout. No floating-point geometry is used.')}


def _gen_worker(k):
    ctx = sim.get_context()
    st = sim.State(ctx)
    before = st.snapshot()
    out = st.apply(sim.generator(ctx, k))
    lab = st.lattice_stickers()
    labels_ok = bool(np.array_equal(lab['labels'], ref_labels([k + 1])))
    frames_ok = bool(lab['frame_agrees'].all())
    bijection = bool(np.array_equal(np.sort(lab['slot']), np.arange(NS))
                     and np.array_equal(np.sort(lab['labels']), np.arange(NS)))
    orientations = bool(((0 <= lab['orientation']) & (lab['orientation'] < 12)).all())
    centre = int(ctx.data.centre_pieces[ctx.data.centre_poles == k // 2][0])
    centre_ok = (st.pose(centre) == sim.generator(ctx, k).matrix and st.pose(centre) != ctx.kplus.matrix(0)
                 and np.array_equal(lab['slot'][ctx.data.centre_slots], ctx.data.centre_slots)
                 and np.array_equal(lab['labels'][ctx.data.centre_slots], ctx.data.centre_slots))
    cp = st.checkpoint()['checkpoint']
    st.undo()
    undo_ok = st.snapshot() == before
    out2 = st.apply(sim.primitive(ctx, -(k + 1)))
    inv_ok = out2.applied and bool(np.array_equal(st.lattice_stickers()['labels'], ref_labels([-(k + 1)])))
    return (k, out.applied, out.moved == 3097, out.certificates['signature'] == NP, labels_ok, frames_ok, cp,
            undo_ok, inv_ok, bijection, orientations, centre_ok)


def _a4_worker(c):
    ctx = sim.get_context()
    st = sim.State(ctx)
    before = st.snapshot()
    ok = word_ok = 0
    for i in range(1, 12):
        tw = sim.a4_element(ctx, c, i)
        word_ok += generator_word_matrix(ctx.kplus, tw.params['word']) == tw.matrix
        out = st.apply(tw)
        ok += out.applied and out.moved == 3097
        st.undo()
    return c, ok, word_ok, st.snapshot() == before


# ---------------------------------------------------------------------------------------------
# 2. witness E2/E3 reproduced by the simulator

def witness_twists(ctx):
    c = 0
    d = sorted(range(600), key=lambda e: -float(W.NF[e] @ W.NF[c]))[1]
    g = sim.plane(ctx, c, d, degrees=10, max_den=1000)
    td = next(a for a in W.a4(d) if sum((a[i][i] for i in range(4)), ZERO) == ONE and W.pole_perm(a)[c] != c)
    return c, d, g, sim.Twist(ctx, d, td)


def section_witness(ctx):
    wr = json.loads((HERE.parent / 'witness-results.json').read_text())
    c, d, g, T = witness_twists(ctx)
    gw, _, _ = W.witness_rotation(c, d)
    res = {'c': c, 'd': d, 'g_equals_witness_rotation': g.matrix == to_tuple(gw), 's': g.params['s'],
           'angle_deg': round(g.angle_deg(), 6), 'g_retained': g.retained, 'T_d_retained': T.retained}
    st = sim.State(ctx)
    solved = st.snapshot()
    out = st.apply(g)
    res['E2_apply'] = out.as_dict()
    sv2 = st.survey()
    b2 = sorted(e for e, r in sv2.items() if r['status'] == 'blocked')
    nb = [int(x) for x in wr['E2']['admissible_after_g']['interacting_poles_of_c']]
    res['E2'] = {'admissible': 600 - len(b2), 'blocked': len(b2), 'blocked_set_equals_witness': b2 == wr['E2']['admissible_after_g']['blocked'],
                 'interacting_statuses_equal_witness': {str(e): sv2[e]['status'] for e in nb} == wr['E2']['admissible_after_g']['interacting_poles_of_c'],
                 'certificate_kinds': _kinds(sv2), 'blocked_set': b2}
    # negative control (witness: grip 1, its second A4 element)
    before = st.snapshot()
    neg = st.apply(sim.Twist(ctx, 1, W.a4(1)[1]))
    wn = wr['E3_negative_control']
    res['negative_control'] = {'pole': 1, 'status': neg.status, 'unchanged': st.snapshot() == before,
                               'straddling_piece': neg.straddle['piece'],
                               'h_below': neg.straddle['h_below_float'], 'h_above': neg.straddle['h_above_float'],
                               'matches_witness_certificate': (neg.straddle['piece'] == wn['certificate']['piece']
                                                               and abs(neg.straddle['h_below_float'] - wn['certificate']['h_below']) < 1e-15
                                                               and abs(neg.straddle['h_above_float'] - wn['certificate']['h_above']) < 1e-15)}
    e2_state = sim.State.replay(st.journal, ctx)
    out = st.apply(T)
    res['E3_apply'] = out.as_dict()
    sv3 = st.survey()
    b3 = sorted(e for e, r in sv3.items() if r['status'] == 'blocked')
    res['E3'] = {'admissible': 600 - len(b3), 'blocked': len(b3), 'blocked_set_equals_witness': b3 == wr['E3']['blocked_after_g_then_t'],
                 'interacting_statuses_equal_witness': {str(e): sv3[e]['status'] for e in nb} == wr['E3']['interacting_poles_of_c_after_g_then_t'],
                 'c_status': sv3[c]['status'], 'off_lattice_pieces': st.off_lattice_count(),
                 'certificate_kinds': _kinds(sv3), 'blocked_set': b3}
    # alternative unblock (d, g T_d^-1), checked on the state and undone
    snap = st.snapshot()
    alt = sim.Twist(ctx, d, matmul(g.matrix, transpose(T.matrix)))
    oa = st.apply(alt)
    res['E3_alternative_unblock'] = {'admissible': oa.applied, 'moved': oa.moved,
                                     'c_status_after': st.survey([c])[c]['status'],
                                     'differs_from_before_T': st.digest() != e2_state.digest()}
    st.undo()
    res['E3_alternative_unblock']['undo_restores_bytes'] = st.snapshot() == snap
    e3_state = sim.State.replay(st.journal, ctx)
    o1 = st.undo()
    o2 = st.undo()
    res['E4'] = {'reverse_T_admissible': o1.applied, 'reverse_g_admissible': o2.applied,
                 'all_poses_identity': st.moved_count() == 0, 'bytes_equal_solved': st.snapshot() == solved}
    res['blocked_certificates'] = {'E2': {str(e): _cert_brief(sv2[e]['certificate']) for e in b2},
                                   'E3': {str(e): _cert_brief(sv3[e]['certificate']) for e in b3}}
    return res, e2_state, e3_state, sv2, sv3


def _kinds(sv):
    return {k: int(sum(r['certificates'][k] > 0 for r in sv.values() if r['status'] == 'admissible'))
            for k in ('group_superset', 'piece_vertices')}


def _cert_brief(cert):
    return {k: cert[k] for k in ('piece', 'h_below', 'h_above', 'point_below', 'point_above')}


# ---------------------------------------------------------------------------------------------
# 3. independent certification of blocked and unblocked cases

def verify_certificate(ctx, st, cert):
    """Literal re-check of a straddle certificate from its JSON: both exact points lie in the
    closed posed region (all 600 cut and 600 facet constraints, in plain Q5 arithmetic) and h
    has opposite strict signs at them."""
    data = ctx.data
    p, e = cert['piece'], cert['grip']
    g = st.pose(p)
    sig = data.signature(p)
    ok = True
    signs = []
    for key in ('point_below', 'point_above'):
        x = [q5_from_json(v) for v in cert[key]]
        signs.append(W.h(e, x).sign())
        z = matvec(transpose(g), x)
        for q in range(600):
            v = dot(data.N[q], z)
            if (v - data.NN).sign() > 0:
                ok = False
            s = (v - data.KAPPA).sign()
            if (q in sig and s < 0) or (q not in sig and s > 0):
                ok = False
    return ok and signs == [-1, 1]


def _indep_dd(args):
    p, x = args
    cap = INDEP['caps'][x]
    _, verts, full, _ = W.build_region((p, cap['poles'], cap['facets']))
    return p, verts


def _indep_grip(e):
    """Status of grip e from the independent data: fresh regions, plain Q5 evaluation of h."""
    posed, groups, caps_posed = INDEP['posed'], INDEP['groups'], INDEP['caps_posed']
    for (pid, x), pieces in groups.items():
        hs = [W.h(e, v).sign() for v in caps_posed[(pid, x)]]
        if min(hs) >= 0 or max(hs) <= 0:
            continue
        for p in pieces:
            sg = [W.h(e, v).sign() for v in posed[p]]
            if min(sg) < 0 < max(sg):
                return e, 'blocked'
    return e, 'admissible'


def independent_survey(ctx, st):
    """All 600 grip statuses recomputed without the simulator's classifier: every off-lattice
    piece region by a fresh double description of its own constraints (no K+ transport), posed
    by its exact pose, h evaluated in plain Q5 (no integer kernel), and group supersets from fresh
    witness.cap_region computations. Lattice pieces are chambers of the retained arrangement and
    cannot straddle any retained cut."""
    global INDEP
    data = ctx.data
    off = np.nonzero(~st.lattice_flags())[0]
    choice = {}
    for p in off.tolist():
        sig = data.signature(p)
        choice[p] = 0 if 0 in sig else 13 if 13 in sig else min(sig)
    caps = {}
    for x in sorted(set(choice.values())):
        verts = W.cap_region(x)
        near = [e for e in range(600) if W.NF[e] @ W.NF[x] > 0.6 * (W.NF[x] @ W.NF[x])]
        out = W.implied(verts)
        caps[x] = {'verts': verts, 'facets': near, 'poles': [e for e in range(600) if e not in out and e != x]}
    INDEP = {'caps': caps}
    with Pool(WORKERS) as pool:
        built = dict(pool.map(_indep_dd, list(choice.items()), chunksize=64))
    posed, groups, caps_posed = {}, {}, {}
    for p in off.tolist():
        pid = int(st.pose_id[p])
        g = st.pose(p)
        posed[p] = [matvec(g, v) for v in built[p]]
        groups.setdefault((pid, choice[p]), []).append(p)
        if (pid, choice[p]) not in caps_posed:
            caps_posed[(pid, choice[p])] = [matvec(g, v) for v in caps[choice[p]]['verts']]
    INDEP = {'posed': posed, 'groups': groups, 'caps_posed': caps_posed}
    with Pool(WORKERS) as pool:
        status = dict(pool.map(_indep_grip, range(600), chunksize=10))
    return status, posed


def float_survey(posed, band=1e-9):
    """Float cross-check: blocked when some piece has float h below -band and above +band."""
    nf = W.NF
    nn = (nf * nf).sum(1)
    pts, owner = [], []
    for i, (p, vs) in enumerate(posed.items()):
        for v in vs:
            pts.append([float(x) for x in v])
            owner.append(i)
    pts = np.array(pts)
    owner = np.array(owner)
    h = pts @ nf.T / nn - 121 / 125
    starts = np.flatnonzero(np.r_[True, np.diff(owner) != 0])
    lo = np.minimum.reduceat(h, starts, axis=0)
    hi = np.maximum.reduceat(h, starts, axis=0)
    blocked = ((lo < -band) & (hi > band)).any(axis=0)
    return {e: 'blocked' if blocked[e] else 'admissible' for e in range(600)}


def section_independent(ctx, e2_state, e3_state, sv2, sv3):
    res = {}
    for name, st, sv in (('E2', e2_state, sv2), ('E3', e3_state, sv3)):
        certs = [r['certificate'] for r in sv.values() if r['status'] == 'blocked']
        verified = sum(verify_certificate(ctx, st, c) for c in certs)
        status, posed = independent_survey(ctx, st)
        fl = float_survey(posed)
        res[name] = {'blocked_certificates_verified_literally': f'{verified}/{len(certs)}',
                     'independent_exact_statuses_equal': f'{sum(status[e] == sv[e]["status"] for e in range(600))}/600',
                     'float_cross_check_statuses_equal': f'{sum(fl[e] == sv[e]["status"] for e in range(600))}/600',
                     'independent_blocked': sum(v == 'blocked' for v in status.values()),
                     'independent_unblocked': sum(v == 'admissible' for v in status.values())}
    # signature certificates of lattice pieces against geometric classification
    res['signature_vs_geometry'] = signature_vs_geometry(ctx)
    res['method'] = ('Blocked: each straddle certificate is re-checked from its JSON against all 1,200 posed '
                     'constraints in plain Q5. Unblocked and blocked: all 600 statuses are recomputed from fresh '
                     'double descriptions of each off-lattice piece (no K+ transport, no integer kernel). A float '
                     'cross-check with a 1e-9 band is a third, non-certifying comparison.')
    return res


def signature_vs_geometry(ctx):
    """On lattice states, the signature certificate must agree with exact geometric
    classification of every piece whose posed chamber meets the neighbourhood of the grip."""
    data, kp, reg = ctx.data, ctx.kplus, ctx.regions
    rng = random.Random(SEED + 3)
    st_r = sim.State(ctx)
    for _ in range(30):
        st_r.apply(sim.primitive(ctx, rng.choice([1, -1]) * rng.randrange(1, 1201)))
    nbr = [list(np.argsort(-(data.NF @ data.NF[e]))[:57]) for e in range(600)]
    checked = disagreements = straddles = 0
    grips = [0, 13, 108] + rng.sample(range(600), 5)
    for st in (sim.State(ctx), st_r):
        K = st._kpose()[st.pose_id]
        comp = kp.compose_vec(K, reg.transport)
        posed_piece = reg.piece_of[reg.orbit_of, comp]
        for e in grips:
            near = np.zeros(NP, bool)
            for q in nbr[e]:
                near[data.cap_members(q)] = True
            sel = np.nonzero(near[posed_piece])[0]
            cl = st.classify(e)
            inside = np.zeros(NP, bool)
            inside[cl.inside] = True
            for k in np.unique(K[sel]).tolist():
                ev = Evaluator(pullback_form(kp.matrix(k), data.N[e]))
                for p in sel[K[sel] == k].tolist():
                    r = ev.classify(reg.vform(p))
                    checked += 1
                    straddles += r[0] == 'straddle'
                    disagreements += (r[0] == 'in') != bool(inside[p])
    return {'states': ['solved', 'random retained word of length 30'], 'grips': grips,
            'pieces_checked': checked, 'disagreements': disagreements, 'straddles': straddles}


# ---------------------------------------------------------------------------------------------
# 4. uncertain handling, shallow crossings, rejection without change

def section_controls(ctx):
    res = {}
    # E0 control regions (exact simplices) through the exact kernel and the A2 filter
    from sim.kernel import classify_signs, float_rows, rows_form
    cases = {'shallow_below': (Q5(-5, 0, 10 ** 10), Q5(1, 0, 1000)), 'shallow_above': (Q5(-1, 0, 1000), Q5(5, 0, 10 ** 10)),
             'contact_inside': (ZERO, Q5(1, 0, 1000)), 'contact_outside': (Q5(-1, 0, 1000), ZERO)}
    ev = Evaluator(pullback_form(ctx.kplus.matrix(0), ctx.data.N[0]))
    ctl = {}
    for name, (lo, hi) in cases.items():
        vf = rows_form(W.control_region(0, lo, hi))
        ex = ev.classify(vf)[0]
        fs = classify_signs(filtered_signs(ev, vf, float_rows(vf))[0])[0]
        ctl[name] = {'exact': ex, 'filtered': fs}
    res['E0_control_regions'] = ctl
    # real-geometry shallow crossings: a jumble twist by about 1e-9 degrees of rotation scale
    st = sim.State(ctx)
    st.apply(sim.plane(ctx, 0, 13, s=Q5(1, 0, 10 ** 9)))
    sv = st.survey(range(0, 120))
    blocked = [e for e, r in sv.items() if r['status'] == 'blocked']
    small = [min(abs(sv[e]['certificate']['h_below_float']), abs(sv[e]['certificate']['h_above_float'])) for e in blocked]
    before = st.snapshot()
    e_bad = blocked[0]
    o = st.apply(sim.a4_element(ctx, e_bad, 1))
    res['shallow_crossings'] = {'twist': 'plane(0, 13, s = 1e-9)', 'angle_deg': sim.plane(ctx, 0, 13, s=Q5(1, 0, 10 ** 9)).angle_deg(),
                                'grips_surveyed': 120, 'blocked': len(blocked),
                                'smallest_certified_abs_h': min(small), 'largest_of_smaller_sides': max(small),
                                'both_sides_shallow': sum(abs(sv[e]['certificate']['h_below_float']) < 1e-8
                                                          and abs(sv[e]['certificate']['h_above_float']) < 1e-8 for e in blocked),
                                'blocked_twist_rejected': o.status, 'unchanged': st.snapshot() == before}
    # uncertain: unrepresentable rotation
    rot = np.eye(4)
    rot[2:, 2:] = [[math.cos(1.0), -math.sin(1.0)], [math.sin(1.0), math.cos(1.0)]]
    st = sim.State(ctx)
    st.apply(sim.plane(ctx, 0, 13, degrees=10))
    before = st.snapshot()
    o = st.apply(sim.UnrepresentableTwist(0, rot))
    res['unrepresentable_rotation'] = {'status': o.status, 'reason': o.reason, 'unchanged': st.snapshot() == before}
    # uncertain: exact work budget exhausted before every piece has a certificate
    o = st.apply(sim.a4_element(ctx, 13, 1), budget=100)
    after_budget = st.snapshot() == before
    o2 = st.apply(sim.a4_element(ctx, 13, 1))
    st.undo()
    res['budget_exhausted'] = {'status': o.status, 'uncertain_pieces': o.uncertain, 'unchanged': after_budget,
                               'same_twist_without_budget': o2.status, 'exact_evaluations_needed': o2.exact_evaluations}
    # invalid inputs are refused before any state access
    bad = {}
    try:
        sim.Twist(ctx, 0, [[Q5(1) if i == j else ZERO for j in range(4)] for i in range(3)] + [[ZERO, ZERO, ZERO, Q5(2)]])
        bad['non_rotation'] = 'accepted'
    except sim.TwistError as exc:
        bad['non_rotation'] = f'rejected: {exc}'
    try:
        sim.Twist(ctx, 5, sim.plane(ctx, 0, 13, degrees=10).matrix)
        bad['does_not_fix_grip'] = 'accepted'
    except sim.TwistError as exc:
        bad['does_not_fix_grip'] = f'rejected: {exc}'
    rec = sim.plane(ctx, 0, 13, degrees=10).record()
    rec['params']['s'] = [16, 0, 961]
    try:
        sim.Twist.from_record(ctx, rec)
        bad['tampered_record'] = 'accepted'
    except sim.TwistError as exc:
        bad['tampered_record'] = f'rejected: {exc}'
    res['invalid_inputs'] = bad
    return res


# ---------------------------------------------------------------------------------------------
# 5. exact round trips of random admissible sequences with jumble twists

def random_twist(ctx, rng, recent, near):
    while True:
        try:
            return _random_twist(ctx, rng, recent, near)
        except sim.TwistError:
            continue


def _random_twist(ctx, rng, recent, near):
    last = recent[-1] if recent else None
    while True:
        r = rng.random()
        if recent and r < 0.15:
            c = rng.choice(recent)
        elif recent and r < 0.85:
            c = int(rng.choice(near[rng.choice(recent)]))
        else:
            c = rng.randrange(600)
        if c != last:
            break
    kind = rng.choices(['retained', 'plane', 'cayley'], [0.4, 0.3, 0.3])[0]
    if kind == 'retained':
        return sim.a4_element(ctx, c, rng.randrange(1, 12))
    if kind == 'plane':
        d = int(rng.choice(near[c][1:17]))
        return sim.plane(ctx, c, d, degrees=rng.uniform(3, 40) * rng.choice([-1, 1]), max_den=200)
    tw, _ = sim.cayley_axis_angle(ctx, c, [rng.gauss(0, 1) for _ in range(3)], rng.uniform(3, 60), max_den=64)
    return tw


def _roundtrip_worker(i):
    ctx = sim.get_context()
    nf = ctx.data.NF
    near = [list(np.argsort(-(nf @ nf[c]))[:57]) for c in range(600)]
    rng = random.Random(SEED * 1000 + i)
    st = sim.State(ctx)
    solved = st.snapshot()
    target = 8 + rng.randrange(5)
    rejections = Counter()
    unchanged = True
    recent = []
    geometric = 0
    max_off = 0
    attempts = 0
    t0 = time.time()

    def mixed():
        fam = [r['family'] for r in st.journal]
        return fam.count('retained') >= 2 and sum(f != 'retained' for f in fam) >= 3 and len({r['grip'] for r in st.journal}) >= 3

    while (len(st.journal) < target or not mixed()) and attempts < 400:
        attempts += 1
        tw = random_twist(ctx, rng, recent, near)
        before = st.snapshot()
        out = st.apply(tw)
        if out.applied:
            recent.append(tw.grip)
            geometric += (out.certificates['group_superset'] + out.certificates['piece_vertices']) > 0
            max_off = max(max_off, st.off_lattice_count())
        else:
            rejections[out.status] += 1
            unchanged &= st.snapshot() == before
    final = st.digest()
    fams = Counter(r['family'] for r in st.journal)
    # replay from the JSON journal
    rep = sim.State.replay(st.journal_json(), ctx)
    replay_ok = rep.digest() == final and rep.snapshot() == st.snapshot()
    # explicit inverse sequence, journalled as ordinary twists
    for rec in list(reversed(rep.journal)):
        o = rep.apply(sim.Twist.from_record(ctx, rec).inverse())
        if not o.applied:
            break
    inverse_ok = rep.moved_count() == 0 and rep.digest() == sim.State(ctx).digest()
    # undo
    while st.journal:
        st.undo()
    undo_ok = st.snapshot() == solved
    return {'seed_index': i, 'length': sum(fams.values()), 'families': dict(fams),
            'distinct_grips': len(set(recent)), 'twists_needing_geometric_certificates': geometric,
            'max_off_lattice_pieces': max_off, 'rejections': dict(rejections), 'rejections_left_state_unchanged': unchanged,
            'replay_from_json_equal': replay_ok, 'inverse_sequence_returns_to_solved': inverse_ok,
            'undo_returns_to_solved_bytes': undo_ok, 'final_digest': final[:16], 'runtime_s': round(time.time() - t0, 1)}


def section_round_trips(ctx):
    with Pool(WORKERS) as pool:
        rows = pool.map(_roundtrip_worker, range(20), chunksize=1)
    return {'sequences': rows, 'count': len(rows), 'seed': SEED,
            'all_length_at_least_8': all(r['length'] >= 8 for r in rows),
            'all_include_jumble_and_retained': all(r['families'].get('retained', 0) >= 2 and r['length'] - r['families'].get('retained', 0) >= 3 for r in rows),
            'all_replay_equal': all(r['replay_from_json_equal'] for r in rows),
            'all_inverse_sequences_exact': all(r['inverse_sequence_returns_to_solved'] for r in rows),
            'all_undo_exact': all(r['undo_returns_to_solved_bytes'] for r in rows),
            'all_rejections_unchanged': all(r['rejections_left_state_unchanged'] for r in rows),
            'total_rejections': dict(sum((Counter(r['rejections']) for r in rows), Counter()))}


def _compare_grip(e):
    st = COMPARE_STATE
    grouped = st.classify(e)
    per_piece = st.classify_per_piece(e)
    inside_checked = grouped.status == per_piece.status == 'admissible'
    return {'status_equal': grouped.status == per_piece.status,
            'admissible': inside_checked,
            'inside_equal': inside_checked and bool(np.array_equal(grouped.inside, per_piece.inside)),
            'reference_pieces': per_piece.counts['piece_vertices'],
            'reference_has_no_superset_or_filter': per_piece.counts['group_superset'] == per_piece.filtered_evaluations == 0,
            'uncertain': grouped.status == 'uncertain' or per_piece.status == 'uncertain'}


def section_grouped_per_piece(ctx, e2_state, e3_state):
    """G2: all off-lattice pieces, with no group bound, anchor or filter in the reference."""
    global COMPARE_STATE
    rng = random.Random(SEED + 17)
    near = [list(np.argsort(-(ctx.data.NF @ ctx.data.NF[c]))[:57]) for c in range(600)]
    states = [('E2', e2_state, list(range(600))), ('E3', e3_state, list(range(600)))]
    for i in range(2):
        st = sim.State(ctx)
        recent = []
        attempts = 0
        while len(st.journal) < 5 or not any(r['family'] == 'retained' for r in st.journal) or not any(r['family'] != 'retained' for r in st.journal):
            attempts += 1
            if attempts > 400:
                raise AssertionError('seeded mixed-state fixture failed to finish')
            tw = random_twist(ctx, rng, recent, near)
            if st.apply(tw).applied:
                recent.append(tw.grip)
        grips = sorted(rng.sample(range(600), 60))
        states.append((f'random_{i}', st, grips))
    res = {'seed': SEED + 17, 'method': 'State.classify_per_piece; every off-lattice piece individually; exact integer Q(sqrt5) signs; no superset, anchor or filter'}
    for name, st, grips in states:
        COMPARE_STATE = st
        with Pool(WORKERS) as pool:
            rows = pool.map(_compare_grip, grips, chunksize=10)
        admissible = sum(r['admissible'] for r in rows)
        res[name] = {'grips': grips, 'off_lattice_pieces': st.off_lattice_count(),
                     'statuses_equal': f'{sum(r["status_equal"] for r in rows)}/{len(rows)}',
                     'admissible_inside_sets_equal': f'{sum(r["inside_equal"] for r in rows)}/{admissible}',
                     'reference_piece_evaluations': sum(r['reference_pieces'] for r in rows),
                     'every_off_lattice_piece_evaluated': all(r['reference_pieces'] == st.off_lattice_count() for r in rows),
                     'reference_has_no_superset_or_filter': all(r['reference_has_no_superset_or_filter'] for r in rows),
                     'no_uncertain': not any(r['uncertain'] for r in rows),
                     'families': dict(Counter(r['family'] for r in st.journal)),
                     'journal': json.loads(st.journal_json())}
    COMPARE_STATE = None
    return res


# ---------------------------------------------------------------------------------------------
# 6. checkpoints and handoff (contract section 5)

def section_checkpoints(ctx):
    rng = random.Random(SEED + 11)
    res = {}
    word = [rng.choice([1, -1]) * rng.randrange(1, 1201) for _ in range(40)]
    st = sim.State(ctx)
    for mid in word:
        assert st.apply(sim.primitive(ctx, mid)).applied
    cp = st.checkpoint()
    labels, _ = st.export_retained()
    res['retained_word'] = {'length': len(word), 'checkpoint_from_journal': cp['checkpoint'],
                            'export_equals_primitives_replay': bool(np.array_equal(labels, ref_labels(word))),
                            'supplied_witness': st.checkpoint(word)['checkpoint'],
                            'frames_agree': bool(st.lattice_stickers()['frame_agrees'].all())}
    # excursion through jumble twists that merge into a retained twist of the same grip
    c = 0
    g = sim.plane(ctx, c, 13, degrees=10)
    a = sim.a4_element(ctx, c, 5)
    st2 = sim.State.replay(st.journal, ctx)
    o1 = st2.apply(g)
    o2 = st2.apply(sim.Twist(ctx, c, matmul(a.matrix, transpose(g.matrix))))
    cp2 = st2.checkpoint()
    lab2 = st2.export_retained()[0] if cp2['checkpoint'] else None
    res['merged_jumble_excursion'] = {'applied': o1.applied and o2.applied, 'lattice': st2.is_lattice(),
                                      'checkpoint_from_journal': cp2['checkpoint'],
                                      'export_equals_primitives_replay': lab2 is not None and bool(np.array_equal(lab2, ref_labels(cp2['witness_word'])))}
    excursion = sim.State(ctx)
    applied = excursion.apply(g).applied
    was_off_lattice = not excursion.is_lattice()
    applied &= excursion.apply(sim.Twist(ctx, c, matmul(a.matrix, transpose(g.matrix)))).applied
    retained = sim.State(ctx)
    retained.apply(a)
    cp_exc = excursion.checkpoint()
    res['same_cap_excursion'] = {'applied': applied, 'was_off_lattice': was_off_lattice,
                                  'lattice': excursion.is_lattice(), 'checkpoint': cp_exc['checkpoint'],
                                  'nonidentity_retained_pose': excursion.digest() == retained.digest() != sim.State(ctx).digest(),
                                  'witness_word': cp_exc.get('witness_word'),
                                  'export_equals_primitives_replay': bool(np.array_equal(excursion.export_retained()[0], ref_labels(cp_exc['witness_word'])))}
    # Global rotations are lattice configurations whose centres occupy other chambers.
    kp, data = ctx.kplus, ctx.data
    moving_pole = int(np.flatnonzero(kp.perms[:, 0] != 0)[0])
    global_rows = []
    for k in (kp.stab0[1], moving_pole):
        rotated = sim.State.from_global_rotation(ctx, k)
        supplied = [1, 2, -1]
        refused = []
        for witness in (None, supplied):
            try:
                rotated.export_retained(witness)
                refused.append(False)
            except ValueError:
                refused.append(True)
        global_rows.append({'kplus_index': k, 'fixes_pole_0': int(kp.perms[k, 0]) == 0,
                            'lattice': rotated.is_lattice(), 'digest_differs_from_solved': rotated.digest() != sim.State(ctx).digest(),
                            'centres_in_other_chambers': int((kp.perms[k, data.centre_poles] != data.centre_poles).sum()),
                            'checkpoint': rotated.checkpoint()['checkpoint'],
                            'checkpoint_with_supplied_word': rotated.checkpoint(supplied)['checkpoint'],
                            'export_refused': all(refused)})
    res['global_rotation_control'] = global_rows
    # lattice configuration reached through jumble twists that do not cancel: no witness unless supplied
    far = int(np.argmin(ctx.data.NF @ ctx.data.NF[0]))
    st3 = sim.State(ctx)
    r = [st3.apply(g).applied, st3.apply(sim.a4_element(ctx, far, 3)).applied, st3.apply(g.inverse()).applied]
    cp3 = st3.checkpoint()
    w = sim.a4_element(ctx, far, 3).params['word']
    try:
        st3.export_retained()
        export_refused = False
    except ValueError:
        export_refused = True
    res['unwitnessed_lattice_state'] = {'applied': all(r), 'lattice': st3.is_lattice(), 'checkpoint_without_witness': cp3['checkpoint'],
                                        'reason': cp3['reason'], 'export_refused': export_refused,
                                        'checkpoint_with_supplied_witness': st3.checkpoint(w)['checkpoint'],
                                        'wrong_witness_rejected': not st3.checkpoint(w + w)['checkpoint']}
    # off-lattice configuration
    st4 = sim.State(ctx)
    st4.apply(g)
    cp4 = st4.checkpoint()
    res['off_lattice_state'] = {'checkpoint': cp4['checkpoint'], 'reason': cp4['reason'], 'off_lattice': cp4.get('off_lattice')}
    # full witness round trip E2 -> E4 is solved again
    cc, d, gg, T = witness_twists(ctx)
    st5 = sim.State(ctx)
    for tw in (gg, T, T.inverse(), gg.inverse()):
        st5.apply(tw)
    cp5 = st5.checkpoint()
    res['witness_excursion_back_to_solved'] = {'checkpoint': cp5['checkpoint'], 'witness_word': cp5.get('witness_word')}
    return res


# ---------------------------------------------------------------------------------------------
# 7. A2 filter (off by default)

def section_filter(ctx, e2_state, e3_state, sv2, sv3):
    res = {'default_off': sim.State(ctx).filtered is False}
    for name, st, sv in (('E2', e2_state, sv2), ('E3', e3_state, sv3)):
        stf = sim.State.replay(st.journal, ctx, filtered=True)
        fv = stf.survey()
        res[name] = {'statuses_and_certificate_counts_equal': all(fv[e]['status'] == sv[e]['status'] and fv[e]['certificates'] == sv[e]['certificates'] for e in range(600)),
                     'filter_decided': sum(r['filtered_evaluations'] for r in fv.values()),
                     'exact_fallback': sum(r['exact_evaluations'] for r in fv.values())}
        res[name]['certificate_fields'] = check_filter_records(stf, 13 if name == 'E2' else 0)
    # sign agreement over many vertices of random jumbled states and of a 1e-9 twist
    rng = random.Random(SEED + 5)
    states = []
    for i in range(3):
        st = sim.State(ctx)
        nf = ctx.data.NF
        near = [list(np.argsort(-(nf @ nf[c]))[:57]) for c in range(600)]
        recent = []
        while len(st.journal) < 8:
            tw = random_twist(ctx, rng, recent, near)
            if st.apply(tw).applied:
                recent.append(tw.grip)
        states.append(st)
    tiny = sim.State(ctx)
    tiny.apply(sim.plane(ctx, 0, 13, s=Q5(1, 0, 10 ** 9)))
    states.append(tiny)
    reg = ctx.regions
    dis = dec = fb = 0
    for st in states:
        moved = np.nonzero(~st.lattice_flags())[0]
        for pid, members in st._groups(moved):
            g = st._pose_matrix(pid)
            for e in rng.sample(range(600), 60):
                ev = Evaluator(pullback_form(g, ctx.data.N[e]))
                for p in members[::7].tolist():
                    vf = reg.vform(p)
                    signs, nd, ne = filtered_signs(ev, vf, reg.frows(p))
                    dis += signs != [ev.sign_row(vf[0], row) for row in vf[1]]
                    dec += nd
                    fb += ne
    res['sign_agreement'] = {'piece_evaluations_with_disagreement': dis, 'vertex_signs_decided_by_filter': dec,
                             'vertex_signs_left_to_exact': fb}
    res['bound'] = 'sign(S) = sign(S^) when |S^| > 2^-48 (sum m(y_i) m(x_i) + m(kappa)) + 2^-1000; derivation in kernel.py'
    return res


def check_filter_records(st, grip):
    """Rebuild every recorded evaluation's exact inputs; check signs independently in Q5."""
    cl = st.classify(grip, record=True)
    ctx, data, reg = st.ctx, st.ctx.data, st.ctx.regions
    digest = st.digest()
    required = {'pose_key', 'vertex_set', 'vertex', 'vertex_form', 'normal_form', 'cut_offset',
                'state_digest', 'model_identity', 'coverage', 'arithmetic_version',
                'error_bound_version', 'enclosure', 'accepted_sign', 'method', 'grip'}
    fields_ok = signs_ok = bounds_ok = coverage_ok = 0
    float_n = fallback_n = 0
    for rec in cl.decisions:
        fields_ok += required <= rec.keys() and rec['state_digest'] == digest and rec['model_identity'] == data.identity
        key = rec['pose_key']
        pose = ctx.kplus.matrix(key[1]) if key[0] == 'K' else matrix_from_json(key[1])
        if 'anchor' in rec:
            vf = reg.cap_vform(rec['anchor'])
            members = np.array(rec['coverage']['members'], np.int64)
            covered = (rec['coverage']['kind'] == 'constraint_superset'
                       and rec['coverage']['constraint_subset'] == {'inside_cuts': [rec['anchor']], 'facets': list(range(600))}
                       and data.in_sig(members, np.full(len(members), rec['anchor'])).all()
                       and all(st.pose(p) == pose for p in members.tolist()))
        else:
            vf = reg.vform(rec['piece'])
            covered = rec['coverage']['kind'] == 'complete_vertex_set' and st.pose(rec['piece']) == pose
        row = vf[1][rec['vertex']]
        v = [Q5(row[i], row[i + 4], vf[0]) for i in range(4)]
        exact = data.h(grip, matvec(pose, v)).sign()
        signs_ok += exact == rec['accepted_sign']
        coverage_ok += (covered and rec['coverage']['vertex_count'] == len(vf[1])
                        and rec['vertex_form'] == {'denominator': vf[0], 'row': list(row)})
        if rec['method'] == 'filtered':
            float_n += 1
            lo, hi = rec['enclosure']
            bounds_ok += (math.isfinite(lo) and math.isfinite(hi) and lo <= hi
                          and (lo > 0 and exact == 1 or hi < 0 and exact == -1))
        else:
            fallback_n += 1
            if 'fallback_reason' not in rec:
                fields_ok -= 1
    return {'grip': grip, 'float_decisions': float_n, 'exact_fallbacks': fallback_n,
            'all_evaluations_recorded': len(cl.decisions) == cl.filtered_evaluations + cl.exact_evaluations,
            'fields_complete': f'{fields_ok}/{len(cl.decisions)}',
            'coverage_verified': f'{coverage_ok}/{len(cl.decisions)}',
            'accepted_signs_equal_exact': f'{signs_ok}/{len(cl.decisions)}',
            'float_enclosures_exclude_zero_with_exact_sign': f'{bounds_ok}/{float_n}'}


# ---------------------------------------------------------------------------------------------
# 8. twists (A1), menus (A4) and sticker frames off the lattice (A3)

def section_twists(ctx):
    data = ctx.data
    rng = random.Random(SEED + 13)
    res = {}
    # A1: plane rotations are Cayley rotations; inverses
    eq = 0
    cases = [(0, 13, Q5(15, 0, 961))]
    nf = data.NF
    for _ in range(19):
        c = rng.randrange(600)
        d = int(rng.choice(list(np.argsort(-(nf @ nf[c]))[1:57])))
        cases.append((c, d, Q5(rng.randrange(-50, 50) or 1, rng.randrange(-5, 5), rng.randrange(60, 400))))
    for c, d, s in cases:
        pl = sim.plane(ctx, c, d, s=s)
        om = [-s * dot(u, data.N[d]) for u in sim.cap_frame(data.N[c])]
        cy = sim.cayley(ctx, c, om)
        eq += pl.matrix == cy.matrix and cy.inverse().matrix == sim.cayley(ctx, c, [-x for x in om]).matrix
    res['plane_equals_cayley_and_inverse'] = f'{eq}/{len(cases)}'
    reqs = []
    for axis, deg, den in (([1, 0, 0], 10, 1000), ([1, 2, 3], 17.5, 100), ([0.3, -0.2, 0.9], 72, 64), ([1, 1, 1], 120, 1000), ([0, 0, 1], 36, 10000)):
        tw, rep = sim.cayley_axis_angle(ctx, 7, axis, deg, max_den=den)
        reqs.append({'axis': axis, 'requested_deg': deg, 'max_den': den, 'omega': tw.params['omega'],
                     'realised_deg': rep['realised_angle_deg'], 'axis_error_deg': rep['axis_error_deg'],
                     'rotation_error_deg': rep['rotation_error_deg'], 'retained': tw.retained,
                     'N': rep['N'], 'D': rep['D'], 'distance': rep['distance'], 'realised_axis': rep['realised_axis']})
    res['axis_angle_requests'] = reqs
    try:
        ht, report = sim.cayley_axis_angle(ctx, 7, [1, 0, 0], 180)
        res['half_turn_request'] = 'accepted' if ht.family == 'half_turn' else 'wrong family'
    except sim.TwistError as exc:
        res['half_turn_request'] = f'rejected: {exc}'
    # A4: menus
    menu = sim.TwistMenu.a4(ctx)
    same = sum({tw.matrix for tw in menu.for_cap(c)} == {sim.a4_element(ctx, c, i).matrix for i in range(12)} for c in range(600))
    st = sim.State(ctx, menu=menu)
    before = st.snapshot()
    oj = st.apply(sim.plane(ctx, 0, 13, degrees=10))
    unchanged = st.snapshot() == before
    orr = st.apply(menu.for_cap(0)[3])
    cm = sim.TwistMenu.cayley_set(ctx, 'cayley-quarter', [[Q5(1, 0, 4), ZERO, ZERO]])
    member = sum(cm.contains(tw) for c in rng.sample(range(600), 20) for tw in cm.for_cap(c))
    res['menus'] = {'a4_menu_size': len(menu), 'a4_menu_invariant': menu.invariant,
                    'a4_menu_equals_A4_c_for_all_caps': f'{same}/600',
                    'a4_menu_state_rejects_jumble': oj.status, 'rejection_unchanged': unchanged,
                    'a4_menu_state_accepts_retained': orr.status,
                    'cayley_menu_size_after_closure': len(cm), 'cayley_menu_invariant': cm.invariant,
                    'cayley_menu_transported_members_recognised': f'{member}/{20 * len(cm)}',
                    'cayley_menu_rejects_other_angle': not cm.contains(sim.cayley(ctx, 0, [Q5(1, 0, 5), ZERO, ZERO]))}
    # A3 off the lattice: sticker frames are carried by the pose
    st = sim.State(ctx)
    g = sim.plane(ctx, 0, 13, degrees=10)
    st.apply(g)
    kp = ctx.kplus
    ok = 0
    cap_slots = [s for s in range(0, NS, 7) if 0 in data.signature(int(data.slot_piece[s]))]
    for s in cap_slots:
        fr = st.sticker_frame(s)
        f = s // CELL_SLOTS
        ok += (fr == to_tuple(matmul(g.matrix, kp.matrix(int(kp.frame_idx[f]))))
               and matvec(fr, data.N[0]) == matvec(g.matrix, data.N[f]))
    res['off_lattice_sticker_frames'] = {'stickers_checked': len(cap_slots), 'frame_equals_pose_times_cell_frame': ok}
    return res


def negative_control_twist(ctx):
    """The half-turn fixing n_0 and n_13, built through A1's exact axis branch."""
    axis = [dot(u, ctx.data.N[13]) / ctx.data.NN for u in sim.cap_frame(ctx.data.N[0])]
    return sim.half_turn(ctx, 0, axis)


def section_negative_control(ctx):
    g = negative_control_twist(ctx)
    h = sim.generator(ctx, 0)
    product = matmul(h.matrix, g.matrix)
    trace = sum((product[i][i] for i in range(4)), ZERO)
    menu = sim.TwistMenu(ctx, 'NC', [('g', g.matrix)])
    st = sim.State(ctx, menu=menu)
    centre = int(ctx.data.centre_pieces[ctx.data.centre_poles == 0][0])
    digests, heights, outcomes = [st.digest()], [], []
    for _ in range(12):
        for tw in (h, g):
            outcome = st.apply(tw)
            outcomes.append(outcome.status)
            digests.append(st.digest())
        heights.append(max(max(abs(x.a), abs(x.b), x.d) for row in st.pose(centre) for x in row))
    return {'g': g.record(), 'g_not_in_a4': not g.retained,
            'g_squared_is_identity': to_tuple(matmul(g.matrix, g.matrix)) == ctx.kplus.matrix(0),
            'g_fixes_pole_13': matvec(g.matrix, ctx.data.N[13]) == ctx.data.N[13],
            'trace_H0_g': [trace.a, trace.b, trace.d], 'trace_is_four_thirds': trace == Q5(4, 0, 3),
            'infinite_order_argument': 'A finite-order rotation has algebraic-integer trace; the rational 4/3 is not an algebraic integer.',
            'menu': menu.record(), 'rounds': 12, 'outcomes': outcomes,
            'all_twists_admissible': all(s == 'admissible' for s in outcomes),
            'digests': digests, 'digests_pairwise_distinct': len(set(digests)) == len(digests),
            'cap_0_pose_entry_heights': heights, 'entry_height_grows': heights[-1] > heights[0],
            'replay_equal': sim.State.replay(st.journal_json(), ctx, menu=menu).digest() == st.digest(),
            'journal': json.loads(st.journal_json())}


def _matrix_int_key(m):
    a, b, d = int_form([x for row in m for x in row])
    common = math.gcd(d, *a, *b)
    return tuple(x // common for x in a + b) + (d // common,)


def _transport_keys(ctx, menu_forms, t):
    """Exact batched conjugation in integer Q5 form. Object dtype prevents overflow."""
    a, b, e = menu_forms
    x, y, d = ctx.kplus.int_matrix(t)
    x, y = x.astype(object), y.astype(object)
    p, q = x @ a + 5 * (y @ b), x @ b + y @ a
    r, s = p @ x.T + 5 * (q @ y.T), p @ y.T + q @ x.T
    denominator = int(d) * int(d) * e
    keys = set()
    for rr, ss in zip(r, s):
        values = tuple(rr.ravel()) + tuple(ss.ravel())
        common = math.gcd(denominator, *values)
        keys.add(tuple(v // common for v in values) + (denominator // common,))
    return keys


def _menu_control_cap(c):
    ctx = sim.get_context()
    f = int(ctx.kplus.frame_idx[c])
    rows = []
    for menu, forms in MENU_CONTROLS:
        canonical = _transport_keys(ctx, forms, f)
        production = {_matrix_int_key(tw.matrix) for tw in menu.for_cap(c)}
        independent = production == canonical
        for a in ctx.kplus.stab0:
            fa = ctx.kplus.compose(f, a)
            independent &= _transport_keys(ctx, forms, fa) == canonical
        rows.append(independent)
    return rows


def section_menu_controls(ctx):
    global MENU_CONTROLS
    menus = [sim.TwistMenu.a4(ctx), sim.TwistMenu.s4(ctx),
             sim.TwistMenu(ctx, 'NC', [('g', negative_control_twist(ctx).matrix)])]
    MENU_CONTROLS = []
    for menu in menus:
        forms = [int_form([x for row in m for x in row]) for _, m in menu.items]
        e = 1
        for _, _, d in forms:
            e = lcm(e, d)
        a = np.array([[x * (e // d) for x in aa] for aa, _, d in forms], dtype=object).reshape(-1, 4, 4)
        b = np.array([[x * (e // d) for x in bb] for _, bb, d in forms], dtype=object).reshape(-1, 4, 4)
        MENU_CONTROLS.append((menu, (a, b, e)))
    with Pool(WORKERS) as pool:
        rows = pool.map(_menu_control_cap, range(600), chunksize=10)
    res = {'transporters_per_cap': 12, 'method': 'exact Q(sqrt5) conjugation by F_c and each K+ product F_c a; integer object arrays, no floats'}
    for i, menu in enumerate(menus):
        res[menu.name] = {'size': len(menu), 'identity': menu.identity,
                          'inverse_closed': menu.inverse_closed, 'a4_invariant': menu.a4_invariant,
                          'contains_a4': menu.contains_a4,
                          'all_transports_equal': f'{sum(row[i] for row in rows)}/600',
                          'record_round_trip': sim.TwistMenu.from_record(ctx, menu.record()).identity == menu.identity}
    MENU_CONTROLS = None
    return res


def section_input_map(ctx):
    """Exhaustive independent small-box checks of A1's minimum and tie order."""
    rows = []
    for axis, deg, n, dmax in (([1, 2, 3], 67, 2, 5), ([1, 3, 2], 179.6, 2, 4),
                               ([1, 2, -3], -179.7, 2, 3), ([1, 0, 0], 180, 2, 7),
                               ([1, 2, 3], 0, 2, 4)):
        unit = np.array(axis, float) / np.linalg.norm(axis)
        x, y, z = unit
        w = np.array([[0, -z, y], [z, 0, -x], [-y, x, 0]])
        theta = math.radians(deg)
        target = np.eye(3) + math.sin(theta) * w + (1 - math.cos(theta)) * (w @ w)
        candidates = []
        for nums in itertools.product(range(-n, n + 1), repeat=3):
            p = np.array(nums, float)
            for den in range(1, dmax + 1):
                x, y, z = p / den
                w = np.array([[0, -z, y], [z, 0, -x], [-y, x, 0]])
                rotation = np.eye(3) + 2 * (w + w @ w) / (1 + (p @ p) / (den * den))
                candidates.append((float(np.linalg.norm(rotation - target)), den, nums, 'cayley'))
            if abs(abs(math.remainder(deg, 360)) - 180) <= 1 and np.any(p):
                rotation = 2 * np.outer(p, p) / (p @ p) - np.eye(3)
                candidates.append((float(np.linalg.norm(rotation - target)), 1, nums, 'half_turn'))
        minimum = min(r[0] for r in candidates)
        chosen = min((den, nums, fam) for dist, den, nums, fam in candidates if dist <= minimum + 1e-12)
        tw, report = sim.cayley_axis_angle(ctx, 7, axis, deg, max_num=n, max_den=dmax)
        rows.append({'axis': axis, 'angle_deg': deg, 'N': n, 'D': dmax,
                     'global_minimum_equal': abs(report['minimum_distance'] - minimum) < 1e-12,
                     'tie_order_equal': (report['denominator'], tuple(report['numerators']), report['family']) == chosen,
                     'journal_records_bounds_and_distance': all(tw.params['requested'][k] == report[k] for k in ('N', 'D', 'distance')),
                     'exact_record_round_trip': sim.Twist.from_record(ctx, tw.record()).matrix == tw.matrix,
                     'report': report})
    # The field-valued API does not turn an explicitly unrepresentable rotation into a twist.
    angle = 2 * math.pi / 7
    r = np.eye(4)
    r[2:, 2:] = [[math.cos(angle), -math.sin(angle)], [math.sin(angle), math.cos(angle)]]
    st = sim.State(ctx)
    before = st.snapshot()
    out = st.apply(sim.UnrepresentableTwist(0, r, 'seventh-turn trace is outside Q(sqrt5)'))
    menu = sim.TwistMenu.a4(ctx)
    mapped, _ = sim.cayley_axis_angle(ctx, 0, [1, 1, 1], 119, max_num=1, max_den=1)
    return {'exhaustive_cases': rows, 'nonrepresentable_rejected_unchanged': out.status == 'uncertain' and st.snapshot() == before,
            'menu_request_never_approximated': sim.State(ctx, menu=menu).apply(mapped).status == 'invalid'}


def section_journal_identity(ctx):
    menu = sim.TwistMenu.a4(ctx)
    st = sim.State(ctx, menu=menu)
    st.apply(sim.generator(ctx, 0))
    doc = json.loads(st.journal_json())
    refused = {}
    for field in ('model_identity', 'menu_identity', 'contract_revision'):
        wrong = dict(doc, **{field: 'wrong'})
        try:
            sim.State.replay(wrong, ctx, menu=menu)
            refused[field] = False
        except ValueError:
            refused[field] = True
    legacy = {'format': doc['format'], 'records': doc['records']}
    return {'model_identity': ctx.data.identity, 'menu_identity': menu.identity, 'contract_revision': CONTRACT_REVISION,
            'model_hashes_match_assets': all(hashlib.sha256((ctx.data.root / path).read_bytes()).hexdigest() == sha for path, sha in ctx.data.identity.items()),
            'identity_fields_recorded': all(field in doc for field in refused), 'mismatches_refused': refused,
            'matching_identities_replay': sim.State.replay(st.journal_json(), ctx, menu=menu).snapshot() == st.snapshot(),
            'legacy_document_replays': sim.State.replay(legacy, ctx).snapshot() == st.snapshot()}


# ---------------------------------------------------------------------------------------------

def main():
    global PRIM
    t0 = time.time()
    print('building context', flush=True)
    ctx = sim.get_context()
    PRIM = ctx.data.primitives()
    t_ctx = round(time.time() - t0, 1)
    res = {'contract': CONTRACT_REVISION + '; sections 2, 3, 5, 6, 7; plan J1 items 1-9',
           'model_identity': ctx.data.identity,
           'evidence_kind': 'source and synthetic geometry (cloud, headless); not Windows, Direct3D or performance evidence',
           'machine': {'python': platform.python_version(), 'numpy': np.__version__, 'cpus': os.cpu_count(), 'workers': WORKERS},
           'context_build_s': t_ctx}
    print('section build', flush=True)
    res['build'] = timed(section_build, ctx)
    print('section generators', flush=True)
    res['generators'] = timed(section_generators, ctx)
    print('section witness', flush=True)
    t = time.time()
    wit, e2_state, e3_state, sv2, sv3 = section_witness(ctx)
    wit['runtime_s'] = round(time.time() - t, 1)
    res['witness'] = wit
    print('section independent', flush=True)
    res['independent_certification'] = timed(section_independent, ctx, e2_state, e3_state, sv2, sv3)
    print('section controls', flush=True)
    res['uncertain_and_rejection'] = timed(section_controls, ctx)
    print('section round trips', flush=True)
    res['round_trips'] = timed(section_round_trips, ctx)
    print('section grouped against per-piece', flush=True)
    res['grouped_per_piece'] = timed(section_grouped_per_piece, ctx, e2_state, e3_state)
    print('section checkpoints', flush=True)
    res['checkpoints'] = timed(section_checkpoints, ctx)
    print('section filter', flush=True)
    res['filter_A2'] = timed(section_filter, ctx, e2_state, e3_state, sv2, sv3)
    print('section twists', flush=True)
    res['twists_menus_frames'] = timed(section_twists, ctx)
    print('section menu controls', flush=True)
    res['menu_controls'] = timed(section_menu_controls, ctx)
    print('section negative control', flush=True)
    res['negative_control'] = timed(section_negative_control, ctx)
    print('section input map', flush=True)
    res['input_map_A1'] = timed(section_input_map, ctx)
    print('section journal identity', flush=True)
    res['journal_identity'] = timed(section_journal_identity, ctx)
    res['summary'] = summarise(res)
    res['runtime_s'] = round(time.time() - t0, 1)
    OUT.write_text(json.dumps(res, indent=1, default=_json_default) + '\n')
    print(json.dumps(res['summary'], indent=1))
    print('runtime', res['runtime_s'], 's')
    if not all(res['summary'].values()):
        sys.exit(1)


def _json_default(x):
    if isinstance(x, (np.integer,)):
        return int(x)
    if isinstance(x, (np.bool_,)):
        return bool(x)
    if isinstance(x, np.floating):
        return float(x)
    raise TypeError(type(x))


def _full(s):
    a, b = s.split('/')
    return a == b


def summarise(r):
    b, g, w, ind, ctl, rt, cp, fl, tw = (r['build'], r['generators'], r['witness'], r['independent_certification'],
                                         r['uncertain_and_rejection'], r['round_trips'], r['checkpoints'], r['filter_A2'],
                                         r['twists_menus_frames'])
    reg = b['regions']
    proj = g['projection_bijection_and_centres']
    nc, im, ji = r['negative_control'], r['input_map_A1'], r['journal_identity']
    return {
        'build_kplus_exact': b['kplus']['order'] == 7200 and b['kplus']['non_rotations'] == 0 and b['kplus']['pole_image_failures'] == 0,
        'build_kplus_orbits_are_unions_of_g_orbits': b['orbits']['g_class_in_one_kplus_orbit'] and b['orbits']['kplus_orbits'] == 36,
        'build_regions_certified': (reg['rep_vertices_violating_a_constraint'] == 0 and reg['rep_vertices_without_rank4_tight_set'] == 0
                                    and reg['pieces_with_transported_signature_mismatch'] == 0
                                    and reg['pieces_with_transported_host_mismatch'] == 0
                                    and _full(reg['rep_full_dimensional']) and _full(reg['rep_host_patches_match_retained'])
                                    and _full(reg['reps_touching_every_signature_cut_exactly'])),
        'build_frames_exact': b['frames']['frame_c_maps_pole_0_to_c'] and b['frames']['max_float_frame_deviation'] < 1e-12
                              and _full(b['frames']['slot_layout_frame_covariant']),
        '1_generators_combinatorial_all_1200': g['combinatorial_all_1200']['all_pass'],
        '1_generators_state_api_all_1200': all(_full(v) for v in g['state_api_all_1200'].values()),
        '1_retained_a4_twists_admissible_with_exact_contact': all(_full(v) for v in g['retained_a4_twists_from_solved'].values() if isinstance(v, str)),
        '2_witness_E2_reproduced': w['g_equals_witness_rotation'] and w['E2']['blocked'] == 54 and w['E2']['blocked_set_equals_witness'] and w['E2']['interacting_statuses_equal_witness'],
        '2_witness_E3_reproduced': w['E3']['blocked'] == 65 and w['E3']['blocked_set_equals_witness'] and w['E3']['interacting_statuses_equal_witness'],
        '2_witness_negative_control_and_E4': (w['negative_control']['status'] == 'blocked' and w['negative_control']['unchanged']
                                              and w['negative_control']['matches_witness_certificate'] and all(w['E4'].values())
                                              and w['E3_alternative_unblock']['admissible'] and w['E3_alternative_unblock']['c_status_after'] == 'admissible'),
        '2_independent_certification': all(_full(ind[n][k]) for n in ('E2', 'E3') for k in ('blocked_certificates_verified_literally', 'independent_exact_statuses_equal')),
        '2_float_cross_check': all(_full(ind[n]['float_cross_check_statuses_equal']) for n in ('E2', 'E3')),
        '2_signature_agrees_with_geometry': ind['signature_vs_geometry']['disagreements'] == 0 and ind['signature_vs_geometry']['straddles'] == 0,
        '3_uncertain_rejected': (ctl['unrepresentable_rotation']['status'] == 'uncertain' and ctl['budget_exhausted']['status'] == 'uncertain'
                                 and all(v.startswith('rejected') for v in ctl['invalid_inputs'].values())),
        '3_shallow_and_contact_controls': (ctl['E0_control_regions'] == {'shallow_below': {'exact': 'straddle', 'filtered': 'straddle'},
                                                                         'shallow_above': {'exact': 'straddle', 'filtered': 'straddle'},
                                                                         'contact_inside': {'exact': 'in', 'filtered': 'in'},
                                                                         'contact_outside': {'exact': 'out', 'filtered': 'out'}}
                                           and ctl['shallow_crossings']['blocked'] > 0 and ctl['shallow_crossings']['blocked_twist_rejected'] == 'blocked'),
        '4_rejection_without_change': (ctl['unrepresentable_rotation']['unchanged'] and ctl['budget_exhausted']['unchanged']
                                       and ctl['shallow_crossings']['unchanged'] and rt['all_rejections_unchanged']),
        '5_round_trips': (rt['count'] >= 20 and rt['all_length_at_least_8'] and rt['all_include_jumble_and_retained'] and rt['all_replay_equal']
                          and rt['all_inverse_sequences_exact'] and rt['all_undo_exact']),
        'checkpoints_section_5': (cp['retained_word']['checkpoint_from_journal'] and cp['retained_word']['export_equals_primitives_replay']
                                  and cp['merged_jumble_excursion']['checkpoint_from_journal'] and cp['merged_jumble_excursion']['export_equals_primitives_replay']
                                  and not cp['unwitnessed_lattice_state']['checkpoint_without_witness'] and cp['unwitnessed_lattice_state']['export_refused']
                                  and cp['unwitnessed_lattice_state']['checkpoint_with_supplied_witness'] and cp['unwitnessed_lattice_state']['wrong_witness_rejected']
                                  and not cp['off_lattice_state']['checkpoint'] and cp['witness_excursion_back_to_solved']['checkpoint']),
        'A2_filter_off_by_default_and_never_disagrees': (fl['default_off'] and fl['sign_agreement']['piece_evaluations_with_disagreement'] == 0
                                                         and fl['E2']['statuses_and_certificate_counts_equal'] and fl['E3']['statuses_and_certificate_counts_equal']),
        'A1_cayley_twists': _full(tw['plane_equals_cayley_and_inverse']) and tw['half_turn_request'] == 'accepted',
        'A4_menus': (tw['menus']['a4_menu_invariant'] and _full(tw['menus']['a4_menu_equals_A4_c_for_all_caps'])
                     and tw['menus']['a4_menu_state_rejects_jumble'] == 'invalid' and tw['menus']['rejection_unchanged']
                     and tw['menus']['a4_menu_state_accepts_retained'] == 'admissible' and tw['menus']['cayley_menu_invariant']
                     and _full(tw['menus']['cayley_menu_transported_members_recognised']) and tw['menus']['cayley_menu_rejects_other_angle']),
        'A3_off_lattice_frames': tw['off_lattice_sticker_frames']['stickers_checked'] == tw['off_lattice_sticker_frames']['frame_equals_pose_times_cell_frame'],
        '1_projection_bijection_and_centres': (proj['solved_bijection'] and proj['solved_orientations_in_a4']
                                               and _full(proj['generator_bijections']) and _full(proj['generator_orientations_in_a4'])
                                               and _full(proj['centre_pose_changes_with_fixed_labels'])
                                               and proj['centre_stickers_checked'] == 600 * 1201
                                               and g['state_api_all_1200']['labels_equal_primitives'] == '1200/1200'),
        '5_grouped_equals_per_piece': all(_full(row['statuses_equal']) and _full(row['admissible_inside_sets_equal'])
                                         and row['every_off_lattice_piece_evaluated'] and row['reference_has_no_superset_or_filter'] and row['no_uncertain']
                                         and len(row['grips']) >= (600 if name in ('E2', 'E3') else 60)
                                         for name, row in r['grouped_per_piece'].items() if isinstance(row, dict)),
        '7_global_rotation_control': (len(cp['global_rotation_control']) == 2
                                       and [row['fixes_pole_0'] for row in cp['global_rotation_control']] == [True, False]
                                       and all(row['lattice'] and row['digest_differs_from_solved'] and row['centres_in_other_chambers'] > 0
                                               and not row['checkpoint'] and not row['checkpoint_with_supplied_word'] and row['export_refused']
                                               for row in cp['global_rotation_control'])),
        '7_same_cap_excursion': all(cp['same_cap_excursion'][k] for k in ('applied', 'was_off_lattice', 'lattice', 'checkpoint',
                                                                          'nonidentity_retained_pose', 'export_equals_primitives_replay')),
        '8_menu_controls': (r['menu_controls']['A4']['size'] == 12 and r['menu_controls']['S4']['size'] == 24
                              and all(row['inverse_closed'] and row['a4_invariant'] and row['contains_a4'] and _full(row['all_transports_equal'])
                                      and row['record_round_trip'] for row in (r['menu_controls'][name] for name in ('A4', 'S4', 'NC')))),
        '9_negative_control': all(nc[k] for k in ('g_not_in_a4', 'g_squared_is_identity', 'g_fixes_pole_13', 'trace_is_four_thirds',
                                                  'all_twists_admissible', 'digests_pairwise_distinct', 'entry_height_grows', 'replay_equal')) and nc['rounds'] >= 12,
        'A1_input_map_and_half_turns': (tw['half_turn_request'] == 'accepted' and im['nonrepresentable_rejected_unchanged']
                                       and im['menu_request_never_approximated']
                                       and all(all(row[k] for k in ('global_minimum_equal', 'tie_order_equal', 'journal_records_bounds_and_distance',
                                                                      'exact_record_round_trip')) for row in im['exhaustive_cases'])),
        'A2_certificate_fields': all(row['float_decisions'] > 0 and row['all_evaluations_recorded']
                                     and all(_full(row[k]) for k in ('fields_complete', 'coverage_verified', 'accepted_signs_equal_exact',
                                                                     'float_enclosures_exclude_zero_with_exact_sign'))
                                     for row in (fl[name]['certificate_fields'] for name in ('E2', 'E3'))),
        'identity_in_journal': (ji['identity_fields_recorded'] and ji['model_hashes_match_assets']
                                and all(ji['mismatches_refused'].values()) and ji['matching_identities_replay'] and ji['legacy_document_replays']),
    }


if __name__ == '__main__':
    main()
