"""LX-2's fault windows in the product shell (C9 U7; RTE.42 conditions 3 and 4, CT-RTE-10). Standard library only.

    python -B tools/perf/c9/lx2.py post --fixtures DIR --run DIR [--keep-open] [--dry-run]
    python -B tools/perf/c9/lx2.py analyze --fixtures DIR RUN_DIR --faults RECORD [--out RECORD.json] [--synthetic]

RTE.42 injects allocation to its limit, recursion to its limit, a loop with no kernel call and a forced kill, and a
placement passes only if every fault meets four conditions: (1) the journal rows and the state hash afterwards equal a
control run's; (2) the job ends with a typed outcome and the session stays open; (3) the renderer gate and LX-4's echo
bar hold during the fault window; (4) at least 100 commits issued during fault windows meet charter E2. c600-lx2-faults
(rust/c600-lx2) checks (1) and (2) for every fault, alone and with a session open, and writes its record
(`magic600-c9-lx2-faults-v1`). This tool measures (3) and (4) in the product shell's frame loop, and `analyze` takes
that record in, so that one record judges all four conditions for every fault.

The run (the runbook's U7): `renderer/shell/launch.py --out <run dir> --lx4 --lx4-fixtures <fixtures> --lx2-injection
<c600-lx2-child>` with PresentMon alongside (`presentmon.csv` in the run directory, presentmon_capture.py), then
`post`. The shell then runs on the injection child (never shipped) instead of the build's runtime, with LX-2's module
and LX-4's module; `launch.json` names the child's file and SHA-256. `post --dry-run` reads the fixtures and prints
the plan and its time; it finds no window and posts nothing. Without `--dry-run`, `post` needs Windows, the running
shell and the foreground (the owner starts it from the desktop, with the desktop unlocked).

Faults, in this order (`FAULTS`; wParam 1 to 7 of the registered message `Magic600.LX2.Phase`): `alloc_metered`,
`recursion_metered` and `loop_metered` (view jobs the meter stops at its limit), `forced_kill` (the module kills the
child that runs the job), `alloc_ceiling` (the child's allocator refuses past c600-lx2's 32 MiB ceiling, so the child
aborts), `recursion_native` and `loop_native` (the injection device's native recursion until the stack overflows, and
native loop until the host's view deadline, 60 s, ends the child). The shell's LX-2 module
(renderer/shell/project/Lx2/Lx2Module.cs) runs the windows; its summary is the protocol. A window runs from the module's
`start` row (the fault's view job submitted) to its `recovered` row (a control probe published after the fault), and
windows run back to back while a phase lasts. Each window has a kind (`KINDS`):

- A turn window holds one turn of the shell's turn key J that the module posts where the fault has taken effect (the
  native loop: one every 500 ms while its child spins). In a metered fault's turn window the commit cancels the view
  job (`KN7:cancelled.revision_replaced`), a typed end.
- A limit window (metered faults only; wParam 8 to 10, `len(FAULTS)` plus the fault's number) posts no turn: the job
  runs to the limit that stops it in the product, with no commit, and the window counts when its end is one of
  `LIMIT_ENDS`: the meter's limit or the host's view deadline (`KN7:resource_limit.safety_stop`), whichever comes
  first. Under the product's view limits (2^32 steps, 2^32 bytes) the 60 s deadline always comes first
  (`LIMIT_FACTS`, in every record).

The phases run in `PHASES` order: each metered fault's limit windows, then its turn windows; then the other faults'
turn windows.

`post` waits for the LX-2 and LX-4 module rows in the shell's live log (`events.body.jsonl`, lx4.py), then for each
phase: posts it; runs LX-4 echo samples back to back (the S module's inside-body edit, timed on its first typed key, as
lx4.py's echo metric, every `ECHO_PACE_S`); counts the module's turns and the echo samples whose timed post falls
inside a window of this fault and kind, the open window included; posts phase 0 once its targets are reached (a limit
phase: `TARGET` echo samples; a metered fault's turn phase: `TARGET` turns; another fault's: both) or after
`TIME_BOX_S` (the module takes it after the window that runs); and waits for the module's `phase_end` row. After the
last phase it posts WM_CLOSE to the shell's window, as closing the window would, so that the shell seals its log and
launch.py writes launch.json (`--keep-open` leaves the shell running). It writes `lx2-posts.jsonl`
(`magic600-c9-lx2-posts-v1`): a `run` row, `phase` rows, `post` and `sample` rows as lx4.py's (with the fault and
kind), one `fault` row per phase with its counts, and a `close` row.

`analyze` reads the sealed `events.jsonl`, `lx2-posts.jsonl`, `launch.json`, `presentmon.csv` and the faults record:

- A window counts only when it is effective: it ended with `recovered` and the fault's view job ended refused. A turn
  window holds an accepted turn; a metered fault's turn came before its job ended; forced_kill killed at least one
  child; the native faults ran armed (their variable as the module sets it) and the native recursion and the allocator
  ceiling ended with the child's loss (`KN7:resource_limit.runtime_lost`); the native loop's child lived at least
  `LOOP_LIFE_MIN_S` before it exited. A limit window holds no turn row, and its job ended with one of the fault's
  `LIMIT_ENDS`.
- Condition 4, per fault: the key rows (charter E2, key_to_frame.py's rules and run checks) whose press lies inside an
  effective turn window of the fault: at least 100 confirmed, nearest-rank p95 at most 100 ms, with each key without a
  confirmed frame counted as one failed key above 100 ms (charter E2's counting, owner, 20 November 2026, item 2).
- Condition 3, per fault, over its effective limit windows (a metered fault) or turn windows (the others): LX-4's echo
  samples whose timed post lies inside one, timed as lx4.py times them (to the core-confirmed present of the frame its
  echo row names): at least 100, none failed, p95 at most 50 ms and p99 at most 67 ms; and the renderer gate over the
  core swap chain's presents whose QPC time lies inside one (`renderer_gate.judge` over PresentMon's
  msBetweenPresents): at least `GATE_FRAMES_MIN` presents, fps at least 30 and p99 at most 33.3 ms.
- Conditions 1 and 2: the faults record agrees, every fault and the control ran alone and in a session that stayed
  open and agreed with the control, and its injection device is the child launch.json names.

The verdict is `met` only when every condition holds for every fault; `not-met` when a condition with enough data
fails; `insufficient` otherwise; `invalid` when the run is (key_to_frame.py's run checks, a module error, posts or
fixtures of another run). Limitations (in every record): the whole run uses the injection child; the view jobs read
no host rows (NoHost), as LX-4's.
"""
from __future__ import annotations

