"""Export exact jumbling scenes for the J2 viewer (research/jumbling/viewer/).

Run from the repository root (about 2 to 3 minutes on four cores):

    python research/jumbling/viewer/export_scene.py
    python research/jumbling/viewer/export_scene.py --source sim-s4

Writes scene.json (header, poses, twists, grip surveys, certificates) and scene.bin (sticker
meshes and per-state pose indices) next to this file. The sim-s4 source writes scene-s4.json
and scene-s4.bin instead. Reads assets/model.npz read-only through research/jumbling/witness.py.

Exact and float parts:
- Exact (Q(sqrt 5)): piece regions, sticker vertex sets and their faces and edges (from exact
  tight constraint sets), every pose, every twist, every grip status and every straddling
  certificate. Legality is never decided here in floating point.
- Float: the exported coordinates (float32 of the exact vertices, divided by the facet distance
  |n|), pose and twist matrices (float64 of exact matrices), the twist-plane decomposition used
  for animation, nearest lattice poses (a float search over the 7,200 rotations of K+), the
  host-cell colouring, the cell cage and the outer-shape statistics.

The sources witness_source() and sim_s4_source() return the same exact-data dictionary.
"""
import argparse
import hashlib
import json
import math
import multiprocessing as mp
import pickle
import sys
import time
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
JUMBLING = HERE.parent
sys.path.insert(0, str(JUMBLING))
import witness as W  # noqa: E402
from exact import ONE, ZERO, Q5, affine_rank, dot, rank, transpose  # noqa: E402


# ---------------------------------------------------------------------------------------------
# exact per-piece geometry (runs in worker processes)

def _piece_geometry(args):
    """Exact region of one piece, its edges, and its stickers with ordered 2-faces.

    A sticker is the 3-polytope where the region meets one of its host facets: the region
    vertices lying exactly on that facet. Its 2-faces are the vertex sets that are also tight on
    one other constraint and have affine rank 2; they are ordered along the exact region edges.
    """
    p, cand_poles, cand_facets = args
    cons, tags = W.piece_constraints(p, cand_poles, cand_facets)
    verts, _ = W.double_description(cons)
    tight = [frozenset(j for j, (a, b) in enumerate(cons) if (dot(a, v) - b).is_zero()) for v in verts]
    nv = len(verts)
    edges = []
    for i in range(nv):
        for k in range(i + 1, nv):
            com = tight[i] & tight[k]
            if len(com) < 3:
                continue
            if any(m != i and m != k and com <= tight[m] for m in range(nv)):
                continue
            if rank([cons[j][0] for j in com]) == 3:
                edges.append((i, k))
    stickers = []
    for e in W.hosts(p):
        kidx = tags.index(('facet', e))
        s = [i for i in range(nv) if kidx in tight[i]]
        sset = set(s)
        if affine_rank([verts[i] for i in s]) != 3:
            raise AssertionError(f'piece {p} host {e}: sticker is not three-dimensional')
        sedges = [(i, k) for i, k in edges if i in sset and k in sset]
        faces = []
        seen = set()
        for j in range(len(cons)):
            if j == kidx:
                continue
            f = frozenset(i for i in s if j in tight[i])
            if len(f) < 3 or f in seen:
                continue
            seen.add(f)
            if affine_rank([verts[i] for i in f]) == 2:
                faces.append(f)
        polys = []
        for f in faces:
            adj = {i: [] for i in f}
            for i, k in sedges:
                if i in f and k in f:
                    adj[i].append(k)
                    adj[k].append(i)
            if any(len(x) != 2 for x in adj.values()):
                raise AssertionError(f'piece {p} host {e}: face is not a simple cycle')
            start = min(f)
            cyc, prev, cur = [start], None, start
            while True:
                a, b = adj[cur]
                nxt = a if a != prev else b
                if nxt == start:
                    break
                cyc.append(nxt)
                prev, cur = cur, nxt
            if len(cyc) != len(f):
                raise AssertionError(f'piece {p} host {e}: face cycle misses vertices')
            polys.append(cyc)
        if len(s) - len(sedges) + len(faces) != 2:
            raise AssertionError(f'piece {p} host {e}: Euler characteristic is not 2')
        stickers.append((e, polys))
    return p, verts, edges, stickers


# ---------------------------------------------------------------------------------------------
# the exact source: research/jumbling/witness.py

def _survey(cfg, group_cap, caps):
    """Status of all 600 grips with every straddling certificate (exact, contract section 3)."""
    bounds = {}
    for p in cfg.sigs:
        r = cfg.pose.get(p)
        if r is None or cfg.lattice[p]:
            continue
        key = tuple(tuple(x) for x in r)
        home = group_cap[key]
        if key not in bounds:
            bounds[key] = [W.matvec(r, v) for v in caps[home]['verts']]
    rows = []
    for e in range(600):
        _, _, st = W.classify_grouped(cfg, e, bounds)
        rows.append({'status': 'blocked' if st else 'admissible', 'straddling': st})
    return rows


