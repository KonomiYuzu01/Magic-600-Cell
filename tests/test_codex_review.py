"""Deterministic tests for tools/agents/codex_review.py with a fake Codex executable."""
from __future__ import annotations

import contextlib
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools" / "agents"))
import codex_review  # noqa: E402

TEST_ENV = {
    "GITHUB_PERSONAL_ACCESS_TOKEN": "dummy-token",
    "gh_token": "dummy-token",
    "OPENAI_API_KEY": "dummy-key",
    "MY_SECRET_VALUE": "dummy-secret",
    "mixed_kEy_name": "dummy-key",
    "PATH": "ordinary-path",
}

FAKE = r'''
import json, os, re, sys, time
args = sys.argv[1:]
mode = os.environ.get("FAKE_CODEX_MODE", "pass")
if args[:2] == ["login", "status"]:
    print("Logged in using an API key" if mode == "api-key" else "Logged in using ChatGPT")
    sys.exit(0)
provider = "custom" if mode == "wrong-provider" else "openai"
model = args[args.index("-m") + 1]
effort = re.search(r'model_reasoning_effort="(\w+)"', " ".join(args)).group(1)
sandbox = args[args.index("--sandbox") + 1]
out = args[args.index("-o") + 1]
if mode == "wrong-model": model = "gpt-other"
if mode == "wrong-effort": effort = "low"
if mode == "wrong-sandbox": sandbox = "workspace-write"
sys.stdin.read()
sys.stderr.write(f"OpenAI Codex v0.0.0-fake\n--------\nmodel: {model}\nprovider: {provider}\nsandbox: {sandbox}\nreasoning effort: {effort}\n--------\n")
sys.stderr.write('    54\tTIER_DROPPED_RE = re.compile(r"service tier .* not advertised")\n')  # echoed file content
if mode == "tier-dropped":
    sys.stderr.write("warning: Configured service tier `fast` is not advertised as supported for model `x` and will be omitted from requests.\n")
if mode == "sleep":
    time.sleep(120)
if mode == "exit1":
    sys.exit(1)
result = {"verdict": "pass", "summary": "fake", "findings": []}
if mode == "findings":
    result = {"verdict": "findings", "summary": "fake", "findings": [{"id": "F1", "severity": "major", "title": "t", "detail": "d",
              "evidence": [{"path": "core.py", "line": 1, "sha256": None}], "counterexample": None, "suggested_experiment": None,
              "verification_status": "unverified"}]}
if mode == "pass-with-blocker":
    result = {"verdict": "pass", "summary": "fake", "findings": [{"id": "F1", "severity": "blocker", "title": "t", "detail": "d",
              "evidence": [], "counterexample": None, "suggested_experiment": None, "verification_status": "verified"}]}
if mode == "schema-invalid":
    result = {"verdict": "maybe", "summary": "fake", "findings": []}
if mode == "missing":
    sys.exit(0)
with open(out, "w", encoding="utf-8") as f:
    f.write("not json" if mode == "bad-json" else json.dumps(result))
'''


class CodexReviewTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        tmp = Path(self.tmp.name)
        fake = tmp / "fake_codex.py"
        fake.write_text(FAKE, encoding="utf-8")
        self.packet = tmp / "packet.md"
        self.packet.write_text("# Problem packet\nTest only.\n", encoding="utf-8")
        self.saved = (codex_review.REVIEWS, codex_review.LEDGER, os.environ.get("MAGIC600_CODEX_CMD"), os.environ.get("FAKE_CODEX_MODE"))
        codex_review.REVIEWS = tmp / "reviews"
        codex_review.LEDGER = tmp / "ledger" / "codex.jsonl"
        os.environ["MAGIC600_CODEX_CMD"] = json.dumps([sys.executable, str(fake)])

    def tearDown(self):
        codex_review.REVIEWS, codex_review.LEDGER, cmd, mode = self.saved
        for key, value in (("MAGIC600_CODEX_CMD", cmd), ("FAKE_CODEX_MODE", mode)):
            if value is None:
                os.environ.pop(key, None)
            else:
                os.environ[key] = value
        self.tmp.cleanup()

    def run_wrapper(self, mode="pass", *extra):
        os.environ["FAKE_CODEX_MODE"] = mode
        err = io.StringIO()
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(err):
            code = codex_review.main(["--kind", "review", "--packet", str(self.packet), *extra])
        return code, err.getvalue()

    def last_ledger(self):
        return json.loads(codex_review.LEDGER.read_text(encoding="utf-8").splitlines()[-1])

    def only_meta(self):
        metas = list(codex_review.REVIEWS.glob("*/meta.json"))
        self.assertEqual(len(metas), 1)
        return json.loads(metas[0].read_text(encoding="utf-8"))

    def assert_filtered_env(self, env, allowed_names=()):
        self.assertIsInstance(env, dict)
        self.assertEqual(env["PATH"], TEST_ENV["PATH"])
        for name in env:
            if name not in allowed_names:
                self.assertNotRegex(name, r"(?i)KEY|SECRET|TOKEN")

    def test_child_and_sandbox_environments_filter_credentials(self):
        with mock.patch.dict(os.environ, TEST_ENV, clear=True), \
                mock.patch.object(codex_review, "WORKTREES", Path(self.tmp.name) / "worktrees"):
            for builder in (codex_review.child_env, codex_review.sandbox_env):
                with self.subTest(builder=builder.__name__):
                    env = builder()
                    keys = [f"GIT_CONFIG_KEY_{i}" for i in range(len(codex_review.CHILD_GIT_CONFIG))]
                    self.assert_filtered_env(env, keys)
                    self.assertEqual(env["PYTHONDONTWRITEBYTECODE"], "1")
                    self.assertEqual(env["GIT_CONFIG_COUNT"], str(len(codex_review.CHILD_GIT_CONFIG)))
                    for i, (key, value) in enumerate(codex_review.CHILD_GIT_CONFIG.items()):
                        self.assertEqual(env[f"GIT_CONFIG_KEY_{i}"], key)
                        self.assertEqual(env[f"GIT_CONFIG_VALUE_{i}"], value)
                    if builder is codex_review.sandbox_env:
                        self.assertEqual(env["CODEX_HOME"], str(Path(self.tmp.name) / "codex-sandbox-home"))
            for name, value in TEST_ENV.items():
                self.assertEqual(os.environ[name], value)

    def test_plan_and_review_pass_filtered_environment_to_run_codex(self):
        for kind in ("plan", "review"):
            with self.subTest(kind=kind), mock.patch.dict(os.environ, TEST_ENV, clear=True), \
                    mock.patch.object(codex_review, "source_identity", return_value={"digest": "test-source"}), \
                    mock.patch.object(codex_review, "codex_command", return_value=["fake-codex"]), \
                    mock.patch.object(subprocess, "run", return_value=subprocess.CompletedProcess(
                        [], 0, "Logged in using ChatGPT", "")), \
                    mock.patch.object(codex_review, "run_codex", return_value=(1, "", "")) as run:
                code, _ = self.run_wrapper("pass", "--kind", kind)
                self.assertEqual(code, 2)  # the mocked model call exits unsuccessfully
                run.assert_called_once()
                self.assert_filtered_env(run.call_args.kwargs.get("env"))

    def test_login_preflight_uses_filtered_environment_for_every_call_kind(self):
        contract = {"allowed_files": ["example.py"], "acceptance_check": ["python", "example.py"],
                    "stop_condition": "acceptance passes"}
        self.packet.write_text("# Test packet\n```implement-contract\n" + json.dumps(contract) + "\n```\n", encoding="utf-8")
        for kind in ("plan", "review", "implement"):
            with self.subTest(kind=kind), mock.patch.dict(os.environ, TEST_ENV, clear=True), \
                    mock.patch.object(codex_review, "source_identity", return_value={"digest": "test-source"}), \
                    mock.patch.object(codex_review, "codex_command", return_value=["fake-codex"]), \
                    mock.patch.object(subprocess, "run", return_value=subprocess.CompletedProcess([], 1, "", "")) as run:
                code, _ = self.run_wrapper("pass", "--kind", kind)
                self.assertEqual(code, 2)  # refuse before any model call or worktree creation
                run.assert_called_once()
                self.assertEqual(run.call_args.args[0], ["fake-codex", "login", "status"])
                self.assert_filtered_env(run.call_args.kwargs.get("env"))

    def test_valid_pass(self):
        code, _ = self.run_wrapper("pass")
        self.assertEqual(code, 0)
        meta = self.only_meta()
        self.assertTrue(meta["valid"])
        self.assertEqual(meta["verdict"], "pass")
        self.assertEqual(meta["resolved"], {"model": "gpt-6.1-sol", "effort": "max", "sandbox": "read-only", "provider": "openai"})
        entry = self.last_ledger()
        self.assertEqual(entry["outcome"], "pass")
        self.assertIsNone(entry["cost"]["usd"])  # unknown cost is null, never zero

    def test_findings_are_valid_results(self):
        code, _ = self.run_wrapper("findings")
        self.assertEqual(code, 0)
        self.assertEqual(self.only_meta()["verdict"], "findings")

    def test_reported_mismatch_invalidates_run(self):
        for mode in ("wrong-model", "wrong-effort", "wrong-sandbox", "wrong-provider"):
            with self.subTest(mode=mode):
                code, err = self.run_wrapper(mode)
                self.assertEqual(code, 2)
                self.assertIn("invalid run", err)
                self.assertEqual(self.last_ledger()["outcome"], "invalid")

    def test_bad_outputs_invalidate_run(self):
        for mode in ("bad-json", "schema-invalid", "missing", "exit1", "pass-with-blocker"):
            with self.subTest(mode=mode):
                code, _ = self.run_wrapper(mode)
                self.assertEqual(code, 2)

    def test_dropped_speed_tier_invalidates_run(self):
        os.environ["FAKE_CODEX_MODE"] = "tier-dropped"
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            code = codex_review.main(["--kind", "plan", "--packet", str(self.packet), "--speed", "fast"])
        self.assertEqual(code, 2)

    def test_speed_defaults(self):
        seen = []
        original = codex_review.run_codex
        codex_review.run_codex = lambda cmd, prompt, timeout, env=None: (seen.append(cmd), (1, "", ""))[1]
        try:
            self.run_wrapper("pass")
            self.run_wrapper("pass", "--model", "gpt-6-astra", "--effort", "ultra", "--gate", "architecture-freeze")
        finally:
            codex_review.run_codex = original
        self.assertIn('service_tier="default"', seen[0])
        self.assertIn('service_tier="default"', seen[1])

    def test_astra_runs_without_a_gate(self):
        code, _ = self.run_wrapper("pass", "--model", "gpt-6-astra")
        self.assertEqual(code, 0)
        entry = self.last_ledger()
        self.assertEqual(entry["requested"]["model"], "gpt-6-astra")
        self.assertIsNone(entry["gate"])

    def test_policy_refusals_are_not_ledgered_as_model_calls(self):
        code, _ = self.run_wrapper("pass", "--gate", "day7-go-no-go", "--effort", "ultra")  # gate without Astra
        self.assertEqual(code, 2)
        self.assertFalse(codex_review.LEDGER.exists())
        self.run_wrapper("pass")
        entry = self.last_ledger()
        self.assertIsInstance(entry["packet_sha256"], str)

    def test_api_key_login_is_refused_before_any_model_call(self):
        code, err = self.run_wrapper("api-key")
        self.assertEqual(code, 2)
        self.assertIn("ChatGPT subscription", err)
        self.assertFalse(list(codex_review.REVIEWS.glob("*/meta.json")))

    def test_review_without_source_identity_is_refused(self):
        original = codex_review.source_identity

        def broken(*_a, **_k):
            raise subprocess.CalledProcessError(1, "git")
        codex_review.source_identity = broken
        try:
            code, err = self.run_wrapper("pass")
        finally:
            codex_review.source_identity = original
        self.assertEqual(code, 2)
        self.assertIn("source identity", err)

    def test_policy_refusals(self):
        cases = [
            ["--gate", "day7-go-no-go", "--effort", "ultra"],            # gate without Astra
            ["--model", "gpt-6-astra", "--gate", "day7-go-no-go"],       # gate below ultra
            ["--model", "gpt-6-astra", "--effort", "ultra", "--gate", "day7-go-no-go", "--speed", "fast"],  # fast gate
            ["--timeout", "5"],
        ]
        for extra in cases:
            with self.subTest(extra=extra):
                code, err = self.run_wrapper("pass", *extra)
                self.assertEqual(code, 2)
                self.assertIn("refused", err)

    def test_rejects_unknown_arguments_and_models(self):
        for extra in (["--sandbox", "workspace-write"], ["--model", "gpt-other"], ["--effort", "low"], ["--kind", "execute"]):
            with self.subTest(extra=extra), contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit):
                    codex_review.main(["--kind", "review", "--packet", str(self.packet), *extra])

    def test_command_is_explicit_and_read_only(self):
        seen = {}

        def fake_run(cmd, prompt, timeout, env=None):
            seen["cmd"], seen["prompt"] = cmd, prompt
            return 1, "", ""

        original = codex_review.run_codex
        codex_review.run_codex = fake_run
        try:
            self.run_wrapper("pass")
        finally:
            codex_review.run_codex = original
        cmd = seen["cmd"]
        self.assertEqual(cmd[cmd.index("-m") + 1], "gpt-6.1-sol")
        self.assertIn('service_tier="default"', cmd)  # standard tier, always explicit
        self.assertIn('model_provider="openai"', cmd)
        self.assertIn('forced_login_method="chatgpt"', cmd)
        self.assertIn('model_reasoning_effort="max"', cmd)
        self.assertEqual(cmd[cmd.index("--sandbox") + 1], "read-only")
        self.assertIn("--ignore-user-config", cmd)  # no user MCP servers, plugins or sandbox settings
        if os.name == "nt":
            self.assertIn('windows.sandbox="unelevated"', cmd)  # otherwise every command is rejected
        self.assertNotIn("resume", cmd)
        self.assertIn("Review only.", seen["prompt"])
        self.assertIn("follow-up work", seen["prompt"])


