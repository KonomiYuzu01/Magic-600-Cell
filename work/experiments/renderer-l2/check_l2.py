"""Source/fixture acceptance for L2-F; never starts a renderer or capture."""
import sys

sys.dont_write_bytecode = True

import contextlib
import copy
import csv
import hashlib
import io
import json
import os
import shutil
import subprocess
from collections import Counter
from pathlib import Path

import finalize_run as finalizer

HERE = Path(__file__).resolve().parent
PID = 4242
FREQUENCY = 6_000_000
START = 20_000_000
CHAIN = '0xABC'
SIZE = {'width': 2560, 'height': 1600}
REFUSALS = Counter()
CASES = 0


def expect(condition, message):
    if not condition:
        raise AssertionError(message)


def write_json(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, allow_nan=False) + '\n',
                    encoding='utf-8', newline='\n')


def read_json(path):
    return json.loads(path.read_text(encoding='utf-8'))


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def make_parts(root):
    parts = root / 'parts'
    parts.mkdir()
    for name in ('sa2_interop.dll', *sorted(finalizer.SHADERS), 'godot.exe', 'SA2Smoke.dll',
                 'project.godot', 'Main.tscn', 'sd.exe', 'Qt6Core.dll', 'Qt6Quick.dll'):
        (parts / name).write_bytes(('synthetic L2 part: ' + name + '\n').encode())
    return parts


def identity(parts):
    def part(name):
        return {'file': name, 'sha256': digest(parts / name)}
    return {'dll': part('sa2_interop.dll'), 'shaders': [part(name) for name in sorted(finalizer.SHADERS)]}


