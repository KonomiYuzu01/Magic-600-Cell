"""Configurations of the rigid jumbling puzzle (state contract sections 2, 3 and 5).

Representation. Every piece has an exact pose. Poses are interned in a pose table and each
piece stores a pose id (np.int32); id 0 is the identity, so no matrix is stored for a piece in
its home pose. A pose in K+ is stored by its K+ index; any other pose by its exact matrix over
Q(sqrt5). The lattice flag of a piece is "pose in K+", which is exactly the contract's lattice
condition for that piece: K+ maps retained chambers onto retained chambers (the signature set
is closed under K+, checked in regions.py), so g_A(A0) is then a retained chamber closure.

Admissibility of a twist (c, g) (contract section 3), with h_A = n_c . x / |n|^2 - alpha:
- lattice piece: classified by its transported signature. Its posed region is the chamber of
  signature g_A(M(A)), which is cut out by the constraint n_c . x >= kappa when c is in that
  signature and by n_c . x <= kappa otherwise, so the defining constraint itself certifies
  inside or outside exactly ("signature" certificate);
- off-lattice piece: pieces with one pose are a pose group. A group is split by anchor caps a
  (a in M(A) for every piece of the subgroup); G(cap region of a) contains every posed piece
  region of the subgroup, so an exact one-sided bound over its 40 exact vertices certifies the
  whole subgroup ("group_superset"). Otherwise every piece is classified over the complete
  exact vertex set of its posed region ("piece_vertices"). A straddling piece yields two exact
  vertices with h < 0 and h > 0, each re-checked against all 1,200 constraints of the posed
  region before the certificate is issued.
- a piece left without certificate (only possible when an exact work budget runs out) makes
  the twist uncertain, which is rejected. Blocked and uncertain twists change nothing.
"""
import hashlib
import json

import numpy as np

from exact import matmul, matvec, transpose

from .kernel import Evaluator, KAPPA_FORM, classify_signs, filtered_signs, pullback_form, rows_form, sign5
from .kplus import matrix_json, q5_json, to_tuple
from .model import CELL_SLOTS, NP, NS
from .twists import Twist, UnrepresentableTwist, primitive


class Pose:
    __slots__ = ('kidx', 'matrix', 'key')

    def __init__(self, kidx, matrix, key):
        self.kidx = kidx
        self.matrix = matrix
        self.key = key


class Classification:
    def __init__(self, grip):
        self.grip = grip
        self.inside = None
        self.straddle = None
        self.uncertain = 0
        self.counts = {'signature': 0, 'group_superset': 0, 'piece_vertices': 0}
        self.exact_evaluations = 0
        self.filtered_evaluations = 0

    @property
    def status(self):
        if self.straddle is not None:
            return 'blocked'
        if self.uncertain:
            return 'uncertain'
        return 'admissible'


class Outcome:
    def __init__(self, status, grip, applied=False, moved=0, cls=None, reason=None, twist=None):
        self.status = status
        self.grip = grip
        self.applied = applied
        self.moved = moved
        self.reason = reason
        self.twist = twist
        self.certificates = dict(cls.counts) if cls is not None else {}
        self.straddle = cls.straddle if cls is not None else None
        self.uncertain = cls.uncertain if cls is not None else 0
        self.exact_evaluations = cls.exact_evaluations if cls is not None else 0

    def as_dict(self):
        d = {'status': self.status, 'grip': self.grip, 'applied': self.applied, 'moved': self.moved,
             'certificates': self.certificates}
        if self.straddle is not None:
            d['straddle'] = self.straddle
        if self.uncertain:
            d['uncertain_pieces'] = self.uncertain
        if self.reason:
            d['reason'] = self.reason
        return d

    def __repr__(self):
        return f'Outcome({self.status}, grip={self.grip}, moved={self.moved})'


