"""Replay the oracle cases through the Rust kernel (LP2, M4; CT-KN2-01, the 600-cell-Full half).

The check builds the oracle adapter of `rust/c600-kernel` with `cargo build --release --locked --offline` in rust/,
checks the seven frozen case files O01 to O07 against MANIFEST-oracle.json and the reference they were generated from,
lets the adapter write one trace per case into a temporary folder, and requires `tools/oracle/compare.py` to report
"equal" with no omitted fields for every case (3,201 steps). Two controls follow: a trace with one tampered state hash
must fail at that step (exit 1), and a case that names another model identity must be refused by the adapter (exit 2)
without a trace.

The differential checks of the M4 leads follow (work/reviews/leads/2026-11-16-m4-differential-tests.md, leads only
until a check confirms them): L3, the frozen cases still match `core.py`, `session.py` and `make_cases.py`; question
4, the kernel's refusal texts are 0.4's; L2, a trace does not depend on which cases ran before it in the process; L4
and L1, type, shape, limit and JSON confusions beyond O07 are `invalid-input` steps with O07's unchanged state
(`tests/test_oracle.py` checks that 0.4 refuses each of them); L5 to L7, the language's new reference cases N01 to
N04 of `tools/oracle/make_new_cases.py` (PHS.18; tests/fixtures/oracle-new) replay equal; and CT-KN2-05, no float on
a decision path of the kernel's sources.

Last, the adapter's `agree` command runs the KN2.9 agreement test: the star cache's net maps against per-primitive
replay for the first, middle and last frame node of every orbit and both signs (210 stars), and the word [1, -1]. It
runs here, in the release binary, because replaying its 385,836 primitives one by one takes too long in the
unoptimized profile of `cargo test`, which runs a short version of the same comparison on one orbit.

Standard library only: the tracked case files are the inputs, so no step needs NumPy. A missing Rust toolchain fails
the check: a test that did not run is no pass (PHS.63). A pass is headless source and fixture evidence, not
Windows/DirectX, input, long-session or performance evidence; the timings it prints are informational.
"""
from __future__ import annotations

import gzip
import hashlib
import json
import re
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tools.oracle import invalid_recipes  # noqa: E402  (standard library only)

WORKSPACE = ROOT / "rust"
KERNEL = WORKSPACE / "c600-kernel"
ASSETS = ROOT / "assets"
INPUTS = ROOT / "work" / "experiments" / "language-lx" / "inputs"
NEW_INPUTS = ROOT / "tests" / "fixtures" / "oracle-new"
NEW_MANIFEST = "MANIFEST-oracle-new.json"
COMPARE = ROOT / "tools" / "oracle" / "compare.py"
GENERATOR = "tools/oracle/make_cases.py"
NEW_GENERATOR = "tools/oracle/make_new_cases.py"
ADAPTER = "c600-oracle-adapter"
CANDIDATE = "c600-kernel-full-candidate"
STEPS = {"O01": 2400, "O02": 500, "O03": 210, "O04": 25, "O05": 50, "O06": 6, "O07": 10}
TOTAL_STEPS = 3201
AGREEMENT_STARS = 210
TAMPERED_CASE, TAMPERED_INDEX = "O07", 8
# O01 and O02 hold words only, which never touch the star cache: the first run replays them on a cold cache, the
# reversed run after every star case. The cases with stars also run alone.
ALONE = ("O03", "O04", "O05", "O06", "O07")
# The refusal texts of the kernel and the 0.4 source whose ValueError carries each (question 4 of the M4 leads).
REFUSAL_TEXTS = {
    "engine.rs": ("core.py", ("STEP_COUNT", "BAD_STEP", "WORD_LENGTH", "BAD_PRIMITIVE", "BAD_STAR", "BAD_NODE",
                              "BAD_KIND", "CEILING")),
    "lib.rs": ("session.py", ("PROTECTED_MESSAGE",)),
}
CERTIFICATE_TEXTS = 7
# CT-KN2-05: the sources the float scan reads, and the one float they hold, the adapter's timing summary, which is
# informational output that no decision reads.
SCANNED = (KERNEL / "src", WORKSPACE / "c600-digest" / "src")
FLOAT_SITES = {("c600-kernel/src/bin/c600-oracle-adapter.rs",
                'Value::Number(format!("{:.3}", start.elapsed().as_secs_f64()))')}
