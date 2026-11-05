"""Verify LX-5 without changing stored files; --record explicitly refreshes results."""
import argparse
from copy import deepcopy
from datetime import datetime, timezone
import json
import subprocess
import sys

sys.dont_write_bytecode = True
from common import BUILD, CANDIDATES, ROOT, cleanup, compile_candidates, json_bytes, reference_bytes, run, sha, versions, write
from compare import compare, self_test as compare_self_test
from make_fixtures import freeze
from measure import source_identity
from reference import load, self_test as reference_self_test


CHECK_VOLATILE_FIELDS = (
    ("started_utc",),  # The run's wall-clock timestamp changes each time.
    ("smoke_sha256",),  # The smoke digest includes fresh timing samples.
    ("build_digests", "rust_executable"),  # Two clean builds differed: 3e4db8b0803c / cdd3431cd67e.
)


def stable_check_report(report):
    result = deepcopy(report)
    for path in CHECK_VOLATILE_FIELDS:
        parent = result
        for key in path[:-1]:
            parent = parent.get(key, {})
        parent.pop(path[-1], None)
    return result


def stable_smoke_report(report):
    result = deepcopy(report)
    result.pop("started_utc", None)
    for candidate in result["candidates"].values():
        candidate.pop("raw_timing_sha256", None)
        candidate.pop("raw_timing_bytes", None)
        for row in candidate.get("rows", []):
            for series in row["series"]:
                for key in ("median_ns", "p99_ns", "min_ns", "max_ns", "raw_ns"):
                    series.pop(key, None)
    return result


def folder_digests():
    return {path.relative_to(ROOT).as_posix(): sha(path.read_bytes())
            for path in ROOT.rglob("*") if path.is_file() and not path.is_relative_to(BUILD)}


def verify_records():
    for name in ("SOURCE-MANIFEST.json", "operation-results.tsv"):
        assert (ROOT / name).read_bytes() == (BUILD / name).read_bytes(), "stored file differs: " + name
    for name, normalize in (("check-results.json", stable_check_report), ("smoke.json", stable_smoke_report)):
        assert normalize(json.loads((ROOT / name).read_bytes())) == normalize(json.loads((BUILD / name).read_bytes())), "stored file differs: " + name
    stored = json.loads((ROOT / "check-results.json").read_bytes())
    write(BUILD / "RESULT.md", report_markdown(stored))
    assert (ROOT / "RESULT.md").read_bytes() == (BUILD / "RESULT.md").read_bytes(), "stored file differs: RESULT.md"


def pass_list(candidates):
    return [name for name in CANDIDATES if candidates[name]["status"] == "exact"]