def _pose_groups_by_cap(cfg, c, d):
    """Each off-lattice pose group must lie in one cap, whose region then bounds the group."""
    out = {}
    for p, sig in cfg.sigs.items():
        r = cfg.pose.get(p)
        if r is None or cfg.lattice[p]:
            continue
        key = tuple(tuple(x) for x in r)
        caps_of = {x for x in (c, d) if x in sig}
        out.setdefault(key, set(caps_of))
        out[key] &= caps_of
    group_cap = {}
    for key, common in out.items():
        if not common:
            raise AssertionError('an off-lattice pose group spans both caps; no cap bound applies')
        group_cap[key] = c if c in common else d
    return group_cap


def witness_source(workers=4):
    """Exact data for the witness sequence solved -> (c, g) -> (d, T_d) -> (d, T_d^-1) -> (c, g^-1).

    Returns a plain dictionary; every matrix and vertex is exact (lists of Q5):
      alpha, c, d, norm2 (|n|^2, exact), normals (600 x 4 exact poles),
      pieces: {piece: {'signature', 'hosts', 'verts', 'edges', 'stickers': [(host, polygons)]}},
      states: [{'label', 'pose': {piece: matrix}, 'lattice': {piece: bool}, 'survey': [...]}],
      twists: [{'label', 'grip', 'matrix', 'retained', 'from', 'to', 'moved': [pieces]}],
      attempts: [{'state', 'grip', 'applied', 'unchanged', 'certificate'}],
      checks: {name: bool or value}
    """
    t0 = time.time()
    c = 0
    d = sorted(range(600), key=lambda e: -float(W.NF[e] @ W.NF[c]))[1]
    caps = {}
    for x in (c, d):
        verts = W.cap_region(x)
        near = [e for e in range(600) if W.NF[e] @ W.NF[x] > 0.6 * (W.NF[x] @ W.NF[x])]
        out = W.implied(verts)
        caps[x] = {'verts': verts, 'facets': near, 'poles': [e for e in range(600) if e not in out and e != x]}
    in_play = [p for p in range(len(W.MO) - 1) if c in W.signature(p) or d in W.signature(p)]
    sigs = {p: W.signature(p) for p in in_play}
    jobs = []
    for p in in_play:
        x = c if c in sigs[p] else d
        jobs.append((p, caps[x]['poles'], caps[x]['facets']))
    print(f'building {len(jobs)} exact regions on {workers} workers', flush=True)
    ctx = mp.get_context('fork') if 'fork' in mp.get_all_start_methods() else mp.get_context()
    with ctx.Pool(workers) as pool:
        built = pool.map(_piece_geometry, jobs, chunksize=16)
    print(f'regions done in {time.time() - t0:.0f} s', flush=True)
    pieces = {p: {'signature': sigs[p], 'hosts': W.hosts(p), 'verts': v, 'edges': e, 'stickers': s}
              for p, v, e, s in built}
    regions = {p: pieces[p]['verts'] for p in in_play}

    cfg = W.Config(regions, sigs)
    g, angle, s_par = W.witness_rotation(c, d)
    t_d = next(a for a in W.a4(d) if sum((a[i][i] for i in range(4)), ZERO) == ONE and W.pole_perm(a)[c] != c)
    seq = [
        ('(c, g)', c, g, False),
        ('(d, T_d)', d, t_d, True),
        ('(d, T_d^-1)', d, transpose(t_d), True),
        ('(c, g^-1)', c, transpose(g), False),
    ]
    state_labels = ['solved', 'after (c, g)', 'after (c, g), (d, T_d)', 'after (d, T_d^-1)', 'after (c, g^-1): solved']

    def capture(label):
        return {'label': label, 'pose': dict(cfg.pose), 'lattice': dict(cfg.lattice), 'snapshot': cfg.snapshot()}

    states = [capture(state_labels[0])]
    twists = []
    for k, (label, grip, r, retained) in enumerate(seq):
        before = dict(cfg.pose)
        ok, info = cfg.apply(grip, r, retained=retained)
        if not ok:
            raise AssertionError(f'twist {label} was not admissible: {info}')
        moved = [p for p in in_play if cfg.pose.get(p) != before.get(p)]
        twists.append({'label': label, 'grip': grip, 'matrix': r, 'retained': retained, 'from': k, 'to': k + 1,
                       'moved': moved})
        states.append(capture(state_labels[k + 1]))
    checks = {
        'state_3_equals_state_1_exactly': states[3]['snapshot'] == states[1]['snapshot'],
        'state_4_is_solved_exactly': not states[4]['pose'],
        'g_angle_deg': round(angle, 6),
        'g_parameter_s': f'{s_par.a}/{s_par.d}',
    }
    if not (checks['state_3_equals_state_1_exactly'] and checks['state_4_is_solved_exactly']):
        raise AssertionError('reverse twists did not restore the configurations exactly')

    # grip surveys at every exact state (states 3 and 4 equal states 1 and 0 exactly)
    work = W.Config(regions, sigs)
    surveys = {}
    for k in (0, 1, 2):
        work.pose, work.lattice = dict(states[k]['pose']), dict(states[k]['lattice'])
        surveys[k] = _survey(work, _pose_groups_by_cap(work, c, d), caps)
        print(f'survey of state {k}: {sum(r["status"] == "blocked" for r in surveys[k])} blocked '
              f'({time.time() - t0:.0f} s)', flush=True)
    surveys[3], surveys[4] = surveys[1], surveys[0]
    for k, st in enumerate(states):
        st['survey'] = surveys[k]

    # negative control at state 1, as in witness.py: a certified blocked grip is rejected
    work.pose, work.lattice = dict(states[1]['pose']), dict(states[1]['lattice'])
    blocked1 = [e for e in range(600) if surveys[1][e]['status'] == 'blocked']
    e_bad = blocked1[0]
    before = work.snapshot()
    ok, cert = work.apply(e_bad, W.a4(e_bad)[1], retained=True)
    attempts = [{'state': 1, 'grip': e_bad, 'twist': f'({e_bad}, a) with a in A4_{e_bad}', 'applied': ok,
                 'unchanged': work.snapshot() == before, 'certificate': cert}]

    # agreement with the recorded witness results
    rec = json.loads((JUMBLING / 'witness-results.json').read_text())
    checks['blocked_after_g_matches_witness_results'] = blocked1 == rec['E2']['admissible_after_g']['blocked']
    checks['blocked_after_g_then_t_matches_witness_results'] = (
        [e for e in range(600) if surveys[2][e]['status'] == 'blocked'] == rec['E3']['blocked_after_g_then_t'])
    checks['negative_control_matches_witness_results'] = (e_bad == rec['E3_negative_control']['pole'] and not ok)
    for st in states:
        del st['snapshot']
    return {'alpha': W.ALPHA, 'c': c, 'd': d, 'norm2': W.NN[c], 'normals': W.N, 'pieces': pieces,
            'states': states, 'twists': twists, 'attempts': attempts, 'checks': checks,
            'source': 'research/jumbling/witness.py (exact witness E2-E4)'}


