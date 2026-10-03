"""Probe harness for importer stages 2 to 5 (proposal section 3), with barrier hooks and a stub publication.

Stage 2 arms the watch and takes the pre-lock snapshot; stage 3 takes the read-only lock and
the lock-time digests; stage 4 runs a copier; stage 5 takes the final snapshot and digests,
releases the lock, drains the watch and decides. Only an accepted decision publishes.

Barriers: `B1` right after lock acquisition, `stage4` inside the copy, `B2` between stages 4
and 5, `B3` right after engine.lock is read in the final digests. A hook receives the context
(source, watch, lock) and records what it did itself: the decision never names a writer.

The stub publication stands in for P2's `publish_generation`; these fixtures check only that
nothing is published after a rejected decision.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import secrets
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import fixtures as F  # noqa: E402
import platform_ops as P  # noqa: E402

BARRIERS = ('B1', 'stage4', 'B2', 'B3')


def null_copier(source: Path, temp: Path, during) -> None:
    """Stage 4 that opens nothing in the source; `during` is the stage 4 barrier."""
    during()


def strategy_copier(strategy: str):
    def copier(source: Path, temp: Path, during) -> None:
        connection = P.open_source_for_backup(source / 'session.sqlite3', strategy)
        try:
            connection.backup(temp / 'copy.sqlite3', fail_after_first_step=False, after_first_step=during)
        finally:
            connection.close()
    return copier


COPIERS = {'null': lambda: null_copier, 'C1b': lambda: strategy_copier('C1b')}


def listing(directory: Path) -> list:
    """Names and sizes under a destination; enough to show that nothing was published."""
    if not directory.exists():
        return []
    return sorted((path.relative_to(directory).as_posix(), path.stat().st_size if path.is_file() else -1)
                  for path in directory.rglob('*'))


def stub_publish(dest: Path, temp: Path) -> str:
    generation = 'g-' + secrets.token_hex(4)
    target = dest / 'generations' / generation
    target.mkdir(parents=True)
    (target / 'IMPORTED').write_text(json.dumps(sorted(item.name for item in temp.iterdir())), encoding='utf-8')
    pointer = dest / 'ACTIVE.tmp'
    pointer.write_text(generation, encoding='utf-8')
    os.replace(pointer, dest / 'ACTIVE')
    return generation


def run_import(source: Path, dest: Path, temp: Path, copier, hooks: dict | None = None) -> dict:
    hooks = hooks or {}
    context = dict(source=source, notes={})

    def barrier(name: str) -> None:
        if name in hooks:
            hooks[name](context)

    dest_before = listing(dest)
    watch = P.DirectoryWatch(source)
    watch.start()
    context['watch'] = watch
    pre = P.snapshot(source)
    try:
        lock = P.ReadOnlyLock(source)
        lock.guard(pre)
    except P.LockRefused as refused:
        watch.stop_and_drain()
        return dict(stage='lock', refused=refused.reason, accepted=False, published=False,
                    destination_unchanged=listing(dest) == dest_before, notes=context['notes'])
    context['lock'] = lock
    error = None
    try:
        barrier('B1')
        lock_digests = P.digests(source, lock, pre)
        temp.mkdir(parents=True)
        try:
            copier(source, temp, lambda: barrier('stage4'))
        except Exception as failure:  # the copy's own failure fails the import; recorded without paths
            error = '%s: %s' % (type(failure).__name__, str(failure).splitlines()[0][:120] if str(failure) else '')
        barrier('B2')
        final = P.snapshot(source)
        final_digests = P.digests(source, lock, final, after_engine_lock=lambda: barrier('B3'))
    finally:
        lock.release()
    result = watch.stop_and_drain()
    decision = P.decide(result, pre, final, lock_digests, final_digests)
    generation = None
    if decision.accepted and error is None:
        generation = stub_publish(dest, temp)
    return dict(stage='decided', error=error, accepted=decision.accepted, reasons=decision.reasons,
                events=[(action, name) for _, action, name in result.events], overflows=result.overflows,
                overflow_forms=sorted(set(result.overflow_forms)), watch_errors=result.errors,
                snapshot_changes=decision.snapshot_changes, digest_changes=decision.digest_changes,
                published=generation is not None, destination_unchanged=listing(dest) == dest_before,
                notes=context['notes'])


# Test doubles. Each records itself in context['notes']; the decision does not know about them.

def f15_importer_bug(context) -> None:
    """F15: an importer bug double changes the source database's last-write time through its own handle."""
    import winapi as W
    with W.open_handle(context['source'] / 'session.sqlite3', W.FILE_READ_ATTRIBUTES | W.FILE_WRITE_ATTRIBUTES,
                       W.FILE_SHARE_ALL, W.OPEN_EXISTING, W.FILE_FLAG_OPEN_REPARSE_POINT) as handle:
        info = W.basic_info(handle)
        info.LastWriteTime += 10_000_000  # one second
        W.set_basic_info(handle, info)
    context['notes']['injected'] = 'importer-bug-double:last-write-time'


