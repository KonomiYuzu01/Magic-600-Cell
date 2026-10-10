"""LL6 linked views, using the staged headless project and its directory guard."""
import re


CHECKS = (
    ("ll6-views", r"LOOKLAB_LL6_VIEWS_PASS layouts=4 sizes=2 modes=2 placements=2 arrangements=32 hidden=2 checks=1"),
    ("ll6-link", r"LOOKLAB_LL6_LINK_PASS drags=2 steps=9 boundaries=2 checks=1"),
    ("ll6-colour", r"LOOKLAB_LL6_COLOUR_PASS modes=4 states=12 late=passed checks=1"),
    ("ll6-framing", r"LOOKLAB_LL6_FRAMING_PASS settled=1e-6 steps=120 caps=passed global=neutral checks=1"),
)
FAULTS = (
    ("ll6-views", "ll6-local-hidden"),
    ("ll6-views", "ll6-panel-covers-local"),
    ("ll6-link", "ll6-local-drag-ignored"),
    ("ll6-link", "ll6-local-clock"),
    ("ll6-colour", "ll6-global-only-cvd"),
    ("ll6-framing", "ll6-neutral-local-frame"),
)


def run(godot, stage, temporary, env, common, godot_run, failure):
    for mode, pattern in CHECKS:
        result = godot_run(godot, stage, temporary, env, ["--headless", *common, mode])
        if sum(re.fullmatch(pattern, line) is not None for line in result.stdout.splitlines()) != 1:
            raise failure("LL6 result missing, repeated or wrong counts: " + mode)
    for mode, fault in FAULTS:
        godot_run(godot, stage, temporary, env, ["--headless", *common, mode, fault], expected_failure=fault)
        print("looklab LL6: injected fault rejected: " + fault, flush=True)
    print("looklab LL6: 4 headless checks and 6 injected faults passed", flush=True)
