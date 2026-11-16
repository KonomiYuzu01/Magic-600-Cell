"""Generate the language's new oracle cases N01 to N04 against the read-only 0.4 reference (PHS.18; LP2, M4).

`make_cases.py` and the frozen cases O01 to O07 stay unchanged. This script reuses its reference step and writes four
more cases in the same format, from leads L5 to L7 of the M4 differential tests
(work/reviews/leads/2026-11-16-m4-differential-tests.md):

- N01 (L5): star-only recipes in the shapes of 0.4's planner, per orbit after O03's scramble: a position insertion
  [star(a, -1), star(b)], an orientation transfer [star(n), star(n0, -1)] within one position, a three-star
  correction [star(0), star(n), star(0)], the eight-star commutator p + v + inv(p) + inv(v), and a star followed by
  its own inverse.
- N02 (L6): per orbit the deepest frame node and the first node whose path ends with each of up to 20 seeded tree
  letters, both signs; then every orbit's deepest-node star again, which the 96-entry star cache has evicted by then
  for all orbits but the last three.
- N03 (L7): the target and every collateral orbit of a star with two or more collateral orbits are protected; that
  star (refused, every support row in conflict), a star that moves some protected orbits (refused, those rows in
  conflict), a star that avoids them (accepted) and a generator (refused).
- N04 (L7): every orbit is protected: a star and its inverse (an empty net, accepted), a generator (refused) and a
  star conjugated by that generator (refused).

Run it in the pinned engine environment (tools/.venv/engine). It writes N01.json.gz to N04.json.gz and
MANIFEST-oracle-new.json into a new or empty folder; tests/fixtures/oracle-new holds the tracked copy, which
tests/test_kernel_oracle.py replays through the Rust kernel. Two runs write the same bytes.
"""
from __future__ import annotations

import argparse
import gzip
import io
import json
import platform
import random
import sys
import tempfile
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))

import numpy as np

from core import Model, PuzzleState, canonical, invrecipe
from session import Session
from tools.oracle.compare import CASE_FORMAT
from tools.oracle.make_cases import (SCRAMBLE_MOVES, SEEDS, check_output_directory, random_word, reference_step,
                                     sha256, star, word)

SCRIPT = "tools/oracle/make_new_cases.py"
MANIFEST = "MANIFEST-oracle-new.json"
NEW_SEEDS = {"N01": 600201, "N02": 600202, "N03": 600203, "N04": 600204}
LAST_LETTERS = 20


def planner_shapes(model):
    """L5: five star-only recipes per orbit in the shapes `core.py`'s planner returns."""
    rng = random.Random(NEW_SEEDS["N01"])
    recipes = []
    for orbit in range(35):
        tree, by = model.trees[orbit], model.bypos[orbit]
        third = tree["third"]
        x, y = rng.sample(sorted(position for position in by if position != third), 2)
        node = rng.randrange(len(tree["parent"]))
        p = [star(orbit, by[third][-1]), star(orbit, by[third][0], -1)]
        v = [star(orbit, by[y][-1]), star(orbit, by[y][0], -1)]
        recipes += [
            [star(orbit, by[y][0], -1), star(orbit, by[x][0])],
            [star(orbit, by[x][-1]), star(orbit, by[x][0], -1)],
            [star(orbit, 0), star(orbit, by[third][-1]), star(orbit, 0)],
            p + v + invrecipe(p) + invrecipe(v),
            [star(orbit, node), star(orbit, node, -1)],
        ]
    return recipes


def node_tour(model):
    """L6: per orbit the deepest node and the first node for each of up to 20 seeded last path letters, both signs;
    then each orbit's deepest-node star again."""
    rng = random.Random(NEW_SEEDS["N02"])
    recipes, again = [], []
    for orbit in range(35):
        tree = model.trees[orbit]
        nodes = range(1, len(tree["parent"]))
        deepest = max(nodes, key=lambda node: (tree["depth"][node], -node))
        # A node's last path letter is the letter of the edge from its parent.
        first = {}
        for node in nodes:
            first.setdefault(tree["move"][node], node)
        letters = sorted(rng.sample(sorted(first), min(LAST_LETTERS, len(first))))
        chosen = [deepest] + [first[letter] for letter in letters if first[letter] != deepest]
        for node in chosen:
            recipes += [[star(orbit, node)], [star(orbit, node, -1)]]
        again.append([star(orbit, chosen[0])])
    return recipes + again


def moved_orbits(model, recipe):
    source, _, _, _ = model.net(recipe)
    return {row["orbit"] for row in model.support(source)}


