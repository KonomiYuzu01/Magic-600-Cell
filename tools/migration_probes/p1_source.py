"""Probe P1 (source preservation): which open_source_for_backup strategy copies WAL-only commits
and never writes the source directory, not even transiently.

Each attempt uses one fresh 0.4 session and one fresh child process:
watch, pre-lock snapshot, read-only lock, digests, open, read meta.model, online backup into a
1.0-owned temporary directory (or an injected failure after the first step), close, final
snapshot and digests, release, drain, decide. Mode `deny` adds a deny-write ACE for the
current user on the source directory before the attempt and removes it afterwards; the
decision is the same in both modes.

Controls: a deliberate read-write open (must be reported), an undersized watch buffer
(overflow must abort), a mapped write-and-restore of an existing -shm (recorded either way:
it tells whether the observer can see mapped writes at all), an external 0.4 lock attempt
inside the copy window (must abort; the writer is known from the harness, not from timing),
and a fixture built through the forwarding venv launcher (the worker PID must be bound).
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import fixtures as F  # noqa: E402
import platform_ops as P  # noqa: E402
from engine_process import python_child_command  # noqa: E402  (fixtures put the repository root on the path)

# Runs 0.4's own SessionLock, read-only use of the 0.4 module, in a separate process: it appends
# one byte to engine.lock and is refused while the importer holds byte 0.
SESSION_LOCK_ATTEMPT = (
    'import sys; sys.path.insert(0, sys.argv[1]); from pathlib import Path; from session_lock import SessionLock\n'
    'try:\n    SessionLock(Path(sys.argv[2]))\nexcept RuntimeError:\n    sys.exit(3)\nsys.exit(0)\n')

MODES = ('watch', 'deny')
OUTCOMES = ('success', 'failure')
DENY_RIGHTS = '(OI)(CI)(WD,AD,WEA,WA,DE,DC)'


def short_error(error: BaseException) -> str:
    """Exception class and message without any path."""
    text = str(error).splitlines()[0] if str(error) else ''
    for marker in (':\\', ':/', '\\\\'):
        if marker in text:
            text = text.split(marker)[0].rsplit(' ', 1)[0] + ' <path>'
    return '%s: %s' % (type(error).__name__, text[:160])


def verify_copy(copy: Path, expected: dict) -> dict:
    connection = sqlite3.connect(copy)
    try:
        integrity = connection.execute('PRAGMA integrity_check').fetchone()[0]
        head = connection.execute("SELECT value FROM meta WHERE key='head'").fetchone()
        post = connection.execute('SELECT post FROM events WHERE id=?', (expected['head'],)).fetchone()
        events = connection.execute('SELECT count(*) FROM events').fetchone()[0]
    finally:
        connection.close()
    ok = integrity == 'ok' and head is not None and int(head[0]) == expected['head'] \
        and post is not None and post[0] == expected['state_hash']
    return dict(ok=ok, integrity=integrity, head_matches=head is not None and int(head[0]) == expected['head'],
                head_event_present=post is not None and post[0] == expected['state_hash'], events=events)


def external_lock_attempt(source: Path) -> int:
    """A separate process runs 0.4's SessionLock on the source; returns its exit code (3 = refused)."""
    command, env = python_child_command(['-c', SESSION_LOCK_ATTEMPT, str(F.ROOT), str(source)])
    return subprocess.run(command, env=env, capture_output=True, timeout=60).returncode


