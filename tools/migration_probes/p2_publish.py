"""Probe P2: generation publication, conservative power loss and Windows interruption."""
from __future__ import annotations

import argparse
from dataclasses import dataclass, field
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import stat
import sys
import threading

sys.path.insert(0, str(Path(__file__).resolve().parent))

BARRIERS = tuple('%s-S%d' % (side, step) for step in range(9) for side in ('before', 'after'))
DIRECTORY_ACCESS = ('FILE_ADD_FILE | FILE_ADD_SUBDIRECTORY', 'GENERIC_WRITE', 'GENERIC_READ')


def short_error(error: BaseException) -> dict:
    return dict(error=type(error).__name__, winerror=getattr(error, 'winerror', None))


def _json_bytes(value) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(',', ':')).encode('utf-8')


def _plain_name(name) -> bool:
    return isinstance(name, str) and name not in ('', '.', '..') \
        and not any(char in name for char in ('/', '\\', ':', '\x00'))


def _file_name(name) -> bool:
    return _plain_name(name) and name.upper() != 'MANIFEST.JSON'


def _complete(fs, directory, manifest_sha256=None) -> bool:
    if not fs.is_dir(directory):
        return False
    try:
        manifest = fs.read_file(directory / 'MANIFEST.json')
    except (FileNotFoundError, IsADirectoryError):
        return False
    if manifest_sha256 is not None and hashlib.sha256(manifest).hexdigest() != manifest_sha256:
        return False
    try:
        document = json.loads(manifest)
    except (ValueError, UnicodeError):
        return False
    if not isinstance(document, dict) or not isinstance(document.get('files'), dict):
        return False
    for name, info in document['files'].items():
        if not _file_name(name) or not isinstance(info, dict) or type(info.get('size')) is not int \
                or info['size'] < 0 or not isinstance(info.get('sha256'), str):
            return False
        try:
            data = fs.read_file(directory / name)
        except (FileNotFoundError, IsADirectoryError):
            return False
        if len(data) != info['size'] or hashlib.sha256(data).hexdigest() != info['sha256']:
            return False
    return True


def recover(fs, dest) -> dict:
    """Validate the pointer, remove its stale temporary file and report incomplete generations."""
    active, valid, removed_tmp = None, True, False
    if not fs.exists(dest):
        return dict(active=None, valid=True, removed_tmp=False, incomplete=[])
    pointer_path = dest / 'ACTIVE'
    if fs.exists(pointer_path):
        valid = False
        try:
            pointer = json.loads(fs.read_file(pointer_path))
        except (ValueError, UnicodeError, IsADirectoryError):
            pointer = None
        if isinstance(pointer, dict) and _plain_name(pointer.get('generation')):
            active = pointer['generation']
            digest = pointer.get('manifest_sha256')
            valid = isinstance(digest, str) and _complete(fs, dest / 'generations' / active, digest)
    temporary = dest / 'ACTIVE.tmp'
    if fs.exists(temporary):
        fs.remove(temporary)
        removed_tmp = True
    generations = dest / 'generations'
    incomplete = []
    if fs.is_dir(generations):
        incomplete = sorted(gen for gen in fs.listdir(generations)
                            if gen != active and not _complete(fs, generations / gen))
    return dict(active=active, valid=valid, removed_tmp=removed_tmp, incomplete=incomplete)


