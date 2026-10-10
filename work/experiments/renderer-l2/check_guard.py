"""Windows fixture checks for the level 2 file guard; no renderer or capture."""
import sys

sys.dont_write_bytecode = True

import contextlib
import ctypes
from ctypes import wintypes as wt
import errno
import hashlib
import io
import json
import os
from pathlib import Path
import queue
import re
import shutil
import subprocess
import tempfile
import threading
import uuid
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
INVALID_HANDLE = ctypes.c_void_p(-1).value
READ, WRITE, DELETE = 0x80000000, 0x40000000, 0x10000
DIRECTORY, REPARSE = 0x02000000, 0x00200000
COUNTS = {'pass': 0, 'not run': 0, 'fail': 0}


def expect(condition, message):
    if not condition:
        raise AssertionError(message)


class NotRun(Exception):
    pass


def case(name, action):
    try:
        action()
    except NotRun as error:
        COUNTS['not run'] += 1
        print(f'{name}: not run: {error}', flush=True)
    except Exception as error:
        COUNTS['fail'] += 1
        print(f'{name}: fail: {type(error).__name__}: {error}', flush=True)
    else:
        COUNTS['pass'] += 1
        print(f'{name}: pass', flush=True)


def bind(library, name, arguments, result):
    function = getattr(library, name)
    function.argtypes, function.restype = arguments, result
    return function


def native_api():
    global create_file, close_handle, move_file, short_path, get_security, set_security, convert_sd, local_free
    global create_mapping, map_view, unmap_view
    kernel = ctypes.WinDLL('kernel32', use_last_error=True)
    security = ctypes.WinDLL('advapi32', use_last_error=True)
    create_file = bind(kernel, 'CreateFileW',
                       [wt.LPCWSTR, wt.DWORD, wt.DWORD, ctypes.c_void_p, wt.DWORD, wt.DWORD, wt.HANDLE], wt.HANDLE)
    close_handle = bind(kernel, 'CloseHandle', [wt.HANDLE], wt.BOOL)
    move_file = bind(kernel, 'MoveFileExW', [wt.LPCWSTR, wt.LPCWSTR, wt.DWORD], wt.BOOL)
    create_mapping = bind(kernel, 'CreateFileMappingW',
                          [wt.HANDLE, ctypes.c_void_p, wt.DWORD, wt.DWORD, wt.DWORD, wt.LPCWSTR], wt.HANDLE)
    map_view = bind(kernel, 'MapViewOfFile',
                    [wt.HANDLE, wt.DWORD, wt.DWORD, wt.DWORD, ctypes.c_size_t], ctypes.c_void_p)
    unmap_view = bind(kernel, 'UnmapViewOfFile', [ctypes.c_void_p], wt.BOOL)
    short_path = bind(kernel, 'GetShortPathNameW', [wt.LPCWSTR, wt.LPWSTR, wt.DWORD], wt.DWORD)
    get_security = bind(security, 'GetFileSecurityW',
                        [wt.LPCWSTR, wt.DWORD, ctypes.c_void_p, wt.DWORD, ctypes.POINTER(wt.DWORD)], wt.BOOL)
    set_security = bind(security, 'SetFileSecurityW', [wt.LPCWSTR, wt.DWORD, ctypes.c_void_p], wt.BOOL)
    convert_sd = bind(security, 'ConvertStringSecurityDescriptorToSecurityDescriptorW',
                      [wt.LPCWSTR, wt.DWORD, ctypes.POINTER(ctypes.c_void_p), ctypes.POINTER(wt.DWORD)], wt.BOOL)
    local_free = bind(kernel, 'LocalFree', [ctypes.c_void_p], ctypes.c_void_p)


@contextlib.contextmanager
def raw_handle(path, access, flags=0, share=7):
    handle = create_file(str(path), access, share, None, 3, flags, None)
    if handle == INVALID_HANDLE:
        raise ctypes.WinError(ctypes.get_last_error())
    try:
        yield handle
    finally:
        close_handle(handle)


@contextlib.contextmanager
def fixture(root):
    directory = root / uuid.uuid4().hex
    directory.mkdir()
    try:
        yield directory
    finally:
        expect(directory.resolve().parent == root.resolve(), 'fixture cleanup boundary')
        shutil.rmtree(directory)


def part(directory, name='part.dll', data=b'guarded bytes\x00\xff\n'):
    path = directory / name
    path.write_bytes(data)
    return path


