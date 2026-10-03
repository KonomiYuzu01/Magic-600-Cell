"""Probe candidates for the importer's platform operations (proposal section 4) and the stage 5 decision.

Windows only, except `decide` and `diff_snapshots`, which are pure and tested headless.
Nothing here writes into a source directory: the lock handle is read-only, the
watch and the snapshot open handles that can only list or read attributes, and
digests read through read-only handles (engine.lock through the lock handle).
"""
from __future__ import annotations

import ctypes
import hashlib
import os
import sqlite3
import sys
import threading
import time
from dataclasses import dataclass, field
from pathlib import Path

if os.name == 'nt':
    import winapi as W
else:  # pragma: no cover - the decision logic is still importable for headless tests
    W = None


class LockRefused(RuntimeError):
    """The read-only lock was not granted; `reason` is short and public."""

    def __init__(self, reason: str, code: int | None = None):
        super().__init__(reason)
        self.reason = reason
        self.code = code


class ReadOnlyLock:
    """Proposal 4.1: an exclusive lock of engine.lock byte 0 through a read-only handle; never creates the file."""

    def __init__(self, directory):
        path = Path(directory) / 'engine.lock'
        try:
            self.handle = W.open_handle(path, W.GENERIC_READ, W.FILE_SHARE_READ | W.FILE_SHARE_WRITE,
                                        W.OPEN_EXISTING, W.FILE_FLAG_OPEN_REPARSE_POINT)
        except OSError as error:
            if error.winerror == W.ERROR_FILE_NOT_FOUND:
                raise LockRefused('engine-lock-missing', error.winerror) from None
            raise LockRefused('engine-lock-open-failed', error.winerror) from None
        try:
            W.lock_range(self.handle, 0, 1)
        except OSError as error:
            self.handle.close()
            reason = 'engine-running' if error.winerror == W.ERROR_LOCK_VIOLATION else 'lock-failed'
            raise LockRefused(reason, error.winerror) from None
        self.held = True
        self.write_guard = None
        self.directory = Path(directory)

    def guard(self, entries: dict) -> None:
        """Hold the WriteGuard for the files of the pre-lock snapshot; refuses (and releases) on failure."""
        try:
            self.write_guard = WriteGuard(self.directory, entries)
        except LockRefused:
            self.release()
            raise

    def read_bytes(self) -> bytes:
        return W.read_all(self.handle)

    def release(self) -> None:
        if self.write_guard is not None:
            self.write_guard.release()
            self.write_guard = None
        if self.held:
            self.held = False
            try:
                W.unlock_range(self.handle, 0, 1)
            finally:
                self.handle.close()


class WriteGuard:
    """MPR-04: prevention in the importer's own path. While the lock is held, a read handle that
    shares read only is open on every existing file of the source except engine.lock, so no
    process can open one of them for writing or deletion (and so cannot map it writable).

    The watch cannot see a mapped write that is restored before the final digest; this guard
    makes such a write impossible instead. New files are not covered: their names are what the
    watch reports reliably. engine.lock stays writable so that a 0.4 start fails with its own
    message (and its appended byte is detected). The handles read nothing and change nothing.
    """

    def __init__(self, directory, entries: dict):
        self.handles = []
        root = Path(directory)
        try:
            for relative in sorted(entries):
                meta = entries[relative]
                if relative in ('.', 'engine.lock') or meta['directory']:
                    continue
                self.handles.append(W.open_handle(root / relative, W.GENERIC_READ, W.FILE_SHARE_READ,
                                                  W.OPEN_EXISTING, W.FILE_FLAG_OPEN_REPARSE_POINT))
        except OSError as error:
            self.release()
            # A sharing violation means another process has the file open for writing.
            raise LockRefused('source-in-use' if error.winerror == ERROR_SHARING_VIOLATION else 'guard-failed',
                              error.winerror) from None

    def release(self) -> None:
        while self.handles:
            self.handles.pop().close()


ERROR_SHARING_VIOLATION = 32


