"""LX-9 acceptance: frozen inputs, Windows legs, exact controls, cleanup."""
import sys
sys.dont_write_bytecode = True

import argparse
import gzip
import itertools
import json
import os
from pathlib import Path
import shutil
import subprocess
import time

from common import (HERE, ROOT, differences, digest, json_bytes, read_requests,
                    read_results, rotation_key, unbits, verify_candidate, verify_manifest)
from make_requests import freeze_step2, verify_construction
from pure_input_map import choose

PRIMARY = ("reference", "pure", "rust")
BUILD = HERE / "build"
PREFIX = HERE.relative_to(ROOT).as_posix()
LINUX_COMMAND = f"wsl.exe --exec python3 -B {PREFIX}/linux_leg.py"
COMPARE_COMMAND = f"python -B {PREFIX}/compare_legs.py"


def file_hashes():
    return {p.relative_to(HERE).as_posix(): digest(p.read_bytes())
            for p in HERE.rglob("*") if not p.is_relative_to(BUILD) and p.is_file()}


def verify_unchanged(before):
    after = file_hashes()
    changed = sorted(name for name in before.keys() | after.keys()
                     if before.get(name) != after.get(name))
    assert not changed, "experiment files changed during acceptance: " + ", ".join(changed)


def cleanup():
    if BUILD.exists():
        assert not BUILD.is_symlink() and not BUILD.is_junction()
        assert BUILD.resolve() == HERE / "build" and BUILD.resolve().parent == HERE
        shutil.rmtree(BUILD)


def command(args, env, timeout=240):
    result = subprocess.run(list(map(str, args)), cwd=ROOT, env=env, check=True,
                            capture_output=True, text=True, timeout=timeout)
    if result.stdout.strip():
        print(result.stdout.strip(), flush=True)
    return result.stdout.strip()


def engine_python():
    choices = [ROOT / "tools/.venv/engine/Scripts/python.exe",
               ROOT / "../../../../../../../../tools/.venv/engine/Scripts/python.exe"]
    return next(p.resolve() for p in choices if p.is_file())


def candidate_files():
    local = sorted([*HERE.glob("*.py"), *HERE.glob("*.rs"), HERE / "CARD.md"])
    reference = [ROOT / "research/jumbling" / name for name in
                 ("exact.py", "sim/__init__.py", "sim/twists.py", "sim/kplus.py", "sim/kernel.py")]
    return {p.relative_to(ROOT).as_posix(): digest(p.read_bytes()) for p in local + reference}


def validate_results(rows, requests, validate_scores=False):
    assert set(rows) == {r[0] for r in requests}
    for name, axis, angle, n, d in requests:
        target, choice = rows[name]
        den, p, family = choice
        assert 1 <= den <= d and all(abs(x) <= n for x in p)
        if family == "half_turn":
            assert den == 1 and any(p) and abs(abs(angle % 360) - 180) <= 1
        rotation_key(choice)
        if validate_scores:
            assert choice == choose(list(map(unbits, target)), angle, n, d), name + " Rust exact score differs"


def run_step(step, engine, exe, env):
    print(f"Running step {step} on the frozen requests", flush=True)
    filename = f"step{step}.tsv"
    requests = read_requests(HERE / filename)
    raw, rows, metadata = {}, {}, {}
    for leg in (*PRIMARY, "rust-fma"):
        output = BUILD / f"{leg}-{step}.tsv"
        if leg in ("reference", "pure"):
            interpreter = engine if leg == "reference" else Path(sys.executable)
            command([interpreter, "-B", HERE / "run_leg.py", leg, filename,
                     f"build/{output.name}"], env, timeout=300)
            metadata[leg] = json.loads(output.with_suffix(".json").read_bytes())
        else:
            command([exe, HERE / filename, output, "fma" if leg == "rust-fma" else "plain"], env)
        raw[leg] = output.read_bytes()
        rows[leg] = read_results(raw[leg])
        validate_results(rows[leg], requests, validate_scores=leg.startswith("rust"))
    comparisons = {}
    for left, right in itertools.combinations(PRIMARY, 2):
        diff = differences(rows[left], rows[right])
        # Equal target bits must use the same exact tie rule, regardless of runtime.
        assert set(diff["parameters"]) <= set(diff["target"]), (left, right, "chooser port error")
        comparisons[f"{left}/{right}"] = diff
    metadata["rust_fma_vs_reference"] = {
        k: len(v) for k, v in differences(rows["reference"], rows["rust-fma"]).items()}
    metadata["rust_score_controls"] = {"requests_each_mode": len(requests), "mismatches": 0}
    return {"raw": raw, "rows": rows, "metadata": metadata, "comparisons": comparisons}


