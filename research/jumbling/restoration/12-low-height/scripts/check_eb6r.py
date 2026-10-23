# Acceptance check for eb6r.py (E22-2, theory round 22, line d): the four-letter commutators of E-B6 after one
# retained letter, r·C·r⁻¹ (R22D-01). It checks, at the depth 100 stop of the stage 11 census:
# 1. the classification rule of R22D-01: for a piece inside the clean cap c, its class at cap x after the retained
#    letter (c, j) is its class at Z at cap k⁻¹·x, where k is the letter's element of K⁺; other pieces keep theirs.
#    eb6r.ZData.predict(c, j) must give, for every one of the 600 caps, the same inside set and the same clean flag as
#    a direct exact classification after the letter is applied, and the same straddler set at the caps near 61;
# 2. a run of the command line on two grips with --identity: a well-formed record, the state digest unchanged, the
#    letters at grip 61 (whose cap holds every high unit) searched, and the search at Z itself (the identity
#    preparation) equal, unit by unit, to the counts of the committed E-B6 record runs/eb6-k100.json.
# Exact geometry; research only, headless Linux. Run from the repository root (about 20 to 30 minutes):
#   python3 -I -B research/jumbling/restoration/12-low-height/scripts/check_eb6r.py
import json
import subprocess
import sys
import tempfile
from pathlib import Path

import numpy as np

REPO = Path.cwd()
HERE = Path(__file__).resolve().parent
sys.path[:0] = [str(HERE)]
REC = 'research/jumbling/restoration/11-scramble-census/runs/round2/ia-k100-s7102.json'
EB6 = json.loads((HERE.parent / 'runs/eb6-k100.json').read_text())
KEYS = ['pairs', 'pairs_last_cap_blocked', 'pairs_unit_not_carried', 'pairs_all_items', 'pairs_commuting_items',
        'commuting_item_pairs', 'first_letters', 'words', 'admissible', 'strict', 'strict_lowering']


def fail(msg):
    print('FAIL', msg, flush=True)
    sys.exit(1)


def part1():
    import eb6r
    import eb6
    from screen import load
    st, M, _ = load('record', REC)
    st.journal = []
    digest0 = st.digest()
    if digest0 != EB6['digest']:
        fail('the depth 100 stop does not match runs/eb6-k100.json')
    Z = eb6r.ZData(st, M)
    if int(Z.clean.sum()) != EB6['clean_caps']:
        fail(f'clean caps {int(Z.clean.sum())} != {EB6["clean_caps"]}')
    clean = np.flatnonzero(Z.clean).tolist()
    near61 = [int(x) for x in eb6.R.NEAR[61]]
    picks = [61, min(x for x in clean if x in near61 and x != 61), clean[0], clean[-1]]
    checked = 0
    for c in picks:
        items = Z.retained(c)
        if len(items) != 11:
            fail(f'grip {c}: {len(items)} retained non-identity items, expected 11')
        j = items[len(items) // 2]
        ins_y, strad_y, clean_y = Z.predict(c, j)
        o = st.apply(eb6.sim.Twist(st.ctx, c, eb6.R.element(M, c, j), 'menu', {'menu': M.name}))
        if not o.applied or o.uncertain:
            fail(f'letter ({c}, {j}) was not applied')
        for x in range(600):
            cl = st.classify(x, need_inside=True, stop_at_straddle=False)
            if cl.uncertain:
                fail(f'uncertain classification at {x}')
            if not np.array_equal(np.asarray(ins_y[x]), cl.inside):
                fail(f'letter ({c}, {j}), cap {x}: predicted inside set differs')
            if bool(clean_y[x]) != (cl.straddle is None):
                fail(f'letter ({c}, {j}), cap {x}: predicted clean flag differs')
        for x in near61[:12]:
            if not np.array_equal(np.asarray(strad_y[x]), eb6.straddler_set(st, x)):
                fail(f'letter ({c}, {j}), cap {x}: predicted straddler set differs')
        st.undo()
        checked += 1
        print(json.dumps({'letter': [c, j], 'caps_compared': 600, 'straddler_sets_compared': 12}), flush=True)
    if st.digest() != digest0:
        fail('the state changed')
    return picks


def part2(picks):
    grips = sorted({picks[0], picks[1]})
    with tempfile.TemporaryDirectory() as td:
        out = Path(td) / 'eb6r.json'
        cmd = [sys.executable, '-I', '-B', str(HERE / 'eb6r.py'), 'record', REC, str(out),
               '--grips', ','.join(map(str, grips)), '--identity']
        r = subprocess.run(cmd, cwd=REPO)
        if r.returncode != 0:
            fail(f'eb6r.py exited with {r.returncode}')
        doc = json.loads(out.read_text())
    if doc.get('digest') != EB6['digest']:
        fail('record digest differs')
    if doc.get('complete') is not True:
        fail('the run is not marked complete')
    firsts = [u['first'] for u in EB6['units']]
    if sorted(u['first'] for u in doc['units']) != sorted(firsts):
        fail('the high units differ from runs/eb6-k100.json')
    ident = doc['identity']['units']
    for u in EB6['units']:
        got = ident[str(u['first'])]
        for k in KEYS:
            if got[k] != u['comm4'][k]:
                fail(f'identity search, unit {u["first"]}, {k}: {got[k]} != {u["comm4"][k]}')
    letters = doc['letters']
    want = {(c, j) for c in grips for j in range(60)}
    seen = {(e['grip'], e['item']) for e in letters}
    if not seen or not seen <= want or len(seen) != len(letters):
        fail('the letters of the record are not distinct letters at the requested grips')
    if len({e['item'] for e in letters if e['grip'] == 61}) != 11:
        fail('grip 61 does not have its 11 retained letters')
    for e in letters:
        if e['status'] not in ('searched', 'pruned'):
            fail(f'unknown status {e["status"]}')
        if e['grip'] == 61 and e['status'] != 'searched':
            fail('a letter at grip 61 moves every high unit and cannot be pruned')
        if e['status'] == 'searched':
            if sorted(e['units']) != sorted(map(str, firsts)):
                fail(f'letter {e["grip"], e["item"]}: not every unit searched')
            for v in e['units'].values():
                for k in KEYS:
                    if not isinstance(v.get(k), int):
                        fail(f'letter {e["grip"], e["item"]}: field {k} missing')
        for w in e.get('words', []):
            if len(w['word']) != 6 or w['word'][0] != [e['grip'], e['item']]:
                fail('a kept word is not r, C, r^-1')
            if not isinstance(w['replay'].get('admissible'), bool):
                fail('a kept word has no replay result')
    t = doc['totals']
    if t['letters'] != len(letters) or t['searched'] + t['pruned'] != len(letters):
        fail('totals do not add up')
    print(json.dumps({'letters': len(letters), 'searched': t['searched'], 'pruned': t['pruned'],
                      'strict_words': t['strict_words']}), flush=True)


def main():
    picks = part1()
    part2(picks)
    print('PASS', flush=True)


if __name__ == '__main__':
    main()
