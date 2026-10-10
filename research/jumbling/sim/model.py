"""Read-only model data for the jumbling simulator.

Everything comes from `assets/model.npz` (through `witness.py`, which loads it read-only) and,
for the retained slot moves used only in comparisons, `assets/primitives.npz`. Exact poles are
recovered with `exact.from_float`, which rejects any entry that is not a unique
(p + q sqrt5)/2 value.
"""
import hashlib
from io import BytesIO

import numpy as np

import witness as W
from exact import Q5

NP = 177120          # pieces, cell centres included
NS = 259800          # labelled sticker slots
CELL_SLOTS = 433     # slots per cell; slot s lies on facet s // 433
CONTRACT_REVISION = 'state-contract 2026-10-09 A1-A4'


class ModelData:
    def __init__(self):
        self.root = W.ROOT
        model_bytes = (self.root / 'assets' / 'model.npz').read_bytes()
        self._primitive_bytes = (self.root / 'assets' / 'primitives.npz').read_bytes()
        self.identity = {'assets/model.npz': hashlib.sha256(model_bytes).hexdigest(),
                         'assets/primitives.npz': hashlib.sha256(self._primitive_bytes).hexdigest()}
        m = np.load(BytesIO(model_bytes))
        # The region builder uses witness's read-only model. It must describe the same bytes.
        for key in ('normals', 'mask_offsets', 'mask_values', 'face_offsets', 'face_values'):
            if not np.array_equal(m[key], W.M[key]):
                raise ValueError('model changed after witness loaded it')
        self.NF = W.NF
        self.N = W.N                               # exact poles, lists of Q5
        assert len(set(W.NN)) == 1, 'poles must share one length'
        self.NN = W.NN[0]                          # |n|^2 = 12 + 4 sqrt5
        self.KAPPA = W.KAPPA[0]                    # alpha |n|^2
        self.ALPHA = W.ALPHA
        assert self.NN == Q5(12, 4, 1) and self.KAPPA == Q5(1452, 484, 125)
        self.POLE = W.POLE                         # exact pole tuple -> index
        self.mask_offsets = m['mask_offsets'].astype(np.int64)
        self.mask_values = m['mask_values'].astype(np.int64)
        self.face_offsets = m['face_offsets'].astype(np.int64)
        self.face_values = m['face_values'].astype(np.int64)
        self.slot_piece = m['slot_piece'].astype(np.int64)
        self.orbit_id = m['orbit_id'].astype(np.int64)
        self.centre_pieces = np.flatnonzero(self.orbit_id == -1)
        self.centre_poles = self.mask_values[self.mask_offsets[self.centre_pieces]]
        assert len(self.centre_pieces) == 600 and np.all(np.diff(self.mask_offsets)[self.centre_pieces] == 1)
        self.centre_slots = np.flatnonzero(self.orbit_id[self.slot_piece] == -1)
        assert len(self.centre_slots) == 600
        self.rotperms = m['rotperms'].astype(np.int64)
        self.move_src = m['move_src'].astype(np.int64)
        self.move_dst = m['move_dst'].astype(np.int64)
        self.frames = m['frames']
        self.frameperms = m['frameperms'].astype(np.int64)
        self.base_twists = m['base_twists']
        assert len(self.mask_offsets) == NP + 1 and len(self.slot_piece) == NS
        self.basis = [int(x) for x in W.basis_poles(0)]
        # integer pole forms: n = (N1 + N2 sqrt5) / 2
        n1 = np.zeros((600, 4), np.int64)
        n2 = np.zeros((600, 4), np.int64)
        for i, row in enumerate(self.N):
            for j, x in enumerate(row):
                assert 2 % x.d == 0
                n1[i, j] = x.a * (2 // x.d)
                n2[i, j] = x.b * (2 // x.d)
        self.N1, self.N2 = n1, n2
        self.pole_key = {(tuple(n1[i]), tuple(n2[i])): i for i in range(600)}
        # cap membership: the pieces whose signature contains c, as CSR
        lengths = np.diff(self.mask_offsets)
        owner = np.repeat(np.arange(NP, dtype=np.int64), lengths)
        order = np.lexsort((owner, self.mask_values))
        self.cap_piece = owner[order]
        self.cap_offsets = np.searchsorted(self.mask_values[order], np.arange(601))
        # packed signature bits, packbits order (bit 7 - c % 8 of byte c // 8)
        bits = np.zeros((NP, 75), np.uint8)
        np.bitwise_or.at(bits, (owner, self.mask_values >> 3),
                         (128 >> (self.mask_values & 7)).astype(np.uint8))
        self.sigbits = bits
        # (piece, facet) -> slot
        keys = self.slot_piece * 600 + np.arange(NS, dtype=np.int64) // CELL_SLOTS
        self.slot_key_order = np.argsort(keys, kind='stable')
        self.slot_keys_sorted = keys[self.slot_key_order]
        assert np.all(np.diff(self.slot_keys_sorted) > 0), 'a piece has two slots on one facet'
        m.close()

    # ------------------------------------------------------------------------------------
    def signature(self, p):
        return frozenset(int(x) for x in self.mask_values[self.mask_offsets[p]:self.mask_offsets[p + 1]])

    def signature_list(self, p):
        return [int(x) for x in self.mask_values[self.mask_offsets[p]:self.mask_offsets[p + 1]]]

    def hosts(self, p):
        return [int(x) for x in self.face_values[self.face_offsets[p]:self.face_offsets[p + 1]]]

    def cap_members(self, c):
        return self.cap_piece[self.cap_offsets[c]:self.cap_offsets[c + 1]]

    def in_sig(self, pieces, caps):
        """Vectorised test: caps[i] in signature(pieces[i])."""
        pieces = np.asarray(pieces, np.int64)
        caps = np.asarray(caps, np.int64)
        return ((self.sigbits[pieces, caps >> 3] >> (7 - (caps & 7))) & 1).astype(bool)

    def sig_matrix(self, pieces):
        """Boolean (len(pieces), 600) signature matrix."""
        return np.unpackbits(self.sigbits[np.asarray(pieces, np.int64)], axis=1)[:, :600].astype(bool)

    def slot_of(self, pieces, facets):
        """Vectorised slot lookup for (piece, facet) pairs; raises if a pair has no slot."""
        keys = np.asarray(pieces, np.int64) * 600 + np.asarray(facets, np.int64)
        pos = np.searchsorted(self.slot_keys_sorted, keys)
        pos = np.minimum(pos, NS - 1)
        if not np.array_equal(self.slot_keys_sorted[pos], keys):
            raise KeyError('no slot for some (piece, facet) pair')
        return self.slot_key_order[pos]

    def h(self, e, x):
        """Exact signed offset of point x from the cut of pole e (fraction of facet distance)."""
        return W.h(e, x)

    def primitives(self):
        """Retained slot moves (src, dst, offsets) from assets/primitives.npz, read-only."""
        with np.load(BytesIO(self._primitive_bytes)) as z:
            return z['src'].astype(np.int64), z['dst'].astype(np.int64), z['offsets'].astype(np.int64)
