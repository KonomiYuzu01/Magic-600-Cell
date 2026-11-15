"""LX-5 crate follow-up: LX-5's frozen Rust candidate with num-bigint 0.5.1 in place of its own big integers.

Verify mode (the default) builds offline from the pinned crates after checking each archive against Cargo.lock,
compares the candidate's answers with LX-5's frozen reference on every fixture case, runs LX-5's Rust self-test and a
smoke timing, compares the fresh records with the stored ones, removes build/ and checks that neither this folder nor
LX-5's changed. --record refreshes the stored records instead. --full runs three series of 100 samples per measured
case with a public D6 conditions record; only such a run on the owner's quiet machine can be decisive. Smoke timing
decides nothing.

    python -I -B work/experiments/language-lx/lx5-crate/check.py [--record]
    python -I -B work/experiments/language-lx/lx5-crate/check.py --full --conditions <d6.json> [--output <file>]

-I applies because the check reads downloaded archives. CARD.md says how the archives are fetched.
"""
import argparse
from copy import deepcopy
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tomllib

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
LX5 = HERE.parent / "lx5-arithmetic"
REPO = HERE.parents[3]
BUILD = HERE / "build"
REGISTRY = "registry+https://github.com/rust-lang/crates.io-index"
INDEX = "index.crates.io-1949cf8c6b5b557f"
NAME = "rust-num-bigint"
SOURCES = ("CARD.md", "Cargo.lock", "Cargo.toml", "candidate.rs", "check.py", "make_candidate.py")
D6_KEYS = ("power_source", "windows_power_mode", "vendor_performance_mode", "driver_vsync", "overlays", "frame_generation")
DROPPED = {"RUSTFLAGS", "RUSTDOCFLAGS", "RUSTC", "RUSTDOC", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "RUSTC_BOOTSTRAP"}
TIMING_SCOPE = ("operation on already parsed operands, including canonical output formatting; "
                "excludes startup and file input except word stream I/O")
# The run time, the fresh smoke samples, and the executable (clean builds may differ, as LX-5 saw for its own Rust).
CHECK_VOLATILE = ("started_utc", "smoke_sha256", "build_digest")
sys.path[:0] = [str(HERE), str(LX5)]
import common  # noqa: E402  LX-5's helpers; its build/ holds the word streams during a run
from compare import compare  # noqa: E402
from make_fixtures import freeze  # noqa: E402
from measure import source_identity as lx5_identity, stats  # noqa: E402
from reference import load  # noqa: E402
import make_candidate  # noqa: E402


def sha(data):
    return hashlib.sha256(data).hexdigest()


def folder(root):
    """SHA-256 of every file under root except its build/ directory."""
    build = root / "build"
    return {path.relative_to(root).as_posix(): sha(path.read_bytes())
            for path in sorted(root.rglob("*")) if path.is_file() and not path.is_relative_to(build)}


def changed(before, after):
    return sorted(name for name in before.keys() | after.keys() if before.get(name) != after.get(name))


def identity():
    return {name: sha((HERE / name).read_bytes()) for name in SOURCES}


def _writable(function, path, _error):
    os.chmod(path, 0o700)
    function(path)


def remove_build():
    if BUILD.resolve().parent != HERE or BUILD.is_symlink():
        raise RuntimeError("build directory escaped the experiment")
    if BUILD.exists():
        shutil.rmtree(BUILD, onexc=_writable)


def show(error):
    """Console-only diagnostics. Records never carry raw output or paths."""
    stderr = getattr(error, "stderr", None)
    if stderr:
        print(stderr.decode("utf-8", "replace")[-6000:], flush=True)
    elif not isinstance(error, subprocess.SubprocessError):
        print(repr(error)[:2000], flush=True)


def prepare_cargo_home(crates):
    """A private cargo home: the fetched index cache plus only the archives whose SHA-256 matches Cargo.lock."""
    home = BUILD / "cargo-home"
    shutil.copytree(crates / "registry/index" / INDEX, home / "registry/index" / INDEX)
    cache = home / "registry/cache" / INDEX
    cache.mkdir(parents=True)
    verified = {}
    for package in tomllib.loads((HERE / "Cargo.lock").read_text("utf-8"))["package"]:
        if "source" not in package:
            continue  # the candidate itself
        if package["source"] != REGISTRY:
            raise RuntimeError("Cargo.lock names a source other than crates.io")
        archive = f"{package['name']}-{package['version']}.crate"
        data = (crates / "registry/cache" / INDEX / archive).read_bytes()
        if sha(data) != package["checksum"]:
            raise RuntimeError("archive differs from its Cargo.lock checksum: " + archive)
        (cache / archive).write_bytes(data)
        verified[f"{package['name']} {package['version']}"] = package["checksum"]
    return home, verified


