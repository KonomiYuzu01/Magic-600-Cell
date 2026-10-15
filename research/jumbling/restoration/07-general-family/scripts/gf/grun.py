"""General family run: cap restorations with carriers, executed and checked on the simulator.

Step kinds (each restores every off content of one cap, others unchanged):
- S: (c, X) at a cap with no lattice piece, unstraddled, whose off contents X sends to K+ (Lemma S);
- SV: V (c, X) V^-1 with V a word of K+ letters at unstraddled grips that empties cap c of lattice pieces while
  keeping its off contents single-frame (Lemma V' with an S core; clear.py).
Each step is applied on the simulator and checked: the cap's contents on the lattice, collateral 0, every other off
pose unchanged (exact pose codes), and the virtual contents equal to the simulator's off set.

usage: grun.py <research/jumbling> <E-chain scripts> <table> <out dir> [--blocks <blocks json>] [--max-steps N]
       [--no-clear] [--save-every K]
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import glib  # noqa: E402

J, SC, TABLE, OUT = sys.argv[1:5]
a = sys.argv[5:]
BLOCKS = a[a.index('--blocks') + 1] if '--blocks' in a else None
MAXS = int(a[a.index('--max-steps') + 1]) if '--max-steps' in a else 10 ** 6
SAVE = int(a[a.index('--save-every') + 1]) if '--save-every' in a else 10
CLEAR = '--no-clear' not in a
OUT = Path(OUT)
OUT.mkdir(parents=True, exist_ok=True)
glib.setup(J + '/theory', SC)
import buffer as BU  # noqa: E402
import clear  # noqa: E402
import conjclose as CC  # noqa: E402

W = glib.World(TABLE, BLOCKS)
R, st = W.R, W.st
codes = CC.Codes()
_letters = {}


def letters_for(c, doms):
    key = (c, tuple(doms))
    if key not in _letters:
        _letters[key] = [Xi for Xi in range(W.n_items)
                         if all(W.F.mul(W.to_tuple(W.E(c, Xi)), W.E(c, Xi), d)[0] == 0 for d in doms)]
    return _letters[key]


C = clear.Clearer(W, letters_for)


def cap_row(vs, c, offv):
    off = vs.off
    nocc = int((C.capm(c) & vs.occ()).sum())
    lo, hi = W.cap_range(offv, [c])
    near = np.flatnonzero(hi[:, 0] >= -1e-6)
    s = W.sides(vs.dom[off[near]], vs.q[off[near]], c) if len(near) else np.zeros(0, np.int64)
    inside = off[near[s == 1]]
    strad = int((s < 0).sum())
    doms = sorted(set(vs.dom[inside].tolist()))
    lets = letters_for(c, doms) if (strad == 0 and len(inside)) else []
    return {'cap': c, 'occupied': nocc, 'inside': inside, 'straddlers': strad, 'letters': lets, 'domains': doms}


def scan(vs):
    offv = vs.off_vertices()
    return [cap_row(vs, c, offv) for c in range(600)]


state = {'codes': None, 'digest': None, 'step': 0, 'steps': []}


def execute(vs, word, target, kind, cap, vs_end):
    """Apply the word on the simulator, check, record; return vs_end (the virtual state after the word)."""
    t0 = time.time()
    c_s = state['codes'] if state['codes'] is not None else codes.of(st)
    d_in = state['digest'] if state['digest'] is not None else st.digest()
    for c, i in word:
        R.apply(int(c), int(i))
    c_e = codes.of(st)
    d_out = st.digest()
    off_s, off_e = c_s < 0, c_e < 0
    inT = np.zeros(len(c_s), bool)
    inT[target] = True
    prot = off_s & ~inT
    state['step'] += 1
    step = state['step']
    row = {'step': step, 'kind': kind, 'cap': int(cap), 'length': len(word), 'target': int(len(target)),
           'blocks': sorted(set(W.block_of[target].tolist())),
           'off_before': int(off_s.sum()), 'off_after': int(off_e.sum()),
           'target_on': int((inT & ~off_e).sum()), 'collateral': int((~off_s & off_e).sum()),
           'protected': int(prot.sum()), 'protected_changed': int((prot & (c_e != c_s)).sum()),
           'virtual_matches': bool(np.array_equal(np.flatnonzero(vs_end.dom != 0), np.flatnonzero(off_e))),
           'input_digest': d_in, 'output_digest': d_out, 'sec': round(time.time() - t0, 1)}
    row['ok'] = (row['target_on'] == row['target'] and row['collateral'] == 0 and row['protected_changed'] == 0
                 and row['virtual_matches'])
    (OUT / f'step{step:03d}-word.json').write_text(json.dumps(
        {'word': [[int(c), int(i)] for c, i in word], 'input_digest': d_in, 'output_digest': d_out, 'kind': kind,
         'cap': int(cap)}) + '\n')
    (OUT / f'step{step:03d}-target.json').write_text(json.dumps({'pieces': [int(p) for p in target]}) + '\n')
    with open(OUT / 'run.jsonl', 'a') as f:
        f.write(json.dumps(row) + '\n')
    glib.log(event='step', **{k: v for k, v in row.items() if k not in ('input_digest', 'output_digest')})
    if not row['ok']:
        raise SystemExit('step check failed')
    state['codes'], state['digest'] = c_e, d_out
    state['steps'].append({'label': f'{kind}{step:03d}-cap{cap}', 'word': f'step{step:03d}-word.json',
                           'target': f'step{step:03d}-target.json'})
    if step % SAVE == 0:
        BU.export_pose_table(st, str(OUT / f'step{step:03d}-table.json'))
    return vs_end


def write_chain(start_digest):
    (OUT / 'chain.json').write_text(json.dumps({'name': OUT.name, 'start': str(Path(TABLE).resolve()),
                                                'start_digest': start_digest, 'steps': state['steps']},
                                               indent=1) + '\n')


vs = glib.VState(W)
start_digest = st.digest()
state['digest'] = start_digest
failed_clear = set()
while state['step'] < MAXS:
    t0 = time.time()
    rows = scan(vs)
    s_caps = [r for r in rows if r['straddlers'] == 0 and r['occupied'] == 0 and len(r['inside']) and r['letters']]
    sv_caps = [r for r in rows if r['straddlers'] == 0 and r['occupied'] > 0 and len(r['inside']) and r['letters']]
    glib.log(event='scan', off=int(len(vs.off)), s_caps=len(s_caps), sv_caps=len(sv_caps),
             mixed=sum(1 for r in rows if r['straddlers'] == 0 and len(r['inside']) and not r['letters']),
             sec_scan=round(time.time() - t0, 1))
    progressed = False
    for r in sorted(s_caps, key=lambda r: -len(r['inside'])):
        r2 = cap_row(vs, r['cap'], vs.off_vertices())
        if r2['straddlers'] == 0 and r2['occupied'] == 0 and len(r2['inside']) and r2['letters']:
            vs2 = vs.apply([(r2['cap'], r2['letters'][0])])
            vs = execute(vs, [(r2['cap'], r2['letters'][0])], r2['inside'], 'S', r2['cap'], vs2)
            progressed = True
            if state['step'] >= MAXS:
                break
    if progressed:
        write_chain(start_digest)
        continue
    if not CLEAR:
        break
    for r in sorted(sv_caps, key=lambda r: r['occupied']):
        key = (r['cap'], int(len(vs.off)))
        if key in failed_clear:
            continue
        t1 = time.time()
        res = C.search(vs, r['cap'], depth=8, beam=4, expand=12)
        if res is None:
            failed_clear.add(key)
            glib.log(event='clear_fail', cap=r['cap'], occupied=r['occupied'], t=round(time.time() - t1, 1))
            continue
        V, vsV, U, lets = res
        word = list(V) + [(r['cap'], lets[0])] + W.inverse_word(V)
        vs_end = vs.apply(word)
        if vs_end is None:
            glib.log(event='clear_dead', cap=r['cap'])
            failed_clear.add(key)
            continue
        glib.log(event='clear_ok', cap=r['cap'], V=len(V), contents=int(len(U)), t=round(time.time() - t1, 1))
        vs = execute(vs, word, U, 'SV', r['cap'], vs_end)
        progressed = True
        break  # rescan: S caps may have appeared
    write_chain(start_digest)
    if not progressed:
        break
BU.export_pose_table(st, str(OUT / 'end-table.json'))
write_chain(start_digest)
glib.log(event='end', steps=state['step'], off=int(len(vs.off)))
