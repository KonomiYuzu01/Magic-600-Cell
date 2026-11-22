"""LX-4's harness on route B (C9; chapter 11, RTE.47 to RTE.53, CT-RTE-12, CT-EV1-06). Standard library only.

    python -B tools/perf/c9/lx4.py schedule --fixtures DIR
    python -B tools/perf/c9/lx4.py post --fixtures DIR --puzzle jumble-off|jumble-on --run DIR
        [--turn-key KeyJ|KeyK|KeyL] [--dry-run]
    python -B tools/perf/c9/lx4.py analyze --fixtures DIR RUN_DIR [RUN_DIR ...] [--e2 RECORD] [--gate SUMMARY]
        [--gate-reference SUMMARY] [--out RECORD.json] [--synthetic]

The fixtures are the folder `c600-lx4-fixtures --out` writes (MANIFEST.json, EDITS.json and the modules); every
module's and edit's digest is checked before anything else happens. `schedule` prints the sample accounting and runs
nothing. `post` is RTE.51 step 5's timed input: owned-window messages posted to the product shell's window queue, one
puzzle per run directory (step 3: the shell started with Jumble off or on, at a fixture's end state, on fresh
isolated data). `post --dry-run` reads the fixtures and prints what it would post; it finds no window and posts
nothing. Without `--dry-run`, `post` needs Windows, the running product shell and the foreground (the owner starts it
from the desktop). `analyze` matches every timed post to the first presented frame showing its result by frame id
(step 5) and judges RTE.49's bars with RTE.53's rule. The tests run `schedule`, `post --dry-run` and `analyze` only.

Schedule (DRAFT order, RTE.51 step 7, RTE.52). Per puzzle: one smoke series (one sample per metric and size, one W
sample and one turn), then three series. A series holds, per size S, M and L and per metric, 100 samples in a row,
then 100 W samples (W's dependent re-check, additional work), with one E2 turn after every 19th sample (step 8: at
least 100 turns per series). S/M/L samples: 6 metrics x 3 sizes x 2 puzzles x 3 series x 100 = 10,800.

Metrics and the script's edit that drives each (DRAFT mapping of RTE.49's intervals to RTE.51's four edit kinds):

| metric | edit kind | timed message | endpoint |
| --- | --- | --- | --- |
| echo | inside-body | the first typed key | the frame showing that key |
| syntax | syntax-break | the deleting key | syntax feedback shown |
| completion | completion-trigger | the last key | the completion list shown |
| diagnostics | exported-signature | the burst's last key | type and effect diagnostics shown |
| stale | inside-body | the last key | stale state shown on a view (charter D2) |
| rerun | inside-body | the last key | the reference view re-run and its updated view presented |
| dependents (W) | exported-signature at W00 | the burst's last key | W00's dependents re-checked and shown |

Messages (DRAFT protocol; the product shell implements it): registered `Magic600.LX4.Reset` (wParam the edit's index in
EDITS.json, with `RERUN_FLAG` (bit 16) added for a rerun sample; lParam the sample id): the editor loads the edit's
target baseline and checks its digest, and the first frame that draws the baseline writes the sample's `ready` row (a
handshake, no metric). The harness posts the rest only after it reads that row, so no timed message waits behind the
Reset's work; while an earlier sample's re-run runs, the row waits for it. Then for each replacement, in descending
offset order: registered `Magic600.LX4.Select` (wParam start, lParam end, UTF-8 byte offsets in the current buffer),
which the module acknowledges with an `lx4-select` row ({"t": "lx4-select", "sample", "select"}, the Select's ordinal
in the sample from 1) once the editor has applied it; the harness posts the replacement's keys only after it reads that
row, because on 20 Nov 2026 (U8) the shell applied Selects after keys posted later. Then WM_CHAR per UTF-16 unit of the insert (a newline as 13, Enter; scan code 0), or WM_KEYDOWN and WM_KEYUP of VK_BACK
with its scan code 0x0E when the insert is empty. Automatic insertions (indent, closing brackets) are off, so the buffer
equals the script's result. A turn is WM_KEYDOWN and WM_KEYUP of one of the shell's turn keys (`--turn-key`, default
KeyJ: physical J, K and L are twists 1, 2 and 3), with the key's scan code in lParam bits 16 to 23: the shell routes by
Godot's physical keycode, which Godot takes from the scan code, so a bare virtual-key code with scan code 0 does not
turn. Godot's loop translates a turn's key-down into a WM_CHAR with the same scan code; the shell's LX-4 module keeps
such characters out of the buffer, and the harness's own WM_CHARs carry scan code 0, so the two never mix.

The rerun endpoint (RTE.50; C9, 20 Nov 2026). The reference view is the edited S, M or L module run from its
`exported` function, which imports nothing and reads no host row. After a rerun sample's last input, the shell runs
the final buffer as the host's view job (`m6h_run_view`, the language runtime with no host rows). The `rerun` row names
the first core-confirmed frame that drew the run's publication (`m6h_view_result`), with the run's source digest as
`buffer_sha256`. The host does not draw the run's value yet, so that frame shows the reference publication the run
delivered (a limitation, below). A refused run writes no row, so the sample times out.

Logs. The harness writes `lx4-posts.jsonl` in the run directory (format `magic600-c9-lx4-posts-v1`): a `run` row, one
`post` row per message (`sample`, `index`, `message`, `qpc` read just before PostMessageW, `ok`) and one `sample` row
per sample (`status` done, timeout, not-selected when a Select's `lx4-select` row did not come within the timeout and
nothing after it was posted, or not-ready when no `ready` row came within the timeout and nothing after the Reset
was posted; `foreground`). The shell writes `events.jsonl` (`magic600-c9-shell-events-v1`, key_to_frame.py) whose `run`
row also carries `puzzle` and `window` (the HWND, an integer), with frame, presented and key rows as for E2, and
`{"t": "lx4", "sample": int, "endpoint": str, "frame_id": int, "buffer_sha256": str}` when a frame first shows an
endpoint. The shell seals `events.jsonl` only when it closes, because the run row is known only then; while it runs,
its rows go to `events.body.jsonl` in buffered blocks. So `post` reads that live body, and the window, run id, build and
puzzle from the module's own row, `{"t": "module", "module": "lx4", "window": int, "run_id": str, "build_identity":
str, "puzzle": str, "qpc_frequency": int}`, which the module writes when it starts (`post` waits up to
`START_TIMEOUT_S` for it).

Rules (DRAFT, as key_to_frame.py). A sample's time runs from its timed post's QPC to the core-confirmed display QPC of
the frame its endpoint row names. A sample whose endpoint frame has no confirmed display gets no time and is counted
apart, in the confirmation gap: a known limit, published with its count per run and cell (`confirmation_gap`), and its
run stays valid. A cell's p95 and p99 are taken over its samples with a confirmed frame (owner decision, 22 November
2026, item 10, which replaces item 2's counting of such a sample as a failed one; charter E2 keeps its own rule). A
buffer that differs from the edit's result (the correctness control, step 7), a failed post, a timeout, a missing
`ready` row, a lost foreground, a frame submitted before its post and the contradictions key_to_frame.py refuses make
the run invalid. An invalid run is kept and repeated whole; it is never pooled. Each puzzle needs exactly one valid
run, both of one build.

Verdict (RTE.53). Per puzzle, series, size and metric: nearest-rank p95 and p99 against RTE.49's bars, each bar below
100 ms raised to 100 ms (owner decision, 22 November 2026, item 1). `met` only when every bar holds in each of the
three series at S, M and L on both puzzles, W's bar holds, all 10,800 S/M/L samples are accounted for (timed or in
the confirmation gap), and
the E2 record (key_to_frame.py over these same run directories, matched by events.jsonl digests) and a renderer gate
summary of the same build are met. The decision names RTE.53's case: `fail-s-m` (a miss at S or M), `fail-gate-or-e2`,
`owner-l-or-w` (misses only at L or W go to the owner with the numbers, never an all-bars pass) or `all-bars`. The gate
summary is checked for its build and verdicts only; that it comes from these runs' captures is the integrator's check
(`gate_same_run_checked` false). renderer_gate.py reads acceptance builds only, never the product build these runs use,
so no summary passes the build check (C9, 20 Nov 2026). `--gate-reference` records such a summary in `gate_reference`
with its digest, builds and verdicts, and among the inputs. It counts as the gate only when the owner accepted that
summary for runs of that build (`ACCEPTED_GATE_REFERENCES`: U4's W3 summary for the C9 sitting's U8 runs, owner
decision, 22 November 2026, item 2) and its every scene is met; otherwise the reason reads `gate-summary-other-build`
instead of `gate-summary-missing`.

Limitations (stated in every record, `limitations`). Top integrator, 19 November 2026: the module set's views read no
host rows on either puzzle (the host runs them with `NoHost`), so jumble-off and jumble-on time the same rerun path.
RTE.51 does not require jumbling rows, and the bars are unchanged. C9, 20 November 2026: the rerun endpoint's frame
draws the run's reference publication, not its value, which the host does not draw yet.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import evidence  # noqa: E402
import key_to_frame  # noqa: E402
from evidence import RunError, integer, require  # noqa: E402
from b412_summary import statistics  # noqa: E402

POSTS = 'lx4-posts.jsonl'
# The shell's live log: its rows while it runs, before it seals events.jsonl at close (EventLog.BodyName).
LIVE = 'events.body.jsonl'
MODULE_ROW = 'module'
START_TIMEOUT_S = 60.0
POSTS_FORMAT = 'magic600-c9-lx4-posts-v1'
FIXTURES_FORMAT = 'magic600-c9-lx4-fixtures-v1'
EDITS_FORMAT = 'magic600-c9-lx4-edits-v1'
PUZZLES = ('jumble-off', 'jumble-on')
SIZES = ('S', 'M', 'L')
SERIES = 3
PER_SERIES = 100
TURN_EVERY = 19
TOTAL_SML = 10_800
TIMEOUT_S = 10.0
PACE_S = 1.0
# metric: (edit kind, timed message: 'first' or 'last' key)
METRICS = {'echo': ('inside-body', 'first'), 'syntax': ('syntax-break', 'last'),
           'completion': ('completion-trigger', 'last'), 'diagnostics': ('exported-signature', 'last'),
           'stale': ('inside-body', 'last'), 'rerun': ('inside-body', 'last')}
W_METRIC = ('dependents', 'exported-signature', 'last')
# RTE.49: (p95, p99) in ms per metric and size; W's dependent re-check. Every bar below 100 ms is 100 ms (owner
# decision, 22 November 2026, item 1): typed echo at S, M and L was 50/67 and syntax feedback at S and M was 67/100.
BARS = {'echo': {'S': (100, 100), 'M': (100, 100), 'L': (100, 100)},
        'syntax': {'S': (100, 100), 'M': (100, 100), 'L': (100, 150)},
        'completion': {'S': (100, 150), 'M': (100, 150), 'L': (150, 250)},
        'diagnostics': {'S': (150, 250), 'M': (200, 300), 'L': (500, 750)},
        'stale': {'S': (100, 150), 'M': (100, 150), 'L': (100, 150)},
        'rerun': {'S': (250, 400), 'M': (250, 400), 'L': (500, 750)},
        'dependents': {'W': (1000, 1500)}}
RESET, SELECT = 'Magic600.LX4.Reset', 'Magic600.LX4.Select'
# A rerun sample's Reset carries this flag in wParam beside the edit's index (the library's LX4_RESET_RERUN).
RERUN_FLAG = 1 << 16
READY = 'ready'
# The module's acknowledgment of an applied Select: {"t": SELECTED, "sample", "select"} (C9 fix, 20 Nov 2026).
SELECTED = 'lx4-select'
WM_KEYDOWN, WM_KEYUP, WM_CHAR, VK_BACK = 0x0100, 0x0101, 0x0102, 0x08
KEYUP_LPARAM = 0xC0000001
SCAN_BACK = 0x0E
# The shell's turn keys (C9 product, 19 November 2026): W3C code -> (virtual-key code, scan code). Shift gives the
# inverse twist; the harness turns forward only.
TURN_KEYS = {'KeyJ': (0x4A, 0x24), 'KeyK': (0x4B, 0x25), 'KeyL': (0x4C, 0x26)}
TURN_KEY = 'KeyJ'
# Renderer gate summaries of another build that the owner accepted as the gate of runs of one build: (the summary's
# file SHA-256, the runs' build identity) -> the decision. U4's W3 summary of the C9 sitting, for its U8 runs (same
# commit, same night; owner decision, 22 November 2026, item 2).
ACCEPTED_GATE_REFERENCES = {
    ('c7a43ae68505f1c4df5d59400189070346765dad5a119333533e6a2ac968a0ca', 'c9-product-2efda50d54592f20'):
        'owner decision 2026-11-22 (LX bars), item 2: U4 W3 summary of the C9 sitting'}
BAR = {'text': ('RTE.49 p95/p99 bars per metric and size (S/M/L), W dependents 1,000/1,500 ms, every bar below 100 ms '
                'raised to 100 ms (owner, 22 November 2026, item 1); RTE.53: every bar in each of three series at S, '
                'M and L on both puzzles, with the renderer gate and charter E2 in the same runs'),
       'rule': ('timed post to the core-confirmed present of the first frame showing the endpoint, matched by frame '
                'id; nearest-rank p95 and p99 per puzzle, series, size and metric over the samples with a confirmed '
                'frame; 10,800 S/M/L samples accounted for, each sample without a confirmed frame counted apart in '
                'the confirmation gap, a known limit (owner, 22 November 2026, item 10)'),
       'bars_ms': BARS, 'samples': TOTAL_SML,
       'source': ['RTE.47', 'RTE.48', 'RTE.49', 'RTE.51', 'RTE.52', 'RTE.53', 'CT-RTE-12', 'CT-EV1-06',
                  'owner decision 2026-11-22 (LX bars, items 1, 2 and 10)']}
GAP_SOURCE = 'owner decision 2026-11-22 (LX bars), item 10'
LIMITATIONS = ['the module set\'s views read no host rows on either puzzle (NoHost), so jumble-off and jumble-on time '
               'the same rerun path; RTE.51 does not require jumbling rows, and the bars are unchanged',
               'the rerun endpoint\'s frame is the first confirmed frame that drew the run\'s reference publication; '
               'the host does not draw the run\'s value yet']
DRAFTS = ['metric to edit-kind mapping', 'schedule order and smoke series', 'message protocol Magic600.LX4.*',
          'lx4 rows in magic600-c9-shell-events-v1',
          f'post timeout {TIMEOUT_S:g} s, pace {PACE_S:g} s', 'gate summary checked for build and verdict only',
          'fixture set DRAFT until RTE.51 step 1 freezes it']


class FixtureError(Exception):
    pass


def sha256_bytes(data):
    return hashlib.sha256(data).hexdigest()


def load_fixtures(directory):
    """(manifest, edits, digests) after checking every module and edit; FixtureError otherwise."""
    directory = Path(directory)
    try:
        manifest_bytes = (directory / 'MANIFEST.json').read_bytes()
        edits_bytes = (directory / 'EDITS.json').read_bytes()
        manifest, script = json.loads(manifest_bytes), json.loads(edits_bytes)
    except (OSError, ValueError, UnicodeError) as error:
        raise FixtureError(f'fixtures unreadable: {type(error).__name__}') from None
    if manifest.get('format') != FIXTURES_FORMAT or script.get('format') != EDITS_FORMAT:
        raise FixtureError('not c600-lx4 fixtures')
    modules = {}
    for module in manifest.get('modules', []):
        try:
            data = (directory / module['path']).read_bytes()
        except (OSError, KeyError, TypeError):
            raise FixtureError(f"module {module.get('name')} missing") from None
        if sha256_bytes(data) != module.get('sha256'):
            raise FixtureError(f"module {module['name']} differs from the manifest")
        modules[module['name']] = data.decode('utf-8')
    edits = script.get('edits', [])
    for index, edit in enumerate(edits):
        source = modules.get(edit.get('target'))
        if source is None or sha256_bytes(source.encode()) != edit.get('base_sha256'):
            raise FixtureError(f'edit {index} names no module of the manifest')
        if sha256_bytes(apply(source, edit).encode()) != edit.get('result_sha256'):
            raise FixtureError(f'edit {index} does not give its result')
    for size in (*SIZES, 'W00'):
        for kind in {kind for kind, _ in METRICS.values()}:
            if edit_index(edits, size, kind) is None:
                raise FixtureError(f'no {kind} edit at {size}')
    return manifest, edits, {'manifest_sha256': sha256_bytes(manifest_bytes), 'edits_sha256': sha256_bytes(edits_bytes),
                             'modules': len(modules), 'edits': len(edits)}


def apply(source, edit):
    data = source.encode('utf-8')
    for replacement in sorted(edit['replacements'], key=lambda r: -r['offset']):
        start, delete = replacement['offset'], replacement['delete'].encode('utf-8')
        if data[start:start + len(delete)] != delete:
            raise FixtureError(f"edit {edit.get('target')} {edit.get('kind')}: preimage differs")
        data = data[:start] + replacement['insert'].encode('utf-8') + data[start + len(delete):]
    return data.decode('utf-8')


def edit_index(edits, target, kind):
    found = [index for index, edit in enumerate(edits) if edit.get('target') == target and edit.get('kind') == kind]
    return found[0] if len(found) == 1 else None


def schedule(edits, puzzle):
    """The run's samples in order: dicts with id, series (0 = smoke), size, metric, edit (index) or a turn."""
    samples = []

    def add(series, size, metric, kind):
        samples.append({'id': len(samples), 'puzzle': puzzle, 'series': series, 'size': size, 'metric': metric,
                        'edit': None if kind is None else edit_index(edits, 'W00' if size == 'W' else size, kind)})

    for size in SIZES:
        for metric, (kind, _) in METRICS.items():
            add(0, size, metric, kind)
    add(0, 'W', W_METRIC[0], W_METRIC[1])
    add(0, None, 'turn', None)
    for series in range(1, SERIES + 1):
        timed = 0
        blocks = [(size, metric, kind) for size in SIZES for metric, (kind, _) in METRICS.items()]
        blocks.append(('W', W_METRIC[0], W_METRIC[1]))
        for size, metric, kind in blocks:
            for _ in range(PER_SERIES):
                add(series, size, metric, kind)
                timed += 1
                if timed % TURN_EVERY == 0:
                    add(series, None, 'turn', None)
    return samples


