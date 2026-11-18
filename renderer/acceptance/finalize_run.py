"""Finalize framework/DLL/PresentMon evidence without changing its inputs.

C9 acceptance copy of work/experiments/renderer-l2/finalize_run.py (see PROVENANCE.md): the Godot host (sa2)
only, with run, geometry and lattice modes, and W-J's scenes (ABI 3) beside W1..W4.
"""
from __future__ import annotations

import sys

sys.dont_write_bytecode = True

import argparse
import bisect
import copy
import csv
import hashlib
import json
import math
import ntpath
import os
import re
import struct
import uuid
from pathlib import Path

import file_guard
from file_guard import SHADERS

REPOSITORY = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPOSITORY / 'tools' / 'perf'))
import renderer_gate as gate

DEFAULT_ADAPTER = 'NVIDIA GeForce RTX 4070 Laptop GPU'
OUTPUTS = ('run.json', 'fault-run.json', 'geometry-record.json', 'lattice-record.json', 'refusal.json')
SCENES = ('w1', 'w2', 'w3', 'w4', 'wj')
# W-J's menus and the fixture names its native record reports (renderer/core/src/scene_record.cpp).
MENUS = {'S4': 'wj-s4', 'I_a': 'wj-i-a', 'I_b': 'wj-i-b'}
MODES = ('run', 'geometry', 'lattice')
CHECKS = {
    'harness': 'The harness is missing, malformed or does not match the launched app.',
    'app-exit': 'The app exit is inconsistent or is neither zero nor two.',
    'native': 'The DLL outputs are missing, malformed or inconsistent with the harness.',
    'identity': 'A required build part is missing or its digest does not match.',
    'adapter': 'The framework adapter does not match the expected adapter name.',
    'size-mismatch': 'The physical sizes differ or changed during the trace.',
    'scaling': 'The framework texture mapping is not one physical pixel per target pixel.',
    'conditions-not-enforced': 'The capture did not enforce visibility and foreground conditions.',
    'conditions': 'The condition samples contradict the trace or the declared conditions.',
    'debug-switch': 'A debug switch or the device debug layer is enabled for a gate run.',
    'vram-missing': 'Peak process-local VRAM is not a positive finite number.',
    'presentmon': 'The PresentMon CSV is incomplete, malformed or has no matching trace rows.',
    'swap-chain': 'More than one swap chain presented during the trace.',
    'sync-interval': 'A trace present has a nonzero sync interval.',
    'blind-seconds': 'A checked second contains no displayed present.',
    'trace-steps': 'A checked present step does not contain exactly one trace entry.',
    'operator': 'The gate run has no operator confirmation or fault-injection declaration.',
    'gate-shape': 'The completed record does not satisfy the renderer gate input contract.',
    'output-exists': 'A finalizer output already exists; no file was written.',
}
CSV_COLUMNS = ('ProcessID', 'SwapChainAddress', 'QPCTime', 'msBetweenPresents',
               'Dropped', 'SyncInterval', 'PresentMode', 'AllowsTearing')
# The DLL checks its geometry against S-B's reference set and verifies the files' digests in this index.
REFERENCE_INDEX = REPOSITORY / 'work' / 'experiments' / 'renderer-sb' / 'reference' / 'index.json'
LABEL_COUNTERS = ('mismatches', 'binding_mismatches', 'late_adoptions', 'missing')
# W-J's geometry references (ref_<menu>_index.json), checked by the DLL against their digests.
WJ_REFERENCE = REPOSITORY / 'work' / 'experiments' / 'renderer-wj'
# W-J's label and pose checks share these frame-record counters and add hash failures (renderer/core/src/wj/probe.cpp).
WJ_RECORD_COUNTERS = ('missing', 'late_adoptions', 'binding_mismatches')
WJ_COUNTERS = (*WJ_RECORD_COUNTERS, 'mismatches', 'hash_failures')
WJ_CELLS, WJ_VERTICES_PER_CELL, WJ_SLOTS = 600, 30480, 259800
LATTICE_STATES = ('lattice-start', 'lattice-retained')
LATTICE_COUNTERS = ('label_failures', 'center_failures', 'geometry_bounds_failures', 'lattice_failures')
POSE_UPLOAD = 'whole index, matrix table and lattice table; atomic fenced bundle'


def require(condition):
    if not condition:
        raise ValueError('invalid evidence')


def finite(value):
    return gate.number(value)


def integer(value):
    return type(value) is int


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'),
                      ensure_ascii=False, allow_nan=False).encode('utf-8')


def reject_constant(value):
    raise ValueError('non-finite JSON')


def object_pairs(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result)
        result[key] = value
    return result


def read_json(path):
    with path.open(encoding='utf-8') as stream:
        value = json.load(stream, parse_constant=reject_constant, object_pairs_hook=object_pairs)
    # Also reject numbers such as 1e999, which parse_constant does not see.
    canonical(value)
    require(isinstance(value, dict))
    return value


def fields(value, names):
    require(isinstance(value, dict) and all(name in value for name in names))