class TimeoutTest(unittest.TestCase):
    def test_windows_timeout_taskkill_uses_filtered_environment(self):
        proc = mock.Mock(pid=1234)
        proc.communicate.side_effect = [subprocess.TimeoutExpired("fake-codex", 1), ("out", "err")]
        with mock.patch.dict(os.environ, TEST_ENV, clear=True), mock.patch.object(os, "name", "nt"), \
                mock.patch.object(subprocess, "CREATE_NEW_PROCESS_GROUP", 512, create=True), \
                mock.patch.object(subprocess, "Popen", return_value=proc), \
                mock.patch.object(subprocess, "run") as kill:
            result = codex_review.run_codex(["fake-codex"], "", 1, env={"PATH": TEST_ENV["PATH"]})
        self.assertEqual(result, (None, "out", "err"))
        kill.assert_called_once()
        self.assertEqual(kill.call_args.args[0], ["taskkill", "/F", "/T", "/PID", "1234"])
        env = kill.call_args.kwargs.get("env")
        self.assertIsInstance(env, dict)
        self.assertEqual(env["PATH"], TEST_ENV["PATH"])
        for name in env:
            self.assertNotRegex(name, r"(?i)KEY|SECRET|TOKEN")

    def test_run_codex_timeout_returns_none(self):
        code, _, _ = codex_review.run_codex([sys.executable, "-c", "import time; time.sleep(30)"], "", 1)
        self.assertIsNone(code)


if __name__ == "__main__":
    unittest.main(verbosity=2)
