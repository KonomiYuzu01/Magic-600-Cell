"""Export the whole W-J state through magic600-jumbling-render/1, without changing J1.

Geometry is copied byte-for-byte after manifest and shape checks. wj.export writes the
state arrays; an independent sweep-before replay certifies the moving set. Generated
files default to full/ (ignored); use --out or --scratch for isolated checks.
"""
from __future__ import annotations

import argparse
import base64
import hashlib
import json
import math
import sys
import tempfile
from pathlib import Path

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
OUT = HERE / 'full'
sys.path.insert(0, str(HERE.parent / 'fixtures'))
sys.path.insert(0, str(ROOT / 'tools' / 'perf'))

import numpy as np  # noqa: E402
import wj  # noqa: E402
from check_renderer_assets import check as check_assets  # noqa: E402

COUNTS = {'base_vertices': 30480, 'base_stickers': 433, 'cells': 600,
          'slots': 259800, 'pieces': 177120}
GEOMETRY = {'mesh_vertices.f32': ('<f4', [30480, 4]),
            'mesh_sticker.u32': ('<u4', [30480]),
            'mesh_centers.f32': ('<f4', [433, 4]),
            'cell_frames.f32': ('<f4', [600, 4, 4]),
            'slot_piece.u32': ('<u4', [259800])}


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=1, allow_nan=False) + '\n', encoding='utf-8')


def geometry(ctx, out):
    problems = check_assets()
    if problems:
        raise ValueError('; '.join(problems))
    manifest_raw = (ROOT / 'assets' / 'manifest.json').read_bytes()
    manifest = json.loads(manifest_raw)
    files = {}
    for name in (*GEOMETRY, 'mesh.json'):
        raw = (ROOT / 'assets' / name).read_bytes()
        if digest(raw) != manifest['files'].get(f'assets/{name}'):
            raise ValueError(f'{name}: manifest SHA-256 mismatch')
        desc = {'sha256': digest(raw), 'bytes': len(raw)}
        if name in GEOMETRY:
            dtype, shape = GEOMETRY[name]
            a = np.frombuffer(raw, dtype=dtype)
            if a.size != math.prod(shape) or (a.dtype.kind == 'f' and not np.isfinite(a).all()):
                raise ValueError(f'{name}: wrong shape or non-finite geometry')
            desc.update(dtype=dtype, shape=shape)
        files[name] = (raw, desc)
    mesh = json.loads(files['mesh.json'][0])
    if any(mesh.get(k) != v for k, v in COUNTS.items() if k != 'pieces'):
        raise ValueError('mesh.json: not the full model')
    if (any((b - a) % 3 for a, b in zip(mesh['offsets'], mesh['offsets'][1:]))
            or len(mesh['normal']) != 4 or not all(map(math.isfinite, mesh['normal']))
            or not math.isfinite(mesh['normal_length']) or mesh['normal_length'] <= 0):
        raise ValueError('mesh.json: invalid triangle ranges or normal')
    pieces = np.frombuffer(files['slot_piece.u32'][0], '<u4')
    if (not np.array_equal(pieces, ctx.data.slot_piece)
            or not np.array_equal(np.unique(pieces), np.arange(COUNTS['pieces']))):
        raise ValueError('slot_piece.u32: must cover all J1 pieces unchanged')
    # The asset frame and J1 must describe the same coordinates, not only the same ids.
    frames = np.frombuffer(files['cell_frames.f32'][0], '<f4').reshape(600, 4, 4).transpose(0, 2, 1)
    if not np.allclose(frames, ctx.data.frames, rtol=0, atol=1e-7):
        raise ValueError('facet frames differ from J1')
    out.mkdir(parents=True, exist_ok=True)
    for name, (raw, _) in files.items():
        (out / name).write_bytes(raw)
    write_json(out / 'geometry.json', {
        'format': 'magic600-jumbling-geometry/1', 'dimension': 4, **COUNTS,
        'model_identity': ctx.data.identity, 'retained_model_id': manifest['model_id'],
        'manifest_sha256': digest(manifest_raw),
        'files': {name: desc for name, (_, desc) in files.items()},
        'frame_convention': 'cell_frames column-major; vertices in the asset frame',
    })
    print('geometry: manifest hashes and shapes ok; 600 facets, 259800 stickers, 177120 pieces', flush=True)


