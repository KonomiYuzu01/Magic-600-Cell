# E-B4, the closures: for each unit U whose first passing retained letter A = (g, i) of eb4.py passes N1 and N2 at
# Y = A(Z), build the toolkit's closure C at Y with the steps of the driver's pure-twist step: the class-cell pure
# twist W of A(U)'s edge cell (cellpure4.py --search, a copy of the driver's cellpure3.py that finds the unit's content
# by its pieces, since a frame domain may have contents in several cells at a deep state), then V W V^-1 by tclose2.py,
# and by tclose2m3.py on 'no V'. The step at Z is A C A^-1 (M1-18 with r = 1). Every step is replayed at Z on a fresh
# simulator, every twist checked admissible, against the strict' contract (R21F-01): U must end on the lattice, no
# lattice piece of Z may leave it (collateral 0), and every other off piece of Z must end on the lattice or at its
# exact pose. The closure protects the exact poses of the off pieces outside U's free set (the other frames of U's
# block, which move with U); the free set is checked the same way at the end.
# Usage (from the repository root):
#   python3 -I -B eb4_close.py <Z pose table json> <eb4 out json> <work dir> [--par K] [--cp3-seconds N]
#       [--close-seconds N] [--greedy-seconds N] [--units k,k,...]
import json
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

import numpy as np

REPO = Path.cwd()
HERE = Path(__file__).resolve().parent
J = REPO / 'research/jumbling'
TH = J / 'theory'
GM = J / 'restoration/09-general-method/scripts/gm'
EC = J / 'restoration/09-general-method/scripts/echain'
sys.path[:0] = [str(TH), str(J), str(J / 'fixtures'), str(EC)]
import buffer as B  # noqa: E402
import blocks as BL  # noqa: E402
import conjprep as CP  # noqa: E402
import rim  # noqa: E402

O = B.O
PY = [sys.executable, '-I', '-B']
T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)


def opt(a, name, default):
    return a[a.index(name) + 1] if name in a else default


def run(cmd, logf, timeout):
    with open(logf, 'w') as f:
        try:
            return subprocess.run(cmd, stdout=f, stderr=subprocess.STDOUT, timeout=timeout).returncode
        except subprocess.TimeoutExpired:
            return 'timeout'