def publish_generation(fs, dest, gen, files: dict[str, bytes], barrier=None) -> dict:
    """Publish in S0 to S8 order; S7 is the pointer's commit point. Hooks interrupt by raising."""
    if not _plain_name(gen):
        return dict(status='aborted', step='S0', reason='invalid-generation-name')
    if any(not _file_name(name) for name in files):
        return dict(status='aborted', step='S0', reason='invalid-file-name')
    files = dict(files)
    generations = dest / 'generations'
    directory = generations / gen
    manifest = _json_bytes({'files': {name: dict(size=len(data), sha256=hashlib.sha256(data).hexdigest())
                                    for name, data in files.items()}})
    pointer = _json_bytes(dict(generation=gen, manifest_sha256=hashlib.sha256(manifest).hexdigest()))
    temporary, active = dest / 'ACTIVE.tmp', dest / 'ACTIVE'

    def ancestors():
        for path in (dest, generations):
            if not fs.exists(path):
                fs.mkdir(path)
        for path in (dest.parent, dest, generations):
            fs.flush_dir(path)

    def create_files():
        for name, data in files.items():
            fs.create_file(directory / name, data)

    def flush_files():
        for name in files:
            fs.flush_file(directory / name)

    def create_manifest():
        fs.create_file(directory / 'MANIFEST.json', manifest)
        fs.flush_file(directory / 'MANIFEST.json')

    def flush_generation():
        fs.flush_dir(directory)
        fs.flush_dir(generations)

    def create_pointer():
        if fs.exists(temporary):
            fs.remove(temporary)
        fs.create_file(temporary, pointer)
        fs.flush_file(temporary)

    steps = (
        ('S0', ancestors),
        ('S1', lambda: fs.mkdir(directory)),
        ('S2', create_files),
        ('S3', flush_files),
        ('S4', create_manifest),
        ('S5', flush_generation),
        ('S6', create_pointer),
        ('S7', lambda: fs.move_write_through(temporary, active)),
        ('S8', lambda: fs.flush_dir(dest)),
    )
    for step, operation in steps:
        if barrier is not None:
            barrier('before-' + step)
        try:
            operation()
        except Exception as failure:
            if step == 'S1' and isinstance(failure, FileExistsError):
                return dict(status='aborted', step=step, reason='generation-exists')
            error = short_error(failure)
            result = dict(step=step, reason=error['error'], winerror=error['winerror'])
            if step == 'S7':
                try:
                    recovered = recover(fs, dest)
                except Exception as recovery_failure:
                    return dict(result, status='publication-unknown', recovery_error=short_error(recovery_failure))
                if not recovered['valid']:
                    return dict(result, status='publication-unknown')
                if recovered['active'] != gen:
                    return dict(result, status='aborted')
            else:
                status = 'switched-durability-unconfirmed' if step == 'S8' else 'aborted'
                return dict(result, status=status)
        if barrier is not None:
            barrier('after-' + step)
    return dict(status='published', generation=gen)


@dataclass
class _Node:
    directory: bool
    entries: dict = field(default_factory=dict)
    durable_entries: dict = field(default_factory=dict)
    data: bytes = b''
    durable_data: bytes = b''