def refused(paths, step=None):
    try:
        held = guard.acquire([str(path) for path in paths])
    except guard.GuardRefused as error:
        expect(isinstance(error.step, str) and error.step and isinstance(error.path, str), 'refusal fields')
        if step:
            expect(error.step == step, f'expected {step}, got {error.step}')
        return
    held.close()
    raise AssertionError('guard accepted an unsupported or mutable path')


def writers(root, mapped=False):
    with fixture(root) as directory:
        earlier = part(directory, 'a.dll')
        path = part(directory, 'z.dll')
        if mapped:
            with raw_handle(path, READ | WRITE) as file_handle:
                mapping = create_mapping(file_handle, None, 4, 0, 0, None)  # PAGE_READWRITE
                if not mapping:
                    raise ctypes.WinError(ctypes.get_last_error())
            # Close the original file handle before checking the mapping. A
            # Python mmap may keep its own duplicate write handle alive.
            try:
                view = map_view(mapping, 2, 0, 0, 0)  # FILE_MAP_WRITE
                if not view:
                    raise ctypes.WinError(ctypes.get_last_error())
                try:
                    refused([earlier, path], 'file-open')
                finally:
                    unmap_view(view)
            finally:
                close_handle(mapping)
        else:
            with raw_handle(path, WRITE):
                refused([earlier, path], 'file-open')
        expect(guard.write_probe(str(earlier)) == 'opened', 'partial acquisition leaked a file handle')
        renamed = directory.with_name(directory.name + '-renamed')
        directory.rename(renamed)
        renamed.rename(directory)
        with contextlib.closing(guard.acquire([str(path)])) as held:
            expect(held.check(), 'acquisition after writer closed')


def mutation(path, operation):
    if operation == 'write':
        with path.open('r+b') as stream:
            stream.write(b'changed')
    elif operation == 'truncate':
        with path.open('wb'):
            pass
    elif operation == 'delete':
        path.unlink()
    elif operation == 'rename':
        path.rename(path.with_suffix('.renamed'))
    else:
        replacement = path.with_suffix('.replacement')
        if not move_file(str(replacement), str(path), 1):
            raise ctypes.WinError(ctypes.get_last_error())


def mutation_cycle(root, operation):
    with fixture(root) as directory:
        path = part(directory)
        original = path.read_bytes()
        replacement = part(directory, 'part.replacement', b'replacement')
        expect(guard.write_probe(str(path)) == 'opened', 'fixture was not writable before acquisition')
        held = guard.acquire([str(path)])
        try:
            try:
                mutation(path, operation)
            except OSError as error:
                # CRT-backed Python opens translate the sharing failure to
                # EACCES; MoveFileEx(REPLACE_EXISTING) reports ACCESS_DENIED.
                expect(error.winerror in (5, 32) or (error.winerror is None and error.errno == errno.EACCES),
                       f'unexpected {operation} failure: {error}')
            else:
                raise AssertionError(f'{operation} succeeded while guarded')
            expect(path.read_bytes() == original, 'a refused mutation changed bytes')
        finally:
            held.close()
        mutation(path, operation)
        if operation == 'write':
            expect(path.read_bytes().startswith(b'changed'), 'write after release')
        elif operation == 'truncate':
            expect(path.stat().st_size == 0, 'truncate after release')
        elif operation in ('delete', 'rename'):
            expect(not path.exists(), 'delete/rename after release')
        else:
            expect(path.read_bytes() == b'replacement' and not replacement.exists(), 'replace after release')


def ancestor_cycle(root):
    with fixture(root) as directory:
        deep = directory / 'one' / 'two'
        deep.mkdir(parents=True)
        path = part(deep)
        held = guard.acquire([str(path)])
        ancestors = [deep, deep.parent, directory]
        try:
            recorded = {entry['path'].casefold() for entry in held.record()['directories']}
            expect(recorded == {str(p).casefold() for p in path.parents}, 'ancestors through volume root not held')
            for ancestor in ancestors:
                try:
                    ancestor.rename(ancestor.with_name(ancestor.name + '-moved'))
                except OSError as error:
                    expect(error.winerror == 32, 'ancestor rename did not fail for sharing')
                else:
                    raise AssertionError('ancestor rename succeeded')
            # A DELETE-access open is the rename prerequisite, without risking a
            # rename of any system ancestor if the guard has a planted defect.
            for ancestor in path.parents:
                try:
                    with raw_handle(ancestor, DELETE, DIRECTORY):
                        raise AssertionError('ancestor allowed DELETE access')
                except OSError as error:
                    expect(error.winerror in (5, 32), 'unexpected ancestor DELETE probe')
            sibling = part(deep, 'unprotected.tmp')
            sibling.rename(deep / 'unprotected-renamed.tmp')
            (deep / 'unprotected-renamed.tmp').unlink()
        finally:
            held.close()
        for ancestor in ancestors:
            moved = ancestor.with_name(ancestor.name + '-moved')
            ancestor.rename(moved)
            moved.rename(ancestor)


