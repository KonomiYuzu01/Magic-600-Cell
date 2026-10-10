"""Stdlib-only W-J reference: home shrink, piece pose, swept turn, S-B projection."""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import struct
import sys
from array import array
from pathlib import Path

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'work/experiments/renderer-sb'))
from reference_geometry import binary, matvec, camera_matrix, turn_vertex, project, PARAMETERS, ASPECT

NAMES = ('mesh.json', 'mesh_vertices.f32', 'mesh_sticker.u32', 'mesh_centers.f32',
         'cell_frames.f32', 'slot_piece.u32')
MENUS = ('S4', 'I_a', 'I_b')


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def geometry():
    manifest = json.loads((ROOT / 'assets/manifest.json').read_bytes())
    raw = {name: (ROOT / 'assets' / name).read_bytes() for name in NAMES}
    for name, data in raw.items():
        if digest(data) != manifest['files'][f'assets/{name}']:
            raise ValueError(f'{name}: manifest hash mismatch')
    mesh = json.loads(raw['mesh.json'])
    return (mesh, binary(raw['mesh_vertices.f32'], 'f', 30480 * 4, 'vertices'),
            binary(raw['mesh_sticker.u32'], 'I', 30480, 'stickers'),
            binary(raw['mesh_centers.f32'], 'f', 433 * 4, 'centers'),
            binary(raw['cell_frames.f32'], 'f', 600 * 16, 'frames'),
            binary(raw['slot_piece.u32'], 'I', 259800, 'slot_piece'),
            {f'assets/{n}': digest(b) for n, b in raw.items()})


def read_state(directory, fixture, stage):
    header = json.loads((directory / 'header.json').read_bytes())
    if header['format'] != 'magic600-jumbling-render/1' or header['dimension'] != 4 or header['pieces'] != 177120:
        raise ValueError('unsupported render header')
    ref = fixture['stages'][stage] if stage in fixture['stages'] else fixture['sweep'][stage.split('-')[1]]
    if header['digest'] != ref['digest'] or header['files'] != ref['arrays']:
        raise ValueError(f'{stage}: export header differs from fixture')
    revision = (ref['records'] if stage in fixture['stages']
                else fixture['sweep']['record'] + (stage == 'sweep-after'))
    if header['revision'] != revision or any(header[k] != fixture['journal'][k]
            for k in ('model_identity', 'menu_identity', 'contract_revision')):
        raise ValueError(f'{stage}: export revision or identity differs from fixture')
    out = {}
    for name, desc in header['files'].items():
        raw = (directory / name).read_bytes()
        if digest(raw) != desc['sha256']:
            raise ValueError(f'{stage}/{name}: export hash mismatch')
        count = math.prod(desc['shape'])
        out[name] = binary(raw, 'f' if name == 'poses.f32' else 'i', count, name)
    ids, poses = out['pose_index.i32'], out['poses.f32']
    if len(poses) != header['poses'] * 16 or any(pid < 0 or pid >= header['poses'] for pid in ids):
        raise ValueError('pose index outside pose table')
    if tuple(poses[:16]) != tuple(1.0 if i % 5 == 0 else 0.0 for i in range(16)):
        raise ValueError('pose zero is not identity')
    return out


def pose_point(poses, pid, world):
    # Poses are rows; facet frames and the S-B camera are columns.
    return tuple(sum(poses[pid * 16 + row * 4 + col] * world[col] for col in range(4))
                 for row in range(4))


