"""SA2 source/fixture acceptance. Never starts Godot, .NET, or a GPU process."""

import copy
import ctypes
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET
from pathlib import Path
from unittest.mock import patch

sys.dont_write_bytecode = True
from sa2_reference import crc16, decode_image, decode_viewport, encode, expected_image
from run_smoke import DRAIN_LINE, INJECT, commands, judge, matrix, run_environment
from smoke_summary import GODOT_VERSION, build_summary, driver_version, scan_public, write_summary
import blank_control
import prepare_l2

HERE = Path(__file__).resolve().parent
PROJECT = HERE / "project"
REQUIRED = ("project.godot", "SA2Smoke.csproj", "nuget.config", "Main.tscn",
            "Smoke.cs", "Native.cs", "CodeLayout.cs", "Arguments.cs", "RunResult.cs", ".gitignore",
            "Level2.cs", "Level2Arguments.cs", "Level2Result.cs", "Level2Windows.cs")
EXPORT_BINDING = re.compile(r'\b(sa2_\w+) = \(delegate\* unmanaged\[Cdecl\]<([^>]+)>\)Export\("(\w+)"\);')


def require(condition, message):
    if not condition:
        raise AssertionError(message)


def rejects(check, *args):
    """True if a check refuses a planted defect."""
    try:
        check(*args)
    except AssertionError:
        return True
    return False


def between(text, start, end):
    require(start in text and end in text.split(start, 1)[1], f"missing source section {start}")
    return text.split(start, 1)[1].split(end, 1)[0]


def check_settings():
    for name in REQUIRED:
        require((PROJECT / name).is_file(), f"missing project/{name}")
    for name in ("run_smoke.py", "smoke_summary.py", "sa2_reference.py", "check_project.py", "README.md"):
        require((HERE / name).is_file(), f"missing {name}")
    settings = {}
    section = ""
    for line in (PROJECT / "project.godot").read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line.startswith("["):
            section = line[1:-1]
        elif "=" in line and section:
            key, value = line.split("=", 1)
            settings[f"{section}/{key}"] = json.loads(value)
    expected = {
        "application/config/name": "SA2 Smoke", "application/run/main_scene": "res://Main.tscn",
        "display/window/size/viewport_width": 1280, "display/window/size/viewport_height": 720,
        "display/window/size/resizable": True, "display/window/vsync/vsync_mode": 0,
        "dotnet/project/assembly_name": "SA2Smoke", "rendering/rendering_device/driver.windows": "d3d12",
        "rendering/rendering_device/fallback_to_vulkan": False, "rendering/rendering_device/fallback_to_opengl3": False,
        "rendering/renderer/rendering_method": "forward_plus", "debug/file_logging/enable_file_logging.pc": False,
        "rendering/shader_compiler/shader_cache/enabled": False, "rendering/rendering_device/pipeline_cache/enable": False,
    }
    for key, value in expected.items():
        require(settings.get(key) == value, f"project setting {key}")
    project = ET.parse(PROJECT / "SA2Smoke.csproj").getroot()
    require(project.attrib.get("Sdk") == "Godot.NET.Sdk/4.7.2", "Godot SDK version")
    for key, value in {"TargetFramework": "net8.0", "EnableDynamicLoading": "true",
                       "AllowUnsafeBlocks": "true", "NuGetAudit": "false"}.items():
        require(project.findtext(f".//{key}") == value, f"csproj {key}")
    require(not any(node.tag.rsplit("}", 1)[-1] == "PackageReference" for node in project.iter()), "PackageReference forbidden")
    nuget = ET.parse(PROJECT / "nuget.config").getroot()
    sources = list(nuget.find("packageSources"))
    require(len(sources) == 2 and sources[0].tag == "clear" and sources[1].tag == "add", "offline NuGet sources")
    require(sources[1].attrib == {"key": "GodotLocal", "value": "%M600_GODOT_NUPKGS%"}, "offline package location")
    require("http" not in (PROJECT / "nuget.config").read_text(encoding="utf-8").lower(), "network package source")
    scene = (PROJECT / "Main.tscn").read_text(encoding="utf-8")
    require('path="res://Smoke.cs"' in scene and "uid=" not in scene, "script path without UID")
    require('[node name="Smoke" type="Control"]' in scene, "root Control")
    require(scene.count('type="TextureRect"') == 1, "one TextureRect")
    for setting in ("texture_filter = 1", "stretch_mode = 2", "expand_mode = 1", "offset_left = 0.0", "offset_top = 0.0"):
        require(setting in scene, f"scene {setting}")
    require(scene.count("anchor_right = 1.0") == 2 and scene.count("anchor_bottom = 1.0") == 2, "full rect anchors")
    ignored = (PROJECT / ".gitignore").read_text(encoding="utf-8").splitlines()
    for entry in (".godot/", "bin/", "obj/", ".sandbox-build/"):
        require(entry in ignored, f"ignore {entry}")


