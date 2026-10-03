"""Probe P3 (lock compatibility) and fixture F20: the importer's read-only lock against the real 0.4 engine.

0.4 is started only through `EngineProcess` (engine_process.py, read-only use): `server.py
--data <fresh case dir> --port 0 --no-browser`, authenticated readiness, owned shutdown.

- P3a: 0.4 running. The importer's lock is refused; engine.lock is unchanged (metadata,
  bytes 1..N, which 0.4 does not lock) and the watch reports nothing on it.
- P3b: the importer holds the lock. The 0.4 start is refused with its own message; it has
  appended exactly one b'0' to engine.lock (expected 0.4 behaviour, session_lock.py:6); the
  importer's decision rejects the import.
- P3c: the importer alone: lock, digests, release with the watch armed: nothing changes.
- F20: P3b inside the stage 2 to 5 pipeline at barriers B1, B2 and B3.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import secrets
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import fixtures as F  # noqa: E402
import platform_ops as P  # noqa: E402
import pipeline  # noqa: E402
from engine_process import EngineProcess  # noqa: E402  (read-only use)

REFUSAL_TEXT = 'This session is already open in another C600 Studio'
F20_BARRIERS = ('B1', 'B2', 'B3')


def start_04(data: Path, run_root: Path, timeout: float = 180) -> tuple:
    """Start 0.4 on `data`. Returns (engine or None, record); the caller closes a started engine."""
    data = F.guard(data, run_root)
    token = secrets.token_hex(4)
    logs = F.guard(Path(run_root) / 'logs', run_root)
    logs.mkdir(exist_ok=True)
    log_file = logs / ('engine-%s.log' % token)
    engine = EngineProcess(F.ROOT, data, logs / ('launch-%s.json' % token), log_file, timeout=timeout,
                           hidden_console=True)
    try:
        engine.start()
        return engine, dict(started=True, refused_with_own_message=False)
    except Exception:  # EngineProcess.start raises once the engine exits before readiness
        text = log_file.read_text(encoding='utf-8', errors='replace') if log_file.exists() else ''
        code = engine.process.returncode if engine.process is not None else None
        return None, dict(started=False, refused_with_own_message=REFUSAL_TEXT in text, exit_code=code)


def _sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def p3a_attempt(source: Path) -> dict:
    """Runs while 0.4 holds the session. Reads engine.lock only from byte 1 (byte 0 is 0.4's lock)."""
    import winapi as W
    path = source / 'engine.lock'
    # While 0.4 holds the handle it wrote through, the first open by anyone publishes 0.4's pending
    # directory-entry update as one 'modified' notification (bytes and metadata unchanged). Observed
    # on this machine: later opens and lock attempts raise none. Settle it in its own window and record it.
    settle = P.DirectoryWatch(source)
    settle.start()
    W.open_metadata(path).close()
    settled = settle.stop_and_drain()
    watch = P.DirectoryWatch(source)
    watch.start()
    before = P.snapshot(source)['engine.lock']
    with W.open_handle(path, W.GENERIC_READ, W.FILE_SHARE_READ | W.FILE_SHARE_WRITE, W.OPEN_EXISTING) as handle:
        tail_before = W.read_all(handle, start=1)
    refused = None
    try:
        P.ReadOnlyLock(source).release()
    except P.LockRefused as error:
        refused = error.reason
    with W.open_handle(path, W.GENERIC_READ, W.FILE_SHARE_READ | W.FILE_SHARE_WRITE, W.OPEN_EXISTING) as handle:
        tail_after = W.read_all(handle, start=1)
    after = P.snapshot(source)['engine.lock']
    result = watch.stop_and_drain()
    lock_events = [action for _, action, name in result.events if name == 'engine.lock']
    changed = sorted(key for key in before if before[key] != after[key])
    return dict(settle_events=[(action, name) for _, action, name in settled.events],
                refused=refused, engine_lock_size=before['size'], metadata_changes=changed,
                tail_unchanged=tail_before == tail_after, engine_lock_events=lock_events,
                other_events=len(result.events) - len(lock_events), overflows=result.overflows,
                watch_errors=result.errors,
                passed=(refused == 'engine-running' and not changed and tail_before == tail_after
                        and not lock_events and not result.overflows and not result.errors))


def p3b_attempt(source: Path, run_root: Path) -> dict:
    watch = P.DirectoryWatch(source)
    watch.start()
    pre = P.snapshot(source)
    lock = P.ReadOnlyLock(source, guard_entries=pre)
    try:
        lock_digests = P.digests(source, lock, pre)
        bytes_before = lock.read_bytes()
        engine, start = start_04(source, run_root)
        if engine is not None:
            engine.close()
        bytes_after = lock.read_bytes()
        final = P.snapshot(source)
        final_digests = P.digests(source, lock, final)
    finally:
        lock.release()
    result = watch.stop_and_drain()
    decision = P.decide(result, pre, final, lock_digests, final_digests)
    appended = bytes_after[len(bytes_before):] if bytes_after.startswith(bytes_before) else None
    lock_events = sorted({action for _, action, name in result.events if name == 'engine.lock'})
    return dict(start_04=start, engine_lock_size_before=len(bytes_before), engine_lock_size_after=len(bytes_after),
                appended_by_04=appended.decode('ascii', 'replace') if appended is not None else None,
                engine_lock_events=lock_events, accepted=decision.accepted, reasons=decision.reasons,
                snapshot_changes=decision.snapshot_changes, digest_changes=decision.digest_changes,
                passed=(not start['started'] and start['refused_with_own_message'] and start['exit_code'] not in (0, None)
                        and appended == b'0' and 'modified' in lock_events and not decision.accepted))


def p3c_attempt(source: Path) -> dict:
    watch = P.DirectoryWatch(source)
    watch.start()
    pre = P.snapshot(source)
    lock = P.ReadOnlyLock(source, guard_entries=pre)
    try:
        lock_digests = P.digests(source, lock, pre)
        final = P.snapshot(source)
        final_digests = P.digests(source, lock, final)
    finally:
        lock.release()
    released = P.snapshot(source)
    result = watch.stop_and_drain()
    decision = P.decide(result, pre, final, lock_digests, final_digests)
    after_release = P.diff_snapshots(pre, released)
    return dict(accepted=decision.accepted, reasons=decision.reasons, event_count=len(result.events),
                snapshot_changes_after_release=after_release,
                passed=decision.accepted and not after_release)


def f20_hook(run_root: Path):
    def hook(context) -> None:
        engine, start = start_04(context['source'], run_root)
        if engine is not None:
            engine.close()
        context['notes']['injected'] = 'external-0.4-start'
        context['notes']['start_04'] = start
    return hook


def f20_verdict(record: dict) -> bool:
    start = record.get('notes', {}).get('start_04', {})
    return (not record.get('harness_error') and not start.get('started', True)
            and start.get('refused_with_own_message') is True and not record['accepted']
            and not record['published'] and record['destination_unchanged']
            and ('modified', 'engine.lock') in set(map(tuple, record['events'])))


def _child(args) -> int:
    request, run_root, result = F.load_request(args.request, args.result)
    source = F.guard(request['source'], run_root)
    kind = request['kind']
    if kind == 'p3a':
        record = p3a_attempt(source)
    elif kind == 'p3b':
        record = p3b_attempt(source, run_root)
    elif kind == 'p3c':
        record = p3c_attempt(source)
    elif kind == 'f20':
        dest = F.guard(request['dest'], run_root)
        temp = F.guard(request['temp'], run_root)
        record = pipeline.run_import(source, dest, temp, pipeline.null_copier,
                                     {request['barrier']: f20_hook(run_root)})
    else:
        raise F.ProbeRefusal('unknown-request')
    F.write_result(result, record)
    return 0


def run(run_root: Path) -> dict:
    base = F.guard(run_root / 'p3', run_root)
    results = {}

    def child(name: str, request: dict) -> dict:
        work = F.guard(base / 'work' / name, run_root)
        work.mkdir(parents=True)
        return F.run_request(Path(__file__), dict(run_root=str(run_root), **request), work, timeout=600)

    # P3a: 0.4 owns the session through EngineProcess while the importer tries the lock.
    source = base / 'sources' / 'p3a'
    F.build_sessions('clean', [source], run_root)
    engine, start = start_04(source, run_root)
    try:
        record = child('p3a', dict(kind='p3a', source=str(source))) if engine is not None \
            else dict(harness_error=True, error='0.4-did-not-start', passed=False)
    finally:
        if engine is not None:
            engine.close()
    record['start_04'] = start
    record['stop_04'] = {key: value for key, value in (engine.cleanup_result or {}).items()
                         if key in ('graceful_request', 'forced', 'exit_code')} if engine is not None else None
    results['P3a'] = record

    for name in ('p3b', 'p3c'):
        source = base / 'sources' / name
        F.build_sessions('clean', [source], run_root)
        results[name.replace('p3', 'P3')] = child(name, dict(kind=name, source=str(source)))

    for barrier in F20_BARRIERS:
        name = 'F20-' + barrier
        source = base / 'sources' / name
        F.build_sessions('crash-noshm', [source], run_root)
        dest = F.guard(base / 'dest' / name, run_root)
        dest.mkdir(parents=True)
        record = child(name, dict(kind='f20', source=str(source), dest=str(dest), barrier=barrier,
                                  temp=str(base / 'importer-temp' / name)))
        record.update(barrier=barrier, passed=f20_verdict(record))
        results[name] = record
    return results


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='command', required=True)
    child = sub.add_parser('child')
    child.add_argument('request')
    child.add_argument('result')
    args = parser.parse_args(argv)
    return _child(args)


if __name__ == '__main__':
    raise SystemExit(main())
