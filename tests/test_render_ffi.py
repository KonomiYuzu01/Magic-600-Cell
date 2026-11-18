"""Run the tests of C9's foreign bridge half and the shell's native host library: `cargo test --locked --offline
--workspace` in renderer/bridge (c600-render-ffi, and its member host/, c600-shell-host).

The crate holds the C function table of the S-A2 core's ABI and its implementation of c600-render's CoreAbi. It is
not a member of the language workspace (rust/), because CT-RPI-01 keeps every member free of unsafe code and foreign
declarations, so `tests/test_language_rust.py` does not run it. Its tests drive the table through a C shim over the
fake core, check every function's signature against renderer/core/include/sa2_interop.h, and run c600-render's host
scenarios again through the table. The host library's tests drive its C interface over the same shim. The run uses
the vendored crates in rust/vendor (renderer/bridge/.cargo/config.toml), so it needs no network, and a lockfile that
does not match the manifests fails it. A missing Rust toolchain
fails the check: a test that did not run is no pass (PHS.63). A pass is headless source and fixture evidence, not
Windows/DirectX, input, long-session or performance evidence.
"""
from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CRATE = ROOT / "renderer" / "bridge"
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
        print("FAIL: cargo is not on PATH, so the bridge's foreign half was not tested")
        return 1
    print(version(cargo), flush=True)
    # Cargo reads .cargo/config.toml from the directory it runs in, so the run starts in the crate.
    command = [cargo, "test", "--locked", "--offline", "--workspace"]
    try:
        code = subprocess.run(command, cwd=CRATE, timeout=TIME_LIMIT_SECONDS).returncode
    except subprocess.TimeoutExpired:
        print(f"FAIL: cargo test ran over {TIME_LIMIT_SECONDS} s")
        return 1
    if code != 0:
        print(f"FAIL: cargo test --locked --offline --workspace in renderer/bridge exited with {code}")
        return 1
    print("PASS: cargo test --locked --offline --workspace in renderer/bridge")
    return 0


if __name__ == "__main__":
    sys.exit(main())