@contextlib.contextmanager
def deny_path_read(path):
    needed = wt.DWORD()
    get_security(str(path), 4, None, 0, ctypes.byref(needed))
    expect(needed.value > 0, 'cannot capture fixture DACL')
    original = ctypes.create_string_buffer(needed.value)
    if not get_security(str(path), 4, original, needed, ctypes.byref(needed)):
        raise ctypes.WinError(ctypes.get_last_error())
    # SECURITY_DESCRIPTOR_RELATIVE.Control is the USHORT at offset 2.
    control = int.from_bytes(original.raw[2:4], 'little')
    descriptor = ctypes.c_void_p()
    if not convert_sd('D:P(D;;0x1;;;WD)(A;;FA;;;WD)', 1, ctypes.byref(descriptor), None):
        raise ctypes.WinError(ctypes.get_last_error())
    changed = False
    try:
        if not set_security(str(path), 4 | 0x80000000, descriptor):
            raise ctypes.WinError(ctypes.get_last_error())
        changed = True
        yield
    finally:
        local_free(descriptor)
        if changed:
            protection = 0x80000000 if control & 0x1000 else 0x20000000
            if not set_security(str(path), 4 | protection, original):
                raise ctypes.WinError(ctypes.get_last_error())


def acl_part(directory, name, data):
    path = directory / name
    descriptor = ctypes.c_void_p()
    if not convert_sd('D:P(A;;FA;;;WD)', 1, ctypes.byref(descriptor), None):
        raise ctypes.WinError(ctypes.get_last_error())

    class SecurityAttributes(ctypes.Structure):
        _fields_ = [('nLength', wt.DWORD), ('lpSecurityDescriptor', ctypes.c_void_p), ('bInheritHandle', wt.BOOL)]

    # Inherited sandbox fixture permissions allow data writes but not
    # WRITE_DAC. Grant it on this newly created synthetic file only.
    try:
        attributes = SecurityAttributes(ctypes.sizeof(SecurityAttributes), descriptor, False)
        value = create_file(str(path), WRITE, 7, ctypes.byref(attributes), 1, 0, None)  # CREATE_NEW
        if value == INVALID_HANDLE:
            raise ctypes.WinError(ctypes.get_last_error())
        close_handle(value)
    finally:
        local_free(descriptor)
    path.write_bytes(data)
    return path


def same_handle_hash(root):
    with fixture(root) as directory:
        data = b'held-handle hashing\x00\xff' * 100000
        path = acl_part(directory, 'acl-identity.dll', data)
        expected = (len(data), hashlib.sha256(data).hexdigest())
        with guard.open_identity(str(path)) as handle:
            with deny_path_read(path):
                def planted_reopen(open_handle):
                    with open(open_handle.path, 'rb') as stream:
                        return len(data), hashlib.file_digest(stream, 'sha256').hexdigest()

                try:
                    planted_reopen(handle)
                except PermissionError as error:
                    expect(error.winerror == 5 or error.errno == errno.EACCES,
                           'planted reopen failed for an unexpected reason')
                else:
                    raise AssertionError('planted reopen was not caught by the DACL test')
                expect(guard.sha256_handle(handle) == expected, 'hash depends on reopening the denied path')
                expect(guard.sha256_handle(handle) == expected, 'hash did not restart at offset zero')
        empty = part(directory, 'empty.dll', b'')
        with guard.open_identity(str(empty)) as handle:
            expect(guard.sha256_handle(handle) == (0, hashlib.sha256(b'').hexdigest()), 'empty file hash')


