"""Static checks of C9's product shell project (renderer/shell); never builds, launches Godot or touches a GPU.

The shell builds with renderer/shell/build_shell.py on a machine with the pinned Godot 4.7.2 .NET install; this check
runs everywhere and keeps the project's fixed points: Godot.NET.Sdk 4.7.2 on net10.0 (ADR-R01.5), an offline
package source only, the route-B window settings, and the core's export list and ABI version equal to the bridge's.
A pass is source evidence only.
"""
from __future__ import annotations

import importlib
import json
import re
import shutil
import sys
import tempfile
import unittest
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SHELL = ROOT / "renderer" / "shell"
PROJECT = SHELL / "project"


def text(path: Path) -> str:
    return path.read_bytes().decode("utf-8")


def godot_settings(source: str) -> dict[str, str]:
    """project.godot as section/key -> raw value."""
    settings, section = {}, ""
    for line in source.splitlines():
        line = line.strip()
        if not line or line.startswith(";"):
            continue
        if line.startswith("[") and line.endswith("]"):
            section = line[1:-1]
        elif "=" in line:
            key, value = line.split("=", 1)
            settings[f"{section}/{key}" if section else key] = value
    return settings


def rust_symbols() -> list[str]:
    source = text(ROOT / "renderer" / "bridge" / "src" / "lib.rs")
    block = re.search(r"pub const SYMBOLS: \[&str; (\d+)\] = \[(.*?)\];", source, re.S)
    assert block, "SYMBOLS in renderer/bridge/src/lib.rs"
    names = re.findall(r'"(sa2_[a-z_]+)"', block.group(2))
    assert len(names) == int(block.group(1))
    return names


def csharp_list(name: str, file: str = "CoreLibrary.cs") -> list[str]:
    source = text(PROJECT / file)
    block = re.search(rf"\b{name} = new\[\]\s*\{{(.*?)\}};", source, re.S)
    assert block, f"{name} in {file}"
    return re.findall(r'"((?:sa2|m6h)_[a-z_0-9]+)"', block.group(1))


def csharp_symbols() -> list[str]:
    return csharp_list("Symbols")


def header_exports() -> set[str]:
    header = text(ROOT / "renderer" / "core" / "include" / "sa2_interop.h")
    return set(re.findall(r"^SA2_API [^(]*?\b(sa2_[a-z0-9_]+)\(", header, re.M))


