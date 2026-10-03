"""ctypes bindings for the Windows migration probes P1 to P3 (Windows only).

Every call raises OSError with the Win32 error code on failure. Nothing here
creates, writes or deletes a file unless the caller asks for that access.
"""
from __future__ import annotations

import ctypes
import ctypes.wintypes as w
import os

if os.name != 'nt':
    raise ImportError('tools/migration_probes/winapi.py is Windows only')

kernel32 = ctypes.WinDLL('kernel32', use_last_error=True)
advapi32 = ctypes.WinDLL('advapi32', use_last_error=True)

GENERIC_READ = 0x80000000
GENERIC_WRITE = 0x40000000
DELETE = 0x00010000
FILE_LIST_DIRECTORY = 0x0001
FILE_ADD_FILE = 0x0002
FILE_ADD_SUBDIRECTORY = 0x0004
FILE_READ_ATTRIBUTES = 0x0080
FILE_WRITE_ATTRIBUTES = 0x0100
FILE_SHARE_READ = 0x1
FILE_SHARE_WRITE = 0x2
FILE_SHARE_DELETE = 0x4
FILE_SHARE_ALL = FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE
CREATE_NEW = 1
CREATE_ALWAYS = 2
OPEN_EXISTING = 3
FILE_ATTRIBUTE_NORMAL = 0x80
FILE_ATTRIBUTE_DIRECTORY = 0x10
FILE_ATTRIBUTE_REPARSE_POINT = 0x400
FILE_FLAG_BACKUP_SEMANTICS = 0x02000000
FILE_FLAG_OPEN_REPARSE_POINT = 0x00200000
FILE_FLAG_OVERLAPPED = 0x40000000
LOCKFILE_FAIL_IMMEDIATELY = 0x1
LOCKFILE_EXCLUSIVE_LOCK = 0x2
MOVEFILE_REPLACE_EXISTING = 0x1
MOVEFILE_WRITE_THROUGH = 0x8

ERROR_FILE_NOT_FOUND = 2
ERROR_ACCESS_DENIED = 5
ERROR_HANDLE_EOF = 38
ERROR_LOCK_VIOLATION = 33
ERROR_OPERATION_ABORTED = 995
ERROR_IO_PENDING = 997
ERROR_NOTIFY_ENUM_DIR = 1022

FILE_NOTIFY_CHANGE_FILE_NAME = 0x001
FILE_NOTIFY_CHANGE_DIR_NAME = 0x002
FILE_NOTIFY_CHANGE_ATTRIBUTES = 0x004
FILE_NOTIFY_CHANGE_SIZE = 0x008
FILE_NOTIFY_CHANGE_LAST_WRITE = 0x010
FILE_NOTIFY_CHANGE_LAST_ACCESS = 0x020
FILE_NOTIFY_CHANGE_CREATION = 0x040
FILE_NOTIFY_CHANGE_SECURITY = 0x100
# Everything that can change a directory entry, its bytes or its metadata; last access is excluded.
WATCH_FILTER = (FILE_NOTIFY_CHANGE_FILE_NAME | FILE_NOTIFY_CHANGE_DIR_NAME | FILE_NOTIFY_CHANGE_ATTRIBUTES
                | FILE_NOTIFY_CHANGE_SIZE | FILE_NOTIFY_CHANGE_LAST_WRITE | FILE_NOTIFY_CHANGE_CREATION
                | FILE_NOTIFY_CHANGE_SECURITY)
NOTIFY_ACTIONS = {1: 'added', 2: 'removed', 3: 'modified', 4: 'renamed-old', 5: 'renamed-new'}

WAIT_OBJECT_0 = 0
PROCESS_TERMINATE = 0x0001
PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
SYNCHRONIZE = 0x00100000
STILL_ACTIVE = 259
WAIT_TIMEOUT = 0x102
INFINITE = 0xFFFFFFFF

FILE_BASIC_INFO_CLASS = 0
FILE_STANDARD_INFO_CLASS = 1
FILE_ID_INFO_CLASS = 18

INVALID_HANDLE_VALUE = w.HANDLE(-1).value