class DoubleFS:
    """Entries and data persist independently. Initial paths map to bytes or None for directories."""

    def __init__(self, initial=None, *, fail_flush=None, move_fault=None):
        self.root = _Node(directory=True)
        self.fail_flush = fail_flush
        self.move_fault = move_fault

        def seed_directory(path):
            if self.exists(path):
                if not self.is_dir(path):
                    raise NotADirectoryError()
                return
            seed_directory(path.parent)
            self.mkdir(path)

        for path, data in (initial or {}).items():
            path = self._path(path)
            if data is None:
                seed_directory(path)
            else:
                seed_directory(path.parent)
                self.create_file(path, data)

        def persist(node):
            node.durable_entries = dict(node.entries)
            node.durable_data = node.data
            for child in node.entries.values():
                persist(child)

        persist(self.root)

    @staticmethod
    def _path(path):
        path = PurePosixPath(path)
        if '..' in path.parts:
            raise ValueError('parent-path')
        return path if path.is_absolute() else PurePosixPath('/') / path

    def _node(self, path):
        node = self.root
        for name in self._path(path).parts[1:]:
            if not node.directory:
                raise NotADirectoryError()
            if name not in node.entries:
                raise FileNotFoundError()
            node = node.entries[name]
        return node

    def _parent(self, path):
        path = self._path(path)
        parent = self._node(path.parent)
        if not parent.directory:
            raise NotADirectoryError()
        return parent, path.name

    def exists(self, path):
        try:
            self._node(path)
            return True
        except FileNotFoundError:
            return False

    def is_dir(self, path):
        try:
            return self._node(path).directory
        except FileNotFoundError:
            return False

    def listdir(self, path):
        node = self._node(path)
        if not node.directory:
            raise NotADirectoryError()
        return sorted(node.entries)

    def mkdir(self, path):
        if self.exists(path):
            raise FileExistsError()
        parent, name = self._parent(path)
        parent.entries[name] = _Node(directory=True)

    def create_file(self, path, data):
        if self.exists(path):
            raise FileExistsError()
        parent, name = self._parent(path)
        parent.entries[name] = _Node(directory=False, data=bytes(data))

    def read_file(self, path):
        node = self._node(path)
        if node.directory:
            raise IsADirectoryError()
        return node.data

    def flush_file(self, path):
        node = self._node(path)
        if node.directory:
            raise IsADirectoryError()
        if self.fail_flush is not None and self.fail_flush('file', self._path(path)):
            raise OSError('flush-failed')
        node.durable_data = node.data

    def flush_dir(self, path):
        node = self._node(path)
        if not node.directory:
            raise NotADirectoryError()
        if self.fail_flush is not None and self.fail_flush('dir', self._path(path)):
            raise OSError('flush-failed')
        node.durable_entries = dict(node.entries)

    def move_write_through(self, src, dst):
        if self.move_fault == 'before':
            raise OSError('move-failed')
        source, source_name = self._parent(src)
        destination, destination_name = self._parent(dst)
        if source_name not in source.entries:
            raise FileNotFoundError()
        node = source.entries.pop(source_name)
        destination.entries[destination_name] = node
        source.durable_entries.pop(source_name, None)
        destination.durable_entries[destination_name] = node
        if self.move_fault == 'after':
            raise OSError('move-failed')

    def remove(self, path):
        node = self._node(path)
        if node.directory and node.entries:
            raise OSError('directory-not-empty')
        parent, name = self._parent(path)
        del parent.entries[name]

    def power_loss(self):
        def restore(node):
            node.entries = dict(node.durable_entries)
            node.data = node.durable_data
            for child in node.entries.values():
                restore(child)

        restore(self.root)


def _windows_modules():
    if os.name != 'nt':
        raise ImportError('Windows-only')
    import fixtures as F
    import winapi as W
    return F, W


class WindowsFS:
    """Win32 persistence operations on guarded disposable paths only."""

    def __init__(self, run_root, directory_access=None):
        self.F, self.W = _windows_modules()
        self.run_root = Path(run_root)
        self.directory_access = directory_access if directory_access is not None \
            else self.W.FILE_ADD_FILE | self.W.FILE_ADD_SUBDIRECTORY

    def _path(self, path):
        return self.F.guard(path, self.run_root)

    def exists(self, path):
        path = self._path(path)
        try:
            path.stat()
            return True
        except FileNotFoundError:
            return False

    def is_dir(self, path):
        path = self._path(path)
        try:
            return stat.S_ISDIR(path.stat().st_mode)
        except FileNotFoundError:
            return False

    def listdir(self, path):
        return sorted(self._path(item).name for item in self._path(path).iterdir())

    def mkdir(self, path):
        self._path(path).mkdir()

    def create_file(self, path, data):
        W = self.W
        with W.open_handle(self._path(path), W.GENERIC_WRITE, W.FILE_SHARE_ALL, W.CREATE_NEW) as handle:
            W.write_all(handle, data)

    def read_file(self, path):
        return self._path(path).read_bytes()

    def flush_file(self, path):
        W = self.W
        with W.open_handle(self._path(path), W.GENERIC_WRITE, W.FILE_SHARE_ALL, W.OPEN_EXISTING) as handle:
            W.flush(handle)

    def flush_dir(self, path):
        W = self.W
        with W.open_handle(self._path(path), self.directory_access, W.FILE_SHARE_ALL, W.OPEN_EXISTING,
                           W.FILE_FLAG_BACKUP_SEMANTICS) as handle:
            W.flush(handle)

    def move_write_through(self, src, dst):
        self.W.move_replace_write_through(self._path(src), self._path(dst))

    def remove(self, path):
        path = self._path(path)
        if self.is_dir(path):
            path.rmdir()
        else:
            path.unlink()