def check_abi(native=None):
    header = (HERE / "native/include/sa2_interop.h").read_text(encoding="utf-8")
    native = (PROJECT / "Native.cs").read_text(encoding="utf-8") if native is None else native
    code = (PROJECT / "CodeLayout.cs").read_text(encoding="utf-8")
    h_constants = {key: int(value, 0) for key, value in re.findall(r"^#define\s+(SA2_\w+)\s+(0x[0-9A-Fa-f]+|\d+)u?\b", header, re.M)}
    c_constants = {key: int(value, 0) for key, value in re.findall(r"const (?:int|uint) (SA2_\w+) = (0x[0-9A-Fa-f]+|\d+);", native + code)}
    require(h_constants == c_constants, "C#/header constant mismatch")
    require(h_constants["SA2_ABI_VERSION"] == 2, "ABI 2 pin")
    types = {"uint32_t": "uint", "int32_t": "int", "uint64_t": "ulong", "char*": "byte*",
             "sa2_context*": "nint", "sa2_context**": "nint*", "sa2_device_info*": "Sa2DeviceInfo*",
             "sa2_config*": "Sa2Config*", "sa2_debug_counts*": "Sa2DebugCounts*", "uint64_t*": "ulong*",
             "uint32_t*": "uint*", "int32_t*": "int*", "sa2_scene_config*": "Sa2SceneConfig*"}
    declared = {}
    for return_type, name, params in re.findall(r"SA2_API\s+(uint32_t|int32_t)\s+(sa2_\w+)\((.*?)\);", header, re.S):
        parameters = [] if params.strip() == "void" else params.split(",")
        converted = []
        for param in parameters:
            c_type = re.sub(r"\b\w+$", "", param.strip()).replace("const", "")
            converted.append(types[re.sub(r"\s+", "", c_type)])
        declared[name] = converted + [types[return_type]]
    bound = {name: [part.strip() for part in parameters.split(",")]
             for parameters, name in re.findall(r"public readonly delegate\* unmanaged\[Cdecl\]<([^>]+)> (sa2_\w+);", native)}
    require(declared == bound, "every export must bind the ABI parameter count/types and return type")
    require(len(bound) == 27, "all 27 ABI 2 exports")
    check_export_bindings(native, bound)
    # Planted defect: two same-signature pointers bound to each other's export.
    pair = ("sa2_signal_godot_free", "sa2_godot_wait_ready")
    swapped = re.sub(r'Export\("(%s|%s)"\)' % pair,
                     lambda m: 'Export("%s")' % pair[1 - pair.index(m.group(1))], native)
    require(bound[pair[0]] == bound[pair[1]] and swapped != native, "planted export swap")
    require(rejects(check_export_bindings, swapped, bound), "swapped export bindings must be refused")
    require("NativeLibrary.Load(absolutePath)" in native and "NativeLibrary.GetExport(_library, name)" in native, "absolute DLL loader")
    for name, size in (("DeviceInfoSize", 48), ("ConfigSize", 28), ("DebugCountsSize", 128), ("SceneConfigSize", 32)):
        require(f"const int {name} = {size};" in native, f"struct size {name}")
    for name, size_name in (("Sa2DeviceInfo", "DeviceInfoSize"), ("Sa2Config", "ConfigSize"), ("Sa2DebugCounts", "DebugCountsSize"),
                            ("Sa2SceneConfig", "SceneConfigSize")):
        require(f"sizeof({name}) != {size_name}" in native, f"runtime struct guard {name}")
    require("fixed int Ids[16]" in native and native.count("StructLayout(LayoutKind.Sequential)") == 4, "native struct layout")
    require("sa2_abi_version() != SA2_ABI_VERSION" in native, "ABI version guard")

    # Derive layouts from the immutable C declarations and compare the C# fields.
    scalar_c = {"uint32_t": ctypes.c_uint32, "int32_t": ctypes.c_int32, "uint64_t": ctypes.c_uint64,
                "double": ctypes.c_double, "char*": ctypes.c_void_p}
    scalar_cs = {"uint32_t": "uint", "int32_t": "int", "uint64_t": "ulong", "double": "double", "char*": "byte*"}
    for c_name, cs_name, expected_size in (("sa2_device_info", "Sa2DeviceInfo", 48),
                                         ("sa2_config", "Sa2Config", 28), ("sa2_debug_counts", "Sa2DebugCounts", 128),
                                         ("sa2_scene_config", "Sa2SceneConfig", 32)):
        c_body = re.search(r"typedef struct " + c_name + r"\s*\{(.*?)\}", header, re.S).group(1)
        c_body = re.sub(r"/\*.*?\*/", "", c_body, flags=re.S)
        c_body = re.sub(r"\bconst\s+", "", c_body)
        c_fields = re.findall(r"(uint32_t|int32_t|uint64_t|double|char\*)\s+(\w+)(?:\[(\d+)\])?\s*;", c_body)
        struct = type(cs_name, (ctypes.Structure,), {"_fields_": [(n, scalar_c[t] * int(length) if length else scalar_c[t]) for t, n, length in c_fields]})
        require(ctypes.sizeof(struct) == expected_size, f"header layout {c_name}")
        cs_body = re.search(r"struct " + cs_name + r"\s*\{(.*?)\}", native, re.S).group(1)
        actual_fields = []
        for fixed, field_type, names in re.findall(r"public\s+(fixed\s+)?(uint|int|ulong|double|byte\*)\s+([^;]+);", cs_body):
            for name in names.split(","):
                actual_fields.append((field_type, re.sub(r"\s+", "", name)))
        expected_fields = [(scalar_cs[t], "Ids[16]" if n == "ids" else n) for t, n, _ in c_fields]
        require(actual_fields == expected_fields, f"C# fields/order {cs_name}")
    layout = json.loads((HERE / "code_layout.json").read_text(encoding="utf-8"))
    for key, suffix in (("block_px", "CODE_BLOCK_PX"), ("grid_cols", "CODE_GRID_COLS"), ("grid_rows", "CODE_GRID_ROWS"),
                        ("min_texture_px", "MIN_TEXTURE_PX"), ("ring_slots", "RING_SLOTS")):
        require(c_constants["SA2_" + suffix] == layout[key], f"code layout {key}")
    for field in layout["fields"]:
        require(c_constants["SA2_CODE_" + field["name"].upper() + "_BITS"] == field["bits"], "code field bits")
    require("0x1021" in code and "0xFFFF" in code and "bytes[..6]" in code, "C# CRC parameters")


def check_abi_defects():
    source = (PROJECT / "Native.cs").read_text(encoding="utf-8")
    for original, defect in (("SA2_ABI_VERSION = 2", "SA2_ABI_VERSION = 1"),
                             ("SceneConfigSize = 32", "SceneConfigSize = 28"),
                             ("sizeof(Sa2SceneConfig) != SceneConfigSize", "false"),
                             ("public double turn_ms;", "public ulong turn_ms;"),
                             ('Export("sa2_scene_trace_begin")', 'Export("sa2_scene_trace_end")')):
        require(original in source and rejects(check_abi, source.replace(original, defect, 1)), f"ABI 2 planted defect {original}")


def check_export_bindings(native, bound):
    """Each delegate field is assigned once, from the export of its own name, with its own type."""
    assigned = {}
    for field, signature, export in EXPORT_BINDING.findall(native):
        require(field == export, f"{field} is bound to export {export}")
        require(field not in assigned, f"{field} is bound twice")
        assigned[field] = [part.strip() for part in signature.split(",")]
    require(assigned == bound, "every delegate field is bound from its export with its declared type")
    require(native.count('Export("') == len(bound), "no export lookup outside the delegate bindings")


def check_reference():
    layout = json.loads((HERE / "code_layout.json").read_text(encoding="utf-8"))
    for vector in layout["test_vectors"]:
        if "input" in vector:
            require(crc16(vector["input"].encode("ascii")) == int(vector["crc16"], 16), "CRC test vector")
        else:
            require(encode(vector["sequence"], vector["slot"], vector["generation"]) == int(vector["code"], 16), "code test vector")
    for width, height in ((128, 128), (160, 144)):
        for vector in layout["test_vectors"][1:]:
            sequence, slot, generation = (vector[k] for k in ("sequence", "slot", "generation"))
            image = expected_image(width, height, sequence, slot, generation)
            require(decode_image(image, width, height) == int(vector["code"], 16), "two-corner image round trip")
            bad = bytearray(image)
            bad[(4 * width + 4) * 4] ^= 255
            require(decode_image(bad, width, height) is None, "one flipped block rejected")
            bad[((height - 60) * width + width - 60) * 4] ^= 255
            require(decode_image(bad, width, height) is None, "two equally corrupted corners fail CRC")
            viewport_width, viewport_height = width + 40, height + 32
            for x, y in ((0, 0), (13, 11)):
                viewport = bytearray(viewport_width * viewport_height * 4)
                for row in range(height):
                    start = ((y + row) * viewport_width + x) * 4
                    viewport[start:start + width * 4] = image[row * width * 4:(row + 1) * width * 4]
                require(decode_viewport(viewport, viewport_width, viewport_height, width, height, x, y) == int(vector["code"], 16), "larger viewport readback")
            require(decode_image(image[:-4], width, height) is None, "truncated readback rejected")
    require(encode(2 ** 32 + 1, 16, 4096) == encode(1, 0, 0), "field wraparound")


def check_harness_source(source=None):
    source = (PROJECT / "Smoke.cs").read_text(encoding="utf-8") if source is None else source
    for token in ("RenderingServer.CallOnRenderThread", "RenderingServer.FramePreDraw", "TextureGetDataAsync",
                  "TextureGetData(_flushTexture", "TextureCreateFromExtension", "sa2_remove_device", "resource-changed",
                  "texture2drd-refused", "not-drawn", "ObserveViewport", "FreeRingRids", "sa2_debug_messages"):
        require(token in source, f"harness path {token}")
    # Check the safety-critical release ordering, including the pending result,
    # written to disk before a DLL drain which may terminate the process without unwinding.
    release = between(source, "private unsafe bool ReleaseRing", "private unsafe void ReadDebug")
    order = ("bool firstFlush = Flush()", "FreeRingRids()", "bool disposed = Flush()", 'teardown.phase = "pending"',
             "WriteResult();", "sa2_drain", "sa2_unregister_slot", "sa2_release_texture")
    positions = [release.find(token) for token in order]
    require(-1 not in positions and positions == sorted(positions),
            "ordered flush/free/flush/pending/write/drain/unregister/release")
    require("RunResult.WriteAtomic(_args.out_path" in between(source, "private void WriteResult()", "private void QuitMain"),
            "WriteResult writes the result file")
    for match in re.finditer(r"lock\s*\(_gate\)", source):
        start = match.end()
        while source[start].isspace():
            start += 1
        if source[start] == "{":
            end, depth = start + 1, 1
            while depth:
                depth += (source[end] == "{") - (source[end] == "}")
                end += 1
            body = source[start:end]
        else:
            body = source[start:source.index(";", start)]
        require(not any(token in body for token in ("_rd.", "_native.", "RenderingServer.", "GetTree()", "GetWindow()")), "no Godot/native call under shared lock")


