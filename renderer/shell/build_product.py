"""Build C9's product on route B offline into work/build/c9-product/<stamp>/; never launch Godot, a window or a GPU.

A product build holds eight parts, each built from this checkout:

- `native/`: the S-A2 core (`renderer/core/build.cmd`), `sa2_interop.dll` with the shaders it loads beside it;
- `host/c600_shell_host.dll`: the shell's native host (`renderer/bridge/host`), a release build;
- `runtime/c600-runtime.exe`: the language runtime's process (`rust/c600-runtime`, route RT1), a release build, which
  the host starts (charter E3, a draft);
- `shell/`: the Godot .NET shell (`build_shell.py`);
- `assets/`: a copy of the shipped model's tracked files, which the host loads;
- `menu/`: a copy of the twist menu that the turns' session pins (charter E2, a draft);
- `session-fixture/`: charter E3's existing session (a draft): the host's `c600-shell-session-fixture` writes a
  session of a few committed presses through the same turns, with `fixture.json`, and starts the build's runtime once to
  show that it starts. A launch reopens a fresh copy of that session.
- `session-fixture-jumble-on/`: the same with Jumble on (`--puzzle jumble-on`): the session commits `jumble.on` at its
  start and the presses commit `jumble.twist.menu`. A launch with `--puzzle jumble-on` reopens a fresh copy of it.

`product.json` (`magic600-c9-product-build-v1`) names every part by its SHA-256 and gives the build identity, a digest
of those parts, which the shell's logs carry (`M600_BUILD_IDENTITY`). `launch.py` starts a product build after
`verify()` has recomputed every digest. A pass is source-build evidence only: no launch, no window, no GPU.

  python -B renderer/shell/build_product.py [--godot <exe>]
"""
from __future__ import annotations

import sys

sys.dont_write_bytecode = True

import argparse
import datetime as dt
import hashlib
import json
import os
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SHELL = ROOT / "renderer" / "shell"
BUILD_ROOT = ROOT / "work" / "build" / "c9-product"
FORMAT = "magic600-c9-product-build-v1"
RECORD = "product.json"
CORE_DLL = "native/sa2_interop.dll"
HOST_DLL = "host/c600_shell_host.dll"
PROJECT = "shell/project"
ASSETS = "assets"
# The twist menu that the turns' session pins (charter E2; draft: the I_a menu that KN3's and c600-input's tests pin;
# G6 owns the menu).
MENU_SOURCE = "research/jumbling/fixtures/wj-I_a.json"
MENU = "menu/wj-I_a.json"
RUNTIME = "runtime/c600-runtime.exe"
# Each puzzle's session fixture folder and the prefix of its part names (the shell's puzzle names, EventLog.Puzzles).
FIXTURES = {"jumble-off": "session-fixture", "jumble-on": "session-fixture-jumble-on"}
FIXTURE_PARTS = {"jumble-off": "fixture:", "jumble-on": "fixture-jumble-on:"}
FIXTURE = FIXTURES["jumble-off"]
FIXTURE_RECORD = FIXTURE + "/fixture.json"
FIXTURE_SESSION = FIXTURE + "/session"


def fixture_record(puzzle: str) -> str:
    """The build-relative path of `puzzle`'s fixture record."""
    return FIXTURES[puzzle] + "/fixture.json"


def fixture_session(puzzle: str) -> str:
    """The build-relative path of `puzzle`'s fixture session."""
    return FIXTURES[puzzle] + "/session"

BRIDGE = ROOT / "renderer" / "bridge"
RUST = ROOT / "rust"
sys.path.insert(0, str(SHELL))
sys.path.insert(0, str(ROOT / "renderer" / "acceptance"))
import build_shell  # noqa: E402
import prepare_host  # noqa: E402  (the harness's Godot locator, logged runs and digests)


class ProductError(Exception):
    """A product build that is missing, malformed or changed since it was built."""


def core_command(native: Path) -> list[str]:
    return ["cmd.exe", "/d", "/c", "call", str(ROOT / "renderer" / "core" / "build.cmd"), str(native)]


def host_command() -> list[str]:
    return ["cargo", "build", "--locked", "--offline", "--release", "--manifest-path", str(BRIDGE / "Cargo.toml"),
            "-p", "c600-shell-host"]


def runtime_command() -> list[str]:
    return ["cargo", "build", "--locked", "--offline", "--release", "--manifest-path", str(RUST / "Cargo.toml"),
            "-p", "c600-runtime"]


