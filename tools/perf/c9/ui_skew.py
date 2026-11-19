"""Route B check B2: the skew between the UI frame and the 3D frame (C9). Standard library only; reads logs only.

On route B the core presents the 3D view in its own composition swap chain beneath Godot's per-pixel transparent
window, so the UI frame and the 3D frame are no longer one atomic frame (owner decision, 18 November 2026). Check 2
of that decision is "a measurement of the skew between the UI frame and the 3D frame". The decision sets no bar, so
this tool measures and reports; the verdict is `no-bar` for a valid measurement and the integrator puts the numbers
in front of the owner. The tool never calls a measurement a pass.

    python -B tools/perf/c9/ui_skew.py RUN_DIR [RUN_DIR ...] [--out RECORD.json] [--synthetic]

Each run directory holds the shell's `events.jsonl` and `presentmon.csv`. The capture is required: Godot's own
presents are seen only by PresentMon.

Log rows (DRAFT, format `magic600-c9-shell-events-v1`, beside E2's rows; key_to_frame.py describes `run`, `frame`
and `presented`):

- the `run` row also carries `process_id` and `ui_swap_chain`: Godot's swap chain as PresentMon prints its address.
  Godot exposes no swap chain, so the shell writes `ui_swap_chain: null` with `swap_chain`, the core's own
  composition chain ("0x" and 16 hex digits, from the core's optional export sa2_surface_swap_chain); the UI chain is
  then the one other swap chain of `process_id` in the capture, and a run whose capture shows no other chain or more
  than one is unreadable. The core writes `swap_chain: null` when the run saw more than one chain of its own (a resize
  or a recovery) or the core gave no address, and such a run is unreadable too (C9 product, 19 November 2026). Chains
  compare by their numeric value when both are hexadecimal, so leading zeros and case do not matter;
- `{"t": "ui_frame", "ui_seq": int, "qpc": int, "frame_id": int, "drawn_qpc": int}`: one row per Godot frame that
  has a 3D frame submitted in the same process frame (a Godot frame without one gets no row, and `ui_seq` stays
  contiguous), `ui_seq` 0, 1, 2, ..., `qpc` read before Godot draws the frame (RenderingServer's frame_pre_draw: on
  Godot 4.7's RenderingDevice path the present happens inside the draw, before frame_post_draw), `drawn_qpc` read at
  frame_post_draw and kept for the check below, and `frame_id` the 3D frame the shell submitted in the same process
  frame, which is the 3D state the UI frame shows.

Matching (DRAFT). A UI frame's display time is that of the first displayed present of the UI swap chain in the
PresentMon capture whose present call starts at or after the row's `qpc` and before the next row's `qpc`; a UI frame
with no such present, or with two, is unmatched. Its 3D display time is the core's confirmed `display_qpc` of its
`frame_id` (route B); a frame without a confirmed present is unmatched. The reading that Godot presents inside the
draw comes from Godot's source and is unverified on the machine, so each run reports how many matched UI presents
start at or before their row's `drawn_qpc` (`present_in_draw`); a share well below one says the window is off by a
frame. The skew of a matched pair is the UI display
time minus the 3D display time, in ms: positive when the UI appears after the 3D view. Unmatched frames get no value
and are counted, never filled in. A run is invalid on a contradiction (UI rows out of order, a UI row naming an
unknown frame, frames or presents that key_to_frame.py also refuses), when the window lost the foreground, when the
correctness control failed, without a capture, or with fewer than 100 matched pairs (DRAFT).

Result: per run and pooled over valid runs of one build, the signed skew's and the absolute skew's median, p95, p99
and largest value, and the share of pairs within one refresh (from the 3D swap chain's median present interval).
"""
from __future__ import annotations

import argparse
import bisect
import csv
import statistics as stats_module
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import evidence  # noqa: E402
import key_to_frame  # noqa: E402
from evidence import RunError, integer, require  # noqa: E402
from b412_summary import csv_integer, number, statistics  # noqa: E402
from renderer_gate import CAPTURE_COLUMNS  # noqa: E402

