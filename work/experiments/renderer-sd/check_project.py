"""Packets SD-Q and L2-Q: source/static evidence and Python fixtures only."""
import sys
sys.dont_write_bytecode = True

import copy
import ctypes
import fnmatch
import json
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import tempfile
import uuid
from unittest.mock import patch

import prepare_l2
import run_smoke
import sd_reference as reference
import smoke_summary

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
LAYOUT = ROOT / "work/experiments/renderer-sa2/code_layout.json"
HEADER = ROOT / "work/experiments/renderer-sa2/native/include/sa2_interop.h"


def require(condition, message):
    if not condition:
        raise AssertionError(message)


def abi_checks(header, loader, structs):
    require(re.search(r"^#define SA2_ABI_VERSION 2u$", header, re.M), "ABI 2 pin")
    declarations = re.findall(r"SA2_API\s+\w+\s+(sa2_\w+)\(", header)
    bindings = re.findall(r"SD_BIND\((sa2_\w+)\)", loader)
    fields = re.findall(r"SD_FUNCTION\((sa2_\w+)\)", structs)
    require(len(declarations) == len(set(declarations)) == 27, "27 header exports")
    require(len(bindings) == len(fields) == 27 and set(declarations) == set(bindings) == set(fields), "bind every ABI export exactly by name")
    require("GetProcAddress(module, #name)" in loader and "reinterpret_cast<decltype(&name)>" in loader
            and "decltype(&name) fn_##name" in structs, "ABI-derived function types")
    require("LoadLibraryW(absolutePath)" in loader and "fn_sa2_abi_version() != SA2_ABI_VERSION" in loader, "ABI check before other calls")
    require(loader.index("fn_sa2_abi_version()") < loader.index("SD_BIND(sa2_last_error)"), "ABI call must precede the rest")
    types = {"uint32_t": ctypes.c_uint32, "int32_t": ctypes.c_int32, "uint64_t": ctypes.c_uint64,
             "double": ctypes.c_double, "const char*": ctypes.c_void_p}
    require(ctypes.sizeof(ctypes.c_void_p) == 8, "x64 fixture ABI")
    for name, size in (("sa2_device_info", 48), ("sa2_config", 28), ("sa2_debug_counts", 128), ("sa2_scene_config", 32)):
        declaration = "sizeof(" + ("struct " if name == "sa2_debug_counts" else "") + name + ") == " + str(size)
        require(declaration in structs, "ABI size assertion missing: " + name)
        body = re.search(r"typedef struct " + name + r"\s*\{(.*?)\}\s*" + name + ";", header, re.S).group(1)
        body = re.sub(r"/\*.*?\*/", "", body, flags=re.S)
        fields = []
        for field in body.split(";"):
            if not field.strip():
                continue
            parsed = re.fullmatch(r"\s*(uint32_t|int32_t|uint64_t|double|const char\*)\s+(\w+)(?:\[(\d+)\])?\s*", field)
            require(parsed is not None, "unknown ABI field layout")
            kind, field_name, count = parsed.groups()
            fields.append((field_name, types[kind] * int(count) if count else types[kind]))
        native_struct = type(name, (ctypes.Structure,), {"_fields_": fields})
        require(ctypes.sizeof(native_struct) == size, "header-derived struct size: " + name)


def cpp_body(source, name):
    start = source.index(name + "(")
    start = source.index("{", start)
    depth = 1
    end = start + 1
    while depth:
        require(end < len(source), "unterminated function: " + name)
        depth += (source[end] == "{") - (source[end] == "}")
        end += 1
    return source[start:end]


