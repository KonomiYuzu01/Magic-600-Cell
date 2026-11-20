"""Static checks of C9's product shell project (renderer/shell); never builds, launches Godot or touches a GPU.

The shell builds with renderer/shell/build_shell.py on a machine with the pinned Godot 4.7.2 .NET install; this check
runs everywhere and keeps the project's fixed points: Godot.NET.Sdk 4.7.2 on net10.0 (ADR-R01.5), an offline
package source only, the route-B window settings, and the core's export list and ABI version equal to the bridge's.
A pass is source evidence only.
"""
from __future__ import annotations

import hashlib
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
                 "E_POISONED": "Poisoned", "E_SESSION": "SessionRefused", "E_RUNTIME": "RuntimeRefused"}
        self.assertEqual(set(rust_codes), set(names))
        for rust, name in names.items():
            self.assertEqual(csharp_codes.get(name), rust_codes[rust], rust)
        rust_presses = dict(re.findall(r"pub const M6H_PRESS_([A-Z]+): u32 = (\d+);", source))
        csharp_presses = dict(re.findall(r"public const uint Press(\w+) = (\d+);", csharp))
        self.assertEqual(set(rust_presses), {"NONE", "COMMITTED", "REFUSED", "OTHER"})
        self.assertEqual({name.upper(): value for name, value in csharp_presses.items()}, rust_presses)
        # The session's puzzle codes (LX-4's jumble-on half): the same values on both sides, each the index of its
        # name in the shell's puzzle list, which the host's turns name in the same order.
        rust_puzzles = dict(re.findall(r"pub const M6H_PUZZLE_([A-Z_]+): u32 = (\d+);", source))
        csharp_puzzles = dict(re.findall(r"public const uint Puzzle(\w+) = (\d+);", csharp))
        self.assertEqual(rust_puzzles, {"JUMBLE_OFF": "0", "JUMBLE_ON": "1"})
        self.assertEqual({re.sub(r"(?<!^)(?=[A-Z])", "_", name).upper(): value
                          for name, value in csharp_puzzles.items()}, rust_puzzles)
        shell_names = re.search(r"public static readonly string\[\] Puzzles = \{ (.*?) \};",
                                text(PROJECT / "EventLog.cs"))
        turns_names = re.search(r"pub const NAMES: \[&'static str; 2\] = \[(.*?)\];",
                                text(ROOT / "renderer" / "bridge" / "host" / "src" / "turns.rs"))
        self.assertTrue(shell_names and turns_names)
        names = re.findall(r'"([a-z-]+)"', shell_names.group(1))
        self.assertEqual(names, re.findall(r'"([a-z-]+)"', turns_names.group(1)))
        for rust, value in rust_puzzles.items():
            self.assertEqual(names.index(rust.lower().replace("_", "-")), int(value), rust)
        self.assertIn("(uint)Array.IndexOf(EventLog.Puzzles, puzzle)", text(PROJECT / "Shell.cs"))
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
        # C9 U8's view-job record: the same fields, types and order on both sides.
        rust_row = re.search(r"#\[repr\(C\)\][^{]*?pub struct M6hViewResult \{(.*?)\}", source, re.S)
        csharp_row = re.search(r"LayoutKind\.Sequential\)\]\s*public struct ViewResult\s*\{(.*?)\}", csharp, re.S)
        self.assertTrue(rust_row and csharp_row)
        rust_fields = re.findall(r"pub (\w+): (u32|u64|\[u8; 32\]),", rust_row.group(1))
        self.assertEqual([name for name, _ in rust_fields],
                         ["status", "kind", "revision", "source_digest", "frame", "cleared_frame", "has_value",
                          "reserved", "value_sha256"])
        types = {"u32": "uint", "u64": "ulong", "[u8; 32]": "byte[32]"}
        expected = [(types[kind], "".join(part.title() for part in name.split("_"))) for name, kind in rust_fields]
        csharp_fields = [(kind + (f"[{size}]" if size else ""), name) for kind, name, size in
                         re.findall(r"public (?:fixed )?(\w+) (\w+)(?:\[(\d+)\])?;", csharp_row.group(1))]
        self.assertEqual(csharp_fields, expected)
        self.assertIn("size_of::<M6hViewResult>() == 104", source)
        self.assertIn("sizeof(ViewResult) != 104", csharp)
        rust_views = dict(re.findall(r"pub const M6H_VIEW_((?:STATUS|KIND)_[A-Z]+): u32 = (\d+);", source))
        csharp_views = re.findall(r"public const uint View(Status|Kind)(\w+) = (\d+);", csharp)
        self.assertEqual(set(rust_views), {"STATUS_PENDING", "STATUS_PUBLISHED", "STATUS_REFUSED", "KIND_RUN",
                                           "KIND_DRAFT"})
        self.assertEqual({f"{group.upper()}_{name.upper()}": value for group, name, value in csharp_views},
                         rust_views)

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
        self.assertIn("public sealed record ShellContext(IntPtr Window, Control Root, EventLog? Log, HostLibrary Host, "
                      "IntPtr HostHandle);", module)
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
        self.assertIn("_modules.Add(new Lx4.Lx4Module(new ShellContext(window, this, _log, hostLibrary, _host)));", shell)
        self.assertIn('if (System.Environment.GetEnvironmentVariable("M600_LX2") == "1")', shell)
        self.assertIn("_modules.Add(new Lx2.Lx2Module(new ShellContext(window, this, _log, hostLibrary, _host)));", shell)
        self.assertLess(shell.index("new Lx2.Lx2Module"), shell.index("WindowHook.Install(window, _modules);"))
        self.assertIn("WindowHook.Install(window, _modules);", shell)
        self.assertIn("EachModule(module => module.OnFrame(frame, submitted));", shell)
        self.assertIn("EachModule(module => module.OnPresented(row));", shell)
        close = shell[shell.index("private void CloseCore()"):]
        self.assertLess(close.index("WindowHook.Remove();"), close.index("EachModule(module => module.Dispose());"))
        self.assertLess(close.index("EachModule(module => module.Dispose());"), close.index("_log?.Dispose();"))
        # The modules go before the host: a module may call the host until its Dispose.
        self.assertLess(close.index("EachModule(module => module.Dispose());"), close.index("library.Destroy(_host);"))
        # The run row names the window and the puzzle, and a module appends its own rows.
        log = text(PROJECT / "EventLog.cs")
        run = row_fields(log)["run"]
        self.assertIn("window", run)
        self.assertIn("puzzle", run)
        self.assertIn("public void Append(Action<Utf8JsonWriter> fields) => Write(fields);", log)
        self.assertIn("log.Describe(window, puzzle);", shell)
        # LX-4's module is the C9 measurement line's (tests/test_c9_measure.py checks it against its library): here
        # only that it is a module that takes LX-4's messages and writes its rows through the log.
        lx4 = text(PROJECT / "Lx4" / "Lx4Module.cs")
        self.assertIn("public sealed unsafe class Lx4Module : IShellModule", lx4)
        for needle in ('Register("Magic600.LX4.Reset")', 'Register("Magic600.LX4.Select")',
                       'json.WriteString("t", "lx4");', "Context.Log?.Append("):
            self.assertIn(needle, lx4)
        # LX-2's module likewise (tests/test_c9_measure.py checks its schedule): a module that takes LX-2's phase
        # message, writes its rows through the log and sets the fault variables only in the shell's process.
        lx2 = text(PROJECT / "Lx2" / "Lx2Module.cs")
        self.assertIn("public sealed class Lx2Module : IShellModule", lx2)
        for needle in ('Register("Magic600.LX2.Phase")', 'json.WriteString("t", "lx2");', "Context.Log?.Append(",
                       '"C600_LX2_CEILING"', '"C600_LX2_FAULT"'):
            self.assertIn(needle, lx2)

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
                 "assets/manifest.json": b"{}", "menu/wj-I_a.json": b"{}", "runtime/c600-runtime.exe": b"runtime",
                 "session-fixture/fixture.json": b"{}", "session-fixture/session/lock": b"",
                 "session-fixture/session/journal": b"journal",
                 "session-fixture-jumble-on/fixture.json": b"{}", "session-fixture-jumble-on/session/lock": b"",
                 "session-fixture-jumble-on/session/journal": b"jumbled journal"}
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
                                                 "menu:wj-I_a.json", "runtime:c600-runtime.exe", "fixture:fixture.json",
                                                 "fixture:session/lock", "fixture:session/journal",
                                                 "fixture-jumble-on:fixture.json", "fixture-jumble-on:session/lock",
                                                 "fixture-jumble-on:session/journal"})
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

    def test_a_build_without_a_fixture_session_is_refused(self):
        for folder in ("session-fixture", "session-fixture-jumble-on"):
            with self.subTest(folder=folder):
                session = self.build / folder / "session"
                moved = self.temp / "moved"
                shutil.move(session, moved)
                with self.assertRaises(self.build_product.ProductError):
                    self.build_product.record(self.build, self.godot, "0" * 40, False)
                with self.assertRaises(self.build_product.ProductError):
                    self.build_product.verify(self.build)
                shutil.move(moved, session)
                self.build_product.verify(self.build)

    def test_the_build_writes_its_fixture_with_its_own_model_menu_and_runtime(self):
        # One fixture per puzzle the shell names, each in its own folder.
        self.assertEqual(tuple(self.build_product.FIXTURES), self.launch.PUZZLES)
        for puzzle, folder in (("jumble-off", "session-fixture"), ("jumble-on", "session-fixture-jumble-on")):
            command = self.build_product.fixture_command(self.build, puzzle)
            self.assertEqual(Path(command[0]).name, "c600-shell-session-fixture.exe")
            flags = dict(zip(command[1::2], command[2::2]))
            self.assertEqual(flags, {"--assets": str(self.build / "assets"),
                                     "--menu": str(self.build / "menu" / "wj-I_a.json"),
                                     "--out": str(self.build / folder), "--puzzle": puzzle,
                                     "--runtime": str(self.build / "runtime" / "c600-runtime.exe")})
            self.assertEqual(self.build_product.fixture_record(puzzle), folder + "/fixture.json")
            self.assertEqual(self.build_product.fixture_session(puzzle), folder + "/session")
        self.assertEqual(self.build_product.fixture_command(self.build),
                         self.build_product.fixture_command(self.build, "jumble-off"))
        self.assertEqual(self.build_product.runtime_command()[:6],
                         ["cargo", "build", "--locked", "--offline", "--release", "--manifest-path"])
        self.assertEqual(self.build_product.runtime_command()[-2:], ["-p", "c600-runtime"])
        main = text(ROOT / "renderer" / "bridge" / "host" / "src" / "bin" / "c600-shell-session-fixture.rs")
        for flag in flags:
            self.assertIn(f'"{flag}" =>', main)

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
        child = self.temp / "c600-lx2-child.exe"
        child.write_bytes(b"injection child")
        lx2 = ["--lx2-injection", str(child)]
        for flags in ([], ["--echo"], ["--lx4"], ["--puzzle", "jumble-off", "--lx4", "--echo"], ["--cold-start"], lx2,
                      ["--lx4", *lx2]):
            lines = []
            out = self.temp / "run"
            code = self.launch.main(["--out", str(out), "--build", str(self.build), "--dry-run", *flags],
                                    popen=refuse, out_stream=lines.append)
            self.assertEqual(code, 0, lines)
            self.assertFalse(out.exists())
            names = {line.split("=", 1)[0][len("env: "):] for line in lines if line.startswith("env: ")}
            expected = ({"M600_CORE_DLL", "M600_HOST_DLL", "M600_ASSETS", "M600_EVENTS_DIR", "M600_BUILD_IDENTITY",
                         "M600_RUN_ID", "M600_SESSION_DIR", "M600_TWIST_MENU", "M600_PUZZLE", "M600_SESSION_FIXTURE",
                         "M600_RUNTIME"}
                        | ({"M600_INPUT_ECHO"} if "--echo" in flags else set())
                        | ({"M600_LX4"} if "--lx4" in flags else set())
                        | ({"M600_LX2"} if "--lx2-injection" in flags else set())
                        | ({"M600_EXIT_AT_FIRST_FRAME"} if "--cold-start" in flags else set()))
            self.assertEqual(names, expected)
            self.assertIn("env: M600_PUZZLE=jumble-off", lines)
            if "--lx2-injection" in flags:
                self.assertIn(f"env: M600_RUNTIME={child.resolve()}", lines)
                digest = hashlib.sha256(b"injection child").hexdigest()
                self.assertTrue(any(digest in line and "c600-lx2-child.exe" in line for line in lines), lines)

    def test_lx2_injection_is_refused_with_a_cold_start_or_without_its_file(self):
        def refuse(*args, **kwargs):
            raise AssertionError("a refused launch started a process")
        child = self.temp / "c600-lx2-child.exe"
        child.write_bytes(b"injection child")
        for flags, says in ((["--lx2-injection", str(child), "--cold-start"], "--cold-start measures"),
                            (["--lx2-injection", str(self.temp / "missing.exe")], "no file missing.exe")):
            lines = []
            out = self.temp / "run"
            code = self.launch.main(["--out", str(out), "--build", str(self.build), *flags], popen=refuse,
                                    out_stream=lines.append)
            self.assertEqual(code, 2)
            self.assertFalse(out.exists())
            self.assertIn(says, lines[-1])
            self.assertNotIn(str(self.temp), lines[-1])

    def test_an_lx2_launch_runs_the_child_and_records_its_digest_and_no_path(self):
        started = {}

        class Process:
            def wait(self):
                return 0

        def popen(arguments, cwd, env):
            started.update(arguments=arguments, cwd=cwd, env=env)
            return Process()
        child = self.temp / "c600-lx2-child.exe"
        child.write_bytes(b"injection child")
        out = self.temp / "run"
        machine = FakeMachine()
        code = self.launch.main(["--out", str(out), "--build", str(self.build), "--run-id", "r2", "--lx4",
                                 "--lx2-injection", str(child)], popen=popen, out_stream=lambda line: None,
                                machine=machine, state=self.temp / "last-exit.json")
        self.assertEqual(code, 0)
        env = started["env"]
        self.assertEqual(env["M600_RUNTIME"], str(child.resolve()))
        self.assertEqual((env["M600_LX2"], env["M600_LX4"]), ("1", "1"))
        record = json.loads((out / "launch.json").read_text(encoding="utf-8"))
        self.assertEqual(record["lx2_injection"],
                         {"name": "c600-lx2-child.exe", "sha256": hashlib.sha256(b"injection child").hexdigest()})
        self.assertTrue(record["lx4"])
        for temp in {str(self.temp), str(self.temp.resolve())}:
            self.assertNotIn(temp, json.dumps(record))
            self.assertNotIn(temp.replace("\\", "/"), json.dumps(record))
        # A launch without the flag records null and keeps the build's runtime.
        self.assertIsNone(json.loads((self.run_plain() / "launch.json").read_text(encoding="utf-8"))["lx2_injection"])

    def run_plain(self) -> Path:
        class Process:
            def wait(self):
                return 0
        out = self.temp / "plain"
        self.assertEqual(self.launch.main(["--out", str(out), "--build", str(self.build), "--run-id", "r3"],
                                          popen=lambda arguments, cwd, env: Process(), out_stream=lambda line: None,
                                          machine=FakeMachine(), state=self.temp / "last-exit.json"), 0)
        return out

    def test_jumble_on_is_refused_until_the_host_runs_it(self):
        def refuse(*args, **kwargs):
            raise AssertionError("a refused launch started a process")
        lines = []
        out = self.temp / "run"
        code = self.launch.main(["--out", str(out), "--build", str(self.build), "--puzzle", "jumble-on"],
                                popen=refuse, out_stream=lines.append)
        self.assertEqual(code, 2)
        self.assertFalse(out.exists())
        self.assertIn("does not draw jumble-on yet", lines[-1])
        # The launcher and the shell name the same puzzles, and both refuse every other one.
        log = text(PROJECT / "EventLog.cs")
        puzzles = re.search(r"public static readonly string\[\] Puzzles = \{ (.*?) \};", log)
        self.assertTrue(puzzles)
        self.assertEqual(tuple(re.findall(r'"([a-z-]+)"', puzzles.group(1))), self.launch.PUZZLES)
        self.assertEqual(self.launch.RUNNABLE_PUZZLES, self.launch.PUZZLES[:1])
        # The shell runs Jumble on only with a session, whose host opens the jumbled puzzle (LX-4's jumble-on half);
        # the launcher above refuses it until it passes one.
        shell = text(PROJECT / "Shell.cs")
        self.assertNotIn("does not run {puzzle} yet", shell)
        self.assertIn('$"the shell runs {puzzle} only with a session (M600_SESSION_DIR and M600_TWIST_MENU)");',
                      shell)

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
                                out_stream=lambda line: None, machine=FakeMachine(), state=self.temp / "last-exit.json")
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
        self.assertEqual(env["M600_SESSION_FIXTURE"], str(build / "session-fixture" / "fixture.json"))
        self.assertEqual(env["M600_RUNTIME"], str(build / "runtime" / "c600-runtime.exe"))
        self.assertNotIn("M600_INPUT_ECHO", env)
        self.assertNotIn("M600_LX4", env)
        self.assertNotIn("M600_EXIT_AT_FIRST_FRAME", env)
        self.assertEqual(env["M600_PUZZLE"], "jumble-off")
        # The run's session is a fresh copy of the fixture's, its lock file included.
        self.assertEqual((out / "session" / "journal").read_bytes(), b"journal")
        self.assertTrue((out / "session" / "lock").is_file())
        record = json.loads((out / "launch.json").read_text(encoding="utf-8"))
        self.assertEqual((record["format"], record["build_identity"], record["exit_code"], record["echo"],
                          record["puzzle"], record["lx4"], record["cold_start"]),
                         ("magic600-c9-launch-v1", product["identity"], 0, False, "jumble-off", False, False))
        rows = [json.loads(line) for line in (out / "startup.jsonl").read_text(encoding="utf-8").splitlines()]
        self.assertEqual([row["t"] for row in rows], ["run", "process_start"])
        for temp in {str(self.temp), str(self.temp.resolve())}:
            self.assertNotIn(temp, json.dumps(record))
            self.assertNotIn(temp.replace("\\", "/"), json.dumps(record))

    def test_inherited_shell_variables_are_dropped(self):
        product = self.build_product.verify(self.build)
        env = self.launch.environment(product, self.build, self.temp / "run", "r1", False,
                                      base={"M600_INPUT_ECHO": "1", "m600_core_dll": "elsewhere", "PATH": "p",
                                            "M600_LX4": "1", "m600_puzzle": "jumble-on", "M600_LX2": "1",
                                            "C600_LX2_CEILING": "1", "c600_lx2_fault": "kill"})
        for dropped in ("M600_LX2", "C600_LX2_CEILING", "c600_lx2_fault"):
            self.assertNotIn(dropped, env)
        self.assertNotIn("M600_INPUT_ECHO", env)
        self.assertNotIn("M600_LX4", env)
        self.assertNotIn("m600_puzzle", env)
        self.assertEqual(env["M600_PUZZLE"], "jumble-off")
        self.assertEqual(env["M600_SESSION_FIXTURE"], str(self.build / "session-fixture" / "fixture.json"))
        # With Jumble on the host reopens the jumbled fixture: a fixture opens only as its own puzzle.
        jumbled = self.launch.environment(product, self.build, self.temp / "run", "r1", False, base={},
                                          puzzle="jumble-on")
        self.assertEqual(jumbled["M600_PUZZLE"], "jumble-on")
        self.assertEqual(jumbled["M600_SESSION_FIXTURE"],
                         str(self.build / "session-fixture-jumble-on" / "fixture.json"))
        self.assertNotIn("m600_core_dll", env)
        self.assertEqual(env["PATH"], "p")
        self.assertEqual(env["M600_CORE_DLL"], str(self.build / "native" / "sa2_interop.dll"))


