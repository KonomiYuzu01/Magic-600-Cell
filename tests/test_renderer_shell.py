"""Static checks of C9's product shell project (renderer/shell); never builds, launches Godot or touches a GPU.

The shell builds with renderer/shell/build_shell.py on a machine with the pinned Godot 4.7.2 .NET install; this check
runs everywhere and keeps the project's fixed points: Godot.NET.Sdk 4.7.2 on net10.0 (ADR-R01.5), an offline
package source only, the route-B window settings, and the core's export list and ABI version equal to the bridge's.
A pass is source evidence only.
"""
from __future__ import annotations

import hashlib
import importlib
import importlib.util
import json
import math
import re
import shutil
import subprocess
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
    return re.findall(r'"((?:sa2|m6h|m6e)_[a-z_0-9]+)"', block.group(1))


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
        # LP4-3: the focus codes of m6h_set_focus.
        rust_focus = dict(re.findall(r"pub const M6H_FOCUS_([A-Z]+): u32 = (\d+);", source))
        csharp_focus = dict(re.findall(r"public const uint Focus(\w+) = (\d+);", csharp))
        self.assertEqual(rust_focus, {"PUZZLE": "0", "TEXT": "1"})
        self.assertEqual({name.upper(): value for name, value in csharp_focus.items()}, rust_focus)

    def test_the_editor_library_calls_match_its_rust_source(self):
        """LP4-3: the editor library's exports, version and codes, the same on both sides."""
        source = text(ROOT / "renderer" / "bridge" / "editor" / "src" / "lib.rs")
        csharp = text(PROJECT / "Editor" / "EditorLibrary.cs")
        exports = re.findall(r'#\[no_mangle\]\s*pub (?:unsafe )?extern "C" fn (m6e_[a-z_]+)\(', source)
        self.assertIn("m6e_create", exports)
        self.assertEqual(csharp_list("Exports", "Editor/EditorLibrary.cs"), exports)
        rust_version = re.search(r"pub const M6E_ABI_VERSION: u32 = (\d+);", source)
        csharp_version = re.search(r"public const uint AbiVersion = (\d+);", csharp)
        self.assertTrue(rust_version and csharp_version)
        self.assertEqual(rust_version.group(1), csharp_version.group(1))
        rust_codes = dict(re.findall(r"pub const M6E_(OK|E_[A-Z_]+): i32 = (\d+);", source))
        csharp_codes = dict(re.findall(r"public const int (\w+) = (\d+);", csharp))
        names = {"OK": "Ok", "E_INVALID_ARGUMENT": "InvalidArgument", "E_REFUSED": "Refused",
                 "E_RUNTIME": "RuntimeRefused", "E_POISONED": "Poisoned"}
        self.assertEqual(set(rust_codes), set(names))
        for rust, name in names.items():
            self.assertEqual(csharp_codes.get(name), rust_codes[rust], rust)
        for group in ("UNITS", "RUN"):
            with self.subTest(group=group):
                rust_values = dict(re.findall(rf"pub const M6E_{group}_([A-Z0-9]+): u32 = (\d+);", source))
                prefix = group.title()
                csharp_values = dict(re.findall(rf"public const uint {prefix}(\w+) = (\d+);", csharp))
                self.assertTrue(rust_values)
                self.assertEqual({name.upper(): value for name, value in csharp_values.items()}, rust_values)

    def test_the_editor_panel_keeps_the_write_check_run_flow_s_fixed_points(self):
        """LP4-3's greybox panel: created by M600_EDITOR, activation an explicit palette entry with no default key, a
        run that commits nothing, and keys that reach the code field while it has the focus."""
        shell = text(PROJECT / "Shell.cs")
        self.assertIn('if (System.Environment.GetEnvironmentVariable("M600_EDITOR") == "1")', shell)
        self.assertIn("_editor = new Editor.EditorPanel(new ShellContext(window, this, _log, hostLibrary, _host));",
                      shell)
        self.assertIn("_modules.Add(_editor);", shell)
        self.assertLess(shell.index("new Editor.EditorPanel"), shell.index("WindowHook.Install(window, _modules);"))
        self.assertIn("_editor?.TextFocused != true", shell)
        panel = text(PROJECT / "Editor" / "EditorPanel.cs")
        self.assertIn("public sealed class EditorPanel : IShellModule", panel)
        self.assertIn('GetEnvironmentVariable("M600_EDITOR_DLL")', panel)
        self.assertIn('GetEnvironmentVariable("M600_RUNTIME")', panel)
        self.assertIn("M600_EDITOR_DLL", self.shell_variables())
        # Check and Activate are the panel's own palette entries; only Activate registers a row, and only after the
        # library activated the candidate.
        self.assertIn('public const string CheckEntry = "editor.check";', panel)
        self.assertIn('public const string ActivateEntry = "editor.activate";', panel)
        activate = panel[panel.index("private void Activate()"):panel.index("private void Pump()")]
        check = panel[panel.index("private void Check()"):panel.index("private void Activate()")]
        self.assertNotIn("RegisterRow", check)
        self.assertNotIn("_library.Activate", check)
        self.assertLess(activate.index("_library.Activate("), activate.index("RegisterRow("))
        self.assertEqual(panel.count("RegisterRow("), 1)
        # The row's key is no default binding of the one checked key map (LP4-8 moved it to c600_command::keys, which
        # c600-input re-exports), and the panel's own entries have none.
        key = re.search(r'public const string Key = "([A-Za-z+]+)";', panel)
        self.assertTrue(key)
        self.assertNotIn(f'"{key.group(1)}"', text(ROOT / "rust" / "c600-command" / "src" / "keys" / "keymap.rs"))
        # A run commits nothing: the panel invokes rows only through the palette route, routes no key itself, and its
        # run's word only becomes the draft, which the solver previews by key.
        for absent in ("KeyDown(", "operation.execute", "operation.preview\"", "Invoke(Context.HostHandle, \"operation"):
            self.assertNotIn(absent, panel)
        self.assertEqual(panel.count("SetDraftWord("), 1)
        pump = panel[panel.index("private void Pump()"):panel.index("private void Shown(")]
        for needle in ("host.TakeJob(", "_library.Run(", "_library.RunPoll(", "host.FinishJob("):
            self.assertIn(needle, pump)
        # The code field's focus is the router's text focus.
        self.assertIn("_code.FocusEntered += () => Focus(true);", panel)
        self.assertIn("HostLibrary.FocusText : HostLibrary.FocusPuzzle", panel)
        # The library counts positions in scalars, as Godot's columns do.
        self.assertIn("_library.Create(EditorLibrary.UnitsScalar, out _editor)", panel)

    def shell_variables(self) -> tuple[str, ...]:
        return shell_module("launch").SHELL_VARIABLES

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
        # ADR-D02 B18c: a composite frame's row adds its pane count and held flag; a one-view row is unchanged.
        self.assertEqual(written["frame"], [*key_to_frame.FIELDS["frame"], "engine_revision", "publication_revision",
                                            "returned_qpc", "panes", "held"])
        self.assertIn('if (frame.Panes > 0)\n            {\n                json.WriteNumber("panes", frame.Panes);', log)
        # A key row: seq and qpc, the QPC when m6h_key_down returned, then frame_id as a number or as null.
        self.assertEqual(written["key"][:2], list(key_to_frame.FIELDS["key"]))
        self.assertEqual(written["key"][2:], ["handled_qpc", "frame_id", "frame_id"])
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
        self.assertIn("WriteKey(press.Qpc, press.Handled, null);", log)
        # handled_qpc and returned_qpc are read as m6h_key_down and m6h_frame return, before anything else, so a slow
        # core call shows in the log apart from a slow present.
        shell = text(PROJECT / "Shell.cs")
        turn = shell[shell.index("private void Turn("):shell.index("private void CreateEcho()")]
        self.assertLess(turn.index("library.KeyDown(_host, chord, key.Echo, out var result);"),
                        turn.index("var handled = EventLog.Now();"))
        self.assertLess(turn.index("var handled = EventLog.Now();"), turn.index("if (answer != HostLibrary.Ok)"))
        self.assertEqual(turn.count("_log?.Press(qpc, handled, "), 2)
        process = shell[shell.index("public override void _Process"):shell.index("private void OnFramePreDraw")]
        self.assertLess(process.index("library.Frame(_host, out var frame);"),
                        process.index("var returned = EventLog.Now();"))
        self.assertLess(process.index("var returned = EventLog.Now();"), process.index("if (code == HostLibrary.Ok)"))
        self.assertIn("_log?.Frame(frame, submitted, returned);", process)

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
        # The chord tables are c600_command::keys::chord's since LP4-8; c600-input re-exports them.
        chord = text(ROOT / "rust" / "c600-command" / "src" / "keys" / "chord.rs")
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
                 "host/c600_shell_host.dll": b"host", "lx4/c600_shell_lx4.dll": b"lx4",
                 "editor/c600_shell_editor.dll": b"editor",
                 f"shell/project/{self.build_product.build_shell.ASSEMBLY}": b"asm",
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
        # LX-4's fixture folder as c600-lx4 --out writes it; the launcher reads only its manifest.
        self.fixtures = self.temp / "lx4-fixtures"
        self.fixtures.mkdir()
        (self.fixtures / "MANIFEST.json").write_bytes(b'{"format": "lx4 fixtures"}')
        (self.fixtures / "EDITS.json").write_bytes(b"{}")
        self.lx4 = ["--lx4", "--lx4-fixtures", str(self.fixtures)]

    def test_a_build_verifies_and_names_every_part_it_loads(self):
        product = self.build_product.verify(self.build)
        self.assertEqual(set(product["parts"]), {"core:sa2_interop.dll", "core:draw_vs.dxil", "host:c600_shell_host.dll",
                                                 "lx4:c600_shell_lx4.dll", "editor:c600_shell_editor.dll",
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
        for command, package in ((self.build_product.host_command(), "c600-shell-host"),
                                 (self.build_product.lx4_command(), "c600-shell-lx4"),
                                 (self.build_product.editor_command(), "c600-shell-editor")):
            self.assertEqual(command[:6], ["cargo", "build", "--locked", "--offline", "--release", "--manifest-path"])
            self.assertEqual(command[-2:], ["-p", package])
        # The libraries the build copies are the ones that LX-4's crate and the editor's crate build, both members of
        # the bridge's workspace.
        for folder, package, dll, constant in (("lx4", "c600-shell-lx4", "lx4/c600_shell_lx4.dll", "LX4_DLL"),
                                               ("editor", "c600-shell-editor", "editor/c600_shell_editor.dll",
                                                "EDITOR_DLL")):
            manifest = text(ROOT / "renderer" / "bridge" / folder / "Cargo.toml")
            self.assertIn(f'name = "{package}"', manifest)
            self.assertIn('"cdylib"', manifest)
            self.assertEqual(getattr(self.build_product, constant), dll)
            self.assertIn(f'"{folder}"', text(ROOT / "renderer" / "bridge" / "Cargo.toml"))
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
        for flags in ([], ["--echo"], self.lx4, ["--puzzle", "jumble-off", *self.lx4, "--echo"], ["--cold-start"], lx2,
                      [*self.lx4, *lx2], ["--editor"]):
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
                        | ({"M600_LX4", "M600_LX4_DLL", "M600_LX4_FIXTURES"} if "--lx4" in flags else set())
                        | ({"M600_LX2"} if "--lx2-injection" in flags else set())
                        | ({"M600_EDITOR", "M600_EDITOR_DLL"} if "--editor" in flags else set())
                        | ({"M600_EXIT_AT_FIRST_FRAME"} if "--cold-start" in flags else set()))
            self.assertEqual(names, expected)
            self.assertIn("env: M600_PUZZLE=jumble-off", lines)
            if "--editor" in flags:
                self.assertIn(f"env: M600_EDITOR_DLL={self.build.resolve() / 'editor' / 'c600_shell_editor.dll'}",
                              lines)
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

    def test_lx4_is_refused_without_its_fixtures_and_hands_the_module_what_it_loads(self):
        def refuse(*args, **kwargs):
            raise AssertionError("a refused launch started a process")
        empty = self.temp / "empty"
        empty.mkdir()
        for flags, says in ((["--lx4"], "go together"), (["--lx4-fixtures", str(self.fixtures)], "go together"),
                            (["--lx4", "--lx4-fixtures", str(empty)], "no MANIFEST.json"),
                            (["--lx4", "--lx4-fixtures", str(self.temp / "missing")], "no MANIFEST.json")):
            with self.subTest(flags=flags[:2]):
                lines = []
                out = self.temp / "run"
                code = self.launch.main(["--out", str(out), "--build", str(self.build), *flags], popen=refuse,
                                        out_stream=lines.append)
                self.assertEqual(code, 2)
                self.assertFalse(out.exists())
                self.assertIn(says, lines[-1])
                self.assertNotIn(str(self.temp), lines[-1])
        # The variables are the ones the shell's module requires.
        module = text(PROJECT / "Lx4" / "Lx4Module.cs")
        for name in ("M600_LX4_DLL", "M600_LX4_FIXTURES"):
            self.assertIn(f'Required("{name}")', module)
            self.assertIn(name, self.launch.SHELL_VARIABLES)
        self.assertIn('GetEnvironmentVariable("M600_LX4") == "1"', text(PROJECT / "Shell.cs"))

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
        code = self.launch.main(["--out", str(out), "--build", str(self.build), "--run-id", "r2", *self.lx4,
                                 "--lx2-injection", str(child)], popen=popen, out_stream=lambda line: None,
                                machine=machine, state=self.temp / "last-exit.json")
        self.assertEqual(code, 0)
        env = started["env"]
        self.assertEqual(env["M600_RUNTIME"], str(child.resolve()))
        self.assertEqual((env["M600_LX2"], env["M600_LX4"]), ("1", "1"))
        # LX-4's library from the verified build and the fixture folder, both resolved like --build.
        self.assertEqual(env["M600_LX4_DLL"], str(self.build.resolve() / "lx4" / "c600_shell_lx4.dll"))
        self.assertEqual(env["M600_LX4_FIXTURES"], str(self.fixtures.resolve()))
        record = json.loads((out / "launch.json").read_text(encoding="utf-8"))
        self.assertEqual(record["lx2_injection"],
                         {"name": "c600-lx2-child.exe", "sha256": hashlib.sha256(b"injection child").hexdigest()})
        self.assertTrue(record["lx4"])
        self.assertEqual(record["lx4_fixtures"],
                         {"manifest_sha256": hashlib.sha256(b'{"format": "lx4 fixtures"}').hexdigest()})
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

    def test_jumble_on_launches_on_its_own_fixture(self):
        started = {}

        class Process:
            def wait(self):
                return 0

        def popen(arguments, cwd, env):
            started.update(env=env)
            return Process()
        out = self.temp / "run"
        code = self.launch.main(["--out", str(out), "--build", str(self.build), "--run-id", "r4", "--puzzle",
                                 "jumble-on"], popen=popen, out_stream=lambda line: None, machine=FakeMachine(),
                                state=self.temp / "last-exit.json")
        self.assertEqual(code, 0)
        self.assertEqual(started["env"]["M600_PUZZLE"], "jumble-on")
        # main() resolves --build (a runner's temp folder may be an 8.3 short name).
        self.assertEqual(started["env"]["M600_SESSION_FIXTURE"],
                         str(self.build.resolve() / "session-fixture-jumble-on" / "fixture.json"))
        self.assertEqual((out / "session" / "journal").read_bytes(), b"jumbled journal")
        self.assertEqual(json.loads((out / "launch.json").read_text(encoding="utf-8"))["puzzle"], "jumble-on")
        # The launcher and the shell name the same puzzles, and both refuse every other one.
        log = text(PROJECT / "EventLog.cs")
        puzzles = re.search(r"public static readonly string\[\] Puzzles = \{ (.*?) \};", log)
        self.assertTrue(puzzles)
        self.assertEqual(tuple(re.findall(r'"([a-z-]+)"', puzzles.group(1))), self.launch.PUZZLES)
        self.assertFalse(hasattr(self.launch, "RUNNABLE_PUZZLES"), "every puzzle the launcher names runs")
        # The shell runs Jumble on only with a session, whose host opens the jumbled puzzle (LX-4's jumble-on half);
        # every launch passes one.
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
        for name in ("M600_LX4", "M600_LX4_DLL", "M600_LX4_FIXTURES", "M600_EDITOR", "M600_EDITOR_DLL"):
            self.assertNotIn(name, env)
        self.assertNotIn("M600_EXIT_AT_FIRST_FRAME", env)
        self.assertEqual(env["M600_PUZZLE"], "jumble-off")
        # The run's session is a fresh copy of the fixture's, its lock file included.
        self.assertEqual((out / "session" / "journal").read_bytes(), b"journal")
        self.assertTrue((out / "session" / "lock").is_file())
        record = json.loads((out / "launch.json").read_text(encoding="utf-8"))
        self.assertEqual((record["format"], record["build_identity"], record["exit_code"], record["echo"],
                          record["puzzle"], record["lx4"], record["lx4_fixtures"], record["cold_start"]),
                         ("magic600-c9-launch-v1", product["identity"], 0, False, "jumble-off", False, None, False))
        self.assertIs(record["editor"], False)
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
                                            "C600_LX2_CEILING": "1", "c600_lx2_fault": "kill",
                                            "M600_LX4_DLL": "elsewhere", "m600_lx4_fixtures": "elsewhere",
                                            "M600_EDITOR": "1", "m600_editor_dll": "elsewhere"})
        for dropped in ("M600_LX2", "C600_LX2_CEILING", "c600_lx2_fault", "M600_LX4_DLL", "m600_lx4_fixtures",
                        "M600_EDITOR", "m600_editor_dll"):
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

    def foreground(self):
        return 4242


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