def main():
    ztable, eb4out, work = sys.argv[1], sys.argv[2], Path(sys.argv[3])
    a = sys.argv[4:]
    par, cp3s, cls_s = int(opt(a, '--par', 2)), int(opt(a, '--cp3-seconds', 1800)), int(opt(a, '--close-seconds', 3600))
    gsec = opt(a, '--greedy-seconds', '600')
    only = {int(x) for x in opt(a, '--units', '').split(',') if x}
    work.mkdir(parents=True, exist_ok=True)
    doc = json.loads(Path(eb4out).read_text())
    R, M = O.start(ztable)
    st = R.st
    assert st.digest() == doc['digest'], 'the pose table is not the state of the eb4 run'
    bl = BL.Blocks(R, M)
    D = rim.Domains(st)
    dom = np.asarray(D.dom)
    unit_pieces = {}
    k = 0
    for b, pieces in enumerate(bl.blocks):
        pieces = np.asarray(pieces)
        for d in np.unique(dom[pieces]).tolist():
            unit_pieces[k] = (sorted(int(p) for p in pieces[dom[pieces] == d]),
                              sorted(int(p) for p in pieces[dom[pieces] != d]))
            k += 1
    picks = [r for r in doc['stage2_at_Z'] if r.get('N1_N2') and r['kind'] == 'edge'
             and (not only or r['unit'] in only)]
    log(picks=len(picks), units=[r['unit'] for r in picks])
    jobs = []
    for r in picks:
        u, (g, i) = r['unit'], r['letter']
        wd = work / f'u{u:03d}'
        wd.mkdir(exist_ok=True)
        R.apply(g, i)
        B.export_pose_table(st, wd / 'Y-table.json')
        R.undo()
        T, F = unit_pieces[u]
        (wd / 'unit.json').write_text(json.dumps({'pieces': T}) + '\n')
        if F:
            (wd / 'free.json').write_text(json.dumps({'pieces': F}) + '\n')
        jobs.append((r, wd, T, F))
    assert st.digest() == doc['digest']

    def job(r, wd, T, F):
        res = {'unit': r['unit'], 'letter': r['letter'], 'size': len(T), 'free': len(F)}
        t = time.time()
        rc = run(PY + [str(HERE / 'cellpure4.py'), str(TH), str(wd / 'Y-table.json'), str(wd / 'unit.json'),
                       str(wd / 'cp3.json'), '--search'], wd / 'cp3.log', cp3s)
        res['cp3'] = {'rc': rc, 'seconds': round(time.time() - t)}
        wf = wd / 'cp3-word.json'
        if rc != 0 or not wf.exists():
            return res
        Wd = json.loads(wf.read_text())['word']
        res['cp3']['W'] = len(Wd)
        (wd / 'w.json').write_text(json.dumps({'P': [], 'Q': Wd}) + '\n')
        for script, tag in (('tclose2.py', 'c2'), ('tclose2m3.py', 'c3')):
            d = wd / tag
            cmd = PY + [str(GM / script), str(TH), str(EC), str(wd / 'Y-table.json'), str(wd / 'unit.json'),
                        str(wd / 'w.json'), str(d), '--greedy-seconds', gsec]
            if F and script == 'tclose2.py':
                cmd += ['--free', str(wd / 'free.json')]
            t = time.time()
            rc = run(cmd, str(d) + '.log', cls_s)
            rr = json.loads((d / 'result.json').read_text()) if rc == 0 and (d / 'result.json').exists() else {}
            end = rr.get('end')
            ok = bool(end and end['T_restored'] == len(T) and end['collateral'] == 0
                      and end['protected_exact_pose_changed'] == 0 and (d / 'word-full.json').exists())
            res[tag] = {'rc': rc, 'result': rr.get('result', 'closed' if end else None), 'ok': ok,
                        'length': rr.get('length'), 'seconds': round(time.time() - t)}
            if ok:
                res['closure'] = str(d.relative_to(work))
                return res
            if rr.get('result') != 'no V':
                break
        return res

    with ThreadPoolExecutor(max_workers=par) as ex:
        futs = [ex.submit(job, *j) for j in jobs]
        results = []
        for f in futs:
            results.append(f.result())
            log(result=results[-1])
    # the steps A C A^-1 at Z, replayed on a fresh simulator from the Z table
    steps = []
    for res, (r, wd, T, F) in zip(results, jobs):
        if 'closure' not in res:
            continue
        C = [tuple(x) for x in json.loads((work / res['closure'] / 'word-full.json').read_text())['word']]
        A = [tuple(r['letter'])]
        word = A + C + [tuple(x) for x in CP.inverse_word(M, A)]
        R2, _ = O.start(ztable)
        s2 = R2.st
        off0 = B.off_pieces(s2)
        pose0 = {int(p): B.matrix_json(s2._pose_matrix(int(s2.pose_id[p]))) for p in off0.tolist()}
        ok_adm = True
        try:
            for c, i in word:
                R2.apply(c, i)
        except RuntimeError as exc:
            ok_adm = str(exc)[:200]
        off1 = set(B.off_pieces(s2).tolist())
        prot = [p for p in off0.tolist() if p not in set(T) | set(F)]

        def exact(p):
            return p in off1 and B.matrix_json(s2._pose_matrix(int(s2.pose_id[p]))) == pose0[p]
        row = {'unit': r['unit'], 'letter': r['letter'], 'length': len(word), 'all_admissible': ok_adm is True,
               'U_on_lattice': sum(p not in off1 for p in T), 'U_size': len(T),
               'collateral': len(off1 - set(off0.tolist())),
               'protected': len(prot), 'protected_exact': sum(exact(p) for p in prot),
               'protected_landed': sum(p not in off1 for p in prot),
               'free': len(F), 'free_on_lattice': sum(p not in off1 for p in F), 'free_exact': sum(exact(p) for p in F),
               'off_before': len(off0), 'off_after': len(off1), 'end_digest': s2.digest()}
        # a strict' step (R21F-01): U lands, no lattice piece leaves, every other off piece lands or keeps its pose
        row['clean_step'] = bool(row['all_admissible'] and row['U_on_lattice'] == len(T) and row['collateral'] == 0
                                 and row['protected_exact'] + row['protected_landed'] == len(prot)
                                 and row['free_exact'] + row['free_on_lattice'] == len(F))
        (wd / 'step-word.json').write_text(json.dumps({'word': [list(x) for x in word], 'input_digest': doc['digest'],
                                                       'output_digest': s2.digest()}) + '\n')
        steps.append(row)
        log(step=row)
    out = {'digest': doc['digest'], 'picks': len(picks), 'results': results, 'steps': steps,
           'clean_steps': sum(s['clean_step'] for s in steps), 'seconds': round(time.time() - T0, 1)}
    (work / 'close.json').write_text(json.dumps(out, indent=1) + '\n')
    print(json.dumps({k: v for k, v in out.items() if k not in ('results', 'steps')}))


if __name__ == '__main__':
    main()