EVENTS = key_to_frame.EVENTS
CAPTURE = key_to_frame.CAPTURE
PAIRS_MIN = 100
BAR = {'text': 'Route B check 2: a measurement of the skew between the UI frame and the 3D frame.',
       'rule': ('no bar: the owner decision of 18 November 2026 sets none; the record reports the numbers and the '
                'owner judges them'),
       'limit_ms': None, 'observations_min': PAIRS_MIN,
       'source': ['owner decision 2026-11-18 route B, check 2', 'RM.26', 'ONE.41']}
DRAFTS = ['ui_frame rows in magic600-c9-shell-events-v1',
          'UI frame time = Godot frame_pre_draw (present inside the draw on the RenderingDevice path)',
          'UI present = first displayed present of the UI swap chain after the row and before the next',
          'at least 100 matched pairs per run', 'one refresh = median confirmed 3D present interval']


def check_rows(rows):
    head = key_to_frame.check_rows(rows)
    for row in rows[1:]:
        if row.get('t') == 'ui_frame':
            require(all(integer(row.get(field)) for field in ('ui_seq', 'qpc', 'frame_id')), 'bad-ui_frame')
            require(row.get('drawn_qpc') is None or (integer(row['drawn_qpc']) and row['drawn_qpc'] >= row['qpc']),
                    'bad-ui_frame')
    return head


def read_ui_presents(path, process_id, swap_chain, frequency):
    """(present start ticks, display ticks or None) of the UI swap chain's presents (`swap_chain` as chain_key
    gives it) in a PresentMon v1 CSV."""
    columns, presents, chain = CAPTURE_COLUMNS, [], swap_chain
    try:
        with Path(path).open(encoding='utf-8-sig', newline='') as stream:
            reader = csv.DictReader(stream, strict=True)
            header = reader.fieldnames or []
            require(all(name in header for name in (columns['process'], columns['swap_chain'], columns['time'],
                                                    key_to_frame.DISPLAYED)), 'presentmon-missing-column')
            for row in reader:
                if csv_integer(row.get(columns['process'])) != process_id:
                    continue
                if chain_key(row.get(columns['swap_chain'])) != chain:
                    continue
                start = csv_integer(row.get(columns['time']))
                require(start is not None, 'bad-field')
                displayed = None
                if (row.get(key_to_frame.DROPPED) or '0').strip() in ('0', ''):
                    try:
                        until = float(row.get(key_to_frame.DISPLAYED))
                    except (TypeError, ValueError):
                        until = None
                    if until is not None and number(until) and until >= 0:
                        displayed = start + round(until * frequency / 1000)
                presents.append((start, displayed))
    except OSError:
        raise RunError('missing-file') from None
    except (csv.Error, UnicodeError):
        raise RunError('bad-field') from None
    return sorted(presents)


def chain_key(text):
    """A swap chain address in comparable form: its value when it is hexadecimal, else the case-folded text."""
    text = (text or '').strip()
    try:
        return int(text, 16)
    except ValueError:
        return text.casefold()


def capture_chains(path, process_id):
    """The swap chains (as chain_key gives them) of `process_id`'s presents in a PresentMon v1 CSV."""
    columns, chains = CAPTURE_COLUMNS, set()
    try:
        with Path(path).open(encoding='utf-8-sig', newline='') as stream:
            reader = csv.DictReader(stream, strict=True)
            header = reader.fieldnames or []
            require(all(name in header for name in (columns['process'], columns['swap_chain'])),
                    'presentmon-missing-column')
            for row in reader:
                if csv_integer(row.get(columns['process'])) == process_id:
                    if (row.get(columns['swap_chain']) or '').strip():
                        chains.add(chain_key(row.get(columns['swap_chain'])))
    except OSError:
        raise RunError('missing-file') from None
    except (csv.Error, UnicodeError):
        raise RunError('bad-field') from None
    return chains