def fixture(root, name, candidate='sa2', mode='run', scene='w3', seconds=30, parts=None):
    directory = root / name
    directory.mkdir()
    parts = parts or root / 'parts'
    declared = {'frame_generation': False, 'upscaling': False}
    files = {'dll': str(parts / 'sa2_interop.dll')}
    if candidate == 'sa2':
        files.update({key: str(parts / value) for key, value in
                      {'godot:exe': 'godot.exe', 'godot:assembly': 'SA2Smoke.dll',
                       'godot:project.godot': 'project.godot', 'godot:Main.tscn': 'Main.tscn'}.items()})
        configuration = {'framework': 'godot', 'framework_version': '4.7.2.stable.mono.official',
                         'rendering_driver': 'd3d12', 'window_mode': 'exclusive_fullscreen',
                         'vsync': 'disabled', 'route': 'export', 'queue': 'same', 'handover': 'tracked',
                         'barriers': 'match', 'render_thread': 'safe', 'warmup_frames': 3}
        scaling = {'content_scale_mode': 'disabled', 'content_scale_factor': 1.0, 'texture_stretch': 'none'}
    else:
        files.update({key: str(parts / value) for key, value in
                      {'qt:exe': 'sd.exe', 'qt:Qt6Core.dll': 'Qt6Core.dll', 'qt:Qt6Quick.dll': 'Qt6Quick.dll'}.items()})
        configuration = {'framework': 'qt', 'framework_version': '6.10.3', 'graphics_api': 'd3d12',
                         'render_loop': 'threaded', 'swap_interval': 0, 'route': 'import-direct',
                         'device': 'qt', 'queue': 'same', 'handover': 'tracked', 'barriers': 'legacy'}
        scaling = {'device_pixel_ratio': 1.25, 'item_width': 2048.0, 'item_height': 1280.0,
                   'texture_stretch': 'none'}
    samples = seconds * 10
    h = {
        'format': 'magic600-l2-harness-v1', 'candidate': candidate,
        'mode': 'geometry' if mode == 'geometry' else 'run', 'run_id': name,
        'scene': None if mode == 'geometry' else scene, 'process_id': PID, 'exit_code': 0, 'reason': None,
        'options': {'trace_ms': seconds * 1000, 'preroll_ms': 4000, 'turn_ms': 190,
                    'inject': None, 'declared': declared.copy(), 'gpu_validation': mode == 'validation',
                    'conditions': 'record' if mode == 'validation' else 'enforce',
                    'no_vram': False, 'debug_half_target': False},
        'configuration': configuration, 'files': files, 'dll_identity': identity(parts),
        'dll_status': {'abi_version': 2, 'last_status': 0, 'last_error': ''},
        'qpc_frequency': FREQUENCY,
        'sizes': {**{key: SIZE.copy() for key in ('display', 'window', 'backbuffer', 'displayed', 'target')},
                  'samples': samples, 'samples_changed': 0},
        'scaling': scaling, 'dpi_awareness': 'per-monitor-v2',
        'environment': {'power_source': 'mains', 'power_mode': 'max_performance',
                        'presenting_adapter': finalizer.DEFAULT_ADAPTER + ", driver fixture, drives the window's display",
                        'presentation_interval': 0, 'declared': declared.copy(),
                        'display': {**SIZE, 'refresh_hz': 240}, 'backbuffer': SIZE.copy(),
                        'power_samples': {'samples': samples, 'mains': samples, 'battery': 0},
                        'vsync': False, 'adapter': finalizer.DEFAULT_ADAPTER, 'driver': 'fixture',
                        'msaa': 1, 'warp': False},
        'window': {'topmost': True, 'display_required': True, 'foreground_at_trace_start': True,
                   'sample_period_ms': 100, 'samples': samples, 'samples_not_visible': 0,
                   'samples_covered': 0, 'samples_not_foreground': 0, 'visible_throughout': True,
                   'foreground_throughout': True, 'presents': seconds * 60},
        'debug': {'enabled': mode == 'validation', 'debug_layer': 1 if mode == 'validation' else 0,
                  'counts': {'error': 0, 'corruption': 0, 'mentioning_sa2': 0} if mode == 'validation' else None,
                  'messages': '' if mode == 'validation' else None},
    }
    write_json(directory / 'harness.json', h)
    if mode == 'geometry':
        geometry = {'format': 'magic600-sb-geometry-check-v1', 'status': 'pass',
                    'build_identity': hashlib.sha256(finalizer.canonical(h['dll_identity'])).hexdigest(),
                    'adapter': finalizer.DEFAULT_ADAPTER,
                    'results': [{'camera': camera, 'pose': pose, 'samples': 9066, 'max_abs_error': 0,
                                 'failures': 0, 'status': 'pass'}
                                for camera in ('c0', 'c1', 'c2') for pose in ('start', 'mid', 'end')],
                    'per_cell_count': {'cells': 600, 'expected': 30480, 'failures': 0,
                                       'counts': [30480] * 600, 'status': 'pass'}}
        write_json(directory / 'geometry.json', geometry)
        return directory
    n = {'format': 'magic600-sa2-scene-native-v1', 'scene': scene, 'qpc_frequency': FREQUENCY,
         'markers': {'trace_start_qpc': START, 'trace_stop_qpc': START + seconds * FREQUENCY},
         'frames': seconds * 60, 'injection_applied': False, 'vram_peak_mb': 1000.5,
         'vram_samples': seconds * 60, 'target': SIZE.copy(), 'viewport': SIZE.copy(),
         'identity': copy.deepcopy(h['dll_identity']), 'queue_mode': 'same', 'barrier_api': 'legacy'}
    if scene == 'w3':
        n['turn_ms'] = 190
    if scene in ('w3', 'w4'):
        last = (seconds * 60 - 1) * (FREQUENCY // 60) // (190 * FREQUENCY // 1000)
        n['label_check'] = {'status': 'pass', 'copies': last, 'revisions': last, 'mismatches': 0,
                            'binding_mismatches': 0, 'late_adoptions': 0, 'missing': 0,
                            'oracle_sha256': {'even': '0' * 64, 'odd': '1' * 64}}
    write_json(directory / 'native.json', n)
    with (directory / 'trace.jsonl').open('w', encoding='utf-8', newline='\n') as stream:
        for frame in range(seconds * 60):
            elapsed = frame * (FREQUENCY // 60)
            turn, remainder = divmod(elapsed, 190 * FREQUENCY // 1000)
            entry = {'frame': frame, 'qpc': START + elapsed,
                     'revision': turn if scene in ('w3', 'w4') else 0,
                     'turn': turn if scene == 'w3' else None,
                     'phase': remainder / (190 * FREQUENCY // 1000) if scene == 'w3' else None,
                     'camera': frame + 1 if scene in ('w2', 'w3') else 0}
            stream.write(json.dumps(entry) + '\n')
    if mode in ('run', 'short'):
        with (directory / 'presentmon.csv').open('w', encoding='utf-8-sig', newline='') as stream:
            writer = csv.DictWriter(stream, fieldnames=finalizer.CSV_COLUMNS, lineterminator='\n')
            writer.writeheader()
            for frame in range(seconds * 60):
                writer.writerow({'ProcessID': PID, 'SwapChainAddress': CHAIN,
                                 'QPCTime': START + frame * (FREQUENCY // 60) + FREQUENCY // 1000,
                                 'msBetweenPresents': 1000 / 60, 'Dropped': 0, 'SyncInterval': 0,
                                 'PresentMode': 'Hardware: Independent Flip', 'AllowsTearing': 1})
    return directory


def edit(directory, name, action):
    value = read_json(directory / name)
    action(value)
    write_json(directory / name, value)


def csv_edit(directory, action):
    path = directory / 'presentmon.csv'
    with path.open(encoding='utf-8-sig', newline='') as stream:
        reader = csv.DictReader(stream, strict=True)
        header, rows = reader.fieldnames, list(reader)
    action(rows)
    with path.open('w', encoding='utf-8-sig', newline='') as stream:
        writer = csv.DictWriter(stream, fieldnames=header, lineterminator='\n')
        writer.writeheader()
        writer.writerows(rows)


def trace_edit(directory, action):
    path = directory / 'trace.jsonl'
    entries = [json.loads(line) for line in path.read_text(encoding='utf-8').splitlines()]
    action(entries)
    path.write_text(''.join(json.dumps(entry) + '\n' for entry in entries), encoding='utf-8', newline='\n')


def invoke(directory, mode='run', candidate='sa2', app_exit=0, overlays='fixture: watched throughout',
           extra=(), expected=0):
    global CASES
    inputs = {path.name: digest(path) for path in directory.iterdir() if path.name not in finalizer.OUTPUTS}
    arguments = [str(directory), '--candidate', candidate, '--mode', mode,
                 '--launched-pid', str(PID), '--app-exit', str(app_exit), *extra]
    if overlays is not None:
        arguments.extend(('--overlays', overlays))
    error_text = io.StringIO()
    existing_output = any((directory / name).exists() for name in finalizer.OUTPUTS)
    with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(error_text):
        try:
            code = finalizer.main(arguments)
        except SystemExit as error:
            code = error.code
    expect(code == expected, f'{directory.name}: expected exit {expected}, got {code}')
    if existing_output:
        expect('output-exists:' in error_text.getvalue(), 'output-exists reason was not reported')
    expect(all(digest(directory / name) == value for name, value in inputs.items()),
           directory.name + ': finalizer changed an input')
    CASES += 1
    return code


def refusal(root, name, code, mutation, mode='run', candidate='sa2', **invoke_options):
    directory = fixture(root, name, candidate=candidate, mode=mode)
    mutation(directory)
    invoke(directory, mode=mode, candidate=candidate, expected=5, **invoke_options)
    record = read_json(directory / 'refusal.json')
    expect(record['format'] == 'magic600-l2-refusal-v1' and record['mode'] == mode
           and record['candidate'] == candidate and record['reasons'] == [code],
           name + ': wrong refusal: ' + repr(record))
    expect(list(record['details']) == [code] and bool(record['details'][code]), name + ': missing detail')
    expect({path.name for path in directory.iterdir()} & set(finalizer.OUTPUTS) == {'refusal.json'},
           name + ': refusal wrote another output')
    REFUSALS[code] += 1
    return directory


def h_edit(action):
    return lambda directory: edit(directory, 'harness.json', action)


def n_edit(action):
    return lambda directory: edit(directory, 'native.json', action)


def valid_modes(root):
    for candidate in ('sa2', 'sd'):
        identities = []
        for mode, name in zip(('run', 'short', 'geometry', 'validation'), finalizer.OUTPUTS):
            directory = fixture(root, f'valid-{candidate}-{mode}', candidate=candidate, mode=mode)
            if mode == 'geometry':
                expect({p.name for p in directory.iterdir()} == {'harness.json', 'geometry.json'},
                       'geometry inputs must not include native, trace or capture')
            invoke(directory, mode=mode, candidate=candidate)
            expect({p.name for p in directory.iterdir()} & set(finalizer.OUTPUTS) == {name},
                   'exactly one output per mode')
            record = read_json(directory / name)
            identities.append(record['build']['build_identity'])
            expect(record['build']['parts'] == sorted(record['build']['parts'], key=lambda part: part['name']),
                   'unsorted build parts')
            if mode == 'run':
                finalizer.gate.validate_run(record)
                expect(record['l2']['expected_adapter'] == finalizer.DEFAULT_ADAPTER, 'expected adapter missing')
                expect(set(record['l2']['checks']) == set(finalizer.CHECKS)
                       and set(record['l2']['checks'].values()) == {'pass'}, 'check inventory')
                expect(record['environment']['tearing'] is True and not record['l2']['condition_reasons'],
                       'valid conditions or tearing')
            if mode == 'geometry':
                expect(record['status'] == 'pass' and len(record['geometry']['results']) == 9, 'geometry summary')
            raw = (directory / name).read_bytes()
            expect(raw.endswith(b'\n') and b'\r' not in raw, 'output line endings')
        expect(len(set(identities)) == 1, candidate + ': modes changed build identity')
        for mode in ('run', 'short', 'validation', 'geometry'):
            refusal(root, f'wrong-app-mode-{candidate}-{mode}', 'harness',
                    h_edit(lambda h: h.update(mode='run' if mode == 'geometry' else 'geometry')),
                    candidate=candidate, mode=mode)


def gate_series(root):
    for candidate in ('sa2', 'sd'):
        directories = [fixture(root, f'gate-{candidate}-{i}', candidate=candidate, seconds=192) for i in range(3)]
        for directory in directories:
            invoke(directory, candidate=candidate)
        result = finalizer.gate.summarize_private(directories)
        expect(not result['invalid_runs'] and not result['unreadable'], 'synthetic gate inputs rejected')
        expect(len(result['scenes']) == 1 and result['scenes'][0]['verdict'] == 'met'
               and len(result['scenes'][0]['runs']) == 3, candidate + ': three runs did not meet the gate')
    print('Synthetic W3: three 192 s, 60 fps runs per candidate accepted; both fixture gate verdicts met.')


def refusal_cases(root):
    refusal(root, 'harness-missing', 'harness', lambda d: (d / 'harness.json').unlink())
    refusal(root, 'harness-json', 'harness', lambda d: (d / 'harness.json').write_text('{', encoding='utf-8'))
    refusal(root, 'harness-pid', 'harness', h_edit(lambda h: h.update(process_id=PID + 1)))
    refusal(root, 'harness-candidate', 'harness', h_edit(lambda h: h.update(candidate='sd')))
    refusal(root, 'app-exit-different', 'app-exit', h_edit(lambda h: h.update(exit_code=2)))
    refusal(root, 'app-exit-three', 'app-exit', h_edit(lambda h: h.update(exit_code=3)), app_exit=3)
    refusal(root, 'native-missing', 'native', lambda d: (d / 'native.json').unlink())
    refusal(root, 'trace-missing', 'native', lambda d: (d / 'trace.jsonl').unlink())
    refusal(root, 'native-scene', 'native', n_edit(lambda n: n.update(scene='w2')))
    refusal(root, 'native-count', 'native', n_edit(lambda n: n.update(frames=n['frames'] - 1)))
    refusal(root, 'native-presents', 'native', h_edit(lambda h: h['window'].update(presents=1)))
    refusal(root, 'native-frame-order', 'native', lambda d: trace_edit(d, lambda e: e[2].update(frame=4)))
    refusal(root, 'native-turn', 'native', n_edit(lambda n: n.update(turn_ms=191)))
    refusal(root, 'native-clock', 'native', n_edit(lambda n: n.update(qpc_frequency=FREQUENCY + 1)))
    refusal(root, 'native-nan', 'native', lambda d: (d / 'native.json').write_text('{"format":NaN}\n', encoding='utf-8'))
    refusal(root, 'identity-required-godot', 'identity', h_edit(lambda h: h['files'].pop('godot:assembly')))
    refusal(root, 'identity-required-qt', 'identity',
            h_edit(lambda h: [h['files'].pop(key) for key in ('qt:Qt6Core.dll', 'qt:Qt6Quick.dll')]), candidate='sd')
    refusal(root, 'identity-file-missing', 'identity',
            h_edit(lambda h: h['files'].update({'godot:exe': str(root / 'missing.exe')})))
    refusal(root, 'identity-dll-digest', 'identity',
            h_edit(lambda h: h['dll_identity']['dll'].update(sha256='0' * 64)))
    refusal(root, 'identity-shader-digest', 'identity',
            h_edit(lambda h: h['dll_identity']['shaders'][0].update(sha256='0' * 64)))
    refusal(root, 'identity-native-digest', 'identity',
            n_edit(lambda n: n['identity']['dll'].update(sha256='0' * 64)))
    for candidate in ('sa2', 'sd'):
        for mode in ('run', 'short', 'geometry', 'validation'):
            refusal(root, f'adapter-{candidate}-{mode}', 'adapter',
                    h_edit(lambda h: h['environment'].update(adapter='Other adapter')), mode=mode, candidate=candidate)
    refusal(root, 'size-changed', 'size-mismatch', h_edit(lambda h: h['sizes'].update(samples_changed=1)))
    refusal(root, 'size-backbuffer', 'size-mismatch', h_edit(lambda h: h['sizes']['backbuffer'].update(width=1280)))
    refusal(root, 'size-native-viewport', 'size-mismatch', n_edit(lambda n: n['viewport'].update(height=800)))
    refusal(root, 'scaling-godot', 'scaling', h_edit(lambda h: h['scaling'].update(content_scale_factor=0.5)))
    refusal(root, 'scaling-qt', 'scaling', h_edit(lambda h: h['scaling'].update(item_width=2048.1)), candidate='sd')
    refusal(root, 'scaling-stretch', 'scaling', h_edit(lambda h: h['scaling'].update(texture_stretch='linear')))
    refusal(root, 'conditions-mode', 'conditions-not-enforced', h_edit(lambda h: h['options'].update(conditions='record')))
    refusal(root, 'conditions-battery-contradiction', 'conditions',
            h_edit(lambda h: h['environment']['power_samples'].update(mains=299, battery=1)))
    refusal(root, 'conditions-zero', 'conditions', h_edit(lambda h: h['window'].update(samples=0)))
    refusal(root, 'conditions-too-few', 'conditions',
            h_edit(lambda h: (h['window'].update(samples=149),
                              h['environment']['power_samples'].update(samples=149, mains=149))))
    refusal(root, 'conditions-power-count', 'conditions',
            h_edit(lambda h: h['environment']['power_samples'].update(samples=299)))
    refusal(root, 'conditions-power-overflow', 'conditions',
            h_edit(lambda h: h['environment']['power_samples'].update(battery=1)))
    for key in ('samples_not_visible', 'samples_covered', 'samples_not_foreground'):
        refusal(root, 'conditions-' + key, 'conditions', h_edit(lambda h: h['window'].update({key: 1})))
    for key in ('visible_throughout', 'foreground_throughout', 'foreground_at_trace_start'):
        refusal(root, 'conditions-' + key, 'conditions', h_edit(lambda h: h['window'].update({key: False})))
    for key in ('gpu_validation', 'no_vram', 'debug_half_target'):
        refusal(root, 'debug-' + key, 'debug-switch', h_edit(lambda h: h['options'].update({key: True})))
    refusal(root, 'debug-layer', 'debug-switch', h_edit(lambda h: h['debug'].update(debug_layer=1)))
    for name, value in (('null', None), ('zero', 0), ('negative', -1), ('bool', True)):
        refusal(root, 'vram-' + name, 'vram-missing', n_edit(lambda n: n.update(vram_peak_mb=value)))
    refusal(root, 'presentmon-missing', 'presentmon', lambda d: (d / 'presentmon.csv').unlink())
    refusal(root, 'presentmon-last-byte', 'presentmon',
            lambda d: (d / 'presentmon.csv').write_bytes((d / 'presentmon.csv').read_bytes().rstrip(b'\n')))
    refusal(root, 'presentmon-column', 'presentmon',
            lambda d: (d / 'presentmon.csv').write_bytes((d / 'presentmon.csv').read_bytes().replace(b'QPCTime', b'CPUStartQPC')))
    refusal(root, 'presentmon-other-pid', 'presentmon',
            lambda d: csv_edit(d, lambda rows: [r.update(ProcessID=str(PID + 1)) for r in rows]))
    refusal(root, 'presentmon-clock', 'presentmon',
            lambda d: csv_edit(d, lambda rows: [r.update(QPCTime=str(START - 1)) for r in rows]))
    refusal(root, 'presentmon-malformed', 'presentmon',
            lambda d: (d / 'presentmon.csv').write_bytes((d / 'presentmon.csv').read_bytes() + b'"unclosed\n'))
    refusal(root, 'swap-chain', 'swap-chain', lambda d: csv_edit(d, lambda rows: rows[800].update(SwapChainAddress='0xDEF')))
    refusal(root, 'sync-interval', 'sync-interval', lambda d: csv_edit(d, lambda rows: rows[800].update(SyncInterval='1')))
    def blind(directory):
        csv_edit(directory, lambda rows: [r.update(Dropped='1') for r in rows
                                         if START + 10 * FREQUENCY <= int(r['QPCTime']) < START + 11 * FREQUENCY])
    refusal(root, 'blind-seconds', 'blind-seconds', blind)
    refusal(root, 'blind-seconds-short', 'blind-seconds', blind, mode='short')
    def duplicate_step(directory):
        trace_edit(directory, lambda entries: entries[800].update(qpc=entries[801]['qpc']))
    refusal(root, 'trace-steps', 'trace-steps', duplicate_step)
    refusal(root, 'trace-steps-short', 'trace-steps', duplicate_step, mode='short')
    refusal(root, 'operator', 'operator', lambda d: None, overlays=None)
    def gate_shape(directory):
        edit(directory, 'harness.json', lambda h: h['options'].update(turn_ms=0))
        edit(directory, 'native.json', lambda n: n.update(turn_ms=0))
    refusal(root, 'gate-shape', 'gate-shape', gate_shape)
    for key in ('error', 'corruption', 'mentioning_sa2'):
        refusal(root, 'validation-' + key, 'validation',
                h_edit(lambda h: h['debug']['counts'].update({key: 1})), mode='validation')
    refusal(root, 'validation-no-counts', 'validation', h_edit(lambda h: h['debug'].update(counts=None)), mode='validation')
    refusal(root, 'validation-layer', 'validation', h_edit(lambda h: h['debug'].update(debug_layer=0)), mode='validation')
    refusal(root, 'validation-option', 'validation', h_edit(lambda h: h['options'].update(gpu_validation=False)), mode='validation')
    refusal(root, 'geometry-format', 'native', lambda d: edit(d, 'geometry.json', lambda g: g.update(format='wrong')), mode='geometry')
    refusal(root, 'geometry-extra-native', 'native', lambda d: write_json(d / 'native.json', {}), mode='geometry')
    refusal(root, 'geometry-extra-trace', 'native', lambda d: (d / 'trace.jsonl').write_text('', encoding='utf-8'), mode='geometry')
    refusal(root, 'geometry-summary-missing', 'native',
            lambda d: edit(d, 'geometry.json', lambda g: g.update(per_cell_count={})), mode='geometry')
    refusal(root, 'geometry-results-malformed', 'native',
            lambda d: edit(d, 'geometry.json', lambda g: g['results'][0].update(samples='unknown')), mode='geometry')
    refusal(root, 'geometry-identity', 'identity',
            h_edit(lambda h: h['dll_identity']['shaders'][0].update(sha256='0' * 64)), mode='geometry')
    # L2-B-01: the DLL's geometry hash must be the hash of the verified identity.
    refusal(root, 'geometry-build-identity', 'identity',
            lambda d: edit(d, 'geometry.json', lambda g: g.update(build_identity='0' * 64)), mode='geometry')
    refusal(root, 'geometry-build-identity-format', 'native',
            lambda d: edit(d, 'geometry.json', lambda g: g.update(build_identity='fixture')), mode='geometry')
    # L2-B-02: each geometry summary agrees with its parts, and a checked record covers S-B's reference set.
    for name, action in (
            ('result-summary', lambda g: g['results'][0].update(failures=1)),
            ('overall-summary', lambda g: g['results'][0].update(failures=1, status='fail')),
            ('count-summary', lambda g: g['per_cell_count']['counts'].__setitem__(0, 1)),
            ('count-failures', lambda g: g['per_cell_count'].update(failures=1)),
            ('count-status', lambda g: g['per_cell_count'].update(status='fail')),
            ('count-cells', lambda g: g['per_cell_count'].update(cells=599)),
            ('incomplete', lambda g: g['results'].pop()),
            ('duplicate', lambda g: g['results'].__setitem__(1, dict(g['results'][0]))),
            ('camera', lambda g: g['results'][0].update(camera='front')),
            ('samples', lambda g: g['results'][0].update(samples=64)),
            ('failures-range', lambda g: g['results'][0].update(failures=3 * 9066 + 1, status='fail')),
            ('not-checked-pass', lambda g: (g.update(error='fixture', results=[]),
                                            g['per_cell_count'].update(status='not-checked', counts=None, failures=None))),
            ('not-checked-results', lambda g: (g.update(status='fail', error='fixture'),
                                               g['per_cell_count'].update(status='not-checked', counts=None, failures=None))),
            ('not-checked-error', lambda g: (g.update(status='fail', results=[]),
                                             g['per_cell_count'].update(status='not-checked', counts=None, failures=None)))):
        refusal(root, 'geometry-' + name, 'native', lambda d, action=action: edit(d, 'geometry.json', action), mode='geometry')
    # L2-B-02: a label summary agrees with S-B's counters and covers every revision the trace reached.
    for name, action in (
            ('status', lambda labels: labels.update(mismatches=1)),
            ('binding', lambda labels: labels.update(status='fail', binding_mismatches=1)),
            ('unreached', lambda labels: labels.update(injection_not_reached=True)),
            ('unreached-type', lambda labels: labels.update(status='fail', injection_not_reached='yes')),
            ('copies', lambda labels: labels.update(copies=labels['copies'] - 1)),
            ('revisions', lambda labels: labels.update(copies=0, revisions=0)),
            ('negative', lambda labels: labels.update(status='fail', missing=-1)),
            ('fields', lambda labels: labels.pop('late_adoptions')),
            ('oracle', lambda labels: labels.update(oracle_sha256={}))):
        refusal(root, 'label-' + name, 'native', n_edit(lambda n, action=action: action(n['label_check'])))
    def half_target(directory):
        edit(directory, 'harness.json', lambda h: (h['options'].update(debug_half_target=True),
                                                  h['sizes']['target'].update(width=1280, height=800)))
        edit(directory, 'native.json', lambda n: (n['target'].update(width=1280, height=800),
                                                 n['viewport'].update(width=1280, height=800)))
    refusal(root, 'short-half-target', 'size-mismatch', half_target, mode='short')
    def no_vram(directory):
        edit(directory, 'harness.json', lambda h: h['options'].update(no_vram=True))
        edit(directory, 'native.json', lambda n: n.update(vram_peak_mb=None, vram_samples=0))
    refusal(root, 'short-no-vram', 'vram-missing', no_vram, mode='short')
    for output in finalizer.OUTPUTS:
        directory = fixture(root, 'output-exists-' + output.replace('.', '-'))
        (directory / output).write_bytes(b'existing output\n')
        before = {path.name: digest(path) for path in directory.iterdir()}
        invoke(directory, expected=5)
        expect(before == {path.name: digest(path) for path in directory.iterdir()}, 'output-exists changed files')
        REFUSALS['output-exists'] += 1


def extra_cases(root):
    for source, mains, battery in (('battery', 0, 300), ('changed', 299, 1),
                                   ('unknown', 299, 0), ('unknown', 298, 1)):
        directory = fixture(root, f'consistent-{source}-{mains}-{battery}')
        edit(directory, 'harness.json', lambda h: (h['environment'].update(power_source=source),
                                                  h['environment']['power_samples'].update(mains=mains, battery=battery)))
        invoke(directory)
        record = read_json(directory / 'run.json')
        expect(finalizer.gate.condition_reasons(record['environment']) == ['conditions-not-met']
               and record['l2']['condition_reasons'] == ['conditions-not-met'], 'consistent power must reach gate')
    directory = fixture(root, 'conditions-missing-declaration')
    edit(directory, 'harness.json', lambda h: h['environment']['declared'].pop('frame_generation'))
    invoke(directory)
    expect(read_json(directory / 'run.json')['l2']['condition_reasons'] == ['conditions-missing'], 'missing conditions report')
    directory = fixture(root, 'conditions-half-boundary')
    edit(directory, 'harness.json', lambda h: (h['window'].update(samples=150),
                                              h['environment']['power_samples'].update(samples=150, mains=150)))
    invoke(directory)
    for mode, output in (('run', 'run.json'), ('short', 'short-check.json'), ('validation', 'validation-record.json')):
        directory = fixture(root, 'failed-label-' + mode, mode=mode)
        edit(directory, 'harness.json', lambda h: h.update(exit_code=2))
        edit(directory, 'native.json', lambda n: n['label_check'].update(status='fail', mismatches=1))
        invoke(directory, mode=mode, app_exit=2, expected=2)
        expect(read_json(directory / output)['label_check']['status'] == 'fail', 'failed label record missing')
    directory = fixture(root, 'geometry-failed', mode='geometry')
    edit(directory, 'harness.json', lambda h: h.update(exit_code=2))
    edit(directory, 'geometry.json', lambda g: (g.update(status='fail', error='synthetic reference mismatch', results=[]),
                                               g['per_cell_count'].update(status='not-checked', counts=None, failures=None)))
    invoke(directory, mode='geometry', app_exit=2, expected=2)
    expect(read_json(directory / 'geometry-record.json')['status'] == 'fail', 'failed geometry record missing')
    directory = fixture(root, 'geometry-failed-counts', mode='geometry')
    edit(directory, 'harness.json', lambda h: h.update(exit_code=2))
    edit(directory, 'geometry.json', lambda g: (g.update(status='fail'), g['results'][4].update(failures=3, status='fail'),
                                               g['per_cell_count']['counts'].__setitem__(7, 30479),
                                               g['per_cell_count'].update(failures=1, status='fail')))
    invoke(directory, mode='geometry', app_exit=2, expected=2)
    expect(read_json(directory / 'geometry-record.json')['status'] == 'fail', 'consistent failed geometry record missing')
    # L2-V-001: the DLL counts each failed coordinate, three per sample, so failures may exceed samples.
    for failures in (9066 + 1, 3 * 9066):
        directory = fixture(root, f'geometry-failed-coordinates-{failures}', mode='geometry')
        edit(directory, 'harness.json', lambda h: h.update(exit_code=2))
        edit(directory, 'geometry.json', lambda g, failures=failures: (
            g.update(status='fail'), g['results'][2].update(failures=failures, status='fail')))
        invoke(directory, mode='geometry', app_exit=2, expected=2)
        expect(read_json(directory / 'geometry-record.json')['geometry']['results'][2]['failures'] == failures,
               'failed coordinate count lost')
    # A W3 trace shorter than one turn reaches no revision; its clean label check copied nothing.
    directory = fixture(root, 'label-no-revision')
    trace_edit(directory, lambda entries: [entry.update(revision=0) for entry in entries])
    edit(directory, 'native.json', lambda n: n['label_check'].update(copies=0, revisions=0))
    invoke(directory)
    directory = fixture(root, 'injected-label')
    edit(directory, 'harness.json', lambda h: (h.update(exit_code=2), h['options'].update(inject='corrupt-label')))
    edit(directory, 'native.json', lambda n: (n.update(injection_applied=True),
                                             n['label_check'].update(status='fail', mismatches=1)))
    invoke(directory, app_exit=2, overlays=None, extra=('--fault-injection',), expected=2)
    record = read_json(directory / 'run.json')
    expect(record['injected_fault'] == 'corrupt-label' and record['injection_applied'] is True
           and record['environment']['declared']['overlays'] == 'fault-injection run; not gate evidence', 'injection declaration')
    for scene in ('w1', 'w2', 'w4'):
        directory = fixture(root, 'valid-' + scene, scene=scene)
        invoke(directory)
        camera = read_json(directory / 'run.json')['camera']
        expect(camera == {'plane': [0, 3] if scene == 'w2' else None,
                          'step_rad': 0.002 if scene == 'w2' else 0, 'per': 'frame' if scene == 'w2' else 'none'},
               scene + ': S-B camera metadata')
    for mode in ('run', 'short', 'geometry', 'validation'):
        directory = fixture(root, 'custom-adapter-' + mode, candidate='sd', mode=mode)
        edit(directory, 'harness.json', lambda h: h['environment'].update(adapter='Fixture adapter'))
        invoke(directory, mode=mode, candidate='sd', extra=('--adapter', 'Fixture adapter'))
    directory = fixture(root, 'qt-scaling-tolerance', candidate='sd')
    edit(directory, 'harness.json', lambda h: h['scaling'].update(item_width=(2560 + 0.0000009) / 1.25))
    invoke(directory, candidate='sd')
    refusal(root, 'qt-scaling-above-tolerance', 'scaling',
            h_edit(lambda h: h['scaling'].update(item_width=(2560 + 0.0000011) / 1.25)), candidate='sd')
    directory = fixture(root, 'presentmon-extra-rows')
    def other_rows(rows):
        rows.extend(({**rows[0], 'ProcessID': str(PID + 1), 'SwapChainAddress': 'other'},
                     {**rows[0], 'QPCTime': str(START - 1), 'SwapChainAddress': 'before'},
                     {**rows[-1], 'QPCTime': str(START + 30 * FREQUENCY), 'SwapChainAddress': 'after'}))
    csv_edit(directory, other_rows)
    invoke(directory)
    expect(read_json(directory / 'run.json')['l2']['presentmon']['rows'] == 1800, 'rows outside trace were counted')
    directory = fixture(root, 'presentmon-no-tearing')
    csv_edit(directory, lambda rows: rows[800].update(AllowsTearing='0'))
    invoke(directory)
    expect(read_json(directory / 'run.json')['environment']['tearing'] is False, 'tearing must use every displayed row')
    # A present dropped before any flip or blit event shows AllowsTearing 0 in PresentMon; dropped rows do not decide tearing.
    directory = fixture(root, 'presentmon-dropped-no-tearing')
    csv_edit(directory, lambda rows: rows[800].update(AllowsTearing='0', Dropped='1'))
    invoke(directory)
    expect(read_json(directory / 'run.json')['environment']['tearing'] is True, 'a dropped row decided tearing')
    directory = fixture(root, 'trace-present-boundary')
    trace_edit(directory, lambda entries: [entry.update(qpc=entry['qpc'] + FREQUENCY // 1000) for entry in entries])
    invoke(directory)
    # A last row at stop is excluded; a step ending at stop - 1 s is included.
    directory = fixture(root, 'trace-last-checked-step', mode='short')
    trace_edit(directory, lambda entries: entries[29 * 60 - 1].update(qpc=entries[29 * 60]['qpc']))
    invoke(directory, mode='short', expected=5)
    expect(read_json(directory / 'refusal.json')['reasons'] == ['trace-steps'], 'last included step was missed')
    REFUSALS['trace-steps'] += 1
    # A trailing 0.5 ms of the short interval without a present is not a blind second; the last whole second is checked.
    def trailing_fraction(directory):
        edit(directory, 'native.json',
             lambda n: n['markers'].update(trace_stop_qpc=START + 30 * FREQUENCY + FREQUENCY // 2000))
    directory = fixture(root, 'blind-seconds-trailing-fraction', mode='short')
    trailing_fraction(directory)
    invoke(directory, mode='short')
    refusal(root, 'blind-seconds-last-whole-second', 'blind-seconds',
            lambda d: (trailing_fraction(d),
                       csv_edit(d, lambda rows: [r.update(Dropped='1') for r in rows
                                                 if START + 28 * FREQUENCY <= int(r['QPCTime']) < START + 29 * FREQUENCY])),
            mode='short')
    # Several independent defects produce a sorted refusal, without a partial run.
    directory = fixture(root, 'multiple-reasons')
    edit(directory, 'harness.json', lambda h: (h['environment'].update(adapter='wrong'),
                                              h['scaling'].update(texture_stretch='wrong')))
    invoke(directory, overlays=None, expected=5)
    expect(read_json(directory / 'refusal.json')['reasons'] == ['adapter', 'operator', 'scaling'], 'sorted reasons')


def identity_cases(root):
    baseline = read_json(root / 'valid-sa2-run' / 'run.json')['build']['build_identity']
    changed_parts = root / 'changed-parts'
    shutil.copytree(root / 'parts', changed_parts)
    # Relocating unchanged bytes does not add private paths to the identity.
    directory = fixture(root, 'identity-relocated', parts=changed_parts)
    invoke(directory)
    expect(read_json(directory / 'run.json')['build']['build_identity'] == baseline, 'paths entered identity')
    (changed_parts / 'sa2_interop.dll').write_bytes(b'only the DLL changed\n')
    directory = fixture(root, 'identity-new-dll', parts=changed_parts)
    invoke(directory)
    expect(read_json(directory / 'run.json')['build']['build_identity'] != baseline, 'DLL did not change identity')
    directory = fixture(root, 'identity-options', scene='w1', mode='validation', seconds=20)
    edit(directory, 'harness.json', lambda h: h['options'].update(
        preroll_ms=0, turn_ms=900, inject=None, declared={'upscaling': True},
        no_vram=True, debug_half_target=True))
    invoke(directory, mode='validation')
    expect(read_json(directory / 'validation-record.json')['build']['build_identity'] == baseline, 'options entered identity')
    directory = fixture(root, 'identity-canonical-settings')
    edit(directory, 'harness.json', lambda h: h.update(configuration={**h['configuration'], 'fixture': '\u00e9'}))
    invoke(directory)
    h = read_json(directory / 'harness.json')
    record = read_json(directory / 'run.json')
    canonical = json.dumps(h['configuration'], sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode('utf-8')
    settings = next(part for part in record['build']['parts'] if part['name'] == 'settings')
    expect(settings['sha256'] == hashlib.sha256(canonical).hexdigest(), 'settings canonical JSON differs')
    text = ''.join(p['name'] + '=' + p['sha256'] + '\n' for p in record['build']['parts']).encode('utf-8')
    expect(record['build']['build_identity'] == hashlib.sha256(text).hexdigest(), 'composite identity differs')
    expect(record['build']['build_identity'] != baseline, 'settings did not change identity')
    directory = fixture(root, 'identity-reordered')
    edit(directory, 'harness.json', lambda h: h.update(configuration=dict(reversed(list(h['configuration'].items()))),
                                                      files=dict(reversed(list(h['files'].items())))))
    invoke(directory)
    expect(read_json(directory / 'run.json')['build']['build_identity'] == baseline, 'object ordering changed identity')


def static_runner():
    path = HERE / 'run_scene.ps1'
    raw = path.read_bytes()
    text = raw.decode('ascii')
    expect(b'\r\n' in raw and b'\n' not in raw.replace(b'\r\n', b''), 'PowerShell must be ASCII with CRLF')
    for token in ('PresentMon-2.6.0-x64.exe', 'Intel/PresentMon/PresentMonConsoleApplication',
                  '--v1_metrics', '--qpc_time', '--process_id', '--output_file', '--session_name',
                  '--terminate_existing_session', 'magic600-sb-capture', 'magic600-sa2-capture',
                  'magic600-sd-capture', 'Threading.Mutex', 'AbandonedMutexException', 'Assert-NoSession',
                  'Assert-Idle', 'Administrator', 'WaitForExit(30000)', 'WaitForExit(5000)',
                  'WaitForExit(60000)', 'Start-Sleep -Seconds 2', 'Start-Sleep -Seconds 20',
                  'ReadByte() -eq 10', 'did you watch the whole run', 'Type yes to keep it',
                  'operator-declared after the run', 'finalize_run.py', 'renderer_gate.py',
                  '--launched-pid', '--app-exit', '--adapter', 'validation_environment',
                  "Replace('{out}', $directory)", '$info.EnvironmentVariables.Remove',
                  '$info.FileName = $launch.executable', '$launchedPid = $process.Id',
                  'codex_review', 'VBCSCompiler', 'blender', 'ffmpeg', 'Get-ChildItem', 'Remove-Session'):
        expect(token in text, 'runner guard missing: ' + token)
    expect('PresentMon.*)' not in text and '^(PresentMon|PresentMonUI|PresentMon-.+)' in text,
           "the idle guard must allow PresentMon's always-on service")
    expect("foreach ($fact in 'frame_generation','upscaling')" in text and '-cmatch "^$fact=(true|false)$"' in text,
           'run mode must require the two gate declarations')
    expect('WriteAllText' not in text and 'Set-Content' not in text and 'Import-Csv' not in text,
           'runner must delegate records and CSV interpretation to the finalizer')
    powershell = shutil.which('powershell.exe')
    if powershell:
        escaped = str(path).replace("'", "''")
        command = ("$tokens = $null; $errors = $null; "
                   "[System.Management.Automation.Language.Parser]::ParseFile('" + escaped
                   + "', [ref]$tokens, [ref]$errors) | Out-Null; "
                   "if ($errors.Count) { $errors | ForEach-Object { $_.Message }; exit 1 }; exit 0")
        result = subprocess.run([powershell, '-NoProfile', '-Command', command],
                                capture_output=True, text=True, timeout=30)
        expect(result.returncode == 0, 'PowerShell parser: ' + result.stdout + result.stderr)
        print('Runner: ASCII/CRLF, guard inventory and PowerShell language parser passed.')
    else:
        print('Runner: ASCII/CRLF and guard inventory passed; PowerShell parser skipped (powershell.exe unavailable).')


def main():
    root = HERE / f'check-{os.getpid()}'
    # Never delete a pre-existing directory belonging to another invocation.
    root.mkdir()
    try:
        make_parts(root)
        valid_modes(root)
        gate_series(root)
        refusal_cases(root)
        extra_cases(root)
        identity_cases(root)
        directory = fixture(root, 'usage')
        before = {p.name: digest(p) for p in directory.iterdir()}
        invoke(directory, extra=('--unknown',), expected=1)
        expect(before == {p.name: digest(p) for p in directory.iterdir()}, 'usage wrote an output')
        expect(set(REFUSALS) == set(finalizer.CHECKS), 'refusal inventory incomplete')
        static_runner()
        print('Refusal fixtures with exactly one reason: ' + ', '.join(f'{key}={REFUSALS[key]}' for key in sorted(REFUSALS)))
        print(f'L2-F acceptance: {CASES} finalizer cases passed; source/fixture evidence only.')
        return 0
    finally:
        # root is the exact newly created child of this script directory.
        expect(root.resolve().parent == HERE and root.name == f'check-{os.getpid()}', 'cleanup boundary')
        shutil.rmtree(root)


if __name__ == '__main__':
    sys.exit(main())
