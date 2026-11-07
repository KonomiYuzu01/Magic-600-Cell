"""Check a closure packet's manifest against the findings it owns. Writes nothing.

Usage:
  python docs/architecture/1.0/language/closure/check_closure.py --packet p2a
  python docs/architecture/1.0/language/closure/check_closure.py --all

The check is mechanical. It shows that each finding cites anchored clause or requirement definitions
and a concrete expected result for each named vector. It cannot show that the text is right: that is the review's job.
"""
import argparse
import hashlib
import importlib.util
import json
import pathlib
import re
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[4]
LANGUAGE = HERE.parent
PROGRESS = ROOT / "docs" / "progress" / "1.0"
EXTRA_TEXT = [PROGRESS / "jumbling-interfaces.md", PROGRESS / "layer-interfaces.md", PROGRESS / "command-table.md"]
DEFINITION = re.compile(r"^\s*(?:- |\| )?\*\*([A-Z]{2,3}\d?\.\d+[a-z]?)(?: \[[^\]]*\])?\*\*", re.M)
REQUIREMENT_DEFINITION = re.compile(r"^\|[ \t]*(CT-[A-Z0-9]+-\d+)[ \t]*\|", re.M)
ERROR_CODE = re.compile(r"^[A-Z0-9]{2,4}:[a-z_]+\.[a-z0-9_.]+$")
VAGUE = {"", "see text", "see the text", "tbd", "todo", "to do", "as above", "n/a"}
DELTA_ROW_FIELDS = {"id", "label", "kind", "inventory", "gates", "lane", "preview", "undo", "contexts", "repeat", "job",
                    "key_0_4", "disposition", "note", "source"}
errors = []


def fail(text):
    errors.append(text)


def texts():
    files = sorted(LANGUAGE.rglob("*.md")) + [p for p in EXTRA_TEXT if p.is_file()]
    return [(p, p.read_bytes().decode("utf-8")) for p in files]


def defined_ids():
    """Accept clause markers and first-column requirement rows, never citations."""
    ids = set()
    for _, text in texts():
        ids |= set(DEFINITION.findall(text)) | set(REQUIREMENT_DEFINITION.findall(text))
    return ids


def duplicate_definitions():
    seen = {}
    dups = []
    for path, text in texts():
        for name in DEFINITION.findall(text):
            if name in seen and seen[name] != path:
                dups.append(f"{name}: {seen[name].name} and {path.name}")
            elif name in seen:
                dups.append(f"{name}: twice in {path.name}")
            seen[name] = path
    return dups


def concrete(value):
    return isinstance(value, str) and value.strip().lower() not in VAGUE


def check_vector(where, tag, vector, ids):
    if not isinstance(vector, dict):
        fail(f"{where}: vector {tag} is not an object")
        return
    for field in ("given", "expected"):
        if not concrete(vector.get(field)):
            fail(f"{where}: vector {tag} has no concrete {field}")
    if vector.get("clause") not in ids:
        fail(f"{where}: vector {tag} cites unknown clause {vector.get('clause')!r}")


def check_delta():
    path = PROGRESS / "command-rows-jumbling.json"
    if not path.is_file():
        fail("p1: command-rows-jumbling.json is missing")
        return
    delta = json.loads(path.read_bytes())
    if delta.get("format") != "c600-command-rows-delta-1":
        fail("p1: delta format differs")
    spec = importlib.util.spec_from_file_location("build_command_table", PROGRESS / "build_command_table.py")
    builder = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(builder)
    current = {r["id"]: r for r in builder.build_table(draft_without_jumbling=True)["commands"]}
    seen_add = set()
    for row in delta.get("add", []):
        missing = DELTA_ROW_FIELDS - set(row)
        if missing:
            fail(f"p1: add {row.get('id')}: missing {sorted(missing)}")
        if row.get("id") in current or row.get("id") in seen_add:
            fail(f"p1: add {row.get('id')}: the ID already exists")
        seen_add.add(row.get("id"))
    fields = set()
    for item in delta.get("amend", []):
        name = item.get("id")
        if name not in current:
            fail(f"p1: amend {name}: no such row")
            continue
        for key, value in item.get("expect", {}).items():
            if current[name].get(key) != value:
                fail(f"p1: amend {name}: expected {key}={value!r}, row has {current[name].get(key)!r}")
        for key in item.get("set", {}):
            if (name, key) in fields:
                fail(f"p1: amend {name}: {key} set twice")
            fields.add((name, key))
        if not item.get("set") or not item.get("source"):
            fail(f"p1: amend {name}: needs set and source")


