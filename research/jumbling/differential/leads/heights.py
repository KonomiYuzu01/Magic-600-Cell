"""Entry-height and exact-sign operand measurements on J1 replays.

Run with ``python -B .../heights.py --out /outside/the/repository``.  These
are source/fixture measurements of research code, not product performance
evidence.  A height includes denominators and ignores K+ poses, as wj.height
does; it is measured on the live (compacted) pose table, not its history.
"""

import sys

sys.dont_write_bytecode = True

import argparse
from copy import deepcopy
from dataclasses import dataclass, field, replace
import gzip
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
from time import perf_counter

ROOT = Path(__file__).resolve().parents[4]
JUMBLING = ROOT / "research" / "jumbling"
if str(JUMBLING) not in sys.path:
    sys.path.insert(0, str(JUMBLING))

import numpy as np
import sim
from differential import harness, j1_state, menu_order
from differential.leads import census
from sim import kernel

EVIDENCE_KIND = "source and fixture replay with the J1 reference engine; cloud research timings, not product performance evidence"
H6030_DIGEST = "cb1e89ed6b0b1581f8acea27e22b443df5784d928d4284579d69c444f60b37ad"
CORRECTION35_DIGEST = "80642dd327e7c86b085019db9fda41c87a2f540386d2bfeb90397bf1abf325d1"
WIDTHS = (31, 53, 63, 64)


def output_directory(path):
    """Resolve and validate before creating anything, including parent directories."""
    path = Path(path).resolve()
    if path.is_relative_to(ROOT):
        raise ValueError("--out must be outside the repository")
    path.mkdir(parents=True, exist_ok=True)
    return path


def environment(evidence_kind=EVIDENCE_KIND, *, command=None):
    try:
        commit = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                capture_output=True, text=True, check=True,
                                timeout=10).stdout.strip()
    except (OSError, subprocess.SubprocessError):
        commit = None
    return {"cpu_count": os.cpu_count(), "python": platform.python_version(),
            "numpy": np.__version__, "commit": commit, "evidence_kind": evidence_kind,
            "production_command": command}