def ui_chain(head, capture):
    """(Godot's swap chain, whether it was inferred): declared in the run row, or the one chain of the process in the
    capture other than the core's `swap_chain` when `ui_swap_chain` is null."""
    declared, core = head.get('ui_swap_chain'), head.get('swap_chain')
    if declared is None and 'swap_chain' in head:
        require(isinstance(core, str) and bool(core.strip()), 'core-swap-chain-unknown')
        others = capture_chains(capture, head['process_id']) - {chain_key(core)}
        require(len(others) <= 1, 'ui-swap-chain-ambiguous')
        require(len(others) == 1, 'ui-swap-chain-absent')
        return next(iter(others)), True
    require(isinstance(declared, str) and bool(declared.strip()), 'ui-swap-chain-undeclared')
    return chain_key(declared), False


def pair(ui_rows, frames, presented, ui_presents, frequency, contradictions):
    """One entry per UI frame: its skew in ms, or why it has none."""
    starts = [start for start, _ in ui_presents]
    entries = []
    for index, row in enumerate(ui_rows):
        if row['ui_seq'] != index or (index and row['qpc'] < ui_rows[index - 1]['qpc']):
            contradictions.add('ui-order')
        entry = {'ui_seq': row['ui_seq'], 'frame_id': row['frame_id'], 'status': 'matched', 'skew_ms': None}
        if row['frame_id'] not in frames:
            contradictions.add('ui-names-unknown-frame')
            entry['status'] = 'unknown-frame'
            entries.append(entry)
            continue
        end = ui_rows[index + 1]['qpc'] if index + 1 < len(ui_rows) else None
        low = bisect.bisect_left(starts, row['qpc'])
        high = len(starts) if end is None else bisect.bisect_left(starts, end)
        window = ui_presents[low:high]
        if len(window) != 1 or window[0][1] is None:
            entry['status'] = 'ui-present-unmatched' if len(window) != 1 else 'ui-present-not-displayed'
        elif row['frame_id'] not in presented:
            entry['status'] = '3d-unconfirmed'
        else:
            entry['skew_ms'] = (window[0][1] - presented[row['frame_id']]['display_qpc']) * 1000 / frequency
            if row.get('drawn_qpc') is not None:
                entry['present_in_draw'] = window[0][0] <= row['drawn_qpc']
        if entry['skew_ms'] is None and entry['status'] == 'matched':
            entry['status'] = 'unmatched'
        entries.append(entry)
    return entries


def refresh_ms(presented, frequency):
    """The median interval between consecutive confirmed 3D presents, or None."""
    ticks = sorted(row['display_qpc'] for row in presented.values())
    gaps = [(after - before) * 1000 / frequency for before, after in zip(ticks, ticks[1:]) if after > before]
    return stats_module.median(gaps) if gaps else None


def summary(skews, refresh):
    if not skews:
        return None
    absolute = [abs(value) for value in skews]
    result = {'signed': statistics(skews), 'absolute': statistics(absolute), 'refresh_ms': refresh,
              'within_one_refresh': None}
    if refresh:
        result['within_one_refresh'] = sum(value <= refresh for value in absolute) / len(absolute)
    return result


def analyze(rows, ui_presents):
    head = check_rows(rows)
    frequency = head['qpc_frequency']
    contradictions, reasons = set(), set()
    frames, presented = key_to_frame.check_frames(rows[1:], contradictions)
    ui_rows = [row for row in rows[1:] if row['t'] == 'ui_frame']
    entries = pair(ui_rows, frames, presented, ui_presents, frequency, contradictions)
    skews = [entry['skew_ms'] for entry in entries if entry['skew_ms'] is not None]
    counts = {}
    for entry in entries:
        counts[entry['status']] = counts.get(entry['status'], 0) + 1
    refresh = refresh_ms(presented, frequency)
    in_draw = [entry['present_in_draw'] for entry in entries if 'present_in_draw' in entry]
    run = {'run_id': head['run_id'], 'build_identity': head['build_identity'], 'synthetic': head['synthetic'],
           'ui_frames': len(entries), 'pairs': len(skews), 'statuses': dict(sorted(counts.items())),
           'present_in_draw': {'checked': len(in_draw), 'within': sum(in_draw)},
           'contradictions': sorted(contradictions), 'stats': summary(skews, refresh)}
    if contradictions:
        reasons.add('contradiction')
    if len(skews) < PAIRS_MIN:
        reasons.add('too-few-pairs')
    if not head['foreground_throughout']:
        reasons.add('not-foreground')
    if head['correctness'] != 'pass':
        reasons.add('correctness-failed')
    run['status'], run['reasons'] = ('invalid' if reasons else 'valid'), sorted(reasons)
    run['skews_ms'] = skews
    return run


