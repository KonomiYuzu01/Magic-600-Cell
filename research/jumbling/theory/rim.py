"""J3-R rim test: misalignment of a cut, decided exactly (restoration.md, section 2).

A domain D is a left coset gamma K+ of poses. A piece of D can have a 3-face on the cut hyperplane
H_c only if e = gamma^-1 n_c is a pole; in D's frame H_c is then the lattice cut H_e, tiled by
cut cells, one per pair of chambers with signatures s and s + {e} (s empty: the fixed core).
In_D(c) is the set of cut cells whose upper chamber holds a piece of D (inside c), Out_D(c) the
set whose lower chamber holds a piece of D outside c (or the core, for the lattice domain).
The misalignment N_c = sum over D of |In_D symmetric difference Out_D|, and the rim of c is
closed when N_c = 0. Every decision is exact: cosets and poles are tested in Q(sqrt5), and
chambers are found by signature arithmetic (a sum of random 64-bit weights per cap, checked
collision-free over all 177,120 signatures).

Run from the repository root:
    python research/jumbling/theory/rim.py --demo    the rim table of I_a seed 2 (restoration.md)
"""
import sys
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE.parent / 'fixtures'))

import sim  # noqa: E402
from exact import identity, matmul, matvec, transpose  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

ctx = sim.get_context()
kp, reg, data = ctx.kplus, ctx.regions, ctx.data
IDK = kp.index_of_matrix(to_tuple(identity(4)))
orbit_of, transport, piece_of = reg.orbit_of, reg.transport, reg.piece_of
NP = len(data.sigbits)
CORE = NP                                   # pseudo-chamber id of the fixed core
_W = np.random.default_rng(12345).integers(1, 2**63 - 1, size=600, dtype=np.uint64)


def _signature_hashes():
    out = np.zeros(NP, np.uint64)
    for s0 in range(0, NP, 8192):
        bits = np.unpackbits(data.sigbits[s0:s0 + 8192], axis=1)[:, :600].astype(np.uint64)
        out[s0:s0 + 8192] = (bits * _W[None, :]).sum(axis=1, dtype=np.uint64)
    return out


_H = _signature_hashes()
_ORD = np.argsort(_H)
_HS = _H[_ORD]
if len(np.unique(_HS)) != NP:
    raise RuntimeError('signature hash collision; change the seed')


def partner(Q, e, add):
    """Chamber whose signature is sig(Q) - {e} (add=False) or sig(Q) + {e} (add=True); CORE for the
    empty signature; -1 if there is no such chamber."""
    Q = np.asarray(Q, np.int64)
    e = np.asarray(e, np.int64)
    h = np.where(Q == CORE, np.uint64(0), _H[np.minimum(Q, NP - 1)])
    h = h + _W[e] if add else h - _W[e]
    pos = np.minimum(np.searchsorted(_HS, h), NP - 1)
    out = np.where(_HS[pos] == h, _ORD[pos], -1)
    if not add:
        out = np.where(h == 0, CORE, out)
    return out


def chamber_image(k, Q):
    """Images of chambers Q (CORE allowed) under k in K+."""
    Q = np.asarray(Q, np.int64)
    out = np.full(len(Q), CORE, np.int64)
    m = Q != CORE
    if m.any():
        q = Q[m]
        out[m] = piece_of[orbit_of[q], kp.compose_vec(np.full(len(q), k, np.int64), transport[q])]
    return out


