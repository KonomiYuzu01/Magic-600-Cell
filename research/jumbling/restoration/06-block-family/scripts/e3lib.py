"""Shared helpers for the 3E seed table and conflict graph (menu I_a, depth-one off pieces).

Exact where it decides: menu membership of conjugated letters (exact Q(sqrt5) products), the cap status of an off
piece (sim.kernel.Evaluator on its exact pose), admissibility (the simulator). Floats only index K+ elements; every
index used in a word is confirmed by an exact product.

Notation. gamma = the first jumble element at grip c0 (default 0). Every depth-one off piece p has pose
X = K_k^-1 gamma K_r with k, r in K+; its region is K_k^-1 gamma A_s with s = r(p), the position. Positions are defined up
to S' = gamma^-1 (K+ ∩ gamma K+ gamma^-1) gamma (order 360); the position class of p is the S'-orbit of s, and two off
regions lie in one K+-orbit iff their positions lie in one class.
"""
import json
import sys
from collections import Counter, deque
from pathlib import Path

import numpy as np

REGNAMES = ['0', '1', '2|5a', '2|5b', '3', '4', '6']


def setup(J):
    J = Path(J).resolve()
    for p in (J / 'theory', J, J / 'fixtures'):
        if str(p) not in sys.path:
            sys.path.insert(0, str(p))
    sys.dont_write_bytecode = True