def read_harness(directory, candidate, mode, pid):
    h = read_json(directory / 'harness.json')
    fields(h, ('format', 'candidate', 'mode', 'run_id', 'scene', 'menu', 'data', 'process_id', 'exit_code',
               'reason', 'options', 'configuration', 'files', 'dll_identity', 'dll_status',
               'qpc_frequency', 'sizes', 'scaling', 'dpi_awareness', 'environment', 'window', 'debug'))
    require(h['format'] == 'magic600-l2-harness-v1' and h['candidate'] == candidate == 'sa2')
    require(integer(h['process_id']) and h['process_id'] == pid)
    require(mode in MODES and h['mode'] == mode)
    require(isinstance(h['run_id'], str)
            and re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9._-]{0,127}', h['run_id']) is not None)
    require(h['scene'] in SCENES or (mode == 'geometry' and h['scene'] is None))
    require(mode != 'lattice' or h['scene'] == 'wj')
    if h['scene'] == 'wj':
        require(h['menu'] in MENUS and isinstance(h['data'], str) and ntpath.isabs(h['data']))
    else:
        require(h['menu'] is None and h['data'] is None)
    require(integer(h['exit_code']))
    fields(h['options'], ('trace_ms', 'preroll_ms', 'turn_ms', 'inject', 'declared',
                          'gpu_validation', 'conditions', 'no_vram', 'debug_half_target'))
    require(isinstance(h['options']['declared'], dict))
    require(all(type(h['options'][key]) is bool
                for key in ('gpu_validation', 'no_vram', 'debug_half_target')))
    require(all(isinstance(h[key], dict) for key in
                ('configuration', 'files', 'dll_identity', 'dll_status', 'sizes', 'scaling',
                 'environment', 'window', 'debug')))
    fields(h['sizes'], ('display', 'window', 'backbuffer', 'displayed', 'target', 'samples', 'samples_changed'))
    fields(h['environment'], ('adapter', 'declared', 'display', 'backbuffer', 'power_source', 'power_samples'))
    require(isinstance(h['environment']['declared'], dict))
    fields(h['window'], ('topmost', 'display_required', 'foreground_at_trace_start', 'sample_period_ms',
                         'samples', 'samples_not_visible', 'samples_covered', 'samples_not_foreground',
                         'visible_throughout', 'foreground_throughout', 'presents'))
    fields(h['debug'], ('enabled', 'debug_layer', 'counts', 'messages'))
    return h


def size(value):
    fields(value, ('width', 'height'))
    require(all(integer(value[key]) and value[key] > 0 for key in ('width', 'height')))
    return value['width'], value['height']


def geometry_reference():
    """Camera and pose pairs, sample count, cells and per-cell count of S-B's reference set."""
    index = read_json(REFERENCE_INDEX)
    pairs = set()
    for name in index['files']:
        stem, _, suffix = name.rpartition('.')
        if suffix == 'f32':
            camera, _, pose = stem.rpartition('_')
            pairs.add((camera, pose))
    require(pairs and all(integer(index[key]) and index[key] > 0
                          for key in ('sample_count', 'cells', 'per_cell_vertex_count')))
    return pairs, index['sample_count'], index['cells'], index['per_cell_vertex_count']


def wj_reference(menu):
    """Projection files and sample count of W-J's reference set for one menu."""
    index = read_json(WJ_REFERENCE / f'ref_{menu}_index.json')
    require(index.get('menu') == menu and isinstance(index.get('outputs'), list) and index['outputs'])
    files = [output.get('file') for output in index['outputs'] if isinstance(output, dict)]
    require(len(files) == len(index['outputs']) == len(set(files))
            and all(isinstance(name, str) and name.endswith('.f32') for name in files))
    require(integer(index.get('samples')) and index['samples'] > 0)
    return set(files), index['samples']


def check_wj_geometry(g, menu):
    """W-J's geometryCheck record: all 18 projections and the per-cell invocation count."""
    fields(g, ('format', 'status', 'build_identity', 'adapter', 'results', 'cells', 'vertices_per_cell',
               'invocation_count_failures'))
    require(g['format'] == 'magic600-wj-geometry-check/1' and g['status'] in ('pass', 'fail'))
    require(isinstance(g['build_identity'], str)
            and re.fullmatch(r'[0-9a-f]{64}', g['build_identity']) is not None
            and isinstance(g['adapter'], str) and isinstance(g['results'], list))
    require(g['cells'] == WJ_CELLS and g['vertices_per_cell'] == WJ_VERTICES_PER_CELL)
    files, samples = wj_reference(menu)
    if 'error' in g:
        # The DLL's error record: a refused reference or stage export, no comparison ran.
        require(g['status'] == 'fail' and g['results'] == [] and g['invocation_count_failures'] is None
                and isinstance(g['error'], str) and bool(g['error']))
        return
    failures = g['invocation_count_failures']
    require(integer(failures) and 0 <= failures <= WJ_CELLS)
    seen = set()
    for result in g['results']:
        fields(result, ('file', 'samples', 'max_abs_error', 'failures', 'status'))
        require(result['file'] in files and result['file'] not in seen)
        seen.add(result['file'])
        # NDC x/y and clip w per sample, each compared with W-J's tolerance.
        require(result['samples'] == samples and integer(result['failures'])
                and 0 <= result['failures'] <= 3 * samples)
        require(result['status'] == ('fail' if result['failures'] else 'pass')
                and finite(result['max_abs_error']) and result['max_abs_error'] >= 0)
    require(seen == files)
    passed = failures == 0 and all(result['status'] == 'pass' for result in g['results'])
    require(g['status'] == ('pass' if passed else 'fail'))


def check_lattice(g):
    """W-J's latticeCheck record: both controls at every slot."""
    fields(g, ('format', 'status', 'build_identity', 'adapter', 'states'))
    require(g['format'] == 'magic600-wj-lattice-check/1' and g['status'] in ('pass', 'fail'))
    require(isinstance(g['build_identity'], str)
            and re.fullmatch(r'[0-9a-f]{64}', g['build_identity']) is not None
            and isinstance(g['adapter'], str) and isinstance(g['states'], list))
    if 'error' in g:
        require(g['status'] == 'fail' and g['states'] == [] and isinstance(g['error'], str) and bool(g['error']))
        return
    require([state.get('state') if isinstance(state, dict) else None for state in g['states']] == list(LATTICE_STATES))
    for state in g['states']:
        fields(state, ('state', 'status', 'slots', *LATTICE_COUNTERS))
        require(state['slots'] == WJ_SLOTS
                and all(integer(state[key]) and 0 <= state[key] <= WJ_SLOTS for key in LATTICE_COUNTERS))
        require(state['status'] == ('fail' if any(state[key] for key in LATTICE_COUNTERS) else 'pass'))
    require(g['status'] == ('pass' if all(state['status'] == 'pass' for state in g['states']) else 'fail'))