FLOAT_TYPE = re.compile(r"f(?:32|64)\b")
FLOAT_LITERAL = re.compile(r"(?<![\w.])\d[\d_]*(?:\.\d[\d_]*(?:[eE][+-]?\d[\d_]*)?|[eE][+-]?\d[\d_]*)(?!\w)")
RAW_STRING = re.compile(r'b?r(#*)"')
# Below the CI runner's limit per check (tools/ci/run_headless.py), so a slow step is reported here.
BUILD_LIMIT_SECONDS = 900
RUN_LIMIT_SECONDS = 300


class Failed(Exception):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise Failed(message)


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def version(tool: str) -> str:
    try:
        out = subprocess.run([tool, "--version"], capture_output=True, text=True, timeout=60)
    except (OSError, subprocess.TimeoutExpired) as error:
        return f"{Path(tool).name}: unavailable ({error})"
    return out.stdout.strip() or f"{Path(tool).name}: no version (exit {out.returncode})"


def run(command: list[str], limit: int = RUN_LIMIT_SECONDS) -> subprocess.CompletedProcess:
    try:
        return subprocess.run(command, capture_output=True, text=True, timeout=limit)
    except subprocess.TimeoutExpired:
        raise Failed(f"{Path(command[1]).name if len(command) > 1 else command[0]} ran over {limit} s") from None


def one_line(out: subprocess.CompletedProcess, what: str) -> dict:
    lines = out.stdout.strip().splitlines()
    require(len(lines) == 1, f"{what} printed {len(lines)} lines instead of one JSON line")
    try:
        return json.loads(lines[0])
    except ValueError:
        raise Failed(f"{what} printed no JSON") from None


def verify_inputs() -> list[Path]:
    """The seven frozen case files, checked against their manifest, and the reference they were generated from (L3):
    `core.py` and `session.py` must have the digests MANIFEST-oracle.json pins, and every case's generator block must
    name those digests and the digest of `make_cases.py`, so the kernel is never judged against an old reference."""
    manifest = json.loads((INPUTS / "MANIFEST-oracle.json").read_text(encoding="utf-8"))
    reference = {"core_sha256": sha256_file(ROOT / "core.py"), "session_sha256": sha256_file(ROOT / "session.py")}
    for key, digest in reference.items():
        require(manifest[key] == digest, f"{key[:-7]}.py differs from the digest MANIFEST-oracle.json pins")
    script = sha256_file(ROOT / GENERATOR)
    paths = []
    for name in STEPS:
        path = INPUTS / "oracle" / f"{name}.json.gz"
        entry = manifest["files"].get(path.name)
        require(entry is not None, f"{path.name} is not in MANIFEST-oracle.json")
        data = path.read_bytes()
        require(len(data) == entry["bytes"] and hashlib.sha256(data).hexdigest() == entry["sha256"],
                f"{path.name} differs from MANIFEST-oracle.json")
        generator = json.loads(gzip.decompress(data))["generator"]
        require(generator["script"] == GENERATOR and generator["script_sha256"] == script
                and all(generator[key] == digest for key, digest in reference.items()),
                f"{path.name} names another make_cases.py, core.py or session.py than the checked-in ones")
        paths.append(path)
    require(len(manifest["files"]) == len(STEPS), "MANIFEST-oracle.json lists other case files")
    print(f"inputs: {len(paths)} case files match MANIFEST-oracle.json; core.py, session.py and make_cases.py have "
          "the digests the manifest and every case's generator block name")
    return paths