def check_pending_write_defects():
    """Planted defects: the pending result must be on disk before the native drain."""
    source = (PROJECT / "Smoke.cs").read_text(encoding="utf-8")
    pending = 'Update(r => r.teardown.phase = "pending"); WriteResult();'
    drain = "int drain = _native.sa2_drain(_context, (uint)_args.timeout_ms);"
    require(source.count(pending) == 1 and source.count(drain) == 1, "pending-write planted defect anchors")
    dropped = source.replace(pending, 'Update(r => r.teardown.phase = "pending");')
    late = dropped.replace(drain, drain + " WriteResult();")
    for planted in (dropped, late):
        require(rejects(check_harness_source, planted), "pending result not written before the drain must be refused")


def check_declared_options():
    """Godot consumes --gpu-validation/--render-thread; the runner declares them to the harness."""
    arguments = (PROJECT / "Arguments.cs").read_text(encoding="utf-8")
    smoke = (PROJECT / "Smoke.cs").read_text(encoding="utf-8")
    parsed = re.findall(r'case "(--sa2-[a-z-]+)":', arguments)
    require(len(parsed) == len(set(parsed)), "harness arguments parsed once")
    for name in ("--sa2-gpu-validation", "--sa2-render-thread"):
        require(name in parsed and f'!seen.Contains("{name}")' in arguments, f"required argument {name}")
    require(not re.search(r"\brender_thread_separate\b", arguments + smoke), "no unmeasured render-thread field")
    require("render_thread_separate_observed = !RenderingServer.IsOnRenderThread()"
            in between(smoke, "public override void _Ready()", "private void Update("), "render thread measured in _Ready")
    require("_args.gpu_validation && (counts.error != 0 || counts.corruption != 0)" in smoke, "declared validation flag")
    for row in matrix(True)[1:]:
        command = commands(row, "<godot>", "<project>", "<native>", "<result>", "<log>")
        engine, user = command[:command.index("--")], command[command.index("--") + 1:]
        values = dict(zip(user[0::2], user[1::2]))
        require(len(user) % 2 == 0 and sorted(user[0::2]) == sorted(parsed), f"{row['id']} passes each harness argument once")
        require(values["--sa2-gpu-validation"] == ("1" if row["validation"] else "0")
                and ("--gpu-validation" in engine) == row["validation"], f"{row['id']} validation declaration")
        require(values["--sa2-render-thread"] == row["render_thread"] == engine[engine.index("--render-thread") + 1],
                f"{row['id']} render-thread declaration")


def ordered(source, tokens, message):
    start = 0
    for token in tokens:
        found = source.find(token, start)
        require(found >= 0, f"{message}: {token}")
        start = found + len(token)


def l2_sources():
    return {name: (PROJECT / (name + ".cs")).read_text(encoding="utf-8")
            for name in ("Level2", "Level2Arguments", "Level2Result", "Level2Windows")}


