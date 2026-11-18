"""PresentMon 2.6.0.0 capture wrapper for the C9 runs. Standard library only.

    python -B tools/perf/c9/presentmon_capture.py (--process-name NAME | --process-id PID) --output CSV
        --seconds N [--dry-run]

`--dry-run` finds PresentMon, reads the installed version without running PresentMon, builds and checks the command
line and prints it; it exits 1 when PresentMon is missing, its version is not the pinned 2.6.0.0 or the command fails
its check. The version comes from the executable's version resource when it has one (PresentMon-2.6.0-x64.exe has
none), else from the installer's uninstall entry in the registry, the record that `winget list` shows for this
machine-scope install. Without `--dry-run` it captures: Windows and an administrator
shell only (PresentMon needs both; renderer/acceptance/README.md). The capture mode has not been exercised yet (DRAFT).

PresentMon is found as the toolchain entry `presentmon` describes it (tools/toolchain.lock.json: winget
Intel.PresentMon 2.6.0.0, machine scope, probe `PresentMon --version`, "with no command on PATH, winget's record
confirms the installed version"): first the pinned location that renderer/acceptance/run_scene.ps1 uses,
`%ProgramFiles%/Intel/PresentMon/PresentMonConsoleApplication/PresentMon-2.6.0-x64.exe`, then `PresentMon` on PATH
and in winget's Links folder.

The command matches run_scene.ps1's capture (`--v1_metrics --qpc_time`, the CSV that renderer_gate.py's
CAPTURE_COLUMNS and key_to_frame.py read), with a process filter and a timed capture that terminates itself:
`--process_name NAME | --process_id PID`, `--output_file CSV`, `--session_name magic600-c9-capture`, `--timed N`,
`--terminate_after_timed`. This wrapper parses no CSV; the analysis tools do.
"""
from __future__ import annotations

import argparse
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

PINNED_VERSION = '2.6.0.0'
PINNED_RELATIVE = ('Intel', 'PresentMon', 'PresentMonConsoleApplication', 'PresentMon-2.6.0-x64.exe')
SESSION = 'magic600-c9-capture'
SESSION_PATTERN = re.compile(r'magic600-[a-z0-9-]+-capture')
SECONDS_MAX = 3600
FILTERS = ('--process_name', '--process_id')
REQUIRED_FLAGS = ('--v1_metrics', '--qpc_time', '--terminate_after_timed')
VALUED = ('--process_name', '--process_id', '--output_file', '--session_name', '--timed')
UNINSTALL_KEY = r'SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall'
DISPLAY_NAME = re.compile(r'(Intel\(R\) )?PresentMon', re.IGNORECASE)


def locate(environ=None, which=shutil.which):
    """The PresentMon executable and where it was found, or (None, 'missing')."""
    environ = os.environ if environ is None else environ
    program_files = environ.get('ProgramFiles')
    if program_files:
        pinned = Path(program_files).joinpath(*PINNED_RELATIVE)
        if pinned.is_file():
            return pinned, 'pinned location'
    found = which('PresentMon')
    if found:
        return Path(found), 'PATH'
    local = environ.get('LOCALAPPDATA')
    if local:
        link = Path(local) / 'Microsoft' / 'WinGet' / 'Links' / 'PresentMon.exe'
        if link.is_file():
            return link, 'winget links'
    return None, 'missing'


def file_version(path):
    """The file version from a Windows executable's version resource, without running it; None if unreadable."""
    if os.name != 'nt':
        return None
    import ctypes
    from ctypes import wintypes
    try:
        library = ctypes.WinDLL('version')
    except OSError:
        return None
    library.GetFileVersionInfoSizeW.argtypes = (wintypes.LPCWSTR, ctypes.POINTER(wintypes.DWORD))
    library.GetFileVersionInfoSizeW.restype = wintypes.DWORD
    library.GetFileVersionInfoW.argtypes = (wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, ctypes.c_void_p)
    library.GetFileVersionInfoW.restype = wintypes.BOOL
    library.VerQueryValueW.argtypes = (ctypes.c_void_p, wintypes.LPCWSTR, ctypes.POINTER(ctypes.c_void_p),
                                       ctypes.POINTER(wintypes.UINT))
    library.VerQueryValueW.restype = wintypes.BOOL
    size = library.GetFileVersionInfoSizeW(str(path), None)
    if not size:
        return None
    data = ctypes.create_string_buffer(size)
    if not library.GetFileVersionInfoW(str(path), 0, size, data):
        return None
    pointer, length = ctypes.c_void_p(), wintypes.UINT()
    if not library.VerQueryValueW(data, '\\', ctypes.byref(pointer), ctypes.byref(length)) or length.value < 52:
        return None
    info = ctypes.cast(pointer, ctypes.POINTER(ctypes.c_uint32 * 13)).contents
    if info[0] != 0xFEEF04BD:  # VS_FIXEDFILEINFO signature
        return None
    high, low = info[2], info[3]
    return f'{high >> 16}.{high & 0xFFFF}.{low >> 16}.{low & 0xFFFF}'


