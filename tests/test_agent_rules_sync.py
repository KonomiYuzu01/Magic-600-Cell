"""Check that agent rules, the briefing, settings, hooks and skills stay consistent."""
from __future__ import annotations

import fnmatch
import json
import re
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools" / "agents"))
import codex_review  # noqa: E402

AGENTS = (ROOT / "AGENTS.md").read_text(encoding="utf-8")
CLAUDE = (ROOT / "CLAUDE.md").read_text(encoding="utf-8")
BRIEFING = (ROOT / "docs" / "development-guide" / "AGENT_BRIEFING.md").read_text(encoding="utf-8")
HUMAN = (ROOT / "docs" / "development-guide" / "HUMAN_GUIDE.md").read_text(encoding="utf-8")
CJK = re.compile(r"[\u3040-\u30ff\u3400-\u9fff\uac00-\ud7af]")

REQUIRED_SHARED_RULES = [
    "259,800 labelled sticker slots",
    "1,200 legal generators",
    "`assets/manifest.json` as an immutable model boundary",
    "full collateral effects for every macro",
    "protected-orbit constraints",
    "Keep reset/import transactional and recoverable",
    "`EngineProcess`",
    "python tests/test_core.py",
    "python tests/test_reference_maps.py",
    "python tests/test_crash.py",
    "python tests/test_engine_lifecycle.py",
    "tests/native/NativeHostRegression.cs",
    "Use fresh isolated test data",
    "Headless results cannot establish Windows/DirectX, input, long-session or performance claims",
    "Andrey Astrelin's primary MPUlt credit",
    "Do not redistribute Microsoft Managed DirectX DLLs",
    "Keep public UI and documentation in English",
    "Stage only reviewed files",
    "`gpt-6.1-sol`",
    "`gpt-6-astra`",
    "review only; do not perform follow-up work",
    "Model agreement is never a pass criterion",
    "USD 100, then 150, then 300",
    "OpenAI models",
    "checksum-verified allowlist",
    "Never install from startup or review hooks",
    "Produce every formal non-software artifact with approved locally installed tools",
    "Authority order",
    "A checksum or signature mismatch blocks the installation",
]