def checked_state(st, ref, where):
    if st.digest() != ref['digest']:
        raise ValueError(f'{where}: J1 digest mismatch')
    arrays = wj.arrays(st)
    for name, a in arrays.items():
        if {'sha256': wj.sha(a), 'shape': list(a.shape), 'dtype': a.dtype.str} != ref['arrays'][name]:
            raise ValueError(f'{where}/{name}: fixture hash, shape or type mismatch')
    off = int((arrays['pose_lattice.i32'][arrays['pose_index.i32']] < 0).sum())
    if 'off_lattice' in ref and off != ref['off_lattice']:
        raise ValueError(f'{where}: fixture lattice count mismatch')
    return arrays, off


def exported_state(name, stage, directory, ref, out):
    # Use the engine-side contract writer itself; do not invent another state format.
    wj.export(name, stage, directory)
    header = json.loads((directory / 'header.json').read_text())
    if header['dimension'] != 4 or header['pieces'] != COUNTS['pieces'] or header['digest'] != ref['digest']:
        raise ValueError(f'{name}/{stage}: invalid exported header')
    arrays = {}
    for fname, expected in ref['arrays'].items():
        raw = (directory / fname).read_bytes()
        if digest(raw) != expected['sha256'] or header['files'][fname] != expected:
            raise ValueError(f'{name}/{stage}/{fname}: exported array differs from the fixture')
        arrays[fname] = np.frombuffer(raw, expected['dtype']).reshape(expected['shape'])
    off = int((arrays['pose_lattice.i32'][arrays['pose_index.i32']] < 0).sum())
    if 'off_lattice' in ref and off != ref['off_lattice']:
        raise ValueError(f'{name}/{stage}: off-lattice count mismatch')
    print(f'{name}/{stage}: digest and 3 array hashes ok; {off} off the lattice', flush=True)
    return {'header': directory.relative_to(out).as_posix() + '/header.json',
            'revision': header['revision'], 'digest': header['digest'], 'off_lattice': off}


