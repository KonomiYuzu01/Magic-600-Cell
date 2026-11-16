"""Public API of J1 (JI.52): the one module that touches J1's and the theory code's private members.

The differential harness binds to J1 and to the theory code (`research/jumbling/theory/`) only through
this module and public names. Each function calls J1 as it stands; none changes a result. Python
standard library and NumPy only.
"""
import copy
import importlib
import sys
from collections import namedtuple
from pathlib import Path

import numpy as np

from .model import NP
from .state import State
from .twists import Twist

_JUMBLING = Path(__file__).resolve().parent.parent

# One entry of a state's pose table: the K+ index (-1 off the lattice), the exact matrix (None for a
# K+ pose, whose matrix is the K+ element) and the canonical key.
PoseEntry = namedtuple('PoseEntry', 'kidx matrix key')


def _theory_module(name):
    """A module of the theory code or of the fixtures, with the paths its own scripts use."""
    for sub in (_JUMBLING / 'fixtures', _JUMBLING, _JUMBLING / 'theory'):
        if str(sub) not in sys.path:
            sys.path.insert(0, str(sub))
    return importlib.import_module(name)


# ------------------------------------------------------------------------------ applying a twist
def apply_with_inside(state, twist, budget=None):
    """Apply `twist` as `state.apply` does and return (outcome, inside).

    `inside` is the sorted piece set J1 committed, with no second classification. A refusal returns
    the empty set and changes nothing."""
    captured = []
    own = state.__dict__
    had = '_commit' in own
    previous = own.get('_commit')
    commit = type(state)._commit

    def capture(tw, inside, journal):
        commit(state, tw, inside, journal)
        captured.append(inside.copy())

    state._commit = capture
    try:
        outcome = state.apply(twist, budget=budget)
    finally:
        if had:
            state._commit = previous
        else:
            del state._commit
    if not outcome.applied:
        return outcome, np.zeros(0, np.int64)
    if len(captured) != 1:
        raise AssertionError('J1 admitted a twist without committing its inside set exactly once')
    return outcome, captured[0]


# ------------------------------------------------------------------------------------ pose table
def pose_entries(state):
    """Per pose ID, its PoseEntry (K+ index, exact matrix, key): a read-only copy of the state's table."""
    return tuple(PoseEntry(p.kidx, p.matrix, p.key) for p in state._poses)


def piece_kplus(state):
    """Per piece, its K+ index, or -1 when the piece is off the lattice."""
    return np.array([p.kidx for p in state._poses], np.int64)[state.pose_id]


def pose_matrix(state, pid):
    """The exact matrix of pose ID `pid`."""
    return state._pose_matrix(pid)


def pose_groups(state, pieces):
    """The pose groups of `pieces`: a list of (pose ID, pieces) pairs, as J1 groups them."""
    return state._groups(pieces)


def anchors(state, pid, members):
    """The anchor caps of one pose group: a list of (anchor, subgroup) pairs (cached by the state)."""
    return state._anchors(pid, members)


def evaluate(state, ev, vform, frows_fn, cl, budget, inputs=None):
    """J1's exact evaluation of one vertex form against the evaluator `ev`, charged to classification `cl`."""
    return state._eval(ev, vform, frows_fn, cl, budget, inputs)


def straddle_certificate(state, piece, pid, grip, below, above):
    """The straddle certificate of `piece` in pose `pid` for cap `grip`, after its exact re-check."""
    return state._straddle_certificate(piece, pid, grip, below, above)


def from_pose_table(ctx, entries, pose_ids, menu=None, filtered=False, compact=True):
    """A state from a raw pose table: `entries[i]` is (K+ index, exact matrix or None, ...) and
    `pose_ids` gives every piece's entry.

    An entry with a matrix is interned by its matrix (a K+ matrix becomes its K+ pose), otherwise by its
    K+ index. The table is then compacted canonically (`compact=False` keeps the interned order) and the
    caches are empty (KN2.14). The journal is empty."""
    state = State(ctx, filtered=filtered, menu=menu)
    ids = np.asarray(pose_ids)
    if ids.shape != (NP,):
        raise ValueError('pose_ids needs one entry per piece')
    if ids.size and (int(ids.min()) < 0 or int(ids.max()) >= len(entries)):
        raise ValueError('pose_ids is outside the pose table')
    remap = []
    for entry in entries:
        kidx, matrix = entry[0], entry[1]
        if matrix is not None:
            remap.append(state._intern_matrix(matrix))
        else:
            if not 0 <= int(kidx) < len(ctx.kplus.perms):
                raise ValueError('a K+ pose needs a K+ index')
            remap.append(state._intern_k(int(kidx)))
    state.pose_id = np.asarray(remap, np.int32)[ids]
    if compact:
        state._compact()
    state.journal, state._anchor_cache = [], {}
    return state


# ---------------------------------------------------------------------------------- copy, adopt
def copy_state(state):
    """An independent copy sharing only the context, the menu and the exact pose entries; its journal
    is a deep copy and its caches are empty."""
    trial = copy.copy(state)
    trial.pose_id = state.pose_id.copy()
    trial._poses, trial._lookup = list(state._poses), dict(state._lookup)
    trial.journal = copy.deepcopy(state.journal)
    trial._anchor_cache = {}
    return trial


def adopt(state, trial):
    """Hand a finished copy's whole state over to the original. Call it only after the copy's work
    succeeded, so a refusal leaves the original unchanged; the copy is not used afterwards."""
    state.__dict__.update(trial.__dict__)
    return state


# ---------------------------------------------------------------------------------- theory code
def seam_restorer(state, menu, domains=None, inside_sets=None):
    """A `seams.SeamRestorer` over `state` and `menu`, reading the supplied domains and inside sets.

    `inside_sets` maps a cap to its inside piece array, or to None for a refused cap; the restorer keeps
    this dict as its classification cache, so entries added later are seen."""
    restorer = _theory_module('seams').SeamRestorer(state, menu)
    if domains is not None:
        restorer._dom = domains
    if inside_sets is not None:
        restorer._cls = inside_sets
    return restorer


def fixture_state(ctx, name, t):
    """(state, menu) after the first `t` records of the W-J fixture `name`: an empty journal and the
    fixture's menu. A record that J1 does not admit raises a RuntimeError."""
    fixture, menu = _theory_module('wj').load(ctx, name)
    state = State(ctx, menu=menu)
    for rec in fixture['journal']['records'][:t]:
        if not state.apply(Twist.from_record(ctx, rec)).applied:
            raise RuntimeError('fixture record not admissible')
    state.journal = []
    return state, menu