def timing_of(metric):
    return W_METRIC[2] if metric == W_METRIC[0] else METRICS[metric][1]


def key_lparams(scan):
    """(WM_KEYDOWN lParam, WM_KEYUP lParam) of one press of the key with `scan`: repeat count 1 and the scan code in
    bits 16 to 23; the key-up also sets the previous-state and transition bits."""
    return 1 | scan << 16, KEYUP_LPARAM | scan << 16


def messages(edits, sample, turn_key=TURN_KEY):
    """[(message name or number, wParam, lParam)] and the timed message's index."""
    if sample['metric'] == 'turn':
        if turn_key not in TURN_KEYS:
            raise FixtureError(f"{turn_key!r} is not one of the shell's turn keys ({', '.join(TURN_KEYS)})")
        vk, scan = TURN_KEYS[turn_key]
        down, up = key_lparams(scan)
        return [(WM_KEYDOWN, vk, down), (WM_KEYUP, vk, up)], 0
    edit = edits[sample['edit']]
    flag = RERUN_FLAG if sample['metric'] == 'rerun' else 0
    out, keys = [(RESET, sample['edit'] | flag, sample['id'])], []
    for replacement in sorted(edit['replacements'], key=lambda r: -r['offset']):
        start = replacement['offset']
        out.append((SELECT, start, start + len(replacement['delete'].encode('utf-8'))))
        if replacement['insert']:
            units = replacement['insert'].replace('\n', '\r').encode('utf-16-le')
            for unit in range(0, len(units), 2):
                keys.append(len(out))
                out.append((WM_CHAR, int.from_bytes(units[unit:unit + 2], 'little'), 1))
        else:
            keys.append(len(out))
            down, up = key_lparams(SCAN_BACK)
            out.extend([(WM_KEYDOWN, VK_BACK, down), (WM_KEYUP, VK_BACK, up)])
    return out, keys[0] if timing_of(sample['metric']) == 'first' else keys[-1]