def report_markdown(report):
    candidates = report["candidates"]
    fixtures = json.loads((ROOT / "fixtures/MANIFEST.json").read_bytes())
    poses = json.loads((ROOT / "fixtures/pose-recipe.json").read_bytes())
    lines = ["# LX-5 exact-arithmetic result", "",
        "Experiment code and source/synthetic evidence only. No route, engine language, syntax, dependency or LQ is selected.", "",
        "| CT-RTE-05 field | Record |", "| --- | --- |",
        "| LX card / experiment ID | LX-5 (RTE.54, RTE.55) / E-2.4-L5 |",
        "| Owner allowance | `docs/wiki/decisions/owner-decisions-2026-11-05.md`, decision 2; assigned implementation packet |",
        "| Time box | what can be done now; one wrapper call of at most two hours |",
        "| Source scope | Packet base `1e7d0074`; only `work/experiments/language-lx/lx5-arithmetic/` added |",
        f"| Run UTC | {report['started_utc']} |",
        f"| Frozen candidate / harness digests | `SOURCE-MANIFEST.json`, SHA-256 `{report['source_manifest_sha256']}`; executable digests in `check-results.json` |",
        f"| Frozen input digests | `fixtures/MANIFEST.json`, SHA-256 `{report['fixture_manifest_sha256']}`; decoded reference `{fixtures['reference_decoded']['sha256']}` |",
        "| Evidence class | source and synthetic; smoke timings are leads on a shared, sandboxed machine |",
        "| Kill criteria | Any exact difference removes that candidate globally; keep operation and operands. Timeout leaves affected sizes untested. A reached limit is no pass. |",
        f"| Actual result | Reference self-test: {report['reference_self_test']}; comparer planted-fault tests: {report['comparer_self_test']}; report complete: {str(report['complete']).lower()} |",
        "| Decision / decider | open / owner (all route, LQ8, LQ9, syntax, dependency and licence decisions) |", "",
        "Record of runs: The first run's acceptance passed and was refused only because its check rewrote `RESULT.md`, `check-results.json` and `smoke.json`; this run landed the same work with a check that writes nothing in its verify mode.", "",
        "Acceptance command: `python work/experiments/language-lx/lx5-arithmetic/check.py`.", "",
        "Only `python work/experiments/language-lx/lx5-arithmetic/check.py --record` refreshes the stored check artifacts. Verify mode compares fresh results, requires all candidates to remain exact, removes `build/` and checks every retained file's SHA-256. The frozen card is unchanged; this packet's acceptance additionally rejects completed negative numeric results.", "",
        "Check comparison excludes only `CHECK_VOLATILE_FIELDS` in `check.py`: the run timestamp, the smoke digest and the Rust executable digest (two clean local builds differed; the .NET assembly was byte-identical). Smoke comparison excludes its timestamp, timing summaries/samples and raw timing digest/byte count; sample counts, series IDs and all correctness and identity fields stay compared. `RESULT.md` is checked against its stored check record.", "",
        "Tools: " + "; ".join(f"{key} `{value}`" for key, value in sorted(report["tools"].items())) + ".", "",
        f"The common frozen list has {fixtures['cases']:,} cases. Equality is byte-for-byte over canonical answer files, including signs, reduced denominators, decimal/hex encodings, full letter histograms, counts, sums and stream SHA-256. Large answer files are deleted; their digests and regeneration command are retained. The frozen reference is gzip-compressed with mtime 0.", "",
        "| Candidate | Overall status | Equal cases | Different cases |", "| --- | --- | ---: | ---: |"]
    for name in CANDIDATES:
        c = candidates[name]
        equal = sum(r["equal"] for r in c.get("by_size", {}).values())
        lines.append(f"| {name} | {c['status']} | {equal} | {c.get('difference_count', 'not run')} |")
    lines += ["", "Passing correctness candidates: " + (", ".join(report["pass_list"]) or "none") + ". This is no dependency approval.", "",
              "| Size / corpus | Cases | Node | .NET | Own Rust limbs |", "| --- | ---: | --- | --- | --- |"]
    for size in report["sizes"]:
        cells = []
        for name in CANDIDATES:
            c = candidates[name]
            row = c.get("by_size", {}).get(size)
            if row:
                cell = f"{row['equal']}/{row['cases']} exact"
                if row["different"]: cell += f"; {row['different']} different"
                if c["status"] == "removed": cell += "; candidate removed"
            else:
                cell = c["status"]
            cells.append(cell)
        lines.append(f"| {size} | {report['sizes'][size]} | " + " | ".join(cells) + " |")
    lines += ["", "`check-results.json` has per-size counts, candidate output digests and any exact differences with their operation, operand references and values. `operation-results.tsv` has each operation/size status. `smoke.json` has each measured operation/size's median, nearest-rank p99 and three raw samples; no threshold is applied.", "",
              "The first case for each measured operation/size is timed after one warmup; input parsing and process startup are excluded, canonical answer formatting is included, and word measurements include file I/O. Random and limb-boundary cases check correctness only. Three smoke samples do not estimate a performance tail; their p99 is simply their maximum.", "",
              "Synthetic pose recipe: exact column-vector powers `(g H)^n` of the first two J1 negative-control journal matrices. The trace 4/3, g squared, and all twelve recorded heights are checked before extension. The largest-component entry (row-major first tie) is used. Powers / actual maximum component bits: " + ", ".join(str(p["rounds"]) + "/" + str(p["component_bits"]) for p in poses["powers"]) + ". These are synthetic extensions, not replayed states. The separate Pell cases give exact, extremely small sign margins without a float decision.", "",
              "Word coverage: recorded word 00014 is read in full (2,352,984 letters); all 53 files are concatenated once in recorded order and read as one 13,231,266-letter stream. The 10,000,000-letter bound uses the frozen xorshift32 recipe. The 672,417- and additional 871,582-letter cases are explicitly labelled prefixes of word 00014: they cover those literal sizes, not the earlier E2/T6 witnesses. Every stream is consumed in 64 KiB chunks with persistent digest and counters. The 166,788,405 length and the recorded-length sum are integer arithmetic only, never a claimed replay.", "",
              "Rust is own experimental code with no crate and an ongoing audit burden, including its own SHA-256. Multiplication is schoolbook O(n*m) in 32-bit limbs. Division is normalized Knuth Algorithm D with quotient estimation, correction and add-back; it uses O((n-m+1)*m) limb work for n >= m >= 2, O(n+m) scratch space, plus normalization. A one-limb divisor uses a linear scan. Euclidean gcd and decimal formatting repeatedly use division. This implementation is not an audited general-purpose bigint dependency.", "",
              "Rust self-test: " + report.get("rust_self_test", "not run here") + ".", "",
              "Licence evidence and unverified recollections are separated in `LICENCES.md`. The unfetched num-bigint, dashu, ibig, malachite and rug candidates remain an open owner question; copyleft needs the owner. No package, crate, network access or installation was used.", "",
              "Full mode: **not run here**. D6 power source, Windows power mode, vendor performance mode, driver V-Sync, overlays and frame generation were not recorded; no decisive timing is claimed. Record them before an owner-machine measurement. Run `python work/experiments/language-lx/lx5-arithmetic/measure.py` for three series of 100 samples per metric; optionally pass `--conditions work/experiments/language-lx/lx5-arithmetic/measurement-conditions.json` after writing a public D6 record. The full-mode harness has not itself been exercised here.", "",
              "Linux/WSL leg: **not run here**. With the same candidates already available, run `python3 work/experiments/language-lx/lx5-arithmetic/check.py` in a Linux checkout. No Linux install is authorized by this packet. Integrator review is still open before integration; no Git index, refs, configuration or hooks were changed.", "",
              "Cannot show: that the synthetic entries occurred in a saved state, that the largest length was played, a general speed threshold, or owner approval of a dependency (the card's limits). It also cannot establish whole-engine correctness, legal witness replay, Windows/DirectX, renderer, GPU, native input or long-session behavior. GPU and replay runs are outside LX-5, so there is no LX-5 GPU command to run.", ""]
    if report.get("failure"):
        lines += ["Unfinished acceptance work: " + report["failure"] + ".", ""]
    return "\n".join(lines)


