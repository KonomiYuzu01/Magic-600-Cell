"""Run the Rust tests of 600 Cell Script: `cargo test --locked --offline` in rust/ (LP2 plan, section 3).

The workspace uses the standard library and the owner-approved big-integer crates, which are vendored in rust/vendor,
so the run needs no network, and a lockfile that does not match the manifests fails it. The kernel's oracle replay
runs in release in its own check, `tests/test_kernel_oracle.py`. A missing Rust toolchain fails the check: a test
that did not run is no pass (PHS.63). A pass is headless source and fixture evidence, not Windows/DirectX, input,
long-session or performance evidence.
"""
from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WORKSPACE = ROOT / "rust"
# Below the CI runner's limit per check (tools/ci/run_headless.py), so a slow run is reported here.
TIME_LIMIT_SECONDS = 1100


def version(tool: str) -> str:
    try:
        out = subprocess.run([tool, "--version"], capture_output=True, text=True, timeout=60)
    except (OSError, subprocess.TimeoutExpired) as error:
        return f"{Path(tool).name}: unavailable ({error})"
    return out.stdout.strip() or f"{Path(tool).name}: no version (exit {out.returncode})"


def main() -> int:
    cargo = shutil.which("cargo")
    if cargo is None:
        print("FAIL: cargo is not on PATH, so the Rust tests did not run")
        return 1
    print(version(cargo))
    print(version(shutil.which("rustc") or "rustc"), flush=True)
    command = [cargo, "test", "--locked", "--offline"]
    try:
        code = subprocess.run(command, cwd=WORKSPACE, timeout=TIME_LIMIT_SECONDS).returncode
    except subprocess.TimeoutExpired:
        print(f"FAIL: cargo test ran over {TIME_LIMIT_SECONDS} s")
        return 1
    if code != 0:
        print(f"FAIL: cargo test --locked --offline exited with {code}")
        return 1
    print("PASS: cargo test --locked --offline in rust/")
    return 0


if __name__ == "__main__":
    sys.exit(main())
