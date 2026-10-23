"""Independent height and kernel contracts for runner-run/3, headless research.

The caller replays each word: prepare copies its start, on_letter observes the
two-letter height guards, and finish checks the end. Only puzzle definitions are
shared with the player. These hooks do not choose rules or verify stuck claims.
check_high_phase checks the cumulative kernel letter bound at the phase boundary.
"""

from dataclasses import dataclass

import numpy as np
from sim.kernel import Evaluator, pullback_form
import trees

UNCHECKED = 'unchecked_contract'
ST_LIMIT = 964
_HEIGHT_MEMO = {}
_FAMILIES = ('F1', 'F2', 'F3', 'd19', 'S', 'ST', 'T1')


def straddlers(st, tau):
    """Sorted off-piece ids straddling tau, using exact anchor and vertex tests."""
    off = np.flatnonzero(st._kpose()[st.pose_id] < 0)
    if not off.size:
        return np.empty(0, np.int64)
    regions, out = st.ctx.regions, []
    for pid, members in st._groups(off):
        ev = Evaluator(pullback_form(st._pose_matrix(pid), st.ctx.data.N[tau]))
        for anchor, sub in st._anchors(pid, members):
            label = ev.classify(regions.cap_vform(anchor))[0]
            if label not in ('in', 'out', 'straddle'):
                raise RuntimeError('non-exact anchor classification')
            if label != 'straddle':
                continue
            for piece in sub.tolist():
                label = ev.classify(regions.vform(piece))[0]
                if label not in ('in', 'out', 'straddle'):
                    raise RuntimeError('non-exact piece classification')
                if label == 'straddle':
                    out.append(piece)
    return np.array(sorted(out), np.int64)


def _integer(value, minimum=0):
    return type(value) is int and value >= minimum


def _add(fails, name):
    if name not in fails:
        fails.append(name)


class _Snapshot:
    def __init__(self, st):
        self.ids = st.pose_id.copy()
        self.off = st._kpose()[self.ids] < 0
        self.count = int(self.off.sum())
        # Include lattice matrices too: an explicit kernel keep-list may name them.
        self.matrices = {int(pid): st._pose_matrix(int(pid)) for pid in np.unique(self.ids)}
        self.heights = np.zeros(len(self.ids), np.int64)
        self.histogram = {}
        pids, counts = np.unique(self.ids[self.off], return_counts=True)
        for pid, count in zip(pids.tolist(), counts.tolist()):
            matrix = self.matrices[pid]
            height = _HEIGHT_MEMO.get(matrix)
            if height is None:
                height = int(trees.dist(trees.factors(matrix)[0]))
                _HEIGHT_MEMO[matrix] = height
            self.heights[self.ids == pid] = height
            key = str(height)
            self.histogram[key] = self.histogram.get(key, 0) + count
        self.maximum = int(self.heights[self.off].max(initial=0))

    def n(self, height):
        return self.histogram.get(str(height), 0)


def _same(before, after, pieces):
    """Compare exact matrices for unique pose-id pairs; ids themselves can change."""
    pairs = np.column_stack((before.ids[pieces], after.ids[pieces]))
    return all(before.matrices[int(a)] == after.matrices[int(b)]
               for a, b in np.unique(pairs, axis=0))


def _kept(before, after, height):
    pieces = before.off & (before.heights == height)
    return bool(after.off[pieces].all()) and _same(before, after, pieces)


@dataclass(frozen=True)
class _HeightRule:
    subphase: str
    counts: tuple = ()                 # (height, 'fall' or 'keep')
    length: int = 1
    maximum: int = 6
    keep: int = 0
    nullable: bool = False
    same_tau: bool = True
    u_progress: str = ''
    tier: object = None


_HEIGHT_RULES = {
    'r22d75/T0': _HeightRule('r22d75', tier=0),
    'r22d75/T1': _HeightRule('r22d75', tier=1),
    'r22d75/T2': _HeightRule('r22d75', tier=2),
    'r22d81/H6': _HeightRule('r22d81', ((6, 'fall'),), nullable=True, same_tau=False),
    'r22d81/U': _HeightRule('r22d81', ((6, 'keep'),), keep=6, u_progress='fall'),
    'r22d81/H4': _HeightRule('r22d81', ((6, 'keep'), (4, 'fall')), keep=6,
                           nullable=True, u_progress='keep'),
    'r22d81/H2': _HeightRule('r22d81', ((6, 'keep'), (4, 'keep'), (2, 'fall')),
                           keep=6, nullable=True, u_progress='keep'),
    'r22d88/U': _HeightRule('r22d81', ((6, 'keep'),), keep=6, u_progress='fall'),
    'r22d88/U2': _HeightRule('r22d81', ((6, 'keep'),), length=2, keep=6, u_progress='fall'),
    'h4/L4': _HeightRule('h4', ((4, 'fall'),), maximum=4, nullable=True, same_tau=False),
    'h4/U': _HeightRule('h4', ((4, 'keep'),), maximum=4, keep=4, u_progress='fall'),
    'h4/L2': _HeightRule('h4', ((4, 'keep'), (2, 'fall')), maximum=4, keep=4, u_progress='keep'),
    'h4/pair': _HeightRule('h4', ((4, 'keep'),), length=2, maximum=4, keep=4, u_progress='fall'),
}