def wj_counters(value):
    fields(value, (*WJ_COUNTERS, 'required', 'status'))
    require(value['status'] in ('pass', 'fail'))
    require(all(integer(value[key]) and value[key] >= 0 for key in WJ_COUNTERS))
    required = value['required']
    require(isinstance(required, list) and all(integer(r) and r >= 0 for r in required)
            and all(a < b for a, b in zip(required, required[1:])))
    return all(value[key] == 0 for key in WJ_COUNTERS)


def check_wj_labels(labels, entries, unreached):
    """W-J's checkLabels: every counter zero, and an injection not reached fails it with clean counters."""
    clean = wj_counters(labels)
    fields(labels, ('copies', 'revisions'))
    require(integer(labels['copies']) and labels['copies'] >= 0)
    # One frame-use record per drawn trace frame requires that frame's turn.
    require(labels['required'] == sorted({entry['turn'] for entry in entries}))
    require(labels['revisions'] == len(labels['required']))
    require(labels['status'] == ('pass' if clean and not unreached else 'fail'))
    if clean:
        # No required revision without exactly one first-use copy and no copy of another revision.
        require(labels['copies'] == labels['revisions'])


def check_pose(pose, native_pose, labels, unreached):
    """W-J's checkPoses (pose_check.json) agrees with its parts, with the label check and with native.json."""
    clean = wj_counters(pose)
    fields(pose, ('revisions', 'checks'))
    require(all(pose[key] == labels[key] for key in (*WJ_RECORD_COUNTERS, 'required')))
    require(pose.get('injection_not_reached', False) is unreached
            and ('injection_not_reached' in pose) == unreached)
    require(isinstance(pose['checks'], list) and len(pose['checks']) == labels['copies'])
    matched, mismatches, hash_failures = set(), 0, 0
    for check in pose['checks']:
        fields(check, ('revision', 'frame', 'resource', 'pose_count', 'arrays', 'status'))
        require(all(integer(check[key]) and check[key] >= 0 for key in ('revision', 'frame', 'pose_count')))
        require(check['resource'] in (1, 2, 3) and check['status'] in ('pass', 'fail'))
        require(isinstance(check['arrays'], list) and len(check['arrays']) == 3)
        equal = True
        for array in check['arrays']:
            fields(array, ('sha256', 'expected_sha256', 'sha256_equal', 'element_equal', 'element_mismatches'))
            require(all(isinstance(array[key], str) and re.fullmatch(r'[0-9a-f]{64}', array[key]) is not None
                        for key in ('sha256', 'expected_sha256')))
            require(array['sha256_equal'] is (array['sha256'] == array['expected_sha256']))
            require(integer(array['element_mismatches']) and array['element_mismatches'] >= 0
                    and array['element_equal'] is (array['element_mismatches'] == 0))
            mismatches += array['element_mismatches']
            hash_failures += not array['sha256_equal']
            equal = equal and array['sha256_equal'] and array['element_equal']
        # A pass also needs the logical pose count, which only the DLL knows.
        require(equal or check['status'] == 'fail')
        if check['status'] == 'pass':
            matched.add(check['revision'])
    require(pose['mismatches'] == mismatches and pose['hash_failures'] == hash_failures)
    require(pose['revisions'] == sorted(matched))
    passed = clean and not unreached and pose['revisions'] == pose['required']
    require(pose['status'] == ('pass' if passed else 'fail'))
    require(native_pose == {'status': pose['status'], 'revisions': pose['revisions']})


def check_labels(labels, entries):
    """S-B's checkLabels passes only when every failure counter is zero; a summary must agree with its counters."""
    fields(labels, ('status', 'copies', 'revisions', *LABEL_COUNTERS, 'oracle_sha256'))
    fields(labels['oracle_sha256'], ('even', 'odd'))
    require(labels['status'] in ('pass', 'fail'))
    require(all(integer(labels[key]) and labels[key] >= 0 for key in ('copies', 'revisions', *LABEL_COUNTERS)))
    require(labels['binding_mismatches'] <= labels['mismatches'])
    unreached = labels.get('injection_not_reached', False)
    require(type(unreached) is bool)
    clean = not unreached and all(labels[key] == 0 for key in LABEL_COUNTERS)
    require(labels['status'] == ('pass' if clean else 'fail'))
    if clean:
        # A clean check adopted each turn's revision on time with exactly one copy, and the trace
        # records the adopted revision per frame, so the check covered every revision the trace reached.
        last = max((entry['revision'] for entry in entries), default=0)
        require(labels['copies'] == labels['revisions'] == last)