def attempt(source: Path, copy_dir: Path, strategy: str, outcome: str, expected: dict,
            external: bool = False) -> dict:
    """Runs inside a fresh child process. Returns a JSON-safe record.

    `external`: after the first backup step, another process runs 0.4's lock attempt (the
    MPR-06 control). The harness records that writer itself; the decision never names one.
    """
    database = source / 'session.sqlite3'
    record = dict(strategy=strategy, outcome=outcome)
    watch = P.DirectoryWatch(source)
    watch.start()
    pre = P.snapshot(source)
    try:
        lock = P.ReadOnlyLock(source, guard_entries=pre)  # MPR-04: prevention in the importer's own path
    except P.LockRefused as refused:
        watch.stop_and_drain()
        record.update(error='refused: ' + refused.reason, accepted=False, preserved=False, reached=False,
                      passed=False, inconclusive=False)
        return record
    lock_digests = P.digests(source, lock, pre)
    copy = copy_dir / 'copy.sqlite3'
    copy_dir.mkdir(parents=True)
    connection, error, stalled = None, None, False
    try:
        connection = P.open_source_for_backup(database, strategy, staging=copy_dir / 'staged')
        copied = getattr(connection, 'copied', None)
        if copied is not None:
            record['staged'] = dict(files=sorted(copied),
                                    matches_lock_digests=all(lock_digests.get(name) == digest
                                                             for name, digest in copied.items()))
        record['model_matches'] = connection.model() == expected['model']
        hook = None
        if external:
            def hook():
                record['injected'] = dict(writer='external-0.4-session-lock', exit=external_lock_attempt(source))
        connection.backup(copy, fail_after_first_step=(outcome == 'failure'), after_first_step=hook)
    except P.InjectedFailure:
        error = 'injected'
    except (sqlite3.Error, OSError, P.BackupStalled) as failure:
        error = short_error(failure)
        stalled = isinstance(failure, P.BackupStalled)
    finally:
        if connection is not None:
            record['backup_statuses'] = sorted(set(getattr(connection, 'statuses', [])))
            try:
                connection.close()
            except sqlite3.Error as failure:
                error = error or short_error(failure)
    if error is not None and copy.exists():
        copy.unlink()  # failure cleanup in the 1.0-owned temporary directory, never in the source
    final = P.snapshot(source)
    final_digests = P.digests(source, lock, final)
    lock.release()
    result = watch.stop_and_drain()
    decision = P.decide(result, pre, final, lock_digests, final_digests)
    record.update(error=error, opened=connection is not None,
                  events=[(action, name) for _, action, name in result.events],
                  overflows=result.overflows, watch_errors=result.errors,
                  accepted=decision.accepted, reasons=decision.reasons,
                  snapshot_changes=decision.snapshot_changes, digest_changes=decision.digest_changes)
    if outcome == 'success' and error is None:
        record['copy'] = verify_copy(copy, expected)
    record['preserved'] = decision.accepted
    # C3: the staged bytes must equal the bytes under the lock, whatever the outcome (MPR-A04).
    staged_ok = record.get('staged', {}).get('matches_lock_digests', True)
    if outcome == 'success':
        record['reached'] = error is None and record.get('copy', {}).get('ok', False) and staged_ok
    else:
        record['reached'] = error == 'injected' and staged_ok
    record['passed'] = record['preserved'] and record['reached']
    # MPR-07: a resource bound with no observed change says nothing about preservation.
    record['inconclusive'] = stalled and decision.accepted
    return record


def negative_control(source: Path) -> dict:
    """A deliberate default read-write open that creates and removes -wal and -shm must be reported."""
    watch = P.DirectoryWatch(source)
    watch.start()
    pre = P.snapshot(source)
    lock = P.ReadOnlyLock(source)
    lock_digests = P.digests(source, lock, pre)
    connection = sqlite3.connect(source / 'session.sqlite3')
    seen = connection.execute('SELECT count(*) FROM events').fetchone()[0]
    sidecars_open = sorted(item.name for item in source.iterdir() if item.name.startswith('session.sqlite3-'))
    connection.close()
    sidecars_after = sorted(item.name for item in source.iterdir() if item.name.startswith('session.sqlite3-'))
    final = P.snapshot(source)
    final_digests = P.digests(source, lock, final)
    lock.release()
    result = watch.stop_and_drain()
    decision = P.decide(result, pre, final, lock_digests, final_digests)
    shm_events = sorted({action for _, action, name in result.events if name == 'session.sqlite3-shm'})
    return dict(events_seen=seen, sidecars_while_open=sidecars_open, sidecars_after_close=sidecars_after,
                shm_events=shm_events, event_count=len(result.events), accepted=decision.accepted,
                reasons=decision.reasons, snapshot_changes=decision.snapshot_changes,
                passed=(not decision.accepted and 'event' in decision.reasons
                        and {'added', 'removed'} <= set(shm_events)))