def fixture_command(build: Path, puzzle: str = "jumble-off") -> list[str]:
    """The host's fixture writer of `puzzle` over the build's own model, menu and runtime."""
    return [str(BRIDGE / "target" / "release" / "c600-shell-session-fixture.exe"), "--assets", str(build / ASSETS),
            "--menu", str(build / MENU), "--out", str(build / FIXTURES[puzzle]), "--puzzle", puzzle,
            "--runtime", str(build / RUNTIME)]


def part_paths(build: Path) -> dict[str, Path]:
    """Every file the product loads, by part name: the core with its shaders, the host, the runtime, the shell's
    assembly and project files (not Godot's or dotnet's generated folders), the model's files, the twist menu and each
    puzzle's session fixture."""
    native, project, assets = build / "native", build / PROJECT, build / ASSETS
    paths = {"core:sa2_interop.dll": build / CORE_DLL, "host:c600_shell_host.dll": build / HOST_DLL,
             "runtime:" + Path(RUNTIME).name: build / RUNTIME, "shell:assembly": project / build_shell.ASSEMBLY,
             "menu:" + Path(MENU).name: build / MENU}
    for puzzle, prefix in FIXTURE_PARTS.items():
        paths[prefix + "fixture.json"] = build / fixture_record(puzzle)
    if native.is_dir():
        paths.update({"core:" + p.name: p for p in native.iterdir() if p.is_file() and p.suffix.lower() == ".dxil"})
    if project.is_dir():
        paths.update({"shell:" + p.relative_to(project).as_posix(): p for p in project.rglob("*")
                      if p.is_file() and not any(part in prepare_host.EXCLUDED
                                                 for part in p.relative_to(project).parts[:-1])})
    if assets.is_dir():
        paths.update({"assets:" + p.relative_to(assets).as_posix(): p for p in assets.rglob("*") if p.is_file()})
    for puzzle, prefix in FIXTURE_PARTS.items():
        fixture = build / FIXTURES[puzzle]
        if fixture.is_dir():
            paths.update({prefix + p.relative_to(fixture).as_posix(): p for p in fixture.rglob("*") if p.is_file()})
    return paths


def copy_assets(target: Path) -> int:
    """Copy the shipped model's tracked files (assets/ in this checkout) into `target`; the count copied."""
    names = [name for name in prepare_host.git("ls-files", "-z", "--", "assets").split("\0") if name]
    if "assets/manifest.json" not in names:
        raise ProductError("this checkout tracks no assets/manifest.json")
    for name in names:
        destination = target / Path(name).relative_to("assets")
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(ROOT / name, destination)
    return len(names)


def copy_menu(target: Path) -> None:
    """Copy the tracked twist menu record (MENU_SOURCE) to `target`."""
    if prepare_host.git("ls-files", "--", MENU_SOURCE).strip() != MENU_SOURCE:
        raise ProductError(f"this checkout tracks no {MENU_SOURCE}")
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(ROOT / MENU_SOURCE, target)


def identity(digests: dict[str, str]) -> str:
    """The build identity: a digest of every part's name and SHA-256, in name order."""
    canonical = json.dumps(dict(sorted(digests.items())), separators=(",", ":"), sort_keys=True)
    return "c9-product-" + hashlib.sha256(canonical.encode("utf-8")).hexdigest()[:16]


def record(build: Path, executable: Path, head: str, dirty: bool) -> dict:
    parts = {name: {"path": path.relative_to(build).as_posix(), "sha256": prepare_host.sha256(path)}
             for name, path in sorted(part_paths(build).items())}
    shaders = [name for name in parts if name.startswith("core:") and name.endswith(".dxil")]
    if not shaders:
        raise ProductError("the core build has no shaders beside its DLL")
    for puzzle, prefix in FIXTURE_PARTS.items():
        if not any(name.startswith(prefix + "session/") for name in parts):
            raise ProductError(f"the {puzzle} session fixture holds no session")
    return {
        "format": FORMAT,
        "evidence": "source build only: never launched",
        "identity": identity({name: part["sha256"] for name, part in parts.items()}),
        "head": head,
        "dirty": dirty,
        "godot": {"executable": str(executable), "sha256": prepare_host.sha256(executable),
                  "version_expected": prepare_host.GODOT_VERSION},
        "parts": parts,
        "commands": {"core": "renderer/core/build.cmd <build>/native",
                     "host": ("cargo build --locked --offline --release --manifest-path renderer/bridge/Cargo.toml "
                              "-p c600-shell-host"),
                     "runtime": "cargo build --locked --offline --release --manifest-path rust/Cargo.toml -p c600-runtime",
                     "shell": " ".join(build_shell.build_command()),
                     **{"fixture " + puzzle: ("c600-shell-session-fixture --assets <build>/assets --menu "
                                              f"<build>/menu/wj-I_a.json --out <build>/{folder} --puzzle {puzzle} "
                                              "--runtime <build>/runtime/c600-runtime.exe")
                        for puzzle, folder in FIXTURES.items()}},
    }