def inclusion_ids(step):
    return sorted({k for diff in step["comparisons"].values()
                   for kind in ("target", "rotation") for k in diff[kind]})


def write_amendment(steps, output_dir):
    examples = []
    for number, step in steps.items():
        requests = {r[0]: r for r in read_requests(HERE / f"step{number}.tsv")}
        for pair, diff in step["comparisons"].items():
            for name in diff["rotation"][:3]:
                left, right = pair.split("/")
                req = requests[name]
                examples.append(f"| {number} | {name} | {pair} | `{rotation_key(step['rows'][left][name][1])}` "
                                f"| `{rotation_key(step['rows'][right][name][1])}` |")
    if not examples:
        return
    text = """# LX-9 amendment evidence — unaccepted

Decision: **open**; decider: **owner**, after EN1's Astra check. This file prepares
evidence under KN3.4 and RTE.66; it changes no contract or product behavior.

The same frozen float requests selected different exact rotations in the Windows
legs. An unqualified fresh-request cross-runtime agreement claim is therefore
stopped. This does not contradict replay of an already recorded exact rotation.
`WINDOWS.json`, `differences.tsv`, the frozen requests and compressed result
tables retain all differences; examples below use exact projective quaternion
keys in the common cap frame. The manifests bind the sources and bytes.

| Step | Request | Pair | Left exact key | Right exact key |
| --- | --- | --- | --- | --- |
""" + "\n".join(examples) + """

Proposed amendment path, not an accepted design:

1. Record the fresh request as a dyadic Cayley vector and read its coordinates
   exactly, removing platform trigonometry and normalization from this decision.
2. Compare a menu-mode drag's 60 candidates exactly in Q(sqrt(5)), retaining the
   pinned index tie rule and an explicit metric and menu-order identity.
3. Take the evidence and a versioned contract proposal to EN1's Astra check,
   then to the owner. Only an owner-approved amendment could change A1.

Open: the dyadic representation, half-turn representation, request/UI boundary,
fixed drag metric, 60-candidate implementation, compatibility and migration
proof, Linux measurements, independent review and owner decision. This packet
implements none of these. The Rust FMA control in RESULT.md isolates whether
ordinary norm accumulation explains the observed Windows differences; differences
must not be attributed to sin/cos without that control.
"""
    (output_dir / "AMENDMENT.md").write_bytes(text.encode())


