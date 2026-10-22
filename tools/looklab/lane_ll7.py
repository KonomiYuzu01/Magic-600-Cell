"""LL7 layout bindings, using the staged headless project and its directory guard."""
import re


CHECKS = (
    ("ll7-presets", r"LOOKLAB_LL7_PRESETS_PASS presets=7 kinds=6 uniforms=bound checks=1"),
    ("ll7-spin", r"LOOKLAB_LL7_SPIN_PASS global=passed inertia=passed local=passed compare=2 checks=1"),
    ("ll7-layout", r"LOOKLAB_LL7_LAYOUT_PASS kinds=4 classes=5 texture=exact palette=passed checks=1"),
    ("ll7-framing", r"LOOKLAB_LL7_FRAMING_PASS kinds=3 settled=1e-6 global=neutral checks=1"),
)
FAULTS = (
    ("ll7-presets", "ll7-projection-clamped"),
    ("ll7-spin", "ll7-compare-spin-not-copied"),
    ("ll7-layout", "ll7-palette-old-colouring"),
    ("ll7-framing", "ll7-framing-ignores-spin"),
)


def run(godot, stage, temporary, env, common, godot_run, failure):
    for mode, pattern in CHECKS:
        result = godot_run(godot, stage, temporary, env, ["--headless", *common, mode])
        if sum(re.fullmatch(pattern, line) is not None for line in result.stdout.splitlines()) != 1:
            raise failure("LL7 result missing, repeated or wrong counts: " + mode)
    for mode, fault in FAULTS:
        godot_run(godot, stage, temporary, env, ["--headless", *common, mode, fault], expected_failure=fault)
        print("looklab LL7: injected fault rejected: " + fault, flush=True)
    print("looklab LL7: 4 headless checks and 4 injected faults passed", flush=True)
