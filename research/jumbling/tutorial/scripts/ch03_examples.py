"""Chapter 3 of the jumbling tutorial: the height-phase facts it quotes that no committed record holds (headless research).

Read-only. It runs the committed height rule programs from the repository root on two states of the R22D-36 fixture,
hands each log to the height plugin's own `_call` (the function the runner uses for every rule-program call), and
writes one JSON record. It never runs the runner, the census or any acceptance seed.

Run from the repository root (37 s on a cloud machine for the committed record):
    python3 -I -B research/jumbling/tutorial/scripts/ch03_examples.py --out <new file> --work <new dir>

Sections of the record:
  A ends      real program runs at Z36 (the R22D-36 fixture state) and S2 = Z36 . (12, 14), each mapped by
              rules_height._call with the arguments next_step passes (rules_height.py:546, :563-564):
              A0 r22d81_rule.py at Z36 with the acceptance clock: START, then the step r22d81/H6 (12, 14)
              A1 r22d81_pairs.py at S2 with --target 35, where u = 0: START, then UNLOCKED with 0 steps
              A2 r22d81_pairs.py at S2 with --target 0 and --cpu 0.001: START, then BUDGET with 0 steps
              A3 r22d81_rule.py at Z36 with the first CPU budget of a fixed list that prints START and then BUDGET
                 with 0 steps (which budget does it depends on the machine)
              The child runs to its own end here; the runner would also stop it at its first STEP line, which only
              A0 prints, and _call stops reading at that line in both cases (rules_height.py:226-228).
  B formats   _call on end lines written in each rule program's own print format (the field sets of its report
              and print lines), one case per row of chapter 3's table of program ends
  C r22d75    the rule code of the R22D-75 scripts compared with r22d75_from_snapshot.py, from `def info` to the
              end of the file: the differing lines
  D sha256    the committed cloud-line scripts against the sha256 table of their record RESULT.md
"""
import argparse
import difflib
import hashlib
import importlib.util
import json
import os
import re
import subprocess
import sys
import time
import types
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--out', required=True, type=Path)
parser.add_argument('--work', required=True, type=Path)
parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[4])
args = parser.parse_args()
ROOT = args.root.resolve()
if args.out.exists():
    parser.error('--out must be a new file')
WORK = args.work.resolve()
WORK.mkdir(parents=True, exist_ok=False)
CACHE = WORK / 'cache'
CACHE.mkdir()
ENV = dict(os.environ, OPENBLAS_NUM_THREADS='1', OMP_NUM_THREADS='1', MKL_NUM_THREADS='1',
           PYTHONDONTWRITEBYTECODE='1', JUMBLING_HOME_CACHE=str(CACHE / 'home_centroids.npy'))
os.environ.update(ENV)
T13 = ROOT / 'research/jumbling/restoration/13-tutorial-runner/scripts'
R22 = ROOT / 'research/jumbling/restoration/12-low-height/scripts/round22'
CLOUD = R22 / 'cloud'
spec = importlib.util.spec_from_file_location('tutorial_runner', T13 / 'runner.py')
R = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = R
spec.loader.exec_module(R)         # puts the runner's scripts directory on sys.path (runner.py:35)
import rules_height as H  # noqa: E402
import runner_api as api  # noqa: E402

T0 = time.monotonic()
# The R22D-36 fixture (PROOFS-B.md:2206-2211), as test_rules_height.py:39-45 states it.
Z36 = '8733a0aafd8e24ea1da990ebab0347074c20b11cf61d9ed04d34d25ef6f67261'
S2 = '7cd868f6466953eb6ea1e0d466ed66857d34edaea7dd7c554c1e74975005541b'
WORD36 = [[0, 12], [74, 8], [188, 4], [74, 4], [188, 8], [188, 4], [188, 4],
          [74, 8], [188, 8], [74, 4], [188, 8], [178, 1], [188, 4], [74, 8],
          [188, 4], [74, 4], [188, 8], [188, 8], [188, 4], [74, 8], [188, 8],
          [74, 4], [178, 1], [0, 45], [14, 12], [12, 12]]


def sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


out = {'format': 'tutorial-ch03-examples/1',
       'sources': {p.relative_to(ROOT).as_posix(): sha256(p) for p in
                   [T13 / 'rules_height.py', T13 / 'runner_api.py', T13 / 'check_contracts.py',
                    *[ROOT / s for s in H.RULE_SOURCES.values()]]}}

