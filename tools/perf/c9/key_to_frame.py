"""Key-to-frame timer for charter E2 on route B (C9). Standard library only; reads logs, never runs anything.

Charter E2: "A turn at full detail responds within 100 ms (the B4-12 M1 interval) on the target machine." LP3 reads it
as the full interval from the key press to the first correct frame that shows the press, with p95 at most 100 ms over
at least 100 observations; a kernel round trip, an interpreter timer or renderer FPS cannot replace it (RTE.43,
V11.1). On route B the interval ends when the core's own swap chain confirms that frame as presented, so it includes
about one more refresh (owner decision, 18 November 2026).

    python -B tools/perf/c9/key_to_frame.py RUN_DIR [RUN_DIR ...] [--out RECORD.json] [--synthetic]

Each run directory holds `events.jsonl` and, optionally, `presentmon.csv` (PresentMon 2.6 with `--v1_metrics
--qpc_time`; see presentmon_capture.py), which is then used as a cross-check.

Log schema `magic600-c9-shell-events-v1` (DRAFT: the product shell is not built yet and will write it). One JSON
object per line; every QPC value is an integer tick count of the machine's QueryPerformanceCounter:

- `{"t": "run", "format": "magic600-c9-shell-events-v1", "run_id": str, "build_identity": str, "qpc_frequency": int,
  "synthetic": bool, "foreground_throughout": bool, "correctness": "pass" | "fail", "process_id": int,
  "swap_chain": str}`: the first row, once. `correctness` is the run's correctness control (PHS.64); `process_id` and
  `swap_chain` (the core's composition swap chain as PresentMon prints its address) are needed only for the
  cross-check.
- `{"t": "key", "seq": int, "qpc": int, "frame_id": int | null}`: one row per measured press, `seq` 0, 1, 2, ...
  `qpc` is the earliest QPC the shell has for the press (DRAFT: when it receives the key-down). `frame_id` is the
  first frame whose preview carries the press, or null when the shell saw none.
- `{"t": "frame", "frame_id": int, "qpc": int, "device_epoch": int, "surface": int}`: every frame the shell hands to
  the core, logged at submission; frame ids strictly increase.
- `{"t": "presented", "frame_id": int, "device_epoch": int, "surface": int, "sequence": int, "display_qpc": int}`:
  one `sa2_presented` row as the shell receives it from the core: a frame the core confirmed as presented through its
  own swap chain's frame statistics, `sequence` the frame's sequence as the core's row carries it (it rises with every
  frame) and `display_qpc` the SyncQPCTime of the statistics row that confirmed it (`sa2_presented_row.display_qpc`).
  A frame without its own row is not confirmed (route B).

Rules. A key is matched to its frame by frame id only. A key whose frame has no presented row, or that names no
frame, is unconfirmed: it gets no time, is printed on stderr and is never filled in from another frame. It counts as
one failed key, with a time above 100 ms, in its run's p95 and in the pooled p95, and its run stays valid (owner,
20 November 2026, item 2: docs/wiki/decisions/owner-decisions-2026-11-20-lp3-exit.md). A percentile that falls above
100 ms where a failed key could stand has no number (None). These contradictions make a run invalid:
a presented row for a frame the log never named, a frame presented or named twice, a presented row whose device
epoch or surface differs from its frame, a display before submission, a key naming an unknown frame or a frame
submitted before the key, keys out of order or naming frames out of order, and presents out of order on one swap
chain. A run is also invalid when the window lost the foreground, the correctness control failed, it has no keys, or
its PresentMon capture disagrees: the first present of the declared swap chain at or after a confirmed frame's
submission is missing or later than the frame's confirmed display time. PresentMon marks every present of the core's
composition chain Dropped (20 Nov 2026 rehearsal), so the display time comes from the core's frame statistics alone
and PresentMon checks only that the frame was presented in between, whatever its Dropped column says (owner, 20
November 2026, item 9). Invalid runs are kept and reported, never pooled (PHS.64).

Verdict: nearest-rank percentiles over the confirmed and failed keys of the valid runs, which must share one build
identity. Met only if at least 100 confirmed keys are pooled and the pooled p95 and every valid run's p95, failed keys
counted, are at most 100 ms (the B4-12 rule that the pooled value and every run meet a target). One run of 100 presses is enough (RTE.43); B4-12
D5's three series of 100 are not required (DRAFT).
"""
from __future__ import annotations

