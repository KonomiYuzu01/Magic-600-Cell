"""W-J fixtures: scripted scrambles of candidate menus, replayed by the J1 reference engine.

A fixture `wj-<menu>.json` holds the exact menu record, the J1 journal with its model identity,
menu identity and contract revision, and references at three stages: start (solved), mid (after
half the records) and end. It also describes the last swept twist of the script at its midpoint. Every reference is
recomputed from the exact J1 state; floats appear only in the renderer arrays and sampled
positions, converted from exact values at the end.

    python research/jumbling/fixtures/wj.py build <menu> <source.json>   # S4, I_a or I_b
    python research/jumbling/fixtures/wj.py check [<menu> ...]          # replay and compare
    python research/jumbling/fixtures/wj.py export <menu> <stage> <dir> # renderer arrays

Stages: start, mid, end, and sweep-before and sweep-after around the swept twist.

The source is a walk output of `research/jumbling/theory/walk.py`. Only its journal records are
used; J1 replays them under the exact menu and must apply each one as recorded.

Evidence kind: source and synthetic geometry. Timings are cloud timings of research code, not
performance evidence.
"""
import argparse
import hashlib
import json
import math
import random
import sys
import time
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
JUMBLING = HERE.parent
sys.path.insert(0, str(JUMBLING))
import sim  # noqa: E402
from exact import Q5, ZERO, matvec  # noqa: E402
from sim.kernel import q5_float  # noqa: E402
from sim.twists import TwistMenu, Twist  # noqa: E402

FORMAT = 'magic600-jumbling-wj/1'
MENUS = ('S4', 'I_a', 'I_b')
STAGES = ('start', 'mid', 'end', 'sweep-before', 'sweep-after')
SAMPLES = 48          # sampled pieces with posed centroids at every stage
SWEEP_SAMPLES = 16    # sampled moving pieces at the swept-twist midpoint
SEED = 20261009


def make_menu(ctx, name):
    if name == 'S4':
        return TwistMenu.s4(ctx)
    sys.path.insert(0, str(JUMBLING / 'theory'))
    from groups import build, lift
    ex = build()
    return TwistMenu(ctx, name, [(f'{name}[{i}]', lift(m)) for i, m in enumerate(ex[name])], close=True)


def to_float(x):
    v = q5_float(x)
    if v is None:
        raise ValueError('non-finite display value')
    return v


def pose_float(st, pid):
    return np.array([[to_float(x) for x in row] for row in st._pose_matrix(pid)], np.float64)


def height(st):
    """Largest |numerator| or denominator over the entries of the non-K+ poses."""
    h = d = 0
    for p in st._poses:
        if p.kidx >= 0:
            continue
        for row in p.matrix:
            for x in row:
                h = max(h, abs(x.a), abs(x.b), x.d)
                d = max(d, x.d)
    return h, d


def centroid(ctx, p):
    vs = ctx.regions.vertices(p)
    s = [sum((v[i] for v in vs), ZERO) for i in range(4)]
    return [x * Q5(1, 0, len(vs)) for x in s]


def arrays(st):
    """Renderer arrays: per-piece pose index, float32 pose matrices (x' = M x, row-major) and
    the K+ index of each pose (-1 off the lattice)."""
    pose_index = st.pose_id.astype('<i4')
    poses = np.stack([pose_float(st, i) for i in range(len(st._poses))]).astype('<f4')
    kplus = np.array([p.kidx for p in st._poses], '<i4')
    return {'pose_index.i32': pose_index, 'poses.f32': poses, 'pose_kplus.i32': kplus}


def sha(a):
    return hashlib.sha256(np.ascontiguousarray(a).tobytes()).hexdigest()


def array_refs(st):
    return {name: {'sha256': sha(a), 'shape': list(a.shape), 'dtype': a.dtype.str} for name, a in arrays(st).items()}


