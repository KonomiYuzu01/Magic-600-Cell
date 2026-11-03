"""Run the headless checks of the Windows CI job, one after another.

Each check runs in its own process from the repository root, with a time
limit, and its output is printed as it runs. The run fails if any check
fails or times out. A pass shows only that these headless checks pass on
the runner: it is not Windows/DirectX, input, long-session or performance
evidence (AGENTS.md "Evidence and publishing").

With --base (or RUN_HEADLESS_BASE, which the workflow sets for pull requests),
a pull request whose changes are only Markdown in the documentation trees
below runs the light checks (owner, 3 November 2026); any other change, an
unreadable diff and every push to main run all checks.

The workflow runs a full set as parallel shards: --plan-github gives the
shard numbers, and --shard K/N runs the K-th of N time-balanced shards.

Usage: python tools/ci/run_headless.py [--list] [--base COMMIT] [--plan | --plan-github] [--shard K/N]
"""
from __future__ import annotations

import argparse
import json
import os
import re
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
    ["tests/test_ci_headless.py"],
    ["tests/test_core.py"],
    ["tests/test_reference_maps.py"],
    ["tests/test_crash.py"],
    ["tests/test_engine_lifecycle.py"],
    ["tests/test_oracle.py"],
    ["tests/test_full_reference_replay.py"],
    ["tests/test_jumbling_differential.py"],
    ["tests/test_jumbling_store.py"],
    ["tests/test_jumbling_j1_replay.py"],
    ["tests/test_jumbling_x1_witness.py"],
    ["tests/test_jumbling_store_j1.py"],
    ["tests/test_jumbling_straddlers.py"],
    ["tests/test_jumbling_leads_repair.py"],
    ["tests/test_jumbling_leads_covers.py"],
    ["tests/test_jumbling_leads_scale.py"],
    ["tests/test_jumbling_nu.py"],
    ["tests/test_jumbling_observations.py"],
    ["tests/test_jumbling_word_check.py"],
    ["tests/test_jumbling_session.py"],
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
    ["tests/test_feature_costs.py"],
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

# The checks that read the documentation trees below: the wiki and its lint,
# the agent rules and briefing, the review wrapper, the Stop gate and the
# workbench (status and gallery), plus the byte, skills and CI-runner checks.
LIGHT_CHECKS: list[list[str]] = [
    ["tools/checkout_bytes.py"],
    ["tools/wiki/lint.py"],
    ["tools/skills/sync.py", "--check"],
    ["tests/test_checkout_bytes.py"],
    ["tests/test_ci_headless.py"],
    ["tests/test_agent_rules_sync.py"],
    ["tests/test_codex_review.py"],
    ["tests/test_stop_gate.py"],
    ["tests/test_wiki_lint.py"],
    ["tests/test_workbench.py"],
]

# Markdown under these trees may run the light checks only.
DOCS_ONLY_TREES = ("docs/wiki/", "docs/development-guide/", "docs/progress/")
# Markdown inside those trees that a full check reads (audited 3 November 2026).
FULL_CHECK_DOCS = (
    "docs/progress/1.0/jumbling-midstage.md",  # tests/test_jumbling_word_check.py
    "docs/progress/1.0/packets/renderer/",     # tests/test_renderer_tools.py
    "docs/progress/1.0/inventory/",            # tests/test_inventory_check.py
    "docs/progress/1.0/dispositions/",         # next to the inventory checker's inputs
)
_PATH_LITERAL = re.compile(r"""['"]([A-Za-z0-9_.\-]+(?:/[A-Za-z0-9_.\-]+)+/?)['"]""")


def changed_files(base: str, root: Path = ROOT) -> list[str] | None:
    """Paths changed from the merge base of base and HEAD, or None if git cannot say."""
    if not base:
        return None
    try:
        merge_base = subprocess.run(["git", "merge-base", base, "HEAD"], cwd=root, check=True,
                                    capture_output=True, text=True).stdout.strip()
        out = subprocess.run(["git", "diff", "--name-only", "--no-renames", "-z", merge_base, "HEAD"],
                             cwd=root, check=True, capture_output=True).stdout
    except (OSError, subprocess.CalledProcessError):
        return None
    names = [name for name in out.decode("utf-8", "surrogateescape").split("\0") if name]
    return names or None


def full_check_literals(root: Path = ROOT) -> set[str]:
    """Repository paths written as string literals in the full checks (a guard for new readers)."""
    light = {check[0] for check in LIGHT_CHECKS}
    literals: set[str] = set()
    for check in CHECKS:
        if check[0] in light:
            continue
        try:
            text = (root / check[0]).read_text(encoding="utf-8")
        except OSError:
            continue
        literals.update(match.rstrip("/") for match in _PATH_LITERAL.findall(text))
    return literals


def select_checks(changed: list[str] | None, literals: set[str]) -> tuple[str, str, list[list[str]]]:
    """Return (mode, reason, checks) for the changed paths."""
    if not changed:
        return "full", "no pull-request diff", CHECKS
    for path in changed:
        if not path.endswith(".md") or not path.startswith(DOCS_ONLY_TREES):
            return "full", f"{path} is not Markdown in a documentation tree", CHECKS
        if any(path == entry or (entry.endswith("/") and path.startswith(entry)) for entry in FULL_CHECK_DOCS):
            return "full", f"{path} is read by a full check", CHECKS
        if any(path == literal or path.startswith(literal + "/") for literal in literals):
            return "full", f"{path} is named in a full check", CHECKS
    return "docs-only", f"{len(changed)} changed Markdown file(s) in the documentation trees", LIGHT_CHECKS


