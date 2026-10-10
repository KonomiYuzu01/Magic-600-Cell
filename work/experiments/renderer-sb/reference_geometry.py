"""Generate the stdlib-only S-B geometry reference defined by SPEC.md."""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import struct
import sys
from array import array
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
PREFIX = 'work/experiments/renderer-sb'
ASSET_NAMES = ('mesh.json', 'mesh_vertices.f32', 'mesh_sticker.u32',
               'mesh_centers.f32', 'cell_frames.f32')
BASE_VERTICES, BASE_STICKERS, CELLS = 30480, 433, 600
SLOTS = CELLS * BASE_STICKERS
SAMPLE_STRIDE = 4099
ASPECT = 1.6
PARAMETERS = {'cs': 0.76, 'ss': 0.82, 'd4': 1.18, 'zoom': 1.15}


def binary(raw, code, count, name):
    values = array(code)
    if values.itemsize != 4 or len(raw) != count * 4:
        raise ValueError(f'{name}: expected {count} 32-bit values')
    values.frombytes(raw)
    if sys.byteorder != 'little':
        values.byteswap()
    if code == 'f' and not all(math.isfinite(value) for value in values):
        raise ValueError(f'{name}: non-finite float32 value')
    return values


def sticker_anchors(offsets, vertices):
    """Return the SPEC section 3 step 1 anchors: 433 x 4 float32 values in an array('f').

    Each anchor is the area-weighted centroid of its sticker's triangles, in float64,
    summed in file order and rounded once to float32.
    """
    anchors = array('f')
    for local, (begin, end) in enumerate(zip(offsets, offsets[1:])):
        total, weighted = 0.0, [0.0, 0.0, 0.0, 0.0]
        for t in range(begin, end, 3):
            a, b, c = (vertices[k * 4:k * 4 + 4] for k in (t, t + 1, t + 2))
            e1 = [b[i] - a[i] for i in range(4)]
            e2 = [c[i] - a[i] for i in range(4)]
            d11 = e1[0] * e1[0] + e1[1] * e1[1] + e1[2] * e1[2] + e1[3] * e1[3]
            d22 = e2[0] * e2[0] + e2[1] * e2[1] + e2[2] * e2[2] + e2[3] * e2[3]
            d12 = e1[0] * e2[0] + e1[1] * e2[1] + e1[2] * e2[2] + e1[3] * e2[3]
            area = 0.5 * math.sqrt(max(d11 * d22 - d12 * d12, 0.0))
            total += area
            for i in range(4):
                weighted[i] += area * ((a[i] + b[i] + c[i]) / 3.0)
        anchor = [value / total for value in weighted] if total > 0 else []
        if not math.isfinite(total) or total <= 0 or not all(map(math.isfinite, anchor)):
            raise ValueError(f'sticker {local}: no finite positive triangle area')
        anchors.extend(anchor)
    return anchors


def matvec(matrix, vector):
    return tuple(sum(matrix[col * 4 + row] * vector[col] for col in range(4))
                 for row in range(4))


def camera_matrix(rotations):
    q = [1.0 if i % 5 == 0 else 0.0 for i in range(16)]
    for a, b, theta in rotations:
        if (type(a) is not int or type(b) is not int or a == b
                or not 0 <= a < 4 or not 0 <= b < 4 or not math.isfinite(theta)):
            raise ValueError('cameras.json: invalid rotate step')
        c, s = math.cos(theta), math.sin(theta)
        for j in range(4):
            x, y = q[a * 4 + j], q[b * 4 + j]
            q[a * 4 + j] = c * x - s * y
            q[b * 4 + j] = s * x + c * y
    return q


def turn_vertex(world, u, v, theta):
    if theta == 0:
        return world
    x = sum(world[i] * u[i] for i in range(4))
    y = sum(world[i] * v[i] for i in range(4))
    c, s = math.cos(theta), math.sin(theta)
    du, dv = (c - 1.0) * x - s * y, s * x + (c - 1.0) * y
    return tuple(world[i] + du * u[i] + dv * v[i] for i in range(4))


def project(world, q, aspect):
    world = matvec(q, world)
    d4, zoom = PARAMETERS['d4'], PARAMETERS['zoom']
    p = tuple(d4 * world[i] / (d4 - world[3]) for i in range(3))
    w = 5.0 - p[2]
    return p[0] * zoom / aspect / w, p[1] * zoom / w, w


