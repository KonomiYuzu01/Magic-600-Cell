"""Orientation model of one class: the class puzzle with every content's accumulated rotation (R1, card 4, E).

e3_class.py tracks a content of class P by (cell, label). That is too coarse to move vacant chambers: with
label 0 a content of an edge cell has 2 orientations, of a triangle cell 3, of a tetrahedron cell 12 and of a
vertex cell 12, and which chambers of its cell a content leaves vacant depends on the orientation. This model
tracks, for every content, the element h of Sym+(P) accumulated since a reference state; its pieces then have
poses h * (reference pose), so on the lattice they occupy h applied to their reference chambers.

Coordinates. Fix the first pole c0 of class P and write N0 = N[c0]. Every pole of P is N0 w with w a unit of
2I (classes.classes). In these standard coordinates the element x -> (N0 a N0^-1) x b^-1 of Sym+(P) is the
pair (a, b) of units, with (a, b) = (-a, -b); it maps the pole w to a w b^-1. The menu element of grip c
with right factor u is (w_c u w_c^-1, u). An element lies in K+ exactly when N0 a N0^-1 is a unit, that is
when a lies in T = {a : N0 a N0^-1 in 2I} (24 units); the label of a piece is a left coset of T, label 0 = T.

Words. A word acts on the cells of P (the cell below every cut and the faces of the 600-cell on P) as a
labelled permutation (F, A, B): a content of cell f ends in cell F[f], multiplied by (A[f], B[f]). The
generators are the twists of the free grips of P: admissible grips whose cap holds only lattice pieces and
frame pieces of P (own pieces). Such twists stay admissible during any word of them (own pieces never straddle
a cut of P, card 3, B; every other piece lies outside every free cap, so it never moves), so the model is
exact for those words. `check` replays a word on the exact simulator and compares every piece: other pieces
unchanged, own pieces on the lattice by K+ index, own pieces off it by exact pose.

Start states. `start` gives the E2 end of I_a@40, or a current-only pose table (checked against its digest),
followed by word files {'word', 'input_digest', 'output_digest'} replayed with every twist checked admissible
and both digests checked. results/e-chain/ holds the handed-over words from the E2 end to 547, 499, 388 and
277 off-lattice pieces (files 1 to 4 end at 388, all six at 277).

Run from the repository root:
    python research/jumbling/theory/orient.py --cls 0 1 2 3 4 --check 8 --retained-checks 4 --out <file>
        build the model of each class at the E2 end and replay random words of its generators (all, then
        retained only) on the exact simulator; --check-words adds whole words, such as the E3 word
    python research/jumbling/theory/orient.py --words research/jumbling/theory/results/e-chain/*.json ...
        the same at the 277 state, reached from the E2 end by the e-chain words (about 9 minutes of replay)
"""
import argparse
import json
import random
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
import restore  # noqa: E402
import seams  # noqa: E402
import trees  # noqa: E402
from exact import Q5, matmul, matvec  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

T0 = time.time()
U = trees.UNITS
UIDX = {tuple(u): i for i, u in enumerate(U)}
ONE = UIDX[tuple(trees.ONEQ)]
MUL = np.array([[UIDX[tuple(trees.qmul(a, b))] for b in U] for a in U], np.int16)
INV = np.array([UIDX[tuple(trees.qconj(a))] for a in U], np.int16)
NEG = np.array([UIDX[tuple(-x for x in a)] for a in U], np.int16)
HALF_PHI = Q5(1, 1, 4)                                   # cos 36 degrees: neighbouring vertices of a 600-cell


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)


def _ray(q):
    k = next(i for i in range(4) if not q[i].is_zero())
    return (k,) + tuple(q[j] / q[k] for j in range(4))


RAY = {}
for _i, _u in enumerate(U):
    RAY.setdefault(_ray(_u), _i)


def unit_of(q):
    """Index of a unit proportional to the quaternion q (either sign)."""
    return RAY[_ray(q)]


def canon(a, b):
    """Canonical code of the element (a, b) = (-a, -b); works on arrays."""
    a = np.asarray(a, np.int32)
    b = np.asarray(b, np.int32)
    return np.minimum(a * 120 + b, NEG[a].astype(np.int32) * 120 + NEG[b])


IDENT = int(canon(ONE, ONE))