class FakeMachine:
    """A machine whose clocks the test sets: the counter at 10 MHz, the wall clock and the boot's tick count."""

    def __init__(self, wall=1_000_000.0, ticks_ms=5_000_000, images=()):
        self.now_wall, self.now_ticks, self.counter, self.images, self.slept = wall, ticks_ms, 1_000_000_000, images, []

    def qpc(self):
        self.counter += 1
        return self.counter

    def qpc_frequency(self):
        return 10_000_000

    def ticks_ms(self):
        return self.now_ticks

    def wall(self):
        return self.now_wall

    def sleep(self, seconds):
        self.slept.append(seconds)
        self.now_wall += seconds
        self.now_ticks += int(seconds * 1000)

    def process_images(self):
        if isinstance(self.images, Exception):
            raise self.images
        return list(self.images)


class ColdStart(unittest.TestCase):
    """Charter E3's launch: the pause, the cold flags and the sealed startup log that cold_start.py reads."""

    setUp = ProductBuild.setUp

    def cold_start_tool(self):
        sys.path.insert(0, str(ROOT / "tools" / "perf" / "c9"))
        try:
            return importlib.import_module("cold_start")
        finally:
            sys.path.remove(str(ROOT / "tools" / "perf" / "c9"))

    def launch_once(self, machine, state, name, shell_rows=None, flags=("--cold-start",)):
        started = {}

        class Process:
            def wait(self):
                return 0

        def popen(arguments, cwd, env):
            started.update(env=env, qpc=machine.counter)
            events = Path(env["M600_EVENTS_DIR"])
            if shell_rows is not None:
                body = "".join(json.dumps(row(machine.counter)) + "\n" for row in shell_rows)
                (events / self.launch.STARTUP_BODY).write_text(body, encoding="utf-8", newline="\n")
            return Process()
        out = self.temp / name
        lines = []
        code = self.launch.main(["--out", str(out), "--build", str(self.build), "--run-id", name, *flags], popen=popen,
                                out_stream=lines.append, machine=machine, state=state)
        self.assertEqual(code, 0, lines)
        return out, started, lines

    def test_a_cold_start_waits_seals_its_log_and_the_timer_reads_it(self):
        machine, state = FakeMachine(), self.temp / "last-exit.json"
        # The previous exit was 5 s ago: the launcher waits the other 15 s.
        state.write_text(json.dumps({"format": self.launch.LAST_EXIT_FORMAT, "wall": machine.now_wall - 5,
                                     "ticks_ms": machine.now_ticks - 5000}), encoding="utf-8")
        shell = [lambda start: {"t": "session_opened", "qpc": start + 2_000_000},
                 lambda start: {"t": "runtime_loaded", "qpc": start + 5_000_000},
                 lambda start: {"t": "first_correct_frame", "frame_id": 3, "display_qpc": start + 12_000_000,
                                "check": "pass"}]
        out, started, lines = self.launch_once(machine, state, "cold-1", shell)
        self.assertEqual(machine.slept, [15.0])
        self.assertIn("waiting 15.0 s", lines[0])
        self.assertEqual(started["env"]["M600_EXIT_AT_FIRST_FRAME"], "1")
        self.assertFalse((out / self.launch.STARTUP_BODY).exists(), "the body is removed once sealed")
        rows = [json.loads(line) for line in (out / "startup.jsonl").read_text(encoding="utf-8").splitlines()]
        self.assertEqual([row["t"] for row in rows],
                         ["run", "process_start", "session_opened", "runtime_loaded", "first_correct_frame"])
        self.assertEqual(rows[1]["qpc"], started["qpc"], "process_start is the counter right before the process")
        self.assertEqual(rows[0]["cold"], {"new_process": True, "no_product_process_at_launch": True,
                                           "fresh_session_copy": True, "seconds_since_previous_exit": 20.0})
        self.assertEqual((rows[0]["format"], rows[0]["synthetic"], rows[0]["qpc_frequency"]),
                         ("magic600-c9-startup-v1", False, 10_000_000))
        tool = self.cold_start_tool()
        self.assertEqual(tool.LOG_FORMAT, self.launch.STARTUP_FORMAT)
        self.assertEqual(tool.PAUSE_S, self.launch.PAUSE_S)
        run = tool.read_run(out)
        self.assertEqual((run["status"], run["within_bar"]), ("valid", True), run)
        self.assertAlmostEqual(run["seconds"], 1.2)
        self.assertTrue(json.loads((out / "launch.json").read_text(encoding="utf-8"))["cold_start"])
        # The exit is recorded, so a launch right after it waits the full pause.
        self.assertEqual(json.loads(state.read_text(encoding="utf-8"))["ticks_ms"], machine.now_ticks)
        out, _, _ = self.launch_once(machine, state, "cold-2", shell)
        self.assertEqual(machine.slept, [15.0, 20.0])
        self.assertEqual(tool.read_run(out)["status"], "valid")

    def test_a_shell_that_wrote_nothing_still_gives_a_failed_run(self):
        out, _, _ = self.launch_once(FakeMachine(), self.temp / "last-exit.json", "silent")
        rows = [json.loads(line) for line in (out / "startup.jsonl").read_text(encoding="utf-8").splitlines()]
        self.assertEqual([row["t"] for row in rows], ["run", "process_start"])
        self.assertIsNone(rows[0]["cold"]["seconds_since_previous_exit"])
        run = self.cold_start_tool().read_run(out)
        self.assertEqual(run["status"], "failed", run)

    def test_a_running_product_or_an_unread_process_list_is_not_cold(self):
        godot = json.loads((self.build / "product.json").read_text(encoding="utf-8"))["godot"]["executable"]
        for images in ([godot], [str(self.build.resolve() / "runtime" / "c600-runtime.exe")], OSError("no")):
            with self.subTest(images=images):
                name = f"busy-{len(list(self.temp.glob('busy-*')))}"
                out, _, _ = self.launch_once(FakeMachine(images=images), self.temp / "last-exit.json", name, [])
                head = json.loads((out / "startup.jsonl").read_text(encoding="utf-8").splitlines()[0])
                self.assertFalse(head["cold"]["no_product_process_at_launch"])
        self.assertFalse(self.launch.product_running(FakeMachine(images=["C:/other/godot.exe"]), {godot}))

    def test_a_previous_exit_from_another_boot_counts_as_none(self):
        machine, state = FakeMachine(), self.temp / "last-exit.json"
        since = self.launch.seconds_since_previous_exit
        self.assertIsNone(since(machine, state))
        self.launch.record_exit(machine, state)
        machine.sleep(30)
        self.assertEqual(since(machine, state), 30.0)
        machine.now_ticks = 1000  # the tick count restarted: another boot
        self.assertIsNone(since(machine, state))
        machine.now_ticks, machine.now_wall = 5_030_000 + 100_000, machine.now_wall + 10  # clocks 90 s apart
        self.assertIsNone(since(machine, state))
        state.write_text("not json", encoding="utf-8")
        self.assertIsNone(since(machine, state))
        # The record sits beside the builds, and the newest build is still found.
        self.assertEqual(self.launch.LAST_EXIT.parent, self.build_product.BUILD_ROOT)
        (self.temp / self.launch.LAST_EXIT.name).write_text("{}", encoding="utf-8")
        self.assertEqual(self.build_product.newest(self.temp), self.build)

    def test_the_shell_writes_the_rows_the_cold_start_timer_reads(self):
        startup = text(PROJECT / "StartupLog.cs")
        self.assertIn(f'public const string BodyName = "{self.launch.STARTUP_BODY}";', startup)
        for event in ("session_opened", "runtime_loaded"):
            self.assertIn(f'Event("{event}", qpc)', startup)
        for field in ('"t", "first_correct_frame"', '"frame_id"', '"display_qpc"', '"check", pass ? "pass" : "fail"'):
            self.assertIn(field, startup)
        shell = text(PROJECT / "Shell.cs")
        names = set(re.findall(r'GetEnvironmentVariable\("(M600_[A-Z0-9_]+)"\)', shell))
        self.assertTrue({"M600_SESSION_FIXTURE", "M600_RUNTIME", "M600_EXIT_AT_FIRST_FRAME"} <= names)
        self.assertTrue(names <= set(self.launch.SHELL_VARIABLES), names - set(self.launch.SHELL_VARIABLES))
        self.assertIn('GetEnvironmentVariable("M600_EXIT_AT_FIRST_FRAME") == "1"', shell)
        self.assertIn("_startup?.SessionOpened(EventLog.Now());", shell)
        self.assertIn("_startup?.RuntimeLoaded(EventLog.Now());", shell)


if __name__ == "__main__":
    unittest.main()
