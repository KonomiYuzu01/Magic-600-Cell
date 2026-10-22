"""Plain-data exports from J1, independent of the candidate store."""

import base64 as _base64
import hashlib as _hashlib

import numpy as _np

from exact import matmul as _matmul, transpose as _transpose
from sim.kplus import matrix_json as _matrix_json, to_tuple as _to_tuple
from sim.twists import Twist as _Twist


def state_record(state) -> dict:
    """Export J1's pose table, little-endian pose ids and exact state digest."""
    table = [{"k": int(p.kidx)} if p.kidx >= 0 else {"m": _matrix_json(p.matrix)}
             for p in state._poses]
    raw = _np.asarray(state.pose_id, dtype="<u4").tobytes()
    return {
        "pose_table": table,
        "pose_ids": {"encoding": "u32le-base64", "count": int(state.pose_id.size),
                     "sha256": _hashlib.sha256(raw).hexdigest(),
                     "data": _base64.b64encode(raw).decode("ascii")},
        "digest": state.digest(),
    }


def kplus_matrix(ctx):
    """Return a K+ index lookup producing exact JSON matrix triples."""
    return lambda index: _matrix_json(ctx.kplus.matrix(index))


def letters_from_records(ctx, menu, records) -> list[list[int]]:
    """Match records to menu items using their exact transported matrices."""
    indices = {matrix: i for i, (_, matrix) in enumerate(menu.items)}
    frames, matched, letters = {}, {}, []
    for index, record in enumerate(records):
        grip = record["grip"]
        key = (grip, tuple(tuple(tuple(x) for x in row) for row in record["matrix"]))
        if key not in matched:
            if grip not in frames:
                frame = menu.frame(grip)
                frames[grip] = (frame, _transpose(frame))
            frame, inverse = frames[grip]
            matrix = _Twist.from_record(ctx, record).matrix
            local = _to_tuple(_matmul(_matmul(inverse, matrix), frame))
            if local not in indices:
                raise ValueError(f"record {index}: exact matrix has no item in the menu")
            matched[key] = indices[local]
        letters.append([grip, matched[key]])
    return letters


def twists_from_letters(menu, letters):
    """Return J1 menu twists, caching each grip's transported item list."""
    caps, twists = {}, []
    for grip, item in letters:
        if grip not in caps:
            caps[grip] = menu.for_cap(grip)
        twists.append(caps[grip][item])
    return twists
