"""Read-only H-08 table checks. No engine, input-device or timing evidence.

Run directly, including without -B: importing the builder creates no bytecode.
Every negative case alters an in-memory copy; no temporary directories or files.
"""
import sys
sys.dont_write_bytecode = True

from collections import Counter
from contextlib import redirect_stderr, redirect_stdout
from copy import deepcopy
import io
from pathlib import Path
import unittest
from unittest.mock import patch

import build_command_table as builder

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
FIXTURE = ROOT / "docs/architecture/1.0/language/closure/fixtures/jumbling-rows-example.json"


class CommandTableTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.sources = builder.decision_sources()
        cls.join = builder.load(HERE / "command-join.json")
        cls.aliases = builder.load(HERE / "command-aliases.json")
        cls.fixture = builder.load(FIXTURE)
        # H2's historical fixture predates H15's explicit argument schemas.
        # Adapt only this in-memory copy; never refresh the shared fixture.
        cls.fixture["add"][0]["arguments"] = {"revision": "RevisionRef", "token": "UserActionToken"}
        cls.fixture["amend"][0]["set"]["arguments"] = deepcopy(builder.ARGUMENTS["macro.add"])
        cls.draft = builder.build_table(draft_without_jumbling=True)
        cls.delta = builder.load(HERE / "command-rows-jumbling.json")
        cls.language = builder.load(HERE / "command-rows-language.json")
        cls.table = builder.build_table(delta=[cls.delta, cls.language])
        cls.by_id = {r["id"]: r for r in cls.table["commands"]}

    def build(self, **overrides):
        inputs = dict(join=self.join, aliases=self.aliases, sources=self.sources, draft_without_jumbling=True)
        inputs.update(overrides)
        return builder.build_table(**inputs)

    def altered_row(self, name, **changes):
        rows = deepcopy(builder.rows)
        next(r for r in rows if r["id"] == name).update(changes)
        return rows

    def test_stored_table_matches_mode_and_lf(self):
        stored = (HERE / "command-table.json").read_bytes()
        self.assertEqual(stored, builder.encoded(self.table))
        self.assertTrue(self.table["final"])
        self.assertNotIn(b"\r", stored)
        self.assertFalse(self.draft["final"])
        self.assertNotIn("example.jumble.row", self.by_id)
        self.assertFalse(any(r["id"].startswith("jumble.") for r in self.draft["commands"]))

    def test_census_and_coverage_per_decision(self):
        self.assertEqual(len(self.sources), 556)
        self.assertEqual(Counter(s["disposition"] for s in self.sources.values()),
                         {"keep": 52, "redesign": 486, "delete": 12, "automate": 6})
        user = [s for s in self.sources.values() if s["user_entry"] and s["disposition"] in {"keep", "redesign"}]
        self.assertEqual(len(user), 377)
        self.assertEqual(sum(bool(s["legacy_ids"]) for s in user), 268)
        self.assertEqual(sum(not s["legacy_ids"] for s in user), 109)
        self.assertEqual(len({x for s in user for x in s["legacy_ids"]}), 270)
        self.assertEqual({d["decision"] for d in self.join["decisions"]}, self.sources.keys())

    def test_removed_mapped_split_is_not_covered_by_its_siblings(self):
        join = deepcopy(self.join)
        join["decisions"] = [d for d in join["decisions"] if d["decision"] != "U4-069/3"]
        self.assertTrue(any(d["decision"] == "U4-069/1" for d in join["decisions"]))
        with self.assertRaisesRegex(builder.TableError, "missing decisions.*U4-069/3"):
            self.build(join=join)

    def test_removed_command_and_duplicate_decision_refused(self):
        with self.assertRaisesRegex(builder.TableError, "mapped command"):
            self.build(base_rows=[r for r in builder.rows if r["id"] != "macro.add"])
        join = deepcopy(self.join)
        join["decisions"].append(deepcopy(join["decisions"][0]))
        with self.assertRaisesRegex(builder.TableError, "duplicate or unknown decision"):
            self.build(join=join)

    def test_signed_sources_cannot_be_reclassified(self):
        join = deepcopy(self.join)
        join["decisions"][0]["disposition"] = "delete"
        with self.assertRaisesRegex(builder.TableError, "differs from signed source"):
            self.build(join=join)

    def test_automate_and_delete_get_no_command_even_with_shared_ids(self):
        decisions = {d["decision"]: d for d in self.join["decisions"]}
        for d in decisions.values():
            if d["disposition"] in {"delete", "automate"}:
                self.assertEqual(d["commands"], [])
                self.assertEqual(d["outcome"], "none")
                self.assertTrue(d["reason"])
        self.assertIn("macro.library.check", decisions["U6-031/2"]["commands"])
        self.assertEqual(decisions["U3-085"]["commands"], [])
        self.assertNotIn("macro.library.check", decisions["U6-031/1"]["commands"])
        self.assertEqual(decisions["U6-037/2"]["commands"], [])
        join = deepcopy(self.join)
        entry = next(d for d in join["decisions"] if d["decision"] == "U3-085")
        entry.update(commands=["macro.library.check"], outcome="mapped")
        with self.assertRaisesRegex(builder.TableError, "automate gets no command"):
            self.build(join=join)

    def test_many_to_many_dispositions_are_aggregated(self):
        self.assertEqual(self.by_id["session.undo"]["disposition"], ["keep", "redesign"])
        for row in self.table["commands"]:
            expected = sorted({d["disposition"] for d in self.join["decisions"] if row["id"] in d["commands"]})
            self.assertEqual(row["disposition"], expected, row["id"])

    def test_alias_digest_and_coverage_are_checked_independently(self):
        aliases = deepcopy(self.aliases)
        aliases["rows"][0]["cases"][0]["reason"] += " changed"
        with self.assertRaisesRegex(builder.TableError, "digest mismatch"):
            self.build(aliases=aliases)
        aliases = deepcopy(self.aliases)
        aliases["rows"].pop()
        aliases["content_digest"] = builder.digest({k: v for k, v in aliases.items() if k != "content_digest"})
        with self.assertRaisesRegex(builder.TableError, "missing legacy aliases"):
            self.build(aliases=aliases)

    def test_aliases_preserve_unresolved_banks_and_deleted_planner(self):
        cases = {c["legacy_id"].removeprefix("NativeWorkbench.KeyAction:"): c for row in self.aliases["rows"] for c in row["cases"]}
        for name in ("bank", "bank-A", "bank-Functions", "cycles-refresh", "suggest"):
            self.assertIsNone(cases[name]["target"])
            self.assertTrue(cases[name]["reason"])
        self.assertEqual(cases["functions-toggle"]["target"]["command"], "command.find")
        self.assertEqual(cases["frame-toggle"]["target"]["command"], "display.frame.toggle")
        self.assertEqual(cases["detail-toggle"]["target"]["command"], "display.adaptive.toggle")
        undo = cases["undo"]["target"]
        self.assertEqual(undo["command"], "session.undo")
        self.assertNotIn("dispatch_by_action_class", undo)
        self.assertEqual(set(undo["invoke_requires"]), {"hint", "token"})
        self.assertIn("token", undo["invoke_requires"])
        self.assertFalse(any(r.startswith("token:") for r in undo["requires"]))

    def test_every_alias_requires_each_mandatory_argument(self):
        # Remove one binding at a time, including literals and fresh guards.
        # Validate the final schemas so delta-amended consumers are covered.
        builder.validate_aliases(self.aliases, self.sources, self.by_id)
        for group_index, group in enumerate(self.aliases["rows"]):
            for case_index, case in enumerate(group["cases"]):
                target = case["target"]
                if target is None:
                    continue
                for argument, kind in self.by_id[target["command"]]["arguments"].items():
                    if kind.endswith("?"):
                        continue
                    with self.subTest(alias=case["legacy_id"], argument=argument):
                        aliases = deepcopy(self.aliases)
                        changed = aliases["rows"][group_index]["cases"][case_index]["target"]
                        changed["arguments"].pop(argument, None)
                        changed.get("invoke_requires", {}).pop(argument, None)
                        changed["requires"] = [r for r in changed["requires"] if r.partition(":")[0] != argument]
                        aliases["content_digest"] = builder.digest({k: v for k, v in aliases.items() if k != "content_digest"})
                        with self.assertRaisesRegex(builder.TableError, "missing required alias arguments|fresh hint and token"):
                            builder.validate_aliases(aliases, self.sources, self.by_id)

    def test_review_alias_binds_snapshot_fields_and_chunk_limit(self):
        review = next(c["target"] for g in self.aliases["rows"] for c in g["cases"] if c["legacy_id"] == "review")
        self.assertEqual(review["command"], "operation.check")
        self.assertEqual({r.partition(":")[0] for r in review["requires"]}, {"draft", "fields", "max_chunk"})
        self.assertEqual(set(review["invoke_requires"]), {"snapshot"})
        self.assertIn("kernel.snapshot.open(current)", review["invoke_requires"]["snapshot"])
        self.assertIn("LI.11", review["invoke_requires"]["snapshot"])
        # S1-03's original alias must fail even after its digest is refreshed.
        aliases = deepcopy(self.aliases)
        target = next(c["target"] for g in aliases["rows"] for c in g["cases"] if c["legacy_id"] == "review")
        target.pop("invoke_requires")
        target["requires"] = [r for r in target["requires"] if r.startswith("draft:")]
        aliases["content_digest"] = builder.digest({k: v for k, v in aliases.items() if k != "content_digest"})
        with self.assertRaisesRegex(builder.TableError, "missing required alias arguments.*fields.*max_chunk.*snapshot"):
            self.build(aliases=aliases, delta=self.delta, draft_without_jumbling=False)

    def test_filter_alias_uses_submitted_filter_preview(self):
        target = next(c["target"] for g in self.aliases["rows"] for c in g["cases"] if c["legacy_id"] == "filter-apply")
        self.assertEqual(target["command"], "filter.apply")
        self.assertEqual(target["arguments"], {})
        self.assertEqual(target["requires"], [])
        self.assertEqual(set(target["invoke_requires"]), {"preview"})
        requirement = target["invoke_requires"]["preview"]
        for phrase in ("FilterPreviewRef", "KN0.9", "no presentation", "no pending word preview"):
            self.assertIn(phrase, requirement)
        # LI.41 specifies equal outcomes before/after a relevant dependency change.
        # Here we check identical dispatch/arguments and gates, not engine outcomes.
        direct = {"row": "filter.apply", "arguments": {"preview": "submitted-filter-f"}}
        aliased = {"row": target["command"], "arguments": {**target["arguments"], "preview": "submitted-filter-f"}}
        self.assertEqual(aliased, direct)
        self.assertEqual(self.by_id[aliased["row"]]["gates"], ["fresh_filter_preview"])

    def test_missing_delta_requires_explicit_draft(self):
        with self.assertRaisesRegex(builder.TableError, "missing jumbling delta"):
            self.build(draft_without_jumbling=False)
        with self.assertRaisesRegex(builder.TableError, "cannot include a delta"):
            self.build(delta=self.fixture)

    def test_fixture_delta_and_inputs_are_immutable(self):
        before = deepcopy((builder.rows, self.join, self.aliases, self.fixture))
        result = self.build(delta=self.fixture, draft_without_jumbling=False)
        commands = {r["id"]: r for r in result["commands"]}
        self.assertTrue(result["final"])
        self.assertEqual(commands["macro.add"]["note"], "test fixture amendment")
        self.assertEqual(commands["example.jumble.row"]["lane"], "commit")
        self.assertEqual(commands["example.jumble.row"]["arguments"]["token"], "UserActionToken")
        self.assertEqual((builder.rows, self.join, self.aliases, self.fixture), before)
        delta = deepcopy(self.fixture)
        delta["amend"][0]["expect"]["disposition"] = ["redesign"]
        self.assertTrue(self.build(delta=delta, draft_without_jumbling=False)["final"])

    def test_h2_refusals(self):
        cases = []
        delta = deepcopy(self.fixture)
        delta["add"][0]["id"] = "macro.add"
        cases.append(("add existing id", delta))
        delta = deepcopy(self.fixture)
        delta["add"].append(deepcopy(delta["add"][0]))
        cases.append(("add existing id", delta))
        delta = deepcopy(self.fixture)
        delta["amend"][0]["id"] = "missing.command"
        cases.append(("amend missing id", delta))
        delta = deepcopy(self.fixture)
        delta["amend"][0]["expect"]["kind"] = "commit"
        cases.append(("incompatible amendment", delta))
        delta = deepcopy(self.fixture)
        delta["amend"].append(deepcopy(delta["amend"][0]))
        cases.append(("field amended twice", delta))
        for field, value in (("lane", "busy"), ("kind", "execute"), ("preview", "optional"),
                             ("undo", "conditional"), ("gates", ["not_busy"]), ("contexts", ["modal"])):
            delta = deepcopy(self.fixture)
            delta["add"][0][field] = value
            cases.append((f"invalid {field}", delta))
        delta = deepcopy(self.fixture)
        del delta["add"][0]["source"]
        cases.append(("missing required H2 fields", delta))
        delta = deepcopy(self.fixture)
        delta["format"] = "unknown"
        cases.append(("invalid delta format", delta))
        for message, delta in cases:
            with self.subTest(message=message), self.assertRaisesRegex(builder.TableError, message):
                self.build(delta=delta, draft_without_jumbling=False)

    def test_repeat_allowlist_and_duplicate_ids(self):
        for name in ("macro.add", "operation.preview", "object.inspect", "display.frame.toggle", "twist.apply"):
            with self.subTest(name=name), self.assertRaisesRegex(builder.TableError, "forbidden repeat"):
                self.build(base_rows=self.altered_row(name, repeat=True))
        with self.assertRaisesRegex(builder.TableError, "duplicate command id"):
            self.build(base_rows=builder.rows + [builder.rows[0]])
        with self.assertRaisesRegex(builder.TableError, "invalid inventory"):
            self.build(base_rows=self.altered_row("macro.add", inventory=["T-999"]))

    def test_other_row_and_join_failures(self):
        for name, changes, message in (
                ("session.timer.start", {"undo": "journal"}, "timer must be"),
                ("twist.apply", {"gates": []}, "preserve ONE.78"),
                ("operation.execute", {"preview": "none"}, "preview and fresh_preview disagree"),
                ("session.undo", {"arguments": {"action": "ActionNumber"}}, "user action token"),
                ("macro.add", {"repeat": "false"}, "must be booleans")):
            with self.subTest(name=name), self.assertRaisesRegex(builder.TableError, message):
                self.build(base_rows=self.altered_row(name, **changes))
        join = deepcopy(self.join)
        next(d for d in join["decisions"] if d["outcome"] == "none")["reason"] = ""
        with self.assertRaisesRegex(builder.TableError, "none needs a reason"):
            self.build(join=join)

    def test_binding_routing(self):
        self.build(bindings=[{"routing": "text", "command": None}, {"routing": "capture", "command": None},
                             {"routing": "list", "command": "object.inspect"},
                             {"routing": "list", "command": "list.row.copy"},
                             {"routing": "list", "command": "list.move", "repeat": True},
                             {"routing": "global", "command": "operation.execute"}])
        for binding in ({"routing": "text", "command": "session.undo"},
                        {"routing": "capture", "command": "twist.apply"},
                        {"routing": "list", "command": "macro.add"},
                        {"routing": "global", "command": "macro.add", "repeat": True}):
            with self.subTest(binding=binding), self.assertRaises(builder.TableError):
                self.build(bindings=[binding])

    def test_xm2_slot_map_write_surface(self):
        for name in ("camera.rotate", "camera.zoom", "filter.apply", "twist.apply", "object.inspect", "work.surface.show"):
            self.assertEqual(self.by_id[name]["slot_map"], "preserve", name)
        direct = [r["id"] for r in self.table["commands"] if r["slot_map"] == "rebind"]
        self.assertEqual(direct, ["grip.rebind"])
        for name in ("session.undo", "session.redo"):
            self.assertEqual(self.by_id[name]["slot_map"], "by_target")
        with self.assertRaisesRegex(builder.TableError, "only grip.rebind"):
            self.build(base_rows=self.altered_row("camera.rotate", slot_map="rebind"))
        with self.assertRaisesRegex(builder.TableError, "only session undo/redo"):
            self.build(base_rows=self.altered_row("camera.rotate", slot_map="by_target"))

    def test_xm3_named_undo_contract(self):
        self.assertEqual(self.table["undo_dispatch"]["stale"], "KN5:stale.undo_superseded")
        for name in ("session.undo", "session.redo"):
            self.assertEqual(self.by_id[name]["arguments"], {"hint": "ActionHint", "token": "UserActionToken"})
            with self.assertRaisesRegex(builder.TableError, "ActionHint"):
                self.build(base_rows=self.altered_row(name, action_guard="any_action"))
        self.assertIn("release_confirmation", self.table["undo_dispatch"]["protection"])

    def test_xm4_admission_surface_and_held_keys(self):
        self.assertNotIn("not_busy", self.table["vocabularies"]["gates"])
        for name in ("session.undo", "session.redo", "twist.apply", "operation.execute"):
            self.assertEqual(self.by_id[name]["lane"], "commit")
            self.assertFalse(self.by_id[name]["repeat"])
        self.assertEqual(self.by_id["session.undo"]["gates"], [])
        self.assertEqual(self.by_id["operation.preview"]["lane"], "exclusive")
        with self.assertRaisesRegex(builder.TableError, "commit lane"):
            self.build(base_rows=self.altered_row("session.undo", lane="exclusive"))
        with self.assertRaisesRegex(builder.TableError, "ungated"):
            self.build(base_rows=self.altered_row("session.undo", gates=["confirm"]))

    def test_xm5_resolved_rotation_and_distinct_destinations(self):
        live, draft = self.by_id["twist.apply"], self.by_id["operation.phase.append_twist"]
        for key in ("cap", "rotation"):
            self.assertEqual(live["arguments"][key], draft["arguments"][key])
        self.assertEqual(live["arguments"]["rotation"], "ExactRotationRef")
        self.assertEqual(live["undo"], "journal")
        self.assertEqual(draft["undo"], "workspace")
        self.assertEqual(draft["gates"], [])
        self.assertEqual(live["preview"], "none")  # ONE.78; owner, 6 November 2026, decision 10.
        with self.assertRaisesRegex(builder.TableError, "draft twist"):
            self.build(base_rows=self.altered_row(draft["id"], gates=["protection"]))

    def test_c7_timer_and_structure(self):
        for name in ("session.timer.start", "session.timer.stop"):
            self.assertEqual(self.by_id[name]["kind"], "workspace")
            self.assertEqual(self.by_id[name]["undo"], "none")
        self.assertNotIn("bank.select", self.by_id)
        self.assertNotIn("bank.functions.toggle", self.by_id)
        for name in ("structure.find", "structure.compare", "structure.layer", "view.cell_views", "camera.center", "set.save"):
            self.assertIn("redesign", self.by_id[name]["disposition"])

    def test_shared_undo_entries_and_order_target_binding(self):
        self.assertEqual(self.table["undo_dispatch"], {
            "entry": {"undo": "session.undo", "redo": "session.redo"}, "classes": ["journal", "workspace"],
            "guard": "action_hint", "stale": "KN5:stale.undo_superseded",
            "protection": "original_permission_path_including_release_confirmation"})
        for old in ("workspace.undo", "workspace.redo"):
            self.assertNotIn(old, self.by_id)
            row = deepcopy(self.by_id["session.undo"])
            row["id"], row["slot_map"] = old, "preserve"
            with self.assertRaisesRegex(builder.TableError, "duplicate purpose"):
                self.build(base_rows=builder.rows + [row])
        self.assertIn("U5-036", {d["decision"] for d in self.by_id["session.undo"]["decisions"]})
        for name in ("session.undo", "session.redo"):
            for changes in ({"arguments": {"action": "ActionNumber", "token": "UserActionToken"}},
                            {"slot_map": "preserve"}, {"gates": ["confirm"]}):
                with self.subTest(name=name, changes=changes), self.assertRaisesRegex(builder.TableError, "ActionHint"):
                    self.build(base_rows=self.altered_row(name, **changes))
        # Row-level binding evidence only; the engine must prove the stale refusal.
        hint = {"order": 42, "target": 41}
        request = {"row": "session.undo", "arguments": {"hint": hint, "token": "synthetic-token"}}
        for changed in ({"order": 43, "target": 41}, {"order": 42, "target": 40}):
            other = deepcopy(request)
            other["arguments"]["hint"] = changed
            self.assertNotEqual(builder.digest(request), builder.digest(other))

    def test_alias_identity_and_shared_undo_guards(self):
        self.assertEqual(self.aliases["version"], "en4-2026-11-05.3")
        payload = {k: v for k, v in self.aliases.items() if k != "content_digest"}
        self.assertEqual(builder.digest(payload), self.aliases["content_digest"])
        for group in self.aliases["rows"]:
            for case in group["cases"]:
                target = case["target"]
                if target and target["command"] in {"session.undo", "session.redo"}:
                    self.assertEqual(set(target["invoke_requires"]), {"hint", "token"})
                    self.assertEqual(target["arguments"], {})
                    self.assertNotIn("dispatch_by_action_class", target)
        aliases = deepcopy(self.aliases)
        target = next(c["target"] for g in aliases["rows"] for c in g["cases"] if c["legacy_id"] == "draft-undo")
        target["invoke_requires"]["action"] = target["invoke_requires"].pop("hint")
        aliases["content_digest"] = builder.digest({k: v for k, v in aliases.items() if k != "content_digest"})
        with self.assertRaisesRegex(builder.TableError, "fresh hint and token"):
            self.build(aliases=aliases)

    def test_preview_conflict_is_a_verdict_and_filter_has_own_gate(self):
        self.assertNotIn("protection", self.by_id["operation.preview"]["gates"])
        self.assertIn("protection", self.by_id["operation.execute"]["gates"])
        self.assertEqual(self.by_id["filter.apply"]["gates"], ["fresh_filter_preview"])
        self.assertEqual(self.by_id["filter.apply"]["preview"], "filter")
        for name, changes, message in (
                ("operation.preview", {"gates": ["protection"]}, "protection is a verdict"),
                ("operation.execute", {"gates": ["fresh_preview"]}, "protection gate required"),
                ("filter.apply", {"preview": "none"}, "fresh_filter_preview disagree"),
                ("filter.apply", {"gates": []}, "fresh_filter_preview disagree"),
                ("filter.apply", {"preview": "required", "gates": ["fresh_preview"]}, "requires filter freshness"),
                ("macro.add", {"preview": "filter", "gates": ["fresh_filter_preview"]}, "only filter.apply")):
            with self.subTest(name=name, changes=changes), self.assertRaisesRegex(builder.TableError, message):
                self.build(base_rows=self.altered_row(name, **changes))
        delta = deepcopy(self.delta)
        next(a for a in delta["amend"] if a["id"] == "operation.preview")["set"]["gates"] = ["protection"]
        with self.assertRaisesRegex(builder.TableError, "protection is a verdict"):
            self.build(delta=delta, draft_without_jumbling=False)

    def test_lane_targets_cover_every_view_and_latest_wins_row(self):
        for row in self.table["commands"]:
            if row["lane"] in {"view", "latest_wins"}:
                target = row["lane_target"]
                self.assertEqual(len(target), 1)
                self.assertTrue(set(target) <= {"arg", "view", "control"})
                if "arg" in target:
                    self.assertEqual(row["arguments"][target["arg"]], "ViewId")
        for name in ("operation.phase.select", "view.preview.scrub", "keyboard.toggle", "display.frame.toggle", "display.adaptive.toggle"):
            self.assertEqual(self.by_id[name]["lane_target"], {"control": name})
        self.assertEqual(self.by_id["camera.rotate"]["lane_target"], {"arg": "view"})
        self.assertEqual(self.by_id["camera.zoom"]["lane_target"], {"arg": "view"})
        for target in (None, {}, {"unknown": "V"}, {"view": "V", "control": "C"},
                       {"view": ""}, {"control": 2}, {"arg": "missing"}, {"arg": "rotation"}):
            with self.subTest(target=target), self.assertRaisesRegex(builder.TableError, "lane_target"):
                self.build(base_rows=self.altered_row("camera.rotate", lane_target=target))
        # All three declared forms work; same session/target can be shared across rows.
        self.build(base_rows=self.altered_row("camera.rotate", lane_target={"view": "V"}))
        delta = deepcopy(self.delta)
        next(r for r in delta["add"] if r["id"] == "jumble.release.check").pop("lane_target")
        with self.assertRaisesRegex(builder.TableError, "lane_target"):
            self.build(delta=delta, draft_without_jumbling=False)

    def test_jumbling_schemas_and_distinct_named_cap_bindings(self):
        for row in self.delta["add"]:
            self.assertEqual(self.by_id[row["id"]]["arguments"], row["arguments"])
            self.assertTrue(row["arguments"])
        for amendment in self.delta["amend"]:
            self.assertEqual(self.by_id[amendment["id"]]["arguments"], amendment["set"]["arguments"])
        self.assertEqual(self.by_id["operation.check"]["arguments"]["unit"], "PieceSet?")
        self.assertEqual(self.by_id["operation.check"]["arguments"]["cap"], "CapId?")
        self.assertEqual(self.by_id["jumble.twist.menu"]["arguments"]["letter"], "PinnedMenuLetter")
        self.assertEqual(self.by_id["jumble.twist.append"]["arguments"]["letter"], "PinnedMenuLetter")
        self.assertIn("CorrectionId", self.by_id["jumble.corrections.check"]["arguments"]["corrections"])
        for collection in ("add", "amend"):
            for empty in (False, True):
                delta = deepcopy(self.delta)
                row = delta[collection][0] if collection == "add" else delta[collection][0]["set"]
                if empty:
                    row["arguments"] = {}
                else:
                    row.pop("arguments")
                with self.subTest(collection=collection, empty=empty), self.assertRaisesRegex(builder.TableError, "argument schema|required H2 fields"):
                    self.build(delta=delta, draft_without_jumbling=False)
        arguments = {"snapshot": "S7", "cap": 0, "max_chunk": 32}
        other = dict(arguments, cap=1)
        self.assertEqual(set(arguments), set(self.by_id["jumble.straddlers"]["arguments"]))
        self.assertNotEqual(builder.digest(arguments), builder.digest(other))
        self.assertNotEqual(builder.digest({"row": "jumble.straddlers", "arguments": arguments}),
                            builder.digest({"row": "jumble.straddlers", "arguments": other}))

    def test_each_delta_row_rejects_each_missing_or_optional_required_argument(self):
        required = builder.DELTA_REQUIRED_ARGUMENTS
        deltas = (self.delta, self.language)
        self.assertEqual(set(required), {r["id"] for part in deltas for group in ("add", "amend") for r in part[group]})
        for part in deltas:
            for group in ("add", "amend"):
                for index, row in enumerate(part[group]):
                    self.assertLessEqual(required[row["id"]], self.by_id[row["id"]]["arguments"].keys())
                    for argument in sorted(required[row["id"]]):
                        for mutation in ("missing", "optional"):
                            with self.subTest(row=row["id"], argument=argument, mutation=mutation):
                                delta = deepcopy(part)
                                changed = delta[group][index]
                                schema = (changed if group == "add" else changed["set"])["arguments"]
                                if mutation == "optional":
                                    schema[argument] += "?"
                                    message = "required arguments cannot be optional"
                                else:
                                    # Single-argument rows also exercise a nonempty mutant;
                                    # an unrelated optional field cannot replace their input.
                                    if len(schema) == 1:
                                        schema["control"] = "Nat?"
                                    del schema[argument]
                                    message = "missing required arguments|token-only"
                                self.assertTrue(schema)
                                with self.assertRaisesRegex(builder.TableError, message):
                                    self.build(delta=delta if part is self.delta else [self.delta, delta],
                                               draft_without_jumbling=False)
        delta = deepcopy(self.fixture)
        delta["add"][0]["arguments"] = {"token": "UserActionToken"}
        with self.assertRaisesRegex(builder.TableError, "token-only"):
            self.build(delta=delta, draft_without_jumbling=False)

    def test_language_rows_match_the_answers_of_6_november(self):
        expected = {
            "view.open": ("view", "view", "none", False, {"view": "ViewId", "source": "ViewSourceRef"}),
            "view.close": ("view", "view", "none", False, {"view": "ViewId"}),
            "recording.start": ("workspace", "none", "none", False, {"name": "Text"}),
            "recording.stop": ("workspace", "none", "none", False, {"recording": "RecordingRef"}),
            "recording.play": ("workspace", "program", "none", True, {"recording": "RecordingRef", "from": "Nat?"}),
            "recording.convert": ("workspace", "none", "workspace", False,
                                  {"recording": "RecordingRef", "target": "draft | source"}),
        }
        self.assertEqual({r["id"] for r in self.language["add"]}, set(expected))
        for name, (kind, lane, undo, job, arguments) in expected.items():
            row = self.by_id[name]
            with self.subTest(row=name):
                self.assertEqual((row["kind"], row["lane"], row["undo"], row["job"], row["arguments"]),
                                 (kind, lane, undo, job, arguments))
                self.assertEqual(row["gates"], [])
                self.assertEqual(row["preview"], "none")
        for name in ("view.open", "view.close"):
            self.assertEqual(self.by_id[name]["lane_target"], {"arg": "view"})
        counts = Counter(r["kind"] for r in self.table["commands"])
        self.assertEqual(len(self.table["commands"]), 174)
        self.assertEqual((counts["workspace"], counts["view"]), (66, 45))
        for name in ("twist.apply", "jumble.twist.menu"):
            row = self.by_id[name]
            self.assertEqual(row["preview"], "none")
            self.assertIn("decision 10", row["note"])
            self.assertNotIn("pending owner confirmation", row["note"])
            self.assertNotIn("owner alternative", row["note"])
        mutant = deepcopy(self.language)
        del next(r for r in mutant["add"] if r["id"] == "view.open")["lane_target"]
        with self.assertRaisesRegex(builder.TableError, "lane_target"):
            self.build(delta=[self.delta, mutant], draft_without_jumbling=False)

    def test_a_field_amended_in_two_deltas_is_refused(self):
        again = deepcopy(next(r for r in self.language["amend"] if r["id"] == "twist.apply"))
        again["expect"] = {"note": self.by_id["twist.apply"]["note"]}
        third = {"format": builder.DELTA_FORMAT, "add": [], "amend": [again]}
        with self.assertRaisesRegex(builder.TableError, "field amended twice"):
            self.build(delta=[self.delta, self.language, third], draft_without_jumbling=False)

    def test_delta_token_only_schema_rejects_optional_extras(self):
        delta = deepcopy(self.fixture)
        delta["add"][0]["arguments"] = {
            "token": "UserActionToken", "revision": "RevisionRef?", "cap": "CapId?"}
        with self.assertRaisesRegex(builder.TableError, "token-only"):
            self.build(delta=delta, draft_without_jumbling=False)

    def test_jumble_on_rejects_optional_revision(self):
        delta = deepcopy(self.delta)
        arguments = next(row for row in delta["add"] if row["id"] == "jumble.on")["arguments"]
        self.assertEqual(arguments["revision"], "RevisionRef")
        arguments["revision"] += "?"
        with self.assertRaisesRegex(builder.TableError, r"jumble\.on: .*required arguments cannot be optional.*revision"):
            self.build(delta=delta, draft_without_jumbling=False)

    def test_one_scrub_row_for_retained_and_jumbling_previews(self):
        self.assertNotIn("jumble.preview.scrub", self.by_id)
        scrub = self.by_id["view.preview.scrub"]
        self.assertEqual(scrub["arguments"], {"preview": "PreviewId", "prefix": "WordPosition", "revision": "RevisionRef"})
        self.assertTrue(scrub["job"])
        self.assertTrue(scrub["repeat"])
        self.assertIn("jumbling", scrub["note"])
        self.assertEqual({r["id"] for r in self.table["commands"] if r["repeat"]}, builder.REPEAT)
        delta = deepcopy(self.delta)
        duplicate = deepcopy(scrub)
        duplicate.update(id="jumble.preview.scrub", repeat=False)
        delta["add"].append(duplicate)
        with self.assertRaisesRegex(builder.TableError, "duplicate purpose"):
            self.build(delta=delta, draft_without_jumbling=False)

    def test_services_are_registered_separately(self):
        services = self.table["services"]
        self.assertEqual({s["id"] for s in services}, {
            "kernel.snapshot.open", "kernel.snapshot.close", "kernel.stream.next", "kernel.value.retain", "kernel.value.release",
            "kernel.outcome", "kernel.job_result", "kernel.undo_hint", "language.publish_view"})
        self.assertEqual(len(services), 9)
        self.assertFalse({s["id"] for s in services} & self.by_id.keys())
        self.assertIn("jumble.observe", self.by_id)
        for service in services:
            self.assertEqual(set(service), {"id", "kind", "caller", "lane", "clauses", "note"})
        for change, message in (({"id": "operation.check"}, "collision"), ({"alias": "open"}, "requires only"),
                                ({"undo": "none"}, "requires only"), ({"availability": True}, "requires only"),
                                ({"key": "K"}, "requires only"), ({"caller": "any"}, "invalid service"),
                                ({"kind": "command"}, "invalid service"), ({"clauses": []}, "needs clauses")):
            changed = deepcopy(services)
            changed[0].update(change)
            with self.subTest(change=change), self.assertRaisesRegex(builder.TableError, message):
                self.build(services=changed)
        with self.assertRaisesRegex(builder.TableError, "collision"):
            self.build(services=services + [services[0]])

    def test_markdown_describes_the_final_table_and_layer_path(self):
        text = (HERE / "command-table.md").read_text(encoding="utf-8")
        self.assertNotIn("Each layer may call only the layer below", text)
        self.assertIn("LI.3", text)
        self.assertIn("LI.4", text)
        for row in self.table["commands"]:
            self.assertIn(f"| `{row['id']}` | {row['kind']} | {row['lane']} |", text)
        for old in ("workspace.undo", "workspace.redo", "jumble.preview.scrub"):
            self.assertNotIn(old, text)

    def test_cli_refusals_write_nothing_and_out_writes_lf(self):
        for argv in ([], ["--delta", "missing-delta.json"], ["--delta", str(FIXTURE), "--draft-without-jumbling"]):
            with self.subTest(argv=argv), patch.object(Path, "is_file", return_value=False), \
                    patch.object(Path, "write_bytes") as write, redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit) as stopped:
                    builder.main(argv)
                self.assertEqual(stopped.exception.code, 1)
                write.assert_not_called()
        # Capture the output call; no actual output file is created or changed.
        with patch.object(Path, "write_bytes", autospec=True) as write, redirect_stdout(io.StringIO()):
            self.assertEqual(builder.main(["--draft-without-jumbling", "--out", "chosen-output.json"]), 0)
            write.assert_called_once_with(Path("chosen-output.json"), builder.encoded(self.draft))


if __name__ == "__main__":
    unittest.main(verbosity=2)
