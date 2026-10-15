"""Skeptic check of finding F1 (report-vs-data): which parked pieces C does Q2 select?

Independent of window-E1.json and of the other auditor's script: start at E1-table.json (not after-V-table.json),
replay V2 and P2 from the part files with digest checks at both ends, then step through Q2 and P2^-1. Selection at a
twist is measured by exact pose comparison over all 177,120 pieces (canonical exact keys of the simulator's pose
table: a K+ index or the exact matrix), cross-checked against the simulator's own moved count in its journal.
C = pieces on the lattice at E1 that are off after P2 (the parked pieces). T = off at E1 minus B'.
Read-only on every input. Output: JSON on stdout.
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
S = Path('<work>')
TH = S / 'wt-e3/research/jumbling/theory'
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
import blocks as BL  # noqa: E402

O = BL.O
C0 = S / 'v26/E2/c0'
T0 = time.time()
BP = np.array([16245, 16249, 32088])


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


def part(name):
    d = json.loads((C0 / 'parts' / f'{name}.json').read_text())
    return [(int(c), int(i)) for c, i in d['word']], d['input_digest'], d['output_digest']


CODE = {}


def codes(st):
    """Canonical exact-pose code per piece (equal code <=> equal exact matrix)."""
    per = np.array([CODE.setdefault(p.key, len(CODE)) for p in st._poses], np.int64)
    return per[st.pose_id]


def lattice(st):
    return st._kpose()[st.pose_id] >= 0


def main():
    R, M = O.start(str(C0 / 'E1-table.json'))
    st = R.st
    out = {'E1_digest': st.digest()}
    L0 = lattice(st)
    off0 = np.flatnonzero(~L0)
    T = np.setdiff1d(off0, BP)
    pre = json.loads((C0 / 'pre.json').read_text())
    out['E1_off'] = int(len(off0))
    out['T_size'] = int(len(T))
    out['T_equals_pre_target_block'] = bool(np.array_equal(T, np.sort(np.array(pre['target_block']))))
    out['Bp_subset_of_off_E1'] = bool(np.isin(BP, off0).all())
    log(stage='E1 loaded', digest=out['E1_digest'][:16])

    V, vin, vout = part('V')
    assert st.digest() == vin, 'E1 is not V input'
    for c, i in V:
        R.apply(c, i)
    out['after_V_digest_matches_part'] = st.digest() == vout
    out['lattice_after_V_equals_E1'] = bool(np.array_equal(lattice(st), L0))
    log(stage='after V', ok=out['after_V_digest_matches_part'])

    P, pin, pout = part('P')
    Q, qin, qout = part('Q')
    Pinv, iin, iout = part('Pinv')
    assert st.digest() == pin
    rows = {'P': [], 'Q': [], 'Pinv': []}
    cP = None
    sets = {'P': [], 'Q': [], 'Pinv': []}
    Cmask = None
    for name, w, wout in (('P', P, pout), ('Q', Q, qout), ('Pinv', Pinv, iout)):
        if name == 'Q':
            # define C after P: lattice at E1 and off now
            Cmask = L0 & ~lattice(st)
            cP = codes(st)
            for nm in ('P',):
                for r, sel in zip(rows[nm], sets[nm]):
                    r['C'] = int(Cmask[sel].sum())
        for j, (c, i) in enumerate(w):
            b = codes(st)
            R.apply(c, i)
            a = codes(st)
            sel = np.flatnonzero(a != b)
            jm = int(st.journal[-1]['moved'])
            r = {'j': j, 'letter': [c, i], 'pose_changed': int(len(sel)), 'journal_moved': jm,
                 'T': int(np.isin(sel, T).sum()), 'Bp': int(np.isin(sel, BP).sum())}
            if Cmask is not None:
                r['C'] = int(Cmask[sel].sum())
            rows[name].append(r)
            sets[name].append(sel)
        out[f'after_{name}_digest_matches_part'] = st.digest() == wout
        if name == 'Q':
            cQ = codes(st)
            net = np.flatnonzero(cQ != cP)
            out['Q_net_moved'] = int(len(net))
            out['Q_net_moved_equals_T'] = bool(np.array_equal(net, T))
            out['Q_C_all_back_exact'] = bool((cQ[Cmask] == cP[Cmask]).all())
        log(stage=f'after {name}', ok=out[f'after_{name}_digest_matches_part'])

    Cidx = np.flatnonzero(Cmask)
    out['C_size'] = int(len(Cidx))
    qC = [np.intersect1d(s, Cidx) for s in sets['Q']]
    union_Q = np.unique(np.concatenate(qC)) if qC else np.zeros(0, np.int64)
    out['Q_letters_selecting_C'] = [j for j, s in enumerate(qC) if len(s)]
    out['Q_union_C_selected'] = int(len(union_Q))
    nz = [s for s in qC if len(s)]
    out['Q_C_sets_identical_across_selecting_letters'] = bool(all(np.array_equal(nz[0], s) for s in nz))
    out['Q_C_never_selected'] = int(len(Cidx) - len(union_Q))
    pC = [np.intersect1d(s, Cidx) for s in sets['Pinv']]
    out['Pinv_letters_selecting_all_C'] = [j for j, s in enumerate(pC) if len(s) == len(Cidx)]
    out['Pinv_union_C_selected'] = int(len(np.unique(np.concatenate(pC))))
    out['Q_union_C_inside_Pinv_full_letters'] = bool(all(np.isin(union_Q, pC[j]).all()
                                                         for j in out['Pinv_letters_selecting_all_C']))
    out['pose_change_equals_journal_moved_everywhere'] = all(r['pose_changed'] == r['journal_moved']
                                                             for v in rows.values() for r in v)
    out['rows'] = rows
    out['Q_union_C_first10'] = union_Q[:10].tolist()
    out['seconds'] = round(time.time() - T0, 1)
    print(json.dumps(out, indent=1))


if __name__ == '__main__':
    main()