class D02Launch(unittest.TestCase):
    """ADR-D02 build step B4: the D02 switch, staging, the scripted runner and the watch command, over the fake build.
    Nothing is built, and the only process started is a trivial Python."""

    setUp = ProductBuild.setUp

    def launched(self, flags):
        started = {}

        class Process:
            def wait(self):
                return 0

        def popen(arguments, cwd, env, **options):
            started.update(arguments=arguments, env=env, options=options)
            return Process()
        out = self.temp / "run"
        code = self.launch.main(["--out", str(out), "--build", str(self.build), *flags], popen=popen,
                                out_stream=lambda line: None, machine=FakeMachine(), state=self.temp / "last-exit.json")
        return code, started, out

    def d02_inputs(self):
        fixture = self.temp / "d02-fixture"
        (fixture / "session").mkdir(parents=True)
        (fixture / "fixture.json").write_bytes(b'{"state": "wj-ia-300"}')
        (fixture / "session" / "journal").write_bytes(b"d02 journal")
        script = self.temp / "smoke.json"
        script.write_bytes(b'{"format": "magic600-d02-script/1"}')
        data = self.temp / "data"
        (data / "samples").mkdir(parents=True)
        return fixture, script, data

    def test_the_build_stages_the_samples_and_the_d02_sources(self):
        build_shell = shell_module("build_shell")
        project = self.temp / "staged"
        build_shell.stage(project)
        samples = ROOT / "tools" / "looklab" / "data" / "samples"
        self.assertEqual((project / "samples" / "jumble" / "states.json").read_bytes(),
                         (samples / "jumble" / "states.json").read_bytes())
        for name in ("command-rows-d02.json", "jumble-words.json", "command-table.json", "command-labels.json"):
            self.assertTrue((project / "design" / name).is_file(), name)
        for name in ("Chrome/Ui/ScreenText.cs", "Chrome/Model/ToolSlots.cs", "D02/ShellJumble.cs", "D02/D02Window.cs"):
            self.assertTrue((project / name).is_file(), name)
        self.assertIn("stage(project)", text(SHELL / "build_shell.py"))

    def test_d02_needs_the_frame_and_its_options_need_d02(self):
        def refuse(*args, **kwargs):
            raise AssertionError("a refused launch started a process")
        fixture, script, data = self.d02_inputs()
        empty = self.temp / "empty-fixture"
        empty.mkdir()
        for flags in (["--d02"], ["--frame", "--d02-state", "wj-ia-300"], ["--frame", "--d02-script", str(script)],
                      ["--frame", "--d02-fixture", str(fixture)], ["--frame", "--d02-data", str(data)],
                      ["--frame", "--d02", "--cold-start"], ["--frame", "--d02", "--d02-fixture", str(empty)],
                      ["--frame", "--d02", "--d02-script", str(self.temp / "missing.json")],
                      ["--frame", "--d02", "--d02-data", str(empty)], ["--frame", "--d02", "--position", "12"]):
            out = self.temp / "refused"
            self.assertEqual(self.launch.main(["--out", str(out), "--build", str(self.build), *flags], popen=refuse,
                                              out_stream=lambda line: None), 2, flags)
            self.assertFalse(out.exists(), flags)

    def test_a_d02_launch_shows_without_activation_and_records_no_private_path(self):
        fixture, script, data = self.d02_inputs()
        code, started, out = self.launched(["--frame", "--puzzle", "jumble-on", "--d02", "--d02-state", "wj-ia-300",
                                            "--d02-fixture", str(fixture), "--d02-script", str(script),
                                            "--d02-data", str(data), "--position", "100,80"])
        self.assertEqual(code, 0)
        env, options, arguments = started["env"], started["options"], started["arguments"]
        self.assertEqual((env["M600_D02"], env["M600_CHROME"], env["M600_D02_STATE"]), ("1", "1", "wj-ia-300"))
        self.assertEqual(env["M600_D02_SCRIPT"], str(script.resolve()))
        self.assertEqual(env["M600_D02_FIXTURE"], str(fixture.resolve() / "fixture.json"))
        self.assertEqual(env["M600_SESSION_FIXTURE"], env["M600_D02_FIXTURE"])
        self.assertEqual(env["M600_D02_DATA"], str(data.resolve()))
        # The window that had the foreground before the launch, which the shell hands it back to.
        self.assertEqual(env["M600_D02_FOREGROUND"], "4242")
        info = options["startupinfo"]
        self.assertTrue(info.dwFlags & self.launch.STARTF_USESHOWWINDOW)
        self.assertEqual((self.launch.STARTF_USESHOWWINDOW, self.launch.SW_SHOWNOACTIVATE, info.wShowWindow), (1, 4, 4))
        self.assertEqual(arguments[arguments.index("--position") + 1], "100,80")
        for flag in ("-t", "--always-on-top", "-f", "--fullscreen", "-m", "--maximized"):
            self.assertNotIn(flag, arguments)
        self.assertEqual((out / "session" / "journal").read_bytes(), b"d02 journal")
        record = json.loads((out / "launch.json").read_text(encoding="utf-8"))
        self.assertEqual(record["d02"], {
            "state": "wj-ia-300", "fixture": {"sha256": hashlib.sha256(b'{"state": "wj-ia-300"}').hexdigest()},
            "script": {"name": "smoke.json",
                       "sha256": hashlib.sha256(b'{"format": "magic600-d02-script/1"}').hexdigest()},
            "data_reload": True})
        for temp in {str(self.temp), str(self.temp.resolve())}:
            self.assertNotIn(temp, json.dumps(record))
            self.assertNotIn(temp.replace("\\", "/"), json.dumps(record))

    def test_without_d02_a_launch_is_unchanged(self):
        code, started, out = self.launched(["--frame"])
        self.assertEqual(code, 0)
        self.assertEqual(started["options"], {}, "a measured run gets no STARTUPINFO")
        self.assertFalse([key for key in started["env"] if key.startswith("M600_D02")])
        self.assertIsNone(json.loads((out / "launch.json").read_text(encoding="utf-8"))["d02"])
        product = self.build_product.verify(self.build)
        env = self.launch.environment(product, self.build, self.temp / "run", "r1", False,
                                      base={"M600_D02": "1", "m600_d02_foreground": "9", "M600_D02_SCRIPT": "s"})
        self.assertFalse([key for key in env if key.upper().startswith("M600_D02")])

    def test_no_topmost_flag_and_no_activation_outside_the_gate_path(self):
        sources = list(SHELL.rglob("*.cs")) + list(SHELL.glob("*.py")) + list(
            (ROOT / "tools" / "looklab" / "app" / "Chrome").rglob("*.cs"))
        self.assertTrue(any(path.name == "D02Window.cs" for path in sources))
        for path in sources:
            body = text(path)
            for pattern in ("HWND_TOPMOST", "AlwaysOnTop", "always-on-top", "always_on_top", "BringWindowToTop",
                            "SetActiveWindow", "MoveToForeground", "AllowSetForegroundWindow",
                            "AttachThreadInput", "WindowFlags.NoFocus"):
                self.assertNotIn(pattern, body, f"{path.name}: {pattern}")
            # user32's SetFocus is never imported (HostLibrary.SetFocus is the router's text-field focus, not a window's).
            self.assertIsNone(re.search(r"extern\s+\w+\s+SetFocus\s*\(|EntryPoint\s*=\s*\"SetFocus\"", body), path.name)
            calls = re.findall(r"SetForegroundWindow\((\w+)\)", body)
            if calls:
                # The one activation is the hand-back: of the window that had the foreground, never the shell's own.
                self.assertEqual((path.name, calls), ("D02Window.cs", ["previous"]))
        window = text(PROJECT / "D02" / "D02Window.cs")
        # The window only ever goes behind another one (the previous or the current foreground window); behind a
        # topmost window it goes behind the launch's previous window, or to the bottom (checkpoint C1); never to the
        # top of the normal windows, and never activated. A topmost style that appears anyway is cleared, then bottom.
        self.assertEqual(re.findall(r"SetWindowPos\(own, (\w+), 0, 0, 0, 0, SwpNoSize \| SwpNoMove \| SwpNoActivate\);",
                                    window), ["after", "HwndNoTopmost", "HwndBottom"])
        self.assertEqual(len(re.findall(r"SetWindowPos\(", window)), 4)  # the import and the three calls
        self.assertIn("var after = !IsTopmost(other) ? other", window)
        self.assertIn(": previous != IntPtr.Zero && previous != own && IsWindow(previous) && !IsTopmost(previous) ? previous", window)
        self.assertIn("            : HwndBottom;", window)
        self.assertIn("private static readonly IntPtr HwndNoTopmost = new(-2);", window)
        self.assertIn("private static readonly IntPtr HwndBottom = new(1);", window)
        # Back in front after a check found another window there: only the owner brings it back, so the checks stop,
        # even after a long session open held them up (checkpoint C1).
        settle_body = window[window.index("public static void Settle()"):]
        self.assertLess(settle_body.index("if (!first && foreground == own && _sawElsewhere)"),
                        settle_body.index("SetForegroundWindow(previous);"))
        self.assertIn("_sawElsewhere = true;", settle_body)
        self.assertEqual(re.findall(r"Behind\(own, (\w+)\);", window), ["previous", "foreground"])
        # After the first check, a mouse button or a moved or resized window ends the checks for good: the owner's own
        # clicks and switches stand. Keys do not count.
        settle = window[window.index("public static void Settle()"):]
        self.assertLess(settle.index("else if (MouseButtonDown() || DisplayServer.WindowGetPosition() != _position || "
                                     "DisplayServer.WindowGetSize() != _size)"),
                        settle.index("SetForegroundWindow(previous);"))
        self.assertIn("if (own == IntPtr.Zero || OwnerActed)", settle)
        self.assertNotIn("GetKeyState", window.replace("GetAsyncKeyState(Vk", ""))
        self.assertNotIn("always_on_top", text(PROJECT / "project.godot"))
        shell = text(PROJECT / "Shell.cs")
        self.assertIn("if (D02.ShellJumble.On)\n        {\n            D02.D02Window.Settle();", shell)

    def test_the_draft_rows_are_registered_only_with_d02_on(self):
        chrome = text(PROJECT / "ShellChrome.cs")
        table_at = chrome.index("CommandRegistry.FromTable(")
        rows_at = chrome.index("ShellJumble.AddRows(registry, folder);")
        self.assertLess(table_at, rows_at)
        self.assertLess(rows_at, chrome.index("Keymap.Load("))
        self.assertLess(chrome.index("new CommandPalette("), chrome.index("ShellJumble.Attach(parent, frame, palette, () => cards?.ObservePicks());"))
        self.assertIn("_ when ShellJumble.On && ShellJumble.Drafts.Contains(command) => ShellJumble.DraftReason,", chrome)
        jumble = text(PROJECT / "D02" / "ShellJumble.cs")
        self.assertIn('System.Environment.GetEnvironmentVariable("M600_D02") == "1"', jumble)
        # Relabel refuses a label for an unknown command: every label names a table row or a D02 draft row.
        design = ROOT / "tools" / "looklab" / "data" / "design"
        known = {row["id"] for row in json.loads((ROOT / "docs/progress/1.0/command-table.json").read_text(
            encoding="utf-8"))["commands"]} | {row["id"] for row in json.loads(
            (design / "command-rows-d02.json").read_text(encoding="utf-8"))["commands"]}
        labels = json.loads((design / "command-labels.json").read_text(encoding="utf-8"))["labels"]
        self.assertFalse(set(labels) - known)

    def smoke_dumps(self, source, d02_run):
        window = {"foreground": False, "topmost": False, "gave_back": 0}
        rows = d02_run.palette_matches("tuning.", source["rows"], source["table"])
        listed = [["overlay:CommandPalette", "tuning."]]
        for label in rows:
            listed += [["overlay:CommandPalette", label], ["overlay:CommandPalette", "draft row"],
                       ["overlay:CommandPalette", "tuning"]]
        return [{"step": 0, "do": "start", "window": window, "lines": [["frame", "D02 · no state"]]},
                {"step": 1, "do": "chord F1", "window": window, "lines": [["overlay:CommandPalette", "No command"]]},
                {"step": 2, "do": "text tuning.", "window": window, "lines": listed},
                {"step": 3, "do": "chord Escape", "window": window, "lines": [["frame", "D02 · no state"]]},
                {"step": 4, "do": "wait 6000 ms", "window": window, "lines": [["frame", "D02 · no state"]]}]

    def test_the_smoke_expectation_reads_the_data_and_sees_a_missing_row(self):
        d02_run = shell_module("d02_run")
        source = d02_run.data()
        script = json.loads((SHELL / "d02_scripts" / "smoke.json").read_text(encoding="utf-8"))
        self.assertEqual((script["format"], script["name"], len(script["steps"])), ("magic600-d02-script/1", "smoke", 4))
        dumps = self.smoke_dumps(source, d02_run)
        self.assertEqual(len(d02_run.palette_matches("tuning.", source["rows"], source["table"])), 9)
        self.assertEqual(d02_run.expect_smoke(dumps, source, None), [])
        self.assertEqual(d02_run.window_problems(dumps), [])
        dumps[2]["lines"] = [line for line in dumps[2]["lines"] if line[1] != "Tuning: swap compare slots"]
        self.assertTrue(d02_run.expect_smoke(dumps, source, None))
        dumps[4]["window"] = {"foreground": False, "topmost": True, "gave_back": 1}
        self.assertEqual(d02_run.window_problems(dumps), ["step 4: the shell's window is topmost"])

    def test_a_run_fails_when_the_foreground_moves(self):
        d02_run = shell_module("d02_run")
        source = d02_run.data()
        dumps = self.smoke_dumps(source, d02_run)

        def launcher(arguments, popen, out_stream):
            out = Path(arguments[arguments.index("--out") + 1])
            out.mkdir()
            popen([sys.executable, "-c", "pass"], cwd=str(self.temp), env=None).wait()
            (out / d02_run.SCREEN).write_text("".join(json.dumps(dump) + "\n" for dump in dumps), encoding="utf-8")
            return 0

        class Desktop:
            def __init__(self, windows):
                self.answers = list(windows)

            def foreground(self):
                return self.answers.pop(0)

            def windows_of(self, pid):
                return [77]

            def topmost(self, window):
                return False
        code, record = d02_run.run("smoke", self.temp / "kept", None, None, desktop=Desktop([5, 5]),
                                   launcher=launcher, sleep=lambda seconds: None)
        self.assertEqual((code, record["steps"], record["mismatches"], record["foreground_kept"]), (0, 5, [], True))
        code, record = d02_run.run("smoke", self.temp / "moved", None, None, desktop=Desktop([5, 6]),
                                   launcher=launcher, sleep=lambda seconds: None)
        self.assertEqual(code, 1)
        self.assertIn("the foreground changed during the first five seconds", record["mismatches"])
        with self.assertRaises(ValueError):
            d02_run.run("smoke", self.temp / "other", None, "no-such-state", desktop=Desktop([5, 5]),
                        launcher=launcher)

    def test_the_watch_command_starts_a_watched_window_and_follows(self):
        watch = shell_module("watch_d02")
        place = {"position": [10, 20], "size": [800, 600], "topmost": False, "gave_back": 0}
        arguments = watch.launch_arguments(self.build, self.temp / "watched", "wj-ia-300", place)
        for flag in ("--frame", "--d02", "--d02-data"):
            self.assertIn(flag, arguments)
        self.assertEqual(arguments[arguments.index("--puzzle") + 1], "jumble-on")
        self.assertEqual(arguments[arguments.index("--position") + 1], "10,20")
        self.assertEqual(arguments[arguments.index("--resolution") + 1], "800x600")
        for flag in ("-t", "--always-on-top", "--cold-start"):
            self.assertNotIn(flag, arguments)
        lines = []
        self.assertEqual(self.launch.main(arguments[3:] + ["--dry-run"], popen=None, out_stream=lines.append), 0, lines)
        self.assertIn("env: M600_D02=1", lines)
        # A data-only change is seen.
        data = self.temp / "data"
        (data / "samples").mkdir(parents=True)
        (data / "design").mkdir()
        (data / "design" / "system.json").write_text("{}", encoding="utf-8")
        before = watch.data_digest(data)
        (data / "samples" / "new.json").write_text("{}", encoding="utf-8")
        self.assertNotEqual(watch.data_digest(data), before)
        self.assertTrue(str(watch.WATCH_ROOT).replace("\\", "/").endswith("work/d02-watch"))