def accounting(samples_by_puzzle):
    timed = [sample for samples in samples_by_puzzle for sample in samples if sample['series'] > 0]
    return {'sml_samples': sum(sample['size'] in SIZES for sample in timed),
            'w_samples': sum(sample['size'] == 'W' for sample in timed),
            'turns': sum(sample['metric'] == 'turn' for sample in timed),
            'smoke_samples': sum(sample['series'] == 0 for samples in samples_by_puzzle for sample in samples)}


# ---- post (live: Windows, the running shell, the foreground) ----

class Windows:
    """The few user32/kernel32 calls the poster makes. Created only by a live `post`."""

    def __init__(self):
        import ctypes
        from ctypes import wintypes
        self.ctypes, self.user32, self.kernel32 = ctypes, ctypes.WinDLL('user32'), ctypes.WinDLL('kernel32')
        self.user32.PostMessageW.argtypes = (wintypes.HWND, wintypes.UINT, wintypes.WPARAM, wintypes.LPARAM)
        self.user32.PostMessageW.restype = wintypes.BOOL
        self.user32.RegisterWindowMessageW.argtypes = (wintypes.LPCWSTR,)
        self.user32.RegisterWindowMessageW.restype = wintypes.UINT
        self.user32.GetForegroundWindow.restype = wintypes.HWND
        self.user32.IsWindow.argtypes = (wintypes.HWND,)

    def register(self, name):
        return self.user32.RegisterWindowMessageW(name)

    def qpc(self):
        value = self.ctypes.c_int64()
        self.kernel32.QueryPerformanceCounter(self.ctypes.byref(value))
        return value.value

    def frequency(self):
        value = self.ctypes.c_int64()
        self.kernel32.QueryPerformanceFrequency(self.ctypes.byref(value))
        return value.value

    def post(self, hwnd, message, wparam, lparam):
        return bool(self.user32.PostMessageW(hwnd, message, wparam, lparam))

    def foreground(self, hwnd):
        return (self.user32.GetForegroundWindow() or 0) == hwnd

    def alive(self, hwnd):
        return bool(self.user32.IsWindow(hwnd))


