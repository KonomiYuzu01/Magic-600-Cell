"""Chapter 5 of the jumbling tutorial: what the tail rules give at the state with 71 off pieces (headless research).

Read-only. It drives the tutorial runner's own methods (runner.py: Runner.units, Runner.script, Runner.require,
Runner.close, Runner.t1prime, Runner.execute) on E2's table after its b15 step (stage 12, literal run 2b,
z-04-b15.json.gz), in development mode with the default options, and writes one JSON record. Every child program
runs through the runner's --_script wrapper with the arguments the runner passes. Every file goes under --work and
--cache; nothing is written under the repository.

Run from the repository root (about five minutes on a four-core cloud machine; the slow parts run concurrently):
    python3 -I -B research/jumbling/tutorial/scripts/ch05_examples.py --out <new file> --work <new dir> \
        --cache <new dir>

Sections of the record:
  units       Runner.units at the state: digest, off count, blocks (index, size, movers, least piece, frames) and the
              units in unit order, each compared with E2's unit file of the same pieces
  census      transport_census.py for the first unit, stopped after its class line (the per-unit rows that zero_bad and
              the second core of close read are not computed here; the full census takes much longer)
  pure        cellpure5.py --search for each unit, as the shared search 'pure' runs it: its branch, counts, exact tries
              and word (length and sha256), and whether the word equals E2's word for the same unit
  close_t1prime
              close: Runner.close for the first unit with core P = [], Q = E2's 432-letter word (equal to the pure word
              when pure.<first unit>.word_equals_e2 is true): the attempt row the runner records, and tclose2.py's
              premises
              t1prime: Runner.t1prime for the first unit with W = the same word: the landing check, Z1, and the
              attempt row
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
import sys
import time
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--out', required=True, type=Path)
parser.add_argument('--work', required=True, type=Path)
parser.add_argument('--cache', required=True, type=Path)
parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[4])
args = parser.parse_args()
ROOT = args.root.resolve()
WORK, CACHE = args.work.resolve(), args.cache.resolve()
for d in (WORK, CACHE):
    if d.exists() and any(d.iterdir()):
        raise SystemExit(f'{d.name}: the directory must be new or empty')
    d.mkdir(parents=True, exist_ok=True)
if args.out.exists():
    raise SystemExit('the output file must be new')
# Before any import: seams.py reads this variable once, else it caches under the repository.
os.environ['JUMBLING_HOME_CACHE'] = str(CACHE / 'home_centroids.npy')
for name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ[name] = '1'
RUNNER = ROOT / 'research/jumbling/restoration/13-tutorial-runner/scripts'
sys.path.insert(0, str(RUNNER))
import runner as R  # noqa: E402

LINE_A = ROOT / 'research/jumbling/restoration/12-low-height/runs/round22/line-a'
TABLE = LINE_A / 'literal/run2b/z-04-b15.json.gz'
E2_WORD = LINE_A / 'literal/run2b/05-b05-cp4-word.json'
E2_UNITS = {'b05': LINE_A / 'units/eb5-k40-k00-b05.json', 'b06': LINE_A / 'units/eb5-k40-k00-b06.json'}
PROGRAMS = ['research/jumbling/restoration/13-tutorial-runner/scripts/runner.py',
            'research/jumbling/restoration/13-tutorial-runner/scripts/runner_api.py',
            'research/jumbling/restoration/09-general-method/scripts/gm/mkblocks2.py',
            'research/jumbling/restoration/09-general-method/scripts/gm/tclose2.py',
            'research/jumbling/restoration/12-low-height/scripts/cellpure5.py',
            'research/jumbling/restoration/12-low-height/scripts/round22/transport_census.py',
            'research/jumbling/restoration/12-low-height/scripts/round22/stepchk.py',
            'research/jumbling/restoration/12-low-height/scripts/round22/prep_t1prime.py',
            'research/jumbling/theory/e3_class.py', 'research/jumbling/theory/buffer.py']
T0 = time.time()


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def rows(log):
    return [json.loads(line) for line in Path(log).read_text().splitlines() if line.startswith('{')]


def short(value, keep=('word',)):
    """A record copy without long lists: a word becomes its length and sha256."""
    if isinstance(value, dict):
        out = {}
        for k, v in value.items():
            if k in keep and isinstance(v, list):
                out[k + '_letters'], out[k + '_sha256'] = len(v), R.word_sha256(v)
            else:
                out[k] = short(v, keep)
        return out
    if isinstance(value, list):
        return [short(v, keep) for v in value]
    return value


mechanics = R.Mechanics(CACHE)
start = WORK / 'input.json'
R.atomic_json(start, R.read_json(TABLE))
rargs = R.parse_args(['--table', str(start), '--out', str(WORK / 'run'), '--cache', str(CACHE)])
runner = R.Runner(rargs, mechanics)
runner.initialize()
table = runner.table
record = {'format': 'tutorial-ch05-examples/1',
          'source': R.source_identity(),
          'programs': {p: sha(ROOT / p) for p in PROGRAMS},
          'input': {'table': TABLE.relative_to(ROOT).as_posix(), 'sha256': sha(TABLE),
                    'digest': runner.state.digest(), 'off': int(runner.state.off_lattice_count())},
          'options': {'mode': rargs.mode, 'tail_rules': rargs.tail_rules, 'greedy_seconds': rargs.greedy_seconds,
                      'rule_seconds': rargs.rule_seconds, 'literal_limit': rargs.literal_limit, 'seeds': rargs.seeds}}

# A. units
t = time.time()
work = runner.out / 'attempts' / '00000'
units = runner.units(table, work / 'units')
census_rows = rows(next((runner.out / 'logs').glob('*-units.log')))
names = {}
for u in units:
    pieces = R.read_json(u.target)['pieces']
    e2 = next((n for n, f in E2_UNITS.items() if sorted(R.read_json(f)['pieces']) == sorted(pieces)), None)
    names[u.key] = e2
record['units'] = {'seconds': round(time.time() - t, 1), 'digest': census_rows[-1]['digest'],
                   'off': census_rows[-1]['off'],
                   'blocks': [dict(zip(('block', 'size', 'movers', 'least', 'frames'), b))
                              for b in census_rows[-1]['blocks']],
                   'order': [{'key': u.key, 'size': u.size, 'file': u.target.name, 'free_file': None if u.free is None
                              else u.free.name, 'e2_unit': names[u.key]} for u in units]}


def census_class_line(u):
    """transport_census.py as the shared search 'transport' runs it, stopped after its class line."""
    w = work / f'u{u.key}'
    w.mkdir(parents=True, exist_ok=True)
    command = [sys.executable, '-I', '-B', R.SELF, '--_script', R.ROOT / R.LOW / 'round22/transport_census.py',
               '--cache', runner.cache, '--'] + [table, w / 'transport.json', u.target]
    child = runner.execute(command, f'census-u{u.key}', None,
                           stop_when=lambda line: line.startswith('{') and '"home_cells"' in line)
    lines = rows(child.log)
    cls = [r for r in lines if 'home_cells' in r]
    return {'unit_key': u.key, 'stopped_after_class_line': child.stopped, 'seconds': round(child.seconds, 1),
            'class_line': cls[-1] if cls else None, 'first_line': lines[0] if lines else None}


def pure(u):
    """cellpure5.py --search as the shared search 'pure' runs it (Runner.try_unit, dependency)."""
    w = work / f'u{u.key}'
    w.mkdir(parents=True, exist_ok=True)
    out = w / 'pure.json'
    child = runner.script(R.LOW / 'cellpure5.py', [R.TH, table, u.target, out, '--search'], f'pure-u{u.key}')
    runner.require(child, screens=R.SCREENS)
    doc = R.read_json(out)
    wf = out.with_name(out.stem + '-word.json')
    res = {'unit_key': u.key, 'seconds': round(child.seconds, 1), 'rc': child.rc, 'word_file': wf.exists()}
    for key in ('off_before', 'class', 'free_grips', 'blocked', 'own_contents', 'reachable_cells', 'generators',
                'carrier_cell', 'holes', 'chambers_of_X', 'X0_in_carrier_orbit', 'pair_tools', 'cell_pairs',
                'edge_cells', 'X_in_nodes', 'cycles_found', 'model_check_pass', 'exact_tries', 'cycle', 'multiples',
                'target_exponent', 'word_len', 'exact', 'transport'):
        if key in doc:
            res[key] = doc[key]
    if wf.exists():
        word = R.read_json(wf)
        res['word'] = {'letters': len(word['word']), 'sha256': R.word_sha256(word['word']),
                       'input_digest': word['input_digest'], 'output_digest': word['output_digest']}
        e2 = R.read_json(E2_WORD)
        res['word_equals_e2'] = word['word'] == e2['word']
    return res


def attempt(fn):
    """One rule call; a RuleFailure becomes the attempt row the runner records (Runner.try_unit, failed)."""
    started = time.time()
    try:
        candidate = fn()
        return {'candidate': True, 'gate': candidate.gate, 'letters': len(candidate.word['word']),
                'seconds': round(time.time() - started, 1)}
    except R.RuleFailure as exc:
        row = R.attempt_row(exc, seconds=round(time.time() - started, 1))
        return {'candidate': False, 'attempt': short(row)}


def close_and_t1prime(u):
    """Runner.close and Runner.t1prime with E2's word for this unit (the pure word when they are equal)."""
    w = work / f'u{u.key}'
    w.mkdir(parents=True, exist_ok=True)
    word = R.read_json(E2_WORD)
    res = {'unit_key': u.key, 'core': {'P': 0, 'Q_letters': len(word['word']), 'Q_sha256': R.word_sha256(word['word'])}}
    res['close'] = attempt(lambda: runner.close(table, u, {'P': [], 'Q': word['word']}, w / 'close-pure'))
    pre = R.read_json(w / 'close-pure' / 'closure' / 'pre.json')
    res['close']['pre'] = {'ghost': {k: v for k, v in pre['ghost'].items() if not isinstance(v, list)},
                           'menu': pre['menu'],
                           'vacancy': {'N1': pre['vacancy']['N1'], 'N2': pre['vacancy']['N2'],
                                       'fixed_occupied': len(pre['vacancy']['fixed_occupied']),
                                       'deficits': pre['vacancy']['deficits']},
                           'greedy_ran': 'greedy' in pre}
    res['close']['result'] = R.read_json(w / 'close-pure' / 'closure' / 'result.json').get('result')
    res['t1prime'] = attempt(lambda: runner.t1prime(table, u, E2_WORD, w / 't1prime'))
    landing = w / 't1prime' / 'landing' / 'check.json'
    if landing.exists():
        res['t1prime']['landing'] = R.read_json(landing)
    z1 = w / 't1prime' / 't1-z1-table.json'
    if z1.exists():
        q = R.read_json(w / 't1prime' / 't1-Q.json')
        res['t1prime']['z1'] = {'digest': R.read_json(z1)['digest'], 'Q_pieces': len(q['pieces'])}
    pre = w / 't1prime' / 'inverse' / 'closure' / 'pre.json'
    if pre.exists():
        pre = R.read_json(pre)
        res['t1prime']['pre'] = {'ghost': {k: v for k, v in pre['ghost'].items() if not isinstance(v, list)},
                                 'menu': pre.get('menu'),
                                 'vacancy': None if 'vacancy' not in pre else {
                                     'N1': pre['vacancy']['N1'], 'N2': pre['vacancy']['N2'],
                                     'fixed_occupied': len(pre['vacancy']['fixed_occupied']),
                                     'deficits': pre['vacancy']['deficits']},
                                 'greedy_ran': 'greedy' in pre}
        res['t1prime']['result'] = R.read_json(w / 't1prime' / 'inverse' / 'closure' / 'result.json').get('result')
    return res


first = units[0]
with ThreadPoolExecutor(max_workers=4) as pool:
    jobs = {'census': pool.submit(census_class_line, first),
            'pure': [pool.submit(pure, u) for u in units],
            'close': pool.submit(close_and_t1prime, first)}
    record['census'] = jobs['census'].result()
    record['pure'] = {str(u.key): f.result() for u, f in zip(units, jobs['pure'])}
    record['close_t1prime'] = jobs['close'].result()
record['seconds'] = round(time.time() - T0, 1)
R.atomic_json(args.out, record)
print(json.dumps({'out': args.out.name, 'seconds': record['seconds']}))