import argparse
import csv
import json
import math
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import evidence  # noqa: E402
import key_to_frame  # noqa: E402
import lx4  # noqa: E402
from evidence import RunError, integer, require  # noqa: E402
from b412_summary import csv_integer, nearest_rank, number, statistics  # noqa: E402
from renderer_gate import CAPTURE_COLUMNS, FPS_MIN, P99_MAX_MS  # noqa: E402
import renderer_gate  # noqa: E402

FAULTS = ('alloc_metered', 'recursion_metered', 'loop_metered', 'forced_kill', 'alloc_ceiling', 'recursion_native',
          'loop_native')
METERED = FAULTS[:3]
KINDS = ('limit', 'turn')
# (wParam, fault, kind) in the order post runs them: a metered fault's limit windows, then its turn windows.
PHASES = tuple(phase for number, fault in enumerate(FAULTS, 1) for phase in (
    ((len(FAULTS) + number, fault, 'limit'), (number, fault, 'turn')) if fault in METERED else
    ((number, fault, 'turn'),)))
# The meter's typed end at its limit, per metered fault, and the host's typed end at its view deadline.
BUDGET = {'alloc_metered': 'KN7:budget.memory', 'recursion_metered': 'KN7:budget.memory',
          'loop_metered': 'KN7:budget.steps'}
SAFETY_STOP = 'KN7:resource_limit.safety_stop'
# The ends that make a limit window count: the limit that stops the job in the product, the meter's or the host's view
# deadline, whichever comes first (top integrator, 19 November 2026, option 1; the owner may override it).
LIMIT_ENDS = {fault: (code, SAFETY_STOP) for fault, code in BUDGET.items()}
# Stated facts about the limit windows (in every record, under `limit_windows`).
LIMIT_FACTS = [
    'the limit windows end at the host\'s 60 s view deadline (KN7:resource_limit.safety_stop), under the '
    'product\'s own view limits, with no commits',
    'c600-lx2\'s view_limits_probe (in process, release build, 19 November 2026, the CPU shared with a build) shows '
    'that the 2^32 meter limits are not reachable within 60 s: the loop runs about 6 M steps/s (2^28 steps in 44 s); '
    'the recursion reached 2^30 bytes in 58 s; the allocation\'s push grows quadratically, 2^18 bytes in 37 s']
NATIVE_LOST = ('alloc_ceiling', 'recursion_native')
# What the module sets in the shell's process before the job of a native fault (Lx2Module.Arm).
ARMED = {'alloc_ceiling': 'C600_LX2_CEILING=33554432', 'recursion_native': 'C600_LX2_FAULT=recursion@0',
         'loop_native': 'C600_LX2_FAULT=loop@0'}
RUNTIME_LOST = 'KN7:resource_limit.runtime_lost'
PHASE = 'Magic600.LX2.Phase'
MODULE = 'lx2'
POSTS = 'lx2-posts.jsonl'
POSTS_FORMAT = 'magic600-c9-lx2-posts-v1'
FAULTS_FORMAT = 'magic600-c9-lx2-faults-v1'
LAUNCH = 'launch.json'
WM_CLOSE = 0x0010
ECHO, ECHO_SIZE, ECHO_KIND = 'echo', 'S', 'inside-body'
ECHO_BAR_MS = lx4.BARS[ECHO][ECHO_SIZE]
E2_P95_MS = key_to_frame.P95_MAX_MS
OBSERVATIONS_MIN = 100
TARGET = 110
GATE_FRAMES_MIN = 300
TIME_BOX_S = 360.0
PHASE_END_TIMEOUT_S = 150.0
ECHO_PACE_S = 0.25
# Lx2Module's LeadMs, PaceMs and GapMs, and the host's VIEW_DEADLINE (renderer/bridge/host).
LEAD_S, PACE_S, GAP_S = 0.3, 0.5, 0.2
VIEW_DEADLINE_S = 60.0
LOOP_LIFE_MIN_S = 0.9 * VIEW_DEADLINE_S
# DRAFT: a window's recovery, the probe's run (with a fresh child after a fault), for the time estimate only.
RECOVERY_S = 0.3
SOURCES = ['RTE.42', 'RTE.43', 'RTE.49', 'CT-RTE-10', 'charter E2', 'renderer gate', 'option (a), 18 November 2026',
           'limit and turn windows, top integrator, 19 November 2026', 'owner decision 2026-11-20 (LP3 exit, item 2)']
