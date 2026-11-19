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
                 "E_POISONED": "Poisoned", "E_SESSION": "SessionRefused"}
        self.assertEqual(set(rust_codes), set(names))
        for rust, name in names.items():
            self.assertEqual(csharp_codes.get(name), rust_codes[rust], rust)
        rust_presses = dict(re.findall(r"pub const M6H_PRESS_([A-Z]+): u32 = (\d+);", source))
        csharp_presses = dict(re.findall(r"public const uint Press(\w+) = (\d+);", csharp))
        self.assertEqual(set(rust_presses), {"NONE", "COMMITTED", "REFUSED", "OTHER"})
        self.assertEqual({name.upper(): value for name, value in csharp_presses.items()}, rust_presses)
        key = re.search(r"#\[repr\(C\)\][^{]*?pub struct M6hKey \{(.*?)\}", source, re.S)
        csharp_key = re.search(r"LayoutKind\.Sequential\)\]\s*public struct KeyResult\s*\{(.*?)\}", csharp, re.S)
        self.assertTrue(key and csharp_key)
        self.assertEqual(re.findall(r"pub (\w+): (u\d+),", key.group(1)),
                         [("press", "u32"), ("reserved", "u32"), ("engine_revision", "u64")])
        self.assertEqual(re.findall(r"public (\w+) (\w+);", csharp_key.group(1)),
                         [("uint", "Press"), ("uint", "Reserved"), ("ulong", "EngineRevision")])
        control = re.search(r'pub const CONTROL: &str = "([a-z-]+)";',
                            text(ROOT / "renderer" / "bridge" / "host" / "src" / "turns.rs"))
        self.assertTrue(control)
        self.assertIn(f'public const string CorrectnessControl = "{control.group(1)}";', csharp)
        rows = {"M6hFrame": ("Submitted", ["sequence", "device_epoch", "surface", "engine_revision",
                                           "publication_revision"]),
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
        self.assertEqual(written["presented"], list(key_to_frame.FIELDS["presented"]))
        self.assertEqual(written["frame"], [*key_to_frame.FIELDS["frame"], "engine_revision", "publication_revision"])
        # A key row: seq and qpc, then frame_id as a number or as null.
        self.assertEqual(written["key"][:2], list(key_to_frame.FIELDS["key"]))
        self.assertEqual(written["key"][2:], ["frame_id", "frame_id"])
        self.assertIn('json.WriteNull("frame_id");', log)
        run = written["run"]
        for field in ("format", "run_id", "build_identity", "qpc_frequency", "synthetic", "foreground_throughout",
                      "correctness", "correctness_control", "correctness_detail", "process_id"):
            self.assertIn(field, run)
        # Correctness passes only when the session's control passed; without a session it is "fail".
        self.assertIn('json.WriteString("correctness", _correct ? "pass" : "fail");', log)
        self.assertEqual(re.findall(r"_correct = (\w+);", log), ["pass"])
        self.assertIn('private string _control = "none";', log)
        self.assertIn("_log?.Correctness(pass, HostLibrary.CorrectnessControl, library.Check(_host));",
                      text(PROJECT / "Shell.cs"))
        self.assertIn('json.WriteBoolean("synthetic", false);', log)
        # A press waits for a frame that draws its revision over a publication of it; the rest close without one.
        self.assertIn("f.EngineRevision >= press.Revision && f.PublicationRevision >= press.Revision", log)
        self.assertIn("WriteKey(press.Qpc, null);", log)

    def test_the_event_log_writes_the_rows_the_ui_skew_tool_reads(self):
        sys.path.insert(0, str(ROOT / "tools" / "perf" / "c9"))
        try:
            import key_to_frame
        finally:
            sys.path.pop(0)
        self.assertIn("ui_frame", key_to_frame.OTHER_ROWS)
        log = text(PROJECT / "EventLog.cs")
        written = row_fields(log)
        self.assertEqual(written["ui_frame"], ["ui_seq", "qpc", "frame_id", "drawn_qpc"])
        self.assertIn("var seq = _uiFrames++;", log)
        # The run row names the one swap chain of the run, or null; Godot's own chain is never named.
        self.assertEqual([field for field in written["run"] if "swap_chain" in field],
                         ["swap_chain", "swap_chain", "ui_swap_chain"])
        self.assertIn("if (_swapChains is [{ } chain])", log)
        self.assertIn('json.WriteNull("swap_chain");', log)
        self.assertIn('json.WriteNull("ui_swap_chain");', log)
        self.assertIn('"0x" + address.ToString("X16", System.Globalization.CultureInfo.InvariantCulture)', log)
        shell = text(PROJECT / "Shell.cs")
        for line in ("RenderingServer.FramePreDraw += OnFramePreDraw;", "RenderingServer.FramePostDraw += OnFramePostDraw;",
                     "RenderingServer.FramePreDraw -= OnFramePreDraw;", "RenderingServer.FramePostDraw -= OnFramePostDraw;",
                     "_log?.UiFrame(_preDrawQpc, frameId, drawn);", "_submittedThisFrame = frame.Sequence;"):
            self.assertIn(line, shell)
        self.assertEqual(shell.count("SwapChain(surface.SwapChain());"), 3, "after the attach, a resize and a recovery")
        process = shell[shell.index("public override void _Process"):shell.index("private void OnFramePreDraw")]
        self.assertLess(process.index("_submittedThisFrame = null;"), process.index("library.Frame(_host, out var frame)"))
        close = shell[shell.index("private void CloseCore()"):]
        self.assertLess(close.index("FramePostDraw -= OnFramePostDraw"), close.index("_log?.Dispose();"))
        # The core's diagnostic export is declared and resolved as optional.
        core = text(PROJECT / "CoreLibrary.cs")
        self.assertIn('public const string SwapChainCall = "sa2_surface_swap_chain";', core)
        self.assertIn("NativeLibrary.TryGetExport(handle, SwapChainCall, out var swapChain);", core)
        self.assertNotIn("sa2_surface_swap_chain", csharp_list("ShellCalls"))
        self.assertIn("sa2_surface_swap_chain", header_exports())

    def test_the_shell_gives_modules_the_window_and_its_frames(self):
        module = text(PROJECT / "ShellModule.cs")
        self.assertIn("public sealed record ShellContext(IntPtr Window, Control Root, EventLog? Log);", module)
        for member in ("bool OnWindowMessage(uint message, nuint wParam, nint lParam, long qpc);",
                       "void OnFrame(HostLibrary.Submitted frame, long qpc);",
                       "void OnPresented(HostLibrary.Presented row);"):
            self.assertIn(member, module)
        hook = text(PROJECT / "WindowHook.cs")
        self.assertIn("private const int GwlpWndProc = -4;", hook)
        self.assertIn("[UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvStdcall) })]", hook)
        self.assertIn("return CallWindowProcW(_original, window, message, wParam, lParam);", hook)
        receive = hook[hook.index("private static nint Receive"):]
        self.assertLess(receive.index("var qpc = EventLog.Now();"), receive.index("foreach"))
        self.assertIn("catch (Exception error)", receive, "no exception crosses into the native caller")
        shell = text(PROJECT / "Shell.cs")
        self.assertIn('if (System.Environment.GetEnvironmentVariable("M600_LX4") == "1")', shell)
        self.assertIn("_modules.Add(new Lx4.Lx4Module(new ShellContext(window, this, _log)));", shell)
        self.assertIn("WindowHook.Install(window, _modules);", shell)
        self.assertIn("EachModule(module => module.OnFrame(frame, submitted));", shell)
        self.assertIn("EachModule(module => module.OnPresented(row));", shell)
        close = shell[shell.index("private void CloseCore()"):]
        self.assertLess(close.index("WindowHook.Remove();"), close.index("EachModule(module => module.Dispose());"))
        self.assertLess(close.index("EachModule(module => module.Dispose());"), close.index("_log?.Dispose();"))
        # The run row names the window and the puzzle, and a module appends its own rows.
        log = text(PROJECT / "EventLog.cs")
        run = row_fields(log)["run"]
        self.assertIn("window", run)
        self.assertIn("puzzle", run)
        self.assertIn("public void Append(Action<Utf8JsonWriter> fields) => Write(fields);", log)
        self.assertIn("log.Describe(window, puzzle);", shell)
        # The stub has the hook points only: LX-4's messages and rows are its owner's, so the runbook does not call
        # U8 ready before they exist.
        stub = text(PROJECT / "Lx4" / "Lx4Module.cs")
        self.assertIn("public sealed class Lx4Module : IShellModule", stub)
        self.assertIn("public bool OnWindowMessage(uint message, nuint wParam, nint lParam, long qpc) => false;", stub)
        for needle in ("Magic600.LX4", '"lx4"', "RegisterWindowMessage"):
            self.assertNotIn(needle, stub)

    def test_the_shell_routes_physical_keys_in_c600_inputs_chord_form(self):
        chord = text(ROOT / "rust" / "c600-input" / "src" / "chord.rs")
        known = set()
        for name in ("CHARACTER_KEYS", "NAMED_KEYS"):
            block = re.search(rf"const {name}: \[&str; (\d+)\] = \[(.*?)\];", chord, re.S)
            self.assertTrue(block, name)
            codes = re.findall(r'"(\w+)"', block.group(2))
            self.assertEqual(len(codes), int(block.group(1)))
            known.update(codes)
        source = text(PROJECT / "KeyChord.cs")
        named = set(re.findall(r'=> "(\w+)"', source))
        self.assertTrue(named and named <= known, sorted(named - known))
        self.assertIn('return "Key" + (char)value;', source)
        self.assertIn('return "Digit" + (char)value;', source)
        self.assertIn('return "F" + (value - (long)Key.F1 + 1);', source)
        self.assertTrue({f"Key{c}" for c in "ABCDEFGHIJKLMNOPQRSTUVWXYZ"} <= known)
        self.assertTrue({f"Digit{d}" for d in range(10)} | {f"F{n}" for n in range(1, 13)} <= known)
        # Modifiers in c600-input's order, the physical key, and the platform's repeat flag.
        self.assertIn('(key.CtrlPressed ? "Ctrl+" : "") + (key.ShiftPressed ? "Shift+" : "") + (key.AltPressed ? '
                      '"Alt+" : "")', source)
        shell = text(PROJECT / "Shell.cs")
        self.assertIn("KeyChord.Code(key.PhysicalKeycode)", shell)
        self.assertIn("library.KeyDown(_host, chord, key.Echo, out var result)", shell)
        self.assertIn("library.FocusLost(_host);", shell)
        close = shell[shell.index("private void Close()"):shell.index("private void CloseSession()")]
        self.assertLess(close.index("CloseSession();"), close.index("CloseCore();"),
                        "the control runs before the log closes")

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
                 "assets/manifest.json": b"{}", "menu/wj-I_a.json": b"{}"}
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
                                                 "shell:assembly", "shell:Main.tscn", "assets:manifest.json",
                                                 "menu:wj-I_a.json"})
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

    def test_the_twist_menu_is_a_tracked_menu_record(self):
        source = ROOT / self.build_product.MENU_SOURCE
        self.assertIn("menu", json.loads(source.read_text(encoding="utf-8")))
        self.assertEqual(Path(self.build_product.MENU).name, source.name)

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
        for flags in ([], ["--echo"], ["--lx4"], ["--puzzle", "jumble-off", "--lx4", "--echo"]):
            lines = []
            out = self.temp / "run"
            code = self.launch.main(["--out", str(out), "--build", str(self.build), "--dry-run", *flags],
                                    popen=refuse, out_stream=lines.append)
            self.assertEqual(code, 0, lines)
            self.assertFalse(out.exists())
            names = {line.split("=", 1)[0][len("env: "):] for line in lines if line.startswith("env: ")}
            expected = ({"M600_CORE_DLL", "M600_HOST_DLL", "M600_ASSETS", "M600_EVENTS_DIR", "M600_BUILD_IDENTITY",
                         "M600_RUN_ID", "M600_SESSION_DIR", "M600_TWIST_MENU", "M600_PUZZLE"}
                        | ({"M600_INPUT_ECHO"} if "--echo" in flags else set())
                        | ({"M600_LX4"} if "--lx4" in flags else set()))
            self.assertEqual(names, expected)
            self.assertIn("env: M600_PUZZLE=jumble-off", lines)

    def test_jumble_on_is_refused_until_the_host_runs_it(self):
        def refuse(*args, **kwargs):
            raise AssertionError("a refused launch started a process")
        lines = []
        out = self.temp / "run"
        code = self.launch.main(["--out", str(out), "--build", str(self.build), "--puzzle", "jumble-on"],
                                popen=refuse, out_stream=lines.append)
        self.assertEqual(code, 2)
        self.assertFalse(out.exists())
        self.assertIn("does not run jumble-on yet", lines[-1])
        # The launcher and the shell name the same puzzles, and both refuse every other one.
        log = text(PROJECT / "EventLog.cs")
        puzzles = re.search(r"public static readonly string\[\] Puzzles = \{ (.*?) \};", log)
        self.assertTrue(puzzles)
        self.assertEqual(tuple(re.findall(r'"([a-z-]+)"', puzzles.group(1))), self.launch.PUZZLES)
        self.assertEqual(self.launch.RUNNABLE_PUZZLES, self.launch.PUZZLES[:1])
        self.assertIn('throw new NotSupportedException($"the shell does not run {puzzle} yet");',
                      text(PROJECT / "Shell.cs"))

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
        # The launcher resolves --build; on Windows the temporary folder may be an 8.3 short path.
        build = self.build.resolve()
        self.assertEqual(arguments[arguments.index("--path") + 1], str(build / "shell" / "project"))
        self.assertEqual(env["M600_BUILD_IDENTITY"], product["identity"])
        self.assertEqual(env["M600_ASSETS"], str(build / "assets"))
        self.assertEqual(env["M600_EVENTS_DIR"], str(out.resolve()))
        self.assertEqual(env["M600_RUN_ID"], "r1")
        self.assertEqual(env["M600_SESSION_DIR"], str(out.resolve() / "session"))
        self.assertEqual(env["M600_TWIST_MENU"], str(build / "menu" / "wj-I_a.json"))
        self.assertNotIn("M600_INPUT_ECHO", env)
        self.assertNotIn("M600_LX4", env)
        self.assertEqual(env["M600_PUZZLE"], "jumble-off")
        record = json.loads((out / "launch.json").read_text(encoding="utf-8"))
        self.assertEqual((record["format"], record["build_identity"], record["exit_code"], record["echo"],
                          record["puzzle"], record["lx4"]),
                         ("magic600-c9-launch-v1", product["identity"], 0, False, "jumble-off", False))
        for temp in {str(self.temp), str(self.temp.resolve())}:
            self.assertNotIn(temp, json.dumps(record))
            self.assertNotIn(temp.replace("\\", "/"), json.dumps(record))

    def test_inherited_shell_variables_are_dropped(self):
        product = self.build_product.verify(self.build)
        env = self.launch.environment(product, self.build, self.temp / "run", "r1", False,
                                      base={"M600_INPUT_ECHO": "1", "m600_core_dll": "elsewhere", "PATH": "p",
                                            "M600_LX4": "1", "m600_puzzle": "jumble-on"})
        self.assertNotIn("M600_INPUT_ECHO", env)
        self.assertNotIn("M600_LX4", env)
        self.assertNotIn("m600_puzzle", env)
        self.assertEqual(env["M600_PUZZLE"], "jumble-off")
        self.assertNotIn("m600_core_dll", env)
        self.assertEqual(env["PATH"], "p")
        self.assertEqual(env["M600_CORE_DLL"], str(self.build / "native" / "sa2_interop.dll"))


if __name__ == "__main__":
    unittest.main()
