"""Hold level 2 identity files and their ancestors until the runner releases them."""
from __future__ import annotations

import sys

sys.dont_write_bytecode = True

import ctypes
from ctypes import wintypes as wt
import hashlib
import json
import ntpath
import os

SHADERS = {'count_vs.dxil', 'draw_ps.dxil', 'draw_vs.dxil', 'geometry_cs.dxil'}
GENERIC_READ, GENERIC_WRITE = 0x80000000, 0x40000000
FILE_LIST_DIRECTORY = 1
FILE_SHARE_READ, FILE_SHARE_WRITE = 1, 2
OPEN_EXISTING = 3
FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT = 0x02000000, 0x00200000
FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT = 0x10, 0x400
FILE_CS_FLAG_CASE_SENSITIVE_DIR = 1
INVALID_HANDLE_VALUE = ctypes.c_void_p(-1).value


class GuardRefused(Exception):
    def __init__(self, step, path):
        self.step, self.path = step, path
        super().__init__(f'{step}: {path}')


class _AttributeTagInfo(ctypes.Structure):
    _fields_ = [('FileAttributes', wt.DWORD), ('ReparseTag', wt.DWORD)]


class _FileIdInfo(ctypes.Structure):
    _fields_ = [('VolumeSerialNumber', ctypes.c_ulonglong), ('FileId', ctypes.c_ubyte * 16)]


class _CaseSensitiveInfo(ctypes.Structure):
    _fields_ = [('Flags', wt.DWORD)]


def _bind(name, arguments, result):
    function = getattr(_kernel, name)
    function.argtypes, function.restype = arguments, result
    return function


if os.name == 'nt':
    _kernel = ctypes.WinDLL('kernel32', use_last_error=True)
    _create_file = _bind('CreateFileW',
                         [wt.LPCWSTR, wt.DWORD, wt.DWORD, ctypes.c_void_p, wt.DWORD, wt.DWORD, wt.HANDLE], wt.HANDLE)
    _close_handle = _bind('CloseHandle', [wt.HANDLE], wt.BOOL)
    _drive_type = _bind('GetDriveTypeW', [wt.LPCWSTR], wt.UINT)
    _file_information = _bind('GetFileInformationByHandleEx',
                              [wt.HANDLE, ctypes.c_int, ctypes.c_void_p, wt.DWORD], wt.BOOL)
    _final_path_name = _bind('GetFinalPathNameByHandleW', [wt.HANDLE, wt.LPWSTR, wt.DWORD, wt.DWORD], wt.DWORD)
    _file_size = _bind('GetFileSizeEx', [wt.HANDLE, ctypes.POINTER(ctypes.c_longlong)], wt.BOOL)
    _set_pointer = _bind('SetFilePointerEx',
                         [wt.HANDLE, ctypes.c_longlong, ctypes.POINTER(ctypes.c_longlong), wt.DWORD], wt.BOOL)
    _read_file = _bind('ReadFile',
                       [wt.HANDLE, ctypes.c_void_p, wt.DWORD, ctypes.POINTER(wt.DWORD), ctypes.c_void_p], wt.BOOL)
    _open_process = _bind('OpenProcess', [wt.DWORD, wt.BOOL, wt.DWORD], wt.HANDLE)
    _process_times = _bind('GetProcessTimes', [wt.HANDLE, *([ctypes.POINTER(wt.FILETIME)] * 4)], wt.BOOL)
    _wait = _bind('WaitForSingleObject', [wt.HANDLE, wt.DWORD], wt.DWORD)
    _precise_time = _bind('GetSystemTimePreciseAsFileTime', [ctypes.POINTER(wt.FILETIME)], None)


class _Handle:
    def __init__(self, value, path):
        self.value, self.path = value, path

    def close(self):
        if self.value is not None:
            _close_handle(self.value)
            self.value = None

    def __enter__(self):
        return self

    def __exit__(self, *exception):
        self.close()


def _path_form(path):
    # Only change separators. In particular, never resolve or normalize away
    # dot components, short names, junctions or a substituted drive.
    if not isinstance(path, str):
        raise GuardRefused('path-form', str(path))
    path = path.replace('/', '\\')
    if (os.name != 'nt' or len(path) < 3 or path[0] not in 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz'
            or path[1:3] != ':\\'):
        raise GuardRefused('path-form', path)
    components = path[3:].split('\\') if path[3:] else []
    if any(not component or component in ('.', '..')
           or any(ord(char) < 32 or char in '<>:"|?*' for char in component) for component in components):
        raise GuardRefused('path-form', path)
    if _drive_type(path[:3]) != 3:  # DRIVE_FIXED
        raise GuardRefused('path-form', path)
    return path


