"""General exact simulator for rigid jumbling of the 600-cell-Full geometry (workstream J1).

Usage, from the repository root:

    import sys; sys.path.insert(0, 'research/jumbling')
    import sim
    st = sim.State()
    st.apply(sim.plane(st.ctx, 0, 13, degrees=10))

See README.md in this directory. Python standard library and NumPy only.
"""
import sys
from pathlib import Path

_JUMBLING = Path(__file__).resolve().parent.parent
if str(_JUMBLING) not in sys.path:
    sys.path.insert(0, str(_JUMBLING))

import numpy as np  # noqa: E402

from .kplus import KPlus  # noqa: E402
from .model import CELL_SLOTS, ModelData  # noqa: E402
from .regions import Regions  # noqa: E402


class Context:
    """Read-only shared data: exact poles, K+, representative regions and lookup tables."""

    def __init__(self):
        self.data = ModelData()
        self.kplus = KPlus(self.data)
        self.regions = Regions(self.data, self.kplus)
        kp, reg, data = self.kplus, self.regions, self.data
        self.stab0_pos = -np.ones(len(kp.perms), np.int64)
        for i, s in enumerate(kp.stab0):
            self.stab0_pos[s] = i
        # base-cell region permutation of each element of the stabiliser of pole 0
        j = np.arange(CELL_SLOTS)
        piece = data.slot_piece[j]
        rows = []
        for s in kp.stab0:
            comp = kp.compose_vec(np.full(CELL_SLOTS, s), reg.transport[piece])
            img = reg.piece_of[reg.orbit_of[piece], comp]
            rows.append(data.slot_of(img, np.zeros(CELL_SLOTS, np.int64)))
        self.base_region_perm = np.array(rows, np.int64)
        assert all(sorted(r.tolist()) == list(range(CELL_SLOTS)) for r in self.base_region_perm)


_CTX = None


def get_context():
    global _CTX
    if _CTX is None:
        _CTX = Context()
    return _CTX


from .state import Outcome, State, posed_point_check  # noqa: E402,F401
from .twists import (Twist, TwistError, TwistMenu, UnrepresentableTwist, a4_element,  # noqa: E402,F401
                     cap_frame, cayley, cayley_axis_angle, generator, half_turn, plane, primitive)
