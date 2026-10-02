"""Map the 0.4.1 screening findings to current source locations by symbol.

Usage:
  python docs/progress/1.0/inventory/screening_map.py           write screening-map.json
  python docs/progress/1.0/inventory/screening_map.py --check   verify screening-map.json is current

`docs/progress/0.4.1/screening-findings.md` gives coordinates at commit 9768913. For each finding this
script keeps its ID, severity, recorded status and historical locations, finds the innermost function, method or type
that encloses each historical line (Python by `ast` line ranges, C# by brace matching), and looks up the same symbol in the
current file. The current line range of that symbol is where the merge attaches the finding; a symbol
longer than 80 lines is narrowed to the historical offset +-15 lines and marked `narrowed`. A location
without a line, or a symbol that no longer exists, is kept with the reason it cannot be attached.
Standard library and `git show` only; nothing is written outside this folder.
"""
from __future__ import annotations

import ast
import json
import re
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
FINDINGS = ROOT / "docs/progress/0.4.1/screening-findings.md"
OUT = HERE / "screening-map.json"
BASE = "9768913"
NARROW_OVER = 80
WINDOW = 15
SHARDS = {"A": "core and persistence", "B": "process and HTTP", "C": "adapter", "D": "workflow modules",
          "E": "native renderer", "F": "experiment UI"}
CS_DEF = re.compile(r"^(\s*)(?:(?:public|private|internal|protected|static|override|async|sealed|readonly|unsafe|new|virtual|abstract|extern)\s+)*"
                    r"(?!(?:return|if|else|var|new|throw|foreach|for|while|using|switch|case|await|catch|lock)\b)"
                    r"(?:([A-Za-z_][\w<>\[\],.?]*(?:<[^()]*>)?)\s+)?(\w+)\s*\(")
CS_TYPE = re.compile(r"^\s*(?:(?:public|private|internal|protected|static|sealed|abstract|partial|readonly)\s+)*"
                     r"(?:class|struct|interface|record|enum)\s+(\w+)")


def git_show(rev: str, path: str) -> list[str] | None:
    r = subprocess.run(["git", "-C", str(ROOT), "show", f"{rev}:{path}"], capture_output=True)
    return r.stdout.decode("utf-8", "replace").splitlines() if r.returncode == 0 else None


