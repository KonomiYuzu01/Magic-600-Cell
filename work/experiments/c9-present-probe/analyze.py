"""Analyzer for the C9 confirmed-presentation probe (route B). Standard library only.

    python work/experiments/c9-present-probe/analyze.py RUN_DIR      writes RUN_DIR/summary.json and summary.md
    python work/experiments/c9-present-probe/analyze.py --selftest   checks the analyzer on synthetic logs

RUN_DIR holds one folder per harness (control/, godot/), each with the core.jsonl that c9pp.dll wrote,
plus the harness's meta.json and result.json, and the runner's runner.json. Probe evidence for a PHS.52
design proposal only; never product or gate evidence.

Terms
- A frame is a c9pp_frame call that presented (status "presented"); its present count P comes from
  GetLastPresentCount right after Present.
- Confirmed (RM.26): a GetFrameStatistics row of the same swap chain segment reported PresentCount == P
  ("exact", display time = SyncQPCTime). "superseded": only a later count was reported. "unconfirmed": neither.
  A stats row whose PresentCount exceeds its own GetLastPresentCount is "ahead" and is never used.
- A screen sample decodes the frame code (frame id, low 8 bits of the epoch) from a BitBlt of the screen.
- Contradiction: a sample taken MARGIN after an exact confirmation shows an older frame of the same
  epoch, an unknown frame, a frame presented after the sample, or no code although the host UI is visible.
- Seen never confirmed: a sample shows a frame that no statistics row ever confirmed (a false negative of
  the confirmation). Seen before confirmation: the confirming row was polled after the sample (latency only).
"""
import argparse
import bisect
import json
import math
import os
import sys
import tempfile
from pathlib import Path

sys.dont_write_bytecode = True
HARNESSES = ("control", "godot")
MARGIN_MS = 50.0
COLOUR_TOLERANCE = 28
HR_NAMES = {(code - (1 << 32)) if code else 0: name for code, name in (
    (0, "S_OK"), (0x887A000B, "DXGI_ERROR_FRAME_STATISTICS_DISJOINT"), (0x887A0001, "DXGI_ERROR_INVALID_CALL"),
    (0x887A0005, "DXGI_ERROR_DEVICE_REMOVED"), (0x887A0006, "DXGI_ERROR_DEVICE_HUNG"),
    (0x887A0007, "DXGI_ERROR_DEVICE_RESET"), (0x887A0004, "DXGI_ERROR_UNSUPPORTED"),
    (0x887A000A, "DXGI_ERROR_WAS_STILL_DRAWING"), (0x80004005, "E_FAIL"), (0x80070057, "E_INVALIDARG"),
    (0x80004002, "E_NOINTERFACE"), (0x88980800, "DCOMPOSITION_ERROR_WINDOW_ALREADY_COMPOSED"))}
DECODED = ("match", "one_corner")


def hr_name(value):
    return HR_NAMES.get(value, f"0x{value & 0xFFFFFFFF:08X}")


def read_jsonl(path):
    rows, bad = [], 0
    with open(path, "r", encoding="utf-8", errors="replace") as handle:
        for line in handle:
            line = line.strip()
            if not line:
                continue
            try:
                row = json.loads(line)
            except json.JSONDecodeError:
                bad += 1  # a torn last line after a crash
                continue
            if isinstance(row, dict) and "t" in row:
                rows.append(row)
            else:
                bad += 1
    return rows, bad


def read_json(path):
    try:
        with open(path, "r", encoding="utf-8") as handle:
            return json.load(handle)
    except (OSError, ValueError):
        return None


def percentile(values, q):
    if not values:
        return None
    ordered = sorted(values)
    k = (len(ordered) - 1) * q
    lo, hi = math.floor(k), math.ceil(k)
    value = ordered[lo] + (ordered[hi] - ordered[lo]) * (k - lo)
    return round(value, 3)


def near(rgb, target, tolerance=COLOUR_TOLERANCE):
    return isinstance(rgb, list) and len(rgb) == 3 and all(abs(a - b) <= tolerance for a, b in zip(rgb, target))