def _attempt(operation):
    try:
        operation()
        return dict(ok=True)
    except Exception as failure:
        return dict(ok=False, **short_error(failure))


def qualify(run_root) -> dict:
    fs = WindowsFS(run_root)
    W = fs.W
    work = fs._path(fs.run_root / 'qualification')
    fs.mkdir(work)
    file = work / 'file.bin'
    fs.create_file(file, b'P2 file flush qualification')
    record = dict(file_system=None, file_flush=_attempt(lambda: fs.flush_file(file)), directory_flush={})
    try:
        record['file_system'] = W.volume_file_system(fs._path(work))
    except Exception as failure:
        record['file_system_error'] = short_error(failure)
    accesses = {'GENERIC_READ': W.GENERIC_READ, 'GENERIC_WRITE': W.GENERIC_WRITE,
                DIRECTORY_ACCESS[0]: W.FILE_ADD_FILE | W.FILE_ADD_SUBDIRECTORY}
    for name in ('GENERIC_READ', 'GENERIC_WRITE', DIRECTORY_ACCESS[0]):
        fs.directory_access = accesses[name]
        record['directory_flush'][name] = _attempt(lambda: fs.flush_dir(work))
    selected = next((name for name in DIRECTORY_ACCESS if record['directory_flush'][name]['ok']), None)
    record['directory_access'] = selected

    def move():
        source, destination = work / 'move.tmp', work / 'move'
        fs.create_file(source, b'new pointer')
        fs.flush_file(source)
        fs.create_file(destination, b'old pointer')
        fs.flush_file(destination)
        fs.move_write_through(source, destination)
        if fs.exists(source) or fs.read_file(destination) != b'new pointer':
            raise RuntimeError('move-content-mismatch')

    record['move_write_through'] = _attempt(move)
    record['passed'] = record['file_system'] is not None and record['file_flush']['ok'] \
        and selected is not None and record['move_write_through']['ok']
    return record


