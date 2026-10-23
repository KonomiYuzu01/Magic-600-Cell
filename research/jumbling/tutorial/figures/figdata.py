"""Recompute the tutorial's missing observations from committed research inputs.

No runner, private snapshots, acceptance seeds or persistent simulation caches.
The CLI writes figdata.json only after every selected computation succeeds.
"""

import argparse
import array
import ast
from collections import Counter, defaultdict
from contextlib import contextmanager
from functools import cached_property
import gzip
import hashlib
import importlib.util
import json
import os
from pathlib import Path, PurePosixPath
import re
import sys
import types


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
J = "research/jumbling"
T = J + "/tutorial"
F = T + "/figures"
R = J + "/restoration"
ROUND = R + "/12-low-height/runs/round22"
S12 = R + "/12-low-height/scripts/round22"
S13 = R + "/13-tutorial-runner/scripts"
CLOUD = ROUND + "/cloud/height4-descent-bbef1c8e"
CS = S12 + "/cloud/height4-descent-bbef1c8e"
CENSUS = R + "/11-scramble-census/runs/"
FORMAT = "magic600-tutorial-figdata/1"
NAMES = ("bulk-conj-7101-s30", "03-counts-7302", "03-r22d75-prediction-cap64",
         "ch04-six-stops-caps", "ch06-twisted-cycle", "appa-inventory", "appa-u2-tree",
         "appa-h4-pair-tree", "appa-pure-tree", "appa-correction-tree", "appa-late-cycle-tree")
CONCEPTS = ("ch04-point-model", "05-class-cells-71")
NAMES += CONCEPTS
G9 = R + "/09-general-method"
LIBRARY = G9 + "/data/library-merged.json.gz"
LATE = R + "/13-tutorial-runner/runs/depth20-seed7106/lattice/solutions.json.gz"
D0 = R + "/13-tutorial-runner/runs/dev/d0-s7106/run"
PURE = ROUND + "/line-a/replay"
U2 = ROUND + "/cloud/unlock-cap311-cap290/drive311_after75.json"
POINT_RECORD = CENSUS + "round2/ia-k100-s7102.json"
POINT_STEP = ROUND + "/r22d16_step.txt"
CLASS_TABLE = ROUND + "/line-a/literal/run2b/z-04-b15.json.gz"
CELL_MODEL = R + "/12-low-height/scripts/cellpure5.py"
CELL_CENSUS = S12 + "/transport_census.py"


def inventory_paths():
    """Discover public development evidence without opening reserved-seed files."""
    paths = []
    for directory, pattern in ((R + "/13-tutorial-runner/runs", "run.json"),
                               (CENSUS.rstrip("/"), "*.json")):
        for path in sorted(input_path(directory).rglob(pattern)):
            relative = path.relative_to(ROOT).as_posix()
            if any(9011 <= int(s) <= 9073 for s in re.findall(r"(?:s|seed-?)(\d+)", relative)):
                continue
            paths.append(relative)
    return paths


def evidence_paths(name):
    """A fixed inventory makes --only independent of earlier computations."""
    common = ["assets/manifest.json", "assets/model.npz", "assets/primitives.npz",
              J + "/theory/restore.py", J + "/theory/seams.py", J + "/theory/trees.py",
              J + "/theory/orient.py", R + "/12-low-height/scripts/eb6.py",
              S12 + "/r22d25_state.py", J + "/sim/state.py", J + "/sim/kernel.py"]
    height = [CENSUS + "heights/ia-k3000-s7302.json", ROUND + "/r22d75_height_first.txt"]
    run = R + "/13-tutorial-runner/runs/dev/d0-s7106/run"
    by_name = {
        NAMES[0]: [CENSUS + "round2/ia-k40-s7101.json", T + "/records/ch01-examples.json",
                   J + "/manual/scripts/examples.py", J + "/manual/scripts/examples.out"],
        NAMES[1]: height + [F + "/diagrams.py", ROUND + "/r22d81_rule.txt", CLOUD + "/RESULT.md",
                           CLOUD + "/end_summary.txt", S12 + "/r22d81_rule.py", CS + "/h4_unlock.py"]
                  + [CLOUD + f"/h4_unlock_run{i}.txt" for i in range(1, 5)]
                  + [CLOUD + f"/two_letter_u_round{i}_shard0.txt" for i in (2, 3, 4)],
        NAMES[2]: height + [CS + "/r22d75_from_snapshot.py", R + "/12-low-height/PROOFS-B.md"],
        NAMES[3]: height + [ROUND + "/r22d81_rule.txt", ROUND + "/kernel-d19/bbef1c8e/kernel/run.json",
                           S12 + "/r22d25_search.py"],
        NAMES[4]: [run + "/run.json", run + "/tables/bulk-end.json", run + "/words/00019.ia16.gz",
                   run + "/step-trees/00019.json.gz", T + "/records/ch06-examples.json", S13 + "/runner_api.py"],
        "appa-inventory": inventory_paths(),
        "appa-u2-tree": [ROUND + "/r22d81_pairs_cap311_tail.txt", U2, S13 + "/rules_height.py"],
        "appa-h4-pair-tree": [CLOUD + f"/two_letter_u_round2_shard{i}.txt" for i in range(3)]
                             + [CLOUD + "/end_summary.txt", S13 + "/rules_height.py"],
        "appa-pure-tree": [PURE + "/steps/b17-pure.json", PURE + "/words/b17.json",
                           PURE + "/tables/823.json.gz", PURE + "/tables/737.json.gz", S13 + "/runner.py"],
        "appa-correction-tree": [T + "/records/ch06-examples.json", LIBRARY,
                                 ROUND + "/line-a/literal/lattice/restore-summary.json",
                                 S13 + "/lattice_phase.py", J + "/viewer/export_tutorial.py"],
        "appa-late-cycle-tree": [T + "/records/ch06-examples.json", LATE, "assets/execution_trees.json.gz",
                                 S13 + "/literal_lattice.py", S13 + "/lattice_phase.py",
                                 D0 + "/tables/bulk-end.json", "assets/seed_atlas.json"],
        "ch04-point-model": [POINT_RECORD, POINT_STEP, S12 + "/r22d16_step.py", S12 + "/r22d19_rule.py",
                              F + "/diagrams.py", T + "/04-high.md", T + "/A-macros.md"],
        "05-class-cells-71": [CLASS_TABLE, CELL_MODEL, CELL_CENSUS, T + "/05-tail.md",
                              T + "/records/ch05-examples.json", T + "/scripts/ch05_examples.py"]
                             + [ROUND + f"/line-a/units/eb5-k40-k00-{n}.json" for n in ("b05", "b06")],
    }
    extra = [F + "/figdata.py", S13 + "/runner_api.py"] if name.startswith("appa-") else []
    if name in CONCEPTS:
        extra += [F + "/figdata.py", J + "/theory/e3_class.py", J + "/theory/buffer.py",
                  J + "/theory/classes.py", J + "/exact.py", J + "/sim/kplus.py"]
    return sorted(set(common + by_name[name] + extra))


def sha256(raw):
    return hashlib.sha256(raw).hexdigest()