def registry_version():
    """DisplayVersion of the one installed PresentMon uninstall entry (machine or user scope), read-only; else None."""
    if os.name != 'nt':
        return None
    import winreg
    places = [(winreg.HKEY_LOCAL_MACHINE, view) for view in (winreg.KEY_WOW64_64KEY, winreg.KEY_WOW64_32KEY)]
    places.append((winreg.HKEY_CURRENT_USER, 0))
    versions = set()
    for hive, view in places:
        try:
            uninstall = winreg.OpenKey(hive, UNINSTALL_KEY, 0, winreg.KEY_READ | view)
        except OSError:
            continue
        with uninstall:
            index = 0
            while True:
                try:
                    name = winreg.EnumKey(uninstall, index)
                except OSError:
                    break
                index += 1
                try:
                    with winreg.OpenKey(uninstall, name) as entry:
                        display = str(winreg.QueryValueEx(entry, 'DisplayName')[0])
                        version = str(winreg.QueryValueEx(entry, 'DisplayVersion')[0])
                except OSError:
                    continue
                if DISPLAY_NAME.fullmatch(display.strip()):
                    versions.add(version.strip())
    return versions.pop() if len(versions) == 1 else None


def read_version(path):
    """(version, where it came from), without running the executable."""
    version = file_version(path)
    if version is not None:
        return version, 'version resource'
    version = registry_version()
    return (version, 'uninstall entry') if version is not None else (None, 'unreadable')


def same_version(installed, pinned=PINNED_VERSION):
    """Versions compare equal when they differ only by trailing zero parts (bootstrap.py's winget rule)."""
    def parts(version):
        values = [int(part) for part in version.split('.')]
        while values and values[-1] == 0:
            values.pop()
        return values
    try:
        return installed is not None and parts(installed) == parts(pinned)
    except ValueError:
        return False


def installed(environ=None, which=shutil.which, version_reader=read_version):
    """{'path', 'found', 'version', 'version_from', 'version_ok'} for the PresentMon this machine has."""
    path, found = locate(environ, which)
    version, source = version_reader(path) if path else (None, 'not checked')
    return {'path': path, 'found': found, 'version': version, 'version_from': source,
            'version_ok': same_version(version)}


def build_command(executable, output, seconds, *, process_name=None, process_id=None, session=SESSION):
    """The capture command line; ValueError when an argument is not acceptable."""
    if (process_name is None) == (process_id is None):
        raise ValueError('give exactly one process filter: a process name or a process id')
    if process_name is not None and not re.fullmatch(r'[A-Za-z0-9._ -]+\.exe', process_name):
        raise ValueError('the process name must be an executable file name such as Godot.exe')
    if process_id is not None and not (type(process_id) is int and process_id > 0):
        raise ValueError('the process id must be a positive integer')
    if not (type(seconds) is int and 0 < seconds <= SECONDS_MAX):
        raise ValueError(f'the capture length must be a whole number of seconds from 1 to {SECONDS_MAX}')
    if Path(output).suffix.lower() != '.csv':
        raise ValueError('the output file must end in .csv')
    if not SESSION_PATTERN.fullmatch(session):
        raise ValueError('the session name must look like magic600-<name>-capture')
    target = ['--process_name', process_name] if process_name is not None else ['--process_id', str(process_id)]
    return [str(executable), '--v1_metrics', '--qpc_time', *target, '--output_file', str(output),
            '--session_name', session, '--timed', str(seconds), '--terminate_after_timed']