def verify_new_inputs() -> list[Path]:
    """The language's new cases N01 to N04 (PHS.18; leads L5 to L7), checked against MANIFEST-oracle-new.json and the
    reference: `core.py`, `session.py` and `make_new_cases.py` must have the digests the manifest and every case's
    generator block name. Each case must also hold what its lead asks for, so a regenerated case cannot silently stop
    testing it."""
    manifest = json.loads((NEW_INPUTS / NEW_MANIFEST).read_text(encoding="utf-8"))
    reference = {"core_sha256": sha256_file(ROOT / "core.py"), "session_sha256": sha256_file(ROOT / "session.py"),
                 "script_sha256": sha256_file(ROOT / NEW_GENERATOR)}
    require(manifest["script"] == NEW_GENERATOR and all(manifest[key] == value for key, value in reference.items()),
            f"core.py, session.py or make_new_cases.py differs from the digests {NEW_MANIFEST} pins")
    require(sorted(manifest["files"]) == [f"{name}.json.gz" for name in NEW_STEPS],
            f"{NEW_MANIFEST} lists other case files")
    paths = []
    for name, steps in NEW_STEPS.items():
        path = NEW_INPUTS / f"{name}.json.gz"
        entry = manifest["files"][path.name]
        data = path.read_bytes()
        require(len(data) == entry["bytes"] and hashlib.sha256(data).hexdigest() == entry["sha256"],
                f"{path.name} differs from {NEW_MANIFEST}")
        case = json.loads(gzip.decompress(data))
        generator = case["generator"]
        require(generator["script"] == NEW_GENERATOR
                and all(generator[key] == value for key, value in reference.items()),
                f"{path.name} names another make_new_cases.py, core.py or session.py than the checked-in ones")
        require(case["case_id"] == name and len(case["steps"]) == steps, f"{path.name} does not hold {steps} steps")
        NEW_CASE_CHECKS[name]([step["expect"] for step in case["steps"]], case)
        paths.append(path)
    print(f"inputs: {len(paths)} new case files match {NEW_MANIFEST} and hold what leads L5 to L7 ask for")
    return paths


def l5_planner_shapes(expects: list[dict], case: dict) -> None:
    """N01: every recipe is star-only, and the fifth of each orbit, a star and its own inverse, has an empty net,
    keeps the state and expands to twice the star's letters."""
    empty = hashlib.sha256(b"").hexdigest()
    for index, step in enumerate(case["steps"]):
        require(all(part["kind"] == "star" for part in step["recipe"]), "N01 holds a word")
        if index % 5 == 4:
            expect = expects[index]
            require(expect["accepted"] and expect["net_moved_stickers"] == 0 and expect["net_sha256"] == empty
                    and expect["support"] == [] and expect["state_hash"] == expects[index - 1]["state_hash"]
                    and expect["primitive_count"] % 2 == 0 and expect["primitive_count"] > 0,
                    f"N01 step {index}, a star and its inverse, does not have an empty net")
    require(any(len(step["recipe"]) == 8 for step in case["steps"]), "N01 holds no eight-star commutator")


def l6_node_tour(expects: list[dict], case: dict) -> None:
    """N02: every step is one star and is accepted; the last 35 repeat a star of each orbit seen before."""
    steps = case["steps"]
    require(all(len(step["recipe"]) == 1 and step["recipe"][0]["kind"] == "star" for step in steps)
            and all(expect["accepted"] for expect in expects), "N02 holds a step that is not one accepted star")
    again = [step["recipe"] for step in steps[-35:]]
    require([recipe[0]["orbit"] for recipe in again] == list(range(35))
            and all(recipe in [step["recipe"] for step in steps[:-35]] for recipe in again),
            "N02 does not end with a star of every orbit seen before")


def l7_several_protected(expects: list[dict], case: dict) -> None:
    """N03: the star whose whole support is protected is refused with every support row in conflict, a star that
    moves some protected orbits is refused with just those rows, an allowed star is accepted and a generator, which
    moves every orbit, is refused."""
    protected = set(case["protected"])
    require(len(protected) >= 3, "N03 protects fewer than three orbits")
    require([expect["error"] for expect in expects] == ["protected", "protected", None, "protected"],
            "N03's steps are not refused, refused, accepted, refused")
    require(expects[0]["conflicts"] == expects[0]["support"], "N03's first star does not conflict on all its support")
    rows = [row for row in expects[1]["support"] if row["orbit"] in protected]
    require(expects[1]["conflicts"] == rows and rows != expects[1]["support"],
            "N03's second star does not conflict on just its protected rows")


