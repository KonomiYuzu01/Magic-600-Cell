"""Compile and launch the native host without modifying an old install.

WinForms fixture windows run only in explicit developer test modes. The separate
live bridge still verifies the actual MPUlt model at every connection.
"""
from __future__ import annotations
import argparse, hashlib, json, locale, os, pathlib, shutil, struct, subprocess, sys, tempfile, time
ROOT = pathlib.Path(__file__).resolve().parent.parent
VERSION = '0.3'
sys.path.insert(0, str(ROOT))
from engine_process import EngineProcess, read_launch


# The historical references: the five named here plus the .NET Framework 4.x default
# csc.rsp set and the implicit mscorlib. Compilation seals them: /noconfig and /nostdlib+
# stop csc from reading csc.rsp or adding mscorlib itself, and every system reference is
# passed by full path from the compiler's own Framework directory.
EXPLICIT_REFERENCES = ('System.dll', 'System.Core.dll', 'System.Drawing.dll',
                       'System.Windows.Forms.dll', 'System.Web.Extensions.dll')
CSC_RSP_REFERENCES = (
    'Accessibility.dll', 'Microsoft.CSharp.dll', 'System.Configuration.dll', 'System.Configuration.Install.dll',
    'System.Core.dll', 'System.Data.dll', 'System.Data.DataSetExtensions.dll', 'System.Data.Linq.dll',
    'System.Data.OracleClient.dll', 'System.Deployment.dll', 'System.Design.dll', 'System.DirectoryServices.dll',
    'System.dll', 'System.Drawing.Design.dll', 'System.Drawing.dll', 'System.EnterpriseServices.dll',
    'System.Management.dll', 'System.Messaging.dll', 'System.Runtime.Remoting.dll',
    'System.Runtime.Serialization.dll', 'System.Runtime.Serialization.Formatters.Soap.dll', 'System.Security.dll',
    'System.ServiceModel.dll', 'System.ServiceModel.Web.dll', 'System.ServiceProcess.dll', 'System.Transactions.dll',
    'System.Web.dll', 'System.Web.Extensions.Design.dll', 'System.Web.Extensions.dll', 'System.Web.Mobile.dll',
    'System.Web.RegularExpressions.dll', 'System.Web.Services.dll', 'System.Windows.Forms.dll',
    'System.Workflow.Activities.dll', 'System.Workflow.ComponentModel.dll', 'System.Workflow.Runtime.dll',
    'System.Xml.dll', 'System.Xml.Linq.dll')
COMPILER_FILES = ('csc.exe', 'csc.exe.config', 'alink.dll', '1033/cscui.dll', '1033/alinkui.dll',
                  'default.win32manifest')


def system_references():
    names, seen = [], set()
    for name in ('mscorlib.dll',) + EXPLICIT_REFERENCES + CSC_RSP_REFERENCES:
        if name.lower() not in seen:
            seen.add(name.lower())
            names.append(name)
    return names


def compile_recipe(main_type):
    """Everything that decides the compilation except file contents; the compiler is a role."""
    return dict(compiler='csc', flags=['/nologo', '/noconfig', '/nostdlib+',
        '/target:exe' if main_type.endswith('Regression') else '/target:winexe', '/platform:x86', '/debug+',
        '/optimize+', '/utf8output', '/codepage:65001', '/win32manifest:default.win32manifest',
        '/main:' + main_type], references=system_references(), config_copy='native/NativeHost.exe.config')


def compile_command(csc, output, sources, main_type, extra_refs=()):
    """The sealed csc invocation for compile_recipe(main_type); extra references keep their given paths."""
    framework = pathlib.Path(csc).resolve().parent
    if os.environ.get('LIB'):
        raise RuntimeError('LIB is set; csc would search it for references. Unset LIB and build again.')
    recipe = compile_recipe(main_type)
    refs = []
    for name in recipe['references']:
        path = framework / name
        if not path.is_file():
            raise RuntimeError('Reference assembly missing from the compiler directory: ' + name)
        refs.append(path)
    extras = [pathlib.Path(p).resolve() for p in extra_refs]
    taken = {p.name.lower() for p in refs}
    for path in extras:
        if not path.is_file() or path.is_symlink():
            raise RuntimeError('Extra reference is not a regular file: ' + str(path))
        if path.name.lower() in taken:
            raise RuntimeError('Extra reference duplicates a system reference: ' + path.name)
        taken.add(path.name.lower())
    flags = [flag.replace('default.win32manifest', str(framework / 'default.win32manifest')) for flag in recipe['flags']]
    return ([str(csc)] + flags + ['/out:' + str(output)] + ['/reference:' + str(p) for p in refs + extras]
            + [str(p) for p in sources])


