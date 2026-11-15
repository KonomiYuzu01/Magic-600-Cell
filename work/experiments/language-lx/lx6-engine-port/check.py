"""Packet acceptance: frozen inputs, Rust self-tests, all seven oracle cases, smoke, cleanup."""
import json
import subprocess
import sys

sys.dont_write_bytecode = True
from export_model import BUILD, HERE, sha, verify_inputs
from measure import prepare_frozen
from run_oracle import cleanup, environment
import run_oracle


def snapshot():
    return {path.relative_to(HERE).as_posix(): sha(path.read_bytes())
            for path in HERE.rglob("*") if path.is_file() and not path.is_relative_to(BUILD)}


def stable_smoke(path):
    report = json.loads(path.read_bytes())
    for backend in report["smoke"]["backends"].values():
        for row in backend["rows"]:
            del row["ns"]
    return report


def check():
    candidate = prepare_frozen()
    # The unchanged oracle driver writes its reports relative to HERE.
    (BUILD / "RESULT.md").write_bytes((HERE / "RESULT.md").read_bytes())
    original_here = run_oracle.HERE
    try:
        run_oracle.HERE = BUILD
        code = run_oracle.run_cases(candidate)
    finally:
        run_oracle.HERE = original_here
    if code:
        return code
    if json.loads((BUILD / "ORACLE_RESULTS.json").read_bytes()) != json.loads(
            (HERE / "ORACLE_RESULTS.json").read_bytes()):
        raise ValueError("Fresh oracle record differs from ORACLE_RESULTS.json")
    output = BUILD / "SMOKE.json"
    subprocess.run([sys.executable, "-B", str(HERE / "measure.py"), "--smoke", "--output", str(output)],
                   env=environment(), check=True, timeout=600)
    if stable_smoke(output) != stable_smoke(HERE / "SMOKE.json"):
        raise ValueError("Fresh smoke differs from SMOKE.json outside timing fields")
    verify_inputs()
    return 0


def main():
    before = snapshot()
    code = 1
    try:
        code = check()
    except (OSError, ValueError, RuntimeError, KeyError, TypeError, subprocess.SubprocessError) as error:
        # No machine-specific paths or raw diagnostics are saved in the public records.
        print("FAIL: " + type(error).__name__ + ": " + str(error), file=sys.stderr)
    finally:
        try:
            cleanup()
        finally:
            after = snapshot()
            changed = sorted(name for name in before.keys() | after.keys() if before.get(name) != after.get(name))
            if changed:
                print("FAIL: experiment files changed: " + ", ".join(changed), file=sys.stderr)
                code = 1
    if code == 0:
        print("PASS: all seven cases, every field/step, frozen digests, timing smoke and unchanged files.", flush=True)
    return code


if __name__ == "__main__":
    raise SystemExit(main())