class Orient:
    def __init__(self, R, M, members, cls, P, free=None):
        self.R, self.M, self.P = R, M, P
        self.poles = poles = list(members[P])
        self.kof = {c: k for k, c in enumerate(poles)}
        self.N0, self.N0i = CL.N[poles[0]], CL.ninv(poles[0])
        self.w = np.array([UIDX[tuple(trees.qmul(self.N0i, CL.N[d]))] for d in poles], np.int16)
        self.at = {int(x): k for k, x in enumerate(self.w)}
        units = set(UIDX)
        self.T = [a for a in range(120) if tuple(self.left(a)) in units]
        assert len(self.T) == 24
        cos, reps = np.full(120, -1, np.int16), []
        for a in [ONE] + [x for x in range(120) if x != ONE]:
            if cos[a] < 0:
                for t in self.T:
                    cos[MUL[a, t]] = len(reps)
                reps.append(a)
        assert len(reps) == 5 and (cos >= 0).all()
        self.cos, self.reps = cos, np.array(reps, np.int16)
        self._kidx = {}
        self._faces()
        self._state(members, cls)
        self._generators(free)

    # ------------------------------------------------------------------ algebra
    def left(self, a):
        """The actual left factor N0 a N0^-1 of the unit a."""
        return trees.qmul(trees.qmul(self.N0, U[int(a)]), self.N0i)

    def matrix(self, a, b):
        """Exact matrix of the element (a, b)."""
        return CL.lr_matrix(self.left(a), U[int(b)])

    def kidx(self, code):
        """K+ index of an element of H_P = Sym+(P) cap K+, by canonical code."""
        code = int(code)
        if code not in self._kidx:
            a, b = divmod(code, 120)
            assert self.cos[a] == 0, 'not in K+'
            k = CL.ctx.kplus.index_of_matrix(self.matrix(a, b))
            assert k >= 0
            self._kidx[code] = k
        return self._kidx[code]

    def pole_perm(self, a, b):
        return [self.at[int(MUL[MUL[a, self.w[j]], INV[b]])] for j in range(120)]

    # ------------------------------------------------------------------ cells
    def _faces(self):
        nbr = {i for i, u in enumerate(U) if u[0] == HALF_PHI}
        assert len(nbr) == 12
        w = self.w
        adj = [[m for m in range(120) if int(MUL[INV[w[k]], w[m]]) in nbr] for k in range(120)]
        assert all(len(x) == 12 for x in adj)
        A = [set(x) for x in adj]
        edges = [(k, m) for k in range(120) for m in adj[k] if k < m]
        tris = [(k, m, n) for k, m in edges for n in sorted(A[k] & A[m]) if n > m]
        tets = [(k, m, n, o) for k, m, n in tris for o in sorted(A[k] & A[m] & A[n]) if o > n]
        assert (len(edges), len(tris), len(tets)) == (720, 1200, 600)
        self.adj = A
        self.faces = [()] + [(k,) for k in range(120)] + edges + tris + tets
        self.fid = {f: i for i, f in enumerate(self.faces)}
        self.NF = len(self.faces)
        self.ftype = np.array([len(f) for f in self.faces], np.int8)
        self.incap = np.zeros((120, self.NF), bool)
        for i, f in enumerate(self.faces):
            for k in f:
                self.incap[k, i] = True

    def face(self, grips):
        return self.fid[tuple(sorted(self.kof[c] for c in grips))]

    def grips(self, f):
        return [self.poles[k] for k in self.faces[f]]

    # ------------------------------------------------------------------ state
    def _state(self, members, cls):
        R, st = self.R, self.R.st
        D = R.domains()
        dom = np.asarray(D.dom)
        dcls = {}
        for d in range(1, len(D.reps)):
            h = CL.hub(D.reps[d], members, cls)
            dcls[d] = h[0][1] if len(h) == 1 else None
        frag = [d for d, v in dcls.items() if v == self.P]
        own = (dom == 0) | np.isin(dom, frag)
        NP = len(dom)
        sig = np.zeros((NP, 120), bool)
        self.blocked, self.free_at_start = [], []
        for k, c in enumerate(self.poles):
            cl = st.classify(int(c), need_inside=True, stop_at_straddle=False)
            if cl.uncertain:
                raise AssertionError(f'uncertain classification at grip {c}')
            ins = np.asarray(cl.inside, np.int64)
            sig[ins, k] = True
            if cl.straddle is not None:
                self.blocked.append(int(c))
            elif not (~own)[ins].any():
                self.free_at_start.append(int(c))
        self.face_of_piece = np.full(NP, -1, np.int64)
        for p in np.flatnonzero(own).tolist():
            key = tuple(np.flatnonzero(sig[p]).tolist())
            if key not in self.fid:
                raise AssertionError(f'piece {p} lies in caps {key} of class {self.P}, which are not a face')
            self.face_of_piece[p] = self.fid[key]
        self.label = np.zeros(NP, np.int8)
        self.frag = {}
        for d in frag:
            l, r = trees.factors(D.reps[d])
            assert E3.prop_unit(r)
            js = [j for j in range(5) if E3.prop_unit(trees.qmul(trees.qconj(self.left(self.reps[j])), l))]
            assert len(js) == 1 and js[0] != 0
            self.label[D.members[d]] = js[0]
            self.frag[d] = {'pieces': len(D.members[d]), 'label': js[0]}
        self.pose_id0 = st.pose_id.copy()
        self.keys0 = [q.key for q in st._poses]
        self.kpose0 = st._kpose()
        self.K0 = self.kpose0[self.pose_id0]
        # the simulator renumbers its pose table after every twist, so keep the start matrices themselves
        self.start_pose = {int(i): st._pose_matrix(int(i))
                           for i in np.unique(self.pose_id0[own & (self.K0 < 0)]).tolist()}
        reg, kp = CL.ctx.regions, CL.ctx.kplus
        lat = np.flatnonzero(self.K0 >= 0)
        ch = reg.piece_of[reg.orbit_of[lat], kp.compose_vec(self.K0[lat], reg.transport[lat])]
        if len(set(ch.tolist())) != len(lat):
            raise AssertionError('two lattice pieces in one chamber')
        self.chamber = np.full(NP, -1, np.int64)
        self.chamber[lat] = ch
        off, cp = np.asarray(CL.data.cap_offsets), np.asarray(CL.data.cap_piece)
        home = np.zeros((NP, 120), bool)
        for k, c in enumerate(self.poles):
            home[cp[off[c]:off[c + 1]], k] = True
        self.home = home
        if not (home[ch] == sig[lat]).all():
            raise AssertionError('chamber of a lattice piece disagrees with its caps of class P')
        self.occupied = np.zeros(NP, bool)
        self.occupied[ch] = True
        self.chamber_face = np.array([self.fid.get(tuple(np.flatnonzero(r).tolist()), -1) for r in home], np.int64)
        self.content = {}
        for p in np.flatnonzero(own).tolist():
            self.content.setdefault(int(self.face_of_piece[p]), []).append(p)
        self.content = {f: np.array(v, np.int64) for f, v in self.content.items()}
        self.own = own
        log(cls=self.P, own_contents=len(self.content), free=len(self.free_at_start), blocked=self.blocked,
            fragments=self.frag, vacant=int((~self.occupied).sum()))

    # ------------------------------------------------------------------ generators
    def _generators(self, free):
        A = self.free_at_start if free is None else list(free)
        self.free = A
        NI = len(self.M.items)
        NF = self.NF
        gens, F, GA, GB, elem, byc = [], [], [], [], [], {}
        rng = random.Random(1)
        for c in A:
            k = self.kof[c]
            wc = int(self.w[k])
            cap = np.flatnonzero(self.incap[k])
            for i in range(NI):
                g = restore.element(self.M, c, i)
                _, r = trees.factors(g)
                u = unit_of(r)
                a, b = int(MUL[MUL[wc, u], INV[wc]]), u
                if self.matrix(a, b) != g:
                    raise AssertionError(f'menu element ({c}, {i}) is not (w u w^-1, u)')
                if int(canon(a, b)) == IDENT:
                    continue
                perm = self.pole_perm(a, b)
                if rng.random() < 0.02:                         # spot check against the exact pole images
                    img = [self.kof[CL.pole(matvec(g, CL.N[d]))] for d in self.poles]
                    assert img == perm
                fmap = np.arange(NF, dtype=np.int32)
                for f in cap.tolist():
                    fmap[f] = self.fid[tuple(sorted(perm[j] for j in self.faces[f]))]
                ga = np.full(NF, ONE, np.int16)
                gb = np.full(NF, ONE, np.int16)
                ga[cap], gb[cap] = a, b
                byc.setdefault(c, []).append(len(gens))
                gens.append((int(c), int(i)))
                F.append(fmap)
                GA.append(ga)
                GB.append(gb)
                elem.append((a, b))
        self.gens, self.byc = gens, byc
        self.GF, self.GA, self.GB = np.array(F), np.array(GA), np.array(GB)
        self.elem = np.array(elem, np.int16)
        self.retained = self.cos[self.elem[:, 0]] == 0         # in K+: every content stays on the lattice
        code = {}
        for n, (c, _) in enumerate(gens):
            code[(c, int(canon(*elem[n])))] = n
        self.inv = np.array([code[(c, int(canon(INV[elem[n][0]], INV[elem[n][1]])))] for n, (c, _) in enumerate(gens)])
        self.gid = {g: n for n, g in enumerate(gens)}
        log(generators=len(gens))

    # ------------------------------------------------------------------ words
    def identity(self):
        return (np.arange(self.NF, dtype=np.int32), np.full(self.NF, ONE, np.int16), np.full(self.NF, ONE, np.int16))

    def labelled(self, word, start=None):
        F, A, B = self.identity() if start is None else start
        for g in word:
            A = MUL[self.GA[g][F], A]
            B = MUL[self.GB[g][F], B]
            F = self.GF[g][F]
        return F, A, B

    @staticmethod
    def compose(L1, L2):
        """L1 then L2."""
        F1, A1, B1 = L1
        F2, A2, B2 = L2
        return F2[F1], MUL[A2[F1], A1], MUL[B2[F1], B1]

    @staticmethod
    def invert(L):
        F, A, B = L
        Fi = np.empty_like(F)
        Fi[F] = np.arange(len(F), dtype=F.dtype)
        return Fi, INV[A[Fi]], INV[B[Fi]]

    def word_inverse(self, word):
        return [int(self.inv[g]) for g in reversed(word)]

    def support(self, L, faces=None):
        """Cells moved or rotated by a labelled permutation (all cells, or the given ones)."""
        F, A, B = L
        f = np.arange(self.NF) if faces is None else np.asarray(faces)
        return f[(F[f] != f) | (canon(A[f], B[f]) != IDENT)]

    # ------------------------------------------------------------------ predictions
    def start_matrix(self, p):
        """Exact pose of piece p at the reference state."""
        return self.start_matrix_of(int(self.pose_id0[p]))

    def start_matrix_of(self, pid):
        """Exact matrix of pose pid of the reference state's pose table."""
        k = int(self.kpose0[pid])
        return CL.ctx.kplus.matrix(k) if k >= 0 else self.start_pose[pid]

    def predict(self, L, pieces):
        """Predicted (on lattice, K+ index or -1) of own pieces after a word with labelled permutation L."""
        F, A, B = L
        f = self.face_of_piece[pieces]
        a, b = A[f], B[f]
        lab = self.cos[MUL[a, self.reps[self.label[pieces]]]]
        on = lab == 0
        K = np.full(len(pieces), -1, np.int64)
        kp = CL.ctx.kplus
        was = self.K0[pieces] >= 0
        codes = canon(a, b)
        for code in np.unique(codes[on & was]).tolist():
            sel = np.flatnonzero(on & was & (codes == code))
            K[sel] = kp.compose_vec(np.full(len(sel), self.kidx(code)), self.K0[pieces[sel]])
        for n in np.flatnonzero(on & ~was).tolist():
            p = int(pieces[n])
            m = matmul(self.matrix(a[n], b[n]), self.start_matrix(p))
            K[n] = kp.index_of_matrix(m)
            assert K[n] >= 0
        return on, K

    def chambers_after(self, L, pieces):
        """Predicted chambers of own pieces after the word (-1 off the lattice)."""
        on, K = self.predict(L, pieces)
        reg = CL.ctx.regions
        out = np.full(len(pieces), -1, np.int64)
        s = np.flatnonzero(on)
        out[s] = reg.piece_of[reg.orbit_of[pieces[s]], CL.ctx.kplus.compose_vec(K[s], reg.transport[pieces[s]])]
        return out

    # ------------------------------------------------------------------ exact check
    def check(self, word):
        """Replay a word of generator indices on the exact simulator and compare every piece with the model."""
        R, st = self.R, self.R.st
        L = self.labelled(word)
        pid0, keys0 = st.pose_id.copy(), [q.key for q in st._poses]
        code = {}
        c0 = np.array([code.setdefault(k, len(code)) for k in keys0])[pid0]
        assert (c0 == self.code0(code)).all(), 'the simulator is not at the reference state'
        done = 0
        try:
            for g in word:
                R.apply(*self.gens[g])                  # raises if a twist is not admissible
                done += 1
            pid1 = st.pose_id.copy()
            c1 = np.array([code.setdefault(q.key, len(code)) for q in st._poses])[pid1]
            K1 = st._kpose()[pid1]
            same = c0 == c1
            own = np.flatnonzero(self.face_of_piece >= 0)
            other = np.flatnonzero(self.face_of_piece < 0)
            F, A, B = L
            f = self.face_of_piece[own]
            ident = canon(A[f], B[f]) == IDENT
            on, K = self.predict(L, own)
            res = {'twists': len(word),
                   'other_pieces_unchanged': bool(same[other].all()),
                   'unmoved_own_pieces_unchanged': bool(same[own[ident]].all()),
                   'lattice_flags_agree': bool(((K1[own] >= 0) == on).all()),
                   'lattice_poses_agree': bool((K1[own[on]] == K[on]).all())}
            # every own piece now off the lattice: predicted (a, b) * start pose against its exact pose, computed
            # once per (start pose, content element) and once per current pose
            idx = np.flatnonzero(~on)
            pcs = own[idx]
            keys = self.pose_id0[pcs].astype(np.int64) * 14400 + canon(A[f[idx]], B[f[idx]])
            uk, ki = np.unique(keys, return_inverse=True)
            up, pi = np.unique(pid1[pcs], return_inverse=True)
            tid = {}
            pred = np.array([tid.setdefault(to_tuple(matmul(self.matrix(*divmod(int(k) % 14400, 120)),
                                                            self.start_matrix_of(int(k) // 14400))), len(tid))
                             for k in uk.tolist()], np.int64)
            act = np.array([tid.setdefault(to_tuple(st._pose_matrix(int(x))), len(tid)) for x in up.tolist()], np.int64)
            exact = int((pred[ki] == act[pi]).sum()) if len(idx) else 0
            res['off_lattice_pieces'] = len(idx)
            res['off_lattice_poses_exact'] = f'{exact}/{len(idx)}'
            res['ok'] = all(v for k, v in res.items() if isinstance(v, bool)) and exact == len(idx)
            return res
        finally:
            for _ in range(done):
                R.undo()
            back = np.array([code.setdefault(q.key, len(code)) for q in st._poses])[st.pose_id]
            assert (back == c0).all()

    def code0(self, code):
        """Pose codes of the reference state under a shared key numbering."""
        return np.array([code.setdefault(k, len(code)) for k in self.keys0])[self.pose_id0]

def set_pose_table(st, doc):
    """Set a simulator state to a current-only pose table {'poses': [exact matrices], 'pose_id': [...]}."""
    from sim.kplus import matrix_from_json
    from sim.state import Pose
    st._poses, st._lookup = [Pose(0, None, ('K', 0))], {('K', 0): 0}
    ids = np.array([st._intern_matrix(matrix_from_json(m)) for m in doc['poses']], np.int32)
    st.pose_id = ids[np.asarray(doc['pose_id'], np.int64)]
    st._compact()
    st.journal, st._anchor_cache = [], {}
    if 'digest' not in doc:
        raise AssertionError('pose table has no digest to check')
    if st.digest() != doc['digest']:
        raise AssertionError('pose table does not reproduce its digest')


def start(table=None, words=()):
    """Restorer at the E2 end of I_a@40 or at a current-only pose table, then after word files replayed with
    every twist checked admissible; each word file {'word', 'input_digest', 'output_digest'} is checked at both ends."""
    if table is None:
        R, M = E3.e2_end()
    else:
        st, M = seams._fixture_state('I_a', 40)
        set_pose_table(st, json.loads(Path(table).read_text()))
        R = seams.SeamRestorer(st, M)
    for name in words:
        doc = json.loads(Path(name).read_text())
        if R.st.digest() != doc['input_digest']:
            raise AssertionError(f'{name}: the state before the word is not its input state')
        for c, i in doc['word']:
            R.apply(c, i)
        if R.st.digest() != doc['output_digest']:
            raise AssertionError(f'{name}: the word does not end at its output state')
        log(replayed=name, twists=len(doc['word']), off=R.st.off_lattice_count())
    return R, M


def build(P=1, moves=(), table=None, free=None, words=()):
    """Orientation model of class P at a start state (see `start`), optionally after further moves."""
    members, cls = CL.classes()
    R, M = start(table, words)
    for c, i in moves:
        R.apply(c, i)
    return Orient(R, M, members, cls, P, free), members, cls


def validate(o, n_all, n_retained, length, seed=7):
    """Random words of all generators and of retained generators, each replayed on the simulator."""
    rng = random.Random(seed)
    pools = [('all', list(range(len(o.gens))), n_all), ('retained', np.flatnonzero(o.retained).tolist(), n_retained)]
    rows = []
    for kind, pool, n in pools:
        for k in range(n):
            word = [rng.choice(pool) for _ in range(rng.randint(1, length))]
            r = dict(kind=kind, **o.check(word))
            rows.append(r)
            log(cls=o.P, check=len(rows), **r)
    return rows


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--cls', type=int, nargs='+', default=[1])
    ap.add_argument('--table', help='current-only pose table to start from (default: the E2 end)')
    ap.add_argument('--words', nargs='*', default=[], help='word files replayed after the start')
    ap.add_argument('--check', type=int, default=0, help='random words of all generators checked per class')
    ap.add_argument('--retained-checks', type=int, default=0, help='random retained words checked per class')
    ap.add_argument('--length', type=int, default=30)
    ap.add_argument('--check-words', nargs='*', default=[],
                    help="files with a 'word' of (grip, menu index) twists, checked as whole words by every selected "
                         "class whose generators contain all of its twists; a word no selected class checks fails the run")
    ap.add_argument('--out', help='write the summary and the checks here')
    a = ap.parse_args()
    members, cls = CL.classes()
    R, M = start(a.table, a.words)
    out = {'start': 'E2 end of I_a@40' if a.table is None else 'current-only pose table',
           'words': [Path(w).name for w in a.words], 'digest': R.st.digest(), 'off_lattice': R.st.off_lattice_count(),
           'classes': []}
    checked_by = {name: [] for name in a.check_words}      # keyed by the path as given, so distinct files never merge
    for P in a.cls:
        o = Orient(R, M, members, cls, P)
        rows = validate(o, a.check, a.retained_checks, a.length)
        skipped = []
        for name in a.check_words:
            twists = [(int(c), int(i)) for c, i in json.loads(Path(name).read_text())['word']]
            if not all(t in o.gid for t in twists):
                skipped.append(name)                             # not a word of this class's generators
                continue
            rows.append(dict(kind=name, **o.check([o.gid[t] for t in twists])))
            checked_by[name].append(P)
            log(cls=P, check=len(rows), **rows[-1])
        out['classes'].append({
            'class': P, 'free_grips': len(o.free), 'blocked_grips': o.blocked,
            'fragments': [{'pieces': v['pieces'], 'label': v['label']} for v in o.frag.values()],
            'own_contents': len(o.content), 'generators': len(o.gens), 'retained_generators': int(o.retained.sum()),
            'checks': rows, 'skipped_words': skipped, 'all_ok': bool(rows) and all(r['ok'] for r in rows)})
    out['check_words_checked_by'] = checked_by
    out['unchecked_words'] = [name for name, by in checked_by.items() if not by]
    out['all_ok'] = bool(out['classes']) and all(c['all_ok'] for c in out['classes']) and not out['unchecked_words']
    out['seconds'] = round(time.time() - T0)
    text = json.dumps(out, indent=1) + '\n'
    if a.out:
        Path(a.out).write_text(text)
    print(json.dumps({'digest': out['digest'], 'classes': [(c['class'], c['free_grips'], len(c['checks']), c['all_ok'])
                                                           for c in out['classes']],
                      'unchecked_words': out['unchecked_words'], 'all_ok': out['all_ok']}))
    sys.exit(0 if out['all_ok'] else 1)


if __name__ == '__main__':
    main()