def check_sb_geometry(g):
    """S-B's geometry/count record (W1..W4)."""
    fields(g, ('format', 'status', 'build_identity', 'adapter', 'results', 'per_cell_count'))
    require(g['format'] == 'magic600-sb-geometry-check-v1' and g['status'] in ('pass', 'fail'))
    require(isinstance(g['build_identity'], str)
            and re.fullmatch(r'[0-9a-f]{64}', g['build_identity']) is not None
            and isinstance(g['adapter'], str) and isinstance(g['results'], list))
    pairs, samples, cells, expected = geometry_reference()
    seen = set()
    for result in g['results']:
        fields(result, ('camera', 'pose', 'samples', 'max_abs_error', 'failures', 'status'))
        pair = (result['camera'], result['pose'])
        require(pair in pairs and pair not in seen)
        seen.add(pair)
        # The DLL compares three coordinates per sample and counts each one that fails.
        require(result['samples'] == samples and integer(result['failures'])
                and 0 <= result['failures'] <= 3 * samples)
        require(result['status'] == ('fail' if result['failures'] else 'pass')
                and finite(result['max_abs_error']) and result['max_abs_error'] >= 0)
    counts = g['per_cell_count']
    fields(counts, ('cells', 'expected', 'failures', 'counts', 'status'))
    require(counts['cells'] == cells and counts['expected'] == expected)
    if counts['status'] == 'not-checked':
        # The DLL's error record: no comparison ran.
        require(g['status'] == 'fail' and not g['results'] and isinstance(g.get('error'), str)
                and bool(g['error']) and counts['counts'] is None and counts['failures'] is None)
    else:
        # A checked record compares every reference pair, and each summary agrees with its parts.
        require(seen == pairs and isinstance(counts['counts'], list) and len(counts['counts']) == cells
                and all(integer(value) and value >= 0 for value in counts['counts']))
        require(counts['failures'] == sum(value != expected for value in counts['counts']))
        require(counts['status'] == ('fail' if counts['failures'] else 'pass'))
        passed = counts['status'] == 'pass' and all(result['status'] == 'pass' for result in g['results'])
        require(g['status'] == ('pass' if passed else 'fail'))


def read_native(directory, h, mode):
    """Return the native record, the trace entries and W-J's full pose check."""
    if mode in ('geometry', 'lattice'):
        # An untimed check writes only its own record.
        other = {'geometry': 'lattice_check.json', 'lattice': 'geometry.json'}[mode]
        require(not any(os.path.lexists(directory / name)
                        for name in ('native.json', 'trace.jsonl', 'pose_check.json', other)))
        g = read_json(directory / ('geometry.json' if mode == 'geometry' else 'lattice_check.json'))
        if mode == 'lattice':
            check_lattice(g)
        elif h['scene'] == 'wj':
            check_wj_geometry(g, h['menu'])
        else:
            check_sb_geometry(g)
        return g, None, None
    n = read_json(directory / 'native.json')
    fields(n, ('format', 'scene', 'qpc_frequency', 'markers', 'frames', 'injection_applied',
               'vram_peak_mb', 'vram_samples', 'target', 'viewport', 'identity', 'queue_mode', 'barrier_api'))
    require(n['format'] == 'magic600-sa2-scene-native-v1' and n['scene'] == h['scene'])
    require(integer(n['qpc_frequency']) and n['qpc_frequency'] > 0)
    require(n['qpc_frequency'] == h['qpc_frequency'])
    fields(n['markers'], ('trace_start_qpc', 'trace_stop_qpc'))
    start, stop = (n['markers'][key] for key in ('trace_start_qpc', 'trace_stop_qpc'))
    require(integer(start) and integer(stop) and stop > start)
    require(integer(n['frames']) and n['frames'] >= 0 and type(n['injection_applied']) is bool)
    require(integer(n['vram_samples']) and n['vram_samples'] >= 0)
    require(n['queue_mode'] in ('same', 'own') and n['barrier_api'] in ('legacy', 'enhanced'))
    if n['scene'] in ('w3', 'wj'):
        require(finite(n.get('turn_ms')) and n['turn_ms'] == h['options']['turn_ms'])
    if n['scene'] == 'wj':
        require(n.get('fixture') == MENUS[h['menu']] and n.get('overlay') == 'none'
                and n.get('pose_upload') == POSE_UPLOAD)
    else:
        require(not any(key in n for key in ('fixture', 'overlay', 'pose_check', 'pose_upload')))
        require(not os.path.lexists(directory / 'pose_check.json'))
    # The DLL applies an injection at turn 20 only; without one it reports none applied.
    require(h['options']['inject'] is not None or n['injection_applied'] is False)
    entries = []
    with (directory / 'trace.jsonl').open(encoding='utf-8') as stream:
        for index, line in enumerate(stream):
            entry = json.loads(line, parse_constant=reject_constant, object_pairs_hook=object_pairs)
            canonical(entry)
            fields(entry, ('frame', 'qpc', 'revision', 'turn', 'phase', 'camera'))
            require(integer(entry['frame']) and entry['frame'] == index)
            require(integer(entry['qpc']) and start <= entry['qpc'] < stop)
            require(not entries or entry['qpc'] >= entries[-1]['qpc'])
            require(integer(entry['revision']) and entry['revision'] >= 0)
            require(entry['turn'] is None or (integer(entry['turn']) and entry['turn'] >= 0))
            require(entry['phase'] is None or (finite(entry['phase']) and 0 <= entry['phase'] <= 1))
            require(integer(entry['camera']) and entry['camera'] >= 0)
            # W-J's trace always carries its turn and phase (PROTOCOL.md).
            require(n['scene'] != 'wj' or (entry['turn'] is not None and entry['phase'] is not None))
            entries.append(entry)
    require(n['frames'] == len(entries) and integer(h['window']['presents'])
            and n['frames'] == h['window']['presents'])
    pose = None
    if n['scene'] in ('w3', 'w4'):
        check_labels(n.get('label_check'), entries)
    elif n['scene'] == 'wj':
        unreached = h['options']['inject'] is not None and n['injection_applied'] is False
        check_wj_labels(n.get('label_check'), entries, unreached)
        pose = read_json(directory / 'pose_check.json')
        check_pose(pose, n.get('pose_check'), n['label_check'], unreached)
    return n, entries, pose