def overflow_control(source: Path, buffer_size: int = 64, burst: int = 300) -> dict:
    """An undersized buffer plus a burst; timestamps restored so that snapshots match; overflow must abort."""
    import winapi as W
    watch = P.DirectoryWatch(source, buffer_size=buffer_size)
    watch.start()
    pre = P.snapshot(source)
    lock = P.ReadOnlyLock(source)
    lock_digests = P.digests(source, lock, pre)
    with W.open_handle(source, W.FILE_READ_ATTRIBUTES | W.FILE_WRITE_ATTRIBUTES, W.FILE_SHARE_ALL, W.OPEN_EXISTING,
                       W.FILE_FLAG_BACKUP_SEMANTICS) as handle:
        times = W.basic_info(handle)
        # Names longer than the buffer, so that every record is discarded and only the overflow remains.
        for index in range(burst):
            path = source / ('overflow-control-burst-file-%06d.tmp' % index)
            path.write_bytes(b'x')
            path.unlink()
        W.set_basic_info(handle, times)
    final = P.snapshot(source)
    final_digests = P.digests(source, lock, final)
    lock.release()
    result = watch.stop_and_drain()
    decision = P.decide(result, pre, final, lock_digests, final_digests)
    return dict(buffer_size=buffer_size, burst=burst, overflows=result.overflows,
                overflow_forms=sorted(set(result.overflow_forms)), event_count=len(result.events),
                snapshots_equal=not decision.snapshot_changes, snapshot_changes=decision.snapshot_changes,
                accepted=decision.accepted, reasons=decision.reasons,
                passed=(not decision.accepted and 'overflow' in decision.reasons))


def mapped_write_control(source: Path, guard: bool = False, offset: int = 4096) -> dict:
    """MPR-04: a conditional copier double. If it can open the existing -shm for writing, it changes and
    restores one byte through a writable mapping, unflushed, as a SQLite WAL index would; otherwise it
    records the refusal and carries on. Unguarded, this shows what the observer can miss; with the
    importer's WriteGuard held, the writable open must be refused."""
    import mmap
    shm = source / 'session.sqlite3-shm'
    watch = P.DirectoryWatch(source)
    watch.start()
    pre = P.snapshot(source)
    lock = P.ReadOnlyLock(source, guard_entries=pre if guard else None)
    lock_digests = P.digests(source, lock, pre)
    record = dict(guarded=guard)
    try:
        with open(shm, 'r+b') as stream:
            view = mmap.mmap(stream.fileno(), 0)
            original = view[offset]
            view[offset] = original ^ 0xFF
            record['byte_changed_in_view'] = view[offset] != original
            view[offset] = original
            view.close()
        record['writable_open'] = 'allowed'
    except PermissionError:
        record['writable_open'] = 'refused'
    final = P.snapshot(source)
    final_digests = P.digests(source, lock, final)
    lock.release()
    result = watch.stop_and_drain()
    decision = P.decide(result, pre, final, lock_digests, final_digests)
    record.update(event_count=len(result.events), events=[(action, name) for _, action, name in result.events],
                  snapshot_changes=decision.snapshot_changes, digest_changes=decision.digest_changes,
                  accepted=decision.accepted, reasons=decision.reasons,
                  write_happened_undetected=record['writable_open'] == 'allowed' and decision.accepted)
    return record


def mapped_write_prevention(source: Path) -> dict:
    """With the deny ACE in place, a writable open of the existing -shm (needed for any writable mapping) must fail."""
    try:
        with open(source / 'session.sqlite3-shm', 'r+b'):
            return dict(prevented=False)
    except PermissionError:
        return dict(prevented=True)


def _child(args) -> int:
    request, run_root, result = F.load_request(args.request, args.result)
    source = F.guard(request['source'], run_root)
    if request['kind'] == 'attempt':
        copy_dir = F.guard(request['copy_dir'], run_root)
        record = attempt(source, copy_dir, request['strategy'], request['outcome'], request['expected'],
                         external=request.get('external', False))
    elif request['kind'] == 'negative-control':
        record = negative_control(source)
    elif request['kind'] == 'mapped-write-control':
        record = mapped_write_control(source)
    elif request['kind'] == 'mapped-write-guarded':
        record = mapped_write_control(source, guard=True)
    elif request['kind'] == 'mapped-write-prevention':
        record = mapped_write_prevention(source)
    elif request['kind'] == 'overflow-control':
        record = overflow_control(source)
    else:
        raise F.ProbeRefusal('unknown-request')
    F.write_result(result, record)
    return 0


