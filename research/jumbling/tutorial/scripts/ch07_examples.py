"""Chapter 7 of the jumbling tutorial: the record and checker examples it quotes (headless research).

Read-only on the repository. It copies the committed D0 record of seed 7001 (stage 13, runs/dev/d0-s7001/run) into
--work, checks the copy and four changed copies with the committed checker, runs the committed runner twice in
development mode (seed 7001 at depth 10, and the R22D-36 fixture state Z36 with no bulk step and no tail rule),
checks both records, and computes three tail-step trees with the runner's own tree helper. Every file goes under
--work and --cache; nothing is written under the repository. It never runs a seed of the acceptance test set.

Run from the repository root (about two minutes on a four-core cloud machine):
    python3 -I -B research/jumbling/tutorial/scripts/ch07_examples.py --out <new file> --work <new dir> \
        --cache <new dir>

Sections of the record:
  A d0         the committed record: its provenance and configuration, its scripts map against the runner's and the
               plugins' lists, every step's word and tree against the row's hashes, the committed format 1 record of
               the same scramble, and the checker's rows on an unchanged copy against the committed check.jsonl
  B tampered   four changed copies of that record and the checker's rows on each:
               a  the item of step 3's letter raised by one in its word file
               b  the same change, with step 3's word_sha256 updated to match
               c  step 2's tree span moved from [0, 1] to [1, 2], with the tree's sha256 updated
               d  config.lattice.restorer and the recorded acceptance options set to late, and the recorded sha256
                  set to that of acceptance-config.json with schedule11 replaced by late
  C dev7001    runner.py --scramble 10 7001 in development mode: every row against the committed acceptance record,
               the configuration keys that differ, and the checker's rows
  D z36        runner.py --table <Z36> --bulk-steps 0 --tail-rules '' --fallback-max-length 0 in development mode:
               the rows, the phases, the stuck block and the checker's rows
  E trees      Runner.step_tree for the part lengths of three tail steps of the stage 13 tests (b04, b20, b07)
"""
import argparse
import copy
import gzip
import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
import time
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
parser.add_argument('--out', required=True, type=Path)
parser.add_argument('--work', required=True, type=Path)
parser.add_argument('--cache', required=True, type=Path)
parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[4])
args = parser.parse_args()
ROOT = args.root.resolve()
if args.out.exists():
    parser.error('--out must be a new file')
WORK = args.work.resolve()
WORK.mkdir(parents=True, exist_ok=False)
CACHE = args.cache.resolve()
CACHE.mkdir(parents=True, exist_ok=True)
ENV = dict(os.environ, OPENBLAS_NUM_THREADS='1', OMP_NUM_THREADS='1', MKL_NUM_THREADS='1',
           PYTHONDONTWRITEBYTECODE='1', JUMBLING_HOME_CACHE=str(CACHE / 'home_centroids.npy'))
os.environ.update(ENV)                      # before any theory module reads its cache path
J = ROOT / 'research/jumbling'
T13 = J / 'restoration/13-tutorial-runner'
S13 = T13 / 'scripts'
D0 = T13 / 'runs/dev/d0-s7001'
F1 = T13 / 'runs/depth10-seed7001'
sys.path[:0] = [str(J / 'theory'), str(J), str(J / 'fixtures'), str(S13)]
import numpy as np  # noqa: E402
import restore  # noqa: E402
import sim  # noqa: E402
import buffer  # noqa: E402  (export_pose_table only, for the Z36 table)
import runner as R  # noqa: E402  (SCRIPTS, Runner.step_tree, word_sha256)
import runner_api as api  # noqa: E402
import rules_height  # noqa: E402  (SCRIPTS only)
import rules_kernel  # noqa: E402  (SCRIPTS only)
import lattice_phase  # noqa: E402  (SCRIPTS only)

T0 = time.monotonic()
# The R22D-36 fixture: 26 letters from the solved state (stage 12, test_r22d25.py:35, :39-42).
Z36 = '8733a0aafd8e24ea1da990ebab0347074c20b11cf61d9ed04d34d25ef6f67261'
WORD36 = [[0, 12], [74, 8], [188, 4], [74, 4], [188, 8], [188, 4], [188, 4],
          [74, 8], [188, 8], [74, 4], [188, 8], [178, 1], [188, 4], [74, 8],
          [188, 4], [74, 4], [188, 8], [188, 8], [188, 4], [74, 8], [188, 8],
          [74, 4], [178, 1], [0, 45], [14, 12], [12, 12]]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def load(path):
    raw = Path(path).read_bytes()
    return json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)


def dump(path, doc):
    """The runner's own JSON layout (runner.py:89-98, atomic_json)."""
    Path(path).write_text(json.dumps(doc, indent=1, allow_nan=False) + '\n')