import argparse
import bisect
import csv
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import evidence  # noqa: E402
from evidence import RunError, integer, require  # noqa: E402
from b412_summary import csv_integer  # noqa: E402
from renderer_gate import CAPTURE_COLUMNS  # noqa: E402

LOG_FORMAT = 'magic600-c9-shell-events-v1'
EVENTS = 'events.jsonl'
CAPTURE = 'presentmon.csv'
P95_MAX_MS = 100.0
OBSERVATIONS_MIN = 100
DISPLAYED = 'msUntilDisplayed'
DROPPED = 'Dropped'
FIELDS = {'key': ('seq', 'qpc'), 'frame': ('frame_id', 'qpc', 'device_epoch', 'surface'),
          'presented': ('frame_id', 'device_epoch', 'surface', 'sequence', 'display_qpc')}
# Rows of the same log that other tools check (ui_skew.py's ui_frame, lx4.py's lx4 and lx4-select, lx2.py's lx2, and
# the `module` row a shell module writes when it starts, which a live poster reads, and the `steps` row of a key's or a
# bind's frame-thread steps, SF of 27 Nov 2026); E2 skips them, so E2 runs over the same run directories as B2, LX-4
# and LX-2 (RTE.51 step 8).
OTHER_ROWS = ('ui_frame', 'lx4', 'lx4-select', 'lx2', 'module', 'steps')
BAR = {'text': ('Charter E2: A turn at full detail responds within 100 ms (the B4-12 M1 interval) on the target '
                'machine.'),
       'rule': ('key press to the core-confirmed present of the first frame carrying it; nearest-rank p95 <= 100 ms '
                'over at least 100 confirmed keys of valid runs of one build; met only if the pooled p95 and every '
                "valid run's p95 meet it; a key without a confirmed frame counts as one failed key above 100 ms in its "
                "run's and the pooled p95, its run stays valid, and it is never filled in"),
       'limit_ms': P95_MAX_MS, 'observations_min': OBSERVATIONS_MIN,
       'source': ['charter E2', 'RTE.43', 'V11.1', 'B4-12 method', 'PHS.64', 'owner decision 2026-11-18 (route B)',
                  'owner decision 2026-11-20 (LP3 exit, item 2)']}
DRAFTS = ['log schema magic600-c9-shell-events-v1', 'press time = shell key-down receipt',
          'PresentMon cross-check by present order (route B chain reported Dropped)', 'one run of 100 presses suffices']


def e2_percentile(latencies, failed, percentile):
    """Nearest-rank percentile over the confirmed times and `failed` keys, each above the bar (item 2); None when it
    lies above the bar where a failed key could stand, since a failed key has no number."""
    ordered = sorted(latencies)
    count = len(ordered) + failed
    if not count:
        return None
    rank = (percentile * count + 99) // 100
    if not failed:
        return ordered[rank - 1]
    within = bisect.bisect_right(ordered, P95_MAX_MS)
    return ordered[rank - 1] if rank <= within else None


def e2_statistics(latencies, failed):
    """A run's or the pool's statistics, failed keys counted (item 2): `n` all keys, `confirmed`, `failed_keys`;
    `mean` and `max` only without failed keys; `p95_met` is the bar's test. None without keys."""
    count = len(latencies) + failed
    if not count:
        return None
    stats = {'n': count, 'confirmed': len(latencies), 'failed_keys': failed,
             'mean': math.fsum(latencies) / len(latencies) if latencies and not failed else None,
             'p50': e2_percentile(latencies, failed, 50), 'p95': e2_percentile(latencies, failed, 95),
             'p99': e2_percentile(latencies, failed, 99), 'max': max(latencies) if latencies and not failed else None}
    stats['p95_met'] = stats['p95'] is not None and stats['p95'] <= P95_MAX_MS
    return stats