def build_reference():
    """Return all output file bytes, including index.json, without writing files."""
    names = ('assets/manifest.json', *(f'assets/{name}' for name in ASSET_NAMES),
             f'{PREFIX}/cameras.json', f'{PREFIX}/workload/turn.json')
    inputs = {name: (ROOT / name).read_bytes() for name in names}
    manifest = json.loads(inputs['assets/manifest.json'])
    for name in ASSET_NAMES:
        path = f'assets/{name}'
        if hashlib.sha256(inputs[path]).hexdigest() != manifest['files'].get(path):
            raise ValueError(f'{path}: SHA-256 digest mismatch with manifest.json')

    mesh = json.loads(inputs['assets/mesh.json'])
    counts = {'base_vertices': BASE_VERTICES, 'base_stickers': BASE_STICKERS,
              'cells': CELLS, 'slots': SLOTS}
    if any(type(mesh.get(name)) is not int or mesh[name] != value
           for name, value in counts.items()):
        raise ValueError('mesh.json: counts do not match the full 600-cell model')
    offsets = mesh['offsets']
    if (len(offsets) != BASE_STICKERS + 1
            or not all(type(value) is int for value in offsets)
            or offsets[0] != 0 or offsets[-1] != BASE_VERTICES
            or any(begin >= end or (end - begin) % 3
                   for begin, end in zip(offsets, offsets[1:]))):
        raise ValueError('mesh.json: offsets must span 30480 vertices in 433 triangle lists')
    normal, radius = mesh['normal'], mesh['normal_length']
    if (len(normal) != 4 or not all(math.isfinite(value) for value in normal)
            or not math.isfinite(radius) or radius <= 0):
        raise ValueError('mesh.json: invalid normal or normal_length')

    vertices = binary(inputs['assets/mesh_vertices.f32'], 'f', BASE_VERTICES * 4,
                      'mesh_vertices.f32')
    stickers = binary(inputs['assets/mesh_sticker.u32'], 'I', BASE_VERTICES,
                      'mesh_sticker.u32')
    centers = binary(inputs['assets/mesh_centers.f32'], 'f', BASE_STICKERS * 4,
                     'mesh_centers.f32')
    frames = binary(inputs['assets/cell_frames.f32'], 'f', CELLS * 16,
                    'cell_frames.f32')
    if any(stickers[vi] != local
           for local, (begin, end) in enumerate(zip(offsets, offsets[1:]))
           for vi in range(begin, end)):
        raise ValueError('mesh_sticker.u32: vertex sticker indices do not match offsets')

    cameras = json.loads(inputs[f'{PREFIX}/cameras.json'])
    if (cameras['format'] != 'magic600-sb-cameras-v1' or cameras['aspect'] != ASPECT
            or [camera['name'] for camera in cameras['cameras']] != ['c0', 'c1', 'c2']):
        raise ValueError('cameras.json: expected c0..c2 with aspect 1.6')
    matrices = [(camera['name'], camera_matrix(camera['rotations']))
                for camera in cameras['cameras']]
    turn = json.loads(inputs[f'{PREFIX}/workload/turn.json'])
    if turn['format'] != 'magic600-sb-turn-v1' or turn['model_id'] != manifest['model_id']:
        raise ValueError('turn.json: format or model identity mismatch')
    u, v, angle = turn['plane_u'], turn['plane_v'], turn['angle']
    if (len(u) != 4 or len(v) != 4 or not math.isfinite(angle)
            or not all(math.isfinite(value) for value in (*u, *v))):
        raise ValueError('turn.json: invalid rotation plane or angle')
    moving = set(turn['moving_slots'])
    if (len(moving) != len(turn['moving_slots'])
            or any(type(slot) is not int or not 0 <= slot < SLOTS for slot in moving)):
        raise ValueError('turn.json: moving slots must be distinct valid slot ids')

    sample = set(range(0, CELLS * BASE_VERTICES, SAMPLE_STRIDE))
    sample.update((slot // BASE_STICKERS) * BASE_VERTICES + offsets[slot % BASE_STICKERS]
                  for slot in moving)
    sample = sorted(sample)
    world_sample = []
    cs, ss = PARAMETERS['cs'], PARAMETERS['ss']
    for g in sample:
        cell, vi = divmod(g, BASE_VERTICES)
        local = stickers[vi]
        center = centers[local * 4:local * 4 + 4]
        vertex = vertices[vi * 4:vi * 4 + 4]
        # array('f') elements become Python float64; round only at output packing.
        vertex = tuple((normal[i] + cs * (center[i] - normal[i])
                        + cs * ss * (vertex[i] - center[i])) / radius for i in range(4))
        world = matvec(frames[cell * 16:cell * 16 + 16], vertex)
        world_sample.append((world, cell * BASE_STICKERS + local in moving))

    files = {'sample.u32': struct.pack(f'<{len(sample)}I', *sample)}
    poses = {'start': 0.0, 'mid': angle * 0.5, 'end': angle}
    for pose, theta in poses.items():
        worlds = [turn_vertex(world, u, v, theta) if animated else world
                  for world, animated in world_sample]
        for camera, q in matrices:
            files[f'{camera}_{pose}.f32'] = b''.join(
                struct.pack('<3f', *project(world, q, ASPECT)) for world in worlds)

    index = {
        'format': 'magic600-sb-reference-v1',
        'parameters': {**PARAMETERS, 'R': radius, 'near': 0.05, 'far': 100.0},
        'aspect': ASPECT,
        'sample_stride': SAMPLE_STRIDE,
        'sample_count': len(sample),
        'moving_sample_count': sum(animated for _, animated in world_sample),
        'cells': CELLS,
        'per_cell_vertex_count': sum(end - begin for begin, end in zip(offsets, offsets[1:])),
        'offsets_check': {'count': len(offsets), 'first': offsets[0], 'last': offsets[-1],
                          'strictly_increasing': True, 'triangle_aligned': True,
                          'matches_stickers': True},
        'poses': poses,
        'tolerance': {'absolute': 1e-4, 'relative': 1e-4},
        'inputs': {name: hashlib.sha256(data).hexdigest() for name, data in inputs.items()},
        'files': {name: hashlib.sha256(data).hexdigest() for name, data in files.items()},
    }
    files['index.json'] = (json.dumps(index, indent=2, sort_keys=True) + '\n').encode('utf-8')
    return files


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, default=HERE / 'reference')
    args = parser.parse_args(argv)
    try:
        files = build_reference()
        args.out.mkdir(parents=True, exist_ok=True)
        for name, data in files.items():
            (args.out / name).write_bytes(data)
    except (OSError, ValueError, KeyError, TypeError) as exc:
        print(f'S-B reference: fail ({exc})')
        return 1
    index = json.loads(files['index.json'])
    print(f'S-B reference: wrote {len(files)} files '
          f'({index["sample_count"]:,} samples; {index["moving_sample_count"]:,} moving)')
    return 0


if __name__ == '__main__':
    sys.exit(main())
