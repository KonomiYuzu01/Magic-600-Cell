"""One continuous replay of the whole history of E3 from the solved state (menu I_a), every twist checked.
Segments, in order:
  1. the first 40 journal records of the W-J fixture wj-I_a (the I_a@40 scramble), each checked to be an I_a menu twist;
  2. the 62 moves of the seams stop (results/seams-I_a-40-stop.json);
  3. the 24 moves of the E2 run (results/joint-attack-I_a-40.json, e2_stop_no_history) -> the E2 end;
  4. the first 36,563 letters of the owner's 41,457-letter word from the E2 end -> prefix 36563;
  5. u, the 302 letters of X's reachability certificate -> X;
  6. D17 (parts V, P, Q, Pinv, Vinv; 1,112 twists) -> E1;
  7. D87 (1,210 twists) -> E2 of the chain;
  8. D3 (79 twists) -> E3.
Every twist must be admissible (sim.State.apply(...).applied); the simulator digest must equal the recorded digest at
every segment end. Nothing is written except the output JSON.
usage: replay_e3_history.py <repo root> <owner word json> <X-reachability json> <D17 parts dir> <D87 word json>
                            <D3 word json> <out json>"""
import json, sys, time
from pathlib import Path
REPO = Path(sys.argv[1]).resolve()
TH = REPO / 'research/jumbling/theory'
sys.path[:0] = [str(TH), str(REPO / 'research/jumbling'), str(REPO / 'research/jumbling/fixtures')]
sys.dont_write_bytecode = True
import numpy as np
import sim, seams, wj
import restore as RS
t0 = time.time()
ctx = sim.get_context()
OWNER, XR, D17, D87, D3, OUT = sys.argv[2:8]
fixture, M = wj.load(ctx, 'I_a')
recs = fixture['journal']['records'][:40]
st = sim.State(ctx, menu=M)
start_digest = st.digest()
menus = {}
def letter(c, i):
    if c not in menus:
        menus[c] = M.for_cap(c)
    tw = menus[c][i]
    assert tw.matrix == RS.element(M, c, i)
    return tw
rows = []
def segment(name, twists, expect=None):
    n0, s0 = len(rows), time.time()
    offmax = 0
    for k, tw in enumerate(twists):
        r = st.apply(tw, journal=False)
        if not r.applied:
            raise SystemExit(f'{name}: twist {k} not admissible')
        if k % 500 == 0 or k == len(twists) - 1:
            offmax = max(offmax, st.off_lattice_count())
    dg = st.digest()
    row = {'segment': name, 'twists': len(twists), 'end_digest': dg, 'expected': expect,
           'digest_ok': (expect is None) or dg == expect, 'off_end': int(st.off_lattice_count()),
           'seconds': round(time.time() - s0, 1)}
    rows.append(row)
    print(json.dumps(row), flush=True)
    if expect is not None and dg != expect:
        raise SystemExit(f'{name}: digest mismatch')
# 1. fixture records: each must be an I_a menu twist at its grip
fx = []
for r in recs:
    tw = sim.Twist.from_record(ctx, r)
    c = tw.grip
    mats = [m.matrix for m in M.for_cap(c)]
    assert tw.matrix in mats, 'fixture record is not an I_a menu twist'
    fx.append(tw)
segment('fixture I_a@40 (40 records)', fx)
stop = json.loads((TH / 'results/seams-I_a-40-stop.json').read_text())['moves']
e2 = json.loads((TH / 'results/joint-attack-I_a-40.json').read_text())['e2_stop_no_history']['moves']
assert len(stop) == 62 and len(e2) == 24
segment('seams stop (62)', [letter(c, i) for c, i in stop])
ow = json.load(open(OWNER))
assert len(ow['word']) == 41457
segment('E2 run (24)', [letter(c, i) for c, i in e2], ow['input_digest'])
xr = json.load(open(XR))
segment("owner's word, prefix 36563", [letter(c, i) for c, i in ow['word'][:36563]], xr['seed_table_digest'])
segment('u (302) -> X', [letter(c, i) for c, i in xr['u']], xr['X_digest'])
d17 = []
first_in = None
for p in ['V', 'P', 'Q', 'Pinv', 'Vinv']:
    doc = json.load(open(Path(D17) / f'{p}.json'))
    d17 += doc['word']
    last_out = doc.get('output_digest')
d87 = json.load(open(D87)); d3 = json.load(open(D3))
segment('D17 (V P Q Pinv Vinv) -> E1', [letter(c, i) for c, i in d17], d87['input_digest'])
segment('D87 -> E2 (chain)', [letter(c, i) for c, i in d87['word']], d87['output_digest'])
assert d3['input_digest'] == d87['output_digest']
segment('D3 -> E3', [letter(c, i) for c, i in d3['word']], d3['output_digest'])
lat = st.lattice_flags()
out = {'claim': 'E3 is reachable from the solved state of menu I_a by admissible twists (one continuous replay)',
       'start_digest_solved': start_digest, 'segments': rows, 'total_twists': sum(r['twists'] for r in rows),
       'end_digest': st.digest(), 'end_off': int((~lat).sum()), 'all_digests_ok': all(r['digest_ok'] for r in rows),
       'seconds': round(time.time() - t0, 1)}
json.dump(out, open(OUT, 'w'), indent=1)
print(json.dumps({k: v for k, v in out.items() if k != 'segments'}), flush=True)