def check_l2_source(sources=None):
    sources = l2_sources() if sources is None else sources
    app, args, result, windows = (sources[name] for name in ("Level2", "Level2Arguments", "Level2Result", "Level2Windows"))
    smoke = (PROJECT / "Smoke.cs").read_text(encoding="utf-8")
    ready = between(smoke, "public override void _Ready()", "private void Update(")
    ordered(ready, ("Level2Arguments.Requested(userArgs)", "new Level2()", "AddChild(_level2)", "return;", "Arguments.Parse(userArgs)"), "level 2 dispatch before level 1 parsing")
    require("if (_level2 != null) return;" in smoke, "level 1 processing disabled only in level 2")
    require('StartsWith("--l2-", StringComparison.Ordinal)' in args, "missing l2-mode is a level 2 usage error")

    options = {"--l2-mode", "--l2-scene", "--l2-out", "--l2-run-id", "--l2-dll", "--l2-trace-ms", "--l2-preroll-ms",
               "--l2-turn-ms", "--l2-inject", "--l2-declare", "--l2-gpu-validation", "--l2-conditions",
               "--l2-no-vram", "--l2-debug-half-target"}
    parsed = re.findall(r'case "(--[a-z0-9-]+)":', args)
    require(len(parsed) == len(set(parsed)), "level 2 option parsed once")
    require(set(parsed) | {"--l2-no-vram", "--l2-debug-half-target"} == options | {
        "--sa2-route", "--sa2-queue", "--sa2-handover", "--sa2-barriers", "--sa2-render-thread", "--sa2-warmup",
        "--sa2-timeout-ms", "--sa2-gpu-validation"}, "exact meaningful level 2 options")
    pins = ('mode = Choice(value, "run", "geometry")', 'scene = Choice(value, "w1", "w2", "w3", "w4")',
            'trace_ms = Number(value, 1000, 3600000)', 'preroll_ms = Number(value, 0, 60000)',
            'a.turn_ms <= 0 || a.turn_ms > 10000', '!double.IsFinite(a.turn_ms)',
            'inject = Choice(value, "corrupt-label", "swap-same-colour", "delay-adoption", "stale-binding")',
            'gpu_validation = Choice(value, "0", "1") == "1"', 'conditions = Choice(value, "enforce", "record")',
            'key == "--l2-no-vram"', 'key == "--l2-debug-half-target"',
            'key != "--l2-declare" && !seen.Add(key)', '++i >= values.Length', 'n < minimum || n > maximum',
            'a.mode == null || a.out_path == null || a.run_id == null || a.dll_path == null',
            '(a.mode == "run" && a.scene == null)', r'\A[A-Za-z0-9][A-Za-z0-9._-]{0,127}\z',
            'Path.IsPathFullyQualified(a.dll_path)', 'Path.GetFileName(a.dll_path), "sa2_interop.dll"',
            'seen.Contains("--l2-turn-ms") && a.scene != "w3"', '(a.scene != "w3" && a.scene != "w4")',
            'value[..equal] == "overlays"', 'a.declared.ContainsKey(value[..equal])',
            'declaredValue == "true" ? true : declaredValue == "false" ? false',
            'trace_ms = 192000, preroll_ms = 4000', 'turn_ms = 190',
            'route = "export", queue = "same", handover = "tracked", barriers = "match", render_thread = "safe"',
            'warmup = 3, timeout_ms = 5000', 'timeout_ms = Number(value, 5000, 5000)',
            'a.warmup < 3', 'noWarmup && a.warmup != 0', 'validation options disagree',
            'default: throw new ArgumentException')
    for token in pins:
        require(token in args, f"level 2 parser/default/range {token}")
    reserved = re.findall(r'"([^"]+)"', between(args, "string[] Outputs =", ";"))
    require(reserved == ["harness.json", "harness.json.tmp", "native.json", "trace.jsonl", "geometry.json"], "only app/DLL outputs reserved")
    output = between(args, "public static bool OutputUsable", "// Recover")
    for token in ("Directory.Exists(path)", "Array.TrueForAll(Outputs", "File.Exists(Path.Combine(path, name))", "Directory.Exists(Path.Combine(path, name))"):
        require(token in output, "l2-out refuses only reserved names")
    require("GetFiles" not in output and "Enumerate" not in output, "logs/PresentMon accepted in output directory")

    for field in ("format", "candidate", "mode", "run_id", "scene", "process_id", "exit_code", "reason", "options", "configuration",
                  "files", "dll_identity", "dll_status", "qpc_frequency", "sizes", "scaling", "dpi_awareness", "environment", "window", "debug"):
        require(re.search(r"public (?:string|int|long\?|object|Level2Sizes|Dictionary<string, object>)[^;]*\b" + field + r"\b", result),
                f"harness top-level field {field}")
    shapes = (
        (between(result, "options = new()", "configuration = new()"), "trace_ms preroll_ms turn_ms inject declared gpu_validation conditions no_vram debug_half_target"),
        (between(result, "configuration = new()", 'environment["declared"]'), "framework framework_version rendering_driver window_mode vsync route queue handover barriers render_thread warmup_frames engine_arguments scaling_3d_mode scaling_3d_scale"),
        (between(result, "public Dictionary<string, object> dll_status", "public long? qpc_frequency"), "abi_version last_status last_error"),
        (between(result, "public Dictionary<string, object> files", "public object dll_identity"), "dll godot:exe godot:assembly godot:project.godot godot:Main.tscn godot:Smoke.cs"),
        (between(result, "public Dictionary<string, object> scaling", "public string dpi_awareness"), "content_scale_mode content_scale_factor texture_stretch"),
        (between(result, "public Dictionary<string, object> environment", "public Dictionary<string, object> window"), "power_source power_mode presenting_adapter presentation_interval declared display width height refresh_hz backbuffer power_samples samples mains battery vsync adapter driver msaa warp"),
        (between(result, "public Dictionary<string, object> window", "public Dictionary<string, object> debug"), "topmost display_required foreground_at_trace_start sample_period_ms samples samples_not_visible samples_covered samples_not_foreground visible_throughout foreground_throughout presents"),
        (between(result, "public Dictionary<string, object> debug", "public Level2Result("), "enabled debug_layer counts messages"),
        (between(app, 'debug["counts"]', "byte* messages"), "struct_size distinct_id_count corruption error warning info message mismatching_clear_value mentioning_sa2 ids"),
    )
    for body, keys in shapes:
        for key in keys.split():
            require(f'["{key}"]' in body, f"harness nested key {key}")
    for token in ('format = "magic600-l2-harness-v1", candidate = "sa2"', 'process_id = Environment.ProcessId',
                  'samples == 0 ? "unknown"', 'mains == samples ? "mains"', 'battery == samples ? "battery"',
                  'mains + battery == samples && mains > 0 && battery > 0 ? "changed" : "unknown"',
                  'FileMode.CreateNew', 'File.Move(temporary, destination, false)', 'stream.Flush(true)', 'IncludeFields = true'):
        require(token in result, f"harness writer/power-source {token}")
    sizes = between(result, "public sealed class Level2Sizes", "public sealed class Level2Result")
    for name in ("display", "window", "backbuffer", "displayed", "target", "samples", "samples_changed"):
        require(name in sizes, f"five-size record {name}")
    for token in ("display.Same(window)", "display.Same(backbuffer)", "display.Same(displayed)", "target.Same(other.target)"):
        require(token in sizes, f"physical size comparison {token}")

    for token in ('ContentScaleMode = Window.ContentScaleModeEnum.Disabled', 'ContentScaleFactor = 1.0f',
                  'ContentScaleSize = Vector2I.Zero', 'Scaling3DScale = 1.0f', 'Scaling3DMode = Viewport.Scaling3DModeEnum.Bilinear',
                  'WindowSetCurrentScreen(DisplayServer.GetPrimaryScreen())', 'WindowSetMode(DisplayServer.WindowMode.ExclusiveFullscreen)',
                  'WindowSetFlag(DisplayServer.WindowFlags.AlwaysOnTop, true)', 'WindowGetVsyncMode() == DisplayServer.VSyncMode.Disabled',
                  'Input.MouseMode = Input.MouseModeEnum.Hidden', 'GetGlobalRect().Size', 'ScreenGetWidth()', 'ScreenGetHeight()',
                  '"godot:exe"', 'Environment.ProcessPath', '"godot:assembly"', 'ProjectAssemblyPath(typeof(Smoke).Assembly)',
                  'GetProperty("AssemblyLoadedPath")', 'AssemblyName.GetAssemblyName(path).FullName == assembly.FullName',
                  'throw new InvalidOperationException("assembly-path")',
                  '"project.godot", "Main.tscn", "Smoke.cs"', 'JsonSerializer.Deserialize<JsonElement>',
                  'Level2Windows.DpiAwareness', '_result.dpi_awareness != "per-monitor-v2" && _result.dpi_awareness != "system"',
                  'info.umd_version >> 48', 'info.umd_version >> 32', 'info.umd_version >> 16',
                  'debug_callback = _args.gpu_validation ? 1 : 0', 'wait_timeout_ms = 5000', 'conditions == "enforce"'):
        require(token in app, f"level 2 framework fact/path {token}")
    for token in ('MonitorFromWindow(_window, 1)', 'GetMonitorInfoW(', 'EnumDisplaySettingsW(monitor.device, -1',
                  'GetClientRect(_window', 'EnumDisplayDevicesW(null, i', 'device.name == monitor.device',
                  'Awareness(GetThreadDpiAwarenessContext())', 'AreDpiAwarenessContextsEqual(context, (nint)(-4)) ? "per-monitor-v2"',
                  'AreDpiAwarenessContextsEqual(context, (nint)(-2)) ? "system" : "unknown"',
                  'SetThreadExecutionState(0x80000000u | 1u | 2u)', 'SetThreadExecutionState(0x80000000u)',
                  'SetForegroundWindow(window)', 'GetWindowLongPtrW(_window, -20)', 'PowerRegisterForEffectivePowerModeNotifications(2',
                  'PowerUnregisterFromEffectivePowerModeNotifications', 'GetCommandLineW()', 'CommandLineToArgvW(GetCommandLineW()', 'LocalFree(arguments)',
                  'processArguments[i] != "--"', 'value == "--path" || value == "--log-file"',
                  'value == "--gpu-validation" || value == "--gpu-abort"', 'JsonSerializer.Serialize(kept)'):
        require(token in windows, f"Windows condition/configuration path {token}")
    require(windows.count("SetForegroundWindow(window)") == 1, "one foreground request")
    require(not any(token in app + windows for token in ("AttachThreadInput", "SendInput", "mouse_event", "keybd_event", "OS.GetCmdlineArgs()")), "no foreground workaround or consumed-argument identity")
    # Godot loads the project assembly from memory, so its Location is empty (FRAMEWORK-FACTS G9).
    require("Assembly.Location" not in app, "the project assembly path comes from its load context")
    covered = between(windows, "private bool Covered()", "public bool Sample(")
    require(covered.count("new(r.") == 5 and "GetAncestor(hit, 2) != _window" in covered, "five-point root-window covered test")
    sample = between(windows, "public bool Sample(", "private static string PowerModeName(")
    for token in ("IsWindowVisible(_window)", "IsIconic(_window)", "DwmGetWindowAttribute(_window, 14", "GetSystemPowerStatus(out PowerStatus power)",
                  'power.ac == 1', 'power.ac == 0', 'mode != _firstPowerMode', 'Level2Result.PowerSource(_samples, _mains, _battery)', '_powerChanged ? "changed"'):
        require(token in sample, f"S-B condition sampling {token}")
    for name in ("battery_saver", "better_battery", "balanced", "high_performance", "max_performance", "game_mode", "mixed_reality"):
        require(f'"{name}"' in windows, f"S-B effective power mode {name}")

    settle = between(app, "private void Settle(", "private unsafe void NativeStatus(")
    ordered(settle, ("if (_initializing) return;", "ReadSizes(displayed)", "current.WindowSettled()", "if (_settled < 2)", "_initializing = true", "Initialize("), "ring only after final fullscreen extent")
    require("_windows.Frequency * 5" in settle, "bounded fullscreen settle")
    initialize = between(app, "private unsafe void Initialize(", "private Rid MakeTexture(")
    ordered(initialize, ("new Native(_args.dll_path)", "_native.sa2_probe", "_native.sa2_attach", "BuildRing()"), "probe/attach/ring order")
    require('_args.debug_half_target ? width / 2 : width' in initialize and '_args.debug_half_target ? height / 2 : height' in initialize, "half target uses physical floor")
    require(app.count("BuildRing();") == 1 and "_width =" not in between(app, "private unsafe void Frame(", "private unsafe void EndTrace()"), "ring never rebuilt during trace")
    require('_native.sa2_register_slot' in between(app, "private unsafe void BuildRing()", "private void CopyDisplay(") and
            '_native.sa2_scene_load' in between(app, "private unsafe void LoadScene()", "private unsafe void Frame("), "register before load")
    frame = between(app, "private unsafe void Frame(", "private unsafe void EndTrace()")
    ordered(frame, ("MarkPrevious();", "if (!_sceneLoaded)", "CheckWrap(wrapped)", "Flush();", "_native.sa2_drain", "LoadScene();",
                    'if (_args.mode == "geometry")', "_native.sa2_scene_geometry_check",
                    "now - _traceStart >=", "EndTrace(); _writeRun = true;", "_args.preroll_ms", "_windows.Frequency * 5",
                    'Fail(3, "foreground")', "ReadSizes(displayed)", "_native.sa2_scene_trace_begin", "_windows.Sample(_result)",
                    'Fail(3, "conditions")', "_native.sa2_signal_godot_free(_context, frame - 1)",
                    "_native.sa2_scene_produce(_context, (uint)(frame % 3), frame)", "_native.sa2_godot_wait_ready(_context, frame)", "_pendingShown = frame"), "scene clock and per-frame order")
    require(app.count("_native.sa2_scene_produce(") == 1 and "_native.sa2_produce(" not in app, "only DLL scenes, one produce path")
    require("if ((Aborted || _ending) && !finishing) return;" in app and frame.count("_ending = true;") == 2,
            "queued callbacks stop after geometry output or trace end")
    for token in ('_traceBegun && now >= _nextSample', '_nextSample = now + _windows.Frequency / 10', '_result.sizes.samples++',
                  '_result.sizes.samples_changed++', 'if (_traceBegun) _result.window["presents"]', '_pendingShown % 3',
                  'RenderingServer.FramePreDraw += PreDraw', '_preDrawIteration != _iteration - 1', 'Fail(1, "not-drawn")'):
        require(token in app, f"trace frame/sample guard {token}")
    finish = between(app, "private void FinishMain()", "private void Flush()")
    ordered(finish, ("_image.Texture = null", "texture.Dispose()", "_views.Clear()", "Queue(Teardown, finishing: true)"), "clear framework wrappers before teardown")
    teardown = between(app, "private unsafe void Teardown()", "private unsafe void ReadDebug()")
    ordered(teardown, ("MarkPrevious(); EndTrace(); Flush();", "_native.sa2_drain",
                       'if (drain == Native.SA2_E_DEVICE_REMOVED) Fail(1, "sa2_drain");', "_writeRun && !Aborted",
                       "_native.sa2_scene_write_run", "_native.sa2_scene_unload", "_native.sa2_unregister_slot",
                       "_rd.FreeRid(_rids[i])", "Flush();", "_native.sa2_drain", "ReadDebug()", "_native.sa2_release_texture", "_native.sa2_detach", "Complete();"), "scene output and safe release order")
    for token in ('GetTree().Quit(1)', '_exitCode = 2; _reason = reason', 'status == Native.SA2_E_CHECK_FAILED',
                  '_result.exit_code = _exitCode; _result.reason = _reason', '_result.Write(_args.out_path)', 'GetTree().Quit(_exitCode)',
                  'usage.reason = "usage"', 'Level2Arguments.UsageOutput(values)', 'GetTree().AutoAcceptQuit = false', 'window.CloseRequested += CloseRequested'):
        require(token in app, f"usage/exit/output path {token}")