class State:
    def __init__(self, ctx=None, filtered=False, menu=None):
        if ctx is None:
            from . import get_context
            ctx = get_context()
        self.ctx = ctx
        self.filtered = bool(filtered)       # amendment A2; OFF by default
        self.menu = menu                     # amendment A4; None = every exact twist
        self.pose_id = np.zeros(NP, np.int32)
        self._poses = [Pose(0, None, ('K', 0))]
        self._lookup = {('K', 0): 0}
        self.journal = []
        self._anchor_cache = {}

    # ---------------------------------------------------------------------------- poses
    def _intern_k(self, k):
        key = ('K', int(k))
        i = self._lookup.get(key)
        if i is None:
            i = len(self._poses)
            self._poses.append(Pose(int(k), None, key))
            self._lookup[key] = i
        return i

    def _intern_matrix(self, m):
        k = self.ctx.kplus.index_of_matrix(m)
        if k >= 0:
            return self._intern_k(k)
        key = ('M', m)
        i = self._lookup.get(key)
        if i is None:
            i = len(self._poses)
            self._poses.append(Pose(-1, m, key))
            self._lookup[key] = i
        return i

    def _pose_matrix(self, pid):
        p = self._poses[pid]
        return self.ctx.kplus.matrix(p.kidx) if p.kidx >= 0 else p.matrix

    def _kpose(self):
        return np.array([p.kidx for p in self._poses], np.int64)

    def pose(self, piece):
        """Exact pose of a piece (tuple of rows of Q5)."""
        return self._pose_matrix(int(self.pose_id[piece]))

    def lattice_flags(self):
        return self._kpose()[self.pose_id] >= 0

    def off_lattice_count(self):
        return int((~self.lattice_flags()).sum())

    def is_lattice(self):
        return bool(self.lattice_flags().all())

    def moved_count(self):
        return int((self.pose_id != 0).sum())

    # --------------------------------------------------------------------- classification
    def _groups(self, off):
        ids = self.pose_id[off]
        order = np.argsort(ids, kind='stable')
        off, ids = off[order], ids[order]
        cuts = np.flatnonzero(np.diff(ids)) + 1
        return [(int(ids[s[0]]), off[s]) for s in np.split(np.arange(len(off)), cuts)]

    def _anchors(self, pid, members):
        key = (pid, len(members), hashlib.sha1(members.tobytes()).digest())
        out = self._anchor_cache.get(key)
        if out is None:
            bits = self.ctx.data.sig_matrix(members)
            left = np.ones(len(members), bool)
            out = []
            while left.any():
                a = int(np.argmax(bits[left].sum(axis=0)))
                sel = left & bits[:, a]
                out.append((a, members[sel]))
                left &= ~sel
            self._anchor_cache[key] = out
        return out

    def _eval(self, ev, vform, frows_fn, cl, budget):
        left = None if budget is None else max(0, budget - cl.exact_evaluations)
        if self.filtered:
            r = filtered_signs(ev, vform, frows_fn(), left)
            if r is None:
                return ('uncertain', 0)
            signs, nf, ne = r
            cl.filtered_evaluations += nf
            cl.exact_evaluations += ne
            return classify_signs(signs)
        r = ev.classify(vform, left)
        cl.exact_evaluations += r[1]
        return r

    def classify(self, e, need_inside=True, stop_at_straddle=True, budget=None):
        """Exact classification of every piece for the cut of pole e, with certificates."""
        ctx = self.ctx
        data, kp, reg = ctx.data, ctx.kplus, ctx.regions
        cl = Classification(int(e))
        kpose = self._kpose()
        K = kpose[self.pose_id]
        lat = K >= 0
        parts = []
        cl.counts['signature'] = int(lat.sum())
        if need_inside:
            lp = np.nonzero(lat)[0]
            ep = kp.perms[kp.inv[K[lp]], int(e)]
            parts.append(lp[data.in_sig(lp, ep)])
        off = np.nonzero(~lat)[0]
        for pid, members in (self._groups(off) if off.size else []):
            g = self._pose_matrix(pid)
            ev = Evaluator(pullback_form(g, data.N[e]))
            for anchor, sub in self._anchors(pid, members):
                if budget is not None and cl.exact_evaluations >= budget:
                    cl.uncertain += len(sub)
                    continue
                r = self._eval(ev, reg.cap_vform(anchor), lambda a=anchor: reg.cap_frows(a), cl, budget)
                if r[0] == 'uncertain':
                    cl.uncertain += len(sub)
                    continue
                if r[0] != 'straddle':
                    cl.counts['group_superset'] += len(sub)
                    if r[0] == 'in' and need_inside:
                        parts.append(sub)
                    continue
                ins = []
                for p in sub.tolist():
                    r = self._eval(ev, reg.vform(p), lambda q=p: reg.frows(q), cl, budget)
                    if r[0] == 'uncertain':
                        cl.uncertain += 1
                        continue
                    if r[0] == 'straddle':
                        cert = self._straddle_certificate(p, pid, int(e), r[2], r[3])
                        if cl.straddle is None:
                            cl.straddle = cert
                        if stop_at_straddle:
                            return cl
                        continue
                    cl.counts['piece_vertices'] += 1
                    if r[0] == 'in':
                        ins.append(p)
                if need_inside and ins:
                    parts.append(np.array(ins, np.int64))
        if need_inside:
            cl.inside = np.sort(np.concatenate(parts)) if parts else np.zeros(0, np.int64)
        return cl

    def _straddle_certificate(self, p, pid, e, jneg, jpos):
        data, reg = self.ctx.data, self.ctx.regions
        g = self._pose_matrix(pid)
        home = reg.vertices(p)
        xb, xa = matvec(g, home[jneg]), matvec(g, home[jpos])
        hb, ha = data.h(e, xb), data.h(e, xa)
        ok_b = posed_point_check(data, data.signature(p), g, xb)
        ok_a = posed_point_check(data, data.signature(p), g, xa)
        if not (hb.sign() < 0 < ha.sign() and ok_b and ok_a):
            raise AssertionError('straddle certificate failed its exact re-check')
        o, k = reg.rep_of(p)
        return {'piece': int(p), 'grip': e, 'orbit': o, 'transport': k,
                'vertex_below': int(jneg), 'vertex_above': int(jpos),
                'point_below': [q5_json(x) for x in xb], 'point_above': [q5_json(x) for x in xa],
                'h_below': q5_json(hb), 'h_above': q5_json(ha),
                'h_below_float': float(hb), 'h_above_float': float(ha),
                'points_checked_against_all_constraints': True}

    # ------------------------------------------------------------------------------ apply
    def apply(self, twist, budget=None, journal=True):
        """Apply a twist if it is certified admissible; otherwise change nothing."""
        if isinstance(twist, UnrepresentableTwist):
            return Outcome('uncertain', twist.grip, reason=twist.reason)
        if not isinstance(twist, Twist):
            raise TypeError('apply needs a Twist')
        if self.menu is not None and not self.menu.contains(twist):
            return Outcome('invalid', twist.grip, reason=f'twist is not in menu {self.menu.name}')
        cl = self.classify(twist.grip, need_inside=True, stop_at_straddle=True, budget=budget)
        if cl.straddle is not None:
            return Outcome('blocked', twist.grip, cls=cl, twist=twist)
        if cl.uncertain:
            return Outcome('uncertain', twist.grip, cls=cl, reason='some pieces have no certificate', twist=twist)
        self._commit(twist, cl.inside, journal)
        return Outcome('admissible', twist.grip, applied=True, moved=int(cl.inside.size), cls=cl, twist=twist)

    def _commit(self, twist, inside, journal):
        """Replace g_A by g o g_A for the certified inside pieces. All-or-nothing: on any
        exception the poses, the pose table and the journal are restored before re-raising."""
        saved = (self.pose_id.copy(), list(self._poses), dict(self._lookup), len(self.journal))
        try:
            if inside.size:
                uniq, inv = np.unique(self.pose_id[inside], return_inverse=True)
                new = np.array([self._compose(twist, int(u)) for u in uniq], np.int32)
                self.pose_id[inside] = new[inv]
            self._compact()
            if journal:
                rec = twist.record()
                rec['moved'] = int(inside.size)
                self.journal.append(rec)
        except BaseException:
            self.pose_id, self._poses, self._lookup = saved[0], saved[1], saved[2]
            del self.journal[saved[3]:]
            self._anchor_cache = {}
            raise

    @staticmethod
    def _sort_key(pose):
        if pose.kidx >= 0:
            return 'K%05d' % pose.kidx
        return 'M' + json.dumps(matrix_json(pose.matrix))

    def _compact(self):
        """Drop unused poses and order the rest canonically (identity first, then by exact key),
        so that equal configurations with equal journals serialise to equal bytes."""
        used = np.unique(self.pose_id)
        keep = [0] + sorted((int(i) for i in used if i != 0), key=lambda i: self._sort_key(self._poses[i]))
        if keep == list(range(len(self._poses))):
            return
        remap = np.zeros(len(self._poses), np.int32)
        remap[keep] = np.arange(len(keep), dtype=np.int32)
        self.pose_id = remap[self.pose_id]
        self._poses = [self._poses[i] for i in keep]
        self._lookup = {p.key: i for i, p in enumerate(self._poses)}
        self._anchor_cache = {}

    def _compose(self, twist, pid):
        p = self._poses[pid]
        if twist.kidx >= 0 and p.kidx >= 0:
            return self._intern_k(self.ctx.kplus.compose(twist.kidx, p.kidx))
        return self._intern_matrix(to_tuple(matmul(twist.matrix, self._pose_matrix(pid))))

    def survey(self, grips=None, budget=None):
        """Status of every grip (default all 600), each with its certificate."""
        out = {}
        for e in (range(600) if grips is None else grips):
            cl = self.classify(int(e), need_inside=False, stop_at_straddle=True, budget=budget)
            row = {'status': cl.status, 'certificates': dict(cl.counts),
                   'exact_evaluations': cl.exact_evaluations}
            if self.filtered:
                row['filtered_evaluations'] = cl.filtered_evaluations
            if cl.straddle is not None:
                row['certificate'] = cl.straddle
            if cl.uncertain:
                row['uncertain_pieces'] = cl.uncertain
            out[int(e)] = row
        return out

    # ---------------------------------------------------------------- journal and replay
    def undo(self):
        """Undo the last journal record by its exact inverse. The inverse is always admissible:
        g fixes n_c, so the inside set of the cut of c is unchanged by (c, g). Undo is a journal
        operation, so a twist menu does not restrict it."""
        if not self.journal:
            raise IndexError('journal is empty')
        rec = self.journal[-1]
        tw = Twist.from_record(self.ctx, rec).inverse()
        cl = self.classify(tw.grip, need_inside=True, stop_at_straddle=True)
        if cl.straddle is not None or cl.uncertain or cl.inside.size != rec['moved']:
            raise AssertionError('exact inverse is not admissible with the recorded inside set; '
                                 'the contract is violated (state unchanged)')
        self._commit(tw, cl.inside, journal=False)
        self.journal.pop()
        return Outcome('admissible', tw.grip, applied=True, moved=int(cl.inside.size), cls=cl, twist=tw)

    def journal_json(self):
        return json.dumps({'format': 'jumbling-journal-1', 'records': self.journal}, sort_keys=True)

    @classmethod
    def replay(cls, records, ctx=None, filtered=False, menu=None):
        """Rebuild a configuration from journal records; every record must apply as recorded."""
        if isinstance(records, str):
            doc = json.loads(records)
            if doc.get('format') != 'jumbling-journal-1':
                raise ValueError('unknown journal format')
            records = doc['records']
        st = cls(ctx, filtered=filtered, menu=menu)
        for i, rec in enumerate(records):
            out = st.apply(Twist.from_record(st.ctx, rec))
            if not out.applied or out.moved != rec.get('moved', out.moved):
                raise ValueError(f'replay: record {i} gave {out.status} with {out.moved} moved pieces')
        return st

    # --------------------------------------------------------------- snapshots and digest
    def snapshot(self):
        """Byte serialisation of the whole state (poses, pose table, journal)."""
        parts = [self.pose_id.tobytes()]
        parts += [repr(p.key).encode() for p in self._poses]
        parts.append(json.dumps(self.journal, sort_keys=True).encode())
        return b'\x00'.join(parts)

    def digest(self):
        """Canonical digest of the configuration: the exact pose of every non-identity piece.
        Two configurations are equal (contract section 2) exactly when their digests agree."""
        h = hashlib.sha256()
        moved = np.nonzero(self.pose_id != 0)[0]
        items = []
        for pid, members in (self._groups(moved) if moved.size else []):
            key = json.dumps(matrix_json(self._pose_matrix(pid)))
            items.append((key, np.sort(members).astype(np.int32).tobytes()))
        items.sort()
        for key, b in items:
            h.update(key.encode())
            h.update(b)
        return h.hexdigest()

    # ------------------------------------------------------------------- checkpoints (5)
    def _journal_witness(self):
        """A retained word derived from the journal by exact identities only: consecutive
        twists of one grip merge into one twist, (c, g1)(c, g2) = (c, g2 g1), because g1 keeps
        every piece on its side of the cut of c; identity twists drop. If what is left consists
        of retained twists, their generator words form a witness."""
        kp = self.ctx.kplus
        stack = []
        for rec in self.journal:
            m = Twist.from_record(self.ctx, rec).matrix
            c = int(rec['grip'])
            if stack and stack[-1][0] == c:
                _, prev = stack.pop()
                m = to_tuple(matmul(m, prev))
            if kp.index_of_matrix(m) != 0:
                stack.append((c, m))
        word = []
        for c, m in stack:
            k = kp.index_of_matrix(m)
            if k < 0:
                return None, 'the journal keeps jumble twists that do not cancel; supply a witness word'
            word += kp.a4_word(c, k)
        return word, 'journal reduced to retained twists'

    def checkpoint(self, witness=None):
        """Contract section 5: a retained-state checkpoint is a lattice configuration certified
        equal to the state reached from solved by a witnessed word in the 1,200 generators.
        `witness` is a word of signed 1-based primitive ids; without one, a witness is derived
        from the journal by exact identities only. The word is always replayed exactly."""
        if not self.is_lattice():
            return {'checkpoint': False, 'lattice': False, 'off_lattice': self.off_lattice_count(),
                    'reason': 'some piece is off the lattice'}
        if witness is None:
            word, why = self._journal_witness()
            source = 'journal'
            if word is None:
                return {'checkpoint': False, 'lattice': True, 'reason': why}
        else:
            word, why, source = [int(x) for x in witness], 'supplied', 'supplied'
        ref = State(self.ctx)
        for mid in word:
            out = ref.apply(primitive(self.ctx, mid), journal=False)
            if not out.applied:
                return {'checkpoint': False, 'lattice': True, 'reason': f'witness move {mid} rejected: {out.status}'}
        equal = ref.digest() == self.digest()
        return {'checkpoint': equal, 'lattice': True, 'witness_word': word, 'source': source,
                'replay_equal': equal, 'reason': why if equal else 'witness replay differs from this configuration'}

    def export_retained(self, witness=None):
        """Handoff to the retained solver: the labelled slot permutation, only at a witnessed
        retained-state checkpoint. Returns (labels, checkpoint record) with labels[slot] = home
        slot of the sticker now in slot (the convention of core.Model.word_net)."""
        cp = self.checkpoint(witness)
        if not cp['checkpoint']:
            raise ValueError('not a retained-state checkpoint: ' + cp['reason'])
        return self.lattice_stickers()['labels'], cp

    # ------------------------------------------------------- labelled stickers and frames
    def lattice_stickers(self):
        """Labelled stickers on a lattice configuration (amendment A3).

        Sticker (A, f) is the home slot s of piece A on facet f. With pose k in K+ it sits on
        facet f' = k(f) of piece B = k(A), in slot slot(B, f'); its frame k F_f equals F_f' L
        with L in A4_0 (L = F_f'^-1 k F_f), and the slot layout must place it at region index
        L(j) of the base cell, where j = s mod 433. Returns the slot of every sticker, labels,
        the orientation index of L in the stabiliser of pole 0 and the frame agreement flag."""
        if not self.is_lattice():
            raise ValueError('labelled slots are defined only on lattice configurations')
        data, kp, reg = self.ctx.data, self.ctx.kplus, self.ctx.regions
        K = self._kpose()[self.pose_id]
        comp = kp.compose_vec(K, reg.transport)
        dest_piece = reg.piece_of[reg.orbit_of, comp]
        s = np.arange(NS, dtype=np.int64)
        a = data.slot_piece
        f = s // CELL_SLOTS
        ka = K[a]
        f2 = kp.perms[ka, f].astype(np.int64)
        dest = data.slot_of(dest_piece[a], f2)
        labels = np.empty(NS, np.int64)
        labels[dest] = s
        if not np.array_equal(np.sort(dest), s):
            raise AssertionError('sticker map is not a bijection')
        # frames: L = F_f'^-1 o k o F_f
        fi = kp.frame_idx
        lidx = kp.compose_vec(kp.inv[fi[f2]], kp.compose_vec(ka, fi[f]))
        orient = self.ctx.stab0_pos[lidx]
        if np.any(orient < 0):
            raise AssertionError('a sticker frame is not related by an element of A4_0')
        consistent = self.ctx.base_region_perm[orient, s % CELL_SLOTS] == dest % CELL_SLOTS
        return {'slot': dest, 'labels': labels, 'orientation': orient, 'frame_agrees': consistent}

    def sticker_frame(self, s):
        """Exact frame of sticker s (home slot) in any configuration: pose of its piece times
        the retained frame of its home cell."""
        data, kp = self.ctx.data, self.ctx.kplus
        a = int(data.slot_piece[s])
        f = int(s) // CELL_SLOTS
        return to_tuple(matmul(self.pose(a), kp.matrix(int(kp.frame_idx[f]))))


def posed_point_check(data, sig, g, x):
    """Exact check that x lies in the closed posed region g(A0) of a piece with signature sig:
    z = g^T x must satisfy all 600 cut constraints (inside for poles in sig, outside otherwise)
    and all 600 facet constraints. Uses integer arithmetic; falls back to Q5 for large values."""
    z = matvec(transpose(g), x)
    e, rows = rows_form([z])
    x1 = np.array(rows[0][:4], dtype=object)
    x2 = np.array(rows[0][4:], dtype=object)
    n1, n2 = data.N1.astype(object), data.N2.astype(object)
    p1 = n1 @ x1 + 5 * (n2 @ x2)
    p2 = n2 @ x1 + n1 @ x2
    for q in range(600):
        if sign5(int(p1[q]) - 24 * e, int(p2[q]) - 8 * e) > 0:
            return False
        s = sign5(KAPPA_FORM[2] * int(p1[q]) - 2 * e * KAPPA_FORM[0], KAPPA_FORM[2] * int(p2[q]) - 2 * e * KAPPA_FORM[1])
        if (q in sig and s < 0) or (q not in sig and s > 0):
            return False
    return True
