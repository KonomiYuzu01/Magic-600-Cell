"""Stage 16 packet acceptance gate: every G0--G7 assertion is mandatory.

One thread, sequential searches, external TemporaryDirectory scratch only.
A wall alarm fails the gate at 3600 seconds. The measurements concern exact
synthetic geometry, not native-host or performance evidence.

Run: python3 -I -B research/jumbling/restoration/16-six-jams/scripts/test_six_jams.py
"""
from contextlib import redirect_stdout
import hashlib
import io
import json
import os
from pathlib import Path
import resource
import signal
import subprocess
import sys
import tempfile
import time
from unittest.mock import patch

sys.dont_write_bytecode = True
for _var in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ[_var] = '1'


def expired(signum, frame):
    raise AssertionError('acceptance timeout: 3600 wall seconds')


signal.signal(signal.SIGALRM, expired)
signal.alarm(3600)
SCRIPTS = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPTS))
import common as C  # noqa: E402
import transport as X  # noqa: E402
import release as R  # noqa: E402
import t1 as T1  # noqa: E402
import pxrun as PX  # noqa: E402
import routing as RR  # noqa: E402

np, A = C.np, C.A
ASSERTIONS = 0


def expect(ok, detail):
    global ASSERTIONS
    ASSERTIONS += 1
    if not ok:
        raise AssertionError(detail)


def cpu_total():
    children = resource.getrusage(resource.RUSAGE_CHILDREN)
    return time.process_time() + children.ru_utime + children.ru_stime


class Row:
    def __init__(self, name):
        self.name = name

    def __enter__(self):
        self.cpu = cpu_total()

    def __exit__(self, kind, value, tb):
        print(f'{self.name}: {"FAIL" if kind else "PASS"} cpu_s={cpu_total()-self.cpu:.3f}', flush=True)


def read(path):
    return json.loads(Path(path).read_text())


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def counters(counts):
    return {k: v for k, v in counts.items() if k not in ('K', 'minimal')}


def fields(result, expected, name):
    for key, value in expected.items():
        expect(result[key] == value, f'{name} {key}: {result[key]!r} != {value!r}')


def metadata(result, kind, seed):
    fields(result, dict(format=f's16-{kind}/1', table_digest=C.jam(seed)['table_digest'],
                        digest=C.jam(seed)['end_digest'], phi=C.jam(seed)['end_phi'], word=''), kind)
    expect(isinstance(result['options'], dict), f'{kind} options recorded')
    expect(isinstance(result['cpu_s'], (int, float)) and result['cpu_s'] >= 0, f'{kind} CPU recorded')


G1_COUNTS = dict(nodes=1, letters=418, leaf_tests=1257, hits=36,
                 skipped_same=0, skipped_commute=0, skipped_minimal=0)
G1_BEST = [
    dict(word=[[597, 8], [590, 14]], terminal_phi=494, F=0, D=2, target=590),
    dict(word=[[597, 9], [523, 14]], terminal_phi=494, F=0, D=2, target=523),
    dict(word=[[597, 11], [591, 14]], terminal_phi=494, F=0, D=2, target=591),
]
G2_COUNTS = dict(nodes=419, letters=94985, leaf_tests=369040, hits=348,
                 skipped_same=4598, skipped_commute=75020, skipped_minimal=0)
G2_BEST = [
    dict(word=[[382, 1], [516, 9], [494, 13]], terminal_phi=1206, F=0, D=7, target=494),
    dict(word=[[448, 1], [516, 2], [543, 12]], terminal_phi=1206, F=0, D=7, target=543),
    dict(word=[[516, 2], [543, 11], [448, 15]], terminal_phi=1206, F=0, D=7, target=448),
    dict(word=[[516, 9], [494, 2], [536, 15]], terminal_phi=1206, F=0, D=7, target=536),
    dict(word=[[516, 9], [494, 4], [276, 15]], terminal_phi=1206, F=0, D=7, target=276),
    dict(word=[[516, 9], [494, 10], [403, 12]], terminal_phi=1206, F=0, D=7, target=403),
    dict(word=[[516, 9], [494, 11], [382, 15]], terminal_phi=1206, F=0, D=7, target=382),
]
G3_PRIZE = dict(r=41, m=1, x=2, F=0, D=309, value=618, S_r=132)
# The complete twenty printed records, compressed only by their identical
# prefix and the three explicitly listed (helper item, prize item) groups.
G3_PREFIX = [[23, 10], [20, 2], [112, 2], [23, 11], [112, 2], [23, 6]]
G3_HITS = [dict(word=G3_PREFIX + [[20, a], [23, 7], [41, m]], x=2, F=0, D=309, terminal_phi=554)
           for a, ms in ((1, (1, 2, 3, 4, 5, 6, 8, 9)), (2, (1, 2, 3, 4, 5, 6, 8, 9)),
                         (3, (1, 2, 3, 4))) for m in ms]