def acquire_hash_after_denial(root):
    with fixture(root) as directory:
        data = b'acquire hashing\x00\xff' * 100000
        path = acl_part(directory, 'acl-acquire.dll', data)
        expected = (len(data), hashlib.sha256(data).hexdigest())
        real_final_path = guard._final_path
        with contextlib.ExitStack() as stack:
            # Deny read by path after acquire opened the file and before it hashes.
            def deny_after_open(handle):
                final_path = real_final_path(handle)
                stack.enter_context(deny_path_read(path))
                try:
                    open(path, 'rb').close()
                except PermissionError as error:
                    expect(error.winerror == 5 or error.errno == errno.EACCES,
                           'denied reopen failed for an unexpected reason')
                else:
                    raise AssertionError('reopen by path was not denied')
                return final_path

            with patch.object(guard, '_final_path', side_effect=deny_after_open):
                held = guard.acquire([str(path)])
            try:
                entries = held.record()['files']
                expect(len(entries) == 1 and (entries[0]['size'], entries[0]['sha256']) == expected,
                       'acquire hash depends on reopening the denied path')
            finally:
                held.close()


def alias_spellings(root):
    with fixture(root) as directory:
        path = part(directory)
        held = guard.acquire([str(path), str(path)])
        try:
            expect(len(held.record()['files']) == 1, 'an exact duplicate was not merged')
        finally:
            held.close()
        upper = path.parent / path.name.upper()
        expect(str(upper) != str(path), 'fixture name has no letters')
        refused([path, upper], 'path-form')
        other = part(directory, 'other.dll')
        folded = directory.parent / directory.name.upper() / other.name
        expect(str(folded.parent) != str(directory), 'fixture directory has no letters')
        refused([path, folded], 'path-form')
        # Python case folding is not the NTFS upcase table: these are two files.
        sharp, double = part(directory, 'stra\u00dfe.dll'), directory / 'strasse.dll'
        try:
            with open(double, 'xb') as stream:
                stream.write(b'second')
        except FileExistsError:
            raise NotRun('the volume folds the sharp s')
        expect(sharp.stat().st_ino != double.stat().st_ino, 'casefold fixture is one file')
        refused([sharp, double], 'path-form')


def junction(root):
    import _winapi

    with fixture(root) as directory:
        target = directory / 'target'
        target.mkdir()
        path = part(target)
        link = directory / 'junction'
        _winapi.CreateJunction(str(target), str(link))
        try:
            refused([link / path.name], 'directory-reparse')
        finally:
            link.rmdir()


def symlink(root):
    with fixture(root) as directory:
        path = part(directory)
        link = directory / 'leaf-link.dll'
        try:
            os.symlink(path, link)
        except OSError as error:
            raise NotRun(f'os.symlink unavailable (Windows error {error.winerror})') from error
        try:
            refused([link], 'file-reparse')
            expect(guard.write_probe(str(path)) == 'opened', 'symlink refusal leaked a target handle')
        finally:
            link.unlink()


@contextlib.contextmanager
def case_sensitive(directory):
    command = ['fsutil', 'file', 'setCaseSensitiveInfo', str(directory)]
    try:
        result = subprocess.run([*command, 'enable'], capture_output=True, text=True,
                                timeout=15, creationflags=subprocess.CREATE_NO_WINDOW)
    except OSError as error:
        raise NotRun(f'fsutil unavailable (Windows error {error.winerror})') from error
    if result.returncode:
        reason = ' '.join((result.stdout + result.stderr).split())
        raise NotRun(f'fsutil enable exited {result.returncode}: {reason}')
    try:
        yield
    finally:
        result = subprocess.run([*command, 'disable'], capture_output=True, text=True,
                                timeout=15, creationflags=subprocess.CREATE_NO_WINDOW)
        expect(result.returncode == 0, 'cannot restore fixture case sensitivity')


def case_sensitive_refusal(root, same_fold=False):
    with fixture(root) as directory:
        with case_sensitive(directory):
            if same_fold:
                import _winapi

                target = directory / 'CaseTarget'
                target.mkdir()
                path = part(target)
                link = directory / 'casetarget'
                _winapi.CreateJunction(str(target), str(link))
                try:
                    expect(str(link).casefold() == str(target).casefold(), 'same-fold fixture')
                    refused([link / path.name], 'directory-case-sensitive')
                finally:
                    link.rmdir()
                    path.unlink()
                    target.rmdir()
            else:
                path = part(directory)
                refused([path], 'directory-case-sensitive')
                path.unlink()