def check_rows(rows):
    head = evidence.read_header(rows, LOG_FORMAT)
    require(type(head.get('foreground_throughout')) is bool and head.get('correctness') in ('pass', 'fail'),
            'bad-header')
    for row in rows[1:]:
        if row.get('t') in OTHER_ROWS:
            continue
        fields = FIELDS.get(row.get('t'))
        require(fields is not None, 'bad-row')
        require(all(integer(row.get(field)) for field in fields), f"bad-{row['t']}")
        if row['t'] == 'key':
            require(row.get('frame_id') is None or integer(row['frame_id']), 'bad-key')
    return head


def check_frames(rows, contradictions):
    """The frames and presented rows by frame id, adding the contradictions they show."""
    frames, presented, last = {}, {}, None
    for row in rows:
        if row['t'] == 'frame':
            if row['frame_id'] in frames:
                contradictions.add('frame-named-twice')
            elif last is not None and row['frame_id'] < last:
                contradictions.add('frame-order')
            frames.setdefault(row['frame_id'], row)
            last = row['frame_id'] if last is None else max(last, row['frame_id'])
        elif row['t'] == 'presented':
            if row['frame_id'] in presented:
                contradictions.add('presented-twice')
            presented.setdefault(row['frame_id'], row)
    chains = {}
    for frame_id in sorted(presented):
        row, frame = presented[frame_id], frames.get(frame_id)
        if frame is None:
            contradictions.add('presented-unnamed-frame')
            continue
        if (row['device_epoch'], row['surface']) != (frame['device_epoch'], frame['surface']):
            contradictions.add('presented-mismatch')
        if row['display_qpc'] < frame['qpc']:
            contradictions.add('displayed-before-submit')
        chains.setdefault((row['device_epoch'], row['surface']), []).append(row)
    for chain in chains.values():
        if any(after['sequence'] <= before['sequence'] or after['display_qpc'] < before['display_qpc']
               for before, after in zip(chain, chain[1:])):
            contradictions.add('present-order')
    return frames, presented


def match_keys(keys, frames, presented, frequency, contradictions):
    """One entry per key: confirmed with its latency in ms, unconfirmed, or no-frame; never a filled-in time."""
    entries = []
    for index, key in enumerate(keys):
        if key['seq'] != index or (index and key['qpc'] < keys[index - 1]['qpc']):
            contradictions.add('key-order')
        entry = {'seq': key['seq'], 'frame_id': key.get('frame_id'), 'status': 'no-frame', 'latency_ms': None}
        frame_id = key.get('frame_id')
        if frame_id is not None:
            entry['status'] = 'unconfirmed'
            frame = frames.get(frame_id)
            if frame is None:
                contradictions.add('key-names-unknown-frame')
            elif frame['qpc'] < key['qpc']:
                contradictions.add('frame-before-key')
            elif frame_id in presented:
                entry['status'] = 'confirmed'
                entry['latency_ms'] = (presented[frame_id]['display_qpc'] - key['qpc']) * 1000 / frequency
        entries.append(entry)
    named = [key['frame_id'] for key in keys if key.get('frame_id') is not None]
    if any(after < before for before, after in zip(named, named[1:])):
        contradictions.add('key-frames-out-of-order')
    return entries


def cross_check(entries, frames, presented, present_ticks):
    """Confirmed frames whose first present of the declared chain at or after their submission is missing or later
    than their confirmed display time (owner, 20 November 2026, item 9)."""
    confirmed = sorted({entry['frame_id'] for entry in entries if entry['status'] == 'confirmed'})
    misses = 0
    for frame_id in confirmed:
        index = bisect.bisect_left(present_ticks, frames[frame_id]['qpc'])
        if index == len(present_ticks) or present_ticks[index] > presented[frame_id]['display_qpc']:
            misses += 1
    return {'frames_checked': len(confirmed), 'disagreements': misses, 'presentmon_presents': len(present_ticks)}