def publish_results(steps, manifest, versions, elapsed, output_dir):
    summary = {"schema": "lx9-windows-v1", "evidence_class": "synthetic",
               "decision": "open", "decider": "owner", "acceptance": "passed",
               "input_manifest_sha256": digest((HERE / "MANIFEST.json").read_bytes()),
               "candidate_manifest_sha256": digest((HERE / "CANDIDATE.json").read_bytes()),
               "versions": versions, "steps": {}, "archives": {},
               "elapsed_seconds_lead_only": round(elapsed, 3)}
    table = ["step\tid\tpair\ttarget_bits_differ\trotation_differs\tparameters_differ\n"]
    for number, step in steps.items():
        summary["steps"][str(number)] = {
            "requests": len(step["rows"]["reference"]), "controls": step["metadata"],
            "raw_result_sha256": {k: digest(v) for k, v in step["raw"].items()},
            "comparisons": {pair: {k: len(v) for k, v in diff.items()}
                            for pair, diff in step["comparisons"].items()}}
        for leg in PRIMARY:
            data = step["raw"][leg]
            assert len(data) < 2_000_000, "raw result exceeds retention limit"
            name = f"windows-{leg}-step{number}.tsv.gz"
            packed = gzip.compress(data, mtime=0)
            (output_dir / name).write_bytes(packed)
            summary["archives"][name] = {"sha256": digest(packed), "bytes": len(packed),
                                         "raw_sha256": digest(data), "raw_bytes": len(data)}
        for pair, diff in step["comparisons"].items():
            sets = {k: set(v) for k, v in diff.items()}
            for name in sorted(set.union(*sets.values())):
                table.append(f"{number}\t{name}\t{pair}\t{int(name in sets['target'])}\t"
                             f"{int(name in sets['rotation'])}\t{int(name in sets['parameters'])}\n")
    (output_dir / "differences.tsv").write_bytes("".join(table).encode())
    summary["difference_table_sha256"] = digest((output_dir / "differences.tsv").read_bytes())
    (output_dir / "WINDOWS.json").write_bytes(json_bytes(summary))
    write_amendment(steps, output_dir)
    report(summary, manifest, output_dir)


def verify_record():
    fresh = json.loads((BUILD / "WINDOWS.json").read_bytes())
    stored = json.loads((HERE / "WINDOWS.json").read_bytes())
    for record in (fresh, stored):
        del record["elapsed_seconds_lead_only"]
    assert json_bytes(fresh) == json_bytes(stored), "WINDOWS.json differs beyond elapsed_seconds_lead_only"
    for name in (*fresh["archives"], "differences.tsv"):
        assert (BUILD / name).read_bytes() == (HERE / name).read_bytes(), name + " differs"


