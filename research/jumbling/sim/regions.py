"""Exact piece regions: one double description per K+-orbit representative, exact transport.

Home region of piece A (state contract section 2):
    A0 = cl(int P ∩ ⋂_{c in M(A)} U_c ∩ ⋂_{c not in M(A)} {n_c . x < kappa}).
For a rotation r in K+ (a symmetry of the pole set, hence of P and of every cut), r(A0(M)) is
the chamber with signature r(M). So once the region of one representative of a K+-orbit is
known exactly, the region of every other piece B = r(rep) is r applied to its vertices. The
transport element r_B is found by matching signatures, which is a combinatorial identity; the
geometric statement then follows from r_B being an exact pole symmetry.

Representatives lie in cap 0, so their regions are computed by `witness.build_region` with the
candidate constraints of cap 0. Constraints that are dropped there are implied by the cap
region, which `witness.cap_region` certifies against every facet; `certify` additionally checks
every representative vertex against all 600 cut and 600 facet constraints exactly.
"""
from collections import OrderedDict

import numpy as np

import witness as W
from exact import affine_rank, dot, matvec, rank

from .kernel import KAPPA_FORM, float_rows, rows_form, sign5_vec
from .model import NP


class LRU(OrderedDict):
    def __init__(self, maxsize):
        super().__init__()
        self.maxsize = maxsize

    def get_or(self, key, make):
        if key in self:
            self.move_to_end(key)
            return self[key]
        v = make()
        self[key] = v
        if len(self) > self.maxsize:
            self.popitem(last=False)
        return v


def transport_rows(intmat, vform):
    """Integer rows of r v for an int-form K+ matrix r and an int-form vertex set."""
    g1, g2, d = intmat
    e, rows = vform
    a = np.array(rows, dtype=np.int64)
    x1, x2 = a[:, :4], a[:, 4:]
    y1 = x1 @ g1.T + 5 * (x2 @ g2.T)
    y2 = x2 @ g1.T + x1 @ g2.T
    out = np.hstack([y1, y2])
    return d * e, [tuple(r) for r in out.tolist()]