def l7_all_protected(expects: list[dict], case: dict) -> None:
    """N04: with every orbit protected, a star and its inverse (an empty net) are accepted, a generator and a star
    conjugated by it are refused."""
    require(case["protected"] == list(range(35)), "N04 does not protect every orbit")
    require([expect["error"] for expect in expects] == [None, "protected", "protected"]
            and expects[0]["net_moved_stickers"] == 0, "N04's steps are not accepted, refused, refused")


# The step counts of N01 to N04 and the check of what each case holds.
NEW_STEPS = {"N01": 175, "N02": 1505, "N03": 4, "N04": 3}
NEW_CASE_CHECKS = {"N01": l5_planner_shapes, "N02": l6_node_tour, "N03": l7_several_protected,
                   "N04": l7_all_protected}


def rust_text(source: str, name: str) -> str:
    match = re.search(rf'\bconst {name}: &str =\s*"([^"\\]*)";', source)
    require(match is not None, f"the kernel has no plain text constant {name}")
    return match.group(1)


def check_refusal_texts() -> None:
    """Question 4 of the M4 leads: every refusal text of the kernel and every star certificate text is the text of
    0.4's ValueError, read from both sources. A trace compares only the error class, so a changed text shows here."""
    count = 0
    for file, (python, names) in REFUSAL_TEXTS.items():
        source = (KERNEL / "src" / file).read_text(encoding="utf-8")
        reference = (ROOT / python).read_text(encoding="utf-8")
        if file == "engine.rs":
            require(set(re.findall(r"\bconst (\w+): &str", source)) == set(names),
                    "engine.rs has a text constant this check does not know")
        for name in names:
            require(f"raise ValueError('{rust_text(source, name)}')" in reference,
                    f"the kernel's {name} is not the text of a ValueError in {python}")
            count += 1
    engine = (KERNEL / "src" / "engine.rs").read_text(encoding="utf-8")
    core = (ROOT / "core.py").read_text(encoding="utf-8")
    certificates = re.findall(r'\bcertificate\("([^"\\]*)"\)', engine)
    require(len(certificates) == CERTIFICATE_TEXTS,
            f"engine.rs has {len(certificates)} star certificate texts instead of {CERTIFICATE_TEXTS}")
    for text in certificates:
        require(f"raise ValueError('{text}')" in core, f"the certificate text {text!r} is not one of core.py")
    print(f"PASS: the kernel's {count} refusal texts and {len(certificates)} star certificate texts are 0.4's")


def blank_comments_and_literals(text: str) -> str:
    """Rust source with comments and string, byte-string and character literals replaced by spaces, lines kept."""
    out = list(text)
    i, n = 0, len(text)

    def blank(start: int, end: int) -> None:
        for k in range(start, end):
            if out[k] != "\n":
                out[k] = " "

    while i < n:
        raw = RAW_STRING.match(text, i)
        if text.startswith("//", i):
            end = text.find("\n", i)
            end = n if end < 0 else end
        elif text.startswith("/*", i):
            depth, end = 1, i + 2
            while end < n and depth:
                step = text[end:end + 2]
                depth += 1 if step == "/*" else -1 if step == "*/" else 0
                end += 2 if step in ("/*", "*/") else 1
        elif raw and (i == 0 or not (text[i - 1].isalnum() or text[i - 1] == "_")):
            close = '"' + raw.group(1)
            end = text.find(close, raw.end())
            require(end >= 0, "unterminated raw string in a kernel source")
            end += len(close)
        elif text[i] == '"':
            end = i + 1
            while end < n and text[end] != '"':
                end += 2 if text[end] == "\\" else 1
            end += 1
        elif text[i] == "'" and i + 1 < n and text[i + 1] == "\\":
            end = text.find("'", i + 3) + 1
        elif text[i] == "'" and i + 2 < n and text[i + 2] == "'":
            end = i + 3
        else:
            i += 1
            continue
        blank(i, end)
        i = end
    return "".join(out)