@dataclass
class WatchResult:
    events: list = field(default_factory=list)   # [(monotonic time, action, relative name)]
    overflows: int = 0
    overflow_forms: list = field(default_factory=list)  # e.g. 'ok-zero-bytes', 'error-1022'
    errors: list = field(default_factory=list)
    completions: int = 0
    armed_before_snapshot: bool = False

    @property
    def coverage_valid(self) -> bool:
        return self.armed_before_snapshot and not self.overflows and not self.errors


class DirectoryWatch:
    """Proposal stage 2 and 5: ReadDirectoryChangesW armed from before the pre-lock snapshot to after the drain."""

    def __init__(self, directory, buffer_size: int = 65536):
        self.directory = Path(directory)
        self.buffer_size = buffer_size
        self.result = WatchResult()
        self._mutex = threading.Lock()
        self._completed = threading.Condition(self._mutex)
        self._thread = None
        self.started_at = None
        self.last_completion = None

    def start(self) -> None:
        self.handle = W.open_handle(self.directory, W.FILE_LIST_DIRECTORY, W.FILE_SHARE_ALL, W.OPEN_EXISTING,
                                    W.FILE_FLAG_BACKUP_SEMANTICS | W.FILE_FLAG_OVERLAPPED)
        self.event = W.create_event()
        self.stop_event = W.create_event()
        # ctypes buffers are malloc-aligned, which satisfies the DWORD alignment the call requires.
        self.buffer = ctypes.create_string_buffer(self.buffer_size)
        self.overlapped = W.OVERLAPPED()
        self.overlapped.hEvent = self.event.value
        W.read_directory_changes(self.handle, self.buffer, self.buffer_size, self.overlapped)
        self.started_at = time.monotonic()
        self.result.armed_before_snapshot = True
        self._thread = threading.Thread(target=self._run, name='migration-probe-watch', daemon=True)
        self._thread.start()

    def record(self, ok: bool, size: int, error: int, *, injected: str | None = None) -> None:
        """Classify one completion. The same path serves real completions and the F21 injections."""
        with self._mutex:
            now = time.monotonic()
            self.result.completions += 1
            self.last_completion = now
            prefix = 'injected-' if injected else ''
            if ok and size == 0:
                # Windows also leaves ERROR_NOTIFY_ENUM_DIR as the last error; record it, but the zero size decides.
                self.result.overflows += 1
                self.result.overflow_forms.append(prefix + 'ok-zero-bytes' + ('-last-error-%d' % error if error else ''))
            elif not ok and error == W.ERROR_NOTIFY_ENUM_DIR:
                self.result.overflows += 1
                self.result.overflow_forms.append(prefix + 'error-1022')
            elif not ok:
                self.result.errors.append(prefix + 'error-%d' % error)
            else:
                for action, name in W.parse_notifications(self.buffer, size):
                    self.result.events.append((now, action, name))
            self._completed.notify_all()

    def inject_zero_completion(self) -> None:
        """F21: a successful completion with zero bytes, as Windows reports a discarded buffer."""
        self.record(True, 0, 0, injected='zero')

    def inject_error(self, code: int = 87) -> None:
        """F21: a failed completion (ERROR_INVALID_PARAMETER by default)."""
        self.record(False, 0, code, injected='error')

    def wait_for_completion(self, after: int, timeout: float = 2.0) -> bool:
        deadline = time.monotonic() + timeout
        with self._mutex:
            while self.result.completions <= after:
                left = deadline - time.monotonic()
                if left <= 0:
                    return False
                self._completed.wait(left)
            return True

    def completions(self) -> int:
        with self._mutex:
            return self.result.completions

    def _run(self) -> None:
        while True:
            index = W.wait_any([self.event, self.stop_event], W.INFINITE)
            if index == 1:
                return
            ok, size, error = W.overlapped_result(self.handle, self.overlapped, wait=False)
            if not ok and error == W.ERROR_OPERATION_ABORTED:
                return
            self.record(ok, size, error)
            try:
                W.reset_event(self.event)
                W.read_directory_changes(self.handle, self.buffer, self.buffer_size, self.overlapped)
            except OSError as failure:
                with self._mutex:
                    self.result.errors.append('rearm-error-%d' % (failure.winerror or 0))
                return

    def stop_and_drain(self, quiet: float = 0.25, limit: float = 5.0) -> WatchResult:
        """Wait until no completion arrives for `quiet` seconds, then cancel the outstanding read and collect it."""
        begin = time.monotonic()
        while True:
            with self._mutex:
                last = self.last_completion or self.started_at
            now = time.monotonic()
            if now - last >= quiet:
                break
            if now - begin >= limit:
                with self._mutex:
                    self.result.errors.append('drain-timeout')
                break
            time.sleep(0.02)
        W.set_event(self.stop_event)
        self._thread.join(5)
        if self._thread.is_alive():
            self.result.errors.append('watch-thread-stuck')
        W.cancel_io(self.handle, self.overlapped)
        ok, size, error = W.overlapped_result(self.handle, self.overlapped, wait=True)
        if ok or error != W.ERROR_OPERATION_ABORTED:
            self.record(ok, size, error)
        self.handle.close()
        self.event.close()
        self.stop_event.close()
        return self.result