class Regions:
    def __init__(self, data, kp, cache_size=60000):
        self.data = data
        self.kp = kp
        self.report = {}
        self._orbits()
        self._build()
        self._vcache = LRU(cache_size)
        self._fcache = LRU(cache_size)
        self._xcache = LRU(cache_size // 4)
        self._capcache = {}
        self._capfloat = {}

    # ------------------------------------------------------------------------------------
    def _orbits(self):
        data, kp = self.data, self.kp
        mo, mv = data.mask_offsets, data.mask_values
        sig_index = {tuple(mv[mo[p]:mo[p + 1]].tolist()): p for p in range(NP)}
        if len(sig_index) != NP:
            raise ValueError('signatures are not distinct')
        orbit_of = -np.ones(NP, np.int64)
        transport = -np.ones(NP, np.int64)
        reps, rows = [], []
        perms = kp.perms.astype(np.int64)
        for p in data.cap_members(0):
            p = int(p)
            if orbit_of[p] >= 0:
                continue
            o = len(reps)
            sig = np.array(data.signature_list(p))
            imgs = np.sort(perms[:, sig], axis=1)
            row = np.array([sig_index.get(tuple(r)) if tuple(r) in sig_index else -1
                            for r in imgs.tolist()], np.int64)
            if np.any(row < 0):
                raise ValueError('the signature set is not closed under K+')
            uniq, first = np.unique(row, return_index=True)
            orbit_of[uniq] = o
            transport[uniq] = first
            reps.append(p)
            rows.append(row)
        if np.any(orbit_of < 0):
            raise ValueError('some K+-orbit does not meet cap 0')
        self.reps = reps
        self.piece_of = np.array(rows, np.int64)       # (orbits, 7200)
        self.orbit_of = orbit_of
        self.transport = transport
        # K+-orbits against the retained G-orbit classes (orbit_id; -1 = fixed centres)
        union = {}
        for o in range(len(reps)):
            ids = sorted(set(data.orbit_id[self.piece_of[o]].tolist()))
            union[o] = ids
        classes = {}
        for o, ids in union.items():
            for i in ids:
                classes.setdefault(i, set()).add(o)
        self.report['kplus_orbits'] = len(reps)
        self.report['orbit_sizes'] = [int(len(np.unique(r))) for r in rows]
        self.report['kplus_orbit_g_classes'] = {str(o): ids for o, ids in union.items()}
        self.report['g_class_in_one_kplus_orbit'] = all(len(v) == 1 for v in classes.values())

    def _build(self):
        data = self.data
        cap = W.cap_region(0)      # asserts every facet constraint at every vertex
        near = [e for e in range(600) if data.NF[e] @ data.NF[0] > 0.6 * (data.NF[0] @ data.NF[0])]
        out = W.implied(cap)
        candp = [e for e in range(600) if e not in out and e != 0]
        self.cap_vertices = cap
        self.cap_vform0 = self._exact_rows(cap)
        built = [W.build_region((p, candp, near)) for p in self.reps]
        self.rep_vertices, self.rep_hosts, self.rep_vform = [], [], []
        host_ok = full_ok = 0
        for (p, verts, full, host_dim) in built:
            three = sorted(e for e, k in host_dim.items() if k == 3)
            host_ok += three == data.hosts(p)
            full_ok += bool(full)
            self.rep_vertices.append(verts)
            self.rep_hosts.append(three)
            self.rep_vform.append(self._exact_rows(verts))
        # a far facet cannot carry a 3-dimensional patch of any piece region inside cap 0
        far_rank = 0
        for e in range(600):
            if e in near:
                continue
            on = [v for v in cap if (dot(data.N[e], v) - data.NN).is_zero()]
            if on:
                far_rank = max(far_rank, affine_rank(on))
        self.report.update({
            'cap_region_vertices': len(cap), 'cap_candidate_poles': len(candp), 'cap_near_facets': len(near),
            'far_facet_max_patch_dim_in_cap': far_rank,
            'rep_full_dimensional': f'{full_ok}/{len(self.reps)}',
            'rep_host_patches_match_retained': f'{host_ok}/{len(self.reps)}',
            'rep_vertex_counts': [len(v) for v in self.rep_vertices],
        })
        if full_ok != len(self.reps) or host_ok != len(self.reps) or far_rank >= 3:
            raise ValueError('representative region reconstruction failed its checks')

    @staticmethod
    def _exact_rows(points):
        return rows_form(points)

    def certify(self):
        """Exact checks of the representative regions, and their transport to every piece.

        1. every representative vertex satisfies all 600 cut constraints (inside for the poles
           of its signature, outside otherwise) and all 600 facet constraints;
        2. every vertex is a vertex: its tight constraints have rank 4;
        3. every representative touches every cut of its signature exactly (min h = 0);
        4. for all 177,120 pieces, r_p(signature(rep)) = signature(p) and
           r_p(hosts(rep)) = hosts(p), so the transported region is the chamber of p and its
           transported 3-dimensional facet patches are exactly the retained stickers."""
        data, kp = self.data, self.kp
        n1, n2 = data.N1, data.N2
        bad_constraint = bad_vertex = 0
        exact_contact = []
        for o, (p, (e, rows)) in enumerate(zip(self.reps, self.rep_vform)):
            a = np.array(rows, np.int64)
            x1, x2 = a[:, :4], a[:, 4:]
            p1 = x1 @ n1.T + 5 * (x2 @ n2.T)            # n_q . v = (p1 + p2 sqrt5) / (2e)
            p2 = x2 @ n1.T + x1 @ n2.T
            assert np.abs(p1).max() < 2 ** 24 and np.abs(p2).max() < 2 ** 24
            facet = sign5_vec(p1 - 24 * e, p2 - 8 * e)                      # n.v - (12 + 4 sqrt5)
            cut = sign5_vec(KAPPA_FORM[2] * p1 - 2 * e * KAPPA_FORM[0], KAPPA_FORM[2] * p2 - 2 * e * KAPPA_FORM[1])
            sig = np.zeros(600, bool)
            sig[data.signature_list(p)] = True
            ok = (facet <= 0).all() and (cut[:, sig] >= 0).all() and (cut[:, ~sig] <= 0).all()
            bad_constraint += not ok
            # tight constraints of rank 4 at every vertex
            for j, v in enumerate(self.rep_vertices[o]):
                tight = [data.N[q] for q in range(600) if facet[j, q] == 0 or cut[j, q] == 0]
                if rank(tight) != 4:
                    bad_vertex += 1
            # exact contact: every cut of the signature has min h = 0 over the vertices, so by
            # transport every piece touches every cut of its signature exactly (E0 contact)
            exact_contact.append(bool(all((cut[:, q] == 0).any() for q in np.nonzero(sig)[0])))
        # transport of signatures and hosts to every piece (combinatorial)
        perms = kp.perms.astype(np.int64)
        sig_fail = host_fail = 0
        for o, p in enumerate(self.reps):
            members = np.nonzero(self.orbit_of == o)[0]
            r = perms[self.transport[members]]
            sig = np.array(data.signature_list(p))
            got = np.sort(r[:, sig], axis=1)
            mo = data.mask_offsets
            want = np.stack([data.mask_values[mo[q]:mo[q + 1]] for q in members])
            sig_fail += int((got != want).any(axis=1).sum())
            hs = np.array(self.rep_hosts[o])
            goth = np.sort(r[:, hs], axis=1)
            fo = data.face_offsets
            wanth = np.stack([data.face_values[fo[q]:fo[q + 1]] for q in members])
            host_fail += int((goth != wanth).any(axis=1).sum())
        return {
            'rep_vertices_violating_a_constraint': bad_constraint,
            'rep_vertices_without_rank4_tight_set': bad_vertex,
            'reps_touching_every_signature_cut_exactly': f'{sum(exact_contact)}/{len(self.reps)}',
            'pieces_with_transported_signature_mismatch': sig_fail,
            'pieces_with_transported_host_mismatch': host_fail,
        }

    # ------------------------------------------------------------------------------------
    def rep_of(self, p):
        return int(self.orbit_of[p]), int(self.transport[p])

    def vertices(self, p):
        """Exact vertices of the home region of piece p (transported, cached)."""
        def make():
            o, k = self.rep_of(p)
            m = self.kp.matrix(k)
            return [matvec(m, v) for v in self.rep_vertices[o]]
        return self._xcache.get_or(int(p), make)

    def vform(self, p):
        """Integer form (E, rows) of the home vertices of piece p (transported, cached)."""
        def make():
            o, k = self.rep_of(p)
            return transport_rows(self.kp.int_matrix(k), self.rep_vform[o])
        return self._vcache.get_or(int(p), make)

    def frows(self, p):
        return self._fcache.get_or(int(p), lambda: float_rows(self.vform(p)))

    def cap_vform(self, a):
        """Integer form of the vertices of the cap region of pole a (Frame[a] of cap 0)."""
        f = self._capcache.get(a)
        if f is None:
            f = transport_rows(self.kp.int_matrix(int(self.kp.frame_idx[a])), self.cap_vform0)
            self._capcache[a] = f
        return f

    def cap_frows(self, a):
        f = self._capfloat.get(a)
        if f is None:
            f = float_rows(self.cap_vform(a))
            self._capfloat[a] = f
        return f
