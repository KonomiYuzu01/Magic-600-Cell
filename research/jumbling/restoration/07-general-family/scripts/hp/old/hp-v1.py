"""Heap peeling: restore a jumbled state with single-letter cap steps found from the state alone (menu I_a).

Step kinds (one letter (c, X); every content inside cap c moves by X, nothing else moves):
- PS: no content straddles cap c and cap c holds no lattice piece; X puts the contents of one frame inside the cap on
  K+ and moves the other contents inside the cap to other frames. Zero collateral.
- PSC: the same at a cap that holds lattice pieces; those leave the lattice (collateral). Taken only when the step
  restores more pieces than it sends off.
Choice: the candidates of largest gain (restored minus collateral) are evaluated one step ahead (the best gain on the
state after them) and the best sum is taken; within the frame's coset of 12 letters the letter is chosen by --letter
(first: the lowest index; look: the best one-step-ahead gain, ties to the lowest index).
Execution: every step is applied on the simulator; the off set after it must equal the exact virtual off set, and
the restored, collateral and moved sets are recorded for the independent replay (verify_hp.py).

usage: hp.py <research/jumbling> <E-chain scripts> <table> <out dir> [--max-steps N] [--look K] [--letter first|look]
       [--save-every K]
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / 'gf'))
import glib  # noqa: E402

J, SC, TABLE, OUT = sys.argv[1:5]
a = sys.argv[5:]
opt = lambda k, d: type(d)(a[a.index(k) + 1]) if k in a else d  # noqa: E731
MAXS, LOOK, SAVE, LETTER = opt('--max-steps', 10 ** 6), opt('--look', 3), opt('--save-every', 10), opt('--letter', 'look')
OUT = Path(OUT)
OUT.mkdir(parents=True, exist_ok=True)
glib.setup(J + '/theory', SC)
import buffer as BU  # noqa: E402
import conjclose as CC  # noqa: E402
import hpcore as H  # noqa: E402

W = glib.World(TABLE, '-')
R, st = W.R, W.st
codes = CC.Codes()
G = H.Geo(W)
t0 = time.time()
hs = H.HState.from_world(W, G)
tab = H.Table(hs)
glib.log(event='table', off=int(len(hs.off)), sec_table=round(time.time() - t0, 1))


def evaluate(tab, c, d):
    """Letter of the coset restoring frame d at c, and its one-step-ahead value: (value, X, state, table).

    Cheap filter (exact occupancy): the letters after which the most caps overlapping cap c hold no lattice piece.
    Among those, the best gain the step newly makes available among the caps it affects (exact rows), then the
    number of affected caps with a positive gain, then the lowest letter index."""
    st0 = tab.st
    row = tab.rows[c]
    lets = G.letters(c, d)
    if LETTER == 'first':
        lets = lets[:1]
    ins = row['inside']
    lat = np.flatnonzero(st0.dom == 0)
    lat_in = lat[G.capm[c][st0.q[lat]]]
    ov = G.overlap(c)
    occ = st0.occ().copy()
    occ[st0.q[lat_in]] = False
    capov = G.capm[ov]
    scored = []
    for X in lets:
        d2, q2 = glib.moved_contents(W, st0.dom[ins], st0.q[ins], W.E(c, X))
        o = occ.copy()
        o[q2[d2 == 0]] = True
        scored.append((int(((capov & o).sum(axis=1) == 0).sum()), X))
    mx = max(f for f, _ in scored)
    tied = [X for f, X in scored if f == mx]
    best = None
    for X in tied[:4]:
        s2 = st0.apply([(c, X)])
        if s2 is None:
            continue
        t2 = tab.after(s2)
        lb, npos = t2.local_best(t2.aff)
        v = (lb if lb is not None else -10 ** 9, npos, mx)
        if best is None or v > best[0]:
            best = (v, X, s2, t2)
    if best is None:
        return None
    return (best[0][0], best[1], best[2], best[3], {'free': mx, 'tied': len(tied), 'npos': best[0][1]})


state = {'codes': None, 'digest': None, 'step': 0, 'steps': []}


def execute(word, s_before, s_after, kind, cap, gain):
    t1 = time.time()
    c_s = state['codes'] if state['codes'] is not None else codes.of(st)
    d_in = state['digest'] if state['digest'] is not None else st.digest()
    for c, i in word:
        R.apply(int(c), int(i))
    c_e = codes.of(st)
    d_out = st.digest()
    off_s, off_e = c_s < 0, c_e < 0
    restored = np.flatnonzero(off_s & ~off_e)
    collateral = np.flatnonzero(~off_s & off_e)
    moved = np.flatnonzero(off_s & off_e & (c_s != c_e))
    virt = np.flatnonzero(s_after.dom != 0)
    state['step'] += 1
    k = state['step']
    row = {'step': k, 'kind': kind, 'cap': int(cap), 'word': [[int(c), int(i)] for c, i in word], 'gain': int(gain),
           'off_before': int(off_s.sum()), 'off_after': int(off_e.sum()), 'restored': int(len(restored)),
           'collateral': int(len(collateral)), 'moved_off': int(len(moved)),
           'virtual_matches': bool(np.array_equal(virt, np.flatnonzero(off_e))), 'sec': round(time.time() - t1, 1)}
    row['ok'] = row['virtual_matches'] and row['off_after'] < row['off_before']
    (OUT / f'step{k:03d}-word.json').write_text(json.dumps(
        {'word': row['word'], 'input_digest': d_in, 'output_digest': d_out, 'kind': kind, 'cap': int(cap)}) + '\n')
    (OUT / f'step{k:03d}-sets.json').write_text(json.dumps(
        {'restored': restored.tolist(), 'collateral': collateral.tolist(), 'moved': moved.tolist()}) + '\n')
    with open(OUT / 'run.jsonl', 'a') as f:
        f.write(json.dumps(row) + '\n')
    glib.log(event='step', **{x: y for x, y in row.items() if x != 'word'})
    if not row['ok']:
        raise SystemExit('step check failed')
    state['codes'], state['digest'] = c_e, d_out
    state['steps'].append({'label': f'{kind}{k:03d}-cap{cap}', 'word': f'step{k:03d}-word.json',
                           'sets': f'step{k:03d}-sets.json'})
    if k % SAVE == 0:
        BU.export_pose_table(st, str(OUT / f'step{k:03d}-table.json'))


def write_chain(start_digest):
    (OUT / 'chain.json').write_text(json.dumps({'name': OUT.name, 'start': str(Path(TABLE).resolve()),
                                                'start_digest': start_digest, 'steps': state['steps']},
                                               indent=1) + '\n')


start_digest = st.digest()
state['digest'] = start_digest
while state['step'] < MAXS and len(tab.st.off):
    t1 = time.time()
    cands = [x for x in tab.candidates() if x[0] > 0]
    if not cands:
        glib.log(event='stuck', off=int(len(tab.st.off)), best=tab.candidates()[:3])
        break
    # one candidate per cap (its best frame), the LOOK best by gain
    seen, top = set(), []
    for x in cands:
        if x[1] in seen:
            continue
        seen.add(x[1])
        top.append(x)
        if len(top) >= LOOK:
            break
    best = None
    for gain, c, d, n, occ in top:
        ev = evaluate(tab, c, d)
        if ev is None:
            continue
        v = gain + max(ev[0], 0)
        glib.log(event='cand', cap=c, frame=d, n=n, occ=occ, gain=gain, letter=ev[1], ahead=ev[0], **ev[4])
        if best is None or v > best[0]:
            best = (v, gain, c, d, occ, ev)
    if best is None:
        glib.log(event='stuck_dead', off=int(len(tab.st.off)))
        break
    v, gain, c, d, occ, (ahead, X, s2, t2, info) = best
    execute([(c, X)], tab.st, s2, 'PS' if occ == 0 else 'PSC', c, gain)
    tab = t2
    write_chain(start_digest)
    glib.log(event='chosen', cap=c, frame=d, letter=X, gain=gain, ahead=ahead, off=int(len(tab.st.off)),
             sec_step=round(time.time() - t1, 1))
BU.export_pose_table(st, str(OUT / 'end-table.json'))
write_chain(start_digest)
glib.log(event='end', steps=state['step'], off=int(len(tab.st.off)))
