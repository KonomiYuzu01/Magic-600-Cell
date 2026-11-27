"""ADR-D02 B19/B24: the gate row reader (tools/perf/d02/gate_row.py), the repeat block of d02_run.py and gate_d02.py's
pure parts, on synthetic event logs. Headless; nothing launches."""
from __future__ import annotations

import sys

sys.dont_write_bytecode = True

import json
import re
import unittest
from pathlib import Path
from unittest.mock import patch

from test_d02_drawing import workspace_fixture

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools" / "perf" / "d02"))
sys.path.insert(0, str(ROOT / "renderer" / "shell"))
import gate_row  # noqa: E402

FREQ = 10_000_000
LINE = re.compile(r"^D02_GATE row=G\d+ panes=\d+ view_vertex_evals=(\d+|not_measured) fps_avg=\d+\.\d\d "
                  r"p99_ms=\d+\.\d\d vram_gb=(\d+\.\d\d|not_measured) pass=(true|false)$")


def events(intervals_ms: list[float], panes: int = 2, evals: int | None = 1000, start_qpc: int = 1_000_000) -> list[dict]:
    """A run row, then a frame row and a presented row per frame, the frames presented at the given intervals."""
    rows = [{"t": "run", "qpc_frequency": FREQ}]
    qpc = start_qpc
    for index, gap in enumerate([0.0] + intervals_ms):
        qpc += round(gap * FREQ / 1000)
        frame = {"t": "frame", "frame_id": index, "qpc": qpc - 1000}
        if panes > 1:
            frame["panes"] = panes
        if evals is not None:
            frame.update(core_frames_reported=index, core_view_vertex_evals_total=index * evals)
        rows += [frame, {"t": "presented", "frame_id": index, "display_qpc": qpc}]
    return rows