def analyze_harness(rows, bad_lines, meta, result):
    attach = next((r for r in rows if r["t"] == "attach"), None)
    out = {"rows": len(rows), "bad_lines": bad_lines, "attach": None, "phases": {}, "errors": [],
           "stats_hr": {}, "contradictions": [], "checks": {}}
    colours = {"ui": [230, 120, 20], "frame": [60, 60, 60], "base": [24, 64, 140]}
    for r in rows:
        if r["t"] == "note" and r.get("kind") == "host_colours" and isinstance(r.get("data"), dict):
            colours = {"ui": r["data"].get("ui_rgb", colours["ui"]), "frame": r["data"].get("frame_rgb", colours["frame"]),
                       "base": r["data"].get("base_rgb", colours["base"])}
    if attach is None or not attach.get("ok", False):
        out["attach"] = {"ok": False, "error": attach.get("error") if attach else "no attach row",
                         "target_hr": hr_name(attach["target_hr"]) if attach and "target_hr" in attach else None}
        out["checks"] = {"attached": False}
        out["errors"] = [{"function": r.get("function"), "status": r.get("status"), "text": r.get("text")}
                         for r in rows if r["t"] == "error"][:20]
        return out
    frequency = attach.get("qpc_frequency") or 10_000_000
    margin = MARGIN_MS / 1000.0 * frequency
    out["attach"] = {k: attach.get(k) for k in ("device_source", "device_independent", "swap_effect", "dcomp_device",
                                                 "window_awareness", "window_dpi", "host_same_adapter", "vendor_id",
                                                 "device_id", "cell_px", "max_latency", "duration_us")}
    out["attach"].update(ok=True, factory_hr=hr_name(attach.get("factory_hr", 0)), target_hr=hr_name(attach.get("target_hr", 0)))

    # Pass 1: frames and stats in file order, split into swap chain segments.
    frames, by_id = [], {}
    segment_of_epoch, last_count = {}, {}
    stats = []
    for index, r in enumerate(rows):
        t = r["t"]
        epoch = r.get("epoch", 0)
        if t == "resize" or t == "recover":
            segment_of_epoch[epoch] = segment_of_epoch.get(epoch, 0) + 1
            last_count.pop(epoch, None)
        if t == "frame":
            seg = (epoch, segment_of_epoch.get(epoch, 0))
            frame = {"index": index, "id": r.get("frame_id"), "epoch": epoch, "segment": seg, "phase": r.get("phase"),
                     "status": r.get("status"), "sync": r.get("sync"), "count": r.get("present_count"),
                     "qpc_present": r.get("qpc_present"), "wait_result": r.get("wait_result"), "wait_us": r.get("wait_us"),
                     "confirm": None, "display_qpc": None, "confirm_poll_qpc": None, "first_seen_qpc": None}
            frames.append(frame)
            if frame["id"] is not None:
                by_id[frame["id"]] = frame
        elif t == "poll":
            seg = (epoch, segment_of_epoch.get(epoch, 0))
            name = hr_name(r.get("hr", 0))
            out["stats_hr"][name] = out["stats_hr"].get(name, 0) + 1
            usable = r.get("hr") == 0 and r.get("present_count", 0) <= r.get("last_present_count", 0)
            ahead = r.get("hr") == 0 and r.get("present_count", 0) > r.get("last_present_count", 0)
            stats.append({"index": index, "segment": seg, "count": r.get("present_count", 0), "sync_qpc": r.get("sync_qpc"),
                          "poll_qpc": r.get("poll_qpc"), "usable": usable, "ahead": ahead, "phase": r.get("phase")})
        elif t == "error":
            out["errors"].append({"function": r.get("function"), "status": r.get("status"), "text": r.get("text"),
                                  "phase": r.get("phase")})

    # Pass 2: confirmation by the first usable statistics row of the same segment at or past the count.
    usable = {}
    for st in stats:
        if st["usable"]:
            usable.setdefault(st["segment"], []).append(st)
    keys = {seg: [st["index"] for st in group] for seg, group in usable.items()}
    for frame in frames:
        if frame["status"] != "presented" or frame["count"] is None:
            continue
        candidates = usable.get(frame["segment"], [])
        for st in candidates[bisect.bisect_right(keys.get(frame["segment"], []), frame["index"]):]:
            if st["count"] == frame["count"]:
                frame["confirm"], frame["display_qpc"] = "exact", st["sync_qpc"]
            elif st["count"] > frame["count"]:
                frame["confirm"], frame["display_qpc"] = "superseded", st["poll_qpc"]
            else:
                continue
            frame["confirm_poll_qpc"] = st["poll_qpc"]
            break
        if frame["confirm"] is None:
            frame["confirm"] = "unconfirmed"

    # Pass 3: samples against confirmations.
    order = {f["id"]: i for i, f in enumerate(frames) if f["id"] is not None}
    samples = []
    exact = [f for f in frames if f["confirm"] == "exact" and f["display_qpc"]]
    for r in rows:
        if r["t"] != "sample":
            continue
        T = r.get("qpc_before", 0)
        ui_visible = near(r.get("ui_rgb"), colours["ui"])
        sample = {"phase": r.get("phase"), "status": r.get("status"), "ui_visible": ui_visible,
                  "bg_is_base": near(r.get("bg_rgb"), colours["base"]), "dpi_consistent": r.get("dpi_consistent"),
                  "foreground": r.get("foreground"), "center_is_host": r.get("center_is_host"), "lag_frames": None,
                  "lag_ms": None, "issue": None}
        latest = None  # the newest frame of this epoch confirmed exactly at least MARGIN before the sample
        for f in exact:
            if f["epoch"] == r.get("epoch") and f["display_qpc"] <= T - margin:
                if latest is None or order[f["id"]] > order[latest["id"]]:
                    latest = f
        if r.get("status") in DECODED:
            shown = by_id.get(r.get("frame_id"))
            if shown is None or shown["status"] != "presented" or (shown["epoch"] & 255) != r.get("code_epoch"):
                sample["issue"] = "unknown_frame"
            elif shown["qpc_present"] and shown["qpc_present"] > r.get("qpc_after", T):
                sample["issue"] = "from_future"
            else:
                if shown["first_seen_qpc"] is None:
                    shown["first_seen_qpc"] = T
                if latest is not None and order[shown["id"]] < order[latest["id"]]:
                    sample["issue"] = "older_than_confirmed"
                elif shown["confirm"] == "unconfirmed":
                    sample["issue"] = "seen_never_confirmed"
                elif shown["confirm_poll_qpc"] and shown["confirm_poll_qpc"] > T:
                    sample["issue"] = "seen_before_confirmation"
                newest = [f for f in frames if f["status"] == "presented" and f["epoch"] == shown["epoch"]
                          and f["qpc_present"] and f["qpc_present"] <= T]
                if newest:
                    sample["lag_frames"] = order[newest[-1]["id"]] - order[shown["id"]]
                if shown["qpc_present"]:
                    sample["lag_ms"] = round((T - shown["qpc_present"]) * 1000.0 / frequency, 3)
        elif r.get("status") in ("undecodable", "mismatch") and latest is not None and ui_visible:
            sample["issue"] = "confirmed_not_visible"
        if sample["issue"] in ("unknown_frame", "from_future", "older_than_confirmed", "confirmed_not_visible"):
            out["contradictions"].append({"phase": sample["phase"], "issue": sample["issue"], "qpc": T,
                                          "frame_id": r.get("frame_id"), "status": r.get("status")})
        samples.append(sample)

    # Per phase.
    phase_names = []
    for r in rows:
        if r["t"] == "phase" and r.get("name") not in phase_names:
            phase_names.append(r.get("name"))
    for f in frames + samples:
        if f["phase"] not in phase_names:
            phase_names.append(f["phase"])
    for name in phase_names:
        fs = [f for f in frames if f["phase"] == name]
        presented = [f for f in fs if f["status"] == "presented"]
        latency = [(f["display_qpc"] - f["qpc_present"]) * 1000.0 / frequency for f in presented
                   if f["confirm"] == "exact" and f["qpc_present"] and f["display_qpc"]]
        seen = [(f["first_seen_qpc"] - f["display_qpc"]) * 1000.0 / frequency for f in presented
                if f["confirm"] == "exact" and f["first_seen_qpc"] and f["display_qpc"]]
        ss = [s for s in samples if s["phase"] == name]
        visible = [s for s in ss if s["ui_visible"]]
        st = [s for s in stats if s["phase"] == name]
        out["phases"][name] = {
            "frames": len(fs), "presented": len(presented),
            "present_failed": sum(f["status"] == "present_failed" for f in fs),
            "device_removed": sum(f["status"] == "device_removed" for f in fs),
            "wait_timeouts": sum(f["wait_result"] == 1 for f in fs),
            "confirmed_exact": sum(f["confirm"] == "exact" for f in presented),
            "superseded": sum(f["confirm"] == "superseded" for f in presented),
            "unconfirmed": sum(f["confirm"] == "unconfirmed" for f in presented),
            "polls": len(st), "polls_usable": sum(s["usable"] for s in st), "polls_ahead": sum(s["ahead"] for s in st),
            "present_to_display_ms": {"median": percentile(latency, 0.5), "p95": percentile(latency, 0.95)},
            "display_to_first_seen_ms": {"median": percentile(seen, 0.5), "p95": percentile(seen, 0.95)},
            "samples": len(ss), "samples_host_ui_visible": len(visible),
            "samples_decoded": sum(s["status"] in DECODED for s in visible),
            "samples_bg_is_base": sum(s["bg_is_base"] for s in visible),
            "samples_capture_failed": sum(s["status"] == "capture_failed" for s in ss),
            "samples_dpi_inconsistent": sum(s["dpi_consistent"] is False for s in ss),
            "seen_never_confirmed": sum(s["issue"] == "seen_never_confirmed" for s in ss),
            "seen_before_confirmation": sum(s["issue"] == "seen_before_confirmation" for s in ss),
            "contradictions": sum(s["issue"] in ("unknown_frame", "from_future", "older_than_confirmed",
                                                 "confirmed_not_visible") for s in ss),
            "lag_frames_max": max((s["lag_frames"] for s in ss if s["lag_frames"] is not None), default=None),
        }

    host = [r for r in rows if r["t"] == "host_status"]
    removal = [r for r in rows if r["t"] == "remove_device"]
    recover = [r for r in rows if r["t"] == "recover"]
    notes = [r for r in rows if r["t"] == "note" and r.get("kind") in ("loss_skipped", "frame_after_removal", "harness_stop")]
    out["loss"] = {
        "removal": [{"refused": r.get("refused"), "reason": hr_name(r["reason"]) if "reason" in r else None,
                     "device_independent": r.get("device_independent")} for r in removal],
        "recover": [{"epoch_after": r.get("epoch_after"), "dcomp_recreated": r.get("dcomp_recreated"),
                     "recreate_target": r.get("recreate_target"), "set_content_hr": hr_name(r.get("set_content_hr", 0)),
                     "commit2_hr": hr_name(r.get("commit2_hr", 0)), "device_independent": r.get("device_independent"),
                     "duration_us": r.get("duration_us")} for r in recover],
        "notes": [{"kind": r.get("kind"), "data": r.get("data")} for r in notes],
        "host_status_rows": len(host), "host_status_not_ok": sum(r.get("hr", 0) != 0 for r in host),
    }
    resize = [r for r in rows if r["t"] == "resize"]
    out["resize"] = [{"phase": r.get("phase"), "w": r.get("w"), "h": r.get("h"), "resize_hr": hr_name(r.get("resize_hr", 0)),
                      "commit_hr": hr_name(r.get("commit_hr", 0)), "duration_us": r.get("duration_us")} for r in resize]
    host_frames = [r for r in rows if r["t"] == "note" and r.get("kind") in ("host_frame", "godot_frame")]
    out["host_frames"] = len(host_frames)
    out["result"] = result
    out["meta"] = meta

    steady = out["phases"].get("steady", {})
    hold = out["phases"].get("hold", {})
    recovered = out["phases"].get("recovered", {})
    presented = steady.get("presented", 0)
    confirmed = steady.get("confirmed_exact", 0) + steady.get("superseded", 0)
    visible = steady.get("samples_host_ui_visible", 0) + hold.get("samples_host_ui_visible", 0)
    decoded = steady.get("samples_decoded", 0) + hold.get("samples_decoded", 0)
    out["checks"] = {
        "attached": True,
        "device_independent": attach.get("device_independent") == 1,
        "frame_statistics_available": out["stats_hr"].get("S_OK", 0) > 0,
        "steady_confirmed_fraction": round(confirmed / presented, 4) if presented else None,
        "steady_exact_fraction": round(steady.get("confirmed_exact", 0) / presented, 4) if presented else None,
        "host_ui_visible_samples": visible,
        "decoded_fraction_when_ui_visible": round(decoded / visible, 4) if visible else None,
        "contradictions": len(out["contradictions"]),
        "seen_never_confirmed": sum(p.get("seen_never_confirmed", 0) for p in out["phases"].values()),
        "seen_before_confirmation": sum(p.get("seen_before_confirmation", 0) for p in out["phases"].values()),
        "host_unaffected_by_core_removal": out["loss"]["host_status_not_ok"] == 0,
        "removal_done": any(r.get("refused") is False for r in removal),
        "recovered_confirmed": (recovered.get("confirmed_exact", 0) + recovered.get("superseded", 0)) > 0,
        "harness_exit_code": (result or {}).get("exit_code"),
    }
    return out