def run_child(request: dict, work: Path, timeout: float = 300) -> dict:
    """Run one attempt or control in a fresh engine-environment process."""
    return F.run_request(Path(__file__), request, work, timeout)


def apply_deny(directory: Path, sid: str) -> dict:
    """Deny-write ACE for the current user on the source directory and its existing children; self-tested."""
    done = subprocess.run(['icacls', str(directory), '/deny', '*%s:%s' % (sid, DENY_RIGHTS)],
                          capture_output=True, text=True)
    if done.returncode != 0:
        return dict(applied=False, effective=False)
    checks = {}
    try:
        with open(directory / 'session.sqlite3', 'r+b'):
            checks['existing-file-write-open'] = 'allowed'
    except PermissionError:
        checks['existing-file-write-open'] = 'denied'
    probe = directory / 'deny-self-test.tmp'
    try:
        with open(probe, 'xb'):
            checks['create-file'] = 'allowed'
        probe.unlink()  # the ACE did not work; the attempt is skipped as a harness error
    except PermissionError:
        checks['create-file'] = 'denied'
    return dict(applied=True, effective=all(value == 'denied' for value in checks.values()), checks=checks)


def remove_deny(directory: Path, sid: str) -> bool:
    done = subprocess.run(['icacls', str(directory), '/remove:d', '*%s' % sid], capture_output=True, text=True)
    return done.returncode == 0


def strategy_verdict(mine: list) -> dict:
    """pass: every attempt passed. fail: some attempt failed for a reason other than a resource bound
    (an observed change, a wrong copy, a SQLite error, a harness error). inconclusive: otherwise."""
    failed = [item for item in mine if not item.get('passed') and not item.get('inconclusive')]
    verdict = 'pass' if all(item.get('passed') for item in mine) and mine else 'fail' if failed else 'inconclusive'
    return dict(verdict=verdict, attempts=len(mine), passed_attempts=sum(bool(item.get('passed')) for item in mine),
                inconclusive_attempts=sum(bool(item.get('inconclusive')) for item in mine))


def run(run_root: Path, strategies=None) -> dict:
    """The whole P1 matrix plus every control. Returns the raw (unsanitized) result."""
    import winapi as W
    strategies = list(strategies or P.STRATEGIES)
    sid = W.current_user_sid()
    base = F.guard(run_root / 'p1', run_root)
    plan = [(case, strategy, mode, outcome) for case in F.CASES for strategy in strategies
            for mode in MODES for outcome in OUTCOMES]
    infos = {}
    for case in F.CASES:
        names = ['%s-%s-%s-%s' % (case, s, m, o) for c, s, m, o in plan if c == case]
        built = F.build_sessions(case, [base / 'sources' / name for name in names], run_root)
        infos.update(zip(names, built))
    attempts = []
    for case, strategy, mode, outcome in plan:
        name = '%s-%s-%s-%s' % (case, strategy, mode, outcome)
        source = base / 'sources' / name
        work = F.guard(base / 'work' / name, run_root)
        work.mkdir(parents=True)
        deny = None
        if mode == 'deny':
            deny = apply_deny(source, sid)
        try:
            if mode == 'deny' and not deny['effective']:
                record = dict(harness_error=True, error='deny-ace-not-effective', passed=False)
            else:
                info = infos[name]
                record = run_child(dict(kind='attempt', run_root=str(run_root), source=str(source),
                                        copy_dir=str(base / 'importer-temp' / name), strategy=strategy,
                                        outcome=outcome, expected=dict(head=info['head'], state_hash=info['state_hash'],
                                                                       model=info['model'])), work)
        finally:
            if mode == 'deny':
                deny['removed'] = remove_deny(source, sid)
        record.update(case=case, strategy=strategy, mode=mode, outcome=outcome, deny=deny)
        attempts.append(record)
    controls = run_controls(run_root, base, sid)
    verdicts = {strategy: strategy_verdict([item for item in attempts if item['strategy'] == strategy])
                for strategy in strategies}
    chosen = next((strategy for strategy in strategies if verdicts[strategy]['verdict'] == 'pass'), None)
    return dict(strategies={key: P.STRATEGIES[key]['text'] for key in strategies}, attempts=attempts,
                controls=controls, verdicts=verdicts, chosen=chosen)