def reference_bindings(extra_refs):
    """Role (lower-case file name) to sha256 for each extra reference; test builds only, never the product identity."""
    return {'extra:' + pathlib.Path(p).name.lower(): hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest()
            for p in extra_refs}


def compile_program(csc, output, sources, main_type, log, extra_refs=()):
    """Compile; returns the extra references' role bindings, rechecked after compilation."""
    command = compile_command(csc, output, sources, main_type, extra_refs)
    bindings = reference_bindings(extra_refs)
    work = pathlib.Path(tempfile.mkdtemp(prefix='magic600-csc-'))  # an empty directory: no same-name DLL to find
    try:
        result = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, cwd=work,
                                encoding='utf-8', errors='replace')
    finally:
        shutil.rmtree(work, ignore_errors=True)
    log.write_text(result.stdout, encoding='utf-8')
    if result.returncode:
        raise RuntimeError('C# compilation failed. Read ' + str(log) + '\n' + result.stdout)
    shutil.copy2(ROOT / recipe_config(), output.with_suffix('.exe.config'))
    if reference_bindings(extra_refs) != bindings:
        raise RuntimeError('An extra reference changed during compilation; ' + str(output) + ' is not bound')
    return bindings


def recipe_config():
    return compile_recipe('Program')['config_copy']


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--runtime', type=pathlib.Path)
    parser.add_argument('--data', type=pathlib.Path)
    parser.add_argument('--startup-timeout', type=float, default=180.0, help='State engine readiness deadline in seconds; failures include the last stage and log')
    parser.add_argument('--self-test-only', action='store_true', help='Compile and test real WinForms controls; do not start MPUlt or open the puzzle database')
    parser.add_argument('--renderer-test', action='store_true', help='Run actual MPUlt/DirectX rendering and journal regressions. Requires an explicit separate --data test directory.')
    parser.add_argument('--performance-test', action='store_true', help='Measure actual native camera, UI responsiveness and scramble performance with correctness gates. Requires a fresh separate --data directory.')
    args = parser.parse_args()
    test_mode = args.renderer_test or args.performance_test
    if test_mode and (args.data is None or args.self_test_only or (args.renderer_test and args.performance_test)):
        parser.error('Choose one native test mode with a separate --data directory, without --self-test-only')
    if not 1 <= args.startup_timeout <= 3600:
        parser.error('--startup-timeout must be between 1 and 3600 seconds')
    if struct.calcsize('P') != 8:
        raise SystemExit('Use 64-bit Python for the state engine.')
    if os.name != 'nt':
        raise SystemExit('The native host requires Windows. No WinForms/DirectX test has run on this platform.')
    csc = pathlib.Path(os.environ.get('WINDIR', r'C:\Windows')) / 'Microsoft.NET/Framework/v4.0.30319/csc.exe'
    if not csc.is_file():
        raise RuntimeError('.NET Framework 4.x csc.exe was not found. No application or security settings were changed.')
    data = (args.data or pathlib.Path(os.environ.get('LOCALAPPDATA', str(pathlib.Path.home()))) / 'C600Studio').resolve()
    if test_mode and (data / 'session.sqlite3').exists():
        raise RuntimeError('Native regression writes test moves. Choose a fresh --data directory with no existing session.sqlite3.')
    build = data / ('native-host-' + VERSION)
    diagnostics = build / 'diagnostics'
    diagnostics.mkdir(parents=True, exist_ok=True)
    sources = [ROOT / 'native' / name for name in ('NativeHost.cs', 'NativeDockLayout.cs', 'NativeDiagnostics.cs', 'NativeRendererLifecycle.cs', 'NativeSnapshot.cs', 'NativeStickerAccess.cs', 'NativeRenderSubset.cs', 'NativePickingVisibility.cs', 'NativeFullRenderer.cs', 'NativeColorGraph.cs', 'NativeStructureExplorer.cs', 'NativeCellView.cs', 'NativeAuxiliaryViews.cs')]
    sources_hash = hashlib.sha256(b''.join(p.read_bytes() for p in sources)).hexdigest()
    (diagnostics / 'build-info.json').write_text(json.dumps(dict(version=VERSION, source_sha256=sources_hash,
          python=sys.version, process_bits=64, native_bits=32), indent=2), encoding='utf-8')
    regression = diagnostics / 'NativeHostRegression.exe'
    print('Compiling native host...', flush=True)
    host = build / 'C600Native.exe'
    compile_program(csc, host, sources, 'Program', diagnostics / 'host-build.log')
    if args.self_test_only or test_mode:
        compile_program(csc, regression, sources + [ROOT / 'tests/native/NativeHostRegression.cs'],
                        'NativeHostRegression', diagnostics / 'self-test-build.log')
        report = diagnostics / 'winforms-self-test.json'
        report.unlink(missing_ok=True)
        print('Running explicit developer WinForms startup/layout regression...', flush=True)
        result = subprocess.run([str(regression), str(report)], cwd=diagnostics,
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=120,
                                encoding=locale.getpreferredencoding(False), errors='replace')
        (diagnostics / 'winforms-self-test.log').write_text(result.stdout, encoding='utf-8')
        if result.returncode or not report.exists() or not json.loads(report.read_text(encoding='utf-8-sig')).get('passed'):
            raise RuntimeError('WinForms startup regression failed. Native launch stopped before loading MPUlt. Read ' + str(diagnostics))
        print('WinForms startup/layout regression passed. Report: ' + str(report), flush=True)
    if args.self_test_only:
        return 0
    exe = (args.runtime or ROOT / 'native/runtime/MPUlt.exe').resolve()
    if not exe.is_file():
        raise RuntimeError('MPUlt.exe is missing: ' + str(exe))
    from inspect_runtime import inspect
    check = inspect(exe)
    if check['missing']:
        raise RuntimeError('Unsupported MPUlt reflection surface: ' + ', '.join(check['missing']))
    runtime = build / 'runtime' / hashlib.sha256(exe.read_bytes()).hexdigest()[:16]
    runtime.mkdir(parents=True, exist_ok=True)
    source = exe.parent
    shutil.copy2(exe, runtime / 'MPUlt.exe')
    from directx_runtime import find_directx
    for name, found in find_directx(preferred=(source, source / 'v9.02.2904', runtime)).items():
        if found.resolve() != (runtime / name).resolve():
            shutil.copy2(found, runtime / name)
    old_builds = [data / ('native-host-' + v) for v in ('0.2.4', '0.2.3', '0.2.2', '0.2.1', '0.2')]
    for name in ('MPUlt_puzzles.txt', 'MPUlt_settings.txt'):
        if not (runtime / name).exists():
            prior = next((old / 'runtime' / runtime.name / name for old in old_builds
                          if (old / 'runtime' / runtime.name / name).exists()), ROOT / 'native/runtime' / name)
            shutil.copy2(prior, runtime / name)
    if not (build / 'native_keys.json').exists():
        prior = next((old / 'native_keys.json' for old in old_builds if (old / 'native_keys.json').exists()), None)
        if prior is not None:
            shutil.copy2(prior, build / 'native_keys.json')
    launch = build / 'launch.json'
    launch.unlink(missing_ok=True)
    exe = runtime / 'MPUlt.exe'
    if test_mode:
        test_type = 'NativePerformanceRegression' if args.performance_test else 'NativeRendererRegression'
        host = build / (test_type + '.exe')
        directx = [runtime / name for name in ('Microsoft.DirectX.dll', 'Microsoft.DirectX.Direct3D.dll', 'Microsoft.DirectX.Direct3DX.dll')]
        references = compile_program(csc, host, sources + [ROOT / 'tests/native' / (test_type + '.cs')] + ([] if args.performance_test else [ROOT / 'tests/native/NativePickingRegression.cs', ROOT / 'tests/native/NativeSessionLogRegression.cs', ROOT / 'tests/native/NativeFramePolicyRegression.cs', ROOT / 'tests/native/NativeFeatureRegression.cs', ROOT / 'tests/native/NativeAuxiliaryNativeRegression.cs']),
                        test_type, diagnostics / ('performance-test-build.log' if args.performance_test else 'renderer-test-build.log'),
                        directx)
        # A local test receipt: the DirectX assemblies are bound by role and hash only, never copied into the repository.
        (diagnostics / (test_type + '-references.json')).write_text(json.dumps(references, indent=2), encoding='utf-8')
    with EngineProcess(ROOT, data, launch, build / 'engine.log',
                       timeout=args.startup_timeout, progress=lambda s: print(s, flush=True)) as engine:
        info = engine.info
        print('Runtime copy: ' + str(exe), flush=True)
        print('Session: ' + str(data), flush=True)
        print('State engine verified. Starting the actual MPUlt window and live geometry bridge.', flush=True)
        print('Logs: ' + str(diagnostics), flush=True)
        command = [str(host), str(exe), info['base'], info['token']]
        if test_mode:
            if reference_bindings(directx) != references:
                raise RuntimeError('A DirectX reference changed after compilation; the regression did not run')
            command.append(str(diagnostics))
            return subprocess.run(command, cwd=runtime, timeout=1800 if args.performance_test else 600).returncode
        return subprocess.call(command, cwd=runtime)


if __name__ == '__main__':
    try:
        sys.exit(main())
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        raise SystemExit(str(error))
