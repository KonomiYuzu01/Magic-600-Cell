"""Run one Python leg in a selected interpreter, with no writes outside build/."""
import sys
sys.dont_write_bytecode = True

import argparse
import json
import platform

from common import HERE, ROOT, RESULT_HEADER, bits, json_bytes, read_requests, result_line
from pure_input_map import input_quaternion, nearest_parameter


def reference_module():
    sys.path.insert(0, str(ROOT / "research" / "jumbling"))
    from sim import twists
    return twists


def run(requests, reference=False, fused=True):
    twists = reference_module() if reference else None
    lines = [RESULT_HEADER]
    control = {"requests": len(requests), "pure_target_differences": 0,
               "naive_target_differences": 0, "pure_parameter_differences": 0}
    for name, axis, angle, n, d in requests:
        if reference:
            _, target = twists._input_quaternion(axis, angle)
            nums, den, report = twists.nearest_parameter(axis, angle, d, n)
            choice = (den, tuple(nums), report["family"])
            pure_target, pure_choice = nearest_parameter(axis, angle, d, n)
            control["pure_target_differences"] += list(map(bits, target)) != list(map(bits, pure_target))
            control["pure_parameter_differences"] += choice != pure_choice
            _, naive = input_quaternion(axis, angle, fused=False)
            control["naive_target_differences"] += list(map(bits, target)) != list(map(bits, naive))
        else:
            target, choice = nearest_parameter(axis, angle, d, n, fused)
        lines.append(result_line(name, target, choice))
    return "".join(lines).encode(), control


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("mode", choices=("reference", "pure"))
    parser.add_argument("requests")
    parser.add_argument("output")
    args = parser.parse_args()
    inputs = HERE / args.requests
    output = (HERE / args.output).resolve()
    assert inputs.resolve().parent == HERE and output.parent == HERE / "build"
    data, control = run(read_requests(inputs), reference=args.mode == "reference")
    output.write_bytes(data)
    metadata = {"python": platform.python_version(), "platform": sys.platform,
                "implementation": platform.python_implementation(), "control": control}
    if args.mode == "reference":
        import numpy
        metadata["numpy"] = numpy.__version__
    output.with_suffix(".json").write_bytes(json_bytes(metadata))
    print(json.dumps({"mode": args.mode, **metadata}, sort_keys=True), flush=True)
    if args.mode == "reference":
        assert control["pure_target_differences"] == 0, "pure target control failed"
        assert control["pure_parameter_differences"] == 0, "pure chooser control failed"


if __name__ == "__main__":
    main()