def check_l2_defects():
    sources = l2_sources()
    # Each source check with a level 1 counterpart is exercised on a current
    # candidate and a planted defect, including same-signature scene call swaps.
    defects = (
        ("Level2Arguments", 'trace_ms = Number(value, 1000, 3600000)', 'trace_ms = Number(value, 1, 3600000)'),
        ("Level2Arguments", 'preroll_ms = Number(value, 0, 60000)', 'preroll_ms = Number(value, 0, 60001)'),
        ("Level2Arguments", 'a.turn_ms > 10000', 'a.turn_ms > 10001'),
        ("Level2Arguments", 'a.mode == null ||', ''),
        ("Level2Arguments", 'key != "--l2-declare" && !seen.Add(key)', 'false'),
        ("Level2Arguments", '"geometry.json"', '"godot.log"'),
        ("Level2Arguments", 'value[..equal] == "overlays"', 'false'),
        ("Level2Arguments", 'default: throw new ArgumentException', 'default: break; //'),
        ("Level2Result", '["gpu_validation"]', '["validation"]'),
        ("Level2Result", '["samples_changed"]', '["changed"]'),
        ("Level2Result", 'battery == samples ? "battery"', 'battery == samples ? "mains"'),
        ("Level2Result", 'FileMode.CreateNew', 'FileMode.Create'),
        ("Level2Result", 'File.Move(temporary, destination, false)', 'File.Move(temporary, destination, true)'),
        ("Level2", 'ProjectAssemblyPath(typeof(Smoke).Assembly)', 'Path.GetFullPath(typeof(Smoke).Assembly.Location)'),
        ("Level2", 'AssemblyName.GetAssemblyName(path).FullName == assembly.FullName', 'AssemblyName.GetAssemblyName(path).FullName != null'),
        ("Level2", 'WindowMode.ExclusiveFullscreen', 'WindowMode.Fullscreen'),
        ("Level2", 'ContentScaleFactor = 1.0f', 'ContentScaleFactor = 0.5f'),
        ("Level2", 'WindowGetVsyncMode() == DisplayServer.VSyncMode.Disabled', 'true'),
        ("Level2", 'if (_settled < 2)', 'if (_settled < 0)'),
        ("Level2", '_args.debug_half_target ? width / 2 : width', 'width'),
        ("Level2", '_native.sa2_scene_trace_begin(_context)', '_native.sa2_scene_trace_end(_context)'),
        ("Level2", 'Require("sa2_drain", _native.sa2_drain(_context, 5000));\n            LoadScene();', 'LoadScene();'),
        ("Level2", '_native.sa2_scene_write_run(_context, path)', '_native.sa2_scene_geometry_check(_context, path)'),
        ("Level2", '_writeRun && !Aborted', '_writeRun'),
        ("Level2", 'if (drain == Native.SA2_E_DEVICE_REMOVED) Fail(1, "sa2_drain");', ''),
        ("Level2", 'if ((Aborted || _ending) && !finishing) return;', 'if (Aborted && !finishing) return;'),
        ("Level2", '_nextSample = now + _windows.Frequency / 10', '_nextSample = now + _windows.Frequency'),
        ("Level2", 'Fail(3, "conditions")', 'Fail(1, "conditions")'),
        ("Level2Windows", 'GetSystemPowerStatus(out PowerStatus power)', 'false'),
        ("Level2Windows", 'GetAncestor(hit, 2) != _window', 'false'),
        ("Level2Windows", 'JsonSerializer.Serialize(kept)', 'JsonSerializer.Serialize(processArguments)'),
        ("Level2Windows", '(nint)(-2)) ? "system"', '(nint)(-1)) ? "system"'),
        ("Level2", '&& _result.dpi_awareness != "system"', '&& _result.dpi_awareness != "unknown"'),
    )
    for name, original, defect in defects:
        # samples_changed is a field rather than a dictionary key.
        if original == '["samples_changed"]':
            original, defect = "samples_changed;", "changed;"
        require(original in sources[name], f"level 2 planted defect anchor {original}")
        planted = dict(sources)
        planted[name] = sources[name].replace(original, defect, 1)
        require(rejects(check_l2_source, planted), f"level 2 defect must be caught: {original}")


