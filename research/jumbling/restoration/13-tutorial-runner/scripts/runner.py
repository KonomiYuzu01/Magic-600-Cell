"""State-only, literal-word tutorial runner (headless research, runner-run/2).

Run from the repository root with Python -I -B. For example:
  python3 -I -B research/jumbling/restoration/13-tutorial-runner/scripts/runner.py \
      --scramble 10 7001 --out /tmp/tutorial-run --cache /tmp/tutorial-cache

The output directory must be empty. Existing research programs own the searches;
this module only orders them, checks their literal words and checkpoints progress.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass, field
import gzip
import hashlib
import json
import math
import os
from pathlib import Path
import runpy
import shutil
import signal
import subprocess
import sys
import threading
import time
import traceback

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[5]
SELF = Path(__file__).resolve()
J = Path('research/jumbling')
TH = J / 'theory'
GM = J / 'restoration/09-general-method/scripts/gm'
EC = GM.parent / 'echain'
LOW = J / 'restoration/12-low-height/scripts'
LAT = GM.parent / 'lattice/scripts'
CHECKER = GM.parent / 'lattice/checker/verify_late.py'
LIBRARY = J / 'restoration/09-general-method/data/library-merged.json.gz'
SOLVED = hashlib.sha256(b'').hexdigest()
BULK_ORDER = ['seam', 'cover', 'close', 'conj', 'lock', 'improve', 'shift', 'reopen']
TAIL_RULES = ['zero_bad', 'pure', 'close', 't1prime']
SCRIPTS = [SELF.relative_to(ROOT), TH / 'seams.py', TH / 'restore.py',
           TH / 'orient.py', TH / 'buffer.py', TH / 'trees.py',
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


def source_identity():
    result = {'commit': None, 'status_sha256': None, 'dirty': None}
    for key, argv in [('commit', ['git', 'rev-parse', 'HEAD']),
                      ('status', ['git', '--no-optional-locks', 'status', '--porcelain=v1', '--untracked-files=all', '--', 'research/jumbling'])]:
        try:
            output = subprocess.check_output(argv, cwd=ROOT, stderr=subprocess.DEVNULL)
        except (OSError, subprocess.CalledProcessError):
            continue
        if key == 'commit':
            result['commit'] = output.decode('ascii').strip()
        else:
            result.update(status_sha256=hashlib.sha256(output).hexdigest(), dirty=bool(output))
    return result


def premise_details(details):
    return {key: details[key] for key in ('N1', 'N2', 'deficits', 'T_size', 'T_digest', 'V_length')
            if key in details}


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


class Mechanics:
    """Lazy imports keep record and scheduling tests independent of geometry."""
    def __init__(self, cache):
        sys.path[:0] = [str(ROOT / TH), str(ROOT / J), str(ROOT / J / 'fixtures')]
        import numpy as np
        import restore
        import seams
        import sim
        import orient
        import buffer
        import trees
        seams.CACHE = Path(cache) / 'home_centroids.npy'
        self.np, self.restore, self.seams, self.sim = np, restore, seams, sim
        self.orient, self.buffer, self.trees = orient, buffer, trees
        self.menu = restore.menu('I_a')

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


class RuleFailure(Exception):
    def __init__(self, outcome, reason, **details):
        super().__init__(reason)
        self.outcome, self.reason, self.details = outcome, safe_reason(reason), details


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
    def __init__(self, args, mechanics=None):
        self.args = args
        self.out = Path(args.out).resolve()
        if '.json' in str(self.out):
            raise ValueError('out path must not contain .json')
        self.cache = Path(args.cache).resolve() if args.cache else self.out / 'cache'
        for name in ('words', 'units', 'tables', 'attempts', 'logs'):
            (self.out / name).mkdir(parents=True, exist_ok=True)
        self.m = mechanics
        self.cancel = threading.Event()
        self.signal_number = None
        self.started = time.monotonic()
        self.record = None
        self.state = self.table = None

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
        for phase in ('bulk', 'tail', 'fallback'):
            totals[phase + '_twists'] = sum(s['letters'] for s in self.record['steps'] if s['phase'] == phase)
        totals['seconds'] = round(time.monotonic() - self.started, 6)
        atomic_json(self.out / 'run.json', self.record)

    def initialize(self):
        source_id = source_identity()           # before the run writes anything under research/jumbling
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
        budgets = {name: getattr(self.args, name) for name in
                   ('bulk_steps', 'bulk_step_seconds', 'bulk_seconds', 'tail_steps',
                    'rule_seconds', 'greedy_seconds', 'cert_seconds', 'fallback_max_length', 'par')}
        for name in ('bulk_seconds', 'tail_steps'):
            budgets[name] = budgets[name] or None
        self.record = {'format': 'runner-run/2', 'menu': 'I_a',
            'start': dict(source, **self.m.snapshot(self.state)),
            'config': {'literal_only': True, 'bulk_order': BULK_ORDER.copy(),
                'tail_rules': self.args.tail_rules, 'budgets': budgets,
                'literal_limit': self.args.literal_limit, 'seeds': self.args.seeds,
                'source': source_id,
                'scripts': {p.as_posix(): hashlib.sha256((ROOT / p).read_bytes()).hexdigest() for p in SCRIPTS}},
            'phases': {'bulk': {'result': 'budget', 'steps': 0, 'twists': 0,
                'end': dict(self.m.snapshot(self.state), table=self.rel(self.table))},
                'tail': {'result': 'budget', 'steps': 0}},
            'steps': [], 'stuck': None, 'lattice': None, 'result': 'budget', 'error': None,
            'totals': {'bulk_twists': 0, 'tail_twists': 0, 'fallback_twists': 0,
                       'lattice_twists': None, 'seconds': 0.0}}
        self.save()

    def execute(self, argv, tag, seconds, *, cwd=None):
        """Each subprocess owns a process group; timeout also kills descendants."""
        log = self.out / 'logs' / (tag.replace('/', '-') + '.log')
        start = time.monotonic()
        if self.cancel.is_set() or self.signal_number is not None:
            return Child(-1, 0.0, log, cancelled=True)
        env = self.child_env()
        timed_out = cancelled = False
        with log.open('w') as output:
            proc = subprocess.Popen([str(x) for x in argv], cwd=cwd or ROOT, env=env,
                stdout=output, stderr=subprocess.STDOUT, start_new_session=True)
            try:
                while proc.poll() is None:
                    elapsed = time.monotonic() - start
                    cancelled = self.cancel.is_set() or self.signal_number is not None
                    timed_out = seconds is not None and elapsed >= seconds
                    if cancelled or timed_out:
                        kill_child(proc)
                        break
                    try:
                        proc.wait(timeout=0.05)
                    except subprocess.TimeoutExpired:
                        pass
            finally:
                kill_child(proc)
        return Child(proc.returncode, time.monotonic() - start, log, timed_out, cancelled)

    def script(self, script, argv, tag, *, unlimited=False, cwd=None):
        # The adapter redirects seams' cache before executing *any* existing rule.
        command = [sys.executable, '-I', '-B', SELF, '--_script', ROOT / script,
                   '--cache', self.cache, '--'] + list(argv)
        return self.execute(command, tag, None if unlimited else self.args.rule_seconds, cwd=cwd)

    def require(self, child, *, returncodes=(0,)):
        self.check_signal()
        if child.timeout:
            raise RuleFailure('timeout', 'rule subprocess exceeded rule_seconds')
        if child.cancelled:
            raise RuleFailure('cancelled', 'a preceding unit supplied the step')
        if child.rc not in returncodes:
            raise RuleFailure('not applicable', f'child exit {child.rc}; see child log')

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
                        if ((self.args.bulk_seconds and now - start >= self.args.bulk_seconds) or
                                (step_start is not None and now - step_start >= self.args.bulk_step_seconds)):
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
        self.check_signal()
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
            raise RuleFailure('no word', 'stepchk wrote no end table')
        check = read_json(report)
        if (check['start_digest'] != before or check.get('output_digest_matches') is False or
                read_json(end)['digest'] != check['end_digest']):
            raise RuleFailure('digest mismatch', 'stepchk digest mismatch')
        if strict and not check['strict_prime']:
            raise RuleFailure('not strict_prime', 'whole literal word fails strict_prime')
        return Candidate(rule, literal(word, before, check['end_digest']), check, end, parts)

    def closure(self, table, unit, core, work, *, prefix_length=0):
        work.mkdir(parents=True, exist_ok=True)
        core_file = work / 'core.json'
        atomic_json(core_file, core)
        out = work / 'closure'
        argv = [TH, EC, table, unit.target, core_file, out,
                '--greedy-seconds', self.args.greedy_seconds]
        child = self.script(GM / 'tclose2.py', argv, self.rel(work) + '-close')
        try:
            pre = read_json(out / 'pre.json') if (out / 'pre.json').exists() else {}
        except (OSError, ValueError):
            # A timeout may interrupt the existing script's non-atomic write.
            # Its process outcome remains the failed attempt's primary evidence.
            pre = {}
        pre = pre if isinstance(pre, dict) else {}
        vacancy = pre.get('vacancy', {})
        vacancy = vacancy if isinstance(vacancy, dict) else {}
        details = {key: vacancy[key] for key in ('N1', 'N2', 'deficits') if key in vacancy}
        try:
            self.require(child)
            result = read_json(out / 'result.json') if (out / 'result.json').exists() else {}
            result = result if isinstance(result, dict) else {}
            middle = literal({'word': core['P'] + core['Q'] + self.m.inverse_word(core['P'])})['word']
            if (out / 'word-full.json').exists():
                word = literal(read_json(out / 'word-full.json'), read_json(table)['digest'])
                n, remainder = divmod(len(word['word']) - len(middle), 2)
                v = word['word'][:n]
                if (n < 0 or remainder or word['word'][n:n + len(middle)] != middle or
                        word['word'][n + len(middle):] != self.m.inverse_word(v)):
                    raise RuleFailure('contract failed', 'tclose2 word is not V middle V_inverse')
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
            if exc.outcome == 'not applicable':
                exc.outcome = 'no literal closure'
            exc.details.update(premise_details(details))
            raise
        except (OSError, EOFError, ValueError, KeyError, TypeError) as exc:
            raise RuleFailure('no literal closure', 'invalid closure evidence', **premise_details(details)) from exc

    def certificate_word(self, table, middle, ghost, work, details, prefix_length):
        """Build V at this table; a certificate never crosses the literal boundary."""
        target = ghost['badW']
        if (not isinstance(target, list) or any(type(c) is not int for c in target) or
                target != sorted(set(target)) or word_sha256(target) != ghost['badW_digest']):
            raise RuleFailure('digest mismatch', 'Bad(W) digest mismatch')
        digest = read_json(table)['digest']
        details.update(T_size=len(target), T_digest=ghost['badW_digest'])
        tf = work / 'target.json'
        atomic_json(tf, {'T': target, 'T_digest': details['T_digest'], 'state_digest': digest})
        child = self.script(LOW / 'round22/jordan_target.py', [tf, table], self.rel(work) + '-jordan')
        # Its CPU cap can cut off the later Jordan search after CONDITION is
        # complete. The certificate builder/checker supplies the actual witness.
        self.require(child, returncodes=(0, 124))
        conditions = [json.loads(line[len('CONDITION '):]) for line in child.log.read_text().splitlines()
                      if line.startswith('CONDITION ')]
        if not conditions:
            raise RuleFailure('no literal closure', 'no target condition')
        condition = conditions[-1]
        details.update(N1=condition['fixed_vacant'], N2=condition['necessary'])
        if not (details['N1'] is True and details['N2'] is True):
            details['deficits'] = condition.get('necessary_deficits', {})
            raise RuleFailure('no literal closure', 'N1 or N2 fails')
        cert = work / 'certificate.json.gz'
        child = self.script(LOW / 'r22b10_cert.py', ['table', table, tf, cert,
            '--seconds', self.args.cert_seconds, '--seeds', self.args.seeds], self.rel(work) + '-cert')
        self.require(child)
        evidence = read_json(cert)
        # Both r22b10/1 and r22b10/2 share these three fields.
        if evidence.get('complete') is not True:
            raise RuleFailure('no literal closure', 'incomplete certificate within search budget')
        if evidence.get('state_digest') != digest:
            raise RuleFailure('digest mismatch', 'certificate state digest mismatch')
        n = evidence['V_length']
        if type(n) is not int or n < 0:
            raise RuleFailure('no literal closure', 'invalid certificate length')
        child = self.script(LOW / 'check_r22b10.py', ['table', table, tf, cert], self.rel(work) + '-cert-check')
        self.require(child)
        tokens = child.log.read_text().split()
        if not tokens or tokens[-1] != 'PASS':
            raise RuleFailure('no literal closure', 'certificate check did not end in PASS')
        if prefix_length + 2 * n + len(middle) > self.args.literal_limit:
            raise RuleFailure('over literal limit', 'whole step exceeds literal_limit', V_length=str(n))
        vf = work / 'V.json.gz'
        child = self.script(LOW / 'expand_slp.py', [cert, vf, '--limit', self.args.literal_limit],
                            self.rel(work) + '-expand')
        if child.rc == 3 and not child.timeout and not child.cancelled:
            self.check_signal()
            rows = json_rows(child.log)
            if rows and rows[-1].get('over_limit') is True:
                raise RuleFailure('over literal limit', 'expansion exceeds literal_limit', V_length=str(n))
        self.require(child)
        rows = json_rows(child.log)
        expansion = rows[-1] if rows else {}
        v = literal(read_json(vf), digest)['word']
        if expansion.get('letters') != n or len(v) != n or expansion.get('word_sha256') != word_sha256(v):
            raise RuleFailure('no literal closure', 'literal expansion length or hash mismatch')
        details.update(V_source='certificate', parts=[['V', n], ['middle', len(middle)], ['V_inverse', n]],
                       V_letters=n, V_sha256=word_sha256(v))
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
            raise RuleFailure('contract failed', 'literal parts do not cover the word')
        program, end = work / 'program.json', work / 'end-table.json'
        atomic_json(program, {'segments': segments})
        child = self.script(LOW / 'slp_apply.py', ['table', table, program, end, '--unit', unit.target], self.rel(work))
        try:
            self.require(child)
        except RuleFailure as exc:
            if exc.outcome == 'not applicable':
                exc.outcome = 'no literal closure'
            raise
        try:
            rows = json_rows(child.log)
            check = rows[-1] if rows else {}
            if check.get('strict_prime') is not True:
                raise RuleFailure('not strict_prime', 'literal program fails strict_prime')
            if check.get('start_digest') != before or read_json(end)['digest'] != check.get('end_digest'):
                raise RuleFailure('digest mismatch', 'slp_apply digest mismatch')
            part_rows = [r for r in rows if 'segment' in r]
            if len(part_rows) != len(expected):
                raise RuleFailure('digest mismatch', 'slp_apply segment count mismatch')
            for index, (row, (kind, n, sha)) in enumerate(zip(part_rows, expected)):
                if (row.get('segment') != index or row.get('kind') != kind or row.get('length') != n or
                        not isinstance(row.get('digest'), str) or len(row['digest']) != 64 or
                        any(c not in '0123456789abcdef' for c in row['digest']) or
                        (kind == 'safe_word_file' and row.get('word_sha256') != sha)):
                    raise RuleFailure('digest mismatch', 'slp_apply literal segment mismatch')
            if prefix_digest is not None and part_rows[0]['digest'] != prefix_digest:
                raise RuleFailure('digest mismatch', 'slp_apply W end digest mismatch')
            if part_rows[-1]['digest'] != check['end_digest']:
                raise RuleFailure('digest mismatch', 'slp_apply final segment digest mismatch')
            return Candidate(rule, literal(word, before, check['end_digest']), check, end, parts, 'slp_apply')
        except (OSError, ValueError, KeyError, TypeError) as exc:
            raise RuleFailure('no literal closure', 'invalid slp_apply evidence') from exc

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
            raise RuleFailure('no literal closure', 'T1prime preparation produced no inputs')
        q = Unit(unit.key, len(read_json(target)['pieces']), target)
        core = read_json(inverse)
        if (read_json(z1)['digest'] != landing.check['end_digest'] or
                core != {'P': [], 'Q': self.m.inverse_word(landing.word['word'])}):
            raise RuleFailure('contract failed', 'T1prime inverse or prepared state mismatch')
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
                        self.require(child)
                        wf = out.with_name(out.stem + '-word.json')
                        if not wf.exists():
                            raise RuleFailure('no word', 'pure search found no word')
                        cached[name] = wf
                    else:
                        child = self.script(LOW / 'round22/transport_census.py', [table, out, unit.target],
                                            self.rel(work) + '-transport')
                        self.require(child)
                        rows = read_json(out)['units']
                        cached[name] = rows[unit.target.stem]
                except RuleFailure as exc:
                    cached[name] = exc
            if isinstance(cached[name], RuleFailure):
                raise cached[name]
            return cached[name]

        def failed(rule, start, exc, source=None):
            attempt = {'unit_key': unit.key, 'rule': rule, 'outcome': exc.outcome,
                       'reason': exc.reason, 'seconds': round(time.monotonic() - start, 6), **exc.details}
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
        chosen_unit = candidate = None
        retry = []
        try:
            for unit, future in zip(units, futures):
                trial = future.result()
                attempts.extend(trial.attempts)
                by_unit.append({'unit_key': unit.key, 'size': unit.size, 'attempts': trial.attempts.copy()})
                if trial.candidate is not None:
                    chosen_unit, candidate = unit, trial.candidate
                    break
                if self.args.par > 1 and any(a['outcome'] == 'timeout' for a in trial.attempts):
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
        return chosen_unit, candidate, attempts, by_unit

    def fallback(self, unit, work):
        out = work / 'fallback'
        child = self.script(GM / 'fallback.py', [TH, EC, self.table, unit.target, out,
            '--max-length', self.args.fallback_max_length], self.rel(work) + '-fallback')
        if child.rc == 3 and not child.timeout:
            raise RuleFailure('bound exhausted', 'no word within fallback_max_length')
        self.require(child)
        if not (out / 'word-full.json').exists():
            raise RuleFailure('no word', 'fallback wrote no literal word')
        candidate = self.checked(self.table, unit, read_json(out / 'word-full.json'),
                                 work / 'fallback-check', 'fallback', strict=False)
        if (candidate.check['collateral'] != 0 or
                candidate.check['unit_on_lattice'] != candidate.check['unit_size']):
            raise RuleFailure('contract failed', 'fallback does not land the unit without collateral')
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
        if candidate.gate == 'slp_apply':
            self.state = self.m.load(candidate.end_table)
            if self.state.digest() != word['output_digest']:
                raise ValueError('slp_apply end digest mismatch on load')
        else:
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
        shutil.copyfile(candidate.end_table, self.table)
        row = {'index': index, 'phase': phase, 'rule': candidate.rule,
            'unit': self.rel(unit_file), 'unit_key': unit.key, 'unit_size': unit.size,
            'word': self.rel(word_file), 'letters': len(word['word']),
            'contract': 'fallback' if phase == 'fallback' else 'strict_prime',
            'gate': candidate.gate,
            'before': before, 'after': after, 'landed': unit.size,
            'others_landed': candidate.check['others_landed'], 'attempts': attempts,
            'seconds': round(seconds + time.monotonic() - replay_started, 6)}
        if candidate.parts is not None:
            row['parts'] = candidate.parts
        row.update(candidate.metadata)
        self.record['steps'].append(row)
        self.record['phases']['tail']['steps'] += 1
        self.save()

    def tail(self):
        phase = self.record['phases']['tail']
        while self.state.off_lattice_count():
            if self.args.tail_steps and phase['steps'] >= self.args.tail_steps:
                phase['result'] = self.record['result'] = 'budget'
                self.save()
                return
            started = time.monotonic()
            work = self.out / 'attempts' / f'{len(self.record["steps"]):05d}'
            units = self.units(self.table, work / 'units')
            unit, candidate, attempts, by_unit = self.choose(units, work)
            if candidate is None and units and self.args.fallback_max_length:
                unit = units[0]
                start = time.monotonic()
                try:
                    candidate = self.fallback(unit, work)
                except RuleFailure as exc:
                    attempt = {'unit_key': unit.key, 'rule': 'fallback', 'outcome': exc.outcome,
                        'reason': exc.reason, 'seconds': round(time.monotonic() - start, 6), **exc.details}
                    by_unit[0]['attempts'].append(attempt)
            if candidate is None:
                self.record['stuck'] = dict(self.m.snapshot(self.state), units=by_unit)
                phase['result'] = self.record['result'] = 'stuck'
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
        self.save()

    def run(self):
        previous = {}
        for number in (signal.SIGTERM, signal.SIGHUP):
            previous[number] = signal.signal(number, self.on_signal)
        try:
            self.initialize()
            self.check_signal()
            self.bulk()
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
        return self.record


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
    restorer = runner.m.seams.SeamRestorer(state, runner.m.menu)
    assert list(restorer.order) == BULK_ORDER
    methods = {'seam': restorer.seam, 'cover': restorer.cover, 'close': restorer.level1,
               'lock': restorer.level2, 'improve': restorer.improve, 'shift': restorer.shift,
               'conj': restorer.conj, 'reopen': restorer.reopen}
    phase = record['phases']['bulk']
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
        record['steps'].append({'index': index, 'phase': 'bulk', 'rule': rule, 'word': runner.rel(path),
            'letters': len(word), 'before': before, 'after': after,
            'seconds': round(time.monotonic() - started, 6)})
        phase.update(steps=phase['steps'] + 1, twists=phase['twists'] + len(word),
                     end=dict(runner.m.snapshot(state), table=runner.rel(runner.table)))
        runner.save()
        atomic_json(out / 'bulk-progress.json', {'step_started': None})
    atomic_json(out / 'bulk-progress.json', {'step_started': None})
    runner.save()


def parse_args(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument('--scramble', nargs=2, type=int, metavar=('K', 'SEED'))
    source.add_argument('--table', type=Path)
    parser.add_argument('--out', required=True, type=Path)
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
    args = parser.parse_args(argv)
    if '.json' in str(args.out.resolve()):
        parser.error('out path must not contain .json')
    args.tail_rules = args.tail_rules.split(',') if args.tail_rules else []
    if len(set(args.tail_rules)) != len(args.tail_rules) or any(r not in TAIL_RULES for r in args.tail_rules):
        parser.error('tail-rules must be distinct names from ' + ','.join(TAIL_RULES))
    for name in ('bulk_steps', 'bulk_seconds', 'tail_steps', 'greedy_seconds', 'fallback_max_length', 'literal_limit'):
        value = getattr(args, name)
        if value is not None and (value < 0 or (isinstance(value, float) and not math.isfinite(value))):
            parser.error(name.replace('_', '-') + ' must be finite and nonnegative')
    for name in ('bulk_step_seconds', 'rule_seconds', 'cert_seconds', 'par'):
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
    if args.out.exists() and any(args.out.iterdir()):
        raise SystemExit('output directory must be empty')
    record = Runner(args).run()
    print(json.dumps({'result': record['result'], 'steps': len(record['steps']), 'totals': record['totals']}))
    return 2 if record['result'] == 'error' else 0


if __name__ == '__main__':
    sys.exit(main())