class Tail:
    """New complete rows of a growing JSONL file."""

    def __init__(self, path):
        self.path, self.offset, self.buffer = Path(path), 0, b''

    def rows(self):
        try:
            with self.path.open('rb') as stream:
                stream.seek(self.offset)
                data = stream.read()
        except OSError:
            return []
        self.offset += len(data)
        self.buffer += data
        *lines, self.buffer = self.buffer.split(b'\n')
        out = []
        for line in lines:
            try:
                row = json.loads(line)
            except ValueError:
                continue
            if isinstance(row, dict):
                out.append(row)
        return out


def module_row(tail, name, clock=time.monotonic, sleep=time.sleep, timeout=START_TIMEOUT_S):
    """The module row of `name` in the shell's live log (its window, run, build and puzzle), or None after `timeout`."""
    started = clock()
    while True:
        for row in tail.rows():
            if row.get('t') == MODULE_ROW and row.get('module') == name and integer(row.get('window')):
                return row
        if clock() - started >= timeout:
            return None
        sleep(0.05)


def post_run(fixtures, puzzle, run_dir, turn_key, system, clock=time.monotonic, sleep=time.sleep, out=print):
    """Posts one puzzle's schedule to the shell whose LX-4 module row is in run_dir's live log; 0 when every sample
    completed."""
    manifest, edits, digests = load_fixtures(fixtures)
    run_dir = Path(run_dir)
    tail = Tail(run_dir / LIVE)
    head = module_row(tail, 'lx4', clock, sleep)
    if head is None:
        out(f'the shell has written no lx4 module row to {LIVE} within {START_TIMEOUT_S:g} s; start it with --lx4 '
            '--lx4-fixtures <fixtures> first')
        return 2
    if head.get('puzzle') != puzzle:
        out(f"the shell runs {head.get('puzzle')!r}, not {puzzle!r}")
        return 2
    hwnd, names = head['window'], {RESET: system.register(RESET), SELECT: system.register(SELECT)}
    samples = schedule(edits, puzzle)
    failures = 0
    with (run_dir / POSTS).open('x', encoding='utf-8', newline='\n') as log:
        def write(row):
            log.write(json.dumps(row, separators=(',', ':')) + '\n')
            log.flush()

        write({'t': 'run', 'format': POSTS_FORMAT, 'run_id': head.get('run_id'), 'puzzle': puzzle,
               'build_identity': head.get('build_identity'), 'qpc_frequency': system.frequency(),
               'synthetic': False, 'turn_key': turn_key, **digests,
               'harness_sha256': sha256_bytes(Path(__file__).read_bytes())})
        endpoints, keys = set(), 0

        def drain():
            nonlocal keys
            for row in tail.rows():
                if row.get('t') == 'lx4':
                    endpoints.add((row.get('sample'), row.get('endpoint')))
                elif row.get('t') == SELECTED:
                    endpoints.add((row.get('sample'), (SELECTED, row.get('select'))))
                elif row.get('t') == 'key':
                    keys += 1

        def wait_for(key):
            """Whether the shell writes the lx4 row `key` (or, for a turn, a key row) before the sample's timeout."""
            while clock() - started < TIMEOUT_S:
                drain()
                if key in endpoints or (key[1] == 'turn' and keys > 0):
                    return True
                sleep(0.005)
            return False

        for sample in samples:
            drain()
            keys = 0
            started = clock()
            sent, timed = messages(edits, sample, turn_key)
            ok_all, status, selects = system.alive(hwnd), None, 0
            for index, (message, wparam, lparam) in enumerate(sent):
                number = names.get(message, message)
                qpc = system.qpc()
                ok = system.post(hwnd, number, wparam, lparam)
                ok_all = ok_all and ok
                write({'t': 'post', 'sample': sample['id'], 'index': index, 'message': number, 'wparam': wparam,
                       'lparam': lparam, 'qpc': qpc, 'ok': ok, 'timed': index == timed})
                if message == RESET and not (ok_all and wait_for((sample['id'], READY))):
                    status = 'not-ready'
                    break
                selects += message == SELECT
                if message == SELECT and not (ok_all and wait_for((sample['id'], (SELECTED, selects)))):
                    status = 'not-selected'
                    break
            if status is None:
                status = 'done' if ok_all and wait_for((sample['id'], sample['metric'])) else 'timeout'
            foreground = system.foreground(hwnd)
            write({'t': 'sample', 'sample': sample['id'], 'status': status, 'foreground': foreground})
            done = status == 'done'
            failures += not (done and foreground)
            sleep(max(0.0, PACE_S - (clock() - started)))
    out(f'{puzzle}: {len(samples)} samples posted, {failures} timed out or lost the foreground')
    return 0 if failures == 0 else 3