class ShellProject(unittest.TestCase):
    def test_the_project_is_godot_472_on_net10(self):
        root = ET.fromstring(text(PROJECT / "Magic600Shell.csproj"))
        self.assertEqual(root.get("Sdk"), "Godot.NET.Sdk/4.7.2")
        frameworks = [e.text for e in root.iter("TargetFramework")]
        self.assertEqual(frameworks, ["net10.0"])
        settings = godot_settings(text(PROJECT / "project.godot"))
        self.assertEqual(settings["dotnet/project/assembly_name"], '"Magic600Shell"')
        self.assertTrue(text(SHELL / "build_shell.py").count('ASSEMBLY = ".godot/mono/temp/bin/Debug/Magic600Shell.dll"'))

    def test_packages_come_from_the_godot_install_only(self):
        root = ET.fromstring(text(PROJECT / "nuget.config"))
        sources = root.find("packageSources")
        self.assertEqual([child.tag for child in sources], ["clear", "add"])
        self.assertEqual(sources[1].get("value"), "%M600_GODOT_NUPKGS%")

    def test_the_window_lets_the_core_layer_show_beneath(self):
        settings = godot_settings(text(PROJECT / "project.godot"))
        for key, value in {
            "rendering/rendering_device/driver.windows": '"d3d12"',
            "rendering/rendering_device/fallback_to_vulkan": "false",
            "rendering/rendering_device/fallback_to_opengl3": "false",
            "display/window/per_pixel_transparency/allowed": "true",
            "display/window/size/transparent": "true",
            "rendering/viewport/transparent_background": "true",
            "rendering/environment/defaults/default_clear_color": "Color(0, 0, 0, 0)",
        }.items():
            self.assertEqual(settings.get(key), value, key)
        self.assertIn("route B, accepted for now by the owner on 18 Nov 2026, pending three owner-machine checks",
                      text(PROJECT / "project.godot"))

    def test_the_core_exports_and_abi_match_the_bridge(self):
        self.assertEqual(csharp_symbols(), rust_symbols())
        shell_abi = re.search(r"public const uint AbiVersion = (\d+);", text(PROJECT / "CoreLibrary.cs"))
        rust_abi = re.search(r"pub const ABI_VERSION: u32 = (\d+);", text(ROOT / "rust/c600-render/src/abi.rs"))
        header = re.search(r"#define SA2_ABI_VERSION (\d+)u", text(ROOT / "renderer/core/include/sa2_interop.h"))
        self.assertTrue(shell_abi and rust_abi and header)
        self.assertEqual({shell_abi.group(1), rust_abi.group(1), header.group(1)}, {header.group(1)})

    def test_the_shell_calls_are_route_b_exports_outside_the_table(self):
        calls = csharp_list("ShellCalls")
        exports = header_exports()
        self.assertIn("sa2_frame_submit", exports, "the header's exports are read")
        self.assertEqual(len(calls), len(set(calls)))
        for name in ["sa2_create", "sa2_surface_attach", "sa2_surface_move", "sa2_surface_detach"]:
            self.assertIn(name, calls)
        for name in calls:
            self.assertIn(name, exports, name)
            self.assertNotIn(name, rust_symbols(), name)

    def test_the_host_library_calls_match_its_rust_source(self):
        source = text(ROOT / "renderer" / "bridge" / "host" / "src" / "lib.rs")
        csharp = text(PROJECT / "HostLibrary.cs")
        exports = re.findall(r'#\[no_mangle\]\s*pub (?:unsafe )?extern "C" fn (m6h_[a-z_]+)\(', source)
        self.assertIn("m6h_create", exports)
        self.assertEqual(csharp_list("Exports", "HostLibrary.cs"), exports)
        rust_version = re.search(r"pub const M6H_ABI_VERSION: u32 = (\d+);", source)
        csharp_version = re.search(r"public const uint AbiVersion = (\d+);", csharp)
        self.assertTrue(rust_version and csharp_version)
        self.assertEqual(rust_version.group(1), csharp_version.group(1))
        rust_codes = dict(re.findall(r"pub const M6H_(OK|E_[A-Z_]+): i32 = (\d+);", source))
        csharp_codes = dict(re.findall(r"public const int (\w+) = (\d+);", csharp))
        names = {"OK": "Ok", "E_INVALID_ARGUMENT": "InvalidArgument", "E_MODEL": "ModelNotLoaded",
                 "E_CORE": "CoreRefused", "E_HOST": "HostRefused", "E_UNAVAILABLE": "Unavailable",
                 "E_POISONED": "Poisoned"}
        self.assertEqual(set(rust_codes), set(names))
        for rust, name in names.items():
            self.assertEqual(csharp_codes.get(name), rust_codes[rust], rust)
        rows = {"M6hFrame": ("Submitted", ["sequence", "device_epoch", "surface"]),
                "M6hPresented": ("Presented", ["sequence", "device_epoch", "surface", "engine_revision", "camera",
                                               "display_qpc"])}
        for rust_name, (csharp_name, fields) in rows.items():
            with self.subTest(row=rust_name):
                rust_row = re.search(rf"#\[repr\(C\)\][^{{]*?pub struct {rust_name} \{{(.*?)\}}", source, re.S)
                csharp_row = re.search(rf"LayoutKind\.Sequential\)\]\s*public struct {csharp_name}\s*\{{(.*?)\}}",
                                       csharp, re.S)
                self.assertTrue(rust_row and csharp_row)
                self.assertEqual(re.findall(r"pub (\w+): u64,", rust_row.group(1)), fields)
                pascal = ["".join(part.title() for part in field.split("_")) for field in fields]
                self.assertEqual(re.findall(r"public ulong (\w+);", csharp_row.group(1)), pascal)

    def test_the_event_log_writes_the_rows_the_e2_timer_reads(self):
        sys.path.insert(0, str(ROOT / "tools" / "perf" / "c9"))
        try:
            import key_to_frame
        finally:
            sys.path.pop(0)
        log = text(PROJECT / "EventLog.cs")
        self.assertIn(f'public const string Format = "{key_to_frame.LOG_FORMAT}";', log)
        self.assertIn(f'public const string FileName = "{key_to_frame.EVENTS}";', log)
        written = {}
        for block in re.findall(r'json\.WriteString\("t", "(\w+)"\);(.*?)\}\)', log, re.S):
            written[block[0]] = re.findall(r'json\.Write\w+\("(\w+)"', block[1])
        for kind in ("frame", "presented"):
            with self.subTest(row=kind):
                self.assertEqual(written[kind], list(key_to_frame.FIELDS[kind]))
        self.assertNotIn("key", written, "no key rows until a turn exists")
        run = written["run"]
        for field in ("format", "run_id", "build_identity", "qpc_frequency", "synthetic", "foreground_throughout",
                      "correctness", "process_id"):
            self.assertIn(field, run)
        self.assertIn('json.WriteString("correctness", "fail");', log, "no correctness control yet: never pass")
        self.assertIn('json.WriteBoolean("synthetic", false);', log)

    def test_the_surface_uses_every_shell_call_and_no_other(self):
        used = set(re.findall(r'Call\("(sa2_[a-z_]+)"\)', text(PROJECT / "CoreSurface.cs")))
        self.assertEqual(used, set(csharp_list("ShellCalls")))

    def test_the_build_never_launches(self):
        sys.path.insert(0, str(SHELL))
        try:
            import build_shell
        finally:
            sys.path.remove(str(SHELL))
        command = build_shell.build_command()
        self.assertEqual(command[:3], ["dotnet", "build", "Magic600Shell.csproj"])
        self.assertTrue(str(build_shell.BUILD_ROOT).replace("\\", "/").endswith("work/build/c9-shell"))
        source = text(SHELL / "build_shell.py")
        self.assertNotIn("Popen(", source)
        self.assertNotIn("launch_value", source)

    def test_a_mismatched_export_list_is_seen(self):
        names = csharp_symbols()
        self.assertNotEqual(names[:-1] + ["sa2_device_state"], rust_symbols())