def _tau(value, nullable):
    return (value is None and nullable) or (type(value) is int and 0 <= value < 600)


def _u(st, tau):
    return 0 if tau is None else len(straddlers(st, tau))


def _prepare_height(ctx, row, st):
    fails = ctx['fails']
    name = row.get('rule')
    spec = _HEIGHT_RULES.get(name) if isinstance(name, str) else None
    ctx['spec'] = spec
    if spec is None:
        _add(fails, 'height_rule')
        return
    if row.get('contract') != name or row.get('subphase') != spec.subphase:
        _add(fails, 'height_rule')
    if type(row.get('letters')) is not int or row['letters'] != spec.length:
        _add(fails, 'height_length')
    if spec.tier is not None:
        if (any(key not in row or row[key] is not None
                for key in ('tau_before', 'tau_after', 'u_before', 'u_after'))
                or row.get('u_mid') is not None):
            _add(fails, 'height_fields')
        if any(key not in row or row[key] is not None for key in ('tau_before', 'tau_after')):
            _add(fails, 'height_tau')
        return
    if (not _integer(row.get('u_before')) or not _integer(row.get('u_after'))
            or (not _integer(row.get('u_mid')) if spec.length == 2 else row.get('u_mid') is not None)):
        _add(fails, 'height_fields')
    valid_before = 'tau_before' in row and _tau(row['tau_before'], spec.nullable)
    valid_after = 'tau_after' in row and _tau(row['tau_after'], spec.nullable)
    if (not valid_before or not valid_after
            or (spec.same_tau and row.get('tau_before') != row.get('tau_after'))):
        _add(fails, 'height_tau')
    ctx['tau'] = row.get('tau_before')
    ctx['valid_before'] = valid_before
    # Equality rows always count at tau_before, even when tau_after is forged.
    ctx['end_tau'] = ctx['tau'] if spec.same_tau else row.get('tau_after')
    ctx['valid_after'] = valid_before if spec.same_tau else valid_after
    ctx['u_before'] = _u(st, ctx['tau']) if valid_before else None
    if valid_before and _integer(row.get('u_before')) and row['u_before'] != ctx['u_before']:
        _add(fails, 'height_u')


def _prepare_kernel(ctx, row, st):
    fails, size = ctx['fails'], len(st.pose_id)
    family, kind, params = row.get('family'), row.get('kind'), row.get('params')
    fields_ok = (isinstance(family, str) and family in _FAMILIES
                 and row.get('rule') == 'r22d25/' + family
                 and isinstance(kind, str) and kind in ('landing', 'lowering')
                 and row.get('contract') == 'kernel_' + kind
                 and type(row.get('high_exact')) is bool and isinstance(params, dict)
                 and 'subphase' in row and row['subphase'] is None
                 and type(row.get('off_before')) is int and type(row.get('off_after')) is int
                 and isinstance(row.get('heights_after'), dict)
                 and all(isinstance(k, str) and type(v) is int for k, v in row['heights_after'].items()))
    keep = params.get('high_exact_pieces', []) if isinstance(params, dict) else []
    keep_ok = isinstance(keep, list) and all(type(p) is int and 0 <= p < size for p in keep)
    if not fields_ok or not keep_ok:
        _add(fails, 'kernel_fields')
    ctx['keep'] = keep if keep_ok else []
    target = row.get('target')
    valid = (isinstance(target, list) and bool(target)
             and all(type(p) is int and 0 <= p < size for p in target))
    ctx['target'] = sorted(set(target)) if valid else None
    if not valid or not ctx['before'].off[ctx['target']].all():
        _add(fails, 'kernel_target')
        ctx['target'] = None


def prepare(row, st):
    """Copy the start state and validate row fields, without changing mechanics."""
    row = row if isinstance(row, dict) else {}
    ctx = {'phase': row.get('phase'), 'before': _Snapshot(st), 'fails': [], 'mid_seen': False}
    if ctx['phase'] == 'height':
        _prepare_height(ctx, row, st)
    elif ctx['phase'] == 'high':
        _prepare_kernel(ctx, row, st)
    else:
        _add(ctx['fails'], 'contract_phase')
    return ctx


