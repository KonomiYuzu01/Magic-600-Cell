"""Glued pairs and their split candidates at a state (R1, card 4, E, part B).

Two pieces are glued when every one of the 600 cuts has both of them strictly on the same side (equal sign vectors,
no straddle). No retained word separates glued pieces: a retained twist maps the arrangement of the 600 cuts to
itself, so it moves both or neither and they stay glued. A jumble step J = (d, g) at a cap d containing both moves
them by g, and afterwards a cut c can have exactly one of them inside: c separates them after J exactly when the
hyperplane g^-1 H_c separates their regions.

At a state given as a current-only pose table (checked against its digest), or as the E2 end followed by word
files, this script lists:
- glued pairs: every off-lattice piece whose sides of the 600 cuts are all 'in' or 'out' and equal those of a
  lattice piece. The partner is found through the chamber whose home signature equals the off piece's inside
  signature (the cuts whose inside set contains it), and both pieces are then checked exactly cut by cut;
- for each glued pair and each cap d containing both: whether d is admissible and, for each menu element i of d
  that is not retained, the cuts c that separate the pair after J = (d, i) (exact sides of both pieces over the
  600 cuts); for admissible d, which of them are admissible after J and the off-lattice count after J;
- loops J C J^-1 C^-1 with C = (c, x), for the admissible (d, i, c) with the fewest off-lattice pieces after J and
  every non-identity menu element x of c: the word as (grip, menu index) twists, whether every twist is
  admissible, whether the pair ends separated (the two pieces' net rotations differ), the digest at the end, and
  the collateral: the other pieces whose pose changed, the pieces that left or joined the lattice (lists of piece
  ids) and the off-lattice count at the end. Every loop is applied and undone on the exact simulator. --out keeps
  the piece lists of the --lists loops with the fewest other pieces moved; --full keeps them for every loop.

Run from the repository root:
    python research/jumbling/theory/split_pairs.py --words research/jumbling/theory/results/e-chain/*.json \
        --loops 1000 --out <file> [--full <file>]            (the 277 state; e-chain/[1-4]-*.json for 388)
"""
import argparse
import json
import sys
import time
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE.parent / 'fixtures'))

import classes as CL  # noqa: E402
import e3_class as E3  # noqa: E402
import orient as O  # noqa: E402
import trees  # noqa: E402
from exact import matmul, transpose  # noqa: E402
from sim.kernel import Evaluator, pullback_form  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)


def side(pose, p, c):
    return Evaluator(pullback_form(pose, CL.data.N[c])).classify(CL.ctx.regions.vform(p))[0]


def sides(pose, p):
    return [side(pose, p, c) for c in range(600)]


def retained(g):
    return all(E3.prop_unit(q) for q in trees.factors(g))