def shell_module(name: str):
    sys.path.insert(0, str(SHELL))
    try:
        return importlib.import_module(name)
    finally:
        sys.path.remove(str(SHELL))


def row_fields(source: str) -> dict[str, list[str]]:
    """The fields each JSON row kind of a C# log writes, in order."""
    return {kind: re.findall(r'json\.Write\w+\("(\w+)"', body)
            for kind, body in re.findall(r'json\.WriteString\("t", "(\w+)"\);(.*?)\}\)', source, re.S)}


class InputEcho(unittest.TestCase):
    """Route B's check B1: the echo's log is a file of its own that the E2 timer never reads."""

    def test_the_input_log_is_a_file_of_its_own(self):
        sys.path.insert(0, str(ROOT / "tools" / "perf" / "c9"))
        try:
            import key_to_frame
        finally:
            sys.path.pop(0)
        log = text(PROJECT / "InputLog.cs")
        self.assertIn('public const string Format = "magic600-c9-shell-input-v1";', log)
        self.assertIn('public const string FileName = "input.jsonl";', log)
        self.assertNotEqual("input.jsonl", key_to_frame.EVENTS)
        written = row_fields(log)
        self.assertEqual(written["press"], ["seq", "qpc", "keycode", "label"])
        self.assertEqual(written["click"], ["seq", "qpc", "button", "x", "y"])
        self.assertEqual(written["run"], ["format", "run_id", "build_identity", "qpc_frequency", "synthetic",
                                          "foreground_throughout", "process_id", "presses", "clicks"])
        self.assertFalse(set(written) & set(key_to_frame.FIELDS), "no row kind of the E2 log")

    def test_the_shell_echoes_only_when_asked(self):
        shell = text(PROJECT / "Shell.cs")
        self.assertIn('GetEnvironmentVariable("M600_INPUT_ECHO") == "1"', text(PROJECT / "InputLog.cs"))
        self.assertIn("if (InputLog.Requested())", shell)
        self.assertIn("public override void _Input(InputEvent @event)", shell)
        self.assertIn("InputEventKey { Pressed: true, Echo: false }", shell, "a held key's repeats are not presses")
        self.assertIn("MouseButton.WheelUp", shell, "the wheel is not a click")
        # A core that does not open closes the core's parts only; the echo stays.
        ready = shell[shell.index("public override void _Ready()"):shell.index("private void Open(")]
        self.assertIn("CloseCore();", ready)
        self.assertNotIn("Close();", ready.replace("CloseCore();", ""))


