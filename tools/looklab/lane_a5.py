"""A5 design-system build: headless checks on the staged project and its directory guard."""
import re


CHECKS = (
    ("a5-live", r"LOOKLAB_A5_LIVE_PASS reloads=1 refusals=1 restart=kept parts=13 checks=7"),
    ("a5-sweep", r"LOOKLAB_A5_SWEEP_PASS commands=[1-9]\d* clicks=[1-9]\d* errors=0"),
)
FAULTS = (
    ("a5-live", "a5-view-dropped"),
    ("a5-live", "a5-bad-data-shown"),
    ("a5-live", "a5-view-not-written"),
    ("a5-live", "a5-pair-hidden"),
    ("a5-live", "a5-knob-ignored"),
    ("a5-live", "a5-unavailable-ran"),
    ("a5-live", "a5-settle-ignored"),
    ("a5-sweep", "a5-missing-token"),
    ("a5-sweep", "a5-listener-throws"),
)


def run(godot, stage, temporary, env, common, godot_run, failure):
    for mode, pattern in CHECKS:
        result = godot_run(godot, stage, temporary, env, ["--headless", *common, mode])
        if sum(re.fullmatch(pattern, line) is not None for line in result.stdout.splitlines()) != 1:
            raise failure("A5 result missing, repeated or wrong counts: " + mode)
    for mode, fault in FAULTS:
        godot_run(godot, stage, temporary, env, ["--headless", *common, mode, fault], expected_failure=fault)
        print("looklab A5: injected fault rejected: " + fault, flush=True)
    print(f"looklab A5: {len(CHECKS)} headless checks and {len(FAULTS)} injected faults passed", flush=True)
