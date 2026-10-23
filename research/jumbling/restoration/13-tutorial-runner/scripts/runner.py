"""State-only, literal-word tutorial runner (headless research, runner-run/3).

Run from the repository root with Python -I -B. For example:
  python3 -I -B research/jumbling/restoration/13-tutorial-runner/scripts/runner.py \
      --scramble 10 7001 --out /tmp/tutorial-run --cache /tmp/tutorial-cache

The output directory must be empty unless --resume continues its recorded steps.
Existing research programs own the searches;
this module only orders them, checks their literal words and checkpoints progress.
The phases run in a fixed order and never return: bulk, height (r22d75, r22d81,
h4), high, tail, lattice (owner, 20 October 2026; stage 13 README, section 1).
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass, field
import gzip
import hashlib
import json
import math
import os
import resource
from pathlib import Path
import re
import runpy
import shutil
import signal
import subprocess
import sys
import threading
import time
import traceback
import zlib

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[5]
SELF = Path(__file__).resolve()
sys.path.insert(0, str(SELF.parent))
import runner_api as api  # noqa: E402
J = Path('research/jumbling')
TH = J / 'theory'
GM = J / 'restoration/09-general-method/scripts/gm'
EC = GM.parent / 'echain'
LOW = J / 'restoration/12-low-height/scripts'
LAT = GM.parent / 'lattice/scripts'
CHECKER = GM.parent / 'lattice/checker/verify_late.py'
LIBRARY = J / 'restoration/09-general-method/data/library-merged.json.gz'
SOLVED = hashlib.sha256(b'').hexdigest()
# reopen reads the solver's earlier twists, so the v3 bulk leaves it out: the integrator's default under the
# owner's reading of 20 October 2026 that every rule reads only the current state.
BULK_ORDER = ['seam', 'cover', 'close', 'conj', 'lock', 'improve', 'shift']
TAIL_RULES = ['zero_bad', 'pure', 'close', 't1prime']
# Clocks that acceptance mode lifts: the runner's own are off, a child's gets api.CLOCK_SENTINEL.
CLOCKS = ('bulk_step_seconds', 'bulk_seconds', 'rule_seconds', 'greedy_seconds', 'cert_seconds',
          'height_seconds', 'height_cpu', 'kernel_seconds')
PLUGINS = {'height': 'rules_height', 'high': 'rules_kernel', 'lattice': 'lattice_phase'}
# The options every acceptance run uses (plan revision 3e, E3); --mode acceptance refuses any other value.
ACCEPTANCE_CONFIG = SELF.parents[1] / 'acceptance-config.json'
ACCEPTANCE_OPTIONS = ('literal_limit', 'seeds', 'tail_rules', 'lattice_restorer', 'fallback_max_length', 'par',
                      'pair_shards', 'bulk_steps', 'tail_steps', 'high_steps', 'no_lattice')
# r22b10_cert.py's own deadline reasons (:67, :72, :741, :860, :888, :909, :912). Its fixed
# size caps (:123, :316) are not clocks and are not listed.
CERT_DEADLINE_PHRASES = ('wall-clock budget exhausted', 'serialization budget exhausted', 'exceeded budget')
# Applicability screens that cellpure5.py (:191, :207, :212) and transport_census.py (:110, :165)
# write as asserts: the rule does not apply to the unit. config.scripts pins these lines; any other
# failed assert of theirs is a crash.
SCREENS = {('cellpure5.py', 191), ('cellpure5.py', 207), ('cellpure5.py', 212),
           ('transport_census.py', 110), ('transport_census.py', 165)}
# check_r22b10.py rejections (:305-318, :472-477) that contradict what the runner has established:
# the state and target digests, N1 and N2, and a complete certificate. They are integrity failures.
CHECK_INTEGRITY = ('digest', 'N1 fails', 'N2 fails', 'incomplete certificate')
# jordan_target.py's fixed CPU cap (:10-14): SIGXCPU at 840 s exits 124; the hard limit SIGKILLs.
JORDAN_CPU_SECONDS = 840
# The keys the runner writes in a played step's row; a plugin's own fields may not use them.
ROW_KEYS = frozenset({'index', 'phase', 'subphase', 'rule', 'word', 'letters', 'encoding', 'word_sha256', 'tree',
                      'contract', 'gate', 'standing', 'before', 'after', 'table', 'seconds', 'attempts'})
SCRIPTS = [SELF.relative_to(ROOT), SELF.with_name('runner_api.py').relative_to(ROOT), TH / 'seams.py', TH / 'restore.py',
           TH / 'seamsx.py', TH / 'rim.py', TH / 'rimx.py', TH / 'orient.py', TH / 'buffer.py', TH / 'trees.py',
           J / 'sim/state.py', GM / 'mkblocks2.py', GM / 'tclose2.py',
           GM / 'fallback.py', LOW / 'cellpure5.py',
           LOW / 'round22/transport_census.py', LOW / 'round22/stepchk.py',
           LOW / 'round22/prep_t1prime.py', LOW / 'round22/jordan_target.py',
           LOW / 'r22b10_cert.py', LOW / 'check_r22b10.py', LOW / 'expand_slp.py',
           LOW / 'slp_apply.py', LAT / 'restore_lattice_late.py', CHECKER]


def read_json(path):
    raw = Path(path).read_bytes()
    return json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)


def atomic_json(path, doc):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(path.name + '.tmp')
    data = (json.dumps(doc, indent=1, allow_nan=False) + '\n').encode('utf-8')
    with temporary.open('wb') as f:
        f.write(gzip.compress(data, mtime=0) if path.suffix == '.gz' else data)
        f.flush()
        os.fsync(f.fileno())
    os.replace(temporary, path)


def safe_reason(value):
    text = str(value)
    return 'see child log' if '/' in text or '\\' in text else text[:400]


def word_sha256(word):
    return hashlib.sha256(json.dumps(word).encode('ascii')).hexdigest()


def json_rows(log):
    return [json.loads(line) for line in Path(log).read_text().splitlines() if line.startswith('{')]


def failed_assert(log):
    """(script name, line) of the assert whose traceback ended a child's log, else None."""
    lines = Path(log).read_text(errors='replace').splitlines()
    starts = [i for i, line in enumerate(lines) if line == 'Traceback (most recent call last):']
    frame = None
    for line in lines[starts[-1] + 1:] if starts else []:
        if line.startswith('  File "'):
            frame = re.match(r'  File "(.+)", line (\d+), in ', line)
        elif not line.startswith(' '):
            # The exception line; output the child had buffered may follow it.
            if frame and (line == 'AssertionError' or line.startswith('AssertionError:')):
                return Path(frame.group(1)).name, int(frame.group(2))
            break
    return None


def source_identity(exclude=()):
    result = {'commit': None, 'status_sha256': None, 'dirty': None}
    paths = ['research/jumbling']
    for path in exclude:
        path = Path(path).resolve()
        if path.is_relative_to(ROOT):
            paths.append(':(exclude,literal)' + path.relative_to(ROOT).as_posix())
    for key, argv in [('commit', ['git', 'rev-parse', 'HEAD']),
                      ('status', ['git', '--no-optional-locks', 'status', '--porcelain=v1', '--untracked-files=all', '--', *paths])]:
        try:
            output = subprocess.check_output(argv, cwd=ROOT, stderr=subprocess.DEVNULL)
        except (OSError, subprocess.CalledProcessError):
            continue
        if key == 'commit':
            result['commit'] = output.decode('ascii').strip()
        else:
            result.update(status_sha256=hashlib.sha256(output).hexdigest(), dirty=bool(output))
    return result


def code_identity(out):
    """Pin all jumbling inputs and the interpreter, excluding generated run/cache trees."""
    import numpy as np
    out = Path(out).resolve()
    excluded = (out, out.with_name(out.name + '-checkpoint'))
    rows = []
    for directory, dirs, files in os.walk(ROOT / J):
        base = Path(directory)
        dirs[:] = sorted(name for name in dirs if name not in ('runs', 'cache', '__pycache__')
                         and not any((base / name).resolve().is_relative_to(p) for p in excluded))
        for name in sorted(files):
            path = base / name
            if name in ('runs', 'cache', '__pycache__') or any(path.resolve().is_relative_to(p) for p in excluded):
                continue
            rows.append((path.relative_to(ROOT).as_posix(), hashlib.sha256(path.read_bytes()).hexdigest()))
    rows += [('Python', sys.version), ('NumPy', np.__version__)]
    raw = json.dumps(sorted(rows), ensure_ascii=True, separators=(',', ':')).encode('ascii')
    return {'sha256': hashlib.sha256(raw).hexdigest(), 'files': len(rows)}


def phase_entry(name, snapshot):
    hist = {int(h): n for h, n in snapshot['heights'].items()}
    top, n4, n6 = max(hist, default=0), hist.get(4, 0), hist.get(6, 0)
    off = snapshot['off']
    return {'r22d75': top <= 6 and n4 + n6 > 0, 'r22d81': top <= 6 and n6 > 0,
            'h4': top <= 4 and n4 > 0, 'high': top <= 6 and n4 + n6 > 0,
            'tail': off > 0, 'lattice': off == 0}[name]


def resume_point(record, state):
    """Pure pipeline classification; state is the rebuilt mechanics snapshot.

    A saved row precedes its phase's next entry test. Account for that pending
    lattice/handoff transition without treating a stuck plugin as a stuck run.
    """
    result, error = record['result'], record.get('error')
    if (result in ('solved', 'stuck') or record.get('stuck') is not None
            or result not in ('budget', 'error')
            or (result == 'error' and not (isinstance(error, str) and error.startswith('signal ')))):
        return 'terminal'
    phases, steps = record['phases'], record['steps']
    pipeline = [(name, phases['height']['sub'][name]) for name in api.SUBPHASES['height']]
    pipeline += [('high', phases['high'])]
    later = [data for _, data in pipeline] + [phases['tail'], phases['lattice']]
    if (any(data.get('result') in ('budget', 'timeout', 'error') for _, data in pipeline)
            or phases['tail'].get('result') in ('budget', 'timeout', 'stuck', 'error')
            or phases['lattice'].get('result') is not None):
        return 'terminal'
    if (phases['bulk'].get('completed') is not True
            and not any(d.get('start') is not None or d.get('result') is not None for d in later)):
        return 'bulk'
    for name, data in pipeline:
        if data.get('result') in ('budget', 'timeout', 'error'):
            return 'terminal'
        if data.get('result') is not None:
            continue
        own = [s for s in steps if (s['subphase'] or s['phase']) == name]
        if own and (state['off'] == 0 or not phase_entry(name, state)):
            continue
        return name
    tail = phases['tail']
    if tail.get('result') in ('budget', 'timeout', 'stuck', 'error'):
        return 'terminal'
    if tail.get('result') is None:
        if state['off'] or not any(s['phase'] in ('tail', 'fallback') for s in steps):
            return 'tail'
    return 'lattice' if state['off'] == 0 and phases['lattice'].get('result') is None else 'terminal'


def recount(record):
    """Only committed rows count; tail includes fallback and height includes its sub-phases."""
    for phase, data in record['phases'].items():
        rows = [s for s in record['steps'] if s['phase'] == phase or
                (phase == 'tail' and s['phase'] == 'fallback')]
        data['steps'] = len(rows)
        if 'twists' in data:
            data['twists'] = sum(s['letters'] for s in rows) if rows or phase != 'lattice' else data['twists']
        if phase == 'height':
            for name, sub in data['sub'].items():
                own = [s for s in rows if s['subphase'] == name]
                sub['steps'] = len(own)
                # Older format 3 records did not store sub-phase twist counts.
                if 'twists' in sub:
                    sub['twists'] = sum(s['letters'] for s in own)