def path_key(path):
    """Change separators only; never hide a dot component or a directory alias."""
    require(isinstance(path, str))
    path = path.replace('/', '\\')
    require(re.match(r'^[A-Za-z]:\\', path) is not None)
    components = path[3:].split('\\') if path[3:] else []
    require(all(component and component not in ('.', '..')
                and not any(ord(char) < 32 or char in '<>:"|?*' for char in component)
                for component in components))
    return path.casefold()


def check_guard_live(guard):
    require(file_guard.process_created(guard['pid']) == guard['created'])
    require(all(file_guard.write_probe(entry['path']) == 'sharing-violation' for entry in guard['files']))


def read_guard(directory, launched_created):
    raw = (directory / 'guard.json').read_bytes()
    guard = json.loads(raw, parse_constant=reject_constant, object_pairs_hook=object_pairs)
    canonical(guard)
    fields(guard, ('format', 'pid', 'created', 'ready', 'files', 'directories'))
    require(guard['format'] == 'magic600-l2-guard-v1')
    require(integer(guard['pid']) and 0 < guard['pid'] < 2**32)
    require(all(integer(guard[key]) and 0 < guard[key] < 2**64 for key in ('created', 'ready')))
    require(guard['created'] <= guard['ready'])
    require(integer(launched_created) and guard['ready'] < launched_created < 2**64)
    seen, files = set(), {}
    for kind in ('files', 'directories'):
        require(isinstance(guard[kind], list) and guard[kind])
        for entry in guard[kind]:
            fields(entry, ('path', 'volume_serial', 'file_id'))
            key = path_key(entry['path'])
            require(key not in seen)
            seen.add(key)
            require(integer(entry['volume_serial']) and 0 <= entry['volume_serial'] < 2**64)
            require(isinstance(entry['file_id'], str) and re.fullmatch(r'[0-9a-f]{32}', entry['file_id']) is not None)
            if kind == 'files':
                fields(entry, ('final_path', 'size', 'sha256'))
                require(path_key(entry['final_path']) == key)
                require(integer(entry['size']) and 0 <= entry['size'] < 2**63)
                require(isinstance(entry['sha256'], str) and re.fullmatch(r'[0-9a-f]{64}', entry['sha256']) is not None)
                files[key] = entry
    check_guard_live(guard)
    binding = {'format': guard['format'], 'files': len(guard['files']), 'directories': len(guard['directories']),
               'guard_sha256': hashlib.sha256(raw).hexdigest(), 'rule': 'guard-before-launch'}
    return guard, files, binding


def final_guard_check(directory, launched_created, binding):
    """The same guard must still hold the build when the finalizer accepts its output."""
    require(read_guard(directory, launched_created)[2] == binding)


def guarded_digest(path, files, assembly=False):
    entry = files[path_key(str(path))]
    with file_guard.open_identity(str(path)) as handle:
        require(file_guard.file_id(handle) == (entry['volume_serial'], entry['file_id']))
        size, digest = file_guard.sha256_handle(handle)
        require(size == entry['size'] and digest == entry['sha256'])
        data = None
        if assembly:
            import msvcrt
            # Transfer the same Win32 handle to a CRT stream, without opening the path again.
            descriptor = msvcrt.open_osfhandle(handle.value, os.O_RDONLY | os.O_BINARY)
            handle.value = None  # The stream now owns and closes this handle.
            with os.fdopen(descriptor, 'rb') as stream:
                stream.seek(0)
                data = stream.read()
            require(len(data) == size and hashlib.sha256(data).hexdigest() == digest)
    return entry['sha256'], data


def assembly_mvid(data):
    """ECMA-335 II.24.2/22.30: CLI metadata, Module row 1 and the 1-based GUID heap."""
    def take(buffer, offset, length):
        require(0 <= offset <= len(buffer) and 0 <= length <= len(buffer) - offset)
        return buffer[offset:offset + length]

    def number(buffer, offset, code='I'):
        return struct.unpack('<' + code, take(buffer, offset, struct.calcsize('<' + code)))[0]

    require(take(data, 0, 2) == b'MZ')
    pe = number(data, 0x3C)
    require(take(data, pe, 4) == b'PE\0\0')
    section_count, optional_size = number(data, pe + 6, 'H'), number(data, pe + 20, 'H')
    optional = take(data, pe + 24, optional_size)
    magic = number(optional, 0, 'H')
    require(magic in (0x10B, 0x20B))
    directories = 96 if magic == 0x10B else 112
    require(number(optional, directories - 4) > 14)
    cli_rva, cli_size = number(optional, directories + 14 * 8), number(optional, directories + 14 * 8 + 4)
    sections = take(data, pe + 24 + optional_size, section_count * 40)

    def rva_bytes(rva, length):
        require(rva > 0)
        headers = number(optional, 60)
        if rva < headers:
            require(length <= headers - rva)
            return take(data, rva, length)
        for offset in range(0, len(sections), 40):
            address = number(sections, offset + 12)
            raw_size, raw_offset = number(sections, offset + 16), number(sections, offset + 20)
            delta = rva - address
            if 0 <= delta < raw_size:
                require(length <= raw_size - delta)
                return take(data, raw_offset + delta, length)
        raise ValueError('unmapped CLI RVA')

    require(cli_size >= 72)
    cli = rva_bytes(cli_rva, cli_size)
    require(72 <= number(cli, 0) <= cli_size)
    metadata = rva_bytes(number(cli, 8), number(cli, 12))
    require(take(metadata, 0, 4) == b'BSJB')
    version_size = number(metadata, 12)
    require(version_size > 0 and version_size % 4 == 0)
    position = 16 + version_size
    take(metadata, 16, version_size)
    stream_count = number(metadata, position + 2, 'H')
    position += 4
    streams = {}
    for _ in range(stream_count):
        offset, length = number(metadata, position), number(metadata, position + 4)
        position += 8
        end = metadata.find(b'\0', position, min(position + 32, len(metadata)))
        require(end >= position)
        name = take(metadata, position, end - position).decode('ascii')
        require(name not in streams)
        streams[name] = (offset, length)
        position = (end + 4) & ~3
    require(all(offset >= position for offset, _ in streams.values()))
    tables = take(metadata, *streams['#~'])
    guids = take(metadata, *streams['#GUID'])
    heaps = number(tables, 6, 'B')
    valid = number(tables, 8, 'Q')
    require(valid & 1 and number(tables, 24) == 1)
    row = 24 + valid.bit_count() * 4
    take(tables, 24, valid.bit_count() * 4)
    string_size, guid_size = (4 if heaps & 1 else 2), (4 if heaps & 4 else 2)
    take(tables, row, 2 + string_size + 3 * guid_size)
    index = number(tables, row + 2 + string_size, 'I' if guid_size == 4 else 'H')
    require(index > 0)
    return str(uuid.UUID(bytes_le=take(guids, (index - 1) * 16, 16)))


