"""Read-only J1/ν adapter for the optional-session prototype's injected protocol.

The independent retained-label path uses only assets/primitives.npz. It never
infers poses from labels or manufactures a retained witness. Import census
first so any research centroid cache is redirected outside the repository.
"""

import sys

sys.dont_write_bytecode = True

import base64  # noqa: E402
import copy  # noqa: E402
import hashlib  # noqa: E402
import json  # noqa: E402
from pathlib import Path  # noqa: E402

from differential.leads import census  # noqa: E402,F401

import numpy as np  # noqa: E402
import sim  # noqa: E402
from sim.kplus import matrix_from_json  # noqa: E402
from sim.model import NP  # noqa: E402

from differential import j1_state, menu_order  # noqa: E402
from differential.references import nu  # noqa: E402


ROOT = Path(__file__).resolve().parents[4]


def _word(generators):
    result = list(generators)
    if any(type(g) is not int or not 1 <= abs(g) <= 1200 for g in result):
        raise ValueError("retained generators must be signed 1-based ids in ±1..±1200")
    return result


class J1SessionEngine:
    """Exact menu replay and label checkpoints; no search or word derivation."""

    def __init__(self, menu_name="I_a"):
        self.ctx = sim.get_context()
        fixture = json.loads((ROOT / "research/jumbling/fixtures" /
                              f"wj-{menu_name}.json").read_text(encoding="utf-8"))
        self.menu = sim.TwistMenu.from_record(self.ctx, fixture["menu"])
        menu_order.check_menu(fixture["menu"])
        self.identities = {"model": copy.deepcopy(fixture["journal"]["model_identity"]),
                           "menu": menu_order.menu_identity(fixture["menu"]),
                           "menu_order": menu_order.order_digest(fixture["menu"]),
                           "contract_revision": nu.CONTRACT_REVISION}
        if self.identities["model"] != self.ctx.data.identity:
            raise ValueError("fixture model_identity differs from the J1 context")
        with np.load(ROOT / "assets/primitives.npz", allow_pickle=False) as primitives:
            self._src = primitives["src"].astype(np.int64)
            self._dst = primitives["dst"].astype(np.int64)
            self._offsets = primitives["offsets"].astype(np.int64)
        for values in (self._src, self._dst, self._offsets):
            values.flags.writeable = False
        self._caps = {}

    def solved(self):
        return sim.State(self.ctx, menu=self.menu)

    def solved_labels(self):
        return np.arange(self.ctx.data.slot_piece.size, dtype=np.int64)

    def state_record(self, state):
        return j1_state.state_record(state)

    def state_from_record(self, record):
        """Rebuild poses exactly, checking ids, counts and the computed digest."""
        encoded = record["pose_ids"]
        if encoded["encoding"] != "u32le-base64":
            raise ValueError("pose_ids.encoding differs from u32le-base64")
        raw = base64.b64decode(encoded["data"], validate=True)
        if len(raw) != 4 * encoded["count"] or encoded["count"] != NP:
            raise ValueError("pose_ids.count differs from the full piece count")
        if hashlib.sha256(raw).hexdigest() != encoded["sha256"]:
            raise ValueError("pose_ids.sha256 differs from the stored ids")
        ids = np.frombuffer(raw, dtype="<u4")
        table = record["pose_table"]
        if not table or table[0] != {"k": 0} or bool(np.any(ids >= len(table))):
            raise ValueError("pose_table or pose_ids is out of range")
        state = self.solved()
        remap = []
        for pose in table:
            if set(pose) == {"k"}:
                k = pose["k"]
                if type(k) is not int or not 0 <= k < len(self.ctx.kplus.perms):
                    raise ValueError("pose_table K+ index is out of range")
                remap.append(state._intern_k(k))
            elif set(pose) == {"m"}:
                remap.append(state._intern_matrix(matrix_from_json(pose["m"])))
            else:
                raise ValueError("pose_table entry needs one of k or m")
        state.pose_id = np.asarray(remap, dtype=np.int32)[ids]
        actual = state.digest()
        if actual != record["digest"]:
            raise ValueError(f"pose digest: computed {actual}, stored {record['digest']}")
        return state

    def replay_retained(self, generators, *, start=None):
        state = self.solved() if start is None else self.state_from_record(start)
        for index, generator in enumerate(_word(generators)):
            outcome = state.apply(sim.primitive(self.ctx, generator))
            if not outcome.applied:
                raise ValueError(f"retained step {index}: status {outcome.status}, expected admissible")
        return state

    def replay_journal(self, records, *, start=None):
        state = self.solved() if start is None else self.state_from_record(start)
        for index, record in enumerate(records):
            outcome = state.apply(sim.Twist.from_record(self.ctx, record))
            if not outcome.applied or outcome.moved != record["moved"]:
                raise ValueError(f"journal step {index}: status {outcome.status}, moved "
                                 f"{outcome.moved}; expected admissible, moved {record['moved']}")
        return state

    def apply_letter(self, state, letter):
        if not isinstance(letter, (tuple, list)) or len(letter) != 2:
            raise ValueError("a menu letter needs [grip, item]")
        grip, item = letter
        if (type(grip) is not int or type(item) is not int or not 0 <= grip < 600
                or not 0 <= item < len(self.menu.items)):
            raise ValueError("menu-letter grip or item is out of range")
        if grip not in self._caps:
            self._caps[grip] = self.menu.for_cap(grip)
        outcome = state.apply(self._caps[grip][item])
        if not outcome.applied:
            raise ValueError(f"menu letter status {outcome.status}, expected admissible")
        return copy.deepcopy(state.journal[-1])

    @staticmethod
    def digest(state):
        return state.digest()

    @staticmethod
    def is_lattice(state):
        return state.is_lattice()

    @staticmethod
    def centres_home(state):
        return nu.centres_home(state)

    def nu(self, state):
        value = nu.nu(state)
        if self.menu.name == "I_a":
            nu.check_fixed(value)
        return value

    @staticmethod
    def labels(state):
        return nu.labels(state)

    @staticmethod
    def labels_digest(values):
        return nu.labels_digest(values)

    def label_certificate(self, state):
        return nu.label_certificate(state, i_a=self.menu.name == "I_a")

    def apply_generators(self, labels, generators):
        """Apply source-to-destination permutations in chronological order."""
        values = np.array(labels, dtype=np.int64, copy=True)
        expected = self.solved_labels()
        if values.shape != expected.shape or not np.array_equal(np.sort(values), expected):
            raise ValueError("labels are not a permutation of all 259,800 sticker slots")
        for generator in _word(generators):
            index = abs(generator) - 1
            lo, hi = self._offsets[index:index + 2]
            src, dst = self._src[lo:hi], self._dst[lo:hi]
            if generator < 0:
                src, dst = dst, src
            values[dst] = values[src]
        return values