def stage_refs(ctx, st, samples, records):
    flags = st.lattice_flags()
    h, d = height(st)
    out = {'records': records, 'digest': st.digest(), 'off_lattice': int((~flags).sum()),
           'poses': len(st._poses), 'kplus_poses': sum(p.kidx >= 0 for p in st._poses),
           'max_entry_height': h, 'max_denominator': d, 'arrays': array_refs(st)}
    rows = []
    for p in samples:
        pid = int(st.pose_id[p])
        x = matvec(st._pose_matrix(pid), centroid(ctx, p))
        rows.append({'piece': int(p), 'pose': pid, 'lattice': bool(flags[p]),
                     'centroid': [to_float(v) for v in x]})
    out['samples'] = rows
    return out


def simple_rotation(r):
    """(u, v, theta) with R(phi) = I + (cos phi - 1)(uu^T + vv^T) + sin phi (vu^T - uv^T) and
    R(theta) = r, for a rotation fixing a plane pointwise with 0 < theta < pi (the J2 family)."""
    cos_t = float(np.clip((np.trace(r) - 2) / 2, -1, 1))
    theta = math.acos(cos_t)
    sym = (r + r.T - 2 * cos_t * np.eye(4)) / (2 * (1 - cos_t))
    q = np.eye(4) - sym
    k = int(np.argmax(np.linalg.norm(q, axis=0)))
    u = q[:, k] / np.linalg.norm(q[:, k])
    v = ((r - r.T) / (2 * math.sin(theta))) @ u
    v /= np.linalg.norm(v)
    return u, v, theta


def family(u, v, phi):
    return (np.eye(4) + (math.cos(phi) - 1) * (np.outer(u, u) + np.outer(v, v))
            + math.sin(phi) * (np.outer(v, u) - np.outer(u, v)))


def sweep_refs(ctx, st, tw, rng):
    """The swept twist at t = 1/2: the moving set, the family parameters and posed centroids."""
    cl = st.classify(tw.grip)
    if cl.status != 'admissible':
        raise AssertionError('swept twist is not admissible')
    g = np.array([[to_float(x) for x in row] for row in tw.matrix])
    u, v, theta = simple_rotation(g)
    half = family(u, v, theta / 2)
    if float(np.abs(family(u, v, theta) - g).max()) > 1e-9:
        raise AssertionError('swept twist is not a simple rotation')
    inside = [int(p) for p in cl.inside]
    rows = []
    for p in sorted(rng.sample(inside, min(SWEEP_SAMPLES, len(inside)))):
        x = matvec(st._pose_matrix(int(st.pose_id[p])), centroid(ctx, p))
        start = np.array([to_float(c) for c in x])
        end = np.array([to_float(c) for c in matvec(tw.matrix, x)])
        rows.append({'piece': p, 'start': start.tolist(), 'mid': (half @ start).tolist(), 'end': end.tolist()})
    return {'record': len(st.journal), 'grip': tw.grip, 'angle_deg': math.degrees(theta), 'moved': len(inside),
            'moved_sha256': sha(np.array(inside, '<i4')), 'plane_u': u.tolist(), 'plane_v': v.tolist(),
            'before': {'digest': st.digest(), 'arrays': array_refs(st)}, 'samples': rows}


def survey_refs(st):
    """All 600 grips with their status; each blocked grip with its exact straddle certificate
    (piece and the two certified points, also as floats for overlays)."""
    t0 = time.time()
    sv = st.survey()
    if sorted(sv) != list(range(600)) or any(r['status'] not in ('admissible', 'blocked') for r in sv.values()):
        raise AssertionError('survey is incomplete or uncertain')
    blocked = []
    for e, r in sorted(sv.items()):
        if r['status'] != 'blocked':
            continue
        c = r['certificate']
        if c['grip'] != e:
            raise AssertionError('certificate names another grip')
        blocked.append({'grip': e, 'piece': c['piece'], 'point_below': c['point_below'],
                        'point_above': c['point_above'],
                        'point_below_float': [to_float(Q5(*x)) for x in c['point_below']],
                        'point_above_float': [to_float(Q5(*x)) for x in c['point_above']]})
    status = ''.join('b' if sv[e]['status'] == 'blocked' else 'a' for e in range(600))
    return {'admissible': status.count('a'), 'blocked': len(blocked), 'status': status,
            'certificates': blocked, 'survey_s': round(time.time() - t0, 1)}


