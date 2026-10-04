"""Fresh synthetic 0.4 sessions for the migration probes, built by the 0.4 Session in child processes.

Every directory lives inside a run root that this module creates fresh and marks.
Nothing here accepts a path outside that run root, and the B4-12 guard also refuses
the 0.4 and C600 Studio default data directories.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import secrets
import subprocess
import sys
import tempfile
import threading
import time

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools' / 'perf'))
from b412_fixture import FixtureRefusal, guard_path  # noqa: E402
sys.path.insert(1, str(ROOT))
from engine_process import python_child_command  # noqa: E402  (read-only use of the 0.4 launch convention)

MARKER = '.migration-probe-run'
WORD = [1, 601]
CASES = ('clean', 'crash', 'crash-noshm')
EXPECTED_FILES = {
    'clean': ['engine.lock', 'session.sqlite3'],
    'crash': ['engine.lock', 'session.sqlite3', 'session.sqlite3-shm', 'session.sqlite3-wal'],
    'crash-noshm': ['engine.lock', 'session.sqlite3', 'session.sqlite3-wal'],
}


class ProbeRefusal(FixtureRefusal):
    """A path or input the probes refuse; messages are short and public."""


class FixtureFailure(RuntimeError):
    """A synthetic session did not reach its expected state."""


def allowed_bases() -> list:
    return [(ROOT / 'work' / 'migration-probes' / 'runs').resolve(), Path(tempfile.gettempdir()).resolve()]


def create_run_root(path) -> Path:
    path = guard_path(Path(path))
    if not any(path != base and path.is_relative_to(base) for base in allowed_bases()):
        raise ProbeRefusal('run-root-outside-allowed-bases')
    if path.exists():
        raise ProbeRefusal('run-root-exists')
    path.mkdir(parents=True)
    (path / MARKER).write_text(json.dumps({'run': path.name, 'token': secrets.token_hex(8)}), encoding='utf-8')
    return path


def guard(path, run_root) -> Path:
    """Resolve a probe path; refuse anything outside a marked run root or behind a link or junction."""
    run_root = guard_path(Path(run_root))
    if not (run_root / MARKER).is_file():
        raise ProbeRefusal('run-root-unmarked')
    path = guard_path(Path(path))
    if path == run_root or not path.is_relative_to(run_root):
        raise ProbeRefusal('outside-run-root')
    current = run_root
    for part in path.relative_to(run_root).parts:
        current = current / part
        if current.is_symlink() or (hasattr(os.path, 'isjunction') and os.path.isjunction(current)):
            raise ProbeRefusal('link-in-run-root')
    return path


def _sha256(path: Path) -> str:
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def _child(case: str, run_root, directories: list) -> None:
    """Runs in the engine environment. Clean sessions close; crash sessions stay open until the parent kills us.
    Every directory is checked again here, before any 0.4 Session is constructed: inside the marked
    run root and not yet existing (review finding MPR-A02)."""
    directories = [guard(directory, run_root) for directory in directories]
    if any(directory.exists() for directory in directories):
        raise ProbeRefusal('fixture-exists')
    sys.path.insert(0, str(ROOT))
    from core import Model
    from session import Session

    model = Model()
    recipe = [{'kind': 'word', 'moves': WORD}]
    sessions, infos = [], []
    for directory in directories:
        directory = Path(directory)
        session = Session(model, directory)
        sessions.append(session)
        session.commit(session.preview(recipe, note='migration probe fixture')['token'])
        info = {'model': model.model_id}
        if case == 'clean':
            session.commit(session.preview(recipe, note='migration probe fixture')['token'])
        else:
            busy, _, _ = session.db.execute('PRAGMA wal_checkpoint(TRUNCATE)').fetchone()
            database = directory / 'session.sqlite3'
            before = _sha256(database)
            session.commit(session.preview(recipe, note='migration probe fixture')['token'])
            info.update(checkpoint_busy=busy, main_sha_before_last_commit=before,
                        main_sha_after_last_commit=_sha256(database),
                        wal_bytes=(directory / 'session.sqlite3-wal').stat().st_size)
        status = session.status()
        info.update(head=int(status['head']), state_hash=status['state_hash'])
        infos.append(info)
        if case == 'clean':
            session.close()
    print('READY ' + json.dumps(dict(pid=os.getpid(), infos=infos)), flush=True)
    # Clean: the parent closes stdin. Crash: the parent terminates this process (bound by PID) first.
    sys.stdin.read()


def _unlink_when_released(path: Path, timeout: float = 10.0) -> None:
    """Fixture setup only: a killed process's mapping (or a scanner) can hold the file for a moment."""
    deadline = time.monotonic() + timeout
    while True:
        try:
            path.unlink()
            return
        except PermissionError:
            if time.monotonic() > deadline:
                raise FixtureFailure('sidecar-still-open') from None
            time.sleep(0.05)


