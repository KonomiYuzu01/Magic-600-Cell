"""Static checks of C9's product shell project (renderer/shell); never builds, launches Godot or touches a GPU.

The shell builds with renderer/shell/build_shell.py on a machine with the pinned Godot 4.7.2 .NET install; this check
runs everywhere and keeps the project's fixed points: Godot.NET.Sdk 4.7.2 on net10.0 (ADR-R01.5), an offline
package source only, the route-B window settings, and the core's export list and ABI version equal to the bridge's.
A pass is source evidence only.
"""
from __future__ import annotations

import re
import sys
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


def csharp_list(name: str) -> list[str]:
    source = text(PROJECT / "CoreLibrary.cs")
    block = re.search(rf"\b{name} = new\[\]\s*\{{(.*?)\}};", source, re.S)
    assert block, f"{name} in CoreLibrary.cs"
    return re.findall(r'"(sa2_[a-z_]+)"', block.group(1))


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


if __name__ == "__main__":
    unittest.main()
