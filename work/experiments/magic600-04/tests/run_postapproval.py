"""Exclusive actual native fixture, fresh session; no personal data."""
from pathlib import Path
import argparse
import json
import platform
import os
import subprocess
import sys
import time

HERE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(HERE))
from native_launch import (ROOT, RETAINED, EXPERIMENT_SOURCES, BACKEND_SOURCES, SHARED_BACKEND_SOURCES,
    compile_program, compile_recipe, EngineProcess, capture_identity, hash_files, hash_file, check_evidence,
    write_evidence, prepare_runtime, identity_v2)


def main():
    sys.stdout.reconfigure(encoding='utf-8')
    parser = argparse.ArgumentParser()
    parser.add_argument('--mode', choices=('g1', 'g2'), default='g2')
    parser.add_argument('--focus', choices=('cycles', 'names', 'macro-use', 'intents', 'recommendation', 'endgame', 'continuous', 'session', 'compatibility', 'keymap-files', 'keyboard', 'display', 'latency', 'protection'))
    parser.add_argument('--latency-series', choices=('diagnostic', 'full'), help='Explicit diagnostic 5+5 or full 100-per-category native series')
    parser.add_argument('--latency-notes', default='Concurrent workloads not controlled; not a quiet-machine benchmark')
    parser.add_argument('--compile-only', action='store_true', help='Compile and bind inputs without starting engine or GUI')
    parser.add_argument('--recorder', type=Path, help='FFmpeg executable for actual desktop frames; without it each frame is logged as SKIP')
    args = parser.parse_args()
    if (args.focus == 'latency') != (args.latency_series is not None):
        parser.error('--focus latency requires explicit --latency-series diagnostic|full; other focuses do not accept it')
    if args.focus in ('display', 'latency', 'protection') and args.mode != 'g2':
        parser.error('The display/latency/protection fixtures currently use the g2 harness')
    out = HERE / 'native-baseline' / ('postapproval-' + args.mode + '-' + time.strftime('%Y%m%d-%H%M%S'))
    out.mkdir(parents=True, exist_ok=False)
    sources = [ROOT/'native'/p for p in RETAINED] + [HERE/'native'/p for p in EXPERIMENT_SOURCES]
    sources.extend(HERE/'tests'/name for name in ('SessionLogNativeChecks.cs','KeymapFileNativeChecks.cs','DisplayNativeChecks.cs','NativeLatencyChecks.cs'))
    sources.extend(HERE/'tests'/name for name in ('PostApprovalNativeRegression.cs','KeyboardFeedbackNativeChecks.cs','WorkspaceFeedbackNativeChecks.cs','KeyboardCompatibilityNativeChecks.cs','MacroVariantsNativeChecks.cs','StopAcknowledgementNativeChecks.cs','SolveWindowNativeChecks.cs','MacroUseNativeChecks.cs','FunctionsNativeChecks.cs','CycleNativeChecks.cs','MathematicalNameNativeChecks.cs','WorkIntentNativeChecks.cs','CurrentScoreNativeChecks.cs','EndgameNativeChecks.cs','SessionNativeChecks.cs'))
    compiler = Path(os.environ['WINDIR'])/'Microsoft.NET/Framework/v4.0.30319/csc.exe'
    sources.extend(HERE/'tests'/name for name in ('ContinuousSolverNativeChecks.cs','KeyboardBatchNativeChecks.cs','ProtectionPickerNativeChecks.cs'))
    recorder = args.recorder.resolve() if args.recorder else None
    if recorder is not None and not recorder.is_file():
        parser.error('--recorder must name an existing FFmpeg executable')
    product = sources[:len(RETAINED) + len(EXPERIMENT_SOURCES)]
    # The receipt's identity is the product build's; the fixture sources, cases and this script are harness.
    harness = sources[len(product):] + [HERE/'tests/final_workflow_cases.json', Path(__file__).resolve()]
    manifest = capture_identity(product, 'ExperimentProgram', harness=harness)
    manifest['harness_recipe'] = compile_recipe('PostApprovalNativeRegression')
    if recorder is not None:
        manifest['recorder_sha256'] = hash_file(recorder)  # a local tool: its hash is recorded, never its path
    manifest['scope'] = dict(mode=args.mode, focus=args.focus or 'full',
        latency_series=args.latency_series, compile_only=args.compile_only)
    record = out/'build.json'
    exe = out/'PostApprovalNativeRegression.exe'
    compile_program(compiler, exe, sources, 'PostApprovalNativeRegression', out/'compile.log')
    backend = [HERE/name for name in BACKEND_SOURCES] + [ROOT/name for name in SHARED_BACKEND_SOURCES]
    manifest.update(identity_v2.legacy_aliases(manifest['identity']['product'], ROOT, product, backend),
        artifacts=identity_v2.hash_inputs([exe, exe.with_suffix('.exe.config')], ROOT),
        executable_sha256=hash_file(exe))
    unchanged = check_evidence(manifest, 'after_build')
    write_evidence(record, manifest)
    if not unchanged:
        raise RuntimeError('Sources changed during build; run not started')
    if args.compile_only:
        manifest['run_outcome'] = dict(status='not-run', reason='explicit compile-only',
            final_acceptance='not-assessed')
        write_evidence(record, manifest)
        print('Compile only: '+str(record), flush=True)
        return 0
    runtime=out/'runtime'
    manifest['runtime'] = prepare_runtime(runtime)
    if args.focus == 'latency':
        environment = dict(platform=platform.platform(), processor=platform.processor(),
            processor_count=os.cpu_count(), concurrent_work_notes=args.latency_notes,
            build_receipt='build.json', executable_sha256=manifest['executable_sha256'],
            build_identity=manifest['build_identity'], source_hashes=manifest['identity']['product']['sources'],
            model=manifest['identity']['product']['model'],
            toolchain=dict(compiler=manifest['identity']['compiler'], interpreter=manifest['identity']['interpreter']),
            runtime=manifest['runtime'],
            series=args.latency_series, gpu_inventory_scope='Installed Windows adapters; not proof of which adapter presented a sample')
        try:
            gpu = subprocess.run(['powershell.exe', '-NoProfile', '-NonInteractive', '-Command',
                '[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding; Get-CimInstance Win32_VideoController | Select-Object Name,DriverVersion,VideoModeDescription,CurrentHorizontalResolution,CurrentVerticalResolution | ConvertTo-Json -Compress'],
                stdout=subprocess.PIPE, stderr=subprocess.PIPE, encoding='utf-8', errors='replace', timeout=20)
            environment['gpu_inventory'] = json.loads(gpu.stdout) if gpu.returncode == 0 else dict(status='unavailable', exit_code=gpu.returncode)
        except (OSError, subprocess.TimeoutExpired, ValueError) as error:
            environment['gpu_inventory'] = dict(status='unavailable', error_type=type(error).__name__)
        environment_path = out/'latency-environment.json'
        environment_path.write_text(json.dumps(environment, indent=2), encoding='utf-8')
        manifest['artifacts'].update(hash_files([environment_path]))
        write_evidence(record, manifest)
    print('Evidence: '+str(out),flush=True)
    result = None
    try:
        if not check_evidence(manifest, 'before_engine'):
            raise RuntimeError('Inputs changed before engine start; no native run started')
        write_evidence(record, manifest)
        with EngineProcess(ROOT,out/'session',out/'launch.json',out/'engine.log',engine_command=[sys.executable,'-B',str(HERE/'engine.py')],hidden_console=True,timeout=180) as engine:
            if not check_evidence(manifest, 'before_start'):
                raise RuntimeError('Inputs changed during engine startup; no native run started')
            write_evidence(record, manifest)
            with (out/'run.log').open('w',encoding='utf-8') as log:
                capture_env = os.environ.copy()
                for key in ('MAGIC600_NATIVE_FOCUS', 'MAGIC600_NATIVE_LATENCY', 'MAGIC600_LATENCY_SERIES', 'MAGIC600_LATENCY_ENVIRONMENT'):
                    capture_env.pop(key, None)
                if args.focus:
                    capture_env['MAGIC600_NATIVE_FOCUS'] = args.focus
                if args.focus == 'latency':
                    capture_env['MAGIC600_NATIVE_LATENCY'] = '1'
                    capture_env['MAGIC600_LATENCY_SERIES'] = args.latency_series
                    capture_env['MAGIC600_LATENCY_ENVIRONMENT'] = str(environment_path)
                capture_env.pop('MAGIC600_CAPTURE_FFMPEG', None)
                if recorder is not None:
                    capture_env['MAGIC600_CAPTURE_FFMPEG'] = str(recorder)
                capture_env['MAGIC600_CONTINUOUS_CASES'] = str(HERE/'tests/final_workflow_cases.json')
                capture_env['MAGIC600_NATIVE_OUTPUT'] = str(out)
                result=subprocess.run([str(exe),str(runtime/'MPUlt.exe'),engine.info['base'],engine.info['token'],str(out),args.mode],cwd=runtime,stdout=log,stderr=subprocess.STDOUT,timeout=1800 if args.focus in ('continuous', 'latency') else 900,env=capture_env)
            lines=(out/'run.log').read_text(encoding='utf-8').splitlines()
            print(f'Exit: {result.returncode}; passed assertions: {sum(line.startswith("PASS ") for line in lines)}; full log: {out / "run.log"}',flush=True)
            if result.returncode:
                print('\n'.join(lines[-24:]),flush=True)
    finally:
        unchanged = check_evidence(manifest, 'after_run')
        manifest['run_outcome'] = dict(exit_code=None if result is None else result.returncode,
            returned=result is not None, immutable_inputs_unchanged=unchanged,
            final_acceptance='not-assessed')
        write_evidence(record, manifest)
    if not unchanged:
        print('Immutable inputs changed during the run; results are not current. Read build.json checks.', flush=True)
        return 1
    return result.returncode


if __name__=='__main__':
    raise SystemExit(main())
