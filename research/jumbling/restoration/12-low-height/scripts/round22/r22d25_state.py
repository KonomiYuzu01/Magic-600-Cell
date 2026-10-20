"""Exact state and gates of r22d25-cat/1. Research, headless evidence only.

Snapshots own the full simulator state. Pose-table indices are local to that
copy; only stable exact keys may be compared across twists.
"""
import os
import sys
from collections import Counter
from dataclasses import dataclass
from functools import lru_cache
from pathlib import Path

for _name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ.setdefault(_name, '1')
sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[6]
J = ROOT / 'research/jumbling'
sys.path[:0] = [str(J / 'restoration/12-low-height/scripts'), str(J / 'theory'),
                str(J), str(J / 'fixtures')]
import numpy as np
import eb6 as E
import buffer as B
import blocks as BL
import classes as CL
import orient
import rim
import seams
from exact import matmul
from sim.kernel import Evaluator, pullback_form
from sim.kplus import to_tuple


def set_cache(directory):
    directory = Path(directory)
    directory.mkdir(parents=True, exist_ok=True)
    seams.CACHE = directory / 'home_centroids.npy'
    os.environ['JUMBLING_HOME_CACHE'] = str(seams.CACHE.resolve())


def copy_state(st):
    if len(st.pose_id) != len(st.ctx.data.sigbits):
        raise ValueError('full_model')
    out = E.sim.State(st.ctx, filtered=False, menu=E.R.menu('I_a'))
    out.pose_id = st.pose_id.copy()
    out._poses = list(st._poses)
    out._lookup = dict(st._lookup)
    return out


@lru_cache(maxsize=32768)
def pose_info(matrix):
    left, right = E.T.factors(matrix)
    if E.T.dist(right) != 0:
        raise ValueError('I_a requires right height zero')
    return int(E.T.dist(left)), int(E.T.branch(left))


def heights(st):
    h = np.array([0 if p.kidx >= 0 else pose_info(p.matrix)[0] for p in st._poses], np.int32)
    return h[st.pose_id]


def census(st):
    h = heights(st)
    return {str(k): int(v) for k, v in sorted(Counter(h[h > 0].tolist()).items())}


def all_labels(st, pieces=None):
    """0 out, 1 in, 2 straddle; rows are pieces and columns are all 600 caps.

    R22D-82's complete exact classification, including every member of each
    anchor subgroup. A representative never certifies the other members.
    """
    C = st.ctx
    n = len(st.pose_id)
    pick = np.arange(n) if pieces is None else np.asarray(pieces, np.int64)
    out = np.zeros((len(pick), 600), np.uint8)
    pos = np.full(n, -1, np.int32)
    pos[pick] = np.arange(len(pick))
    k = st._kpose()[st.pose_id]
    lp, op = pick[k[pick] >= 0], pick[k[pick] < 0]
    groups = [(pid, st._anchors(pid, mem)) for pid, mem in st._groups(op)] if len(op) else []
    for c in range(600):
        out[pos[lp], c] = C.data.in_sig(lp, C.kplus.perms[C.kplus.inv[k[lp]], c])
        for pid, anchors in groups:
            ev = Evaluator(pullback_form(st._pose_matrix(pid), C.data.N[c]))
            for a, sub in anchors:
                lab = ev.classify(C.regions.cap_vform(a))[0]
                if lab == 'uncertain':
                    raise ValueError('uncertain anchor classification')
                if lab != 'straddle':
                    if lab == 'in':
                        out[pos[sub], c] = 1
                    continue
                for p in sub.tolist():
                    lab = ev.classify(C.regions.vform(p))[0]
                    if lab == 'uncertain':
                        raise ValueError('uncertain piece classification')
                    out[pos[p], c] = {'in': 1, 'out': 0, 'straddle': 2}[lab]
    return out


