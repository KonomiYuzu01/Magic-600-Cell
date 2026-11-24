"""Run the Rust tests of 600 Cell Script: `cargo test --locked --offline` in rust/ (LP2 plan, section 3).

The workspace uses the standard library and the owner-approved big-integer crates, which are vendored in rust/vendor,
so the run needs no network, and a lockfile that does not match the manifests fails it. The kernel's oracle replay
runs in release in its own check, `tests/test_kernel_oracle.py`, and LP5's KN6 checker crate, `c600-checkers`, runs
in its own check, `tests/test_language_rust_checkers.py`, so that the two land on different Windows shards: on 24
November 2026 the one run of the whole workspace went over its limit, about half of it the checkers' mutation gates.
A missing Rust toolchain fails the check: a test that did not run is no pass (PHS.63). A pass is headless source and
fixture evidence, not Windows/DirectX, input, long-session or performance evidence.
"""
from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WORKSPACE = ROOT / "rust"
# Below the CI runner's limit for this check (LONG_CHECK_SECONDS in tools/ci/run_headless.py), so a slow run is
# reported here. The run took 1045 s on the Windows runner on 17 November 2026, 2240 to 2305 s on 20 November 2026
# (one run stopped at the earlier 2300 s limit with every test it reached passing), 1778 s on 23 November 2026 before
# LP5 landed, and over 3500 s with LP5's checkers in it.
TIME_LIMIT_SECONDS = 3500
# This check's packages: the workspace without the checker crate, which its own check runs.
PACKAGES = ["--workspace", "--exclude", "c600-checkers"]


def version(tool: str) -> str:
    try:
        out = subprocess.run([tool, "--version"], capture_output=True, text=True, timeout=60)
    except (OSError, subprocess.TimeoutExpired) as error:
        return f"{Path(tool).name}: unavailable ({error})"
    return out.stdout.strip() or f"{Path(tool).name}: no version (exit {out.returncode})"


def run(packages: list[str], limit: int = TIME_LIMIT_SECONDS) -> int:
    """`cargo test --locked --offline` over `packages` in rust/, where cargo reads `rust/.cargo/config.toml` (the
    vendored crates); a run over `limit` seconds fails."""
    cargo = shutil.which("cargo")
    if cargo is None:
        print("FAIL: cargo is not on PATH, so the Rust tests did not run")
        return 1
    print(version(cargo))
    print(version(shutil.which("rustc") or "rustc"), flush=True)
    command = [cargo, "test", "--locked", "--offline", *packages]
    shown = " ".join(["cargo", *command[1:]])
    try:
        code = subprocess.run(command, cwd=WORKSPACE, timeout=limit).returncode
    except subprocess.TimeoutExpired:
        print(f"FAIL: cargo test ran over {limit} s")
        return 1
    if code != 0:
        print(f"FAIL: {shown} exited with {code}")
        return 1
    print(f"PASS: {shown} in rust/")
    return 0


def main() -> int:
    return run(PACKAGES)


if __name__ == "__main__":
    sys.exit(main())
