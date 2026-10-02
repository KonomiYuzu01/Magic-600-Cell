"""Generate O01..O07 against the read-only 0.4 reference in minutes."""
from __future__ import annotations

import argparse
import gzip
import hashlib
import io
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
from tools.oracle.compare import CASE_FORMAT, EFFECT_FIELDS

SEEDS = {"O01": 600101, "O02": 600102, "O03": 600103, "O04": 600104,
         "O05": 600104, "O06": 600106, "O07": 600107}
CHAIN_MOVES = 500
SCRAMBLE_MOVES = 32
MIXED_RECIPES = 25


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def word(moves):
    return [{"kind": "word", "moves": moves}]


def star(orbit, node=0, sign=1):
    return {"kind": "star", "orbit": orbit, "node": node, "sign": sign}


def random_word(rng, count):
    return [rng.choice((-1, 1)) * rng.randint(1, 1200) for _ in range(count)]


def expansion_digest(model, recipe):
    primitives = np.fromiter(model.expand(recipe), dtype="<i4")
    return sha256(primitives.tobytes()), int(len(primitives))


def progress_digest(state):
    return sha256(canonical(state.progress()).encode("utf-8"))


def reference_step(model, state, recipe, session=None):
    """Validate the complete input before any state or session mutation."""
    try:
        normalized, _ = model.normalize(recipe)
    except ValueError:
        return dict(accepted=False, error="invalid-input", state_hash=state.hash,
                    progress_sha256=progress_digest(state),
                    **{field: None for field in EFFECT_FIELDS})

    expansion, count = expansion_digest(model, normalized)
    if session is None:
        source, destination, _, _ = model.net(normalized)
        effect = dict(net_sha256=sha256(source.astype("<i4").tobytes()
                                       + destination.astype("<i4").tobytes()),
                      net_moved_stickers=int(len(source)),
                      net_moved_pieces=int(len(np.unique(model.sp[source]))),
                      support=model.support(source), conflicts=[])
        state.apply(source, destination)
        accepted, error = True, None
    else:
        preview = session.preview(normalized)
        effect = {field: preview[field] for field in
                  ("net_moved_stickers", "net_moved_pieces", "support", "conflicts")}
        effect["net_sha256"] = preview["source_to_destination_sha256"]
        before = session.st.hash
        before_progress = progress_digest(session.st)
        try:
            session.commit(preview["token"])
            accepted, error = True, None
        except ValueError:
            if not preview["conflicts"]:
                raise
            if session.st.hash != before or progress_digest(session.st) != before_progress:
                raise RuntimeError("Protection refusal changed the reference state")
            accepted, error = False, "protected"
        state = session.st
    return dict(accepted=accepted, error=error, state_hash=state.hash,
                expansion_sha256=expansion, primitive_count=count,
                progress_sha256=progress_digest(state), **effect)