def l2_static_checks(sources):
    main, smoke, l2 = (sources[name] for name in ("main.cpp", "smoke.cpp", "l2.cpp"))
    parser = cpp_body(l2, "parseL2")
    options = {"mode", "scene", "out", "run-id", "dll", "trace-ms", "preroll-ms", "turn-ms", "inject", "declare",
               "gpu-validation", "conditions", "no-vram", "debug-half-target"}
    require(set(re.findall(r'key == "--l2-([\w-]+)"', parser)) == options, "every level 2 option parsed")
    for token in ('key != "--l2-declare"', '!seen.insert(key).second', 'else usage();', '"run", "geometry"',
                  '"w1", "w2", "w3", "w4"', 'integer(1000, 3600000)', 'integer(0, 60000)',
                  '!std::isfinite(result)', 'result <= 0 || result > 10000', 'integer(0, 1)', '"enforce", "record"',
                  'options.scene != "w3"', 'options.scene != "w4"', 'name == "overlays"', 'options.declared.contains(name)',
                  'declared == "true"', 'declared == "false"', '[A-Za-z0-9][A-Za-z0-9._-]{0,127}',
                  'options.mode.isEmpty()', 'options.mode == "run" && options.scene.isEmpty()', 'options.runId.isEmpty()',
                  'QFileInfo(options.dll).isAbsolute()', 'QFileInfo(options.dll).isFile()', '"sa2_interop.dll"',
                  '"corrupt-label", "swap-same-colour", "delay-adoption", "stale-binding"', 'outputDirectory(options.out, true)'):
        require(token in parser, "L2 parsing rule missing: " + token)
    for name in ("frames", "resize-every", "verify-every", "device-loss-at", "debug-layer", "out", "dll"):
        require('key == "--sd-' + name + '"' not in parser, "meaningless smoke option accepted")
    require('"rhi-upload"' not in parser, "level 2 must draw DLL scenes")
    for token in ('"qt", "from-rhi", "from-device"', '"import-copy", "export-copy", "import-direct"', '"same", "own"',
                  '"tracked", "declared"', '"legacy", "match"', 'integer(5000, 5000)', 'M600_SA2_INJECT_UNCONFIRMED_DRAIN'):
        require(token in parser, "L2 framework options")
    directory = cpp_body(l2, "outputDirectory")
    names = set(re.findall(r'"([\w.]+)"', directory))
    require(names == {"harness.json", "harness.json.tmp", "native.json", "trace.jsonl", "geometry.json"}, "output check must allow logs and CSV")
    require('directory.exists(name)' in directory and '!QFileInfo(path).isDir()' in directory, "output directory checks")
    window = cpp_body(main, "level2Main")
    for token in ('sd::usableL2Out', 'result.failure("usage")', 'QGuiApplication::primaryScreen()', 'window->setScreen(screen)',
                  'Qt::FramelessWindowHint', 'Qt::WindowStaysOnTopHint', 'HWND_TOPMOST', 'Qt::BlankCursor',
                  'window->showFullScreen()', 'surface.setSwapInterval(0)', 'graphics.setDebugLayer(options.gpuValidation)',
                  'debug->EnableDebugLayer()', 'setFixedColorBufferWidth', 'setFixedColorBufferHeight',
                  'QQuickWindow::afterFrameEnd', 'Qt::DirectConnection', 'window.reset()', 'owned.release(harness)', 'result.write()'):
        require(token in window, "L2 window/setup setting: " + token)
    require("SetEnableGPUBasedValidation" not in window, "validation uses the debug layer only")
    require(window.count('SetForegroundWindow(') == 1 and window.index('window->showFullScreen()') < window.index('SetForegroundWindow('), "one foreground request after show")
    require('starts_with("--l2-")' in main and 'return level2Main(argc, argv)' in main, "L2 dispatch before smoke parsing")
    require('framework.route = "import-direct"' in sources["l2.h"] and 'device = "qt"' in sources["smoke.h"], "Q5 defaults")
    for forbidden in ("AttachThreadInput", "SendInput", "QQml", "QQuickView"):
        require(forbidden not in main + l2 + smoke, "unexpected foreground helper or overlay")
    init = cpp_body(smoke, "Harness::initialized")
    require(init.index('fn_sa2_probe(') < init.index('fn_sa2_attach(') < init.index('fn_sa2_identity('), "probe/attach/identity on render thread")
    require('DXGI_GPU_PREFERENCE_HIGH_PERFORMANCE' in init and 'expected.AdapterLuid.LowPart != description.AdapterLuid.LowPart' in init,
            "actual high-performance adapter checked")
    require('rhi_->driverInfo().deviceName).toStdString() != adapterName' in init, "QRhi adapter name agrees with DXGI")
    require('qputenv("QT_D3D_ADAPTER_INDEX"' in l2 and 'selectHighPerformanceAdapter()' in window, "Qt adapter selection")
    require('l2 ? int(l2->options.gpuValidation) : 1' in cpp_body(smoke, "Harness::config"), "scene callback only with validation")
    render = cpp_body(smoke, "Harness::renderScene")
    for token in ('if (ringSize_.isEmpty())', 'l2->finalSize(window)', 'displayed.width() / 2', 'displayed.height() / 2',
                  'l2->options.traceMs', 'l2->options.turnMs', 'SA2_SCENE_NO_VRAM', 'l2->options.mode == "geometry"',
                  'fn_sa2_scene_geometry_check(', 'l2->readyToTrace()', 'l2->sample(window, ringSize_)', 'frame_ - 1',
                  'frame_ % SA2_RING_SLOTS', 'setNativeLayout', 'showSlot(k)', 'l2->firstProduce = L2::qpc()'):
        require(token in render, "scene protocol: " + token)
    ordered = ('createRing(wanted)', 'fn_sa2_scene_load(', 'fn_sa2_scene_trace_begin(', 'fn_sa2_signal_godot_free(',
               'fn_sa2_scene_produce(', 'fn_sa2_godot_wait_ready(')
    positions = [render.index(token) for token in ordered]
    require(positions == sorted(positions) and render.count('createRing(') == render.count('fn_sa2_scene_produce(') == 1,
            "one ring and one produce per render step")
    require('releaseRing(' not in render and 'ringSize_ != size' not in render and 'fn_sa2_mark_shown(' not in render, "no trace rebuild or early mark")
    after = cpp_body(smoke, "Harness::afterFrameEnd")
    require(after.index('fn_sa2_mark_shown(') < after.index('l2->presented(') < after.index('fn_sa2_scene_trace_end('), "mark after submit/present; stop on boundary")
    require('fail("present")' in after and after.index('frameFailed()') < after.index('fn_sa2_mark_shown('), "no mark or trace end after a failed endFrame (Q9)")
    require(window.index('sd::watchFrameFailures()') < window.index('QGuiApplication app(argc, argv)'), "failed endFrame watched before Qt starts")
    watch = cpp_body(l2, "watchFrameFailures")
    require('previousHandler_ = qInstallMessageHandler(frameFailureHandler)' in watch, "message handler installed")
    require('previousFilter_ = QLoggingCategory::installFilter(keepDefaultWarnings)' in watch, "default warnings kept on (Q9)")
    keep = cpp_body(l2, "keepDefaultWarnings")
    require('qstrcmp(category->categoryName(), "default") == 0' in keep and 'defaultCategory(' not in keep
            and 'setEnabled(QtWarningMsg, true)' in keep
            and keep.index('previousFilter_(category)') < keep.index('setEnabled(QtWarningMsg, true)'),
            "the rules apply first, then the default category's warnings stay on")
    handler = cpp_body(l2, "frameFailureHandler")
    require('message.startsWith(QLatin1String("Failed to end frame"))' in handler and 'previousHandler_(type, context, message)' in handler,
            "Q9 warning flags the frame; the previous handler still logs")
    cleanup = cpp_body(smoke, "Harness::teardownScene")
    order = ('fn_sa2_scene_trace_end(', 'fn_sa2_drain(', 'fn_sa2_scene_write_run(', 'fn_sa2_scene_unload(',
             'fn_sa2_unregister_slot(', 'destroyWrappers()', 'fn_sa2_release_texture(', 'fn_sa2_detach(')
    positions = [cleanup.index(token) for token in order]
    require(positions == sorted(positions), "end/drain/write/unload/unregister/release/detach order")
    require('l2->traceCompleted && observations.reasons.empty() && l2->exitCode != 3' in cleanup, "no write_run on enforce failure")
    require('SA2_E_CHECK_FAILED' in cpp_body(smoke, "Harness::call") and 'exitCode = 2' in cpp_body(l2, "L2::outcome"), "check failures return 2")
    for token in ('failure("foreground", 3)', 'failure(!visible ? "visibility" : "foreground", 3)',
                  'now - foregroundWait_ < 5 * frequency_', 'qint64(options.prerollMs) * frequency_',
                  'qint64(options.traceMs) * frequency_'):
        require(token in l2, "timing and exit-3 rule: " + token)
    ctor = cpp_body(l2, "L2::L2")
    required_keys = {"format", "candidate", "mode", "run_id", "scene", "process_id", "exit_code", "reason", "options", "configuration", "files",
                     "dll_identity", "dll_status", "qpc_frequency", "sizes", "scaling", "dpi_awareness", "environment", "window", "debug",
                     "trace_ms", "preroll_ms", "turn_ms", "inject", "declared", "gpu_validation", "conditions", "no_vram", "debug_half_target",
                     "abi_version", "last_status", "last_error", "display", "backbuffer", "displayed", "target", "samples", "samples_changed",
                     "device_pixel_ratio", "item_width", "item_height", "texture_stretch", "power_source", "power_mode", "presenting_adapter",
                     "presentation_interval", "refresh_hz", "power_samples", "mains", "battery", "vsync", "adapter", "driver", "msaa", "warp",
                     "topmost", "display_required", "foreground_at_trace_start", "sample_period_ms", "samples_not_visible", "samples_covered",
                     "samples_not_foreground", "visible_throughout", "foreground_throughout", "presents", "enabled", "debug_layer", "counts", "messages"}
    require(required_keys <= set(re.findall(r'\{\s*"(\w+)"\s*,', ctor)), "complete harness shape on failure paths")
    require('"magic600-l2-harness-v1"' in ctor and '{"candidate", "sd"}' in ctor, "harness identity")
    for token in ('GetThreadDpiAwarenessContext()', 'DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2',
                  'ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED', 'PowerRegisterForEffectivePowerModeNotifications',
                  'PowerUnregisterFromEffectivePowerModeNotifications', 'rhi->driverInfo().deviceName', 'EnumDisplayDevicesW',
                  'MONITOR_DEFAULTTOPRIMARY', 'EnumDisplaySettingsW', 'ENUM_CURRENT_SETTINGS', 'GetClientRect', 'currentPixelSize()',
                  'itemSize_.width() * ratio', 'itemSize_.height() * ratio', 'EnumProcessModules', '"qt:exe"', 'file.startsWith(directory',
                  '"qt:" + QFileInfo(file).fileName()', 'name != "QSG_RHI_DEBUG_LAYER"', 'name.startsWith("QSG_")', 'name.startsWith("QT_")'):
        require(token in l2, "runtime fact recorded: " + token)
    sampler = cpp_body(l2, "L2::sample")
    for token in ('frequency_ / 10', 'IsWindowVisible', 'IsIconic', 'DWMWA_CLOAKED', 'covered(hwnd)', 'GetForegroundWindow()',
                  'powerModeAtStart_ = mode', 'powerChanged_ = true', 'GetSystemPowerStatus', 'power.ACLineStatus == 1',
                  'power.ACLineStatus == 0', 'sizes(window, target) != initialSizes_', 'options.enforce && (!visible || !foreground)'):
        require(token in sampler, "100 ms condition sampling: " + token)
    cover = cpp_body(l2, "covered")
    require(cover.count('w / 10') == cover.count('h / 10') == 4 and 'GA_ROOT' in cover and 'WindowFromPoint' in cover, "five-point root hit test")
    conditions = cpp_body(l2, "L2::updateConditions")
    for token in ('samples_ && mains_ == samples_ ? "mains"', 'samples_ && battery_ == samples_ ? "battery"',
                  'mains_ && battery_ && mains_ + battery_ == samples_ ? "changed" : "unknown"', 'powerChanged_ ? QString("changed")'):
        require(token in conditions, "derive power source/mode from samples")
    debug = cpp_body(l2, "L2::debug")
    for field in ('struct_size', 'distinct_id_count', 'corruption', 'error', 'warning', 'info', 'message', 'mismatching_clear_value', 'mentioning_sa2', 'ids'):
        require('{"' + field + '"' in debug, "all debug-count fields")
    writer = cpp_body(l2, "L2::write")
    require('QIODevice::WriteOnly | QIODevice::NewOnly' in writer and 'output.flush()' in writer and 'MoveFileExW' in writer
            and 'MOVEFILE_WRITE_THROUGH' in writer and 'MOVEFILE_REPLACE_EXISTING' not in writer, "atomic exclusive harness writer")
    require('outputDirectory(options.out, false)' in writer and '"harness.json.tmp"' in writer and '"harness.json"' in writer,
            "writer refuses existing outputs")
    for body in (render, after[:after.index('const auto s = snapshot();')], sampler):
        require('std::cout' not in body and 'std::cerr' not in body, "no per-frame logging")


