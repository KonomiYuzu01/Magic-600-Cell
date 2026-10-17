# E-B2, the strict-step falsifier R21-02 (SENIOR-REVIEW.md, round 21): for a unit U of Z put P = off(Z) \ U and let
# F_t(P) be the lattice chambers of type t whose interior meets the interior of a piece of P. A strict step for U
# needs |F_t(P)| <= |P_t| for every type t. This script bounds F(P) from both sides, exactly (Q(sqrt5)):
# - upper bound: a piece whose region (its domain representative applied to the vertices of its chamber in the
#   domain's frame, rim.Domains) lies in the closed half-space N_c . y >= |n|^2 of one facet c of the body meets no
#   chamber (every chamber lies in the open body), so it adds nothing to F;
# - lower bound: for the other pieces, sampled interior points inside the open body and on no cut lie in the interior
#   of exactly one chamber (as in stage 08 hloc.py), each an exact overlap certificate.
# A unit is refuted (no strict step) when the lower bound already exceeds |P_t| for some t; it passes for certain
# when the upper bound (pieces not separated by a facet, with all their candidate chambers) cannot exceed |P_t|.
# Run from the repository root:
#   python3 -I -B r2102.py table <pose table json or json.gz> <out json> [--samples N]
#   python3 -I -B r2102.py record <census record json> <out json> [--samples N]
import sys, json, time
from collections import Counter, defaultdict
from pathlib import Path
from types import SimpleNamespace
import numpy as np
REPO = Path.cwd()
TH = REPO / 'research/jumbling/theory'
sys.path[:0] = [str(TH), str(REPO / 'research/jumbling'), str(REPO / 'research/jumbling/fixtures'),
                str(Path(__file__).resolve().parent)]
import restore, rim, blocks as BL, classes as CL
from exact import Q5, dot, matvec
from screen import load

T0 = time.time()
a = sys.argv[4:]
NS = int(a[a.index('--samples') + 1]) if '--samples' in a else 24
RNG = np.random.default_rng(20261017)


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)


def main():
    st, M, src = load(sys.argv[1], sys.argv[2])
    out = Path(sys.argv[3])
    data, reg = CL.data, CL.ctx.regions
    TYPE = np.asarray(reg.orbit_of)
    D = rim.Domains(st)
    bl = BL.Blocks(SimpleNamespace(st=st), M)
    off = bl.off
    sigkey = {bytes(data.sigbits[x]): x for x in range(len(st.pose_id))}
    NN, KAPPA = data.NN, data.KAPPA
    N = data.N
    sep, found, cand, why = {}, {}, {}, Counter()
    for n, p in enumerate(off.tolist()):
        d, q = int(D.dom[p]), int(D.cham[p])
        V = reg.vertices(q)
        Y = [matvec(D.reps[d], v) for v in V]
        facet = next((c for c in range(600) if all((dot(N[c], y) - NN).sign() >= 0 for y in Y)), None)
        sep[p] = facet
        if facet is not None:
            continue
        xs = set()
        for j in range(NS):
            w = [1] * len(V) if j == 0 else RNG.integers(1, 1000, len(V)).tolist()
            tot = Q5(sum(w))
            y = [sum((Y[k][i] * Q5(int(w[k])) for k in range(len(V))), Q5(0)) / tot for i in range(4)]
            bits = np.zeros(8 * data.sigbits.shape[1], bool)
            r = 'in'
            for c in range(600):
                v = dot(N[c], y)
                if (v - NN).sign() >= 0:
                    r = 'outside'; break
                s = (v - KAPPA).sign()
                if s == 0:
                    r = 'on_cut'; break
                bits[c] = s > 0
            if r == 'in':
                x = sigkey.get(bytes(np.packbits(bits)))
                r = 'in' if x is not None else 'no_chamber'
                if x is not None:
                    xs.add(int(x))
            why[r] += 1
        found[p] = sorted(xs)
        # candidate chambers: signature equal to the piece's side of every cut that does not straddle it
        lab = bl.labels[n]
        keep = np.packbits(lab != 2, bitorder='big')
        want = np.packbits(lab == 1, bitorder='big')
        ok = ((data.sigbits & keep) == (want & keep)).all(axis=1)
        cand[p] = np.flatnonzero(ok).tolist()
        if n % 200 == 0:
            log(piece=n, of=len(off))
    # units: blocks split by frame domain
    dom = np.asarray(D.dom)
    units = []
    for b in bl.blocks:
        b = np.asarray(b)
        for d in np.unique(dom[b]).tolist():
            units.append(b[dom[b] == d])
    rows = []
    for u, U in enumerate(units):
        Us = set(U.tolist())
        Pp = [p for p in off.tolist() if p not in Us]
        P_t = Counter(int(TYPE[p]) for p in Pp)
        lo = Counter(int(TYPE[x]) for x in {x for p in Pp for x in found.get(p, [])})
        unsep = [p for p in Pp if sep[p] is None]
        viol = {str(t): [lo[t], P_t[t]] for t in lo if lo[t] > P_t[t]}
        hi = Counter(int(TYPE[x]) for x in {x for p in unsep for x in cand[p]})
        hold = all(hi[t] <= P_t[t] for t in hi)
        rows.append({'unit': u, 'first': int(U.min()), 'size': len(U), 'P': len(Pp),
                     'P_pieces_not_separated': len(unsep), 'F_lower': sum(lo.values()),
                     'F_exactly_empty': not unsep, 'F_upper': sum(hi.values()), 'holds_for_certain': hold,
                     'violations': viol, 'refuted': bool(viol)})
    doc = {'source': src, 'digest': st.digest(), 'off': len(off), 'samples': NS,
           'pieces_separated_by_a_facet': sum(1 for p in sep if sep[p] is not None),
           'samples_by_outcome': dict(why), 'units': rows,
           'units_refuted': sum(r['refuted'] for r in rows), 'units_F_exactly_empty': sum(r['F_exactly_empty'] for r in rows),
           'units_holding_for_certain': sum(r['holds_for_certain'] for r in rows),
           'state_refuted': bool(rows) and all(r['refuted'] for r in rows), 'seconds': round(time.time() - T0, 1)}
    out.write_text(json.dumps(doc, indent=1) + '\n')
    print(json.dumps({k: v for k, v in doc.items() if k != 'units'}))


main()