class ShellDocks(unittest.TestCase):
    """The shell's dock panels sit on the sides the layouts name (workspaces.json): a dock whose side lacks the
    named panel stays shut (Frame.cs), which left both Solve docks shut until B4a."""

    def test_every_dock_panel_a_layout_names_is_on_that_side(self):
        chrome = text(PROJECT / "ShellChrome.cs")
        placed = {}
        for side, panel in re.findall(r"\(DockSide\.(\w+), \"([\w.]+)\"", chrome):
            placed.setdefault(panel, set()).add(side.lower())
        for side, panel in re.findall(r"AddDockPanel\(DockSide\.(\w+), \"([\w.]+)\"", chrome):
            placed.setdefault(panel, set()).add(side.lower())
        self.assertEqual(placed["history"], {"right"})
        layouts = json.loads((ROOT / "tools" / "looklab" / "data" / "design" / "workspaces.json").read_text(
            encoding="utf-8"))["workspaces"]
        for layout in layouts:
            for side, panel in layout["arrangement"]["docks"].items():
                if panel is not None:
                    self.assertIn(side, placed.get(panel, set()), f"{layout['name']}: {panel} is not in the {side} dock")
        self.assertEqual([layout["name"] for layout in layouts], ["Solve", "Script", "Analyse", "Learn", "Gallery"])

class SampleMode(unittest.TestCase):
    """B4a: ADR-D01's whole frame in the product shell on sample data (launch.py --sample), with Look Lab's sample
    wiring staged from its one copy, a visible sample chip, and the real session's parts kept apart."""

    setUp = ProductBuild.setUp
    launched = D02Launch.launched

    def test_the_build_stages_the_shared_wiring_and_its_data(self):
        build_shell = shell_module("build_shell")
        project = self.temp / "staged"
        build_shell.stage(project)
        live = ROOT / "tools" / "looklab" / "app" / "Live"
        for name in build_shell.LIVE_FILES:
            self.assertEqual((project / "Live" / name).read_bytes(), (live / name).read_bytes(), name)
        self.assertTrue((project / "sample-root" / "assets" / "seed_atlas.json").is_file())
        for pattern in build_shell.SAMPLE_ROOT_GLOBS:
            found = sorted(ROOT.glob(pattern))
            self.assertTrue(found, pattern)
            for path in found:
                self.assertTrue((project / "sample-root" / path.relative_to(ROOT)).is_file(), path.name)
        # One copy: the shell's own sources hold none of the shared wiring.
        for name in build_shell.LIVE_FILES:
            self.assertFalse(list(SHELL.rglob(name)), name)
        # The shared wiring needs nothing of Look Lab's engine.
        for name in build_shell.LIVE_FILES:
            self.assertNotIn("LookLab.Core", text(live / name), name)
            self.assertNotIn("LabApp", text(live / name), name)

    def test_sample_needs_the_frame_and_is_never_measured(self):
        def refuse(*args, **kwargs):
            raise AssertionError("a refused launch started a process")
        for flags in (["--sample"], ["--frame", "--sample", "--cold-start"]):
            out = self.temp / "refused"
            self.assertEqual(self.launch.main(["--out", str(out), "--build", str(self.build), *flags], popen=refuse,
                                              out_stream=lambda line: None), 2, flags)
            self.assertFalse(out.exists(), flags)

    def test_a_sample_launch_sets_the_switch_and_records_it(self):
        code, started, out = self.launched(["--frame", "--sample"])
        self.assertEqual(code, 0)
        self.assertEqual((started["env"]["M600_SAMPLE"], started["env"]["M600_CHROME"]), ("1", "1"))
        self.assertTrue(json.loads((out / "launch.json").read_text(encoding="utf-8"))["sample"])
        shutil.rmtree(out)
        code, started, out = self.launched(["--frame"])
        self.assertNotIn("M600_SAMPLE", started["env"])
        self.assertFalse(json.loads((out / "launch.json").read_text(encoding="utf-8"))["sample"])
        product = self.build_product.verify(self.build)
        env = self.launch.environment(product, self.build, self.temp / "run", "r1", False, base={"M600_SAMPLE": "1"})
        self.assertNotIn("M600_SAMPLE", env)

    def test_the_real_session_keeps_its_parts(self):
        mount = text(ROOT / "tools" / "looklab" / "app" / "Live" / "SampleMount.cs")
        self.assertIn('Kept = new HashSet<string>(StringComparer.Ordinal) { "session.undo", "session.redo" };', mount)
        self.assertIn("public bool Handles(string command) => !Kept.Contains(command) &&", mount)
        # The sample history is a panel of its own; the History dock stays the session's.
        self.assertIn('frame.AddDockPanel(DockSide.Right, HistoryPanel, "History (sample)", "panel.history", Demo.History);', mount)
        self.assertNotIn('SetContent("history"', mount)
        chrome = text(PROJECT / "ShellChrome.cs")
        self.assertIn('public static bool SampleOn => System.Environment.GetEnvironmentVariable("M600_SAMPLE") == "1";', chrome)
        self.assertIn('SessionWidgets.Chip("sample", frame.Tokens.C("text.dim"), SampleWords);', chrome)
        self.assertIn("_ when sample != null && sample.Handles(command) => sample.Unavailable(command, argument),", chrome)
        self.assertLess(chrome.index("_ when ShellJumble.On && ShellJumble.Drafts.Contains(command)"),
                        chrome.index("_ when sample != null && sample.Handles(command)"))
        # The History dock and the position readout read this session's moves.
        self.assertIn('frame.AddDockPanel(DockSide.Right, "history", "History", "panel.history", history);', chrome)
        self.assertIn("_position.Show(HistoryReadouts.Position(Moves));", chrome)
        # Look Lab builds its inspector on the shared one, with its own View fields.
        self.assertIn("inspect = LabInspect.Create(app, frame, demo);", text(ROOT / "tools" / "looklab" / "app" / "Live" / "LabFrame.cs"))

    def test_a_twist_key_reaches_a_live_session_in_sample_mode(self):
        # The owner's list: J did not twist in sample mode. The keymap binds J to the twist row, which sample mode's
        # draft phase carries out, so the chrome took J. With a live session a twist key is not taken; the frame's own
        # keys are still asked first (Claims), and the twist scripts run in sample mode.
        keymap = json.loads(text(ROOT / "tools" / "looklab" / "data" / "design" / "keymap.json"))
        twists = [b for b in keymap["bindings"] if b["command"] == "operation.phase.append_twist"]
        self.assertEqual([(b["key"], b["context"]) for b in twists], [("J", "global")])
        chrome = text(PROJECT / "ShellChrome.cs")
        takes = chrome[chrome.index("    public bool Takes(InputEventKey key)"):chrome.index("    public void Committed(")]
        self.assertIn("var takes = Frame.Claims(name, binding => _unavailable(binding.Command, binding.Argument) == null &&\n"
                      "            !(_live && KeyMapModel.IsTwist(binding.Command)));", takes)
        frame = text(ROOT / "tools" / "looklab" / "app" / "Chrome" / "Ui" / "ChromeFrame.cs")
        claims = frame[frame.index("    public bool Claims(string key"):frame.index("    public bool Press(string key)")]
        self.assertLess(claims.index("HostKeys.ContainsKey(key)) return true;"), claims.index("can(binding)"))
        self.assertLess(claims.index("is LineEdit or TextEdit or IOwnKeys) return true;"), claims.index("can(binding)"))
        run = shell_module("d02_run")
        self.assertTrue({"bind", "two-views"} <= run.IN_SAMPLE_MODE)

    def test_the_watch_command_starts_sample_mode(self):
        watch = shell_module("watch_d02")
        arguments = watch.launch_arguments(self.build, self.temp / "watched", None)
        self.assertIn("--sample", arguments)
        lines = []
        self.assertEqual(self.launch.main(arguments[3:] + ["--dry-run"], popen=None, out_stream=lines.append), 0, lines)
        self.assertIn("env: M600_SAMPLE=1", lines)