def on_letter(ctx, k, st):
    """Only the first prefix of a two-letter height step needs observation."""
    if ctx['phase'] != 'height':
        return
    spec = ctx.get('spec')
    if spec is None or spec.length != 2 or k != 0:
        return
    before, middle = ctx['before'], _Snapshot(st)
    ctx['mid_seen'] = True
    if (middle.maximum > spec.maximum or middle.n(spec.keep) != before.n(spec.keep)
            or not _kept(before, middle, spec.keep)):
        _add(ctx['fails'], 'height_guard')
    if ctx['valid_before']:
        ctx['u_mid'] = _u(st, ctx['tau'])


def _finish_height(ctx, row, st, after):
    fails, before, spec = ctx['fails'], ctx['before'], ctx.get('spec')
    if spec is None:
        return
    if before.maximum > spec.maximum or after.maximum > spec.maximum:
        _add(fails, 'height_max')
    if spec.tier is not None:
        a, b = tuple(before.n(h) for h in (6, 4, 2)), tuple(after.n(h) for h in (6, 4, 2))
        if b >= a:
            _add(fails, 'height_counts')
        first = next((i for i in range(3) if a[i] != b[i]), None)
        if first != spec.tier:
            _add(fails, 'height_tier')
        return
    for height, change in spec.counts:
        a, b = before.n(height), after.n(height)
        if (change == 'fall' and b >= a) or (change == 'keep' and b != a):
            _add(fails, 'height_counts')
    if spec.keep and not _kept(before, after, spec.keep):
        _add(fails, 'height_poses')
    u_after = _u(st, ctx['end_tau']) if ctx['valid_after'] else None
    if u_after is not None and _integer(row.get('u_after')) and u_after != row['u_after']:
        _add(fails, 'height_u')
    if spec.length == 2:
        if not ctx['mid_seen']:
            _add(fails, 'height_guard')
        elif 'u_mid' in ctx and _integer(row.get('u_mid')) and ctx['u_mid'] != row['u_mid']:
            _add(fails, 'height_u')
    u_before = ctx['u_before']
    if u_before is not None and u_after is not None:
        if ((spec.u_progress == 'fall' and u_after >= u_before)
                or (spec.u_progress == 'keep' and u_after > u_before)):
            _add(fails, 'height_u_progress')


def _finish_kernel(ctx, row, after):
    fails, before, target = ctx['fails'], ctx['before'], ctx['target']
    if (after.off & ~before.off).any():
        _add(fails, 'kernel_collateral')
    if target is not None:
        target_mask = np.zeros(len(before.ids), bool)
        target_mask[target] = True
        others_off = before.off & ~target_mask & after.off
        if not _same(before, after, others_off):
            _add(fails, 'kernel_others')
        if row.get('high_exact') is True:
            keep = before.off & (before.heights >= 4) & ~target_mask
            if not _same(before, after, keep):
                _add(fails, 'kernel_high_exact')
        keep = [p for p in ctx['keep'] if not target_mask[p]]
        if not _same(before, after, keep):
            _add(fails, 'kernel_high_exact_pieces')
        if row.get('kind') == 'lowering':
            if (not (before.heights[target] == 6).all() or not after.off[target].all()
                    or not (after.heights[target] == 4).all()):
                _add(fails, 'kernel_target_height')
        elif row.get('kind') == 'landing' and after.off[target].any():
            _add(fails, 'kernel_target_landed')
    if ((row.get('kind') == 'landing' and after.count >= before.count)
            or (row.get('kind') == 'lowering' and after.n(6) >= before.n(6))):
        _add(fails, 'kernel_progress')
    if (row.get('off_before') != before.count or row.get('off_after') != after.count
            or row.get('heights_after') != after.histogram):
        _add(fails, 'kernel_counts')


def finish(ctx, row, st):
    """Return unique failure names in discovery order; no unchecked contracts."""
    row = row if isinstance(row, dict) else {}
    if ctx['phase'] == 'height':
        _finish_height(ctx, row, st, _Snapshot(st))
    elif ctx['phase'] == 'high':
        _finish_kernel(ctx, row, _Snapshot(st))
    return list(ctx['fails'])


def check_high_phase(start, rows):
    """The run-wide bound applies even when the phase has no completed result."""
    bad = ['kernel_letter_bound']
    if not isinstance(start, dict) or not _integer(start.get('off')):
        return bad
    hist = start.get('heights')
    if (not isinstance(hist, dict)
            or any(not isinstance(k, str) or not _integer(v) for k, v in hist.items())
            or not isinstance(rows, (list, tuple))):
        return bad
    played = 0
    for row in rows:
        if not isinstance(row, dict) or not _integer(row.get('letters')):
            return bad
        played += row['letters']
    return bad if played > 6 * hist.get('6', 0) + ST_LIMIT * start['off'] else []