G4_PER_LEAF = [dict(leaf=[23, h], a=394, lead=[394, 148 if h in (17, 24, 48, 54) else 112, 12, 0, 0])
               for h in range(12, 60)]
G4B_PER_LEAF = [dict(leaf=[543, h], a=222, lead=[222, 448, 12, 0, 0]) for h in range(12, 60)]
for _h, _g in ((16, 15), (34, 12), (51, 14), (56, 13)):
    G4B_PER_LEAF[_h-12].update(lead=[0, 448, _g, 0, 111], **{'return': [12, 0, 0, 0]})
G6_SEQS = [
    [1, 6, 9], [2, 5, 8], [2, 10, 1], [4, 10, 1], [5, 2, 8], [5, 11, 8], [6, 1, 9],
    [6, 7, 2], [6, 7, 9], [7, 6, 2], [7, 6, 9], [7, 10, 2], [8, 10, 1], [9, 10, 1],
    [10, 2, 1], [10, 4, 1], [10, 7, 2], [10, 8, 1], [10, 9, 1], [10, 11, 1], [11, 5, 8], [11, 10, 1],
]
G7_BEST = [
    dict(value=[0, 0, 0], tail=[[448, 3], [597, 2]], word='543:1,494:9,543:8'),
    dict(value=[0, 0, 0], tail=[[448, 3], [597, 1]], word='543:1,494:9,516:5,543:8'),
    dict(value=[0, 0, 0], tail=[[448, 3], [597, 3]], word='543:1,494:9,516:8,543:8'),
    dict(value=[0, 0, 0], tail=[[448, 3], [597, 2]], word='543:1,516:4,494:3,543:8'),
    dict(value=[0, 0, 0], tail=[[448, 3], [597, 2]], word='543:1,516:4,494:8,543:8'),
]


def px_cli(args):
    return subprocess.run([sys.executable, '-I', '-B', str(SCRIPTS/'pxrun.py'), *map(str, args)],
                          capture_output=True, text=True, check=False)