def overlay(ctx, name, fixture, menu, out):
    sw = fixture['sweep']
    records = fixture['journal']['records']
    st = wj.sim.State.replay({**fixture['journal'], 'records': records[:sw['record']]}, ctx=ctx, menu=menu)
    _, before_off = checked_state(st, sw['before'], f'{name}/sweep-before')
    cl = st.classify(sw['grip'])
    if cl.status != 'admissible':
        raise ValueError(f'{name}: swept grip is {cl.status}')
    moving = np.array(sorted(int(p) for p in cl.inside), '<i4')
    if len(moving) != sw['moved'] or wj.sha(moving) != sw['moved_sha256']:
        raise ValueError(f'{name}: sorted J1 moving-set hash differs from the fixture')
    tw = wj.Twist.from_record(ctx, records[sw['record']])
    u, v = np.array(sw['plane_u']), np.array(sw['plane_v'])
    angle = math.radians(sw['angle_deg'])
    g = np.array([[wj.to_float(x) for x in row] for row in tw.matrix])
    if (tw.grip != sw['grip'] or not 0 < angle < math.pi
            or not np.allclose([u @ u, v @ v, u @ v], [1, 1, 0], rtol=0, atol=1e-12)
            or not np.allclose(wj.family(u, v, angle), g, rtol=0, atol=1e-9)):
        raise ValueError(f'{name}: swept rotation does not match the exact twist')
    home, samples = [], sw['samples']
    if len(samples) != 16 or len({s['piece'] for s in samples}) != 16:
        raise ValueError(f'{name}: expected 16 distinct sweep samples')
    for sample in samples:
        p = sample['piece']
        if p not in moving:
            raise ValueError(f'{name}: sample piece outside moving set')
        x = np.array([wj.to_float(c) for c in wj.centroid(ctx, p)])
        home.append(x)
        start = wj.pose_float(st, int(st.pose_id[p])) @ x
        for key, phi in (('start', 0), ('mid', angle / 2), ('end', angle)):
            if not np.allclose(wj.family(u, v, phi) @ start, sample[key], rtol=1e-10, atol=1e-10):
                raise ValueError(f'{name}: sweep sample {p}/{key} differs from J1')
    applied = st.apply(tw)
    if not applied.applied or applied.moved != len(moving):
        raise ValueError(f'{name}: swept twist did not apply as recorded')
    _, after_off = checked_state(st, sw['after'], f'{name}/sweep-after')
    survey = fixture['end_survey']
    status, certs = survey['status'], survey['certificates']
    if (len(status) != COUNTS['cells'] or set(status) - {'a', 'b'}
            or status.count('b') != survey['blocked'] or status.count('a') != survey['admissible']
            or sorted(c['grip'] for c in certs) != [i for i, s in enumerate(status) if s == 'b']):
        raise ValueError(f'{name}: incomplete end survey')
    for c in certs:
        if not 0 <= c['piece'] < COUNTS['pieces']:
            raise ValueError(f'{name}: invalid certificate piece')
        for side in ('below', 'above'):
            point = [wj.Q5(*x) for x in c[f'point_{side}']]
            h = ctx.data.h(c['grip'], point)
            if not (h < wj.ZERO if side == 'below' else h > wj.ZERO):
                raise ValueError(f'{name}: invalid exact certificate sign')
            if not np.allclose([wj.to_float(x) for x in point], c[f'point_{side}_float'], rtol=0, atol=1e-12):
                raise ValueError(f'{name}: certificate float differs from its exact point')
    directory = out / name
    directory.mkdir(parents=True, exist_ok=True)
    files = {}
    for fname, a in {'moving.i32': moving, 'sweep_pieces.i32': np.array([s['piece'] for s in samples], '<i4'),
                     'sweep_home.f32': np.array(home, '<f4')}.items():
        raw = a.tobytes()
        (directory / fname).write_bytes(raw)
        files[fname] = {'sha256': digest(raw), 'shape': list(a.shape), 'dtype': a.dtype.str}
    write_json(directory / 'overlay.json', {
        'format': 'magic600-jumbling-overlay/1', 'dimension': 4, 'menu': name,
        **{k: fixture['journal'][k] for k in ('menu_identity', 'model_identity', 'contract_revision')},
        'fixture_sha256': digest(wj.fixture_path(name).read_bytes()),
        'end': {k: fixture['stages']['end'][k] for k in ('digest', 'records', 'off_lattice')},
        'grip_status': status, 'certificates': certs,
        'cut': {'alpha': '121/125', 'frame': 'asset frame; divide points by geometry normal_length'},
        'sweep': {'grip': sw['grip'], 'plane_u': sw['plane_u'], 'plane_v': sw['plane_v'], 'angle': angle,
                  'moving': {'file': 'moving.i32', 'count': len(moving), 'sha256': wj.sha(moving)},
                  'from_revision': sw['record'], 'to_revision': sw['record'] + 1,
                  'before': {**sw['before'], 'off_lattice': before_off},
                  'after': {**sw['after'], 'off_lattice': after_off}, 'samples': samples},
        'files': files,
    })
    print(f'{name}/overlay: before/after digests and hashes ok; moving set {len(moving)} '
          f'({wj.sha(moving)}); 16 sweep samples and {len(certs)} certificates ok', flush=True)