def report(summary, manifest, output_dir):
    rows = []
    rotation_count = 0
    for step, entry in summary["steps"].items():
        for pair, counts in entry["comparisons"].items():
            rows.append(f"| {step} | {entry['requests']} | {pair} | {counts['target']} | "
                        f"{counts['rotation']} | {counts['parameters']} |")
            rotation_count += counts["rotation"]
    controls = []
    for step, entry in summary["steps"].items():
        ref = entry["controls"]["reference"]["control"]
        rust = entry["controls"]["rust_fma_vs_reference"]
        controls.append(f"| {step} | {ref['pure_target_differences']} | {ref['pure_parameter_differences']} | "
                        f"{ref['naive_target_differences']} | {rust['target']} | {rust['rotation']} |")
    outcome = ("Different exact choices were observed. `AMENDMENT.md` prepares the required unaccepted evidence; "
               "fresh-request agreement cannot be claimed." if rotation_count else
               "No different exact choices were observed on these requests. This is a finite-sample result only; "
               "the amendment decision remains open.")
    text = f"""# LX-9 Windows result

{outcome}

| CT-RTE-05 field | Record |
| --- | --- |
| LX card / experiment ID | LX-9, RTE.64 to RTE.66 / E-2.4-L9 |
| Owner allowance | `docs/wiki/decisions/owner-decisions-2026-11-05.md`; assigned implementation packet |
| Time box | what can be done now; one wrapper call of at most two hours |
| Evidence class | synthetic; actual Windows CPython/NumPy and Rust execution, no native input or performance claim |
| Frozen candidate digest | `CANDIDATE.json`: `{summary['candidate_manifest_sha256']}` |
| Frozen input manifest digest | `MANIFEST.json`: `{summary['input_manifest_sha256']}` |
| Step 1 input digest | `{manifest['files']['step1.tsv']['sha256']}` |
| Step 2 input digest | `{manifest['files']['step2.tsv']['sha256']}` |
| Commands and tool versions | Below; all detailed raw-output digests and controls in `WINDOWS.json` |
| Kill criteria | Different exact rotation stops an unqualified cross-runtime claim and prepares amendment evidence; target-only differences feed step 2; timeout leaves work open |
| Actual result | All 13,600 step-1 requests and all {manifest['step2_count']} step-2 requests ran in all three primary Windows legs; acceptance passed |
| Decision with decider | **open**, owner; no route, language, syntax, LQ answer or contract change |

## Comparisons

| Step | Requests | Pair (left/right) | Target-bit differences | Exact rotation differences | Raw parameter differences |
| --- | --- | --- | --- | --- | --- |
{chr(10).join(rows)}

Step 2 contains 1,216 certified near-bisector requests plus
{len(manifest['step2_included_step1_ids'])} unchanged requests carried from step 1.
The carried requests are not counted as near-bisector samples. All pairwise
differences are retained in `differences.tsv`, the request sets and the small
compressed result tables. Counts across pairs/steps must not be added as unique
requests. Exact keys are primitive projective integer quaternions; raw parameter
comparisons additionally control the tie order.

## Port and norm controls

| Step | Same-interpreter pure target differences | Same-interpreter pure parameter differences | Unfused Python target differences | Rust FMA target differences vs reference | Rust FMA rotation differences vs reference |
| --- | --- | --- | --- | --- | --- |
{chr(10).join(controls)}

The bit-for-bit pure control runs inside the reference interpreter on every
request. NumPy's short dot products are reproduced with sequential FMA;
ordinary loop summation has the measured differences above. The primary Rust
candidate deliberately uses ordinary separate multiply/add; the extra Rust FMA
leg changes only the two norms. Its remaining differences, if any, are recorded
above and must not automatically be attributed to trigonometry. Both Rust modes'
exact parameter choices were independently re-scored with Python integers on
every emitted target, with zero mismatches. Equal target bits also required
identical raw parameter choices between all primary legs.

The reference uses the unchanged NumPy screening search with
`INPUT_SEARCH_MARGIN = 1e-12`; the ports exhaust their bounded candidate sets.
The frozen construction verifies the half-turn boundaries exactly in Q(sqrt(2))
and the angle bounds with Fraction arithmetic. Acceptance rechecks construction
and digests, includes every step-1 difference, and removes `build/` in `finally`.

## Reproduction

Run from the repository root. No network, installation or personal data is used.

```text
python {PREFIX}/check.py
```

Tool versions: Windows standalone CPython {summary['versions']['pure_python']};
reference CPython {summary['versions']['reference_python']}, NumPy {summary['versions']['numpy']};
{summary['versions']['rustc']}; target {summary['versions']['rust_host']}.
Rust command (the check supplies TMP and TEMP under its local build folder):

```text
rustc --edition=2024 -C opt-level=2 -C debuginfo=0 -C panic=abort {PREFIX}/rust_input_map.rs -o {PREFIX}/build/float-map.exe
```

Initial production was `python {PREFIX}/make_requests.py`, then
`python {PREFIX}/check.py --freeze-step2`. That one-time second command freezes
the dependent inclusion list and candidate sources. It refuses to overwrite a
frozen step 2. Normal acceptance verifies the frozen inputs and sources. Sources,
bit-encoded requests and compressed result tables are retained; each raw table
is under 2 MB, and its uncompressed digest is recorded. The extra Rust FMA raw
tables are regenerated by the check and removed; their digests are retained.
Elapsed acceptance time is a sandbox lead only, not a benchmark.

Run record: the first run's acceptance passed and was refused only because its check rewrote `elapsed_seconds_lead_only` in `WINDOWS.json`; this re-run preserves the same work with a check that writes nothing outside `build/` and removes it before exit; this run updated the source digest pin for `check.py` only.

## Not run here and open

Linux CPython 3.14.4 in WSL is **not run here**. From a Windows shell at the
repository root, the integrator must run exactly:

```text
{LINUX_COMMAND}
{COMPARE_COMMAND}
```

The Linux script requires only the Python standard library. It writes the same
result format plus a manifest, retains every new Linux step-1 difference by the
frozen inclusion rule, and runs those requests in step 2. The comparison uses
Windows step-1 exact choices for any newly included requests. It records counts
and all difference IDs and updates amendment evidence if Linux adds a rotation
difference. No cross-platform agreement is claimed before those commands run.

R2-E2 records no seed or axes. The 13,600 requests here reuse only its stated
construction, with the explicit replacements in `CARD.md`; they are **not** a
byte-identical reconstruction of its historical corpus. N=1/D=4 in step 1 and
N=1/D=4 or 16 in step 2 bound this result. Product-default N=16/D=1000, world-frame
input, the proposed 60-candidate menu-mode drag chooser, Linux and native input
remain open. The Rust source is an LX-9 candidate using the specified LX-6
toolchain, not evidence that an LX-6 engine implementation was executed.

Cannot show: equality for every request from a finite sample, input latency, or
an approved change to state contract A1. Replay of recorded rotations is a
different test. No state contract, model, product code, journal or user session
was changed. Integrator review and any EN1 Astra/owner amendment decision remain
outside this packet. No decision field is closed by a passing harness.
"""
    (output_dir / "RESULT.md").write_bytes(text.encode())


