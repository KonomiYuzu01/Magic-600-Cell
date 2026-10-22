"""X3: whole-word J1 previews from recorded start states.

Each letter is applied exactly once, in order. A refusal ends the preview at
that prefix. Timings measure research code on this host, not a product engine.
No word-length policy is imposed by this measurement.
"""

import sys

sys.dont_write_bytecode = True

import argparse
import json
from pathlib import Path
from statistics import median
from time import perf_counter

ROOT = Path(__file__).resolve().parents[4]
JUMBLING = ROOT / "research" / "jumbling"
if str(JUMBLING) not in sys.path:
    sys.path.insert(0, str(JUMBLING))

import numpy as np
import sim
from differential import harness, j1_state, menu_order
from differential.leads import census, heights

U1_PATH = JUMBLING / "restoration/12-low-height/runs/lc13-k40/steps/u1.json"


def validate_letters(letters, menu):
    """Validate the complete word before any application (including bool/negative ids)."""
    if not isinstance(letters, list):
        raise ValueError("word must be a list of [grip, item] letters")
    for i, letter in enumerate(letters):
        if (not isinstance(letter, (list, tuple)) or len(letter) != 2
                or type(letter[0]) is not int or not 0 <= letter[0] < 600
                or type(letter[1]) is not int or not 0 <= letter[1] < len(menu)):
            raise ValueError(f"letter {i} must contain a grip in 0..599 and a menu item")


def load_word(path):
    doc = json.loads(Path(path).read_text(encoding="utf-8"))
    if not isinstance(doc, dict) or not isinstance(doc.get("word"), list):
        raise ValueError("word file must be an object with a word list")
    # The harness checks letter ids and the pinned I_a menu order without applying.
    harness.load_steps({"from": "solved", "word": doc["word"]}, census.menu().record())
    for field in ("input_digest", "output_digest"):
        value = doc.get(field)
        if field in doc and (not isinstance(value, str) or len(value) != 64
                             or any(c not in "0123456789abcdef" for c in value)):
            raise ValueError(f"{field} must be a SHA-256 digest")
    return doc


def check_input_digest(state, doc):
    if "input_digest" in doc and doc["input_digest"] != state.digest():
        raise ValueError("word input_digest differs from the start state")


def then_letters(value):
    if value is None:
        return []
    if isinstance(value, list):
        return value
    try:
        return json.loads(value)
    except ValueError:
        return json.loads(Path(value).read_text(encoding="utf-8"))


def start_state(spec=("solved",), *, then=None, start_digest=None, ctx=None):
    """Build solved/census/record end states, then apply the untimed start prefix."""
    ctx = sim.get_context() if ctx is None else ctx
    menu = census.menu()
    if list(spec) == ["solved"]:
        state = sim.State(ctx, menu=menu)
    elif len(spec) == 2 and spec[0] in ("census", "record"):
        value = int(spec[1]) if spec[0] == "census" else Path(spec[1])
        state = None
        for step in census.walk(value, end=True):
            if step.rule is None:
                state = step.S.st
        if state is None:
            raise ValueError("census walk did not yield its end state")
    else:
        raise ValueError("start must be solved, census SEED, or record PATH")
    prefix = [] if then is None else then
    validate_letters(prefix, menu)
    for i, twist in enumerate(j1_state.twists_from_letters(menu, prefix)):
        result = state.apply(twist, journal=False)
        if not result.applied:
            raise ValueError(f"--then letter {i} is {result.status}")
    if start_digest is not None and state.digest() != start_digest:
        raise ValueError("--start-digest differs from the rebuilt start state")
    return state


def preview(state, letters, menu, *, label, sample_every=100):
    """One chronological replay; effects compare immutable exact keys, not pose ids.

    ``seconds.total`` sums apply-only letter times; twist construction and wall
    time for the complete preview are reported separately. Samples include step
    zero and the final attempted step. Refusal indices are 0-based and steps are
    1-based. The state's classify attribute is restored even if apply raises.
    """
    if type(sample_every) is not int or sample_every < 1:
        raise ValueError("sample_every must be a positive integer")
    validate_letters(letters, menu)
    if state.menu is not None:
        menu_order.check_menu(menu.record(), identity=state.menu.identity,
                              order=menu_order.order_digest(state.menu.record()))
    started = perf_counter()
    start_digest = state.digest()
    before = state.lattice_flags()
    before_ids = state.pose_id.copy()
    before_keys = [p.key for p in state._poses]
    rows, samples, times = [], [], []
    height_cache = {}

    def sample(step):
        h, _ = heights.entry_height(state, height_cache)
        return {"step": step, "off_lattice": state.off_lattice_count(),
                "poses": len(state._poses), "height_bits": h.bit_length()}

    samples.append(sample(0))
    t = perf_counter()
    twists = j1_state.twists_from_letters(menu, letters)
    construction_seconds = perf_counter() - t
    refused = None
    applied = 0
    classification = [None]
    original_classify = state.classify
    missing = object()
    previous_attribute = state.__dict__.get("classify", missing)

    def measured_classify(*args, **kwargs):
        classification[0] = original_classify(*args, **kwargs)
        return classification[0]

    # J1 Outcome retains the exact count but omits the filtered count. Capture
    # its actual classification, without another classify or any J1 changes.
    state.classify = measured_classify
    try:
        for index, (letter, twist) in enumerate(zip(letters, twists)):
            classification[0] = None
            t = perf_counter()
            outcome = state.apply(twist, journal=False)
            elapsed = perf_counter() - t
            cl = classification[0]
            step = index + 1
            times.append(elapsed)
            rows.append({"label": label, "step": step, "index": index,
                         "grip": letter[0], "item": letter[1], "status": outcome.status,
                         "seconds": elapsed, "moved": outcome.moved,
                         "exact_evaluations": outcome.exact_evaluations,
                         "filtered_evaluations": 0 if cl is None else cl.filtered_evaluations})
            if step % sample_every == 0:
                samples.append(sample(step))
            if not outcome.applied:
                refused = {"index": index, "step": step, "status": outcome.status}
                break
            applied += 1
    finally:
        if previous_attribute is missing:
            del state.classify
        else:
            state.classify = previous_attribute
    if samples[-1]["step"] != len(rows):
        samples.append(sample(len(rows)))
    after = state.lattice_flags()
    other_off = np.flatnonzero(~before & ~after)
    unchanged = sum(before_keys[int(before_ids[p])] == state._poses[int(state.pose_id[p])].key
                    for p in other_off)
    return {"label": label, "status": "refused" if refused else "complete",
            "refused": refused, "expanded_length": len(letters), "steps": applied,
            "attempted_steps": len(rows), "start_digest": start_digest,
            "end_digest": state.digest(), "off_before": int((~before).sum()),
            "off_after": int((~after).sum()),
            "lattice_pieces_lifted_off": int((before & ~after).sum()),
            "off_pieces_landed": int((~before & after).sum()),
            "other_off_exact_pose_unchanged": int(unchanged),
            "other_off_exact_pose_changed": int(len(other_off) - unchanged),
            "letters": rows, "samples": samples,
            "seconds": {"total": sum(times), "median": median(times) if times else 0.0,
                        "p95": float(np.percentile(times, 95)) if times else 0.0,
                        "largest": max(times, default=0.0),
                        "twist_construction": construction_seconds,
                        "preview_wall": perf_counter() - started}}


