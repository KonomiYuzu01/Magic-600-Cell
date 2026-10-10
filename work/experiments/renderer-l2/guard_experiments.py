"""Synthetic L2-V-002 experiments on this machine: no GPU, PresentMon or window.

PLAN-L2-V-002.md section 7, owner-machine experiment 1. Each experiment prints one
line with its result and the observed facts, never a path or the user name.

Usage: python -B work/experiments/renderer-l2/guard_experiments.py --build work/sdb/<stamp>
The build folder must hold the Qt build of build.cmd: app/sd_module_probe.dll,
app/sd_module_observer_test.exe and the windeployqt output in deploy/. All
fixtures live in a fresh folder under it and are removed on every exit.
"""
import sys

sys.dont_write_bytecode = True

import argparse
import contextlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import time
import uuid

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import file_guard as guard  # noqa: E402

COUNTS = {'pass': 0, 'not run': 0, 'fail': 0}
LOADER = r'''#include <windows.h>
int wmain(int argc, wchar_t** argv) {
    if (argc != 2) return 2;
    const HMODULE module = LoadLibraryW(argv[1]);
    if (!module) return 3;
    wchar_t path[32768];
    const DWORD length = GetModuleFileNameW(module, path, 32768);
    if (!length || length >= 32768) return 4;
    char text[98304];
    const int bytes = WideCharToMultiByte(CP_UTF8, 0, path, int(length), text, int(sizeof(text)), nullptr, nullptr);
    DWORD written = 0;
    if (bytes <= 0 || !WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), text, DWORD(bytes), &written, nullptr)) return 5;
    return 0;
}
'''


class NotRun(Exception):
    pass


def report(name, action):
    try:
        detail = action()
        status = 'pass'
    except NotRun as reason:
        status, detail = 'not run', str(reason)
    except Exception as error:  # Every experiment reports; none stops the others.
        status, detail = 'fail', f'{type(error).__name__}: {error}'
    COUNTS[status] += 1
    print(f'{name}: {status}: {detail}', flush=True)


def expect(condition, message):
    if not condition:
        raise AssertionError(message)


def refusal(paths):
    try:
        held = guard.acquire([str(path) for path in paths])
    except guard.GuardRefused as error:
        return error.step
    held.close()
    return None


@contextlib.contextmanager
def fresh(root):
    directory = root / uuid.uuid4().hex
    directory.mkdir()
    try:
        yield directory
    finally:
        expect(directory.parent == root, 'fixture cleanup boundary')
        shutil.rmtree(directory)


def subst_alias(root):
    with fresh(root) as directory:
        (directory / 'part.dll').write_bytes(b'subst alias')
        drives = {drive[0].upper() for drive in os.listdrives()}
        free = [letter for letter in 'ZYXWVUTSRQPONMLKJ' if letter not in drives]
        if not free:
            raise NotRun('no free drive letter')
        drive = free[0] + ':'
        subprocess.run(['subst', drive, str(directory)], check=True, capture_output=True, timeout=30)
        try:
            step = refusal([drive + '\\part.dll'])
        finally:
            subprocess.run(['subst', drive, '/d'], check=True, capture_output=True, timeout=30)
        expect(free[0] not in {item[0].upper() for item in os.listdrives()}, 'subst drive left behind')
        expect(step is not None, 'guard accepted a subst drive alias')
        return f'guard refused the subst drive alias at {step}'


def symlink_alias(root):
    with fresh(root) as directory:
        target = directory / 'part.dll'
        target.write_bytes(b'symlink alias')
        try:
            os.symlink(target, directory / 'link.dll')
        except OSError as error:
            raise NotRun(f'os.symlink unavailable (Windows error {error.winerror})')
        step = refusal([directory / 'link.dll'])
        expect(step is not None, 'guard accepted a symbolic link')
        return f'guard refused the symbolic link at {step}'