class OVERLAPPED(ctypes.Structure):
    _fields_ = [('Internal', ctypes.c_size_t), ('InternalHigh', ctypes.c_size_t),
                ('Offset', w.DWORD), ('OffsetHigh', w.DWORD), ('hEvent', w.HANDLE)]


class FILE_BASIC_INFO(ctypes.Structure):
    _fields_ = [('CreationTime', ctypes.c_longlong), ('LastAccessTime', ctypes.c_longlong),
                ('LastWriteTime', ctypes.c_longlong), ('ChangeTime', ctypes.c_longlong),
                ('FileAttributes', w.DWORD)]


class FILE_STANDARD_INFO(ctypes.Structure):
    _fields_ = [('AllocationSize', ctypes.c_longlong), ('EndOfFile', ctypes.c_longlong),
                ('NumberOfLinks', w.DWORD), ('DeletePending', w.BOOLEAN), ('Directory', w.BOOLEAN)]


class FILE_ID_INFO(ctypes.Structure):
    _fields_ = [('VolumeSerialNumber', ctypes.c_ulonglong), ('FileId', ctypes.c_ubyte * 16)]


class SID_AND_ATTRIBUTES(ctypes.Structure):
    _fields_ = [('Sid', w.LPVOID), ('Attributes', w.DWORD)]


def _proto(function, restype, *argtypes):
    function.restype = restype
    function.argtypes = list(argtypes)
    return function


_CreateFileW = _proto(kernel32.CreateFileW, w.HANDLE, w.LPCWSTR, w.DWORD, w.DWORD, w.LPVOID, w.DWORD, w.DWORD, w.HANDLE)
_CloseHandle = _proto(kernel32.CloseHandle, w.BOOL, w.HANDLE)
_LockFileEx = _proto(kernel32.LockFileEx, w.BOOL, w.HANDLE, w.DWORD, w.DWORD, w.DWORD, w.DWORD, ctypes.POINTER(OVERLAPPED))
_UnlockFileEx = _proto(kernel32.UnlockFileEx, w.BOOL, w.HANDLE, w.DWORD, w.DWORD, w.DWORD, ctypes.POINTER(OVERLAPPED))
_ReadFile = _proto(kernel32.ReadFile, w.BOOL, w.HANDLE, w.LPVOID, w.DWORD, ctypes.POINTER(w.DWORD), ctypes.POINTER(OVERLAPPED))
_WriteFile = _proto(kernel32.WriteFile, w.BOOL, w.HANDLE, w.LPCVOID, w.DWORD, ctypes.POINTER(w.DWORD), ctypes.POINTER(OVERLAPPED))
_FlushFileBuffers = _proto(kernel32.FlushFileBuffers, w.BOOL, w.HANDLE)
_MoveFileExW = _proto(kernel32.MoveFileExW, w.BOOL, w.LPCWSTR, w.LPCWSTR, w.DWORD)
_GetFileInformationByHandleEx = _proto(kernel32.GetFileInformationByHandleEx, w.BOOL, w.HANDLE, ctypes.c_int, w.LPVOID, w.DWORD)
_SetFileInformationByHandle = _proto(kernel32.SetFileInformationByHandle, w.BOOL, w.HANDLE, ctypes.c_int, w.LPVOID, w.DWORD)
_ReadDirectoryChangesW = _proto(kernel32.ReadDirectoryChangesW, w.BOOL, w.HANDLE, w.LPVOID, w.DWORD, w.BOOL, w.DWORD,
                                ctypes.POINTER(w.DWORD), ctypes.POINTER(OVERLAPPED), w.LPVOID)