class D02Fixtures(unittest.TestCase):
    """ADR-D02 build step B5: the words the session fixtures are written from, the digest check that ties a fixture to
    its sample, the watch command's and the runner's use of a written fixture, the host's state-digest export, and the
    shell's binding by digests. Nothing is built, and the fixture writer is a fake."""

    def setUp(self):
        self.fixtures = shell_module("build_d02_fixtures")
        self.temp = Path(tempfile.mkdtemp(prefix="d02-fixtures-"))
        self.addCleanup(shutil.rmtree, self.temp, True)

    def test_the_words_rebuild_each_sample_from_its_records(self):
        words = self.fixtures.words()
        index = json.loads((self.fixtures.SAMPLES / "states.json").read_text(encoding="utf-8"))
        states = [state["state"] for state in index["states"]]
        self.assertEqual(sorted(set(states) - set(words)), sorted(self.fixtures.NOT_BUILT))
        self.assertFalse(set(words) - set(states))
        lengths = {state: len(words[state]) for state in ("on-zero", "wj-ia-300", "wj-ia-420", "wj-ia-end", "ia-40",
                                                          "deep-1", "s7001-0")}
        self.assertEqual(lengths, {"on-zero": 0, "wj-ia-300": 300, "wj-ia-420": 420, "wj-ia-end": 841, "ia-40": 102,
                                   "deep-1": 3000, "s7001-0": 10})
        self.assertEqual(words["wj-ia-300"], words["wj-ia-end"][:300])
        self.assertEqual(words["wj-ia-300"][0], [57, 17], "record 0: grip 57, label I_a[17]")
        self.assertEqual(words["deep-2"][:3000], words["deep-1"])
        self.assertEqual(words["deep-2"][-12:], self.fixtures.H4_LETTERS)
        self.assertEqual(words["s7001-9"][:10], words["s7001-0"])
        # The research records are the ones the samples were made from.
        spec = importlib.util.spec_from_file_location("make_d02_samples", ROOT / "tools/looklab/fixtures/make_d02_samples.py")
        samples = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(samples)
        for name in ("WJ", "DEEP", "S7001", "S7001_WORDS", "H4_LETTERS"):
            self.assertEqual(getattr(self.fixtures, name), getattr(samples, name), name)
        # The menu's identity item moves no piece; the stop-40 restorer has four of its letters.
        self.assertEqual(self.fixtures.identity_items(), {0})
        kept, left_out = self.fixtures.moving(words["ia-40"], {0})
        self.assertEqual((len(kept), left_out), (98, 4))
        self.assertEqual(self.fixtures.moving(words["wj-ia-end"], {0})[1], 0)

    def written(self, record):
        def run(command, **options):
            folder = Path(command[command.index("--out") + 1])
            (folder / "session").mkdir(parents=True)
            (folder / "fixture.json").write_text(json.dumps(record), encoding="utf-8")
            return subprocess.CompletedProcess(command, 0, stdout="fixture: jumble-on\nreopen_ms: 12\n", stderr="")
        return run

    def test_a_fixture_must_match_its_sample_by_all_three_digests(self):
        sample = self.fixtures.sample_digests("wj-ia-300")
        letters = [[57, 17]]
        good = {"digest": sample["adapter"], "word": letters,
                "state_digests": {"digest": sample["adapter"], "pose_ids_sha256": sample["pose_ids_sha256"],
                                  "pose_lattice_sha256": sample["pose_lattice_sha256"]}}
        out = self.temp / "fixtures"
        row = self.fixtures.write(Path("writer.exe"), "wj-ia-300", letters, out, run=self.written(good))
        self.assertEqual((row["kept"], row["reopen_ms"], row["digest"]), (False, 12, sample["adapter"]))
        word = json.loads((out / "wj-ia-300" / "word.json").read_text(encoding="utf-8"))
        self.assertEqual((word["format"], word["letters"]), (self.fixtures.WORD_FORMAT, letters))

        def refuse(command, **options):
            raise AssertionError("a matching fixture is written again")
        self.assertTrue(self.fixtures.write(Path("writer.exe"), "wj-ia-300", letters, out, run=refuse)["kept"])

        for name in ("digest", "pose_ids_sha256", "pose_lattice_sha256"):
            shutil.rmtree(out / "wj-ia-300")
            bad = json.loads(json.dumps(good))
            if name == "digest":
                bad["digest"] = "0" * 64
            else:
                bad["state_digests"][name] = "0" * 64
            with self.assertRaises(self.fixtures.FixtureError, msg=name):
                self.fixtures.write(Path("writer.exe"), "wj-ia-300", letters, out, run=self.written(bad))
            self.assertFalse((out / "wj-ia-300").exists(), name)
            (out / "wj-ia-300").mkdir()

    def test_the_watch_command_and_the_runner_open_a_written_fixture(self):
        watch = shell_module("watch_d02")
        run = shell_module("d02_run")
        fixtures = self.temp / "d02-fixtures"
        for module in (watch, run):
            old = module.FIXTURES
            module.FIXTURES = fixtures
            self.addCleanup(setattr, module, "FIXTURES", old)
        build, out = self.temp / "build", self.temp / "out"
        self.assertNotIn("--d02-fixture", watch.launch_arguments(build, out, "wj-ia-300"))
        with self.assertRaises(ValueError):
            run.run("bind", out, build, "wj-ia-300", desktop=object(), launcher=None)
        (fixtures / "wj-ia-300").mkdir(parents=True)
        (fixtures / "wj-ia-300" / "fixture.json").write_text("{}", encoding="utf-8")
        arguments = watch.launch_arguments(build, out, "wj-ia-300")
        self.assertEqual(arguments[arguments.index("--d02-fixture") + 1], str(fixtures / "wj-ia-300"))
        self.assertNotIn("--d02-fixture", watch.launch_arguments(build, out, "wj-ia-420"))

    def test_the_bind_expectation_needs_bound_unbound_bound(self):
        run = shell_module("d02_run")
        steps = ["start", "wait 3000 ms", "chord J", "wait 1500 ms", "chord Ctrl+Z", "wait 3000 ms"]

        def dumps(chips):
            return [{"do": do, "lines": [["frame", "D02 · wj-ia-300"]] + ([["frame", chip]] if chip else [])}
                    for do, chip in zip(steps, chips)]
        good = ["unbound · stale", "bound · wj-ia-300", None, "unbound · stale", None, "bound · wj-ia-300"]
        self.assertEqual(run.expect_bind(dumps(good), {}, "wj-ia-300"), [])
        stuck = good[:3] + ["bound · wj-ia-300"] + good[4:]
        self.assertEqual(len(run.expect_bind(dumps(stuck), {}, "wj-ia-300")), 1)
        self.assertEqual(len(run.expect_bind(dumps(good[:5] + ["unbound · stale"]), {}, "wj-ia-300")), 1)
        self.assertTrue(run.expect_bind(dumps(good)[:4], {}, "wj-ia-300"))
        self.assertTrue((SHELL / "d02_scripts" / "bind.json").is_file())
        self.assertEqual(run.BOUND, "bound · ")
        jumble = text(PROJECT / "D02" / "ShellJumble.cs")
        self.assertIn('public const string Unbound = "unbound · stale";', jumble)
        self.assertIn('public static string BoundWords(string state) => "bound · " + state;', jumble)

    def test_the_host_gives_the_state_digest_and_the_shell_binds_only_by_it(self):
        lib = text(ROOT / "renderer" / "bridge" / "host" / "src" / "lib.rs")
        self.assertIn("pub const M6H_ABI_VERSION: u32 = 14;", lib)
        self.assertIn('pub unsafe extern "C" fn m6h_state_digest(handle: *mut ShellHost) -> i32', lib)
        self.assertIn("public const uint AbiVersion = 14;", text(PROJECT / "HostLibrary.cs"))
        self.assertIn("refuses another version than its `AbiVersion` (14 since ADR-D02 B17's picks; 13 at B23's Hopf "
                      "rings; 12 at B18c's view focus; 11 at B18b's two views; 10 at B14's style, 9 at the view shape, 8 at B5)",
                      text(SHELL / "README.md"))
        jumble = text(PROJECT / "D02" / "ShellJumble.cs")
        # A sample binds by its state digest and its two pose-array digests (checkpoint C1: pose ids are ranks in a
        # table compacted per state); the asked state only breaks a tie between equal states.
        self.assertIn('Key(root.GetProperty("digest").GetString(), root.GetProperty("pose_ids_sha256").GetString(),', jumble)
        self.assertIn('root.GetProperty("pose_lattice_sha256").GetString()), out var ids)', jumble)
        self.assertIn('var key = Key(digests.GetProperty("adapter").GetString(), digests.GetProperty("pose_ids_sha256").GetString(),', jumble)
        self.assertIn("bound = Asked is { } asked && ids.Contains(asked) ? asked : ids[0];", jumble)
        self.assertEqual(jumble.count("Bound = "), 1)
        shell = text(PROJECT / "Shell.cs")
        self.assertIn("if (D02.ShellJumble.On && _session && _adoptedRevision != _boundRevision)", shell)
        self.assertIn("_adoptedRevision = frame.EngineRevision;", shell)
        self.assertEqual(shell.count("StateDigest("), 1)


class D02Shape(unittest.TestCase):
    """ADR-D02's view shape (host version 9 over the core's ABI 6): the shell sets the owner's opening values from Look
    Lab's opening file, only with D02 on; the values live in the file, never in the shell's code."""

    def test_the_host_and_the_shell_expose_the_view_shape(self):
        lib = text(ROOT / "renderer" / "bridge" / "host" / "src" / "lib.rs")
        self.assertIn('pub unsafe extern "C" fn m6h_set_view_shape(handle: *mut ShellHost, view: u64, '
                      'shape: *const f32) -> i32', lib)
        self.assertIn("shell.host.backend.set_view_shape(id, shape)", lib)
        host = text(PROJECT / "HostLibrary.cs")
        self.assertIn("public int SetViewShape(IntPtr host, ulong view, float[]? shape)", host)
        self.assertIn('_exports["m6h_set_view_shape"])(host, view, s)', host)
        self.assertIn('"sa2_view_shape_set"', text(PROJECT / "CoreLibrary.cs"))

    def test_the_shell_reads_the_opening_values_only_with_d02_on(self):
        shape = text(PROJECT / "D02" / "ShellShape.cs")
        # Set sends the shape and keeps it as the opening the wheel zoom scales (owner, 26 November 2026).
        self.assertIn("ShellJumble.On ? Set(library, host, HostLibrary.View, Read(OpeningPath())) : "
                      "HostLibrary.Ok", shape)
        self.assertIn("var code = library.SetViewShape(host, view, shape);", shape)
        self.assertIn('return new[] { 1.0f - gap, sticker, D4, zoom };', shape)
        # B23: with the Hopf rings the file's field of view gives the zoom, as Look Lab's does.
        self.assertIn("zoom = (float)(1 / Math.Tan(fov.GetDouble() / 2));", shape)
        self.assertIn('GetProperty("gap")', shape)
        self.assertIn('GetProperty("stickerShrink")', shape)
        self.assertIn('ProjectSettings.GlobalizePath("res://look/opening.json")', shape)
        # The chain projection keeps the core's zoom until the Hopf-ring layout exists; the file's field of view
        # belongs to that layout.
        self.assertIn("public const float Zoom = 1.15f;", shape)
        self.assertIn("Hopf-ring layout", shape)
        # No opening value is a constant in code.
        self.assertNotRegex(shape, r"0\.3f?\b|0\.55f?\b|0\.7f?\b")
        opening = json.loads(text(ROOT / "tools" / "looklab" / "data" / "opening.json"))
        self.assertEqual(opening["format"], "magic600-look-opening")
        gap, sticker = opening["params"]["gap"], opening["params"]["stickerShrink"]
        self.assertTrue(0 < 1 - gap <= 1 and 0 < sticker <= 1, "the opening values lie in the core's range")
        self.assertLess(sticker, 0.82, "the opening stickers are smaller than the core's default")

    def test_the_build_stages_the_opening_file(self):
        build_shell = shell_module("build_shell")
        temp = Path(tempfile.mkdtemp(prefix="c9-shape-"))
        self.addCleanup(shutil.rmtree, temp, True)
        project = temp / "staged"
        build_shell.stage(project)
        self.assertEqual((project / "look" / "opening.json").read_bytes(),
                         (ROOT / "tools" / "looklab" / "data" / "opening.json").read_bytes())
        self.assertTrue((project / "D02" / "ShellShape.cs").is_file())



class D02Panes(unittest.TestCase):
    """ADR-D02 B18b (host version 11 over the core's ABI 8): Local and the composite frame, the same on both sides of
    the host library's C interface. The shell calls them from B18c on, only with --d02 (tests/test_d02_drawing.py)."""

    def test_the_frame_row_and_the_view_numbers_match(self):
        lib = text(ROOT / "renderer" / "bridge" / "host" / "src" / "lib.rs")
        host = text(PROJECT / "HostLibrary.cs")
        rust_row = re.search(r"#\[repr\(C\)\][^{]*?pub struct M6hFrame \{(.*?)\}", lib, re.S)
        csharp_row = re.search(r"LayoutKind\.Sequential\)\]\s*public struct Submitted\s*\{(.*?)\}", host, re.S)
        self.assertTrue(rust_row and csharp_row)
        self.assertEqual(re.findall(r"pub (\w+): u32,", rust_row.group(1)), ["panes", "held"])
        self.assertEqual(re.findall(r"public uint (\w+);", csharp_row.group(1)), ["Panes", "Held"])
        self.assertIn("size_of::<M6hFrame>() == 48", lib)
        self.assertIn("pub const LOCAL: ViewId = ViewId(2);", lib)
        self.assertIn("public const ulong Local = 2;", host)
        self.assertIn("pub const M6H_PANES_MAX: u32 = 4;", lib)
        self.assertIn("public const uint PanesMax = 4;", host)
        self.assertIn("const _: () = assert!(M6H_PANES_MAX == PANES_MAX);", lib)

    def test_one_view_frames_stay_the_path_without_panes(self):
        lib = text(ROOT / "renderer" / "bridge" / "host" / "src" / "lib.rs")
        self.assertIn("if shell.panes.iter().all(Option::is_none) { shell.frame_one() } else { shell.frame_panes() }",
                      lib)
        # B18c: Local opens only with --d02; without it the shell sets no pane and draws one-view frames.
        shell = text(PROJECT / "Shell.cs")
        self.assertEqual(shell.count("OpenLocal(hostLibrary);"), 1)
        self.assertIn("if (D02.ShellJumble.On)\n        {\n            OpenLocal(hostLibrary);", shell)
        self.assertIn("if (_localOpen && area == _area)", shell)