# ---------------------------------------------------------------------------------------------------- A: real ends
m = R.Mechanics(CACHE)
z36 = m.sim.State(m.restore.ctx, menu=m.menu)
m.apply_word(z36, {'word': WORD36, 'input_digest': R.SOLVED, 'output_digest': Z36})
snap_z36 = m.snapshot(z36)
m.export(z36, WORK / 'z36.json')
s2 = m.load(WORK / 'z36.json')
m.apply_word(s2, {'word': [[12, 14]], 'input_digest': Z36, 'output_digest': S2})
snap_s2 = m.snapshot(s2)
paths = {'Z36': WORK / 'z36.pkl', 'S2': WORK / 's2.pkl'}
H.write_snapshot(paths['Z36'], z36, snap_z36['digest'], snap_z36['off'])
H.write_snapshot(paths['S2'], s2, snap_s2['digest'], snap_s2['off'])
import check_contracts as CC  # noqa: E402  (the checker's own straddler count, independent of the programs)
u_s2 = {cap: int(len(CC.straddlers(s2, cap))) for cap in (0, 35)}
out['states'] = {'Z36': snap_z36, 'S2': snap_s2, 'S2_u': {str(k): v for k, v in u_s2.items()},
                 'setup_seconds': round(time.monotonic() - T0, 1)}
print('states', out['states'], flush=True)


def context(cpu, mode):
    """The part of api.Ctx that _call uses. execute runs the argv _call builds, from the repository root."""
    work = WORK / 'calls'
    work.mkdir(exist_ok=True)

    def execute(argv, tag, seconds, stop_when=None):
        log = work / (tag + '.log')
        began = time.monotonic()
        with log.open('w') as f:
            rc = subprocess.run([str(a) for a in argv], cwd=ROOT, env=ENV, stdout=f,
                                stderr=subprocess.STDOUT).returncode
        return types.SimpleNamespace(rc=rc, seconds=time.monotonic() - began, stopped=False, cancelled=False,
                                     timeout=False, log=log)
    budgets = {'height_cpu': cpu, 'height_seconds': 3600}
    return types.SimpleNamespace(work_dir=work, execute=execute,
                                 child_clock=lambda name: api.child_clock(mode, budgets.get(name)),
                                 wall=lambda name: None if mode == 'acceptance' else budgets.get(name))


def case(label, state, name, cpu, mode, extra=(), expected=None):
    snap = snap_z36 if state == 'Z36' else snap_s2
    ctx = context(cpu, mode)
    call = H._call(ctx, snap, paths[state], name, extra, expected=expected)
    lines = (ctx.work_dir / (call['attempt']['tag'] + '.log')).read_text()
    row = {'label': label, 'state': state, 'program': H.RULE_SOURCES[name].as_posix(), 'mode': mode,
           'cpu_argument': float(api.child_clock(mode, cpu)),
           'extra': [str(x) for x in extra], 'expected': expected,
           'tags': [line.split(' ', 1)[0] for line in lines.splitlines()],
           'lines': [line for line in lines.splitlines() if line.split(' ', 1)[0] in
                     ('START', 'STEP', 'STUCK', 'UNLOCKED', 'BUDGET', 'HANDOFF', 'NOT_APPLICABLE')],
           'result': call['result'], 'reason': call['reason'],
           'attempt': {k: v for k, v in call['attempt'].items() if k not in ('seconds', 'tag')}}
    print(label, row['tags'], '->', row['result'], row['reason'], row['attempt']['outcome'], flush=True)
    return row


ends = [case('A0 r22d81 at Z36, acceptance clock', 'Z36', 'r22d81', None, 'acceptance'),
        case('A1 r22d81_pairs at S2, --target 35 (u = 0)', 'S2', 'r22d81_pairs', None, 'acceptance',
             ('--target', 35), {'tau': 35, 'u': u_s2[35]}),
        case('A2 r22d81_pairs at S2, --target 0, --cpu 0.001', 'S2', 'r22d81_pairs', 0.001, 'development',
             ('--target', 0), {'tau': 0, 'u': u_s2[0]})]
tries = []
for budget in (2.2, 2.0, 2.4, 1.8, 2.6, 1.6, 2.8, 3.0, 1.4, 3.3, 3.6, 4.0, 4.5, 5.0):
    row = case(f'A3 r22d81 at Z36, --cpu {budget}', 'Z36', 'r22d81', budget, 'development')
    tries.append({'cpu': budget, 'tags': row['tags'], 'result': row['result'], 'reason': row['reason']})
    if 'START' in row['tags'] and 'BUDGET' in row['tags'] and 'STEP' not in row['tags']:
        ends.append(row)
        break