def py_defs(lines: list[str]) -> list[tuple[int, int, str]]:
    """(start, end, qualified name) of every class and function, from the parser's own line ranges."""
    out = []

    def walk(node, prefix):
        for child in ast.iter_child_nodes(node):
            if isinstance(child, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
                name = prefix + child.name
                start = min([child.lineno] + [d.lineno for d in child.decorator_list])
                out.append((start, child.end_lineno, name))
                walk(child, name + ".")
            else:
                walk(child, prefix)

    walk(ast.parse("\n".join(lines) + "\n"), "")
    return out


def cs_block_end(lines: list[str], n: int) -> int | None:
    """Last line of the brace block (or `=>` expression) that starts at 1-based line `n`."""
    depth, opened, i = 0, False, n - 1
    in_block_comment = False
    while i < len(lines):
        line, j = lines[i], 0
        while j < len(line):
            two = line[j:j + 2]
            if in_block_comment:
                if two == "*/":
                    in_block_comment, j = False, j + 2
                    continue
                j += 1
                continue
            if two == "//":
                break
            if two == "/*":
                in_block_comment, j = True, j + 2
                continue
            ch = line[j]
            if ch in "\"'":
                verbatim = j > 0 and line[j - 1] == "@"
                j += 1
                while j < len(line) and line[j] != ch:
                    j += 2 if line[j] == "\\" and not verbatim else 1
                j += 1
                continue
            if ch == "{":
                depth, opened = depth + 1, True
            elif ch == "}":
                depth -= 1
                if opened and depth == 0:
                    return i + 1
            elif ch == ";" and not opened:
                return i + 1
            j += 1
        i += 1
    return None


def cs_defs(lines: list[str]) -> list[tuple[int, int, str]]:
    """(start, end, qualified name) of every C# type and method, with ends found by brace matching."""
    types = {m.group(1) for m in map(CS_TYPE.match, lines) if m}
    out = []
    for n, line in enumerate(lines, 1):
        t = CS_TYPE.match(line)
        m = None if t else CS_DEF.match(line)
        if m and m.group(2) is None and m.group(3) not in types:
            m = None  # no return type: only a constructor of a type in this file is a definition
        if not (t or m) or line.rstrip().endswith(";"):
            continue
        end = cs_block_end(lines, n)
        if end is not None:
            out.append((n, end, t.group(1) if t else m.group(3)))
    named = []
    for start, end, name in out:
        owners = [o for o in out if o[0] < start and o[1] >= end]
        named.append((start, end, ".".join([o[2] for o in owners] + [name])))
    return named


def symbols(lines: list[str], path: str) -> list[tuple[int, int, str]]:
    if path.endswith(".py"):
        return py_defs(lines)
    if path.endswith(".cs"):
        return cs_defs(lines)
    return []


def enclosing(lines: list[str], path: str, line: int) -> tuple[str, int, int] | None:
    """The innermost definition whose range holds `line`: (name, start, end)."""
    hits = [(s, e, name) for s, e, name in symbols(lines, path) if s <= line <= e]
    if not hits:
        return None
    s, e, name = max(hits, key=lambda h: (h[0], -h[1]))
    return name, s, e


def locate(lines: list[str], path: str, name: str) -> tuple[int, int] | None:
    hits = [(s, e) for s, e, n in symbols(lines, path) if n == name]
    return hits[0] if len(hits) == 1 else None


def parse_findings() -> list[dict]:
    text = FINDINGS.read_text(encoding="utf-8")
    fixed = set(re.findall(r"^\| ([A-Z]-\d+) \| Fixed", text, re.M))
    section, out = None, []
    for line in text.splitlines():
        if line.startswith("## "):
            section = line[3:].strip()
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if section == "Ranked confirmed findings" or section is None:
            pass
        if len(cells) >= 6 and re.fullmatch(r"\d+", cells[0]):
            fid = cells[1].split()[0]
            status = "fixed" if fid in fixed else "open"
            out.append({"id": fid, "severity": cells[2], "status": status, "summary": cells[3], "where": cells[4]})
        elif section == "Rejected with evidence" and len(cells) == 3 and re.fullmatch(r"[A-Z]-\d+", cells[0]):
            out.append({"id": cells[0], "severity": None, "status": "rejected", "summary": cells[1], "where": ""})
        elif section == "Needing verification" and len(cells) == 4 and re.fullmatch(r"[A-Z]-\d+", cells[0]):
            out.append({"id": cells[0], "severity": cells[1], "status": "needs verification", "summary": cells[2], "where": ""})
    return out


def locations(where: str) -> list[tuple[str, int | None]]:
    """`path:line` and `:line` (same path as before) references; a path without a line has line None."""
    out, last = [], None
    for ref in re.findall(r"`([^`]+)`", where):
        m = re.fullmatch(r"([^:]*)(?::(\d+))?", ref)
        if not m:
            continue
        path = m.group(1) or last
        if path is None:
            continue
        if "/" not in path and last and "/" in last and not (ROOT / path).exists():
            path = last.rsplit("/", 1)[0] + "/" + path  # a bare file name continues the previous directory
        last = path
        out.append((path, int(m.group(2)) if m.group(2) else None))
    return out


def build() -> dict:
    findings = []
    for f in parse_findings():
        entry = {k: f[k] for k in ("id", "severity", "status", "summary")}
        entry["screening_shard"] = SHARDS.get(f["id"][0], "integrator experiment")
        entry["historical"] = [f"{p}:{n}" if n else p for p, n in locations(f["where"])]
        entry["current"], entry["unattached"] = [], []
        if not entry["historical"]:
            entry["unattached"].append("no source location recorded in the screening")
        for path, line in locations(f["where"]):
            if line is None:
                entry["unattached"].append(f"{path}: no line recorded")
                continue
            old = git_show(BASE, path)
            cur_path = ROOT / path
            if old is None or not cur_path.is_file():
                entry["unattached"].append(f"{path}:{line}: file missing at {BASE} or now")
                continue
            sym = enclosing(old, path, line)
            if sym is None:
                entry["unattached"].append(f"{path}:{line}: no enclosing definition (module level)")
                continue
            cur = locate(cur_path.read_text(encoding="utf-8").splitlines(), path, sym[0])
            if cur is None:
                entry["unattached"].append(f"{path}:{line}: symbol {sym[0]} not found once in the current file")
                continue
            start, end, narrowed = cur[0], cur[1], False
            if end - start + 1 > NARROW_OVER:
                # A long symbol would attach the finding to every row inside it: keep the historical
                # offset within the symbol and a window around it, and say so.
                at = cur[0] + (line - sym[1])
                start, end, narrowed = max(cur[0], at - WINDOW), min(cur[1], at + WINDOW), True
            entry["current"].append({"path": path, "symbol": sym[0], "start": start, "end": end, "narrowed": narrowed})
        findings.append(entry)
    return {"format": "C600-SCREENING-MAP-draft", "base": BASE,
            "source": "docs/progress/0.4.1/screening-findings.md", "findings": findings}


def dump(doc: dict) -> str:
    return json.dumps(doc, indent=1, sort_keys=True, ensure_ascii=False) + "\n"


def main(argv: list[str]) -> int:
    text = dump(build())
    if argv == ["--check"]:
        ok = OUT.is_file() and OUT.read_text(encoding="utf-8") == text
        print("screening map: ok" if ok else "screening-map.json is stale; run screening_map.py")
        return 0 if ok else 1
    if argv:
        print(__doc__.strip().splitlines()[2].strip())
        return 2
    OUT.write_text(text, encoding="utf-8")
    doc = json.loads(text)
    print("screening map:", len(doc["findings"]), "findings,",
          sum(1 for f in doc["findings"] if f["current"]), "attached")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