@lru_cache(maxsize=1)
def model_classes():
    C = E.sim.get_context()
    members, cls = CL.classes()
    branches = tuple(int(E.T.branch(tuple(n))) for n in C.data.N)
    mapping = {}
    M = E.R.menu('I_a')
    jumble = E.R.jumble_items(M)[0]
    for c, b in enumerate(branches):
        if b not in mapping:
            matrix = E.R.element(M, c, jumble)
            hubs = CL.hub(matrix, members, cls)
            if len(hubs) != 1 or pose_info(matrix) != (2, b):
                raise ValueError('branch has no unique height-2 model class')
            mapping[b] = int(hubs[0][1])
        if b in mapping and mapping[b] != int(cls[c]):
            raise ValueError('branch maps to more than one model class')
    if len(mapping) != 5:
        raise ValueError('incomplete branch-to-class map')
    return members, cls, branches, mapping


def high_units(st, labels=None, h=None):
    h = heights(st) if h is None else h
    high = np.flatnonzero(h >= 4)
    lab = all_labels(st, high) if labels is None else labels[high]
    groups = {}
    for row, p in enumerate(high.tolist()):
        # pose.key is exact and independent of pose-table numbering.
        key = (st._poses[int(st.pose_id[p])].key, (lab[row] == 1).tobytes())
        groups.setdefault(key, []).append(p)
    out = []
    for ps in sorted(groups.values(), key=lambda v: v[0]):
        p = ps[0]
        branch = pose_info(st._pose_matrix(int(st.pose_id[p])))[1]
        row = np.searchsorted(high, p)
        out.append({'first': p, 'pieces': len(ps), 'piece_ids': ps, 'height': int(h[p]),
                    'branch': branch, 'caps_whole': np.flatnonzero(lab[row] == 1).tolist()})
    return out


def height2_units(st, labels=None, h=None):
    """mkblocks2's rigid components, split by frame domain, without a free set.

    Equal complete label vectors have identical neighbours in the no-cut-
    separates graph. Quotienting those twins before the repository's component
    routine preserves its components and avoids a quadratic piece-level scan.
    The components include *all* off pieces before splitting and selecting h=2.
    """
    h = heights(st) if h is None else h
    off = np.flatnonzero(h > 0)
    if not len(off):
        return []
    lab = all_labels(st, off) if labels is None else labels[off]
    packed = np.concatenate((np.packbits(lab == 1, axis=1), np.packbits(lab == 0, axis=1)), axis=1)
    _, first, inverse = np.unique(packed, axis=0, return_index=True, return_inverse=True)
    components = BL.rigid_components(np.arange(len(first)), lab[first])
    D = rim.Domains(st)
    result = []
    for component in components:
        ps = off[np.isin(inverse, component)]
        for d in sorted(set(D.dom[ps].tolist())):
            unit = ps[D.dom[ps] == d]
            # Do not silently split a unit further by height.
            if not (h[unit] == 2).all():
                continue
            result.append({'first': int(unit[0]), 'pieces': len(unit), 'piece_ids': unit.tolist(),
                           'height': 2, 'block_first': int(ps[0]), 'domain': int(d)})
    return sorted(result, key=lambda u: u['first'])


@dataclass
class Snapshot:
    state: object
    digest: str
    lattice: object
    off: object
    pose_keys: object
    heights: object
    heights_census: dict
    high_units: list
    height2_units: list
    labels: object
    straddlers: tuple
    clean: tuple
    cap_branches: tuple
    branch_to_class: dict
    free_alphabet: dict
    safe: tuple
    holes: object
    lattice_inside: object

    @property
    def off_count(self):
        return int(self.off.sum())

    @property
    def largest_height(self):
        return int(self.heights.max(initial=0))