def _entry_metadata(path: Path) -> dict:
    with W.open_metadata(path) as handle:
        basic = W.basic_info(handle)
        standard = W.standard_info(handle)
        identity = W.file_id(handle)
    return dict(size=standard.EndOfFile, links=standard.NumberOfLinks, directory=bool(standard.Directory),
                attributes=basic.FileAttributes, created=basic.CreationTime, written=basic.LastWriteTime,
                changed=basic.ChangeTime, id=identity)


def snapshot(directory) -> dict:
    """Membership and handle-based metadata of the directory and every entry; reads no file content."""
    root = Path(directory)
    entries = {'.': _entry_metadata(root)}
    pending = [root]
    while pending:
        current = pending.pop()
        with os.scandir(current) as listing:
            for item in listing:
                path = Path(item.path)
                relative = path.relative_to(root).as_posix()
                meta = _entry_metadata(path)
                entries[relative] = meta
                if meta['directory'] and not meta['attributes'] & W.FILE_ATTRIBUTE_REPARSE_POINT:
                    pending.append(path)
    return entries


def digests(directory, lock: ReadOnlyLock, entries: dict, after_engine_lock=None) -> dict:
    """Byte SHA-256 of every file in a snapshot; engine.lock through the lock handle (byte 0 is locked)."""
    root = Path(directory)
    result = {}
    for relative in sorted(entries):
        meta = entries[relative]
        if relative == '.' or meta['directory']:
            continue
        if relative == 'engine.lock':
            data = lock.read_bytes()
            result[relative] = hashlib.sha256(data).hexdigest()
            if after_engine_lock:
                after_engine_lock()
            continue
        with W.open_handle(root / relative, W.GENERIC_READ, W.FILE_SHARE_READ | W.FILE_SHARE_WRITE,
                           W.OPEN_EXISTING, W.FILE_FLAG_OPEN_REPARSE_POINT) as handle:
            result[relative] = hashlib.sha256(W.read_all(handle)).hexdigest()
    return result


def diff_snapshots(before: dict, after: dict) -> list:
    """Sorted list of 'name:field' (or 'name:added'/'name:removed') differences."""
    changes = []
    for name in sorted(set(before) | set(after)):
        if name not in after:
            changes.append(name + ':removed')
        elif name not in before:
            changes.append(name + ':added')
        else:
            changes.extend('%s:%s' % (name, key) for key in sorted(set(before[name]) | set(after[name]))
                           if before[name].get(key) != after[name].get(key))
    return changes


@dataclass
class Decision:
    accepted: bool
    reasons: list
    snapshot_changes: list
    digest_changes: list