class RuleSyncTests(unittest.TestCase):
    def test_agents_keeps_every_shared_rule(self):
        for rule in REQUIRED_SHARED_RULES:
            with self.subTest(rule=rule):
                self.assertIn(rule, AGENTS)

    def test_claude_imports_shared_rules_and_briefing(self):
        self.assertIn("\n@AGENTS.md\n", CLAUDE)
        self.assertIn("\n@docs/development-guide/AGENT_BRIEFING.md\n", CLAUDE)

    def test_claude_does_not_restate_shared_rules(self):
        for rule in REQUIRED_SHARED_RULES[:18]:
            with self.subTest(rule=rule):
                self.assertNotIn(rule, CLAUDE)
        self.assertNotIn("xhigh", CLAUDE + AGENTS + BRIEFING)

    def test_public_rule_files_are_english(self):
        for name, text in (("AGENTS.md", AGENTS), ("CLAUDE.md", CLAUDE), ("AGENT_BRIEFING.md", BRIEFING), ("HUMAN_GUIDE.md", HUMAN)):
            with self.subTest(name=name):
                self.assertIsNone(CJK.search(text))

    def test_briefing_has_phase_and_every_critical_path(self):
        self.assertRegex(BRIEFING, r"(?m)^Current phase: .+$")
        patterns = json.loads((ROOT / "tools" / "agents" / "critical_paths.json").read_text(encoding="utf-8"))["patterns"]
        for pattern in patterns:
            with self.subTest(pattern=pattern):
                self.assertIn(f"`{pattern}`", BRIEFING)

    def test_briefing_superseded_line_references_point_at_the_right_text(self):
        arch = (ROOT / "docs" / "architecture" / "1.0" / "10_V1_ARCHITECTURE.md").read_text(encoding="utf-8").splitlines()
        expected = {13: "B4-12", 29: "Subtitles remain English only", 45: "0.4 contracts still apply", 115: "egui", 746: "wgpu", 786: "Switch to Qt", 806: "sequential candidates"}
        for line, text in expected.items():
            with self.subTest(line=line):
                self.assertIn(text, arch[line - 1])

    def test_wrapper_models_and_gates_match_the_rules(self):
        self.assertEqual(codex_review.DEFAULT_MODEL, "gpt-6.1-sol")
        self.assertEqual(set(codex_review.MODELS), {"gpt-6.1-sol", "gpt-6-astra"})
        for gate in codex_review.GATES:
            self.assertIn(gate, BRIEFING)

    def test_settings_hooks_and_permissions(self):
        settings = json.loads((ROOT / ".claude" / "settings.json").read_text(encoding="utf-8"))
        self.assertNotIn("enabledPlugins", settings)
        self.assertNotIn("extraKnownMarketplaces", settings)
        for event, groups in settings["hooks"].items():
            for group in groups:
                for hook in group["hooks"]:
                    script = hook["args"][-1].replace("${CLAUDE_PROJECT_DIR}/", "")  # the script is the last argument, also behind a guard
                    with self.subTest(event=event):
                        self.assertEqual(hook["command"], "python")
                        self.assertTrue((ROOT / script).is_file(), script)
        perms = settings["permissions"]
        self.assertIn("Bash(python tools/toolchain/bootstrap.py approve)", perms["deny"])
        self.assertTrue(all("install" not in rule for rule in perms["allow"]))
        self.assertFalse(any(rule.startswith("Bash(git") for rule in perms["allow"]))
        tools = [rule for rule in perms["allow"] if not rule.startswith("Bash(python tests/")]  # test files: checked below
        self.assertFalse(any("approve" in rule or "implement" in rule for rule in tools))
        self.assertIn("Bash(python tools/workbench/progress.py:*)", perms["allow"])  # owner decision, 1 October 2026
        # Test files run without a prompt only as exact commands without arguments: unittest treats extra
        # arguments as names of callables to run, so a wildcard would allow arbitrary Python (PR #32 review B-01).
        tests = [rule for rule in perms["allow"] if rule.startswith("Bash(python tests/")]
        self.assertIn("Bash(python tests/test_core.py)", tests)
        for rule in tests:
            with self.subTest(rule=rule):
                self.assertRegex(rule, r"^Bash\(python tests/test_[a-z0-9_]+\.py\)$")
                self.assertTrue((ROOT / rule[len("Bash(python "):-1]).is_file())
        self.assertFalse(any(rule.startswith("Bash(python tests") and ("*" in rule or ":" in rule) for rule in perms["allow"]))

    def test_critical_paths_cover_their_own_enforcement(self):
        patterns = json.loads((ROOT / "tools" / "agents" / "critical_paths.json").read_text(encoding="utf-8"))["patterns"]
        for path in (".claude/hooks/stop_gate.py", "tools/agents/codex_review.py", "tools/toolchain/bootstrap.py", "AGENTS.md", "session.py"):
            with self.subTest(path=path):
                self.assertTrue(any(fnmatch.fnmatch(path, p) for p in patterns))

    def test_skills_are_in_sync(self):
        r = subprocess.run([sys.executable, str(ROOT / "tools" / "skills" / "sync.py"), "--check"], capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stdout)

    def test_referenced_files_exist(self):
        for rel in ("templates/problem-packet.md", "schemas/review-result.schema.json", "schemas/ledger-entry.schema.json",
                    ".claude/agents/fable-solver.md", "docs/wiki/SCHEMA.md", "docs/wiki/index.md", "docs/wiki/log.md",
                    "tools/wiki/lint.py", "tools/toolchain/bootstrap.py", "tools/skills/sync.py"):
            with self.subTest(rel=rel):
                self.assertTrue((ROOT / rel).is_file())
        for role in ("staff-orchestrator", "architect", "planner", "implementer", "reviewer", "verifier"):
            self.assertTrue((ROOT / "templates" / "roles" / f"{role}.md").is_file())


if __name__ == "__main__":
    unittest.main(verbosity=2)
