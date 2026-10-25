"""The seven-part packet acceptance gate; one process/thread, external scratch.

No required assertion is optional. A wall alarm fails the gate at 3600 seconds.
The printed timings are measurements of this headless synthetic-geometry gate.
Run: python3 -I -B research/jumbling/restoration/15-lowering-machine/scripts/test_macro.py
"""
import copy
import json
import os
from pathlib import Path
import resource
import signal
import sys
import tempfile
import time

sys.dont_write_bytecode = True
for _var in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ[_var] = '1'
STARTED = time.monotonic()


def expired(signum, frame):
    raise AssertionError('acceptance timeout: 3600 wall seconds')


signal.signal(signal.SIGALRM, expired)
signal.alarm(3600)
sys.path.insert(0, str(Path(__file__).resolve().parent))
import macro as A  # noqa: E402
import pstar as S  # noqa: E402

np, P, J = A.np, A.P, A.J
DEV = Path('research/jumbling/restoration/13-tutorial-runner/runs/dev')
TABLES = {
    7304: DEV/'d2-s7304/component/tables/00005.json',
    7306: DEV/'d2-s7306/component-checkpoint/tables/00017.json',
    7307: DEV/'d2-s7307/component/tables/00005.json',
}
PREFIXES = {
    7304: '344:9,451:55,522:14',
    7306: '208:5,336:15,445:55,252:13,506:7,561:14',
    7307: '516:14,597:10,543:13,10:8,147:2,102:15,216:2,266:14,275:1,435:12,435:7,319:12,94:12',
}
ROWS = {
    'R04': (7304, ['277:1,375:1,384:7,162:13', '375:15'], 0,
            [1498, 1498, 1498, 1276, 1228], ['ffba95e3eed9e1ff', 'a7be8c10d2bb1582']),
    'R05': (7304, ['522:8,451:8,522:15'], 0, [1498, 1498, 1472], ['6131c20380db5964']),
    'R06': (7304, ['522:8,451:44,522:15'], 0, [1498, 1498, 1472], ['956d3d3e5189646a']),
    'R16': (7306, ['487:33,240:14'], 20, [2704, 1850], ['348070c19855ce80']),
    'R17': (7307, ['102:8,67:2,10:1,102:12'], 0,
            [1396, 1396, 1396, 1284], ['63e9f6c3eb4d7dee']),
}
ASSERTIONS = 0
REPORT = []


def expect(ok, detail):
    global ASSERTIONS
    ASSERTIONS += 1
    if not ok:
        raise AssertionError(detail)


class Part:
    def __init__(self, number, name):
        self.number, self.name = number, name

    def __enter__(self):
        self.wall, self.cpu, self.count = time.monotonic(), time.process_time(), ASSERTIONS
        print(json.dumps(dict(part=self.number, name=self.name, status='running')), flush=True)

    def __exit__(self, kind, value, tb):
        peak_rss = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss * 1024
        if kind is None:
            expect(peak_rss <= 3_000_000_000, 'peak RSS must not exceed 3 GB')
        row = dict(part=self.number, name=self.name, status='failed' if kind else 'passed',
                   assertions=ASSERTIONS-self.count, wall_s=round(time.monotonic()-self.wall, 3),
                   cpu_s=round(time.process_time()-self.cpu, 3), peak_rss_bytes=peak_rss)
        REPORT.append(row)
        print(json.dumps(row, sort_keys=True), flush=True)


def read(path):
    return json.loads(Path(path).read_text())