out['A_ends'] = {'cases': ends, 'A3_tries': tries}

# ------------------------------------------------------------------------------------------------- B: formats
D = 'a' * 64
COUNTS = {'digest': D, 'n2_n4_n6': [3, 2, 1], 'off': 6, 'N': 1, 'cpu': 0.0}


def line(tag, **kw):
    """A line as report() prints it: the fields given, then digest, counts, off, N and cpu."""
    return tag + ' ' + json.dumps(dict(kw, **COUNTS)) + '\n'


def line75(tag, **kw):
    return tag + ' ' + json.dumps(dict(kw, digest=D, n6_n4_n2=[1, 2, 3], off=6, N=1, cpu=0.0)) + '\n'


def mapped(name, text, expected=None, rc=0):
    work = WORK / 'formats'
    work.mkdir(exist_ok=True)
    log = work / f'{len(list(work.iterdir())):02d}.log'
    log.write_text(text)
    child = types.SimpleNamespace(rc=rc, seconds=1.0, stopped=False, cancelled=False, timeout=False, log=log)
    ctx = types.SimpleNamespace(child_clock=lambda n: 3600.0, work_dir=work, execute=lambda *a, **k: child,
                                wall=lambda n: 3600)
    call = H._call(ctx, {'digest': D}, work / 'x.pkl', name, expected=expected)
    return call['result'], call['reason'], call['attempt']['outcome']


s81 = line('START', prefix=[], tau=274, u=3, targets=[[274, 159]])
sp0 = line('START', prefix=[], u=0, meet_prune=True, tau=274)
sp3 = line('START', prefix=[], u=3, meet_prune=True, tau=274)
s4 = line('START', tau=482, u=80, targets=[[482, 284]])
FORMAT_CASES = [
    ('r22d75: START, STUCK', 'r22d75', 'r22d75_from_snapshot.py:91, :95',
     line75('START') + line75('STUCK', steps=0, exhausted='all single letters', checks={}), None),
    ('r22d75: START, BUDGET', 'r22d75', ':91, :100',
     line75('START') + line75('BUDGET', steps=0, limit_cpu=3600.0), None),
    ('r22d81: START, STUCK with tau and u', 'r22d81', 'r22d81_rule.py:244, :293',
     s81 + line('STUCK', steps=0, tau=274, u=3, checks={}), None),
    ('r22d81: START, STUCK with tau null', 'r22d81', ':244, :293',
     line('START', prefix=[], tau=None, u=0, targets=[]) + line('STUCK', steps=0, tau=None, u=0, checks={}), None),
    ('r22d81: START, BUDGET with 0 steps', 'r22d81', ':244, :310', s81 + line('BUDGET', steps=0, limit_cpu=3600.0),
     None),
    ('r22d81: BUDGET before START', 'r22d81', ':310', line('BUDGET', steps=0, limit_cpu=3600.0), None),
    ('r22d81: START, BUDGET with 1 step and an unreported H6 letter', 'r22d81', ':296, :310',
     s81 + line('BUDGET', steps=1, limit_cpu=3600.0,
                played_unreported={'step': 1, 'family': 'H6', 'letter': [274, 13], 'length': 1, 'input_digest': D}),
     None),
    ('r22d81: NOT_APPLICABLE', 'r22d81', ':240',
     'NOT_APPLICABLE ' + json.dumps({'digest': D, 'max_height': 8}) + '\n', None),
    ('pairs: START with u = 0, UNLOCKED with 0 steps', 'r22d81_pairs', 'r22d81_pairs.py:196, :298',
     sp0 + line('UNLOCKED', steps=0, held6=5, lowered_to_4_by_items_1_to_59=[0] * 59, tau=274),
     {'tau': 274, 'u': 0}),
    ('pairs: START, BUDGET with 0 steps', 'r22d81_pairs', ':196, :300',
     sp3 + line('BUDGET', steps=0, limit_cpu=3600.0, tau=274), {'tau': 274, 'u': 3}),
    ('pairs: START, STUCK with 0 steps', 'r22d81_pairs', ':196, :284-285',
     sp3 + line('STUCK', steps=0, u=3, checks={}, best_pair=None, tau=274), {'tau': 274, 'u': 3}),
    ('pairs: START whose u differs from the STUCK line of r22d81', 'r22d81_pairs', ':196',
     sp3 + line('STUCK', steps=0, u=3, checks={}, best_pair=None, tau=274), {'tau': 274, 'u': 4}),
    ('pairs: START whose digest differs from the state', 'r22d81_pairs', ':196',
     sp3.replace(D, 'b' * 64), {'tau': 274, 'u': 3}),
    ('h4: START with tau null', 'h4', 'h4_unlock.py:259', line('START', tau=None, u=0, targets=[]), None),
    ('h4: START, STUCK with a table', 'h4', ':259, :305',
     s4 + line('STUCK', steps=0, tau=482, u=80, targets=[[482, 284]], families={}, checks={}), None),
    ('h4: START, BUDGET with 0 steps', 'h4', ':259, :320',
     s4 + line('BUDGET', steps=0, tau=482, u=80, families={}, limit_cpu=3600.0), None),
    ('h4: BUDGET before START', 'h4', ':256, :320',
     line('BUDGET', steps=0, tau=None, u=None, families={}, limit_cpu=3600.0), None),
    ('h4: START, BUDGET with 1 step, families {L4: 1}', 'h4', ':259, :320',
     s4 + line('BUDGET', steps=1, tau=482, u=80, families={'L4': 1}, limit_cpu=3600.0), None),
    ('h4: STUCK without START', 'h4', ':305',
     line('STUCK', steps=0, tau=482, u=80, targets=[[482, 284]], families={}, checks={}), None),
    ('r22d81: START whose digest differs from the state', 'r22d81', ':244',
     s81.replace(D, 'b' * 64) + line('STUCK', steps=0, tau=274, u=3, checks={}), None),
    ('r22d81: START, then a Traceback (exit 1)', 'r22d81', '-',
     s81 + 'Traceback (most recent call last):\n', None, 1),
    ('r22d81: START only, exit 0', 'r22d81', '-', s81, None),
    ('r22d81: START, a STEP numbered 2', 'r22d81', ':304-305',
     s81 + line('STEP', step=2, family='U', letter=[124, 2], length=1, input_digest=D, tau=274, u=0,
                targets=None, checks={}), None),
]
out['B_formats'] = []
for label, name, where, text, expected, *rc in FORMAT_CASES:
    result = mapped(name, text, expected, rc[0] if rc else 0)
    out['B_formats'].append({'label': label, 'program_lines': where, 'result': result[0], 'reason': result[1],
                             'outcome': result[2]})
    print(f'{label:62s} -> {result}', flush=True)