class ProductBuild(unittest.TestCase):
    """build_product.py's record and verify, and launch.py, over a fake build; nothing is built or started."""

    def setUp(self):
        self.build_product = shell_module("build_product")
        self.launch = shell_module("launch")
        self.temp = Path(tempfile.mkdtemp(prefix="c9-product-"))
        self.addCleanup(shutil.rmtree, self.temp, True)
        self.build = self.temp / "20261118T000000Z"
        files = {"native/sa2_interop.dll": b"core", "native/draw_vs.dxil": b"vs", "native/build.ninja": b"rules",
                 "host/c600_shell_host.dll": b"host", f"shell/project/{self.build_product.build_shell.ASSEMBLY}": b"asm",
                 "shell/project/Main.tscn": b"scene", "shell/project/obj/Generated.cs": b"generated",
                 "assets/manifest.json": b"{}"}
        for name, data in files.items():
            (self.build / name).parent.mkdir(parents=True, exist_ok=True)
            (self.build / name).write_bytes(data)
        self.godot = self.temp / "godot" / self.build_product.prepare_host.GODOT_EXE
        self.godot.parent.mkdir()
        self.godot.write_bytes(b"godot")
        record = self.build_product.record(self.build, self.godot, "0" * 40, False)
        (self.build / "product.json").write_text(json.dumps(record), encoding="utf-8")

    def test_a_build_verifies_and_names_every_part_it_loads(self):
        product = self.build_product.verify(self.build)
        self.assertEqual(set(product["parts"]), {"core:sa2_interop.dll", "core:draw_vs.dxil", "host:c600_shell_host.dll",
                                                 "shell:assembly", "shell:Main.tscn", "assets:manifest.json"})
        self.assertTrue(product["identity"].startswith("c9-product-"))
        self.assertTrue(all(not Path(part["path"]).is_absolute() for part in product["parts"].values()))

    def test_a_changed_or_added_part_is_refused(self):
        host = self.build / "host" / "c600_shell_host.dll"
        host.write_bytes(b"other host")
        with self.assertRaises(self.build_product.ProductError):
            self.build_product.verify(self.build)
        host.write_bytes(b"host")
        self.build_product.verify(self.build)
        (self.build / "shell" / "project" / "Extra.cs").write_bytes(b"extra")
        with self.assertRaises(self.build_product.ProductError):
            self.build_product.verify(self.build)

    def test_the_identity_follows_every_digest_and_no_order(self):
        identity = self.build_product.identity
        digests = {"a": "1" * 64, "b": "2" * 64}
        self.assertEqual(identity(digests), identity(dict(reversed(list(digests.items())))))
        self.assertNotEqual(identity(digests), identity({**digests, "b": "3" * 64}))

    def test_the_build_never_launches(self):
        self.assertEqual(self.build_product.host_command()[:6],
                         ["cargo", "build", "--locked", "--offline", "--release", "--manifest-path"])
        self.assertTrue(str(self.build_product.BUILD_ROOT).replace("\\", "/").endswith("work/build/c9-product"))
        source = text(SHELL / "build_product.py")
        self.assertNotIn("--path", source)
        self.assertNotIn("launch_value", source)

    def test_a_dry_run_creates_and_starts_nothing(self):
        def refuse(*args, **kwargs):
            raise AssertionError("a dry run started a process")
        for echo in ([], ["--echo"]):
            lines = []
            out = self.temp / "run"
            code = self.launch.main(["--out", str(out), "--build", str(self.build), "--dry-run", *echo],
                                    popen=refuse, out_stream=lines.append)
            self.assertEqual(code, 0, lines)
            self.assertFalse(out.exists())
            names = {line.split("=", 1)[0][len("env: "):] for line in lines if line.startswith("env: ")}
            expected = {"M600_CORE_DLL", "M600_HOST_DLL", "M600_ASSETS", "M600_EVENTS_DIR", "M600_BUILD_IDENTITY",
                        "M600_RUN_ID"} | ({"M600_INPUT_ECHO"} if echo else set())
            self.assertEqual(names, expected)

    def test_a_used_run_folder_or_a_changed_build_is_refused(self):
        def refuse(*args, **kwargs):
            raise AssertionError("a refused launch started a process")
        used = self.temp / "used"
        used.mkdir()
        self.assertEqual(self.launch.main(["--out", str(used), "--build", str(self.build)], popen=refuse,
                                          out_stream=lambda line: None), 2)
        (self.build / "native" / "draw_vs.dxil").write_bytes(b"other shader")
        self.assertEqual(self.launch.main(["--out", str(self.temp / "fresh"), "--build", str(self.build)],
                                          popen=refuse, out_stream=lambda line: None), 2)
        self.assertFalse((self.temp / "fresh").exists())

    def test_a_launch_starts_godot_on_the_build_and_records_no_private_path(self):
        started = {}

        class Process:
            def wait(self):
                return 0

        def popen(arguments, cwd, env):
            started.update(arguments=arguments, cwd=cwd, env=env)
            return Process()
        out = self.temp / "run"
        code = self.launch.main(["--out", str(out), "--build", str(self.build), "--run-id", "r1"], popen=popen,
                                out_stream=lambda line: None)
        self.assertEqual(code, 0)
        product = self.build_product.verify(self.build)
        arguments, env = started["arguments"], started["env"]
        self.assertEqual(Path(arguments[0]).name, self.build_product.prepare_host.GODOT_EXE)
        self.assertEqual(arguments[arguments.index("--path") + 1], str(self.build / "shell" / "project"))
        self.assertEqual(env["M600_BUILD_IDENTITY"], product["identity"])
        self.assertEqual(env["M600_ASSETS"], str(self.build / "assets"))
        self.assertEqual(env["M600_EVENTS_DIR"], str(out.resolve()))
        self.assertEqual(env["M600_RUN_ID"], "r1")
        self.assertNotIn("M600_INPUT_ECHO", env)
        record = json.loads((out / "launch.json").read_text(encoding="utf-8"))
        self.assertEqual((record["format"], record["build_identity"], record["exit_code"], record["echo"]),
                         ("magic600-c9-launch-v1", product["identity"], 0, False))
        self.assertNotIn(str(self.temp), json.dumps(record))
        self.assertNotIn(str(self.temp).replace("\\", "/"), json.dumps(record))

    def test_inherited_shell_variables_are_dropped(self):
        product = self.build_product.verify(self.build)
        env = self.launch.environment(product, self.build, self.temp / "run", "r1", False,
                                      base={"M600_INPUT_ECHO": "1", "m600_core_dll": "elsewhere", "PATH": "p"})
        self.assertNotIn("M600_INPUT_ECHO", env)
        self.assertNotIn("m600_core_dll", env)
        self.assertEqual(env["PATH"], "p")
        self.assertEqual(env["M600_CORE_DLL"], str(self.build / "native" / "sa2_interop.dll"))


if __name__ == "__main__":
    unittest.main()
