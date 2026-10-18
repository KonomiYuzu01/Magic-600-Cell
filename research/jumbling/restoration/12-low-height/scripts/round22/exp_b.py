# Round 22, line (c), adjudicating experiment B: the template A.T1'.A^-1 at the depth 60 stop for a unit of the
# blocking cycle {130, 180}. A is the unit's first readiness letter of E-B4 (runs/eb4-k60.json, stage2_at_Z); at
# Y = A(Z) the class-cell pure twist W (cellpure4.py --search), then T1' at Y: the closure V'.W^-1.V'^-1 of the
# displaced content Q at Z1 = W(Y) (prep_t1prime.py, tclose2.py); the step at Z is A.W.V'.W^-1.V'^-1.A^-1, replayed
# from Z against the strict' contract (stepchk.py). Exact; headless; research only.
# Usage (from the repository root): python3 -I -B exp_b.py <Z table> <unit> <grip> <item> <work dir>
import json
import subprocess
import sys
from pathlib import Path

import numpy as np

REPO = Path.cwd()
J = REPO / 'research/jumbling'
TH = J / 'theory'
GM = J / 'restoration/09-general-method/scripts/gm'
EC = J / 'restoration/09-general-method/scripts/echain'
L = J / 'restoration/12-low-height/scripts'
HERE = Path(__file__).resolve().parent
sys.path[:0] = [str(TH), str(J), str(J / 'fixtures'), str(EC)]
import buffer as B  # noqa: E402
import blocks as BL  # noqa: E402
import conjprep as CP  # noqa: E402
import rim  # noqa: E402

O = B.O
PY = [sys.executable, '-I', '-B']


def main():
    ztable, u, g, i, wd = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4]), Path(sys.argv[5])
    wd.mkdir(parents=True, exist_ok=True)
    R, M = O.start(ztable)
    st = R.st
    z = st.digest()
    bl = BL.Blocks(R, M)
    dom = np.asarray(rim.Domains(st).dom)
    units = []
    for pieces in bl.blocks:
        pieces = np.asarray(pieces)
        for d in np.unique(dom[pieces]).tolist():
            units.append((sorted(int(p) for p in pieces[dom[pieces] == d]),
                          sorted(int(p) for p in pieces[dom[pieces] != d])))
    T, F = units[u]
    assert not F, 'the unit has other frames in its block; not handled here'
    (wd / 'unit.json').write_text(json.dumps({'pieces': T}) + '\n')
    R.apply(g, i)
    B.export_pose_table(st, wd / 'Y-table.json')
    R.undo()
    assert st.digest() == z
    res = {'unit': u, 'letter': [g, i], 'size': len(T), 'z_digest': z}
    with open(wd / 'cp4.log', 'w') as f:
        rc = subprocess.run(PY + [str(L / 'cellpure4.py'), str(TH), str(wd / 'Y-table.json'), str(wd / 'unit.json'),
                                  str(wd / 'cp4.json'), '--search'], stdout=f, stderr=subprocess.STDOUT).returncode
    res['cp4_rc'] = rc
    wf = wd / 'cp4-word.json'
    if rc != 0 or not wf.exists():
        res['result'] = 'no W at Y'
        print(json.dumps(res))
        return
    Wd = json.loads(wf.read_text())
    res['W'] = len(Wd['word'])
    with open(wd / 'prep.out', 'w') as f:
        rc = subprocess.run(PY + [str(HERE / 'prep_t1prime.py'), str(wd / 'Y-table.json'), str(wf), 'u', str(wd)],
                            stdout=f, stderr=subprocess.STDOUT).returncode
    res['prep_rc'] = rc
    if rc != 0:
        res['result'] = 'prep failed'
        print(json.dumps(res))
        return
    with open(wd / 'c2.log', 'w') as f:
        rc = subprocess.run(PY[:2] + [str(GM / 'tclose2.py'), str(TH), str(EC), str(wd / 'u-z1-table.json'),
                                      str(wd / 'u-Q.json'), str(wd / 'u-winv.json'), str(wd / 'c2'),
                                      '--greedy-seconds', '300'], stdout=f, stderr=subprocess.STDOUT).returncode
    res['tclose2_rc'] = rc
    rr = json.loads((wd / 'c2' / 'result.json').read_text()) if (wd / 'c2' / 'result.json').exists() else {}
    res['tclose2_result'] = rr.get('result', 'closed' if rr.get('end') else None)
    full = wd / 'c2' / 'word-full.json'
    if not full.exists():
        res['result'] = 'no closure of W^-1 at Z1'
        print(json.dumps(res))
        return
    C = json.loads(full.read_text())['word']
    A = [(g, i)]
    word = [list(x) for x in A] + Wd['word'] + C + [list(x) for x in CP.inverse_word(M, A)]
    (wd / 'step-word.json').write_text(json.dumps({'word': word, 'input_digest': z}) + '\n')
    with open(wd / 'stepchk.out', 'w') as f:
        rc = subprocess.run(PY + [str(HERE / 'stepchk.py'), ztable, str(wd / 'unit.json'), str(wd / 'step-word.json'),
                                  str(wd / 'step.json')], stdout=f, stderr=subprocess.STDOUT).returncode
    res['stepchk_rc'] = rc
    res['step'] = json.loads((wd / 'step.json').read_text()) if (wd / 'step.json').exists() else None
    res['result'] = 'strict-prime step' if res['step'] and res['step']['strict_prime'] else 'step fails'
    print(json.dumps(res))


if __name__ == '__main__':
    main()
