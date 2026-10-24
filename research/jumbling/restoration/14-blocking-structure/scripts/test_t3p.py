"""Packet acceptance gate: one process, one thread, external temporary files.

Run: python3 -I -B research/jumbling/restoration/14-blocking-structure/scripts/test_t3p.py
The 1500-second deadline includes imports, the fresh label build and all seven
parts. Assertion counts below count explicit acceptance assertions, not internal
simulator assertions. Timings are diagnostics, not native/performance evidence.
"""
import os
import sys
import time

START = time.monotonic()
CPU_START = time.process_time()
sys.dont_write_bytecode = True
for _name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ[_name] = '1'

import signal  # noqa: E402


def timeout(signum, frame):
    raise TimeoutError('T3P acceptance exceeded 1500 wall seconds')


signal.signal(signal.SIGALRM, timeout)
signal.setitimer(signal.ITIMER_REAL, 1500)

from contextlib import contextmanager  # noqa: E402
import copy  # noqa: E402
import hashlib  # noqa: E402
import json  # noqa: E402
from pathlib import Path  # noqa: E402
import resource  # noqa: E402
import tempfile  # noqa: E402

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import numpy as np  # noqa: E402
import t3p as P  # noqa: E402
import levels as L  # noqa: E402
import locality as LOC  # noqa: E402
from exact import Q5  # noqa: E402

REPO = HERE.parents[4]
TABLE = REPO / 'research/jumbling/restoration/13-tutorial-runner/runs/dev/d2-s7304/component/tables/00005.json'
RESULTS = HERE.parent / 'results'
DIGEST = 'ac127dc3488832b1d1b75a043d63e84b75addd1f4e73fe7e961407aea2ff0980'
W3 = [(344, 9), (451, 55), (522, 14)]
W5 = [(344, 9), (451, 55), (522, 49), (451, 44), (522, 15)]
COUNTS = 0
REPORT = []
COMMANDS = {}


def require(condition, message):
    global COUNTS
    COUNTS += 1
    if not condition:
        raise AssertionError(message)


@contextmanager
def part(name):
    wall, cpu, count = time.monotonic(), time.process_time(), COUNTS
    print(json.dumps(dict(part=name, status='running')), flush=True)
    try:
        yield
    except BaseException:
        print(json.dumps(dict(part=name, status='failed', assertions=COUNTS - count,
                              wall_s=round(time.monotonic() - wall, 3))), flush=True)
        raise
    row = dict(part=name, status='passed', assertions=COUNTS - count,
               wall_s=round(time.monotonic() - wall, 3), cpu_s=round(time.process_time() - cpu, 3))
    REPORT.append(row)
    print(json.dumps(row, sort_keys=True), flush=True)


def certificate(*rounds):
    return dict(format='t3p-certificate/1', table_digest=DIGEST, rounds=list(rounds))


def target_piece(state, x, h=None):
    for pid, members, holders in P.high_units(state):
        if x in holders and (h is None or state.oracle.lowers(
                state.st, pid, x, (h,), int(state.h[members[0]]))[0]):
            return int(members[0])
    raise AssertionError(f'no high unit at target {x}')


def fork(base):
    return P.State(P.BM.load_table(TABLE), base.lab.copy(), base.h.copy(), base.b.copy())


def second_w5_piece():
    # Independent preview has no label cache: query exact labels of its actual
    # endpoint. Undo every preview twist by its inverse, then check the digest.
    st = P.BM.load_table(TABLE)
    applied = []
    try:
        for c, i in W5[:3]:
            require(P.J.twist(st, c, P.matrix(c, i)), 'W5 preview admissibility')
            applied.append((c, i))
        require(st.digest().startswith('d4851f18'), 'W5 round-two start digest')
        h, b = P.BM.tree_info(st)
        lx = P.J.labels_at(st, np.arange(len(h)), 522)
        oracle = P.TreeOracle()
        for pid in np.unique(st.pose_id[h >= 4]).tolist():
            members = np.flatnonzero(st.pose_id == pid)
            if ((lx[members] == 1).all() and b[members[0]] == P.CLS[522]
                    and oracle.lowers(st, pid, 522, (15,), int(h[members[0]]))[0]):
                return int(members[0])
        raise AssertionError('W5 second round unit not found')
    finally:
        for c, i in reversed(applied):
            require(P.J.twist(st, c, P.to_tuple(P.transpose(P.matrix(c, i)))), 'preview inverse')
        require(st.digest() == DIGEST, 'W5 preview restored X')


def trace_checks(result, phis, digests):
    rows = [r for rd in result['rounds'] for r in rd['replay']]
    require(len(rows) == len(phis), 'all replay letters recorded')
    for row, phi, digest in zip(rows, phis, digests):
        require(row['admissible'], 'replay letter admissible')
        require(row['moved_set_equal'], 'replay moved set equals the theorem set')
        require(row['phi'] == phi, 'replay Phi agrees with packet')
        require(row['digest'].startswith(digest), 'replay digest agrees with packet')