def msvc():
    """The linker and library path LX-5's common.py uses for its own Rust candidate."""
    program_files = Path(os.environ["ProgramFiles(x86)"])
    vswhere = program_files / "Microsoft Visual Studio/Installer/vswhere.exe"
    vs = Path(subprocess.run([str(vswhere), "-latest", "-products", "*", "-requires",
                              "Microsoft.VisualStudio.Component.VC.Tools.x86.x64", "-property", "installationPath"],
                             capture_output=True, timeout=120, check=True).stdout.decode().strip())
    tools = sorted((vs / "VC/Tools/MSVC").iterdir())[-1]
    kit = sorted((program_files / "Windows Kits/10/Lib").iterdir())[-1]
    return tools / "bin/Hostx64/x64/link.exe", os.pathsep.join(
        str(path) for path in (tools / "lib/x64", kit / "ucrt/x64", kit / "um/x64"))


def cargo_environment(home):
    tmp = BUILD / "tmp"
    tmp.mkdir(parents=True, exist_ok=True)
    env = {key: value for key, value in os.environ.items()
           if not key.upper().startswith("CARGO") and key.upper() not in DROPPED}
    env.update(CARGO_HOME=str(home), CARGO_TARGET_DIR=str(BUILD / "target"), CARGO_NET_OFFLINE="true",
               CARGO_TERM_COLOR="never", CARGO_INCREMENTAL="0", RUSTUP_AUTO_INSTALL="0", TMP=str(tmp), TEMP=str(tmp))
    if os.name == "nt":
        linker, lib = msvc()
        env.update(LIB=lib, CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=str(linker))
    return env


def tool_versions(env):
    def output(*args):
        return subprocess.run(args, cwd=HERE, env=env, capture_output=True, timeout=120, check=True).stdout.decode().strip()
    return {"python": sys.version.split()[0], "platform": sys.platform,
            "rustc": output("rustc", "--version"), "cargo": output("cargo", "--version")}


def build(env):
    if any((path / ".cargo").exists() for path in (HERE, *HERE.parents)):
        raise RuntimeError("a .cargo directory above the experiment would change the build")
    subprocess.run(["cargo", "build", "--release", "--offline", "--locked", "--manifest-path", str(HERE / "Cargo.toml")],
                   cwd=HERE, env=env, capture_output=True, timeout=1800, check=True)
    return BUILD / "target/release" / ("candidate.exe" if os.name == "nt" else "candidate")


def licences(home):
    """Licence field and licence files of each archive cargo extracted from the verified cache."""
    result = {}
    for path in sorted((home / "registry/src" / INDEX).iterdir()):
        package = tomllib.loads((path / "Cargo.toml").read_text("utf-8"))["package"]
        files = sorted(item.name for item in path.iterdir()
                       if item.name.upper().startswith(("LICENSE", "LICENCE", "COPYING")))
        result[path.name] = {"license": package.get("license", "missing"), "files": files}
    return result


def run_candidate(exe, output, *extra, timeout):
    return common.run([exe, LX5 / "fixtures/operands.tsv", common.BUILD / "words", output, *extra], timeout=timeout)


def correctness(exe, expected, cases, values):
    output = BUILD / "answers.tsv"
    try:
        run_candidate(exe, output, timeout=300)
    except subprocess.TimeoutExpired:
        print(NAME + ": timeout; sizes remain untested", flush=True)
        return {"status": "not run to completion", "reason": "300 second timeout"}
    except subprocess.CalledProcessError as error:
        show(error)
        return {"status": "not run to completion", "reason": "candidate exit " + str(error.returncode)}
    actual = output.read_bytes()
    result = compare(expected, actual, cases, values)
    got = actual.splitlines(keepends=True)
    result["differing_case_ids"] = [case["id"] for i, (case, line) in enumerate(zip(cases, expected.splitlines(keepends=True)))
                                    if i >= len(got) or got[i] != line]
    print(f"{NAME}: {result['status']}, {result['difference_count']} differences on {len(cases)} fixtures", flush=True)
    return result


