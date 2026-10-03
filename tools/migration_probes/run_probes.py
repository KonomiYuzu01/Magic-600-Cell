"""Run the Windows migration probes in a fresh run root; keep the raw result private, publish a sanitized one.

    tools/.venv/engine/Scripts/python.exe tools/migration_probes/run_probes.py p3 pipeline p1 \
        --out docs/progress/1.0/migration-probe-results.json

Probes: p3 (P3 and F20), pipeline (F15, F18, F21 and baselines), p1 (P1 matrix, F4 and controls),
p2 (barrier qualification and process interruption; the test double runs in tests/test_migration_p2.py).
The raw result stays in the run root under work/ (ignored). `--out` is merged per probe and is
written only when the sanitized text has no absolute path, account or host identifier.
"""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import json
import platform
from pathlib import Path
import sqlite3
import sys
import time

sys.path.insert(0, str(Path(__file__).resolve().parent))
import fixtures as F  # noqa: E402
import sanitize  # noqa: E402

PROBES = ('p3', 'pipeline', 'p1', 'p2')


def environment(run_root: Path) -> dict:
    import winapi as W
    return dict(os=platform.system(), os_release=platform.release(), os_build=platform.version(),
                python=platform.python_version(), python_bits=64 if sys.maxsize > 2 ** 32 else 32,
                sqlite=sqlite3.sqlite_version, file_system=W.volume_file_system(run_root),
                engine_environment=sys.prefix != sys.base_prefix,
                evidence_class='actual Windows (synthetic sessions; no performance claims)')


def summarize_p1(result: dict) -> dict:
    keep = ('case', 'strategy', 'mode', 'outcome', 'passed', 'inconclusive', 'preserved', 'reached', 'error',
            'opened', 'model_matches', 'reasons', 'snapshot_changes', 'digest_changes', 'backup_statuses',
            'overflows', 'watch_errors', 'harness_error')
    attempts = []
    for record in result['attempts']:
        item = {key: record[key] for key in keep if key in record}
        item['events'] = sorted({'%s %s' % (action, name) for action, name in record.get('events', [])})
        if 'copy' in record:
            item['copy'] = {key: record['copy'][key] for key in ('ok', 'integrity', 'head_matches', 'head_event_present')}
        if record.get('deny'):
            item['deny'] = {key: record['deny'].get(key) for key in ('applied', 'effective', 'removed')}
        attempts.append(item)
    return dict(strategies=result['strategies'], verdicts=result['verdicts'], chosen=result['chosen'],
                controls=result['controls'], attempts=attempts)


def run_probe(name: str, run_root: Path):
    if name == 'p3':
        import p3_lock
        return p3_lock.run(run_root)
    if name == 'pipeline':
        import pipeline
        return pipeline.run(run_root)
    if name == 'p1':
        import p1_source
        return p1_source.run(run_root)
    import p2_publish
    return p2_publish.run(run_root)


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('probes', nargs='+', choices=PROBES)
    parser.add_argument('--out', type=Path)
    args = parser.parse_args(argv)
    run_id = datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ')
    run_root = F.create_run_root(F.ROOT / 'work' / 'migration-probes' / 'runs' / run_id)
    raw = dict(run=run_id, environment=environment(run_root), probes={})
    for name in args.probes:
        began = time.monotonic()
        raw['probes'][name] = run_probe(name, run_root)
        print('%s finished in %.0f s' % (name, time.monotonic() - began), flush=True)
        (run_root / 'raw.json').write_text(json.dumps(raw, indent=1), encoding='utf-8')
    public = dict(raw)
    public['probes'] = {name: summarize_p1(result) if name == 'p1' else result for name, result in raw['probes'].items()}
    command = 'tools/.venv/engine/Scripts/python.exe tools/migration_probes/run_probes.py ' + ' '.join(args.probes)
    public['commands'] = {name: command for name in args.probes}
    public = sanitize.sanitize(public, {'<run>': run_root, '<repo>': F.ROOT})
    text = json.dumps(public, indent=1, sort_keys=True)
    (run_root / 'public.json').write_text(text, encoding='utf-8')
    found = sanitize.leaks(text)
    if found:
        print('sanitized result still contains: ' + ', '.join(found) + '; not written to --out', file=sys.stderr)
        return 2
    if args.out:
        out = F.ROOT / args.out if not args.out.is_absolute() else args.out
        merged = json.loads(out.read_text(encoding='utf-8')) if out.exists() else dict(runs={}, probes={}, commands={})
        merged['runs'][run_id] = public['environment']
        for name in args.probes:
            merged['probes'][name] = dict(run=run_id, result=public['probes'][name])
            merged['commands'][name] = public['commands'][name]
        out.write_text(json.dumps(merged, indent=1, sort_keys=True) + '\n', encoding='utf-8')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