def committed_references(record):
    """Every string of the rows and of each phase or sub-phase that has a result: the files those parts
    name (a kernel record's `dir`, attempts) stay on resume. An unfinished part's files are discarded."""
    keep = set()

    def walk(value):
        if isinstance(value, dict):
            for item in value.values():
                walk(item)
        elif isinstance(value, list):
            for item in value:
                walk(item)
        elif isinstance(value, str):
            keep.add(value)

    walk(record['steps'])
    for data in record['phases'].values():
        for sub in data.get('sub', {}).values():
            if sub.get('result') is not None:
                walk(sub)
        if data.get('result') is not None:
            walk({key: value for key, value in data.items() if key != 'sub'})
    return keep


class ResumeError(ValueError):
    """Resume refusal: no record, output directory or checkpoint is changed."""


def acceptance_config(args):
    """The committed acceptance options (plan revision 3e, E3), checked against this run's options."""
    raw = ACCEPTANCE_CONFIG.read_bytes()
    doc = json.loads(raw)
    options = doc.get('options') if isinstance(doc, dict) else None
    if (not isinstance(doc, dict) or doc.get('format') != 'acceptance-config/1' or not isinstance(options, dict)
            or set(options) != set(ACCEPTANCE_OPTIONS)):
        raise ValueError('acceptance-config.json is malformed')
    differ = [name for name in ACCEPTANCE_OPTIONS if getattr(args, name) != options[name]]
    if differ:
        raise ValueError('acceptance mode refuses options that differ from acceptance-config.json: '
                         + ', '.join(differ))
    return {'file': ACCEPTANCE_CONFIG.relative_to(ROOT).as_posix(),
            'sha256': hashlib.sha256(raw).hexdigest(), 'options': options}


def premise_details(details):
    return {key: details[key] for key in ('N1', 'N2', 'deficits', 'T_size', 'T_digest', 'V_length', 'timed_out')
            if key in details}


def attempt_row(exc, **fields):
    """A failed attempt; ``timed_out`` is true when any clock ended it (R22D-55)."""
    row = dict(fields, outcome=exc.outcome, reason=exc.reason, **exc.details)
    row['timed_out'] = bool(exc.details.get('timed_out')) or exc.outcome == 'timeout'
    return row


def cert_timed_out(rows, evidence, passed, measured):
    """An incomplete r22b10_cert result ended on a clock (plan revision 3d, tests a, b and c)."""
    if any(r.get('timeout') is True for r in rows):
        return True
    missing = evidence.get('missing')
    if isinstance(missing, list) and any(
            isinstance(m, dict) and isinstance(m.get('reason'), str) and
            any(phrase in m['reason'] for phrase in CERT_DEADLINE_PHRASES) for m in missing):
        return True
    seconds = evidence.get('seconds')
    if type(seconds) not in (int, float) or not math.isfinite(seconds):
        seconds = measured
    return seconds >= passed - min(10, passed / 10)


def verdict(record, final_timed_out):
    """The acceptance verdict, the first match in the plan's order (section 0)."""
    acceptance, result = record['acceptance'], record['result']
    if result == 'error':
        return 'error'
    if record['config']['component'] is not None:
        return 'component'
    if acceptance['certificate_steps']:
        return 'stuck'
    if acceptance['fallback_steps'] or acceptance['probe']:
        return 'coverage_failure'
    if result == 'budget' or (result == 'stuck' and final_timed_out):
        return 'budget'
    return result


def literal(doc, before=None, after=None):
    """Reject compressed programs and invalid letters, rather than interpreting them."""
    if not isinstance(doc, dict) or 'word' not in doc or any(
            key in doc for key in ('segments', 'slp', 'certificate')):
        raise ValueError('a literal word is required')
    word = doc['word']
    if not isinstance(word, list) or any(
            not isinstance(x, (list, tuple)) or len(x) != 2 or
            any(type(v) is not int for v in x) or
            not (0 <= x[0] < 600 and 0 <= x[1] < 60) for x in word):
        raise ValueError('invalid I_a letters')
    result = {'word': [list(x) for x in word]}
    for key, expected in [('input_digest', before), ('output_digest', after)]:
        value = doc.get(key, expected)
        if expected is not None and value != expected:
            raise ValueError('word digest mismatch')
        if value is not None:
            if not isinstance(value, str) or len(value) != 64 or any(c not in '0123456789abcdef' for c in value):
                raise ValueError('invalid word digest')
            result[key] = value
    return result


def lattice_snapshot(doc):
    """A snapshot of a lattice configuration: no piece off, so no heights (Mechanics.heights)."""
    return (isinstance(doc, dict) and set(doc) == {'digest', 'off', 'heights'} and type(doc['off']) is int
            and doc['off'] == 0 and doc['heights'] == {} and isinstance(doc['digest'], str)
            and len(doc['digest']) == 64 and all(c in '0123456789abcdef' for c in doc['digest']))


class Mechanics:
    """Lazy imports keep record and scheduling tests independent of geometry."""
    def __init__(self, cache):
        sys.path[:0] = [str(ROOT / TH), str(ROOT / J), str(ROOT / J / 'fixtures')]
        import numpy as np
        import restore
        import rimx
        import seams
        import seamsx
        import sim
        import orient
        import buffer
        import trees
        seams.CACHE = Path(cache) / 'home_centroids.npy'
        self.np, self.restore, self.seams, self.sim = np, restore, seams, sim
        self.orient, self.buffer, self.trees = orient, buffer, trees
        self.menu = restore.menu('I_a')
        # The bulk child's restorer plays seams.SeamRestorer's words exactly, faster: restored caches on undo
        # and parallel conj trials (seamsx.py; JUMBLING_BULK_WORKERS processes, default 1) over the exact
        # cached domains of rimx.py.
        self.bulk_restorer = type('BulkRestorer', (seamsx.SeamRestorer,), {'DOMAINS': rimx.Domains})

    def scramble(self, k, seed):
        state, _ = self.restore.scramble(self.menu, k, seed)
        return state

    def load(self, path):
        state = self.sim.State(self.restore.ctx, menu=self.menu)
        self.orient.set_pose_table(state, read_json(path))
        return state

    def export(self, state, path):
        self.buffer.export_pose_table(state, path)

    def heights(self, state):
        # census_seams.heights, with its unused off-pose count omitted.
        pid, counts = self.np.unique(state.pose_id, return_counts=True)
        histogram = {}
        for p, count in zip(pid.tolist(), counts.tolist()):
            if state._poses[p].kidx >= 0:
                continue
            h = self.trees.dist(self.trees.factors(state._pose_matrix(p))[0])
            histogram[h] = histogram.get(h, 0) + count
        return {str(h): histogram[h] for h in sorted(histogram)}

    def snapshot(self, state):
        return {'digest': state.digest(), 'off': int(state.off_lattice_count()),
                'heights': self.heights(state)}

    def labels_identity(self, state):
        """Restored means identity sticker labels (owner decision, 15 October 2026)."""
        if state.off_lattice_count():
            return False
        stickers = state.lattice_stickers()
        labels = self.np.asarray(stickers['labels'])
        return bool(self.np.all(stickers['frame_agrees'])) and bool(
            self.np.array_equal(labels, self.np.arange(len(labels))))

    def off_set(self, state):
        return set(self.np.flatnonzero(state._kpose()[state.pose_id] < 0).tolist())

    def apply_word(self, state, doc):
        doc = literal(doc, state.digest())
        for c, i in doc['word']:
            outcome = state.apply(self.sim.Twist(self.restore.ctx, c,
                self.restore.element(self.menu, c, i), 'menu', {'menu': self.menu.name}))
            if not outcome.applied or outcome.uncertain:
                raise ValueError('word contains an inadmissible or uncertain twist')
        if state.digest() != doc['output_digest']:
            raise ValueError('word output digest mismatch on own replay')
        state.journal = []

    def inverse_word(self, word):
        r = self.restore
        lookup = {r.to_tuple(matrix): i for i, (_, matrix) in enumerate(self.menu.items)}
        return [[c, lookup[r.to_tuple(r.transpose(self.menu.items[i][1]))]] for c, i in reversed(word)]


@dataclass
class Unit:
    key: int
    size: int
    target: Path
    free: Path | None = None


@dataclass
class Candidate:
    rule: str
    word: dict
    check: dict
    end_table: Path
    parts: list | None = None
    gate: str = 'stepchk'
    metadata: dict = field(default_factory=dict)
    certificate_sha256: str | None = None


@dataclass
class Trial:
    candidate: Candidate | None = None
    attempts: list = field(default_factory=list)


@dataclass
class Child:
    rc: int
    seconds: float
    log: Path
    timeout: bool = False
    cancelled: bool = False
    stopped: bool = False


class RuleFailure(Exception):
    def __init__(self, outcome, reason, **details):
        super().__init__(reason)
        self.outcome, self.reason, self.details = outcome, safe_reason(reason), details


class RunError(Exception):
    """A crash, truncated or malformed child output, or a digest mismatch. No rule handler catches
    it, so the run ends error and no later rule, unit or phase runs (plan, result table)."""


class RunSignal(Exception):
    pass