def analyze_run(run_dir):
    run_dir = Path(run_dir)
    summary = {"run": run_dir.name, "kind": "probe evidence for a PHS.52 design proposal; not product or gate evidence",
               "runner": read_json(run_dir / "runner.json"), "identity": read_json(run_dir / "identity.json"),
               "gate": read_json(run_dir / "gate.json"), "harnesses": {}}
    for name in HARNESSES:
        folder = run_dir / name
        log = folder / "core.jsonl"
        if not log.is_file():
            summary["harnesses"][name] = {"absent": True, "result": read_json(folder / "result.json")}
            continue
        rows, bad = read_jsonl(log)
        summary["harnesses"][name] = analyze_harness(rows, bad, read_json(folder / "meta.json"), read_json(folder / "result.json"))
    return summary


def markdown(summary):
    lines = [f"# C9 present probe run {summary['run']}", "", f"Kind: {summary['kind']}.", ""]
    runner = summary.get("runner") or {}
    if runner:
        lines += [f"Runner exit code {runner.get('exit_code')}; stopped early: {runner.get('stopped_early')}.", ""]
    for name, h in summary["harnesses"].items():
        lines.append(f"## {name}")
        lines.append("")
        if h.get("absent"):
            lines += ["Not run, or no core log.", ""]
            continue
        if not h["checks"].get("attached"):
            lines += [f"Attach failed: {h['attach'].get('error')} (target {h['attach'].get('target_hr')}).", ""]
            continue
        c = h["checks"]
        lines += [f"- Device independent of the host: {c['device_independent']} (source {h['attach']['device_source']}, "
                  f"factory {h['attach']['factory_hr']}); target {h['attach']['target_hr']}.",
                  f"- GetFrameStatistics results: {h['stats_hr']}.",
                  f"- Steady frames confirmed: {c['steady_confirmed_fraction']} (exact {c['steady_exact_fraction']}).",
                  f"- Samples with the host UI visible: {c['host_ui_visible_samples']}; decoded fraction "
                  f"{c['decoded_fraction_when_ui_visible']}.",
                  f"- Contradictions: {c['contradictions']}; seen but never confirmed: {c['seen_never_confirmed']}; "
                  f"seen before confirmation: {c['seen_before_confirmation']}.",
                  f"- Core removal done: {c['removal_done']}; host unaffected: {c['host_unaffected_by_core_removal']}; "
                  f"recovered frames confirmed: {c['recovered_confirmed']}.",
                  f"- Harness exit code: {c['harness_exit_code']}.", "",
                  "| phase | presented | exact | superseded | unconfirmed | p→d median ms | samples (UI visible) | decoded | contradictions |",
                  "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |"]
        for phase, p in h["phases"].items():
            lines.append(f"| {phase} | {p['presented']} | {p['confirmed_exact']} | {p['superseded']} | {p['unconfirmed']} | "
                         f"{p['present_to_display_ms']['median']} | {p['samples']} ({p['samples_host_ui_visible']}) | "
                         f"{p['samples_decoded']} | {p['contradictions']} |")
        lines.append("")
    return "\n".join(lines) + "\n"


