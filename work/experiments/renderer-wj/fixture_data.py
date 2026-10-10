"""One J1 replay per menu, with snapshots taken before the next record applies."""
from __future__ import annotations

import copy
import hashlib
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/jumbling/fixtures'))
import wj
import numpy as np

STAGES = ('start', 'mid', 'end', 'sweep-before', 'sweep-after')


def reference(fixture, stage):
    return (fixture['stages'][stage] if stage in fixture['stages']
            else fixture['sweep'][stage.split('-')[1]])


def compare_stage(fixture, stage, snapshot):
    """The single comparison used for real fixtures and all altered-fixture controls."""
    expected = reference(fixture, stage)
    if snapshot['digest'] != expected['digest']:
        raise ValueError(f'{stage}: exact J1 digest mismatch')
    for name, a in snapshot['arrays'].items():
        actual = {'sha256': wj.sha(a), 'shape': list(a.shape), 'dtype': a.dtype.str}
        if actual != expected['arrays'][name]:
            raise ValueError(f'{stage}/{name}: array hash, shape or type mismatch')
    if 'off_lattice' in expected:
        a = snapshot['arrays']
        off = int((a['pose_lattice.i32'][a['pose_index.i32']] < 0).sum())
        if off != expected['off_lattice']:
            raise ValueError(f'{stage}: off-lattice count mismatch')


def negative_controls(fixture, snapshots):
    for stage in STAGES:
        bad = copy.deepcopy(fixture)
        ref = reference(bad, stage)
        ref['digest'] = ('0' if ref['digest'][0] != '0' else '1') + ref['digest'][1:]
        try:
            compare_stage(bad, stage, snapshots[stage])
        except ValueError:
            continue
        raise AssertionError(f'{stage}: altered digest was accepted')
    bad = copy.deepcopy(fixture)
    ref = reference(bad, 'mid')['arrays']['poses.f32']
    ref['sha256'] = ('0' if ref['sha256'][0] != '0' else '1') + ref['sha256'][1:]
    try:
        compare_stage(bad, 'mid', snapshots['mid'])
    except ValueError:
        return
    raise AssertionError('altered array hash was accepted')


def snapshot(st):
    return {'digest': st.digest(), 'arrays': wj.arrays(st)}


def replay(ctx, name):
    fixture, menu = wj.load(ctx, name)
    records = fixture['journal']['records']
    sw = fixture['sweep']
    at = {'start': 0, 'mid': len(records) // 2, 'end': len(records),
          'sweep-before': sw['record'], 'sweep-after': sw['record'] + 1}
    st = wj.sim.State(ctx, menu=menu)
    snapshots, moving = {}, None
    for n in range(len(records) + 1):
        for stage, count in at.items():
            if count == n:
                captured = snapshot(st)
                compare_stage(fixture, stage, captured)
                snapshots[stage] = captured
                # Independent fixture centroid checks use the exact J1 regions.
                for sample in reference(fixture, stage).get('samples', []):
                    p = sample['piece']
                    home = np.array([wj.to_float(c) for c in wj.centroid(ctx, p)])
                    posed = wj.pose_float(st, int(st.pose_id[p])) @ home
                    if not np.allclose(posed, sample['centroid'], rtol=1e-10, atol=1e-10):
                        raise ValueError(f'{name}/{stage}: centroid {p} differs')
                print(f'{name}/{stage}: exact digest and three array hashes pass', flush=True)
        if n == len(records):
            break
        tw = wj.Twist.from_record(ctx, records[n])
        if n == sw['record']:
            cl = st.classify(tw.grip)
            moving = np.array(sorted(int(p) for p in cl.inside), '<i4')
            if (cl.status != 'admissible' or tw.grip != sw['grip']
                    or len(moving) != sw['moved'] or wj.sha(moving) != sw['moved_sha256']):
                raise ValueError(f'{name}: swept moving set differs')
            u, v = np.array(sw['plane_u']), np.array(sw['plane_v'])
            angle = math.radians(sw['angle_deg'])
            exact = np.array([[wj.to_float(x) for x in row] for row in tw.matrix])
            if (not 0 < angle < math.pi
                    or not np.allclose([u @ u, v @ v, u @ v], [1, 1, 0], atol=1e-12, rtol=0)
                    or not np.allclose(wj.family(u, v, angle), exact, atol=1e-9, rtol=0)):
                raise ValueError(f'{name}: sweep plane differs from J1')
            for sample in sw['samples']:
                p = sample['piece']
                if p not in moving:
                    raise ValueError('sweep sample is not moving')
                home = np.array([wj.to_float(c) for c in wj.centroid(ctx, p)])
                start = wj.pose_float(st, int(st.pose_id[p])) @ home
                for key, phi in (('start', 0), ('mid', angle / 2), ('end', angle)):
                    if not np.allclose(wj.family(u, v, phi) @ start, sample[key], rtol=1e-10, atol=1e-10):
                        raise ValueError(f'{name}: swept centroid {p}/{key} differs')
        out = st.apply(tw)
        if not out.applied or out.moved != records[n].get('moved', out.moved):
            raise ValueError(f'{name}: record {n} did not apply as recorded')
    negative_controls(fixture, snapshots)
    # The end survey is fixture data, not a classification invented by the renderer.
    survey = fixture['end_survey']
    if (len(survey['status']) != 600 or set(survey['status']) != {'a', 'b'}
            or survey['status'].count('b') != len(survey['certificates'])):
        raise ValueError('incomplete fixture survey')
    return fixture, snapshots, moving


def write_state(directory, fixture, stage, captured, revision):
    directory.mkdir(parents=True, exist_ok=True)
    arrays = captured['arrays']
    header = {'format': wj.RENDER_FORMAT, 'dimension': 4, 'pieces': 177120,
              'poses': len(arrays['poses.f32']), 'revision': revision,
              'digest': captured['digest'],
              **{k: fixture['journal'][k] for k in ('model_identity', 'menu_identity', 'contract_revision')},
              'fixture': {'format': wj.FORMAT, 'menu': fixture['menu_name'], 'stage': stage, 'records': revision},
              'files': {}}
    for name, a in arrays.items():
        (directory / name).write_bytes(np.ascontiguousarray(a).tobytes())
        header['files'][name] = {'sha256': wj.sha(a), 'shape': list(a.shape), 'dtype': a.dtype.str}
    (directory / 'header.json').write_text(json.dumps(header, indent=2) + '\n', encoding='utf-8')


def write_exports(out, fixture, snapshots, moving):
    name = fixture['menu_name']
    records = fixture['journal']['records']
    for stage, captured in snapshots.items():
        n = (fixture['stages'][stage]['records'] if stage in fixture['stages']
             else fixture['sweep']['record'] + (stage == 'sweep-after'))
        write_state(out / name / stage, fixture, stage, captured, n)
    raw = moving.tobytes()
    (out / name / 'moving.i32').write_bytes(raw)
    motion = {**fixture['sweep'], 'format': 'magic600-wj-motion/1', 'dimension': 4,
              'fixture_sha256': hashlib.sha256(wj.fixture_path(name).read_bytes()).hexdigest(),
              'moving_sha256': hashlib.sha256(raw).hexdigest()}
    (out / name / 'motion.json').write_text(json.dumps(motion, indent=2) + '\n', encoding='utf-8')