_GetOverlappedResult = _proto(kernel32.GetOverlappedResult, w.BOOL, w.HANDLE, ctypes.POINTER(OVERLAPPED), ctypes.POINTER(w.DWORD), w.BOOL)
_CancelIoEx = _proto(kernel32.CancelIoEx, w.BOOL, w.HANDLE, ctypes.POINTER(OVERLAPPED))
_CreateEventW = _proto(kernel32.CreateEventW, w.HANDLE, w.LPVOID, w.BOOL, w.BOOL, w.LPCWSTR)
_ResetEvent = _proto(kernel32.ResetEvent, w.BOOL, w.HANDLE)
_SetEvent = _proto(kernel32.SetEvent, w.BOOL, w.HANDLE)
_WaitForMultipleObjects = _proto(kernel32.WaitForMultipleObjects, w.DWORD, w.DWORD, ctypes.POINTER(w.HANDLE), w.BOOL, w.DWORD)
_GetVolumePathNameW = _proto(kernel32.GetVolumePathNameW, w.BOOL, w.LPCWSTR, w.LPWSTR, w.DWORD)
_GetVolumeInformationW = _proto(kernel32.GetVolumeInformationW, w.BOOL, w.LPCWSTR, w.LPWSTR, w.DWORD, ctypes.POINTER(w.DWORD),
                                ctypes.POINTER(w.DWORD), ctypes.POINTER(w.DWORD), w.LPWSTR, w.DWORD)
_GetCurrentProcess = _proto(kernel32.GetCurrentProcess, w.HANDLE)
_LocalFree = _proto(kernel32.LocalFree, w.HLOCAL, w.HLOCAL)
_OpenProcessToken = _proto(advapi32.OpenProcessToken, w.BOOL, w.HANDLE, w.DWORD, ctypes.POINTER(w.HANDLE))
_GetTokenInformation = _proto(advapi32.GetTokenInformation, w.BOOL, w.HANDLE, ctypes.c_int, w.LPVOID, w.DWORD, ctypes.POINTER(w.DWORD))
_ConvertSidToStringSidW = _proto(advapi32.ConvertSidToStringSidW, w.BOOL, w.LPVOID, ctypes.POINTER(w.LPWSTR))
_OpenProcess = _proto(kernel32.OpenProcess, w.HANDLE, w.DWORD, w.BOOL, w.DWORD)
_TerminateProcess = _proto(kernel32.TerminateProcess, w.BOOL, w.HANDLE, w.UINT)
_WaitForSingleObject = _proto(kernel32.WaitForSingleObject, w.DWORD, w.HANDLE, w.DWORD)
_GetExitCodeProcess = _proto(kernel32.GetExitCodeProcess, w.BOOL, w.HANDLE, ctypes.POINTER(w.DWORD))


def _fail(what: str) -> OSError:
    code = ctypes.get_last_error()
    return OSError(0, '%s failed: %s' % (what, ctypes.FormatError(code).strip()), None, code)


class Handle:
    """An owned Win32 handle; close() is idempotent."""

    def __init__(self, value):
        self.value = value

    def close(self):
        if self.value is not None:
            value, self.value = self.value, None
            _CloseHandle(value)

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        self.close()

    def __del__(self):
        self.close()


def open_handle(path, access, share=FILE_SHARE_ALL, disposition=OPEN_EXISTING, flags=0) -> Handle:
    value = _CreateFileW(str(path), access, share, None, disposition, flags, None)
    if value == INVALID_HANDLE_VALUE or value is None:
        raise _fail('CreateFileW')
    return Handle(value)


def open_metadata(path) -> Handle:
    """A handle that can only read attributes; never follows a reparse point."""
    return open_handle(path, FILE_READ_ATTRIBUTES, FILE_SHARE_ALL, OPEN_EXISTING,
                       FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)


def lock_range(handle: Handle, offset: int = 0, length: int = 1, *, exclusive=True, wait=False) -> None:
    flags = (LOCKFILE_EXCLUSIVE_LOCK if exclusive else 0) | (0 if wait else LOCKFILE_FAIL_IMMEDIATELY)
    overlapped = OVERLAPPED(Offset=offset & 0xFFFFFFFF, OffsetHigh=offset >> 32)
    if not _LockFileEx(handle.value, flags, 0, length, 0, ctypes.byref(overlapped)):
        raise _fail('LockFileEx')


def unlock_range(handle: Handle, offset: int = 0, length: int = 1) -> None:
    overlapped = OVERLAPPED(Offset=offset & 0xFFFFFFFF, OffsetHigh=offset >> 32)
    if not _UnlockFileEx(handle.value, 0, length, 0, ctypes.byref(overlapped)):
        raise _fail('UnlockFileEx')


