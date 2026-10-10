"""LL5 steps use the existing staged runner, directory guard and fault checks."""
import re


def _require(run, pattern, failure):
    if len(re.findall(pattern, run.stdout, re.MULTILINE)) != 1:
        raise failure("LL5 result missing, repeated or wrong: " + pattern)


def headless(godot, stage, temporary, env, common, runner, failure):
    for mode, pattern, fault in (
        ("ll5-cvd", r"^LOOKLAB_LL5_CVD_PASS modes=4 samples=52 backgrounds=5 tolerance=2e-6-linear$", "ll5-disable-cvd"),
        ("ll5-cost", r"^LOOKLAB_LL5_COST_PASS nulls=10 measured=1 source=shown live=passed$", "ll5-disable-null-cost"),
        ("ll5-palette", r"^LOOKLAB_LL5_PALETTE_PASS modes=4 pairs=[1-9][0-9]* same_ring=[1-9][0-9]* gamut_failures=3 thresholds=caller$", None),
    ):
        result = runner(godot, stage, temporary, env, ["--headless", *common, mode])
        _require(result, pattern, failure)
        if fault:
            runner(godot, stage, temporary, env, ["--headless", *common, mode, fault], expected_failure=fault)


def graphics(godot, stage, temporary, env, common, runner, failure):
    result = runner(godot, stage, temporary, env, [*common, "ll5-cvd-gpu"], graphics=True)
    _require(result, r"^LOOKLAB_LL5_CVD_GPU_PASS modes=4 samples=52 target=RGBA32F tolerance=2e-6-linear$", failure)
    runner(godot, stage, temporary, env, [*common, "ll5-cvd-gpu", "ll5-wrong-matrix"],
           graphics=True, expected_failure="ll5-wrong-matrix")