def case_sensitive_stub(root):
    with fixture(root) as directory:
        path = part(directory)
        with patch.object(guard, '_case_sensitive', return_value=1):
            refused([path], 'directory-case-sensitive')
        expect(guard.write_probe(str(path)) == 'opened', 'case-sensitive refusal leaked a handle')


def short_alias(root):
    with fixture(root) as directory:
        long = directory / 'directory with a long name'
        long.mkdir()
        path = part(long, 'identity with a long name.dll')
        buffer = ctypes.create_unicode_buffer(32768)
        length = short_path(str(path), buffer, len(buffer))
        if not length:
            raise NotRun(f'GetShortPathNameW failed (Windows error {ctypes.get_last_error()})')
        expect(length < len(buffer), 'short path buffer')
        if buffer.value.casefold() == str(path).casefold():
            raise NotRun('the fixture has no 8.3 short name')
        refused([buffer.value], 'spelling')


def path_form(root, form):
    with fixture(root) as directory:
        path = part(directory)
        paths = {'extended': '\\\\?\\' + str(path),
                 'parent': str(directory) + '\\..\\' + directory.name + '\\part.dll',
                 'dot': str(directory) + '\\.\\part.dll',
                 'relative': 'part.dll', 'drive-relative': path.drive + 'part.dll',
                 'UNC': '\\\\localhost\\C$\\part.dll'}
        refused([paths[form]], 'path-form')


def launch_fixture(directory, candidate):
    executable = part(directory, 'app.exe')
    dll = part(directory, 'sa2_interop.dll')
    for shader in ('count_vs.dxil', 'draw_ps.dxil', 'draw_vs.dxil', 'geometry_cs.dxil'):
        part(directory, shader)
    launch = {'format': 'magic600-l2-launch-v1', 'candidate': candidate,
              'executable': str(executable), 'dll': str(dll), 'working_directory': str(directory)}
    if candidate == 'sa2':
        for name in ('project.godot', 'Main.tscn', 'Smoke.cs'):
            part(directory, name)
        debug = directory / '.godot' / 'mono' / 'temp' / 'bin' / 'Debug'
        debug.mkdir(parents=True)
        part(debug, 'SA2Smoke.dll')
        part(debug, 'Dependency.DLL')
        part(debug, 'ignored.txt')
    else:
        plugins = directory / 'plugins' / 'platforms'
        plugins.mkdir(parents=True)
        part(plugins, 'qwindows.dll')
        part(plugins, 'helper.EXE')
        part(directory, 'Qt6Core.DLL')
        part(directory, 'ignored.txt')
    return launch


def protected_files(root, candidate):
    with fixture(root) as directory:
        launch = launch_fixture(directory, candidate)
        common = [directory / name for name in ('app.exe', 'sa2_interop.dll', 'count_vs.dxil',
                                                'draw_ps.dxil', 'draw_vs.dxil', 'geometry_cs.dxil')]
        extra = ([directory / name for name in ('project.godot', 'Main.tscn', 'Smoke.cs')]
                 + [directory / '.godot/mono/temp/bin/Debug' / name for name in ('SA2Smoke.dll', 'Dependency.DLL')]
                 if candidate == 'sa2' else
                 [directory / 'Qt6Core.DLL', directory / 'plugins/platforms/qwindows.dll',
                  directory / 'plugins/platforms/helper.EXE'])
        expected = sorted(map(str, common + extra), key=str.casefold)
        expect(guard.protected_set(launch, candidate) == expected, 'protected set differs from named fixture files')
        with contextlib.closing(guard.acquire(expected)) as held:
            expect([entry['path'] for entry in held.record()['files']] == expected, 'protected set acquisition')
        for name in ('count_vs.dxil', *(('project.godot', 'Main.tscn', 'Smoke.cs') if candidate == 'sa2' else ())):
            missing = directory / name
            saved = missing.read_bytes()
            missing.unlink()
            try:
                refused(guard.protected_set(launch, candidate))
            finally:
                missing.write_bytes(saved)
        if candidate == 'sa2':
            debug = directory / '.godot/mono/temp/bin/Debug'
            for item in debug.glob('*.dll'):
                item.unlink()
            try:
                guard.protected_set(launch, candidate)
            except guard.GuardRefused:
                pass
            else:
                raise AssertionError('empty Godot assembly set accepted')
        for change in ({'format': 'wrong'}, {'candidate': 'sd' if candidate == 'sa2' else 'sa2'}):
            try:
                guard.protected_set({**launch, **change}, candidate)
            except guard.GuardRefused:
                pass
            else:
                raise AssertionError('invalid launch accepted')


