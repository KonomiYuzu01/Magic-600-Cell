"""Check the S-B assets, turn labels and committed geometry without file writes."""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import struct
import subprocess
import sys
from array import array

# Importing the local reference must not create a __pycache__ directory.
sys.dont_write_bytecode = True
import reference_geometry as reference


def check_workload_assets():
    result = subprocess.run(
        [sys.executable, '-B', str(reference.ROOT / 'tools/perf/check_renderer_assets.py')],
        cwd=reference.ROOT, capture_output=True, text=True)
    if result.returncode:
        raise ValueError(result.stdout.strip() or result.stderr.strip()
                         or f'renderer asset checker exited {result.returncode}')
    return result.stdout.strip()


def slot_list(turn, name, count):
    values = turn[name]
    if (not isinstance(values, list) or len(values) != count
            or any(type(value) is not int or not 0 <= value < reference.SLOTS
                   for value in values)
            or len(set(values)) != count):
        raise ValueError(f'turn.json: {name} must contain {count:,} distinct valid slot ids')
    return values


def apply_move(labels, src, dst):
    before = labels[:]
    for source, destination in zip(src, dst):
        labels[destination] = before[source]


def label_digest(labels):
    if sys.byteorder == 'little':
        return hashlib.sha256(labels.tobytes()).hexdigest()
    encoded = labels[:]
    encoded.byteswap()
    return hashlib.sha256(encoded.tobytes()).hexdigest()


def check_turn():
    turn = json.loads((reference.HERE / 'workload/turn.json').read_bytes())
    if turn['format'] != 'magic600-sb-turn-v1':
        raise ValueError('turn.json: expected magic600-sb-turn-v1')
    moving = set(slot_list(turn, 'moving_slots', 4605))
    u, v = turn['plane_u'], turn['plane_v']
    if (len(u) != 4 or len(v) != 4
            or not all(math.isfinite(value) for value in (*u, *v))
            or abs(sum(value * value for value in u) - 1.0) > 1e-9
            or abs(sum(value * value for value in v) - 1.0) > 1e-9
            or abs(sum(x * y for x, y in zip(u, v))) > 1e-9):
        raise ValueError('turn.json: plane_u and plane_v must be orthonormal within 1e-9')
    moves = []
    for prefix in ('move', 'inverse'):
        src = slot_list(turn, f'{prefix}_src', 4600)
        dst = slot_list(turn, f'{prefix}_dst', 4600)
        if not set(src) <= moving:
            raise ValueError(f'turn.json: {prefix}_src contains a slot outside moving_slots')
        if set(src) != set(dst):
            raise ValueError(f'turn.json: {prefix} must permute its source slots')
        moves.append((src, dst))

    solved = array('I', range(reference.SLOTS))
    if solved.itemsize != 4:
        raise ValueError('turn labels: this platform does not provide 32-bit array("I")')
    turned = solved[:]
    apply_move(turned, *moves[0])
    for revision, labels in (('even', solved), ('odd', turned)):
        if label_digest(labels) != turn['labels'][f'revision_{revision}_sha256']:
            raise ValueError(f'turn.json: revision_{revision}_sha256 does not match rebuilt labels')
    apply_move(turned, *moves[1])
    if turned != solved:
        raise ValueError('turn.json: generator followed by inverse does not restore solved labels')
    return ('S-B turn: ok (4,605 moving slots; 4,600 pairs each way; '
            'orthonormal plane; even/odd label digests; inverse restores solved)')


