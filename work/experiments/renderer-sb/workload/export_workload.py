"""Export the S-B W3 turn workload from the engine model (stage 2.4, packet E-2.4-01).

Run with the engine Python (NumPy): tools/.venv/engine/Scripts/python.exe work/experiments/renderer-sb/workload/export_workload.py

Writes turn.json next to this file: the generator with the largest moved-slot count, its
4D rotation plane and angle (derived as in grips.py), the slots that rotate during the
animation (Model.cap_mask), the move's src -> dst slot pairs (Model.move) and the SHA-256
of the two label arrays the W3 trace alternates between. Labels are synthetic: the solved
state and the state after one turn. Nothing here reads a session.
"""
import hashlib
import json
import sys
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
sys.path.insert(0, str(ROOT))
from core import Model, PuzzleState  # noqa: E402


def label_digest(labels):
    return hashlib.sha256(np.asarray(labels, '<u4').tobytes()).hexdigest()


def main():
    model = Model()
    counts = np.diff(model.offset)
    generator = int(np.argmax(counts)) + 1          # first of the ties: H of cell 0
    cell = (generator - 1) // 2
    perm = model.z['rotperms'][generator - 1]
    rotation = np.linalg.lstsq(model.normals, model.normals[perm], rcond=None)[0].T
    _, _, vh = np.linalg.svd(rotation - np.eye(4))
    u, v = vh[0], vh[1]
    angle = float(np.arctan2(v @ rotation @ u, u @ rotation @ u))
    moving = np.flatnonzero(model.cap_mask(cell)[model.sp]).astype(np.int64)

    src, dst = (np.asarray(a, np.int64) for a in model.move(generator))
    inv_src, inv_dst = (np.asarray(a, np.int64) for a in model.move(-generator))

    # The animation rotates the cap in the plane (u, v) by `angle` and then adopts the new
    # labels, as the web renderer does (web/renderer.js:28-31). The sticker cuts are not exactly
    # symmetric under the rotation, so a rotated slot centre lands near, not on, its destination
    # slot centre; the deviation is recorded, not asserted (it is a visual property only).
    def rotate(points, theta):
        x, y = points @ u, points @ v
        c, s = np.cos(theta), np.sin(theta)
        return points + np.outer((c - 1) * x - s * y, u) + np.outer(s * x + (c - 1) * y, v)
    centers = model.z['slot_centers'].astype(np.float64)
    deviation = np.linalg.norm(rotate(centers[src], angle) - centers[dst], axis=1)
    error = float(deviation.max())
    if not np.isin(src, moving).all():
        raise SystemExit('a moved slot is outside the animated cap')

    state = PuzzleState(model)
    solved = state.labels.copy()
    state.apply(src, dst)
    turned = state.labels.copy()
    state.apply(inv_src, inv_dst)
    if not np.array_equal(state.labels, solved):
        raise SystemExit('generator followed by its inverse does not return to the solved labels')

    out = {
        'format': 'magic600-sb-turn-v1',
        'model_id': model.model_id,
        'source': 'core.Model (move, cap_mask, rotperms); plane and angle as in grips.py',
        'generator': generator, 'inverse': -generator, 'cell': cell,
        'moved_slots': int(counts[generator - 1]), 'max_moved_slots': int(counts.max()),
        'plane_u': [float(x) for x in u], 'plane_v': [float(x) for x in v], 'angle': angle,
        'rotation_vs_move_deviation': {'max': error, 'median': float(np.median(deviation))},
        'moving_slots': moving.tolist(),
        'move_src': src.tolist(), 'move_dst': dst.tolist(),
        'inverse_src': inv_src.tolist(), 'inverse_dst': inv_dst.tolist(),
        'labels': {'encoding': 'u32 little-endian, 259,800 entries; label = original slot id',
                   'revision_even_sha256': label_digest(solved),
                   'revision_odd_sha256': label_digest(turned)},
    }
    (HERE / 'turn.json').write_text(json.dumps(out, separators=(',', ':')) + '\n', encoding='utf-8', newline='\n')
    print(f'generator {generator}: {out["moved_slots"]} moved slots, {len(moving)} animated, '
          f'angle {angle:.6f}, rotated-centre deviation max {error:.3f}')


if __name__ == '__main__':
    main()