BAR = {'text': ('RTE.42: every fault meets (1) journal and state hash equal to a control run, (2) a typed outcome '
                'with the session open, (3) the renderer gate and the LX-4 echo bar during the fault window, (4) at '
                'least 100 commits during fault windows under charter E2'),
       'rule': ('per fault, over its effective windows (start row to recovered row): E2 keys in turn windows '
                'confirmed, at least 100, nearest-rank p95 <= 100 ms with each unconfirmed key a failed key above '
                '100 ms; in limit windows (a metered '
                'fault, its job run to its limit with no commit) or turn windows (the others), LX-4 echo samples at '
                'least 100, none failed, p95 <= 50 ms and p99 <= 67 ms, and the gate over at least 300 presents, fps '
                '>= 30 and p99 <= 33.3 ms; conditions 1 and 2 from c600-lx2-faults with sessions'),
       'e2_p95_ms': E2_P95_MS, 'echo_bar_ms': list(ECHO_BAR_MS), 'gate': {'fps_min': FPS_MIN, 'p99_max_ms': P99_MAX_MS,
                                                                          'presents_min': GATE_FRAMES_MIN},
       'observations_min': OBSERVATIONS_MIN, 'faults': list(FAULTS),
       'limit_ends': {fault: list(codes) for fault, codes in LIMIT_ENDS.items()}, 'source': SOURCES}
LIMITATIONS = [
    'the whole run uses the injection child (c600-lx2-child, never shipped) in place of the build\'s runtime; '
    'launch.json and the faults record name the same child',
    'the view jobs read no host rows (NoHost), as LX-4\'s']
DRAFTS = ['fault order, message Magic600.LX2.Phase and the lx2 rows', 'window = start row to recovered row',
          'effective-window rules per fault', f'post targets {TARGET} turns and echo samples per fault, time box '
          f'{TIME_BOX_S:g} s', f'lead {LEAD_S:g} s, native-loop pace {PACE_S:g} s, gap {GAP_S:g} s',
          f'gate over presents inside windows, at least {GATE_FRAMES_MIN}', 'an in-window echo failure fails the bar',
          'one echo sample per fault every 0.25 s (S inside-body, first key)',
          'limit windows: wParam len(FAULTS) plus the fault\'s number, before its turn windows; the ends that count '
          '(LIMIT_ENDS, the integrator\'s choice; the owner may override it)',
          'a limit window\'s length: the host\'s view deadline (time estimate only)']


def echo_edit(edits):
    index = lx4.edit_index(edits, ECHO_SIZE, ECHO_KIND)
    if index is None:
        raise lx4.FixtureError(f'no {ECHO_KIND} edit at {ECHO_SIZE}')
    return index


def estimate():
    """The plan's seconds per phase, in PHASES order: (fault, kind, nominal, time-boxed bound) with its targets (DRAFT;
    a limit window runs to the host's view deadline, the end measured under the host's view limits)."""
    out = []
    one_turn = TARGET * (LEAD_S + RECOVERY_S + GAP_S)
    loop_turns = int((VIEW_DEADLINE_S - LEAD_S) / PACE_S) + 1
    loop = math.ceil(TARGET / loop_turns) * (VIEW_DEADLINE_S + RECOVERY_S + GAP_S)
    limit = math.ceil(TARGET * ECHO_PACE_S / VIEW_DEADLINE_S) * (VIEW_DEADLINE_S + RECOVERY_S + GAP_S)
    bound = round(TIME_BOX_S + VIEW_DEADLINE_S + RECOVERY_S + GAP_S)
    for _, fault, kind in PHASES:
        nominal = limit if kind == 'limit' else loop if fault == 'loop_native' else one_turn
        out.append((fault, kind, round(nominal), bound))
    return out


def targets_met(fault, kind, turns, echoes):
    """Whether a phase has its targets: a limit phase its echo samples, a metered fault's turn phase its turns (its
    condition 3 comes from the limit windows), another fault's phase both."""
    if kind == 'limit':
        return echoes >= TARGET
    if fault in METERED:
        return turns >= TARGET
    return turns >= TARGET and echoes >= TARGET


# ---- post (live: Windows, the running shell, the foreground) ----

class Live:
    """What the poster reads from the shell's live log: LX-4's endpoints and LX-2's windows, turns and phase ends, by
    fault and kind."""

    def __init__(self, tail):
        self.tail, self.endpoints, self.windows, self.turns = tail, set(), {}, {}
        self.phase_ends, self.errors = [], []

    def drain(self):
        for row in self.tail.rows():
            if row.get('t') == 'lx4':
                self.endpoints.add((row.get('sample'), row.get('endpoint')))
            elif row.get('t') == lx4.SELECTED:
                self.endpoints.add((row.get('sample'), (lx4.SELECTED, row.get('select'))))
            elif row.get('t') == MODULE and integer(row.get('qpc')):
                event, window = row.get('event'), row.get('window')
                phase = (row.get('fault'), row.get('kind'))
                if event == 'start' and integer(window):
                    self.windows[window] = {'phase': phase, 'start': row['qpc'], 'end': None}
                elif event == 'recovered' and window in self.windows:
                    self.windows[window]['end'] = row['qpc']
                elif event == 'turn' and row.get('ok') is True:
                    self.turns[phase] = self.turns.get(phase, 0) + 1
                elif event == 'phase_end':
                    self.phase_ends.append(phase)
                elif event == 'error':
                    self.errors.append(str(row.get('reason')))

    def inside(self, phase, qpc):
        return any(window['phase'] == phase and window['start'] <= qpc
                   and (window['end'] is None or qpc <= window['end']) for window in self.windows.values())