class GateRowTest(unittest.TestCase):
    def test_percentile_nearest_rank(self):
        values = list(range(1, 101))
        self.assertEqual(gate_row.percentile(values, 99), 99)
        self.assertEqual(gate_row.percentile(values, 100), 100)
        self.assertEqual(gate_row.percentile([5.0], 99), 5.0)
        self.assertEqual(gate_row.percentile([1, 2, 3, 40], 99), 40)

    def test_steady_run_passes(self):
        result = gate_row.measure(events([1000 / 60] * 60 * 30), seconds=20, warmup=5)
        self.assertAlmostEqual(result["fps_avg"], 60.0, places=1)
        self.assertAlmostEqual(result["p99_ms"], 1000 / 60, places=2)
        self.assertEqual((result["panes"], result["view_vertex_evals"]), (2, 1000))
        self.assertTrue(result["pass"])
        self.assertIsNone(result["vram_gb"])
        self.assertIn("VRAM not measured", result["pass_basis"])
        self.assertRegex(gate_row.line("G9", result), LINE)

    def test_surface_resizes_in_the_window_are_counted(self):
        # SF5 (27 Nov 2026): the shell's `surface` rows, one per resize of the core's surface; only the window's count.
        rows = events([1000 / 60] * 60 * 30)
        first, last = rows[2]["display_qpc"], rows[-1]["display_qpc"]
        rows += [{"t": "surface", "qpc": first + FREQ, "w": 946, "h": 1093},
                 {"t": "surface", "qpc": last - FREQ, "w": 946, "h": 1116},
                 {"t": "surface", "qpc": last - 2 * FREQ, "w": 946, "h": 1093}]
        result = gate_row.measure(rows, seconds=20, warmup=5)
        self.assertEqual(result["surface_resizes"], 2)
        self.assertEqual(gate_row.measure(events([1000 / 60] * 60 * 30), seconds=20, warmup=5)["surface_resizes"], 0)

    def test_warmup_is_dropped(self):
        # Ten slow seconds at 10 fps, then 30 fast seconds: the last 20 s see only the fast frames.
        rows = events([100.0] * 100 + [1000 / 60] * 60 * 30)
        result = gate_row.measure(rows, seconds=20, warmup=10)
        self.assertLess(result["p99_ms"], 17.0)
        self.assertTrue(result["pass"])
        whole = gate_row.measure(rows, seconds=39, warmup=0)
        self.assertFalse(whole["pass"])

    def test_slow_tail_fails_on_p99(self):
        gaps = ([1000 / 60] * 97 + [50.0] * 3) * 30
        result = gate_row.measure(events(gaps), seconds=40, warmup=5)
        self.assertGreater(result["p99_ms"], gate_row.P99_MAX_MS)
        self.assertFalse(result["pass"])

    def test_missing_fields_refuse(self):
        rows = events([1000 / 60] * 1200)
        broken = [dict(row) for row in rows]
        del next(row for row in broken if row["t"] == "presented")["display_qpc"]
        with self.assertRaises(gate_row.Refused):
            gate_row.measure(broken, seconds=10, warmup=2)
        with self.assertRaises(gate_row.Refused):
            gate_row.measure([row for row in rows if row["t"] != "run"], seconds=10, warmup=2)
        with self.assertRaises(gate_row.Refused):
            gate_row.measure([{**row, "qpc_frequency": None} if row["t"] == "run" else row for row in rows], 10, 2)

    def test_short_run_refuses(self):
        with self.assertRaises(gate_row.Refused):
            gate_row.measure(events([1000 / 60] * 600), seconds=60, warmup=10)

    def test_vram_peak_in_window_and_limit(self):
        rows = events([1000 / 60] * 60 * 30)
        end = max(row["display_qpc"] for row in rows if row["t"] == "presented")
        samples = [{"qpc": end - FREQ * 25, "dedicated_bytes": 9 * gate_row.GIB},  # before the window
                   {"qpc": end - FREQ * 5, "dedicated_bytes": 3 * gate_row.GIB}]
        result = gate_row.measure(rows, seconds=20, warmup=5, vram=samples)
        self.assertAlmostEqual(result["vram_gb"], 3.0)
        self.assertTrue(result["pass"])
        self.assertAlmostEqual(result["vram_gb_whole_run"], 9.0)
        samples.append({"qpc": end - FREQ, "dedicated_bytes": int(7.5 * gate_row.GIB)})
        self.assertFalse(gate_row.measure(rows, seconds=20, warmup=5, vram=samples)["pass"])

    def test_without_counters_evals_not_measured(self):
        result = gate_row.measure(events([1000 / 60] * 1500, panes=1, evals=None), seconds=20, warmup=2)
        self.assertEqual(result["panes"], 1)
        self.assertIsNone(result["view_vertex_evals"])
        self.assertIn("view_vertex_evals=not_measured", gate_row.line("G1", result))

    def test_main_prints_one_line_or_refuses(self):
        with workspace_fixture("d02-gate-test-") as folder:
            path = folder / "events.jsonl"
            path.write_text("".join(json.dumps(r) + "\n" for r in events([1000 / 60] * 1500)), encoding="utf-8")
            said: list[str] = []
            code = gate_row.main(["--events", str(path), "--row", "G2", "--seconds", "20", "--warmup", "2",
                                  "--json", str(folder / "row.json")], out=said.append)
            self.assertEqual((code, len(said)), (0, 1))
            self.assertRegex(said[0], LINE)
            self.assertEqual(json.loads((folder / "row.json").read_text(encoding="utf-8"))["line"], said[0])
            said.clear()
            code = gate_row.main(["--events", str(folder / "none.jsonl"), "--row", "G2"], out=said.append)
            self.assertEqual(code, 2)
            self.assertTrue(said[0].startswith("D02_GATE_REFUSED"))


class GateScriptTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        import d02_run
        import gate_d02
        cls.d02_run, cls.gate_d02 = d02_run, gate_d02

    def test_repeat_unrolls_quietly(self):
        script = {"steps": [{"wait_ms": 4000}, {"repeat": {"steps": [{"chord": "K"}, {"wait_ms": 1000}]}}]}
        out = self.d02_run.expand_repeat(script, 12.5)
        body = out["steps"][1:]
        self.assertEqual(len(body), 26)
        self.assertTrue(all(step.get("quiet") for step in body[:-1]))
        self.assertNotIn("quiet", body[-1])
        self.assertNotIn("quiet", out["steps"][0])
        with self.assertRaises(ValueError):
            self.d02_run.expand_repeat({"steps": [{"repeat": {"steps": [{"chord": "K"}]}}]}, 10)
        with self.assertRaises(ValueError):
            self.d02_run.expand_repeat({"steps": [{"wait_ms": 1}]}, 10)

    def test_every_row_has_a_script_on_a_sample_state(self):
        states = set(self.d02_run.data()["states"])
        self.assertEqual(set(self.gate_d02.ORDER), set(self.gate_d02.ROWS))
        for row, spec in self.gate_d02.ROWS.items():
            name = f"gate-{row.lower()}"
            script = json.loads((self.d02_run.SCRIPTS / f"{name}.json").read_text(encoding="utf-8"))
            self.assertEqual((script["name"], script["state"]), (name, spec["state"]))
            self.assertIn(spec["state"], states)
            self.assertIn(name, self.d02_run.EXPECT)
            self.assertIn(name, self.d02_run.ON_FIXTURE)
            steps = self.d02_run.expand_repeat(script, 70)["steps"]
            self.assertGreaterEqual(sum(s.get("wait_ms", 0) for s in steps if s.get("quiet")), 69_000)
            self.assertNotIn("quiet", steps[-1], f"{name}: no step may follow the repeat block")

    def test_row_limit_includes_warmup_and_deep_close(self):
        with workspace_fixture("d02-gate-test-") as folder:
            for state, actual, expected in ((None, "deep-2", 920), ("deep-1", "deep-1", 920),
                                            ("wj-ia-300", "wj-ia-300", 230)):
                with self.subTest(state=state):
                    with patch.object(self.gate_d02, "launch_run", side_effect=RuntimeError("stop at launch")) as launch:
                        with self.assertRaisesRegex(RuntimeError, "stop at launch"):
                            self.gate_d02.run_row("G1", folder / "g1", 10, 10, state, None, "2560x1600")
                    self.assertEqual(launch.call_args.args[3:5], (actual, None))
                    self.assertEqual(launch.call_args.args[5], expected)

    def test_rows_set_the_local_mode_they_name(self):
        # LF1: G4 names Local's slice and G6 Local's chain carrying the sweeps; LF2: G7 names Local's chain, with no
        # sweep. Each script sets that shell after naming the cap and before its repeat block, and the cap's move
        # (at_cap) keeps the step.
        modes = {row: spec["local"] for row, spec in self.gate_d02.ROWS.items() if "local" in spec}
        self.assertEqual(modes, {"G4": "default", "G6": "3.14", "G7": "3.14"})
        chains = {row for row, spec in self.gate_d02.ROWS.items() if "Local chain" in spec["views"]}
        self.assertEqual({row: modes.get(row) for row in chains}, dict.fromkeys(chains, "3.14"),
                         "every row that names Local's chain sets the shell at its limit")
        for row, shell in modes.items():
            script = json.loads((self.d02_run.SCRIPTS / f"gate-{row.lower()}.json").read_text(encoding="utf-8"))
            for steps in (script["steps"], self.gate_d02.at_cap(script, 7)["steps"]):
                sets = [i for i, s in enumerate(steps) if s.get("command") == "jumble.slice.shell"]
                self.assertEqual([steps[i]["argument"] for i in sets], [shell], row)
                named = next(i for i, s in enumerate(steps) if str(s.get("text", "")).startswith("> jumble.cap.name "))
                block = next(i for i, s in enumerate(steps) if "repeat" in s)
                self.assertTrue(named < sets[0] < block, row)

    def test_dedicated_peak_takes_the_busiest_adapter(self):
        items = [("pid_7_luid_0x00000000_0x0000AAAA_phys_0", 100), ("pid_7_luid_0x00000000_0x0000BBBB_phys_0", 900),
                 ("pid_7_luid_0x00000000_0x0000BBBB_phys_1", 50), ("pid_70_luid_0x00000000_0x0000BBBB_phys_0", 5000)]
        self.assertEqual(self.gate_d02.dedicated_peak(items, 7), ("0x00000000_0x0000BBBB", 950))
        self.assertIsNone(self.gate_d02.dedicated_peak(items, 8))

    def test_occlusion_counts_distinct_pieces_per_preset(self):
        dumps = [{"do": "start"}, {"do": "command view.preset.global.ring_sheet"},
                 {"do": "click 0.3 0.3"}, {"do": "wait 250 ms", "pick": {"selection": {"piece": 4, "sticker": 1}}},
                 {"do": "click 0.4 0.3"}, {"do": "wait 250 ms", "pick": {"selection": {"piece": 4, "sticker": 2}}},
                 {"do": "click 0.5 0.3"}, {"do": "wait 250 ms", "pick": {"selection": None}},
                 {"do": "command view.preset.global.hopf_rings"},
                 {"do": "click 0.3 0.3"}, {"do": "wait 250 ms", "pick": {"selection": {"piece": 9, "sticker": 3}}}]
        found = self.gate_d02.count_picks(dumps)
        self.assertEqual((found["ring_sheet"]["clicks"], found["ring_sheet"]["hits"], found["ring_sheet"]["pieces"]), (3, 2, {4}))
        self.assertEqual(found["hopf_rings"]["pieces"], {9})
        script = self.gate_d02.occlusion_script("deep-2", (2, 3), (0.3, 0.25, 0.7, 0.75))
        self.assertEqual(sum(1 for s in script["steps"] if "click" in s), 12)

    def test_sweeping_rows_select_an_admissible_grip_cap(self):
        # B19: a sweeping row selects its grip cap before the repeat block; G1 hides Local there too.
        for row, spec in self.gate_d02.ROWS.items():
            script = json.loads((self.d02_run.SCRIPTS / f"gate-{row.lower()}.json").read_text(encoding="utf-8"))
            commands = [s.get("command") for s in script["steps"][:-1]]
            self.assertEqual("grip.select" in commands, spec["sweeps"], row)
            self.assertEqual("view.local" in commands, row == "G1", row)
            if spec["sweeps"]:
                cap = int(next(s["argument"] for s in script["steps"] if s.get("command") == "grip.select"))
                self.assertIn(cap, self.gate_d02.admissible_caps(spec["state"]), row)
        self.assertEqual(self.gate_d02.sweep_cap("deep-2", 2), 2)
        self.assertEqual(self.gate_d02.sweep_cap("wj-ia-end", 2), 2)
        moved = self.gate_d02.sweep_cap("wj-ia-300", 2)
        self.assertNotEqual(moved, 2)
        self.assertIn(moved, self.gate_d02.admissible_caps("wj-ia-300"))
        self.assertNotIn(0, self.gate_d02.admissible_caps("wj-ia-300"))
        with workspace_fixture("d02-gate-test-") as folder:
            out = folder / "g1"
            script, path = self.gate_d02.prepare("gate-g1", out, 3, "2560x1600", "wj-ia-300", True)
            self.assertTrue(path.is_file())
            self.assertEqual((script["grip_cap"], script["resolution"]), (moved, "2560x1600"))
            self.assertIn({"command": "grip.select", "argument": str(moved)}, script["steps"])
            self.assertIn({"text": f"> jumble.cap.name {moved}"}, script["steps"])

    def test_window_size_and_opening_placement(self):
        self.assertEqual(self.gate_d02.window_size("1280x720")["source"], "--resolution")
        native = self.gate_d02.window_size(None, read=lambda: "2560x1600")
        self.assertEqual(native["resolution"], "2560x1600")
        self.assertIn("native", native["source"])
        # The dumps are the record (the shell writes no click row to events.jsonl): a click step is inside when its
        # dump reports the opening, its fractions lie in [0, 1], the pick was asked once and a hit names Global.
        frame = {"global_opening": [10, 20, 100, 50]}
        dumps = [{"do": "start", "frame": frame, "pick": {"asked": 0}},
                 {"do": "click 0.05 0.1", "frame": frame, "pick": {"asked": 0}},
                 {"do": "wait 250 ms", "frame": frame, "pick": {"asked": 1, "selection": {"piece": 4, "view": 1}}},
                 {"do": "click 0.5 0.5", "frame": frame, "pick": {"asked": 1}},
                 {"do": "wait 250 ms", "frame": frame, "pick": {"asked": 2, "selection": None}},
                 {"do": "click 1.5 0.5", "frame": frame, "pick": {"asked": 2}},
                 {"do": "wait 250 ms", "frame": frame, "pick": {"asked": 3}},
                 {"do": "click 0.5 0.5", "frame": frame, "pick": {"asked": 3}},
                 {"do": "wait 250 ms", "frame": frame, "pick": {"asked": 3}},
                 {"do": "click 0.5 0.5", "frame": frame, "pick": {"asked": 3}},
                 {"do": "wait 250 ms", "frame": frame, "pick": {"asked": 4, "selection": {"piece": 7, "view": 2}}},
                 {"do": "click 0.5 0.5 in local", "frame": frame, "pick": {"asked": 4}},
                 {"do": "wait 250 ms", "frame": {"global_opening": [10, 20, 90, 50]}, "pick": {"asked": 5}}]
        placed = self.gate_d02.opening_clicks(dumps)
        self.assertEqual((placed["global_opening"], placed["clicks"], placed["inside"], placed["moved"]),
                         ([10, 20, 90, 50], 5, 2, True))
        self.assertEqual(self.gate_d02.opening_clicks([{"do": "click 0.5 0.5"}])["global_opening"], None)

    def test_occlusion_rescore_keeps_the_first_record(self):
        frame = {"global_opening": [10, 20, 100, 50]}
        dumps = [{"do": "start", "frame": frame, "pick": {"asked": 0}}]
        asked = 0
        for preset in self.gate_d02.OCCLUSION_PRESETS:
            dumps.append({"do": f"command view.preset.global.{preset}", "frame": frame, "pick": {"asked": asked}})
            for _ in range(2):
                dumps.append({"do": "click 0.25 0.5", "frame": frame, "pick": {"asked": asked}})
                asked += 1
                dumps.append({"do": "wait 250 ms", "frame": frame,
                              "pick": {"asked": asked, "selection": {"piece": asked, "view": 1}}})
        first = {"format": self.gate_d02.FORMAT, "state": "deep-2", "region": [0, 0, 1, 1], "window": {"resolution": "2x2"},
                 "foreground": True, "run": {"code": 0, "mismatches": []}, "problems": ["0 of 0 clicks (of 4) landed"],
                 "occlusion": [f"D02_OCCLUSION preset={p} grid=2x1 clicks=0" for p in self.gate_d02.OCCLUSION_PRESETS]}
        with workspace_fixture("d02-gate-test-") as folder:
            out = folder / "occlusion"
            out.mkdir()
            (out / self.d02_run.SCREEN).write_text("".join(json.dumps(d) + "\n" for d in dumps), encoding="utf-8")
            (out / self.gate_d02.RECORD).write_text(json.dumps(first), encoding="utf-8")
            shown = []
            self.assertEqual(self.gate_d02.rescore_occlusion(out, (2, 1), shown.append), 0)
            again = json.loads((out / self.gate_d02.RESCORED).read_text(encoding="utf-8"))
            self.assertEqual(json.loads((out / self.gate_d02.RECORD).read_text(encoding="utf-8")), first)
            self.assertEqual((again["problems"], again["placement"]["clicks"], again["placement"]["inside"]), ([], 4, 4))
            self.assertEqual(again["rescored"]["from"], self.gate_d02.RECORD)
            self.assertTrue(all("clicks=2 hits=2 distinct_pieces=2" in line for line in shown[:2]))
            self.assertEqual(self.gate_d02.rescore_occlusion(out, (24, 14), shown.append), 2)
            self.assertEqual(self.gate_d02.main(["--list", "--rescore", str(out)]), 2)

    def test_foreground_runs_drop_only_the_foreground_checks(self):
        dumps = [{"step": 1, "window": {"topmost": False, "foreground": True}}]
        self.assertEqual(len(self.d02_run.window_problems(dumps)), 1)
        self.assertEqual(self.d02_run.window_problems(dumps, foreground=True), [])
        dumps[0]["window"]["topmost"] = True
        self.assertEqual(len(self.d02_run.window_problems(dumps, foreground=True)), 1)


if __name__ == "__main__":
    unittest.main()