def snapshot(state):
    st = copy_state(state)
    digest = st.digest()
    h = heights(st)
    lat = st.lattice_flags()
    labels = all_labels(st)
    straddlers = tuple(np.flatnonzero(labels[:, c] == 2) for c in range(600))
    clean = tuple(c for c, ps in enumerate(straddlers) if not len(ps))
    _, _, cap_branches, mapping = model_classes()
    high = high_units(st, labels, h)
    low = height2_units(st, labels, h)
    # For height-2 poses, hub determines the actual model class, rather than
    # confusing tree-branch numbering with class numbering (R22D-50).
    own = np.full(len(st.pose_id), -1, np.int8)
    members, cls, _, _ = model_classes()
    for pid in np.unique(st.pose_id[h == 2]).tolist():
        hubs = CL.hub(st._pose_matrix(pid), members, cls)
        if len(hubs) != 1:
            raise ValueError('height-2 pose has no unique frame class')
        if mapping[pose_info(st._pose_matrix(pid))[1]] != hubs[0][1]:
            raise ValueError('height-2 hub disagrees with branch-to-class map')
        own[st.pose_id == pid] = hubs[0][1]
    free, safe = {}, []
    for P in range(5):
        grips = [c for c in clean if int(cls[c]) == P
                 and not ((labels[:, c] == 1) & ~lat & (own != P)).any()]
        free[P] = tuple((c, j) for c in grips for j in range(1, 60))
    for c in clean:
        if not ((labels[:, c] == 1) & ~lat).any():
            safe.append(c)
    lattice_inside = (labels[lat] == 1).sum(axis=0)
    result = Snapshot(st, digest, lat, ~lat, E.stable_pose(st), h, census(st), high, low, labels,
                      straddlers, clean, cap_branches, dict(mapping), free, tuple(safe), B.hole_set(st),
                      lattice_inside)
    if st.digest() != digest:
        raise ValueError('snapshot changed state')
    return result


def literal_word(word):
    if isinstance(word, dict):
        word = word['word']
    if not isinstance(word, (list, tuple)) or not word:
        raise ValueError('empty_or_invalid_word')
    out = []
    for letter in word:
        if not isinstance(letter, (list, tuple)) or len(letter) != 2:
            raise ValueError('invalid_letter')
        c, j = letter
        if type(c) is not int or type(j) is not int or not 0 <= c < 600 or not 0 <= j < 60:
            raise ValueError('invalid_letter')
        out.append([c, j])
    return out


def play(state, word):
    """Fresh full replay. No accepted prefix changes the caller's state."""
    st = copy_state(state)
    M = E.R.menu('I_a')
    for index, (c, j) in enumerate(literal_word(word)):
        o = st.apply(E.sim.Twist(st.ctx, c, E.R.element(M, c, j), 'menu', {'menu': M.name}), journal=False)
        if o.uncertain:
            raise ValueError('uncertain_prefix')
        if not o.applied:
            raise ValueError(f'inadmissible_prefix_{index}')
    return st


def _gate(before, U, word, lowering, h, high_exact):
    st = before.state if isinstance(before, Snapshot) else before
    z = before.digest if isinstance(before, Snapshot) else st.digest()
    result = {'ok': False, 'reason': None, 'input_digest': z, 'letters': 0}
    try:
        if st.digest() != z:
            raise ValueError('input_digest')
        if isinstance(word, dict) and word.get('input_digest', z) != z:
            raise ValueError('input_digest')
        w = literal_word(word)
        target = sorted(set(U))
        if not target or any(type(p) is not int or not 0 <= p < len(st.pose_id) for p in target):
            raise ValueError('invalid_target')
        lat0 = st.lattice_flags()
        if lat0[target].any():
            raise ValueError('target_not_off')
        h0 = before.heights if isinstance(before, Snapshot) else heights(st)
        if lowering:
            h = int(h0[target[0]]) if h is None else h
            if h < 4 or not (h0[target] == h).all():
                raise ValueError('target_height')
        keys0 = before.pose_keys if isinstance(before, Snapshot) else E.stable_pose(st)
        end = play(st, w)
        result.update(letters=len(w), output_digest=end.digest())
        if isinstance(word, dict) and word.get('output_digest', result['output_digest']) != result['output_digest']:
            raise ValueError('output_digest')
        lat1, keys1 = end.lattice_flags(), E.stable_pose(end)
        target_mask = np.zeros(len(lat0), bool)
        target_mask[target] = True
        others = ~lat0 & ~target_mask
        collateral = np.flatnonzero(lat0 & ~lat1)
        exact = keys0 == keys1
        changed = np.flatnonzero(others & ~lat1 & ~exact)
        keep = (h0 >= 4) & ~target_mask if high_exact is True else np.zeros(len(lat0), bool)
        if high_exact is not False and high_exact is not True:
            keep[list(high_exact)] = True
            keep &= ~target_mask
        high_bad = np.flatnonzero(keep & ~exact)
        result.update(off_before=int((~lat0).sum()), off_after=int((~lat1).sum()),
                      unit_on_lattice=int(lat1[target].sum()), collateral=len(collateral),
                      lattice_pieces_off=collateral.tolist(), others_exact=int((others & ~lat1 & exact).sum()),
                      others_landed=int((others & lat1).sum()), other_pieces_changed=changed.tolist(),
                      high_pieces_changed=high_bad.tolist(), heights_after=census(end), state=end)
        if len(collateral):
            raise ValueError('lattice_pieces_off')
        if len(changed):
            raise ValueError('other_off_pose')
        if len(high_bad):
            raise ValueError('high_exact')
        if lowering:
            h1 = heights(end)
            if not (lat1[target].all() or (h1[target] == h - 2).all()):
                raise ValueError('target_not_lowered')
            result['kind'] = 'landing' if lat1[target].all() else 'lowering'
        elif not lat1[target].all():
            raise ValueError('target_not_landed')
        else:
            result['kind'] = 'landing'
        if st.digest() != z:
            raise ValueError('input_changed')
        result['ok'] = True
    except (ValueError, KeyError, TypeError, IndexError) as exc:
        result['reason'] = str(exc)
    return result


