"""report-vs-data audit: letters, inverses, K+ membership, word structure, E2 off set and SAFE-ALL menu.

Reads only the E2 pose table and the word files. Usage: python3 -I letters.py <theory dir> <B4 dir> <out json>
"""
import hashlib
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
B4 = Path(sys.argv[2]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
import blocks as BL  # noqa: E402
import buffer as B  # noqa: E402
from sim.kplus import to_tuple, matrix_json  # noqa: E402

T0 = time.time()
O, restore = BL.O, BL.restore
kp = BL.CL.ctx.kplus


def I4(m):
    return all(m[r][q] == (1 if r == q else 0) for r in range(4) for q in range(4))


def main():
    out = {}
    table = B4 / 'E2-table.json'
    out['E2_sha256'] = hashlib.sha256(table.read_bytes()).hexdigest()
    doc = json.loads(table.read_text())
    out['table_digest_field'] = doc.get('digest')
    pid = doc['pose_id']
    out['n_pieces'] = len(pid)
    R, M = O.start(str(table))
    st = R.st
    out['sim_digest'] = st.digest()
    K = st._kpose()[st.pose_id]
    off = np.flatnonzero(K < 0).tolist()
    out['off'] = off
    # B' share one exact pose? (from the raw JSON and from the simulator)
    out['bprime_json_pose_ids'] = [pid[p] for p in off]
    mats = [to_tuple(st.pose(p)) for p in off]
    out['bprime_one_pose_sim'] = len(set(mats)) == 1
    out['bprime_pose_in_Kplus'] = bool(kp.index_of_matrix(mats[0]) >= 0)
    out['bprime_pose_json'] = doc['poses'][pid[off[0]]]
    out['bprime_pose_sim'] = matrix_json(st.pose(off[0]))
    out['off_fingerprint_lib'] = B.off_fingerprint(st)
    # my own fingerprint: sorted off ids with their JSON pose entries
    rec = [[p, doc['poses'][pid[p]]] for p in sorted(off)]
    out['off_fingerprint_json'] = hashlib.sha256(json.dumps(rec, separators=(',', ':')).encode('ascii')).hexdigest()

    nitems = len(M.items)
    out['menu_items'] = nitems

    def inv(c, i):
        e = restore.element(M, c, i)
        hits = [j for j in range(nitems) if I4(restore.matmul(e, restore.element(M, c, j)))]
        return hits

    def inK(c, i):
        return bool(kp.index_of_matrix(to_tuple(restore.element(M, c, i))) >= 0)

    letters = {'R': (237, 13), 'Rinv_claimed': (237, 47), 'x0': (91, 6), 'x0inv_claimed': (91, 7), 'x1': (154, 7),
               'x1inv_claimed': (154, 6), 'x2': (133, 1), 'x3': (183, 6), 'x3inv_in_Pinv': (183, 7), 'F': (162, 1)}
    V = json.loads((B4 / 'c3/parts/V.json').read_text())['word']
    for n, (c, i) in enumerate(V):
        letters[f'V{n}'] = (c, i)
    info = {}
    for name, (c, i) in letters.items():
        info[name] = {'letter': [c, i], 'in_Kplus': inK(c, i), 'inverses': [[c, j] for j in inv(c, i)]}
    out['letters'] = info

    # iterated conjugation from R
    def inverse_word(w):
        res = []
        for c, i in reversed(w):
            h = inv(c, i)
            assert len(h) == 1, (c, i, h)
            res.append((c, h[0]))
        return res
    xs = [(91, 6), (154, 7), (133, 1), (183, 6)]
    P = [(237, 13)]
    levels = [P]
    for x in xs:
        P = P + [x] + inverse_word(P)
        levels.append(P)
    out['P_lengths'] = [len(p) for p in levels]
    P4file = [tuple(x) for x in json.loads((B4 / 'P4.json').read_text())]
    out['P4_equals_P4json'] = levels[4] == P4file
    partsP = [tuple(x) for x in json.loads((B4 / 'c3/parts/P.json').read_text())['word']]
    partsPinv = [tuple(x) for x in json.loads((B4 / 'c3/parts/Pinv.json').read_text())['word']]
    partsV = [tuple(x) for x in V]
    partsVinv = [tuple(x) for x in json.loads((B4 / 'c3/parts/Vinv.json').read_text())['word']]
    partsQ = [tuple(x) for x in json.loads((B4 / 'c3/parts/Q.json').read_text())['word']]
    out['parts_P_equals_P4'] = partsP == levels[4]
    out['parts_Pinv_is_mirror_inverse'] = partsPinv == inverse_word(partsP)
    out['parts_Vinv_is_mirror_inverse'] = partsVinv == inverse_word(partsV)
    out['parts_Q'] = [list(x) for x in partsQ]
    full = [tuple(x) for x in json.loads((B4 / 'c3/word-full.json').read_text())['word']]
    out['word_full_is_concat'] = full == partsV + partsP + partsQ + partsPinv + partsVinv
    out['word_full_len'] = len(full)
    w3 = json.loads((B4 / 'w3-P4-162-1.json').read_text())
    out['w3_P_equals_P4'] = [tuple(x) for x in w3['P']] == levels[4]
    out['w3_Q'] = w3['Q']
    rows = [json.loads(l) for l in open(B4 / 'shrink-237-13.jsonl')]
    out['shrink_P_equal_recursion'] = [[tuple(x) for x in r['P']] == levels[r['level']] for r in rows]
    cs = json.loads((B4 / 'cS-P4-162-1/pre.json').read_text())
    out['typeS_P_equals_P4'] = [tuple(x) for x in cs['P']] == levels[4]
    q0 = [tuple(x) for x in json.loads((B4 / 'cS-P4-162-1/parts/Q0.json').read_text())['word']]
    out['typeS_Q0_len'] = len(q0)
    out['typeS_Q0_equals_V3'] = q0 == partsV
    csfull = [tuple(x) for x in json.loads((B4 / 'cS-P4-162-1/word-full.json').read_text())['word']]
    out['typeS_full_is_P_Q0_F_Pinv'] = csfull == levels[4] + q0 + [(162, 1)] + inverse_word(levels[4])
    out['typeS_len'] = len(csfull)
    # written-out P4 in the report
    rep = (B4 / 'report/D3-E2-E3.md').read_text()
    s = rep.index('Written out, P4 is')
    seg = rep[s:rep.index('(91,7) is x0^-1')]
    import re
    written = [(int(a), int(b)) for a, b in re.findall(r'\((\d+),(\d+)\)', seg)]
    out['report_written_P4_len'] = len(written)
    out['report_written_P4_equals'] = written == levels[4]
    s = rep.index('V3 (8 letters')
    seg = rep[s:rep.index('P4 (31 letters)')]
    wv = [(int(a), int(b)) for a, b in re.findall(r'\((\d+),(\d+)\)', seg)]
    out['report_written_V3_equals'] = wv == partsV
    # SAFE-ALL at E2
    G = B.Alphabet(R, M)
    out['safe_grips'] = len(G.safe)
    out['safe_letters'] = len(G.letters)
    orb, size = G.orbits()
    out['orbits'] = int(len(size))
    out['singletons'] = int((size == 1).sum())
    out['blocked'] = sorted(int(c) for c in G.blocked)
    out['blocked_detail'] = {int(c): v for c, v in G.blocked.items()}
    out['holes'] = [int(h) for h in G.holes]
    lset = {(l[0], l[1]) for l in G.letters}
    out['V3_all_safe_all_letters_at_E2'] = all(tuple(x) in lset for x in partsV)
    out['F_safe_all_at_E2'] = (162, 1) in lset
    out['x_safe_all_at_E2'] = {str(x): (x in lset) for x in xs}
    # compare with pre.json menu
    pre = json.loads((B4 / 'c3/pre.json').read_text())
    out['pre_safe_grips_equal'] = sorted(pre['menu']['safe_grips']) == sorted(int(c) for c in G.safe)
    seg2 = json.loads(Path(sys.argv[4]).read_text()) if len(sys.argv) > 4 else None
    if seg2:
        out['seg2_E2_safe_grips_equal'] = sorted(seg2['safe_all']['grips']) == sorted(int(c) for c in G.safe)
        out['seg2_E2_blocked_equal'] = {int(k): v for k, v in seg2['safe_all']['blocked'].items()} == out['blocked_detail']
    out['holes_orbits'] = {int(h): [int(orb[h]), int(size[orb[h]])] for h in G.holes}
    out['sec'] = round(time.time() - T0, 1)
    Path(sys.argv[3]).write_text(json.dumps(out, indent=1) + '\n')
    print(json.dumps({k: v for k, v in out.items() if k not in ('letters', 'blocked_detail')}, indent=None)[:4000])


if __name__ == '__main__':
    main()