def kill_child(proc):
    # The group may outlive its leader. Always reap our child and stop its group.
    try:
        os.killpg(proc.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    proc.wait()
    if sys.platform == 'linux':
        # SIGKILL delivery and exit are separate events. Reaping the leader can
        # precede a descendant's exit, so wait for every live group member too.
        # Orphan zombies have exited; their new parent owns their reaping.
        while True:
            for entry in Path('/proc').iterdir():
                if not entry.name.isdigit():
                    continue
                try:
                    fields = (entry / 'stat').read_text().rsplit(')', 1)[1].split()
                except (FileNotFoundError, ProcessLookupError, PermissionError):
                    continue
                if int(fields[2]) == proc.pid and fields[0] not in ('Z', 'X'):
                    break
            else:
                break
            time.sleep(0.01)


class Runner:
    def __init__(self, args, mechanics=None, plugins=None):
        self.args = args
        self.plugins = plugins
        self.out = Path(args.out).resolve()
        if '.json' in str(self.out):
            raise ValueError('out path must not contain .json')
        self.cache = Path(args.cache).resolve() if args.cache else self.out / 'cache'
        if not getattr(args, 'resume', False):
            self.directories()
        self.m = mechanics
        self.cancel = threading.Event()
        self.signal_number = None
        self.started = time.monotonic()
        self.record = None
        self.state = self.table = None

    def directories(self):
        for name in ('words', 'units', 'tables', 'attempts', 'logs', 'step-trees'):
            (self.out / name).mkdir(parents=True, exist_ok=True)

    @property
    def mode(self):
        return getattr(self.args, 'mode', 'development')

    @property
    def acceptance(self):
        return self.mode == 'acceptance'

    def needed(self, outcome):
        """A closure with no literal word: certificate_needed in acceptance mode (D1)."""
        return 'certificate_needed' if self.acceptance else outcome

    def component(self, reason):
        if self.record['config']['component'] is None:
            self.record['config']['component'] = reason

    def plugin_modules(self):
        if self.plugins is None:
            import importlib
            self.plugins = {phase: importlib.import_module(name) for phase, name in PLUGINS.items()}
        return self.plugins

    def on_signal(self, number, frame):
        if self.signal_number is None:
            self.signal_number = number

    def check_signal(self):
        if self.signal_number is not None:
            raise RunSignal(f'signal {self.signal_number}')

    def child_env(self):
        return dict(os.environ, OPENBLAS_NUM_THREADS='1', OMP_NUM_THREADS='1',
                    MKL_NUM_THREADS='1', PYTHONDONTWRITEBYTECODE='1',
                    JUMBLING_HOME_CACHE=str(self.cache / 'home_centroids.npy'))

    def rel(self, path):
        return Path(path).resolve().relative_to(self.out).as_posix()

    def save(self):
        totals = self.record['totals']
        for phase in ('bulk', 'height', 'high', 'tail', 'fallback'):
            totals[phase + '_twists'] = sum(s['letters'] for s in self.record['steps'] if s['phase'] == phase)
        totals['seconds'] = round(time.monotonic() - self.started, 6)
        atomic_json(self.out / 'run.json', self.record)

    def configuration(self):
        """The same argument-derived configuration is used at start and before any resume write."""
        acceptance = self.args.mode == 'acceptance'
        frozen = acceptance_config(self.args) if acceptance else None
        budgets = {name: getattr(self.args, name) for name in
                   ('bulk_steps', 'bulk_step_seconds', 'bulk_seconds', 'tail_steps', 'high_steps',
                    'rule_seconds', 'greedy_seconds', 'cert_seconds', 'height_seconds', 'height_cpu',
                    'kernel_seconds', 'pair_shards', 'fallback_max_length', 'par')}
        for name in ('bulk_seconds', 'tail_steps'):
            budgets[name] = budgets[name] or None
        scripts = SCRIPTS + [p for plugin in self.plugin_modules().values()
                             for p in getattr(plugin, 'SCRIPTS', [])]
        d19 = getattr(self.plugin_modules()['high'], 'D19', None)
        if d19 is not None:
            # A rule outside the repository is recorded without its path; a missing one without a digest.
            path = ROOT / d19
            try:
                sha = hashlib.sha256(path.read_bytes()).hexdigest()
            except OSError:
                sha = None
            d19 = {'file': path.relative_to(ROOT).as_posix() if path.is_relative_to(ROOT) else None, 'sha256': sha}
        return {'literal_only': True, 'mode': self.args.mode, 'component': None,
                'bulk_order': BULK_ORDER.copy(), 'phase_order': list(api.PHASE_ORDER),
                'subphase_order': {k: list(v) for k, v in api.SUBPHASES.items()},
                'tail_rules': self.args.tail_rules,
                'budgets': api.recorded_budgets(self.args.mode, budgets, CLOCKS),
                'clock_sentinel': api.CLOCK_SENTINEL if acceptance else None,
                'kernel': {'seconds': None if acceptance else self.args.kernel_seconds,
                           'wall_margin': self.args.kernel_wall_margin, 'd19': d19, 'max_steps': 1},
                'lattice': {'restorer': self.args.lattice_restorer}, 'no_lattice': self.args.no_lattice,
                'literal_limit': self.args.literal_limit, 'seeds': self.args.seeds, 'acceptance_config': frozen,
                'source': source_identity((self.out, self.out.with_name(self.out.name + '-checkpoint'))),
                'code': code_identity(self.out),
                'scripts': {Path(p).as_posix(): hashlib.sha256((ROOT / p).read_bytes()).hexdigest()
                            for p in scripts}}

    def initialize(self):
        config = self.configuration()
        self.m = self.m or Mechanics(self.cache)
        if self.args.table:
            self.state = self.m.load(self.args.table)
            source = {'kind': 'table', 'file': 'start-table.json'}
        else:
            k, seed = self.args.scramble
            self.state = self.m.scramble(k, seed)
            source = {'kind': 'scramble', 'k': k, 'seed': seed}
        self.m.export(self.state, self.out / 'start-table.json')
        self.table = self.out / 'tables/bulk-end.json'
        self.m.export(self.state, self.table)
        start = dict(source, **self.m.snapshot(self.state))
        self.record = {'format': api.FORMAT, 'menu': 'I_a', 'start': start, 'config': config,
            'phases': {'bulk': {'result': 'budget', 'completed': False, 'steps': 0, 'twists': 0,
                'end': dict(self.m.snapshot(self.state), table=self.rel(self.table))},
                'height': {'result': None, 'steps': 0, 'twists': 0, 'start': None, 'end': None,
                           'sub': {name: {'result': None, 'steps': 0, 'twists': 0, 'start': None, 'end': None}
                                   for name in api.SUBPHASES['height']}},
                'high': {'result': None, 'steps': 0, 'twists': 0, 'start': None, 'end': None},
                'tail': {'result': None, 'steps': 0, 'twists': 0},
                'lattice': {'result': None, 'steps': 0, 'twists': None}},
            'steps': [], 'stuck': None, 'lattice': None, 'result': 'budget', 'error': None,
            'acceptance': {'verdict': None, 'certificate_steps': 0, 'fallback_steps': 0,
                           'probe': None, 'timed_out_attempts': 0},
            'totals': {'bulk_twists': 0, 'height_twists': 0, 'high_twists': 0, 'tail_twists': 0,
                       'fallback_twists': 0, 'lattice_twists': None, 'seconds': 0.0}}
        self.save()

    def resume(self):
        """Validate and replay completely before changing even an interrupted temporary file."""
        try:
            record = read_json(self.out / 'run.json')
            if record['format'] != api.FORMAT:
                raise ValueError('resume requires format 3')
            if type(record['phases']['bulk'].get('completed', False)) is not bool:
                raise ValueError('invalid bulk completion marker')
            config = self.configuration()
            old = record['config']
            if old.get('code') is None or old['code'] != config['code']:
                raise ValueError('resume code fingerprint differs or is missing')
            if {k: v for k, v in old.items() if k not in ('component', 'source')} != {
                    k: v for k, v in config.items() if k not in ('component', 'source')}:
                raise ValueError('resume configuration or scripts differ')
            self.m = self.m or Mechanics(self.cache)
            state = self.m.load(self.out / 'start-table.json')
            if state.digest() != record['start']['digest']:
                raise ValueError('resume start table digest differs')
            start = record['start']
            if self.args.table:
                if start['kind'] != 'table' or self.m.load(self.args.table).digest() != start['digest']:
                    raise ValueError('resume table source differs')
            elif start['kind'] != 'scramble' or list(self.args.scramble) != [start['k'], start['seed']]:
                raise ValueError('resume scramble source differs')

            def check_table(name, expected, boundary):
                try:
                    path = (self.out / name).resolve()
                    if not path.is_relative_to(self.out):
                        raise ValueError('table path escapes run')
                    if read_json(path)['digest'] != expected or self.m.load(path).digest() != expected:
                        raise ValueError('table digest differs')
                except Exception as exc:
                    raise ValueError(f'resume committed table for {boundary} is missing, unreadable '
                                     'or its digest differs') from exc

            for index, step in enumerate(record['steps']):
                if step['index'] != index or step.get('encoding') != 'json' or step['phase'] == 'lattice':
                    raise ValueError('resume requires non-lattice literal steps in order')
                path = (self.out / step['word']).resolve()
                if not path.is_relative_to(self.out):
                    raise ValueError('resume word path escapes run')
                word = literal(read_json(path), step['before']['digest'], step['after']['digest'])
                if state.digest() != step['before']['digest'] or word_sha256(word['word']) != step['word_sha256']:
                    raise ValueError('resume step before digest or word hash differs')
                self.m.apply_word(state, word)
                if state.digest() != step['after']['digest']:
                    raise ValueError('resume step after digest differs')
                if 'table' in step:
                    check_table(step['table'], step['after']['digest'], f'step {index}')
            point = resume_point(record, self.m.snapshot(state))
            if point == 'terminal':
                raise ValueError('run is not in flight')
            if point != 'bulk':
                end = record['phases']['bulk']['end']
                check_table(end['table'], end['digest'], 'bulk end')
            seconds = record['totals']['seconds']
            if type(seconds) not in (int, float) or not math.isfinite(seconds) or seconds < 0:
                raise ValueError('invalid recorded running time')
        except (OSError, ValueError, KeyError, TypeError, RuntimeError, EOFError, AttributeError,
                IndexError, zlib.error) as exc:
            reason = safe_reason(exc)
            raise ResumeError('invalid or unreadable resume record' if reason == 'see child log' else reason) from exc
        # Validation ends here. A row is the commit point; discard only uncommitted artifacts.
        # A phase that ended at this step count (high stuck with its kernel record) keeps its files.
        count, keep = len(record['steps']), committed_references(record)
        for name in ('words', 'step-trees', 'tables', 'attempts', 'units'):
            directory = self.out / name
            for path in directory.iterdir() if directory.exists() else ():
                match = re.match(r'^(\d+)(?:[.\-]|$)', path.name)
                relative = path.relative_to(self.out).as_posix()
                if (match and int(match[1]) >= count
                        and not any(ref == relative or ref.startswith(relative + '/') for ref in keep)):
                    shutil.rmtree(path) if path.is_dir() else path.unlink()
        for path in self.out.rglob('*.tmp'):
            if path.is_file():
                path.unlink()
        (self.out / 'bulk-progress.json').unlink(missing_ok=True)
        self.directories()
        self.record, self.state = record, state
        bulk = record['phases']['bulk']
        if bulk.get('completed') is True and bulk['result'] in ('budget', 'timeout'):
            # The child can save its final boundary before the parent labels the component.
            self.component('bulk_budget')
        recount(record)
        self.started = time.monotonic() - seconds
        self.table = self.out / (
            'tables/bulk-end.json' if point == 'bulk' or not count or record['steps'][-1]['phase'] == 'bulk'
            else f'tables/{count - 1:05d}.json')
        self.m.export(state, self.table)
        record.setdefault('resumes', []).append({'at_step': count, 'seconds': seconds, 'source': config['source']})
        record['result'], record['error'] = 'budget', None
        self.save()
        return point

    def execute(self, argv, tag, seconds, *, cwd=None, stop_when=None):
        """Each subprocess owns a process group; timeout also kills descendants.

        ``seconds=None`` means no clock. ``stop_when(line)`` true for a complete
        log line kills the group at once and returns ``stopped``."""
        log = self.out / 'logs' / (tag.replace('/', '-') + '.log')
        start = time.monotonic()
        if self.cancel.is_set() or self.signal_number is not None:
            return Child(-1, 0.0, log, cancelled=True)
        env = self.child_env()
        timed_out = cancelled = stopped = False
        offset, pending = 0, b''

        def watch():
            nonlocal offset, pending
            with log.open('rb') as f:
                f.seek(offset)
                data = f.read()
            offset += len(data)
            *lines, pending = (pending + data).split(b'\n')
            return any(stop_when(line.decode('utf-8', 'replace')) for line in lines)

        with log.open('w') as output:
            proc = subprocess.Popen([str(x) for x in argv], cwd=cwd or ROOT, env=env,
                stdout=output, stderr=subprocess.STDOUT, start_new_session=True)
            try:
                while proc.poll() is None:
                    elapsed = time.monotonic() - start
                    cancelled = self.cancel.is_set() or self.signal_number is not None
                    timed_out = seconds is not None and elapsed >= seconds
                    stopped = stop_when is not None and watch()
                    if cancelled or timed_out or stopped:
                        kill_child(proc)
                        break
                    try:
                        proc.wait(timeout=0.05)
                    except subprocess.TimeoutExpired:
                        pass
            finally:
                kill_child(proc)
        return Child(proc.returncode, time.monotonic() - start, log, timed_out, cancelled, stopped)

    def script(self, script, argv, tag, *, unlimited=False, cwd=None):
        # The adapter redirects seams' cache before executing *any* existing rule.
        command = [sys.executable, '-I', '-B', SELF, '--_script', ROOT / script,
                   '--cache', self.cache, '--'] + list(argv)
        seconds = None if unlimited or self.acceptance else self.args.rule_seconds
        return self.execute(command, tag, seconds, cwd=cwd)

    def require(self, child, *, returncodes=(0,), screens=()):
        """A clock or a cancellation fails the attempt; any other undeclared exit is a crash.

        ``screens`` holds the (script, line) asserts that are the script's applicability test."""
        self.check_signal()
        if child.timeout:
            raise RuleFailure('timeout', 'rule subprocess exceeded rule_seconds', timed_out=True)
        if child.cancelled:
            raise RuleFailure('cancelled', 'a preceding unit supplied the step')
        screen = failed_assert(child.log) if child.rc == 1 and screens else None
        if screen in screens:
            raise RuleFailure('not applicable', 'applicability screen %s:%d' % screen)
        if child.rc not in returncodes:
            raise RunError(f'child exit {child.rc}; see child log {child.log.name}')

    def bulk(self):
        if self.state.off_lattice_count() == 0 or self.args.bulk_steps == 0:
            result = 'lattice' if self.state.off_lattice_count() == 0 else 'budget'
        else:
            progress = self.out / 'bulk-progress.json'
            env = dict(self.child_env(), RUNNER_PARENT_PID=str(os.getpid()))
            start = time.monotonic()
            result = None
            with (self.out / 'logs/bulk.log').open('w') as log:
                proc = subprocess.Popen([sys.executable, '-I', '-B', str(SELF),
                    '--_bulk-child', str(self.out), '--cache', str(self.cache)],
                    cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
                try:
                    while proc.poll() is None:
                        if self.signal_number is not None:
                            kill_child(proc)
                            break
                        now = time.monotonic()
                        heartbeat = read_json(progress) if progress.exists() else {}
                        step_start = heartbeat.get('step_started')
                        total, each = ((None, None) if self.acceptance else
                                       (self.args.bulk_seconds, self.args.bulk_step_seconds))
                        if ((total and now - start >= total) or
                                (step_start is not None and each is not None and now - step_start >= each)):
                            result = 'timeout'
                            kill_child(proc)
                            break
                        try:
                            proc.wait(timeout=0.05)
                        except subprocess.TimeoutExpired:
                            pass
                finally:
                    kill_child(proc)
            self.record = read_json(self.out / 'run.json')
            result = result or self.record['phases']['bulk']['result']
            # No partially completed child state survives a timeout.
            self.state = self.m.load(self.out / 'start-table.json')
            for step in self.record['steps']:
                self.m.apply_word(self.state, read_json(self.out / step['word']))
            if proc.returncode != 0 and result != 'timeout' and self.signal_number is None:
                raise RuntimeError('bulk child failed; see child log')
        self.m.export(self.state, self.table)
        self.record['phases']['bulk']['result'] = result
        self.record['phases']['bulk']['end'] = dict(self.m.snapshot(self.state), table=self.rel(self.table))
        # The child's initial budget result is only an in-flight marker. An
        # operator signal must not turn it into a lasting bulk_budget component.
        self.check_signal()
        if result in ('budget', 'timeout'):
            # A step or clock budget ends the bulk early; later phases still run (plan, result table).
            self.component('bulk_budget')
        self.record['phases']['bulk']['completed'] = True
        self.save()

    def heights(self):
        hist = {int(h): n for h, n in self.m.heights(self.state).items()}
        return max(hist, default=0), hist.get(4, 0), hist.get(6, 0)

    def entry(self, name):
        """The entry test of a phase or height sub-phase, from the current state only."""
        return phase_entry(name, self.m.snapshot(self.state))

    def context(self, phase, sub):
        budgets = {name: getattr(self.args, name) for name in
                   CLOCKS + ('pair_shards', 'kernel_wall_margin', 'lattice_restorer')}
        work = self.out / 'attempts' / f'{len(self.record["steps"]):05d}-{sub or phase}'
        work.mkdir(parents=True, exist_ok=True)
        return api.Ctx(table=self.table, state=self.state, snapshot=lambda: self.m.snapshot(self.state),
                       mechanics=self.m, work_dir=work, cache=self.cache, execute=self.execute,
                       budgets=budgets, mode=self.args.mode, subphase=sub)

    def plugin_phase(self, phase, sub=None):
        """Run one phase (or height sub-phase) of the fixed order to its result."""
        name = sub or phase
        data = self.record['phases'][phase]
        data = data['sub'][sub] if sub else data
        if data['result'] is not None:
            return data['result']
        # A signal can save an unfinished call's attempts without a phase End.
        # Its retry starts from the committed rows, like a discarded word file.
        data.pop('attempts', None)
        data['start'] = data['start'] or self.m.snapshot(self.state)
        result, end = None, None
        if data['steps'] and not self.state.off_lattice_count():
            result = 'lattice'
        elif data['steps'] and not self.entry(name):
            result = 'handoff'
        elif not self.entry(name):
            result = 'not_applicable'
        plugin = self.plugin_modules()[phase]
        while result is None:
            self.check_signal()
            if data['steps'] and not self.entry(name):
                result = 'handoff'
                break
            if phase == 'high' and self.args.high_steps is not None and data['steps'] >= self.args.high_steps:
                end = api.End('budget', 'high_steps')
            else:
                end = plugin.next_step(self.context(phase, sub))
                if self.signal_number is not None:
                    # A signal during the call is the operator's, not the rule's (P1-robust-04).
                    data['attempts'] = list(getattr(end, 'attempts', []))
                    self.save()
                    self.check_signal()
            if isinstance(end, api.Proposal):
                self.record_step(end, phase, sub)
                data['steps'] += 1
                data['twists'] = data.get('twists', 0) + len(end.word['word'])
                end = None
                if self.state.off_lattice_count() == 0:
                    result = 'lattice'
                continue
            if not isinstance(end, api.End):
                raise ValueError('a plugin returns a Proposal or an End')
            if any(type(a.get('timed_out')) is not bool for a in end.attempts):
                raise ValueError('every plugin attempt records timed_out')
            result = end.result
            # The runner decides handoff and lattice from the state; a plugin may not claim them falsely.
            if ((result == 'handoff' and self.entry(name)) or
                    (result == 'lattice' and self.state.off_lattice_count())):
                raise ValueError(f'{name} claims {result} against the state')
            if result == 'not_applicable':
                if not getattr(plugin, 'STUB', False):
                    raise ValueError(f'{name} claims not_applicable against its entry test')
                # A stub before its packet is integrated: no step here, and the run is a component.
                result = 'stuck'
                self.component(f'{phase}_stub')
        data['result'] = result
        data['end'] = self.m.snapshot(self.state)
        data['twists'] = sum(s['letters'] for s in self.record['steps']
                             if s['phase'] == phase and s['subphase'] == sub)
        if end is not None:
            data.update(reason=end.reason, details=end.details, attempts=end.attempts)
        outer = self.record['phases'][phase]
        outer['twists'] = sum(s['letters'] for s in self.record['steps'] if s['phase'] == phase)
        if sub:
            outer['steps'] = sum(d['steps'] for d in outer['sub'].values())
            outer['start'] = outer['start'] or data['start']
            outer['end'] = data['end']
            if result == 'error':
                outer['result'] = 'error'
        self.save()
        if result == 'error':
            raise RuntimeError(f'{name} phase error: {safe_reason(end.reason if end else "")}')
        return result

    def phases_between(self):
        """Height (r22d75, r22d81, h4), then high. Returns 'budget' when a phase ends the run (R22D-55)."""
        results = []
        for sub in api.SUBPHASES['height']:
            results.append(self.plugin_phase('height', sub))
            if results[-1] in ('budget', 'timeout'):
                break
        self.record['phases']['height']['result'] = self.outer_result(results)
        self.save()
        if results[-1] in ('budget', 'timeout') or self.plugin_phase('high') in ('budget', 'timeout'):
            return 'budget'
        return None

    @staticmethod
    def outer_result(results):
        for result in ('error', 'budget', 'timeout', 'lattice'):
            if result in results:
                return result
        applicable = [r for r in results if r != 'not_applicable']
        return applicable[-1] if applicable else 'not_applicable'

    def write_word(self, index, word):
        suffix = '.json.gz' if len(word['word']) > 100000 else '.json'
        path = self.out / 'words' / f'{index:05d}{suffix}'
        atomic_json(path, word)
        return path

    def write_tree(self, index, tree, letters):
        path = self.out / 'step-trees' / f'{index:05d}.json.gz'
        written = api.write_tree(path, tree, letters)
        return {'file': self.rel(path), **written}

    def record_step(self, proposal, phase, sub):
        """Play a plugin's word twist by twist, check its claimed end digest and record it."""
        p = proposal
        if (p.phase != phase or p.subphase != sub or p.standing not in api.STANDINGS
                or ROW_KEYS.intersection(p.fields) or api.FORBIDDEN_KEYS.intersection(p.fields)
                or any(type(a.get('timed_out')) is not bool for a in p.attempts)):
            raise ValueError('proposal does not match its phase or record keys')
        started = time.monotonic()
        index = len(self.record['steps'])
        before = self.m.snapshot(self.state)
        word = literal(p.word, before['digest'])
        if 'output_digest' not in word:
            raise ValueError('a proposal claims its end digest')
        letters = len(word['word'])
        tree = p.tree or api.single(p.rule, letters)
        api.check_tree(tree, letters)            # a malformed tree changes no state
        self.m.apply_word(self.state, word)
        after = self.m.snapshot(self.state)
        word_file = self.write_word(index, word)
        tree = self.write_tree(index, tree, letters)
        self.table = self.out / 'tables' / f'{index:05d}.json'
        self.m.export(self.state, self.table)
        row = {'index': index, 'phase': phase, 'subphase': sub, 'rule': p.rule,
               'word': self.rel(word_file), 'letters': letters, 'encoding': 'json',
               'word_sha256': word_sha256(word['word']), 'tree': tree, 'contract': p.contract,
               'gate': p.gate, 'standing': p.standing, 'before': before, 'after': after,
               'table': self.rel(self.table), 'attempts': p.attempts,
               'seconds': round(time.monotonic() - started, 6), **p.fields}
        self.record['steps'].append(row)
        self.save()

    def units(self, table, work):
        work.mkdir(parents=True, exist_ok=True)
        child = self.script(GM / 'mkblocks2.py', [TH, EC, table, 'state', work], self.rel(work) + '-units')
        self.require(child)
        rows = [json.loads(line) for line in child.log.read_text().splitlines() if line.startswith('{')]
        if not rows or rows[-1]['digest'] != self.state.digest():
            raise ValueError('unit census digest mismatch')
        return sorted([Unit(int(k), int(size), Path(target), Path(free) if free else None)
                       for k, _, size, target, free in rows[-1]['units']], key=lambda u: (-u.size, u.key))

    def checked(self, table, unit, word, work, rule, *, strict=True, parts=None):
        work.mkdir(parents=True, exist_ok=True)
        before = read_json(table)['digest']
        word = literal(word, before)
        wf, report, end = work / 'word.json', work / 'check.json', work / 'end-table.json'
        atomic_json(wf, word)
        child = self.script(LOW / 'round22/stepchk.py', [table, unit.target, wf, report, end], self.rel(work))
        self.require(child)
        if not report.exists() or not end.exists():
            raise RunError('stepchk wrote no end table')
        check = read_json(report)
        if (check['start_digest'] != before or check.get('output_digest_matches') is False or
                read_json(end)['digest'] != check['end_digest']):
            raise RunError('stepchk digest mismatch')
        if strict and not check['strict_prime']:
            raise RuleFailure('not strict_prime', 'whole literal word fails strict_prime')
        return Candidate(rule, literal(word, before, check['end_digest']), check, end, parts)

    def closure(self, table, unit, core, work, *, prefix_length=0):
        work.mkdir(parents=True, exist_ok=True)
        core_file = work / 'core.json'
        atomic_json(core_file, core)
        out = work / 'closure'
        argv = [TH, EC, table, unit.target, core_file, out,
                '--greedy-seconds', api.child_clock(self.mode, self.args.greedy_seconds)]
        child = self.script(GM / 'tclose2.py', argv, self.rel(work) + '-close')
        try:
            pre = read_json(out / 'pre.json')
        except (OSError, ValueError):
            # A timeout may interrupt the existing script's non-atomic write.
            # Its process outcome remains the failed attempt's primary evidence.
            pre = None
        readable = isinstance(pre, dict)       # tclose2 writes pre.json before every exit 0
        pre = pre if readable else {}
        vacancy = pre.get('vacancy', {})
        vacancy = vacancy if isinstance(vacancy, dict) else {}
        details = {key: vacancy[key] for key in ('N1', 'N2', 'deficits') if key in vacancy}
        greedy = pre.get('greedy', {})
        if isinstance(greedy, dict) and greedy.get('timed_out') is True:
            # tclose2 stopped its greedy vacancy search on its own clock (tclose2.py:171-186).
            details['timed_out'] = True
        try:
            self.require(child)
            if not readable:
                raise RunError('tclose2 wrote no readable pre.json')
            result = read_json(out / 'result.json') if (out / 'result.json').exists() else {}
            result = result if isinstance(result, dict) else {}
            middle = literal({'word': core['P'] + core['Q'] + self.m.inverse_word(core['P'])})['word']
            if (out / 'word-full.json').exists():
                word = literal(read_json(out / 'word-full.json'), read_json(table)['digest'])
                n, remainder = divmod(len(word['word']) - len(middle), 2)
                v = word['word'][:n]
                if (n < 0 or remainder or word['word'][n:n + len(middle)] != middle or
                        word['word'][n + len(middle):] != self.m.inverse_word(v)):
                    raise RunError('tclose2 word is not V middle V_inverse')
                details.update(V_source='tclose2', parts=[['V', n], ['middle', len(middle)], ['V_inverse', n]],
                               V_letters=n, V_sha256=word_sha256(v))
                return word, details
            ghost = pre.get('ghost', {})
            if (not self.args.literal_limit or not isinstance(ghost, dict) or not ghost or
                    ghost.get('T_size') is None or
                    ghost.get('T_on_lattice_at_end') != ghost.get('T_size') or
                    ghost.get('protected_exact_at_end') is not True or
                    any(vacancy.get(key) is False for key in ('N1', 'N2'))):
                raise RuleFailure('no literal closure', result.get('result', 'no literal V'))
            return self.certificate_word(table, middle, ghost, work, details, prefix_length), details
        except RuleFailure as exc:
            exc.details.update(premise_details(details))
            raise
        except (OSError, EOFError, ValueError, KeyError, TypeError) as exc:
            # Every child here exited cleanly, so unreadable or inconsistent evidence is malformed output.
            raise RunError(f'invalid closure evidence: {exc}') from exc

    def certificate_word(self, table, middle, ghost, work, details, prefix_length):
        """Build V at this table; a certificate never crosses the literal boundary."""
        target = ghost['badW']
        if (not isinstance(target, list) or any(type(c) is not int for c in target) or
                target != sorted(set(target)) or word_sha256(target) != ghost['badW_digest']):
            raise RunError('Bad(W) digest mismatch')
        digest = read_json(table)['digest']
        details.update(T_size=len(target), T_digest=ghost['badW_digest'])
        tf = work / 'target.json'
        atomic_json(tf, {'T': target, 'T_digest': details['T_digest'], 'state_digest': digest})
        child = self.script(LOW / 'round22/jordan_target.py', [tf, table], self.rel(work) + '-jordan')
        # Its CPU cap can cut off the later Jordan search after CONDITION is
        # complete. The certificate builder/checker supplies the actual witness.
        # The hard limit SIGKILLs when the cap's handler cannot run; the child is
        # single-threaded, so a SIGKILL before the cap's wall time is a crash.
        capped = child.rc == 124 or (child.rc == -signal.SIGKILL and child.seconds >= JORDAN_CPU_SECONDS)
        self.require(child, returncodes=(0, child.rc) if capped else (0,))
        lines = child.log.read_text().splitlines()
        conditions = [json.loads(line[len('CONDITION '):]) for line in lines if line.startswith('CONDITION ')]
        if not conditions:
            if capped or 'CUT_SHORT_CPU_840_SECONDS' in lines:
                raise RuleFailure('timeout', 'jordan_target CPU cap before CONDITION', timed_out=True)
            # jordan_target prints CONDITION before every exit 0 (jordan_target.py:31).
            raise RunError('jordan_target wrote no target condition')
        condition = conditions[-1]
        details.update(N1=condition['fixed_vacant'], N2=condition['necessary'])
        if not (details['N1'] is True and details['N2'] is True):
            details['deficits'] = condition.get('necessary_deficits', {})
            raise RuleFailure('no literal closure', 'N1 or N2 fails')
        cert = work / 'certificate.json.gz'
        cert_seconds = api.child_clock(self.mode, self.args.cert_seconds)
        child = self.script(LOW / 'r22b10_cert.py', ['table', table, tf, cert,
            '--seconds', cert_seconds, '--seeds', self.args.seeds], self.rel(work) + '-cert')
        self.require(child)
        evidence = read_json(cert)
        # Both r22b10/1 and r22b10/2 share these three fields.
        if evidence.get('complete') is not True:
            if cert_timed_out(json_rows(child.log), evidence, cert_seconds, child.seconds):
                raise RuleFailure('timeout', 'certificate builder ended on its clock', timed_out=True)
            raise RuleFailure(self.needed('no literal closure'), 'incomplete certificate within search budget')
        if evidence.get('state_digest') != digest:
            raise RunError('certificate state digest mismatch')
        n = evidence['V_length']
        if type(n) is not int or n < 0:
            raise RunError('invalid certificate length')
        child = self.script(LOW / 'check_r22b10.py', ['table', table, tf, cert], self.rel(work) + '-cert-check')
        # Exit 1 with a last line 'FAIL: <reason>' is the checker's rejection (check_r22b10.py:483-485).
        self.require(child, returncodes=(0, 1))
        lines = child.log.read_text().splitlines()
        if child.rc == 1:
            reason = lines[-1] if lines else ''
            if not reason.startswith('FAIL: ') or any(phrase in reason for phrase in CHECK_INTEGRITY):
                raise RunError('certificate check: ' + (reason or 'exit 1 without a FAIL line'))
            raise RuleFailure(self.needed('no literal closure'), 'certificate check ' + reason)
        tokens = ' '.join(lines).split()
        if not tokens or tokens[-1] != 'PASS':
            raise RunError('certificate check exited 0 without PASS')
        if prefix_length + 2 * n + len(middle) > self.args.literal_limit:
            raise RuleFailure(self.needed('over literal limit'), 'whole step exceeds literal_limit', V_length=str(n))
        vf = work / 'V.json.gz'
        child = self.script(LOW / 'expand_slp.py', [cert, vf, '--limit', self.args.literal_limit],
                            self.rel(work) + '-expand')
        if child.rc == 3 and not child.timeout and not child.cancelled:
            self.check_signal()
            rows = json_rows(child.log)
            if rows and rows[-1].get('over_limit') is True:
                raise RuleFailure(self.needed('over literal limit'), 'expansion exceeds literal_limit', V_length=str(n))
        self.require(child)
        rows = json_rows(child.log)
        expansion = rows[-1] if rows else {}
        v = literal(read_json(vf), digest)['word']
        if expansion.get('letters') != n or len(v) != n or expansion.get('word_sha256') != word_sha256(v):
            raise RunError('literal expansion length or hash mismatch')
        details.update(V_source='certificate', parts=[['V', n], ['middle', len(middle)], ['V_inverse', n]],
                       V_letters=n, V_sha256=word_sha256(v),
                       certificate_sha256=hashlib.sha256(cert.read_bytes()).hexdigest())
        return {'word': v + middle + self.m.inverse_word(v), 'input_digest': digest}

    def closure_checked(self, table, unit, word, details, work, rule):
        try:
            if details['V_source'] == 'certificate':
                candidate = self.slp_checked(table, unit, word, details['parts'], work, rule,
                                             prefix_digest=details.get('_prefix_digest'))
            else:
                candidate = self.checked(table, unit, word, work, rule, parts=details['parts'])
            candidate.metadata = {key: details[key] for key in
                ('V_source', 'V_letters', 'V_sha256', 'T_size', 'T_digest', 'N1', 'N2') if key in details}
            candidate.certificate_sha256 = details.get('certificate_sha256')
            # Vacancy information for literal tclose2 steps belongs only to attempts.
            if details['V_source'] == 'tclose2':
                candidate.metadata = {k: v for k, v in candidate.metadata.items() if k.startswith('V_')}
            return candidate
        except RuleFailure as exc:
            exc.details.update(premise_details(details))
            raise

    def close(self, table, unit, core, work):
        word, details = self.closure(table, unit, core, work)
        return self.closure_checked(table, unit, word, details, work / 'whole', 'close')

    def slp_checked(self, table, unit, word, parts, work, rule, *, prefix_digest=None):
        """Check every literal part and bind the SAFE-ALL hashes to our word."""
        work.mkdir(parents=True, exist_ok=True)
        before = read_json(table)['digest']
        word = literal(word, before)
        segments, expected, offset = [], [], 0
        for name, n in parts:
            part = word['word'][offset:offset + n]
            offset += n
            path = work / (name + '.json')
            atomic_json(path, {'word': part})
            kind = 'safe_word_file' if name in ('V', 'V_inverse') else 'word_file'
            segments.append({kind: str(path)})
            expected.append((kind, n, word_sha256(part)))
        if offset != len(word['word']):
            raise RunError('literal parts do not cover the word')
        program, end = work / 'program.json', work / 'end-table.json'
        atomic_json(program, {'segments': segments})
        child = self.script(LOW / 'slp_apply.py', ['table', table, program, end, '--unit', unit.target], self.rel(work))
        self.require(child)
        try:
            rows = json_rows(child.log)
            check = rows[-1] if rows else {}
            if type(check.get('strict_prime')) is not bool:
                raise RunError('slp_apply wrote no summary row')
            if check.get('start_digest') != before or read_json(end)['digest'] != check.get('end_digest'):
                raise RunError('slp_apply digest mismatch')
            part_rows = [r for r in rows if 'segment' in r]
            if len(part_rows) != len(expected):
                raise RunError('slp_apply segment count mismatch')
            for index, (row, (kind, n, sha)) in enumerate(zip(part_rows, expected)):
                if (row.get('segment') != index or row.get('kind') != kind or row.get('length') != n or
                        not isinstance(row.get('digest'), str) or len(row['digest']) != 64 or
                        any(c not in '0123456789abcdef' for c in row['digest']) or
                        (kind == 'safe_word_file' and row.get('word_sha256') != sha)):
                    raise RunError('slp_apply literal segment mismatch')
            if prefix_digest is not None and part_rows[0]['digest'] != prefix_digest:
                raise RunError('slp_apply W end digest mismatch')
            if part_rows[-1]['digest'] != check['end_digest']:
                raise RunError('slp_apply final segment digest mismatch')
            # slp_apply writes every row and the end table whether or not the step is strict prime,
            # so the evidence is checked first and only a well-formed failure is an attempt.
            if not check['strict_prime']:
                raise RuleFailure('not strict_prime', 'literal program fails strict_prime')
            return Candidate(rule, literal(word, before, check['end_digest']), check, end, parts, 'slp_apply')
        except (OSError, ValueError, KeyError, TypeError) as exc:
            # slp_apply exited cleanly, so unreadable or inconsistent evidence is malformed output.
            raise RunError(f'invalid slp_apply evidence: {exc}') from exc

    def t1prime(self, table, unit, wfile, work):
        """The tail's literal T1' path, also callable with a supplied landing W.

        Returns a checked literal Candidate or raises RuleFailure with premises.
        """
        work.mkdir(parents=True, exist_ok=True)
        landing = self.checked(table, unit, read_json(wfile), work / 'landing', 't1prime', strict=False)
        check = landing.check
        if (check['unit_on_lattice'] != check['unit_size'] or
                check['others_exact'] + check['others_landed'] != check['others']):
            raise RuleFailure('no landing word', 'W fails the T1prime landing premises')
        if check['strict_prime']:
            landing.parts = [['W', len(landing.word['word'])]]
            return landing
        wf = work / 'W.json'
        atomic_json(wf, landing.word)
        child = self.script(LOW / 'round22/prep_t1prime.py', [table, wf, 't1', work], self.rel(work) + '-prep')
        self.require(child)
        z1, target, inverse = [work / ('t1-' + suffix) for suffix in
                               ('z1-table.json', 'Q.json', 'winv.json')]
        if not all(p.exists() for p in (z1, target, inverse)):
            raise RunError('T1prime preparation produced no inputs')
        q = Unit(unit.key, len(read_json(target)['pieces']), target)
        core = read_json(inverse)
        if (read_json(z1)['digest'] != landing.check['end_digest'] or
                core != {'P': [], 'Q': self.m.inverse_word(landing.word['word'])}):
            raise RunError('T1prime inverse or prepared state mismatch')
        closure, details = self.closure(z1, q, core, work / 'inverse',
                                        prefix_length=len(landing.word['word']))
        try:
            closure = literal(closure, landing.word['output_digest'])
            word = {'word': landing.word['word'] + closure['word'], 'input_digest': landing.word['input_digest']}
            if 'output_digest' in closure:
                word['output_digest'] = closure['output_digest']
            details['parts'] = [['W', len(landing.word['word'])]] + details['parts']
            details['_prefix_digest'] = landing.check['end_digest']
            return self.closure_checked(table, unit, word, details, work / 'whole', 't1prime')
        except RuleFailure as exc:
            exc.details.update(premise_details(details))
            raise

    def try_unit(self, table, unit, work):
        work.mkdir(parents=True, exist_ok=True)
        cached = {}
        attempts = []

        def dependency(name):
            if name not in cached:
                try:
                    out = work / (name + '.json')
                    if name == 'pure':
                        child = self.script(LOW / 'cellpure5.py', [TH, table, unit.target, out, '--search'],
                                            self.rel(work) + '-pure')
                        self.require(child, screens=SCREENS)
                        wf = out.with_name(out.stem + '-word.json')
                        if not wf.exists():
                            raise RuleFailure('no word', 'pure search found no word')
                        cached[name] = wf
                    else:
                        child = self.script(LOW / 'round22/transport_census.py', [table, out, unit.target],
                                            self.rel(work) + '-transport')
                        self.require(child, screens=SCREENS)
                        rows = read_json(out)['units']
                        cached[name] = rows[unit.target.stem]
                except RuleFailure as exc:
                    cached[name] = exc
            if isinstance(cached[name], RuleFailure):
                raise cached[name]
            return cached[name]

        def failed(rule, start, exc, source=None):
            attempt = attempt_row(exc, unit_key=unit.key, rule=rule, seconds=round(time.monotonic() - start, 6))
            if source:
                attempt['reason'] = source + ': ' + attempt['reason']
            attempts.append(attempt)

        for rule in self.args.tail_rules:
            if self.cancel.is_set() or self.signal_number is not None:
                break
            start = time.monotonic()
            try:
                if rule == 'zero_bad':
                    zero = dependency('transport').get('zero_bad')
                    if not isinstance(zero, dict) or not zero.get('strict'):
                        raise RuleFailure('no word', 'transport found no strict zero-Bad word')
                    candidate = self.checked(table, unit, {'word': zero['word']}, work / 'zero-check', rule)
                elif rule == 'pure':
                    candidate = self.checked(table, unit, read_json(dependency('pure')), work / 'pure-check', rule)
                elif rule == 't1prime':
                    candidate = self.t1prime(table, unit, dependency('pure'), work / 't1prime')
                else:
                    candidate = None
                    # Fixed seed order, even when the user omits a prerequisite rule.
                    for source in ('pure', 'transport'):
                        start = time.monotonic()
                        details = {}
                        try:
                            if source == 'pure':
                                word = read_json(dependency(source))['word']
                            else:
                                small = dependency(source).get('small_bad')
                                if not isinstance(small, dict) or not small.get('W_ok'):
                                    raise RuleFailure('no word', 'transport found no W_ok core')
                                word = small['word']
                            candidate = self.close(table, unit, {'P': [], 'Q': word}, work / ('close-' + source))
                            break
                        except RuleFailure as exc:
                            exc.details.update(premise_details(details))
                            failed(rule, start, exc, source)
                    if candidate is None:
                        continue
                return Trial(candidate, attempts)
            except RuleFailure as exc:
                failed(rule, start, exc)
        return Trial(attempts=attempts)

    def choose(self, units, work):
        """Evaluate concurrently, consume in the same order as a serial run."""
        attempts, by_unit = [], []
        self.check_signal()
        self.cancel.clear()
        pool = ThreadPoolExecutor(max_workers=self.args.par)
        futures = [pool.submit(self.try_unit, self.table, u, work / f'u{u.key}') for u in units]
        chosen_unit = candidate = error = None
        retry = []
        try:
            for unit, future in zip(units, futures):
                try:
                    trial = future.result()
                except Exception as exc:
                    # A serial run reaches this unit only when no earlier unit's retry below takes a step.
                    error = exc
                    break
                attempts.extend(trial.attempts)
                by_unit.append({'unit_key': unit.key, 'size': unit.size, 'attempts': trial.attempts.copy()})
                if trial.candidate is not None:
                    chosen_unit, candidate = unit, trial.candidate
                    break
                if self.args.par > 1 and any(a.get('timed_out') for a in trial.attempts):
                    retry.append((unit, by_unit[-1]))
        finally:
            self.cancel.set()
            for future in futures:
                future.cancel()
            pool.shutdown(wait=True, cancel_futures=True)
            if self.signal_number is None:
                self.cancel.clear()
        self.check_signal()
        # All competitors have exited before the serial retry gets the machine.
        for unit, entry in retry:
            trial = self.try_unit(self.table, unit, work / f'u{unit.key}-serial')
            attempts.extend(trial.attempts)
            entry['attempts'].extend(trial.attempts)
            self.check_signal()
            if trial.candidate is not None:
                chosen_unit, candidate = unit, trial.candidate
                break
        if candidate is None and error is not None:
            raise error
        return chosen_unit, candidate, attempts, by_unit

    def fallback(self, unit, work):
        out = work / 'fallback'
        child = self.script(GM / 'fallback.py', [TH, EC, self.table, unit.target, out,
            '--max-length', self.args.fallback_max_length], self.rel(work) + '-fallback')
        if child.rc == 3 and not child.timeout:
            raise RuleFailure('bound exhausted', 'no word within fallback_max_length')
        self.require(child)
        if not (out / 'word-full.json').exists():
            raise RunError('fallback wrote no literal word')
        candidate = self.checked(self.table, unit, read_json(out / 'word-full.json'),
                                 work / 'fallback-check', 'fallback', strict=False)
        if (candidate.check['collateral'] != 0 or
                candidate.check['unit_on_lattice'] != candidate.check['unit_size']):
            # fallback.py requires both before it writes its word (:92, :102): the replays disagree.
            raise RunError('fallback does not land the unit without collateral')
        return candidate

    def advance(self, unit, candidate, attempts, seconds):
        replay_started = time.monotonic()
        index = len(self.record['steps'])
        phase = 'fallback' if candidate.rule == 'fallback' else 'tail'
        before = self.m.snapshot(self.state)
        off = self.m.off_set(self.state)
        pieces = read_json(unit.target)['pieces']
        if len(set(pieces)) != unit.size or not set(pieces) <= off:
            raise ValueError('unit is not a current off unit')
        word = literal(candidate.word, before['digest'], candidate.check['end_digest'])
        certificate = candidate.gate == 'slp_apply'
        if certificate and not self.acceptance:
            self.state = self.m.load(candidate.end_table)
            if self.state.digest() != word['output_digest']:
                raise ValueError('slp_apply end digest mismatch on load')
        else:
            # Acceptance mode plays a certificate-built closure twist by twist too (A2).
            self.m.apply_word(self.state, word)
        after = self.m.snapshot(self.state)
        if (after['off'] != candidate.check['off_after'] or
                read_json(candidate.end_table)['digest'] != after['digest'] or
                not self.m.off_set(self.state) <= off - set(pieces)):
            raise ValueError('step failed own replay or off-set contract')
        suffix = '.json.gz' if len(word['word']) > 100000 else '.json'
        word_file = self.out / 'words' / f'{index:05d}{suffix}'
        unit_file = self.out / 'units' / f'{index:05d}.json'
        atomic_json(word_file, word)
        shutil.copyfile(unit.target, unit_file)
        self.table = self.out / 'tables' / f'{index:05d}.json'
        if self.acceptance:
            self.m.export(self.state, self.table)
        else:
            shutil.copyfile(candidate.end_table, self.table)
        letters = len(word['word'])
        tree = self.write_tree(index, self.step_tree(candidate.rule, candidate.parts, letters), letters)
        standing = ('fallback' if phase == 'fallback' else
                    'certificate' if certificate and not self.acceptance else 'rule')
        row = {'index': index, 'phase': phase, 'subphase': None, 'rule': candidate.rule,
            'unit': self.rel(unit_file), 'unit_key': unit.key, 'unit_size': unit.size,
            'word': self.rel(word_file), 'letters': letters, 'encoding': 'json',
            'word_sha256': word_sha256(word['word']), 'tree': tree,
            'contract': 'fallback' if phase == 'fallback' else 'strict_prime',
            'gate': candidate.gate, 'standing': standing,
            'before': before, 'after': after, 'table': self.rel(self.table), 'landed': unit.size,
            'others_landed': candidate.check['others_landed'], 'attempts': attempts,
            'seconds': round(seconds + time.monotonic() - replay_started, 6)}
        if candidate.parts is not None:
            row['parts'] = candidate.parts
        row.update(candidate.metadata)
        if certificate:
            row.update(construction='certificate', certificate_sha256=candidate.certificate_sha256)
        self.record['steps'].append(row)
        self.record['phases']['tail']['steps'] += 1
        self.record['phases']['tail']['twists'] += letters
        self.save()

    @staticmethod
    def step_tree(rule, parts, letters):
        """A tail step's tree from the parts its rule already splits; V⁻¹ names V."""
        if not parts:
            return api.single(rule, letters)
        index = {name: i for i, (name, _) in enumerate(parts)}
        inverse = {index['V_inverse']: index['V']} if {'V', 'V_inverse'} <= set(index) else {}
        if rule == 't1prime' and {'W', 'middle'} <= set(index):
            # T1''s core is P = [], Q = W⁻¹ (checked in t1prime), so its middle is W⁻¹.
            inverse[index['middle']] = index['W']
        names = {'V_inverse': 'V⁻¹'}
        return api.flat(rule, [(names.get(name, name), n) for name, n in parts], inverse=inverse)

    def write_probe(self, unit, candidate):
        """Acceptance mode: a fallback word is evidence of a coverage gap, never played."""
        root = self.out / 'probe'
        root.mkdir(parents=True, exist_ok=True)
        word_file, unit_file = root / 'word.json', root / 'unit.json'
        atomic_json(word_file, candidate.word)
        shutil.copyfile(unit.target, unit_file)
        return {'unit': self.rel(unit_file), 'unit_key': unit.key, 'word': self.rel(word_file),
                'letters': len(candidate.word['word']), 'input_digest': candidate.word['input_digest'],
                'output_digest': candidate.check['end_digest']}

    def tail(self):
        phase = self.record['phases']['tail']
        if phase['result'] is not None:
            return
        if not phase['steps'] and not self.entry('tail'):
            phase['result'] = 'not_applicable'
            self.save()
            return
        while self.state.off_lattice_count():
            if self.args.tail_steps and phase['steps'] >= self.args.tail_steps:
                phase['result'] = self.record['result'] = 'budget'
                self.save()
                return
            started = time.monotonic()
            work = self.out / 'attempts' / f'{len(self.record["steps"]):05d}'
            units = self.units(self.table, work / 'units')
            unit, candidate, attempts, by_unit = self.choose(units, work)
            probe = None
            if candidate is None and units and self.args.fallback_max_length:
                unit = units[0]
                start = time.monotonic()
                try:
                    candidate = self.fallback(unit, work)
                except RuleFailure as exc:
                    by_unit[0]['attempts'].append(attempt_row(exc, unit_key=unit.key, rule='fallback',
                                                              seconds=round(time.monotonic() - start, 6)))
                if self.acceptance:
                    probe = self.write_probe(unit, candidate) if candidate is not None else False
                    candidate = None
            if candidate is None:
                # README section 1, rule 4: a stop with a timed-out attempt is a budget end, not stuck.
                timed_out = any(a.get('timed_out') for u in by_unit for a in u['attempts'])
                self.record['stuck'] = dict(self.m.snapshot(self.state), phase='tail',
                                            reason='timeout' if timed_out else 'no_rule', units=by_unit)
                if probe:
                    self.record['stuck']['probe'] = probe
                if probe is not None:
                    self.record['acceptance']['probe'] = bool(probe)
                phase['result'] = 'timeout' if timed_out else 'stuck'
                self.record['result'] = 'budget' if timed_out else 'stuck'
                self.save()
                return
            self.advance(unit, candidate, attempts, time.monotonic() - started)
        phase['result'] = 'lattice'
        self.save()

    def lattice(self):
        if self.state.digest() == SOLVED:
            self.record['result'] = 'solved'
            self.save()
            return
        if self.args.no_lattice:
            self.record['result'] = 'budget'
            self.save()
            return
        plugin = self.plugin_modules()['lattice']
        data = self.record['phases']['lattice']
        data.pop('attempts', None)              # discard a cancelled, uncommitted play
        end = plugin.play(self.context('lattice', None))
        if self.signal_number is not None:
            # A signal during the phase is the operator's (P3-robust-01).
            data['attempts'] = list(getattr(end, 'attempts', []))
            self.save()
            self.check_signal()
        if isinstance(end, api.LatticeResult):
            self.played_lattice(end)
            return
        if not isinstance(end, api.End):
            raise ValueError('the lattice plugin returns a LatticeResult or an End')
        if end.result != 'not_applicable' or not getattr(plugin, 'STUB', False):
            self.lattice_end(end)
            return
        # The stub before P3 plays nothing: the retained part is checked by composing its program (R22D-57).
        self.component('lattice_composition')
        started = time.monotonic()
        out = self.out / 'lattice'
        out.mkdir(parents=True, exist_ok=True)
        library = out / 'library-merged.json'
        library.write_bytes(gzip.decompress((ROOT / LIBRARY).read_bytes()))
        spec = 'table:' + self.rel(self.table)
        info = {'input_digest': self.state.digest(), 'library': self.rel(library),
            'library_sha256': hashlib.sha256(library.read_bytes()).hexdigest(), 'out': 'lattice/',
            'input': spec, 'restore_rc': None, 'verify_rc': None, 'verify_last_line': '', 'chain': False,
            'twists': None, 'seconds': 0.0}
        self.record['lattice'] = info
        self.save()
        restore = self.script(LAT / 'restore_lattice_late.py',
            [ROOT / LAT, ROOT, ROOT / J, self.rel(library), 'lattice', spec],
            'lattice-restore', unlimited=True, cwd=self.out)
        self.check_signal()
        report_file = out / 'report.json'
        argv = [ROOT, ROOT / J, self.rel(library), 'lattice', 'lattice/report.json']
        verify = self.script(CHECKER, argv, 'lattice-verify', unlimited=True, cwd=self.out)
        self.check_signal()
        lines = verify.log.read_text().splitlines()
        summary = read_json(out / 'restore-summary.json') if (out / 'restore-summary.json').exists() else {}
        report = read_json(report_file) if report_file.exists() else {}
        runs, checked = summary.get('runs', []), report.get('runs', [])
        matching = len(runs) == 1 and runs[0].get('input') == spec
        twists = None
        if matching and 'retained_length' in runs[0]:
            twists = runs[0]['retained_length'] + runs[0].get('correction_twists', 0)
        info.update(restore_rc=restore.rc, verify_rc=verify.rc,
                    verify_last_line=safe_reason(lines[-1]) if lines else '', twists=twists,
                    seconds=round(time.monotonic() - started, 6))
        passed = (restore.rc == 0 and verify.rc == 0 and info['verify_last_line'].startswith('PASS ') and
            report.get('pass') is True and len(checked) == 1 and checked[0].get('input') == spec and
            checked[0].get('restored_to_identity') is True and matching and
            runs[0].get('status') == 'restored' and type(twists) is int and twists >= 0)
        self.record['totals']['lattice_twists'] = twists
        self.record['result'] = 'solved' if passed else 'error'
        self.record['error'] = None if passed else 'lattice verification failed'
        self.record['phases']['lattice'].update(result=self.record['result'], twists=twists)
        self.save()

    def lattice_end(self, end):
        """A lattice phase with no played step: stuck, budget, timeout or error (plan revision 3e, E2)."""
        if end.result not in ('stuck', 'budget', 'timeout', 'error'):
            raise ValueError(f'lattice claims {safe_reason(end.result)} with every piece on the lattice')
        if any(type(a.get('timed_out')) is not bool for a in end.attempts):
            raise ValueError('every plugin attempt records timed_out')
        data = self.record['phases']['lattice']
        # Nothing was played: steps stays 0 and twists null, as for a phase not entered.
        data.update(result=end.result, reason=end.reason, details=end.details, attempts=end.attempts)
        if end.result == 'error':
            self.save()
            raise RuntimeError(f'lattice phase error: {safe_reason(end.reason)}')
        if end.result == 'stuck':
            self.record['stuck'] = dict(self.m.snapshot(self.state), phase='lattice', reason=end.reason, units=[])
        self.record['result'] = 'stuck' if end.result == 'stuck' else 'budget'
        self.save()

    def lattice_report(self, report):
        path = self.out / 'lattice' / 'report.json'
        path.parent.mkdir(parents=True, exist_ok=True)
        atomic_json(path, report)
        return {'file': self.rel(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}

    def played_lattice(self, result):
        """Record the lattice phase the plugin played twist by twist in its child (P3).

        Every step's word file, tree and digest chain is checked before anything changes. The
        runner does not replay the word: it loads the end table, which must reach the last
        step's after digest (plan, section 2); the checker replays every lattice word."""
        steps = result.steps
        if not (isinstance(steps, list) and all(isinstance(s, api.LatticeStep) for s in steps)
                and isinstance(result.report, dict)):
            raise ValueError('a played lattice phase has LatticeSteps and a report')
        expected = self.m.snapshot(self.state)
        if not steps:
            # The restorer scheduled nothing: no row carries labels_identity, so the runner tests the
            # labels of the current state itself, and the checker tests them again (revision 3e, E2).
            player = result.report.get('player', {})
            if (player.get('played', {}).get('total') != 0 or read_json(result.end_table).get('digest')
                    != expected['digest'] or player.get('final', {}).get('labels_identity') is not True):
                raise ValueError('a lattice phase with no step must schedule nothing and keep the state')
            if not self.m.labels_identity(self.state):
                raise ValueError('a lattice phase with no step leaves labels that are not the identity')
            self.record['phases']['lattice'].update(result='solved', steps=0, twists=0,
                                                    report=self.lattice_report(result.report))
            self.record['totals']['lattice_twists'] = 0
            self.record['result'] = 'solved'
            self.save()
            return
        for s in steps:
            if ((s.contract, s.gate, s.standing, s.encoding) != ('lattice_literal', 'player', 'rule', 'ia16')
                    or not isinstance(s.rule, str) or not s.rule.startswith('lattice/')
                    or s.before != expected or not lattice_snapshot(s.after) or not isinstance(s.fields, dict)
                    or ROW_KEYS.intersection(s.fields) or api.FORBIDDEN_KEYS.intersection(s.fields)
                    or type(s.seconds) not in (int, float) or not math.isfinite(s.seconds) or s.seconds < 0):
                raise ValueError('lattice step does not match its phase, record keys or digest chain')
            if api.ia16_digest(s.word_file) != {'sha256': s.word_sha256, 'letters': s.letters}:
                raise ValueError('lattice word file does not match its sha256 or length')
            root, sha = api.read_tree(s.tree_file)
            if sha != s.tree_sha256 or api.check_tree(root, s.letters) != s.tree_size:
                raise ValueError('lattice step tree does not match its sha256 or size')
            expected = s.after
        if steps[-1].fields.get('labels_identity') is not True:
            raise ValueError('the lattice phase does not end with identity labels')
        if read_json(result.end_table).get('digest') != expected['digest']:
            raise ValueError('lattice end table digest differs from the last step')
        state = self.m.load(result.end_table)
        if self.m.snapshot(state) != expected:
            raise ValueError('lattice end table does not load to the last step')
        # Checked: the files move into the record unchanged, and the state follows the end table.
        for s in steps:
            index = len(self.record['steps'])
            word = self.out / 'words' / f'{index:05d}.ia16.gz'
            tree = self.out / 'step-trees' / f'{index:05d}.json.gz'
            shutil.move(s.word_file, word)
            shutil.move(s.tree_file, tree)
            row = {'index': index, 'phase': 'lattice', 'subphase': None, 'rule': s.rule, 'word': self.rel(word),
                   'letters': s.letters, 'encoding': s.encoding, 'word_sha256': s.word_sha256,
                   'tree': {'file': self.rel(tree), 'sha256': s.tree_sha256, 'size': s.tree_size},
                   'contract': s.contract, 'gate': s.gate, 'standing': s.standing, 'before': dict(s.before),
                   'after': dict(s.after), 'attempts': [], 'seconds': s.seconds, **s.fields}
            self.record['steps'].append(row)
        self.table = self.out / 'tables' / f'{index:05d}.json'
        shutil.move(result.end_table, self.table)
        row['table'] = self.rel(self.table)      # only the end table exists (m4)
        self.state = state
        twists = sum(s.letters for s in steps)
        self.record['phases']['lattice'].update(result='solved', steps=len(steps), twists=twists,
                                                report=self.lattice_report(result.report))
        self.record['totals']['lattice_twists'] = twists
        # Solved by identity labels; the digest may differ from SOLVED by rotations they do not see (R3-05).
        self.record['result'] = 'solved'
        self.save()

    def run(self):
        previous = {}
        for number in (signal.SIGTERM, signal.SIGHUP):
            previous[number] = signal.signal(number, self.on_signal)
        try:
            if getattr(self.args, 'resume', False):
                point = self.resume()
            else:
                self.initialize()
                point = 'bulk'
            self.check_signal()
            if point == 'bulk':
                self.bulk()
            if self.phases_between() == 'budget':
                self.record['result'] = 'budget'
                self.save()
            else:
                self.tail()
                if self.state.off_lattice_count() == 0:
                    self.lattice()
            self.check_signal()
        except Exception as exc:
            if self.record is None:
                raise
            with (self.out / 'logs/runner.log').open('w') as log:
                traceback.print_exc(file=log)
            self.record['result'] = 'error'
            self.record['error'] = str(exc) if isinstance(exc, RunSignal) else safe_reason(exc)
            self.save()
        finally:
            for number, handler in previous.items():
                signal.signal(number, handler)
            if self.record is not None:
                self.finish()
        return self.record

    def finish(self):
        """Counts and the acceptance verdict; the checker recomputes them (plan, section 0)."""
        record = self.record
        acceptance, steps = record['acceptance'], record['steps']
        acceptance['certificate_steps'] = sum(1 for s in steps if s.get('standing') == 'certificate')
        acceptance['fallback_steps'] = sum(1 for s in steps if s.get('standing') == 'fallback')
        phases = [record['phases']['height']['sub'][k] for k in api.SUBPHASES['height']]
        phases += [record['phases']['high'], record['phases']['lattice']]
        stuck = record['stuck'] or {}
        final = [a for unit in stuck.get('units', []) for a in unit['attempts']]
        attempts = [a for s in steps for a in s.get('attempts', [])] + final
        attempts += [a for data in phases for a in data.get('attempts', [])]
        acceptance['timed_out_attempts'] = sum(1 for a in attempts if a.get('timed_out'))
        digest = steps[-1]['after']['digest'] if steps else record['phases']['bulk']['end']['digest']
        final += [a for data in phases if data.get('end') and data['end']['digest'] == digest
                  for a in data.get('attempts', [])]
        # A lattice End plays nothing, so its attempts are at the final state (plan revision 3e, E2).
        final += record['phases']['lattice'].get('attempts', [])
        acceptance['verdict'] = verdict(record, any(a.get('timed_out') for a in final))
        self.save()


def bulk_child(out, cache):
    """Only this child invokes the census rules; its record is the commit point."""
    parent = int(os.environ.get('RUNNER_PARENT_PID', os.getppid()))

    def parent_watch():
        while os.getppid() == parent:
            time.sleep(0.1)
        os.killpg(os.getpgrp(), signal.SIGKILL)

    threading.Thread(target=parent_watch, daemon=True).start()
    record = read_json(out / 'run.json')
    budgets = record['config']['budgets']
    args = argparse.Namespace(out=out, cache=cache)
    runner = Runner(args, Mechanics(cache))
    runner.record = record
    runner.started -= record['totals']['seconds']
    runner.state = state = runner.m.load(out / 'start-table.json')
    runner.table = out / 'tables/bulk-end.json'
    for step in record['steps']:
        runner.m.apply_word(state, read_json(out / step['word']))
    recount(record)
    # With reopen excluded, the rules use state-derived caches only; moves is
    # the output of this child's future words, never an input to their choices.
    restorer = runner.m.bulk_restorer(state, runner.m.menu, order=tuple(record['config']['bulk_order']))
    assert list(restorer.order) == BULK_ORDER and 'reopen' not in restorer.order
    methods = {'seam': restorer.seam, 'cover': restorer.cover, 'close': restorer.level1,
               'lock': restorer.level2, 'improve': restorer.improve, 'shift': restorer.shift,
               'conj': restorer.conj}
    phase = record['phases']['bulk']
    phase['completed'] = False
    while True:
        n = int(restorer.misalignment())
        if n == 0:
            phase['result'] = 'lattice'
            break
        if budgets['bulk_steps'] is not None and phase['steps'] >= budgets['bulk_steps']:
            phase['result'] = 'budget'
            break
        started = time.monotonic()
        atomic_json(out / 'bulk-progress.json', {'step_started': started})
        before = {'digest': state.digest(), 'off': int(state.off_lattice_count()), 'N': n}
        first = len(restorer.moves)
        for rule in restorer.order:
            done = methods[rule]()
            if done is not None:
                break
        else:
            phase['result'] = 'stuck'
            break
        word = [list(x) for x in restorer.moves[first:]]
        after = {'digest': state.digest(), 'off': int(state.off_lattice_count()),
                 'N': int(restorer.misalignment())}
        if not word or after['N'] >= before['N']:
            raise ValueError('bulk rule failed its decreasing-N contract')
        index = len(record['steps'])
        suffix = '.json.gz' if len(word) > 100000 else '.json'
        path = out / 'words' / f'{index:05d}{suffix}'
        atomic_json(path, literal({'word': word}, before['digest'], after['digest']))
        runner.m.export(state, runner.table)
        # conj plays (c, x)(d, y)(c, z) (seams.py:220-253): three named one-letter parts. Every other
        # bulk rule is one span (plan revision 3e, E4).
        parts = (api.flat(rule, [('x', 1), ('y', 1), ('z', 1)]) if rule == 'conj' and len(word) == 3
                 else api.single(rule, len(word)))
        tree = runner.write_tree(index, parts, len(word))
        record['steps'].append({'index': index, 'phase': 'bulk', 'subphase': None, 'rule': rule,
            'word': runner.rel(path), 'letters': len(word), 'encoding': 'json',
            'word_sha256': word_sha256(word), 'tree': tree, 'standing': 'rule',
            'before': before, 'after': after, 'seconds': round(time.monotonic() - started, 6)})
        phase.update(steps=phase['steps'] + 1, twists=phase['twists'] + len(word),
                     end=dict(runner.m.snapshot(state), table=runner.rel(runner.table)))
        runner.save()
        atomic_json(out / 'bulk-progress.json', {'step_started': None})
    phase['completed'] = True
    atomic_json(out / 'bulk-progress.json', {'step_started': None})
    runner.save()


def parse_args(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument('--scramble', nargs=2, type=int, metavar=('K', 'SEED'))
    source.add_argument('--table', type=Path)
    parser.add_argument('--out', required=True, type=Path)
    parser.add_argument('--resume', action='store_true', help='continue a matching in-flight runner-run/3 record')
    parser.add_argument('--cache', type=Path)
    parser.add_argument('--bulk-steps', type=int, default=None, help='0 skips bulk; default unlimited')
    parser.add_argument('--bulk-step-seconds', type=float, default=3600)
    parser.add_argument('--bulk-seconds', type=float, default=0)
    parser.add_argument('--tail-rules', default=','.join(TAIL_RULES))
    parser.add_argument('--tail-steps', type=int, default=0)
    parser.add_argument('--rule-seconds', type=float, default=3600)
    parser.add_argument('--greedy-seconds', type=float, default=60)
    parser.add_argument('--literal-limit', type=int, default=10000000)
    parser.add_argument('--seeds', choices=('atlas', 'default'), default='atlas')
    parser.add_argument('--cert-seconds', type=float, default=3600)
    parser.add_argument('--fallback-max-length', type=int, default=1)
    parser.add_argument('--par', type=int, default=1)
    parser.add_argument('--no-lattice', action='store_true')
    parser.add_argument('--mode', choices=api.MODES, default='development',
                        help='acceptance: no clocks, certificate closures played, fallback only as a probe')
    # schedule11 is the acceptance restorer, chosen from the D0 runs (plan revision 3e, E5).
    parser.add_argument('--lattice-restorer', choices=('late', 'schedule11'), default='schedule11')
    parser.add_argument('--height-seconds', type=float, default=3600)
    parser.add_argument('--height-cpu', type=float, default=3600)
    parser.add_argument('--pair-shards', type=int, default=4)
    parser.add_argument('--kernel-seconds', type=float, default=3600)
    parser.add_argument('--kernel-wall-margin', type=float, default=600)
    parser.add_argument('--high-steps', type=int, default=None, help='a step budget for the high phase')
    args = parser.parse_args(argv)
    if '.json' in str(args.out.resolve()):
        parser.error('out path must not contain .json')
    args.tail_rules = args.tail_rules.split(',') if args.tail_rules else []
    if len(set(args.tail_rules)) != len(args.tail_rules) or any(r not in TAIL_RULES for r in args.tail_rules):
        parser.error('tail-rules must be distinct names from ' + ','.join(TAIL_RULES))
    for name in ('bulk_steps', 'bulk_seconds', 'tail_steps', 'greedy_seconds', 'fallback_max_length', 'literal_limit',
                 'high_steps', 'kernel_wall_margin'):
        value = getattr(args, name)
        if value is not None and (value < 0 or (isinstance(value, float) and not math.isfinite(value))):
            parser.error(name.replace('_', '-') + ' must be finite and nonnegative')
    for name in ('bulk_step_seconds', 'rule_seconds', 'cert_seconds', 'par', 'height_seconds', 'height_cpu',
                 'kernel_seconds', 'pair_shards'):
        value = getattr(args, name)
        if value <= 0 or (isinstance(value, float) and not math.isfinite(value)):
            parser.error(name.replace('_', '-') + ' must be finite and positive')
    if args.scramble and args.scramble[0] < 0:
        parser.error('scramble depth must be nonnegative')
    return args


def main(argv=None):
    argv = list(sys.argv[1:] if argv is None else argv)
    if argv and argv[0] == '--_script':
        script, _, cache, separator, *arguments = argv[1:]
        assert argv[2] == '--cache' and separator == '--'
        os.environ['JUMBLING_HOME_CACHE'] = str(Path(cache) / 'home_centroids.npy')
        Mechanics(cache)                    # redirects CACHE before the existing script runs
        sys.argv = [str(script)] + arguments
        runpy.run_path(str(script), run_name='__main__')
        return 0
    if argv and argv[0] == '--_bulk-child':
        _, out, option, cache = argv
        assert option == '--cache'
        bulk_child(Path(out), Path(cache))
        return 0
    args = parse_args(argv)
    # Children inherit this limit, so none of them writes a core file into the repository (P1-robust-02).
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    if args.resume and not (args.out / 'run.json').is_file():
        print('resume requires run.json in the output directory', file=sys.stderr)
        return 2
    if not args.resume and args.out.exists() and any(args.out.iterdir()):
        raise SystemExit('output directory must be empty')
    try:
        record = Runner(args).run()
    except ResumeError as exc:
        print('resume refused: ' + str(exc), file=sys.stderr)
        return 2
    print(json.dumps({'result': record['result'], 'steps': len(record['steps']), 'totals': record['totals']}))
    return 2 if record['result'] == 'error' else 0


if __name__ == '__main__':
    sys.exit(main())