def read_at(handle: Handle, offset: int, size: int) -> bytes:
    buffer = ctypes.create_string_buffer(size)
    done = w.DWORD()
    overlapped = OVERLAPPED(Offset=offset & 0xFFFFFFFF, OffsetHigh=offset >> 32)
    if not _ReadFile(handle.value, buffer, size, ctypes.byref(done), ctypes.byref(overlapped)):
        if ctypes.get_last_error() == ERROR_HANDLE_EOF:
            return b''
        raise _fail('ReadFile')
    return buffer.raw[:done.value]


def read_all(handle: Handle, start: int = 0, chunk: int = 1 << 20) -> bytes:
    """Read from start to the end through explicit offsets (the file pointer is not used)."""
    parts, offset = [], start
    while True:
        data = read_at(handle, offset, chunk)
        if not data:
            return b''.join(parts)
        parts.append(data)
        offset += len(data)


def write_all(handle: Handle, data: bytes) -> None:
    view, done = memoryview(data), w.DWORD()
    while view:
        if not _WriteFile(handle.value, bytes(view[:1 << 20]), min(len(view), 1 << 20), ctypes.byref(done), None):
            raise _fail('WriteFile')
        view = view[done.value:]


def flush(handle: Handle) -> None:
    if not _FlushFileBuffers(handle.value):
        raise _fail('FlushFileBuffers')


def move_replace_write_through(source, destination) -> None:
    if not _MoveFileExW(str(source), str(destination), MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH):
        raise _fail('MoveFileExW')


def basic_info(handle: Handle) -> FILE_BASIC_INFO:
    info = FILE_BASIC_INFO()
    if not _GetFileInformationByHandleEx(handle.value, FILE_BASIC_INFO_CLASS, ctypes.byref(info), ctypes.sizeof(info)):
        raise _fail('GetFileInformationByHandleEx(FileBasicInfo)')
    return info


def standard_info(handle: Handle) -> FILE_STANDARD_INFO:
    info = FILE_STANDARD_INFO()
    if not _GetFileInformationByHandleEx(handle.value, FILE_STANDARD_INFO_CLASS, ctypes.byref(info), ctypes.sizeof(info)):
        raise _fail('GetFileInformationByHandleEx(FileStandardInfo)')
    return info


def file_id(handle: Handle) -> str:
    info = FILE_ID_INFO()
    if not _GetFileInformationByHandleEx(handle.value, FILE_ID_INFO_CLASS, ctypes.byref(info), ctypes.sizeof(info)):
        raise _fail('GetFileInformationByHandleEx(FileIdInfo)')
    return '%016x:%s' % (info.VolumeSerialNumber, bytes(info.FileId).hex())


def set_basic_info(handle: Handle, info: FILE_BASIC_INFO) -> None:
    copy = FILE_BASIC_INFO(info.CreationTime, info.LastAccessTime, info.LastWriteTime, info.ChangeTime, info.FileAttributes)
    if not _SetFileInformationByHandle(handle.value, FILE_BASIC_INFO_CLASS, ctypes.byref(copy), ctypes.sizeof(copy)):
        raise _fail('SetFileInformationByHandle(FileBasicInfo)')


def create_event(manual_reset=True) -> Handle:
    value = _CreateEventW(None, bool(manual_reset), False, None)
    if not value:
        raise _fail('CreateEventW')
    return Handle(value)


def reset_event(event: Handle) -> None:
    if not _ResetEvent(event.value):
        raise _fail('ResetEvent')


def set_event(event: Handle) -> None:
    if not _SetEvent(event.value):
        raise _fail('SetEvent')


def wait_any(handles, timeout_ms: int) -> int:
    """Index of the signalled handle, or -1 on timeout."""
    array = (w.HANDLE * len(handles))(*[h.value for h in handles])
    result = _WaitForMultipleObjects(len(handles), array, False, timeout_ms)
    if result == WAIT_TIMEOUT:
        return -1
    if result < WAIT_OBJECT_0 + len(handles):
        return result - WAIT_OBJECT_0
    raise _fail('WaitForMultipleObjects')