# ---- analyze ----

def read_posts(rows):
    head = rows[0]
    require(head.get('t') == 'run' and head.get('format') == POSTS_FORMAT, 'bad-posts-header')
    require(head.get('puzzle') in PUZZLES and integer(head.get('qpc_frequency')), 'bad-posts-header')
    posts, done = {}, {}
    for row in rows[1:]:
        if row.get('t') == 'post':
            require(integer(row.get('sample')) and integer(row.get('index')) and integer(row.get('qpc')), 'bad-post')
            posts.setdefault(row['sample'], []).append(row)
        elif row.get('t') == 'sample':
            require(integer(row.get('sample')), 'bad-post')
            done[row['sample']] = row
        else:
            raise RunError('bad-post')
    return head, posts, done


def analyze_run(events, post_rows, edits, digests):
    head = key_to_frame.check_rows(events)
    posts_head, posts, done = read_posts(post_rows)
    frequency, reasons, contradictions = head['qpc_frequency'], set(), set()
    if posts_head['qpc_frequency'] != frequency or posts_head.get('build_identity') != head['build_identity']:
        reasons.add('posts-do-not-match-shell')
    if head.get('puzzle') != posts_head['puzzle']:
        reasons.add('puzzle-mismatch')
    if (posts_head.get('manifest_sha256'), posts_head.get('edits_sha256')) != (digests['manifest_sha256'],
                                                                               digests['edits_sha256']):
        reasons.add('other-fixtures')
    frames, presented = key_to_frame.check_frames(events[1:], contradictions)
    endpoints = {}
    for row in events[1:]:
        if row.get('t') == 'lx4':
            require(integer(row.get('sample')) and isinstance(row.get('endpoint'), str), 'bad-lx4')
            endpoints.setdefault((row['sample'], row['endpoint']), row)
    times, gap, failures = {}, {}, {}
    for sample in schedule(edits, posts_head['puzzle']):
        if sample['metric'] == 'turn':
            continue
        cell = (sample['series'], sample['size'], sample['metric'])
        failure = None
        timed = [row for row in posts.get(sample['id'], []) if row.get('timed')]
        state = done.get(sample['id'], {})
        row = endpoints.get((sample['id'], sample['metric']))
        if state.get('status') in ('not-ready', 'not-selected'):
            failure = state['status']
        elif len(timed) != 1 or not all(post.get('ok') for post in posts.get(sample['id'], [])):
            failure = 'post-failed'
        elif state.get('status') != 'done':
            failure = 'timeout'
        elif state.get('foreground') is not True:
            failure = 'not-foreground'
        elif row is None or not integer(row.get('frame_id')):
            failure = 'no-endpoint'
        elif row.get('buffer_sha256') != edits[sample['edit']]['result_sha256']:
            failure = 'wrong-buffer'
        elif row['frame_id'] not in frames:
            failure = 'unknown-frame'
            contradictions.add('lx4-names-unknown-frame')
        elif frames[row['frame_id']]['qpc'] < timed[0]['qpc']:
            failure = 'frame-before-post'
            contradictions.add('frame-before-post')
        elif row['frame_id'] not in presented:
            # A sample without a confirmed frame: counted apart in its cell's confirmation gap, never judged; the run
            # stays valid (owner decision, 22 November 2026, item 10).
            gap[cell] = gap.get(cell, 0) + 1
            continue
        if failure:
            failures[failure] = failures.get(failure, 0) + 1
            continue
        latency = (presented[row['frame_id']]['display_qpc'] - timed[0]['qpc']) * 1000 / frequency
        times.setdefault(cell, []).append(latency)
    if contradictions:
        reasons.add('contradiction')
    if failures:
        reasons.add('failed-samples')
    if not head['foreground_throughout']:
        reasons.add('not-foreground')
    if head['correctness'] != 'pass':
        reasons.add('correctness-failed')
    return {'run_id': head['run_id'], 'build_identity': head['build_identity'], 'synthetic': head['synthetic'],
            'puzzle': posts_head['puzzle'], 'failures': dict(sorted(failures.items())),
            'unconfirmed': sum(gap.values()), 'contradictions': sorted(contradictions),
            'status': 'invalid' if reasons else 'valid', 'reasons': sorted(reasons), 'times': times, 'gap': gap}