# --------------------------------------------------------------------------------------------- C: R22D-75 code
FROM = CLOUD / 'height4-descent-bbef1c8e/r22d75_from_snapshot.py'
OTHERS = [R22 / 'r22d75_height_first.py', R22 / 'r22d75_census.py',
          *sorted(CLOUD.glob('census-k3000-s*/r22d75_census_k3000.py'))]


def rule_code(path):
    text = path.read_text().splitlines()
    first = next(i for i, x in enumerate(text) if x.startswith('def info('))
    return first + 1, text[first:]


start, base = rule_code(FROM)
out['C_r22d75'] = {'reference': FROM.relative_to(ROOT).as_posix(), 'reference_from_line': start,
                   'reference_sha256': sha256(FROM), 'others': []}
for path in OTHERS:
    first, code = rule_code(path)
    diff = [d for d in difflib.unified_diff(code, base, lineterm='', n=0)
            if not d.startswith(('---', '+++'))]
    out['C_r22d75']['others'].append({'file': path.relative_to(ROOT).as_posix(), 'from_line': first,
                                      'sha256': sha256(path), 'differences': diff})
    print(path.name, first, diff, flush=True)

# --------------------------------------------------------------------------------------------- D: sha256 table
RESULT = ROOT / 'research/jumbling/restoration/12-low-height/runs/round22/cloud/height4-descent-bbef1c8e/RESULT.md'
rows = []
for number, text in enumerate(RESULT.read_text().splitlines(), 1):
    hit = re.match(r'\| `cloud/height4-descent-bbef1c8e/([a-z0-9_]+\.py)`.*\| `([0-9a-f]{64})` \|$', text)
    if hit:
        path = CLOUD / 'height4-descent-bbef1c8e' / hit.group(1)
        rows.append({'result_line': number, 'file': path.relative_to(ROOT).as_posix(), 'recorded': hit.group(2),
                     'committed': sha256(path), 'equal': sha256(path) == hit.group(2)})
out['D_sha256'] = rows
print('D', [(r['file'].rsplit('/', 1)[1], r['result_line'], r['equal']) for r in rows], flush=True)

for p in paths.values():
    p.unlink()
out['seconds'] = round(time.monotonic() - T0, 1)
args.out.write_text(json.dumps(out, indent=1) + '\n')
print('wrote', args.out.name, out['seconds'], 's', flush=True)