def check_ct_kn2_05_no_float_on_a_decision_path() -> None:
    """CT-KN2-05 (part: the 600-cell-Full kernel's sources, c600-kernel and the c600-digest crate it hashes with; the
    jumbling half's sources are M5's and are not scanned): a float enters Rust code only through a float type name or
    a float literal, so outside comments and literals the sources hold none, except the oracle adapter's timing
    summary, which no decision reads. INV-4 names no kernel site where a float may stand."""
    hits = set()
    files = 0
    for folder in SCANNED:
        for path in sorted(folder.rglob("*.rs")):
            files += 1
            original = path.read_text(encoding="utf-8").splitlines()
            code = blank_comments_and_literals(path.read_text(encoding="utf-8")).splitlines()
            name = path.relative_to(WORKSPACE).as_posix()
            for number, line in enumerate(code):
                if FLOAT_TYPE.search(line) or FLOAT_LITERAL.search(line):
                    hits.add((name, original[number].strip()))
    unexpected = sorted(hits - FLOAT_SITES)
    require(not unexpected, "a float outside the known site: " + "; ".join(f"{p}: {line}" for p, line in unexpected))
    require(hits == FLOAT_SITES, "the adapter's timing summary is no longer where the scan expects it")
    print(f"PASS: CT-KN2-05 scan of {files} kernel source files: no float type or literal outside the adapter's "
          "timing summary")


def build(cargo: str) -> Path:
    """Builds the adapter in release and returns the executable cargo reports."""
    command = [cargo, "build", "--release", "--locked", "--offline", "-p", "c600-kernel", "--bin", ADAPTER,
               "--message-format=json-render-diagnostics"]
    start = time.monotonic()
    try:
        out = subprocess.run(command, cwd=WORKSPACE, stdout=subprocess.PIPE, text=True,
                             timeout=BUILD_LIMIT_SECONDS)
    except subprocess.TimeoutExpired:
        raise Failed(f"cargo build ran over {BUILD_LIMIT_SECONDS} s") from None
    require(out.returncode == 0, f"cargo build --release --locked --offline exited with {out.returncode}")
    executable = None
    for line in out.stdout.splitlines():
        try:
            message = json.loads(line)
        except ValueError:
            continue
        if (message.get("reason") == "compiler-artifact" and message.get("target", {}).get("name") == ADAPTER
                and message.get("executable")):
            executable = Path(message["executable"])
    require(executable is not None and executable.is_file(), "cargo reported no adapter executable")
    print(f"build: cargo build --release --locked --offline -p c600-kernel --bin {ADAPTER} "
          f"({time.monotonic() - start:.1f} s)")
    return executable


def compare(case: Path, trace: Path) -> tuple[int, dict, str]:
    out = run([sys.executable, str(COMPARE), str(case), str(trace)])
    line = out.stdout.strip()
    return out.returncode, one_line(out, "compare.py"), line


def trace_command(adapter: Path, pairs: list[tuple[Path, Path]]) -> list[str]:
    command = [str(adapter), "trace", "--assets", str(ASSETS)]
    for case, trace in pairs:
        command += [str(case), str(trace)]
    return command


