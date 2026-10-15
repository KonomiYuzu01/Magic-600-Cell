"""Audit 'shrink': positive control for the own ghost (myghost.ghost) against ghostlib.Ctx.bad on words that do
straddle, and a parse of the report's written-out P4 against the recomputed P4.

Usage: python3 -I -B control.py <out json>
"""
import json
import re
import sys
from pathlib import Path

sys.dont_write_bytecode = True
S = Path('<work>')
A = S / 'v26/B4/audit/shrink'
TH = S / 'wt-e3/research/jumbling/theory'
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.append(str(S / 'v26'))
sys.path.append(str(A))

import numpy as np  # noqa: E402

import myghost as MG  # noqa: E402
import ghostlib as GL  # noqa: E402


def main():
    out = Path(sys.argv[1])
    E2 = str(S / 'v26/B4/E2-table.json')
    C = GL.Ctx(E2)
    R, M, st = C.R, C.M, C.st
    res = {}
    # report's written P4
    text = (S / 'v26/B4/report/D3-E2-E3.md').read_text()
    seg = text.split('Written out, P4 is', 1)[1].split('(91,7) is x0^-1', 1)[0]
    rep_p4 = [(int(a), int(b)) for a, b in re.findall(r'\((\d+),(\d+)\)', seg)]
    P = [[(237, 13)]]
    for k in range(4):
        P.append(P[k] + [MG.XS[k]] + MG.inv_word(M, P[k]))
    res['report_P4_len'] = len(rep_p4)
    res['report_P4_equals_recomputed'] = rep_p4 == P[4]
    res['inverse_237_13'] = MG.inv_letter(M, 237, 13)
    res['inverse_91_6'] = MG.inv_letter(M, 91, 6)
    res['inverse_154_7'] = MG.inv_letter(M, 154, 7)
    res['inverse_133_1'] = MG.inv_letter(M, 133, 1)
    # find grips straddled after (237,13) at E2
    R.apply(237, 13)
    strad = []
    for c in range(600):
        cl = st.classify(c, need_inside=False, stop_at_straddle=True)
        if cl.straddle is not None:
            strad.append(c)
    R.undo()
    res['straddled_after_R'] = strad
    ctrl = []
    for c in strad[:3]:
        for word in ([(237, 13), (c, 13)], [(237, 13), (c, 13), (237, 47)]):
            badg, g, chb = C.bad(word)
            deadg = np.sort(chb[~g.alive])
            bad, alive, died, n = MG.ghost(M, word)
            ctrl.append({'word': [list(x) for x in word], 'ghostlib_bad': int(len(badg)),
                         'own_bad': int(bad.sum()), 'ghostlib_dead': int(len(deadg)), 'own_dead': int((~alive).sum()),
                         'bad_equal': bool(np.array_equal(np.sort(badg), np.flatnonzero(bad))),
                         'dead_equal': bool(np.array_equal(deadg, np.flatnonzero(~alive)))})
    res['control'] = ctrl
    out.write_text(json.dumps(res) + '\n')
    print(json.dumps(res))


if __name__ == '__main__':
    main()