def _unique(paths):
    unique = {}
    for path in paths:
        path = _path_form(path)
        # Two spellings of one case-insensitive path would alias one entry.
        if unique.setdefault(path.casefold(), path) != path:
            raise GuardRefused('path-form', path)
    return sorted(unique.values(), key=str.casefold)


def protected_set(launch: dict, candidate: str) -> list[str]:
    if (not isinstance(launch, dict) or launch.get('format') != 'magic600-l2-launch-v1'
            or candidate not in ('sa2', 'sd') or launch.get('candidate') != candidate):
        raise GuardRefused('launch', 'launch.json')
    try:
        executable, dll = _path_form(launch['executable']), _path_form(launch['dll'])
        paths = [executable, dll, *(ntpath.join(ntpath.dirname(dll), name) for name in sorted(SHADERS))]
        if candidate == 'sa2':
            directory = _path_form(launch['working_directory'])
            paths.extend(ntpath.join(directory, name) for name in ('project.godot', 'Main.tscn', 'Smoke.cs'))
            debug = ntpath.join(directory, '.godot', 'mono', 'temp', 'bin', 'Debug')
            with os.scandir(debug) as entries:
                assemblies = [entry.path for entry in entries if entry.name.lower().endswith('.dll')
                              and not entry.is_dir(follow_symlinks=False)]
            if not assemblies:
                raise GuardRefused('assemblies', debug)
            paths.extend(assemblies)
        else:
            # Do not follow junctions during enumeration (including junction
            # cycles). Any such deployment directory is unsupported.
            pending = [ntpath.dirname(executable)]
            while pending:
                directory = pending.pop()
                if os.stat(directory, follow_symlinks=False).st_file_attributes & FILE_ATTRIBUTE_REPARSE_POINT:
                    raise GuardRefused('directory-reparse', directory)
                with os.scandir(directory) as entries:
                    for entry in entries:
                        if entry.is_dir(follow_symlinks=False):
                            pending.append(entry.path)
                        elif entry.name.lower().endswith(('.dll', '.exe')):
                            paths.append(entry.path)
    except (KeyError, TypeError) as error:
        raise GuardRefused('launch', 'launch.json') from error
    except OSError as error:
        raise GuardRefused('protected-set', str(error.filename or directory)) from error
    return _unique(paths)


def _open(path, access, share, flags, step):
    value = _create_file(path, access, share, None, OPEN_EXISTING, flags, None)
    if value == INVALID_HANDLE_VALUE:
        raise GuardRefused(step, path)
    return _Handle(value, path)


def _information(handle, information_class, structure):
    if handle.value is None:
        raise OSError('handle is closed')
    result = structure()
    if not _file_information(handle.value, information_class, ctypes.byref(result), ctypes.sizeof(result)):
        raise ctypes.WinError(ctypes.get_last_error())
    return result


def _attributes(handle):
    return _information(handle, 9, _AttributeTagInfo).FileAttributes


def _case_sensitive(handle):
    return _information(handle, 23, _CaseSensitiveInfo).Flags


def _inspect(handle, directory=False):
    kind = 'directory' if directory else 'file'
    try:
        attributes = _attributes(handle)
        if attributes & FILE_ATTRIBUTE_REPARSE_POINT:
            raise GuardRefused(kind + '-reparse', handle.path)
        if bool(attributes & FILE_ATTRIBUTE_DIRECTORY) != directory:
            raise GuardRefused(kind + '-type', handle.path)
        if directory and _case_sensitive(handle) & FILE_CS_FLAG_CASE_SENSITIVE_DIR:
            raise GuardRefused('directory-case-sensitive', handle.path)
    except OSError as error:
        raise GuardRefused(kind + '-information', handle.path) from error


def open_identity(path: str):
    path = _path_form(path)
    handle = _open(path, GENERIC_READ, FILE_SHARE_READ, FILE_FLAG_OPEN_REPARSE_POINT, 'file-open')
    try:
        _inspect(handle)
        return handle
    except BaseException:
        handle.close()
        raise