def input_evidence(path):
    path = Path(path)
    name = str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else path.name
    return {"file": name, "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}


def entry_height(state, cache=None):
    """wj.height, caching each immutable exact pose key (never a pose-table id)."""
    cache = {} if cache is None else cache
    height = denominator = 0
    for pose in state._poses:
        if pose.kidx >= 0:
            continue
        value = cache.get(pose.key)
        if value is None:
            h = d = 0
            for row in pose.matrix:
                for x in row:
                    h = max(h, abs(x.a), abs(x.b), x.d)
                    d = max(d, x.d)
            value = cache[pose.key] = (h, d)
        height = max(height, value[0])
        denominator = max(denominator, value[1])
    return height, denominator


@dataclass
class ReplayInput:
    """A solved/start state, lazy twists, and references checked during replay.

    Pass the input itself as ``twists`` to run_heights to retain its reference
    checks. A prefix leaves later stages explicitly unchecked.
    """

    state: object
    label: str
    records: list | None = None
    letters: list | None = None
    stages: dict = field(default_factory=dict)
    digests: list | None = None
    cap0_heights: list | None = None
    sources: list = field(default_factory=list)

    def __len__(self):
        return len(self.records if self.records is not None else self.letters)

    def __iter__(self):
        if self.records is not None:
            for record in self.records:
                yield sim.Twist.from_record(self.state.ctx, record)
        else:
            yield from j1_state.twists_from_letters(self.state.menu, self.letters)

    def prefix(self, count):
        if type(count) is not int or count < 0:
            raise ValueError("prefix length must be a nonnegative integer")
        return replace(self, records=None if self.records is None else self.records[:count],
                       letters=None if self.letters is None else self.letters[:count])


def fixture(name, *, ctx=None):
    name = name.removeprefix("wj-")
    if name not in ("I_a", "S4", "I_b"):
        raise ValueError(f"unknown W-J fixture: {name}")
    path = JUMBLING / "fixtures" / f"wj-{name}.json"
    doc = json.loads(path.read_text(encoding="utf-8"))
    steps = harness.load_steps(doc)
    ctx = sim.get_context() if ctx is None else ctx
    menu = sim.TwistMenu.from_record(ctx, steps.menu)
    menu_order.check_menu(menu.record(), order=steps.menu_order_digest)
    return ReplayInput(sim.State(ctx, menu=menu), f"wj-{name}",
                       records=[s["record"] for s in steps.steps],
                       stages=deepcopy(doc["stages"]), sources=[path])


def journal(name, *, ctx=None):
    path = JUMBLING / "sim" / "acceptance.json"
    doc = json.loads(path.read_text(encoding="utf-8"))
    if name == "negative_control":
        source = doc[name]
    elif name in ("E2", "E3", "random_0", "random_1"):
        source = doc["grouped_per_piece"][name]
    else:
        raise ValueError(f"unknown acceptance journal: {name}")
    ctx = sim.get_context() if ctx is None else ctx
    menu_record = source.get("menu")
    steps = harness.load_steps(source["journal"], menu_record)
    menu = None if menu_record is None else sim.TwistMenu.from_record(ctx, menu_record)
    return ReplayInput(sim.State(ctx, menu=menu), name,
                       records=[s["record"] for s in steps.steps],
                       digests=source.get("digests"),
                       cap0_heights=source.get("cap_0_pose_entry_heights"), sources=[path])


def h6030(*, ctx=None):
    path = JUMBLING / "restoration/10-x1-unbounded/witnesses/H6030.json"
    doc = json.loads(path.read_text(encoding="utf-8"))
    menu = census.menu()  # Includes the pinned I_a item-order check.
    menu_order.check_menu(menu.record(), order=menu_order.I_A_ORDER_DIGEST)
    steps = harness.load_steps(doc, menu.record())
    ctx = sim.get_context() if ctx is None else ctx
    return ReplayInput(sim.State(ctx, menu=menu), "H6030", letters=doc["word"],
                       stages={"end": {"records": len(steps.steps), "digest": H6030_DIGEST}},
                       sources=[path])


def correction35(*, ctx=None, limit=None):
    path = JUMBLING / "restoration/09-general-method/data/library-merged.json.gz"
    with gzip.open(path, "rt", encoding="utf-8") as stream:
        doc = json.load(stream)
    if doc["format"] != "I_a-lattice-correction-library-v1":
        raise ValueError("unexpected correction library format")
    macro = next(m for m in doc["macros"] if m["id"] == 35)
    if (macro["family"], macro["length"], len(macro["records"])) != ("card5:MB2-5", 5629, 5629):
        raise ValueError("unexpected correction 35 source")
    if limit is not None and (type(limit) is not int or limit < 0):
        raise ValueError("prefix length must be a nonnegative integer")
    records = macro["records"] if limit is None else macro["records"][:limit]
    ctx = sim.get_context() if ctx is None else ctx
    menu = census.menu()
    return ReplayInput(sim.State(ctx, menu=menu), "correction35",
                       letters=j1_state.letters_from_records(ctx, menu, records),
                       stages={"end": {"records": 5629, "digest": CORRECTION35_DIGEST}},
                       sources=[path])


def run_heights(state, twists, *, label, every=1, widths=False):
    """Apply in chronological order; stop on the first refusal, restoring sign5 on exit.

    Steps are 1-based; ``index`` in a refusal is the 0-based input index.
    Crossings and maxima are checked every step, independently of row sampling.
    With widths=True, operand_widths contains a row for every attempted step.
    Replay seconds time apply only; bookkeeping seconds include observations
    and reference checks, excluding twist construction and iterator overhead.
    """
    if type(every) is not int or every < 1:
        raise ValueError("every must be a positive integer")
    source = twists if isinstance(twists, ReplayInput) else None
    stages = source.stages if source is not None else {}
    rows, stage_checks, digest_checks, centre_checks, operand_widths = [], [], [], [], []
    cache = {}
    crossings = {str(k): None for k in WIDTHS}
    denominator_crossings = {str(k): None for k in WIDTHS}
    maxima = {"height": {"value": -1}, "denominator": {"value": -1}}
    replay_seconds = bookkeeping_seconds = 0.0
    refused = None
    applied = step = 0
    bits = [0, 0]
    original_sign5 = kernel.sign5

    def measured_sign5(p, q):
        bits[0] = max(bits[0], int(p).bit_length())
        bits[1] = max(bits[1], int(q).bit_length())
        return original_sign5(p, q)

    def observe(n):
        h, d = entry_height(state, cache)
        for key, value, found in (("height", h, crossings),
                                  ("denominator", d, denominator_crossings)):
            if value > maxima[key]["value"]:
                maxima[key] = {"value": value, "step": n, "bits": value.bit_length()}
            for width in WIDTHS:
                if found[str(width)] is None and value >= 1 << width:
                    found[str(width)] = {"step": n, "bits": value.bit_length()}
        row = {"label": label, "step": n, "max_entry_height": h,
               "max_denominator": d, "height_bits": h.bit_length(),
               "denominator_bits": d.bit_length()}
        at_stage = [(name, ref) for name, ref in stages.items() if ref["records"] == n]
        check_digest = source is not None and source.digests is not None and n < len(source.digests)
        if n % every == 0 or at_stage:
            row.update(off_lattice=state.off_lattice_count(), poses=len(state._poses))
        digest = state.digest() if at_stage or check_digest else None
        for name, ref in at_stage:
            actual = dict(row, digest=digest, poses=len(state._poses),
                          off_lattice=state.off_lattice_count())
            fields = ("digest", "max_entry_height", "max_denominator", "off_lattice", "poses")
            checks = {k: {"expected": ref[k], "actual": actual[k], "passed": ref[k] == actual[k]}
                      for k in fields if k in ref}
            stage_checks.append({"stage": name, "step": n, "passed": all(
                v["passed"] for v in checks.values()), "fields": checks})
        if check_digest:
            expected = source.digests[n]
            row["digest"] = digest
            digest_checks.append({"step": n, "expected": expected, "actual": digest,
                                  "passed": expected == digest})
        if source is not None and source.cap0_heights is not None and n and n % 2 == 0:
            centre = int(state.ctx.data.centre_pieces[state.ctx.data.centre_poles == 0][0])
            value = max(max(abs(x.a), abs(x.b), x.d) for r in state.pose(centre) for x in r)
            expected = source.cap0_heights[n // 2 - 1]
            row["cap_0_pose_entry_height"] = value
            centre_checks.append({"step": n, "expected": expected, "actual": value,
                                  "passed": expected == value})
        return row

    started = perf_counter()
    t = perf_counter()
    start_digest = state.digest()
    rows.append(observe(0))
    bookkeeping_seconds += perf_counter() - t
    last_row = rows[0]
    if widths:
        kernel.sign5 = measured_sign5
    try:
        for step, twist in enumerate(twists, 1):
            bits[:] = (0, 0)
            t = perf_counter()
            outcome = state.apply(twist, journal=False)
            replay_seconds += perf_counter() - t
            t = perf_counter()
            last_row = observe(step)
            last_row["status"] = outcome.status
            if widths:
                width = {"step": step, "p_bits": bits[0], "q_bits": bits[1],
                         "max_operand_bits": max(bits)}
                operand_widths.append(width)
                last_row["sign_operand_bits"] = max(bits)
            if step % every == 0:
                rows.append(last_row)
            if not outcome.applied:
                refused = {"index": step - 1, "step": step, "status": outcome.status}
            else:
                applied += 1
            bookkeeping_seconds += perf_counter() - t
            if refused is not None:
                break
    finally:
        if widths:
            kernel.sign5 = original_sign5
    t = perf_counter()
    if rows[-1]["step"] != step:
        last_row.update(off_lattice=state.off_lattice_count(), poses=len(state._poses))
        rows.append(last_row)
    checked = {c["stage"] for c in stage_checks}
    stage_checks += [{"stage": name, "step": ref["records"], "passed": None,
                      "reason": "stage not reached"} for name, ref in stages.items() if name not in checked]
    end_digest = state.digest()
    bookkeeping_seconds += perf_counter() - t
    checks = stage_checks + digest_checks + centre_checks
    return {"label": label, "steps": applied, "attempted_steps": step,
            "status": "refused" if refused else "complete", "refused": refused,
            "start_digest": start_digest, "end_digest": end_digest,
            "rows": rows, "maxima": maxima, "crossings": crossings,
            "denominator_crossings": denominator_crossings,
            "stage_checks": stage_checks, "digest_checks": digest_checks,
            "cap0_height_checks": centre_checks,
            "checks_passed": all(c["passed"] is not False for c in checks),
            "operand_widths": operand_widths if widths else None,
            "max_sign_operand_bits": max((w["max_operand_bits"] for w in operand_widths), default=0) if widths else None,
            "seconds": {"replay": replay_seconds, "bookkeeping": bookkeeping_seconds,
                        "total": perf_counter() - started}}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", required=True)
    parser.add_argument("--fixture", action="append", default=[])
    parser.add_argument("--journals", action="store_true")
    parser.add_argument("--h6030", action="store_true")
    parser.add_argument("--correction35", action="store_true")
    parser.add_argument("--word", nargs="+", action="append", default=[],
                        help="PATH followed by solved, census SEED, or record PATH")
    parser.add_argument("--then", help="JSON list of [grip,item] letters, or a JSON list file")
    parser.add_argument("--start-digest")
    parser.add_argument("--every", type=int, default=1)
    parser.add_argument("--widths", action="store_true")
    args = parser.parse_args(argv)
    out = output_directory(args.out)
    if args.every < 1:
        raise ValueError("--every must be positive")
    selected = args.fixture or args.journals or args.h6030 or args.correction35 or args.word
    from differential.leads import x3_previews as x3

    def inputs():
        for name in (args.fixture if selected else ("I_a", "S4", "I_b")):
            yield fixture(name)
        if args.journals or not selected:
            for name in ("E2", "E3", "random_0", "random_1", "negative_control"):
                yield journal(name)
        if args.h6030 or not selected:
            yield h6030()
        if args.correction35 or not selected:
            yield correction35()
        words = args.word if selected else [[str(x3.U1_PATH), "census", "7101"]]
        for spec in words:
            if len(spec) < 2:
                raise ValueError("--word needs PATH and a start: solved, census SEED, or record PATH")
            state = x3.start_state(spec[1:], then=x3.then_letters(args.then),
                                   start_digest=args.start_digest)
            doc = x3.load_word(spec[0])
            x3.check_input_digest(state, doc)
            stages = {"end": {"records": len(doc["word"]), "digest": doc["output_digest"]}} if "output_digest" in doc else {}
            sources = [Path(spec[0])]
            if spec[1] == "record":
                sources.append(Path(spec[2]))
            elif spec[1] == "census":
                sources.append(census.load(int(spec[2]))[0])
            yield ReplayInput(state, Path(spec[0]).stem, letters=doc["word"], stages=stages,
                              sources=sources)

    results = []
    with (out / "heights-steps.jsonl").open("w", encoding="utf-8") as stream:
        for source in inputs():
            result = run_heights(source.state, source, label=source.label,
                                 every=args.every, widths=args.widths)
            for row in result.pop("rows"):
                stream.write(json.dumps(row) + "\n")
            # Unsampled widths remain available without widening the height sampling.
            result["sources"] = [input_evidence(p) for p in source.sources]
            results.append(result)
    command = ["python", "-B", str(Path(__file__).resolve().relative_to(ROOT)),
               *(sys.argv[1:] if argv is None else argv)]
    report = dict(environment(command=command), inputs=results, every=args.every, widths=args.widths)
    (out / "heights-summary.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return report


if __name__ == "__main__":
    main()