def unc_alias(root):
    with fresh(root) as directory:
        target = directory / 'part.dll'
        target.write_bytes(b'unc alias')
        unc = '\\\\localhost\\' + target.drive[0] + '$' + str(target)[2:]
        try:
            with open(unc, 'rb') as stream:
                readable = stream.read() == b'unc alias'
        except OSError as error:
            readable = f'no (Windows error {error.winerror})'
        step = refusal([unc])
        expect(step == 'path-form', f'guard refused the UNC alias at {step}, not path-form')
        return f'administrative share read: {"yes" if readable is True else readable}; guard refused the UNC form at {step}'


def hard_link(root):
    with fresh(root) as directory:
        target = directory / 'part.dll'
        target.write_bytes(b'hard link')
        alias = directory / 'alias.dll'
        held = guard.acquire([str(target)])
        try:
            try:
                os.link(target, alias)
                created = 'yes'
            except OSError as error:
                created = f'no (Windows error {error.winerror})'
            facts = [f'link created while held: {created}']
            removed = False
            if created == 'yes':
                facts.append(f'write through the link: {guard.write_probe(str(alias))}')
                # With a second name, try to remove the guarded name itself.
                try:
                    target.unlink()
                    removed = True
                    facts.append('guarded name removed while a second link exists: yes')
                except OSError as error:
                    facts.append(f'guarded name removed while a second link exists: no (Windows error {error.winerror})')
                try:
                    alias.unlink()
                    facts.append('second link removed while held: yes')
                except OSError as error:
                    facts.append(f'second link removed while held: no (Windows error {error.winerror})')
            check = held.check()
            expect(check is not removed, 'release check result does not match the guarded name state')
            facts.append(f'release check: {"pass" if check else "fail, exit 4"}')
        finally:
            held.close()
        for path in (alias, target):
            if path.exists():
                path.unlink()
        facts.append('a load through the link or a replacement at the guarded name has another path or file ID, '
                     'which the finalizer refuses (G4 checks 2 and 4)')
        return '; '.join(facts)


def deployment_copy(root, build):
    source = build / 'deploy'
    expect((source / 'sd_smoke.exe').is_file(), 'deploy/sd_smoke.exe is missing')
    copy = root / ('deploy-' + uuid.uuid4().hex[:8])
    shutil.copytree(source, copy)
    files = sorted([copy / 'sd_smoke.exe', *copy.rglob('*.dll')], key=lambda path: str(path).casefold())
    return copy, files


def ancestor_rename(root, build):
    copy, files = deployment_copy(root, build)
    try:
        held = guard.acquire([str(path) for path in files])
        try:
            outcomes = []
            for folder in (copy / 'platforms', copy):
                try:
                    folder.rename(folder.with_name(folder.name + '-moved'))
                except OSError as error:
                    outcomes.append(error.winerror)
                else:
                    raise AssertionError('a held folder was renamed')
            expect(all(code in (5, 32) for code in outcomes), f'unexpected rename errors {outcomes}')
            directories = len(held.record()['directories'])
        finally:
            held.close()
        return (f'{len(files)} deployment files, {directories} folders held; renaming the deployment folder '
                f'and platforms/ failed with Windows errors {outcomes}')
    finally:
        shutil.rmtree(copy)


def release_timing(root, build):
    copy, files = deployment_copy(root, build)
    output = root / ('guard-out-' + uuid.uuid4().hex[:8])
    output.mkdir()
    try:
        arguments = [sys.executable, '-B', str(HERE / 'file_guard.py')]
        for path in files:
            arguments += ['--file', str(path)]
        start = time.perf_counter()
        process = subprocess.Popen([*arguments, '--out', str(output)], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                   stderr=subprocess.PIPE, creationflags=subprocess.CREATE_NO_WINDOW)
        try:
            line = process.stdout.readline()
            ready = time.perf_counter()
            expect(line.strip() == b'ready', f'guard did not report ready: {line!r}')
            protected = guard.write_probe(str(files[0]))
            process.stdin.write(b'release\n')
            process.stdin.flush()
            released = process.stdout.readline()
            process.wait(timeout=30)
            end = time.perf_counter()
        finally:
            if process.poll() is None:
                process.kill()
            process.communicate(timeout=15)
        expect(released.strip() == b'released' and process.returncode == 0, 'release failed')
        expect(protected == 'sharing-violation' and guard.write_probe(str(files[0])) == 'opened', 'protection state')
        return (f'{len(files)} files: start to ready {1000 * (ready - start):.0f} ms, '
                f'release to exit {1000 * (end - ready):.0f} ms (includes the write probe)')
    finally:
        shutil.rmtree(output)
        shutil.rmtree(copy)