def owned_digest(paths):
    digest = hashlib.sha256()
    for rel in sorted(paths):
        path = ROOT / rel
        members = sorted(p for p in path.rglob("*") if p.is_file()) if path.is_dir() else ([path] if path.is_file() else [])
        for member in members:
            digest.update(member.relative_to(ROOT).as_posix().encode("utf-8") + b"\0" + member.read_bytes() + b"\0")
    return digest.hexdigest()


def run_extra(name, packet):
    for script in packet.get("extra_checks", []):
        before = owned_digest(packet["files"])
        result = subprocess.run([sys.executable, "-B", str(ROOT / script)], cwd=ROOT, capture_output=True, text=True,
                                encoding="utf-8", errors="replace", timeout=1200)
        print(f"{name}: {script}: exit {result.returncode}")
        if result.returncode:
            fail(f"{name}: {script} failed: {(result.stdout + result.stderr)[-1500:]}")
        if owned_digest(packet["files"]) != before:
            fail(f"{name}: {script} changed an owned file; a check must write nothing that remains")


def check_packet(name, data, ids):
    packet = data["packets"][name]
    for rel in packet["files"]:
        if not (ROOT / rel).exists():
            fail(f"{name}: {rel} is missing")
    path = HERE / "manifests" / f"{name}.json"
    if not path.is_file():
        fail(f"{name}: manifest is missing")
        return
    manifest = json.loads(path.read_bytes())
    if manifest.get("packet") != name:
        fail(f"{name}: manifest names another packet")
    for key in ("assumes", "owner_items", "handoff_changes", "new_error_codes", "new_requirements"):
        if not isinstance(manifest.get(key), list):
            fail(f"{name}: manifest field {key} must be a list")
    for item in manifest.get("assumes", []):
        for key in ("item", "assumed", "if_other"):
            if not concrete(item.get(key)):
                fail(f"{name}: an assumption has no concrete {key}")
    for item in manifest.get("new_error_codes", []):
        if not ERROR_CODE.match(item.get("code", "")):
            fail(f"{name}: malformed error code {item.get('code')!r}")
    for fid, finding in data["findings"].items():
        tags = finding["owners"].get(name)
        if tags is None:
            continue
        entry = manifest.get("findings", {}).get(fid)
        if entry is None:
            fail(f"{name}: {fid} is not answered")
            continue
        status = entry.get("status")
        if status not in ("closed", "partial", "open"):
            fail(f"{name}: {fid}: status {status!r}")
        if status != "closed":
            for key in ("why_open", "who"):
                if not concrete(entry.get(key)):
                    fail(f"{name}: {fid}: {status} needs {key}")
        clauses = entry.get("clauses") or []
        if not clauses:
            fail(f"{name}: {fid}: no clause")
        for clause in clauses:
            if clause not in ids:
                fail(f"{name}: {fid}: unknown clause {clause}")
        vectors = entry.get("vectors") or {}
        for tag in tags:
            if tag not in vectors:
                fail(f"{name}: {fid}: vector {tag} is missing")
        for tag, vector in vectors.items():
            check_vector(f"{name}: {fid}", tag, vector, ids)
        print(f"{name}: {fid}: {status}, {len(clauses)} clauses, {len(vectors)} vectors")
    if name == "p1":
        check_delta()
    run_extra(name, packet)


def main():
    parser = argparse.ArgumentParser()
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--packet")
    group.add_argument("--all", action="store_true")
    args = parser.parse_args()
    data = json.loads((HERE / "findings.json").read_bytes())
    ids = defined_ids()
    if args.packet and args.packet not in data["packets"]:
        sys.exit(f"unknown packet {args.packet}")
    for name in (sorted(data["packets"]) if args.all else [args.packet]):
        check_packet(name, data, ids)
    for line in duplicate_definitions():
        fail(f"duplicate clause definition: {line}")
    if errors:
        print("\n".join(errors))
        sys.exit(f"FAIL: {len(errors)} problems")
    print("closure check: PASS (mechanical only; it cannot show the text is right)")


if __name__ == "__main__":
    main()