def bench(exe, smoke, expected, values, cases):
    """LX-5's per-case timing protocol (measure.py), for this one candidate."""
    count, series_count = (3, 1) if smoke else (100, 3)
    selected = [c for c in cases if c["measured"]]
    ids = {c["id"] for c in selected}
    wanted = b"".join(line for line in expected.splitlines(keepends=True)
                      if line.split(b"\t", 1)[0].decode("ascii") in ids)
    output = BUILD / "bench.tsv"
    try:
        run_candidate(exe, output, "bench", str(count), str(series_count), timeout=600 if smoke else 7200)
    except subprocess.TimeoutExpired:
        return {"status": "not run to completion", "reason": "timeout; no passing timing result"}
    except subprocess.CalledProcessError as error:
        show(error)
        return {"status": "not run to completion", "reason": "candidate exit " + str(error.returncode)}
    comparison = compare(wanted, output.read_bytes(), selected, values)
    raw = output.with_name(output.name + ".timings.tsv").read_bytes()
    buckets = {(c["id"], series): {} for c in selected for series in range(series_count)}
    for line in raw.decode("ascii").splitlines():
        key, series, sample, ns = line.split("\t")
        series, sample, ns = int(series), int(sample), int(ns)
        assert 0 <= sample < count and ns >= 0
        assert sample not in buckets[key, series], "duplicate timing sample"
        buckets[key, series][sample] = ns
    rows = []
    for c in selected:
        per_series = []
        for series in range(series_count):
            bucket = buckets[c["id"], series]
            assert set(bucket) == set(range(count)), "missing timing sample"
            samples = [bucket[i] for i in range(count)]
            row = dict(series=series, **stats(samples))
            if smoke:
                row["raw_ns"] = samples
            per_series.append(row)
        rows.append(dict(case=c["id"], size=c["size"], operation=c["op"], series=per_series))
    print(f"{NAME}: {'smoke' if smoke else 'full'} exact={comparison['status']} metrics={len(rows)}", flush=True)
    return {"status": comparison["status"], "comparison": comparison, "rows": rows,
            "raw_timing_sha256": sha(raw), "raw_timing_bytes": len(raw),
            "raw_retained": "inline samples" if smoke else "digest; regenerate with check.py --full"}


def timing_record(smoke, started, ident, conditions, candidate):
    """Same shape as LX-5's measure.py records, so rows compare case by case."""
    count, series = (3, 1) if smoke else (100, 3)
    return {"format": "lx5-measure/1", "mode": "smoke" if smoke else "full", "started_utc": started,
            "source_digests": ident, "samples_per_series": count, "series": series, "warmups_per_case_per_series": 1,
            "timing_scope": TIMING_SCOPE, "evidence_class": ["source", "synthetic"],
            "timing_status": "lead only; no decisive timing claim",
            "conditions": conditions or {key: "not recorded" for key in D6_KEYS},
            "decision": "open", "decider": "owner", "candidates": {NAME: candidate},
            "complete": candidate["status"] in ("exact", "removed")}


def run_check(mode, crates, conditions):
    started = datetime.now(timezone.utc).isoformat()
    ident = identity()
    smoke = mode == "check"
    report = {"format": "lx5-crate-check/1", "mode": mode, "started_utc": started, "source_digests": ident,
              "candidate": NAME, "correctness": {"status": "not run"}, "rust_self_test": "not run",
              "complete": False, "decision": "open", "decider": "owner"}
    timing = None
    try:
        report["lx5_candidate_sha256"] = make_candidate.verify()
        manifest = (LX5 / "SOURCE-MANIFEST.json").read_bytes()
        if manifest != common.json_bytes(lx5_identity()):
            raise RuntimeError("LX-5 sources differ from their frozen manifest")
        report["lx5_source_manifest_sha256"] = sha(manifest)
        home, report["crates"] = prepare_cargo_home(crates)
        env = cargo_environment(home)
        report["tools"] = tool_versions(env)
        exe = build(env)
        report["build_digest"] = sha(exe.read_bytes())
        report["licences"] = licences(home)
        freeze()
        report["fixture_manifest_sha256"] = sha((LX5 / "fixtures/MANIFEST.json").read_bytes())
        values, cases = load(LX5 / "fixtures/operands.tsv")
        report["sizes"] = {size: sum(c["size"] == size for c in cases) for size in sorted({c["size"] for c in cases})}
        expected = common.reference_bytes()
        report["correctness"] = correctness(exe, expected, cases, values)
        common.write(BUILD / "reference.tsv", expected)
        try:
            report["rust_self_test"] = run_candidate(exe, BUILD / "random.tsv", "selftest", BUILD / "reference.tsv",
                                                     timeout=300).stdout.decode("ascii").strip()
        except (subprocess.CalledProcessError, subprocess.TimeoutExpired) as error:
            report["rust_self_test"] = "failed (" + type(error).__name__ + ")"
            show(error)
        print(report["rust_self_test"], flush=True)
        timing = timing_record(smoke, started, ident, conditions, bench(exe, smoke, expected, values, cases))
        if smoke:
            report["smoke_status"] = timing["candidates"][NAME]["status"]
        if identity() != ident:
            raise RuntimeError("source changed during the run")
        report["complete"] = (report["correctness"]["status"] in ("exact", "removed")
                              and report["rust_self_test"].startswith("Rust self-test:") and timing["complete"])
    except Exception as error:
        report["failure"] = type(error).__name__ + "; incomplete (diagnostics not published)"
        print(report["failure"], flush=True)
        show(error)
    finally:
        common.cleanup()
        remove_build()
        report["build_removed"] = not BUILD.exists() and not common.BUILD.exists()
    return report, timing