def build_identity(h, n, mode, directory, launched_created):
    guard, guarded, binding = read_guard(directory, launched_created)
    files, identity = h['files'], h['dll_identity']
    required = {'godot:exe', 'godot:assembly', 'godot:project.godot', 'godot:Main.tscn'}
    require(required <= files.keys() and 'dll' in files)
    require(all(isinstance(path, str) and Path(path).is_absolute() for path in files.values()))
    dll = Path(files['dll'])
    fields(identity, ('dll', 'shaders'))
    require(isinstance(identity['shaders'], list))
    require(all(isinstance(part, dict) for part in identity['shaders']))
    require({part['file'] for part in identity['shaders']} == SHADERS
            and len(identity['shaders']) == len(SHADERS))
    parts = []
    for prefix, part in [('dll', identity['dll']), *[('shader', p) for p in identity['shaders']]]:
        fields(part, ('file', 'sha256'))
        name = part['file']
        require(isinstance(name, str) and name not in ('', '.', '..')
                and '/' not in name and '\\' not in name and ':' not in name)
        require(prefix != 'dll' or name.casefold() == dll.name.casefold())
        digest, _ = guarded_digest(dll if prefix == 'dll' else dll.parent / name, guarded)
        require(digest == part['sha256'])
        parts.append({'name': f'{prefix}:{name}', 'sha256': digest})
    if n is not None and mode in ('geometry', 'lattice'):
        # The DLL hashes its compact, key-sorted identity JSON (S-B's json.h), which canonical()
        # reproduces for these ASCII names; this ties the geometry result to the measured parts.
        require(n['build_identity'] == hashlib.sha256(canonical(identity)).hexdigest())
    elif n is not None:
        require(canonical(n['identity']) == canonical(identity))
    for name, path in files.items():
        if name != 'dll':
            require(name.startswith('godot:'))
            is_assembly = name == 'godot:assembly'
            digest, data = guarded_digest(Path(path), guarded, assembly=is_assembly)
            if is_assembly:
                mvid = h['assembly_mvid']
                require(isinstance(mvid, str)
                        and re.fullmatch(r'[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}', mvid) is not None)
                require(mvid == assembly_mvid(data))
            parts.append({'name': name, 'sha256': digest})
    configuration = h['configuration']
    require(configuration and all(type(value) in (str, bool) or finite(value)
                                  for value in configuration.values()))
    parts.append({'name': 'settings', 'sha256': hashlib.sha256(canonical(configuration)).hexdigest()})
    parts.sort(key=lambda part: part['name'])
    require(len({part['name'] for part in parts}) == len(parts))
    text = ''.join(f"{part['name']}={part['sha256']}\n" for part in parts).encode('utf-8')
    # The finalizer's own handles are closed before probing, so they cannot mask a dead guard.
    check_guard_live(guard)
    return ({'build_identity': hashlib.sha256(text).hexdigest(), 'parts': parts,
             'dll_identity': identity}, binding)


def check_sizes(h, n):
    sizes = h['sizes']
    values = [size(sizes[key]) for key in ('display', 'window', 'backbuffer', 'displayed', 'target')]
    require(all(value == values[0] for value in values))
    require(integer(sizes['samples_changed']) and sizes['samples_changed'] == 0)
    if n is not None:
        require(size(n['target']) == values[-1] and size(n['viewport']) == values[-1])


def check_scaling(h):
    scaling = h['scaling']
    require(scaling.get('texture_stretch') == 'none')
    require(scaling.get('content_scale_mode') == 'disabled'
            and finite(scaling.get('content_scale_factor')) and scaling['content_scale_factor'] == 1)


def check_conditions(h, n):
    w, e = h['window'], h['environment']
    samples = w['samples']
    require(integer(samples) and samples > 0)
    duration = n['markers']['trace_stop_qpc'] - n['markers']['trace_start_qpc']
    require(2 * samples * n['qpc_frequency'] >= duration * 10)
    p = e['power_samples']
    fields(p, ('samples', 'mains', 'battery'))
    require(all(integer(p[key]) and p[key] >= 0 for key in ('samples', 'mains', 'battery')))
    require(p['samples'] == samples and p['mains'] + p['battery'] <= samples)
    source = ('mains' if p['mains'] == samples else 'battery' if p['battery'] == samples
              else 'changed' if p['mains'] + p['battery'] == samples else 'unknown')
    require(e['power_source'] == source)
    require(all(integer(w[key]) and w[key] == 0
                for key in ('samples_not_visible', 'samples_covered', 'samples_not_foreground')))
    require(all(w[key] is True
                for key in ('visible_throughout', 'foreground_throughout', 'foreground_at_trace_start')))


