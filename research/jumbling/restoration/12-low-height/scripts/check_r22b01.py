# Acceptance check for r22b01_cert.py (E22-1, theory round 22, line b): an explicit vacating word V for b00's
# transport target at the depth 40 stop, built by the conditional construction of R22B-01 (a three-cycle seed and two
# conjugator tables per target orbit; PROOFS-B.md section 3). It runs the builder once and checks its certificate
# without the builder's code:
# 1. the state digest and the target T (runs/lc13-k40/target-b00.json: 111 chambers, all occupied at the state);
# 2. the target orbits: every SAFE-ALL chamber orbit that meets T is listed once, with its least chamber, its size
#    d >= 5, k = |T ∩ Δ| and at least k holes (N2); no target chamber lies in a one-point orbit (N1);
# 3. per orbit, the seed C: on Δ exactly the three-cycle a -> b -> c -> a, on every earlier orbit of the list the
#    identity;
# 4. per orbit, the two tables: every x in Δ \ {a} reaches b along letters that fix a, and every z in Δ \ {a, b} is
#    reached from c along letters that fix a and b; e and f are the depths of the two trees;
# 5. V: SAFE-ALL letters only; after V no chamber of T is occupied; |V| <= 2·Σ k·(s + 2e + 2f), recomputed here.
# Exact; research only, headless Linux. Run from the repository root (at most about 25 minutes; the review wrapper
# stops an acceptance check after 30):
#   python3 -I -B research/jumbling/restoration/12-low-height/scripts/check_r22b01.py
import hashlib
import json
import subprocess
import sys
import tempfile
from pathlib import Path
from types import SimpleNamespace

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path[:0] = [str(HERE)]
REC = 'research/jumbling/restoration/11-scramble-census/runs/round2/ia-k40-s7101.json'
TGT = HERE.parent / 'runs/lc13-k40/target-b00.json'
SECONDS = 900


def fail(msg):
    print('FAIL', msg, flush=True)
    sys.exit(1)


def tree_depths(G, entries, root, fixed, members, forward):
    """Check a table tree and return its depth.

    entries: [point, grip, item, other]. forward=True: the letter sends point to other, and paths lead from every
    member to the root (P). forward=False: the letter sends other to point, and paths lead from the root to every
    member (Q). Every letter must fix each chamber of fixed.
    """
    for e in entries:
        if not (isinstance(e, list) and len(e) == 4 and all(type(v) is int for v in e)):
            fail(f'malformed table entry {str(e)[:200]}')
    arr = np.array(entries, np.int64).reshape(-1, 4)
    letter = np.array([G.index.get((int(g), int(i)), -1) for g, i in arr[:, 1:3]], np.int64)
    if (letter < 0).any():
        fail('a table letter is not a SAFE-ALL letter')
    src, dst = (arr[:, 0], arr[:, 3]) if forward else (arr[:, 3], arr[:, 0])
    fixed = np.array(fixed, np.int64)
    for n in np.unique(letter).tolist():
        m = letter == n
        if not np.array_equal(G.images([n], fixed), fixed):
            fail(f'table letter {G.pairs([n])[0]} moves a fixed anchor')
        if not np.array_equal(G.images([n], src[m]), dst[m]):
            fail(f'table letter {G.pairs([n])[0]} does not send its chambers as stated')
    nxt = dict(zip(arr[:, 0].tolist(), arr[:, 3].tolist()))
    if len(nxt) != len(arr):
        fail('two table entries for one chamber')
    want = set(members) - {root}
    if set(nxt) != want:
        fail(f'table covers {len(nxt)} chambers, expected {len(want)}')
    depth = {root: 0}
    for p in want:
        path, q = [], p
        while q not in depth:
            if q in path:
                fail(f'table has a cycle through {q}')
            path.append(q)
            if q not in nxt:
                fail(f'table path from {p} leaves the orbit at {q}')
            q = nxt[q]
        d = depth[q]
        for q in reversed(path):
            d += 1
            depth[q] = d
    return max(depth.values())