def build_reference(exports, name):
    fixture_path = ROOT / f'research/jumbling/fixtures/wj-{name}.json'
    fixture_raw = fixture_path.read_bytes()
    fixture = json.loads(fixture_raw)
    mesh, vertices, stickers, centers, frames, piece_of, inputs = geometry()
    camera_raw = (ROOT / 'work/experiments/renderer-sb/cameras.json').read_bytes()
    cameras = json.loads(camera_raw)
    motion = json.loads((exports / name / 'motion.json').read_bytes())
    moving_raw = (exports / name / 'moving.i32').read_bytes()
    if (motion['dimension'] != 4 or motion['fixture_sha256'] != digest(fixture_raw)
            or digest(moving_raw) != fixture['sweep']['moved_sha256']):
        raise ValueError('motion or moving-set hash mismatch')
    moving = set(binary(moving_raw, 'i', fixture['sweep']['moved'], 'moving'))
    samples = set(range(0, 600 * 30480, 4099))
    # Every moving sticker, plus every fixture centroid's piece, contributes a vertex.
    highlighted = moving | {row['piece'] for row in fixture['stages']['end']['samples']}
    samples.update((slot // 433) * 30480 + mesh['offsets'][slot % 433]
                   for slot, piece in enumerate(piece_of) if piece in highlighted)
    samples = sorted(samples)
    home = []
    cs, ss, radius, normal = PARAMETERS['cs'], PARAMETERS['ss'], mesh['normal_length'], mesh['normal']
    for g in samples:
        cell, vi = divmod(g, 30480)
        local = stickers[vi]
        center = centers[local * 4:local * 4 + 4]
        vertex = vertices[vi * 4:vi * 4 + 4]
        shrunk = tuple((normal[i] + cs * (center[i] - normal[i])
                        + cs * ss * (vertex[i] - center[i])) / radius for i in range(4))
        home.append((matvec(frames[cell * 16:cell * 16 + 16], shrunk), piece_of[cell * 433 + local]))
    prefix = f'ref_{name}_'
    files = {prefix + 'sample.u32': struct.pack(f'<{len(samples)}I', *samples)}
    stages = ('start', 'mid', 'end', 'sweep-before', 'sweep-after')
    states = {stage: read_state(exports / name / stage, fixture, stage) for stage in stages}
    sw = fixture['sweep']
    outputs = []
    for label, stage, fraction in (('start', 'start', 0), ('mid', 'mid', 0), ('end', 'end', 0),
                                    ('sweep0', 'sweep-before', 0), ('sweep05', 'sweep-before', 0.5),
                                    ('sweep1', 'sweep-before', 1)):
        state = states[stage]
        worlds = []
        for world, piece in home:
            posed = pose_point(state['poses.f32'], state['pose_index.i32'][piece], world)
            if fraction and piece in moving:
                posed = turn_vertex(posed, sw['plane_u'], sw['plane_v'], math.radians(sw['angle_deg']) * fraction)
            worlds.append(posed)
        for camera in cameras['cameras']:
            q = camera_matrix(camera['rotations'])
            filename = prefix + camera['name'] + '_' + label + '.f32'
            files[filename] = b''.join(struct.pack('<3f', *project(w, q, ASPECT)) for w in worlds)
            outputs.append({'file': filename, 'camera': camera['name'], 'stage': stage,
                            'state': label, 'theta': math.radians(sw['angle_deg']) * fraction})
    inputs[f'research/jumbling/fixtures/wj-{name}.json'] = digest(fixture_raw)
    inputs['work/experiments/renderer-sb/cameras.json'] = digest(camera_raw)
    index = {'format': 'magic600-wj-reference/1', 'dimension': 4, 'menu': name,
             'parameters': {**PARAMETERS, 'R': radius}, 'aspect': ASPECT,
             'samples': len(samples), 'sample_stride': 4099, 'outputs': outputs,
             'inputs': inputs, 'stage_arrays': {s: reference_arrays(fixture, s) for s in stages},
             'tolerance': {'absolute': 1e-4, 'relative': 1e-4},
             'files': {n: digest(b) for n, b in files.items()}}
    files[prefix + 'index.json'] = (json.dumps(index, indent=2, sort_keys=True) + '\n').encode()
    return files


def reference_arrays(fixture, stage):
    return (fixture['stages'][stage] if stage in fixture['stages']
            else fixture['sweep'][stage.split('-')[1]])['arrays']


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--exports', required=True, type=Path)
    ap.add_argument('--menu', required=True, choices=MENUS)
    ap.add_argument('--out', required=True, type=Path)
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    for name, raw in build_reference(args.exports, args.menu).items():
        (args.out / name).write_bytes(raw)
    print(f'{args.menu}: wrote immutable reference outputs')


if __name__ == '__main__':
    main()