def decide(watch: WatchResult, pre: dict, final: dict, lock_digests: dict, final_digests: dict) -> Decision:
    """Stage 5: only an empty drained event list, valid coverage and equal snapshots and digests accept the source.

    The same rule applies in every mode (MPR-03): a deny ACE prevents writes, it never
    excuses an event, an overflow or a watch error. The decision does not say who changed
    the directory (MPR-06); a probe that injects a change records that itself.
    """
    reasons = []
    if watch.events:
        reasons.append('event')
    if watch.overflows:
        reasons.append('overflow')
    if watch.errors:
        reasons.append('watch-error')
    if not watch.armed_before_snapshot:
        reasons.append('watch-not-armed')
    snapshot_changes = diff_snapshots(pre, final)
    if snapshot_changes:
        reasons.append('snapshot-diff')
    digest_changes = sorted(name for name in set(lock_digests) | set(final_digests)
                            if lock_digests.get(name) != final_digests.get(name))
    if digest_changes:
        reasons.append('digest-diff')
    return Decision(not reasons, reasons, snapshot_changes, digest_changes)


# Proposal 4.2 candidates, in order of preference. Every strategy sets NO_CKPT_ON_CLOSE before the first read.
STRATEGIES = {
    'C1a': dict(query='mode=ro', pragmas=(), persist_wal=False,
                text='mode=ro, NO_CKPT_ON_CLOSE'),
    'C1b': dict(query='mode=ro&readonly_shm=1', pragmas=(), persist_wal=False,
                text='mode=ro, readonly_shm=1, NO_CKPT_ON_CLOSE'),
    'C1c': dict(query='mode=ro', pragmas=('PRAGMA locking_mode=EXCLUSIVE',), persist_wal=False,
                text='mode=ro, locking_mode=EXCLUSIVE, NO_CKPT_ON_CLOSE'),
    'C1d': dict(query='mode=ro', pragmas=(), persist_wal=True,
                text='mode=ro, SQLITE_FCNTL_PERSIST_WAL=1, NO_CKPT_ON_CLOSE (C API)'),
    'C2a': dict(query='mode=ro&nolock=1', pragmas=(), persist_wal=False,
                text='mode=ro, nolock=1 under the importer lock, NO_CKPT_ON_CLOSE'),
    'C2b': dict(query='mode=ro&nolock=1&readonly_shm=1', pragmas=(), persist_wal=False,
                text='mode=ro, nolock=1, readonly_shm=1, NO_CKPT_ON_CLOSE'),
}


class InjectedFailure(RuntimeError):
    """The P1 failure path: raised from the backup progress callback after the first step."""


class BackupStalled(RuntimeError):
    """The backup hit a resource bound: a BUSY/LOCKED stall, a restart loop, or the total deadline.

    Python's backup() retries BUSY forever, and a backup whose source changes under it
    restarts from page one on every step with status OK (seen with readonly_shm=1 on a
    clean source), so both APIs need an explicit bound. Hitting a bound says nothing about
    whether a strategy preserves the source: it makes an attempt inconclusive, not failed.
    """


SQLITE_BUSY, SQLITE_LOCKED = 5, 6
BACKUP_STALL_SECONDS = 5.0
BACKUP_NO_PROGRESS_STEPS = 100
BACKUP_DEADLINE_SECONDS = 60.0


class BackupBound:
    """Shared progress bound for both APIs; `check` raises BackupStalled.

    - A BUSY/LOCKED stall is timed from its own first BUSY/LOCKED and ends at the next OK step.
    - A restart loop is 100 consecutive OK steps without a new minimum of remaining pages.
    - The total deadline applies only to a backup that has not finished (remaining > 0).
    """

    def __init__(self, clock=time.monotonic):
        self.clock = clock
        self.began = clock()
        self.stall_began = None
        self.best = None
        self.flat = 0
        self.statuses = []

    def check(self, status: int, remaining: int) -> None:
        self.statuses.append(status)
        now = self.clock()
        if status in (SQLITE_BUSY, SQLITE_LOCKED):
            if self.stall_began is None:
                self.stall_began = now
            elif now - self.stall_began > BACKUP_STALL_SECONDS:
                raise BackupStalled('backup-stalled-status-%d' % status)
        else:
            self.stall_began = None
            if remaining == 0:
                return
            if self.best is None or remaining < self.best:
                self.best, self.flat = remaining, 0
            else:
                self.flat += 1
                if self.flat >= BACKUP_NO_PROGRESS_STEPS:
                    raise BackupStalled('backup-no-progress')
        if now - self.began > BACKUP_DEADLINE_SECONDS:
            raise BackupStalled('backup-deadline')