class Splitter:
    def __init__(self, R, M):
        self.R, self.M, self.st = R, M, R.st
        self.n_items = len(M.items)
        self._inv = {}
        st = self.st
        NP = len(st.pose_id)
        K = st._kpose()[st.pose_id]
        self.lat = K >= 0
        reg, kp = CL.ctx.regions, CL.ctx.kplus
        L = np.flatnonzero(self.lat)
        ch = reg.piece_of[reg.orbit_of[L], kp.compose_vec(K[L], reg.transport[L])]
        self.occupant = np.full(NP, -1, np.int64)
        self.occupant[ch] = L
        off = np.flatnonzero(~self.lat)
        sig = np.zeros((len(off), 600), bool)
        self.admissible = np.zeros(600, bool)
        for c in range(600):
            cl = st.classify(int(c), need_inside=True, stop_at_straddle=False)
            if cl.uncertain:
                raise AssertionError(f'uncertain classification at grip {c}')
            self.admissible[c] = cl.straddle is None
            sig[:, c] = np.isin(off, np.asarray(cl.inside, np.int64))
        self.home = CL.data.sig_matrix(np.arange(NP))
        lookup = {r.tobytes(): x for x, r in enumerate(np.packbits(self.home, axis=1))}
        assert len(lookup) == NP, 'two chambers with one home signature'
        D = R.domains()
        dom = np.asarray(D.dom)
        members, cls = CL.classes()
        dcls = {}
        for d in range(1, len(D.reps)):
            h = CL.hub(D.reps[d], members, cls)
            dcls[d] = h[0][1] if len(h) == 1 else None
        self.domains = {d: {'pieces': len(D.members[d]), 'class': dcls[d]} for d in range(1, len(D.reps))}
        self.dom = dom
        self.pairs = []
        self.off_in_a_chamber_region = 0
        for n, p in enumerate(off.tolist()):
            x = lookup.get(np.packbits(sig[n]).tobytes())
            if x is None:
                continue
            self.off_in_a_chamber_region += 1
            if self.occupant[x] < 0:
                continue
            q = int(self.occupant[x])
            sp, sq = sides(st.pose(p), p), sides(st.pose(q), q)
            if sp == sq and 'straddle' not in sp:
                d = int(dom[p])
                self.pairs.append({'off_piece': p, 'lattice_piece': q, 'chamber': int(x), 'domain': d,
                                   'domain_pieces': self.domains[d]['pieces'], 'domain_class': self.domains[d]['class']})
        log(off=len(off), off_in_a_chamber_region=self.off_in_a_chamber_region, glued_pairs=len(self.pairs))

    def inv(self, c, i):
        """Menu index of the inverse of element i of grip c."""
        if (c, i) not in self._inv:
            g = to_tuple(transpose(self.R.g(c, i)))
            js = [j for j in range(self.n_items) if to_tuple(self.R.g(c, j)) == g]
            assert len(js) == 1
            self._inv[(c, i)] = js[0]
        return self._inv[(c, i)]

    def candidates(self, pair):
        R, st = self.R, self.st
        p, q = pair['off_piece'], pair['lattice_piece']
        Pp, Pq = st.pose(p), st.pose(q)
        caps = np.flatnonzero(self.home[pair['chamber']]).tolist()
        rows, tried = [], 0
        for d in caps:
            for i in range(self.n_items):
                g = R.g(d, i)
                if retained(g):
                    continue
                tried += 1
                gp, gq = matmul(g, Pp), matmul(g, Pq)
                sep = [c for c in range(600) if {side(gp, p, c), side(gq, q, c)} == {'in', 'out'}]
                if not sep:
                    continue
                row = {'d': int(d), 'i': int(i), 'd_admissible': bool(self.admissible[d]), 'separating_cuts': sep}
                if self.admissible[d]:
                    R.apply(d, i)
                    row['admissible_after'] = [c for c in sep if R.inside(c) is not None]
                    row['off_lattice_after'] = st.off_lattice_count()
                    R.undo()
                rows.append(row)
        return {'caps_containing_both': caps, 'admissible_caps': [c for c in caps if self.admissible[c]],
                'jumble_steps_tried': tried, 'rows': rows}

    def codes(self, code):
        return np.array([code.setdefault(k.key, len(code)) for k in self.st._poses])[self.st.pose_id]

    def loop(self, pair, d, i, c, x, base):
        R, st = self.R, self.st
        p, q = pair['off_piece'], pair['lattice_piece']
        code, c0, lat0, P0 = base
        steps = [(int(d), int(i)), (int(c), int(x)), (int(d), self.inv(d, i)), (int(c), self.inv(c, x))]
        done = 0
        try:
            for s in steps:
                R.apply(*s)
                done += 1
        except RuntimeError:
            pass
        try:
            out = {'d': d, 'i': i, 'c': c, 'x': x, 'word': [list(t) for t in steps], 'twists_admissible': done}
            if done < 4:
                return out
            c1 = self.codes(code)
            lat1 = st.lattice_flags()
            moved = np.setdiff1d(np.flatnonzero(c1 != c0), [p, q])
            net = [to_tuple(matmul(st.pose(r), transpose(P0[r]))) for r in (p, q)]
            out.update({'separated': net[0] != net[1], 'end_digest': st.digest(),
                        'other_pieces_moved': int(moved.size),
                        'left_lattice': int((lat0 & ~lat1).sum()), 'joined_lattice': int((~lat0 & lat1).sum()),
                        'joined_lattice_by_domain': {int(k): int(v) for k, v in
                                                     zip(*np.unique(self.dom[~lat0 & lat1], return_counts=True))},
                        'off_lattice_after': int((~lat1).sum()),
                        'moved_pieces': moved.tolist(), 'left_lattice_pieces': np.flatnonzero(lat0 & ~lat1).tolist(),
                        'joined_lattice_pieces': np.flatnonzero(~lat0 & lat1).tolist()})
            return out
        finally:
            for _ in range(done):
                R.undo()

    def loops(self, pair, cand, n):
        st = self.st
        code = {}
        base = (code, self.codes(code), st.lattice_flags().copy(),
                {r: st.pose(r) for r in (pair['off_piece'], pair['lattice_piece'])})
        trips = sorted(((r['off_lattice_after'], r['d'], r['i'], c) for r in cand['rows']
                        for c in r.get('admissible_after', [])))[:n]
        out = []
        for _, d, i, c in trips:
            for x in range(1, self.n_items):
                out.append(self.loop(pair, d, i, c, x, base))
            log(loops=len(out), d=d, i=i, c=c)
        after = self.codes(code)
        assert (after == base[1]).all(), 'a loop was not undone'
        return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--table', help='current-only pose table to start from (default: the E2 end)')
    ap.add_argument('--words', nargs='*', default=[], help='word files replayed after the start')
    ap.add_argument('--pairs', type=int, default=0, help='candidates for the first N glued pairs only (0: all)')
    ap.add_argument('--loops', type=int, default=0, help='loops for the N admissible (d, i, c) with fewest off pieces')
    ap.add_argument('--lists', type=int, default=20,
                    help='--out keeps the piece lists of the N loops with the fewest other pieces moved')
    ap.add_argument('--out', required=True)
    ap.add_argument('--full', help='also write every loop with its piece lists here')
    a = ap.parse_args()
    R, M = O.start(a.table, a.words)
    sp = Splitter(R, M)
    out = {'start': 'E2 end of I_a@40' if a.table is None else 'current-only pose table',
           'words': [Path(w).name for w in a.words], 'source_digest': R.st.digest(), 'off_lattice': R.st.off_lattice_count(),
           'domains': sp.domains, 'off_pieces_in_a_chamber_region': sp.off_in_a_chamber_region,
           'glued_pairs': sp.pairs, 'candidates': []}
    for pair in sp.pairs[:a.pairs or None]:
        cand = sp.candidates(pair)
        log(pair=[pair['off_piece'], pair['lattice_piece']], rows=len(cand['rows']))
        if a.loops:
            cand['loops'] = sp.loops(pair, cand, a.loops)
        out['candidates'].append({'pair': [pair['off_piece'], pair['lattice_piece']], **cand})
    out['seconds'] = round(time.time() - T0)
    if a.full:
        Path(a.full).write_text(json.dumps(out) + '\n')
    lists = ('moved_pieces', 'left_lattice_pieces', 'joined_lattice_pieces')
    for cand in out['candidates']:
        legal = [l for l in cand.get('loops', []) if l['twists_admissible'] == 4]
        keep = {id(l) for l in sorted(legal, key=lambda l: (l['other_pieces_moved'], l['off_lattice_after']))[:a.lists]}
        for l in cand.get('loops', []):
            if id(l) not in keep:
                for k in lists:
                    l.pop(k, None)
    Path(a.out).write_text(json.dumps(out, indent=1) + '\n')
    print('wrote', a.out)


if __name__ == '__main__':
    main()
