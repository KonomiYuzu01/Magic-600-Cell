"""LL4 headless checks, using the staged project's existing directory guard."""
import re


def run(godot, stage, temporary, env, common, godot_run, failure):
    checks = (
        ("ll4-metrics", r"LOOKLAB_LL4_METRICS_PASS steps=14 checks=1"),
        ("ll4-refusal", r"LOOKLAB_LL4_REFUSAL_PASS command=target\.next\.activate context=any checks=1"),
        # 215 commands: H-08 with LP5's research delta (docs/progress/1.0/command-rows-research.json) and LP4's
        # document rows (docs/progress/1.0/command-rows-lp4.json, ADR-D01.84), all hidden in context `any`, so
        # placements stay 348.
        ("ll4-greybox", r"LOOKLAB_LL4_GREYBOX_PASS layouts=4 contexts=32 commands=215 placements=348 checks=1"),
    )
    for mode, pattern in checks:
        result = godot_run(godot, stage, temporary, env, ["--headless", *common, mode])
        if sum(re.fullmatch(pattern, line) is not None for line in result.stdout.splitlines()) != 1:
            raise failure("LL4 result missing, repeated or wrong counts: " + mode)
    for mode, fault in (("ll4-refusal", "skip-flow-refusal"), ("ll4-greybox", "skip-catalogue-filter")):
        godot_run(godot, stage, temporary, env, ["--headless", *common, mode, fault], expected_failure=fault)
        print("looklab LL4: injected fault rejected: " + fault, flush=True)
    print("looklab LL4: 3 headless checks and 2 injected faults passed", flush=True)