def sim_s4_source(workers=4):
    """J1's certified S4 script: q at 0, a at d, a blocked attempt, a^-1, q^-1.

    J1 owns every state, twist and survey. Geometry uses the existing exporter builder,
    checked against J1 as exact vertex sets before remapping J1's certificate indices.
    """
    import sim

    t0 = time.time()
    ctx = sim.get_context()
    data, reg = ctx.data, ctx.regions
    c = 0
    menu = sim.TwistMenu.s4(ctx)
    cfg = sim.State(ctx, menu=menu)
    q = sim.cayley(ctx, c, [ONE, ZERO, ZERO])
    qinv = q.inverse()
    cap0 = reg.cap_vertices
    candidates = [e for e in range(600) if e != c
                  and tuple(W.matvec(qinv.matrix, data.N[e])) in data.POLE
                  and any(data.h(e, v).sign() > 0 for v in cap0)]
    if not candidates:
        raise AssertionError('S4 quarter-turn realigns no grip whose cap meets cap 0')
    d = min(candidates, key=lambda e: (-dot(data.N[c], data.N[e]), e))
    a_index, a = next((i, tw) for i in range(12)
                      if sum(((tw := sim.a4_element(ctx, d, i)).matrix[j][j] for j in range(4)), ZERO) == ONE)
    seq = [('(0, q)', q), ('(d, a)', a), ('(d, a^-1)', a.inverse()), ('(0, q^-1)', qinv)]
    labels = ['solved', 'after (0, q)', 'after (0, q), (d, a)',
              'after (d, a^-1)', 'after (0, q^-1): solved']
    applied_inside, twists, attempts = [], [], []

    def capture(label):
        survey = cfg.survey()
        if sorted(survey) != list(range(600)) or any(r['status'] not in ('admissible', 'blocked')
                                                   for r in survey.values()):
            raise AssertionError('J1 survey is incomplete or contains an uncertified grip')
        for e, row in survey.items():
            if row['status'] == 'blocked' and row.get('certificate', {}).get('grip') != e:
                raise AssertionError(f'J1 blocked grip {e} has no straddle certificate')
        changed = np.flatnonzero(cfg.pose_id).tolist()
        lattice = cfg.lattice_flags()
        result = {'label': label, 'pose': {p: cfg.pose(p) for p in changed},
                  'lattice': {p: bool(lattice[p]) for p in changed}, 'digest': cfg.digest(),
                  'survey': [dict(survey[e], straddling=([survey[e]['certificate']]
                                                       if 'certificate' in survey[e] else []))
                             for e in range(600)]}
        print(f'survey of state {len(twists)}: {sum(r["status"] == "blocked" for r in survey.values())} '
              f'blocked ({time.time() - t0:.0f} s)', flush=True)
        return result

    states = [capture(labels[0])]
    for k, (label, tw) in enumerate(seq):
        cl = cfg.classify(tw.grip)
        if cl.status != 'admissible' or not menu.contains(tw):
            raise AssertionError(f'twist {label} is not a certified S4 move: {cl.status}')
        inside = cl.inside.tolist()
        outcome = cfg.apply(tw)
        if not outcome.applied or outcome.moved != len(inside):
            raise AssertionError(f'J1 rejected twist {label}: {outcome}')
        applied_inside.append(set(inside))
        twists.append({'label': label, 'grip': tw.grip, 'matrix': tw.matrix, 'retained': tw.retained,
                       'from': k, 'to': k + 1, 'moved': inside})
        states.append(capture(labels[k + 1]))
        if k == 1:
            bad = next(e for e, row in enumerate(states[-1]['survey']) if row['status'] == 'blocked')
            rejected = sim.a4_element(ctx, bad, 1)
            before = cfg.snapshot()
            out = cfg.apply(rejected)
            unchanged = cfg.snapshot() == before
            if out.status != 'blocked' or out.applied or not unchanged or out.straddle is None:
                raise AssertionError('J1 negative control was not rejected without changing the full state')
            attempts.append({'state': 2, 'grip': bad, 'twist': f'({bad}, a4[1])', 'applied': out.applied,
                             'unchanged': unchanged, 'certificate': out.straddle,
                             'outcome': out.as_dict(), 'exact_twist': rejected.record()})

    in_play = sorted(set().union(*applied_inside))
    sigs = {p: data.signature(p) for p in in_play}
    if any(c not in sig and d not in sig for sig in sigs.values()):
        raise AssertionError('the S4 script moves a piece outside the two home caps')
    caps = {}
    for x in (c, d):
        f = ctx.kplus.matrix(int(ctx.kplus.frame_idx[x]))
        verts = [W.matvec(f, v) for v in cap0]
        out = W.implied(verts)
        caps[x] = {'poles': [e for e in range(600) if e != x and e not in out],
                   'facets': [e for e in range(600) if dot(data.N[e], data.N[x]) > Q5(3, 0, 5) * data.NN]}
    jobs = [(p, caps[c if c in sigs[p] else d]['poles'], caps[c if c in sigs[p] else d]['facets'])
            for p in in_play]
    print(f'building {len(jobs)} exact regions on {workers} workers', flush=True)
    pool_ctx = mp.get_context('fork') if 'fork' in mp.get_all_start_methods() else mp.get_context()
    with pool_ctx.Pool(workers) as pool:
        built = pool.map(_piece_geometry, jobs, chunksize=16)
    pieces, vertex_maps = {}, {}
    for p, verts, edges, stickers in built:
        home = reg.vertices(p)
        lookup = {tuple(v): i for i, v in enumerate(verts)}
        if set(lookup) != {tuple(v) for v in home}:
            raise AssertionError(f'piece {p}: exporter vertex set differs from J1')
        vertex_maps[p] = [lookup[tuple(v)] for v in home]
        pieces[p] = {'signature': sigs[p], 'hosts': data.hosts(p), 'verts': verts,
                     'edges': edges, 'stickers': stickers}
    print(f'regions and exact J1 comparisons done in {time.time() - t0:.0f} s', flush=True)

    def mapped_certificate(cert, state):
        p = cert['piece']
        if p not in pieces:
            raise AssertionError(f'J1 certificate piece {p} is outside the applied inside sets')
        mapped = dict(cert, vertex_below=vertex_maps[p][cert['vertex_below']],
                      vertex_above=vertex_maps[p][cert['vertex_above']],
                      j1_vertex_below=cert['vertex_below'], j1_vertex_above=cert['vertex_above'],
                      h_below_exact=cert['h_below'], h_above_exact=cert['h_above'],
                      h_below=cert['h_below_float'], h_above=cert['h_above_float'])
        for side, sign in (('below', -1), ('above', 1)):
            point = W.matvec(state['pose'][p], pieces[p]['verts'][mapped[f'vertex_{side}']])
            exact_point = [[x.a, x.b, x.d] for x in point]
            h = data.h(cert['grip'], point)
            if (exact_point != cert[f'point_{side}'] or [h.a, h.b, h.d] != cert[f'h_{side}']
                    or h.sign() != sign or not cert['points_checked_against_all_constraints']):
                raise AssertionError(f'piece {p}: mapped certificate differs from J1')
        return mapped

    for state in states:
        for row in state['survey']:
            row['straddling'] = [mapped_certificate(cert, state) for cert in row['straddling']]
    for att in attempts:
        att['certificate'] = mapped_certificate(att['certificate'], states[att['state']])
    journal = json.loads(cfg.journal_json())
    checks = {
        's4_menu_has_24_elements': len(menu) == 24,
        'every_applied_twist_is_in_s4': all(menu.contains(tw) for _, tw in seq),
        'every_applied_twist_is_certified_admissible': len(twists) == 4,
        'd_is_exactly_realigned': tuple(W.matvec(qinv.matrix, data.N[d])) in data.POLE,
        'caps_meet_exactly': any(data.h(d, v).sign() > 0 for v in cap0),
        'surveys_are_complete_and_certified': all(len(st['survey']) == 600 for st in states),
        'negative_control_is_certified_blocked': attempts[0]['outcome']['status'] == 'blocked',
        'negative_control_leaves_full_state_unchanged': attempts[0]['unchanged'],
        'state_3_digest_equals_state_1': states[3]['digest'] == states[1]['digest'],
        'final_digest_equals_solved': states[-1]['digest'] == states[0]['digest'],
        'exported_pieces_equal_union_of_inside_sets': set(pieces) == set().union(*applied_inside),
        'exported_vertices_equal_j1_exactly': len(vertex_maps) == len(pieces),
        'certificate_vertices_and_signs_match_j1_exactly': True,  # checked above for every record
        'journal_has_model_menu_contract_identities': (journal['model_identity'] == data.identity
                                                      and journal['menu_identity'] == menu.identity
                                                      and bool(journal['contract_revision'])),
        'journal_replay_matches_final_digest': sim.State.replay(journal, ctx, menu=menu).digest() == cfg.digest(),
    }
    if not all(value is True for value in checks.values()):
        raise AssertionError(f'S4 source checks failed: {checks}')
    return {'alpha': data.ALPHA, 'c': c, 'd': d, 'norm2': data.NN, 'normals': data.N, 'pieces': pieces,
            'states': states, 'twists': twists, 'attempts': attempts, 'checks': checks,
            'source': 'research/jumbling/sim/ (J1 exact S4 sequence)', 'source_key': 'sim-s4',
            'journal': journal, 'menu': menu.record(),
            'script': {'q_omega': [[1, 0, 1], [0, 0, 1], [0, 0, 1]], 'd': d, 'a4_index': a_index,
                       'realigned_from_pole': data.POLE[tuple(W.matvec(qinv.matrix, data.N[d]))],
                       'applied': [tw.record() for _, tw in seq], 'blocked_grip': attempts[0]['grip']}}