def wait_released(directory: Path, timeout: float = 10.0) -> None:
    """Wait until no other handle is open on any file (a read-only open with no sharing succeeds)."""
    import winapi as W
    deadline = time.monotonic() + timeout
    for path in sorted(Path(directory).iterdir()):
        while True:
            try:
                W.open_handle(path, W.GENERIC_READ, 0, W.OPEN_EXISTING).close()
                break
            except OSError:
                if time.monotonic() > deadline:
                    raise FixtureFailure('fixture-file-still-open') from None
                time.sleep(0.05)


class Worker:
    """A child process bound to the PID it reports (MPR-05).

    The venv python.exe is a forwarding launcher: the PID Popen returns need not be the
    process that holds the session files. The child prints `READY {"pid": ...}` and then
    blocks on stdin; the parent opens a handle to that PID while the child is still
    blocked, so the handle names the worker itself and cannot be confused by PID reuse.
    """

    def __init__(self, script_args: list, log: Path, *, via_launcher: bool = False):
        if via_launcher:
            # The forwarding control: start through sys.executable (the venv launcher) on purpose.
            command, env = [sys.executable, '-X', 'utf8', '-u', *map(str, script_args)], os.environ.copy()
            env.update(PYTHONUTF8='1', PYTHONIOENCODING='utf-8', PYTHONUNBUFFERED='1')
        else:
            command, env = python_child_command([str(arg) for arg in script_args])
        self.log = Path(log).open('wb')
        self.process = subprocess.Popen(command, cwd=str(ROOT), env=env, stdin=subprocess.PIPE,
                                        stdout=subprocess.PIPE, stderr=self.log)
        self.launcher_pid = self.process.pid
        self.pid = None
        self.handle = None

    def handshake(self, timeout: float) -> dict:
        import winapi as W
        box = []
        reader = threading.Thread(target=lambda: box.append(self.process.stdout.readline()), daemon=True)
        reader.start()
        reader.join(timeout)
        line = box[0].decode('utf-8', 'replace') if box else ''
        if not line.startswith('READY '):
            self.close()
            raise FixtureFailure('worker-no-ready' if not box else 'worker-failed')
        payload = json.loads(line[len('READY '):])
        self.pid = int(payload['pid'])
        self.handle = W.open_process(self.pid)
        if W.exit_code(self.handle) != W.STILL_ACTIVE:
            self.close()
            raise FixtureFailure('worker-exited-before-binding')
        return payload

    @property
    def forwarding(self) -> bool:
        return self.pid is not None and self.pid != self.launcher_pid

    def finish(self, timeout: float) -> int:
        """Let a worker that waits on stdin exit by itself; confirm its exit through the bound handle."""
        import winapi as W
        self.process.stdin.close()
        if not W.wait_process(self.handle, int(timeout * 1000)):
            raise FixtureFailure('worker-did-not-exit')
        code = W.exit_code(self.handle)
        self.process.wait(timeout=timeout)
        return code

    def kill(self, timeout: float = 10.0) -> dict:
        """TerminateProcess on the worker itself, then confirm it has exited before anything is inspected."""
        import winapi as W
        W.terminate_process(self.handle)
        if not W.wait_process(self.handle, int(timeout * 1000)):
            raise FixtureFailure('worker-termination-unconfirmed')
        try:
            self.process.wait(timeout=timeout)
            launcher_exited = True
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait(timeout=timeout)
            launcher_exited = False
        return dict(worker_terminated=True, launcher_exited_by_itself=launcher_exited)

    def close(self) -> None:
        import winapi as W
        if self.handle is not None and W.exit_code(self.handle) == W.STILL_ACTIVE:
            W.terminate_process(self.handle)
            W.wait_process(self.handle, 10000)
        if self.process.poll() is None:
            self.process.kill()
            self.process.wait()
        for stream in (self.process.stdin, self.process.stdout):
            if stream and not stream.closed:
                stream.close()
        if self.handle is not None:
            self.handle.close()
        self.log.close()