def file_id(handle) -> tuple[int, str]:
    information = _information(handle, 18, _FileIdInfo)
    return information.VolumeSerialNumber, bytes(information.FileId).hex()


def _final_path(handle):
    length = _final_path_name(handle.value, None, 0, 0)  # FILE_NAME_NORMALIZED | VOLUME_NAME_DOS
    if not length:
        raise ctypes.WinError(ctypes.get_last_error())
    buffer = ctypes.create_unicode_buffer(length + 1)
    written = _final_path_name(handle.value, buffer, len(buffer), 0)
    if not written or written >= len(buffer):
        raise ctypes.WinError(ctypes.get_last_error())
    path = buffer.value
    return path[4:] if path.startswith('\\\\?\\') else path


def sha256_handle(handle) -> tuple[int, str]:
    if handle.value is None:
        raise OSError('handle is closed')
    size = ctypes.c_longlong()
    if not _file_size(handle.value, ctypes.byref(size)) or not _set_pointer(handle.value, 0, None, 0):
        raise ctypes.WinError(ctypes.get_last_error())
    digest, total = hashlib.sha256(), 0
    buffer, count = ctypes.create_string_buffer(1024 * 1024), wt.DWORD()
    while True:
        if not _read_file(handle.value, buffer, len(buffer), ctypes.byref(count), None):
            raise ctypes.WinError(ctypes.get_last_error())
        if not count.value:
            break
        digest.update(buffer.raw[:count.value])
        total += count.value
    if total != size.value:
        raise OSError('file size changed while held')
    return total, digest.hexdigest()


def _filetime(value):
    return (value.dwHighDateTime << 32) | value.dwLowDateTime


def process_created(pid: int) -> int | None:
    if os.name != 'nt' or type(pid) is not int or not 0 < pid < 2**32:
        return None
    handle = _open_process(0x1000 | 0x100000, False, pid)  # QUERY_LIMITED_INFORMATION | SYNCHRONIZE
    if not handle:
        return None
    try:
        created, exited, kernel, user = (wt.FILETIME() for _ in range(4))
        if (_wait(handle, 0) != 258  # WAIT_TIMEOUT means still running, even with an exit code of 259.
                or not _process_times(handle, ctypes.byref(created), ctypes.byref(exited),
                                      ctypes.byref(kernel), ctypes.byref(user))
                or _wait(handle, 0) != 258):
            return None
        return _filetime(created)
    finally:
        _close_handle(handle)


def write_probe(path: str) -> str:
    handle = _create_file(path, GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE | 4, None,
                          OPEN_EXISTING, FILE_FLAG_OPEN_REPARSE_POINT, None)
    if handle == INVALID_HANDLE_VALUE:
        error = ctypes.get_last_error()
        return 'sharing-violation' if error == 32 else f'error:{error}'
    _close_handle(handle)
    return 'opened'


class Guard:
    def __init__(self):
        self._held = []
        self._files, self._directories = [], []
        self._pid, self._created = os.getpid(), process_created(os.getpid())
        if self._created is None:
            raise GuardRefused('process-created', str(self._pid))
        self._closed = False

    def record(self) -> dict:
        return {'format': 'magic600-l2-guard-v1', 'pid': self._pid, 'created': self._created,
                'files': [dict(entry) for entry in sorted(self._files, key=lambda item: item['path'].casefold())],
                'directories': [dict(entry) for entry in sorted(self._directories, key=lambda item: item['path'].casefold())]}

    def check(self) -> bool:
        if self._closed:
            return False
        try:
            return all(file_id(handle) == identity and not _attributes(handle) & FILE_ATTRIBUTE_REPARSE_POINT
                       for handle, identity in self._held)
        except OSError:
            return False

    def close(self):
        self._closed = True
        for handle, _ in reversed(self._held):
            handle.close()