def loads_under_guard(root, build):
    # The usage path loads every static import and writes a record, with no window and no GPU work.
    copy, _ = deployment_copy(root, build)
    output = root / ('usage-out-' + uuid.uuid4().hex[:8])
    output.mkdir()
    try:
        files = sorted([*copy.rglob('*.dll'), *copy.rglob('*.exe')], key=lambda path: str(path).casefold())
        held = guard.acquire([str(path) for path in files])
        try:
            result = subprocess.run([str(copy / 'sd_smoke.exe'), '--l2-out', str(output), '--l2-scene', 'x'],
                                    capture_output=True, timeout=120, creationflags=subprocess.CREATE_NO_WINDOW)
            record = json.loads((output / 'harness.json').read_text(encoding='utf-8'))
            modules = record['modules']
            loaded = {event['path'].casefold() for event in modules['events'] if event['kind'] != 'unload'}
            guarded = {entry['path'].casefold() for entry in held.record()['files']}
            protected = all(guard.write_probe(str(path)) == 'sharing-violation' for path in files)
            intact = held.check()
        finally:
            held.close()
        expect(result.returncode == 1 and record['reason'] == 'usage', f'app exited {result.returncode}')
        expect(modules['observer'] == 'sealed' and modules['failure'] is None and modules['unload_history'] == []
               and modules['overflow'] is False, 'observer state')
        expect(loaded and loaded <= guarded, 'a loaded in-scope module was not guarded')
        expect(protected and intact, 'protection or file identity changed while the app ran')
        return (f'{len(files)} files held; the app loaded its imports from the held files and exited 1 (usage); '
                f'{len(loaded)} in-scope modules recorded, all guarded; every file still held, release check passed')
    finally:
        shutil.rmtree(output)
        shutil.rmtree(copy)


def compiler_environment():
    where = Path(os.environ.get('ProgramFiles(x86)', r'C:\Program Files (x86)')) / 'Microsoft Visual Studio' / 'Installer' / 'vswhere.exe'
    if not where.is_file():
        raise NotRun('vswhere.exe not found')
    found = subprocess.run([str(where), '-latest', '-products', '*', '-requires',
                            'Microsoft.VisualStudio.Component.VC.Tools.x86.x64', '-property', 'installationPath'],
                           capture_output=True, text=True, timeout=60)
    vcvars = Path(found.stdout.strip()) / 'VC' / 'Auxiliary' / 'Build' / 'vcvars64.bat'
    if found.returncode or not vcvars.is_file():
        raise NotRun('Visual Studio C++ x64 tools not found')
    return vcvars


def dll_redirection(root, build):
    probe = build / 'app' / 'sd_module_probe.dll'
    expect(probe.is_file(), 'app/sd_module_probe.dll is missing')
    vcvars = compiler_environment()
    with fresh(root) as directory:
        (directory / 'loader.cpp').write_text(LOADER, encoding='ascii', newline='\n')
        compiled = subprocess.run(f'call "{vcvars}" >nul && cl /nologo /O2 /EHsc loader.cpp /link /MANIFEST:NO',
                                  shell=True, cwd=directory, capture_output=True, text=True, timeout=600)
        expect(compiled.returncode == 0 and (directory / 'loader.exe').is_file(), 'loader build failed')
        (directory / 'probe').mkdir()
        requested = directory / 'probe' / probe.name
        shutil.copyfile(probe, requested)

        def load():
            try:
                result = subprocess.run([str(directory / 'loader.exe'), str(requested)], capture_output=True, timeout=60)
            except OSError as error:
                if error.winerror == 4551:
                    raise NotRun('Smart App Control blocked the fresh loader (Windows error 4551)')
                raise
            expect(result.returncode == 0, f'loader exited {result.returncode}')
            return result.stdout.decode('utf-8')

        control = load()
        expect(control.casefold() == str(requested).casefold(), 'control load reported another path')
        local = directory / 'loader.exe.local'
        local.mkdir()
        shutil.copyfile(probe, local / probe.name)
        actual = load()
        if actual.casefold() == str(requested).casefold():
            return 'no redirection: with a .local copy present, the absolute-path load still reported the requested path'
        expect(actual.casefold() == str(local / probe.name).casefold(), 'redirected to an unexpected path')
        return ('.local redirection applies to an absolute-path load without a manifest, and GetModuleFileNameW '
                'reports the redirected copy; the finalizer refuses that unguarded path (G4 check 4)')


