"""Exercise the acceptance harness without running nested .NET builds."""
import contextlib
import importlib.util
import io
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch
from uuid import uuid4

spec = importlib.util.spec_from_file_location("looklab_check", Path(__file__).resolve().parents[1] / "check.py")
check = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check)


class HarnessTests(unittest.TestCase):
    def test_arguments_refused_before_mkdir(self):
        with patch.object(check.Path, "mkdir") as mkdir, contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(check.main(["--godot"]), 2)
            mkdir.assert_not_called()

    def test_commands_env_and_cleanup(self):
        seen = []

        def run(command, **kwargs):
            env = kwargs["env"]
            temporary = Path(env["DOTNET_CLI_HOME"]).parent
            self.assertTrue(temporary.name.startswith("looklab-check-"))
            self.assertTrue(temporary.is_dir())
            for key in ("DOTNET_CLI_HOME", "NUGET_PACKAGES", "NUGET_HTTP_CACHE_PATH", "NUGET_PLUGINS_CACHE_PATH"):
                self.assertTrue(Path(env[key]).is_dir())
                self.assertEqual(Path(env[key]).parent, temporary)
            for key in ("DOTNET_CLI_TELEMETRY_OPTOUT", "DOTNET_NOLOGO", "DOTNET_SKIP_FIRST_TIME_EXPERIENCE",
                        "MSBUILDDISABLENODEREUSE", "DOTNET_CLI_DO_NOT_USE_MSBUILD_SERVER"):
                self.assertEqual(env[key], "1")
            self.assertEqual(kwargs["cwd"], check.ROOT)
            seen.append((command, temporary))
            return subprocess.CompletedProcess(command, 0, "all tests passed\n")

        with patch.object(check.subprocess, "run", side_effect=run), patch.object(check, "snapshot", return_value={}), contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(check.main([]), 0)
        self.assertEqual(len(seen), 3)
        temporary = seen[0][1]
        art = str(temporary / "art")
        project = "tools/looklab/tests/LookLab.Tests.csproj"
        self.assertEqual(seen[0][0], ["dotnet", "restore", project, "--configfile", "tools/looklab/nuget.config", "--artifacts-path", art, "-nodeReuse:false"])
        self.assertEqual(seen[1][0], ["dotnet", "build", project, "--no-restore", "--artifacts-path", art, "-c", "Debug", "-nologo", "-nodeReuse:false", "-p:UseSharedCompilation=false", "--disable-build-servers"])
        self.assertEqual(seen[2][0], ["dotnet", str(temporary / "art/bin/LookLab.Tests/debug/LookLab.Tests.dll"), str(check.ROOT)])
        self.assertFalse(temporary.exists())

    def test_failure_code_tail_and_cleanup(self):
        paths = []

        def fail(command, **kwargs):
            paths.append(Path(kwargs["env"]["DOTNET_CLI_HOME"]).parent)
            return subprocess.CompletedProcess(command, 17, "\n".join(f"tail-{i:03}" for i in range(100)))

        output = io.StringIO()
        with patch.object(check.subprocess, "run", side_effect=fail) as run, patch.object(check, "snapshot", return_value={}), contextlib.redirect_stdout(output):
            self.assertEqual(check.main([]), 17)
            self.assertEqual(run.call_count, 1)
        self.assertNotIn("tail-000", output.getvalue())
        self.assertIn("tail-040", output.getvalue())
        self.assertIn("tail-099", output.getvalue())
        self.assertFalse(paths[0].exists())

    def test_launch_error_is_reported(self):
        output = io.StringIO()
        with patch.object(check.subprocess, "run", side_effect=OSError("exact sandbox error")), patch.object(check, "snapshot", return_value={}), contextlib.redirect_stdout(output):
            self.assertEqual(check.main([]), 1)
        self.assertIn("exact sandbox error", output.getvalue())

    def test_cleanup_error_names_folder_once(self):
        folders = []

        def removal(path):
            folders.append(path)
            raise OSError("denied")

        output = io.StringIO()
        try:
            with patch.object(check.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, "")), patch.object(check, "snapshot", return_value={}), patch.object(check.shutil, "rmtree", side_effect=removal), contextlib.redirect_stdout(output):
                self.assertEqual(check.main([]), 0)
            self.assertEqual(output.getvalue().count("could not remove"), 1)
            self.assertIn(str(folders[0]), output.getvalue())
        finally:
            for folder in folders:
                shutil.rmtree(folder)

    def test_ignored_file_and_directory_change_fail(self):
        root = Path(tempfile.gettempdir()) / f"looklab-harness-test-{uuid4().hex}"
        root.mkdir()

        def change(command, **kwargs):
            (root / "ignored").mkdir(exist_ok=True)
            (root / "ignored/build.bin").write_bytes(b"unexpected build output")
            return subprocess.CompletedProcess(command, 0, "")

        try:
            before = check.snapshot(root)
            output = io.StringIO()
            with patch.object(check, "ROOT", root), patch.object(check.subprocess, "run", side_effect=change), contextlib.redirect_stdout(output):
                self.assertEqual(check.main([]), 1)
            self.assertIn("ignored/build.bin", output.getvalue())
            self.assertNotEqual(check.snapshot(root), before)
            (root / "empty").mkdir()
            self.assertIn("empty/", check.snapshot(root))
        finally:
            shutil.rmtree(root)


if __name__ == "__main__":
    unittest.main(verbosity=2)