def check_anchors():
    mesh = json.loads((reference.ROOT / 'assets/mesh.json').read_bytes())
    offsets = mesh['offsets']
    vertices = reference.binary((reference.ROOT / 'assets/mesh_vertices.f32').read_bytes(),
                                'f', reference.BASE_VERTICES * 4, 'mesh_vertices.f32')
    frames = reference.binary((reference.ROOT / 'assets/cell_frames.f32').read_bytes(),
                              'f', reference.CELLS * 16, 'cell_frames.f32')
    anchors = reference.sticker_anchors(offsets, vertices)
    digest = hashlib.sha256(struct.pack(f'<{reference.BASE_STICKERS * 4}f',
                                        *anchors)).hexdigest()
    if digest != reference.ANCHOR_SHA256:
        raise ValueError('S-B anchors: SHA-256 mismatch with SPEC section 3')
    turn = json.loads((reference.HERE / 'workload/turn.json').read_bytes())
    pairs = list(zip(turn['move_src'], turn['move_dst']))
    pairs.extend((slot, slot) for slot in sorted(set(turn['moving_slots'])
                                               - set(turn['move_src'])))
    normal, radius = mesh['normal'], mesh['normal_length']
    cs, ss = reference.PARAMETERS['cs'], reference.PARAMETERS['ss']
    u, v, angle = turn['plane_u'], turn['plane_v'], turn['angle']

    def continuity(centers):
        shrunk = []
        for local, (begin, end) in enumerate(zip(offsets, offsets[1:])):
            center = centers[local * 4:local * 4 + 4]
            for vi in range(begin, end):
                shrunk.append(tuple((normal[i] + cs * (center[i] - normal[i])
                                     + cs * ss * (vertices[vi * 4 + i] - center[i]))
                                    / radius for i in range(4)))

        def bounds(slot, theta):
            cell, local = divmod(slot, reference.BASE_STICKERS)
            frame = frames[cell * 16:cell * 16 + 16]
            low, high = [math.inf] * 4, [-math.inf] * 4
            for vi in range(offsets[local], offsets[local + 1]):
                world = reference.turn_vertex(reference.matvec(frame, shrunk[vi]),
                                              u, v, theta)
                for i in range(4):
                    low[i] = min(low[i], world[i])
                    high[i] = max(high[i], world[i])
            return (*low, *high)

        maximum = 0.0
        for src, dst in pairs:
            error = max(abs(left - right) for left, right
                        in zip(bounds(src, angle), bounds(dst, 0.0)))
            if not math.isfinite(error):
                raise ValueError(f'S-B anchors: non-finite bounds error for {src} -> {dst}')
            maximum = max(maximum, error)
        return maximum

    maximum = continuity(anchors)
    if maximum > 2e-6:
        raise ValueError(f'S-B anchors: W3 turn-end bounds error {maximum:.9g} exceeds 2e-6')
    # The control is the retained numbering centres, so verify them like every asset.
    manifest = json.loads((reference.ROOT / 'assets/manifest.json').read_bytes())
    raw_centers = (reference.ROOT / 'assets/mesh_centers.f32').read_bytes()
    if hashlib.sha256(raw_centers).hexdigest() != manifest['files'].get('assets/mesh_centers.f32'):
        raise ValueError('assets/mesh_centers.f32: SHA-256 digest mismatch with manifest.json')
    centers = reference.binary(raw_centers, 'f', reference.BASE_STICKERS * 4, 'mesh_centers.f32')
    control = continuity(centers)
    if control <= 2e-6:
        raise ValueError('S-B anchors: mesh_centers control did not fail continuity')
    return (f'S-B shrink anchors SHA-256: ok ({digest})\n'
            f'S-B W3 turn-end continuity: ok ({len(pairs):,} animated slots; '
            f'maximum {maximum:.9g}; mesh_centers control maximum {control:.9g} > 2e-6)')


def check_reference():
    expected = reference.build_reference()
    directory = reference.HERE / 'reference'
    index = json.loads((directory / 'index.json').read_bytes())
    if {path.name for path in directory.iterdir()} != set(expected):
        raise ValueError('S-B reference: file set differs from the SPEC outputs')
    for name, data in expected.items():
        committed = (directory / name).read_bytes()
        if committed != data:
            raise ValueError(f'S-B reference: {name} differs from regenerated bytes')
        if name != 'index.json' and hashlib.sha256(committed).hexdigest() != index['files'][name]:
            raise ValueError(f'S-B reference: {name} digest differs from index.json')
    return (f'S-B geometry: ok (9 camera/pose files; {index["sample_count"]:,} samples; '
            f'{index["moving_sample_count"]:,} moving; 600 x 30,480 vertices; bytes and digests match)')


def main(argv=None):
    argparse.ArgumentParser(description=__doc__).parse_args(argv)
    for check in (check_workload_assets, check_turn, check_anchors, check_reference):
        try:
            summary = check()
        except (OSError, ValueError, KeyError, TypeError) as exc:
            print(f'S-B check: fail ({exc})')
            return 1
        print(summary)
    return 0


if __name__ == '__main__':
    sys.exit(main())