def module_rows(tail, names, clock, sleep, timeout=lx4.START_TIMEOUT_S):
    """The module rows of `names` in the shell's live log, as far as they came within `timeout`."""
    found, started = {}, clock()
    while True:
        for row in tail.rows():
            if row.get('t') == lx4.MODULE_ROW and row.get('module') in names and integer(row.get('window')):
                found.setdefault(row['module'], row)
        if set(names) <= found.keys() or clock() - started >= timeout:
            return found
        sleep(0.05)


def post_run(fixtures, run_dir, system, clock=time.monotonic, sleep=time.sleep, out=print, close=True):
    """Runs every phase in the shell whose LX-2 and LX-4 module rows are in run_dir's live log; 0 when every phase
    ended with its targets and every echo sample completed."""
    _, edits, digests = lx4.load_fixtures(fixtures)
    edit = echo_edit(edits)
    run_dir = Path(run_dir)
    tail = lx4.Tail(run_dir / lx4.LIVE)
    heads = module_rows(tail, (MODULE, 'lx4'), clock, sleep)
    if MODULE not in heads:
        out(f'the shell has written no lx2 module row to {lx4.LIVE} within {lx4.START_TIMEOUT_S:g} s; start it with '
            '--lx2-injection first')
        return 2
    if 'lx4' not in heads or heads['lx4']['window'] != heads[MODULE]['window']:
        out('the shell runs no LX-4 module in the same window; start it with --lx4 as well')
        return 2
    head = heads[MODULE]
    hwnd = head['window']
    names = {name: system.register(name) for name in (lx4.RESET, lx4.SELECT, PHASE)}
    live = Live(tail)
    code, sample_id = 0, 0
    with (run_dir / POSTS).open('x', encoding='utf-8', newline='\n') as log:
        def write(row):
            log.write(json.dumps(row, separators=(',', ':')) + '\n')
            log.flush()

        def phase(number, fault, kind):
            qpc = system.qpc()
            ok = system.post(hwnd, names[PHASE], number, 0)
            write({'t': 'phase', 'fault': fault, 'kind': kind, 'wparam': number, 'qpc': qpc, 'ok': ok})
            return ok

        write({'t': 'run', 'format': POSTS_FORMAT, 'run_id': head.get('run_id'),
               'build_identity': head.get('build_identity'), 'qpc_frequency': system.frequency(), 'synthetic': False,
               'faults': list(FAULTS), 'phases': [list(entry) for entry in PHASES], 'echo_edit': edit,
               'target': TARGET, **digests,
               'harness_sha256': lx4.sha256_bytes(Path(__file__).read_bytes())})
        for number, fault, kind in PHASES:
            key, label = (fault, kind), f'{fault} ({kind})'
            if not phase(number, fault, kind):
                out(f'{label}: the phase message did not post')
                return 4
            started, stopping, timed, failures = clock(), None, [], 0
            while True:
                live.drain()
                if live.errors:
                    out(f'the LX-2 module stopped: {live.errors[0]}')
                    phase(0, None, None)
                    return 4
                turns = live.turns.get(key, 0)
                echoes = sum(live.inside(key, qpc) for qpc in timed)
                if stopping is None and (targets_met(fault, kind, turns, echoes) or clock() - started >= TIME_BOX_S):
                    if not phase(0, None, None):
                        out(f'{label}: the stop message did not post')
                        return 4
                    stopping = clock()
                if stopping is not None:
                    if key in live.phase_ends:
                        break
                    if clock() - stopping >= PHASE_END_TIMEOUT_S:
                        out(f'{label}: the module wrote no phase_end within {PHASE_END_TIMEOUT_S:g} s')
                        return 4
                sample_id += 1
                begun = clock()
                status, foreground, qpc = echo_sample(system, hwnd, names, edits, edit, sample_id, key, live, write,
                                                      clock, sleep)
                if status == 'done' and foreground:
                    timed.append(qpc)
                else:
                    failures += 1
                sleep(max(0.0, ECHO_PACE_S - (clock() - begun)))
            live.drain()
            turns, echoes = live.turns.get(key, 0), sum(live.inside(key, qpc) for qpc in timed)
            seconds = round(clock() - started, 1)
            write({'t': 'fault', 'fault': fault, 'kind': kind, 'turns': turns, 'echo_samples': echoes,
                   'failed_samples': failures, 'seconds': seconds})
            out(f'{label}: {turns} turns, {echoes} echo samples in windows, {failures} failed samples, {seconds} s')
            if not targets_met(fault, kind, turns, echoes) or failures:
                code = 3
        if close:
            qpc = system.qpc()
            write({'t': 'close', 'qpc': qpc, 'ok': system.post(hwnd, WM_CLOSE, 0, 0)})
    return code


