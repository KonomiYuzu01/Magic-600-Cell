"""Prove the pose-to-slot handoff for all stickers, including mesh vertex sets."""
from __future__ import annotations

import hashlib
import io
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
from fixture_data import ROOT, snapshot, write_state, wj, np
sys.path.insert(0, str(ROOT / 'work/experiments/renderer-sb'))
from reference_geometry import ANCHOR_SHA256, sticker_anchors


def asset(name):
    """Return the bytes of an asset, refusing any whose SHA-256 differs from the manifest."""
    manifest = json.loads((ROOT / 'assets/manifest.json').read_bytes())
    raw = (ROOT / name).read_bytes()
    if hashlib.sha256(raw).hexdigest() != manifest['files'].get(name):
        raise ValueError(f'{name}: SHA-256 digest mismatch with manifest.json')
    return raw


def check_frame(ctx):
    mesh = json.loads(asset('assets/mesh.json'))
    with np.load(io.BytesIO(asset('assets/model.npz'))) as model:
        poles = model['normals'].copy()
        frames64 = model['frames'].copy()
    radius = mesh['normal_length']
    normal = np.array(mesh['normal']) / radius
    if (not np.allclose(np.linalg.norm(poles, axis=1), radius, atol=1e-8, rtol=0)
            or not np.allclose(np.einsum('cij,j->ci', frames64, normal), poles / radius, atol=1e-8, rtol=0)):
        raise ValueError('J1/frame/pole length disagreement')
    frames = np.frombuffer(asset('assets/cell_frames.f32'), '<f4').reshape(600, 4, 4).transpose(0, 2, 1)
    if (not np.allclose(frames, frames64, atol=1e-7, rtol=0)
            or not np.allclose(np.einsum('cij,j->ci', frames, normal), poles / radius, atol=1e-8, rtol=0)):
        raise ValueError('float32 frame/pole disagreement')
    pieces = np.frombuffer(asset('assets/slot_piece.u32'), '<u4')
    if not np.array_equal(pieces, ctx.data.slot_piece):
        raise ValueError('asset sticker-to-piece ids disagree with J1')
    print('frame check: pole length and all 600 float32 frame transports within 1e-8', flush=True)


def controls(ctx, out=None, *, require_agreement=True):
    mesh = json.loads(asset('assets/mesh.json'))
    frames = np.frombuffer(asset('assets/cell_frames.f32'), '<f4').astype(float).reshape(600, 4, 4).transpose(0, 2, 1)
    verts = np.frombuffer(asset('assets/mesh_vertices.f32'), '<f4').astype(float).reshape(-1, 4)
    anchors = np.array(sticker_anchors(mesh['offsets'], verts.ravel().tolist()), dtype='<f4')
    if hashlib.sha256(anchors.tobytes()).hexdigest() != ANCHOR_SHA256:
        raise ValueError('shrink anchors: SHA-256 mismatch with SPEC section 3')
    centers = anchors.astype(np.float64).reshape(433, 4)
    local = np.frombuffer(asset('assets/mesh_sticker.u32'), '<u4')
    normal, radius = np.array(mesh['normal']), mesh['normal_length']
    shrunk = (normal + .76 * (centers[local] - normal) + .76 * .82 * (verts - centers[local])) / radius
    center_home = (normal + .76 * (centers - normal)) / radius
    world_centers = np.einsum('cij,sj->csi', frames, center_home).reshape(-1, 4)
    st = wj.sim.State(ctx)
    for name in ('lattice-start', 'lattice-retained'):
        if name == 'lattice-retained':
            outcome = st.apply(wj.sim.twists.primitive(ctx, 1))
            if not outcome.applied:
                raise ValueError('J1 retained control failed')
        projection = st.lattice_stickers()
        dest, labels = projection['slot'], projection['labels']
        captured = snapshot(st)
        data = captured['arrays']
        mats = data['poses.f32'].astype(float)
        ids = data['pose_index.i32'][ctx.data.slot_piece]
        transformed = np.einsum('sij,sj->si', mats[ids], world_centers)
        if (not np.all(projection['frame_agrees'])
                or not np.array_equal(labels[dest], np.arange(259800))):
            raise ValueError(f'{name}: all-slot labels disagree')
        center_error = np.max(np.abs(transformed - world_centers[dest]), axis=1)
        # Triangulations may have different interior vertices. Unequal world-axis
        # bounds are a sufficient counterexample to equal sticker geometry.
        max_error = 0.0
        for slot in np.flatnonzero(ids).tolist():
            cell, loc = divmod(slot, 433)
            target_cell, target_loc = divmod(int(dest[slot]), 433)
            start, end = mesh['offsets'][loc:loc + 2]
            tstart, tend = mesh['offsets'][target_loc:target_loc + 2]
            left = shrunk[start:end] @ frames[cell].T @ mats[ids[slot]].T
            right = shrunk[tstart:tend] @ frames[target_cell].T
            error = max(float(np.abs(left.min(axis=0) - right.min(axis=0)).max()),
                        float(np.abs(left.max(axis=0) - right.max(axis=0)).max()))
            max_error = max(max_error, error)
        if out is not None:
            document = {'menu_name': 'retained-control', 'journal': json.loads(st.journal_json())}
            directory = out / name
            write_state(directory, document, name, captured, len(st.journal))
            for file, values in (('labels.u32', labels), ('destination.u32', dest)):
                (directory / file).write_bytes(values.astype('<u4').tobytes())
            header = json.loads((directory / 'header.json').read_bytes())
            for file in ('labels.u32', 'destination.u32'):
                raw = (directory / file).read_bytes()
                header['files'][file] = {'sha256': hashlib.sha256(raw).hexdigest(), 'shape': [259800], 'dtype': '<u4'}
            (directory / 'header.json').write_text(json.dumps(header, indent=2) + '\n', encoding='utf-8')
        status = 'fail' if max_error > 2e-6 or center_error.max() > 2e-6 else 'pass'
        print(f'{name}: all 259800 labels pass; geometry bounds {status}; '
              f'max bound error {max_error:.9g}; max center error {center_error.max():.9g}', flush=True)
        if require_agreement and status == 'fail':
            raise ValueError(f'{name}: fixed home shrink does not agree with S-B slot geometry')