def main():
    with tempfile.TemporaryDirectory(prefix='macro-gate-') as scratch:
        out = Path(scratch)
        labels = {n: out/f'{n}.npz' for n in TABLES}
        certs = {}
        with Part(1, 'retained calculus and column reading'):
            state, _, _ = A.load(TABLES[7304], P.parse_word(PREFIXES[7304]), labels[7304])
            expect(state.st.digest() == 'f93e461f9b1c65d5483cf3ba53db266baee71613d8a8808e091ce4a0484daad2', '7304 chain digest')
            expect(J.phi(state.h) == 1498, '7304 chain Phi')
            bad_forward = False
            for c in (277, 451, 522):
                expect(not (state.lab[:, c] == 2).any(), f'{c} clean')
                for i in range(1, 12):
                    prediction = A.column_read(state, c, i, 522)
                    moved = state.inside(c)
                    old_lab, old_h, old_b = state.lab[moved].copy(), state.h[moved].copy(), state.b[moved].copy()
                    _, pre, forward, sigma = A.retained(c, i)
                    with A.evaluated(state) as tokens:
                        # The OLD stage-14 application supplies the independent
                        # updated_rows/tree_rows oracle, once per letter.
                        row, token = state.apply(c, i, record=False)
                        tokens.append((c, i, token))
                        expect(row['admissible'], f'{c}:{i} exact admissibility')
                        exact = state.lab[moved]
                        expect(np.array_equal(old_lab[:, pre], exact), f'{c}:{i} inverse calculus')
                        expect(np.array_equal(state.h[moved], old_h), f'{c}:{i} heights')
                        expect(np.array_equal(state.b[moved], sigma[old_b]), f'{c}:{i} branches')
                        bad_forward |= not np.array_equal(old_lab[:, forward], exact)
                        f, ds = state.budget(522)
                        expect(prediction[0] == int((state.lab[:, 522] == 2).sum()), f'{c}:{i} S column')
                        expect(prediction[1] == f, f'{c}:{i} F column')
                        expect(np.array_equal(prediction[2], ds), f'{c}:{i} D exact poses')
            expect(bad_forward, 'forward permutation must disagree at least once')
            del state

        with Part(2, 'certificates and independent replay'):
            for name, (seed, words, e, profile, digests) in ROWS.items():
                dest = out/f'{name}.json'
                args = ['cert', str(TABLES[seed]), str(dest), '--prefix', PREFIXES[seed],
                        '--labels', str(labels[seed]), '--excursion', str(e)]
                for w in words:
                    args += ['--macro', w]
                if name == 'R16':
                    args += ['--unit-piece', '32322']
                expect(A.main(args) == 0, f'cert {name}: {read(dest)}')
                certs[name] = cert = read(dest)
                observed = [v for spec in cert['macros'] for v in spec['claims']['phi']]
                expect(observed == profile, f'{name} Phi profile')
                for spec, digest in zip(cert['macros'], digests):
                    expect(spec['claims']['digests'][-1].startswith(digest), f'{name} digest {digest}')
                check_out = out/f'{name}-check.json'
                check_args = ['check', str(TABLES[seed]), str(dest), str(check_out), '--labels', str(labels[seed])]
                if name == 'R05':
                    check_args += ['--replay']
                expect(A.main(check_args) == 0, f'check {name}: {read(check_out)}')
                expect(read(check_out)['passed'], f'{name} passed')
                if name == 'R05':
                    expect(read(check_out)['replay']['passed'], 'R05 full-height replay')
                if name == 'R16':
                    claims = cert['macros'][0]['claims']
                    expect((claims['delta'], claims['F'], claims['D'], claims['excursion'])
                           == ([20, -854], 72, 499, 20), 'R16 exact lowering facts')

        with Part(3, 'tampered certificates fail by named test'):
            cases = []
            c = copy.deepcopy(certs['R16']); c['excursion'] = 0
            cases.append((7306, c, 'excursion'))
            c = copy.deepcopy(certs['R04'])
            ev = c['macros'][0]['events']; ev[0], ev[1] = ev[1], ev[0]
            cases.append((7304, c, 'admissibility'))
            c = copy.deepcopy(certs['R05']); c['table_digest'] = '0'*64
            cases.append((7304, c, 'table_digest'))
            c = copy.deepcopy(certs['R05']); c['macros'][0]['claims']['supports'][0]['sha256'] = '0'*64
            cases.append((7304, c, 'support'))
            c = copy.deepcopy(certs['R05']); c['macros'][0]['claims']['D'] += 1
            cases.append((7304, c, 'lowering count'))
            c = copy.deepcopy(certs['R05']); del c['macros'][0]['claims']['digests']
            cases.append((7304, c, 'claims'))
            c = copy.deepcopy(certs['R05']); c['prefix'].append([375, 1])
            cases.append((7304, c, 'admissibility'))
            for j, (seed, cert, test) in enumerate(cases):
                src, dest = out/f'tampered-{j}.json', out/f'tampered-{j}-check.json'
                P.write_json(src, cert)
                expect(A.main(['check', str(TABLES[seed]), str(src), str(dest), '--labels', str(labels[seed])]) == 1,
                       f'tamper {test} exit status')
                expect(test in {e['test'] for e in read(dest)['failures']}, f'tamper named {test}: {read(dest)}')

        with Part(4, '7304 step: T,R,M'):
            dest = out/'7304-step.json'
            expect(S.main(['step', str(TABLES[7304]), str(dest), '--prefix', PREFIXES[7304],
                           '--labels', str(labels[7304]), '--families', 'T,R,M']) == 0, '7304 step exit')
            result = read(dest)
            expect(not result['families']['T']['candidates'], 'T empty at chain end')
            expect(result['families']['T']['exhausted'], 'T exhausted')
            expect(any(c['target'] == 162 and c['terminal_phi'] <= 1300 for c in result['families']['R']['candidates']), 'R descent at 162')
            expect(any(c['target'] == 522 and c['terminal_phi'] <= 1472 for c in result['families']['M']['candidates']), 'M descent at 522')
            expect(result['families']['M']['exhausted'], 'M exhausted')
            expect(not result['families']['R']['exhausted'] and result['families']['R']['exhaustive_through'] == 2, 'R bounded declaration')
            expect(result['choice']['terminal_phi'] <= 1300, 'best selection')
            checked = A.check(TABLES[7304], result['certificate'], labels[7304])
            expect(checked['passed'], f'7304 chosen certificate: {checked}')
            print(json.dumps(dict(measurement='7304-step', cpu_s=result['cpu_s'],
                                  family_cpu_s={f: r['cpu_s'] for f, r in result['families'].items()},
                                  choice=result['choice']['word']), sort_keys=True), flush=True)

        with Part(5, '7306 step: E and zero raising budget'):
            for e in (20, 0):
                dest = out/f'7306-e{e}.json'
                expect(S.main(['step', str(TABLES[7306]), str(dest), '--prefix', PREFIXES[7306],
                               '--labels', str(labels[7306]), '--families', 'E', '--e0', str(e)]) == 0, '7306 E step exit')
                result = read(dest)
                candidates = result['families']['E']['candidates']
                expect(result['families']['E']['exhausted'], f'E exhausted at e0={e}')
                if e:
                    expect(any(c['terminal_phi'] <= 1850 and c['excursion'] == 20 for c in candidates), 'E R16-quality descent')
                    expect(A.check(TABLES[7306], result['certificate'], labels[7306])['passed'], 'E chosen certificate')
                else:
                    expect(all(c['excursion'] < 20 for c in candidates), 'E0 excludes excursions >=20')
                print(json.dumps(dict(measurement=f'7306-E{e}-step', cpu_s=result['cpu_s'],
                                      candidates=len(candidates)), sort_keys=True), flush=True)

        with Part(6, 'run checkpoint and resume'):
            directory = out/'run'
            args = ['run', str(TABLES[7304]), str(directory), '--prefix', PREFIXES[7304],
                    '--labels', str(labels[7304]), '--families', 'T,M']
            expect(S.main(args + ['--max-steps', '1']) == 0, 'first run exit')
            first = read(directory/'run.json')
            expect(len(first['macro_words']) == 1, 'one macro committed')
            expect(first['status'] == 'unknown' and first['reason'] == 'max-steps', 'step resource bound')
            expect(S.main(args + ['--resume', '--max-steps', '2']) == 0, 'resume exit')
            resumed = read(directory/'run.json')
            expect(resumed['resumptions'][-1]['start_digest'] == first['digest'], 'resume starts at checkpoint digest')
            expect(resumed['macro_words'][:1] == first['macro_words'], 'committed word preserved')
            expect(resumed['trajectory'][1]['digest'] == first['digest'], 'trajectory prefix preserved')
            expect(resumed['cpu_s'] > first['cpu_s'], 'CPU cumulative')
            certificate = read(directory/'certificate.json')
            expect(certificate == resumed['certificate'], 'checkpoint certificate sidecar')
            result = A.check(TABLES[7304], certificate, labels[7304])
            expect(result['passed'], f'resumed certificate: {result}')
            with np.load(directory/'labels.npz', allow_pickle=False) as cache:
                expect(str(cache['digest']) == resumed['digest'], 'saved label digest')
            # Exercise interruption of an unfinished family, not just the
            # max-steps boundary. A matching cache keeps setup below this cap.
            limited_dir = out/'cpu-limited'
            expect(S.main(['run', str(TABLES[7304]), str(limited_dir), '--prefix', PREFIXES[7304],
                           '--labels', str(labels[7304]), '--families', 'M', '--cpu-limit', '20']) == 0,
                   'CPU-limited run exit')
            limited = read(limited_dir/'run.json')
            expect(limited['status'] == 'unknown' and limited['reason'] == 'cpu-limit', 'CPU UNKNOWN')
            expect(limited['steps'] == [] and limited['macro_words'] == [], 'unfinished step discarded')
            expect(limited['digest'] == certs['R05']['start']['digest'], 'CPU rollback digest')
            expect(A.check(TABLES[7304], limited['certificate'], labels[7304])['passed'], 'CPU checkpoint certificate')
            first_dir = out/'forced-first'
            expect(S.main(['run', str(TABLES[7304]), str(first_dir), '--prefix', PREFIXES[7304],
                           '--labels', str(labels[7304]), '--families', 'T,M', '--first', '522:8,451:8,522:15',
                           '--max-steps', '1']) == 0, 'forced first macro exit')
            forced = read(first_dir/'run.json')
            expect(forced['digest'] == certs['R05']['macros'][0]['claims']['digests'][-1], 'forced first digest')
            expect(len(forced['macro_words']) == 1 and forced['steps'][0]['kind'] == 'first', 'first before procedure')

        with Part(7, '7304 clean-pair census'):
            dest = out/'7304-census.json'
            expect(S.main(['census', str(TABLES[7304]), str(dest), '--prefix', PREFIXES[7304],
                           '--labels', str(labels[7304])]) == 0, 'census exit')
            result = read(dest)
            expect(result['exhausted'], 'census exhausted')
            expect(result['pair_count'] == len(result['pairs']), 'census count')
            for row in result['pairs']:
                expect(row['lens_tests']['P2']['item_pairs'] == 7200, 'both P2 orientations exhaust items')
            print(json.dumps(dict(measurement='7304-census', pair_count=result['pair_count'],
                                  lens_tests=[r['lens_tests'] for r in result['pairs']], cpu_s=result['cpu_s']),
                             sort_keys=True), flush=True)
    signal.alarm(0)
    print(json.dumps(dict(passed=True, assertions=ASSERTIONS, parts=REPORT,
                          wall_s=round(time.monotonic()-STARTED, 3)), sort_keys=True), flush=True)
    return 0


if __name__ == '__main__':
    sys.exit(main())