def check_traces(adapter: Path, cases: list[Path], folder: Path) -> tuple[dict[str, Path], dict]:
    """CT-KN2-01 (part: the 600-cell-Full half, O01 to O07 here and N01 to N04 in `check_new_cases`; the J1 harness
    half is M5's) and CT-KN2-04 (part: the blocked and invalid steps of 600-cell-Full keep the state and progress
    digests; uncertain words exist only in the jumbling half, M5)."""
    traces = {case.name[:3]: folder / f"{case.name[:3]}.trace.json" for case in cases}
    start = time.monotonic()
    out = run(trace_command(adapter, [(case, traces[case.name[:3]]) for case in cases]))
    wall = time.monotonic() - start
    require(out.returncode == 0, f"the adapter's trace command exited with {out.returncode}: {out.stderr.strip()}")
    summary = one_line(out, "the adapter's trace command")
    require(summary.get("candidate") == CANDIDATE, "the trace summary names another candidate")
    build_string = summary.get("build", "")
    print(f"adapter build: {build_string}")
    print(f"trace: model load {summary['load_seconds']} s, all seven cases {summary['total_seconds']} s "
          f"in the adapter, {wall:.1f} s wall; per case: "
          + ", ".join(f"{c['case_id']} {c['steps']} steps {c['seconds']} s" for c in summary["cases"]))
    steps = 0
    for case in cases:
        name = case.name[:3]
        trace = json.loads(traces[name].read_text(encoding="utf-8"))
        require(trace["candidate"] == {"name": CANDIDATE, "build": build_string}, f"{name}: trace candidate differs")
        code, report, line = compare(case, traces[name])
        print(f"compare {name}: exit {code} {line}")
        require(code == 0, f"compare.py exited with {code} for {name}")
        require(report.get("status") == "equal" and report.get("omitted_fields") == []
                and report.get("case_id") == name and report.get("steps") == STEPS[name],
                f"compare.py did not report {name} equal with every field")
        steps += report["steps"]
    require(steps == TOTAL_STEPS, f"{steps} steps compared instead of {TOTAL_STEPS}")
    print(f"PASS: all seven cases equal, {steps} steps, no omitted fields")
    return traces, summary


def check_tampered_trace(cases: list[Path], traces: dict[str, Path], folder: Path) -> None:
    trace = json.loads(traces[TAMPERED_CASE].read_text(encoding="utf-8"))
    step = next(step for step in trace["steps"] if step["index"] == TAMPERED_INDEX)
    original = step["state_hash"]
    step["state_hash"] = ("1" if original[0] == "0" else "0") + original[1:]
    tampered = folder / "tampered.trace.json"
    tampered.write_text(json.dumps(trace, sort_keys=True, separators=(",", ":")) + "\n", encoding="utf-8")
    case = next(case for case in cases if case.name.startswith(TAMPERED_CASE))
    code, report, line = compare(case, tampered)
    print(f"negative control ({TAMPERED_CASE} step {TAMPERED_INDEX} state_hash changed): exit {code} {line}")
    require(code == 1 and report.get("status") == "mismatch" and report.get("index") == TAMPERED_INDEX
            and [d["field"] for d in report.get("differences", [])] == ["state_hash"],
            "compare.py did not report the tampered state hash at its step")
    print(f"PASS: the tampered trace fails at index {TAMPERED_INDEX}, field state_hash")


def check_identity_refusal(adapter: Path, cases: list[Path], folder: Path) -> None:
    """CT-KN1-01 (part: a case of another 600-cell-Full model identity is refused through the kernel's identity
    check; values of another model offered to the kernel API are in `tests/load.rs`; jumbling identities are M5's)."""
    source = next(case for case in cases if case.name.startswith(TAMPERED_CASE))
    original = json.loads(gzip.decompress(source.read_bytes()))
    for key in ("model_id", "manifest_sha256"):
        case = json.loads(json.dumps(original))
        case["model"][key] = "0" * 64
        path = folder / f"other-{key}.case.json"
        path.write_text(json.dumps(case), encoding="utf-8")
        trace = folder / f"other-{key}.trace.json"
        out = run(trace_command(adapter, [(path, trace)]))
        print(f"identity control (case model.{key} changed): exit {out.returncode} {out.stderr.strip()}")
        require(out.returncode == 2 and not trace.exists() and not out.stdout.strip(),
                f"the adapter did not refuse a case with another model.{key}")
    print("PASS: cases with another model identity are refused without a trace")