def analyze(rows, present_ticks=None):
    """One run from its log rows and, optionally, the PresentMon present times of the declared swap chain."""
    head = check_rows(rows)
    frequency = head['qpc_frequency']
    contradictions, reasons = set(), set()
    frames, presented = check_frames(rows[1:], contradictions)
    keys = [row for row in rows[1:] if row['t'] == 'key']
    entries = match_keys(keys, frames, presented, frequency, contradictions)
    latencies = [entry['latency_ms'] for entry in entries if entry['status'] == 'confirmed']
    unconfirmed = [entry['seq'] for entry in entries if entry['status'] != 'confirmed']
    run = {'run_id': head['run_id'], 'build_identity': head['build_identity'], 'synthetic': head['synthetic'],
           'keys': len(entries), 'confirmed': len(latencies), 'unconfirmed': len(unconfirmed),
           'no_frame': sum(entry['status'] == 'no-frame' for entry in entries), 'unconfirmed_keys': unconfirmed,
           'contradictions': sorted(contradictions), 'per_key': entries,
           'stats': e2_statistics(latencies, len(unconfirmed))}
    if present_ticks is not None:
        run['presentmon'] = cross_check(entries, frames, presented, sorted(present_ticks))
        if run['presentmon']['disagreements']:
            reasons.add('presentmon-disagrees')
    if contradictions:
        reasons.add('contradiction')
    if not entries:
        reasons.add('no-keys')
    if not head['foreground_throughout']:
        reasons.add('not-foreground')
    if head['correctness'] != 'pass':
        reasons.add('correctness-failed')
    run['status'], run['reasons'] = ('invalid' if reasons else 'valid'), sorted(reasons)
    run['latencies_ms'] = latencies
    return run


def read_present_ticks(path, process_id, swap_chain):
    """Present call times (QPC ticks) of the declared swap chain in a PresentMon v1 CSV, dropped or not: PresentMon
    marks every present of the core's composition chain Dropped (owner, 20 November 2026, item 9)."""
    columns, ticks = CAPTURE_COLUMNS, []
    chain = swap_chain.strip().casefold()
    try:
        with Path(path).open(encoding='utf-8-sig', newline='') as stream:
            reader = csv.DictReader(stream, strict=True)
            header = reader.fieldnames or []
            require(all(name in header for name in (columns['process'], columns['swap_chain'], columns['time'])),
                    'presentmon-missing-column')
            for row in reader:
                if csv_integer(row.get(columns['process'])) != process_id:
                    continue
                if (row.get(columns['swap_chain']) or '').strip().casefold() != chain:
                    continue
                ticks.append(csv_integer(row.get(columns['time'])))
    except OSError:
        raise RunError('missing-file') from None
    except (csv.Error, UnicodeError):
        raise RunError('bad-field') from None
    return ticks


def read_run(directory):
    """One run directory; an unreadable run is kept as invalid with its reason code."""
    directory = Path(directory)
    try:
        rows = evidence.read_jsonl(directory / EVENTS)
        head = check_rows(rows)
        display = None
        if (directory / CAPTURE).is_file():
            require(integer(head.get('process_id')) and isinstance(head.get('swap_chain'), str)
                    and bool(head['swap_chain'].strip()), 'presentmon-undeclared')
            display = read_present_ticks(directory / CAPTURE, head['process_id'], head['swap_chain'])
        return analyze(rows, display)
    except RunError as error:
        return {'run_id': directory.name, 'build_identity': None, 'synthetic': None, 'status': 'unreadable',
                'reasons': [str(error)], 'latencies_ms': [], 'stats': None}


