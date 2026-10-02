"""Write a reference trace to stdout by replaying primitive moves, without net caches."""
from __future__ import annotations

import argparse
import hashlib
import struct
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))

import numpy as np

from core import Model, PuzzleState, canonical
from tools.oracle.compare import EFFECT_FIELDS, TRACE_FORMAT, read_document, validate_case


def _digest(data):
    return hashlib.sha256(data).hexdigest()


def _progress(state):
    return _digest(canonical(state.progress()).encode("utf-8"))


def replay_step(model, state, recipe, protected):
    try:
        normalized, _ = model.normalize(recipe)
    except ValueError:
        return state, dict(accepted=False, error="invalid-input", state_hash=state.hash,
                           progress_sha256=_progress(state),
                           **{field: None for field in EFFECT_FIELDS})

    labels = state.labels.copy()
    permutation = model.ids.copy()
    expansion = hashlib.sha256()
    count = 0
    for primitive in model.expand(normalized):
        source, destination = model.move(primitive)
        labels[destination] = labels[source]
        permutation[destination] = permutation[source]
        expansion.update(struct.pack("<i", primitive))
        count += 1

    destination = np.flatnonzero(permutation != model.ids).astype(np.int32)
    source = permutation[destination]
    order = np.argsort(source)
    source, destination = source[order], destination[order]
    support = model.support(source)
    conflicts = [row for row in support if row["orbit"] in protected]
    accepted = not conflicts
    if accepted:
        state = PuzzleState(model, labels, trusted=True)
    return state, dict(
        accepted=accepted, error=None if accepted else "protected", state_hash=state.hash,
        net_sha256=_digest(source.astype("<i4").tobytes() + destination.astype("<i4").tobytes()),
        net_moved_stickers=int(len(source)), net_moved_pieces=int(len(np.unique(model.sp[source]))),
        expansion_sha256=expansion.hexdigest(), primitive_count=count,
        support=support, conflicts=conflicts, progress_sha256=_progress(state),
    )


def replay_case(case, model=None):
    validate_case(case)
    model = model if model is not None else Model()
    identity = dict(model_id=model.model_id,
                    manifest_sha256=_digest((model.root / "manifest.json").read_bytes()))
    if case["model"] != identity:
        raise ValueError("Case model identity differs from the reference")

    labels = model.ids.copy()
    # Initial scrambles take the same uncached primitive path as operation steps.
    for primitive in case["initial"]["moves"]:
        source, destination = model.move(primitive)
        labels[destination] = labels[source]
    initial = PuzzleState(model, labels, trusted=True)
    if initial.hash != case["initial"]["state_hash"]:
        raise ValueError("Initial state_hash differs from the reference")
    state = initial
    steps = []
    for step in sorted(case["steps"], key=lambda item: item["index"]):
        if case["mode"] == "independent":
            state = initial
        state, result = replay_step(model, state, step["recipe"], set(case["protected"]))
        steps.append(dict(index=step["index"], **result))
    build = _digest(Path(__file__).read_bytes() + (ROOT / "core.py").read_bytes())
    return dict(format=TRACE_FORMAT, case_id=case["case_id"], model=identity,
                candidate=dict(name="reference-primitive-replay", build=build), steps=steps)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("case")
    args = parser.parse_args(argv)
    try:
        trace = replay_case(read_document(args.case))
    except (OSError, ValueError):
        print("Oracle replay failed: malformed case or different reference identity",
              file=sys.stderr)
        return 2
    print(canonical(trace))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