def source_uri(database: Path, query: str) -> str:
    return database.resolve().as_uri() + '?' + query


class PySource:
    """A strategy through Python's sqlite3 module."""

    def __init__(self, database: Path, spec: dict):
        self.connection = sqlite3.connect(source_uri(database, spec['query']), uri=True, isolation_level=None)
        self.connection.setconfig(sqlite3.SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE, True)
        for pragma in spec['pragmas']:
            self.connection.execute(pragma).fetchall()

    def model(self) -> str | None:
        row = self.connection.execute("SELECT value FROM meta WHERE key='model'").fetchone()
        return row[0] if row else None

    def backup(self, destination: Path, fail_after_first_step: bool, after_first_step=None) -> None:
        """`after_first_step` runs once inside the copy window (the external-change control)."""
        target = sqlite3.connect(destination)
        bound = BackupBound()
        self.statuses = bound.statuses
        first = [True]

        def progress(status, remaining, total):
            bound.check(status, remaining)
            if status in (SQLITE_BUSY, SQLITE_LOCKED):
                return
            if first[0]:
                first[0] = False
                if after_first_step:
                    after_first_step()
            if fail_after_first_step:
                raise InjectedFailure('injected')
        try:
            self.connection.backup(target, pages=8, progress=progress, sleep=0.05)
        finally:
            target.close()

    def close(self) -> None:
        self.connection.close()


