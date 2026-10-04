"""Owner-machine control for R10: Godot's own output with a blank project.

R10 is the only matrix row with a separate render thread, and its Godot output has an ERROR line
at shutdown. This control runs a blank project (no harness, no DLL) with R10's engine flags under
both render-thread models and counts Godot's own ERROR:/WARNING: lines, so that the line can be
attributed to Godot or to the interop. Raw output stays private; --write-summary writes counts
only to results/sa2-blank-control-summary.json. Run it like the matrix: on an idle machine with
mains power and the intended discrete GPU.
"""

import argparse
import datetime as dt
import os
import re
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
from run_smoke import BUILD_ROOT, HERE, PRIVATE_ROOT, commands, locate_godot, matrix, run_process, source_identity, write_json
from smoke_summary import scan_public

FORMAT = "magic600-sa2-blank-control-v1"
FINALIZE = "ERROR: This function (finalize) can only be called from the render thread."
MODES = ("safe", "separate")
FRAMES = 600
ADAPTER_LINE = re.compile(r"D3D12 \S+ - Forward\+ - Using Device #\d+: .+? - (.+)")
PROJECT_FILES = {
    "project.godot": ('config_version=5\n\n[application]\n\nconfig/name="SA2 blank control"\n'
                      'run/main_scene="res://main.tscn"\nconfig/features=PackedStringArray("4.7")\n\n'
                      '[rendering]\n\nrendering_device/driver.windows="d3d12"\n'),
    "main.tscn": '[gd_scene format=3]\n\n[node name="Main" type="Node2D"]\n',
}


def command(godot, project, mode, log):
    """R10's engine flags with the given render-thread model, without harness arguments, quitting after FRAMES frames."""
    row = dict(next(row for row in matrix() if row["id"] == "R10"), render_thread=mode)
    full = commands(row, godot, project, "<unused>", "<unused>", log)
    return full[:full.index("--")] + ["--quit-after", str(FRAMES)]


def adapter_name(stdout):
    """The device name from Godot's one `Using Device` line, or None."""
    names = [match.group(1).strip() for match in map(ADAPTER_LINE.fullmatch, stdout.splitlines()) if match]
    return names[0] if len(names) == 1 else None


def run_counts(mode, repeat, run, stdout, stderr):
    lines = (stdout + "\n" + stderr).splitlines()
    return {"mode": mode, "repeat": repeat, "exit_code": run["exit_code"], "timed_out": run["timed_out"],
            "adapter_name": adapter_name(stdout),
            "output_line_counts": {key: run["output_line_counts"][key] for key in ("error", "warning")},
            "finalize_lines": sum(line.strip() == FINALIZE for line in lines)}


def build_summary(godot_version, source, runs, current_user=None):
    """Counts only; scan_public refuses private path, environment or user-name text before anything is written."""
    totals = {mode: {"runs": sum(r["mode"] == mode for r in runs),
                     "runs_with_finalize_line": sum(r["mode"] == mode and r["finalize_lines"] > 0 for r in runs),
                     "error_lines": sum(r["output_line_counts"]["error"] for r in runs if r["mode"] == mode)}
              for mode in MODES}
    summary = {"format": FORMAT, "godot_version": godot_version,
               "source": {key: source[key] for key in ("head", "sha256", "matches_head")},
               "command": command("<godot>", "<project>", "<mode>", "<log>"), "frames": FRAMES,
               "finalize_line": FINALIZE, "runs": runs, "totals": totals}
    scan_public(summary, current_user)
    return summary


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--godot", type=Path)
    parser.add_argument("--expect-adapter", default="NVIDIA GeForce RTX 4070 Laptop GPU")
    parser.add_argument("--repeats", type=int, default=3)
    parser.add_argument("--write-summary", action="store_true")
    args = parser.parse_args(argv)
    if not 1 <= args.repeats <= 10:
        parser.error("--repeats must be 1 to 10")
    if os.name != "nt":
        parser.exit(1, "the blank control requires Windows\n")
    try:
        godot, _, version = locate_godot(args.godot)
        source = source_identity()
        stamp = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
        private = PRIVATE_ROOT / f"control-{stamp}"
        project = BUILD_ROOT / f"control-{stamp}"
        private.mkdir(parents=True)
        project.mkdir(parents=True)
        for name, text in PROJECT_FILES.items():
            (project / name).write_text(text, encoding="utf-8", newline="\n")
        runs = []
        for repeat in range(1, args.repeats + 1):
            for mode in MODES:
                name = f"{mode}-{repeat}"
                run, stdout, stderr = run_process(command(godot, project, mode, private / f"{name}.godot.log"),
                                                  project, os.environ.copy(), 180, private / name)
                runs.append(run_counts(mode, repeat, run, stdout, stderr))
                print(f"{name}: exit={run['exit_code']}; timeout={run['timed_out']}; "
                      f"errors={runs[-1]['output_line_counts']['error']}; finalize={runs[-1]['finalize_lines']}")
        summary = build_summary(version, source, runs)
        write_json(private / "summary.json", summary)
        wrong = [f"{r['mode']}-{r['repeat']}" for r in runs if r["timed_out"] or r["adapter_name"] != args.expect_adapter]
        if wrong:
            print(f"sa2 blank control: timed out or not on the expected adapter: {', '.join(wrong)}", file=sys.stderr)
            return 1
        if args.write_summary:
            write_json(HERE / "results/sa2-blank-control-summary.json", summary)
        print(f"Private outputs: {private}")
        return 0
    except (OSError, ValueError, KeyError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"sa2 blank control: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