def main():
    with tempfile.TemporaryDirectory(prefix='six-jams-gate-') as scratch:
        out = Path(scratch)
        cache04, cache07 = out/'jam7304.npz', out/'jam7307.npz'
        with Row('G0 loader'):
            state04 = C.load(7304, save_labels=cache04)
            expect(state04.st.digest() == C.jam(7304)['end_digest'], '7304 loader end digest')
            expect(C.J.phi(state04.h) == 1172, '7304 loader end Phi')
            mismatched = out/'mismatched.npz'
            C.write_labels(mismatched, state04.lab, '0'*64)
            before = sha(mismatched)
            with patch.object(C.BM, 'all_labels', wraps=C.BM.all_labels) as compute:
                missed = C.load(7304, labels=mismatched)
                expect(compute.call_count == 1, 'mismatching digest computes labels')
                expect(compute.call_args.kwargs == dict(workers=1), 'single-worker exact labels')
            expect(sha(mismatched) == before, 'mismatching cache byte-identical')
            expect(np.array_equal(missed.lab, state04.lab), 'cache miss has computed labels')
            del missed
            before = sha(cache04)
            with patch.object(C.BM, 'load_table', side_effect=AssertionError('loaded before refusal')):
                try:
                    C.load(7304, save_labels=cache04)
                except A.Failure as exc:
                    expect(exc.test == 'labels exists', 'existing save_labels named refusal')
                else:
                    expect(False, 'existing save_labels must raise')
            expect(sha(cache04) == before, 'existing save_labels byte-identical')
            with patch.object(C.BM, 'all_labels', side_effect=AssertionError('matching cache not used')):
                matched = C.load(7304, labels=cache04)
            expect(np.array_equal(matched.lab, state04.lab), 'matching cache equals computed labels')
            expect(sha(cache04) == before, 'matching cache byte-identical')
            del matched

        with Row('G1 transport K=1 7302'):
            dest = out/'transport-7302.json'
            with redirect_stdout(io.StringIO()):
                expect(X.main(['7302', '1', str(dest)]) == 0, 'G1 CLI exit')
            result = read(dest)
            metadata(result, 'transport', 7302)
            fields(result, dict(K=1, minimal=False, extra_prefix='', complete=True,
                                counts=G1_COUNTS, best_per_target=G1_BEST), 'G1')

        with Row('G2 transport K=2 7307 off/on'):
            state07 = C.load(7307, save_labels=cache07)
            digest = state07.st.digest()
            rows_off, counts_off = X.search(state07, 2)
            rows_on, counts_on = X.search(state07, 2, minimal=True)
            expect(counters(counts_off) == G2_COUNTS, 'G2 off counters')
            expect(counters(counts_on) == dict(G2_COUNTS, leaf_tests=7085), 'G2 on counters')
            fields(counts_off, dict(K=2, minimal=False), 'G2 off function bookkeeping')
            fields(counts_on, dict(K=2, minimal=True), 'G2 on function bookkeeping')
            expect(rows_off == rows_on, 'G2 minimal differential')
            expect(rows_off == G2_BEST, 'G2 complete reference best_per_target')
            expect(state07.st.digest() == digest, 'G2 source unchanged')

        with Row('G3 release 7304 r=41'):
            dest = out/'release-7304.json'
            printed = io.StringIO()
            with redirect_stdout(printed):
                expect(R.main(['7304', str(dest), '--labels', str(cache04), '--only-r', '41', '--relevant']) == 0,
                       'G3 CLI exit')
            result = read(dest)
            metadata(result, 'release', 7304)
            expect(printed.getvalue().splitlines()[0].startswith(
                '7304: phi 1172, 40 clean grips, 46 prize letters at 14 blocked grips'), 'G3 prize line')
            fields(result, dict(clean=40, prize_letters=46, prize_grips=14, complete=False, search='beam'), 'G3')
            fields(result['options'], dict(width=2000, depth=10, prizes=6, only_r='41', relevant=True,
                                          enable=False, dedup=False, alphabet=''), 'G3 options')
            expect(len(result['results']) == 1, 'G3 only one prize grip')
            fields(result['results'][0], dict(prize=G3_PRIZE, carriers=[23], helpers=[20, 61, 112, 184],
                                             levels=8, best_score=[0, 0], releases=110, n_hits=808,
                                             hits=G3_HITS), 'G3 r=41')

        with Row('G4 T1 7304 grips=23,20'):
            digest = state04.st.digest()
            result = T1.scan(state04, emax=900, shard=(0, 1), grips=[23, 20], verbose=False)
            fields(result, dict(phi=1172, emax=900, grips=[23, 20], shard=[0, 1], leaves=48, leaves_done=48,
                                leaves_all=48, targets_tested=142, returns=0, complete=True,
                                best_pair=[394, [23, 12], [112, 12], 0, 0], best_return=None,
                                n_hits=0, hits=[], per_leaf=G4_PER_LEAF), 'G4')
            expect(state04.st.digest() == digest, 'G4 digest unchanged after scan')

        with Row('G4b T1 7307 grip=543'):
            digest = state07.st.digest()
            result = T1.scan(state07, emax=900, shard=(0, 1), grips=[543], verbose=False)
            fields(result, dict(phi=1220, emax=900, grips=[543], shard=[0, 1], leaves=48, leaves_done=48,
                                leaves_all=48, targets_tested=151, returns=4, complete=True,
                                best_pair=[0, [543, 16], [448, 15], 0, 111],
                                best_return=[0, [543, 16], [448, 15], [543, 12]],
                                n_hits=0, hits=[], per_leaf=G4B_PER_LEAF), 'G4b')
            expect(state07.st.digest() == digest, 'G4b digest unchanged after scan')

        with Row('G5 pxrun plumbing'):
            directory = out/'pxrun'
            before = sha(cache04)
            proc = px_cli(['7304', directory, '1', '--labels', cache04])
            expect(proc.returncode == 0, f'G5a exit: {proc.stderr}')
            result = read(directory/'pxrun.json')
            metadata(result, 'pxrun', 7304)
            expect((result['status'], result['reason']) == ('unknown', 'cpu-limit')
                   or result['status'] == 'stopped', 'G5a valid limited status')
            expect(sha(cache04) == before, 'G5a supplied cache byte-identical')
            record_before = sha(directory/'pxrun.json')
            with patch.object(C, 'load', side_effect=AssertionError('load before output refusal')):
                try:
                    PX.main(['7304', str(directory), '1', '--labels', str(cache04)])
                except A.Failure as exc:
                    expect(exc.test == 'run exists', 'G5b refuses output before loading')
                else:
                    expect(False, 'G5b existing output must raise')
            expect(sha(directory/'pxrun.json') == record_before, 'G5b existing record byte-identical')
            startdir = out/'pxrun-start'
            proc = px_cli(['7304', startdir, '1', '--labels', cache04, '--start', '23:1'])
            expect(proc.returncode == 0, f'G5c exit: {proc.stderr}')
            result = read(startdir/'pxrun.json')
            expect(result['format'] == 's16-pxrun/1', 'G5c valid JSON format')
            expect(result['start_macros'] == [[[23, 1]]], 'G5c start macro retained')
            expect(result['word'] == '23:1', 'G5c extra word recorded')
            expect(result['status'] in ('unknown', 'stopped'), 'G5c valid limited status')
            expect(sha(cache04) == before, 'G5c mismatched supplied cache byte-identical')
            own = startdir/'start-labels.npz'
            expect(own.is_file(), 'G5c creates output-owned start labels')
            with np.load(own, allow_pickle=False) as data:
                expect(str(data['digest']) == result['digest'], 'G5c start labels match start state')
                expect(str(data['digest']) != C.jam(7304)['end_digest'], 'G5c input cache digest mismatches start')
                expect(data['labels'].shape == state04.lab.shape and data['labels'].dtype == np.uint8
                       and not (data['labels'] > 2).any(), 'G5c valid owned labels')

        with Row('G6 routing anatomy 7304 23->41'):
            dest = out/'anatomy-7304.json'
            with redirect_stdout(io.StringIO()):
                expect(RR.main(['7304', str(dest), 'anatomy', '23', '41', '--labels', str(cache04)]) == 0,
                       'G6 CLI exit')
            result = read(dest)
            metadata(result, 'routing', 7304)
            fields(result, dict(seed=7304, mode='anatomy', c=23, r=41, I_c=2825, S_r=132, out_r=0,
                                B=[132, 9, 67, 48, 25, 49, 53, 25, 43, 14, 77, 17], newclean=[61, 112, 184],
                                seqs={'1': [], '2': [], '3': G6_SEQS}, least={'1': 9, '2': 3}, complete=True), 'G6')

        with Row('G7 routing search 7307 543->597'):
            dest = out/'routing-7307.json'
            with redirect_stdout(io.StringIO()):
                expect(RR.main(['7307', str(dest), 'search', '543', '597', '--nmax', '2', '--fixed', '448:3',
                                '--free', '597', '--target', '517', '--cap', '300', '--labels', str(cache07)]) == 0,
                       'G7 CLI exit')
            result = read(dest)
            metadata(result, 'routing', 7307)
            fields(result, dict(seed=7307, mode='search', c=543, r=597, I_c=2875, S_r=86, out_r=0,
                                B=[86, 100, 150, 65, 32, 54, 26, 60, 65, 125, 47, 121], newclean=[],
                                least={'1': 15, '2': 0}, nmax=2, cap=300.0, fixed=[448, 3], free=597, target=517,
                                search=dict(tested=3245, releases=461, complete=True), releases_listed=461,
                                clearable=249, best=G7_BEST, complete=True), 'G7')
            expect([len(result['seqs'][str(n)]) for n in (1, 2, 3)] == [0, 17, 314], 'G7 sequence counts')
    signal.alarm(0)
    print(f'PASS: {ASSERTIONS} assertions', flush=True)
    return 0


if __name__ == '__main__':
    sys.exit(main())