class CSource:
    """A strategy through the SQLite C API of the interpreter's own sqlite3.dll (for SQLITE_FCNTL_PERSIST_WAL)."""

    SQLITE_OK, SQLITE_ROW, SQLITE_DONE = 0, 100, 101
    SQLITE_OPEN_READONLY, SQLITE_OPEN_READWRITE, SQLITE_OPEN_CREATE, SQLITE_OPEN_URI = 0x1, 0x2, 0x4, 0x40
    SQLITE_FCNTL_PERSIST_WAL = 10
    SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE = 1010

    def __init__(self, database: Path, spec: dict):
        self.lib = lib = ctypes.CDLL(str(Path(sys.base_prefix) / 'DLLs' / 'sqlite3.dll'))
        lib.sqlite3_open_v2.argtypes = [ctypes.c_char_p, ctypes.POINTER(ctypes.c_void_p), ctypes.c_int, ctypes.c_char_p]
        lib.sqlite3_errmsg.restype = ctypes.c_char_p
        lib.sqlite3_errmsg.argtypes = [ctypes.c_void_p]
        lib.sqlite3_file_control.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_int, ctypes.c_void_p]
        lib.sqlite3_prepare_v2.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_int,
                                           ctypes.POINTER(ctypes.c_void_p), ctypes.c_void_p]
        lib.sqlite3_step.argtypes = [ctypes.c_void_p]
        lib.sqlite3_column_text.restype = ctypes.c_char_p
        lib.sqlite3_column_text.argtypes = [ctypes.c_void_p, ctypes.c_int]
        lib.sqlite3_finalize.argtypes = [ctypes.c_void_p]
        lib.sqlite3_close_v2.argtypes = [ctypes.c_void_p]
        lib.sqlite3_backup_init.restype = ctypes.c_void_p
        lib.sqlite3_backup_init.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_void_p, ctypes.c_char_p]
        lib.sqlite3_backup_step.argtypes = [ctypes.c_void_p, ctypes.c_int]
        lib.sqlite3_backup_finish.argtypes = [ctypes.c_void_p]
        lib.sqlite3_backup_remaining.argtypes = [ctypes.c_void_p]
        self.db = ctypes.c_void_p()
        uri = source_uri(database, spec['query']).encode()
        rc = lib.sqlite3_open_v2(uri, ctypes.byref(self.db), self.SQLITE_OPEN_READONLY | self.SQLITE_OPEN_URI, None)
        if rc != self.SQLITE_OK:
            self._raise(rc)
        result = ctypes.c_int()
        rc = lib.sqlite3_db_config(self.db, ctypes.c_int(self.SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE), ctypes.c_int(1),
                                   ctypes.byref(result))
        if rc != self.SQLITE_OK or result.value != 1:
            self._raise(rc)
        if spec['persist_wal']:
            flag = ctypes.c_int(1)
            rc = lib.sqlite3_file_control(self.db, b'main', self.SQLITE_FCNTL_PERSIST_WAL, ctypes.byref(flag))
            if rc != self.SQLITE_OK:
                self._raise(rc)

    def _raise(self, rc):
        message = self.lib.sqlite3_errmsg(self.db).decode('utf-8', 'replace') if self.db else 'sqlite error'
        raise sqlite3.OperationalError('%s (rc %d)' % (message, rc))

    def model(self) -> str | None:
        statement = ctypes.c_void_p()
        rc = self.lib.sqlite3_prepare_v2(self.db, b"SELECT value FROM meta WHERE key='model'", -1,
                                         ctypes.byref(statement), None)
        if rc != self.SQLITE_OK:
            self._raise(rc)
        try:
            rc = self.lib.sqlite3_step(statement)
            if rc == self.SQLITE_ROW:
                return self.lib.sqlite3_column_text(statement, 0).decode()
            if rc != self.SQLITE_DONE:
                self._raise(rc)
            return None
        finally:
            self.lib.sqlite3_finalize(statement)

    def backup(self, destination: Path, fail_after_first_step: bool, after_first_step=None) -> None:
        target = ctypes.c_void_p()
        rc = self.lib.sqlite3_open_v2(str(destination).encode(), ctypes.byref(target),
                                      self.SQLITE_OPEN_READWRITE | self.SQLITE_OPEN_CREATE, None)
        try:
            if rc != self.SQLITE_OK:
                raise sqlite3.OperationalError('destination open failed (rc %d)' % rc)
            handle = self.lib.sqlite3_backup_init(target, b'main', self.db, b'main')
            if not handle:
                raise sqlite3.OperationalError('backup init failed')
            failure = None
            bound = BackupBound()
            self.statuses = bound.statuses
            first = True
            while True:
                rc = self.lib.sqlite3_backup_step(handle, 8)
                if rc == self.SQLITE_DONE:
                    self.statuses.append(rc)
                    break
                try:
                    bound.check(rc, self.lib.sqlite3_backup_remaining(handle))
                except BackupStalled:
                    self.lib.sqlite3_backup_finish(handle)
                    raise
                if rc in (SQLITE_BUSY, SQLITE_LOCKED):
                    time.sleep(0.05)
                    continue
                if rc != self.SQLITE_OK:
                    failure = rc
                    break
                if first:
                    first = False
                    if after_first_step:
                        after_first_step()
                if fail_after_first_step:
                    self.lib.sqlite3_backup_finish(handle)
                    raise InjectedFailure('injected')
            rc = self.lib.sqlite3_backup_finish(handle)
            if failure is not None or rc != self.SQLITE_OK:
                self._raise(failure if failure is not None else rc)
        finally:
            self.lib.sqlite3_close_v2(target)

    def close(self) -> None:
        if self.db:
            self.lib.sqlite3_close_v2(self.db)
            self.db = ctypes.c_void_p()


def open_source_for_backup(database: Path, strategy: str):
    spec = STRATEGIES[strategy]
    return CSource(database, spec) if spec['persist_wal'] else PySource(database, spec)