def observer_test(build):
    program = build / 'app' / 'sd_module_observer_test.exe'
    expect(program.is_file(), 'app/sd_module_observer_test.exe is missing')
    result = subprocess.run([str(program)], capture_output=True, text=True, timeout=1800)
    lines = result.stdout.splitlines()
    expect(result.returncode == 0 and lines and lines[-1].startswith('sd_module_observer_test: 10/10 cases'),
           f'observer test exited {result.returncode}')
    cases = [next((line for line in lines if line.startswith(f'case {number}:')), '') for number in (6, 10)]
    return '; '.join([lines[-1].removeprefix('sd_module_observer_test: '), *cases])


# Holds the file the way the guard does, then never reports ready (or reports something else).
FAKE_GUARD = r'''import os, sys, time
import file_guard as guard  # the copy beside this script
out = sys.argv[sys.argv.index('--out') + 1]
held = guard.acquire([os.path.join(out, 'part.dll')])
with open(os.path.join(out, 'held.txt'), 'w', encoding='ascii') as stream:
    stream.write(f'{os.getpid()} {guard.process_created(os.getpid())}')
if os.environ.get('L2_FAKE_LINE'):
    deadline = time.monotonic() + 50
    while not os.path.exists(os.path.join(out, 'go.txt')) and time.monotonic() < deadline:
        time.sleep(0.05)
    print(os.environ['L2_FAKE_LINE'], flush=True)
time.sleep(600)
'''
# Start-Guard and what it uses, taken from the runner itself; single quotes only on this command line.
START_GUARD = r'''$ErrorActionPreference = 'Stop'
$text = [IO.File]::ReadAllText($env:L2_RUNNER)
$line = [regex]::Match($text, '(?m)^\$python = .*$').Value
$native = $text.Substring($text.IndexOf('function Native-Arguments'))
$native = $native.Substring(0, $native.IndexOf('function Number-Argument'))
$start = $text.Substring($text.IndexOf('function Start-Guard'))
$start = $start.Substring(0, $start.IndexOf('function Stop-Guard'))
. ([scriptblock]::Create($line + [Environment]::NewLine + $native + $start))
$guardScript = $env:L2_FAKE_SCRIPT
$buildDirectory = $env:L2_FAKE_OUT
$Candidate = 'sd'
try { $null = Start-Guard $env:L2_FAKE_OUT; 'returned' } catch { 'threw: ' + $_.Exception.Message }
'''