def read_presentmon(directory, n, pid):
    path = directory / 'presentmon.csv'
    with path.open('rb') as stream:
        require(stream.seek(0, 2) > 0)
        stream.seek(-1, 2)
        require(stream.read(1) == b'\n')
    start, stop = (n['markers'][key] for key in ('trace_start_qpc', 'trace_stop_qpc'))
    rows = []
    with path.open(encoding='utf-8-sig', newline='') as stream:
        reader = csv.DictReader(stream, strict=True)
        header = reader.fieldnames or []
        require(len(set(header)) == len(header) and set(CSV_COLUMNS) <= set(header))
        for row in reader:
            require(None not in row and all(row[key] is not None for key in CSV_COLUMNS))
            if gate.csv_integer(row['ProcessID']) != pid:
                continue
            ticks = gate.csv_integer(row['QPCTime'])
            if not start <= ticks < stop:
                continue
            chain = row['SwapChainAddress'].strip()
            require(bool(chain) and bool(row['PresentMode'].strip()))
            gate.csv_interval(row['msBetweenPresents'])
            dropped, sync, tearing = (gate.csv_integer(row[key])
                                      for key in ('Dropped', 'SyncInterval', 'AllowsTearing'))
            require(dropped in (0, 1) and tearing in (0, 1))
            rows.append({'qpc': ticks, 'chain': chain, 'dropped': dropped,
                         'sync': sync, 'tearing': tearing, 'mode': row['PresentMode'].strip()})
    require(bool(rows))
    return sorted(rows, key=lambda row: row['qpc'])


def check_blind_seconds(rows, n, mode):
    frequency, marks = n['qpc_frequency'], n['markers']
    start, stop = marks['trace_start_qpc'], marks['trace_stop_qpc']
    begin = start + (10 if mode == 'run' else 1) * frequency
    end = min(start + 190 * frequency, stop) if mode == 'run' else stop - frequency
    if end <= begin:
        return
    # Whole seconds only: a trailing fraction of a second can be shorter than one frame.
    seconds = (end - begin) // frequency
    end = begin + seconds * frequency
    shown = {(row['qpc'] - begin) // frequency for row in rows
             if begin <= row['qpc'] < end and row['dropped'] == 0}
    require(len(shown) == seconds)


def check_trace_steps(rows, n, entries):
    frequency, marks = n['qpc_frequency'], n['markers']
    begin, end = marks['trace_start_qpc'] + frequency, marks['trace_stop_qpc'] - frequency
    ticks = [row['qpc'] for row in rows if begin <= row['qpc'] <= end]
    times = [entry['qpc'] for entry in entries]
    # Match renderer_gate.trace_step_reasons: (previous present, next present].
    require(all(bisect.bisect_right(times, after) - bisect.bisect_right(times, before) == 1
                for before, after in zip(ticks, ticks[1:])))


def presentmon_summary(rows):
    def counts(key):
        result = {}
        for row in rows:
            name = str(row[key])
            result[name] = result.get(name, 0) + 1
        return dict(sorted(result.items()))
    return {'rows': len(rows), 'present_modes': counts('mode'), 'sync_intervals': counts('sync'),
            'allows_tearing': counts('tearing')}


