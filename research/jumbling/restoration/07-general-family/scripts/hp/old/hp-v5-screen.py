"""Heap peeling: restore a jumbled state with single-letter cap steps found from the state alone (menu I_a).

Step kinds (one letter (c, X); every content inside cap c moves by X, nothing else moves):
- PS: no content straddles cap c and cap c holds no lattice piece; X puts the contents of one frame inside the cap on
  K+ and moves the other contents inside the cap to other frames. Zero collateral.
- PSC: the same at a cap that holds lattice pieces; those leave the lattice (collateral). Taken only when the step
  restores more pieces than it sends off.
Choice: with --letter support, candidates (best frame per cap, by gain; at most --screen caps) whose step lowers the
sibling support of the contents it moves and leaves off are set aside (such a step breaks up the cap-shaped group of a
later twist); the LOOK remaining candidates of largest gain (restored minus collateral) are evaluated one step ahead
(the best gain on the state after them) and the best sum is taken; within the frame's coset of 12 letters the letter is chosen by --letter:
  support (default): sibling support (hpcore.Siblings). The contents that stay off are moved to new (domain, chamber);
    each counts the most off contents of its domain outside cap c that lie in one cap (content coordinates) with its
    chamber, and the letter with the largest sum is taken: the contents a later twist moved rejoin the cap-shaped
    group of their own earlier twist. Ties (always when nothing stays off): hole alignment (below), then the most
    lattice-free overlapping caps, then the lowest index.
  align: hole alignment. A chamber z inside cap c weighs w(z) = sum over the caps c' != c that contain z of the holes
    of c' outside cap c; the letter whose restored pieces land on the least total weight is taken.
    Ties: the most caps overlapping cap c with no lattice piece afterwards, then the lowest index.
  look: the most lattice-free overlapping caps, then the best one-step-ahead gain over at most 4 tied letters.
  first: the lowest index.
Carrier steps (only when no single letter has a positive gain): a K+ letter (c1, k) at an unstraddled cap holding off
contents, followed by the best single-letter step it makes available (kinds KPS, KPSC). A K+ letter changes no piece's
on/off status; it rotates the contents of cap c1 about its grip. The 11 non-identity K+ letters of each such cap are
ranked by the gain in sibling support of the contents inside cap c1 (only letters that improve on the identity), the
--carry best are applied and evaluated one step ahead, and the best positive gain is taken.
Excursion steps (when no carrier step is found): a letter with a gain <= 0 followed by the best letter among the caps it
affects, taken as one step when the net gain is positive (kinds XPS, XPSC); the --excursion best caps by gain are
tried. With --carry 0 and --excursion 0 the run stops instead.
Execution: every step is applied on the simulator; the off set after it must equal the exact virtual off set, and
the restored, collateral and moved sets are recorded for the independent replay (verify_hp.py).

usage: hp.py <research/jumbling> <E-chain scripts> <table> <out dir> [--max-steps N] [--look K]
       [--letter support|align|look|first] [--carry N] [--excursion N] [--screen N] [--save-every K]
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
MAXS, LOOK, SAVE, LETTER = opt('--max-steps', 10 ** 6), opt('--look', 3), opt('--save-every', 10), opt('--letter', 'support')
CARRY, EXC, SCREEN = opt('--carry', 8), opt('--excursion', 12), opt('--screen', 40)
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


def evaluate(tab, c, d, X0=None, glob=None):
    """Letter of the coset restoring frame d at c (--letter rule, exact occupancy; X0 forces it), and its one-step-ahead
    value: (best gain among the caps the step affects, exact rows; X; state; table; info)."""
    st0 = tab.st
    row = tab.rows[c]
    lets = G.letters(c, d)
    if LETTER == 'first':
        lets = lets[:1]
    if X0 is not None:
        lets = [X0]
    ins = row['inside']
    lat = np.flatnonzero(st0.dom == 0)
    lat_in = lat[G.capm[c][st0.q[lat]]]
    ov = G.overlap(c)
    occ = st0.occ().copy()
    occ[st0.q[lat_in]] = False
    capov = G.capm[ov]
    ovx = ov[ov != c]
    capx = G.capm[ovx]
    h_out = (capx & ~G.capm[c] & ~st0.occ()).sum(axis=1)  # holes of each overlapping cap outside cap c
    wz = np.zeros(W.NP, np.int64)  # weight of the chambers inside cap c (restored pieces land there)
    wz[G.cham[c]] = h_out @ capx[:, G.cham[c]].astype(np.int64)
    sib = H.Siblings(G, st0, ins, glob) if LETTER == 'support' and X0 is None else None
    scored = []
    for X in lets:
        d2, q2 = glib.moved_contents(W, st0.dom[ins], st0.q[ins], W.E(c, X))
        o = occ.copy()
        o[q2[d2 == 0]] = True
        free = int(((capov & o).sum(axis=1) == 0).sum())
        align = -int(wz[q2[d2 == 0]].sum())
        key = {'support': lambda: (sib.support(d2, q2) if sib else 0, align, free),
               'align': lambda: (align, free)}.get(LETTER, lambda: (free,))()
        scored.append((key, X))
    mx = max(f for f, _ in scored)
    tied = [X for f, X in scored if f == mx]
    ntied = len(tied)
    if LETTER in ('support', 'align'):
        tied = tied[:1]
    best = None
    for X in tied[:4]:
        s2 = st0.apply([(c, X)])
        if s2 is None:
            continue
        t2 = tab.after(s2)
        lb, npos = t2.local_best(t2.aff)
        v = (lb if lb is not None else -10 ** 9, npos)
        if best is None or v > best[0]:
            best = (v, X, s2, t2)
    if best is None:
        return None
    return (best[0][0], best[1], best[2], best[3], {'score': list(mx), 'tied': ntied, 'npos': best[0][1]})


def carrier(tab):
    """Best two-letter step [(c1, k), (c, X)] with a K+ letter k at c1 and a positive gain, or None."""
    st0 = tab.st
    occ = st0.occ()
    ranked = []
    for c1 in range(600):
        row = tab.rows[c1]
        if row['strad'] or not len(row['inside']):
            continue
        ins = row['inside']
        sib = H.Siblings(G, st0, ins)
        base = sib.support(st0.dom[ins], st0.q[ins])
        for k in W.kit(c1):
            d2, q2 = glib.moved_contents(W, st0.dom[ins], st0.q[ins], W.E(c1, k))
            better = sib.support(d2, q2) - base
            if better > 0:
                ranked.append((better, c1, k))
    ranked.sort(key=lambda r: (-r[0], r[1], r[2]))
    glib.log(event='carrier_scan', candidates=len(ranked), top=ranked[:5])
    best = None
    for better, c1, k in ranked[:CARRY]:
        s1 = st0.apply([(c1, k)])
        if s1 is None:
            continue
        t1 = tab.after(s1)
        g = [x for c2 in t1.aff for x in H.gains(t1.rows[c2], G) if x[0] > 0]
        if not g:
            continue
        gain, c, d, n, occ2 = sorted(g, key=lambda x: (-x[0], x[4], x[1], x[2]))[0]
        ev = evaluate(t1, c, d)
        if ev is None:
            continue
        v = gain + max(ev[0], 0)
        glib.log(event='carrier_cand', c1=c1, k=k, better=better, cap=c, frame=d, gain=gain, letter=ev[1], ahead=ev[0])
        if best is None or v > best[0]:
            best = (v, gain, c1, k, c, d, occ2, ev)
    return best


def screen(tab, c, d, glob):
    """(letter by sibling support, support change of the contents the step moves and leaves off)."""
    st0 = tab.st
    ins = tab.rows[c]['inside']
    sib = H.Siblings(G, st0, ins, glob)
    stay = st0.dom[ins] != d
    before = sib.support(st0.dom[ins] * stay, st0.q[ins])
    best = None
    for X in G.letters(c, d):
        d2, q2 = glib.moved_contents(W, st0.dom[ins], st0.q[ins], W.E(c, X))
        s = sib.support(d2, q2)
        if best is None or s > best[0]:
            best = (s, X)
    return best[1], best[0] - before


def excursion(tab):
    """Best two-letter step [(c, X), (c2, X2)] whose first letter has a gain <= 0 and whose net gain is positive, or
    None. The --excursion best caps by gain (best frame each) are tried; the second letter is the best candidate among
    the caps the first one affects."""
    first, seen = [], set()
    for x in tab.candidates():
        if x[1] in seen:
            continue
        seen.add(x[1])
        first.append(x)
        if len(first) >= EXC:
            break
    best = None
    for gain, c, d, n, occ in first:
        ev = evaluate(tab, c, d)
        if ev is None or gain + ev[0] <= 0:
            continue
        ahead, X, s2, t2, info = ev
        g = sorted((x for c2 in t2.aff for x in H.gains(t2.rows[c2], G) if x[0] == ahead),
                   key=lambda x: (x[4], x[1], x[2]))
        if not g:
            continue
        gain2, c2, d2, n2, occ2 = g[0]
        ev2 = evaluate(t2, c2, d2)
        if ev2 is None:
            continue
        v = gain + gain2 + max(ev2[0], 0)
        glib.log(event='excursion_cand', cap=c, gain=gain, cap2=c2, gain2=gain2, ahead2=ev2[0])
        if best is None or v > best[0]:
            best = (v, gain + gain2, (c, X), (c2, ev2[1]), occ2, ev2)
    return best


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
        cr = carrier(tab) if CARRY else None
        if cr is None and EXC:
            ex = excursion(tab)
            if ex is not None:
                v, net, w1, w2, occ2, (ahead, X2, s3, t3, info) = ex
                execute([w1, w2], tab.st, s3, 'XPS' if occ2 == 0 else 'XPSC', w2[0], net)
                tab = t3
                write_chain(start_digest)
                glib.log(event='chosen', excursion=list(w1), cap=w2[0], letter=X2, gain=net, ahead=ahead,
                         off=int(len(tab.st.off)), sec_step=round(time.time() - t1, 1))
                continue
        if cr is None:
            glib.log(event='stuck', off=int(len(tab.st.off)), best=tab.candidates()[:3])
            break
        v, gain, c1, k, c, d, occ, (ahead, X, s2, t2, info) = cr
        execute([(c1, k), (c, X)], tab.st, s2, 'KPS' if occ == 0 else 'KPSC', c, gain)
        tab = t2
        write_chain(start_digest)
        glib.log(event='chosen', carrier=[c1, k], cap=c, frame=d, letter=X, gain=gain, ahead=ahead,
                 off=int(len(tab.st.off)), sec_step=round(time.time() - t1, 1))
        continue
    # one candidate per cap (its best frame); with --letter support, the LOOK best by gain among those whose step does
    # not lower the sibling support of the contents it moves (scan at most --screen caps; else the LOOK best by gain)
    glob = H.Siblings.all_off(G, tab.st) if LETTER == 'support' else None
    seen, top, scanned = set(), [], []
    for x in cands:
        if x[1] in seen:
            continue
        seen.add(x[1])
        if glob is not None and len(seen) <= SCREEN:
            X0, delta = screen(tab, x[1], x[2], glob)
            scanned.append((x, X0, delta))
            if delta < 0:
                continue
            top.append((x, X0, delta))
        elif glob is None:
            top.append((x, None, 0))
        if len(top) >= LOOK:
            break
    if not top:
        top = scanned[:LOOK]
    best = None
    for (gain, c, d, n, occ), X0, delta in top:
        ev = evaluate(tab, c, d, X0, glob)
        if ev is None:
            continue
        v = gain + max(ev[0], 0)
        glib.log(event='cand', cap=c, frame=d, n=n, occ=occ, gain=gain, letter=ev[1], ahead=ev[0], delta=delta, **ev[4])
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