@contextlib.contextmanager
def guard_process(arguments):
    process = subprocess.Popen([sys.executable, '-B', str(HERE / 'file_guard.py'), *map(str, arguments)],
                               stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                               text=True, creationflags=subprocess.CREATE_NO_WINDOW)
    try:
        yield process
    finally:
        if process.poll() is None:
            process.kill()
        process.communicate(timeout=15)


def await_ready(process):
    lines = queue.Queue()
    reader = threading.Thread(target=lambda: lines.put(process.stdout.readline()), daemon=True)
    reader.start()
    try:
        line = lines.get(timeout=30)
    except queue.Empty:
        process.kill()
        reader.join(timeout=15)
        raise AssertionError('guard did not report ready within 30 seconds')
    reader.join(timeout=15)
    expect(line == 'ready\n', f'expected ready, got {line!r}')


def validate_record(record, paths, pid):
    expect(set(record) == {'format', 'pid', 'created', 'ready', 'files', 'directories'}, 'guard.json keys')
    expect(record['format'] == 'magic600-l2-guard-v1' and record['pid'] == pid, 'format or PID')
    for name in ('pid', 'created', 'ready'):
        expect(type(record[name]) is int and record[name] > 0, f'{name} type/value')
    expect(record['created'] == guard.process_created(pid) and record['ready'] >= record['created'], 'FILETIME fields')
    expect(type(record['files']) is list and type(record['directories']) is list, 'entry lists')
    expect([entry['path'] for entry in record['files']] == sorted(map(str, paths), key=str.casefold), 'file sort/unique')
    for entries in (record['files'], record['directories']):
        spellings = [entry['path'] for entry in entries]
        expect(spellings == sorted(spellings, key=str.casefold), 'entry order')
        expect(len({name.casefold() for name in spellings}) == len(spellings), 'duplicate entries')
        for entry in entries:
            expect(type(entry['path']) is str and Path(entry['path']).is_absolute() and '/' not in entry['path'], 'path form')
            expect(type(entry['volume_serial']) is int and 0 <= entry['volume_serial'] < 2**64, 'volume serial')
            expect(type(entry['file_id']) is str and re.fullmatch('[0-9a-f]{32}', entry['file_id']), '128-bit file ID')
            expect(entry['file_id'] == Path(entry['path']).stat().st_ino.to_bytes(16, 'little').hex(), 'FileIdInfo byte order')
    expect({entry['path'].casefold() for entry in record['directories']}
           == {str(parent).casefold() for path in paths for parent in path.parents}, 'directory set')
    for entry in record['files']:
        expect(set(entry) == {'path', 'final_path', 'volume_serial', 'file_id', 'size', 'sha256'}, 'file fields')
        expect(type(entry['final_path']) is str and entry['final_path'].casefold() == entry['path'].casefold(), 'final spelling')
        data = Path(entry['path']).read_bytes()
        expect(type(entry['size']) is int and entry['size'] == len(data), 'file size')
        expect(type(entry['sha256']) is str and re.fullmatch('[0-9a-f]{64}', entry['sha256'])
               and entry['sha256'] == hashlib.sha256(data).hexdigest(), 'independent digest')
        with guard.open_identity(entry['path']) as handle:
            expect(guard.file_id(handle) == (entry['volume_serial'], entry['file_id']), 'public file_id')
        expect(guard.write_probe(entry['path']) == 'sharing-violation', 'write_probe while held')
    for entry in record['directories']:
        expect(set(entry) == {'path', 'volume_serial', 'file_id'}, 'directory fields')


def cli_cycle(root, launch=False):
    with fixture(root) as directory:
        inputs = directory / 'inputs'
        inputs.mkdir()
        output = directory / 'out'
        output.mkdir()
        if launch:
            data = launch_fixture(inputs, 'sd')
            launch_path = part(directory, 'launch.json', json.dumps(data).encode())
            arguments = ['--launch', launch_path, '--candidate', 'sd']
            paths = list(map(Path, guard.protected_set(data, 'sd')))
        else:
            a, b = part(inputs, 'a.dll'), part(inputs, 'B.dll', b'')
            arguments = ['--file', b, '--file', a, '--file', a]
            paths = [a, b]
        with guard_process([*arguments, '--out', output]) as process:
            await_ready(process)
            record = json.loads((output / 'guard.json').read_text(encoding='utf-8'))
            validate_record(record, paths, process.pid)
            expect(list(output.iterdir()) == [output / 'guard.json'], 'temporary record left behind')
            stdout, stderr = process.communicate('release\n', timeout=15)
            expect(process.returncode == 0 and stdout == 'released\n' and stderr == '', 'release/exit cycle')
            expect(guard.process_created(process.pid) is None, 'exited process still reported alive')
        for path in paths:
            expect(guard.write_probe(str(path)) == 'opened', 'release left file protected')