def planted_static_checks(header, loader, structs, sources):
    for changed_header, changed_loader, changed_structs in (
            (header.replace("SA2_ABI_VERSION 2u", "SA2_ABI_VERSION 1u"), loader, structs),
            (header, loader.replace("    SD_BIND(sa2_scene_load)", ""), structs),
            (header, loader.replace("SD_BIND(sa2_scene_load)", "SD_BIND(sa2_scene_unload)"), structs),
            (header, loader.replace("reinterpret_cast<decltype(&name)>", "reinterpret_cast<void*>"), structs),
            (header, loader.replace("fn_sa2_abi_version() != SA2_ABI_VERSION", "false"), structs),
            (header, loader, structs.replace("sizeof(sa2_scene_config) == 32", "sizeof(sa2_scene_config) == 31")),
            (header.replace("uint32_t flags;", "uint64_t flags;"), loader, structs)):
        try:
            abi_checks(changed_header, changed_loader, changed_structs)
        except AssertionError:
            pass
        else:
            raise AssertionError("planted ABI defect accepted")
    defects = (
        ("l2.cpp", 'integer(1000, 3600000)', 'integer(999, 3600000)'),
        ("l2.cpp", 'integer(0, 60000)', 'integer(0, 60001)'),
        ("l2.cpp", 'result <= 0 || result > 10000', 'result < 0 || result > 10000'),
        ("l2.cpp", 'integer(0, 1)', 'integer(0, 2)'),
        ("l2.cpp", '!seen.insert(key).second', 'false'),
        ("l2.cpp", 'name == "overlays"', 'name == "other"'),
        ("l2.cpp", 'options.scene != "w4"', 'false'),
        ("l2.cpp", '"geometry.json"', '"geometry.json", "qt.log"'),
        ("l2.cpp", 'samples_ && mains_ == samples_ ? "mains"', 'samples_ ? "mains"'),
        ("l2.cpp", 'frequency_ / 10', 'frequency_ / 20'),
        ("l2.cpp", 'powerChanged_ = true', 'powerChanged_ = false'),
        ("l2.cpp", 'samples_not_visible', 'not_visible'),
        ("l2.cpp", '{"qpc_frequency", unknown}', '{"clock", unknown}'),
        ("l2.cpp", 'MOVEFILE_WRITE_THROUGH)', 'MOVEFILE_WRITE_THROUGH | MOVEFILE_REPLACE_EXISTING)'),
        ("l2.cpp", 'QIODevice::WriteOnly | QIODevice::NewOnly', 'QIODevice::WriteOnly'),
        ("l2.cpp", 'file.startsWith(directory', 'file.endsWith(directory'),
        ("l2.cpp", 'name != "QSG_RHI_DEBUG_LAYER"', 'name != "other"'),
        ("l2.cpp", 'exitCode = 2', 'exitCode = 1'),
        ("main.cpp", 'surface.setSwapInterval(0)', 'surface.setSwapInterval(1)'),
        ("main.cpp", 'window->showFullScreen()', 'window->show()'),
        ("main.cpp", 'Qt::WindowStaysOnTopHint', 'Qt::WindowTitleHint'),
        ("main.cpp", 'SetForegroundWindow(result.hwnd)', 'IsWindowVisible(result.hwnd)'),
        ("main.cpp", 'past the W3/W4 turns.\n', 'past the W3/W4 turns.\n            debug->SetEnableGPUBasedValidation(TRUE);\n'),
        ("smoke.cpp", 'fn_sa2_scene_produce(context_, k, frame_)', 'fn_sa2_produce(context_, k, frame_, 0, 0)'),
        ("smoke.cpp", 'createRing(wanted)', 'createRing(size)'),
        ("smoke.cpp", 'l2->exitCode != 3', 'l2->exitCode != 4'),
        ("smoke.cpp", 'rhi_->driverInfo().deviceName).toStdString() != adapterName', 'rhi_->driverInfo().deviceName).toStdString() == adapterName'),
        ("smoke.cpp", 'fn_sa2_mark_shown(context_, unsigned(frame_ % SA2_RING_SLOTS), frame_)', 'fn_sa2_godot_wait_ready(context_, frame_)'),
        ("smoke.cpp", 'if (frameFailed()) { fail("present"); return; }', ''),
        ("main.cpp", '    sd::watchFrameFailures();\n', ''),
        ("l2.cpp", '"Failed to end frame"', '"Failed to present"'),
        ("l2.cpp", 'if (previousHandler_) previousHandler_(type, context, message);', ''),
        ("l2.cpp", '    previousFilter_ = QLoggingCategory::installFilter(keepDefaultWarnings);\n', ''),
        ("l2.cpp", 'category->setEnabled(QtWarningMsg, true)', 'category->setEnabled(QtWarningMsg, false)'),
        ("l2.cpp", 'qstrcmp(category->categoryName(), "default") == 0', 'category == QLoggingCategory::defaultCategory()'),
        ("l2.cpp", '    if (previousFilter_) previousFilter_(category);\n'
                   '    if (qstrcmp(category->categoryName(), "default") == 0) category->setEnabled(QtWarningMsg, true);\n',
                   '    if (qstrcmp(category->categoryName(), "default") == 0) category->setEnabled(QtWarningMsg, true);\n'
                   '    if (previousFilter_) previousFilter_(category);\n'),
    )
    for name, good, bad in defects:
        require(good in sources[name], "defect target missing: " + good)
        broken = dict(sources)
        broken[name] = sources[name].replace(good, bad, 1)
        try:
            l2_static_checks(broken)
        except (AssertionError, ValueError):
            pass
        else:
            raise AssertionError("planted static defect accepted: " + good)


