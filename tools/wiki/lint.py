"""Lint the public wiki in docs/wiki against docs/wiki/SCHEMA.md.

  python tools/wiki/lint.py [--root DIR]

Exit code 0 when clean, 1 when any error is found.
"""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from repo_digest import ROOT, file_sha256  # noqa: E402

TYPES = {"concept": "concepts", "component": "components", "workflow": "workflows", "decision": "decisions",
         "evidence": "evidence", "question": "questions", "source": "sources", "dialogue": "dialogues"}
STATUSES = {"draft", "verified", "stale", "superseded"}
EVIDENCE = {"decision", "source", "fixture", "synthetic_geometry", "actual_windows_directx", "performance"}
REQUIRED = ("id", "type", "status", "visibility", "summary")
LOG_OPS = {"ingest", "query", "lint", "decision", "update", "review", "audit"}
SPECIAL = {"index.md", "log.md", "SCHEMA.md"}

PRIVATE_PATTERNS = [
    (re.compile(r"[A-Za-z]:\\(?:Users|Documents and Settings)\\", re.I), "Windows user path"),
    (re.compile(r"(?<![\w.])/(?:home|Users|root)/[\w.-]+"), "absolute home path"),
    (re.compile(r"%USERPROFILE%|\$HOME/", re.I), "user profile path"),
    (re.compile(r"[\w.+-]+@[\w-]+\.[\w.-]+"), "email address"),
    (re.compile(r"\b(?:sk-[A-Za-z0-9_-]{16,}|ghp_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}|xox[bp]-[A-Za-z0-9-]{10,})"), "credential-like token"),
    (re.compile(r"private:(?!P-(?:\d{4}|NNNN)\b)\S+"), "private reference not in private:P-NNNN form"),
]
CJK = re.compile(r"[぀-ヿ㐀-鿿가-힯]")
LINK = re.compile(r"\[[^\]]*\]\(([^)\s]+)\)")
FLOW_MAP = re.compile(r"^\s*-\s*\{(.*)\}\s*$")


def parse_scalar(value: str):
    value = value.strip()
    if value.startswith("[") and value.endswith("]"):
        inner = value[1:-1].strip()
        return [v.strip() for v in inner.split(",") if v.strip()] if inner else []
    if value in ("null", "~"):
        return None
    if re.fullmatch(r"-?\d{1,9}", value):
        return int(value)
    return value.strip("'\"")


def parse_front_matter(text: str):
    """Parse the restricted front matter described in SCHEMA.md; return (dict, body) or raise ValueError."""
    if not text.startswith("---\n"):
        raise ValueError("missing front matter")
    end = text.find("\n---\n", 4)
    if end < 0:
        raise ValueError("unterminated front matter")
    data, current = {}, None
    for raw in text[4:end].splitlines():
        if not raw.strip():
            continue
        m = FLOW_MAP.match(raw)
        if m and current is not None:
            item = {}
            for part in m.group(1).split(","):
                if ":" not in part:
                    raise ValueError(f"bad claim field: {part.strip()}")
                k, v = part.split(":", 1)
                item[k.strip()] = parse_scalar(v)
            data[current].append(item)
            continue
        if raw.startswith(" ") or ":" not in raw:
            raise ValueError(f"unsupported front matter line: {raw.strip()}")
        key, value = raw.split(":", 1)
        key = key.strip()
        if value.strip() == "":
            data[key], current = [], key
        else:
            data[key], current = parse_scalar(value), None
    return data, text[end + 5:]


def check_text(rel: str, text: str, errors: list) -> None:
    for rx, label in PRIVATE_PATTERNS:
        if rx.search(text):
            errors.append(f"{rel}: {label}")
    if CJK.search(text):
        errors.append(f"{rel}: non-English (CJK) text in a public page")


def check_links(rel: str, path: Path, body: str, wiki: Path, errors: list, linked: set) -> None:
    for target in LINK.findall(body):
        if re.match(r"^[a-z]+:", target) or target.startswith("#"):
            continue
        dest = (path.parent / target.split("#")[0]).resolve()
        if not dest.exists():
            errors.append(f"{rel}: broken link {target}")
        elif wiki.resolve() in dest.parents or dest == wiki.resolve():
            linked.add(dest)


def lint(wiki: Path, repo: Path = ROOT) -> list[str]:
    errors: list[str] = []
    ids: dict[str, str] = {}
    linked_from_index: set = set()
    linked_any: set = set()
    pages = sorted(p for p in wiki.rglob("*.md") if p.relative_to(wiki).as_posix() not in SPECIAL)
    for name in SPECIAL:
        if not (wiki / name).is_file():
            errors.append(f"missing {name}")
    for special in ("index.md", "log.md", "SCHEMA.md"):
        p = wiki / special
        if p.is_file():
            text = p.read_text(encoding="utf-8")
            check_text(special, text, errors)
            check_links(special, p, text, wiki, errors, linked_from_index if special == "index.md" else linked_any)
    log = wiki / "log.md"
    if log.is_file():
        for line in log.read_text(encoding="utf-8").splitlines():
            if line.startswith("## "):
                m = re.fullmatch(r"## \[(\d{4}-\d{2}-\d{2})\] (\w+) \| .+", line)
                if not m or m.group(2) not in LOG_OPS:
                    errors.append(f"log.md: bad entry heading: {line}")
    for page in pages:
        rel = page.relative_to(wiki).as_posix()
        text = page.read_text(encoding="utf-8")
        check_text(rel, text, errors)
        try:
            meta, body = parse_front_matter(text)
        except ValueError as exc:
            errors.append(f"{rel}: {exc}")
            continue
        for key in REQUIRED:
            if key not in meta:
                errors.append(f"{rel}: missing {key}")
        pid = meta.get("id")
        if pid != page.stem:
            errors.append(f"{rel}: id {pid!r} must equal the file name")
        if pid in ids:
            errors.append(f"{rel}: duplicate id (also {ids[pid]})")
        ids[pid] = rel
        ptype = meta.get("type")
        if ptype not in TYPES:
            errors.append(f"{rel}: unknown type {ptype!r}")
        elif page.parent.name != TYPES[ptype]:
            errors.append(f"{rel}: type {ptype} belongs in {TYPES[ptype]}/")
        if meta.get("status") not in STATUSES:
            errors.append(f"{rel}: unknown status {meta.get('status')!r}")
        if meta.get("visibility") != "public":
            errors.append(f"{rel}: visibility must be public")
        for claim in meta.get("claims", []) if isinstance(meta.get("claims"), list) else []:
            cid = claim.get("id", "?")
            if claim.get("evidence_kind") not in EVIDENCE:
                errors.append(f"{rel}: claim {cid} has unknown evidence_kind")
            cpath = claim.get("path")
            if cpath:
                target = (repo / str(cpath)).resolve()
                if repo.resolve() not in target.parents or not target.is_file():
                    errors.append(f"{rel}: claim {cid} cites missing file {cpath}")
                elif claim.get("sha256") and claim["sha256"] != file_sha256(target) and meta.get("status") != "stale":
                    errors.append(f"{rel}: claim {cid} digest changed for {cpath}; update the claim or mark the page stale")
        check_links(rel, page, body, wiki, errors, linked_any)
    for page in pages:
        if page.resolve() not in linked_from_index:
            errors.append(f"{page.relative_to(wiki).as_posix()}: not listed in index.md")
    return errors


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--root", type=Path, default=ROOT / "docs" / "wiki")
    args = parser.parse_args(argv)
    errors = lint(args.root)
    for e in errors:
        print(e)
    print("wiki: ok" if not errors else f"wiki: {len(errors)} error(s)")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