SOURCES = {'witness': witness_source, 'sim-s4': sim_s4_source}


# ---------------------------------------------------------------------------------------------
# float helpers (display only)

def to_float(m):
    return np.array([[float(x) for x in row] for row in m])


def simple_rotation_plane(r):
    """For a rotation fixing a plane pointwise: (u, v, theta) with
    R(phi) = I + (cos phi - 1)(u u^T + v v^T) + sin phi (v u^T - u v^T) and R(theta) = r."""
    tr = np.trace(r)
    cos_t = float(np.clip((tr - 2) / 2, -1, 1))
    theta = math.acos(cos_t)
    sym = (r + r.T - 2 * cos_t * np.eye(4)) / (2 * (1 - cos_t))  # projector onto the fixed plane
    q = np.eye(4) - sym
    k = int(np.argmax(np.linalg.norm(q, axis=0)))
    u = q[:, k] / np.linalg.norm(q[:, k])
    jm = (r - r.T) / (2 * math.sin(theta))
    v = jm @ u
    v /= np.linalg.norm(v)
    rec = rotation_family(u, v, theta)
    err = float(np.abs(rec - r).max())
    if err > 1e-9:
        raise AssertionError(f'not a simple rotation (reconstruction error {err})')
    return u, v, theta, err


def rotation_family(u, v, phi):
    return (np.eye(4) + (math.cos(phi) - 1) * (np.outer(u, u) + np.outer(v, v))
            + math.sin(phi) * (np.outer(v, u) - np.outer(u, v)))


