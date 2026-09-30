"""Isolation tests for codex_review.py --kind implement and --cleanup.

ImplementTests use a fake Codex on a throwaway repository, so they check what the wrapper detects and
prevents by itself. RealSandboxTests run the installed Codex sandbox runner (no model call) and check
that it denies the writes the wrapper relies on it to deny.
"""
from __future__ import annotations

import contextlib
import io
import json
import os
import shutil
import stat
import subprocess
import sys
import tempfile
import unittest
import uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools" / "agents"))
import codex_review  # noqa: E402

FAKE = r'''
import json, os, re, subprocess, sys
args = sys.argv[1:]
mode = os.environ.get("FAKE_CODEX_MODE", "success")
if args[:1] == ["sandbox"]:  # stand-in for the sandbox runner: checks the profile, then runs the command
    joined = " ".join(args)
    if "-P" not in args or ':workspace"' not in joined or "network.enabled=false" not in joined:
        sys.exit("fake: unexpected sandbox profile")
    home = os.environ.get("CODEX_HOME", "")
    if not home.endswith("codex-sandbox-home") or os.path.exists(os.path.join(home, "config.toml")):
        sys.exit("fake: the sandbox runner did not get the isolated Codex home")
    sys.exit(subprocess.run(args[args.index("--") + 1:], cwd=args[args.index("-C") + 1]).returncode)
if args[:2] == ["login", "status"]:
    print("Logged in using ChatGPT")
    sys.exit(0)
if "--ignore-user-config" not in args:
    sys.exit("fake: user config not ignored")
model = args[args.index("-m") + 1]
effort = re.search(r'model_reasoning_effort="(\w+)"', " ".join(args)).group(1)
sandbox = args[args.index("--sandbox") + 1]
wt = args[args.index("-C") + 1]
if os.path.realpath(wt) != os.path.realpath(os.getcwd()):
    sys.exit("fake: cwd is not the -C worktree")
if mode == "wrong-model": model = "gpt-other"
if mode == "wrong-sandbox": sandbox = "danger-full-access"
roots = "workdir, C:\\Users\\x" if mode == "extra-root" else "workdir, /tmp, $TMPDIR"
sys.stdin.read()
sys.stderr.write(f"OpenAI Codex v0.0.0-fake\n--------\nmodel: {model}\nprovider: openai\nsandbox: {sandbox} [{roots}]\nreasoning effort: {effort}\n--------\n")
if mode == "tier-dropped":
    sys.stderr.write("warning: Configured service tier `fast` is not advertised as supported for model `x` and will be omitted from requests.\n")

def git(*a, check=True):
    return subprocess.run(["git", "-c", "user.email=f@example.invalid", "-c", "user.name=f", *a], check=check,
                          capture_output=True, text=True).stdout.strip()

def write(rel, text):
    os.makedirs(os.path.dirname(rel) or ".", exist_ok=True)
    with open(rel, "w", encoding="utf-8") as f:
        f.write(text)

write("src/new.py", '"""Added by the fake implementer."""\n')  # every mode makes one allowed change
common = git("rev-parse", "--path-format=absolute", "--git-common-dir")
if mode == "outside-tracked": write("README.md", "changed\n")
if mode == "outside-untracked": write("other/x.txt", "x\n")
if mode == "outside-ignored": write("work/x.txt", "x\n")
if mode == "outside-delete": os.remove("README.md")
if mode == "pycache": write("src/__pycache__/new.cpython-314.pyc", "cached\n")
if mode == "allowed-ignored": write("work/out.txt", "out\n")
for flag in ("assume-unchanged", "skip-worktree"):
    if mode == flag:
        git("update-index", f"--{flag}", "README.md")
        write("README.md", "hidden\n")
if mode == "critical": write("core.py", "x = 2\n")
if mode.startswith("attributes"):
    write(os.environ.get("FAKE_ATTRIBUTES_FILE", ".gitattributes"), "sample.dat -text\n")
    with open("src/sample.dat", "wb") as f:
        f.write(b"alpha\r\nbeta\r\n")
if mode == "binary":
    with open("src/data.bin", "wb") as f:
        f.write(bytes(range(256)) + b"\r\n\x00end")
if mode == "commit":
    git("add", "src/new.py")
    git("commit", "-qm", "codex commit")
if mode == "tag": git("tag", "codex-tag")
if mode == "branch": git("update-ref", "refs/heads/evil", "HEAD")
if mode == "stash":
    write("src/a.py", "stashed = 1\n")
    git("stash", "push", "-q", "--", "src/a.py")
if mode == "push":
    git("push", "-q", "origin", "HEAD:refs/heads/pushed", check=False)
    git("push", "-q", os.environ["FAKE_REMOTE_PATH"], "HEAD:refs/heads/pushed-by-path", check=False)
if mode == "other-branch":
    commit = git("commit-tree", "HEAD^{tree}", "-p", "HEAD", "-m", "x")
    for ref in git("for-each-ref", "--format=%(refname)", "refs/heads/codex/").split():
        if not ref.endswith(os.path.basename(os.getcwd())):
            git("update-ref", ref, commit)
if mode == "fake-codex-branch": git("update-ref", "refs/heads/codex/20990101T000000Z-00000000", "HEAD")
if mode == "config": git("config", "core.editor", "evil")
if mode == "hook": write(os.path.join(common, "hooks", "pre-commit"), "#!/bin/sh\nexit 0\n")
if mode == "info-exclude": write(os.path.join(common, "info", "exclude"), "README.md\n")
if mode == "pointer":
    with open(".git", "a", encoding="utf-8") as f:
        f.write("\n")
if mode == "nested-repo": git("init", "-q", "src/sub")
if mode == "link":
    target = os.environ["FAKE_LINK_TARGET"]
    if os.name == "nt":
        import _winapi
        _winapi.CreateJunction(target, os.path.abspath("src/link"))
    else:
        os.symlink(target, "src/link")
with open(args[args.index("-o") + 1], "w", encoding="utf-8") as f:
    f.write("fake report\n")
sys.exit(1 if mode == "exit1" else 0)
'''

