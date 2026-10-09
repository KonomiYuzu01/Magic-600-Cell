"""Harness decisions and negative controls without Godot or nested .NET builds."""
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch
from uuid import uuid4

spec = importlib.util.spec_from_file_location("looklab_check", Path(__file__).resolve().parents[1] / "check.py")
check = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check)


class HarnessTests(unittest.TestCase):
    def setUp(self):
        self.folder = Path(tempfile.gettempdir()) / f"looklab-harness-test-{uuid4().hex}"
        self.folder.mkdir()

    def tearDown(self):
        shutil.rmtree(self.folder)

    def reports(self, root=None):
        root = root or self.folder
        return [f"LOOKLAB_DIR {kind} {json.dumps(str(root / kind))}\n" for kind in sorted(check.DIRECTORIES)] + ["LOOKLAB_DIRS_END\n"]

    def test_arguments_refused_before_mkdir(self):
        with patch.object(check.Path, "mkdir") as mkdir, contextlib.redirect_stderr(io.StringIO()):
            for args in (["extra"], ["--godot", "--godot-gpu"], ["--help"], ["--run", "path"]):
                self.assertEqual(check.main(args), 2)
            mkdir.assert_not_called()

    def test_commands_env_and_cleanup(self):
        seen = []

        def run(command, **kwargs):
            env = kwargs["env"]
            temporary = Path(env["DOTNET_CLI_HOME"]).parent
            self.assertTrue(temporary.name.startswith("looklab-check-"))
            self.assertTrue(temporary.is_dir())
            for key in ("DOTNET_CLI_HOME", "NUGET_PACKAGES", "NUGET_HTTP_CACHE_PATH", "NUGET_PLUGINS_CACHE_PATH", "APPDATA", "LOCALAPPDATA", "TEMP", "TMP"):
                self.assertTrue(Path(env[key]).is_dir())
                self.assertEqual(Path(env[key]).parent, temporary)
            for key in ("DOTNET_CLI_TELEMETRY_OPTOUT", "DOTNET_NOLOGO", "DOTNET_SKIP_FIRST_TIME_EXPERIENCE", "MSBUILDDISABLENODEREUSE", "DOTNET_CLI_DO_NOT_USE_MSBUILD_SERVER", "PYTHONDONTWRITEBYTECODE"):
                self.assertEqual(env[key], "1")
            self.assertNotIn("LOOKLAB_TEST_TOKEN", env)
            self.assertEqual(env["LOOKLAB_PYTHON"], sys.executable)
            self.assertEqual(kwargs["cwd"], check.ROOT)
            self.assertEqual(kwargs["timeout"], check.BUILD_TIMEOUT)
            seen.append((command, temporary))
            return subprocess.CompletedProcess(command, 0, "all tests passed\n")

        with patch.dict(os.environ, LOOKLAB_TEST_TOKEN="synthetic"), patch.object(check, "run_process", side_effect=run), patch.object(check, "snapshot", return_value={}), contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(check.main([]), 0)
        self.assertEqual(len(seen), 3)
        for command, _ in seen[:2]:
            for flag in ("-nodeReuse:false", "-p:UseSharedCompilation=false", "--disable-build-servers"):
                self.assertIn(flag, command)
            self.assertIn("--artifacts-path", command)
        self.assertEqual(seen[2][0][-1], str(check.ROOT))
        self.assertFalse(seen[0][1].exists())

    def test_failure_is_normalized_and_cleanup(self):
        paths = []

        def fail(command, **kwargs):
            paths.append(Path(kwargs["env"]["DOTNET_CLI_HOME"]).parent)
            return subprocess.CompletedProcess(command, 17, "exact child diagnostic\n")

        output = io.StringIO()
        with patch.object(check, "run_process", side_effect=fail) as run, patch.object(check, "snapshot", return_value={}), contextlib.redirect_stdout(output):
            self.assertEqual(check.main([]), 1)
            self.assertEqual(run.call_count, 1)
        self.assertIn("exact child diagnostic", output.getvalue())
        self.assertFalse(paths[0].exists())

    def test_run_mode_allows_only_the_app_output_folder(self):
        before = {"./": "directory", "work/": "directory", "tools/a.txt": "x"}
        saved = dict(before, **{"work/loop-memory/": "directory", "work/loop-memory/looklab/": "directory",
                                "work/loop-memory/looklab/presets/": "directory",
                                "work/loop-memory/looklab/presets/look.json": "y"})
        stray = dict(saved, **{"work/other.txt": "z"})
        for args, after, expected in ((["--run"], saved, 0), (["--run"], stray, 1), (["--godot"], saved, 1)):
            with patch.object(check, "core_check"), patch.object(check, "godot_lane"), patch.object(check, "snapshot", side_effect=[before, after]), contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(check.main(args), expected, args)

    def test_launch_error_is_reported(self):
        with patch.object(check.subprocess, "Popen", side_effect=OSError("exact sandbox error")), contextlib.redirect_stdout(io.StringIO()):
            with self.assertRaisesRegex(check.CheckFailure, "exact sandbox error"):
                check.run_process(["unavailable"], cwd=self.folder, env={}, timeout=1)

    def test_cleanup_failure_fails_and_names_folder(self):
        folders = []

        def removal(path):
            folders.append(path)
            raise OSError("denied")

        output = io.StringIO()
        try:
            with patch.object(check, "run_process", return_value=subprocess.CompletedProcess([], 0, "")), patch.object(check, "snapshot", return_value={}), patch.object(check.shutil, "rmtree", side_effect=removal), contextlib.redirect_stdout(output):
                self.assertEqual(check.main([]), 1)
            self.assertEqual(output.getvalue().count("could not remove"), 1)
            self.assertIn(str(folders[0]), output.getvalue())
        finally:
            for folder in folders:
                shutil.rmtree(folder)

    def test_ignored_file_and_directory_change_fail(self):
        def change(command, **kwargs):
            (self.folder / "ignored").mkdir(exist_ok=True)
            (self.folder / "ignored/build.bin").write_bytes(b"unexpected build output")
            return subprocess.CompletedProcess(command, 0, "")

        output = io.StringIO()
        with patch.object(check, "ROOT", self.folder), patch.object(check, "run_process", side_effect=change), contextlib.redirect_stdout(output):
            self.assertEqual(check.main([]), 1)
        self.assertIn("ignored/build.bin", output.getvalue())
        (self.folder / "empty").mkdir()
        self.assertIn("empty/", check.snapshot(self.folder))

    def test_credential_names_case_insensitive(self):
        self.assertEqual(check.scoped_env({"LOOKLAB_TEST_TOKEN": "fake", "lower_key": "fake", "Secret": "fake", "PATH": "kept"}), {"PATH": "kept"})

    def test_real_child_and_descendant_receive_no_credentials(self):
        # Only synthetic values are printed. The grandchild follows the test
        # runner's python/node inheritance path without invoking either engine.
        code = "import os,subprocess,sys; print(os.environ.get('LOOKLAB_TEST_TOKEN','absent')); subprocess.run([sys.executable,'-c',\"import os; print(os.environ.get('LOOKLAB_TEST_TOKEN','absent'))\"],check=True)"
        env = dict(check.scoped_env(), LOOKLAB_TEST_TOKEN="synthetic")
        with contextlib.redirect_stdout(io.StringIO()):
            run = check.run_process([sys.executable, "-B", "-c", code], cwd=self.folder, env=env, timeout=10)
        self.assertEqual(run.returncode, 0)
        self.assertEqual(run.stdout.splitlines(), ["absent", "absent"])

    def test_directory_report_inside(self):
        guard = check.DirectoryGuard(self.folder)
        for line in self.reports():
            guard.line(line)
        guard.finish()
        self.assertTrue(guard.complete)

    def test_directory_outside_stops_immediately(self):
        guard = check.DirectoryGuard(self.folder)
        with self.assertRaisesRegex(check.CheckFailure, "outside"):
            guard.line(f"LOOKLAB_DIR user {json.dumps(str(self.folder.parent / 'other'))}")

    def test_directory_missing_line_and_marker_fail(self):
        for lines in (self.reports()[1:], self.reports()[:-1]):
            guard = check.DirectoryGuard(self.folder)
            with self.assertRaisesRegex(check.CheckFailure, "missing"):
                for line in lines:
                    guard.line(line)
                guard.finish()

    def test_directory_relative_repeated_and_malformed_fail(self):
        for lines in (["LOOKLAB_DIR user \"relative\""], [self.reports()[0], self.reports()[0]], ["LOOKLAB_DIR user invalid"]):
            guard = check.DirectoryGuard(self.folder)
            with self.assertRaises(check.CheckFailure):
                for line in lines:
                    guard.line(line)

    def test_real_appdata_missing_unchanged(self):
        folder = self.folder / "missing"
        check.verify_real_data([folder], [None], "LookLabCheck-test")

    def test_real_appdata_changed_listing_fails(self):
        folder = self.folder / "Godot"
        folder.mkdir()
        (folder / "editor").mkdir()
        recorded = [("editor", 1)]
        with self.assertRaisesRegex(check.CheckFailure, "listing changed"):
            check.verify_real_data([folder], [recorded], "LookLabCheck-test")

    def test_real_appdata_run_folder_fails(self):
        folder = self.folder / "Godot"
        folder.mkdir()
        (folder / "app_userdata/LookLabCheck-test").mkdir(parents=True)
        with self.assertRaisesRegex(check.CheckFailure, "run's user directory"):
            check.verify_real_data([folder], [check.listing(folder)], "LookLabCheck-test")

    def test_finder_zero_one_multiple_and_override(self):
        env = {"LOCALAPPDATA": str(self.folder)}
        with self.assertRaisesRegex(check.CheckFailure, "found 0"):
            check.find_godot(env)
        first = self.folder / "Microsoft/WinGet/Packages/GodotEngine.GodotEngine.Mono_A/nested" / check.GODOT_EXE
        first.parent.mkdir(parents=True)
        first.write_bytes(b"synthetic")
        self.assertEqual(check.find_godot(env), first.resolve())
        second = first.parents[1].parent / "GodotEngine.GodotEngine.Mono_B" / check.GODOT_EXE
        second.parent.mkdir()
        second.write_bytes(b"synthetic")
        with self.assertRaisesRegex(check.CheckFailure, "found 2"):
            check.find_godot(env)
        self.assertEqual(check.find_godot(dict(env, LOOKLAB_GODOT=str(first))), first.resolve())
        with self.assertRaisesRegex(check.CheckFailure, "console executable"):
            check.find_godot(dict(env, LOOKLAB_GODOT=str(self.folder / "missing.exe")))

    def test_version_refuses_wrong_build(self):
        check.check_version("4.7.2.stable.mono.official.abc\n")
        for output in ("4.7.2.stable", "4.7.1.stable.mono", "4.7.20.stable.mono"):
            with self.assertRaises(check.CheckFailure):
                check.check_version(output)

    def test_staging_sources_only_and_local_config(self):
        root = self.folder / "checkout"
        lab = root / "tools/looklab"
        for name in ("app", "core"):
            (lab / name).mkdir(parents=True)
            (lab / name / "source.cs").write_text("source")
            for excluded in ("bin", "obj", ".godot"):
                (lab / name / excluded).mkdir()
                (lab / name / excluded / "product").write_text("ignored")
        (lab / "Directory.Build.props").write_text("<Project />")
        (lab / "app/project.godot").write_text('config/name="LookLab"\n')
        temp = self.folder / "stage-root"
        temp.mkdir()
        before = check.snapshot(root)
        with patch.object(check, "ROOT", root):
            stage = check.stage_project(temp, self.folder / "nupkgs", "LookLabCheck-test")
        for name in ("app", "core"):
            self.assertTrue((stage / name / "source.cs").exists())
            for excluded in ("bin", "obj", ".godot"):
                self.assertFalse((stage / name / excluded).exists())
        self.assertIn("LookLabCheck-test", (stage / "app/project.godot").read_text())
        self.assertIn("<clear />", (stage / "nuget.config").read_text())
        self.assertNotIn("https:", (stage / "nuget.config").read_text())
        self.assertEqual(check.snapshot(root), before)

    def package_cache(self):
        cache, source = self.folder / "packages", self.folder / "nupkgs"
        cache.mkdir(); source.mkdir()
        for name in ("godot.net.sdk", "godot.sourcegenerators", "godotsharp"):
            version = cache / name / "4.7.2"
            version.mkdir(parents=True)
            (version / ".nupkg.metadata").write_text(json.dumps({"source": str(source)}))
            archive = name + ".4.7.2.nupkg"
            (source / archive).write_bytes(name.encode())
            (version / archive).write_bytes(name.encode())
        return cache, source

    def test_package_audit_good_and_wrong_source(self):
        cache, source = self.package_cache()
        check.audit_packages(cache, source)
        (cache / "godotsharp/4.7.2/.nupkg.metadata").write_text(json.dumps({"source": str(self.folder / "other")}))
        with self.assertRaisesRegex(check.CheckFailure, "another source"):
            check.audit_packages(cache, source)

    def test_package_audit_version_archive_missing_and_extra(self):
        cache, source = self.package_cache()
        version = cache / "godotsharp/4.7.2"
        (version / "godotsharp.4.7.2.nupkg").write_bytes(b"changed")
        with self.assertRaisesRegex(check.CheckFailure, "differs"):
            check.audit_packages(cache, source)
        (version / "godotsharp.4.7.2.nupkg").write_bytes(b"godotsharp")
        (cache / "godotsharp/4.7.1").mkdir()
        with self.assertRaisesRegex(check.CheckFailure, "4.7.2"):
            check.audit_packages(cache, source)
        shutil.rmtree(cache / "godotsharp")
        with self.assertRaisesRegex(check.CheckFailure, "missing cached"):
            check.audit_packages(cache, source)
        (cache / "unapproved").mkdir()
        with self.assertRaisesRegex(check.CheckFailure, "unexpected"):
            check.audit_packages(cache, source)

    def test_graphics_evidence_requires_real_driver(self):
        def report(display="windows", driver="vulkan", adapter="Synthetic adapter"):
            return "LOOKLAB_GRAPHICS " + json.dumps(dict(display=display, driver=driver, adapter=adapter))
        check.graphics_report(report())
        check.graphics_report(report(driver="d3d12"))
        for output in (report(display="headless"), report(driver="dummy"), report(adapter=""), ""):
            with self.assertRaises(check.CheckFailure):
                check.graphics_report(output)

    def test_graphics_unavailable_requires_startup_evidence(self):
        self.assertTrue(check.graphics_startup_unavailable("ERROR: Failed to create window"))
        for output in ("headless dummy", "adapter empty", "script failed", ""):
            self.assertFalse(check.graphics_startup_unavailable(output))

    def test_injected_fault_must_fail_its_own_check(self):
        good_dirs = "".join(self.reports())
        graphics = 'LOOKLAB_GRAPHICS {"display":"windows","driver":"vulkan","adapter":"synthetic"}\n'

        def run(command, **kwargs):
            for line in good_dirs.splitlines():
                kwargs["guard"].line(line)
            return subprocess.CompletedProcess(command, code, good_dirs + graphics + message)

        with patch.object(check, "run_process", side_effect=run), contextlib.redirect_stdout(io.StringIO()):
            for code, message in ((0, "LOOKLAB_FAIL float-offset"), (1, "LOOKLAB_FAIL unrelated")):
                with self.assertRaisesRegex(check.CheckFailure, "did not fail its own"):
                    check.godot_run("fake", self.folder, self.folder, {}, [], graphics=True, expected_failure="float-offset")
            code, message = 1, "LOOKLAB_FAIL float-offset: mismatch"
            check.godot_run("fake", self.folder, self.folder, {}, [], graphics=True, expected_failure="float-offset")

    def test_lane_routing_and_exit_codes(self):
        with patch.object(check, "core_check") as core, patch.object(check, "godot_lane") as lane, patch.object(check, "snapshot", return_value={}), contextlib.redirect_stdout(io.StringIO()):
            for mode in ("--godot", "--godot-gpu", "--run"):
                self.assertEqual(check.main([mode]), 0)
                self.assertEqual(lane.call_args.args[0], mode)
            self.assertEqual(core.call_count, 2)
            lane.side_effect = check.GraphicsUnavailable("graphics lane not run")
            self.assertEqual(check.main(["--godot-gpu"]), 3)
            lane.side_effect = check.CheckFailure("dummy renderer")
            self.assertEqual(check.main(["--godot-gpu"]), 1)

    def test_timeout_ends_process_tree(self):
        process = Mock(pid=12345)
        process.poll.return_value = None
        with patch.object(check.os, "name", "nt"), patch.object(check.subprocess, "run") as kill:
            check.end_tree(process, {"LOOKLAB_TEST_TOKEN": "synthetic", "PATH": "kept"})
        self.assertEqual(kill.call_args.args[0], ["taskkill", "/PID", "12345", "/T", "/F"])
        self.assertEqual(kill.call_args.kwargs["env"], {"PATH": "kept"})
        process.wait.assert_called_once_with(timeout=15)

    def test_cpu_and_gpu_lane_schedules(self):
        godot = self.folder / check.GODOT_EXE
        (self.folder / "GodotSharp/Tools/nupkgs").mkdir(parents=True)
        runs = []

        def process(command, **kwargs):
            return subprocess.CompletedProcess(command, 0, check.GODOT_VERSION + "\n" if "--version" in command else "")

        def godot_run(executable, stage, temp, env, arguments, **kwargs):
            runs.append((arguments, kwargs))
            output = ("LOOKLAB_STAGE0_CPU_PASS\nLOOKLAB_STAGE0_GPU_PASS\n"
                      "LOOKLAB_SMOKE_PASS instances=600 vertices=30480 slots=259800 parameters=42 bytes=123\n"
                      "LOOKLAB_GEOMETRY_PASS cases=9 samples=2700 target=RGBA32F\n"
                      "LOOKLAB_DRAW_COUNT_PASS instances=600 visible=600 primitives=6096000\n")
            return subprocess.CompletedProcess(arguments, 0, output)

        with patch.object(check, "find_godot", return_value=godot), patch.object(check, "stage_project", return_value=self.folder), patch.object(check, "audit_packages"), patch.object(check, "run_process", side_effect=process), patch.object(check, "godot_run", side_effect=godot_run), contextlib.redirect_stdout(io.StringIO()):
            check.godot_lane("--godot", self.folder, {"NUGET_PACKAGES": str(self.folder)}, {})
            self.assertTrue(all("--headless" in args for args, _ in runs))
            self.assertEqual([kwargs.get("expected_failure") for _, kwargs in runs if "expected_failure" in kwargs], ["skip-parameter", "wrong-instance-count", "changed-preset-byte"])
            runs.clear()
            check.godot_lane("--godot-gpu", self.folder, {"NUGET_PACKAGES": str(self.folder)}, {})
        self.assertEqual(len(runs), 6)
        self.assertEqual(runs[0][0], ["--headless", "--import"])
        self.assertTrue(all("--headless" not in args and kwargs["graphics"] for args, kwargs in runs[1:]))
        self.assertEqual([kwargs["expected_failure"] for _, kwargs in runs if "expected_failure" in kwargs], ["float-offset", "transpose-q", "missing-instance"])

    def test_graphics_launch_refusal_is_not_a_pass(self):
        run = subprocess.CompletedProcess([], 1, "ERROR: Failed to create window\n")
        with patch.object(check, "run_process", return_value=run), contextlib.redirect_stdout(io.StringIO()):
            with self.assertRaises(check.GraphicsUnavailable):
                check.godot_run("fake", self.folder, self.folder, {}, [], graphics=True)
        run = subprocess.CompletedProcess([], 1, 'ERROR: Failed to create window\nLOOKLAB_GRAPHICS {"display":"headless","driver":"dummy","adapter":""}\n')
        with patch.object(check, "run_process", return_value=run), contextlib.redirect_stdout(io.StringIO()):
            with self.assertRaises(check.CheckFailure) as failure:
                check.godot_run("fake", self.folder, self.folder, {}, [], graphics=True)
            self.assertNotIsInstance(failure.exception, check.GraphicsUnavailable)

    def test_live_guard_acknowledges_and_rejects_unsafe_path(self):
        # Execute only a tiny Python child. It waits exactly like the Godot
        # scripts, proving that the guard is applied while the process lives.
        code = "import sys; " + "; ".join("print(" + repr(line.strip()) + ",flush=True)" for line in self.reports()) + "; print(sys.stdin.readline().strip(),flush=True)"
        with contextlib.redirect_stdout(io.StringIO()):
            guard = check.DirectoryGuard(self.folder)
            run = check.run_process([sys.executable, "-B", "-c", code], cwd=self.folder, env=check.scoped_env(), timeout=10, guard=guard)
        guard.finish()
        self.assertEqual(run.stdout.splitlines()[-1], "LOOKLAB_DIRECTORY_GUARD_OK")
        unsafe = "import time; print(" + repr('LOOKLAB_DIR user ' + json.dumps(str(self.folder.parent / "outside"))) + ",flush=True); time.sleep(60)"
        with contextlib.redirect_stdout(io.StringIO()), patch.object(check, "end_tree") as kill:
            # Keep the termination real, so this negative test leaves no child.
            kill.side_effect = lambda process, env: (process.kill(), process.wait())
            with self.assertRaisesRegex(check.CheckFailure, "outside"):
                check.run_process([sys.executable, "-B", "-c", unsafe], cwd=self.folder, env=check.scoped_env(), timeout=10, guard=check.DirectoryGuard(self.folder))
            kill.assert_called_once()


if __name__ == "__main__":
    unittest.main(verbosity=2)