# Seconds per check on the Windows runner (the full run on main of 3 November 2026), used only
# to balance the parallel shards; a check not listed counts as DEFAULT_SECONDS.
SECONDS = {
    "tests/test_jumbling_store_j1.py": 831, "tests/test_jumbling_j1_replay.py": 784,
    "tests/test_jumbling_x1_witness.py": 559, "tests/test_jumbling_observations.py": 249,
    "tests/test_jumbling_nu.py": 241, "tests/test_jumbling_straddlers.py": 172,
    "tests/test_jumbling_leads_repair.py": 165, "tests/test_jumbling_leads_covers.py": 150,
    "tests/test_jumbling_leads_scale.py": 124, "tests/test_jumbling_word_check.py": 104,
    "tests/test_full_reference_replay.py": 74, "tests/test_codex_implement.py": 63,
    "tests/test_workbench.py": 59, "tests/test_jumbling_session.py": 55,
    "tests/test_feature_costs.py": 38, "tests/test_renderer_gate.py": 33,
    "tests/test_turn_probe.py": 31, "tests/test_workflow_windows.py": 28,
    "tests/test_engine_lifecycle.py": 27, "tests/test_native_bridge.py": 19,
    "tests/test_stop_gate.py": 16, "tests/test_checkout_bytes.py": 15,
    "tests/test_print_identity.py": 14, "tests/test_b412_fixture.py": 13,
    "tests/test_log_workflow.py": 13, "tests/test_core.py": 12,
}
DEFAULT_SECONDS = 5
# Parallel Windows jobs for a full run; the longest check (about 14 minutes) bounds the gain.
FULL_SHARDS = 5


def shard_count(mode: str) -> int:
    return FULL_SHARDS if mode == "full" else 1


def shards(checks: list[list[str]], count: int) -> list[list[list[str]]]:
    """Split checks into count shards of similar time (longest first, then the lightest shard);
    each shard keeps the list's order."""
    loads = [0.0] * count
    bins: list[list[list[str]]] = [[] for _ in range(count)]
    for check in sorted(checks, key=lambda c: (-SECONDS.get(c[0], DEFAULT_SECONDS), " ".join(c))):
        index = min(range(count), key=lambda k: (loads[k], k))
        bins[index].append(check)
        loads[index] += SECONDS.get(check[0], DEFAULT_SECONDS)
    order = {" ".join(check): n for n, check in enumerate(checks)}
    return [sorted(shard, key=lambda c: order[" ".join(c)]) for shard in bins]


def parse_shard(text: str) -> tuple[int, int]:
    """Parse "K/N" with 1 <= K <= N."""
    match = re.fullmatch(r"([1-9][0-9]*)/([1-9][0-9]*)", text)
    if not match or int(match.group(1)) > int(match.group(2)):
        raise argparse.ArgumentTypeError(f"expected K/N with 1 <= K <= N, got {text!r}")
    return int(match.group(1)), int(match.group(2))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--list", action="store_true", help="print the checks and exit")
    parser.add_argument("--base", default=os.environ.get("RUN_HEADLESS_BASE", ""),
                        help="pull-request base commit (default: $RUN_HEADLESS_BASE); empty runs all checks")
    parser.add_argument("--plan", action="store_true", help="print the mode and the checks it runs, and exit")
    parser.add_argument("--plan-github", action="store_true",
                        help="print the mode and the shard numbers as GitHub step outputs, and exit")
    parser.add_argument("--shard", type=parse_shard, metavar="K/N", help="run shard K of N of the selected checks")
    parser.add_argument("--expect-mode", choices=("full", "docs-only"),
                        help="fail unless the selected mode is this one (the plan job's mode)")
    args = parser.parse_args()
    if args.list:
        for check in CHECKS:
            print(" ".join(check))
        return 0
    mode, reason, checks = select_checks(changed_files(args.base), full_check_literals())
    print(f"Mode: {mode} ({reason}); {len(checks)} of {len(CHECKS)} checks",
          file=sys.stderr if args.plan_github else sys.stdout, flush=True)
    if args.expect_mode and args.expect_mode != mode:
        print(f"error: the plan chose {args.expect_mode} mode, this run chose {mode}", file=sys.stderr)
        return 2
    if args.plan_github:
        count = shard_count(mode)
        print(f"mode={mode}")
        print(f"count={count}")
        print(f"shards={json.dumps(list(range(1, count + 1)))}")
        return 0
    if args.shard:
        index, count = args.shard
        checks = shards(checks, count)[index - 1]
        print(f"Shard {index} of {count}: {len(checks)} checks", flush=True)
    if args.plan:
        for check in checks:
            print(" ".join(check))
        return 0
    env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1", PYTHONUTF8="1")
    results: list[tuple[str, str, float]] = []
    for check in checks:
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
    width = max((len(name) for name, _, _ in results), default=0)
    print("\nSummary (headless checks only; not Windows/DirectX or performance evidence)")
    for name, outcome, elapsed in results:
        print(f"{name:<{width}}  {outcome:<18} {elapsed:7.1f} s")
    failed = [name for name, outcome, _ in results if outcome != "pass"]
    print(f"\n{len(results) - len(failed)} of {len(results)} checks passed ({mode} mode)")
    if mode == "docs-only":
        print("The other checks did not run; all of them run on main after the merge.")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