def echo_sample(system, hwnd, names, edits, edit, sample_id, phase, live, write, clock, sleep):
    """One LX-4 echo sample in the phase (fault, kind), posted as lx4.py posts it: (status, foreground, the timed
    post's QPC or None)."""
    fault, kind = phase
    sent, timed = lx4.messages(edits, {'id': sample_id, 'edit': edit, 'metric': ECHO})
    started, status, timed_qpc, selects = clock(), None, None, 0
    ok_all = system.alive(hwnd)

    def wait_for(key):
        while clock() - started < lx4.TIMEOUT_S:
            live.drain()
            if key in live.endpoints:
                return True
            sleep(0.005)
        return False

    for index, (message, wparam, lparam) in enumerate(sent):
        number = names.get(message, message)
        qpc = system.qpc()
        ok = system.post(hwnd, number, wparam, lparam)
        ok_all = ok_all and ok
        if index == timed:
            timed_qpc = qpc
        write({'t': 'post', 'sample': sample_id, 'fault': fault, 'kind': kind, 'index': index, 'message': number,
               'wparam': wparam, 'lparam': lparam, 'qpc': qpc, 'ok': ok, 'timed': index == timed})
        if message == lx4.RESET and not (ok_all and wait_for((sample_id, lx4.READY))):
            status = 'not-ready'
            break
        # As lx4.py: the replacement's keys only after the module acknowledged the Select.
        selects += message == lx4.SELECT
        if message == lx4.SELECT and not (ok_all and wait_for((sample_id, (lx4.SELECTED, selects)))):
            status = 'not-selected'
            break
    if status is None:
        status = 'done' if ok_all and wait_for((sample_id, ECHO)) else 'timeout'
    foreground = system.foreground(hwnd)
    write({'t': 'sample', 'sample': sample_id, 'fault': fault, 'kind': kind, 'status': status,
           'foreground': foreground})
    return status, foreground, timed_qpc


# ---- analyze ----

def first(rows):
    return rows[0] if rows else None


def read_windows(rows):
    """(windows by number with their rows by event, module errors) from the lx2 rows of a sealed log."""
    windows, errors = {}, []
    for row in rows:
        if row.get('t') != MODULE:
            continue
        require(isinstance(row.get('event'), str) and integer(row.get('qpc')) and integer(row.get('window')), 'bad-lx2')
        if row['event'] == 'error':
            errors.append(str(row.get('reason')))
        elif row['event'] not in ('phase', 'phase_end'):
            windows.setdefault(row['window'], {}).setdefault(row['event'], []).append(row)
    return windows, errors


def judge_window(number, events, frequency):
    """One window: its fault, kind, bounds, end code and whether it is effective (and why not)."""
    start, recovered = first(events.get('start', [])), first(events.get('recovered', []))
    if start is None or start.get('fault') not in FAULTS or start.get('kind') not in KINDS:
        return None
    fault, kind = start['fault'], start['kind']
    if kind == 'limit' and fault not in METERED:
        return None
    fault_end = first(events.get('fault_end', []))
    turns = [row for row in events.get('turn', []) if row.get('ok') is True]
    why = None
    if recovered is None:
        why = 'not-recovered'
    elif kind == 'limit':
        if events.get('turn'):
            why = 'turned'
        elif fault_end is None or fault_end.get('status') != 'refused':
            why = 'job-not-refused'
        elif fault_end.get('code') not in LIMIT_ENDS[fault]:
            why = 'limit-not-reached'
    elif not turns:
        why = 'no-turn'
    elif fault_end is None or fault_end.get('status') != 'refused':
        why = 'job-not-refused'
    elif fault in METERED and turns[0]['qpc'] > fault_end['qpc']:
        why = 'turn-after-the-job-ended'
    elif fault == 'forced_kill':
        kill = first(events.get('kill', []))
        if kill is None or not (integer(kill.get('killed')) and kill['killed'] >= 1):
            why = 'nothing-killed'
    elif fault in ARMED and start.get('armed') != ARMED[fault]:
        why = 'not-armed'
    elif fault in NATIVE_LOST and fault_end.get('code') != RUNTIME_LOST:
        why = 'child-not-lost'
    elif fault == 'loop_native':
        ended = first(events.get('child_exit', []))
        if ended is None or ended.get('found') is not True or not integer(ended.get('exit_qpc')):
            why = 'child-not-seen'
        elif (ended['exit_qpc'] - start['qpc']) / frequency < LOOP_LIFE_MIN_S:
            why = 'child-ended-early'
    return {'window': number, 'fault': fault, 'kind': kind, 'start': start['qpc'],
            'end': recovered['qpc'] if recovered else None, 'effective': why is None, 'why': why,
            'code': fault_end.get('code') if fault_end else None, 'turns': len(turns)}


def read_posts(rows):
    head = rows[0]
    require(head.get('t') == 'run' and head.get('format') == POSTS_FORMAT, 'bad-posts-header')
    require(integer(head.get('qpc_frequency')) and integer(head.get('echo_edit')), 'bad-posts-header')
    posts, done = {}, {}
    for row in rows[1:]:
        if row.get('t') == 'post':
            require(integer(row.get('sample')) and integer(row.get('index')) and integer(row.get('qpc')), 'bad-post')
            posts.setdefault(row['sample'], []).append(row)
        elif row.get('t') == 'sample':
            require(integer(row.get('sample')), 'bad-post')
            done[row['sample']] = row
        elif row.get('t') not in ('phase', 'fault', 'close'):
            raise RunError('bad-post')
    return head, posts, done


def read_presents(path, process_id, swap_chain):
    """(QPC time, ms between presents) of the declared swap chain's presents in a PresentMon v1 CSV."""
    columns, presents = CAPTURE_COLUMNS, []
    chain = swap_chain.strip().casefold()
    try:
        with Path(path).open(encoding='utf-8-sig', newline='') as stream:
            reader = csv.DictReader(stream, strict=True)
            header = reader.fieldnames or []
            require(all(columns[name] in header for name in ('process', 'swap_chain', 'time', 'frame_interval')),
                    'presentmon-missing-column')
            for row in reader:
                if csv_integer(row.get(columns['process'])) != process_id:
                    continue
                if (row.get(columns['swap_chain']) or '').strip().casefold() != chain:
                    continue
                try:
                    interval = float(row.get(columns['frame_interval']))
                except (TypeError, ValueError):
                    continue
                if number(interval) and interval > 0:
                    presents.append((csv_integer(row.get(columns['time'])), interval))
    except OSError:
        raise RunError('missing-file') from None
    except (csv.Error, UnicodeError):
        raise RunError('bad-field') from None
    return presents