def check_l2_prepare(temporary):
    build = temporary / "prepared"
    native, project = build / "native", build / "project"
    native.mkdir(parents=True)
    assembly = project / ".godot/mono/temp/bin/Debug/SA2Smoke.dll"
    assembly.parent.mkdir(parents=True)
    executable = temporary / prepare_l2.GODOT_EXE
    for path in (executable, assembly, native / "sa2_interop.dll", native / "draw_vs.dxil", project / "project.godot", project / "Main.tscn"):
        path.write_bytes(b"source/fixture only")
    identity = {"native": str(native), "project": str(project), "source": {"sha256": "a" * 64}, "godot_version": GODOT_VERSION}
    environment = {"NUGET_PACKAGES": str(temporary / "packages"), INJECT: None}
    destination = prepare_l2.write_launch(build, identity, executable, environment)
    launch = json.loads(destination.read_text(encoding="utf-8"))
    require(launch == {"format": "magic600-l2-launch-v1", "candidate": "sa2", "executable": str(executable.resolve()),
                       "dll": str((native / "sa2_interop.dll").resolve()), "working_directory": str(project.resolve()),
                       "arguments": ["--path", str(project.resolve()), "--rendering-driver", "d3d12", "--disable-vsync", "--render-thread", "safe"],
                       "run_arguments": ["--log-file", "{out}\\godot.log"], "validation_arguments": ["--gpu-validation"],
                       "separator": ["--"], "environment": environment, "validation_environment": {}}, "prepared launch.json exact contract")
    require("--gpu-index" not in launch["arguments"] and not destination.with_name("launch.json.tmp").exists(), "high performance default and atomic launch")
    before = destination.read_bytes()
    prepare_l2.write_launch(build, identity, executable, environment)
    require(destination.read_bytes() == before, "unchanged prepared launch reused")
    try:
        prepare_l2.write_launch(build, identity, executable, dict(environment, OTHER="1"))
    except RuntimeError:
        require(destination.read_bytes() == before, "changed launch never overwritten")
    else:
        raise AssertionError("changed launch was overwritten")
    try:
        prepare_l2.launch_value(identity, executable.with_name(prepare_l2.GODOT_CONSOLE), environment)
    except ValueError:
        pass
    else:
        raise AssertionError("console wrapper accepted as presenting executable")
    manifest = {"build": identity, "parts": prepare_l2.part_digests(identity, executable), "environment": environment}
    manifest_path = build / "l2-build-identity.json"
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
    with patch.object(prepare_l2.smoke, "BUILD_ROOT", temporary):
        require(prepare_l2.reuse_prepared(identity["source"], executable)[0] == build.resolve(), "matching prepared digests reused")
        for path in (assembly, native / "sa2_interop.dll", native / "draw_vs.dxil", executable, project / "project.godot"):
            original = path.read_bytes()
            path.write_bytes(original + b"changed")
            try:
                prepare_l2.reuse_prepared(identity["source"], executable)
            except RuntimeError:
                pass
            else:
                raise AssertionError(f"changed prepared part reused: {path.name}")
            finally:
                path.write_bytes(original)
        try:
            prepare_l2.reuse_prepared({"sha256": "b" * 64}, executable)
        except RuntimeError:
            pass
        else:
            raise AssertionError("changed source reused")
    source = (HERE / "prepare_l2.py").read_text(encoding="utf-8")
    for token in ("sys.dont_write_bytecode = True\nimport run_smoke", "smoke.build(", "smoke.offline_environment(", "smoke.BUILD_PATH_LIMIT", "part_digests(identity, executable)"):
        require(token in source, f"offline prepare reuses level 1 recipe {token}")
    require("subprocess" not in source and "smoke.locate_godot(" not in source and "run_process(" not in source, "prepare launches no Godot/editor")


def check_run_environment(build):
    """Only R12 gets the drain injection, even when the base environment inherited it."""
    for base in ({"PATH": "x"}, {"PATH": "x", INJECT: "1"}, {"PATH": "x", INJECT.lower(): "1"}):
        snapshot = dict(base)
        for row in matrix(True):
            env = build(base, row["id"])
            injected = [key for key in env if key.upper() == INJECT]
            if row["id"] == "R12":
                require(injected == [INJECT] and env[INJECT] == "1", "R12 drain injection")
            else:
                require(not injected, f"{row['id']} must not inherit the drain injection")
            require(env.get("PATH") == "x", "base environment kept")
        require(base == snapshot, "base environment unchanged")


def synthetic_config(row):
    return {"route": row["route"], "queue": row["queue"], "handover": row["handover"], "barriers": row["barriers"],
            "frames": row["frames"], "resize_every": row["resize_every"], "gpu_validation": row["validation"],
            "render_thread": row["render_thread"], "render_thread_separate_observed": row["render_thread"] == "separate"}


def synthetic_selftest(*checks):
    return {"format": "magic600-sa2-native-selftest-v1", "mode": "hardware",
            "checks": [{"name": f"check {n}", "status": status, "reason": reason}
                       for n, (status, reason) in enumerate(checks)]}


def synthetic_run(row=None):
    row = dict(matrix()[2], frames=100) if row is None else row
    return {
        "format": "magic600-sa2-smoke-run-v1", "status": "pass", "reasons": [], "config": synthetic_config(row),
        "engine": {"version": GODOT_VERSION, "editor_build": True, "adapter_name": "NVIDIA GeForce RTX 4070 Laptop GPU"},
        "device": {"enhanced_barriers": 1, "debug_layer": 1, "umd_version": (32 << 48) | (0 << 32) | (16 << 16) | 1692,
                   "vendor_id": 123, "device_id": 456},
        "frames": {"run": 100, "produced": 100, "drawn": 100, "not_drawn": 0, "eligible": 100, "verified": 100,
                   "mismatched": 0, "readbacks_requested": 100, "readbacks_completed": 100,
                   "skipped": {"warmup": 3, "transition": 1}},
        "verify": {"native_checks": 2, "rd_checks": 2, "native_mismatched_texels": 0, "rd_mismatched_texels": 0, "mismatched_texels": 0},
        "resize": {"rebuilds": 0, "sizes": [[1280, 720]], "transitions": [], "longest_transition": 0},
        "debug": {"struct_size": 128, "distinct_id_count": 1, "corruption": 0, "error": 0, "warning": 1,
                  "info": 0, "message": 0, "mismatching_clear_value": 1, "mentioning_sa2": 1, "ids": [820]},
        "teardown": {"phase": "complete", "drain_result": 0, "detached": True, "slots_unregistered": 3,
                     "refcount_after": [], "rebuild_drain_results": []}, "sa2_failures": [],
    }


def fixture(row, **result_changes):
    result = synthetic_run(row)
    for path, value in result_changes.items():
        group, key = path.split("__", 1)
        result[group][key] = value
    return {"result": result, "exit_code": 0, "timed_out": False, "output_line_counts": {"error": 0, "warning": 0}}


