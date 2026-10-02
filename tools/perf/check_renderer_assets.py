"""Check the shared renderer workload assets without NumPy or file writes."""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import sys
from array import array
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ASSETS = ROOT / 'assets'
FILES = ('mesh_vertices.f32', 'mesh_sticker.u32', 'mesh.json', 'cell_frames.f32', 'slot_piece.u32')
COUNTS = {'base_vertices': 30480, 'base_stickers': 433, 'cells': 600, 'slots': 259800}
SUCCESS = 'renderer assets: ok (600 cells x 433 stickers = 259,800 slots; 30,480 base vertices)'


def json_object(data, name, problems):
    if data is None:
        return None
    try:
        value = json.loads(data)
    except (ValueError, UnicodeError):
        problems.append(f'assets/{name}: invalid JSON')
        return None
    if not isinstance(value, dict):
        problems.append(f'assets/{name}: expected a JSON object')
        return None
    return value


def check():
    """Return one diagnostic per failed digest or shape check; read only ASSETS."""
    problems, data = [], {}
    for name in ('manifest.json', *FILES):
        try:
            data[name] = (ASSETS / name).read_bytes()
        except OSError:
            problems.append(f'assets/{name}: cannot read file')

    manifest = json_object(data.get('manifest.json'), 'manifest.json', problems)
    if manifest is not None:
        digests = manifest.get('files')
        if not isinstance(digests, dict):
            problems.append('assets/manifest.json: expected a files mapping of SHA-256 digests')
        else:
            for name in FILES:
                if name in data and hashlib.sha256(data[name]).hexdigest() != digests.get(f'assets/{name}'):
                    problems.append(f'assets/{name}: SHA-256 digest mismatch with manifest.json')

    mesh = json_object(data.get('mesh.json'), 'mesh.json', problems)
    offsets_ok = False
    if mesh is not None:
        for name, expected in COUNTS.items():
            if type(mesh.get(name)) is not int or mesh[name] != expected:
                problems.append(f'assets/mesh.json: {name} must be {expected}')
        if (type(mesh.get('cells')) is int and type(mesh.get('base_stickers')) is int
                and mesh.get('slots') != mesh['cells'] * mesh['base_stickers']):
            problems.append('assets/mesh.json: slots must equal cells x base_stickers')
        offsets = mesh.get('offsets')
        offsets_ok = (isinstance(offsets, list) and len(offsets) == COUNTS['base_stickers'] + 1
                      and all(type(value) is int for value in offsets)
                      and offsets[0] == 0 and offsets[-1] == COUNTS['base_vertices']
                      and all(before < after for before, after in zip(offsets, offsets[1:])))
        if not offsets_ok:
            problems.append('assets/mesh.json: offsets must be 434 strictly increasing integers from 0 to 30480')

    def binary(name, code, count):
        raw = data.get(name)
        if raw is None:
            return None
        if len(raw) != count * 4:
            problems.append(f'assets/{name}: size must be {count * 4:,} bytes (got {len(raw):,})')
            return None
        values = array(code)
        if values.itemsize != 4:
            problems.append(f'assets/{name}: this platform does not provide 32-bit array({code!r})')
            return None
        values.frombytes(raw)
        if sys.byteorder != 'little':
            values.byteswap()
        if code == 'f' and not all(math.isfinite(value) for value in values):
            problems.append(f'assets/{name}: non-finite float32 value')
        return values

    binary('mesh_vertices.f32', 'f', COUNTS['base_vertices'] * 4)
    stickers = binary('mesh_sticker.u32', 'I', COUNTS['base_vertices'])
    binary('cell_frames.f32', 'f', COUNTS['cells'] * 16)
    binary('slot_piece.u32', 'I', COUNTS['slots'])
    if stickers is not None:
        if any(value >= COUNTS['base_stickers'] for value in stickers):
            problems.append('assets/mesh_sticker.u32: sticker index outside 0..432')
        if offsets_ok and any(stickers[vertex] != sticker
                              for sticker, (begin, end) in enumerate(zip(offsets, offsets[1:]))
                              for vertex in range(begin, end)):
            problems.append('assets/mesh_sticker.u32: vertex sticker indices do not match mesh.json offsets')
    return problems


def main(argv=None):
    argparse.ArgumentParser(description=__doc__).parse_args(argv)
    problems = check()
    for problem in problems:
        print(problem)
    if not problems:
        print(SUCCESS)
    return int(bool(problems))


if __name__ == '__main__':
    sys.exit(main())