def read_directory_changes(directory: Handle, buffer, size: int, overlapped: OVERLAPPED,
                           subtree=True, notify_filter=WATCH_FILTER) -> None:
    """Issue one overlapped read; completion is collected with overlapped_result."""
    if not _ReadDirectoryChangesW(directory.value, buffer, size, bool(subtree), notify_filter, None,
                                  ctypes.byref(overlapped), None):
        raise _fail('ReadDirectoryChangesW')


def overlapped_result(handle: Handle, overlapped: OVERLAPPED, wait=True):
    """(ok, bytes, last_error) without raising: the caller classifies overflow and errors."""
    done = w.DWORD()
    ok = bool(_GetOverlappedResult(handle.value, ctypes.byref(overlapped), ctypes.byref(done), bool(wait)))
    return ok, done.value, ctypes.get_last_error()


def cancel_io(handle: Handle, overlapped: OVERLAPPED) -> bool:
    return bool(_CancelIoEx(handle.value, ctypes.byref(overlapped)))


def parse_notifications(buffer, size: int):
    """FILE_NOTIFY_INFORMATION records as [(action, relative name)]."""
    raw = ctypes.string_at(buffer, size)
    events, offset = [], 0
    while offset + 12 <= size:
        next_offset, action, length = (int.from_bytes(raw[offset + i:offset + i + 4], 'little') for i in (0, 4, 8))
        name = raw[offset + 12:offset + 12 + length].decode('utf-16-le', 'replace')
        events.append((NOTIFY_ACTIONS.get(action, 'action-%d' % action), name))
        if not next_offset:
            break
        offset += next_offset
    return events


def open_process(pid: int) -> Handle:
    """A handle that can terminate, wait for and query one process (bound to that process, not its PID)."""
    value = _OpenProcess(PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE, False, pid)
    if not value:
        raise _fail('OpenProcess')
    return Handle(value)


def terminate_process(process: Handle, code: int = 1) -> None:
    """TerminateProcess: no cleanup in the target, the kernel releases its handles and locks."""
    if not _TerminateProcess(process.value, code):
        raise _fail('TerminateProcess')


def wait_process(process: Handle, timeout_ms: int) -> bool:
    """True once the process has exited (its handles are closed), False on timeout."""
    result = _WaitForSingleObject(process.value, timeout_ms)
    if result == WAIT_TIMEOUT:
        return False
    if result == WAIT_OBJECT_0:
        return True
    raise _fail('WaitForSingleObject')


def exit_code(process: Handle) -> int:
    code = w.DWORD()
    if not _GetExitCodeProcess(process.value, ctypes.byref(code)):
        raise _fail('GetExitCodeProcess')
    return code.value


def volume_file_system(path) -> str:
    root = ctypes.create_unicode_buffer(1024)
    if not _GetVolumePathNameW(str(path), root, 1024):
        raise _fail('GetVolumePathNameW')
    name = ctypes.create_unicode_buffer(64)
    if not _GetVolumeInformationW(root.value, None, 0, None, None, None, name, 64):
        raise _fail('GetVolumeInformationW')
    return name.value


def current_user_sid() -> str:
    """String SID of the process user (used for the deny ACE; never published)."""
    token = w.HANDLE()
    if not _OpenProcessToken(_GetCurrentProcess(), 0x0008, ctypes.byref(token)):
        raise _fail('OpenProcessToken')
    token = Handle(token.value)
    with token:
        size = w.DWORD()
        _GetTokenInformation(token.value, 1, None, 0, ctypes.byref(size))
        buffer = ctypes.create_string_buffer(size.value)
        if not _GetTokenInformation(token.value, 1, buffer, size, ctypes.byref(size)):
            raise _fail('GetTokenInformation')
        user = ctypes.cast(buffer, ctypes.POINTER(SID_AND_ATTRIBUTES)).contents
        text = w.LPWSTR()
        if not _ConvertSidToStringSidW(user.Sid, ctypes.byref(text)):
            raise _fail('ConvertSidToStringSidW')
        try:
            return text.value
        finally:
            _LocalFree(ctypes.cast(text, w.HLOCAL))