def check_runner():
    env = os.environ.copy()
    env["PYTHONDONTWRITEBYTECODE"] = "1"
    for loss in (False, True):
        command = [sys.executable, str(HERE / "run_smoke.py"), "--dry-run"] + (["--device-loss"] if loss else [])
        dry = subprocess.run(command, env=env, capture_output=True, text=True, timeout=30)
        require(dry.returncode == 0, f"dry run failed: {dry.stderr}")
        require(re.findall(r"^R\d+:", dry.stdout, re.M) == [f"R{n}:" for n in range(14 if loss else 13)], "dry matrix order")
        require(dry.stdout.count("M600_SA2_INJECT_UNCONFIRMED_DRAIN=1") == 1, "R12 injection is scoped")
        godot_runs = 13 if loss else 12
        require(dry.stdout.count("--sa2-gpu-validation") == dry.stdout.count("--sa2-render-thread") == godot_runs,
                "declared engine options in every Godot command")
    only = subprocess.run([sys.executable, str(HERE / "run_smoke.py"), "--dry-run", "--only", "R5,R2"], env=env, capture_output=True, text=True, timeout=30)
    require(only.returncode == 0 and re.findall(r"^R\d+:", only.stdout, re.M) == ["R2:", "R5:"], "--only preserves matrix order")
    rows = matrix(True)
    require(rows[9]["frames"] == 3000 and not rows[9]["validation"] and rows[10]["render_thread"] == "separate", "R9/R10 configuration")
    require(rows[11]["barriers"] == "legacy" and rows[13]["device_loss_at"] == 200, "probe configuration")
    check_run_environment(run_environment)
    require(rejects(check_run_environment, lambda base, run_id: dict(base, **({INJECT: "1"} if run_id == "R12" else {}))),
            "an inherited drain injection must be refused")
    row = dict(rows[2], frames=100)
    run = fixture(row)
    adapter = run["result"]["engine"]["adapter_name"]
    require(judge(row, run, adapter) == ("as-expected", []) and run["judge_notes"] == [], "clean pass evidence")
    for group, key, value in (("frames", "verified", 99), ("frames", "eligible", 89), ("frames", "drawn", 99),
                              ("frames", "readbacks_completed", 99), ("verify", "mismatched_texels", 1),
                              ("debug", "error", 1), ("teardown", "drain_result", None), ("resize", "rebuilds", 1)):
        bad = copy.deepcopy(run)
        bad["result"][group][key] = value
        require(judge(row, bad, adapter)[0] == "unexpected", f"reject bad {group}.{key}")
    # Godot's own ERROR: lines are recorded with a note; they are not a pass condition.
    noisy = dict(fixture(row), output_line_counts={"error": 1, "warning": 2})
    require(judge(row, noisy, adapter) == ("as-expected", []) and noisy["judge_notes"] == ["godot-output-errors"],
            "Godot output errors are a non-gating note")
    # Validation evidence needs an active debug layer; R9 runs without validation.
    require(judge(row, fixture(row, device__debug_layer=0), adapter) == ("unexpected", ["validation-not-active"]),
            "validation row without the debug layer")
    quiet = dict(rows[9], frames=100)
    require(judge(quiet, fixture(quiet, device__debug_layer=0), adapter) == ("as-expected", []), "validation-off row")
    # The result's config echo must equal the row, value and type.
    for key, wrong in (("route", "import-copy"), ("queue", "own"), ("handover", "render-target"), ("barriers", "legacy"),
                       ("frames", 99), ("resize_every", 0), ("gpu_validation", 1), ("render_thread", "separate"),
                       ("render_thread_separate_observed", True)):
        changed, missing = fixture(row, **{"config__" + key: wrong}), fixture(row)
        del missing["result"]["config"][key]
        for bad in (changed, missing):
            require("config-mismatch" in judge(row, bad, adapter)[1], f"config echo {key}")
    compute = dict(rows[1], frames=100)
    baseline = fixture(compute, verify__native_checks=0)
    del baseline["result"]["config"]["handover"]
    require(judge(compute, baseline, adapter) == ("as-expected", []), "rd-compute has no handover to compare")
    separate = dict(rows[10], frames=100)
    require(judge(separate, fixture(separate), adapter) == ("as-expected", []), "separate render thread observed")
    require(judge(separate, fixture(separate, config__render_thread_separate_observed=False), adapter)
            == ("unexpected", ["config-mismatch"]), "separate render thread not observed")
    resized = dict(row, resize_every=20)
    require(judge(resized, fixture(resized, resize__rebuilds=4), adapter) == ("as-expected", []), "rebuilds = (100 - 1) // 20")
    for rebuilds in (3, 5):
        require(judge(resized, fixture(resized, resize__rebuilds=rebuilds), adapter)[0] == "unexpected", "rebuild count")
    recorded = fixture(rows[7], frames__run=300)
    recorded["result"]["status"] = "recorded"
    require(judge(rows[7], recorded, adapter) == ("as-expected", []), "recorded probe")
    recorded["result"]["frames"]["run"] = 299
    require(judge(rows[7], recorded, adapter)[0] == "unexpected", "recorded probe run length")
    refused = fixture(rows[6], frames__run=0)
    refused["result"].update(status="unsupported", reasons=["texture2drd-refused"])
    require(judge(rows[6], refused, adapter) == ("as-expected", []), "R6 refusal has no run frames")
    for checks, verdict in (([("pass", "ok")], "as-expected"), ([("pass", "ok"), ("unsupported", "no OPTIONS12")], "as-expected"),
                            ([("unsupported", "no OPTIONS12")], "unexpected"), ([], "unexpected"),
                            ([("pass", "ok"), ("unsupported", "")], "unexpected"), ([("pass", "ok"), ("fail", "x")], "unexpected")):
        native = {"result": synthetic_selftest(*checks), "exit_code": 0, "timed_out": False}
        require(judge(rows[0], native, adapter)[0] == verdict, f"R0 self-test checks {checks}")
    pending = fixture(rows[12], frames__run=120, teardown__phase="pending")
    pending["exit_code"] = 3
    require(judge(rows[12], pending, adapter, stderr=DRAIN_LINE) == ("as-expected", []), "R12 pending drain evidence")
    require(judge(rows[12], pending, adapter)[0] == "unexpected", "R12 missing abort line")
    pending["result"]["frames"]["run"] = 119
    require(judge(rows[12], pending, adapter, stderr=DRAIN_LINE)[0] == "unexpected", "R12 run length")
    loss = {"result": None, "exit_code": -1, "timed_out": False}
    require(judge(rows[13], loss, adapter)[0] == "as-expected", "R13 engine abort recorded")
    loss["timed_out"] = True
    require(judge(rows[13], loss, adapter)[0] == "unexpected", "R13 timeout rejected")
    command = commands(rows[10], "<godot>", "<project>", "<native>", "<result>", "<log>")
    require(command[command.index("--render-thread") + 1] == "separate" and "--gpu-validation" in command, "Godot command options")