def k_plus(normals):
    """The 7,200 rotations of the 600-cell (float), by closure of a few retained generators."""
    nf = normals
    rp = W.M['rotperms']
    gens = []
    for k in (0, 1, 26, 27, 200, 201, 401, 777):
        perm = rp[k].astype(int)
        a = nf.T @ nf
        b = nf[perm].T @ nf
        gens.append(b @ np.linalg.inv(a))
    seen = {}
    frontier = [np.eye(4)]
    seen[tuple(np.round(np.eye(4), 6).ravel())] = np.eye(4)
    while frontier:
        nxt = []
        for m in frontier:
            for gmat in gens:
                x = gmat @ m
                key = tuple(np.round(x, 6).ravel())
                if key not in seen:
                    seen[key] = x
                    nxt.append(x)
        frontier = nxt
    group = np.array(list(seen.values()))
    if len(group) != 7200:
        raise AssertionError(f'K+ closure has {len(group)} elements')
    return group


def residual_angles(m):
    """The two rotation angles (degrees) of a 4D rotation, larger first."""
    ev = np.linalg.eigvals(m)
    ang = sorted((abs(math.degrees(math.atan2(z.imag, z.real))) for z in ev), reverse=True)
    return [round(ang[0], 4), round(ang[2], 4)]


def greedy_colouring(units, ids, min_deg=26.0):
    """Colour classes for host cells: cells closer than min_deg (face and edge neighbours) differ."""
    order = sorted(ids)
    colour = {}
    for e in order:
        used = {colour[f] for f in colour if math.degrees(math.acos(min(1.0, float(units[e] @ units[f])))) < min_deg}
        k = 0
        while k in used:
            k += 1
        colour[e] = k
    return colour