def case_inputs(model, *, small=False):
    """Fixed full suite; small=True selects a seeded test subset, never a CLI mode."""
    rng = random.Random(SEEDS["O01"])
    generators = sorted(rng.sample(range(1, 1201), 4)) if small else range(1, 1201)
    yield ("O01", "Every legal generator and inverse from solved", "independent", [], [],
           [word([sign * move]) for move in generators for sign in (1, -1)])

    rng = random.Random(SEEDS["O02"])
    moves = random_word(rng, CHAIN_MOVES)
    yield ("O02", "Seeded chronological generator chain", "chain", [], [],
           [word([move]) for move in (moves[:8] if small else moves)])

    rng = random.Random(SEEDS["O03"])
    scramble = random_word(rng, SCRAMBLE_MOVES)
    orbits = (0, 23, 33) if small else range(35)
    recipes = []
    for orbit in orbits:
        nodes = len(model.trees[orbit]["positions"])
        recipes.extend([star(orbit, node, sign)]
                       for node in (0, nodes // 2, nodes - 1) for sign in (1, -1))
    yield ("O03", "First, middle and last stars of every orbit after a seeded scramble",
           "chain", scramble, [], recipes)

    rng = random.Random(SEEDS["O04"])
    mixed = []
    for index in range(MIXED_RECIPES):
        recipe = []
        for part in range(2 + index % 5):
            if (index + part) % 2 == 0:
                recipe.append(word(random_word(rng, rng.randint(1, 5)))[0])
            else:
                orbit = rng.randrange(35)
                recipe.append(star(orbit, rng.randrange(len(model.trees[orbit]["positions"])),
                                   rng.choice((-1, 1))))
        mixed.append(recipe)
    if small:
        mixed = mixed[:5]
    yield ("O04", "Mixed word and star recipes of two to six parts", "chain", [], [], mixed)
    yield ("O05", "Each mixed recipe followed by its chronological inverse", "chain", [], [],
           [item for recipe in mixed for item in (recipe, invrecipe(recipe))])

    # Derive witnesses from complete support rather than guessing collateral.
    for orbit in range(35):
        source, _, _, _ = model.net([star(orbit)])
        collateral = [row["orbit"] for row in model.support(source) if row["orbit"] != orbit]
        if collateral:
            protected = collateral[0]
            collateral_star = star(orbit)
            break
    else:
        raise RuntimeError("No star with collateral found")
    for orbit in range(35):
        source, _, _, _ = model.net([star(orbit)])
        if len(source) and not np.any(model.so[source] == protected):
            allowed_star = star(orbit)
            break
    else:
        raise RuntimeError("No star avoids the protected orbit")
    for generator in range(1, 1201):
        source, _ = model.move(generator)
        if np.any(model.so[source] == protected):
            break
    else:
        raise RuntimeError("No generator touches the protected orbit")
    yield ("O06", "Collateral protection, target protection and net-only cancellation",
           "chain", [], [protected],
           [[allowed_star], [collateral_star], word([generator]),
            word([generator, -generator]), [star(protected)], invrecipe([allowed_star])])

    rng = random.Random(SEEDS["O07"])
    invalid_node = len(model.trees[0]["positions"])
    invalid = [word([move]) for move in (0, 1201, -1201, 1.5)]
    invalid.extend([[], [star(35)], [star(0, sign=0)], [star(0, invalid_node)],
                    word([1, 0]), word([1]) + [star(0, invalid_node)]])
    yield ("O07", "Invalid inputs and invalid suffixes preserve a seeded initial state",
           "independent", random_word(rng, SCRAMBLE_MOVES), [], invalid)


def make_case(model, case_id, description, mode, moves, protected, recipes):
    initial = PuzzleState(model)
    if moves:
        source, destination, _, _ = model.net(word(moves))
        initial.apply(source, destination)
    case = dict(
        format=CASE_FORMAT, case_id=case_id, description=description,
        model=dict(model_id=model.model_id,
                   manifest_sha256=sha256((model.root / "manifest.json").read_bytes())),
        generator=dict(script="tools/oracle/make_cases.py",
                       script_sha256=sha256(Path(__file__).read_bytes()),
                       core_sha256=sha256((ROOT / "core.py").read_bytes()),
                       session_sha256=sha256((ROOT / "session.py").read_bytes()),
                       python=platform.python_version(), numpy=np.__version__, seed=SEEDS[case_id],
                       counts=dict(initial_moves=len(moves), steps=len(recipes))),
        mode=mode, initial=dict(kind="word" if moves else "solved", moves=moves,
                               state_hash=initial.hash), protected=protected, steps=[],
    )

    def run(session=None):
        state = initial
        for index, recipe in enumerate(recipes):
            if session is not None:
                state = session.st
            elif mode == "independent":
                state = PuzzleState(model, initial.labels, trusted=True)
            expect = reference_step(model, state, recipe, session)
            case["steps"].append(dict(index=index, recipe=recipe, expect=expect))

    if protected and mode != "chain":
        # Session steps carry state; an independent protected case would need a session per step.
        raise ValueError("Protected cases must use chain mode")
    if protected:
        # The oracle never accepts an existing session or a data-directory argument.
        with tempfile.TemporaryDirectory(prefix="c600-oracle-", dir=ROOT) as directory:
            session = Session(model, Path(directory))
            try:
                if moves:
                    preview = session.preview(word(moves))
                    session.commit(preview["token"])
                session.save_prefs({"protected": protected})
                run(session)
            finally:
                session.close()
    else:
        run()
    return case


def generate_cases(model=None, *, small=False):
    model = model if model is not None else Model()
    for inputs in case_inputs(model, small=small):
        yield make_case(model, *inputs)


def check_output_directory(out):
    """Only a new or empty real directory is accepted, so no existing file or link is ever written."""
    out = Path(out)
    if out.is_symlink():
        raise ValueError("Refusing a linked output directory")
    if out.exists() and not out.is_dir():
        raise ValueError("Output must be a directory")
    if out.exists() and any(out.rglob("session.sqlite3")):
        raise ValueError("Refusing an output directory containing session.sqlite3")
    if out.exists() and any(out.iterdir()):
        raise ValueError("Refusing a non-empty output directory; use a fresh one")
    return out


def write_cases(out, model=None, *, small=False):
    out = check_output_directory(out)
    out.mkdir(parents=True, exist_ok=True)
    paths = []
    for case in generate_cases(model, small=small):
        buffer = io.BytesIO()
        with gzip.GzipFile(fileobj=buffer, mode="wb", filename="", mtime=0) as stream:
            stream.write(canonical(case).encode("utf-8"))
        path = out / (case["case_id"] + ".json.gz")
        with open(path, "xb") as stream:  # exclusive create: never follows or replaces an entry
            stream.write(buffer.getvalue())
        paths.append(path)
    return paths


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", required=True, help="Oracle output directory")
    args = parser.parse_args(argv)
    try:
        paths = write_cases(args.out)
    except ValueError as exc:
        print("Oracle generation failed: " + str(exc), file=sys.stderr)
        return 2
    except OSError:
        print("Oracle generation failed: cannot read reference or write output", file=sys.stderr)
        return 2
    print("Generated " + ", ".join(path.name for path in paths))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