def cli_bom_release(root):
    with fixture(root) as directory:
        path = part(directory)
        output = directory / 'out'
        output.mkdir()
        process = subprocess.Popen([sys.executable, '-B', str(HERE / 'file_guard.py'), '--file', str(path), '--out', str(output)],
                                   stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                   creationflags=subprocess.CREATE_NO_WINDOW)
        try:
            # The bytes wait in the pipe until the guard reads after ready.
            stdout, stderr = process.communicate(b'\xef\xbb\xbfrelease\r\n', timeout=60)
        finally:
            if process.poll() is None:
                process.kill()
                process.communicate(timeout=15)
        expect(process.returncode == 0 and stdout.splitlines() == [b'ready', b'released'] and stderr == b'',
               'release after a byte-order mark')
        expect(guard.write_probe(str(path)) == 'opened', 'release left the file protected')


def cli_abort(root, mode):
    with fixture(root) as directory:
        path = part(directory)
        output = directory / 'out'
        output.mkdir()
        with guard_process(['--file', path, '--out', output]) as process:
            await_ready(process)
            if mode == 'killed':
                process.kill()
                stdout, stderr = process.communicate(timeout=15)
                expect(process.returncode != 0, 'killed process exit')
            else:
                stdout, stderr = process.communicate('release \n' if mode == 'unknown' else '', timeout=15)
                expect(process.returncode == 5, 'EOF/unknown input exit code')
            expect(stdout == '' and stderr == '', 'unexpected abort output')
        expect(guard.write_probe(str(path)) == 'opened', 'abort leaked file protection')
        moved = directory.with_name(directory.name + '-moved')
        directory.rename(moved)
        moved.rename(directory)


def cli_errors(root):
    with fixture(root) as directory:
        path = part(directory)
        invalid = [[], ['--unknown', 'x'], ['--file'], ['--file', path],
                   ['--file', path, '--out', directory, '--out', directory],
                   ['--launch', path, '--candidate', 'sd', '--candidate', 'sd', '--out', directory],
                   ['--launch', path, '--out', directory],
                   ['--launch', path, '--candidate', 'bad', '--out', directory],
                   ['--file', path, '--candidate', 'sd', '--out', directory],
                   ['--file', path, '--launch', path, '--candidate', 'sd', '--out', directory],
                   ['--file', '--out', directory], ['--file', path, '--out']]
        for arguments in invalid:
            with guard_process(arguments) as process:
                stdout, _ = process.communicate('', timeout=15)
                expect(process.returncode == 2 and stdout == '', f'usage exit: {arguments}')
        expect(not (directory / 'guard.json').exists(), 'usage error wrote record')
        for bad in (str(directory / 'missing.dll'), '\\\\?\\' + str(path)):
            with guard_process(['--file', bad, '--out', directory]) as process:
                stdout, stderr = process.communicate('', timeout=15)
                expect(process.returncode == 3 and stdout == '' and len(stderr.splitlines()) == 1
                       and stderr.startswith('refused: '), 'acquisition refusal exit/output')
        for launch in ({'format': 'wrong', 'candidate': 'sd'}, {'format': 'magic600-l2-launch-v1', 'candidate': 'sa2'}):
            launch_path = part(directory, 'launch.json', json.dumps(launch).encode())
            with guard_process(['--launch', launch_path, '--candidate', 'sd', '--out', directory]) as process:
                stdout, stderr = process.communicate('', timeout=15)
                expect(process.returncode == 3 and stdout == '' and len(stderr.splitlines()) == 1, 'bad launch refusal')
        existing = part(directory, 'guard.json', b'existing record')
        with guard_process(['--file', path, '--out', directory]) as process:
            stdout, stderr = process.communicate('', timeout=15)
            expect(process.returncode == 3 and stdout == '' and stderr.startswith('refused: '), 'existing record refusal')
        expect(existing.read_bytes() == b'existing record' and guard.write_probe(str(path)) == 'opened', 'refusal overwrote or leaked')
        expect(guard.write_probe(str(directory / 'missing.dll')) == 'error:2', 'write_probe missing file')