class D02Host(unittest.TestCase):
    """ADR-D02 build step B6: the Jumble host in Solve exists only with D02 on, is fed only from the host's state
    digests at a shown revision, switches the numerals on the exact lattice flag, clears each view's marks before it
    draws them, and the host script's expectation sees each part it checks. Nothing is built or launched."""

    def test_the_jumble_host_is_built_only_with_d02_on(self):
        chrome = text(PROJECT / "ShellChrome.cs")
        start = chrome.index("        if (ShellJumble.On)\n        {\n            ShellJumble.Attach(parent, frame, palette, () => cards?.ObservePicks());")
        block = chrome[start:chrome.index("\n        }\n", start)]
        self.assertEqual(chrome.count("new JumbleHost("), 1)
        self.assertIn('Jumble = new JumbleHost(frame, ShellJumble.Feed, names, empty, nothing, sentences["totals.unit"], '
                      'sentences["totals.above"]);', block)
        # B7: Global's totals follow the binding, pending while nothing is bound; B8: the cap map follows it too.
        changed = block[block.index("ShellJumble.BindingChanged += () =>"):block.index("};", block.index("ShellJumble.BindingChanged += () =>"))]
        self.assertIn("host.SetTotals(ShellJumble.Totals());", changed)
        self.assertIn("host.SetCapMap(layout, ShellJumble.CapMap());", changed)
        self.assertEqual(block.count("host.SetCapMap(layout, ShellJumble.CapMap());"), 2)
        self.assertIn("var layout = ShellJumble.Layout();", block)
        self.assertIn("frame.SetMainViews(global, Jumble.Surface);", block)
        # The feed adopts only what m6h_state_digest answered at a newly shown revision.
        jumble = text(PROJECT / "D02" / "ShellJumble.cs")
        self.assertIn("Feed.Adopt(JumbleFeed.Read(digests, revision));", jumble)
        # An answer that does not read, or a failed call, unbinds and keeps the numerals undefined (checkpoint C1).
        self.assertIn("Feed.Adopt(Feed.State with { LatticeFlag = false, Revision = revision });", jumble)
        self.assertEqual(jumble.count("Feed.Adopt("), 2)
        self.assertIn("D02.ShellJumble.Unknown((long)_adoptedRevision);", text(PROJECT / "Shell.cs"))
        self.assertIn("D02.ShellJumble.Bind(library.Answer(_host), (long)_adoptedRevision);", text(PROJECT / "Shell.cs"))

    def test_the_numerals_switch_on_the_lattice_flag_and_the_entry_is_chrome_only(self):
        model = text(ROOT / "tools" / "looklab" / "app" / "Chrome" / "Model" / "Jumble.cs")
        self.assertIn("public static bool Undefined(JumbleState state) => state.Mode && !state.LatticeFlag;", model)
        self.assertIn('root.GetProperty("off_lattice").GetInt64() == 0', model)
        host = text(ROOT / "tools" / "looklab" / "app" / "Chrome" / "Ui" / "JumbleHost.cs")
        self.assertIn("bool undefined = JumbleNumerals.Undefined(state);", host)
        self.assertEqual(host.count("SetUndefined("), 2)
        # The host adds one panel to the existing bottom dock, and the Focus layer editor to the right dock (owner,
        # 26 November 2026: the 0.4 Piece Filter in the default view), and no layout and no command.
        # B25 adds the Word panel to the left dock beside the Library, and B26 the ν-corrections rows.
        self.assertEqual(host.count("AddDockPanel("), 4)
        self.assertIn('frame.AddDockPanel(DockSide.Left, NuCorrectionsPanel.PanelId, "ν-corrections", "library.tool", Corrections);', host)
        self.assertIn('frame.AddDockPanel(DockSide.Left, WordPanel.PanelId, "Word", "panel.history", Words);', host)
        self.assertIn('frame.AddDockPanel(DockSide.Right, FocusLayerEditor.PanelId, "Focus layer", "layer.cycle", FilterEditor);', host)
        for word in ("AddLayout", "Commands.", "Invoke("):
            self.assertNotIn(word, host)
        icons = json.loads(text(ROOT / "tools" / "looklab" / "data" / "design" / "icons.json"))
        self.assertIn("mark.jumble", json.dumps(icons))
        self.assertIn("panel.capmap", json.dumps(icons))

    def test_each_view_is_cleared_before_its_marks(self):
        chrome = text(PROJECT / "ShellChrome.cs")
        describe = chrome[chrome.index("    private void Describe()"):]
        for view in ("Global", "Local"):
            clear = describe.index(f"Frame.{view}.ClearMarks();")
            self.assertLess(clear, describe.index(f"Jumble?.Marks(Frame.{view});"))
            sample = describe.index(f"Sample?.Marks(Frame.{view});")
            self.assertLess(clear, sample)
            self.assertLess(sample, describe.index(f"ShellSession.Marks(Frame.{view});"))

    def test_the_host_expectation_sees_each_part(self):
        run = shell_module("d02_run")
        self.assertEqual(run.IN_SAMPLE_MODE, {"host", "nu-band", "fit-2000", "fit-1280", "bind", "two-views",
                                              "library-branch", "research-pane", "research", "documents", "timer"})
        self.assertIn("host", run.ON_FIXTURE)
        self.assertTrue((SHELL / "d02_scripts" / "host.json").is_file())
        steps = ["start", "wait 3000 ms", "wait 3000 ms"]

        samples = ROOT / "tools" / "looklab" / "data" / "samples" / "jumble"

        def dumps(cell, undefined, marks=True, state="ia-40", drop=None, empty=False):
            # B8: the cap map's head and its class counts; a sample without rim rows leaves every cap pending.
            classes = json.loads(text(samples / state / "totals.json"))["cap_classes"]
            if not (samples / state / "rim.json").is_file():
                classes = {"blocked": 0, "closed": 0, "at_floor": 0, "above_floor": 0, "pending": 600}
            words = {"blocked": "blocked", "closed": "closed", "at_floor": "at floor (N_c = F_c > 0)",
                     "above_floor": "above floor", "pending": "pending"}
            # Local is never empty (owner, 26 November 2026): the default cap 0, its readout saying so; with
            # empty=True, the old empty Local.
            lines = [["frame", cell], ["view:Local", run.LOCAL_EMPTY if empty else run.LOCAL_DEFAULT],
                     ["panel:jumble.cap_map", "no cap named"], ["panel:jumble.cap_map", "Cap map · 20 rings of 30 caps"],
                     ["panel:jumble.cap_map", "contact classes · armed   never hidden · armed"]]
            lines += [["panel:jumble.cap_map", f"{words[key]} · {classes[key]}"] for key in words if key != drop]
            lines += [["view:Local", f"▸ {name} · 0"] for name in ("Named cap", "Twist check", "Cap covers", "Repair list")]
            lines += [["frame", "undefined"]] * undefined
            tips = {"Global": ["jumble: Jumble is on"], "Local": ["jumble: Jumble is on"]} if marks else {}
            local = {"cap": -1 if empty else 0, "framed": -1, "opening": False, "words": "" if empty else run.LOCAL_DEFAULT}
            return [{"do": do, "lines": lines, "marks": tips, "named": -1, "local": local} for do in steps]
        self.assertEqual(run.expect_host(dumps("Jumble · off the lattice", 2), {}, "ia-40"), [])
        self.assertEqual(len(run.expect_host(dumps("Jumble · off the lattice", 2, empty=True), {}, "ia-40")), 3)
        self.assertEqual(run.expect_host(dumps("Jumble · on the lattice", 0, state="on-zero"), {}, "on-zero"), [])
        self.assertEqual(len(run.expect_host(dumps("Jumble · off the lattice", 2, drop="blocked"), {}, "ia-40")), 1)
        self.assertEqual(len(run.expect_host(dumps("Jumble · on the lattice", 0, state="ia-40"), {}, "on-zero")), 5)
        self.assertEqual(len(run.expect_host(dumps("Jumble · off the lattice", 2, marks=False), {}, "ia-40")), 2)
        self.assertEqual(len(run.expect_host(dumps("Jumble · on the lattice", 2), {}, "ia-40")), 1)
        self.assertEqual(len(run.expect_host(dumps("Jumble · on the lattice", 1, state="on-zero"), {}, "on-zero")), 1)
        self.assertTrue(run.expect_host(dumps("Jumble · on the lattice", 0, state="on-zero")[:2], {}, "on-zero"))
        self.assertTrue(run.expect_host(dumps("Jumble · on the lattice", 0), {}, None))


class D02NameCap(unittest.TestCase):
    """B9: the four CR-14 draft rows for naming and the threshold work through the palette, the host carries them out
    without invoking the command registry, and the name-cap expectation sees each part it checks."""

    def test_the_host_carries_the_naming_and_threshold_rows(self):
        chrome = text(PROJECT / "ShellChrome.cs")
        self.assertIn("_ when Jumble != null && Jumble.Handles(command) => Jumble.Unavailable(command, argument),", chrome)
        self.assertLess(chrome.index("Jumble.Handles(command) => Jumble.Unavailable"), chrome.index("ShellJumble.Drafts.Contains(command)"))
        self.assertIn("case var jumble when Jumble != null && Jumble.Handles(jumble): Jumble.Execute(command, argument); break;", chrome)
        self.assertIn("host.SetMeeting(ShellJumble.Meeting());", chrome)
        host = text(ROOT / "tools" / "looklab" / "app" / "Chrome" / "Ui" / "JumbleHost.cs")
        for row in ("jumble.cap.name", "jumble.cap.unname", "jumble.capmap.threshold.set", "jumble.capmap.threshold.clear"):
            self.assertIn(f'"{row}"', host)
        rows = json.loads(text(ROOT / "tools" / "looklab" / "data" / "design" / "command-rows-d02.json"))["commands"]
        self.assertTrue({"jumble.cap.name", "jumble.cap.unname", "jumble.capmap.threshold.set", "jumble.capmap.threshold.clear"}
                        <= {row["id"] for row in rows if row["request"] == "CR-14"})

    def test_the_name_cap_expectation_sees_each_part(self):
        run = shell_module("d02_run")
        self.assertIn("name-cap", run.ON_FIXTURE)
        self.assertNotIn("name-cap", run.IN_SAMPLE_MODE)
        self.assertTrue((SHELL / "d02_scripts" / "name-cap.json").is_file())
        rows = json.loads(text(ROOT / "tools" / "looklab" / "data" / "samples" / "jumble" / "ia-40" / "rim.json"))["rows"]
        row = rows[30]
        admissible = [r for r in rows if r["status"] == "admissible"]
        members = sum(1 for r in admissible if r["N_c"] >= 50)
        blocked = sum(1 for r in rows if r["status"] == "blocked")
        counts = f"members {members} · non-members {len(admissible) - members} · blocked {blocked} · pending 0"
        panel = "panel:jumble.cap_map"
        steps = ["start", "wait 2000 ms", "chord F1", "text > jumble.cap.name 30", "chord Enter", "wait 1500 ms", "chord F1",
                 "text > jumble.capmap.threshold.set N_c >= 50", "chord Enter", "wait 3000 ms"]
        # Local is never empty (owner, 26 November 2026): before the naming it draws the default cap 0 and says so.
        unnamed = [[panel, "no cap named"], ["view:Local", "▸ Named cap · 0"], ["view:Local", run.LOCAL_DEFAULT]]
        tier1 = [[panel, f"cap 30 · N_c {row['N_c']} · floor {row['F_c']} · u 0 · 80 meeting caps"], ["view:Local", "▸ Named cap · 1"]]
        tier1 += [["view:Local", str(row[key])] for key in ("In_c", "Out_c", "A_c", "N_c")]

        def dumps(count=counts, empty=False, kept=30):
            default = {"named": -1, "local": {"cap": 0, "framed": 0, "opening": True, "words": run.LOCAL_DEFAULT}}
            if empty:
                default = {"named": -1, "local": {"cap": -1, "framed": -1, "opening": False, "words": ""}}
            before = [["view:Local", run.LOCAL_EMPTY] if empty and line[1] == run.LOCAL_DEFAULT else line for line in unnamed]
            after = {"named": 30, "local": {"cap": 30, "framed": kept, "opening": True, "words": ""}}
            out = [{"do": do, "lines": before, **default} for do in steps[:5]]
            out += [{"do": do, "lines": tier1, **after} for do in steps[5:9]]
            return out + [{"do": steps[9], "lines": tier1 + [[panel, count], [panel, "N_c ≥ 50"]], **after}]
        self.assertEqual(run.expect_name_cap(dumps(), {}, "ia-40"), [])
        self.assertEqual(len(run.expect_name_cap(dumps(empty=True), {}, "ia-40")), 4)
        self.assertEqual(len(run.expect_name_cap(dumps(kept=0), {}, "ia-40")), 2)
        self.assertEqual(len(run.expect_name_cap(dumps(counts.replace("blocked", "blocked 0 ·", 1)), {}, "ia-40")), 1)
        self.assertTrue(run.expect_name_cap(dumps()[:5], {}, "ia-40"))
        self.assertTrue(run.expect_name_cap(dumps(), {}, "on-zero"))


class D02CapPrompt(unittest.TestCase):
    """The owner's bug report of 26 November 2026: jumble.cap.name without a number opens the cap prompt, its caps in
    cap-id order with nothing highlighted, and the run's expectation sees each part it checks."""

    def test_the_row_without_a_number_opens_the_prompt(self):
        host = text(ROOT / "tools" / "looklab" / "app" / "Chrome" / "Ui" / "JumbleHost.cs")
        self.assertIn('case "jumble.cap.name" when argument == null: Prompt.Open(); break;', host)
        self.assertNotIn("it needs a cap number", host)
        self.assertIn("frame.Overlays.AddChild(Prompt);", host)
        prompt = text(ROOT / "tools" / "looklab" / "app" / "Chrome" / "Model" / "CapPrompt.cs")
        for word in ("OrderBy", "Sort(", "Highlighted = 0"):
            self.assertNotIn(word, prompt)
        self.assertIn("Enumerable.Range(0, CapLayout.Caps)", prompt)

    def test_the_cap_prompt_expectation_sees_each_part(self):
        run = shell_module("d02_run")
        self.assertIn("cap-prompt", run.ON_FIXTURE)
        self.assertNotIn("cap-prompt", run.IN_SAMPLE_MODE)
        script = json.loads(text(SHELL / "d02_scripts" / "cap-prompt.json"))
        written = []
        for step in script["steps"]:
            (kind, value), = step.items()
            written.append({"wait_ms": f"wait {value} ms", "chord": f"chord {value}", "text": f"text {value}",
                            "click_control": f"click {value}"}[kind])
        self.assertEqual(["start"] + written, run.CAP_PROMPT_STEPS)
        rows = {row["cap"]: row for row in json.loads(text(ROOT / "tools" / "looklab" / "data" / "samples" / "jumble" / "ia-40" / "rim.json"))["rows"]}
        self.assertEqual(run.prompt_label(rows[0]), "cap 0 · blocked · N_c undefined")
        self.assertEqual(run.prompt_label(rows[1]), "cap 1 · closed · N_c 0")
        panel = "panel:jumble.cap_map"

        def dumps(first=None, window=None):
            out = []
            for index, do in enumerate(run.CAP_PROMPT_STEPS):
                prompt = {"open": index in (4, 5, 6, 7, 12, 17), "rows": 600, "highlighted": -1, "window": list(range(12))}
                named, lines = -1, []
                if index in (4, 12, 17):
                    lines = [["overlay:CapPrompt", run.prompt_label(rows[cap])] for cap in range(12)]
                    lines.append(["overlay:CapPrompt", "Name a cap · cap 150 is named now" if index == 12 else "Name a cap"])
                if index == 4 and first is not None:
                    prompt["highlighted"] = first
                if index == 5:
                    prompt.update(rows=11, window=window or [15] + list(range(150, 160)))
                if index in (6, 7):
                    prompt.update(rows=11, window=[15] + list(range(150, 160)), highlighted=15 if index == 6 else 150)
                if 8 <= index < 13:
                    named = 150
                    lines.append([panel, "cap 150 · N_c 0 · floor 0 · u 0 · 80 meeting caps"])
                if index >= 13:
                    named = 7
                out.append({"do": do, "lines": lines, "cap_prompt": prompt, "named": named})
            return out
        self.assertEqual(run.expect_cap_prompt(dumps(), {}, "ia-40"), [])
        self.assertEqual(len(run.expect_cap_prompt(dumps(first=0), {}, "ia-40")), 1)
        self.assertEqual(len(run.expect_cap_prompt(dumps(window=list(range(150, 160)) + [15]), {}, "ia-40")), 1)
        self.assertTrue(run.expect_cap_prompt(dumps()[:8], {}, "ia-40"))
        self.assertTrue(run.expect_cap_prompt(dumps(), {}, "on-zero"))