def run():
    readonly = [HERE / name for name in ('blockmap.py', 'jamcheck.py', 'peel.py', 'replay_word.py')]
    readonly += [TABLE, HERE.parent / 'README.md']
    readonly += [RESULTS / name for name in
                 ('s7304-moves-522.json', 's7304-peel-522.json', 's7304-peel-522-replay.json')]
    before = {p: hashlib.sha256(p.read_bytes()).hexdigest() for p in readonly}
    with tempfile.TemporaryDirectory(prefix='t3p-gate-') as tmp:
        tmp = Path(tmp)
        with part('1 locality'):
            loc = LOC.analyse()
            COMMANDS['locality'] = dict(cpu_s=loc['cpu_s'])
            r2, delta, partners = LOC.graph()
            require(r2 == Q5(96, -32), 'exact R squared')
            require(loc['R2'] == '96 - 32*sqrt(5)', 'exact string')
            require(loc['equal_partner_counts'] and loc['partner_count'] == 80, 'uniform count 80')
            for c in range(600):
                require(len(partners[c]) == 80 and partners[c] == sorted(set(partners[c])), '80 sorted partners')
                require(c not in partners[c], 'no diagonal interaction')
                require(all(c in partners[d] for d in partners[c]), 'symmetric graph')
                require(all((delta[c][d].sign() > 0) == (d in partners[c])
                            for d in range(600) if c != d), 'all-pair exact delta signs')
            require(delta[451][344].sign() <= 0, '451 and 344 do not interact')
            shells = [s for s in loc['shells'] if s['grips'] != [0]]
            require([s['count'] for s in shells if s['interacts']] == [4, 12, 24, 12, 4, 24], 'six shells')
            require(all(s['interacts'] for s in shells[:6]) and not shells[6]['interacts'],
                    'next shell (about 56.01 degrees) does not interact')
            LOC.write_json(tmp / 'locality.json', loc)

        with part('setup X (fresh exact labels)'):
            base, setup = P.load_input(TABLE, labels=tmp / 'X.npz')
            COMMANDS['setup_X'] = setup
            require(base.st.digest() == DIGEST, 'X digest')
            require(P.J.phi(base.h) == 1594, 'X Phi')
            require(base.lab.shape == (177120, 600), 'full piece/grip label table')
            print(json.dumps(dict(setup=setup), sort_keys=True), flush=True)

        with part('2 levels'):
            result = L.analyse(base)
            COMMANDS['levels'] = dict(cpu_s=result['cpu_s'])
            require(result['histogram'] == {'0': 41, '1': 102, '2': 114, '3': 150, '4': 142, '5': 51},
                    'level histogram at X')
            expected = {540: 0, 522: 1, 229: 1, 336: 1, 2: 2, 375: 2,
                        64: 3, 162: 3, 239: 3, 251: 3, 73: 4, 183: 4, 596: 4}
            own = {h['grip']: h['level'] for u in result['high_units'] for h in u['own_holders']}
            require(own == expected, 'exactly the thirteen own-holder levels')
            for c, level in expected.items():
                require(result['levels'][c] == level, 'own-holder level')
            require(result['lambda'] == 0, 'lambda is zero')
            w = result['lambda_witness']
            require(base.h[w['piece']] >= 4 and base.lab[w['piece'], w['holder']] == 1
                    and result['levels'][w['holder']] == 0, 'lambda witness')
            for u in result['high_units']:
                members = P.unit_members(base, u['unit_piece'])
                holder_sets = [frozenset(np.flatnonzero(base.lab[p] == 1).tolist()) for p in members]
                split = u['cell_split']
                require(split['distinct_holder_rows'] == len(set(holder_sets)), 'distinct per-member rows')
                require(split['intersection_size'] == len(set.intersection(*(set(s) for s in holder_sets))),
                        'holder intersection')
                require(split['union_size'] == len(set.union(*(set(s) for s in holder_sets))), 'holder union')
            toy = np.zeros((5, 600), np.uint8)
            toy[0, [0, 1]] = [1, 2]
            toy[1, [1, 2]] = [1, 2]
            toy[2, [4, 3]] = [1, 2]
            toy[3, [3, 4]] = [1, 2]
            toy[4, 5] = 2
            lev, _ = L.fixed_point(toy)
            require(lev[:6].tolist() == [0, 1, 2, L.INF, L.INF, L.INF],
                    'greatest fixed point keeps a dependency cycle and a holderless straddler infinite')
            LOC.write_json(tmp / 'levels.json', result)

        with part('3 certificates and tampering'):
            p522 = target_piece(base, 522, 14)
            first = dict(x=522, unit_piece=p522, h=14, helpers=[[344, 9], [451, 55]])
            state3 = fork(base)
            c3 = P.check(state3, certificate(first))
            COMMANDS['check_W3'] = dict(cpu_s=c3['cpu_s'])
            require(c3['passed'], 'W3 one-round certificate: ' + str(c3['failures']))
            ref = json.loads((RESULTS / 's7304-peel-522-replay.json').read_text())
            trace_checks(c3, [1594, 1594, 1498],
                         ['ba4214fd', ref['rows'][2]['digest'], ref['rows'][3]['digest']])
            r = c3['rounds'][0]
            require((r['s0'], r['f0'], r['F'], r['D']) == (0, 0, 22, 70), 'W3 residuals and F,D')
            require([(a['grip'], a['inside_size'], a['s'], a['f']) for a in r['helpers']]
                    == [(344, 2334, 0, 0), (451, 2989, 0, 22)], 'W3 helper contributions')
            lower = [i for i, yes in zip(P.JUMBLES, base.oracle.lowers(
                base.st, base.st.pose_id[p522], 522, P.JUMBLES, int(base.h[p522]))) if yes]
            require(lower == [14, 21, 23, *range(43, 52)], 'tree-derived lowering set, no coset convention')
            p_second = second_w5_piece()
            w5first = dict(first, h=49)
            w5second = dict(x=522, unit_piece=p_second, h=15, helpers=[[451, 44]])
            state5 = fork(base)
            c5 = P.check(state5, certificate(w5first, w5second))
            COMMANDS['check_W5'] = dict(cpu_s=c5['cpu_s'])
            require(c5['passed'], 'W5 two-round certificate: ' + str(c5['failures']))
            trace_checks(c5, [1594, 1594, 1498, 1498, 1472],
                         ['ba4214fd', 'c43d0c95', 'd4851f18', '32ae273f',
                          '956d3d3e5189646a38ede9ce134f04662c8c15e245593948c265041107bb9f92'])
            require(c5['rounds'][-1]['replay'][-1]['n4'] == 736, 'W5 final n4')
            r = c5['rounds'][1]
            require((r['s0'], r['f0'], r['F'], r['D']) == (0, 0, 9, 22), 'W5 second round F,D')
            tamper3 = copy.deepcopy(first)
            tamper3['helpers'][1][1] = 2
            bad_h = next(i for i in P.JUMBLES if i not in lower)
            p229 = target_piece(base, 229)
            h229 = next(i for i, yes in zip(P.JUMBLES, base.oracle.lowers(
                base.st, base.st.pose_id[p229], 229, P.JUMBLES, int(base.h[p229]))) if yes)
            bad_cases = [(tamper3, 'P3'), (dict(first, h=bad_h), 'unit not lowered'),
                         (dict(x=229, unit_piece=p229, h=h229, helpers=[[295, 1], [347, 1]]), 'P0')]
            for k, (spec, failure) in enumerate(bad_cases):
                cert_path, out = tmp / f'bad-{k}.json', tmp / f'bad-{k}-result.json'
                LOC.write_json(cert_path, certificate(spec))
                code = P.main(['check', str(TABLE), str(cert_path), str(out), '--labels', str(tmp / 'X.npz')])
                failed = json.loads(out.read_text())
                require(code == 1 and not failed['passed'], 'tampered CLI check exits 1')
                require(any(failure in s for s in failed['failures']), 'named tampered hypothesis: ' + failure)
            LOC.write_json(tmp / 'W3.json', c3)
            LOC.write_json(tmp / 'W5.json', c5)
            del state5

        with part('4 restricted scan'):
            result = P.scan(base, {522, 229, 540, 64})
            COMMANDS['scan_restricted'] = dict(cpu_s=result['cpu_s'], targets=[522, 229, 540, 64])
            require(result['exhausted'] and bool(result['family_exhausted']), 'scan names exhausted family')
            require(result['single_letter_descents'] == [], 'no single tame descent at X')
            for x in (522, 229, 540, 64):
                rows = [r for r in result['rows'] if r['x'] == x]
                require(bool(rows), 'target covered by scan')
                for row in rows:
                    if x == 522:
                        require(row['status'] == 'round' and row['best']['predicted_delta_phi'] == -96,
                                '522 has the optimal -96 round')
                    elif x == 229:
                        require(row['status'] == 'no-disjoint-cover', '229 has no disjoint cover')
                        require(dict(grips=[295, 347], size=186) in row['overlaps'], '229 overlap 186')
                    elif x == 540:
                        require(row['status'] != 'round', '540 has no round')
                    else:
                        require(row['status'] == 'straddler-without-clean-holder' and row['count'] == 41,
                                '64 has 41 straddlers without a clean holder')
                        require(row['pose_groups'] == [dict(pose=1007, count=41)], '64 missing straddlers unit')
            require(result['best_certificate']['format'] == 't3p-certificate/1', 'scan certificate format')
            # Exercise the exact optimiser and precedence cases beyond X's edge-free W3.
            opts = {1: [dict(item=12, f=1, d=[0], dphi=-100, before=[]),
                        dict(item=13, f=5, d=[7], dphi=0, before=[])]}
            feasible = list(P.assignments((1,), opts, 0))
            require(min(v[0] for v in feasible) == -2, 'optimise F-D rather than F')
            require(min(v[1] + 2 * v[0] for v in feasible if v[0] < 0) == -4,
                    'total-delta optimisation still enforces P4')
            opts = {1: [dict(item=1, f=0, d=[0], dphi=0, before=[2])],
                    2: [dict(item=1, f=0, d=[0], dphi=0, before=[1]),
                        dict(item=0, f=0, d=[0], dphi=0, before=[])]}
            feasible = list(P.assignments((1, 2), opts, 0))
            require(len(feasible) == 1 and feasible[0][3] == [1], 'cycle rejected, identity dropped')
            require(P.topological([9, 1, 3], [(9, 1)]) == [3, 9, 1], 'topological grip tie-break')
            require(list(P.maximal_covers([1, 2], {1: 3, 2: 6}, 5)) == [], 'overlapping cover rejected')
            LOC.write_json(tmp / 'scan.json', result)

        with part('5 restricted pairs'):
            result = P.pairs(base, 522, 451, 344, [55, 43, 2], [9, 13, 1])
            COMMANDS['pairs_restricted'] = dict(cpu_s=result['cpu_s'], counts=result['counts'])
            fixture = json.loads((RESULTS / 's7304-peel-522.json').read_text())
            lookup = {(r['a'], r['b']): r for r in fixture['rows']}
            orders = {}
            for row in result['rows']:
                word = row['word']
                by_grip = dict(word)
                key = (by_grip[451], by_grip[344])
                ref = lookup[key]
                require(row['admissible'] and row['tame'], 'restricted pair admissible and tame')
                require((row['S'], row['F']) == (ref['S'], ref['F']), 'pair reproduces committed S,F')
                orders.setdefault(key, []).append((row['S'], row['F'], row['max_D']))
            for values in orders.values():
                require(len(values) == 2 and values[0] == values[1], 'both orders agree')
            require(result['counts'] == dict(words=18, admissible=18, tame=18, clean=12), 'pair counts')
            require(result['best']['F_minus_D'] == -48, 'pair best budget')
            require(base.st.digest() == DIGEST, 'pairs restored X')
            LOC.write_json(tmp / 'pairs.json', result)

        with part('6 plus (limit 1)'):
            result = P.plus(base, limit=1)
            COMMANDS['plus_limit_1'] = dict(cpu_s=result['cpu_s'], tested_pairs=result['tested_pairs'],
                                          tested_items=result['tested_items'], instances=len(result['instances']))
            require(result['passed'], 'all reported T3P+ replay checks pass')
            require(len(result['instances']) <= 1, 'plus respects the limit')
            for row in result['instances']:
                require(row['passed'] and row['replay']['admissible'], 'T3P+ instance verified')
                require(row['delta_phi'] <= 0, 'T3P+ helper tame')
            require(base.st.digest() == DIGEST, 'plus restored X')
            LOC.write_json(tmp / 'plus.json', result)

        with part('7 label update versus full rebuild'):
            rebuilt = P.BM.all_labels(state3.st, workers=1)
            require(np.array_equal(state3.lab, rebuilt), 'all moved-row updates after W3 equal a full rebuild')
            h, b = P.BM.tree_info(state3.st)
            require(np.array_equal(h, state3.h) and np.array_equal(b, state3.b), 'replay heights/branches exact')
            for path, digest in before.items():
                require(hashlib.sha256(path.read_bytes()).hexdigest() == digest, 'source/fixture unchanged')
            require(not any(HERE.glob('__pycache__')), 'no bytecode output in the scripts directory')
            require(resource.getrusage(resource.RUSAGE_SELF).ru_maxrss < 3 * 1024 * 1024, 'RSS at most 3 GiB')
            require(time.monotonic() - START < 1500, 'within the 1500-second deadline')
    print(json.dumps(dict(status='passed', assertions=COUNTS, parts=REPORT, commands=COMMANDS,
                          wall_s=round(time.monotonic() - START, 3),
                          cpu_s=round(time.process_time() - CPU_START, 3),
                          peak_rss_mib=round(resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / 1024, 1)),
                     sort_keys=True), flush=True)


if __name__ == '__main__':
    try:
        run()
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
