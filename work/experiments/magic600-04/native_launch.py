"""Build and run isolated native review samples using the retained Windows runtime."""
from pathlib import Path
import argparse
import hashlib
import json
import os
import platform
import shutil
import struct
import subprocess
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT))
from native.bootstrap import compile_program
from native.directx_runtime import find_directx
from native.inspect_runtime import inspect
from engine_process import EngineProcess

RETAINED = ('NativeHost.cs', 'NativeDockLayout.cs', 'NativeDiagnostics.cs',
    'NativeRendererLifecycle.cs', 'NativeSnapshot.cs', 'NativeStickerAccess.cs',
    'NativeRenderSubset.cs', 'NativePickingVisibility.cs', 'NativeFullRenderer.cs',
    'NativeColorGraph.cs', 'NativeStructureExplorer.cs', 'NativeAuxiliaryViews.cs')
EXPERIMENT_SOURCES = ('ExperimentCellView.cs', 'ExperimentInput.cs', 'ExperimentHub.cs',
    'ExperimentBridge.cs', 'ExperimentShell.cs', 'ExperimentWorkspace.cs', 'ExperimentTools.cs',
    'ExperimentKeyboard.cs', 'ExperimentPhase.cs', 'ExperimentFilter.cs', 'ExperimentWindows.cs', 'ExperimentWorkSheets.cs', 'ExperimentWindowPlacement.cs', 'ExperimentMacroLibrary.cs', 'ExperimentHelp.cs', 'ExperimentMacroRelations.cs', 'ExperimentSolveWindow.cs', 'ExperimentCycles.cs', 'ExperimentHubCycles.cs', 'ExperimentRecommendation.cs', 'ExperimentReferenceVariants.cs', 'ExperimentEndgame.cs', 'ExperimentResidualWorkflow.cs', 'ExperimentCandidates.cs', 'ExperimentSession.cs', 'ExperimentDisplay.cs')
BACKEND_SOURCES = ('engine.py', 'adapter.py', 'draft_inspection.py', 'filter_projection.py',
    'grip_frames.py', 'keymap_catalog.py', 'mathematical_names.py', 'transported_frames.py', 'residuals.py', 'work_intents.py', 'current_recommendation.py', 'position_requirements.py', 'local_geometry.py', 'work_sheets.py', 'macro_library.py', 'macro_use.py', 'cycle_projection.py', 'reference_variants.py', 'endgame_library.py', 'endgame_invariants.py', 'orbit_invariants.py', 'workflow_continuity.py', 'candidate_analysis.py', 'session_workflow.py', 'evidence/orbit-invariants-20260916-generators.json')
SHARED_BACKEND_SOURCES = ('grips.py', 'server.py', 'core.py', 'session.py',
    'session_lock.py', 'enhanced.py', 'native_bridge.py', 'engine_process.py',
    'log_io.py', 'mpult_log.py')
HARNESS_SOURCES = (HERE / 'native_launch.py', HERE / 'tests/run_postapproval.py',
    ROOT / 'native/bootstrap.py', ROOT / 'native/directx_runtime.py',
    ROOT / 'native/inspect_runtime.py', ROOT / 'native/NativeHost.exe.config')


def hash_file(path):
    digest = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            digest.update(block)
    return digest.hexdigest()


def hash_files(paths, root=ROOT):
    return {os.path.relpath(Path(path).resolve(), root): hash_file(path) for path in paths}


def model_evidence(root=ROOT):
    path = root / 'assets/manifest.json'
    manifest = json.loads(path.read_text(encoding='utf-8-sig'))
    assets = hash_files([root / name for name in manifest['files']], root)
    for name, expected in manifest['files'].items():
        if assets[os.path.normpath(name)] != expected:
            raise ValueError('Asset hash mismatch: ' + name)
    return dict(model_id=manifest['model_id'], status='hash-verified',
        manifest=hash_files([path], root), assets=assets)