def input_path(relative):
    """All record names are fixed here; reject reserved seeds before any open."""
    if not isinstance(relative, str) or not relative or "\\" in relative or ":" in relative:
        raise ValueError("expected a repository-relative input")
    p = PurePosixPath(relative)
    if p.is_absolute() or ".." in p.parts or str(p) != relative:
        raise ValueError("expected a normalized repository-relative input")
    for seed in re.findall(r"(?:s|seed-?)(\d+)", relative):
        if 9011 <= int(seed) <= 9073:
            raise ValueError("reserved seed input refused")
    path = ROOT.joinpath(*p.parts)
    if not path.resolve().is_relative_to(ROOT):
        raise ValueError("input leaves the repository")
    return path


class Inputs:
    def __init__(self):
        self.digests = {}

    def raw(self, relative):
        raw = input_path(relative).read_bytes()
        self.digests[relative] = sha256(raw)
        return raw

    def text(self, relative):
        return self.raw(relative).decode("utf-8")

    def json(self, relative):
        raw = self.raw(relative)
        return json.loads(gzip.decompress(raw) if relative.endswith(".gz") else raw)

    def lines(self, relative):
        rows = []
        for line in self.text(relative).splitlines():
            match = re.fullmatch(r"([A-Z][A-Z0-9_]*) (\{.*\})", line)
            if match:
                rows.append((match[1], json.loads(match[2])))
        return rows


def require_equal(actual, expected, label):
    if actual != expected:
        raise ValueError(f"{label} mismatch: {actual!r} != {expected!r}")


def check_digest(state, expected):
    """A recorded prefix may abbreviate a digest, but never replaces it."""
    actual = state.digest()
    if not isinstance(expected, str) or not re.fullmatch(r"[0-9a-f]{8,64}", expected):
        raise ValueError("invalid recorded state digest")
    if not actual.startswith(expected):
        raise ValueError(f"state digest mismatch: {actual} != {expected}")
    return actual


def functions(inputs, path, names, namespace, observer=None):
    """Load named functions/classes, never a research script's CLI or run loop.

    An optional observer adds a read-only callback to the count prediction.
    Arithmetic, ordering and classification stay those of the original function.
    """
    tree = ast.parse(inputs.text(path), filename=path)
    definitions = [node for node in tree.body if isinstance(node, (ast.FunctionDef, ast.ClassDef)) and node.name in names]
    require_equal({node.name for node in definitions}, set(names), "source functions")
    module = ast.Module(body=definitions, type_ignores=[])
    if observer:
        module = observer.visit(module)
    exec(compile(ast.fix_missing_locations(module), path, "exec"), namespace)
    return namespace