def check_summary(temporary):
    row = dict(matrix()[2], frames=100)
    private = {"source": {"head": "a" * 40, "sha256": "b" * 64, "matches_head": False, "file_count": 12},
               "build": {"dll_sha256": "c" * 64, "assembly_sha256": "d" * 64, "godot_version": GODOT_VERSION},
               "runs": [{"id": "R2", "row": row, "result": synthetic_run(), "exit_code": 0,
                         "timed_out": False, "judgement": "as-expected", "judge_reasons": [],
                         "judge_notes": ["godot-output-errors", "free text note"],
                         "output_line_counts": {"error": 1, "warning": 2}},
                        {"id": "R0", "row": matrix()[0], "exit_code": 0, "timed_out": False,
                         "result": synthetic_selftest(("pass", "ok"), ("unsupported", "no OPTIONS12")),
                         "judgement": "as-expected", "judge_reasons": [], "judge_notes": []}]}
    # R0's JSON has no overall status: the public status is derived from its checks.
    for checks, status in (([("pass", "ok"), ("unsupported", "no OPTIONS12")], "pass"),
                           ([("unsupported", "no OPTIONS12")], "fail"), ([("pass", "ok"), ("fail", "x")], "fail"),
                           (None, "missing")):
        variant = copy.deepcopy(private)
        variant["runs"][1]["result"] = None if checks is None else synthetic_selftest(*checks)
        require(build_summary(variant, "sa2_fixture_person")["runs"][1]["status"] == status, f"R0 public status {status}")
    projected = build_summary(private, "sa2_fixture_person")["runs"][0]
    require(projected["judge_notes"] == ["godot-output-errors"] and projected["output_line_counts"] == {"error": 1, "warning": 2},
            "known judge notes and Godot output line counts are carried")
    planted_user = "sa2_fixture_person"
    planted_path = "Q:" + "\\" + "Use" + "rs" + "\\" + planted_user + "\\private.txt"
    planted = copy.deepcopy(private)
    result = planted["runs"][0]["result"]
    result["debug"]["messages"] = planted_path
    result["config"] = {"dll_path": planted_path, "user_name": planted_user}
    result["sa2_failures"] = [{"function": "sa2_probe", "last_error": planted_path + planted_user}]
    result["frames"]["first_mismatch"] = {"personal_log": planted_path}
    public = build_summary(planted, planted_user)
    require(planted_user not in json.dumps(public) and planted_path not in json.dumps(public), "free text stripped")
    require("vendor_id" not in json.dumps(public) and "device_id" not in json.dumps(public), "device IDs stripped")
    for poison in (planted_path, "GPU " + planted_user):
        poisoned = copy.deepcopy(private)
        poisoned["runs"][0]["result"]["engine"]["adapter_name"] = poison
        target = temporary / "refused.json"
        try:
            write_summary(poisoned, target, planted_user)
        except ValueError:
            require(not target.exists(), "refused projection left an output")
        else:
            raise AssertionError("private adapter text was not refused")
    clean_path = temporary / "clean-summary.json"
    clean = write_summary(private, clean_path, planted_user)
    require(clean_path.is_file() and clean["format"] == "magic600-sa2-smoke-summary-v1", "clean summary written")
    scan_public(json.loads(clean_path.read_text(encoding="utf-8")), planted_user)
    require(clean["adapter"]["driver_version"] == "32.0.16.1692", "UMD driver version formatting")
    require(driver_version(0) is None, "unknown UMD version")


def check_blank_control():
    """The R10 control keeps R10's engine flags, varies only the render-thread model and publishes counts only."""
    r10 = next(row for row in matrix() if row["id"] == "R10")
    full = commands(r10, "<godot>", "<project>", "<native>", "<result>", "<log>")
    engine = full[:full.index("--")] + ["--quit-after", str(blank_control.FRAMES)]
    require(blank_control.command("<godot>", "<project>", "separate", "<log>") == engine, "control uses R10's engine flags")
    thread = engine.index("--render-thread") + 1
    require(blank_control.command("<godot>", "<project>", "safe", "<log>") == engine[:thread] + ["safe"] + engine[thread + 1:],
            "control varies only the render-thread model")
    require("--gpu-validation" in engine and "--" not in engine, "control has validation and no harness arguments")
    stdout = "Godot Engine v4.7.2\nD3D12 12_0 - Forward+ - Using Device #0: NVIDIA - NVIDIA GeForce RTX 4070 Laptop GPU\n"
    require(blank_control.adapter_name(stdout) == "NVIDIA GeForce RTX 4070 Laptop GPU", "control adapter line")
    require(blank_control.adapter_name(stdout + stdout) is None and blank_control.adapter_name("") is None, "one adapter line")
    stderr = "WARNING: experimental\n" + blank_control.FINALIZE + " \n   at: finalize (servers/rendering/rendering_device.cpp)\n"
    run = {"exit_code": 0, "timed_out": False, "output_line_counts": {"error": 1, "warning": 1}, "stdout": "private"}
    counts = blank_control.run_counts("separate", 1, run, stdout, stderr)
    require(counts == {"mode": "separate", "repeat": 1, "exit_code": 0, "timed_out": False,
                       "adapter_name": "NVIDIA GeForce RTX 4070 Laptop GPU",
                       "output_line_counts": {"error": 1, "warning": 1}, "finalize_lines": 1}, "control run counts")
    safe = blank_control.run_counts("safe", 1, dict(run, output_line_counts={"error": 0, "warning": 0}), stdout, "")
    source = {"head": "a" * 40, "sha256": "b" * 64, "matches_head": True, "file_count": 3, "project": "private"}
    summary = blank_control.build_summary(GODOT_VERSION, source, [safe, counts], "planted-user")
    require(summary["totals"] == {"safe": {"runs": 1, "runs_with_finalize_line": 0, "error_lines": 0},
                                  "separate": {"runs": 1, "runs_with_finalize_line": 1, "error_lines": 1}}, "control totals")
    require(summary["source"] == {"head": "a" * 40, "sha256": "b" * 64, "matches_head": True}, "control source fields")
    scan_public(summary, "planted-user")
    for planted in ("planted-user GPU", "C:" + "\\x"):
        try:
            blank_control.build_summary(GODOT_VERSION, source, [dict(counts, adapter_name=planted)], "planted-user")
        except ValueError:
            continue
        raise AssertionError("private text in a control summary was not refused")


def check_owned_files():
    paths = [HERE / name for name in ("run_smoke.py", "smoke_summary.py", "sa2_reference.py", "check_project.py",
                                      "blank_control.py", "README.md", "prepare_l2.py")]
    paths.extend(p for p in PROJECT.rglob("*") if p.is_file() and not any(part in {".godot", "bin", "obj", ".sandbox-build", "__pycache__"} for part in p.relative_to(PROJECT).parts))
    users = "Use" + "rs"
    for path in paths:
        source = path.read_text(encoding="utf-8")
        require(not re.search(r"[A-Za-z]:[^\r\n]*[\\/]" + users + r"[\\/]", source, re.I), f"private drive path in {path.name}")
        require("/" + users + "/" not in source, f"private home path in {path.name}")
        require("App" + "Data" + "\\" not in source, f"private local data path in {path.name}")
        if path.suffix == ".py":
            require("mk" + "dtemp(" not in source and "Temporary" + "Directory(" not in source, "plain mkdir temp policy")
    for name in (".godot", "bin", "obj", ".sandbox-build"):
        require(not (PROJECT / name).exists(), f"build output left in source: {name}")


def main():
    temporary = Path(tempfile.gettempdir()) / f"m600-sa2-project-check-{os.getpid()}"
    created = False
    try:
        temporary.mkdir()
        created = True
        check_settings()
        check_abi()
        check_abi_defects()
        check_reference()
        check_harness_source()
        check_pending_write_defects()
        check_declared_options()
        check_l2_source()
        check_l2_defects()
        check_l2_prepare(temporary)
        check_runner()
        check_summary(temporary)
        check_blank_control()
        check_owned_files()
        print("SA2 project: PASS (level 1 checks, ABI 2/27 exports/four layouts, level 2 source and planted defects, prepared launch fixture; source/fixtures only)")
        return 0
    except (AssertionError, OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as error:
        print(f"SA2 project: FAIL: {error}", file=sys.stderr)
        return 1
    finally:
        # Delete only this invocation's plain-mkdir fixture directory.
        if created and temporary.exists() and temporary.resolve().parent == Path(tempfile.gettempdir()).resolve() and temporary.name == f"m600-sa2-project-check-{os.getpid()}":
            shutil.rmtree(temporary)


if __name__ == "__main__":
    raise SystemExit(main())