def capture_evidence(sources, compiler, extra_tools=()):
    import numpy as np
    from numpy._core import _multiarray_umath
    inputs = list(sources) + [HERE / name for name in BACKEND_SOURCES]
    inputs += [ROOT / name for name in SHARED_BACKEND_SOURCES] + list(HARNESS_SOURCES)
    compiler_info = subprocess.run([str(compiler), '/help'], stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT, encoding='utf-8', errors='replace', timeout=20, check=True)
    tools = [Path(sys.executable), Path(np.__file__), Path(_multiarray_umath.__file__), compiler]
    tools += list(extra_tools)
    return dict(evidence_version=1, sources=hash_files(dict.fromkeys(inputs)),
        model=model_evidence(), toolchain=dict(files=hash_files(tools),
            python=dict(version=sys.version, bits=struct.calcsize('P') * 8,
                        executable=os.path.relpath(sys.executable, ROOT)),
            numpy=dict(version=np.__version__, origin=os.path.relpath(np.__file__, ROOT)),
            compiler=dict(banner=compiler_info.stdout.splitlines()[0],
                          path=os.path.relpath(compiler, ROOT), target='x86'),
            platform=platform.platform()), checks={})


def check_evidence(evidence, phase, root=ROOT):
    """Hash only declared immutable inputs; record settings changes separately."""
    model, runtime = evidence.get('model', {}), evidence.get('runtime', {})
    groups = [evidence.get('sources', {}), model.get('manifest', {}), model.get('assets', {}),
        evidence.get('toolchain', {}).get('files', {}), evidence.get('artifacts', {}),
        runtime.get('origins', {}), runtime.get('immutable', {})]
    expected = {}
    for group in groups:
        expected.update(group)
    changed, missing = [], []
    for path, checksum in expected.items():
        try:
            if hash_file(root / path) != checksum:
                changed.append(path)
        except OSError:
            missing.append(path)
    evidence.setdefault('checks', {})[phase] = dict(
        status='changed' if changed or missing else 'unchanged',
        checked_files=len(expected), changed=changed, missing=missing)
    if runtime.get('mutable_initial'):
        runtime['mutable_' + phase] = {
            name: hash_file(root / name) if (root / name).is_file() else None
            for name in runtime['mutable_initial']}
    return not changed and not missing


def write_evidence(path, evidence):
    path.write_text(json.dumps(evidence, indent=2), encoding='utf-8')


def prepare_runtime(runtime):
    """Keep the supported loader and bind the actual copied assemblies."""
    source = ROOT / 'native/runtime/MPUlt.exe'
    metadata = inspect(source)
    if metadata['missing']:
        raise RuntimeError('Retained native reflection contract failed: ' + ', '.join(metadata['missing']))
    runtime.mkdir(parents=True, exist_ok=True)
    assemblies = {'MPUlt.exe': source, **find_directx(preferred=(source.parent, runtime))}
    origins = hash_files(assemblies.values())
    for name, path in assemblies.items():
        if path.resolve() != (runtime / name).resolve():
            shutil.copy2(path, runtime / name)
        if hash_file(runtime / name) != origins[os.path.relpath(path.resolve(), ROOT)]:
            raise RuntimeError('Runtime copy changed: ' + name)
    for name in ('MPUlt_puzzles.txt', 'MPUlt_settings.txt'):
        if not (runtime / name).exists():
            shutil.copy2(source.parent / name, runtime / name)
    # No supported fixture writes puzzle definitions. Only preferences are mutable.
    immutable = hash_files([runtime / name for name in assemblies] + [runtime / 'MPUlt_puzzles.txt'])
    return dict(origins=origins, immutable=immutable,
        mutable_initial=hash_files([runtime / 'MPUlt_settings.txt']),
        reflection_contract='verified', directx_loader='native.directx_runtime.find_directx')


