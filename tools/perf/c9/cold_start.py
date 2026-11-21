"""Cold-start timer for charter E3 (C9). Standard library only; reads logs, never starts anything.

Charter E3 as the owner set it on 18 November 2026: 3 s, "cold start, with an existing session and runtime loaded, to
the first correct frame, in each of three cold runs" (charter E3 proposal; RTE.42: from process start to the first
interactive frame; CT-RTE-11).

    python -B tools/perf/c9/cold_start.py RUN_DIR [RUN_DIR ...] [--out RECORD.json] [--synthetic]

Each run directory holds one `startup.jsonl`. Log schema `magic600-c9-startup-v1` (DRAFT: the launcher and the product
shell are not built yet and will write it; one JSON object per line, QPC values are integer ticks):

- `{"t": "run", "format": "magic600-c9-startup-v1", "run_id": str, "build_identity": str, "qpc_frequency": int,
  "synthetic": bool, "cold": {"new_process": bool, "no_product_process_at_launch": bool, "fresh_session_copy": bool,
  "seconds_since_previous_exit": number | null}}`: the first row, once, from the launcher.
- `{"t": "process_start", "qpc": int}`: the launcher's QPC immediately before it creates the product process (DRAFT:
  the earliest point the launcher can see, so the interval is never short).
- `{"t": "session_opened", "qpc": int}` and `{"t": "runtime_loaded", "qpc": int}`: the shell has opened the existing
  session and loaded the language runtime.
- `{"t": "first_correct_frame", "frame_id": int, "display_qpc": int, "check": "pass" | "fail"}`: the first frame
  whose content passed the correctness check, with the display time of the core's confirmed present (SyncQPCTime).

Cold (DRAFT; the sources do not define it): a new product process, no product process running at launch, a fresh copy
of the session fixture (PHS.64), and at least 20 s since a previous product process exited (run_scene.ps1's pause), or
null when none ran before in this sign-in. The machine is not rebooted and the OS file cache is not flushed; a
reboot-cold reading would be stricter and is the owner's choice. Earlier launches of the same build are allowed: C9
launches a fresh build once, untimed, before its three runs (unattended.ps1 U6), because the first launch of new
binaries also pays Windows' first-run checks (PR 387), and records that launch's time apart, never among the runs.

A run without a passing `first_correct_frame` row, or without `session_opened` or `runtime_loaded`, has failed: the
product did not reach the bar's end point, so the verdict is not met. A run that is not cold, repeats an event, or
shows an event before process start or a load after the first correct frame is invalid: kept, reported, never
counted (PHS.64). The time is (display_qpc - process_start) / qpc_frequency. Met only if at least three valid runs of
one build exist, no run failed, and every valid run takes at most 3.0 s.
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import evidence  # noqa: E402
from evidence import RunError, integer, require  # noqa: E402
from b412_summary import number  # noqa: E402

LOG_FORMAT = 'magic600-c9-startup-v1'
LOG = 'startup.jsonl'
LIMIT_S = 3.0
RUNS_MIN = 3
PAUSE_S = 20
EVENTS = ('process_start', 'session_opened', 'runtime_loaded', 'first_correct_frame')
COLD_FLAGS = ('new_process', 'no_product_process_at_launch', 'fresh_session_copy')
BAR = {'text': ('Charter E3 (owner, 18 November 2026): 3 s, cold start, with an existing session and runtime loaded, '
                'to the first correct frame, in each of three cold runs.'),
       'rule': ('process creation to the confirmed display of the first correct frame; every valid run at most 3.0 s, '
                'at least three valid cold runs of one build, and no failed run'),
       'limit_s': LIMIT_S, 'runs_min': RUNS_MIN,
       'source': ['charter E3', 'owner decision 2026-11-18', 'RTE.42', 'CT-RTE-11', 'PHS.64']}
DRAFTS = ['log schema magic600-c9-startup-v1', 'start = launcher QPC before process creation',
          'cold = new process, none running, fresh session copy, >= 20 s since the last exit; no reboot',
          'one untimed launch of the same build before the runs (first-run checks of new binaries), not counted']


def analyze(rows):
    """One run from its log rows: status valid, failed or invalid, its reasons and its time."""
    head = evidence.read_header(rows, LOG_FORMAT)
    cold = head.get('cold')
    require(isinstance(cold, dict) and all(type(cold.get(flag)) is bool for flag in COLD_FLAGS), 'bad-header')
    gap = cold.get('seconds_since_previous_exit')
    require(gap is None or (number(gap) and gap >= 0), 'bad-header')
    events, reasons = {}, set()
    for row in rows[1:]:
        kind = row.get('t')
        require(kind in EVENTS, 'bad-row')
        require(integer(row.get('display_qpc' if kind == 'first_correct_frame' else 'qpc')), f'bad-{kind}')
        if kind == 'first_correct_frame':
            require(integer(row.get('frame_id')) and row.get('check') in ('pass', 'fail'), 'bad-first_correct_frame')
        if kind in events:
            reasons.add('duplicate-event')
        events.setdefault(kind, row)
    require('process_start' in events, 'missing-process-start')
    start = events['process_start']['qpc']
    ticks = {kind: row.get('display_qpc', row.get('qpc')) for kind, row in events.items()}
    if any(value < start for value in ticks.values()):
        reasons.add('before-process-start')
    if not all(cold[flag] for flag in COLD_FLAGS) or (gap is not None and gap < PAUSE_S):
        reasons.add('not-cold')
    failed = set()
    frame = events.get('first_correct_frame')
    if frame is None:
        failed.add('no-first-correct-frame')
    elif frame['check'] != 'pass':
        failed.add('first-frame-incorrect')
    for kind in ('session_opened', 'runtime_loaded'):
        if kind not in events:
            failed.add(f'{kind.replace("_", "-")}-missing')
        elif frame is not None and ticks[kind] > frame['display_qpc']:
            reasons.add('loaded-after-first-frame')
    seconds = None
    if frame is not None and frame['display_qpc'] >= start:
        seconds = (frame['display_qpc'] - start) / head['qpc_frequency']
    status = 'invalid' if reasons else 'failed' if failed else 'valid'
    return {'run_id': head['run_id'], 'build_identity': head['build_identity'], 'synthetic': head['synthetic'],
            'status': status, 'reasons': sorted(reasons | failed), 'seconds': seconds,
            'within_bar': status == 'valid' and seconds <= LIMIT_S, 'cold': cold}


def read_run(directory):
    directory = Path(directory)
    try:
        return analyze(evidence.read_jsonl(directory / LOG))
    except RunError as error:
        return {'run_id': directory.name, 'build_identity': None, 'synthetic': None, 'status': 'unreadable',
                'reasons': [str(error)], 'seconds': None, 'within_bar': False}


def judge(runs):
    """(verdict, reasons) over the runs: a failed run fails the bar; invalid and unreadable runs never count."""
    counted = [run for run in runs if run['status'] in ('valid', 'failed')]
    if len({run['build_identity'] for run in counted}) > 1:
        return 'invalid', ['mixed-builds']
    if any(run['status'] == 'failed' for run in counted):
        return 'not-met', ['run-failed']
    valid = [run for run in counted if run['status'] == 'valid']
    if not valid:
        return 'invalid', ['no-valid-run']
    if len(valid) < RUNS_MIN:
        return 'insufficient', ['too-few-runs']
    if all(run['seconds'] <= LIMIT_S for run in valid):
        return 'met', []
    return 'not-met', ['over-bar']


def build(directories, argv, synthetic_flag=False):
    runs = [read_run(directory) for directory in directories]
    verdict, reasons = judge(runs)
    identities = sorted({run['build_identity'] for run in runs if run['status'] in ('valid', 'failed')})
    inputs = [evidence.file_entry(f'{run["run_id"]}:{LOG}', Path(directory) / LOG)
              for directory, run in zip(directories, runs) if (Path(directory) / LOG).is_file()]
    synthetic = evidence.is_synthetic([Path(directory) / LOG for directory in directories],
                                      [run.get('synthetic') for run in runs], synthetic_flag)
    seconds = [run['seconds'] for run in runs if run['status'] == 'valid']
    result = {'runs': runs, 'valid_runs': len(seconds), 'max_seconds': max(seconds) if seconds else None}
    return evidence.make_record(
        'charter-e3-cold-start', 'performance', synthetic=synthetic,
        build_identity=identities[0] if len(identities) == 1 else None, inputs=inputs,
        tools=evidence.tool_versions(__file__), command=evidence.portable_command(__file__, argv),
        bar=BAR, result=result, verdict=verdict, reasons=reasons, drafts=DRAFTS)


def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    parser = argparse.ArgumentParser(description='Charter E3 cold-start timer over startup logs.')
    parser.add_argument('runs', nargs='+', type=Path, help='run directories holding startup.jsonl')
    parser.add_argument('--out', type=Path, help='write the evidence record here (never replaces a file)')
    parser.add_argument('--synthetic', action='store_true', help='mark the inputs as synthetic fixtures')
    args = parser.parse_args(argv)
    record = build(args.runs, argv, args.synthetic)
    for run in record['result']['runs']:
        time = f"; {run['seconds']:.3f} s" if run['seconds'] is not None else ''
        print(f"{run['run_id']}: {run['status']}"
              f"{' (' + ', '.join(run['reasons']) + ')' if run['reasons'] else ''}{time}")
    evidence.report(record)
    if args.out:
        try:
            evidence.write_record(args.out, record)
        except OSError as error:
            print(f'cannot write the record: {type(error).__name__}', file=sys.stderr)
            return 1
    return evidence.exit_code(record)


if __name__ == '__main__':
    sys.exit(main())