PASS_CHECK = ["python", "-c", "import ast; assert ast.get_docstring(ast.parse(open('src/new.py').read()))"]
FAIL_CHECK = ["python", "-c", "raise SystemExit(5)"]
SLOW_CHECK = ["python", "-c", "import time; time.sleep(60)"]
EDIT_CHECK = ["python", "-c", "open('src/new.py', 'w').write('x = 1\\n')"]
COMMIT_CHECK = ["python", "-c", "import subprocess; subprocess.run(['git', '-c', 'user.email=a@example.invalid', '-c', 'user.name=a', "
                "'commit', '-q', '--allow-empty', '-m', 'from acceptance'], check=True)"]


def git(repo: Path, *args: str) -> str:
    return subprocess.run(["git", "-c", "user.email=t@example.invalid", "-c", "user.name=t", *args], cwd=repo,
                          check=True, capture_output=True, text=True).stdout


def packet(contract) -> str:
    block = contract if isinstance(contract, str) else json.dumps(contract)
    return f"# Problem packet\n\n## 6. Constraints and owned files\n\n```implement-contract\n{block}\n```\n"


def contract(check=PASS_CHECK, allowed=("src/*",)):
    return {"allowed_files": list(allowed), "acceptance_check": check, "stop_condition": "src/new.py has a docstring"}


def make_repo(repo: Path) -> None:
    (repo / "src").mkdir(parents=True)
    (repo / "src" / "a.py").write_text("a = 1\n", encoding="utf-8")
    (repo / "README.md").write_text("readme\n", encoding="utf-8")
    (repo / "core.py").write_text("x = 1\n", encoding="utf-8")
    (repo / ".gitignore").write_text("work/\n__pycache__/\n", encoding="utf-8")
    git(repo, "init", "-q", "-b", "main")
    git(repo, "add", ".")
    git(repo, "commit", "-qm", "base")


def make_link(target: Path, link: Path) -> None:
    if os.name == "nt":
        import _winapi
        _winapi.CreateJunction(str(target), str(link))
    else:
        os.symlink(target, link)