def state_of(enough, met):
    return 'met' if enough and met else ('not-met' if enough else 'insufficient')


def echo_times(rows, posts, done, edits, edit, frames, presented, frequency, contradictions):
    """Per echo sample: ((fault, kind), anchor QPC, latency in ms or None, failure or None), timed as lx4.py times it;
    an echo row that names an unknown frame or a frame submitted before the post adds a contradiction, as in lx4.py."""
    endpoints = {}
    for row in rows:
        if row.get('t') == 'lx4':
            require(integer(row.get('sample')) and isinstance(row.get('endpoint'), str), 'bad-lx4')
            endpoints.setdefault((row['sample'], row['endpoint']), row)
    samples = []
    for sample, sent in sorted(posts.items()):
        timed = [row for row in sent if row.get('timed')]
        state = done.get(sample, {})
        anchor = timed[0]['qpc'] if timed else sent[0]['qpc']
        row = endpoints.get((sample, ECHO))
        failure = None
        if state.get('status') in ('not-ready', 'not-selected'):
            failure = state['status']
        elif len(timed) != 1 or not all(post.get('ok') for post in sent):
            failure = 'post-failed'
        elif state.get('status') != 'done':
            failure = 'timeout'
        elif state.get('foreground') is not True:
            failure = 'not-foreground'
        elif row is None or not integer(row.get('frame_id')):
            failure = 'no-endpoint'
        elif row.get('buffer_sha256') != edits[edit]['result_sha256']:
            failure = 'wrong-buffer'
        elif row['frame_id'] not in frames:
            failure = 'unknown-frame'
            contradictions.add('lx4-names-unknown-frame')
        elif frames[row['frame_id']]['qpc'] < anchor:
            failure = 'frame-before-post'
            contradictions.add('frame-before-post')
        elif row['frame_id'] not in presented:
            failure = 'unconfirmed'
        latency = None if failure else (presented[row['frame_id']]['display_qpc'] - anchor) * 1000 / frequency
        samples.append(((sent[0].get('fault'), sent[0].get('kind')), anchor, latency, failure))
    return samples


def analyze_run(rows, post_rows, edits, digests, *, launch=None, presents=None, present_ticks=None,
                faults_record=None):
    """The run's per-fault conditions and its reasons; `presents` from read_presents and `present_ticks` from
    key_to_frame.read_present_ticks, or None without a capture."""
    run = key_to_frame.analyze(rows, present_ticks)
    head = key_to_frame.check_rows(rows)
    frequency, reasons = head['qpc_frequency'], set(run['reasons'])
    posts_head, posts, done = read_posts(post_rows)
    if posts_head['qpc_frequency'] != frequency or posts_head.get('build_identity') != head['build_identity']:
        reasons.add('posts-do-not-match-shell')
    if (posts_head.get('manifest_sha256'), posts_head.get('edits_sha256')) != (digests['manifest_sha256'],
                                                                               digests['edits_sha256']):
        reasons.add('other-fixtures')
    if not (isinstance(launch, dict) and launch.get('lx4') is True and isinstance(launch.get('lx2_injection'), dict)):
        reasons.add('launch-record-missing')
    elif (launch.get('run_id'), launch.get('build_identity')) != (head['run_id'], head['build_identity']):
        reasons.add('launch-does-not-match-shell')
    windows, errors = read_windows(rows[1:])
    if errors:
        reasons.add('module-error')
    judged = [window for window in (judge_window(number, events, frequency) for number, events in
                                    sorted(windows.items())) if window is not None]
    frames, presented = key_to_frame.check_frames(rows[1:], set())
    keys = [row for row in rows[1:] if row['t'] == 'key']
    require(0 <= posts_head['echo_edit'] < len(edits), 'bad-posts-header')
    contradictions = set()
    echoes = echo_times(rows[1:], posts, done, edits, posts_head['echo_edit'], frames, presented, frequency,
                        contradictions)
    if contradictions:
        reasons.add('contradiction')
    per_fault = {}
    for fault in FAULTS:
        def effective(kind):
            return [window for window in judged if (window['fault'], window['kind']) == (fault, kind)
                    and window['effective']]

        def inside(windows, qpc):
            return any(window['start'] <= qpc <= window['end'] for window in windows)

        turn_windows = effective('turn')
        over = 'limit' if fault in METERED else 'turn'
        echo_windows = effective(over)
        entries = [entry for key, entry in zip(keys, run['per_key']) if inside(turn_windows, key['qpc'])]
        latencies = [entry['latency_ms'] for entry in entries if entry['status'] == 'confirmed']
        unconfirmed = len(entries) - len(latencies)
        stats = key_to_frame.e2_statistics(latencies, unconfirmed)
        e2 = {'over': 'turn', 'turns': sum(window['turns'] for window in turn_windows), 'confirmed': len(latencies),
              'unconfirmed': unconfirmed, 'stats': stats,
              'state': state_of(len(latencies) >= OBSERVATIONS_MIN, bool(stats and stats['p95_met']))}
        samples = [(latency, failure) for owner, anchor, latency, failure in echoes
                   if owner == (fault, over) and inside(echo_windows, anchor)]
        timed = [latency for latency, failure in samples if failure is None]
        failed = sorted(failure for latency, failure in samples if failure is not None)
        echo = {'over': over, 'timed': len(timed), 'failed': failed, 'stats': statistics(timed) if timed else None,
                'state': state_of(len(timed) >= OBSERVATIONS_MIN,
                                  not failed and timed and nearest_rank(timed, 95) <= ECHO_BAR_MS[0]
                                  and nearest_rank(timed, 99) <= ECHO_BAR_MS[1])}
        if presents is None:
            gate = {'over': over, 'presents': 0, 'state': 'insufficient', 'reason': 'presentmon-missing'}
        else:
            values = [interval for qpc, interval in presents if inside(echo_windows, qpc)]
            judged_gate = renderer_gate.judge(values) if values else None
            gate = {'over': over, 'presents': len(values), 'result': judged_gate,
                    'state': state_of(len(values) >= GATE_FRAMES_MIN, bool(judged_gate and judged_gate['met']))}
        by_kind = {}
        for kind in KINDS if fault in METERED else ('turn',):
            mine = [window for window in judged if (window['fault'], window['kind']) == (fault, kind)]
            ineffective, ends = {}, {}
            for window in mine:
                if not window['effective']:
                    ineffective[window['why']] = ineffective.get(window['why'], 0) + 1
                ends[str(window['code'])] = ends.get(str(window['code']), 0) + 1
            counted = [window for window in mine if window['effective']]
            by_kind[kind] = {'windows': len(mine), 'effective': len(counted),
                             'ineffective': dict(sorted(ineffective.items())), 'ends': dict(sorted(ends.items())),
                             'seconds': round(sum(window['end'] - window['start'] for window in counted)
                                              / frequency, 3)}
        per_fault[fault] = {'windows': by_kind, 'e2': e2, 'echo': echo, 'gate': gate}
    conditions_1_2 = judge_faults(faults_record, launch)
    return {'run_id': head['run_id'], 'build_identity': head['build_identity'], 'synthetic': head['synthetic'],
            'status': 'invalid' if reasons else 'valid', 'reasons': sorted(reasons), 'module_errors': errors,
            'e2_run': {'keys': run['keys'], 'confirmed': run['confirmed'], 'unconfirmed': run['unconfirmed'],
                       'status': run['status']},
            'faults': per_fault, 'conditions_1_2': conditions_1_2}