def main():
    tgt = json.loads(TGT.read_text())
    if hashlib.sha256(json.dumps(sorted(tgt['T'])).encode()).hexdigest() != tgt['T_digest']:
        fail('target file digest')
    T = np.array(tgt['T'], np.int64)
    with tempfile.TemporaryDirectory() as d:
        out = Path(d) / 'cert.json'
        cmd = [sys.executable, '-I', '-B', str(HERE / 'r22b01_cert.py'), REC, str(TGT), str(out),
               '--seconds', str(SECONDS)]
        try:
            r = subprocess.run(cmd, timeout=SECONDS + 300)
        except subprocess.TimeoutExpired:
            fail('the builder exceeded its time bound')
        if r.returncode != 0:
            fail(f'the builder exited with {r.returncode}')
        cert = json.loads(out.read_text())
    if not cert.get('complete'):
        fail(f"certificate incomplete: {json.dumps(cert.get('missing'))[:2000]}")
    import screen
    import buffer as B
    st, M, _ = screen.load('record', REC)
    if st.digest() != tgt['state_digest'] or cert.get('state_digest') != tgt['state_digest']:
        fail('state digest')
    if cert.get('T_digest') != tgt['T_digest']:
        fail('target digest in the certificate')
    G = B.Alphabet(SimpleNamespace(st=st), M)
    occ0 = G.occupied.copy()
    if not occ0[T].all():
        fail('a target chamber is a hole at the state')
    orbit, size = G.orbits()
    if (size[orbit[T]] == 1).any():
        fail('a target chamber is a fixed point of every letter (N1 fails)')
    labels = sorted(set(orbit[T].tolist()))
    listed = [int(orbit[o['least']]) for o in cert['orbits']]
    if sorted(listed) != labels or len(set(listed)) != len(listed):
        fail('the certificate does not list each target orbit exactly once')
    tset = np.zeros(G.NP, bool)
    tset[T] = True
    earlier = np.zeros(0, np.int64)
    bound, rows = 0, []
    for o in cert['orbits']:
        lab = int(orbit[o['least']])
        D = np.flatnonzero(orbit == lab)
        k = int(tset[D].sum())
        if int(D[0]) != o['least'] or len(D) != o['size'] or len(D) < 5 or k != o['k']:
            fail(f"orbit {o['least']}: least, size or k differs")
        if int((~occ0[D]).sum()) < k:
            fail(f"orbit {o['least']}: fewer holes than targets (N2)")
        a, b, c = o['anchors']
        if len({a, b, c}) != 3 or not all(int(orbit[v]) == lab for v in (a, b, c)):
            fail(f"orbit {o['least']}: anchors")
        seed = G.numbers(o['seed'])
        pts = np.concatenate([D, earlier])
        img = G.images(seed, pts)
        want = pts.copy()
        pos = {int(v): i for i, v in enumerate(D[:len(D)])}
        want[pos[a]], want[pos[b]], want[pos[c]] = b, c, a
        if not np.array_equal(img, want):
            fail(f"orbit {o['least']}: the seed is not the three-cycle (a b c) on the orbit and the identity before")
        members = D.tolist()
        e = tree_depths(G, o['P'], b, [a], [v for v in members if v != a], True)
        f = tree_depths(G, o['Q'], c, [a, b], [v for v in members if v not in (a, b)], False)
        s = len(seed)
        bound += k * (s + 2 * e + 2 * f)
        rows.append({'least': int(D[0]), 'd': len(D), 'k': k, 's': s, 'e': e, 'f': f})
        earlier = np.concatenate([earlier, D])
    bound *= 2
    V = G.numbers(cert['V'])
    pV = G.perm(V)
    occ = np.zeros(G.NP, bool)
    occ[pV[np.flatnonzero(occ0)]] = True
    if occ[T].any():
        fail(f'V leaves {int(occ[T].sum())} target chambers occupied')
    if len(V) > bound:
        fail(f'|V| = {len(V)} exceeds the bound {bound}')
    if cert.get('V_length') != len(V) or cert.get('bound') != bound:
        fail('the certificate states another length or bound')
    print(json.dumps({'orbits': rows, 'V_length': len(V), 'bound': bound}), flush=True)
    print('PASS', flush=True)


if __name__ == '__main__':
    main()