def force_rmtree(path: Path) -> None:
    codex_review.remove_links(path)
    shutil.rmtree(path, onexc=lambda fn, p, _e: (os.chmod(p, stat.S_IWRITE), fn(p)))


class WrapperFixture(unittest.TestCase):
    def patch_wrapper(self, repo: Path, outputs: Path):
        self.saved = {name: getattr(codex_review, name) for name in
                      ("ROOT", "REVIEWS", "LEDGER", "WORKTREES", "SANDBOX_DEFAULT_WRITABLE", "ACCEPTANCE_TIMEOUT", "run_codex")}
        codex_review.ROOT = repo
        codex_review.REVIEWS = outputs / "reviews"
        codex_review.LEDGER = outputs / "ledger" / "codex.jsonl"
        codex_review.WORKTREES = repo / "work" / "worktrees"

    def restore_wrapper(self):
        for name, value in self.saved.items():
            setattr(codex_review, name, value)


class ImplementTests(WrapperFixture):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        tmp = Path(self.tmp.name).resolve()
        self.repo = repo = tmp / "repo"
        make_repo(repo)
        git(tmp, "init", "-q", "--bare", "origin.git")
        self.remote = tmp / "origin.git"
        git(repo, "remote", "add", "origin", str(self.remote))
        git(repo, "push", "-q", "-u", "origin", "main")
        hooks = Path(git(repo, "rev-parse", "--path-format=absolute", "--git-common-dir").strip()) / "hooks"
        (hooks / "post-checkout").write_text(f"#!/bin/sh\necho ran > '{(tmp / 'hook-ran').as_posix()}'\n", encoding="utf-8")
        self.base = git(repo, "rev-parse", "HEAD").strip()
        self.link_target = tmp / "outside"
        self.link_target.mkdir()
        (self.link_target / "keep.txt").write_text("keep\n", encoding="utf-8")
        fake = tmp / "fake_codex.py"
        fake.write_text(FAKE, encoding="utf-8")
        self.packet = tmp / "packet.md"
        self.patch_wrapper(repo, tmp)
        codex_review.SANDBOX_DEFAULT_WRITABLE = ()  # the fixture lives in the temp directory
        self.saved_env = {k: os.environ.get(k) for k in ("MAGIC600_CODEX_CMD", "FAKE_CODEX_MODE", "FAKE_LINK_TARGET", "FAKE_REMOTE_PATH",
                                                         "FAKE_ATTRIBUTES_FILE")}
        os.environ["MAGIC600_CODEX_CMD"] = json.dumps([sys.executable, str(fake)])
        os.environ["FAKE_LINK_TARGET"] = str(self.link_target)
        os.environ["FAKE_REMOTE_PATH"] = str(self.remote)

    def tearDown(self):
        self.restore_wrapper()
        for key, value in self.saved_env.items():
            if value is None:
                os.environ.pop(key, None)
            else:
                os.environ[key] = value
        codex_review.remove_links(Path(self.tmp.name))  # never delete through a junction the fake created
        self.tmp.cleanup()

    def run_wrapper(self, mode="success", contract_=None, extra=()):
        os.environ["FAKE_CODEX_MODE"] = mode
        self.packet.write_text(packet(contract_ or contract()), encoding="utf-8")
        return self.call("--kind", "implement", "--packet", str(self.packet), *extra)

    def call(self, *argv):
        before = set(codex_review.REVIEWS.glob("*/meta.json"))
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = codex_review.main(list(argv))
        new = set(codex_review.REVIEWS.glob("*/meta.json")) - before
        if new:
            self.last_meta = new.pop()
        return code, out.getvalue() + err.getvalue()

    def meta(self):
        return json.loads(self.last_meta.read_text(encoding="utf-8")), self.last_meta.parent

    def ledger(self):
        if not codex_review.LEDGER.exists():
            return []
        return [json.loads(line) for line in codex_review.LEDGER.read_text(encoding="utf-8").splitlines()]

    def cleanup_last(self):
        cid = self.meta()[0]["call_id"]
        self.assertEqual(self.call("--cleanup", cid)[0], 0)
        self.assertFalse((codex_review.WORKTREES / cid).exists())
        self.assertEqual(git(self.repo, "branch", "--list", f"codex/{cid}").strip(), "")

    def assert_invalid(self, mode, expect, contract_=None):
        code, out = self.run_wrapper(mode, contract_)
        self.assertEqual(code, 2, out)
        meta, out_dir = self.meta()
        self.assertFalse(meta["valid"])
        self.assertIn(expect, " ".join(meta["problems"]))
        self.assertEqual(self.ledger()[-1]["outcome"], "invalid")
        if not meta["acceptance"]:
            self.assertFalse((out_dir / "changes.patch").exists())  # Codex-authored code never ran

    def test_clean_success(self):
        code, out = self.run_wrapper("success")
        self.assertEqual(code, 0, out)
        meta, out_dir = self.meta()
        self.assertTrue(meta["valid"])
        self.assertEqual(meta["changed_files"], ["src/new.py"])
        self.assertEqual(meta["base"], self.base)
        self.assertEqual(meta["resolved"]["sandbox"], "workspace-write")
        self.assertEqual(meta["sandbox_line"], "workspace-write [workdir, /tmp, $TMPDIR]")
        self.assertEqual(meta["acceptance"]["exit_code"], 0)
        self.assertEqual(meta["critical_paths_touched"], [])
        entry = self.ledger()[-1]
        self.assertEqual((entry["kind"], entry["outcome"], entry["source_identity"]), ("implement", "implemented", self.base))
        self.assertEqual(len(self.ledger()), 1)
        # The main worktree is untouched; the change exists only as a patch and in the packet-owned worktree.
        self.assertEqual(git(self.repo, "status", "--porcelain"), "")
        self.assertEqual(git(self.repo, "rev-parse", "HEAD").strip(), self.base)
        self.assertEqual(git(self.repo, "rev-parse", f"codex/{meta['call_id']}").strip(), self.base)
        git(self.repo, "apply", "--check", str(out_dir / "changes.patch"))
        self.assertFalse((Path(self.tmp.name) / "hook-ran").exists())  # worktree add ran without repository hooks
        self.cleanup_last()
        self.assertTrue((out_dir / "changes.patch").is_file())  # outputs stay after cleanup

    def test_commands_use_the_sandbox_in_the_new_worktree_only(self):
        seen = {}
        original_codex, original_bounded = codex_review.run_codex, codex_review.run_bounded

        def spy_codex(cmd, prompt, timeout, cwd=None, env=None):
            seen.update(cmd=cmd, prompt=prompt, cwd=cwd, env=env)
            return original_codex(cmd, prompt, timeout, cwd, env)

        def spy_bounded(cmd, prompt, timeout, cwd=None, env=None):
            if "sandbox" in cmd:
                seen["acceptance"] = cmd
            return original_bounded(cmd, prompt, timeout, cwd, env)
        codex_review.run_codex, codex_review.run_bounded = spy_codex, spy_bounded
        try:
            self.assertEqual(self.run_wrapper("success")[0], 0)
        finally:
            codex_review.run_bounded = original_bounded
        cmd, wt = seen["cmd"], codex_review.WORKTREES / self.meta()[0]["call_id"]
        self.assertEqual(Path(seen["cwd"]), wt)
        self.assertEqual(Path(cmd[cmd.index("-C") + 1]), wt)
        self.assertEqual(cmd[cmd.index("--sandbox") + 1], "workspace-write")
        for flag in ("--ignore-user-config", "sandbox_workspace_write.writable_roots=[]", "sandbox_workspace_write.network_access=false",
                     'forced_login_method="chatgpt"', 'service_tier="default"', 'shell_environment_policy.set.GIT_CONFIG_KEY_0="protocol.allow"'):
            self.assertIn(flag, cmd)
        if os.name == "nt":
            self.assertIn('windows.sandbox="unelevated"', cmd)  # otherwise Codex falls back to read-only
        for forbidden in ("--add-dir", "--dangerously-bypass-approvals-and-sandbox", "--worktree", "resume"):
            self.assertNotIn(forbidden, cmd)
        self.assertEqual(seen["env"]["PYTHONDONTWRITEBYTECODE"], "1")
        self.assertIn("Do not commit", seen["prompt"])
        self.assertIn("```implement-contract", seen["prompt"])
        acc = seen["acceptance"]
        self.assertEqual(acc[acc.index("sandbox") + 1:acc.index("sandbox") + 3], ["-P", codex_review.SANDBOX_PROFILE])
        self.assertIn(f"permissions.{codex_review.SANDBOX_PROFILE}.network.enabled=false", acc)
        self.assertEqual(Path(acc[acc.index("-C") + 1]), wt)
        self.assertEqual(acc[acc.index("--") + 1], sys.executable)

    def test_writes_outside_the_allowed_files_invalidate(self):
        for mode, path in (("outside-tracked", "README.md"), ("outside-untracked", "other/x.txt"),
                           ("outside-ignored", "work/x.txt"), ("outside-delete", "README.md"),
                           ("assume-unchanged", "README.md"), ("skip-worktree", "README.md")):
            with self.subTest(mode=mode):
                self.assert_invalid(mode, path)
        self.assert_invalid("pycache", "src/__pycache__/new.cpython-314.pyc", contract(allowed=["src/new.py"]))

    def test_allowed_file_inside_an_ignored_directory_is_exported(self):
        code, out = self.run_wrapper("allowed-ignored", contract(allowed=["src/*", "work/out.txt"]))
        self.assertEqual(code, 0, out)
        meta, out_dir = self.meta()
        self.assertEqual(meta["changed_files"], ["src/new.py", "work/out.txt"])
        self.assertIn(b"work/out.txt", (out_dir / "changes.patch").read_bytes())

    def test_git_state_changes_invalidate(self):
        cases = (("commit", "refs/heads/codex/"), ("tag", "refs/tags/codex-tag"), ("branch", "refs/heads/evil"),
                 ("stash", "refs/stash"), ("config", "git config changed"), ("hook", "git hooks changed"),
                 ("info-exclude", "git info"), ("pointer", ".git file changed"), ("nested-repo", "nested repository"),
                 ("fake-codex-branch", "other codex branches changed"))
        for mode, expect in cases:
            with self.subTest(mode=mode):
                self.assert_invalid(mode, expect)

    def test_push_is_prevented(self):
        before = git(self.remote, "for-each-ref")
        self.run_wrapper("push")
        self.assertEqual(git(self.remote, "for-each-ref"), before)  # neither by remote name nor by literal path
        self.assertNotIn("refs/remotes/origin/pushed", git(self.repo, "for-each-ref"))

    def test_parallel_calls_are_isolated_from_each_other(self):
        self.assertEqual(self.run_wrapper("success")[0], 0)
        self.assertEqual(self.run_wrapper("success")[0], 0)  # the first call's branch and worktree do not invalidate
        self.assert_invalid("other-branch", "other codex branches changed")  # but moving another call's branch does

    def test_link_creation_invalidates_and_cleanup_spares_the_target(self):
        self.assert_invalid("link", "link created: src/link")
        self.cleanup_last()
        self.assertTrue((self.link_target / "keep.txt").is_file())

    def test_reported_mismatch_invalidates(self):
        for mode, expect in (("wrong-sandbox", "sandbox danger-full-access"), ("wrong-model", "model gpt-other"),
                             ("exit1", "codex exited 1"), ("extra-root", "writable roots")):
            with self.subTest(mode=mode):
                self.assert_invalid(mode, expect)
        code, _ = self.run_wrapper("tier-dropped", None, ["--speed", "fast"])
        self.assertEqual(code, 2)
        self.assertIn("speed tier fast was dropped", " ".join(self.meta()[0]["problems"]))

    def test_failing_acceptance_check_is_reported(self):
        code, _ = self.run_wrapper("success", contract(FAIL_CHECK))
        self.assertEqual(code, 4)
        meta, _ = self.meta()
        self.assertTrue(meta["valid"])
        self.assertEqual(meta["acceptance"]["exit_code"], 5)
        self.assertEqual(self.ledger()[-1]["outcome"], "acceptance_failed")

    def test_acceptance_check_may_not_change_the_candidate_or_git_state(self):
        for check, expect in ((EDIT_CHECK, "worktree changed during the acceptance check"), (COMMIT_CHECK, "refs/heads/codex/")):
            with self.subTest(check=check[-1][:30]):
                code, out = self.run_wrapper("success", contract(check))
                self.assertEqual(code, 2, out)
                self.assertIn(f"after the acceptance check", " ".join(self.meta()[0]["problems"]))
                self.assertIn(expect, " ".join(self.meta()[0]["problems"]))

    def test_timeouts_and_wrapper_errors_leave_a_removable_record(self):
        codex_review.ACCEPTANCE_TIMEOUT = 3
        code, _ = self.run_wrapper("success", contract(SLOW_CHECK))
        self.assertEqual(code, 4)
        self.assertIsNone(self.meta()[0]["acceptance"]["exit_code"])
        self.cleanup_last()

        codex_review.run_codex = lambda cmd, prompt, timeout, cwd=None, env=None: (None, "", "")
        code, _ = self.run_wrapper("success")
        self.assertEqual(code, 3)
        self.assertEqual(self.ledger()[-1]["outcome"], "timeout")
        self.cleanup_last()

        codex_review.run_codex = lambda cmd, prompt, timeout, cwd=None, env=None: (_ for _ in ()).throw(OSError("gone"))
        code, _ = self.run_wrapper("success")
        self.assertEqual(code, 2)
        self.assertIn("wrapper error: OSError", " ".join(self.meta()[0]["problems"]))
        self.assertEqual(self.ledger()[-1]["outcome"], "invalid")
        self.cleanup_last()

    def test_critical_paths_are_flagged(self):
        code, out = self.run_wrapper("critical", contract(allowed=["src/*", "core.py"]))
        self.assertEqual(code, 0, out)
        self.assertEqual(self.meta()[0]["critical_paths_touched"], ["core.py"])

    def test_incomplete_or_unsafe_packets_are_refused_before_any_call(self):
        good = contract()
        cases = ["not json", {k: v for k, v in good.items() if k != "allowed_files"},
                 {k: v for k, v in good.items() if k != "acceptance_check"},
                 {k: v for k, v in good.items() if k != "stop_condition"},
                 dict(good, allowed_files=[]), dict(good, acceptance_check=[]), dict(good, stop_condition=" "),
                 dict(good, extra=1), dict(good, allowed_files=["*"]), dict(good, allowed_files=["../x"]),
                 dict(good, allowed_files=["/abs"]), dict(good, allowed_files=[".git/*"]), dict(good, allowed_files=["src\\x"])]
        for bad in cases:
            with self.subTest(contract=bad):
                code, out = self.run_wrapper("success", bad)
                self.assertEqual(code, 2)
                self.assertIn("refused", out)
        self.packet.write_text("# Problem packet without a contract\n", encoding="utf-8")
        code, out = self.call("--kind", "implement", "--packet", str(self.packet))
        self.assertEqual(code, 2)
        self.assertIn("implement-contract", out)
        code, _ = self.run_wrapper("success", None, ["--model", "gpt-6-astra", "--effort", "ultra", "--gate", "architecture-freeze"])
        self.assertEqual(code, 2)
        codex_review.SANDBOX_DEFAULT_WRITABLE = (Path(self.tmp.name),)
        code, out = self.run_wrapper("success")
        self.assertEqual(code, 2)
        self.assertIn("leaves writable", out)
        self.assertEqual(self.ledger(), [])
        self.assertFalse(codex_review.WORKTREES.exists() and any(codex_review.WORKTREES.iterdir()))

    def test_batch_launchers_are_replaced_by_the_native_executable(self):
        npm = Path(self.tmp.name) / "npm"
        exe = npm / "node_modules" / "@openai" / "codex" / "node_modules" / "@openai" / "codex-win32-x64" / "vendor" / "x86_64-pc-windows-msvc" / "bin" / "codex.exe"
        (npm / "codex.cmd").parent.mkdir(parents=True)
        (npm / "codex.cmd").write_text("@echo off\n", encoding="utf-8")
        with self.assertRaises(codex_review.Refused):
            codex_review.native_codex(npm / "codex.cmd")
        exe.parent.mkdir(parents=True)
        exe.write_bytes(b"")
        self.assertEqual(codex_review.native_codex(npm / "codex.CMD"), str(exe))
        self.assertEqual(codex_review.native_codex(exe), str(exe))

    def test_patch_export_ignores_filters_and_text_conversion(self):
        (self.repo / ".gitattributes").write_text("*.bin diff=conv\n", encoding="utf-8")
        git(self.repo, "config", "diff.conv.textconv", "echo converted")
        git(self.repo, "add", ".gitattributes")
        git(self.repo, "commit", "-qm", "attributes")
        code, out = self.run_wrapper("binary", contract(["python", "-c", "pass"]))
        self.assertEqual(code, 0, out)
        meta, out_dir = self.meta()
        wt = codex_review.WORKTREES / meta["call_id"]
        git(self.repo, "apply", str(out_dir / "changes.patch"))
        self.assertEqual((self.repo / "src" / "data.bin").read_bytes(), (wt / "src" / "data.bin").read_bytes())
        (self.repo / ".gitattributes").write_text("*.bin filter=evil\n", encoding="utf-8")
        git(self.repo, "add", ".")
        git(self.repo, "commit", "-qm", "filter")
        code, out = self.run_wrapper("success")
        self.assertEqual(code, 2)
        self.assertIn("filter attribute", out)

    def test_attribute_changes_and_unfaithful_patches_invalidate(self):
        for name in (".gitattributes", ".GITATTRIBUTES", "src/.GitAttributes"):
            with self.subTest(name=name):
                os.environ["FAKE_ATTRIBUTES_FILE"] = name
                self.assert_invalid("attributes", f"attribute files belong to the integrator: {name}",
                                    contract(allowed=["src/*", ".gitattributes", ".GITATTRIBUTES"]))
        original = codex_review.worktree_patch
        codex_review.worktree_patch = lambda wt, base, index: b""
        try:
            code, out = self.run_wrapper("success")
        finally:
            codex_review.worktree_patch = original
        self.assertEqual(code, 2, out)
        self.assertIn("does not reproduce the candidate", out)
        self.assertIsNone(self.meta()[0]["acceptance"])

    def test_acceptance_home_must_stay_empty_of_config(self):
        home = codex_review.WORKTREES.parent / "codex-sandbox-home"
        home.mkdir(parents=True)
        (home / "config.toml").write_text("[permissions]\n", encoding="utf-8")
        code, out = self.run_wrapper("success")
        self.assertEqual(code, 2)
        self.assertIn("config.toml must not exist", out)

    def test_cleanup_never_follows_a_replaced_root_or_prunes_other_worktrees(self):
        other = Path(self.tmp.name).resolve() / "other-wt"
        git(self.repo, "worktree", "add", "-q", "-b", "other", str(other))
        force_rmtree(other)  # an unrelated worktree that is temporarily missing
        self.run_wrapper("success")
        cid = self.meta()[0]["call_id"]
        wt = codex_review.WORKTREES / cid
        inner = self.link_target / "inner-link"
        make_link(Path(self.tmp.name), inner)
        moved = wt.with_name(cid + "-moved")
        os.rename(wt, moved)
        make_link(self.link_target, wt)
        code, out = self.call("--cleanup", cid)
        self.assertEqual(code, 2)
        self.assertIn("is a link", out)
        self.assertTrue(codex_review._is_link(inner) and (self.link_target / "keep.txt").is_file())
        os.rmdir(wt)
        os.rename(moved, wt)
        self.cleanup_last()
        self.assertIn(other.as_posix(), git(self.repo, "worktree", "list", "--porcelain"))  # not pruned
        self.run_wrapper("success")
        force_rmtree(codex_review.WORKTREES / self.meta()[0]["call_id"])
        code, out = self.call("--cleanup", self.meta()[0]["call_id"])
        self.assertEqual(code, 2)
        self.assertIn("registered but missing", out)

    def test_cleanup_refusals(self):
        with contextlib.redirect_stderr(io.StringIO()):
            for bad in ("../x", "20260101T000000Z-deadbeef"):
                self.assertEqual(codex_review.main(["--cleanup", bad]), 2)
            review_dir = codex_review.REVIEWS / "20260101T000000Z-0badc0de"
            review_dir.mkdir(parents=True)
            (review_dir / "meta.json").write_text(json.dumps({"kind": "review"}), encoding="utf-8")
            self.assertEqual(codex_review.main(["--cleanup", "20260101T000000Z-0badc0de"]), 2)
            with self.assertRaises(SystemExit):
                codex_review.main(["--cleanup", "20260101T000000Z-0badc0de", "--kind", "implement"])