def read_run(directory, edits, digests):
    directory = Path(directory)
    try:
        return analyze_run(evidence.read_jsonl(directory / key_to_frame.EVENTS),
                           evidence.read_jsonl(directory / POSTS), edits, digests)
    except RunError as error:
        return {'run_id': directory.name, 'build_identity': None, 'synthetic': None, 'puzzle': None,
                'status': 'unreadable', 'reasons': [str(error)], 'times': {}, 'gap': {}}


def cell_statistics(values, unconfirmed):
    """A cell's statistics: `n` all its samples, `timed` those with a confirmed frame, `unconfirmed` its confirmation
    gap; mean, p50, p95, p99 and max over the timed samples only (owner decision, 22 November 2026, item 10)."""
    stats = statistics(values) if values else {'mean': None, 'p50': None, 'p95': None, 'p99': None, 'max': None}
    return {**stats, 'n': len(values) + unconfirmed, 'timed': len(values), 'unconfirmed': unconfirmed}


def series_table(run):
    """Per series, size and metric: statistics and the bar's outcome. Smoke samples (series 0) are listed apart."""
    table = []
    gap = run.get('gap', {})
    for cell in sorted(set(run['times']) | set(gap)):
        series, size, metric = cell
        p95_bar, p99_bar = BARS[metric][size]
        stats = cell_statistics(run['times'].get(cell, []), gap.get(cell, 0))
        table.append({'puzzle': run['puzzle'], 'series': series, 'size': size, 'metric': metric, 'stats': stats,
                      'bar_ms': [p95_bar, p99_bar],
                      'met': series == 0 or (stats['p95'] is not None and stats['p95'] <= p95_bar
                                             and stats['p99'] is not None and stats['p99'] <= p99_bar)})
    return table


def other_record(path):
    try:
        return json.loads(Path(path).read_text(encoding='utf-8'))
    except (OSError, ValueError, UnicodeError):
        return None


