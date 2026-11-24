"""Run the Rust tests of LP5's KN6 checker crate: `cargo test --locked --offline -p c600-checkers` in rust/.

`tests/test_language_rust.py` runs the rest of the workspace. The checker crate runs here, in a check of its own, so
the CI balancer puts the two on different Windows shards: in a debug build the checkers' mutation gates and archive
checks took about 1,800 s of the 3,500 s limit on the Windows runner on 24 November 2026 (kn6_s14 1159 s, kn6_r4
339 s, s18 107 s, kn6_s19 87 s, kn6 85 s). Debug assertions and overflow checks stay on, and no test is ignored for
time. The crate's slow archive cases that are `#[ignore]`d with their reasons run only in release, by hand (LP5 plan,
section 12). A missing Rust toolchain fails the check (PHS.63). A pass is headless source and fixture evidence, not
Windows/DirectX, input, long-session or performance evidence.
"""
from __future__ import annotations

import sys

from test_language_rust import run

# Below the CI runner's limit for this check (LONG_CHECK_SECONDS in tools/ci/run_headless.py).
TIME_LIMIT_SECONDS = 3500
PACKAGES = ["-p", "c600-checkers"]


def main() -> int:
    return run(PACKAGES, TIME_LIMIT_SECONDS)


if __name__ == "__main__":
    sys.exit(main())