class Domains:
    """The domains of a configuration, each piece's domain and its chamber in the domain's frame."""

    def __init__(self, st):
        pids = np.unique(st.pose_id).tolist()
        self.reps = [identity(4)]
        dom_of, k_of = {}, {}
        for pid in pids:
            pose = st._poses[pid]
            if pose.kidx >= 0:
                dom_of[pid], k_of[pid] = 0, pose.kidx
                continue
            M = st._pose_matrix(pid)
            for j, R in enumerate(self.reps):
                kk = kp.index_of_matrix(to_tuple(matmul(transpose(R), M)))
                if kk >= 0:
                    dom_of[pid], k_of[pid] = j, kk
                    break
            else:
                self.reps.append(M)
                dom_of[pid], k_of[pid] = len(self.reps) - 1, IDK
        self.dom = np.zeros(NP, np.int64)
        self.cham = np.zeros(NP, np.int64)
        for pid in pids:
            mem = np.nonzero(st.pose_id == pid)[0]
            self.dom[mem] = dom_of[pid]
            ks = np.full(len(mem), k_of[pid], np.int64)
            self.cham[mem] = piece_of[orbit_of[mem], kp.compose_vec(ks, transport[mem])]
        self.members = [np.nonzero(self.dom == d)[0] for d in range(len(self.reps))]
        self._pole = {}
        self._coset = {}

    def pole(self, d, c):
        """Index of the pole gamma_d^-1 n_c, or -1."""
        key = (d, c)
        if key not in self._pole:
            w = matvec(transpose(self.reps[d]), data.N[c])
            self._pole[key] = data.POLE.get(tuple(w), -1)
        return self._pole[key]

    def coset(self, gkey, g, d):
        """(j, k) with rep_j k = g rep_d, k in K+; (-1, None) if g D_d is not a present domain."""
        key = (gkey, d)
        if key not in self._coset:
            M = matmul(g, self.reps[d])
            res = (-1, None)
            for j, R in enumerate(self.reps):
                kk = kp.index_of_matrix(to_tuple(matmul(transpose(R), M)))
                if kk >= 0:
                    res = (j, kk)
                    break
            self._coset[key] = res
        return self._coset[key]

    def rim_sets(self, c, inside):
        """{domain: (e, In, Out)} for grip c; inside is a boolean mask of c's inside set."""
        out = {}
        for d in range(len(self.reps)):
            mem = self.members[d]
            e = self.pole(d, c) if mem.size else -1
            if e < 0:
                continue
            Q = self.cham[mem]
            ins = inside[mem]
            qi = Q[ins]
            up = data.in_sig(qi, np.full(len(qi), e)) if qi.size else np.zeros(0, bool)
            cin = partner(qi[up], np.full(int(up.sum()), e), add=False)
            cells_in = set(cin[cin >= 0].tolist())
            qo = Q[~ins]
            cells_out = set()
            if qo.size:
                low = qo[~data.in_sig(qo, np.full(len(qo), e))]
                cells_out = set(low[partner(low, np.full(len(low), e), add=True) >= 0].tolist())
            if d == 0:
                cells_out.add(CORE)
            out[d] = (e, cells_in, cells_out)
        return out

    def misalignment(self, c, inside, gkey='I', g=None, rs=None):
        """N_c after the twist (c, g) of the inside set (g = None: the current N_c)."""
        rs = rs if rs is not None else self.rim_sets(c, inside)
        if g is None:
            return sum(len(a ^ b) for _, a, b in rs.values())
        got = {d: set() for d in rs}
        bad = 0
        for d, (_, cells_in, _) in rs.items():
            if not cells_in:
                continue
            d2, k = self.coset(gkey, g, d)
            if d2 < 0 or d2 not in rs:
                bad += len(cells_in)
                continue
            got[d2] |= set(chamber_image(k, np.array(sorted(cells_in), np.int64)).tolist())
        return bad + sum(len(got[d] ^ rs[d][2]) for d in rs)


def demo():
    sys.path.insert(0, str(HERE))
    import restore
    M = restore.menu('I_a')
    st, hist = restore.scramble(M, 10, 2)
    D = Domains(st)
    print('history', hist)
    for c in (168, 173, 455, 236, 389, 36):
        cl = st.classify(c, need_inside=True, stop_at_straddle=True)
        if cl.straddle is not None:
            print('cap', c, 'blocked')
            continue
        inside = np.zeros(NP, bool)
        inside[cl.inside] = True
        rs = D.rim_sets(c, inside)
        now = D.misalignment(c, inside, rs=rs)
        sc = {i: D.misalignment(c, inside, (c, i), restore.element(M, c, i), rs) for i in range(len(M.items))}
        closing = [i for i, v in sc.items() if v == 0]
        print(f'cap {c}: N_c = {now}; closing twists {closing}; best otherwise {min(v for v in sc.values() if v)}')


if __name__ == '__main__':
    if '--demo' in sys.argv:
        demo()
    else:
        print(__doc__)