class D02CapHover(unittest.TestCase):
    """The owner's bug report of 26 November 2026: the cap map's cells say on hover that a click names the cap, and a
    click that names nothing says why; the run's expectation sees each part it checks."""

    def test_the_map_names_through_the_host(self):
        host = text(ROOT / "tools" / "looklab" / "app" / "Chrome" / "Ui" / "JumbleHost.cs")
        self.assertIn("CapMap.Map.Picked += cap => Click(cap);", host)
        self.assertIn("CapMap.Map.Refusal = Refusal;", host)
        self.assertIn("if (Refusal(cap) is { } why) { frame.Say(why, true); return false; }", host)
        cells = text(ROOT / "tools" / "looklab" / "app" / "Chrome" / "Ui" / "CapMapControl.cs")
        self.assertIn("MouseDefaultCursorShape = cap == null ? CursorShape.Arrow : CursorShape.PointingHand;", cells)
        self.assertIn("CapNaming.Tip(Refusal?.Invoke(cap), Describe(cap))", cells)

    def test_the_cap_hover_expectation_sees_each_part(self):
        run = shell_module("d02_run")
        self.assertIn("cap-hover", run.ON_FIXTURE)
        self.assertNotIn("cap-hover", run.IN_SAMPLE_MODE)
        script = json.loads(text(SHELL / "d02_scripts" / "cap-hover.json"))
        written = []
        for step in script["steps"]:
            (kind, value), = step.items()
            written.append({"wait_ms": f"wait {value} ms", "cap_hover": f"cap hover {value}", "cap_click": f"cap click {value}"}[kind])
        self.assertEqual(["start"] + written, run.CAP_HOVER_STEPS)
        refused = "cap 1 is already named; F1 › jumble.cap.unname un-names it"
        panel = "panel:jumble.cap_map"

        def dumps(cursor="PointingHand", said=True, tip=run.HINT):
            out = []
            for index, do in enumerate(run.CAP_HOVER_STEPS):
                hovered = {2: 1, 3: 1, 4: 1, 5: 0, 6: 0}.get(index, -1)
                named = {3: 1, 4: 1, 5: 1, 6: 0}.get(index, -1)
                first = refused if index == 4 else tip
                cell = {"hovered": hovered, "cursor": cursor if hovered >= 0 else "Arrow",
                        "tooltip": f"{first}\ncap {hovered} · ring 1" if hovered >= 0 else ""}
                lines = [[panel, "no cap named"]] if named < 0 else [[panel, f"cap {named} · N_c 0"]]
                if index == 4 and said:
                    lines.append(["frame", refused])
                out.append({"do": do, "lines": lines, "cap_map": cell, "named": named})
            return out
        self.assertEqual(run.expect_cap_hover(dumps(), {}, "ia-40"), [])
        self.assertEqual(len(run.expect_cap_hover(dumps(cursor="Arrow"), {}, "ia-40")), 2)
        self.assertEqual(len(run.expect_cap_hover(dumps(said=False), {}, "ia-40")), 1)
        self.assertEqual(len(run.expect_cap_hover(dumps(tip="cap"), {}, "ia-40")), 2)
        self.assertTrue(run.expect_cap_hover(dumps()[:5], {}, "ia-40"))


class D02LibraryBranch(unittest.TestCase):
    """The script player's command step runs a row as the palette does; in sample mode library.branch.toggle Built-in
    opens that Library branch and no error reaches the error lamp; the run's expectation sees each part it checks."""

    def test_the_player_runs_a_row_as_the_palette_does(self):
        player = text(PROJECT / "D02" / "ShellJumble.cs")
        self.assertIn('step.TryGetProperty("command", out var command)', player)
        self.assertIn("if (ShellChrome.Run is { } run)", player)
        self.assertIn("Safe.Raise(run, row, argument);", player)
        self.assertIn("Run = Execute;", text(PROJECT / "ShellChrome.cs"))

    def test_the_library_branch_expectation_sees_each_part(self):
        run = shell_module("d02_run")
        self.assertIn("library-branch", run.IN_SAMPLE_MODE)
        self.assertNotIn("library-branch", run.ON_FIXTURE)
        script = json.loads(text(SHELL / "d02_scripts" / "library-branch.json"))
        written = []
        for step in script["steps"]:
            if "command" in step:
                written.append(" ".join(["command", step["command"]] + ([step["argument"]] if "argument" in step else [])))
            else:
                written.append(f"wait {step['wait_ms']} ms")
        self.assertEqual(["start"] + written, run.LIBRARY_BRANCH_STEPS)

        def dumps(opens=True, count=0, lamp=True, loop=False):
            out = []
            for index, do in enumerate(run.LIBRARY_BRANCH_STEPS):
                opened = opens and index >= 3
                library = {"left_dock": "library" if index >= 2 else "", "panel": index >= 2,
                           "open": ["Built-in"] if opened else [], "rows": list(run.BUILT_IN) if opened else []}
                errors = {"lamp": lamp, "count": count if index >= 3 else 0, "failures": 0, "last": ""}
                out.append({"step": index, "do": do, "library": library, "errors": errors,
                            "rebuilds": 3 + (20 * index if loop else min(index, 3) - 1 if index > 1 else 0)})
            return out
        self.assertEqual(run.expect_library_branch(dumps(), {}, None), [])
        self.assertEqual(len(run.expect_library_branch(dumps(loop=True), {}, None)), 1)
        self.assertEqual(len(run.expect_library_branch(dumps(opens=False), {}, None)), 4)
        self.assertEqual(len(run.expect_library_branch(dumps(count=1), {}, None)), 2)
        self.assertEqual(len(run.expect_library_branch(dumps(lamp=False), {}, None)), 5)
        self.assertTrue(run.expect_library_branch(dumps()[:4], {}, None))


class D02Documents(unittest.TestCase):
    def test_the_shell_mounts_and_routes_documents(self):
        chrome = text(PROJECT / "ShellChrome.cs")
        mount = chrome.index("ShellDocuments.Load(frame, registry, sample);")
        self.assertLess(chrome.index("ShellResearch.Load(frame, registry"), mount)
        self.assertLess(mount, chrome.index("ChromeErrors.Mount(frame"))
        self.assertIn("ShellDocuments.Unavailable(command, argument)", chrome)
        self.assertIn("ShellDocuments.Execute(frame, command, argument);", chrome)
        documents = text(PROJECT / "D02" / "ShellDocuments.cs")
        self.assertIn("frame.SetDocumentRow(tabs);", documents)
        self.assertIn("frame.Model.Drop(pane, FrameModel.Pair, DropZone.Right)", documents)
        self.assertIn("ShellDocuments.Dump(json);", text(PROJECT / "D02" / "ShellJumble.cs"))

    def test_the_run_uses_sample_mode_and_the_document_expectation(self):
        run = shell_module("d02_run")
        self.assertIs(run.EXPECT["documents"], run.expect_documents)
        self.assertIn("documents", run.IN_SAMPLE_MODE)
        script = json.loads(text(SHELL / "d02_scripts" / "documents.json"))
        self.assertEqual(script["resolution"], "1280x720")
        written = ["start"]
        for step in script["steps"]:
            if "command" in step:
                written.append(" ".join(["command", step["command"]] + ([step["argument"]] if "argument" in step else [])))
            elif "click_control" in step:
                written.append(f"click {step['click_control']}")
            else:
                written.append(f"wait {step['wait_ms']} ms")
        self.assertEqual(written, run.DOCUMENT_STEPS)


class D02Session(unittest.TestCase):
    def test_the_shell_mounts_and_routes_the_session_rows(self):
        chrome = text(PROJECT / "ShellChrome.cs")
        mount = chrome.index("ShellSession.Load(frame, registry, sample, history, Moves, Execute);")
        self.assertLess(chrome.index("ShellDocuments.Load(frame, registry, sample);"), mount)
        self.assertLess(mount, chrome.index("ChromeErrors.Mount(frame"))
        self.assertIn("_ when ShellSession.Handles(command) => ShellSession.Unavailable(command)", chrome)
        self.assertIn("case var session when ShellSession.Handles(session):", chrome)
        self.assertIn("ShellSession.Execute(frame, command, argument);", chrome)
        self.assertIn("ShellSession.Describe(_live && _shown <= int.MaxValue ? (int)_shown : null);", chrome)
        history = chrome[chrome.index("history.Command += (command, argument)"):chrome.index("var find =")]
        self.assertIn('command == "session.completion"', history)
        self.assertIn("Execute(command, argument);", history)
        session = text(PROJECT / "D02" / "ShellSession.cs")
        rows = session[session.index("Rows.UnionWith"):session.index("ShellSession.frame = frame;")]
        for row in ("session.timer.start", "session.timer.stop", "session.practice", "session.completion"):
            self.assertIn(f'"{row}"', rows)
        self.assertIn("timerView.Command += row => execute(row, null);", session)
        self.assertIn("history.Completions = log;", session)
        dump = text(PROJECT / "D02" / "ShellJumble.cs")
        self.assertLess(dump.index("ShellDocuments.Dump(json);"), dump.index("ShellSession.Dump(json);"))

    def test_the_timer_run_uses_the_session_fixture_and_expectation(self):
        run = shell_module("d02_run")
        self.assertIs(run.EXPECT["timer"], run.expect_timer)
        self.assertIn("timer", run.IN_SAMPLE_MODE)
        self.assertIn("timer", run.ON_FIXTURE)
        script = json.loads(text(SHELL / "d02_scripts" / "timer.json"))
        self.assertEqual(script["resolution"], "1280x720")
        written = ["start"]
        for step in script["steps"]:
            if "command" in step:
                written.append(" ".join(["command", step["command"]] + ([step["argument"]] if "argument" in step else [])))
            elif "click_control" in step:
                written.append(f"click {step['click_control']}")
            else:
                written.append(f"wait {step['wait_ms']} ms")
        self.assertEqual(written, run.TIMER_STEPS)


class D02Q3b(unittest.TestCase):
    """B10: the q3b script's expectation is the Q3.b rhythm's own result, and it fails on a cancel that changed the
    screen or on a list that differs."""

    def test_the_q3b_expectation_follows_the_rhythm(self):
        run = shell_module("d02_run")
        self.assertIn("q3b", run.ON_FIXTURE)
        self.assertTrue((SHELL / "d02_scripts" / "q3b.json").is_file())
        self.assertEqual(run.SEAT_RHYTHM, [0, 1, 2, 6, 7, 12, 13, 14, 15, 20, 21, 22])
        host = text(ROOT / "tools" / "looklab" / "app" / "Chrome" / "Ui" / "JumbleHost.cs")
        self.assertIn("SeatRhythm = Array.AsReadOnly(new[] { 0, 1, 2, 6, 7, 12, 13, 14, 15, 20, 21, 22 });", host)
        self.assertIn("private readonly SampleJobs jobs = new(" + str(run.CHECK_TICKS) + ");", host)
        record = json.loads(text(ROOT / "tools" / "looklab" / "data" / "samples" / "jumble" / "wj-ia-300" / "cap-150.json"))
        kept = run.q3b_kept(record)
        self.assertEqual([line.startswith("not checked:") for line in kept], [True, False, True, False, True, False, True, False])
        self.assertEqual(kept[0], "not checked: 150·1 150·2")
        lines = [["view:Local", f"▾ Twist check · {len(kept)}"]] + [["view:Local", line] for line in kept if line.startswith("not")]

        closed = [["view:Local", f"▸ Twist check · {len(kept)}"]]

        def dumps(changed=False, rows=kept, control_after=closed):
            # C2-1: a cancel step carries the whole screen read before it; one that changed any region must count.
            after = lines + [["view:Global", "N 98,048"]] if changed else lines
            return [{"do": "start", "lines": []}, {"do": "wait 2000 ms", "lines": []},
                    {"do": "cancel older", "lines_before": lines, "lines": lines, "cancels": 4},
                    {"do": "cancel older", "lines_before": lines, "lines": after, "cancels": 99},
                    {"do": "wait 1000 ms", "lines": lines, "kept": rows, "cancels": 99},
                    {"do": "screen control", "lines_before": lines, "lines": control_after, "cancels": 99}]
        self.assertEqual(run.expect_q3b(dumps(), {}, "wj-ia-300"), [])
        # A change in a region other than the kept list counts too.
        changed = run.expect_q3b(dumps(changed=True), {}, "wj-ia-300")
        self.assertEqual(len(changed), 1)
        self.assertIn("cancel step 2 changed the screen", changed[0])
        # The negative control: a run whose two reads cannot see a change fails.
        blind = run.expect_q3b(dumps(control_after=lines), {}, "wj-ia-300")
        self.assertEqual(blind, ["the negative control's screen change was not counted; the two reads do not see the screen"])
        no_before = dumps()
        del no_before[2]["lines_before"]
        self.assertEqual(run.expect_q3b(no_before, {}, "wj-ia-300"), ["a cancel or control step's dump has no screen read before it"])
        self.assertEqual(len(run.expect_q3b(dumps(rows=kept[1:]), {}, "wj-ia-300")), 1)
        self.assertTrue(run.expect_q3b(dumps(), {}, "ia-40"))
        script = json.loads(text(SHELL / "d02_scripts" / "q3b.json"))
        self.assertEqual(script["steps"][-1], {"screen_control": True})
        # The cancel goes through the host, with the jobs' clock held; nothing compares only the kept list.
        driver = text(PROJECT / "D02" / "ShellJumble.cs")
        self.assertIn("_cancels += host.CancelOlder();", driver)
        self.assertIn("_screenBefore = ScreenText.Of(_frame);", driver)
        self.assertNotIn("kept.Cancel(", driver)
        self.assertIn("if (Paused) return;", host)