def check_order_independence(adapter: Path, cases: list[Path], traces: dict[str, Path], folder: Path) -> float:
    """Lead L2 (KN2.9: a cached result equals the result with equal inputs): the star cache, its certified seeds and
    its scratch image live as long as the adapter's process, so a trace must not depend on the cases before it. The
    cases with stars run alone on a cold cache, all seven run in reverse order, and O04 runs twice in one process,
    the second time on the cache the first left. Every trace must equal the first run's trace byte for byte."""
    by_name = {case.name[:3]: case for case in cases}
    runs = [[name] for name in ALONE] + [list(reversed(STEPS))] + [["O04", "O04"]]
    start = time.monotonic()
    count = 0
    for number, names in enumerate(runs):
        outputs = [folder / f"order-{number}-{position}-{name}.trace.json" for position, name in enumerate(names)]
        out = run(trace_command(adapter, [(by_name[name], output) for name, output in zip(names, outputs)]))
        require(out.returncode == 0, f"the adapter exited with {out.returncode} on {' '.join(names)}: "
                                     f"{out.stderr.strip()}")
        for position, (name, output) in enumerate(zip(names, outputs)):
            require(output.read_bytes() == traces[name].read_bytes(),
                    f"the trace of {name} at position {position} of the run {' '.join(names)} differs from the "
                    "first run's")
            count += 1
    seconds = time.monotonic() - start
    print(f"PASS: L2, {count} traces from {len(runs)} runs (O03 to O07 alone, all seven reversed, O04 twice in one "
          f"process) equal the first run's byte for byte ({seconds:.1f} s)")
    return seconds


def extended_o07(cases: list[Path], case_id: str, recipes: list[tuple[str, str]], folder: Path) -> tuple[Path, int]:
    """A copy of O07 with each recipe text appended as a step. Every O07 step starts from the same initial state and
    is refused, so each new step's expectation is a copy of step 0's."""
    source = next(case for case in cases if case.name.startswith("O07"))
    case = json.loads(gzip.decompress(source.read_bytes()))
    expect = case["steps"][0]["expect"]
    require(case["mode"] == "independent" and expect["error"] == "invalid-input"
            and all(step["expect"] == expect for step in case["steps"]), "O07's steps no longer share one expectation")
    case["case_id"] = case_id
    case["description"] = f"O07 and {len(recipes)} more inputs that 0.4's Model.normalize refuses"
    marks = {}
    for offset, (_, recipe) in enumerate(recipes):
        mark = f"@recipe-{offset}@"
        marks[json.dumps(mark)] = recipe
        case["steps"].append({"index": len(case["steps"]), "recipe": mark, "expect": expect})
    text = json.dumps(case, sort_keys=True, separators=(",", ":"))
    for mark, recipe in marks.items():
        require(text.count(mark) == 1, "a recipe mark is not unique")
        text = text.replace(mark, recipe)
    path = folder / f"{case_id}.case.json"
    path.write_text(text, encoding="utf-8")
    return path, len(case["steps"])


def check_invalid_steps(adapter: Path, cases: list[Path], folder: Path, lead: str,
                        recipes: list[tuple[str, str]]) -> None:
    """Leads L4 (type, shape and limit confusions) and L1 (JSON that Python reads and a strict reader cannot
    represent), and CT-KN2-04 (part: as in `check_traces`): each recipe of `tools/oracle/invalid_recipes.py` is a
    refused `invalid-input` step that keeps O07's state, progress and null effects. `tests/test_oracle.py` checks in
    the engine environment that 0.4's Model.normalize raises ValueError for each, so the expectation is the
    reference's."""
    path, steps = extended_o07(cases, f"O07-{lead}", recipes, folder)
    trace = folder / f"O07-{lead}.trace.json"
    out = run(trace_command(adapter, [(path, trace)]))
    require(out.returncode == 0, f"{lead}: the adapter refused the whole case (exit {out.returncode}: "
                                 f"{out.stderr.strip()}) instead of {len(recipes)} invalid-input steps")
    code, report, line = compare(path, trace)
    print(f"compare O07-{lead}: exit {code} {line[:300]}")
    require(code == 0 and report.get("status") == "equal" and report.get("steps") == steps
            and report.get("omitted_fields") == [], f"{lead}: compare.py did not report every step equal")
    print(f"PASS: {lead}, {len(recipes)} inputs beyond O07 are invalid-input steps with O07's unchanged state: "
          + ", ".join(name for name, _ in recipes[:6]) + (", ..." if len(recipes) > 6 else ""))