def compact_gzip(path, doc):
    """The runner's tree layout (runner_api.py:210-214): compact JSON, a newline, gzip with mtime 0."""
    data = (json.dumps(doc, separators=(',', ':'), allow_nan=False) + '\n').encode('utf-8')
    Path(path).write_bytes(gzip.compress(data, mtime=0))
    return hashlib.sha256(data).hexdigest()


def status():
    """The repository's status, ignored files included, outside the private work trees (read-only)."""
    out = subprocess.check_output(['git', '--no-optional-locks', 'status', '--porcelain=v1', '--ignored',
                                   '--untracked-files=all', '--', '.', ':(exclude)work', ':(exclude).claude'],
                                  cwd=ROOT)
    return {'sha256': hashlib.sha256(out).hexdigest(), 'lines': len(out.splitlines())}


def check(run_dir, name):
    """The committed checker on a run directory, as the README's replay line runs it."""
    out = WORK / (name + '-check.jsonl')
    started = time.monotonic()
    proc = subprocess.run([sys.executable, '-I', '-B', str(S13 / 'check_run.py'), str(run_dir), '--out', str(out)],
                          cwd=ROOT, capture_output=True, text=True, env=ENV)
    rows = [json.loads(line) for line in out.read_text().splitlines()]
    return {'rc': proc.returncode, 'seconds': round(time.monotonic() - started, 1),
            'steps': [{k: r.get(k) for k in ('index', 'rule', 'pass', 'fails', 'unchecked')} for r in rows[:-1]],
            'final': rows[-1]}


def start_runner(name, extra):
    out, cache = WORK / name, CACHE / name
    cache.mkdir()
    log = (WORK / (name + '.log')).open('w')
    env = dict(ENV, JUMBLING_HOME_CACHE=str(cache / 'home_centroids.npy'))
    proc = subprocess.Popen([sys.executable, '-I', '-B', str(S13 / 'runner.py'), '--out', str(out),
                             '--cache', str(cache), *extra], cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, env=env)
    return {'proc': proc, 'log': log, 'out': out, 'started': time.monotonic()}


def finish_runner(job):
    rc = job['proc'].wait()
    job['log'].close()
    seconds = round(time.monotonic() - job['started'], 1)
    lines = (WORK / (job['out'].name + '.log')).read_text().splitlines()
    return rc, seconds, json.loads(lines[-1]) if lines and lines[-1].startswith('{') else None


def row_summary(base, s):
    word = load(base / s['word'])
    tree_raw = (base / s['tree']['file']).read_bytes()
    tree_data = gzip.decompress(tree_raw)
    return {'index': s['index'], 'phase': s['phase'], 'subphase': s['subphase'], 'rule': s['rule'],
            'letters': s['letters'], 'word': word['word'] if len(word['word']) <= 20 else None,
            'word_sha256': s['word_sha256'], 'word_sha256_recomputed': R.word_sha256(word['word']),
            'word_input_digest_is_before': word['input_digest'] == s['before']['digest'],
            'word_output_digest_is_after': word['output_digest'] == s['after']['digest'],
            'tree': s['tree'], 'tree_mtime_zero': tree_raw[4:8] == bytes(4),
            'tree_sha256_recomputed': hashlib.sha256(tree_data).hexdigest(), 'tree_root': json.loads(tree_data),
            'before': s['before'], 'after': s['after']}


out = {'format': 'tutorial-ch07-examples/1',
       'versions': {'python': platform.python_version(), 'numpy': np.__version__},
       'sources': {name: sha(path) for name, path in (
           ('runner.py', S13 / 'runner.py'), ('runner_api.py', S13 / 'runner_api.py'),
           ('check_run.py', S13 / 'check_run.py'), ('check_contracts.py', S13 / 'check_contracts.py'),
           ('check_lattice.py', S13 / 'check_lattice.py'), ('acceptance-config.json', T13 / 'acceptance-config.json'),
           ('d0-s7001/run/run.json', D0 / 'run/run.json'), ('d0-s7001/check.jsonl', D0 / 'check.jsonl'))},
       'status_before': status()}