def read_run(directory):
    directory = Path(directory)
    try:
        rows = evidence.read_jsonl(directory / EVENTS)
        head = check_rows(rows)
        require((directory / CAPTURE).is_file(), 'no-capture')
        require(integer(head.get('process_id')), 'ui-swap-chain-undeclared')
        chain, inferred = ui_chain(head, directory / CAPTURE)
        presents = read_ui_presents(directory / CAPTURE, head['process_id'], chain, head['qpc_frequency'])
        run = analyze(rows, presents)
        run['ui_swap_chain'], run['ui_swap_chain_inferred'] = chain, inferred
        return run
    except RunError as error:
        return {'run_id': directory.name, 'build_identity': None, 'synthetic': None, 'status': 'unreadable',
                'reasons': [str(error)], 'skews_ms': [], 'stats': None}


def judge(runs):
    valid = [run for run in runs if run['status'] == 'valid']
    if not valid:
        return 'invalid', ['no-valid-run'], None
    if len({run['build_identity'] for run in valid}) > 1:
        return 'invalid', ['mixed-builds'], None
    refreshes = [run['stats']['refresh_ms'] for run in valid if run['stats']['refresh_ms']]
    pooled = summary([value for run in valid for value in run['skews_ms']],
                     stats_module.median(refreshes) if refreshes else None)
    return 'no-bar', [], pooled


def build(directories, argv, synthetic_flag=False):
    runs = [read_run(directory) for directory in directories]
    verdict, reasons, pooled = judge(runs)
    identities = sorted({run['build_identity'] for run in runs if run['status'] == 'valid'})
    inputs = [evidence.file_entry(f'{run["run_id"]}:{name}', Path(directory) / name)
              for directory, run in zip(directories, runs) for name in (EVENTS, CAPTURE)
              if (Path(directory) / name).is_file()]
    synthetic = evidence.is_synthetic([Path(directory) / EVENTS for directory in directories],
                                      [run.get('synthetic') for run in runs], synthetic_flag)
    for run in runs:
        run.pop('skews_ms', None)
    result = {'runs': runs, 'pooled': pooled, 'valid_runs': sum(run['status'] == 'valid' for run in runs),
              'invalid_runs': sum(run['status'] != 'valid' for run in runs)}
    return evidence.make_record(
        'route-b-b2-ui-3d-skew', 'performance', synthetic=synthetic,
        build_identity=identities[0] if len(identities) == 1 else None, inputs=inputs,
        tools=evidence.tool_versions(__file__, ('renderer_gate.py', 'c9/key_to_frame.py')),
        command=evidence.portable_command(__file__, argv), bar=BAR, result=result, verdict=verdict,
        reasons=reasons, drafts=DRAFTS)


def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    parser = argparse.ArgumentParser(description='Route B check B2: UI frame to 3D frame skew from shell logs.')
    parser.add_argument('runs', nargs='+', type=Path, help='run directories holding events.jsonl and presentmon.csv')
    parser.add_argument('--out', type=Path, help='write the evidence record here (never replaces a file)')
    parser.add_argument('--synthetic', action='store_true', help='mark the inputs as synthetic fixtures')
    args = parser.parse_args(argv)
    record = build(args.runs, argv, args.synthetic)
    for run in record['result']['runs']:
        absolute = (run.get('stats') or {}).get('absolute')
        in_draw = run.get('present_in_draw') or {}
        line = (f"{run['run_id']}: {run['status']}"
                f"{' (' + ', '.join(run['reasons']) + ')' if run['reasons'] else ''}; UI frames "
                f"{run.get('ui_frames', 0)}, pairs {run.get('pairs', 0)}")
        if in_draw.get('checked'):
            line += f"; UI present inside the draw {in_draw['within']} of {in_draw['checked']}"
        print(line + (f"; |skew| p95 {absolute['p95']:.1f} ms" if absolute else ''))
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