def reference_counts(reference, build):
    """Whether a gate reference counts as the gate of runs of `build`: the owner accepted that summary for that build
    (ACCEPTED_GATE_REFERENCES) and its every scene is met."""
    return (isinstance(reference, dict) and (reference.get('sha256'), build) in ACCEPTED_GATE_REFERENCES
            and bool(reference.get('verdicts')) and all(verdict == 'met' for verdict in reference['verdicts'].values()))


def judge(runs, e2, gate, valid_events, reference=None):
    """(verdict, reasons, decision, table); `reference` from gate_reference, or None."""
    valid = [run for run in runs if run['status'] == 'valid']
    by_puzzle = {puzzle: [run for run in valid if run['puzzle'] == puzzle] for puzzle in PUZZLES}
    if any(len(chosen) != 1 for chosen in by_puzzle.values()):
        return 'invalid', ['one-valid-run-per-puzzle-needed'], None, []
    if len({run['build_identity'] for run in valid}) != 1:
        return 'invalid', ['mixed-builds'], None, []
    build = valid[0]['build_identity']
    table = [row for run in valid for row in series_table(run)]
    timed = [row for row in table if row['series'] > 0]
    reasons = []
    complete = (sum(row['stats']['n'] for row in timed if row['size'] in SIZES) == TOTAL_SML
                and all(row['stats']['n'] == PER_SERIES for row in timed)
                and len(timed) == len(PUZZLES) * SERIES * (len(SIZES) * len(METRICS) + 1))
    if not complete:
        reasons.append('samples-missing')
    e2_ok = (isinstance(e2, dict) and e2.get('verdict') == 'met' and e2.get('build_identity') == build
             and valid_events <= {entry.get('sha256') for entry in e2.get('inputs', [])})
    gate_ok = (isinstance(gate, dict) and gate.get('format') == 'magic600-renderer-gate-v1' and bool(gate.get('scenes'))
               and all(scene.get('verdict') == 'met' and {run.get('build_identity') for run in scene.get('runs', [])}
                       == {build} for scene in gate['scenes']))
    referenced = gate is None and reference_counts(reference, build)
    if not e2_ok:
        reasons.append('e2-not-met-in-these-runs' if e2 is not None else 'e2-record-missing')
    if not (gate_ok or referenced):
        reasons.append('gate-not-met' if gate is not None else 'gate-summary-missing')
    missed = [row for row in timed if not row['met']]
    if missed:
        reasons.append('bar-missed')
    if any(row['size'] in ('S', 'M') for row in missed):
        decision = 'fail-s-m'
    elif (e2 is not None and not e2_ok) or (gate is not None and not gate_ok):
        decision = 'fail-gate-or-e2'
    elif missed:
        decision = 'owner-l-or-w'
    elif complete and e2_ok and (gate_ok or referenced):
        decision = 'all-bars'
    else:
        decision = None
    if decision == 'all-bars':
        return 'met', [], decision, table
    if decision in ('fail-s-m', 'fail-gate-or-e2', 'owner-l-or-w'):
        return 'not-met', reasons, decision, table
    return 'insufficient', reasons, decision, table


def gate_reference(summary, sha256):
    """A renderer gate summary of another build as recorded beside the verdict: its digest, builds and verdicts;
    `counted_as_gate` is set by build() (reference_counts)."""
    if not (isinstance(summary, dict) and summary.get('format') == 'magic600-renderer-gate-v1'
            and isinstance(summary.get('scenes'), list)):
        raise FixtureError('--gate-reference: not a renderer gate summary')
    return {'counted_as_gate': False, 'accepted_by': None, 'sha256': sha256,
            'build_identities': sorted({str(run.get('build_identity')) for scene in summary['scenes']
                                        for run in scene.get('runs', [])}),
            'verdicts': {str(scene.get('scene')): scene.get('verdict') for scene in summary['scenes']}}


def confirmation_gap(runs):
    """The known limit of item 10: per readable run, its samples without a confirmed frame, in total and per cell
    (series, size, metric), smoke samples (series 0) included; never judged."""
    out = []
    for run in runs:
        if run['status'] == 'unreadable':
            continue
        cells = [{'series': series, 'size': size, 'metric': metric, 'unconfirmed': count}
                 for (series, size, metric), count in sorted(run.get('gap', {}).items())]
        out.append({'run_id': run['run_id'], 'puzzle': run['puzzle'], 'status': run['status'],
                    'unconfirmed': sum(cell['unconfirmed'] for cell in cells), 'cells': cells})
    return {'source': GAP_SOURCE, 'judged': False,
            'rule': 'a sample whose endpoint frame has no confirmed display is counted here and not in its cell\'s '
                    'percentiles', 'runs': out}