class D02Covers(unittest.TestCase):
    """B11: the covers script's expectation reads the record as CapCovers does, with the word record's heads, and it
    fails on a hover that marked the cap map or on a focus key without the sample's count."""

    def test_the_covers_expectation_follows_the_record(self):
        run = shell_module("d02_run")
        self.assertIn("covers", run.ON_FIXTURE)
        script = json.loads(text(SHELL / "d02_scripts" / "covers.json"))
        self.assertEqual(len(script["steps"]) + 1, len(run.COVERS_STEPS))
        model = text(ROOT / "tools" / "looklab" / "app" / "Chrome" / "Model" / "CapCovers.cs")
        words = {s["id"]: s["text"] for s in json.loads(text(ROOT / "tools" / "looklab" / "data" / "design" / "jumble-words.json"))["sentences"]}
        for key in ("covers.covered_by", "covers.covers", "covers.undefined"):
            self.assertIn(f'("{words[key]} · "', model)
        samples = ROOT / "tools" / "looklab" / "data" / "samples" / "jumble" / "wj-ia-300"
        record = json.loads(text(samples / "cap-150.json"))
        lines = run.covers_lines(record)
        self.assertEqual(lines[0], "The 80 caps that meet cap 150 · at least 10 aligned face pieces a side")
        self.assertTrue(lines[1].startswith("covered by · 36 of 80 answered: 8 10 14 "))
        self.assertEqual(lines[2], "covers · 0 of 3 answered")
        self.assertTrue(lines[3].startswith("undefined · 77 (blocked): 8 10 14 "))
        style = json.loads(text(samples / "style.json"))
        lattice = style["lattice_domain"]
        n4 = sum(r["count"] for r in style["rows"] if r["heights"][0] >= 4)
        nd = sum(r["count"] for r in style["rows"] if r["domain"] == lattice)
        marks = "|150|" + ",".join(map(str, range(80))) + "|"

        def dumps(hover_marks=marks, straddlers="Straddlers · 0"):
            out = [{"do": do, "lines": []} for do in run.COVERS_STEPS]
            out[7].update(covers=lines, map_marks=marks, lines=[["view:Local", "▾ Cap covers · 80"], ["view:Local", lines[0]]])
            out[9].update(inspect="cap 8 · blocked · N_c undefined", map_marks=hover_marks)
            for step, key, focused in ((13, f"Height ≥ 4 · {n4:,}", "jumble.height"),
                                       (17, f"Domain {lattice} (lattice) · {nd:,}", "jumble.domain"),
                                       (21, straddlers, "jumble.straddlers")):
                out[step]["focus"] = [f"{key} | focused {focused} | Normal"] * 2
            return out
        self.assertEqual(run.expect_covers(dumps(), {}, "wj-ia-300"), [])
        self.assertEqual(len(run.expect_covers(dumps(hover_marks="|150||8"), {}, "wj-ia-300")), 1)
        self.assertEqual(len(run.expect_covers(dumps(straddlers="Straddlers · 12"), {}, "wj-ia-300")), 2)
        self.assertTrue(run.expect_covers(dumps(), {}, "ia-40"))

    def test_the_host_carries_the_three_focus_rows_and_no_new_key(self):
        host = text(ROOT / "tools" / "looklab" / "app" / "Chrome" / "Ui" / "JumbleHost.cs")
        for row in ("jumble.focus.straddlers", "jumble.focus.domain", "jumble.focus.height"):
            self.assertIn(f'"{row}"', host)
        rows = json.loads(text(ROOT / "tools" / "looklab" / "data" / "design" / "command-rows-d02.json"))["commands"]
        ids = {row["id"] for row in rows}
        self.assertTrue({"jumble.focus.straddlers", "jumble.focus.domain", "jumble.focus.height"} <= ids)
        # The predicates join the existing cycling key (ADR-D01.46): the layer keys bind no new key.
        keys = text(ROOT / "tools" / "looklab" / "app" / "Chrome" / "Ui" / "Gallery.cs")
        self.assertIn("LayerPredicates", keys)
        self.assertNotIn("HostKeys", keys)


class D02Repair(unittest.TestCase):
    """B12: the repair script's expectation follows the record, the list takes its own keys in the shell (no digit
    reaches the session while it has focus), and the words are the word record's."""

    def test_the_repair_expectation_follows_the_record(self):
        run = shell_module("d02_run")
        self.assertIn("repair", run.ON_FIXTURE)
        script = json.loads(text(SHELL / "d02_scripts" / "repair.json"))
        self.assertEqual(len(script["steps"]) + 1, len(run.REPAIR_STEPS))
        record = json.loads(text(ROOT / "tools" / "looklab" / "data" / "samples" / "jumble" / "wj-ia-300" / "cap-150.json"))
        rows = record["repair_list"]["completed"]["rows"]
        moving = {s["row"]: s for s in record["repair_steps"]}[2]["moving_sets"][1]["count"]
        first = rows[0]
        local = [["view:Local", f"▾ Repair list · {len(rows)}"]] + [["view:Local", c] for c in
                 ("#", "x d", "y z", "ΔN", f"{first[0]} {first[1]}", f"{first[2]} {first[3]}", str(first[4]))]

        staged = "[" + " ".join(f"{c}·{i}" for c, i in {s["row"]: s for s in record["repair_steps"]}[2]["letters"]) + "]"

        def dumps(picks=(2,), opened_focus=0, unasked=0):
            out = [{"do": do, "lines": []} for do in run.REPAIR_STEPS]
            out[7].update(lines=local, repair={"rows": len(rows), "focused": opened_focus, "step": "", "picks": []},
                          stage={"word": "", "bar": "", "spans": 0, "records": 0, "asked": 0, "unasked": 0})
            out[12]["repair"] = {"rows": len(rows), "focused": 2, "step": "Repair 1 of 3", "picks": []}
            out[14].update(repair={"rows": len(rows), "focused": 2, "step": "Repair 2 of 3", "picks": []},
                           lines=[["view:Local", f"Repair 2 of 3 · moving {moving:,}"]])
            out[16]["repair"] = {"rows": len(rows), "focused": 2, "step": "Repair 2 of 3", "picks": list(picks)}
            out[16]["stage"] = {"word": staged, "bar": "Whole repair only", "spans": 1, "records": 1, "asked": 3, "unasked": unasked}
            out[16]["lines"] = [["view:Local", "Whole repair only"], ["view:Local", "Staged: " + staged]]
            return out
        self.assertEqual(run.expect_repair(dumps(), {}, "wj-ia-300"), [])
        self.assertEqual(len(run.expect_repair(dumps(opened_focus=1), {}, "wj-ia-300")), 1)
        self.assertEqual(len(run.expect_repair(dumps(picks=()), {}, "wj-ia-300")), 1)
        # B13: an unasked act on the boundary ledger fails the run.
        self.assertEqual(len(run.expect_repair(dumps(unasked=1), {}, "wj-ia-300")), 1)
        self.assertEqual(staged, "[150·1 215·21 150·1]")

    def test_only_the_pick_handler_stages(self):
        host = text(ROOT / "tools" / "looklab" / "app" / "Chrome" / "Ui" / "JumbleHost.cs")
        self.assertEqual(host.count("Stage.Stage("), 1)
        pick = host[host.index("public void Pick(RepairRow row, UserEvent asked)"):]
        self.assertIn("Stage.Stage(Repair, row, RepairSteps, asked, Origin.Solver);", pick[:pick.index(chr(10) + "    }")])
        ui = "".join(text(path) for path in (ROOT / "tools" / "looklab" / "app" / "Chrome" / "Ui").glob("*.cs"))
        self.assertEqual(ui.count(".Stage("), 1)
        lists = text(ROOT / "tools" / "looklab" / "app" / "Chrome" / "Ui" / "JumbleLists.cs")
        self.assertIn('Picked?.Invoke(focused, UserEvent.Key("Enter"))', lists)
        # C2-1's nit: a UserEvent is made only by an input handler, in the list control.
        app = ROOT / "tools" / "looklab" / "app"
        makers = sorted(path.name for path in app.rglob("*.cs") if "UserEvent.Key(" in text(path) or "UserEvent.Click(" in text(path))
        self.assertEqual(makers, ["JumbleHost.cs", "JumbleLists.cs", "PieceFilterUi.cs"])
        # The host makes one, for the palette's Enter on filter.apply; the focus layer editor makes its Apply click.
        host_text = text(app / "Chrome" / "Ui" / "JumbleHost.cs")
        self.assertEqual(host_text.count("UserEvent."), 1)
        self.assertIn('case "filter.apply": Filter?.Apply(UserEvent.Key("palette Enter"));', host_text)

    def test_every_staging_entry_admits_its_origin(self):
        """C2-6: the preview, the repair stage and typed twists take a required origin and admit it themselves, with
        the session's own flag; no caller passes a solving flag in."""
        model = ROOT / "tools" / "looklab" / "app" / "Chrome" / "Model"
        boundary, session = text(model / "Boundary.cs"), text(model / "Session.cs")
        self.assertIn("public SealedSpan? Stage(RepairListModel list, RepairRow row, RepairSteps steps, UserEvent? asked, Origin origin)", boundary)
        self.assertIn("public bool Type(Letter letter, Origin origin)", boundary)
        self.assertIn("internal void Type(Letter letter)", boundary)
        self.assertIn("string? refusal = PreviewAdmission.Admit(origin, solvingSession());", boundary)
        self.assertIn("public Preview Show(Origin origin, string statement,", session)
        self.assertIn("if (Admit(origin, SolvingSession?.Invoke() ?? true) is string refused)", session)
        host = text(ROOT / "tools" / "looklab" / "app" / "Chrome" / "Ui" / "JumbleHost.cs")
        self.assertIn("new RepairStage(Ledger, () => Source.State.SessionKind == JumbleSessionKind.Solving)", host)
        # Admission is called only by the staging entries and the tests, never with a flag a caller chose.
        callers = sorted(path.name for path in (ROOT / "tools" / "looklab" / "app").rglob("*.cs") if "Admit(" in text(path))
        self.assertEqual(callers, ["Boundary.cs", "Session.cs"])

    def test_the_list_takes_its_own_keys_and_the_words_are_the_record(self):
        frame = text(ROOT / "tools" / "looklab" / "app" / "Chrome" / "Ui" / "ChromeFrame.cs")
        self.assertIn("is LineEdit or TextEdit or IOwnKeys", frame)
        lists = text(ROOT / "tools" / "looklab" / "app" / "Chrome" / "Ui" / "JumbleLists.cs")
        self.assertIn("public sealed partial class RepairListControl : VBoxContainer, IOwnKeys", lists)
        self.assertIn("public sealed partial class CapMapControl : Control, IOwnKeys", text(ROOT / "tools" / "looklab" / "app" / "Chrome" / "Ui" / "CapMapControl.cs"))
        model = text(ROOT / "tools" / "looklab" / "app" / "Chrome" / "Model" / "RepairList.cs")
        words = {s["id"]: s for s in json.loads(text(ROOT / "tools" / "looklab" / "data" / "design" / "jumble-words.json"))["sentences"]}
        self.assertIn('"' + words["repair.whole"]["text"] + '"', model)
        self.assertIn('public const string Context = "repair.list.focused";', model)
        # The preview's step keys walk a row's steps: view.preview.scrub is Left and Right in the keymap.
        keymap = json.loads(text(ROOT / "tools" / "looklab" / "data" / "design" / "keymap.json"))
        scrub = sorted(b["key"] for b in keymap["bindings"] if b.get("command") == "view.preview.scrub")
        self.assertEqual(scrub, ["Left", "Right"])
        self.assertIn('case Key.Left: scrub.Scrub("back"); break;', lists)
        self.assertIn('case Key.Right: scrub.Scrub("forward"); break;', lists)


class D02Rotation(unittest.TestCase):
    """The view rotation the owner asked for on 26 November 2026: with --d02 a left-button drag in the Global opening
    turns the core's 4D camera as Look Lab's drag does, a click stays a click, and without --d02 nothing changes."""

    def test_the_drag_turns_the_camera_only_with_d02(self):
        shell = text(PROJECT / "Shell.cs")
        self.assertIn("var turned = D02.ShellJumble.On && Turn(@event);", shell)
        self.assertIn("var global = _camera.Follow(input, InOpening);", shell)
        self.assertIn("if (D02.ShellJumble.On && (_camera.Dirty || turning.Dirty))", shell)
        self.assertEqual(shell.count("SetCamera("), 1)
        camera = text(PROJECT / "D02" / "ShellCamera.cs")
        # Look Lab's drag: x-w and y-w for the 4D camera, x-z and y-z for the layout spin, 0.004 rad per pixel.
        self.assertIn("public const double RadiansPerPixel = .004;", camera)
        # The owner's report of 26 November 2026: left-right was reversed. The core applies Q as it is, where Look
        # Lab's shader applies its transpose, so the x–w turn takes the opposite angle; the y–w turn keeps its own.
        self.assertIn("Rotate(Q, 0, 3, -ax);", camera)
        self.assertNotIn("Rotate(Q, 0, 3, ax);", camera)
        self.assertIn("Rotate(Q, 1, 3, ay);", camera)
        self.assertIn("public const float DragThreshold = 3;", camera)
        # The core reads a column-major float4x4 (sa2_interop.h's camera_matrix).
        self.assertIn("m[column * 4 + row] = (float)Q[row * 4 + column];", camera)
        self.assertIn("column-major float4x4", text(ROOT / "renderer" / "core" / "include" / "sa2_interop.h"))

    def test_the_rotate_expectation_needs_a_turn_per_drag_and_none_per_wait(self):
        run = shell_module("d02_run")
        steps = ["start", "wait 2000 ms", "drag 200 0", "wait 1000 ms", "drag 0 120", "wait 1000 ms"]

        # Each step's camera as ShellCamera.Turn builds it: the x–w turn by -ax, then the y–w turn by ay, column-major.
        # The drags are 200 and 120 pixels at 0.004 rad per pixel; `sign` 1 is the fix, -1 the reversed build.
        angles = [(0, 0), (0, 0), (.8, 0), (.8, 0), (.8, .48), (.8, .48)]

        def camera(ax, ay, sign):
            q = [[float(i == j) for j in range(4)] for i in range(4)]
            for a, angle in ((0, -sign * ax), (1, ay)):
                c, s = math.cos(angle), math.sin(angle)
                for j in range(4):
                    x, y = q[a][j], q[3][j]
                    q[a][j], q[3][j] = c * x - s * y, s * x + c * y
            return [q[row][column] for column in range(4) for row in range(4)]

        def dumps(turns, sign=1):
            return [{"do": do, "camera_turns": t, "lines": [], "camera": camera(ax, ay, sign)}
                    for do, t, (ax, ay) in zip(steps, turns, angles)]
        self.assertEqual(run.expect_rotate(dumps([0, 0, 1, 1, 2, 2]), {}, None), [])
        # The reversed build of 26 November 2026 fails on the rightward drag only.
        reversed_drag = run.expect_rotate(dumps([0, 0, 1, 1, 2, 2], sign=-1), {}, None)
        self.assertEqual(len(reversed_drag), 1)
        self.assertIn("rightward drag moved the near side left", reversed_drag[0])
        no_camera = dumps([0, 0, 1, 1, 2, 2])
        del no_camera[3]["camera"]
        self.assertEqual(run.expect_rotate(no_camera, {}, None), ["a step's dump has no camera"])
        self.assertEqual(len(run.expect_rotate(dumps([0, 0, 0, 0, 1, 1]), {}, None)), 1)
        self.assertEqual(len(run.expect_rotate(dumps([0, 0, 1, 2, 3, 3]), {}, None)), 1)
        self.assertTrue(run.expect_rotate(dumps([0, 0, 1, 1, 2, 2])[:5], {}, None))
        self.assertTrue((SHELL / "d02_scripts" / "rotate.json").is_file())