def load_diagrams():
    spec = importlib.util.spec_from_file_location("figdata_diagrams", HERE / "diagrams.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class Computation:
    def __init__(self):
        self.inputs = Inputs()

    @cached_property
    def mechanics(self):
        # All numerical caches remain in RAM; no .npy or bytecode is written.
        sys.dont_write_bytecode = True
        for name in ("OPENBLAS_NUM_THREADS", "OMP_NUM_THREADS", "MKL_NUM_THREADS"):
            os.environ[name] = "1"
        sys.path[:0] = [str(input_path(p)) for p in
                       (S12, S13, R + "/12-low-height/scripts", J + "/theory", J, J + "/fixtures")]
        import numpy as np
        import eb6 as E
        import orient
        import seams
        import r22d25_state as K
        from sim.state import Classification
        from sim.kernel import Evaluator, pullback_form
        for path in ("assets/manifest.json", "assets/model.npz", "assets/primitives.npz",
                     J + "/theory/restore.py", J + "/theory/seams.py", J + "/theory/trees.py",
                     J + "/theory/orient.py", R + "/12-low-height/scripts/eb6.py",
                     S12 + "/r22d25_state.py", J + "/sim/state.py", J + "/sim/kernel.py"):
            self.inputs.raw(path)
        return types.SimpleNamespace(np=np, E=E, C=E.sim.get_context(), M=E.R.menu("I_a"),
                                     orient=orient, seams=seams, K=K, Classification=Classification,
                                     Evaluator=Evaluator, pullback_form=pullback_form)

    def state(self):
        m = self.mechanics
        return m.E.sim.State(m.C, filtered=False, menu=m.M)

    def play(self, state, word):
        m = self.mechanics
        for c, j in word:
            outcome = state.apply(m.E.sim.Twist(m.C, c, m.E.R.element(m.M, c, j),
                                              "menu", {"menu": m.M.name}), journal=False)
            if not outcome.applied or outcome.uncertain:
                raise ValueError(f"inadmissible or uncertain letter {(c, j)}")

    def restorer(self, state):
        m = self.mechanics
        if m.seams._HOME is None:
            # Same centroids as seams.home_centroids(), without its disk cache.
            m.seams._HOME = m.np.array([[m.seams.wj.to_float(v) for v in m.seams.wj.centroid(m.C, p)]
                                       for p in range(m.seams.rim.NP)])
        return m.seams.SeamRestorer(state, m.M)

    def target_functions(self, height):
        m = self.mechanics
        path = S12 + "/r22d81_rule.py" if height == 6 else CS + "/h4_unlock.py"
        namespace = dict(np=m.np, E=m.E, C=m.C, M=m.M, REG=m.C.regions,
                         Classification=m.Classification, Evaluator=m.Evaluator,
                         pullback_form=m.pullback_form, cache={}, gcache={}, checktime=lambda: None,
                         br=[m.E.T.branch(tuple(n)) for n in m.C.data.N])
        return functions(self.inputs, path, ("info", "heights", "item", "labels", "target"), namespace), path

    @cached_property
    def height_start(self):
        record = self.inputs.json(CENSUS + "heights/ia-k3000-s7302.json")
        require_equal(record["seed"], 7302, "development seed")
        state = self.state()
        letters = record["history"] + record["moves"]
        for i, letter in enumerate(letters, 1):
            self.play(state, [letter])
            if i % 500 == 0:
                print(f"seed 7302 replay: {i}/{len(letters)} letters", flush=True)
        start = next(row for tag, row in self.inputs.lines(ROUND + "/r22d75_height_first.txt") if tag == "START")
        check_digest(state, start["digest"])
        require_equal(int(state.off_lattice_count()), record["off_now"], "census stop off count")
        return state

    def bulk(self):
        path = CENSUS + "round2/ia-k40-s7101.json"
        record = self.inputs.json(path)
        anchors = self.inputs.json(T + "/records/ch01-examples.json")["F_seed7101"]
        manual_path = J + "/manual/scripts/examples.out"
        self.inputs.raw(J + "/manual/scripts/examples.py")
        manual = next(row for row in map(json.loads, self.inputs.text(manual_path).splitlines())
                      if row.get("record") == "ia-k40-s7101.json" and row.get("step") == 30)
        state = self.state()
        self.play(state, record["history"])
        check_digest(state, anchors["start"]["digest"])
        pos = 0
        for index, step in enumerate(record["steps"][:29]):
            # ch01's field is named before_step_16, but its script explicitly
            # replays moves[:16]: it is before zero-based move/step 16.
            if pos == 16:
                check_digest(state, anchors["before_step_16"]["digest"])
            word = record["moves"][pos:pos + step["twists"]]
            self.play(state, word)
            pos += step["twists"]
            require_equal(int(state.off_lattice_count()), step["off_after"], "census off count")
            require_equal(int(self.restorer(state).misalignment()), step["N_after"], "census N")
        word = record["moves"][pos:pos + record["steps"][29]["twists"]]
        require_equal(word, manual["word"], "repair word")
        caps = sorted({c for c, _ in word})
        states = []
        for i in range(4):
            restorer = self.restorer(state)
            values = {str(c): int(restorer.scores(c)[0]) for c in caps}
            states.append({"state": ("X", "X.x", "X.x.y", "X.x.y.z")[i],
                           "state_digest": state.digest(), "N": int(restorer.misalignment()), "N_c": values,
                           "source": J + "/theory/restore.py:Restorer.scores",
                           "N_source": J + "/theory/restore.py:Restorer.misalignment"})
            if i == 0:
                require_equal(values, {str(c): manual["counts_before"][str(c)]["N_c"] for c in caps}, "manual before counts")
                require_equal(states[-1]["N"], manual["N_before"], "manual N before")
            if i < 3:
                c, j = word[i]
                require_equal(values[str(c)], manual["trace"][i]["N_c"], "manual entry count")
                self.play(state, [[c, j]])
                require_equal(int(self.restorer(state).scores(c)[0]), manual["trace"][i]["N_c_after"], "manual exit count")
        require_equal(states[-1]["N"], manual["N_after"], "manual N after")
        require_equal(int(state.off_lattice_count()), record["steps"][29]["off_after"], "repair off count")
        return {"word": word, "states": states, "digest_anchors": {
            "scramble": anchors["start"]["digest"], "before_step_16": anchors["before_step_16"]["digest"]},
            "digest_note": "The census and manual record no step-30 digests; these four digests fingerprint the replay checked at the committed anchors and against every recorded N/off count."}

    @cached_property
    def point_model_start(self):
        record = self.inputs.json(POINT_RECORD)
        require_equal(record["seed"], 7102, "point-model development seed")
        state = self.state()
        self.play(state, record["history"] + record["moves"])
        # The text contains the same JSON result twice, once pretty-printed.
        step = json.JSONDecoder().raw_decode(self.inputs.text(POINT_STEP).lstrip())[0]
        check_digest(state, step["start_digest"])
        require_equal(int(state.off_lattice_count()), step["off_before"], "R22D-16 off count")
        return state

    def point_model(self):
        """Observe the literal R22D-16 word; never discover or search a case."""
        state = self.point_model_start
        m = self.mechanics
        from exact import matmul, matvec, transpose
        from sim.kplus import to_tuple
        namespace = dict(np=m.np, E=m.E, E3=m.orient.E3, B=m.K.B, R=m.E.R, T=m.E.T,
                         sim=m.E.sim, Counter=Counter, defaultdict=defaultdict, contextmanager=contextmanager,
                         matmul=matmul, matvec=matvec, transpose=transpose, to_tuple=to_tuple,
                         SimpleNamespace=types.SimpleNamespace)
        path = S12 + "/r22d19_rule.py"
        functions(self.inputs, path, ("copy_state", "pose_info", "projective", "ClassGeometry", "Lowered",
                                      "_Rows", "PointModel", "Rule"), namespace)
        source = self.inputs.text(S12 + "/r22d16_step.py")
        literal = load_diagrams().literal_assignment
        word = literal(source, "word")
        # docs refers to the `word` variable; read just its literal unit and
        # digest claims instead of evaluating that assignment or its script.
        docs = next(n.value for n in ast.parse(source).body if isinstance(n, ast.Assign)
                    and any(isinstance(t, ast.Name) and t.id == "docs" for t in n.targets))
        fields = dict(zip(map(ast.literal_eval, docs.keys), docs.values))
        pieces = ast.literal_eval(fields["unit"])["pieces"]
        claims = {ast.literal_eval(k): ast.literal_eval(v) for k, v in zip(fields["word"].keys, fields["word"].values)
                  if ast.literal_eval(k) != "word"}
        check_digest(state, claims["input_digest"])
        rule = namespace["Rule"](state, m.M)
        a, v, commutator = word[0], word[1:2], word[2:6]
        exponent, remainder = divmod(len(word) - 4, len(commutator))
        require_equal(remainder, 0, "commutator power length")
        inverse_v = [[c, rule.inverse_items[i]] for c, i in reversed(v)]
        require_equal(word, [a] + v + commutator * exponent + inverse_v
                      + [[a[0], rule.inverse_items[a[1]]]], "literal step shape")
        lowered_state = m.K.copy_state(state)
        self.play(lowered_state, [a])
        lowered = namespace["Lowered"](rule, lowered_state)
        height, right, branch = lowered.info[int(lowered_state.pose_id[pieces[0]])][:3]
        require_equal((height, right), (2, 0), "lowered unit heights")
        clock = types.SimpleNamespace(tick=lambda **position: None)
        contents, point_of = lowered.contents(branch, clock)
        targets = {point_of[p] for p in pieces}
        require_equal(len(targets), 1, "unit content count")
        target = targets.pop()
        model = lowered.model(branch, clock)
        numbers = [model.index[tuple(letter)] for letter in commutator]
        require_equal(numbers, model.comm(numbers[0], numbers[1]), "literal commutator inverses")

        def point(p):
            return {"cell": model.caps(model.cells[int(p) // 5]), "label": int(p) % 5}

        start = model.cid[target[0]] * 5 + target[1]
        cursor, trace = start, []
        for letter, number in zip(commutator, numbers):
            cursor = int(model.G[number][cursor])
            trace.append(dict(point(cursor), letter=letter))
        power = model.array(numbers * exponent)
        # The chapter/research rule defines W = C^e. V is SAFE-ALL but
        # belongs to another branch, so it has no action on Q's point model.
        # Keep the full middle word and its vacancy preimage distinct.
        core_bad, core_bad_cells = model.bad(power)
        alphabet = lowered.safe_alphabet(clock)
        v_numbers = alphabet.numbers(v)
        preimage = alphabet.images([int(alphabet.inv[n]) for n in reversed(v_numbers)], core_bad)
        # V is outside Q, so measure the full middle word on home chambers
        # with the exact simulator. This checks its admissibility as well as
        # the SAFE preimage identity, without extending Q's point action.
        home_state = self.state()
        home_before = home_state.digest()
        require_equal(int(home_state.off_lattice_count()), 0, "home-action initial off count")
        middle_word = word[1:-1]
        self.play(home_state, middle_word)
        moved_chambers = m.np.flatnonzero(~home_state.lattice_flags())
        require_equal(moved_chambers.tolist(), sorted(preimage.tolist()), "full-middle Bad chamber preimage")

        def cell_rows(chambers):
            packed = m.np.packbits(rule.home[m.np.ix_(chambers, model.poles)], axis=1)
            counts = Counter(int.from_bytes(row.tobytes(), 'big') for row in packed)
            return sorted(({"cell": model.caps(s), "chambers": n} for s, n in counts.items()),
                          key=lambda row: row["cell"])

        bad_rows, preimage_rows = cell_rows(core_bad), cell_rows(moved_chambers)
        require_equal([row["cell"] for row in bad_rows], sorted(core_bad_cells), "Bad(W) home cells")
        # Every edge of the complex is a two-cap cell, so no new adjacency
        # construction or floating-point geometry is needed for this patch.
        edges = sorted(model.caps(s) for s in model.cells if s.bit_count() == 2)
        marked_caps = set(model.caps(target[0])) | {c for row in bad_rows + preimage_rows for c in row["cell"]}
        patch_caps = marked_caps | {c for edge in edges if marked_caps.intersection(edge) for c in edge}
        patch_edges = [edge for edge in edges if set(edge) <= patch_caps]
        after = m.K.copy_state(state)
        self.play(after, word)
        check_digest(after, claims["output_digest"])
        return dict(state_digest=state.digest(), lowered_digest=lowered_state.digest(), after_digest=after.digest(),
                    source=path + ":PointModel", branch=int(branch), branch_caps=model.poles,
                    cells=model.NC, labels=list(range(len(model.geom.reps))), points=len(model.ar),
                    unit=pieces, content=dict(point(start), pieces=len(contents[target])),
                    a=a, V=v, C=commutator, exponent=exponent, middle=middle_word,
                    setup_branch=int(rule.branches[v[0][0]]),
                    trace=trace, after_power=point(power[start]),
                    power_is_identity=bool(m.np.array_equal(power, model.ar)),
                    power_moved_points=int((power != model.ar).sum()),
                    bad_count=len(core_bad), bad_cell_counts=bad_rows, bad_source=path + ":PointModel.bad",
                    preimage_count=len(moved_chambers), preimage_cell_counts=preimage_rows,
                    preimage_source=J + "/sim/state.py:State.apply",
                    home_action_before_digest=home_before, home_action_after_digest=home_state.digest(),
                    preimage_is_holes=bool(alphabet.holes.size and m.np.isin(preimage, alphabet.holes).all()),
                    notation_note="W = C^e, as in section 4.10. V^-1(Bad(W)) is measured by exact replay of the "
                                  "middle word V W V^-1 from all home chambers; V belongs to another branch and "
                                  "has no action on Q's point model.",
                    patch_caps=sorted(patch_caps), patch_edges=patch_edges)

    @cached_property
    def class_cells_start(self):
        record = self.inputs.json(T + "/records/ch05-examples.json")["input"]
        require_equal(record["table"], CLASS_TABLE, "class-cell input table")
        table = self.inputs.json(CLASS_TABLE)
        require_equal(self.inputs.digests[CLASS_TABLE], record["sha256"], "class-cell table digest")
        # Check the two committed state claims before importing the pose table.
        require_equal(table["digest"], record["digest"], "class-cell recorded state digest")
        state = self.state()
        self.mechanics.orient.set_pose_table(state, table)
        check_digest(state, record["digest"])
        require_equal(int(state.off_lattice_count()), record["off"], "class-cell off count")
        return state

    def class_cells(self):
        """Model2 and the census's original ownership block, stopped before search."""
        state = self.class_cells_start
        m = self.mechanics
        record = self.inputs.json(T + "/records/ch05-examples.json")
        E3 = m.orient.E3
        members, classes = E3.CL.classes()
        restorer = self.restorer(state)
        domains = restorer.domains()
        selected_class = next(iter(record["pure"].values()))["class"]
        namespace = dict(np=m.np, E3=E3, defaultdict=defaultdict, Counter=Counter,
                         R=restorer, M=m.M, members=members, cls=classes, P=selected_class,
                         data=m.C.data, NPC=len(state.pose_id), dom_s=m.np.asarray(domains.dom))
        functions(self.inputs, CELL_MODEL, ("Model2",), namespace)
        # Owners live in the class loop, rather than a named function. Execute
        # its original setup through `occ`, excluding save/log and every search.
        tree = ast.parse(self.inputs.text(CELL_CENSUS), filename=CELL_CENSUS)
        loop = next(n for n in tree.body if isinstance(n, ast.For) and isinstance(n.target, ast.Tuple)
                    and [getattr(x, "id", None) for x in n.target.elts] == ["P", "names"])
        stop = next(i for i, n in enumerate(loop.body) if isinstance(n, ast.Assign)
                    and any(isinstance(t, ast.Name) and t.id == "occ" for t in n.targets))
        previous_class = E3.P
        try:
            exec(compile(ast.Module(body=loop.body[:stop + 1], type_ignores=[]), CELL_CENSUS, "exec"), namespace)
        finally:
            E3.P = previous_class
        model, owners, special = namespace["m"], namespace["own"], namespace["SPECIAL"]
        names = {}
        for unit in record["units"]["order"]:
            name = unit["e2_unit"]
            pieces = set(self.inputs.json(ROUND + f"/line-a/units/eb5-k40-k00-{name}.json")["pieces"])
            hits = [j for j, ps in enumerate(model.cpieces) if pieces <= set(ps)]
            require_equal(len(hits), 1, "class-cell unit content")
            names[hits[0]] = name

        def content(j):
            if j < 0:
                return None
            if j == special:
                return {"name": "SPECIAL", "id": int(j)}
            return {"id": int(j), "name": names.get(j, "lattice"), "domain": int(model.cdom[j]),
                    "pieces": len(model.cpieces[j]), "key": min(model.cpieces[j])}

        cells = []
        for j, name in names.items():
            cell = int(model.state[j]) // 5
            points = []
            for label in range(5):
                p = cell * 5 + label
                occupants = [content(k) for k, q in enumerate(model.state) if q == p]
                points.append({"label": label, "point": p, "content": occupants,
                               "owner": content(int(owners[p]))})
            cells.append({"unit": name, "caps": model.caps(model.cells[cell]), "cell": cell,
                          "own_pieces": model.cell_size[model.cells[cell]], "points": points})
        home = namespace["home"]
        census = dict(P=int(selected_class), cells=model.NC, free_grips=len(model.A), home_cells=len(home),
                      occupied_points=int((owners >= 0).sum()), foreign_cells=len(namespace["fcells"]),
                      foreign_untracked=namespace["untracked"], off_contents=sum(d != 0 for d in model.cdom),
                      vacant_home_points_by_kind=dict(sorted(Counter(str(int(model.ncaps[c])) for c in home
                                                                    if owners[c * 5] == -1).items())))
        differences = []
        for key, expected in record["census"]["class_line"].items():
            if key in census and census[key] != expected:
                differences.append({"field": key, "computed": census[key], "chapter": expected})
        for cell, unit in zip(cells, record["units"]["order"]):
            expected = record["pure"][str(unit["key"])]["carrier_cell"]
            for key, actual in (("caps", cell["caps"]), ("pieces_in_cell", cell["own_pieces"]),
                                ("label", next(p["label"] for p in cell["points"] if any(
                                    c["name"] == cell["unit"] for c in p["content"])))):
                if actual != expected[key]:
                    differences.append({"field": cell["unit"] + "." + key, "computed": actual, "chapter": expected[key]})
        paragraph = self.inputs.text(T + "/05-tail.md").split(
            "**Example** (**computed**, record sections `pure` and `census`;", 1)[1].split("![Figure 5.3:", 1)[0]
        patterns = {
            "b05": r"b05's content sits at label (\d+) of the edge cell whose caps are (\d+) and (\d+); "
                   r"that cell holds (\d+) own\s+pieces, the\s+(\d+) of b05 and (\d+) lattice pieces",
            "b06": r"b06's content sits at label (\d+) of the vertex cell of cap (\d+); "
                   r"that cell holds (\d+) own\s+pieces, the\s+(\d+) of b06 and (\d+) lattice pieces",
        }
        for cell in cells:
            values = list(map(int, re.search(patterns[cell["unit"]], paragraph).groups()))
            label, total, off_size, lattice_size = values[0], *values[-3:]
            actual_off = next((p, c) for p in cell["points"] for c in p["content"] if c["name"] == cell["unit"])
            actual_lattice = sum(c["pieces"] for p in cell["points"] for c in p["content"] if c["name"] == "lattice")
            for field, actual, expected in (("caps", cell["caps"], values[1:-3]),
                                            ("label", actual_off[0]["label"], label),
                                            ("own_pieces", cell["own_pieces"], total),
                                            ("off_pieces", actual_off[1]["pieces"], off_size),
                                            ("lattice_pieces", actual_lattice, lattice_size)):
                if actual != expected:
                    differences.append({"field": cell["unit"] + "." + field, "computed": actual, "chapter": expected})
        return dict(state_digest=state.digest(), source=CELL_MODEL + ":Model2", owner_source=CELL_CENSUS,
                    off=int(state.off_lattice_count()), class_number=int(selected_class), cells=cells,
                    shared_caps=sorted(set.intersection(*(set(c["caps"]) for c in cells))),
                    own_contents=len(model.state), lattice_contents=sum(d == 0 for d in model.cdom),
                    generators=len(model.gens), blocked=model.blocked, census=census, chapter_differences=differences)

    def prediction(self):
        state = self.mechanics.K.copy_state(self.height_start)
        m = self.mechanics
        observed = {}

        class ObserveDelta(ast.NodeTransformer):
            def visit_Assign(self, node):
                if any(isinstance(t, ast.Name) and t.id == "delta" for t in node.targets):
                    call = ast.parse("capture(c, j, own, foreign, r, lat, delta)").body[0]
                    return [node, ast.copy_location(call, node)]
                return node

        def capture(c, j, own, foreign, lowered, lattice, delta):
            if (c, j) == (64, 15):
                observed.update(F_h=dict(foreign), O_h=dict(own), r_h=dict(lowered), L0=lattice, delta=list(delta))

        namespace = dict(np=m.np, E=m.E, C=m.C, M=m.M, s=state, cache={}, gcache={}, Counter=Counter,
                         br=[m.E.T.branch(tuple(n)) for n in m.C.data.N], checktime=lambda: None, capture=capture)
        path = CS + "/r22d75_from_snapshot.py"
        functions(self.inputs, path, ("info", "heights", "stats", "nextletter"), namespace, ObserveDelta())
        letter, predicted, _ = namespace["nextletter"]()
        require_equal(letter, (64, 15), "first predicted letter")
        require_equal(observed["delta"], [-13, 13, 621], "recorded delta")
        self.play(state, [letter])
        after = next(row for tag, row in self.inputs.lines(ROUND + "/r22d75_height_first.txt") if tag == "STEP")
        check_digest(state, after["digest"])
        require_equal(namespace["stats"](), predicted, "simulated height prediction")
        require_equal(predicted, after["n6_n4_n2"], "recorded height prediction")
        proof = self.inputs.text(R + "/12-low-height/PROOFS-B.md").split("### R22D-74.", 1)[1].split("### R22D-75.", 1)[0]
        proof_count = int(re.search(r"takes ([\d,]+) lattice pieces off", proof)[1].replace(",", ""))
        return dict(observed, state_digest=self.height_start.digest(), after_digest=state.digest(),
                    letter=list(letter), source=path + ":nextletter.rows", proof_lattice_off=proof_count)

    def height_counts(self):
        D = load_diagrams()
        evidence = D.Evidence("03-counts-7302")
        points, boundary = D.height_examples(evidence)
        for path in evidence.inputs:
            self.inputs.raw(path)
        state = self.mechanics.K.copy_state(self.height_start)
        steps = []
        for path in (ROUND + "/r22d75_height_first.txt", ROUND + "/r22d81_rule.txt"):
            for tag, row in self.inputs.lines(path):
                if tag == "STEP":
                    steps.append((row.get("word", [row.get("letter")]), row["digest"]))
        # Includes both letters of each pair; every intermediate full digest is checked.
        cloud = [(row["letter"], row["digest"]) for tag, row in self.inputs.lines(CLOUD + "/end_summary.txt")
                 if tag == "LETTER"][len(steps):]
        targets = {h: self.target_functions(h) for h in (6, 4)}
        cloud_index = 0
        result = []
        for i, point in enumerate(points):
            if 0 < i <= boundary:
                word, expected = steps[i - 1]
                self.play(state, word)
                check_digest(state, expected)
            elif i > boundary:
                while not state.digest().startswith(point["digest"]):
                    if cloud_index >= len(cloud):
                        raise ValueError("chart point missing from committed cloud line")
                    letter, expected = cloud[cloud_index]
                    cloud_index += 1
                    self.play(state, [letter])
                    check_digest(state, expected)
            digest = check_digest(state, point["digest"])
            h = self.mechanics.K.heights(state)
            counts = [int((h == k).sum()) for k in (6, 4, 2)]
            require_equal(counts, point["counts"], "chart heights")
            height = 6 if counts[0] else 4 if counts[1] else None
            if height is None:
                tau, u, path = None, None, S12 + "/r22d81_rule.py"
            else:
                namespace, path = targets[height]
                tau, u, _ = namespace["target"](state)
            row = dict(index=i, state_digest=digest, H=height, tau=tau, u=u if tau is not None else None,
                       source=path + ":target")
            if tau is None:
                row["reason"] = "no target"
            # The H6 HANDOFF line has u=0 for an absent H6 target. The chart
            # switches to H4 at this state; its new target is a different value.
            if point["u"] is not None:
                row["recorded_u"] = point["u"]
                if i == boundary:
                    previous_tau, previous_u, _ = targets[6][0]["target"](state)
                    require_equal(previous_tau, None, "H6 handoff target")
                    require_equal(previous_u, point["u"], "recorded H6 handoff u")
                    row["recorded_u_scope"] = "H6 handoff (no H6 target); recomputed here for H4"
                else:
                    require_equal(u, point["u"], "recorded target u")
            result.append(row)
            if i == boundary:
                self._bbef_state = self.mechanics.K.copy_state(state)
            print(f"height point {i}: H={height}, target={tau}, u={row['u']}", flush=True)
        require_equal(cloud_index, len(cloud), "cloud line consumed")
        return {"points": result, "boundary": boundary}

    def cap_prunes(self):
        m = self.mechanics
        doc = self.inputs.json(ROUND + "/kernel-d19/bbef1c8e/kernel/run.json")
        # Use exactly the kernel revision named by its ledger.
        for path, digest in doc["scripts"].items():
            require_equal(sha256(self.inputs.raw(path)), digest, "kernel source digest")
        if hasattr(self, "_bbef_state"):
            state = m.K.copy_state(self._bbef_state)
        else:
            state = m.K.copy_state(self.height_start)
            for path in (ROUND + "/r22d75_height_first.txt", ROUND + "/r22d81_rule.txt"):
                for tag, row in self.inputs.lines(path):
                    if tag == "STEP":
                        self.play(state, row.get("word", [row.get("letter")]))
                        check_digest(state, row["digest"])
        digest = check_digest(state, doc["start"]["digest"])
        labels, heights = m.K.all_labels(state), m.K.heights(state)
        units = [u for u in m.K.high_units(state, labels, heights) if u["height"] == 4]
        # Only these fields of snapshot() are read by the cap-level S prune;
        # constructing height-2 units and route libraries would do unused work.
        snapshot = types.SimpleNamespace(labels=labels,
            clean=tuple(c for c in range(600) if not (labels[:, c] == 2).any()),
            lattice_inside=(labels[state.lattice_flags()] == 1).sum(axis=0))
        path = S12 + "/r22d25_search.py"
        tree = ast.parse(self.inputs.text(path), filename=path)
        search = next(n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == "search_step")
        expression = next(n.value for n in ast.walk(search) if isinstance(n, ast.Assign)
                          and isinstance(n.value, ast.IfExp) and any(isinstance(t, ast.Name) and t.id == "reason" for t in n.targets))
        code = compile(ast.Expression(expression), path, "eval")
        rows, totals = [], Counter()
        for unit in units:
            reasons = [eval(code, {"s": snapshot, "u": unit, "c": c}) for c in range(600)]
            totals.update(reasons)
            rows.append({"first": unit["first"], "piece_ids": unit["piece_ids"], "reasons": reasons})
        items = len(m.M.items) - 1
        expected = {reason: n // items for reason, n in doc["ledger"]["S"]["pruned"].items()}
        if any(n % items for n in doc["ledger"]["S"]["pruned"].values()):
            raise ValueError("ledger has fractional cap pairs")
        require_equal(dict(totals), expected, "cap prune ledger totals")
        return {"state_digest": digest, "source": path + ":search_step", "caps": list(range(600)),
                "units": rows, "totals": dict(totals), "items_per_cap": items}

    def twisted_cycle(self):
        m = self.mechanics
        run = R + "/13-tutorial-runner/runs/dev/d0-s7106/run"
        document = self.inputs.json(run + "/run.json")
        row = next(s for s in document["steps"] if s["index"] == 19)
        table = self.inputs.json(run + "/tables/bulk-end.json")
        state = self.state()
        m.orient.set_pose_table(state, table)
        digest = check_digest(state, row["before"]["digest"])
        data = self.inputs.json(T + "/records/ch06-examples.json")
        factor = data["D_factors"]["log"]["6"][0]
        tree_raw = gzip.decompress(self.inputs.raw(run + "/" + row["tree"]["file"]))
        require_equal(sha256(tree_raw), row["tree"]["sha256"], "step tree digest")
        tree = json.loads(tree_raw)
        word_raw = gzip.decompress(self.inputs.raw(run + "/" + row["word"]))
        require_equal(sha256(word_raw), row["word_sha256"], "ia16 word digest")
        codes = array.array("H")
        codes.frombytes(word_raw)
        if sys.byteorder != "little":
            codes.byteswap()
        import runner_api as api
        self.inputs.raw(S13 + "/runner_api.py")
        word = [list(divmod(code, 60)) for code in codes]
        for code, (c, j) in zip(codes, word):
            require_equal(api.ia16_code(c, j), code, "ia16 letter")
        require_equal(len(word), row["letters"], "row word length")
        first = tree["parts"][0]
        require_equal(first["letters"], factor["length"], "factor length")
        setup, seed, undo = first["parts"][0]["parts"]
        lengths = [factor["g"], factor["r"], factor["seed"], factor["r"], factor["g"]]
        require_equal([setup["letters"], seed["letters"], undo["letters"]],
                      [sum(lengths[:2]), lengths[2], sum(lengths[3:])], "factor tree lengths")
        lookup = {m.E.R.to_tuple(matrix): i for i, (_, matrix) in enumerate(m.M.items)}

        def inverse(letters):
            return [[c, lookup[m.E.R.to_tuple(m.E.R.transpose(m.M.items[j][1]))]]
                    for c, j in reversed(letters)]

        g, ri, _, r, gi = [word[a:b] for a, b in ((0, 7), (7, 10), (10, 38), (38, 41), (41, 48))]
        require_equal(inverse(g), gi, "g inverse letters")
        require_equal(inverse(ri), r, "r inverse letters")

        def positions():
            stickers = state.lattice_stickers()
            if not m.np.all(stickers["frame_agrees"]):
                raise ValueError("factor sticker frames disagree")
            home, first_slot = m.np.unique(m.C.data.slot_piece, return_index=True)
            if not m.np.array_equal(home, m.np.arange(len(state.pose_id))):
                raise ValueError("position map omits a piece")
            return m.C.data.slot_piece[stickers["slot"][first_slot]]

        original_pose = m.E.stable_pose(state).copy()
        original_positions = positions().copy()
        tracked = [int(m.np.flatnonzero(original_positions == p)[0]) for p in factor["positions"]]
        tracks = [original_positions[tracked].tolist()]
        parts, offset = [], 0
        for name, length in zip(("g", "r^-1", "c", "r", "g^-1"), lengths):
            before_digest = state.digest()
            before_pose, before_positions = m.E.stable_pose(state).copy(), positions().copy()
            letters = word[offset:offset + length]
            self.play(state, letters)
            after_positions = positions()
            changed = m.np.flatnonzero(before_pose != m.E.stable_pose(state)).tolist()
            parts.append({"name": name, "state_digest": before_digest, "after_digest": state.digest(),
                          "source": R + "/12-low-height/scripts/eb6.py:stable_pose",
                          "word": letters, "span": [offset, offset + length], "count": len(changed),
                          "pieces": [[p, int(before_positions[p]), int(after_positions[p])] for p in changed]})
            tracks.append(after_positions[tracked].tolist())
            offset += length
        after_positions = positions()
        whole = m.np.flatnonzero(original_pose != m.E.stable_pose(state)).tolist()
        collateral = [p for p in whole if p not in tracked]
        result = {"state_digest": digest, "after_digest": state.digest(), "orbit": 6, "factor": 0,
                  "source": R + "/12-low-height/scripts/eb6.py:stable_pose", "parts": parts,
                  "piece_row_format": ["piece_id", "position_before", "position_after"],
                  "position_source": J + "/sim/state.py:State.lattice_stickers",
                  "tracked": [{"piece_id": p, "positions": [v[i] for v in tracks]} for i, p in enumerate(tracked)],
                  "whole_count": len(whole), "collateral_count": len(collateral),
                  "whole": [[p, int(original_positions[p]), int(after_positions[p])] for p in whole],
                  "collateral": [[p, int(original_positions[p]), int(after_positions[p])] for p in collateral]}
        # The remaining factors provide a committed digest check beyond the
        # observed factor, for which the record has no separate end digest.
        self.play(state, word[offset:])
        check_digest(state, row["after"]["digest"])
        result["checked_row_after_digest"] = row["after"]["digest"]
        return result

    @cached_property
    def appendix_api(self):
        self.inputs.raw(S13 + "/runner_api.py")
        return load_diagrams().tree_api()

    def appendix_inventory(self):
        rows = []
        for path in inventory_paths():
            doc = self.inputs.json(path)
            if not isinstance(doc, dict) or not isinstance(doc.get("steps"), list):
                continue
            # File names are screened before opening; reject an unexpected seed
            # field too, before interpreting any moves or step observations.
            seed = doc.get("seed", doc.get("start", {}).get("seed"))
            if type(seed) is int and 9011 <= seed <= 9073:
                raise ValueError("reserved seed record refused")
            counts = Counter(s.get("rule", s.get("family")) for s in doc["steps"])
            rows.append({"record": path, "steps": len(doc["steps"]), "rules": dict(counts)})
        absent = ("lock", "shift", "r22d25/F2", "r22d25/F3", "r22d25/ST")
        for row in rows:
            if set(absent).intersection(row["rules"]):
                raise ValueError("appendix missing-example claim no longer holds: " + row["record"])
        return {"records": rows, "absent_rules": list(absent),
                "scope": "All committed non-reserved runner run.json and stage-11 census step records; no search or acceptance-seed reads."}

    def height_tree(self, rule, word, before, after, fields, subphase):
        namespace = dict(api=self.appendix_api, RULE_SOURCES={subphase: Path(subphase + ".py")})
        functions(self.inputs, S13 + "/rules_height.py", ("_proposal",), namespace)
        proposal = namespace["_proposal"](types.SimpleNamespace(subphase=subphase), {"digest": before},
            dict(letters=word, rule=rule, output_digest=after, fields=fields), subphase, [])
        self.appendix_api.check_tree(proposal.tree, len(word))
        return proposal.tree

    def appendix_u2(self):
        rows = self.inputs.lines(ROUND + "/r22d81_pairs_cap311_tail.txt")
        start = next(row for tag, row in rows if tag == "START")
        step = next(row for tag, row in rows if tag == "STEP")
        tape = self.inputs.json(U2)
        word = [row["letter"] for row in tape["letters"][:step["length"]]]
        require_equal(word, step["letters"], "U2 committed letters")
        require_equal(tape["digest"], start["digest"], "U2 input digest")
        require_equal(tape["letters"][len(word) - 1]["digest"], step["digest"], "U2 output digest")
        require_equal(tape["letters"][0]["u"], step["u_mid"], "U2 intermediate u")
        fields = dict(tau_before=start["tau"], tau_after=step["tau"], u_before=start["u"],
                      u_mid=step["u_mid"], u_after=step["u"])
        return dict(tree=self.height_tree("r22d88/U2", word, start["digest"], step["digest"], fields, "r22d81_pairs"),
                    word=word, before_digest=start["digest"], after_digest=step["digest"], measures=fields,
                    heights_before=start["n2_n4_n6"], heights_after=step["n2_n4_n6"],
                    source=S13 + "/rules_height.py:_proposal", note="Fixed tau=311 experiment, not a runner step.")

    def appendix_h4_pair(self):
        parsed = []
        for shard in range(3):
            rows = self.inputs.lines(CLOUD + f"/two_letter_u_round2_shard{shard}.txt")
            context = next(row for tag, row in rows if tag == "CONTEXT")
            tag, end = rows[-1]
            parsed.append(dict(context=context, firsts=[row for tag, row in rows if tag == "FIRST"],
                               status=tag.lower() if tag in ("DONE", "BUDGET") else "truncated",
                               end=dict(tag=tag, payload=end) if tag in ("DONE", "BUDGET") else None))
        namespace = dict(re=re, OutputError=ValueError, RULE_SOURCES={"two_letter_u": Path("two_letter_u.py")})
        functions(self.inputs, S13 + "/rules_height.py",
                  ("_need", "_integer", "_letter", "_pair_choice"), namespace)
        choice = namespace["_pair_choice"](parsed)
        require_equal(choice["result"], "step", "h4 pair choice")
        step = choice["step"]
        tape = [row for tag, row in self.inputs.lines(CLOUD + "/end_summary.txt") if tag == "LETTER"]
        require_equal(step["input_digest"], tape[19]["digest"], "h4 pair input digest")
        require_equal(step["letters"], [row["letter"] for row in tape[20:22]], "h4 pair letters")
        after = tape[21]["digest"]
        return dict(tree=self.height_tree(step["rule"], step["letters"], step["input_digest"], after,
                                         step["fields"], "two_letter_u"), word=step["letters"],
                    before_digest=step["input_digest"], after_digest=after, measures=step["fields"],
                    heights_before=tape[19]["n2_n4_n6"], heights_after=tape[21]["n2_n4_n6"],
                    source=S13 + "/rules_height.py:_proposal", note="Committed cloud-line pair; no runner record holds it.")

    def tail_tree(self, rule, word):
        # Extract only the original method. Importing runner.py would load its
        # process controller and CLI, neither of which is needed for a tree.
        path = S13 + "/runner.py"
        source = ast.parse(self.inputs.text(path), filename=path)
        cls = next(n for n in source.body if isinstance(n, ast.ClassDef) and n.name == "Runner")
        method = next(n for n in cls.body if isinstance(n, ast.FunctionDef) and n.name == "step_tree")
        method.decorator_list = []
        namespace = dict(api=self.appendix_api)
        exec(compile(ast.Module(body=[method], type_ignores=[]), path, "exec"), namespace)
        tree = namespace["step_tree"](rule, None, len(word))
        self.appendix_api.check_tree(tree, len(word))
        return tree

    def appendix_pure(self):
        row = self.inputs.json(PURE + "/steps/b17-pure.json")
        word = self.inputs.json(PURE + "/words/b17.json")
        require_equal(word["input_digest"], row["start_digest"], "pure input digest")
        require_equal(word["output_digest"], row["end_digest"], "pure output digest")
        require_equal(len(word["word"]), row["letters"], "pure length")
        state = self.state()
        self.mechanics.orient.set_pose_table(state, self.inputs.json(PURE + "/tables/823.json.gz"))
        before = check_digest(state, row["start_digest"])
        pose, lattice = self.mechanics.E.stable_pose(state).copy(), state.lattice_flags().copy()
        require_equal(int(state.off_lattice_count()), row["off_before"], "pure off before")
        self.play(state, word["word"])
        after = check_digest(state, row["end_digest"])
        require_equal(after, self.inputs.json(PURE + "/tables/737.json.gz")["digest"], "pure end table digest")
        require_equal(int(state.off_lattice_count()), row["off_after"], "pure off after")
        np = self.mechanics.np
        now, exact = state.lattice_flags(), pose == self.mechanics.E.stable_pose(state)
        require_equal(int((lattice & ~now).sum()), row["collateral"], "pure collateral")
        require_equal(int((~lattice & ~now & exact).sum()), row["others_exact"], "pure others exact")
        require_equal(int((~lattice & now).sum()), row["unit_on_lattice"], "pure landed")
        require_equal(row["strict_prime"], True, "pure accepted gate")
        return dict(tree=self.tail_tree("pure", word["word"]), letters=len(word["word"]),
                    word_file=PURE + "/words/b17.json", before_digest=before, after_digest=after,
                    off_before=row["off_before"], off_after=row["off_after"],
                    moved=np.flatnonzero(~exact).tolist(), source=S13 + "/runner.py:Runner.step_tree")

    def lattice_tree_functions(self):
        return functions(self.inputs, S13 + "/lattice_phase.py", ("orbit_tree", "correction_tree"),
                         dict(api=self.appendix_api, TREE_PART_LIMIT=10**6))

    @cached_property
    def recipe_exporter(self):
        # Import definitions only; its main() is guarded. No export or capture.
        self.inputs.raw(J + "/viewer/export_tutorial.py")
        spec = importlib.util.spec_from_file_location("figdata_recipe_exporter", input_path(J + "/viewer/export_tutorial.py"))
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def appendix_correction(self):
        summary = self.inputs.json(ROUND + "/line-a/literal/lattice/restore-summary.json")
        chapter = self.inputs.json(T + "/records/ch06-examples.json")["F_depth40"]
        require_equal(summary["runs"][0]["T_digest"], chapter["T_digest"], "correction target digest")
        raw = self.inputs.raw(LIBRARY)
        if summary["library"]["sha256"] not in (sha256(raw), sha256(gzip.decompress(raw))):
            raise ValueError("correction library digest mismatch")
        library = json.loads(gzip.decompress(raw))
        correction = summary["runs"][0]["corrections"][0]
        macro = library["macros"][correction["macro"]]
        word = self.recipe_exporter.correction_words(dict(macros=[macro]))[0]
        require_equal(len(word), chapter["corrections"][0]["letters"], "correction length")
        tree = self.lattice_tree_functions()["correction_tree"](0, correction, macro)
        self.appendix_api.check_tree(tree, len(word))
        before = {str(o): list(chapter["nu_input"].get(str(o), [0, 0])) for o in range(35)}
        after = {}
        for o, value in before.items():
            delta = macro["nu"].get(o, [0, 0])
            modulus = 5 if int(o) in (17, 28) else 2
            after[o] = [(value[0] + delta[0]) % 2, (value[1] + delta[1]) % modulus]
        return dict(tree=tree, word=word, macro=correction["macro"], nu_before=before, nu_after=after,
                    representative_digest=chapter["T_digest"], source=S13 + "/lattice_phase.py:correction_tree",
                    note="The recorded program's first correction; pictures use the label-equivalent representative T, not the uncommitted Y.R pose state.")

    def appendix_late_cycle(self):
        m = self.mechanics
        import literal_lattice as LL
        self.inputs.raw(S13 + "/literal_lattice.py")
        chapter = self.inputs.json(T + "/records/ch06-examples.json")
        doc = self.inputs.json(LATE)
        program = LL.Program(doc["nodes"], self.inputs.json("assets/execution_trees.json.gz"))
        all_factors = list(doc["solutions"].values())[0]
        first_step = chapter["E_late"]["steps"][0]
        require_equal(first_step["orbit"], 6, "late first orbit")
        factors = all_factors[:first_step["factors"]]
        observer = LL.ObserverLetters(m.E.sim, m.C, m.M)
        codes = [observer.primitives[move] for j, sg in factors for move in program.expand(j, sg)]
        raw = array.array("H", codes)
        if sys.byteorder != "little":
            raw.byteswap()
        require_equal(sha256(raw.tobytes()), chapter["E_late"]["orbit6"]["word_sha256"], "late orbit word digest")
        build = self.lattice_tree_functions()["orbit_tree"]
        full_tree, _, _ = build(program, factors, 6)
        tree_bytes = (json.dumps(full_tree, separators=(",", ":"), allow_nan=False) + "\n").encode()
        require_equal(sha256(tree_bytes), chapter["E_late"]["orbit6"]["tree_sha256"], "late orbit tree digest")
        tree, _, _ = build(program, factors[:2], 6)
        word = [list(divmod(code, 60)) for code in codes[:tree["letters"]]]
        state = self.state()
        m.orient.set_pose_table(state, self.inputs.json(D0 + "/tables/bulk-end.json"))
        before = check_digest(state, chapter["C_s11"]["start"]["digest"])
        pose = m.E.stable_pose(state).copy()
        with m.np.load(input_path("assets/model.npz"), allow_pickle=False) as model:
            sp, orbit = model["slot_piece"], model["orbit_id"]
        _, first_slots = m.np.unique(sp, return_index=True)
        pieces = m.np.flatnonzero(orbit == 6)

        def misplaced():
            positions = sp[LL.state_labels(state)[first_slots]]
            return int((positions[pieces] != pieces).sum())

        counts, digests = [misplaced()], [before]
        at = 0
        for part in tree["parts"]:
            self.play(state, word[at:at + part["letters"]])
            at += part["letters"]
            counts.append(misplaced())
            digests.append(state.digest())
        self.appendix_api.check_tree(tree, len(word))
        return dict(tree=tree, word=word, factors=factors[:2], before_digest=before, after_digest=state.digest(),
                    boundary_digests=digests, orbit=6, misplaced_positions=counts,
                    off_before=0, off_after=int(state.off_lattice_count()),
                    moved=m.np.flatnonzero(pose != m.E.stable_pose(state)).tolist(),
                    source=S13 + "/lattice_phase.py:orbit_tree",
                    note="First two factors of the committed late program; no committed runner record holds this step.")


def compute(only=None):
    selected = set(only) if only is not None else set(NAMES)
    if not selected or not selected <= set(NAMES):
        raise ValueError("unknown figure selection: " + ", ".join(sorted(selected - set(NAMES))))
    computation = Computation()
    methods = dict(zip(NAMES, ("bulk", "height_counts", "prediction", "cap_prunes", "twisted_cycle",
                              "appendix_inventory", "appendix_u2", "appendix_h4_pair", "appendix_pure",
                              "appendix_correction", "appendix_late_cycle", "point_model", "class_cells")))
    figures = {}
    for name in NAMES:
        if name in selected:
            print("Computing " + name, flush=True)
            value = getattr(computation, methods[name])()
            inputs = {path: sha256(computation.inputs.raw(path)) for path in evidence_paths(name)}
            figures[name] = dict(value, inputs=inputs)
    # JSON object keys (the height indexes in F_h/O_h/r_h) are strings on disk.
    # Normalize before comparing, so --check compares the same representation.
    document = {"format": FORMAT, "figures": figures}
    if "appa-late-cycle-tree" in figures:
        # A literal expansion of committed factors, for recipe_word's existing
        # JSON-word seam. It is not a searched or constructed solving word.
        document["word"] = figures["appa-late-cycle-tree"]["word"]
    return json.loads(json.dumps(document))


def check(saved, fresh, only=None):
    if saved.get("format") != FORMAT or not isinstance(saved.get("figures"), dict):
        return sorted(fresh["figures"])
    different = {name for name, value in fresh["figures"].items() if saved["figures"].get(name) != value}
    if "word" in fresh and saved.get("word") != fresh["word"]:
        different.add("appa-late-cycle-tree")
    if only is None:
        different.update(set(saved["figures"]) - set(fresh["figures"]))
    return sorted(different)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--only", choices=NAMES, nargs="+", metavar="FIGURE")
    args = parser.parse_args(argv)
    path = HERE / "figdata.json"
    try:
        fresh = compute(args.only)
        saved = json.loads(path.read_bytes()) if path.exists() else {}
        if args.check:
            different = check(saved, fresh, args.only)
            if different:
                print("Figure data differ: " + ", ".join(different))
                return 1
            print("Figure data match the committed-state recomputation.")
        else:
            if args.only and saved:
                require_equal(saved.get("format"), FORMAT, "saved format")
                fresh["figures"] = dict(saved["figures"], **fresh["figures"])
                if "word" in saved and "word" not in fresh:
                    fresh["word"] = saved["word"]
            path.write_text(json.dumps(fresh, indent=2, sort_keys=True, ensure_ascii=False) + "\n", encoding="utf-8")
            print("Wrote " + str(len(fresh["figures"])) + " figure data sets.")
        return 0
    except (OSError, ValueError, KeyError, TypeError, AssertionError, StopIteration, IndexError) as error:
        print("Figure data error: " + str(error), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