def release_recheck(root):
    with fixture(root) as directory:
        path = part(directory)
        held = guard.acquire([str(path)])
        try:
            expect('ready' not in held.record() and held.check(), 'Guard.record/check')
            with patch.object(guard, 'file_id', return_value=(0, '0' * 32)):
                expect(not held.check(), 'release accepted changed file ID')
            with patch.object(guard, '_attributes', return_value=0x400):
                expect(not held.check(), 'release accepted reparse attribute')
        finally:
            held.close()
            held.close()
        expect(not held.check(), 'closed guard still checks as held')
        output = directory / 'out'
        output.mkdir()
        stdout, stderr = io.StringIO(), io.StringIO()
        with patch.object(guard.Guard, 'check', return_value=False), patch.object(sys, 'stdin', io.StringIO('release\n')):
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                result = guard.main(['--file', str(path), '--out', str(output)])
        expect(result == 4 and stdout.getvalue() == 'ready\n' and stderr.getvalue() == '', 'failed recheck exit code/output')
        expect(guard.write_probe(str(path)) == 'opened', 'failed recheck leaked handles')


def main():
    if os.name != 'nt':
        print('Windows guard: not run: Windows is required')
        print('Guard acceptance: 0 passed, 1 not run, 0 failed')
        return 0
    if not (HERE / 'file_guard.py').is_file():
        print('guard implementation: fail: file_guard.py does not exist')
        return 1
    global guard
    import file_guard as guard
    import finalize_run as finalizer

    native_api()
    root = Path(tempfile.gettempdir()) / ('magic600-l2-guard-' + uuid.uuid4().hex)
    root.mkdir()
    try:
        case('shared shader names', lambda: expect(guard.SHADERS == finalizer.SHADERS, 'shader set mismatch'))
        case('existing write handle and partial cleanup', lambda: writers(root))
        case('existing writable mapping', lambda: writers(root, mapped=True))
        for operation in ('write', 'truncate', 'delete', 'rename', 'replace'):
            case(operation + ' blocked until release', lambda operation=operation: mutation_cycle(root, operation))
        case('every ancestor held; unprotected siblings remain mutable', lambda: ancestor_cycle(root))
        case('held-handle hash after DACL denial; planted reopen caught', lambda: same_handle_hash(root))
        case('acquire hashes through its handle after DACL denial', lambda: acquire_hash_after_denial(root))
        case('aliasing spellings refused; exact duplicate merged', lambda: alias_spellings(root))
        case('junction component', lambda: junction(root))
        case('symbolic link leaf', lambda: symlink(root))
        case('same-case-fold junction', lambda: case_sensitive_refusal(root, same_fold=True))
        case('case-sensitive directory', lambda: case_sensitive_refusal(root))
        case('case-sensitive info refusal (stub)', lambda: case_sensitive_stub(root))
        case('8.3 component', lambda: short_alias(root))
        for form in ('extended', 'parent', 'dot', 'relative', 'drive-relative', 'UNC'):
            case(form + ' path refused', lambda form=form: path_form(root, form))
        for candidate in ('sa2', 'sd'):
            case(candidate + ' protected set and required missing files', lambda candidate=candidate: protected_files(root, candidate))
        case('CLI files ready/record/release', lambda: cli_cycle(root))
        case('CLI launch ready/record/release', lambda: cli_cycle(root, launch=True))
        case('CLI release after a UTF-8 byte-order mark', lambda: cli_bom_release(root))
        for mode in ('unknown', 'EOF', 'killed'):
            case('CLI ' + mode + ' closes handles', lambda mode=mode: cli_abort(root, mode))
        case('CLI usage, refusal and existing-output errors', lambda: cli_errors(root))
        case('release identity/reparse recheck and exit 4', lambda: release_recheck(root))
    finally:
        expect(root.resolve().parent == Path(tempfile.gettempdir()).resolve()
               and root.name.startswith('magic600-l2-guard-'), 'root cleanup boundary')
        shutil.rmtree(root)
    print(f"Guard acceptance: {COUNTS['pass']} passed, {COUNTS['not run']} not run, {COUNTS['fail']} failed")
    return 1 if COUNTS['fail'] else 0


if __name__ == '__main__':
    sys.exit(main())