def verify(build: Path) -> dict:
    """product.json of `build` after recomputing every part's SHA-256 and the identity; ProductError otherwise."""
    try:
        product = json.loads((build / RECORD).read_text(encoding="utf-8"))
    except (OSError, ValueError, UnicodeError) as error:
        raise ProductError(f"{RECORD} unreadable: {error}") from None
    if not isinstance(product, dict) or product.get("format") != FORMAT or not isinstance(product.get("parts"), dict):
        raise ProductError(f"{RECORD} is not {FORMAT}")
    expected = {name: build / part["path"] for name, part in product["parts"].items()}
    if set(expected) != set(part_paths(build)):
        raise ProductError("the build's files differ from the parts product.json names")
    for name, path in expected.items():
        if not path.is_file() or prepare_host.sha256(path) != product["parts"][name]["sha256"]:
            raise ProductError(f"part {name} changed or is missing")
    if identity({name: part["sha256"] for name, part in product["parts"].items()}) != product.get("identity"):
        raise ProductError("the identity does not match the parts")
    godot = product.get("godot") or {}
    executable = Path(str(godot.get("executable", "")))
    if executable.name != prepare_host.GODOT_EXE or not executable.is_file() \
            or prepare_host.sha256(executable) != godot.get("sha256"):
        raise ProductError("the Godot executable changed or is missing")
    return product


def newest(root: Path = BUILD_ROOT) -> Path | None:
    """The newest product build that holds product.json, or None."""
    found = sorted((path.parent for path in root.glob(f"*/{RECORD}")), key=lambda path: path.name)
    return found[-1] if found else None


def build(godot: str | None) -> Path:
    executable, _ = prepare_host.locate_without_launch(godot)
    head = prepare_host.git("rev-parse", "HEAD").strip()
    dirty = prepare_host.git("status", "--porcelain", "--", "renderer", "rust", "assets").strip() != ""
    out = BUILD_ROOT / dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    native, host, runtime, logs = out / "native", out / "host", out / "runtime", out / "logs"
    if len(str(native)) > prepare_host.BUILD_PATH_LIMIT:
        raise ProductError("checkout path too long for the native build; use a shorter checkout path")
    if any(c in str(native) for c in ('"', "\r", "\n")):
        raise ProductError("unsupported build path")
    # Plain mkdir under the ignored work tree, never %TEMP% (Application Control).
    out.mkdir(parents=True)
    for folder in (native, host, runtime, logs):
        folder.mkdir()
    env = os.environ.copy()
    for name in ("M600_SA2_CPU_CHECK", prepare_host.INJECT):
        env.pop(name, None)
    print(f"core: {native}", flush=True)
    prepare_host.run_logged(core_command(native), ROOT, env, 1200, logs / "core")
    print("host: cargo build --release -p c600-shell-host", flush=True)
    # From renderer/bridge, so that cargo reads its .cargo/config.toml: the vendored crates, with no network.
    prepare_host.run_logged(host_command(), BRIDGE, env, 1800, logs / "host")
    shutil.copy2(BRIDGE / "target" / "release" / "c600_shell_host.dll", host / "c600_shell_host.dll")
    print("runtime: cargo build --release -p c600-runtime", flush=True)
    # From rust/, so that cargo reads its .cargo/config.toml.
    prepare_host.run_logged(runtime_command(), RUST, env, 1800, logs / "runtime")
    shutil.copy2(RUST / "target" / "release" / "c600-runtime.exe", out / RUNTIME)
    build_shell.build(str(executable), out / "shell")
    print(f"assets: {copy_assets(out / ASSETS)} files", flush=True)
    copy_menu(out / MENU)
    for puzzle in FIXTURES:
        print(f"fixture: c600-shell-session-fixture --puzzle {puzzle} (starts the runtime once)", flush=True)
        prepare_host.run_logged(fixture_command(out, puzzle), ROOT, env, 600,
                                logs / ("fixture" if puzzle == "jumble-off" else "fixture-" + puzzle))
    prepare_host.write_new(out / RECORD, record(out, Path(executable).resolve(), head, dirty))
    verify(out)
    return out


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--godot", help="the pinned Godot 4.7.2 .NET executable, if not the winget install")
    args = parser.parse_args(argv)
    try:
        out = build(args.godot)
        product = verify(out)
    except (OSError, RuntimeError, ProductError, subprocess.SubprocessError) as error:
        print(f"FAIL: {error}")
        return 1
    print(f"PASS: product build {product['identity']} in {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
