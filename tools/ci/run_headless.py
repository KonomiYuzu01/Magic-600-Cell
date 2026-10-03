"""Run the headless checks of the Windows CI job, one after another.

Each check runs in its own process from the repository root, with a time
limit, and its output is printed as it runs. The run fails if any check
fails or times out. A pass shows only that these headless checks pass on
the runner: it is not Windows/DirectX, input, long-session or performance
evidence (AGENTS.md "Evidence and publishing").

Usage: python tools/ci/run_headless.py [--list]
"""
from __future__ import annotations

import argparse
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

# Checks that need nothing beyond CPython and NumPy. Harness scripts that take
# a work directory, a URL or a native profile are not tests and are not listed.
CHECKS: list[list[str]] = [
    ["tools/checkout_bytes.py"],
    ["tools/wiki/lint.py"],
    ["tools/skills/sync.py", "--check"],
    ["tests/test_checkout_bytes.py"],
    ["tests/test_core.py"],
    ["tests/test_reference_maps.py"],
    ["tests/test_crash.py"],
    ["tests/test_engine_lifecycle.py"],
    ["tests/test_oracle.py"],
    ["tests/test_full_reference_replay.py"],
    ["tests/test_scramble.py"],
    ["tests/test_piece_filters.py"],
    ["tests/test_frame_preferences.py"],
    ["tests/test_orbit_switch_atomic.py"],
    ["tests/test_v02.py"],
    ["tests/test_log_budget.py"],
    ["tests/test_log_workflow.py"],
    ["tests/test_fixture_respects_locks.py"],
    ["tests/test_debug_cleanup.py"],
    ["tests/test_packaged_engine_command.py"],
    ["tests/test_native_launch_reuse.py"],
    ["tests/test_native_reply_after_commit.py"],
    ["tests/test_native_startup_contract.py"],
    ["tests/test_native_bridge.py"],
    ["tests/test_workflow_windows.py"],
    ["tests/test_experiment_send_outcome.py"],
    ["tests/test_build_identity.py"],
    ["tests/test_print_identity.py"],
    ["tests/test_continuity_04.py"],
    ["tests/test_b412_fixture.py"],
    ["tests/test_b412_summary.py"],
    ["tests/test_turn_probe.py"],
    ["tests/test_renderer_gate.py"],
    ["tests/test_renderer_tools.py"],
    ["tests/test_migration_probes.py"],
    ["tests/test_migration_p2.py"],
    ["tests/test_inventory_check.py"],
    ["tests/test_forecast_guard.py"],
    ["tests/test_agent_rules_sync.py"],
    ["tests/test_codex_review.py"],
    ["tests/test_codex_implement.py"],
    ["tests/test_stop_gate.py"],
    ["tests/test_bootstrap.py"],
    ["tests/test_wiki_lint.py"],
    ["tests/test_workbench.py"],
]

TIME_LIMIT_SECONDS = 1200


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--list", action="store_true", help="print the checks and exit")
    args = parser.parse_args()
    if args.list:
        for check in CHECKS:
            print(" ".join(check))
        return 0
    env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1", PYTHONUTF8="1")
    results: list[tuple[str, str, float]] = []
    for check in CHECKS:
        name = " ".join(check)
        print(f"::group::{name}", flush=True)
        start = time.monotonic()
        try:
            code = subprocess.run([sys.executable, *check], cwd=ROOT, env=env,
                                  timeout=TIME_LIMIT_SECONDS).returncode
            outcome = "pass" if code == 0 else f"fail (exit {code})"
        except subprocess.TimeoutExpired:
            outcome = f"fail (over {TIME_LIMIT_SECONDS} s)"
        elapsed = time.monotonic() - start
        print("::endgroup::", flush=True)
        results.append((name, outcome, elapsed))
    width = max(len(name) for name, _, _ in results)
    print("\nSummary (headless checks only; not Windows/DirectX or performance evidence)")
    for name, outcome, elapsed in results:
        print(f"{name:<{width}}  {outcome:<18} {elapsed:7.1f} s")
    failed = [name for name, outcome, _ in results if outcome != "pass"]
    print(f"\n{len(results) - len(failed)} of {len(results)} checks passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