def check_new_cases(adapter: Path, cases: list[Path], folder: Path) -> float:
    """Leads L5 to L7 and CT-KN2-01 (part: as in `check_traces`): the language's new cases N01 to N04 (PHS.18) replay
    equal in one adapter process: star-only recipes in the planner's shapes, star nodes beyond first, middle and last
    with re-entry after eviction, and protection with several and with all orbits protected."""
    traces = {case.name[:3]: folder / f"{case.name[:3]}.trace.json" for case in cases}
    start = time.monotonic()
    out = run(trace_command(adapter, [(case, traces[case.name[:3]]) for case in cases]))
    seconds = time.monotonic() - start
    require(out.returncode == 0, f"the adapter's trace command exited with {out.returncode} on N01 to N04: "
                                 f"{out.stderr.strip()}")
    summary = one_line(out, "the adapter's trace command")
    print(f"trace N01 to N04: {seconds:.1f} s wall; per case: "
          + ", ".join(f"{c['case_id']} {c['steps']} steps {c['seconds']} s" for c in summary["cases"]))
    for case in cases:
        name = case.name[:3]
        code, report, line = compare(case, traces[name])
        print(f"compare {name}: exit {code} {line}")
        require(code == 0 and report.get("status") == "equal" and report.get("omitted_fields") == []
                and report.get("case_id") == name and report.get("steps") == NEW_STEPS[name],
                f"compare.py did not report {name} equal with every field")
    print(f"PASS: L5 to L7, the new cases N01 to N04 equal, {sum(NEW_STEPS.values())} steps, no omitted fields")
    return seconds


def check_agreement(adapter: Path) -> dict:
    out = run([str(adapter), "agree", "--assets", str(ASSETS)])
    require(out.returncode == 0, f"the adapter's agree command exited with {out.returncode}: {out.stderr.strip()}")
    summary = one_line(out, "the adapter's agree command")
    print(f"agree: {out.stdout.strip()}")
    require(summary.get("stars") == AGREEMENT_STARS, f"agree compared {summary.get('stars')} stars")
    print(f"PASS: KN2.9 agreement, {summary['stars']} stars and {summary['primitives']} primitives "
          f"in {summary['seconds']} s")
    return summary


def main() -> int:
    cargo = shutil.which("cargo")
    if cargo is None:
        print("FAIL: cargo is not on PATH, so the kernel oracle check did not run")
        return 1
    print(version(cargo))
    print(version(shutil.which("rustc") or "rustc"), flush=True)
    start = time.monotonic()
    try:
        cases = verify_inputs()
        new_cases = verify_new_inputs()
        check_refusal_texts()
        check_ct_kn2_05_no_float_on_a_decision_path()
        adapter = build(cargo)
        with tempfile.TemporaryDirectory(prefix="c600-kernel-oracle-") as name:
            folder = Path(name)
            traces, summary = check_traces(adapter, cases, folder)
            check_tampered_trace(cases, traces, folder)
            check_identity_refusal(adapter, cases, folder)
            order = check_order_independence(adapter, cases, traces, folder)
            check_invalid_steps(adapter, cases, folder, "L4", invalid_recipes.type_and_limit_family())
            check_invalid_steps(adapter, cases, folder, "L1", invalid_recipes.json_edges())
            new = check_new_cases(adapter, new_cases, folder)
        agreement = check_agreement(adapter)
    except (Failed, OSError, KeyError, ValueError) as error:
        print(f"FAIL: {error}")
        return 1
    print(f"timings (release): model load {summary['load_seconds']} s, oracle run {summary['total_seconds']} s, "
          f"order runs {order:.1f} s, new cases {new:.1f} s, agreement {agreement['seconds']} s; whole check "
          f"{time.monotonic() - start:.1f} s")
    print("PASS: the kernel replays O01 to O07 and N01 to N04 exactly through its API, and the star cache agrees "
          "with replay")
    return 0


if __name__ == "__main__":
    sys.exit(main())