def finalize(directory, candidate, mode, pid, app_exit, launched_created, overlays=None, fault_injection=False,
             adapter=DEFAULT_ADAPTER, unattended=False):
    """Return an exit code; the only output is one exclusively created record."""
    if any(os.path.lexists(directory / name) for name in OUTPUTS):
        print('output-exists: ' + CHECKS['output-exists'], file=sys.stderr)
        return 5
    reasons = set()

    def check(code, action):
        try:
            return action()
        except (OSError, ValueError, TypeError, KeyError, OverflowError, csv.Error, file_guard.GuardRefused):
            reasons.add(code)
            return None

    h = check('harness', lambda: read_harness(directory, candidate, mode, pid))
    run_id = h['run_id'] if h is not None else None
    output = binding = None
    if h is not None:
        check('app-exit', lambda: require(app_exit == h['exit_code'] and app_exit in (0, 2)))
        check('adapter', lambda: require(h['environment']['adapter'] == adapter))
        native_result = check('native', lambda: read_native(directory, h, mode))
        # W-J's full pose check stays in pose_check.json; run.json carries its status and revisions.
        n, entries, _ = native_result if native_result is not None else (None, None, None)
        identity_result = check('identity', lambda: build_identity(h, n, mode, directory, launched_created))
        build, binding = identity_result if identity_result is not None else (None, None)
        checks = {code: 'pass' for code in sorted(CHECKS)}
        if mode == 'run':
            check('size-mismatch', lambda: check_sizes(h, n))
            check('scaling', lambda: check_scaling(h))
            check('conditions-not-enforced', lambda: require(h['options']['conditions'] == 'enforce'))
            if n is not None:
                check('conditions', lambda: check_conditions(h, n))
                check('vram-missing', lambda: require(finite(n['vram_peak_mb']) and n['vram_peak_mb'] > 0))
                rows = check('presentmon', lambda: read_presentmon(directory, n, pid))
                if rows is not None:
                    chains = {row['chain'] for row in rows}
                    check('swap-chain', lambda: require(len(chains) == 1))
                    if len(chains) == 1:
                        check('sync-interval', lambda: require(all(row['sync'] == 0 for row in rows)))
                        check('blind-seconds', lambda: check_blind_seconds(rows, n, mode))
                        check('trace-steps', lambda: check_trace_steps(rows, n, entries))
            else:
                rows = None
        if mode == 'run':
            check('debug-switch', lambda: require(not any(h['options'][key]
                  for key in ('gpu_validation', 'no_vram', 'debug_half_target'))
                  and integer(h['debug']['debug_layer']) and h['debug']['debug_layer'] == 0))
            # A run with an injection is a fault-injection run and nothing else: it never takes an operator
            # declaration, and a fault-injection finalization of a run without an injection is refused too.
            injected = h['options']['inject'] is not None
            check('operator', lambda: require(
                injected == bool(fault_injection)
                and (overlays is None and not unattended if fault_injection
                     else isinstance(overlays, str) and bool(overlays.strip()))))
        if not reasons:
            if mode in ('geometry', 'lattice'):
                output = {'format': f'magic600-l2-{mode}-record-v1', 'candidate': candidate,
                          'run_id': run_id, 'scene': h['scene'], 'status': n['status'], 'build': build, mode: n}
                if h['scene'] == 'wj':
                    output['fixture'] = MENUS[h['menu']]
                name = f'{mode}-record.json'
                failed = n['status'] == 'fail'
            else:
                pm = {'process_id': pid, 'swap_chain': rows[0]['chain']}
                summary = presentmon_summary(rows)
                failed = (n.get('label_check', {}).get('status') == 'fail'
                          or n.get('pose_check', {}).get('status') == 'fail')
                environment = copy.deepcopy(h['environment'])
                # PresentMon sets AllowsTearing only from a flip or blit event (FRAMEWORK-FACTS P1), so a
                # present dropped before one shows 0 whatever its flags; dropped rows do not decide tearing.
                shown = [row for row in rows if row['dropped'] == 0]
                environment['tearing'] = bool(shown) and all(row['tearing'] == 1 for row in shown)
                environment['declared']['overlays'] = ('fault-injection run; not gate evidence'
                                                       if fault_injection else overlays)
                if not fault_injection:
                    # Machine-readable next to the operator text: an unattended run had no watcher.
                    environment['declared']['watched'] = not unattended
                rotating = h['scene'] in ('w2', 'w3', 'wj')
                output = {'format': 'magic600-renderer-run-v1', 'run_id': run_id,
                          'candidate': candidate, 'scene': h['scene'], 'presentmon': pm,
                          'build': build, 'environment': environment, 'window': h['window'],
                          'camera': {'plane': [0, 3] if rotating else None,
                                     'step_rad': 0.002 if rotating else 0, 'per': 'frame' if rotating else 'none'},
                          'l2': {'sizes': h['sizes'], 'scaling': h['scaling'], 'options': h['options'],
                                 'configuration': h['configuration'], 'expected_adapter': adapter,
                                 'native': {key: n[key] for key in
                                            ('target', 'viewport', 'queue_mode', 'barrier_api', 'vram_samples')},
                                 'presentmon': summary, 'condition_reasons': gate.condition_reasons(environment),
                                 'checks': checks}}
                for key in ('qpc_frequency', 'markers', 'frames', 'injection_applied',
                            'vram_peak_mb', 'turn_ms', 'label_check',
                            'fixture', 'overlay', 'pose_check', 'pose_upload'):
                    if key in n:
                        output[key] = n[key]
                if h['options']['inject'] is not None:
                    output['injected_fault'] = h['options']['inject']
                check('gate-shape', lambda: gate.validate_run(output))
                if h['options']['inject'] is not None:
                    # Never gate evidence: another name and format, so the gate neither finds nor accepts it.
                    output['format'] = 'magic600-l2-fault-run-v1'
                    name = 'fault-run.json'
                else:
                    name = 'run.json'
            output['binding'] = binding
    if binding is not None:
        # Last check before the output, so a guard that failed during the later checks
        # cannot leave an accepted record (L2-V-002).
        check('identity', lambda: final_guard_check(directory, launched_created, binding))
    if reasons:
        output = {'format': 'magic600-l2-refusal-v1', 'candidate': candidate, 'mode': mode,
                  'run_id': run_id, 'reasons': sorted(reasons),
                  'details': {code: CHECKS[code] for code in sorted(reasons)}}
        name, code = 'refusal.json', 5
    else:
        code = 2 if failed else 0
    contents = json.dumps(output, indent=2, ensure_ascii=False, allow_nan=False) + '\n'
    try:
        with (directory / name).open('x', encoding='utf-8', newline='\n') as stream:
            stream.write(contents)
    except FileExistsError:
        print('output-exists: ' + CHECKS['output-exists'], file=sys.stderr)
        return 5
    except OSError:
        print('Cannot create the finalizer output in the run directory.', file=sys.stderr)
        return 1
    print(f'{name}: ' + (', '.join(sorted(reasons)) if reasons else 'fail' if failed else 'pass'))
    return code


class UsageParser(argparse.ArgumentParser):
    def error(self, message):
        self.print_usage(sys.stderr)
        self.exit(1, f'{self.prog}: {message}\n')


def main(argv=None):
    parser = UsageParser(description=__doc__, allow_abbrev=False)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--candidate', choices=('sa2',), required=True)
    parser.add_argument('--mode', choices=MODES, required=True)
    parser.add_argument('--launched-pid', type=int, required=True)
    parser.add_argument('--launched-created', type=int, required=True)
    parser.add_argument('--app-exit', type=int, required=True)
    operator = parser.add_mutually_exclusive_group()
    operator.add_argument('--overlays')
    operator.add_argument('--fault-injection', action='store_true')
    parser.add_argument('--unattended', action='store_true', help='with --overlays: nobody watched the run')
    parser.add_argument('--adapter', default=DEFAULT_ADAPTER)
    args = parser.parse_args(argv)
    if not args.directory.is_dir() or args.launched_pid <= 0 or not args.adapter.strip():
        parser.error('An existing run directory, positive PID and nonempty adapter are required.')
    return finalize(args.directory, args.candidate, args.mode, args.launched_pid,
                    args.app_exit, args.launched_created, args.overlays, args.fault_injection, args.adapter,
                    args.unattended)


if __name__ == '__main__':
    sys.exit(main())