class D02Zoom(unittest.TestCase):
    """The wheel zoom the owner asked for on 26 November 2026 ("我现在没法zoom in"): with --d02 the wheel zooms the view
    under the pointer, wheel up in, within 0.5 to 4 times the opening zoom, and it takes no focus."""

    def test_wheel_up_zooms_in_within_the_limits(self):
        zoom = text(PROJECT / "D02" / "ShellZoom.cs")
        self.assertIn("public const double Step = 1.1;", zoom)
        self.assertIn("public const double MinFactor = .5;", zoom)
        self.assertIn("public const double MaxFactor = 4;", zoom)
        # The direction: wheel up multiplies the zoom by Step (> 1), and the core draws a larger picture for a larger
        # zoom (clip.xy = p.xy * zoom).
        self.assertIn("Wheel(view, button.ButtonIndex == MouseButton.WheelUp ? notches : -notches);", zoom)
        self.assertIn("Math.Clamp(before * Math.Pow(Step, notches), MinFactor, MaxFactor)", zoom)
        self.assertIn("result.clip = float4(p.x*zoom/aspect, p.y*zoom,", text(ROOT / "renderer" / "core" / "shaders" / "geometry.hlsl"))
        # The wheel never reaches the focus hand-back: no focus, no foreground, no window call.
        for word in ("GrabFocus", "GaveBack", "Foreground", "D02Window", "DisplayServer"):
            self.assertNotIn(word, zoom)

    def test_the_shell_zooms_the_view_under_the_pointer(self):
        shell = text(PROJECT / "Shell.cs")
        self.assertIn("if (D02.ShellJumble.On && D02.ShellJumble.Zoom.Follow(@event, ViewAt))", shell)
        # Global in its opening; Local (B18c) in its drawn pane, through the pane layout's hit.
        self.assertIn("private ulong? ViewAt(Vector2 point)\n", shell)
        self.assertIn('ViewPanes.Hit(layout, (int)point.X, (int)point.Y) is { View: "Local" }', shell)
        self.assertIn("D02.ShellShape.ApplyZoom(library, _host, view, D02.ShellJumble.Zoom.Factor(view))", shell)
        shape = text(PROJECT / "D02" / "ShellShape.cs")
        self.assertIn("shape[3] = (float)(opening[3] * factor);", shape)
        # The opening shape goes through Set, so the zoom scales the view's own opening zoom.
        self.assertIn("ShellJumble.On ? Set(library, host, HostLibrary.View, Read(OpeningPath())) : HostLibrary.Ok;", shape)

    def test_the_zoom_expectation_pins_the_direction(self):
        run = shell_module("d02_run")
        factors = [1, 1, 1.1 ** 3, 1.1 ** 3, 1.1 ** -2, 1.1 ** -2, 4.0, 4.0]
        notches = [0, 0, 3, 3, 8, 8, 25, 25]

        def dumps(direction=1, factors=factors):
            return [{"do": do, "camera_turns": 0, "lines": [],
                     "zoom": {"notches": n, "factor": f ** direction, "sent": 1.15 * f ** direction}}
                    for do, f, n in zip(run.ZOOM_STEPS, factors, notches)]
        self.assertEqual(run.expect_zoom(dumps(), {}, None), [])
        # A build whose wheel up zoomed out fails on the direction.
        reversed_wheel = run.expect_zoom(dumps(direction=-1), {}, None)
        self.assertTrue(any("wheel up zoomed out" in problem for problem in reversed_wheel), reversed_wheel)
        # A build without the upper limit fails at the forty notches.
        unclamped = run.expect_zoom(dumps(factors=factors[:6] + [1.1 ** 38, 1.1 ** 38]), {}, None)
        self.assertTrue(any("not 4.0000" in problem for problem in unclamped), unclamped)
        no_sent = dumps()
        del no_sent[2]["zoom"]["sent"]
        self.assertEqual(run.expect_zoom(no_sent, {}, None), ["a step's dump has no zoom sent to the core"])
        turned = dumps()
        turned[4]["camera_turns"] = 1
        self.assertIn("the wheel turned the camera", run.expect_zoom(turned, {}, None))
        self.assertTrue((SHELL / "d02_scripts" / "zoom.json").is_file())



class D02FocusLayer(unittest.TestCase):
    """The Focus layer, the 0.4 Piece Filter in the default view (owner, 26 November 2026: "Focus layer加别名就行"):
    ADR-D01's word on screen, "piece filter" a palette alias, exact counts before apply, and nothing narrowed."""

    def test_the_word_is_focus_layer_and_piece_filter_an_alias(self):
        design = ROOT / "tools" / "looklab" / "data" / "design"
        labels = json.loads(text(design / "command-labels.json"))
        rows = ("filter.open", "filter.preview", "filter.apply", "filter.edit", "filter.preset", "filter.save", "filter.load",
                "structure.filter.compose")
        for row in rows:
            self.assertIn("focus layer", labels["labels"][row].lower())
            self.assertNotIn("filter", labels["labels"][row].lower())
            self.assertIn("piece filter", labels["aliases"][row])
        commands = text(ROOT / "tools" / "looklab" / "app" / "Chrome" / "Model" / "Commands.cs")
        self.assertIn("|| (r.Shipped?.Contains(t, StringComparison.OrdinalIgnoreCase) ?? false));", commands)
        ui = text(ROOT / "tools" / "looklab" / "app" / "Chrome" / "Ui" / "PieceFilterUi.cs")
        self.assertIn('public const string Words = "Focus layer =";', ui)
        host = text(ROOT / "tools" / "looklab" / "app" / "Chrome" / "Ui" / "JumbleHost.cs")
        self.assertIn('global.Tools.AddChild(new FocusLayerEntry(OpenFilter));', host)

    def test_the_filter_never_narrows_the_model(self):
        model = text(ROOT / "tools" / "looklab" / "app" / "Chrome" / "Model" / "PieceFilter.cs")
        # It reads pose rows and makes a focus layer; it names no totals, protection, collateral, key or label.
        for word in ("Totals", "Protection", "Collateral", "Keymap", "Bind(", "Relabel", "Readout"):
            self.assertNotIn(word, model)
        self.assertIn("public PredicateLayer? Layer()", model)
        self.assertIn('if (asked == null) return "a filter applies only on the solver' + chr(39) + 's key or click";', model)

    def test_the_focus_layer_expectation_counts_from_the_rows(self):
        run = shell_module("d02_run")
        self.assertIn("focus-layer", run.ON_FIXTURE)
        off, total = run.pose_count("wj-ia-300", lambda d, h, lattice: d != lattice)
        both, _ = run.pose_count("wj-ia-300", lambda d, h, lattice: d != lattice and h >= 4)
        self.assertEqual((off, total, both), (120343, 177120, 3128))

        def dumps(first_line=f"{off:,} of {total:,} pieces · replace · live query · rev 300", final=f"= not lattice and height >= 4 · {both:,}"):
            def layer(line="", keys=()):
                return {"entry": "Focus layer =", "dock": "focus_layer.editor", "line": line, "expression": "", "keys": list(keys)}
            palette = [["overlay:CommandPalette", label] for label in ("Open focus layer", "Preview focus layer counts", "Apply focus layer")]
            return [{"do": "start", "lines": []},
                    {"do": "wait 801 ms", "lines": [], "focus_layer": layer(first_line)},
                    {"do": "wait 802 ms", "lines": [], "focus_layer": layer(keys=[f"= not lattice · {off:,}"])},
                    {"do": "wait 503 ms", "lines": [], "focus_layer": layer(keys=[f"= not lattice · {off:,}"])},
                    {"do": "wait 804 ms", "lines": [], "focus_layer": layer(f"{both:,} of {total:,} pieces · intersect · live query · rev 300")},
                    {"do": "wait 805 ms", "lines": [], "focus_layer": layer(keys=[final])},
                    {"do": "wait 806 ms", "lines": palette, "focus_layer": layer()}]
        self.assertEqual(run.expect_focus_layer(dumps(), {}, "wj-ia-300"), [])
        self.assertEqual(len(run.expect_focus_layer(dumps(first_line="120,000 of 177,120 pieces"), {}, "wj-ia-300")), 1)
        self.assertEqual(len(run.expect_focus_layer(dumps(final="= height >= 4 · 3,128"), {}, "wj-ia-300")), 1)
        self.assertTrue((SHELL / "d02_scripts" / "focus-layer.json").is_file())


class D02Words(unittest.TestCase):
    """B25: the seed7001 script's expectation follows the word files and the sample totals, part by part."""

    def test_the_seed7001_expectation_follows_the_word_files(self):
        run = shell_module("d02_run")
        words = json.loads(text(ROOT / "tools" / "looklab" / "data" / "samples" / "jumble" / "seed7001-words.json"))
        parts = words["parts"]
        self.assertEqual(len(parts), 9)
        self.assertEqual(parts[-1]["output_digest"], words["solved_digest"])
        done = [1, 2, 2, 3, 4, 5, 6, 7, 8, 9]
        counters = ["part 1 of 9", "part 2 of 9", "part 3 of 9, twist 0 of 1", "part 3 of 9"] + [f"part {k} of 9" for k in range(4, 10)]

        def dumps(crossing_early=False, digest_shift=0):
            out = [{"do": "start", "lines": []}]
            for i, (k, c) in enumerate(zip(done, counters)):
                n = json.loads(text(ROOT / "tools" / "looklab" / "data" / "samples" / "jumble" / parts[k - 1]["after"] / "totals.json"))["N"]
                solved = i == len(done) - 1
                out.append({"do": "wait 300 ms", "lines": [], "word": {
                    "length": "9 twists · alphabet I_a", "counter": c, "state": f"N {n:,} · state x",
                    "signature": "35 slots undefined (the word leaves the lattice) · 9 twists · alphabet I_a",
                    "log": "Log tab: N = 0 · no residual ticks" if solved else f"Log tab: undefined band · N {n:,}",
                    "crossing": "On the lattice, ν = 0: labels pass to 600-cell-Full (reachable)" if solved or (crossing_early and i == 3) else "",
                    "digest": parts[min(8, k - 1 + digest_shift)]["output_digest"]}})
            return out
        self.assertEqual(run.expect_seed7001(dumps(), {}, None), [])
        self.assertTrue(any("before solved" in p for p in run.expect_seed7001(dumps(crossing_early=True), {}, None)))
        self.assertTrue(any("digest" in p for p in run.expect_seed7001(dumps(digest_shift=1), {}, None)))
        self.assertTrue((SHELL / "d02_scripts" / "seed7001.json").is_file())


class D02Nu(unittest.TestCase):
    """B26: the nu-band script runs on nu-c0's fixture in sample mode, and its expectation follows the catalogue."""

    def test_the_nu_band_expectation_follows_the_catalogue(self):
        run = shell_module("d02_run")
        self.assertIn("nu-band", run.ON_FIXTURE)
        self.assertIn("nu-band", run.IN_SAMPLE_MODE)
        want = run.nu_words("nu-c0")
        self.assertEqual(want["band"], "ν band: b8 b25 · O17 dial 0 of 5 · O28 dial 0 of 5 · 9 fixed dots")
        good = {"band": want["band"], "band_visible": True, "band_width": 640.0, "cells": list(range(35)), "rows": "36",
                "selected": "c35 · card5:MB2-5", "set": "0:1 35:1", "preview": want["preview"], "check": want["check"]}
        self.assertEqual(run.expect_nu_band([{"do": "wait 1000 ms", "nu": good}], {}, "nu-c0"), [])
        self.assertTrue(run.expect_nu_band([{"do": "wait 1000 ms", "nu": dict(good, set="0:1")}], {}, "nu-c0"))
        self.assertTrue(run.expect_nu_band([{"do": "wait 1000 ms", "nu": dict(good, band_visible=False)}], {}, "nu-c0"))
        self.assertTrue(run.expect_nu_band([{"do": "wait 1000 ms", "nu": good}], {}, "on-zero"))
        self.assertTrue(want["check"].startswith("ν after this set: b25 · c00×+1 c35×+1, "))
        self.assertTrue((SHELL / "d02_scripts" / "nu-band.json").is_file())

    def test_the_correction_states_are_on_zero_plus_their_letters(self):
        fixtures = shell_module("build_d02_fixtures")
        words = fixtures.words()
        catalogue = json.loads(text(ROOT / "tools" / "looklab" / "data" / "samples" / "jumble" / "corrections.json"))
        letters = {row["id"]: row["letters"] for row in catalogue["catalogue"]["corrections"]}
        for state, cid in fixtures.CORRECTION_STATES.items():
            totals = json.loads(text(ROOT / "tools" / "looklab" / "data" / "samples" / "jumble" / state / "totals.json"))
            self.assertEqual(words[state], letters[cid], state)
            self.assertEqual(totals["correction"]["letters"], letters[cid], state)
        self.assertEqual(fixtures.NOT_BUILT, {})


class D02Fit(unittest.TestCase):
    """The frame fits the window (owner, 26 November 2026: "看不到"): the fit scripts and their expectation."""

    def frame(self, **change):
        frame = {"window": [2000, 1200], "top_bar": [0, 0, 2000, 39], "left_dock": [0, 39, 324, 1161],
                 "right_dock": [1676, 39, 324, 1161], "bottom_dock": [324, 984, 1352, 216],
                 "global": [324, 231, 672, 673], "local": [1211, 231, 465, 673], "min": {}}
        frame.update(change)
        return [{"do": "wait 1000 ms", "frame": frame}]

    def test_the_fit_expectation_finds_what_lies_outside(self):
        run = shell_module("d02_run")
        self.assertEqual(run.expect_fit(self.frame(), {}, "wj-ia-300"), [])
        self.assertTrue(any("local lies outside" in p for p in run.expect_fit(self.frame(local=[1850, 231, 465, 673]), {}, None)))
        self.assertTrue(any("under ADR-D01's 0.2" in p for p in run.expect_fit(self.frame(local=[1211, 231, 100, 673]), {}, None)))
        self.assertTrue(any("not on screen" in p for p in run.expect_fit([{"do": "x", "frame": {"window": [2000, 1200]}}], {}, None)))
        low = self.frame(bottom_dock=[324, 984, 1352, 400])
        self.assertTrue(run.expect_fit(low, {}, None))
        self.assertEqual(run.EXPECT["fit-1280"](low, {}, None), [], "the small window checks the width only")
        for name, size in (("fit-2000", "2000x1200"), ("fit-1280", "1280x720")):
            script = json.loads(text(SHELL / "d02_scripts" / f"{name}.json"))
            self.assertEqual(script["resolution"], size)
            self.assertIn(name, run.ON_FIXTURE)

    def test_the_frame_folds_and_scrolls_instead_of_widening(self):
        frame = text(ROOT / "tools" / "looklab" / "app" / "Chrome" / "Ui" / "ChromeFrame.cs")
        self.assertIn("public HFlowContainer Tools", frame)
        self.assertIn("public VBoxContainer Readouts", frame)
        self.assertIn("TopBarClip", frame)
        self.assertEqual(frame.count('Name = "DockScroll"'), 2)
        self.assertIn("public const float RightDockBelow = 1600, LeftDockBelow = 1440;", frame)
        readouts = text(ROOT / "tools" / "looklab" / "app" / "Chrome" / "Ui" / "JumbleReadouts.cs")
        self.assertIn("public sealed partial class ReadoutRow : HFlowContainer", readouts)

if __name__ == "__main__":
    unittest.main()