def write(run_dir, summary):
    run_dir = Path(run_dir)
    with open(run_dir / "summary.json", "w", encoding="utf-8", newline="\n") as handle:
        json.dump(summary, handle, indent=2)
        handle.write("\n")
    with open(run_dir / "summary.md", "w", encoding="utf-8", newline="\n") as handle:
        handle.write(markdown(summary))


# ------------------------------------------------------------------------------------------------
# Self-test on synthetic logs.
class Synth:
    F = 10_000_000  # QPC ticks per second
    REFRESH = F // 60

    def __init__(self, harness="control"):
        self.rows, self.t, self.epoch, self.phase = [], 1_000_000_000, 1, "init"
        self.next_id, self.count, self.harness = 1, 0, harness
        self.shown = None  # (frame id, epoch) currently on screen

    def emit(self, t, **fields):
        row = {"t": t, "qpc": self.t, "phase": self.phase, "epoch": self.epoch}
        row.update(fields)
        self.rows.append(row)

    def attach(self, ok=True, independent=1):
        if ok:
            self.emit("attach", ok=True, qpc_frequency=self.F, device_source=1, device_independent=independent,
                      factory_hr=0, target_hr=0, swap_effect="flip_discard", dcomp_device="DCompositionCreateDevice3",
                      window_awareness="system", window_dpi=96, host_same_adapter=True, cell_px=24, max_latency=2)
        else:
            self.emit("attach", ok=False, error="CreateTargetForHwnd(hwnd, FALSE) failed: 0x88980800", target_hr=0x88980800 - (1 << 32))
        self.emit("note", kind="host_colours", data={"ui_rgb": [230, 120, 20], "frame_rgb": [60, 60, 60], "base_rgb": [24, 64, 140]})

    def set_phase(self, name):
        self.phase = name
        self.emit("phase", name=name)

    def frame(self, displayed=True, stats=True, stats_count=None):
        fid = self.next_id
        self.next_id += 1
        self.count += 1
        self.t += 1000
        self.emit("frame", frame_id=fid, sync=1, status="presented", wait_result=0, wait_us=100, present_count=self.count,
                  present_hr=0, qpc_present=self.t)
        self.t += self.REFRESH
        if displayed:
            self.shown = (fid, self.epoch)
        if stats:
            count = self.count if stats_count is None else stats_count
            self.emit("poll", hr=0, present_count=count if displayed else self.count - 1, sync_qpc=self.t - 10,
                      poll_qpc=self.t, last_present_count=self.count)
        return fid

    def sample(self, ui=(230, 120, 20), code=True, frame_id=None, epoch=None):
        self.t += 5 * self.REFRESH
        if code and (self.shown or frame_id):
            fid, ep = (frame_id, epoch) if frame_id else self.shown
            self.emit("sample", status="match", frame_id=fid, code_epoch=ep & 255, ui_rgb=list(ui), bg_rgb=[24, 64, 140],
                      dpi_consistent=True, foreground=True, center_is_host=True, qpc_before=self.t, qpc_after=self.t + 100)
        else:
            self.emit("sample", status="undecodable", frame_id=0, code_epoch=0, ui_rgb=list(ui), bg_rgb=[24, 64, 140],
                      dpi_consistent=True, foreground=True, center_is_host=True, qpc_before=self.t, qpc_after=self.t + 100)

    def lines(self, torn=False):
        text = "".join(json.dumps(r, separators=(",", ":")) + "\n" for r in self.rows)
        return text + ('{"t":"frame","frame_id":' if torn else "")


