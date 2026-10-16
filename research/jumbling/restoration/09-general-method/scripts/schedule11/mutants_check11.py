"""Mutation test of the runner part of check_schedule11.py (review findings R11-AC-01 to R11-AC-03).

usage (from the repository root):
    python3 -I -B research/jumbling/restoration/09-general-method/scripts/schedule11/mutants_check11.py RUN
where RUN is an output directory of restore_schedule11.py on uniform:1 and uniform:2 (METHOD.md section 7).

The unchanged output must pass `check_runner_output`, and each mutant must fail it with the expected message:
- no runs, a missing run and a duplicate run (R11-AC-02);
- an empty product and a letter that is not an integer, which the stage 03 checker cannot evaluate either (R11-AC-03);
- identity padding added to the slice of orbit 10, charged to another orbit or reported on orbit 10, and the last
  factor of orbit 10 moved into the next orbit's slice (R11-AC-01).
Exit status 0 only if every case behaves so. Research only: exact synthetic geometry.
"""
import copy
import json
import shutil
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import check_schedule11 as C              # noqa: E402  (runs part 1 and the set-up only)

RUN = Path(sys.argv[1])
SUM = json.loads((RUN / 'restore-summary.json').read_text())
SOL = json.loads((RUN / 'solutions.json').read_text())
BAD = []


def node_lengths(nodes):
    ln = []
    for nd in nodes:
        op = nd['op']
        if op == 'word':
            ln.append(len(nd['w']))
        elif op == 'path':
            ln.append(len(C.tree_path(nd['orbit'], nd['node'])))
        elif op == 'conj':
            ln.append(ln[nd['x']] + 2 * ln[nd['u']])
        else:
            ln.append(sum(ln[j] for j, _ in nd['f']))
    return ln


LN = node_lengths(SOL['nodes'])
CASCADE = SUM['cascade_order']


def slice_of(run, o):
    fpo = run['factors_per_orbit']
    start = sum(fpo[str(p)] for p in CASCADE[:CASCADE.index(o)])
    return start, start + fpo[str(o)]


def case(name, mutate, expect):
    summary, sol = copy.deepcopy(SUM), copy.deepcopy(SOL)
    mutate(summary, sol)
    with tempfile.TemporaryDirectory() as tmp:
        d = Path(tmp)
        for f in RUN.glob('start-labels-*.npy'):
            shutil.copy(f, d / f.name)
        (d / 'restore-summary.json').write_text(json.dumps(summary))
        (d / 'solutions.json').write_text(json.dumps(sol))
        C.FAILS.clear()
        C.check_runner_output(d)
        fails = list(C.FAILS)
    ok = not fails if expect is None else any(expect in f for f in fails)
    print('%-4s %-34s %s' % ('ok' if ok else 'BAD', name, 'passes' if not fails else 'fails: ' + fails[0]),
          flush=True)
    if not ok:
        BAD.append(name)


def no_runs(summary, sol):
    summary['runs'], sol['solutions'] = [], {}


def missing_run(summary, sol):
    summary['runs'] = summary['runs'][:1]
    del sol['solutions']['uniform_2']


def duplicate_run(summary, sol):
    summary['runs'] = [summary['runs'][0], summary['runs'][0]]


def empty_product(summary, sol):
    sol['nodes'].append({'op': 'prod', 'f': []})


def float_letter(summary, sol):
    nodes = sol['nodes']
    x = next(nd['x'] for nd in nodes if nd['op'] == 'conj' and nodes[nd['x']]['op'] == 'word')
    nodes[x]['w'][0] = float(nodes[x]['w'][0])


def padding(charge_elsewhere):
    def mutate(summary, sol):
        run, fs = summary['runs'][0], sol['solutions']['uniform_1']
        a, b = slice_of(run, 10)
        j = max((f[0] for f in fs[a:b]), key=lambda i: LN[i])
        slack = C.BOUND[10] - run['length_per_orbit']['10']
        k = slack // (2 * LN[j]) + 1
        extra = 2 * k * LN[j]
        fs[b:b] = [[j, 1], [j, -1]] * k
        run['factors_per_orbit']['10'] += 2 * k
        run['factors'] += 2 * k
        run['retained_length'] += extra
        if charge_elsewhere:
            o2 = max((o for o in CASCADE if o != 10),
                     key=lambda o: C.BOUND[o] - run['length_per_orbit'][str(o)])
            assert C.BOUND[o2] - run['length_per_orbit'][str(o2)] >= extra
            run['length_per_orbit'][str(o2)] += extra
        else:
            run['length_per_orbit']['10'] += extra
    return mutate


def delayed_factor(summary, sol):
    run, fs = summary['runs'][0], sol['solutions']['uniform_1']
    nxt = CASCADE[CASCADE.index(10) + 1]
    a, b = slice_of(run, 10)
    n = LN[fs[b - 1][0]]
    run['factors_per_orbit']['10'] -= 1
    run['factors_per_orbit'][str(nxt)] += 1
    run['length_per_orbit']['10'] -= n
    run['length_per_orbit'][str(nxt)] += n


case('unchanged output', lambda s, x: None, None)
case('no runs', no_runs, 'expected exactly')
case('missing uniform:2', missing_run, 'expected exactly')
case('duplicate uniform:1', duplicate_run, 'expected exactly')
case('empty product', empty_product, 'empty product')
case('letter 5.0 instead of 5', float_letter, 'independent evaluation failed')
case('padding charged to another orbit', padding(True), 'orbit 10 reported length')
case('padding reported on orbit 10', padding(False), 'orbit 10 length')
case('factor of orbit 10 delayed', delayed_factor, 'after orbit 10 an orbit up to it is not solved')
print('PASS' if not BAD else 'FAIL %s' % BAD)
sys.exit(1 if BAD else 0)
