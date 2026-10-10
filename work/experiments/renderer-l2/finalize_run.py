"""Finalize framework/DLL/PresentMon evidence without changing its inputs."""
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

REPOSITORY = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPOSITORY / 'tools' / 'perf'))
import renderer_gate as gate

DEFAULT_ADAPTER = 'NVIDIA GeForce RTX 4070 Laptop GPU'
OUTPUTS = ('run.json', 'short-check.json', 'geometry-record.json',
           'validation-record.json', 'refusal.json')
SCENES = ('w1', 'w2', 'w3', 'w4')
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
    'validation': 'GPU validation is disabled or reports errors, corruption or SA2 mentions.',
    'gate-shape': 'The completed record does not satisfy the renderer gate input contract.',
    'output-exists': 'A finalizer output already exists; no file was written.',
}
CSV_COLUMNS = ('ProcessID', 'SwapChainAddress', 'QPCTime', 'msBetweenPresents',
               'Dropped', 'SyncInterval', 'PresentMode', 'AllowsTearing')
# The DLL checks its geometry against S-B's reference set and verifies the files' digests in this index.
REFERENCE_INDEX = REPOSITORY / 'work' / 'experiments' / 'renderer-sb' / 'reference' / 'index.json'
LABEL_COUNTERS = ('mismatches', 'binding_mismatches', 'late_adoptions', 'missing')


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
    fields(h, ('format', 'candidate', 'mode', 'run_id', 'scene', 'process_id', 'exit_code',
               'reason', 'options', 'configuration', 'files', 'dll_identity', 'dll_status',
               'qpc_frequency', 'sizes', 'scaling', 'dpi_awareness', 'environment', 'window', 'debug'))
    require(h['format'] == 'magic600-l2-harness-v1' and h['candidate'] == candidate)
    require(integer(h['process_id']) and h['process_id'] == pid)
    require(h['mode'] == ('geometry' if mode == 'geometry' else 'run'))
    require(isinstance(h['run_id'], str)
            and re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9._-]{0,127}', h['run_id']) is not None)
    require(mode == 'geometry' or h['scene'] in SCENES)
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


def read_native(directory, h, mode):
    if mode == 'geometry':
        require(not os.path.lexists(directory / 'native.json')
                and not os.path.lexists(directory / 'trace.jsonl'))
        g = read_json(directory / 'geometry.json')
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
        return g, None
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
    if n['scene'] == 'w3':
        require(finite(n.get('turn_ms')) and n['turn_ms'] == h['options']['turn_ms'])
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
            entries.append(entry)
    require(n['frames'] == len(entries) and integer(h['window']['presents'])
            and n['frames'] == h['window']['presents'])
    if n['scene'] in ('w3', 'w4'):
        check_labels(n.get('label_check'), entries)
    return n, entries


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


def check_modules(h, guarded):
    modules, files = h['modules'], h['files']
    fields(modules, ('observer', 'failure', 'scope', 'unload_history', 'overflow', 'events'))
    require(modules['observer'] == 'sealed' and modules['failure'] is None
            and modules['unload_history'] == [] and modules['overflow'] is False)
    scope = path_key(modules['scope'])
    require(scope == path_key(ntpath.dirname(files['qt:exe'])))
    require(isinstance(modules['events'], list))
    loaded = {}
    for event in modules['events']:
        fields(event, ('kind', 'path'))
        require(event['kind'] in ('snapshot', 'load', 'unload'))
        key = path_key(event['path'])
        require(key.startswith(scope + '\\'))
        if event['kind'] == 'unload':
            require(key in loaded)
        else:
            # A folded lookup can name a different NTFS file. Bind each load path itself,
            # even if another event or files entry has the same Python case folding.
            guarded_digest(event['path'], guarded)
            loaded[key] = event['path']
    excluded = {path_key(files['qt:exe']), path_key(files['dll'])}
    expected = {}
    for key, path in loaded.items():
        if key not in excluded:
            name = ('qt:' + ntpath.basename(path)).casefold()
            require(name not in expected)
            expected[name] = key
    actual = {}
    for name, path in files.items():
        if name not in ('dll', 'qt:exe'):
            require(name.startswith('qt:') and name.casefold() not in actual)
            actual[name.casefold()] = path_key(path)
    require(actual == expected)


def build_identity(h, n, mode, directory, launched_created):
    guard, guarded, binding = read_guard(directory, launched_created)
    files, identity = h['files'], h['dll_identity']
    required = ({'godot:exe', 'godot:assembly', 'godot:project.godot', 'godot:Main.tscn'}
                if h['candidate'] == 'sa2' else {'qt:exe'})
    require(required <= files.keys() and 'dll' in files)
    if h['candidate'] == 'sd':
        require(any(name.casefold().startswith('qt:qt6') for name in files))
        check_modules(h, guarded)
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
    if n is not None and mode == 'geometry':
        # The DLL hashes its compact, key-sorted identity JSON (S-B's json.h), which canonical()
        # reproduces for these ASCII names; this ties the geometry result to the measured parts.
        require(n['build_identity'] == hashlib.sha256(canonical(identity)).hexdigest())
    elif n is not None:
        require(canonical(n['identity']) == canonical(identity))
    for name, path in files.items():
        if name != 'dll':
            require(name.startswith('godot:' if h['candidate'] == 'sa2' else 'qt:'))
            is_assembly = h['candidate'] == 'sa2' and name == 'godot:assembly'
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
    if h['candidate'] == 'sa2':
        require(scaling.get('content_scale_mode') == 'disabled'
                and finite(scaling.get('content_scale_factor')) and scaling['content_scale_factor'] == 1)
    else:
        ratio = scaling.get('device_pixel_ratio')
        require(finite(ratio) and ratio > 0)
        for axis in ('width', 'height'):
            logical = scaling.get(f'item_{axis}')
            require(finite(logical) and logical > 0)
            require(abs(h['sizes']['displayed'][axis] - logical * ratio) <= 0.000001)


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