def selftest():
    failures = []

    def expect(condition, what):
        if not condition:
            failures.append(what)

    with tempfile.TemporaryDirectory(prefix="c9pp-analyze-") as tmp:
        run = Path(tmp) / "run"
        # Clean control run: every frame confirmed exactly, every sample decodes the current frame.
        s = Synth()
        s.attach()
        for phase, n in (("warmup", 5), ("steady", 40), ("hold", 6)):
            s.set_phase(phase)
            for _ in range(n):
                s.frame()
                s.sample()
        s.set_phase("resize_a")
        s.emit("resize", x=40, y=40, w=944, h=560, resize_hr=0, commit_hr=0, duration_us=900)
        s.count = 0  # ResizeBuffers may restart the count: a new segment
        s.emit("poll", hr=0, present_count=500, sync_qpc=s.t, poll_qpc=s.t, last_present_count=0)  # ahead: never used
        for _ in range(5):
            s.frame()
            s.sample()
        s.set_phase("loss")
        s.emit("host_status", hr=0)
        s.emit("remove_device", refused=False, reason=-2005270523, device_independent=1)
        s.emit("host_status", hr=0)
        s.epoch = 2
        s.count = 0
        s.emit("recover", epoch_before=1, epoch_after=2, dcomp_recreated=False, recreate_target=False, set_content_hr=0,
               commit2_hr=0, device_independent=1, duration_us=40000)
        s.set_phase("recovered")
        for _ in range(10):
            s.frame()
            s.sample()
        (run / "control").mkdir(parents=True)
        (run / "control" / "core.jsonl").write_text(s.lines(torn=True), encoding="utf-8", newline="\n")
        (run / "control" / "result.json").write_text(json.dumps({"exit_code": 0}), encoding="utf-8")

        # Faulty godot run: statistics fail for one frame, the screen goes stale behind a confirmation,
        # an unknown frame decodes, and the host device reports removal.
        g = Synth("godot")
        g.attach()
        g.set_phase("steady")
        for _ in range(10):
            g.frame()
            g.sample()
        g.frame(displayed=True, stats=False)        # confirmed only by the next frame's row, after this sample
        g.sample()                                   # seen_before_confirmation
        stale = g.shown
        g.frame()                                   # confirmed exactly ...
        g.shown = stale                              # ... yet the screen still shows the previous frame
        g.sample()                                   # older_than_confirmed
        g.sample(frame_id=999_999, epoch=1)          # unknown_frame
        g.sample(code=False)                         # confirmed_not_visible (UI visible, no code)
        g.sample(code=False, ui=(10, 10, 10))        # covered window: not a contradiction
        g.frame(stats=False)                         # no statistics row ever follows
        g.sample()                                   # seen_never_confirmed
        g.set_phase("loss")
        g.emit("host_status", hr=-2005270523)
        (run / "godot").mkdir(parents=True)
        (run / "godot" / "core.jsonl").write_text(g.lines(), encoding="utf-8", newline="\n")
        (run / "godot" / "result.json").write_text(json.dumps({"exit_code": 3}), encoding="utf-8")
        (run / "runner.json").write_text(json.dumps({"exit_code": 3, "stopped_early": True}), encoding="utf-8")

        summary = analyze_run(run)
        write(run, summary)
        c = summary["harnesses"]["control"]
        expect(c["bad_lines"] == 1, "torn last line counted")
        expect(c["checks"]["steady_exact_fraction"] == 1.0, "clean steady frames all exact")
        expect(c["checks"]["decoded_fraction_when_ui_visible"] == 1.0, "clean samples all decoded")
        expect(c["checks"]["contradictions"] == 0, f"clean run has no contradictions: {c['contradictions'][:3]}")
        expect(c["checks"]["seen_never_confirmed"] == 0 and c["checks"]["seen_before_confirmation"] == 0,
               "clean run sees only confirmed frames")
        expect(c["phases"]["resize_a"]["confirmed_exact"] == 5 and c["phases"]["resize_a"]["polls_ahead"] == 1,
               "resize segment restarts counts and ignores an ahead row")
        expect(c["checks"]["removal_done"] and c["checks"]["host_unaffected_by_core_removal"], "loss bookkeeping")
        expect(c["checks"]["recovered_confirmed"] and c["phases"]["recovered"]["confirmed_exact"] == 10, "recovered epoch")
        expect(abs(c["phases"]["steady"]["present_to_display_ms"]["median"] - (Synth.REFRESH - 10) * 1000 / Synth.F) < 0.01,
               "present-to-display latency")
        expect(c["stats_hr"] == {"S_OK": 52 + 5 + 10}, f"stats hr histogram {c['stats_hr']}")
        gh = summary["harnesses"]["godot"]
        issues = sorted(x["issue"] for x in gh["contradictions"])
        expect(issues == ["confirmed_not_visible", "older_than_confirmed", "unknown_frame"], f"contradictions {issues}")
        expect(gh["checks"]["seen_never_confirmed"] == 1, "seen never confirmed")
        expect(gh["checks"]["seen_before_confirmation"] == 1, "seen before confirmation")
        expect(gh["phases"]["steady"]["unconfirmed"] == 1 and gh["phases"]["steady"]["superseded"] == 1,
               "a frame without its own row is superseded by the next; the last one stays unconfirmed")
        expect(not gh["checks"]["host_unaffected_by_core_removal"], "host removal reported")
        expect(gh["checks"]["harness_exit_code"] == 3, "exit code carried")
        md = (run / "summary.md").read_text(encoding="utf-8")
        expect("## control" in md and "## godot" in md and "| steady |" in md, "markdown tables")
        # A failed attach.
        a = Synth()
        a.attach(ok=False)
        rows, bad = read_jsonl_text(a.lines())
        failed = analyze_harness(rows, bad, None, None)
        expect(failed["checks"] == {"attached": False} and failed["attach"]["target_hr"] == "DCOMPOSITION_ERROR_WINDOW_ALREADY_COMPOSED",
               "attach failure")
        expect(hr_name(0x887A000B - (1 << 32)) == "DXGI_ERROR_FRAME_STATISTICS_DISJOINT"
               and hr_name(-2005270523) == "DXGI_ERROR_DEVICE_REMOVED" and hr_name(-1) == "0xFFFFFFFF", "hr names")
        expect(percentile([1, 2, 3, 4], 0.5) == 2.5 and percentile([], 0.5) is None, "percentile")

    for failure in failures:
        print("FAIL", failure)
    print(f"analyze.py --selftest: {'FAIL' if failures else 'PASS'} ({len(failures)} failed)")
    return 1 if failures else 0


def read_jsonl_text(text):
    with tempfile.NamedTemporaryFile("w", suffix=".jsonl", delete=False, encoding="utf-8", newline="\n") as handle:
        handle.write(text)
        name = handle.name
    try:
        return read_jsonl(name)
    finally:
        os.unlink(name)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("run_dir", nargs="?", type=Path)
    parser.add_argument("--selftest", action="store_true")
    args = parser.parse_args(argv)
    if args.selftest:
        return selftest()
    if args.run_dir is None or not args.run_dir.is_dir():
        parser.error("RUN_DIR is required and must be a directory")
    summary = analyze_run(args.run_dir)
    write(args.run_dir, summary)
    print(f"wrote {args.run_dir / 'summary.json'} and summary.md")
    return 0


if __name__ == "__main__":
    sys.exit(main())