def strict_step(before, U, word, high_exact=False):
    return _gate(before, U, word, False, None, high_exact)


def protected_lowering(before, U, word, h=None, high_exact=False):
    return _gate(before, U, word, True, h, high_exact)


def refresh_after_accept(state, output_digest=None):
    if output_digest is not None and state.digest() != output_digest:
        raise ValueError('output_digest')
    return snapshot(state)


@lru_cache(maxsize=8192)
def element(c, j):
    return E.R.element(E.R.menu('I_a'), c, j)


def moved_pose(st, pid, c, j):
    return to_tuple(matmul(element(c, j), st._pose_matrix(int(pid))))


@lru_cache(maxsize=1)
def nonretained_items():
    return tuple(E.R.jumble_items(E.R.menu('I_a')))


def single_letter_prune(s, U, c, j, lowering=False, high_exact=False):
    """Necessary exact single-letter tests; every survivor still gets a replay."""
    if c not in s.clean:
        return 'blocked'
    if not (s.labels[U, c] == 1).all():
        return 'target_not_whole'
    if j not in nonretained_items():
        return 'retained'
    if s.lattice_inside[c]:
        return 'lattice_inside'
    target = set(U)
    inside = np.flatnonzero(s.labels[:, c] == 1)
    for pid, ps in s.state._groups(inside):
        q = moved_pose(s.state, pid, c, j)
        landed = s.state.ctx.kplus.index_of_matrix(q) >= 0
        if not landed and not lowering:
            return 'inside_not_landed'
        in_target = [int(p) for p in ps if int(p) in target]
        other = [int(p) for p in ps if int(p) not in target]
        if in_target and lowering and not landed and pose_info(q)[0] != int(s.heights[in_target[0]]) - 2:
            return 'target_height'
        if other and not landed:
            return 'other_off_pose'
        if high_exact is True and other and (s.heights[other] >= 4).any():
            return 'high_exact'
        if high_exact is not True and high_exact is not False and set(other) & set(high_exact):
            return 'high_exact'
    return None


def blocker_order(s):
    """R22D-52 step 5: eligible caps, then units by first cap and least piece."""
    eligible = {}
    for u in s.high_units:
        caps = [c for c in u['caps_whole'] if s.cap_branches[c] == u['branch']
                and len(s.straddlers[c]) and (s.heights[s.straddlers[c]] == 2).all()]
        eligible[u['first']] = caps
    caps = sorted({c for cs in eligible.values() for c in cs})
    rows = []
    for u in s.height2_units:
        hits = [c for c in caps if np.isin(s.straddlers[c], u['piece_ids']).any()]
        rows.append((hits[0] if hits else 600, u['first'], u))
    return eligible, [dict(u, blocker_cap=(None if cap == 600 else cap)) for cap, _, u in sorted(rows)]