def judge(runs):
    """The verdict over the valid runs: (verdict, reasons, pooled statistics or None)."""
    valid = [run for run in runs if run['status'] == 'valid']
    if not valid:
        return 'invalid', ['no-valid-run'], None
    if len({run['build_identity'] for run in valid}) > 1:
        return 'invalid', ['mixed-builds'], None
    pooled = [value for run in valid for value in run['latencies_ms']]
    stats = e2_statistics(pooled, sum(run['unconfirmed'] for run in valid))
    if len(pooled) < OBSERVATIONS_MIN:
        return 'insufficient', ['too-few-observations'], stats
    met = stats['p95_met'] and all(run['stats']['p95_met'] for run in valid)
    return ('met', [], stats) if met else ('not-met', ['p95-over-bar'], stats)


def build(directories, argv, synthetic_flag=False):
    runs = [read_run(directory) for directory in directories]
    verdict, reasons, pooled = judge(runs)
    identities = sorted({run['build_identity'] for run in runs if run['status'] == 'valid'})
    inputs = [evidence.file_entry(f'{run["run_id"]}:{name}', Path(directory) / name)
              for directory, run in zip(directories, runs) for name in (EVENTS, CAPTURE)
              if (Path(directory) / name).is_file()]
    paths = [Path(directory) / EVENTS for directory in directories]
    synthetic = evidence.is_synthetic(paths, [run.get('synthetic') for run in runs], synthetic_flag)
    for run in runs:
        run.pop('latencies_ms', None)
    result = {'runs': runs, 'pooled': pooled, 'valid_runs': sum(run['status'] == 'valid' for run in runs),
              'invalid_runs': sum(run['status'] != 'valid' for run in runs),
              'unconfirmed_keys': sum(run.get('unconfirmed', 0) for run in runs),
              'confirmed_keys': sum(run.get('confirmed', 0) for run in runs if run['status'] == 'valid'),
              'failed_keys': sum(run.get('unconfirmed', 0) for run in runs if run['status'] == 'valid')}
    return evidence.make_record(
        'charter-e2-key-to-frame', 'performance', synthetic=synthetic,
        build_identity=identities[0] if len(identities) == 1 else None, inputs=inputs,
        tools=evidence.tool_versions(__file__, ('renderer_gate.py',)),
        command=evidence.portable_command(__file__, argv),
        bar=BAR, result=result, verdict=verdict, reasons=reasons, drafts=DRAFTS)


def p95_text(stats):
    return f"{stats['p95']:.1f} ms" if stats['p95'] is not None else f'above {P95_MAX_MS:.0f} ms (failed keys)'


def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    parser = argparse.ArgumentParser(description='Charter E2 key-to-frame timer over route-B shell event logs.')
    parser.add_argument('runs', nargs='+', type=Path, help='run directories holding events.jsonl')
    parser.add_argument('--out', type=Path, help='write the evidence record here (never replaces a file)')
    parser.add_argument('--synthetic', action='store_true', help='mark the inputs as synthetic fixtures')
    args = parser.parse_args(argv)
    record = build(args.runs, argv, args.synthetic)
    for run in record['result']['runs']:
        stats = run.get('stats')
        print(f"{run['run_id']}: {run['status']}"
              f"{' (' + ', '.join(run['reasons']) + ')' if run['reasons'] else ''}; keys {run.get('keys', 0)}, "
              f"confirmed {run.get('confirmed', 0)}, unconfirmed {run.get('unconfirmed', 0)}"
              f"{'; p95 %s' % p95_text(stats) if stats else ''}")
        if run.get('unconfirmed'):
            print(f"UNCONFIRMED: run {run['run_id']} has {run['unconfirmed']} key(s) without a confirmed frame "
                  f"(seq {', '.join(map(str, run['unconfirmed_keys'][:20]))}"
                  f"{', ...' if run['unconfirmed'] > 20 else ''}); they get no time and count as failed keys",
                  file=sys.stderr)
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