def f18_transient_sidecar(context) -> None:
    """F18: a test double creates and removes -shm in the source during stage 4."""
    path = context['source'] / 'session.sqlite3-shm'
    with open(path, 'xb'):
        pass
    path.unlink()
    context['notes']['injected'] = 'test-double:create-remove-shm'


def f21_zero(context) -> None:
    context['watch'].inject_zero_completion()
    context['notes']['injected'] = 'watch:zero-byte-completion'


def f21_error(context) -> None:
    context['watch'].inject_error()
    context['notes']['injected'] = 'watch:error-87'


CASES = {
    # name: (fixture state, copier, barrier, hook)
    'baseline-null': ('crash-noshm', 'null', None, None),
    'baseline-C1b': ('crash', 'C1b', None, None),
    'F15': ('crash-noshm', 'null', 'B2', f15_importer_bug),
    'F18': ('crash-noshm', 'null', 'stage4', f18_transient_sidecar),
    'F21-zero': ('crash-noshm', 'null', 'B2', f21_zero),
    'F21-error': ('crash-noshm', 'null', 'B2', f21_error),
}


def verdict(name: str, record: dict) -> bool:
    if record.get('harness_error'):
        return False
    if name.startswith('baseline'):
        return record['accepted'] and record['published']
    expected = {'F15': 'snapshot-diff', 'F18': 'event', 'F21-zero': 'overflow', 'F21-error': 'watch-error'}[name]
    ok = not record['accepted'] and not record['published'] and record['destination_unchanged'] \
        and expected in record['reasons']
    if name == 'F18':
        ok = ok and {('added', 'session.sqlite3-shm'), ('removed', 'session.sqlite3-shm')} <= set(map(tuple, record['events']))
    if name.startswith('F21'):
        ok = ok and not record['snapshot_changes'] and not record['digest_changes']
    return ok


def _child(args) -> int:
    request = json.loads(Path(args.request).read_text(encoding='utf-8'))
    run_root = Path(request['run_root'])
    source = F.guard(request['source'], run_root)
    dest = F.guard(request['dest'], run_root)
    temp = F.guard(request['temp'], run_root)
    state, copier, barrier, hook = CASES[request['case']]
    record = run_import(source, dest, temp, COPIERS[copier](), {barrier: hook} if barrier else None)
    Path(args.result).write_text(json.dumps(record, indent=1), encoding='utf-8')
    return 0


def run(run_root: Path) -> dict:
    base = F.guard(run_root / 'pipeline', run_root)
    results = {}
    for name, (state, copier, barrier, hook) in CASES.items():
        source = base / 'sources' / name
        F.build_sessions(state, [source], run_root)
        work = F.guard(base / 'work' / name, run_root)
        work.mkdir(parents=True)
        dest = F.guard(base / 'dest' / name, run_root)
        dest.mkdir(parents=True)
        record = F.run_request(Path(__file__), dict(case=name, run_root=str(run_root), source=str(source),
                                                    dest=str(dest), temp=str(base / 'importer-temp' / name)), work)
        record.update(fixture=state, copier=copier, barrier=barrier, passed=verdict(name, record))
        results[name] = record
    baseline_ok = results['baseline-null']['passed']
    for name in ('F15', 'F18', 'F21-zero', 'F21-error'):
        if not baseline_ok:
            results[name]['inconclusive'] = 'baseline-null-failed'
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