def acquire(paths: list[str]) -> Guard:
    paths = _unique(paths)
    if not paths:
        raise GuardRefused('protected-set', '(empty)')
    guard, directories = Guard(), {}
    try:
        for path in paths:
            ancestor = path[:3]
            for component in [None, *path[3:].split('\\')[:-1]]:
                if component is not None:
                    ancestor = ntpath.join(ancestor, component)
                key = ancestor.casefold()
                if key in directories:
                    if directories[key] != ancestor:
                        raise GuardRefused('path-form', ancestor)
                    continue
                handle = _open(ancestor, FILE_LIST_DIRECTORY, FILE_SHARE_READ | FILE_SHARE_WRITE,
                               FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT, 'directory-open')
                # Keep even an uninspected handle in the cleanup list.
                guard._held.append((handle, None))
                _inspect(handle, directory=True)
                try:
                    identity = file_id(handle)
                except OSError as error:
                    raise GuardRefused('identity', ancestor) from error
                guard._held[-1] = (handle, identity)
                guard._directories.append({'path': ancestor, 'volume_serial': identity[0], 'file_id': identity[1]})
                directories[key] = ancestor
            handle = open_identity(path)
            guard._held.append((handle, None))
            try:
                final_path = _final_path(handle)
            except OSError as error:
                raise GuardRefused('spelling', path) from error
            if final_path.casefold() != path.casefold():
                raise GuardRefused('spelling', path)
            try:
                identity = file_id(handle)
            except OSError as error:
                raise GuardRefused('identity', path) from error
            guard._held[-1] = (handle, identity)
            try:
                size, digest = sha256_handle(handle)
            except OSError as error:
                raise GuardRefused('digest', path) from error
            guard._files.append({'path': path, 'final_path': final_path, 'volume_serial': identity[0],
                                 'file_id': identity[1], 'size': size, 'sha256': digest})
        return guard
    except BaseException:
        guard.close()
        raise


def _options(arguments):
    options, files = {}, []
    index = 0
    while index < len(arguments):
        option = arguments[index]
        if (option not in ('--launch', '--candidate', '--file', '--out') or index + 1 == len(arguments)
                or arguments[index + 1].startswith('--') or option in options):
            raise ValueError('unknown, repeated or incomplete option')
        value = arguments[index + 1]
        if option == '--file':
            files.append(value)
        else:
            options[option] = value
        index += 2
    if '--out' not in options:
        raise ValueError('missing --out')
    if files:
        if set(options) != {'--out'}:
            raise ValueError('mixed command forms')
    elif set(options) != {'--launch', '--candidate', '--out'} or options['--candidate'] not in ('sa2', 'sd'):
        raise ValueError('missing or invalid launch options')
    return options, files


def _write_record(directory, record):
    output = ntpath.join(directory, 'guard.json')
    temporary = ntpath.join(directory, f'guard.{os.getpid()}.tmp')
    created = False
    try:
        if os.path.lexists(output):
            raise GuardRefused('record', output)
        with open(temporary, 'x', encoding='utf-8', newline='\n') as stream:
            created = True
            stream.write(json.dumps(record, ensure_ascii=False, allow_nan=False) + '\n')
        # Windows rename is atomic and fails if the destination already exists.
        os.rename(temporary, output)
        created = False
    except OSError as error:
        raise GuardRefused('record', output) from error
    finally:
        if created:
            os.unlink(temporary)


def main(arguments=None):
    try:
        options, paths = _options(sys.argv[1:] if arguments is None else arguments)
    except ValueError:
        print('usage: file_guard.py (--launch <json> --candidate sa2|sd | --file <path> ...) --out <directory>',
              file=sys.stderr)
        return 2
    held = None
    try:
        if not paths:
            launch_path = options['--launch']
            try:
                with open(launch_path, encoding='utf-8') as stream:
                    launch = json.load(stream)
            except (OSError, ValueError) as error:
                raise GuardRefused('launch', launch_path) from error
            paths = protected_set(launch, options['--candidate'])
        held = acquire(paths)
        record = held.record()
        ready = wt.FILETIME()
        _precise_time(ctypes.byref(ready))
        record['ready'] = _filetime(ready)
        _write_record(options['--out'], record)
        print('ready', flush=True)
        # Read bytes: a Windows PowerShell writer under a UTF-8 console code page
        # sends a byte-order mark first, which an ANSI code page may not decode.
        stream = getattr(sys.stdin, 'buffer', None)
        for line in sys.stdin if stream is None else stream:
            if isinstance(line, bytes):
                line = line.removeprefix(b'\xef\xbb\xbf').decode('ascii', 'replace')
            if line.removesuffix('\n').removesuffix('\r') != 'release':
                return 5
            valid = held.check()
            held.close()
            if not valid:
                return 4
            print('released', flush=True)
            return 0
        return 5
    except GuardRefused as error:
        print(f'refused: {error.step}: {error.path}', file=sys.stderr)
        return 3
    finally:
        if held is not None:
            held.close()


if __name__ == '__main__':
    sys.exit(main())