def guard_cleanup(root, runner=HERE / 'run_scene.ps1'):
    # L2-A-001: a guard that holds files but never becomes ready must be gone when Start-Guard throws.
    facts = []
    for case, line, message in (('no ready line', '', 'was not ready within 60 s'),
                                ('another line', 'refused', 'refused to protect the build')):
        with fresh(root) as directory:
            part = directory / 'part.dll'
            part.write_bytes(b'held by a guard that is never ready')
            # The runner starts '<python> -B <script> --launch ... --out <dir>'; the script imports file_guard beside it.
            script = directory / 'fake_guard.py'
            script.write_text(FAKE_GUARD, encoding='ascii', newline='\n')
            shutil.copyfile(HERE / 'file_guard.py', directory / 'file_guard.py')
            environment = {**os.environ, 'L2_RUNNER': str(runner), 'L2_FAKE_SCRIPT': str(script),
                           'L2_FAKE_OUT': str(directory), 'L2_FAKE_LINE': line}
            environment.pop('GITHUB_PERSONAL_ACCESS_TOKEN', None)
            # A file, not a pipe: a surviving guard inherits the shell's output handle and would hold a pipe open.
            output = open(directory / 'shell.txt', 'wb')
            shell = subprocess.Popen(['powershell', '-NoProfile', '-NonInteractive', '-Command', START_GUARD],
                                     env=environment, stdin=subprocess.DEVNULL, stdout=output, stderr=subprocess.STDOUT,
                                     creationflags=subprocess.CREATE_NO_WINDOW)
            pid = created = None
            try:
                deadline = time.monotonic() + 30
                while not (directory / 'held.txt').is_file() and time.monotonic() < deadline:
                    time.sleep(0.1)
                time.sleep(0.2)
                expect((directory / 'held.txt').is_file(), f'{case}: the fake guard never held the file')
                pid, created = (int(value) for value in (directory / 'held.txt').read_text(encoding='ascii').split())
                held = guard.write_probe(str(part))
                (directory / 'go.txt').write_bytes(b'')  # Only now may the fake guard print its line.
                shell.wait(timeout=180)
                deadline = time.monotonic() + 10
                while guard.process_created(pid) == created and time.monotonic() < deadline:
                    time.sleep(0.1)
                alive = guard.process_created(pid) == created
                released = guard.write_probe(str(part))
            finally:
                if shell.poll() is None:
                    shell.kill()
                    shell.wait(timeout=15)
                if pid is not None and guard.process_created(pid) == created:
                    os.kill(pid, 9)  # TerminateProcess; only the fake guard this experiment started.
                    deadline = time.monotonic() + 10
                    while guard.process_created(pid) == created and time.monotonic() < deadline:
                        time.sleep(0.1)
                output.close()
            text = (directory / 'shell.txt').read_bytes().decode('utf-8', 'replace')
            result = text.strip().splitlines()[-1:] or ['']
            expect(held == 'sharing-violation', f'{case}: the fake guard did not hold the file ({held})')
            expect(result[0].startswith('threw: ') and message in result[0], f'{case}: Start-Guard did not throw as expected')
            expect(not alive, f'{case}: the guard process outlived Start-Guard')
            expect(released == 'opened', f'{case}: the file was still held after Start-Guard threw ({released})')
            facts.append(f'{case}: Start-Guard threw, the guard process was gone and the file writable')
    return '; '.join(facts)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--build', required=True, type=Path)
    build = parser.parse_args().build.resolve()
    if os.name != 'nt':
        print('L2-V-002 experiments: not run: Windows is required')
        return 0
    root = build / ('experiments-' + uuid.uuid4().hex[:8])
    root.mkdir()
    try:
        report('subst drive alias', lambda: subst_alias(root))
        report('symbolic link alias', lambda: symlink_alias(root))
        report('UNC administrative share alias', lambda: unc_alias(root))
        report('hard link while held', lambda: hard_link(root))
        report('ancestor rename on a deployment copy', lambda: ancestor_rename(root, build))
        report('guard start and release timing', lambda: release_timing(root, build))
        report('Qt usage path under the guard', lambda: loads_under_guard(root, build))
        report('DLL redirection through a .local folder', lambda: dll_redirection(root, build))
        report('Qt module observer test', lambda: observer_test(build))
        report('guard that never becomes ready', lambda: guard_cleanup(root))
    finally:
        expect(root.parent == build, 'experiment cleanup boundary')
        shutil.rmtree(root)
    print(f"L2-V-002 experiments: {COUNTS['pass']} passed, {COUNTS['not run']} not run, {COUNTS['fail']} failed")
    return 1 if COUNTS['fail'] else 0


if __name__ == '__main__':
    sys.exit(main())