def protection_inputs(model):
    """L7: N03 with the support of a star of two or more collateral orbits protected, N04 with every orbit protected.

    The first star of two or more collateral orbits (orbit 0's) moves 33 orbits, so no star would avoid its support;
    N03 takes the first such star that leaves another star allowed. Every generator moves all 35 orbits, so a word of
    one generator is refused whenever an orbit is protected."""
    stars = [moved_orbits(model, [star(orbit)]) for orbit in range(35)]

    def allowed(protected):
        return next((orbit for orbit in range(35) if stars[orbit] and not stars[orbit] & protected), None)

    target = next((orbit for orbit in range(35)
                   if len(stars[orbit] - {orbit}) >= 2 and allowed(stars[orbit]) is not None), None)
    if target is None:
        raise RuntimeError("No star with two collateral orbits leaves another star allowed")
    protected = stars[target]
    partial = next((orbit for orbit in range(35) if stars[orbit] & protected and stars[orbit] - protected), None)
    if partial is None:
        raise RuntimeError("No star moves both protected and unprotected orbits")
    yield ("N03", "Several protected orbits: the support of a star with two or more collateral orbits", "chain", [],
           sorted(protected), [[star(target)], [star(partial)], [star(allowed(protected))], word([1])])
    yield ("N04", "Every orbit protected: a star with its inverse, a generator and a conjugated star", "chain", [],
           list(range(35)), [[star(0), star(0, 0, -1)], word([1]), word([1]) + [star(0)] + word([-1])])


def case_inputs(model):
    scramble = random_word(random.Random(SEEDS["O03"]), SCRAMBLE_MOVES)
    yield ("N01", "Star-only recipes in the planner's shapes, per orbit after O03's scramble", "chain", scramble, [],
           planner_shapes(model))
    yield ("N02", "Deepest and seeded last-letter frame nodes of every orbit, both signs, then evicted stars again",
           "chain", scramble, [], node_tour(model))
    yield from protection_inputs(model)


def make_new_case(model, case_id, description, mode, moves, protected, recipes):
    """`make_cases.make_case` for these cases: the same steps and format, this script's generator block."""
    initial = PuzzleState(model)
    if moves:
        source, destination, _, _ = model.net(word(moves))
        initial.apply(source, destination)
    case = dict(
        format=CASE_FORMAT, case_id=case_id, description=description,
        model=dict(model_id=model.model_id,
                   manifest_sha256=sha256((model.root / "manifest.json").read_bytes())),
        generator=dict(script=SCRIPT, script_sha256=sha256(Path(__file__).read_bytes()),
                       core_sha256=sha256((ROOT / "core.py").read_bytes()),
                       session_sha256=sha256((ROOT / "session.py").read_bytes()),
                       python=platform.python_version(), numpy=np.__version__, seed=NEW_SEEDS[case_id],
                       counts=dict(initial_moves=len(moves), steps=len(recipes))),
        mode=mode, initial=dict(kind="word" if moves else "solved", moves=moves,
                               state_hash=initial.hash), protected=protected, steps=[],
    )
    if mode != "chain":
        raise ValueError("The new cases use chain mode")
    if protected:
        # The oracle never accepts an existing session or a data-directory argument.
        with tempfile.TemporaryDirectory(prefix="c600-oracle-", dir=ROOT) as directory:
            session = Session(model, Path(directory))
            try:
                if moves:
                    session.commit(session.preview(word(moves))["token"])
                session.save_prefs({"protected": protected})
                for index, recipe in enumerate(recipes):
                    case["steps"].append(dict(index=index, recipe=recipe,
                                              expect=reference_step(model, session.st, recipe, session)))
            finally:
                session.close()
    else:
        state = initial
        for index, recipe in enumerate(recipes):
            case["steps"].append(dict(index=index, recipe=recipe, expect=reference_step(model, state, recipe)))
    return case


def write_new_cases(out, model=None):
    out = check_output_directory(out)
    out.mkdir(parents=True, exist_ok=True)
    model = model if model is not None else Model()
    files = {}
    for inputs in case_inputs(model):
        case = make_new_case(model, *inputs)
        buffer = io.BytesIO()
        with gzip.GzipFile(fileobj=buffer, mode="wb", filename="", mtime=0) as stream:
            stream.write(canonical(case).encode("utf-8"))
        data = buffer.getvalue()
        name = case["case_id"] + ".json.gz"
        with open(out / name, "xb") as stream:  # exclusive create: never follows or replaces an entry
            stream.write(data)
        files[name] = dict(bytes=len(data), sha256=sha256(data))
    manifest = dict(
        description="The language's new oracle cases (PHS.18) from leads L5 to L7 of the M4 differential tests, "
                    "generated by tools/oracle/make_new_cases.py in the pinned engine environment.",
        script=SCRIPT, script_sha256=sha256(Path(__file__).read_bytes()),
        core_sha256=sha256((ROOT / "core.py").read_bytes()),
        session_sha256=sha256((ROOT / "session.py").read_bytes()),
        python=f"{platform.python_implementation()} {platform.python_version()}", numpy=np.__version__,
        files=files)
    with open(out / MANIFEST, "x", encoding="utf-8", newline="\n") as stream:
        stream.write(json.dumps(manifest, indent=1, sort_keys=True) + "\n")
    return [out / name for name in files]


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", required=True, help="Output directory, new or empty")
    args = parser.parse_args(argv)
    try:
        paths = write_new_cases(args.out)
    except ValueError as exc:
        print("Oracle generation failed: " + str(exc), file=sys.stderr)
        return 2
    except OSError:
        print("Oracle generation failed: cannot read reference or write output", file=sys.stderr)
        return 2
    print("Generated " + ", ".join(path.name for path in paths) + " and " + MANIFEST)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