def build(fixtures, directories, argv, e2_path=None, gate_path=None, synthetic_flag=False, reference_path=None):
    _, edits, digests = load_fixtures(fixtures)
    runs = [read_run(directory, edits, digests) for directory in directories]
    events = {Path(directory).name: evidence.sha256(Path(directory) / key_to_frame.EVENTS)
              for directory, run in zip(directories, runs) if run['status'] == 'valid'}
    e2 = other_record(e2_path) if e2_path else None
    gate = other_record(gate_path) if gate_path else None
    reference = (gate_reference(other_record(reference_path), evidence.sha256(reference_path)) if reference_path
                 else None)
    verdict, reasons, decision, table = judge(runs, e2, gate, set(events.values()), reference)
    identities = sorted({run['build_identity'] for run in runs if run['status'] == 'valid'})
    if reference is not None:
        reasons = ['gate-summary-other-build' if reason == 'gate-summary-missing' else reason for reason in reasons]
        if gate is None and len(identities) == 1 and reference_counts(reference, identities[0]):
            reference.update(counted_as_gate=True,
                             accepted_by=ACCEPTED_GATE_REFERENCES[(reference['sha256'], identities[0])])
    inputs = [evidence.file_entry(f'{run["run_id"]}:{name}', Path(directory) / name)
              for directory, run in zip(directories, runs) for name in (key_to_frame.EVENTS, POSTS)
              if (Path(directory) / name).is_file()]
    inputs += [evidence.file_entry(role, path) for role, path in
               (('e2', e2_path), ('gate', gate_path), ('gate-reference', reference_path)) if path]
    synthetic = evidence.is_synthetic([Path(directory) / key_to_frame.EVENTS for directory in directories],
                                      [run.get('synthetic') for run in runs], synthetic_flag)
    gap = confirmation_gap(runs)
    for run in runs:
        run.pop('times', None)
        run.pop('gap', None)
    sml = [row for row in table if row['series'] > 0 and row['size'] in SIZES]
    result = {'fixtures': digests, 'runs': runs, 'decision': decision, 'series': table,
              'accounting': accounting([schedule(edits, puzzle) for puzzle in PUZZLES]),
              'timed_sml_samples': sum(row['stats']['timed'] for row in sml),
              'unconfirmed_sml_samples': sum(row['stats']['unconfirmed'] for row in sml),
              'confirmation_gap': gap, 'gate_same_run_checked': False, 'gate_reference': reference,
              'limitations': LIMITATIONS}
    return evidence.make_record(
        'lx4-editor-latency', 'performance', synthetic=synthetic,
        build_identity=identities[0] if len(identities) == 1 else None, inputs=inputs,
        tools=evidence.tool_versions(__file__, ('c9/key_to_frame.py',)),
        command=evidence.portable_command(__file__, argv), bar=BAR, result=result, verdict=verdict,
        reasons=reasons, drafts=DRAFTS)


def main(argv=None, system_factory=Windows):
    argv = sys.argv[1:] if argv is None else argv
    parser = argparse.ArgumentParser(description="LX-4's harness: schedule, posted window messages, analysis.")
    commands = parser.add_subparsers(dest='command', required=True)
    plan = commands.add_parser('schedule', help='print the sample accounting; runs nothing')
    plan.add_argument('--fixtures', type=Path, required=True)
    poster = commands.add_parser('post', help="post one puzzle's schedule to the running shell")
    poster.add_argument('--fixtures', type=Path, required=True)
    poster.add_argument('--puzzle', choices=PUZZLES, required=True)
    poster.add_argument('--run', type=Path, help="the shell's run directory (holds its events.jsonl)")
    poster.add_argument('--turn-key', choices=tuple(TURN_KEYS), default=TURN_KEY,
                        help=f"the shell's turn key, a physical key (default {TURN_KEY})")
    poster.add_argument('--dry-run', action='store_true', help='read the fixtures and print; post nothing')
    analysis = commands.add_parser('analyze', help='judge run directories against RTE.49 and RTE.53')
    analysis.add_argument('--fixtures', type=Path, required=True)
    analysis.add_argument('runs', nargs='+', type=Path)
    analysis.add_argument('--e2', type=Path, help='key_to_frame.py record over these same run directories')
    analysis.add_argument('--gate', type=Path, help='renderer_gate.py summary of the same build')
    analysis.add_argument('--gate-reference', type=Path,
                          help='a renderer_gate.py summary of another build (U4), recorded but never counted as the gate')
    analysis.add_argument('--out', type=Path, help='write the evidence record here (never replaces a file)')
    analysis.add_argument('--synthetic', action='store_true')
    args = parser.parse_args(argv)
    try:
        if args.command == 'schedule':
            _, edits, digests = load_fixtures(args.fixtures)
            counts = accounting([schedule(edits, puzzle) for puzzle in PUZZLES])
            print(json.dumps({**digests, **counts}, sort_keys=True))
            return 0 if counts['sml_samples'] == TOTAL_SML else 1
        if args.command == 'post':
            if args.dry_run:
                _, edits, digests = load_fixtures(args.fixtures)
                samples = schedule(edits, args.puzzle)
                sent = sum(len(messages(edits, sample, args.turn_key)[0]) for sample in samples)
                first, timed = messages(edits, samples[0], args.turn_key)
                print(f'{args.puzzle}: {len(samples)} samples, {sent} messages; fixtures {digests["manifest_sha256"]}')
                print(f'sample 0 ({samples[0]["size"]} {samples[0]["metric"]}): {len(first)} messages, timed {timed}')
                vk, scan = TURN_KEYS[args.turn_key]
                print(f'turn key {args.turn_key}: virtual key 0x{vk:02X}, scan code 0x{scan:02X}')
                print('dry run: no window was looked for and nothing was posted')
                return 0
            if args.run is None:
                parser.error('post needs --run unless --dry-run')
            return post_run(args.fixtures, args.puzzle, args.run, args.turn_key, system_factory())
        record = build(args.fixtures, args.runs, argv, args.e2, args.gate, args.synthetic, args.gate_reference)
    except FixtureError as error:
        print(f'lx4: {error}', file=sys.stderr)
        return 2
    for run in record['result']['runs']:
        print(f"{run['run_id']} ({run.get('puzzle')}): {run['status']}"
              f"{' (' + ', '.join(run['reasons']) + ')' if run['reasons'] else ''}")
    print(f"decision: {record['result']['decision']}; timed S/M/L samples {record['result']['timed_sml_samples']} "
          f"and {record['result']['unconfirmed_sml_samples']} in the confirmation gap, of {TOTAL_SML}")
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