def static_checks():
    required = ("app/CMakeLists.txt", "app/src/main.cpp", "app/src/smoke.h", "app/src/smoke.cpp",
                "app/src/native_loader.h", "app/src/native_loader.cpp", "app/src/code_layout.h",
                "app/src/code_layout.cpp", "app/src/code_layout_test.cpp", "build.cmd", "run_smoke.py",
                "smoke_summary.py", "sd_reference.py", "check_project.py", "README.md", "app/src/l2.h", "app/src/l2.cpp", "prepare_l2.py")
    for name in required:
        require((HERE / name).is_file(), "missing " + name)
    cmake = (HERE / "app/CMakeLists.txt").read_text(encoding="utf-8")
    for text in ('cmake_minimum_required(VERSION 3.30)', 'set(CMAKE_CXX_STANDARD 20)',
                 'find_package(Qt6 REQUIRED COMPONENTS Core Gui GuiPrivate Quick)',
                 'NOT Qt6_VERSION STREQUAL "6.10.3"', 'qt_standard_project_setup()',
                 'add_executable(sd_smoke ', 'add_executable(sd_code_layout_test ',
                 'Qt6::Core Qt6::Gui Qt6::GuiPrivate Qt6::Quick d3d12 dxgi dxguid', '../../renderer-sa2/native/include'):
        require(text in cmake, "CMake setting: " + text)
    require(not re.search(r"add_executable\(sd_smoke\s+WIN32", cmake), "console executable required")
    build = (HERE / "build.cmd").read_bytes()
    require(b"\r\n" in build and b"\n" not in build.replace(b"\r\n", b"") and b"\r" not in build.replace(b"\r\n", b""), "build.cmd must be CRLF only")
    require(b"http" not in build.lower(), "offline build only")
    for text in (b"vswhere", b"vcvars64.bat", b"renderer-spike", b"-G Ninja", b"-DCMAKE_BUILD_TYPE=Release", b"-DCMAKE_PREFIX_PATH", b"6.10.3"):
        require(text in build, "build script setting missing")
    header = HEADER.read_text(encoding="utf-8")
    loader = (HERE / "app/src/native_loader.cpp").read_text(encoding="utf-8")
    structs = (HERE / "app/src/native_loader.h").read_text(encoding="utf-8")
    abi_checks(header, loader, structs)
    sources = {path.name: path.read_text(encoding="utf-8") for path in (HERE / "app/src").glob("*") if path.suffix in (".h", ".cpp")}
    l2_static_checks(sources)
    planted_static_checks(header, loader, structs, sources)
    defines = dict(re.findall(r"^#define\s+(SA2_\w+)\s+(\d+)u?\b", header, re.M))
    cpp = "\n".join(path.read_text(encoding="utf-8") for path in (HERE / "app/src").glob("*") if path.suffix in (".h", ".cpp"))
    constants = set(re.findall(r"\bSA2_(?:OK|E_\w+|QUEUE_\w+|BARRIERS_\w+|STATE_\w+)\b", cpp))
    require(constants <= defines.keys(), "unknown status/queue/barrier/state constant")
    require(not re.search(r"(?:#define|constexpr\s+\w+)\s+SA2_", cpp), "use header constants, not copies")
    layout = json.loads(LAYOUT.read_text(encoding="utf-8"))
    code_header = (HERE / "app/src/code_layout.h").read_text(encoding="utf-8")
    for field, macro, attribute in (("block_px", "SA2_CODE_BLOCK_PX", "BLOCK_PX"),
                                   ("grid_cols", "SA2_CODE_GRID_COLS", "GRID_COLS"),
                                   ("grid_rows", "SA2_CODE_GRID_ROWS", "GRID_ROWS"),
                                   ("min_texture_px", "SA2_MIN_TEXTURE_PX", "MIN_TEXTURE_PX"),
                                   ("ring_slots", "SA2_RING_SLOTS", "RING_SLOTS")):
        require(int(defines[macro]) == layout[field] == getattr(reference, attribute), "layout constant " + field)
        require(f"{attribute} = {macro}" in code_header, "C++ layout constant " + field)
    for field in layout["fields"]:
        name = field["name"].upper() + "_BITS"
        require(int(defines["SA2_CODE_" + name]) == field["bits"] == getattr(reference, name), "field width")
        require(f"{name} = SA2_CODE_{name}" in code_header, "C++ field width")
    require([field["first_bit"] for field in layout["fields"]] == [0, 32, 36, 48], "immutable field offsets")
    code_test = (HERE / "app/src/code_layout_test.cpp").read_text(encoding="utf-8")
    for vector in layout["test_vectors"]:
        expected = vector.get("code", vector.get("crc16"))
        require(expected.lower() in code_test.lower(), "C++ immutable test vector missing")
    smoke = (HERE / "app/src/smoke.cpp").read_text(encoding="utf-8")
    main = (HERE / "app/src/main.cpp").read_text(encoding="utf-8")
    for token in ("fromNative", "createFrom", "setNativeLayout", "copyTexture", "uploadTexture", "readBackTexture",
                  "currentFrameCommandBuffer", "sa2_signal_godot_free", "sa2_godot_wait_ready", "sa2_mark_shown",
                  "sa2_drain", "sa2_remove_device", 's.phase = "pending"', "std::chrono::seconds(10)"):
        require(token in smoke, "render protocol source missing " + token)
    pending = smoke.find('s.phase = "pending"')
    write, release = smoke.find("writeResult(false)", pending), smoke.find("releaseRing();", pending)
    require(0 <= pending < write < release, "pending result written before drain")
    for token in ("QQuickGraphicsDevice::fromRhi", "QQuickGraphicsDevice::fromDeviceAndContext",
                  "QQuickWindow::setGraphicsApi(QSGRendererInterface::Direct3D12)", "Qt::QueuedConnection",
                  "Qt::DirectConnection", "owned.release(harness)"):
        require(token in main, "window/lifetime route missing")
    forbidden = re.compile(r"[a-z]:[\\/][^\r\n]*[\\/]users[\\/]|[\\/]users[\\/]|app" + r"data\\", re.I)
    for path in HERE.rglob("*"):
        if path.is_file() and path.suffix in (".py", ".cpp", ".h", ".txt", ".cmd", ".md"):
            text = path.read_text(encoding="utf-8")
            require(not forbidden.search(text), "private path literal in " + path.name)
            if path.suffix == ".py":
                require("sys.dont_write_bytecode = True" in text, "bytecode side effect in " + path.name)
                compile(text, str(path), "exec")
    return layout