def replay_with_refs(ctx, menu, records, samples=None):
    """Replay under the menu, collecting the stage and sweep references. Returns (state, refs)."""
    rng = random.Random(SEED)
    st = sim.State(ctx, menu=menu)
    mid = len(records) // 2
    # the swept twist is the last record with an angle strictly between 0 and 180 degrees, so the
    # W-J trace runs in the most scrambled part of the script (half-turns have no swept direction)
    swept = [i for i, r in enumerate(records) if 0.5 < Twist.from_record(ctx, r).angle_deg() < 179.5]
    if not swept:
        raise AssertionError('no swept twist with an angle strictly between 0 and 180 degrees')
    refs, sweep, t0 = {}, None, time.time()
    for i in range(len(records) + 1):
        if i == mid:
            refs['_mid_state'] = (st.pose_id.copy(), list(st._poses), dict(st._lookup), list(st.journal))
        if i == len(records):
            break
        tw = Twist.from_record(ctx, records[i])
        if i == swept[-1]:
            sweep = sweep_refs(ctx, st, tw, rng)
        out = st.apply(tw)
        if not out.applied or out.moved != records[i].get('moved', out.moved):
            raise AssertionError(f'record {i} gave {out.status} with {out.moved} moved pieces')
        if sweep is not None and sweep['record'] == i:
            sweep['after'] = {'digest': st.digest(), 'arrays': array_refs(st)}
    survey = survey_refs(st)
    if samples is None:
        off = np.flatnonzero(~st.lattice_flags()).tolist()
        on = np.flatnonzero(st.lattice_flags() & (st.pose_id != 0)).tolist()
        samples = sorted(rng.sample(off, min(SAMPLES * 3 // 4, len(off)))
                         + rng.sample(on, min(SAMPLES // 4, len(on))))
    end = stage_refs(ctx, st, samples, len(records))
    pose_id, poses, lookup, journal = refs.pop('_mid_state')
    mid_st = sim.State(ctx, menu=menu)
    mid_st.pose_id, mid_st._poses, mid_st._lookup, mid_st.journal = pose_id, poses, lookup, journal
    refs = {'start': stage_refs(ctx, sim.State(ctx, menu=menu), samples, 0),
            'mid': stage_refs(ctx, mid_st, samples, mid), 'end': end}
    return st, mid_st, {'stages': refs, 'sweep': sweep, 'samples': samples, 'end_survey': survey,
                        'replay_s': round(time.time() - t0, 1)}


def fixture_path(name):
    return HERE / f'wj-{name}.json'


def build(name, source):
    ctx = sim.get_context()
    menu = make_menu(ctx, name)
    doc = json.loads(Path(source).read_text())
    j = doc['journal']
    j = json.loads(j) if isinstance(j, str) else j
    records = j['records']
    st, _, refs = replay_with_refs(ctx, menu, records)
    fixture = {'format': FORMAT, 'menu_name': name, 'menu': menu.record(),
               'journal': json.loads(st.journal_json()),
               'source': {'script': 'research/jumbling/theory/walk.py', 'menu': doc.get('menu'),
                          'seed': doc.get('seed')},
               'coordinates': 'the frame of assets/model.npz: facets n_e . x = |n|^2 with |n|^2 = 12 + 4 sqrt5',
               'evidence': 'source and synthetic geometry; legality and digests exact in Q(sqrt 5); arrays and positions float',
               **{k: refs[k] for k in ('samples', 'stages', 'sweep', 'end_survey')}}
    fixture_path(name).write_text(json.dumps(fixture, indent=1, sort_keys=True) + '\n')
    e = refs['stages']['end']
    print(f'{name}: {len(records)} records, {e["off_lattice"]} off the lattice, {e["poses"]} poses, '
          f'height {e["max_entry_height"]}, {refs["end_survey"]["blocked"]} blocked grips at the end, '
          f'replay {refs["replay_s"]} s (survey {refs["end_survey"]["survey_s"]} s)')


def compare(a, b, path=''):
    """Exact equality except floats, which must agree to 1e-12 relative."""
    if isinstance(a, float) or isinstance(b, float):
        return [] if math.isclose(a, b, rel_tol=1e-12, abs_tol=1e-12) else [f'{path}: {a} != {b}']
    if isinstance(a, dict) and isinstance(b, dict):
        if a.keys() != b.keys():
            return [f'{path}: keys differ']
        return [m for k in a for m in compare(a[k], b[k], f'{path}.{k}')]
    if isinstance(a, list) and isinstance(b, list):
        if len(a) != len(b):
            return [f'{path}: lengths differ']
        return [m for i, (x, y) in enumerate(zip(a, b)) for m in compare(x, y, f'{path}[{i}]')]
    return [] if a == b else [f'{path}: {a!r} != {b!r}']


def load(ctx, name):
    fixture = json.loads(fixture_path(name).read_text())
    if fixture.get('format') != FORMAT:
        raise ValueError('unknown fixture format')
    menu = TwistMenu.from_record(ctx, fixture['menu'])
    expected = {'model_identity': ctx.data.identity, 'menu_identity': menu.identity,
                'contract_revision': sim.model.CONTRACT_REVISION}
    for field, value in expected.items():
        if fixture['journal'].get(field) != value:
            raise ValueError(f'fixture {field} differs from this checkout')
    return fixture, menu


def check(names):
    ctx = sim.get_context()
    bad = 0
    for name in names:
        fixture, menu = load(ctx, name)
        _, _, refs = replay_with_refs(ctx, menu, fixture['journal']['records'], fixture['samples'])
        keys = ('stages', 'sweep', 'end_survey')
        drop = lambda d: {k: ({x: y for x, y in v.items() if x != 'survey_s'} if k == 'end_survey' else v)
                          for k, v in d.items() if k in keys}
        problems = compare(drop(fixture), drop(refs))
        bad += bool(problems)
        print(f'{name}: {"ok" if not problems else "MISMATCH"} (replay {refs["replay_s"]} s)')
        for m in problems[:20]:
            print('  ', m)
    return int(bool(bad))


def export(name, stage, out):
    ctx = sim.get_context()
    fixture, menu = load(ctx, name)
    records = fixture['journal']['records']
    sweep = fixture['sweep']
    n = {'start': 0, 'mid': len(records) // 2, 'end': len(records),
         'sweep-before': sweep['record'], 'sweep-after': sweep['record'] + 1}[stage]
    ref = fixture['stages'][stage] if stage in fixture['stages'] else sweep[stage.split('-')[1]]
    st = sim.State.replay({**fixture['journal'], 'records': records[:n]}, ctx=ctx, menu=menu)
    if st.digest() != ref['digest']:
        raise AssertionError('replayed digest differs from the fixture')
    out = Path(out)
    out.mkdir(parents=True, exist_ok=True)
    header = {'format': FORMAT, 'menu': name, 'stage': stage, 'records': n, 'digest': st.digest(),
              'pieces': int(len(st.pose_id)), 'convention': "x' = M x; M row-major float32; pose 0 is the identity",
              'files': {}}
    for fname, a in arrays(st).items():
        if sha(a) != ref['arrays'][fname]['sha256']:
            raise AssertionError(f'{fname} differs from the fixture hash')
        (out / fname).write_bytes(np.ascontiguousarray(a).tobytes())
        header['files'][fname] = {'sha256': sha(a), 'shape': list(a.shape), 'dtype': a.dtype.str}
    (out / 'header.json').write_text(json.dumps(header, indent=1) + '\n')
    print(f'wrote {out} ({stage}: {header["records"]} records)')


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest='cmd', required=True)
    b = sub.add_parser('build')
    b.add_argument('menu', choices=MENUS)
    b.add_argument('source')
    c = sub.add_parser('check')
    c.add_argument('menus', nargs='*', choices=MENUS)
    e = sub.add_parser('export')
    e.add_argument('menu', choices=MENUS)
    e.add_argument('stage', choices=STAGES)
    e.add_argument('out')
    a = ap.parse_args(argv)
    if a.cmd == 'build':
        return build(a.menu, a.source)
    if a.cmd == 'check':
        return check(a.menus or [m for m in MENUS if fixture_path(m).exists()])
    return export(a.menu, a.stage, a.out)


if __name__ == '__main__':
    sys.exit(main())