@unittest.skipUnless(os.name == "nt" and shutil.which("codex"), "needs the installed Codex Windows sandbox runner")
class RealSandboxTests(WrapperFixture):
    """The sandbox runner itself must deny what the wrapper relies on it to deny. No model call is made."""

    PROBE = r'''
import json, os, subprocess, sys
root, common, remote = sys.argv[1], sys.argv[2], sys.argv[3]
def attempt(fn):
    try:
        fn()
        return "allowed"
    except Exception as exc:
        return "denied"
def write(path):
    with open(path, "a", encoding="utf-8") as f:
        f.write("probe\n")
def run(*cmd):
    subprocess.run(cmd, check=True, capture_output=True)
print(json.dumps({
    "argv": sys.argv[4:],
    "worktree": attempt(lambda: write("inside.txt")),
    "main_worktree": attempt(lambda: write(os.path.join(root, "canary.txt"))),
    "git_config": attempt(lambda: write(os.path.join(common, "config"))),
    "git_hooks": attempt(lambda: write(os.path.join(common, "hooks", "pre-commit"))),
    "commit": attempt(lambda: run("git", "-c", "user.email=p@example.invalid", "-c", "user.name=p", "commit", "-q", "--allow-empty", "-m", "p")),
    "push_by_path": attempt(lambda: run("git", "push", "-q", remote, "HEAD:refs/heads/probe")),
    "push_with_override": attempt(lambda: run("git", "-c", "protocol.allow=always", "push", "-q", remote, "HEAD:refs/heads/probe2")),
}))
'''

    def setUp(self):
        self.area = ROOT / "work" / f"sandbox-probe-{uuid.uuid4().hex[:8]}"  # outside the temp directory
        self.repo = self.area / "repo"
        make_repo(self.repo)
        git(self.area, "init", "-q", "--bare", "remote.git")
        self.patch_wrapper(self.repo, self.area)

    def tearDown(self):
        self.restore_wrapper()
        force_rmtree(self.area)

    def test_sandbox_denies_writes_outside_the_worktree(self):
        wt, _base, paths = codex_review.create_worktree("20990101T000000Z-5a4db0c5")
        probe = self.area / "probe.py"
        probe.write_text(self.PROBE, encoding="utf-8")
        before = git(self.area / "remote.git", "for-each-ref")
        tricky = ["a&echo(INJECTED", "b|c", '"q"', "d^e"]
        cmd = codex_review.sandbox_command(codex_review.codex_command(), wt, [sys.executable, str(probe), str(self.repo),
                                           str(paths["common"]), str(self.area / "remote.git"), *tricky])
        code, out, err = codex_review.run_bounded(cmd, "", 120, cwd=wt, env=codex_review.sandbox_env())
        self.assertEqual(code, 0, err)
        self.assertNotIn("INJECTED\n", out)
        result = json.loads(out.strip().splitlines()[-1])
        self.assertEqual(result, {"argv": tricky, "worktree": "allowed", "main_worktree": "denied", "git_config": "denied",
                                  "git_hooks": "denied", "commit": "denied", "push_by_path": "denied",
                                  "push_with_override": "denied"})
        self.assertFalse((self.repo / "canary.txt").exists())
        self.assertEqual(git(self.area / "remote.git", "for-each-ref"), before)


if __name__ == "__main__":
    unittest.main(verbosity=2)