def judge_faults(record, launch):
    """Conditions 1 and 2 from c600-lx2-faults' record: {'state', 'reasons'}."""
    if not (isinstance(record, dict) and record.get('format') == FAULTS_FORMAT):
        return {'state': 'insufficient', 'reasons': ['faults-record-missing']}
    reasons = []
    trials = {row.get('trial'): row for row in record.get('trials') or [] if isinstance(row, dict)}
    sessions = {row.get('trial'): row for row in record.get('sessions') or [] if isinstance(row, dict)}
    for trial in ('control', *FAULTS):
        if trial not in trials:
            reasons.append(f'trial-missing:{trial}')
        elif trials[trial].get('agrees') is not True:
            reasons.append(f'trial-disagrees:{trial}')
        if trial not in sessions:
            reasons.append(f'session-missing:{trial}')
        elif not (sessions[trial].get('open') is True and sessions[trial].get('agrees_with_control') is True):
            reasons.append(f'session-disagrees:{trial}')
    if record.get('agrees') is not True:
        reasons.append('faults-disagree')
    device = ((record.get('runtime') or {}).get('injection_device') or {}) if isinstance(record.get('runtime'),
                                                                                         dict) else {}
    injection = (launch or {}).get('lx2_injection') if isinstance(launch, dict) else None
    if not isinstance(injection, dict) or not device.get('sha256') or injection.get('sha256') != device['sha256']:
        reasons.append('other-injection-child')
    if not reasons:
        return {'state': 'met', 'reasons': []}
    failed = any('disagree' in reason for reason in reasons)
    return {'state': 'not-met' if failed else 'insufficient', 'reasons': reasons}


def judge(run):
    """(verdict, reasons) over one analyzed run."""
    if run['status'] != 'valid':
        return 'invalid', run['reasons']
    states = [run['conditions_1_2']['state']]
    reasons = [f'conditions-1-2:{reason}' for reason in run['conditions_1_2']['reasons']]
    for fault, result in run['faults'].items():
        for condition in ('e2', 'echo', 'gate'):
            state = result[condition]['state']
            states.append(state)
            if state != 'met':
                reasons.append(f'{fault}:{condition}-{state}')
    if all(state == 'met' for state in states):
        return 'met', []
    return ('not-met' if 'not-met' in states else 'insufficient'), reasons


def read_json(path):
    try:
        value = json.loads(Path(path).read_text(encoding='utf-8'))
    except (OSError, ValueError, UnicodeError):
        return None
    return value if isinstance(value, dict) else None