# A. The committed D0 record of seed 7001 (acceptance mode, made on cd5e6e8).
rec = load(D0 / 'run/run.json')
config = rec['config']
runner_list = [Path(p).as_posix() for p in R.SCRIPTS]
plugin_lists = {m.__name__: [Path(p).as_posix() for p in m.SCRIPTS] for m in (rules_height, rules_kernel, lattice_phase)}
merged = list(dict.fromkeys(runner_list + [p for ps in plugin_lists.values() for p in ps]))
named_twice = sorted({p for p in merged if (runner_list + [q for ps in plugin_lists.values() for q in ps]).count(p) > 1})
committed_rows = [json.loads(line) for line in (D0 / 'check.jsonl').read_text().splitlines()]
f1 = load(F1 / 'run.json')
step0_tree = (D0 / 'run' / rec['steps'][0]['tree']['file']).read_bytes()
expected_tree = (json.dumps(api.single('seam', 1), separators=(',', ':'), allow_nan=False) + '\n').encode()
copy_a0 = WORK / 'A-d0'
shutil.copytree(D0 / 'run', copy_a0)
checked = check(copy_a0, 'A-d0')
out['A_d0'] = {
    'format': rec['format'], 'menu': rec['menu'], 'start': rec['start'],
    'config': {k: v for k, v in config.items() if k != 'scripts'},
    'acceptance_config_file_sha256_now': sha(T13 / 'acceptance-config.json'),
    'acceptance_config_matches_file': config['acceptance_config']['sha256'] == sha(T13 / 'acceptance-config.json'),
    'scripts': {'recorded': len(config['scripts']), 'runner_list': len(runner_list),
                'plugin_lists': {k: len(v) for k, v in plugin_lists.items()}, 'merged': len(merged),
                'added_by_plugins': len(merged) - len(runner_list), 'named_twice': named_twice,
                'recorded_equals_merged': set(config['scripts']) == set(merged),
                'repository_relative': all(not p.startswith('/') for p in config['scripts']),
                'sha256_equal_to_checkout': all(sha(ROOT / p) == h for p, h in config['scripts'].items())},
    'phases': rec['phases'], 'stuck': rec['stuck'], 'lattice': rec['lattice'], 'result': rec['result'],
    'error': rec['error'], 'acceptance': rec['acceptance'], 'totals': rec['totals'],
    'runner_out': json.loads((D0 / 'runner.out').read_text().splitlines()[-1]),
    'steps': [row_summary(D0 / 'run', s) for s in rec['steps']],
    'step0_tree_bytes_are_compact_json_and_newline': gzip.decompress(step0_tree) == expected_tree,
    'step0_word_sha256_of_string': hashlib.sha256(json.dumps([[72, 18]]).encode('ascii')).hexdigest(),
    'format1_record': {'format': f1['format'], 'bulk_order': f1['config'].get('bulk_order'),
                       'same_rules_words_and_digests': [
                           (a['rule'], load(F1 / a['word'])['word'], a['before'], a['after'])
                           == (b['rule'], load(D0 / 'run' / b['word'])['word'], b['before'], b['after'])
                           for a, b in zip(f1['steps'], rec['steps'])],
                       'steps': len(f1['steps'])},
    'check': checked,
    'check_rows_equal_committed': [dict(r) for r in committed_rows] == [
        json.loads(line) for line in (WORK / 'A-d0-check.jsonl').read_text().splitlines()],
}
print('A done', round(time.monotonic() - T0, 1), flush=True)

# B. Four changed copies of that record.
config_text = (T13 / 'acceptance-config.json').read_text()
late_sha = hashlib.sha256(config_text.replace('"schedule11"', '"late"').encode()).hexdigest()


def tamper(name, edit):
    base = WORK / ('B-' + name)
    shutil.copytree(D0 / 'run', base)
    doc = load(base / 'run.json')
    note = edit(base, doc)
    dump(base / 'run.json', doc)
    result = check(base, 'B-' + name)
    result['change'] = note
    return result


def raise_item(base, doc, update_sha):
    path = base / doc['steps'][3]['word']
    word = load(path)
    old = list(word['word'][0])
    word['word'][0][1] += 1
    dump(path, word)
    if update_sha:
        doc['steps'][3]['word_sha256'] = R.word_sha256(word['word'])
    return {'step': 3, 'letter_before': old, 'letter_after': word['word'][0], 'row_sha256_updated': update_sha}


def move_span(base, doc):
    row = doc['steps'][2]
    tree = {'name': 'seam', 'letters': 1, 'span': [1, 2]}
    row['tree']['sha256'] = compact_gzip(base / row['tree']['file'], tree)
    return {'step': 2, 'tree': tree}


def old_restorer(base, doc):
    c = doc['config']
    c['lattice']['restorer'] = 'late'
    c['acceptance_config']['options']['lattice_restorer'] = 'late'
    c['acceptance_config']['sha256'] = late_sha
    return {'restorer': 'late', 'acceptance_config_sha256': late_sha}


out['B_tampered'] = {
    'a': tamper('a', lambda base, doc: raise_item(base, doc, False)),
    'b': tamper('b', lambda base, doc: raise_item(base, doc, True)),
    'c': tamper('c', move_span),
    'd': tamper('d', old_restorer),
}
print('B done', round(time.monotonic() - T0, 1), flush=True)