def check_validation(h):
    debug = h['debug']
    require(h['options']['gpu_validation'] is True and integer(debug['debug_layer'])
            and debug['debug_layer'] == 1)
    counts = debug['counts']
    require(isinstance(counts, dict))
    require(all(integer(counts.get(key)) and counts[key] == 0
                for key in ('error', 'corruption', 'mentioning_sa2')))


def finalize(directory, candidate, mode, pid, app_exit, launched_created, overlays=None, fault_injection=False,
             adapter=DEFAULT_ADAPTER):
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
    output = None
    if h is not None:
        check('app-exit', lambda: require(app_exit == h['exit_code'] and app_exit in (0, 2)))
        check('adapter', lambda: require(h['environment']['adapter'] == adapter))
        native_result = check('native', lambda: read_native(directory, h, mode))
        n, entries = native_result if native_result is not None else (None, None)
        identity_result = check('identity', lambda: build_identity(h, n, mode, directory, launched_created))
        build, binding = identity_result if identity_result is not None else (None, None)
        checks = {code: 'pass' for code in sorted(CHECKS)}
        if mode != 'geometry':
            check('size-mismatch', lambda: check_sizes(h, n))
            check('scaling', lambda: check_scaling(h))
        if mode in ('run', 'short'):
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
            check('operator', lambda: require(fault_injection or isinstance(overlays, str) and bool(overlays.strip())))
        injection = h['options']['inject']
        if isinstance(injection, str) and injection.startswith('module-'):
            check('operator', lambda: require(candidate == 'sd' and mode == 'run' and fault_injection
                  and injection in ('module-late', 'module-transient', 'module-reload')))
        if mode == 'validation':
            check('validation', lambda: check_validation(h))
        if not reasons:
            if mode == 'geometry':
                output = {'format': 'magic600-l2-geometry-record-v1', 'candidate': candidate,
                          'run_id': run_id, 'status': n['status'], 'build': build, 'geometry': n}
                name = 'geometry-record.json'
                failed = n['status'] == 'fail'
            elif mode == 'validation':
                output = {'format': 'magic600-l2-validation-record-v1', 'candidate': candidate,
                          'run_id': run_id, 'scene': h['scene'], 'build': build, 'debug': h['debug']}
                if 'label_check' in n:
                    output['label_check'] = n['label_check']
                name = 'validation-record.json'
                failed = n.get('label_check', {}).get('status') == 'fail'
            else:
                pm = {'process_id': pid, 'swap_chain': rows[0]['chain']}
                summary = presentmon_summary(rows)
                failed = n.get('label_check', {}).get('status') == 'fail'
                if mode == 'short':
                    output = {'format': 'magic600-l2-short-check-v1', 'run_id': run_id,
                              'candidate': candidate, 'scene': h['scene'], 'build': build,
                              'l2': {'presentmon': summary}, 'presentmon': pm, 'sizes': h['sizes'],
                              'vram_peak_mb': n['vram_peak_mb'], 'window': h['window'], 'checks': checks}
                    if 'label_check' in n:
                        output['label_check'] = n['label_check']
                    name = 'short-check.json'
                else:
                    environment = copy.deepcopy(h['environment'])
                    # PresentMon sets AllowsTearing only from a flip or blit event (FRAMEWORK-FACTS P1), so a
                    # present dropped before one shows 0 whatever its flags; dropped rows do not decide tearing.
                    shown = [row for row in rows if row['dropped'] == 0]
                    environment['tearing'] = bool(shown) and all(row['tearing'] == 1 for row in shown)
                    environment['declared']['overlays'] = ('fault-injection run; not gate evidence'
                                                           if fault_injection else overlays)
                    rotating = h['scene'] in ('w2', 'w3')
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
                                'vram_peak_mb', 'turn_ms', 'label_check'):
                        if key in n:
                            output[key] = n[key]
                    if h['options']['inject'] is not None:
                        output['injected_fault'] = h['options']['inject']
                    check('gate-shape', lambda: gate.validate_run(output))
                    name = 'run.json'
            output['binding'] = binding
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
    parser.add_argument('--candidate', choices=('sa2', 'sd'), required=True)
    parser.add_argument('--mode', choices=('run', 'short', 'geometry', 'validation'), required=True)
    parser.add_argument('--launched-pid', type=int, required=True)
    parser.add_argument('--launched-created', type=int, required=True)
    parser.add_argument('--app-exit', type=int, required=True)
    operator = parser.add_mutually_exclusive_group()
    operator.add_argument('--overlays')
    operator.add_argument('--fault-injection', action='store_true')
    parser.add_argument('--adapter', default=DEFAULT_ADAPTER)
    args = parser.parse_args(argv)
    if not args.directory.is_dir() or args.launched_pid <= 0 or not args.adapter.strip():
        parser.error('An existing run directory, positive PID and nonempty adapter are required.')
    return finalize(args.directory, args.candidate, args.mode, args.launched_pid,
                    args.app_exit, args.launched_created, args.overlays, args.fault_injection, args.adapter)


if __name__ == '__main__':
    sys.exit(main())
