"""Recompute the tutorial's missing observations from committed research inputs.

No runner, private snapshots, acceptance seeds or persistent simulation caches.
The CLI writes figdata.json only after every selected computation succeeds.
"""

import argparse
import array
import ast
from collections import Counter
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
         "ch04-six-stops-caps", "ch06-twisted-cycle")


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
    }
    return sorted(set(common + by_name[name]))


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
    """Load named definitions, never a research script's CLI or run loop.

    An optional observer adds a read-only callback to the count prediction.
    Arithmetic, ordering and classification stay those of the original function.
    """
    tree = ast.parse(inputs.text(path), filename=path)
    definitions = [node for node in tree.body if isinstance(node, ast.FunctionDef) and node.name in names]
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


def compute(only=None):
    selected = set(only) if only is not None else set(NAMES)
    if not selected or not selected <= set(NAMES):
        raise ValueError("unknown figure selection: " + ", ".join(sorted(selected - set(NAMES))))
    computation = Computation()
    methods = dict(zip(NAMES, ("bulk", "height_counts", "prediction", "cap_prunes", "twisted_cycle")))
    figures = {}
    for name in NAMES:
        if name in selected:
            print("Computing " + name, flush=True)
            value = getattr(computation, methods[name])()
            inputs = {path: sha256(computation.inputs.raw(path)) for path in evidence_paths(name)}
            figures[name] = dict(value, inputs=inputs)
    # JSON object keys (the height indexes in F_h/O_h/r_h) are strings on disk.
    # Normalize before comparing, so --check compares the same representation.
    return json.loads(json.dumps({"format": FORMAT, "figures": figures}))


def check(saved, fresh, only=None):
    if saved.get("format") != FORMAT or not isinstance(saved.get("figures"), dict):
        return sorted(fresh["figures"])
    different = {name for name, value in fresh["figures"].items() if saved["figures"].get(name) != value}
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
            path.write_text(json.dumps(fresh, indent=2, sort_keys=True, ensure_ascii=False) + "\n", encoding="utf-8")
            print("Wrote " + str(len(fresh["figures"])) + " figure data sets.")
        return 0
    except (OSError, ValueError, KeyError, TypeError, AssertionError, StopIteration, IndexError) as error:
        print("Figure data error: " + str(error), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