def preview_word(state, doc, *, label, sample_every=100):
    """Check a word file's input before applying; check its output at completion."""
    menu = state.menu if state.menu is not None else census.menu()
    validate_letters(doc["word"], menu)
    if "input_digest" in doc and doc["input_digest"] != state.digest():
        result = preview(state, [], menu, label=label, sample_every=sample_every)
        result.update(status="refused", expanded_length=len(doc["word"]),
                      refused={"index": None, "step": 0, "status": "input_digest_mismatch"},
                      checks=[{"field": "input_digest", "expected": doc["input_digest"],
                               "actual": result["start_digest"], "passed": False}])
        return result
    result = preview(state, doc["word"], menu, label=label, sample_every=sample_every)
    result["checks"] = []
    for field, actual in (("input_digest", result["start_digest"]),
                          ("output_digest", result["end_digest"])):
        if field in doc:
            passed = doc[field] == actual
            result["checks"].append({"field": field, "expected": doc[field], "actual": actual,
                                     "passed": passed})
            if not passed and result["status"] == "complete":
                result["status"] = "failed_reference"
    return result


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", required=True)
    parser.add_argument("--u1", action="store_true")
    parser.add_argument("--correction35", action="store_true")
    parser.add_argument("--h6030", action="store_true")
    parser.add_argument("--word")
    parser.add_argument("--start", nargs="+", help="solved, census SEED, or record PATH")
    parser.add_argument("--then", help="JSON list of letters or a JSON list file")
    parser.add_argument("--start-digest")
    parser.add_argument("--label")
    parser.add_argument("--sample-every", type=int, default=100)
    args = parser.parse_args(argv)
    out = heights.output_directory(args.out)
    if args.sample_every < 1:
        raise ValueError("--sample-every must be positive")
    if args.word and args.start is None:
        raise ValueError("--word requires --start")
    if not args.word and (args.start or args.then or args.start_digest or args.label):
        raise ValueError("custom start and label options require --word")
    selected = args.u1 or args.correction35 or args.h6030 or args.word

    def inputs():
        if args.u1 or not selected:
            yield start_state(("census", "7101")), load_word(U1_PATH), "u1", [U1_PATH, census.load(7101)[0]]
        if args.correction35 or not selected:
            source = heights.correction35()
            yield source.state, {"word": source.letters, "output_digest": heights.CORRECTION35_DIGEST}, source.label, source.sources
        if args.h6030 or not selected:
            source = heights.h6030()
            yield source.state, {"word": source.letters, "output_digest": heights.H6030_DIGEST}, source.label, source.sources
        if args.word:
            state = start_state(args.start, then=then_letters(args.then), start_digest=args.start_digest)
            sources = [Path(args.word)]
            if args.start[0] == "record":
                sources.append(Path(args.start[1]))
            elif args.start[0] == "census":
                sources.append(census.load(int(args.start[1]))[0])
            yield state, load_word(args.word), args.label or Path(args.word).stem, sources

    results = []
    with (out / "x3-letters.jsonl").open("w", encoding="utf-8") as stream:
        for state, doc, label, paths in inputs():
            result = preview_word(state, doc, label=label, sample_every=args.sample_every)
            for row in result.pop("letters"):
                stream.write(json.dumps(row) + "\n")
            result["sources"] = [heights.input_evidence(p) for p in paths]
            results.append(result)
    command = ["python", "-B", str(Path(__file__).resolve().relative_to(ROOT)),
               *(sys.argv[1:] if argv is None else argv)]
    report = dict(heights.environment(command=command), inputs=results, sample_every=args.sample_every)
    (out / "x3-summary.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return report


if __name__ == "__main__":
    main()