def tetra_vertices(units, e):
    """Vertices of the tetrahedral cell e (normalised coordinates, facet distance 1)."""
    nbr = sorted(range(600), key=lambda f: -float(units[f] @ units[e]))[1:5]
    out = []
    for skip in range(4):
        rows = [units[e]] + [units[f] for i, f in enumerate(nbr) if i != skip]
        out.append(np.linalg.solve(np.array(rows), np.ones(4)))
    return out


# ---------------------------------------------------------------------------------------------
# export

def export(src, out_dir, stem='scene'):
    t0 = time.time()
    c, d = src['c'], src['d']
    nf = np.array([[float(x) for x in n] for n in src['normals']])
    norm = math.sqrt(float(src['norm2']))
    units = nf / norm
    piece_ids = sorted(src['pieces'])
    index = {p: i for i, p in enumerate(piece_ids)}
    npieces = len(piece_ids)

    # geometry arrays
    vert_start, verts, edge_start, edges = [0], [], [0], []
    st_piece, st_host, tri_start, tris = [], [], [0], []
    piece_flags = []
    for p in piece_ids:
        rec = src['pieces'][p]
        vf = np.array([[float(x) for x in v] for v in rec['verts']]) / norm
        if len(vf) > 255:
            raise AssertionError('piece has more than 255 vertices')
        verts.append(vf)
        vert_start.append(vert_start[-1] + len(vf))
        edges += [x for e in rec['edges'] for x in e]
        edge_start.append(edge_start[-1] + len(rec['edges']))
        sig = rec['signature']
        piece_flags.append((1 if c in sig else 0) | (2 if d in sig else 0))
        for host, polys in rec['stickers']:
            st_piece.append(index[p])
            st_host.append(host)
            for cyc in polys:
                for k in range(1, len(cyc) - 1):
                    tris += [cyc[0], cyc[k], cyc[k + 1]]
            tri_start.append(len(tris) // 3)
    verts = np.vstack(verts).astype(np.float32)

    # poses: one table of distinct exact poses across all states
    pose_keys, pose_mats = {}, []
    ident = tuple(tuple(x) for x in [[ONE if i == j else ZERO for j in range(4)] for i in range(4)])

    def pose_index(m):
        key = ident if m is None else tuple(tuple(x) for x in m)
        if key not in pose_keys:
            pose_keys[key] = len(pose_mats)
            pose_mats.append(key)
        return pose_keys[key]

    pose_index(None)
    state_pose = np.zeros((len(src['states']), npieces), dtype=np.uint8)
    state_lat = np.ones((len(src['states']), npieces), dtype=np.uint8)
    for k, st in enumerate(src['states']):
        for p in piece_ids:
            state_pose[k, index[p]] = pose_index(st['pose'].get(p))
            if p in st['pose'] and not st['lattice'][p]:
                state_lat[k, index[p]] = 0
    group = k_plus(nf)
    poses_json = []
    for key in pose_mats:
        m = to_float(key)
        dist = np.linalg.norm(group - m[None], axis=(1, 2))
        j = int(np.argmin(dist))
        lat = group[j]
        poses_json.append({'m': [round(x, 15) for x in m.ravel().tolist()],
                           'nearest_lattice': [round(x, 12) for x in lat.ravel().tolist()],
                           'residual_deg': residual_angles(lat.T @ m),
                           'in_k_plus': bool(dist[j] < 1e-9)})

    # twists
    twists_json = []
    for tw in src['twists']:
        m = to_float(tw['matrix'])
        u, v, theta, err = simple_rotation_plane(m)
        n = units[tw['grip']]
        twists_json.append({
            'label': tw['label'], 'grip': tw['grip'], 'retained': tw['retained'], 'from': tw['from'], 'to': tw['to'],
            'kind': 'retained (A4)' if tw['retained'] else 'jumble (not in A4)',
            'angle_deg': round(math.degrees(theta), 6), 'theta': theta,
            'u': u.tolist(), 'v': v.tolist(), 'family_error_at_end': err,
            'grip_fixed': float(np.abs(m @ n - n).max()) < 1e-12,
            'moved': len(tw['moved']),
            'exact': [[[x.a, x.b, x.d] for x in row] for row in tw['matrix']],
        })

    # grip surveys and certificates
    patch_grips = sorted(set().union(*(src['pieces'][p]['signature'] for p in piece_ids)))
    states_json = []
    cert = {'grip': [], 'piece': [], 'vb': [], 'va': [], 'hb': [], 'ha': []}
    cert_start = [0]
    for k, st in enumerate(src['states']):
        status = ''.join('b' if r['status'] == 'blocked' else 'a' for r in st['survey'])
        for e, r in enumerate(st['survey']):
            for x in r['straddling']:
                cert['grip'].append(e)
                cert['piece'].append(index[x['piece']])
                cert['vb'].append(x['vertex_below'])
                cert['va'].append(x['vertex_above'])
                cert['hb'].append(x['h_below'])
                cert['ha'].append(x['h_above'])
        cert_start.append(len(cert['grip']))
        poses_here = state_pose[k]
        counts = {str(i): int((poses_here == i).sum()) for i in sorted(set(poses_here.tolist()))}
        states_json.append({
            'label': st['label'], 'grip_status': status,
            'admissible': status.count('a'), 'blocked': status.count('b'),
            'blocked_grips': [e for e, ch in enumerate(status) if ch == 'b'],
            'off_lattice_pieces': int((state_lat[k] == 0).sum()),
            'pieces_per_pose': counts, 'straddling_records': cert_start[-1] - cert_start[-2],
        })
        if 'digest' in st:
            states_json[-1].update(digest=st['digest'], survey=st['survey'])

    # outer shape (float statistics of posed sticker vertices against the 600 facets)
    st_vert_ids = []
    for s, pi in enumerate(st_piece):
        a, b = tri_start[s], tri_start[s + 1]
        ids = sorted(set(tris[3 * a:3 * b]))
        st_vert_ids.append([vert_start[pi] + i for i in ids])
    pose_f = np.array([to_float(k) for k in pose_mats])
    for k, sj in enumerate(states_json):
        prot = []
        moved_out = 0
        for s, pi in enumerate(st_piece):
            m = pose_f[state_pose[k, pi]]
            pv = verts[st_vert_ids[s]].astype(np.float64) @ m.T
            hv = (pv @ units.T).max(axis=1) - 1
            prot.append(hv.max())
            moved_out += int((hv > 1e-7).any())
        sj['outer_shape_float'] = {'max_protrusion_beyond_polytope': round(float(max(prot)), 6),
                                   'stickers_protruding': moved_out,
                                   'note': 'float of exact vertices; positive values lie outside the 600-cell'}

    # host cells: colour classes and cage
    hosts = sorted(set(st_host))
    colour = greedy_colouring(units, hosts)
    cells = []
    for e in hosts:
        cells.append({'id': e, 'colour': colour[e], 'centre': units[e].tolist(),
                      'tetra': [x.tolist() for x in tetra_vertices(units, e)]})

    arrays = {
        'piece_ids': np.array(piece_ids, dtype=np.int32),
        'piece_flags': np.array(piece_flags, dtype=np.uint8),
        'vert_start': np.array(vert_start, dtype=np.uint32),
        'verts': verts.ravel(),
        'edge_start': np.array(edge_start, dtype=np.uint32),
        'edges': np.array(edges, dtype=np.uint8),
        'sticker_piece': np.array(st_piece, dtype=np.uint16),
        'sticker_host': np.array(st_host, dtype=np.uint16),
        'tri_start': np.array(tri_start, dtype=np.uint32),
        'tris': np.array(tris, dtype=np.uint8),
        'state_pose': state_pose.ravel(),
        'state_lattice': state_lat.ravel(),
        # straddling certificates of every blocked grip, state by state (cert_start: per state)
        'cert_start': np.array(cert_start, dtype=np.uint32),
        'cert_grip': np.array(cert['grip'], dtype=np.uint16),
        'cert_piece': np.array(cert['piece'], dtype=np.uint16),
        'cert_vertex_below': np.array(cert['vb'], dtype=np.uint8),
        'cert_vertex_above': np.array(cert['va'], dtype=np.uint8),
        'cert_h_below': np.array(cert['hb'], dtype=np.float32),
        'cert_h_above': np.array(cert['ha'], dtype=np.float32),
    }
    blob, layout = bytearray(), {}
    for name, arr in arrays.items():
        while len(blob) % 8:
            blob.append(0)
        layout[name] = {'dtype': str(arr.dtype), 'offset': len(blob), 'length': int(arr.size)}
        blob += arr.tobytes()
    bin_file, json_file = f'{stem}.bin', f'{stem}.json'
    (out_dir / bin_file).write_bytes(bytes(blob))

    header = {
        'format': 'magic600-jumbling-scene/1',
        'source': src['source'],
        'evidence': 'source and synthetic geometry; legality exact in Q(sqrt 5); coordinates and motion float',
        'alpha': '121/125', 'c': c, 'd': d,
        'units': 'coordinates divided by the facet distance |n|; facets at distance 1, cuts at 121/125',
        'counts': {'pieces': npieces, 'stickers': len(st_piece), 'vertices': int(len(verts)),
                   'triangles': len(tris) // 3, 'edges': len(edges) // 2, 'host_cells': len(hosts),
                   'patch_grips': len(patch_grips), 'poses': len(pose_mats)},
        'bin': {'file': bin_file, 'bytes': len(blob), 'sha256': hashlib.sha256(bytes(blob)).hexdigest(),
                'arrays': layout},
        'grips': {'patch': patch_grips, 'units': {str(e): units[e].tolist() for e in patch_grips}},
        'cells': cells,
        'poses': poses_json,
        'twists': twists_json,
        'states': states_json,
        'attempts': [{'state': a['state'], 'grip': a['grip'], 'twist': a['twist'], 'applied': a['applied'],
                      'configuration_unchanged': a['unchanged'],
                      'certificate': {'piece': index[a['certificate']['piece']],
                                      'piece_id': int(a['certificate']['piece']),
                                      'vertex_below': a['certificate']['vertex_below'],
                                      'vertex_above': a['certificate']['vertex_above'],
                                      'h_below': a['certificate']['h_below'],
                                      'h_above': a['certificate']['h_above']}}
                     for a in src['attempts']],
        'checks': src['checks'],
    }
    if 'journal' in src:
        for key in ('journal', 'menu', 'script'):
            header[key] = src[key]
        for dest, att in zip(header['attempts'], src['attempts']):
            dest['certificate'].update(att['certificate'], piece=index[att['certificate']['piece']],
                                       piece_id=att['certificate']['piece'])
            dest.update(outcome=att['outcome'], exact_twist=att['exact_twist'])
    (out_dir / json_file).write_text(json.dumps(header, separators=(',', ':')) + '\n')
    print(f'export done in {time.time() - t0:.0f} s: {json_file} {(out_dir / json_file).stat().st_size} B, '
          f'{bin_file} {len(blob)} B', flush=True)
    return header


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument('--source', choices=sorted(SOURCES), default='witness')
    ap.add_argument('--workers', type=int, default=4)
    ap.add_argument('--cache', type=Path,
                    help='development: pickle of the exact source, reused when it exists (keep it outside the repository)')
    args = ap.parse_args()
    if args.cache and args.cache.exists():
        src = pickle.loads(args.cache.read_bytes())
        if src.get('source_key', 'witness') != args.source:
            ap.error('the cache belongs to a different source; use a separate cache for each scene')
    else:
        src = SOURCES[args.source](workers=args.workers)
        if args.cache:
            args.cache.write_bytes(pickle.dumps(src))
    header = export(src, HERE, stem='scene-s4' if args.source == 'sim-s4' else 'scene')
    print(json.dumps({'counts': header['counts'], 'checks': header['checks'],
                      'states': [{k: s[k] for k in ('label', 'admissible', 'blocked', 'off_lattice_pieces',
                                                    'pieces_per_pose', 'outer_shape_float')}
                                 for s in header['states']]}, indent=1))


if __name__ == '__main__':
    main()