def operation_table(report, cases):
    lines = ["candidate\tsize\toperation\tcases\tequal\tstatus\n"]
    for name in CANDIDATES:
        c = report["candidates"][name]
        failed = set(c.get("differing_case_ids", []))
        for size, op in sorted({(c["size"], c["op"]) for c in cases}):
            subset = [case for case in cases if case["size"] == size and case["op"] == op]
            equal = sum(case["id"] not in failed for case in subset) if "by_size" in c else 0
            status = "exact" if equal == len(subset) else c["status"]
            if c["status"] == "removed": status += "; candidate removed"
            lines.append(f"{name}\t{size}\t{op}\t{len(subset)}\t{equal}\t{status}\n")
    return "".join(lines)


def check(record):
    identity = source_identity()
    report = {"format": "lx5-check/1", "started_utc": datetime.now(timezone.utc).isoformat(),
              "source_manifest_sha256": sha(json_bytes(identity)), "reference_self_test": "not run",
              "comparer_self_test": "not run", "complete": False, "tools": {}, "sizes": {},
              "decision": "open", "decider": "owner", "candidates": {name: {"status": "not run"} for name in CANDIDATES}}
    cases = []
    smoke_bytes = None
    try:
        reference_self_test(); report["reference_self_test"] = "pass"
        compare_self_test(); report["comparer_self_test"] = "pass"
        # A deliberately rejected candidate can never re-enter the passing list.
        assert pass_list({name: {"status": "removed" if name == "rust" else "exact"} for name in CANDIDATES}) == ["node", "dotnet"]
        freeze()
        report["fixture_manifest_sha256"] = sha((ROOT / "fixtures/MANIFEST.json").read_bytes())
        report["tools"] = versions()
        values, cases = load(ROOT / "fixtures/operands.tsv")
        report["sizes"] = {size: sum(c["size"] == size for c in cases) for size in sorted({c["size"] for c in cases})}
        commands = compile_candidates()
        report["build_digests"] = {"node_source": sha((ROOT / "candidate.mjs").read_bytes()),
            "dotnet_assembly": sha((BUILD / "candidate.dll").read_bytes()),
            "rust_executable": sha(commands["rust"][0].read_bytes())}
        expected = reference_bytes()
        for name in CANDIDATES:
            output = BUILD / (name + ".tsv")
            try:
                run(commands[name] + [ROOT / "fixtures/operands.tsv", BUILD / "words", output], timeout=300)
                actual = output.read_bytes()
                c = compare(expected, actual, cases, values)
                got = actual.splitlines(keepends=True)
                c["differing_case_ids"] = [case["id"] for i, (case, line) in enumerate(zip(cases, expected.splitlines(keepends=True)))
                                            if i >= len(got) or got[i] != line]
                report["candidates"][name] = c
                if c["status"] == "removed":
                    print(f"{name}: REMOVED, {c['difference_count']} exact differences; see check-results.json", flush=True)
                else:
                    print(f"{name}: exact on {len(cases)} fixtures", flush=True)
            except subprocess.TimeoutExpired:
                report["candidates"][name] = {"status": "not run to completion", "reason": "300 second timeout"}
                print(name + ": timeout; sizes remain untested", flush=True)
            except subprocess.CalledProcessError as error:
                report["candidates"][name] = {"status": "not run to completion", "reason": "candidate exit " + str(error.returncode)}
                print(name + ": candidate failed", flush=True)
        write(BUILD / "reference.tsv", expected)
        try:
            result = run(commands["rust"] + [ROOT / "fixtures/operands.tsv", BUILD / "words", BUILD / "rust-random.tsv",
                                            "selftest", BUILD / "reference.tsv"], timeout=300)
            report["rust_self_test"] = result.stdout.decode("ascii").strip()
            print(report["rust_self_test"], flush=True)
        except (subprocess.CalledProcessError, subprocess.TimeoutExpired):
            report["rust_self_test"] = "failed; Rust removed from passing list"
            report["candidates"]["rust"]["status"] = "removed"
        # Keep fresh smoke under build/ until this caller has read it for comparison.
        measured = run([sys.executable, ROOT / "measure.py", "--smoke", "--output", BUILD / "smoke.json", "--keep-build"], timeout=900)
        print(measured.stdout.decode("ascii").strip(), flush=True)
        smoke_bytes = (BUILD / "smoke.json").read_bytes()
        smoke = json.loads(smoke_bytes)
        assert smoke["complete"] and smoke["source_digests"] == identity
        for name in CANDIDATES:
            if smoke["candidates"][name]["status"] == "removed":
                report["candidates"][name]["status"] = "removed"
                report["candidates"][name]["smoke_difference"] = smoke["candidates"][name]["comparison"]
        report["smoke_sha256"] = sha(smoke_bytes)
        assert source_identity() == identity, "source changed during check"
        # Exact differences are completed negative experiments. Unexecuted
        # candidates are not a complete acceptance run, even with a written report.
        report["complete"] = all(c["status"] in ("exact", "removed") for c in report["candidates"].values())
    except Exception as error:
        # Never publish raw subprocess output or private paths.
        report["failure"] = type(error).__name__ + "; acceptance incomplete (diagnostics not published)"
        print(report["failure"], flush=True)
    finally:
        cleanup()
        report["build_removed"] = not BUILD.exists()
        report["pass_list"] = pass_list(report["candidates"])
        # After compiler cleanup, retain only small fresh artifacts for comparison.
        output = ROOT if record else BUILD
        write(output / "SOURCE-MANIFEST.json", json_bytes(identity))
        write(output / "check-results.json", json_bytes(report))
        if "fixture_manifest_sha256" in report:
            write(output / "RESULT.md", report_markdown(report))
        else:
            write(output / "RESULT.md", "# LX-5 incomplete acceptance\n\nReference or fixture verification did not complete. "
                  "No passing experiment is claimed. See check-results.json. Decision: open; decider: owner.\n")
        write(output / "operation-results.tsv", operation_table(report, cases))
        if smoke_bytes is not None:
            write(output / "smoke.json", smoke_bytes)
    if not record:
        verify_records()
    print("Passing candidates: " + (", ".join(report["pass_list"]) or "none"), flush=True)
    accepted = (report["complete"] and report["reference_self_test"] == "pass" and report["comparer_self_test"] == "pass"
                and report["build_removed"] and report["pass_list"] == list(CANDIDATES))
    print("Acceptance: " + ("passed" if accepted else "failed"), flush=True)
    return 0 if accepted else 1


def main(record=False):
    before = folder_digests() if not record else None
    try:
        return check(record)
    finally:
        try:
            cleanup()
        finally:
            if before is not None:
                after = folder_digests()
                changed = sorted(name for name in before.keys() | after.keys() if before.get(name) != after.get(name))
                assert not changed, "folder changed during verification: " + ", ".join(changed)
                print("Folder unchanged; build removed=" + str(not BUILD.exists()).lower(), flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--record", action="store_true", help="refresh the five stored check artifacts")
    raise SystemExit(main(parser.parse_args().record))