def run_controls(run_root: Path, base: Path, sid: str) -> dict:
    """Every control on its own fresh fixture. Pass rules are fixed here, before the run."""
    controls = {}
    sources = {name: F.guard(base / 'controls' / name, run_root)
               for name in ('negative', 'overflow', 'mapped', 'mapped-guarded', 'mapped-deny', 'external',
                            'forwarding')}
    F.build_sessions('clean', [sources['negative'], sources['overflow']], run_root)
    F.build_sessions('crash', [sources['mapped'], sources['mapped-guarded'], sources['mapped-deny']], run_root)
    for name, kind in (('negative', 'negative-control'), ('overflow', 'overflow-control'),
                       ('mapped', 'mapped-write-control'), ('mapped-guarded', 'mapped-write-guarded')):
        work = F.guard(base / 'work' / ('control-' + name), run_root)
        work.mkdir(parents=True)
        controls[kind] = run_child(dict(kind=kind, run_root=str(run_root), source=str(sources[name])), work)
    # Recorded, not pass/fail: whether the observer alone sees a mapped write at all.
    controls['mapped-write-control']['passed'] = None
    guarded = controls['mapped-write-guarded']
    guarded['passed'] = guarded.get('writable_open') == 'refused' and guarded.get('accepted') is True

    work = F.guard(base / 'work' / 'control-mapped-deny', run_root)
    work.mkdir(parents=True)
    deny = apply_deny(sources['mapped-deny'], sid)
    try:
        record = (run_child(dict(kind='mapped-write-prevention', run_root=str(run_root),
                                 source=str(sources['mapped-deny'])), work)
                  if deny['effective'] else dict(harness_error=True, error='deny-ace-not-effective'))
    finally:
        deny['removed'] = remove_deny(sources['mapped-deny'], sid)
    record.update(deny=deny, passed=bool(record.get('prevented')) and deny['removed'])
    controls['mapped-write-prevention'] = record

    controls['external-append-control'] = external_control(run_root, base, sources['external'], 'C3')

    # MPR-05: build a crash fixture through the venv launcher and require the bound worker to be terminated.
    info, = F.build_sessions('crash', [sources['forwarding']], run_root, via_launcher=True)
    launcher = os.path.normcase(sys.executable) != os.path.normcase(getattr(sys, '_base_executable', sys.executable))
    worker = info['worker']
    controls['forwarding-control'] = dict(venv_launcher=launcher, **worker,
                                          passed=worker.get('worker_terminated') is True
                                          and (worker['forwarding'] or not launcher))
    return controls


def external_control(run_root: Path, base: Path, source: Path, strategy: str) -> dict:
    """MPR-06: an external 0.4 lock attempt inside the copy window, on a crash source with a strategy
    that otherwise preserves it, so the external append is the only change expected. The run uses C3,
    the owner's choice of 2026-10-03; the first run used C1b."""
    info, = F.build_sessions('crash', [source], run_root)
    work = F.guard(base / 'work' / 'control-external', run_root)
    work.mkdir(parents=True)
    record = run_child(dict(kind='attempt', run_root=str(run_root), source=str(source),
                            copy_dir=str(base / 'importer-temp' / 'control-external'), strategy=strategy,
                            outcome='success', external=True,
                            expected=dict(head=info['head'], state_hash=info['state_hash'], model=info['model'])),
                       work)
    record['passed'] = (record.get('injected', {}).get('exit') == 3 and record.get('accepted') is False
                        and 'event' in record.get('reasons', []) and 'engine.lock' in record.get('digest_changes', []))
    return record


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