# C and D. Two development-mode runs of the committed runner, concurrently.
menu = restore.menu('I_a')
st = sim.State(restore.ctx, menu=menu)
for c, i in WORD36:
    outcome = st.apply(sim.Twist(restore.ctx, c, restore.element(menu, c, i), 'menu', {'menu': menu.name}),
                       journal=False)
    assert outcome.applied and not outcome.uncertain
assert st.digest() == Z36
buffer.export_pose_table(st, WORK / 'z36.json')
jobs = {'C-dev7001': start_runner('C-dev7001', ['--scramble', '10', '7001']),
        'D-z36': start_runner('D-z36', ['--table', str(WORK / 'z36.json'), '--bulk-steps', '0', '--tail-rules', '',
                                        '--fallback-max-length', '0'])}
finished = {name: finish_runner(job) for name, job in jobs.items()}

rc, seconds, last = finished['C-dev7001']
dev = load(WORK / 'C-dev7001/run.json')
differ = {}
for key in sorted(set(config) | set(dev['config'])):
    if key != 'scripts' and config.get(key) != dev['config'].get(key):
        differ[key] = {'acceptance': config.get(key), 'development': dev['config'].get(key)}
out['C_dev7001'] = {
    'rc': rc, 'seconds': seconds, 'runner_line': last, 'mode': dev['config']['mode'], 'result': dev['result'],
    'acceptance': dev['acceptance'], 'totals_bulk_twists': dev['totals']['bulk_twists'],
    'source': dev['config']['source'],
    'steps': len(dev['steps']),
    'same_rule_word_before_after_as_d0': [
        (a['rule'], load(WORK / 'C-dev7001' / a['word'])['word'], a['before'], a['after'])
        == (b['rule'], load(D0 / 'run' / b['word'])['word'], b['before'], b['after'])
        for a, b in zip(dev['steps'], rec['steps'])],
    'config_keys_that_differ': differ,
    'scripts_equal_to_d0': dev['config']['scripts'] == config['scripts'],
    'check': check(WORK / 'C-dev7001', 'C-dev7001'),
}
print('C done', round(time.monotonic() - T0, 1), flush=True)

rc, seconds, last = finished['D-z36']
z = load(WORK / 'D-z36/run.json')
stuck = z['stuck']
out['D_z36'] = {
    'rc': rc, 'seconds': seconds, 'runner_line': last, 'start': z['start'], 'mode': z['config']['mode'],
    'component': z['config']['component'], 'result': z['result'], 'error': z['error'],
    'acceptance': z['acceptance'], 'totals': z['totals'], 'kernel_d19': z['config']['kernel']['d19'],
    'budgets': z['config']['budgets'],
    'phases': z['phases'],
    'steps': [dict({k: v for k, v in s.items() if k not in ('word', 'tree')},
                   word=load(WORK / 'D-z36' / s['word']), tree_root=load(WORK / 'D-z36' / s['tree']['file']),
                   tree=s['tree'], word_file=s['word']) for s in z['steps']],
    'stuck': {'phase': stuck['phase'], 'reason': stuck['reason'], 'digest': stuck['digest'], 'off': stuck['off'],
              'heights': stuck['heights'], 'units': len(stuck['units']),
              'attempts': sum(len(u['attempts']) for u in stuck['units']),
              'unit_sizes': sorted({u['size'] for u in stuck['units']}), 'first_unit': stuck['units'][0]}
    if stuck else None,
    'check': check(WORK / 'D-z36', 'D-z36'),
}
print('D done', round(time.monotonic() - T0, 1), flush=True)

# E. Tail-step trees from the runner's own helper (runner.py:1233-1244), with the part lengths that
# test_runner.py asserts for b04 (:2770), b20 (:2803) and b07 (:2826).
trees = {}
for name, rule, parts in (('b04', 't1prime', [['W', 288], ['V', 339], ['middle', 288], ['V_inverse', 339]]),
                          ('b20', 'close', [['V', 20232], ['middle', 36], ['V_inverse', 20232]]),
                          ('b07', 't1prime', [['W', 48], ['V', 139774], ['middle', 48], ['V_inverse', 139774]])):
    letters = sum(n for _, n in parts)
    root = R.Runner.step_tree(rule, parts, letters)
    data = (json.dumps(root, separators=(',', ':'), allow_nan=False) + '\n').encode('utf-8')
    trees[name] = {'rule': rule, 'parts': parts, 'letters': letters, 'tree': root,
                   'size': api.check_tree(root, letters), 'sha256': hashlib.sha256(data).hexdigest()}
out['E_trees'] = trees

out['status_after'] = status()
out['status_unchanged'] = out['status_after'] == out['status_before']
out['seconds'] = round(time.monotonic() - T0, 1)
args.out.write_text(json.dumps(out, indent=1, ensure_ascii=False) + '\n')
print('done', out['seconds'], flush=True)