def reference_checks(layout):
    for vector in layout["test_vectors"]:
        if "input" in vector:
            require(reference.crc16(vector["input"].encode("ascii")) == int(vector["crc16"], 16), "CRC vector")
        else:
            fields = tuple(vector[key] for key in ("sequence", "slot", "generation"))
            require(reference.code(*fields) == int(vector["code"], 16), "code vector")
    for width, height in ((128, 128), (160, 144)):
        for fields in ((1, 0, 0), (3735928559, 2, 2748), (1000, 1, 7)):
            image = reference.expected_image(width, height, *fields)
            require(reference.decode(image, width, height) == fields, "round trip both corners")
            fill = bytes((32 + 64 * fields[1], fields[0] % 251, 16 * (fields[2] % 16), 255))
            require(image[64 * 4:65 * 4] == fill, "non-code fill texel")
            require(list(image[:4]) == layout["one_rgba8" if reference.code(*fields) & 1 else "zero_rgba8"], "code block color")
            corrupt = bytearray(image)
            # Flip an entire block, first in one corner, then in both corners (CRC check).
            for x0, y0 in ((0, 0), (width - 64, height - 64)):
                for y in range(8):
                    for x in range(8):
                        corrupt[((y0 + y) * width + x0 + x) * 4] ^= 255
                try:
                    reference.decode(corrupt, width, height)
                except ValueError:
                    pass
                else:
                    raise AssertionError("flipped block accepted")
            cw, ch = width + 43, height + 29
            composite = bytearray(bytes((17, 33, 99, 255)) * (cw * ch))
            for y in range(height):
                composite[y * cw * 4:(y * cw + width) * 4] = image[y * width * 4:(y + 1) * width * 4]
            # Different red/blue values at centers make format selection observable.
            for x0, y0 in ((0, 0), (width - 64, height - 64)):
                for bit in range(64):
                    center = ((y0 + (bit // 8) * 8 + 4) * cw + x0 + (bit % 8) * 8 + 4) * 4
                    composite[center + 2] = 255 - composite[center]
            require(reference.decode(composite, cw, ch, width, height, "RGBA8") == fields, "larger RGBA8 composite")
            for pixel in range(cw * ch):
                composite[4 * pixel], composite[4 * pixel + 2] = composite[4 * pixel + 2], composite[4 * pixel]
            require(reference.decode(composite, cw, ch, width, height, "BGRA8") == fields, "larger BGRA8 composite")
            try:
                reference.decode(composite, cw, ch, width, height, "RGBA8")
            except ValueError:
                pass
            else:
                raise AssertionError("wrong channel order accepted")
    require(reference.code(0x100000001, 16, 4096) == reference.code(1, 0, 0), "field wraparound")


def runner_checks():
    for loss in (False, True):
        command = [sys.executable, str(HERE / "run_smoke.py"), "--dry-run"] + (["--device-loss"] if loss else [])
        completed = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=30, check=False)
        require(completed.returncode == 0, "dry-run failed: " + completed.stderr)
        rows = [json.loads(line.split(" ", 1)[1]) for line in completed.stdout.splitlines()
                if re.match(r"Q\d+b? \{", line)]
        require(rows == run_smoke.matrix(loss), "dry-run matrix mismatch")
        require([row["id"] for row in rows] == ["Q0", "Q0b", *[f"Q{i}" for i in range(1, 16 if loss else 14)]], "matrix IDs")
        require(len([line for line in completed.stdout.splitlines() if line.startswith("COMMAND:")]) == len(rows), "command per row")
    rows = {row["id"]: row for row in run_smoke.matrix(True)}
    require(rows["Q8"]["render_loop"] == "basic" and rows["Q8"]["route"] == "import-direct", "basic-loop row")
    require(rows["Q11"]["frames"] == 3000 and not rows["Q11"]["debug_layer"], "long row")
    require(rows["Q10"]["handover"] == "declared" and rows["Q12"]["barriers"] == "match", "probe rows")
    require([r["id"] for r in rows.values() if r.get("inject_drain")] == ["Q13"], "drain injection scoped to Q13")
    adapter = "Synthetic GPU"
    result = dict(format="magic600-sd-smoke-run-v1", status="pass", device=dict(adapter_name=adapter), teardown=dict(phase="complete"))
    require(run_smoke.judge(rows["Q2"], result, 0, False, False, adapter) == "as-expected", "pass judgement")
    require(run_smoke.judge(rows["Q2"], result, 0, False, False, "other") == "unexpected", "adapter judgement")
    pending = copy.deepcopy(result)
    pending["teardown"]["phase"] = "pending"
    pending["frames"] = {"run": 120}
    require(run_smoke.judge(rows["Q13"], pending, 3, False, True, adapter) == "as-expected", "injected drain judgement")
    require(run_smoke.judge(rows["Q13"], pending, 3, False, False, adapter) == "unexpected", "drain stderr required")
    require(run_smoke.judge(rows["Q14"], None, -1, False, False, adapter) == "as-expected", "bounded loss without result")
    require(run_smoke.judge(rows["Q15"], result, 0, True, False, adapter) == "unexpected", "loss timeout")
    recorded = dict(format="magic600-sd-smoke-run-v1", status="recorded", reasons=["debug-errors", "decode-mismatch", "unverified-frames"],
                    device=dict(adapter_name=adapter), frames=dict(run=300),
                    teardown=dict(phase="complete", drain_confirmed=True, detached=True))
    require(run_smoke.judge(rows["Q10"], recorded, 0, False, False, adapter) == "as-expected", "recorded probe with observations")
    for change in (dict(reasons=["debug-layer-unavailable"], frames=dict(run=0)), dict(reasons=["identity-mismatch"]),
                   dict(frames=dict(run=299)), dict(teardown=dict(phase="pending", drain_confirmed=False, detached=False)),
                   dict(teardown=dict(phase="complete", drain_confirmed=True, detached=False)), dict(reasons=["teardown-incomplete"])):
        require(run_smoke.judge(rows["Q10"], {**recorded, **change}, 0, False, False, adapter) == "unexpected",
                "recorded probe that did not run: " + json.dumps(change))
    selftest = dict(format=smoke_summary.SELFTEST_FORMAT, checks=[dict(name="abi", status="pass"),
                                                              dict(name="warp", status="unsupported", reason="no adapter")])
    require(run_smoke.judge(rows["Q0"], selftest, 0, False, False, adapter) == "as-expected", "self-test with a reasoned unsupported check")
    for broken in ({**selftest, "checks": [dict(name="warp", status="unsupported", reason="no adapter")]},
                   {**selftest, "checks": [dict(name="abi", status="pass"), dict(name="warp", status="unsupported")]},
                   {**selftest, "format": "other"}, {**selftest, "status": "fail"}, None):
        require(run_smoke.judge(rows["Q0"], broken, 0, False, False, adapter) == "unexpected", "broken self-test accepted")
    require(run_smoke.judge(rows["Q0"], selftest, 1, False, False, adapter) == "unexpected", "self-test exit code")
    require(smoke_summary.run_summary(dict(row=rows["Q0"], result=selftest))["status"] == "pass", "Q0 summary status")
    command = run_smoke.build_command(Path("C:/a b/build.cmd"), Path("C:/a b/out"))
    require(isinstance(command, str) and command.startswith('cmd.exe /d /c call "') and '\\"' not in command, "cmd.exe string form")
    native = run_smoke.NATIVE
    require(run_smoke.excluded(native, ("build-check-1234", "x.obj")) and run_smoke.excluded(run_smoke.HERE, ("results", "s.json"))
            and run_smoke.excluded(run_smoke.HERE, ("app", "build", "x.obj")) and not run_smoke.excluded(native, ("build.cmd",))
            and not run_smoke.excluded(run_smoke.HERE, ("app", "src", "main.cpp")), "source walk exclusions")
    specs = run_smoke.diff_pathspecs()
    require(":(exclude)work/experiments/renderer-sd/results" in specs
            and ":(exclude,glob)work/experiments/renderer-sa2/native/build*/**" in specs
            and specs[0] == "work/experiments/renderer-sd" and "work/experiments/renderer-sa2/native" in specs, "diff exclusions")
    with patch.dict(os.environ, {"QSG_RHI_DEBUG_LAYER": "1", "M600_SA2_INJECT_UNCONFIRMED_DRAIN": "1", "VCINSTALLDIR": "vc"}):
        cleaned = run_smoke.clean_environment()
        require("QSG_RHI_DEBUG_LAYER" not in cleaned and "M600_SA2_INJECT_UNCONFIRMED_DRAIN" not in cleaned, "inherited probes removed")
        require(cleaned.get("QT_FORCE_STDERR_LOGGING") == "1", "Qt logs to stderr without a console")
        require("VCINSTALLDIR" not in run_smoke.deploy_environment(cleaned), "no VC redistributable in the deployment")


def summary_checks(directory):
    username = "sd_fixture_private_user"
    drive = "Z:" + chr(92) + "private" + chr(92) + "evidence.log"
    pointer = "0x" + "1234ABCDEF98"
    private = dict(evidence_kind="source-fixture", source=dict(head="a" * 40, sha256="b" * 64, matches_head=True),
                   dll_sha256="c" * 64, executable_sha256="d" * 64, qt_version="6.10.3",
                   deployment=dict(total_bytes=12345, file_count=7), runs=[dict(
                       row=run_smoke.matrix()[3], exit_code=0, timeout=False, judgement="as-expected", result_exists=True,
                       stderr={"total": 1}, result=dict(status="pass", reasons=[], device=dict(
                           adapter_name="Synthetic GPU", umd_version=(31 << 48) | (15 << 16) | 5915, enhanced_barriers=True,
                           identity=dict(device_matches=True, fallback_detected=False)),
                           frames=dict(run=1200, eligible=1200), verify=dict(composite_verified=1200),
                           debug=dict(error=0, corruption=0, ids=[1, 2]), teardown=dict(phase="complete", drain_confirmed=True,
                           detached=True, refcount_after=[0, 0, 0], slots_unregistered=3)))])
    planted = copy.deepcopy(private)
    planted["runs"][0]["result"]["debug"]["messages"] = drive + " " + username + " " + pointer
    planted["runs"][0]["result"]["sa2_failures"] = [{"last_error": drive, "pointer": pointer, "name": username}]
    planted["runs"][0]["row"]["command_line"] = drive
    planted["runs"][0]["result"]["reasons"] = [drive, username, pointer]
    summary = smoke_summary.write_summary(planted, directory / "clean-summary.json", username)
    text = json.dumps(summary)
    require(all(value not in text for value in (drive, username, pointer)), "free text leaked through projection")
    require(summary["driver_version"] == "31.0.15.5915", "driver version formatting")
    smoke_summary.scan(summary)
    for value in (drive, username, pointer, chr(37) + "SECRET" + chr(37)):
        poisoned = copy.deepcopy(private)
        poisoned["runs"][0]["result"]["device"]["adapter_name"] = value
        destination = directory / "refused-summary.json"
        try:
            smoke_summary.write_summary(poisoned, destination, username)
        except ValueError:
            require(not destination.exists(), "privacy rejection wrote a file")
        else:
            raise AssertionError("private adapter string accepted")


def pathspec_checks(directory):
    """Exercise the runner's literal/glob exclusions in a plain fixture, without a Git child process."""
    repo = directory / "pathspec-fixture"
    excluded = ("work/experiments/renderer-sd/results/sd-smoke-summary.json", "work/experiments/renderer-sd/app/build/x.obj",
                "work/experiments/renderer-sa2/native/build-check-1/x.obj")
    sources = ("work/experiments/renderer-sd/run_smoke.py", "work/experiments/renderer-sd/build.cmd",
               "work/experiments/renderer-sa2/native/build.cmd")
    for name in excluded + sources:
        (repo / name).parent.mkdir(parents=True, exist_ok=True)
        (repo / name).write_bytes(b"0")
    baseline = {name: run_smoke.sha256(repo / name) for name in excluded + sources}
    specs = run_smoke.diff_pathspecs()

    def selected(name, spec):
        if spec.startswith(":(exclude,glob)"):
            return fnmatch.fnmatchcase(name, spec[len(":(exclude,glob)"):])
        literal = spec.removeprefix(":(exclude)")
        return name == literal or name.startswith(literal + "/")

    def changed(active_specs=specs):
        included = [spec for spec in active_specs if not spec.startswith(":(exclude")]
        omitted = [spec for spec in active_specs if spec.startswith(":(exclude")]
        return any(run_smoke.sha256(repo / name) != digest and any(selected(name, spec) for spec in included)
                   and not any(selected(name, spec) for spec in omitted) for name, digest in baseline.items())
    for name in excluded:
        (repo / name).write_bytes(b"1")
    require(not changed(), "a change under an excluded folder counts as a source change")
    require(changed([spec for spec in specs if "renderer-sd/results" not in spec and "renderer-sd/**/results" not in spec]),
            "planted missing exclusion not caught")
    for name in sources:
        (repo / name).write_bytes(b"1")
        require(changed(), "source change missed: " + name)
        (repo / name).write_bytes(b"0")


def remove_tree(path):
    """Remove a fixture tree, including Git's read-only object files."""
    def retry(function, name, _):
        os.chmod(name, stat.S_IWRITE)
        function(name)
    shutil.rmtree(path, onexc=retry)


def reuse_checks(directory):
    root = directory / "reuse-fixture"
    build = root / "work/sdb/20261003T000000Z"
    for relative in ("native/sa2_interop.dll", "native/sa2_selftest.exe", "app/sd_code_layout_test.exe",
                     "app/sd_smoke.exe", "deploy/sd_smoke.exe", "deploy/Qt6Core.dll"):
        path = build / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(relative.encode("ascii"))
    source = dict(head="a" * 40, sha256="b" * 64, matches_head=True)
    identity = dict(source=source, qt_version="6.10.3", artifacts=run_smoke.deployment_manifest(build),
                    dll_sha256=run_smoke.sha256(build / "native/sa2_interop.dll"),
                    executable_sha256=run_smoke.sha256(build / "deploy/sd_smoke.exe"))
    run_smoke.atomic_json(build / "build_identity.json", identity)
    with patch.object(run_smoke, "ROOT", root):
        require(run_smoke.reuse_build(source, "6.10.3")[0] == build, "matching build reuse")
        for altered_source, version in ((dict(source, sha256="e" * 64), "6.10.3"), (source, "6.11.0")):
            try:
                run_smoke.reuse_build(altered_source, version)
            except ValueError:
                pass
            else:
                raise AssertionError("stale build reused")
        (build / "app/sd_code_layout_test.exe").write_bytes(b"tampered fixture")
        try:
            run_smoke.reuse_build(source, "6.10.3")
        except ValueError:
            pass
        else:
            raise AssertionError("modified artifact reused")


def launch_checks(directory):
    build = directory / "launch-fixture"
    for relative in ("native/sa2_interop.dll", "deploy/sd_smoke.exe", "deploy/Qt6Core.dll"):
        path = build / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(relative.encode("ascii"))
    foreign_qt = directory / "other-qt/bin"
    foreign_qt.mkdir(parents=True)
    (foreign_qt / "qmake.exe").write_bytes(b"fixture")
    ordinary = directory / "ordinary-bin"
    ordinary.mkdir()
    env = dict(PATH=os.pathsep.join((str(foreign_qt), str(ordinary))), QT_PLUGIN_PATH="other", QML_IMPORT_PATH="other",
               QSG_RHI_DEBUG_LAYER="1", QSG_RENDER_TIMING="1", QT_LOGGING_RULES="*.debug=true")
    with patch.dict(os.environ, env, clear=True):
        record = prepare_l2.write_launch(build)
        require(json.loads((build / "launch.json").read_text(encoding="utf-8")) == record, "launch writer round trip")
    require(set(record) == {"format", "candidate", "executable", "dll", "working_directory", "arguments", "run_arguments",
                            "validation_arguments", "separator", "environment", "validation_environment"}, "launch shape")
    require(record["format"] == "magic600-l2-launch-v1" and record["candidate"] == "sd", "launch identity")
    require(Path(record["executable"]) == build / "deploy/sd_smoke.exe" and Path(record["dll"]) == build / "native/sa2_interop.dll"
            and Path(record["working_directory"]) == build / "deploy", "launch uses deployed presenter and built DLL")
    require(all(Path(record[key]).is_absolute() for key in ("executable", "dll", "working_directory")), "absolute launch paths")
    require(record["separator"] == record["arguments"] == record["run_arguments"] == record["validation_arguments"] == []
            and record["validation_environment"] == {}, "ordinary Qt options; validation via app flag")
    cleaned = record["environment"]
    require(cleaned["PATH"].split(os.pathsep) == [str(build / "deploy"), str(ordinary)], "foreign Qt PATH removed")
    for name in ("QSG_RHI_DEBUG_LAYER", "M600_SA2_INJECT_UNCONFIRMED_DRAIN", "QT_PLUGIN_PATH", "QT_QPA_PLATFORM_PLUGIN_PATH",
                 "QML_IMPORT_PATH", "QML2_IMPORT_PATH", "QSG_RHI_BACKEND", "QSG_INFO", "QSG_RENDER_TIMING", "QSG_RHI_PROFILE", "QSG_VISUALIZE",
                 "QSG_RENDERER_DEBUG", "QT_DEBUG_PLUGINS"):
        require(cleaned[name] is None, "inherited validation/plugin/frame logging removed")
    require(cleaned["QT_ENABLE_HIGHDPI_SCALING"] == "0" and cleaned["QSG_RENDER_LOOP"] == "threaded"
            and cleaned["QT_LOGGING_RULES"] == "qt.rhi.general=true;qt.scenegraph.general=true", "level 1 run environment")
    require(not (build / "launch.json.tmp").exists(), "launch atomic rename completed")
    # The C++ output allowlist is tested on the runner's pre-populated directory, without Qt.
    out = directory / "output-fixture"
    out.mkdir()
    (out / "qt.log").write_text("fixture", encoding="utf-8")
    (out / "presentmon.csv").write_text("fixture", encoding="utf-8")
    body = cpp_body((HERE / "app/src/l2.cpp").read_text(encoding="utf-8"), "outputDirectory")
    reserved = set(re.findall(r'"([\w.]+)"', body))
    require(not any((out / name).exists() for name in reserved), "logs and CSV accepted")
    for name in reserved:
        path = out / name
        path.write_bytes(b"preserve")
        require(any((out / item).exists() for item in reserved), "existing output refused")
        path.unlink()
    source = (HERE / "prepare_l2.py").read_text(encoding="utf-8")
    for token in ('run_smoke.build_all(build, build, env, source)', 'run_smoke.reuse_build(source, version)',
                  'len(str(build / "native")) > 150', 'version != "6.10.3"', 'run_smoke.source_identity()', '"work/sdb"', 'write_launch(build)'):
        require(token in source, "prepare reuses level 1 build contract")
    body = source.split("def main(argv=None):", 1)[1].split('if __name__ == "__main__":', 1)[0]
    require('sd_smoke.exe' not in body, "prepare never starts presenter")


def main():
    layout = static_checks()
    reference_checks(layout)
    runner_checks()
    # Plain mkdir, owned uniquely by this invocation, with cleanup on every path.
    base = Path(tempfile.gettempdir()).resolve()
    directory = base / ("m600-sd-check-" + str(os.getpid()) + "-" + uuid.uuid4().hex)
    directory.mkdir()
    try:
        summary_checks(directory)
        reuse_checks(directory)
        pathspec_checks(directory)
        launch_checks(directory)
    finally:
        require(directory.resolve().parent == base, "temp cleanup boundary")
        remove_tree(directory)
    print("sd project: ok (source/static and Python fixtures; no Qt, C++ build or GPU)")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (AssertionError, OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"sd project: {error}", file=sys.stderr)
        sys.exit(1)