def export_selection(args, out):
    ctx = wj.sim.get_context()
    fixtures = {name: wj.load(ctx, name) for name in dict.fromkeys(args.menus)}
    first = next(iter(fixtures))
    solved_ref = fixtures[first][0]['stages']['start']
    for name, (f, _) in fixtures.items():
        if any(f['stages']['start'][k] != solved_ref[k] for k in ('digest', 'arrays', 'off_lattice')):
            raise ValueError(f'{name}: solved arrays differ across menus')
    geometry(ctx, out)
    solved = exported_state(first, 'start', out / 'solved', solved_ref, out)
    catalog = {'format': 'magic600-full-viewer/1', 'dimension': 4, 'solved': solved, 'menus': {}}
    for name, (fixture, menu) in fixtures.items():
        states = {}
        for stage in dict.fromkeys(args.stages):
            if stage == 'solved':
                continue
            ref = fixture['stages']['end'] if stage == 'end' else fixture['sweep']['before']
            states[stage] = exported_state(name, stage, out / name / stage, ref, out)
        overlay(ctx, name, fixture, menu, out)
        catalog['menus'][name] = {'identity': menu.identity, 'states': states, 'overlay': f'{name}/overlay.json'}
    write_json(out / 'catalog.json', catalog)
    if args.b64:
        destination = args.b64.resolve()
        # Only the files of this export, never stray files in a previous output directory.
        paths = [out / 'geometry.json', out / 'mesh.json', out / 'catalog.json',
                 *(out / n for n in GEOMETRY)]
        for name, item in catalog['menus'].items():
            paths.extend([out / name / n for n in ('overlay.json', 'moving.i32', 'sweep_pieces.i32', 'sweep_home.f32')])
            for state in item['states'].values():
                directory = (out / state['header']).parent
                paths.extend([directory / n for n in ('header.json', 'pose_index.i32', 'poses.f32', 'pose_lattice.i32')])
        paths.extend([out / 'solved' / n for n in ('header.json', 'pose_index.i32', 'poses.f32', 'pose_lattice.i32')])
        for path in paths:
            target = destination / path.relative_to(out)
            target.parent.mkdir(parents=True, exist_ok=True)
            if path.suffix == '.json':
                if target != path.resolve():
                    target.write_bytes(path.read_bytes())
            else:
                Path(str(target) + '.b64').write_text(base64.b64encode(path.read_bytes()).decode('ascii') + '\n', encoding='ascii')
        print('base64: headers copied and every binary encoded', flush=True)
    print('full export: ok (all requested fixture digests, array hashes and moving sets verified)', flush=True)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--menus', nargs='+', choices=wj.MENUS, default=list(wj.MENUS))
    ap.add_argument('--stages', nargs='+', choices=('solved', 'end', 'sweep-before'),
                    default=['solved', 'end', 'sweep-before'], help='solved is always written once; limit expensive stage replays')
    output = ap.add_mutually_exclusive_group()
    output.add_argument('--out', type=Path, help='output directory (default: viewer/full/)')
    output.add_argument('--scratch', action='store_true',
                        help='check in a fresh temporary directory outside the repository, then delete it')
    ap.add_argument('--b64', type=Path, help='also copy headers and write binaries as <name>.b64 under this directory')
    args = ap.parse_args(argv)
    if args.scratch and args.b64:
        ap.error('--scratch cannot retain --b64 output; use --out with --b64')
    try:
        if args.scratch:
            temporary_root = Path(tempfile.gettempdir()).resolve()
            if temporary_root.is_relative_to(ROOT):
                raise ValueError('the system temporary directory is inside the repository; set TMPDIR outside it')
            # The context removes a partially failed export too.
            with tempfile.TemporaryDirectory(prefix='magic600-full-', dir=temporary_root) as directory:
                export_selection(args, Path(directory))
            print('scratch: temporary output deleted; no repository files written', flush=True)
        else:
            export_selection(args, (args.out or OUT).resolve())
        return 0
    except (OSError, ValueError, AssertionError, KeyError, TypeError) as exc:
        print(f'full export: FAIL ({exc})', file=sys.stderr, flush=True)
        return 1


if __name__ == '__main__':
    sys.exit(main())