def build(directory):
    sources = [ROOT / 'native' / name for name in RETAINED]
    sources += [HERE / 'native' / name for name in EXPERIMENT_SOURCES]
    compiler = Path(os.environ.get('WINDIR', r'C:\Windows')) / 'Microsoft.NET/Framework/v4.0.30319/csc.exe'
    manifest = capture_evidence(sources, compiler)
    identity = hashlib.sha256(json.dumps(manifest, sort_keys=True).encode()).hexdigest()
    directory = directory / identity[:16]
    directory.mkdir(parents=True, exist_ok=True)
    host = directory / 'Magic600Experiment.exe'
    record = directory / 'build.json'
    old = json.loads(record.read_text(encoding='utf-8')) if record.exists() else {}
    reusable = (old.get('build_identity') == identity and host.is_file()
        and old.get('checks', {}).get('after_build', {}).get('status') == 'unchanged'
        and old.get('executable_sha256') == hash_file(host)
        and host.with_suffix('.exe.config').is_file()
        and hash_file(host.with_suffix('.exe.config')) == hash_file(ROOT / 'native/NativeHost.exe.config'))
    if not reusable:
        compile_program(compiler, host, sources, 'ExperimentProgram', directory / 'compile.log')
    manifest.update(build_identity=identity,
        artifacts=hash_files([host, host.with_suffix('.exe.config')]),
        executable_sha256=hash_file(host),
        source={str(p.relative_to(ROOT)): manifest['sources'][str(p.relative_to(ROOT))] for p in sources},
        backend_sources={str(p.relative_to(ROOT)): manifest['sources'][str(p.relative_to(ROOT))]
            for p in [HERE / name for name in BACKEND_SOURCES] + [ROOT / name for name in SHARED_BACKEND_SOURCES]},
        backend_sha256=manifest['sources'][str((HERE / 'adapter.py').relative_to(ROOT))],
        stage='Post-approval development; final acceptance not yet established',
        approvals=dict(G1='Approved by user 2026-09-16: a156 sample', G2='Approved by user 2026-09-16: a156 sample'))
    unchanged = check_evidence(manifest, 'after_build')
    write_evidence(record, manifest)
    if not unchanged:
        raise RuntimeError('Native sources changed during compilation; this build is not identified or launched. Rebuild after edits finish.')
    return host


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--mode', choices=('g1', 'g2'), required=True)
    parser.add_argument('--session', default='native-review')
    parser.add_argument('--build-only', action='store_true')
    args = parser.parse_args()
    if not args.session.replace('-', '').replace('_', '').isalnum():
        parser.error('Session name must contain letters, numbers, hyphens or underscores')
    if os.name != 'nt':
        parser.error('This sample requires the retained Windows native runtime')
    directory = HERE / 'native-build'
    host = build(directory)
    print('Native build: ' + str(host.parent), flush=True)
    if args.build_only:
        print(host)
        return 0
    data = HERE / 'sessions' / (args.mode + '-' + args.session)
    runtime = data / 'runtime'
    record = host.parent / 'build.json'
    manifest = json.loads(record.read_text(encoding='utf-8'))
    manifest['runtime'] = prepare_runtime(runtime)
    result = None
    try:
        if not check_evidence(manifest, 'before_engine'):
            raise RuntimeError('Native evidence inputs changed before engine start; rebuild.')
        write_evidence(record, manifest)
        with EngineProcess(ROOT, data, data / 'launch.json', data / 'engine.log',
                engine_command=[sys.executable, '-B', str(HERE / 'engine.py')],
                hidden_console=True, timeout=180, progress=lambda message: print(message, flush=True)) as engine:
            if not check_evidence(manifest, 'before_start'):
                raise RuntimeError('Native evidence inputs changed during engine startup; sample not launched.')
            write_evidence(record, manifest)
            print('Starting ' + args.mode.upper() + ' in the retained native program. Session: ' + str(data), flush=True)
            result = subprocess.call([str(host), str(runtime / 'MPUlt.exe'), engine.info['base'], engine.info['token'], args.mode], cwd=runtime)
    finally:
        unchanged = check_evidence(manifest, 'after_run')
        manifest['run_outcome'] = dict(exit_code=result, returned=result is not None,
            immutable_inputs_unchanged=unchanged, final_acceptance='not-assessed')
        write_evidence(record, manifest)
    return result if unchanged else 1


if __name__ == '__main__':
    sys.exit(main())