def _probe_files(gen):
    seed = sum(gen.encode('utf-8')) % 256
    pattern = bytes((seed + index) % 256 for index in range(256))
    return {'payload-%02d.bin' % size: pattern * (size * 1024 // 256) for size in (4, 16, 64)}


def _child(args) -> int:
    F, _ = _windows_modules()
    print('READY ' + json.dumps(dict(pid=os.getpid())), flush=True)
    request_path = Path(args.request)
    run_root = None
    for parent in request_path.parents:
        try:
            request_path = F.guard(request_path, parent)
            run_root = parent
            break
        except F.ProbeRefusal:
            continue
    if run_root is None:
        raise F.ProbeRefusal('request-outside-run-root')
    result_path = F.guard(args.result, run_root)
    request = json.loads(request_path.read_text(encoding='utf-8'))
    fs = WindowsFS(run_root, request['directory_access'])
    dest = fs._path(request['dest'])

    def barrier(name):
        if name == request.get('barrier'):
            print('BARRIER ' + name, flush=True)
            sys.stdin.read()

    try:
        if request['kind'] == 'publish':
            gen = request['generation']
            record = publish_generation(fs, dest, gen, _probe_files(gen), barrier)
        elif request['kind'] == 'recover':
            record = recover(fs, dest)
        else:
            raise F.ProbeRefusal('unknown-request')
    except Exception as failure:
        record = dict(passed=False, **short_error(failure))
    fs.create_file(result_path, _json_bytes(record))
    return 0


def _request(fs, dest, kind, **extra):
    return dict(kind=kind, dest=str(fs._path(dest)), directory_access=fs.directory_access, **extra)


def _fresh(fs, case, phase, request):
    work = fs._path(case / phase)
    fs.mkdir(work)
    for name in ('request.json', 'result.json', 'child.log'):
        fs._path(work / name)
    result = fs.F.run_request(Path(__file__), request, work, timeout=30)
    if result.get('harness_error'):
        raise fs.F.FixtureFailure('child-request-failed')
    if 'error' in result:
        failure = fs.F.FixtureFailure('child-request-error')
        failure.winerror = result.get('winerror')
        raise failure
    return result


def _wait_barrier(worker, barrier, timeout):
    lines = []
    reader = threading.Thread(target=lambda: lines.append(worker.process.stdout.readline()), daemon=True)
    reader.start()
    reader.join(timeout)
    if not lines:
        raise TimeoutError('worker-barrier-timeout')
    if lines[0].decode('utf-8', 'replace').strip() != 'BARRIER ' + barrier:
        raise RuntimeError('worker-barrier-missing')


def interrupt(run_root):
    qualification = qualify(run_root)
    fs = WindowsFS(run_root)
    if not qualification['passed']:
        print(json.dumps(dict(passed=False, qualification=qualification, error='FixtureFailure', winerror=None)),
              flush=True)
        return False
    access_name = qualification['directory_access']
    fs.directory_access = fs.W.FILE_ADD_FILE | fs.W.FILE_ADD_SUBDIRECTORY if access_name == DIRECTORY_ACCESS[0] \
        else getattr(fs.W, access_name)
    base = fs._path(fs.run_root / 'interruptions')
    fs.mkdir(base)
    all_passed = True
    for start in ('no-previous', 'previous-A'):
        previous = 'A' if start == 'previous-A' else None
        for barrier in BARRIERS:
            record = dict(barrier=barrier, start=start, recovered=None, valid=False, retry_status=None,
                          passed=False, evidence='process-interruption', directory_access=access_name)
            try:
                case = fs._path(base / (start + '-' + barrier))
                fs.mkdir(case)
                dest = fs._path(case / 'dest')
                if previous is not None:
                    seed = _fresh(fs, case, 'seed', _request(fs, dest, 'publish', generation='A'))
                    if seed.get('status') != 'published':
                        raise fs.F.FixtureFailure('previous-generation-failed')
                work = fs._path(case / 'interrupted')
                fs.mkdir(work)
                request_path, result_path = work / 'request.json', fs._path(work / 'result.json')
                request = _request(fs, dest, 'publish', generation='B', barrier=barrier)
                fs.create_file(request_path, _json_bytes(request))
                log = fs._path(fs.F.worker_log(fs.run_root, 'p2'))
                worker = fs.F.Worker([Path(__file__).resolve(), 'child', fs._path(request_path), result_path], log)
                try:
                    worker.handshake(30)
                    _wait_barrier(worker, barrier, 30)
                    worker.kill()
                finally:
                    worker.close()
                recovered = _fresh(fs, case, 'recovery', _request(fs, dest, 'recover'))
                record.update(recovered=recovered['active'], valid=recovered['valid'])
                retry = _fresh(fs, case, 'retry', _request(fs, dest, 'publish', generation='B2'))
                record['retry_status'] = retry.get('status')
                final = _fresh(fs, case, 'retry-recovery', _request(fs, dest, 'recover'))
                record['passed'] = recovered['valid'] and recovered['active'] in (previous, 'B') \
                    and retry.get('status') == 'published' and final['valid'] and final['active'] == 'B2'
            except Exception as failure:
                record.update(short_error(failure))
            all_passed = all_passed and record['passed']
            print(json.dumps(record), flush=True)
    return all_passed


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='command', required=True)
    for command in ('qualify', 'interrupt'):
        sub.add_parser(command).add_argument('run_root')
    child = sub.add_parser('child')
    child.add_argument('request')
    child.add_argument('result')
    args = parser.parse_args(argv)
    try:
        if args.command == 'child':
            return _child(args)
        F, _ = _windows_modules()
        run_root = F.create_run_root(args.run_root)
        if args.command == 'qualify':
            result = qualify(run_root)
            print(json.dumps(result), flush=True)
            return 0 if result['passed'] else 1
        return 0 if interrupt(run_root) else 1
    except Exception as failure:
        print(json.dumps(dict(passed=False, **short_error(failure))), flush=True)
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