def check_command(command):
    """The problems of a capture command line, an empty list when it is acceptable."""
    problems = []
    if not command or not re.fullmatch(r'presentmon[^\\/]*\.exe|presentmon', Path(command[0]).name, re.IGNORECASE):
        problems.append('the executable is not PresentMon')
    arguments = command[1:]
    values = {}
    index = 0
    while index < len(arguments):
        flag = arguments[index]
        if flag in VALUED:
            if index + 1 >= len(arguments) or arguments[index + 1].startswith('--'):
                problems.append(f'{flag} has no value')
                break
            if flag in values:
                problems.append(f'{flag} appears twice')
            values[flag] = arguments[index + 1]
            index += 2
        elif flag in REQUIRED_FLAGS:
            if flag in arguments[:index]:
                problems.append(f'{flag} appears twice')
            index += 1
        else:
            problems.append(f'unexpected argument {flag}')
            index += 1
    if sum(flag in values for flag in FILTERS) != 1:
        problems.append('exactly one process filter is needed')
    if '--process_id' in values and not re.fullmatch(r'[1-9]\d*', values['--process_id']):
        problems.append('the process id is not a positive integer')
    if not values.get('--output_file', '').lower().endswith('.csv'):
        problems.append('the output file must be a .csv file')
    timed = values.get('--timed', '')
    if not (re.fullmatch(r'[1-9]\d*', timed) and int(timed) <= SECONDS_MAX):
        problems.append('a timed capture of 1 to 3600 s is needed')
    if not SESSION_PATTERN.fullmatch(values.get('--session_name', '')):
        problems.append('a magic600-*-capture session name is needed')
    problems.extend(f'{flag} is missing' for flag in REQUIRED_FLAGS if flag not in arguments)
    return problems


def administrator():
    if os.name != 'nt':
        return False
    import ctypes
    try:
        return bool(ctypes.windll.shell32.IsUserAnAdmin())
    except (AttributeError, OSError):
        return False


def complete_csv(path):
    """True when the CSV exists, is not empty and ends with a complete row."""
    try:
        with Path(path).open('rb') as stream:
            stream.seek(0, os.SEEK_END)
            if stream.tell() == 0:
                return False
            stream.seek(-1, os.SEEK_END)
            return stream.read(1) == b'\n'
    except OSError:
        return False


def capture(command, seconds, output, run=subprocess.run):
    """Run one timed capture (administrator only). Returns 0 when PresentMon exits 0 and the CSV is complete."""
    try:
        completed = run(command, stdin=subprocess.DEVNULL, timeout=seconds + 60, check=False)
    except subprocess.TimeoutExpired:
        session = command[command.index('--session_name') + 1]
        run([command[0], '--session_name', session, '--terminate_existing_session'], stdin=subprocess.DEVNULL,
            timeout=60, check=False)
        print('PresentMon did not finish within 60 s of its timed capture; its session was terminated', file=sys.stderr)
        return 1
    if completed.returncode != 0:
        print(f'PresentMon exited {completed.returncode}', file=sys.stderr)
        return 1
    if not complete_csv(output):
        print('the PresentMon CSV is missing or does not end with a complete row', file=sys.stderr)
        return 1
    return 0


def main(argv=None, environ=None, which=shutil.which, version_reader=read_version, run=subprocess.run,
         is_admin=administrator):
    parser = argparse.ArgumentParser(description='PresentMon 2.6.0.0 timed capture for the C9 runs.')
    target = parser.add_mutually_exclusive_group(required=True)
    target.add_argument('--process-name', help='executable name to capture, such as Godot.exe')
    target.add_argument('--process-id', type=int, help='process id to capture')
    parser.add_argument('--output', type=Path, required=True, help='new CSV file to write')
    parser.add_argument('--seconds', type=int, required=True, help='capture length in whole seconds')
    parser.add_argument('--dry-run', action='store_true', help='check and print the command; capture nothing')
    args = parser.parse_args(argv)
    found = installed(environ, which, version_reader)
    executable = found['path'] or Path('PresentMon-2.6.0-x64.exe')
    try:
        command = build_command(executable, args.output, args.seconds, process_name=args.process_name,
                                process_id=args.process_id)
    except ValueError as error:
        parser.error(str(error))
    problems = check_command(command)
    if args.output.exists():
        problems.append('the output file exists; a capture never appends to or replaces a file')
    admin = is_admin()
    print(f"PresentMon: {found['found']}; version {found['version'] or 'unknown'} ({found['version_from']}; "
          f"pinned {PINNED_VERSION})")
    print(f"administrator: {'yes' if admin else 'no'} (a capture needs it)")
    print(f'command: {subprocess.list2cmdline(command)}')
    print(f"command check: {'ok' if not problems else '; '.join(problems)}")
    ready = found['path'] is not None and found['version_ok'] and not problems
    if not found['path']:
        print('PresentMon 2.6.0.0 is missing; the owner installs it (tools/toolchain.lock.json, entry presentmon)',
              file=sys.stderr)
    elif not found['version_ok']:
        print(f'PresentMon is not the pinned version {PINNED_VERSION}', file=sys.stderr)
    if args.dry_run or not ready:
        return 0 if ready else 1
    if os.name != 'nt' or not admin:
        print('a capture needs Windows and an administrator shell', file=sys.stderr)
        return 1
    return capture(command, args.seconds, args.output, run)


if __name__ == '__main__':
    sys.exit(main())
