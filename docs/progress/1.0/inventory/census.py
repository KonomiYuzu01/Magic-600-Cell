"""Source-derived census of 0.4 entry points, used to audit both sealed inventories for omissions.

Usage:
  python docs/progress/1.0/inventory/census.py           write census.json
  python docs/progress/1.0/inventory/census.py --check   verify census.json is current

The census lists what can be found mechanically in the owner's source scope:
- command IDs: `keymap_catalog.keymap_command_ids()` (the IDs every bank may bind);
- engine routes: `path == '...'`, `path in (...)` and `path.startswith('...')` in `server.py` and `adapter.py`;
- adapter command actions: `action == '...'` and `action in (...)` in `adapter.py`;
- CLI options: `add_argument('--...')` in the scope's Python programs (`server.py`, `native/`, the 0.4 workspace).
It does not find button labels, menus, mouse gestures or key bindings; those stay with the two inventories.
It reads source text only (and imports `keymap_catalog`, which has no side effects). Standard library only.
"""
from __future__ import annotations

import importlib.util
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
EXP = ROOT / "work/experiments/magic600-04"
OUT = HERE / "census.json"
ROUTE_FILES = [ROOT / "server.py", EXP / "adapter.py"]
CLI_FILES = sorted([ROOT / "server.py", *(ROOT / "native").glob("*.py"), *EXP.glob("*.py"), *(EXP / "packaging").glob("*.py")])
_STR = r"'([^']*)'"


def rel(p: Path) -> str:
    return p.relative_to(ROOT).as_posix()


def line_of(text: str, pos: int) -> int:
    return text.count("\n", 0, pos) + 1


def command_ids() -> list[dict]:
    spec = importlib.util.spec_from_file_location("keymap_catalog", EXP / "keymap_catalog.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return [{"kind": "command", "id": c, "where": "work/experiments/magic600-04/keymap_catalog.py"}
            for c in sorted(mod.keymap_command_ids())]


def routes() -> list[dict]:
    found = {}
    for f in ROUTE_FILES:
        text = f.read_text(encoding="utf-8")
        for m in re.finditer(r"path\s*==\s*" + _STR, text):
            found.setdefault(m.group(1), f"{rel(f)}:{line_of(text, m.start())}")
        for m in re.finditer(r"path\.startswith\(" + _STR + r"\)", text):
            found.setdefault(m.group(1) + "*", f"{rel(f)}:{line_of(text, m.start())}")
        for m in re.finditer(r"path\s+(?:not\s+)?in\s*\(([^)]*)\)", text):
            for s in re.findall(_STR, m.group(1)):
                if s.startswith("/"):
                    found.setdefault(s, f"{rel(f)}:{line_of(text, m.start())}")
    return [{"kind": "route", "id": k, "where": v} for k, v in sorted(found.items())]


def actions() -> list[dict]:
    f = EXP / "adapter.py"
    text = f.read_text(encoding="utf-8")
    found = {}
    for m in re.finditer(r"action\s*==\s*" + _STR, text):
        found.setdefault(m.group(1), f"{rel(f)}:{line_of(text, m.start())}")
    for m in re.finditer(r"action\s+in\s*\(([^)]*)\)", text):
        for s in re.findall(_STR, m.group(1)):
            found.setdefault(s, f"{rel(f)}:{line_of(text, m.start())}")
    return [{"kind": "action", "id": k, "where": v} for k, v in sorted(found.items()) if k]


def cli_options() -> list[dict]:
    out = []
    for f in CLI_FILES:
        if f.name.startswith("test"):
            continue
        text = f.read_text(encoding="utf-8")
        for m in re.finditer(r"add_argument\(\s*'(--[A-Za-z0-9-]+)'", text):
            out.append({"kind": "cli", "id": f"{f.name} {m.group(1)}", "where": f"{rel(f)}:{line_of(text, m.start())}"})
    return sorted(out, key=lambda e: e["id"])


def build() -> dict:
    items = command_ids() + routes() + actions() + cli_options()
    return {"format": "C600-INVENTORY-CENSUS-draft",
            "counts": {k: sum(1 for i in items if i["kind"] == k) for k in ("command", "route", "action", "cli")},
            "items": items}


def dump(doc: dict) -> str:
    return json.dumps(doc, indent=1, sort_keys=True, ensure_ascii=False) + "\n"


def main(argv: list[str]) -> int:
    text = dump(build())
    if argv == ["--check"]:
        ok = OUT.is_file() and OUT.read_text(encoding="utf-8") == text
        print("census: ok" if ok else "census.json is stale; run census.py")
        return 0 if ok else 1
    if argv:
        print(__doc__.strip().splitlines()[2].strip())
        return 2
    OUT.write_text(text, encoding="utf-8")
    print("census:", json.loads(text)["counts"])
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