def build(fixtures, directory, faults_path, argv, synthetic_flag=False):
    _, edits, digests = lx4.load_fixtures(fixtures)
    directory = Path(directory)
    events = directory / key_to_frame.EVENTS
    launch, faults_record = read_json(directory / LAUNCH), read_json(faults_path)
    try:
        rows = evidence.read_jsonl(events)
        head = key_to_frame.check_rows(rows)
        presents = ticks = None
        if (directory / key_to_frame.CAPTURE).is_file():
            require(integer(head.get('process_id')) and isinstance(head.get('swap_chain'), str)
                    and bool(head['swap_chain'].strip()), 'presentmon-undeclared')
            capture = directory / key_to_frame.CAPTURE
            presents = read_presents(capture, head['process_id'], head['swap_chain'])
            ticks = key_to_frame.read_present_ticks(capture, head['process_id'], head['swap_chain'])
        run = analyze_run(rows, evidence.read_jsonl(directory / POSTS), edits, digests, launch=launch,
                          presents=presents, present_ticks=ticks, faults_record=faults_record)
    except RunError as error:
        run = {'run_id': directory.name, 'build_identity': None, 'synthetic': None, 'status': 'unreadable',
               'reasons': [str(error)]}
    verdict, reasons = judge(run) if run['status'] != 'unreadable' else ('invalid', run['reasons'])
    inputs = [evidence.file_entry(f'{run["run_id"]}:{name}', directory / name)
              for name in (key_to_frame.EVENTS, POSTS, LAUNCH, key_to_frame.CAPTURE) if (directory / name).is_file()]
    if Path(faults_path).is_file():
        inputs.append(evidence.file_entry('faults', faults_path))
    synthetic = evidence.is_synthetic([events], [run.get('synthetic')], synthetic_flag)
    result = {'fixtures': digests, 'run': run, 'limitations': LIMITATIONS,
              'limit_windows': {'ends': {fault: list(codes) for fault, codes in LIMIT_ENDS.items()},
                                'facts': LIMIT_FACTS}}
    return evidence.make_record(
        'lx2-fault-windows', 'performance', synthetic=synthetic, build_identity=run.get('build_identity'),
        inputs=inputs, tools=evidence.tool_versions(__file__, ('c9/key_to_frame.py', 'c9/lx4.py', 'renderer_gate.py')),
        command=evidence.portable_command(__file__, argv), bar=BAR, result=result, verdict=verdict, reasons=reasons,
        drafts=DRAFTS)


def main(argv=None, system_factory=lx4.Windows):
    argv = sys.argv[1:] if argv is None else argv
    parser = argparse.ArgumentParser(description="LX-2's fault windows in the product shell: posts and analysis.")
    commands = parser.add_subparsers(dest='command', required=True)
    poster = commands.add_parser('post', help="run every fault's phase in the running shell")
    poster.add_argument('--fixtures', type=Path, required=True, help="c600-lx4's fixtures, the shell's --lx4-fixtures")
    poster.add_argument('--run', type=Path, help="the shell's run directory")
    poster.add_argument('--keep-open', action='store_true', help='leave the shell running after the last fault')
    poster.add_argument('--dry-run', action='store_true', help='read the fixtures and print the plan; post nothing')
    analysis = commands.add_parser('analyze', help='judge one run directory against RTE.42')
    analysis.add_argument('--fixtures', type=Path, required=True)
    analysis.add_argument('run', type=Path)
    analysis.add_argument('--faults', type=Path, required=True, help="c600-lx2-faults' record (conditions 1 and 2)")
    analysis.add_argument('--out', type=Path, help='write the evidence record here (never replaces a file)')
    analysis.add_argument('--synthetic', action='store_true')
    args = parser.parse_args(argv)
    try:
        if args.command == 'post':
            if args.dry_run:
                _, edits, digests = lx4.load_fixtures(args.fixtures)
                sent, timed = lx4.messages(edits, {'id': 1, 'edit': echo_edit(edits), 'metric': ECHO})
                plan = estimate()
                print(f'faults: {", ".join(FAULTS)} (Magic600.LX2.Phase wParam 1 to {len(FAULTS)} for turn windows, '
                      f'{len(FAULTS) + 1} to {len(FAULTS) + len(METERED)} for the metered faults\' limit windows, '
                      '0 stops)')
                print(f'per phase, time box {TIME_BOX_S:g} s: a limit phase {TARGET} echo samples in windows, a '
                      f'metered fault\'s turn phase {TARGET} turns, another fault\'s both; echo sample: {len(sent)} '
                      f'messages, timed {timed}; fixtures {digests["manifest_sha256"]}')
                print('limit windows count with: ' + '; '.join(f'{fault} {" or ".join(codes)}'
                                                               for fault, codes in LIMIT_ENDS.items()))
                for fault, kind, nominal, bound in plan:
                    print(f'  {fault} ({kind} windows): about {nominal} s, at most {bound} s')
                print(f'shell time: about {math.ceil(sum(entry[2] for entry in plan) / 60)} min, at most '
                      f'{math.ceil(sum(entry[3] for entry in plan) / 60)} min (DRAFT)')
                print('dry run: no window was looked for and nothing was posted')
                return 0
            if args.run is None:
                parser.error('post needs --run unless --dry-run')
            return post_run(args.fixtures, args.run, system_factory(), close=not args.keep_open)
        record = build(args.fixtures, args.run, args.faults, argv, args.synthetic)
    except lx4.FixtureError as error:
        print(f'lx2: {error}', file=sys.stderr)
        return 2
    run = record['result']['run']
    print(f"{run['run_id']}: {run['status']}{' (' + ', '.join(run['reasons']) + ')' if run['reasons'] else ''}")
    for fault, result in (run.get('faults') or {}).items():
        counts = ', '.join(f"{entry['effective']} of {entry['windows']} {kind} windows"
                           for kind, entry in result['windows'].items())
        print(f"  {fault}: {counts} effective; E2 {result['e2']['state']} ({result['e2']['confirmed']} keys), echo "
              f"{result['echo']['state']} ({result['echo']['timed']}), gate {result['gate']['state']} "
              f"({result['gate']['presents']} presents)")
    if 'conditions_1_2' in run:
        print(f"  conditions 1 and 2: {run['conditions_1_2']['state']}")
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