def worker_log(run_root: Path, prefix: str) -> Path:
    """Unsanitized child stderr stays inside the run root (never committed)."""
    logs = guard(Path(run_root) / 'logs', run_root)
    logs.mkdir(exist_ok=True)
    return logs / ('%s-%s.log' % (prefix, secrets.token_hex(4)))


def build_sessions(case: str, directories: list, run_root, timeout: float = 300,
                   via_launcher: bool = False) -> list:
    """Build one fresh session per directory in one bound worker; returns the worker's info for each."""
    if case not in CASES:
        raise ProbeRefusal('invalid-case')
    run_root = Path(run_root)
    directories = [guard(directory, run_root) for directory in directories]
    for directory in directories:
        if directory.exists():
            raise ProbeRefusal('fixture-exists')
    child_case = 'clean' if case == 'clean' else 'crash'
    worker = Worker([Path(__file__).resolve(), 'child', child_case, run_root, *directories],
                    worker_log(run_root, 'fixture'), via_launcher=via_launcher)
    binding = {}
    try:
        infos = worker.handshake(timeout)['infos']
        if child_case == 'clean':
            if worker.finish(timeout) != 0:
                raise FixtureFailure('fixture-child-exit')
        else:
            binding = worker.kill()  # no close, no checkpoint; the kernel releases the locks
    finally:
        worker.close()
    for directory, info in zip(directories, infos):
        if case == 'crash-noshm':
            _unlink_when_released(directory / 'session.sqlite3-shm')
        wait_released(directory)
        names = sorted(item.name for item in directory.iterdir())
        if names != EXPECTED_FILES[case]:
            raise FixtureFailure('unexpected-files:' + ','.join(names))
        if (directory / 'engine.lock').stat().st_size < 1:
            raise FixtureFailure('engine-lock-empty')
        if case != 'clean':
            if info['checkpoint_busy'] != 0 or info['wal_bytes'] <= 32:
                raise FixtureFailure('wal-not-prepared')
            if info['main_sha_before_last_commit'] != info['main_sha_after_last_commit']:
                raise FixtureFailure('last-commit-not-wal-only')
        info['case'] = case
        info['files'] = names
        info['worker'] = dict(forwarding=worker.forwarding, **binding)
    return infos


def load_request(request_path, result_path) -> tuple:
    """Child side of run_request: the request and the result file must lie in the marked run root the
    request names, and the result must not exist yet (review finding MPR-A02). Returns
    (request, run_root, result path); the child still guards every path the request names."""
    request_path = guard_path(Path(request_path))
    request = json.loads(request_path.read_text(encoding='utf-8'))
    run_root = guard_path(Path(request['run_root']))
    guard(request_path, run_root)
    result_path = guard(result_path, run_root)
    if result_path.exists():
        raise ProbeRefusal('result-exists')
    return request, run_root, result_path


def write_result(result_path: Path, record: dict) -> None:
    with open(result_path, 'x', encoding='utf-8') as stream:
        stream.write(json.dumps(record, indent=1))


def run_request(script: Path, request: dict, work: Path, timeout: float = 300) -> dict:
    """Run one probe request in a fresh child of the engine environment (`<script> child <request> <result>`)."""
    request_path = Path(work) / 'request.json'
    result_path = Path(work) / 'result.json'
    request_path.write_text(json.dumps(request), encoding='utf-8')
    command, env = python_child_command([Path(script).resolve(), 'child', request_path, result_path])
    with (Path(work) / 'child.log').open('wb') as log:
        try:
            completed = subprocess.run(command, env=env, cwd=str(ROOT), stdout=log, stderr=subprocess.STDOUT,
                                       timeout=timeout)
        except subprocess.TimeoutExpired:
            return dict(harness_error=True, error='child-timeout', passed=False)
    if completed.returncode != 0 or not result_path.exists():
        lines = (Path(work) / 'child.log').read_text(encoding='utf-8', errors='replace').strip().splitlines()
        return dict(harness_error=True, returncode=completed.returncode, error=(lines[-1:] or [''])[0][:200],
                    passed=False)
    return json.loads(result_path.read_text(encoding='utf-8'))


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='command', required=True)
    child = sub.add_parser('child')
    child.add_argument('case', choices=('clean', 'crash'))
    child.add_argument('run_root')
    child.add_argument('directories', nargs='+')
    args = parser.parse_args(argv)
    if args.command == 'child':
        _child(args.case, args.run_root, args.directories)
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