def main():
    before = file_hashes()
    write_record = False
    try:
        parser = argparse.ArgumentParser(description=__doc__)
        parser.add_argument("--freeze-step2", action="store_true")
        parser.add_argument("--write-record", action="store_true",
                            help="explicitly replace the stored Windows record and reports")
        args = parser.parse_args()
        write_record = args.write_record or args.freeze_step2
        assert sys.platform == "win32", "use linux_leg.py on Linux"
        start = time.monotonic()
        manifest = verify_manifest()
        verify_construction(manifest)
        if args.freeze_step2:
            assert manifest["step2_included_step1_ids"] is None
        else:
            assert manifest["step2_included_step1_ids"] is not None
            verify_candidate()
        cleanup()
        BUILD.mkdir()
        (BUILD / "tmp").mkdir()
        env = {**os.environ, "TMP": str(BUILD / "tmp"), "TEMP": str(BUILD / "tmp"),
               "PYTHONDONTWRITEBYTECODE": "1"}
        steps = {}
        engine = engine_python()
        rust_verbose = command(["rustc", "--version", "--verbose"], env)
        exe = BUILD / "float-map.exe"
        command(["rustc", "--edition=2024", "-C", "opt-level=2", "-C", "debuginfo=0",
                 "-C", "panic=abort", HERE / "rust_input_map.rs", "-o", exe], env)
        steps[1] = run_step(1, engine, exe, env)
        ids = inclusion_ids(steps[1])
        if args.freeze_step2:
            freeze_step2(ids)
            manifest = verify_manifest()
            verify_construction(manifest)
            (HERE / "CANDIDATE.json").write_bytes(json_bytes({
                "schema": "lx9-candidate-v1", "decision": "open", "decider": "owner",
                "files": candidate_files()}))
        assert ids == manifest["step2_included_step1_ids"], "step-1 differences changed; do not silently refreeze"
        steps[2] = run_step(2, engine, exe, env)
        for leg in PRIMARY:
            for name in ids:
                assert steps[1]["rows"][leg][name] == steps[2]["rows"][leg][name]
        meta = steps[1]["metadata"]
        versions = {"pure_python": meta["pure"]["python"],
                    "reference_python": meta["reference"]["python"], "numpy": meta["reference"]["numpy"],
                    "rustc": rust_verbose.splitlines()[0],
                    "rust_host": next(x.split(": ", 1)[1] for x in rust_verbose.splitlines() if x.startswith("host:"))}
        verify_candidate()
        publish_results(steps, manifest, versions, time.monotonic() - start, BUILD)
        if write_record:
            for path in BUILD.iterdir():
                if path.name in ("WINDOWS.json", "RESULT.md", "differences.tsv", "AMENDMENT.md") or path.name.startswith("windows-"):
                    (HERE / path.name).write_bytes(path.read_bytes())
        else:
            verify_record()
    finally:
        try:
            cleanup()
        finally:
            if not write_record:
                verify_unchanged(before)
    assert not BUILD.exists()
    print("PASS: frozen requests, Windows steps 1 and 2, exact controls; build products removed"
          + ("; stored record written." if write_record else "; experiment files unchanged."), flush=True)


if __name__ == "__main__":
    main()