def stable_check(report):
    result = deepcopy(report)
    for key in CHECK_VOLATILE:
        result.pop(key, None)
    return result


def stable_smoke(record):
    result = deepcopy(record)
    result.pop("started_utc", None)
    for candidate in result["candidates"].values():
        candidate.pop("raw_timing_sha256", None)
        candidate.pop("raw_timing_bytes", None)
        for row in candidate.get("rows", []):
            for series in row["series"]:
                for key in ("median_ns", "p99_ns", "min_ns", "max_ns", "raw_ns"):
                    series.pop(key, None)
    return result


def verify_records(fresh):
    stored = {name: (HERE / name).read_bytes() for name in ("check-results.json", "smoke.json", "RESULT.md")}
    stored_report = json.loads(stored["check-results.json"])
    problems = []
    if stable_check(stored_report) != stable_check(json.loads(fresh["check-results.json"])):
        problems.append("check-results.json")
    if "smoke.json" not in fresh or stable_smoke(json.loads(stored["smoke.json"])) != stable_smoke(json.loads(fresh["smoke.json"])):
        problems.append("smoke.json")
    if stored["RESULT.md"] != result_markdown(stored_report).encode("utf-8"):
        problems.append("RESULT.md")
    return problems


def result_markdown(report):
    c = report["correctness"]
    by_size = c.get("by_size", {})
    equal = sum(row["equal"] for row in by_size.values())
    crates = "; ".join(f"{name} `{digest}`" for name, digest in sorted(report.get("crates", {}).items()))
    licence = "; ".join(f"{name}: {item['license']} ({', '.join(item['files']) or 'no licence file'})"
                        for name, item in sorted(report.get("licences", {}).items()))
    tools = "; ".join(f"{key} `{value}`" for key, value in sorted(report.get("tools", {}).items()))
    lines = ["# LX-5 crate follow-up result", "",
        "Experiment code; source and synthetic evidence only. Nothing here selects a route, an engine language, a "
        "dependency or an LQ answer; the owner decides.", "",
        "| Field | Record |", "| --- | --- |",
        "| Card | `CARD.md`: LX-5 follow-up (RTE.54, RTE.55) under decision 13 of `docs/wiki/decisions/owner-decisions-2026-11-06.md` |",
        f"| Run UTC | {report['started_utc']} |",
        f"| Frozen LX-5 sources | candidate SHA-256 `{report.get('lx5_candidate_sha256', 'not checked')}`; "
        f"source manifest `{report.get('lx5_source_manifest_sha256', 'not checked')}` |",
        f"| Frozen fixtures | `fixtures/MANIFEST.json` SHA-256 `{report.get('fixture_manifest_sha256', 'not checked')}` |",
        f"| Crate archives, checked against `Cargo.lock` | {crates or 'not checked'} |",
        f"| Licences, read from the verified archives | {licence or 'not read'} |",
        f"| Tools | {tools or 'not recorded'} |",
        f"| Correctness | {c['status']}; {equal:,} of {sum(report.get('sizes', {}).values()):,} frozen cases "
        "byte-identical to LX-5's reference |",
        f"| LX-5 Rust self-test | {report['rust_self_test']} |",
        f"| Smoke timing | {report.get('smoke_status', 'not run')}; three samples in one series on a shared machine, "
        "a harness check that decides nothing |",
        f"| Complete / build removed | {str(report['complete']).lower()} / {str(report.get('build_removed', False)).lower()} |",
        "| Decision / decider | open / owner |", ""]
    if by_size:
        lines += ["| Size / corpus | Cases | Equal | Different |", "| --- | ---: | ---: | ---: |"]
        lines += [f"| {size} | {row['cases']} | {row['equal']} | {row['different']} |" for size, row in sorted(by_size.items())]
        lines.append("")
    if report.get("failure"):
        lines += ["Unfinished: " + report["failure"] + ".", ""]
    lines += [
        "`candidate.rs` is LX-5's frozen Rust candidate with only its header, imports and `Int` type replaced. `Int` is a "
        "sign and a num-bigint `BigUint` and keeps every method's contract; `make_candidate.py` checks that everything "
        "from `fn rat` on is byte-identical. One known difference: the parser accepts only ASCII digits after an optional "
        "minus sign, while LX-5's own parser also let Rust's integer parser take some plus signs. The frozen inputs have none.", "",
        "Equality is LX-5's: byte for byte over the canonical answer file, through LX-5's own comparer. Verify mode compares "
        "fresh records with the stored ones except the run time, the executable digest and the smoke timings, and "
        "regenerates this page from the stored record. The build uses cargo's default release settings at opt-level 2, "
        "like LX-5's `rustc -O`, without cross-crate link-time optimization.", "",
        "Full mode: **not run here**. It belongs on the owner's quiet machine with a D6 conditions record, right after "
        "LX-5's own `measure.py`, so that both Rust candidates are timed by the same bench code under the same recorded "
        "conditions: `python -I -B work/experiments/language-lx/lx5-crate/check.py --full --conditions <record> --output <file>`.", "",
        "Linux leg: **not run here**; it waits for the owner's WSL installation (decision 14 of the same record).", "",
        "Cannot show: a speed threshold, that num-bigint is fit as a product dependency, whole-engine correctness, or "
        "Windows/DirectX, renderer, input or long-session behaviour.", ""]
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--record", action="store_true", help="refresh check-results.json, smoke.json and RESULT.md")
    parser.add_argument("--full", action="store_true", help="run the three-series measurement instead of the check")
    parser.add_argument("--conditions", type=Path, help="public D6 conditions record (JSON); required with --full")
    parser.add_argument("--output", type=Path, help="full-mode result file (default: full-measurement.json here)")
    parser.add_argument("--crates", type=Path, default=REPO / "work/lx5-crate-fetch/cargo-home",
                        help="cargo home that holds the fetched index cache and crate archives")
    args = parser.parse_args()
    if args.full and (args.record or args.conditions is None):
        parser.error("--full needs --conditions and excludes --record")
    if not args.full and (args.conditions or args.output):
        parser.error("--conditions and --output belong to --full")
    conditions = None
    if args.full:
        conditions = json.loads(args.conditions.read_bytes())
        missing = [key for key in D6_KEYS if not str(conditions.get(key, "")).strip() or conditions[key] == "not recorded"]
        if missing:
            parser.error("the conditions record lacks " + ", ".join(missing))
    for path in (BUILD, common.BUILD):
        if path.exists():
            raise SystemExit(path.relative_to(REPO).as_posix() + " exists; another run may own it")
    lx5_before, here_before = folder(LX5), folder(HERE)
    report, timing = run_check("full" if args.full else "check", args.crates, conditions)
    accepted = (report["complete"] and report["build_removed"] and report["correctness"]["status"] == "exact"
                and report["rust_self_test"].startswith("Rust self-test:")
                and timing is not None and timing["candidates"][NAME]["status"] == "exact")
    if args.full:
        timing = timing or timing_record(False, report["started_utc"], report["source_digests"], conditions,
                                         {"status": "not run"})
        timing["check"] = report
        common.write(args.output or HERE / "full-measurement.json", common.json_bytes(timing))
    else:
        fresh = {}
        if timing is not None:
            fresh["smoke.json"] = common.json_bytes(timing)
            report["smoke_sha256"] = sha(fresh["smoke.json"])
        fresh["check-results.json"] = common.json_bytes(report)
        fresh["RESULT.md"] = result_markdown(report).encode("utf-8")
        if args.record:
            for name, data in fresh.items():
                common.write(HERE / name, data)
        else:
            problems = verify_records(fresh)
            here_changed = changed(here_before, folder(HERE))
            if problems:
                print("Stored records differ: " + ", ".join(problems), flush=True)
            if here_changed:
                print("Folder changed during verification: " + ", ".join(here_changed), flush=True)
            accepted = accepted and not problems and not here_changed
    lx5_changed = changed(lx5_before, folder(LX5))
    if lx5_changed:
        print("LX-5 folder changed: " + ", ".join(lx5_changed), flush=True)
        accepted = False
    print("Build removed: " + str(report["build_removed"]).lower(), flush=True)
    print("Acceptance: " + ("passed" if accepted else "failed"), flush=True)
    return 0 if accepted else 1


if __name__ == "__main__":
    raise SystemExit(main())