class Lib:
    def __init__(self, J, classes_json, arrays_npz, c0=0):
        setup(J)
        import sim
        import restore as RS
        from exact import matmul, transpose
        from sim.kernel import Evaluator, pullback_form
        from sim.kplus import to_tuple
        self.sim, self.RS, self.matmul, self.transpose, self.to_tuple = sim, RS, matmul, transpose, to_tuple
        self.Evaluator, self.pullback_form = Evaluator, pullback_form
        self.ctx = sim.get_context()
        self.d, self.reg, self.kp = self.ctx.data, self.ctx.regions, self.ctx.kplus
        self.M = RS.menu('I_a')
        self.cls = {int(k): v for k, v in json.load(open(classes_json)).items()}
        self.c0 = c0
        fm = lambda m: np.array([[float(v) for v in row] for row in m])
        self.fm = fm
        self.Kf = np.array([fm(self.kp.matrix(i)) for i in range(7200)])
        self.Kidx = {self._key(self.Kf[i]): i for i in range(7200)}
        assert len(self.Kidx) == 7200
        self.perms = self.kp.perms.astype(np.int64)
        self.inv = np.asarray(self.kp.inv, np.int64)
        self.NP = len(self.reg.orbit_of)
        self.orb = self.reg.orbit_of.astype(np.int64)
        self.tr = self.reg.transport.astype(np.int64)
        self.JI = list(RS.jumble_items(self.M))
        self.ident = RS.element(self.M, 0, 0)
        self.RI = [i for i in range(len(self.M.items)) if i not in self.JI and RS.element(self.M, 0, i) != self.ident]
        self.Gx = RS.element(self.M, c0, self.JI[0])
        self.G = fm(self.Gx)
        self.Gi = self.G.T
        # S' as pairs (tau, sigma): tau in K+ ∩ gamma K+ gamma^-1, sigma = gamma^-1 tau gamma in K+
        pairs = []
        for k in range(7200):
            kk = self.Kidx.get(self._key(self.Gi @ self.Kf[k] @ self.G))
            if kk is not None:
                pairs.append((k, kk))
        self.Spairs = np.array(pairs, np.int64)
        arr = Path(arrays_npz)
        if not arr.exists():
            self._build_arrays(arr)
        z = np.load(arr)
        self.cls_of, self.reg_code, self.cell_of, self.core_bad = z['cls_of'], z['reg_code'], z['cell_of'], z['core_bad']
        self.csize = Counter(self.cls_of.tolist())
        self._menu_cache, self._conj_cache, self._kidx_cache, self._ev_cache = {}, {}, {}, {}

    @staticmethod
    def _key(A):
        return tuple(np.round(A, 6).ravel().tolist())

    # ------------------------------------------------------------------ structure arrays (as structure.py)
    def _build_arrays(self, out):
        d, reg, kp = self.d, self.reg, self.kp
        lab = np.arange(self.NP)
        for kk in self.Spairs[:, 1]:
            img = reg.piece_of[self.orb, kp.compose_vec(np.full(self.NP, kk), self.tr)]
            np.minimum(lab, img, out=lab)
        P = self.cls[self.c0]
        classP = np.array([c for c in range(600) if self.cls[c] == P])
        repV = [np.array([[float(v) for v in vert] for vert in vs]) for vs in reg.rep_vertices]
        NF = np.array(d.NF, float)
        KAP = float(d.KAPPA)
        TOL = 1e-7

        def sig_of(V):
            vals = V @ NF[classP].T - KAP
            ins = (vals > -TOL).all(0) & (vals > TOL).any(0)
            return tuple(classP[ins].tolist())
        cell_src, cell_cap, src_cell = Counter(), Counter(), [None] * self.NP
        for s in range(self.NP):
            V = repV[self.orb[s]] @ self.Kf[self.tr[s]].T
            sg = sig_of(V @ self.G.T)
            cell_src[sg] += 1
            src_cell[s] = sg
            cell_cap[sig_of(V)] += 1
        cells = sorted(cell_src, key=lambda t: (len(t), t))
        cidx = {c: i for i, c in enumerate(cells)}
        corb = [-1] * len(cells)
        corbits = []
        for c in cells:
            if corb[cidx[c]] >= 0:
                continue
            mem = {tuple(sorted(self.perms[k, list(c)].tolist())) for k in self.Spairs[:, 0]}
            for m in mem:
                corb[cidx[m]] = len(corbits)
            corbits.append(sorted(mem))
        REG = {(0, 5280, 5280): '0', (1, 385, 385): '1', (2, 86, 86): '2|5', (3, 111, 0): '3', (4, 87, 0): '4',
               (3, 111, 111): '6'}
        names = []
        for mem in corbits:
            k_ = (len(mem[0]), cell_src[mem[0]], cell_cap.get(mem[0], 0))
            n = REG[k_]
            if n == '2|5':
                n += 'a' if not any(x.startswith('2|5') for x in names) else 'b'
            names.append(n)
        reg_code = np.array([REGNAMES.index(names[corb[cidx[src_cell[s]]]]) for s in range(self.NP)], np.int8)
        cell_of = np.array([cidx[src_cell[s]] for s in range(self.NP)], np.int32)
        allV = np.concatenate([repV[self.orb[s]] @ self.Kf[self.tr[s]].T for s in range(self.NP)])
        nr = np.linalg.norm(allV, axis=1)
        Rmax = nr.max()
        pv = []
        for x in allV[nr > Rmax - 1e-6]:
            if all(np.linalg.norm(x - y) > 1e-6 for y in pv):
                pv.append(x)
        core_v = (121 / 125) * (1 - 1e-6) * np.array(pv)

        def fw_dist(A, B, it=200000):
            z = A[0] - B[0]
            for _ in range(it):
                sa = A[np.argmin(A @ z)]
                sb = B[np.argmax(B @ z)]
                s_ = sa - sb
                if z @ s_ > 1e-12:
                    return 'sep'
                dz = s_ - z
                den = dz @ dz
                if den < 1e-30:
                    break
                lam = min(1.0, max(0.0, -(z @ dz) / den))
                z = z + lam * dz
                if np.linalg.norm(z) < 1e-10:
                    return 'int'
            return 'touch'
        core_bad = np.zeros(self.NP, bool)
        for s in np.flatnonzero(reg_code == 0).tolist():
            V = (repV[self.orb[s]] @ self.Kf[self.tr[s]].T) @ self.G.T
            vals = V @ NF.T - KAP
            if ((vals < -TOL).all(1)).any():
                core_bad[s] = True
            elif ((vals > -TOL).all(0)).any():
                pass
            elif fw_dist(V, core_v) != 'sep':
                core_bad[s] = True
        np.savez(out, cls_of=lab.astype(np.int32), reg_code=reg_code, cell_of=cell_of, core_bad=core_bad)

    # ------------------------------------------------------------------ K+ helpers
    def act(self, k, slots):
        """Images of homes/chambers under K+ element k."""
        s = np.asarray(slots, np.int64)
        return self.reg.piece_of[self.orb[s], self.kp.compose_vec(np.full(len(s), k), self.tr[s])]

    def comp(self, a, b):
        return int(self.kp.compose(int(a), int(b)))

    def kidx_letter(self, c, i):
        if (c, i) not in self._kidx_cache:
            self._kidx_cache[(c, i)] = int(self.kp.index_of_matrix(self.RS.element(self.M, c, i)))
        return self._kidx_cache[(c, i)]

    def inv_letter(self, l):
        c, i = l
        T = self.to_tuple(self.transpose(self.RS.element(self.M, c, i)))
        return (c, self.menu_index(c, T))

    def menu_index(self, c, T):
        key = (c, T)
        if key not in self._menu_cache:
            self._menu_cache[key] = next(j for j in range(len(self.M.items)) if self.RS.element(self.M, c, j) == T)
        return self._menu_cache[key]

    def conj_letter(self, m, l):
        """(c, i) -> (m(c), i') with element(m(c), i') = K_m element(c, i) K_m^-1 (exact)."""
        key = (m, l)
        if key not in self._conj_cache:
            c, i = l
            Km = self.kp.matrix(m)
            T = self.to_tuple(self.matmul(self.matmul(Km, self.RS.element(self.M, c, i)), self.transpose(Km)))
            c2 = int(self.perms[m, c])
            self._conj_cache[key] = (c2, self.menu_index(c2, T))
        return self._conj_cache[key]

    # ------------------------------------------------------------------ off pieces
    def canon(self, p, Xf):
        """Return (k, r, s) with pose = K_k^-1 gamma K_r and position s = r(p), or None if the pose is not depth one."""
        A = np.einsum('ij,kjl,lm->kim', self.Gi, self.Kf, Xf)
        keys = np.round(A, 6).reshape(7200, 16)
        for k in range(7200):
            rr = self.Kidx.get(tuple(keys[k].tolist()))
            if rr is not None:
                s = int(self.reg.piece_of[self.orb[p], self.kp.compose_vec(np.array([rr]), np.array([self.tr[p]]))[0]])
                return k, rr, s
        return None

    def describe(self, s):
        return {'position': int(s), 'class': int(self.cls_of[s]), 'class_size': int(self.csize[int(self.cls_of[s])]),
                'registry_orbit': REGNAMES[int(self.reg_code[s])], 'cell': int(self.cell_of[s]),
                'core_forbidden': bool(self.core_bad[s])}

    def conjugators(self, seed, k_p, s_p):
        """All m in K+ with m(region of the seed's off piece) = K_kp^-1 gamma A_sp, through S'."""
        s_q, k_q = seed['s'], seed['k']
        sig = self.Spairs[:, 1]
        imgs = self.reg.piece_of[np.full(len(sig), self.orb[s_q]), self.kp.compose_vec(sig, np.full(len(sig), self.tr[s_q]))]
        out = []
        for tau in self.Spairs[imgs == s_p, 0].tolist():
            m = self.comp(self.inv[k_p], self.comp(tau, k_q))
            out.append(m)
        return sorted(set(out))

    # ------------------------------------------------------------------ states
    def lattice_chambers(self, st):
        """Chamber of every lattice piece (array over pieces, -1 for off pieces)."""
        K = st._kpose()[st.pose_id]
        ch = np.full(self.NP, -1, np.int64)
        lat = np.flatnonzero(K >= 0)
        ch[lat] = self.reg.piece_of[self.orb[lat], self.kp.compose_vec(K[lat], self.tr[lat])]
        return ch

    def vacancies(self, st):
        ch = self.lattice_chambers(st)
        occ = np.zeros(self.NP, bool)
        occ[ch[ch >= 0]] = True
        assert int(occ.sum()) == int((ch >= 0).sum())
        return np.flatnonzero(~occ)

    def status(self, st, p, c):
        """Exact status of piece p (at its current pose) for the cap of grip c: 'in', 'out' or 'straddle'."""
        pid = int(st.pose_id[p])
        key = (int(p), int(c), st._pose_matrix(pid))
        if key not in self._ev_cache:
            ev = self.Evaluator(self.pullback_form(st._pose_matrix(pid), self.d.N[int(c)]))
            self._ev_cache[key] = ev.classify(self.reg.vform(int(p)))[0]
        return self._ev_cache[key]

    def clear_status_cache(self):
        self._ev_cache = {}

    def in_cap_chambers(self, c, chambers):
        ch = np.asarray(chambers, np.int64)
        return self.d.in_sig(ch, np.full(len(ch), int(c), np.int64))
