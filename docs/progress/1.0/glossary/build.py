"""Build and check the glossary (docs/progress/1.0/glossary/) and the language learning path.

Usage:
  python docs/progress/1.0/glossary/build.py --render --browser <chromium> [--mmdc <path>]
                                     render new or changed diagrams/*.mmd to SVG with the local Mermaid CLI, then
                                     write the page
  python docs/progress/1.0/glossary/build.py          write glossary.md and the computed first-use order only
  python docs/progress/1.0/glossary/build.py --check  verify without writing (no browser needed)

`terms.json` is the editable source of the terms; `diagrams/<id>.mmd` is the editable source of each diagram;
`learning-path.json` holds the language learning path's steps and fixture tasks (EV5.6). `diagrams/manifest.json`
records, for every diagram, the SHA-256 of its source and of its rendered SVG, the Mermaid CLI version, the
configuration digest and the production command (EV5.15).

The term check (EV5.10, EV5.18) refuses:
- a term reference `[[Term]]` or `[[shown text|Term]]` in a definition that names no row (unknown term), or names
  more than one row (ambiguous term), such as a bare homonym word with no bare sense;
- a definition that refers to a term above its own level, or a learner-facing row that refers to a row marked
  `"learner_facing": false`;
- a citation in `source` without a registered source prefix (a document, a clause prefix, or a repository file or
  link that exists), such as a bare "Lemma 7", "M3" or "default 10";
- a homonym sense without its own row, and a bare homonym word in a definition where two senses can occur.
Nothing here repairs a reference by guessing a similar name: an unresolved reference fails.

The learning-path check refuses a step concept that is not a learner-facing row, a concept whose definition uses a
term no earlier step teaches (unless the manual or the solving glossary teaches it, or the step records the use with
its reason; a row's recorded level forward does not excuse it), prerequisites that do not form
a DAG, a step order that contradicts the manual's first-use order (owner, 23 November 2026), a fixture whose
recorded instance is missing, changed or lacks its expected value, and a fixture whose answer is a word or a macro
(the human-solve boundary: no fixture chooses or outputs a solving macro).

Standard library only; rendering needs the approved Mermaid CLI under tools/node/mermaid-cli. Files are written with
LF line endings.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
DIAGRAMS = HERE / "diagrams"
MANIFEST = DIAGRAMS / "manifest.json"
PAGE = HERE / "glossary.md"
TERMS = HERE / "terms.json"
PATH = HERE / "learning-path.json"
CONFIG = HERE / "mermaid.json"
MANUAL = ROOT / "research/jumbling/manual"
# The manual's chapters in reading order; its README previews terms before the chapters teach them.
MANUAL_ORDER = ("01-what-you-see.md", "02-descent.md", "03-tail.md", "04-lattice.md")
MMDC = ROOT / "tools/node/mermaid-cli/node_modules/.bin/mmdc"
FIELDS = ("id", "term", "level", "definition", "source")
COMMAND = "mmdc -q -p <puppeteer config> -c docs/progress/1.0/glossary/mermaid.json -i diagrams/<id>.mmd -o diagrams/<id>.svg"
REF = re.compile(r"\[\[([^\[\]|]+)(?:\|([^\[\]|]+))?\]\]")
LINK = re.compile(r"\[([^\]]*)\]\(([^)\s]*)\)")
ANSWER_KINDS = ("count", "evidence word", "digest", "yes or no", "list of objects", "value", "refusal code")


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path: Path, text: str) -> None:
    path.write_bytes(text.encode("utf-8"))


def load(path: Path = TERMS) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


# ------------------------------------------------------------------------------------------------ resolution


def levels_of(doc: dict) -> dict[str, int]:
    return {lv["id"]: i for i, lv in enumerate(doc.get("levels", []))}


def index(doc: dict) -> dict[str, list[str]]:
    """Every name a reference may use, in lower case, with the rows it names: each row's term and aliases, and a
    homonym word's bare sense when the homonym declares one."""
    names: dict[str, list[str]] = {}
    for t in doc.get("terms", []):
        for name in [t.get("term", "")] + list(t.get("aliases", [])):
            key = name.strip().lower()
            if key and t.get("id") not in names.setdefault(key, []):
                names[key].append(t.get("id"))
    by_term = {t.get("term", "").lower(): t.get("id") for t in doc.get("terms", [])}
    for h in doc.get("homonyms", []):
        key = h["word"].lower()
        bare = h.get("bare")
        if bare:
            names[key] = [by_term.get(bare.lower(), bare)]
        else:
            # A homonym word without a bare sense names every sense: a reference to it is ambiguous.
            names[key] = sorted({by_term.get(s.lower(), s) for s in h["senses"]} | set(names.get(key, [])))
    return names


def resolve(names: dict[str, list[str]], name: str) -> tuple[str | None, str | None]:
    """The one row a name names, or the reason it names none."""
    found = names.get(name.strip().lower(), [])
    if not found:
        return None, f"unknown term '{name}'"
    if len(found) > 1:
        return None, f"ambiguous term '{name}' (names {', '.join(sorted(found))}); name the sense"
    return found[0], None


def references(definition: str) -> list[tuple[str, str]]:
    """The (shown text, named term) pairs of a definition's references. `[[text|~]]` marks an ordinary word that
    names no term, such as a verb spelled like a homonym; it is no reference."""
    return [(m.group(1), m.group(2) or m.group(1)) for m in REF.finditer(definition) if m.group(2) != "~"]


def parts(source: str) -> list[str]:
    """A source's citations: split at semicolons outside parentheses and brackets."""
    out, depth, current = [], 0, ""
    for ch in source:
        depth += ch in "([" and 1 or ch in ")]" and -1 or 0
        if ch == ";" and depth == 0:
            out.append(current)
            current = ""
        else:
            current += ch
    return [p.strip() for p in out + [current]]


def plain(definition: str) -> str:
    return REF.sub(lambda m: m.group(1), definition)


# ------------------------------------------------------------------------------------------------ citations


def citation_errors(doc: dict, tid: str, source: str) -> list[str]:
    reg = doc.get("citations", {})
    documents = [d.lower() for d in reg.get("documents", [])]
    clause = re.compile(r"^(?:" + "|".join(re.escape(p) for p in reg.get("clauses", [])) + r")\.?\d")
    errors = []
    for part in parts(source):
        if not part:
            errors.append(f"{tid}: empty citation")
            continue
        links = LINK.findall(part)
        for _, target in links:
            path = target.split("#", 1)[0]
            if not path or not (HERE / path).resolve().is_file():
                errors.append(f"{tid}: citation link '{target}' names no file")
        if part.startswith("[") and links:
            continue
        head = re.split(r"[ ,(]", part, maxsplit=1)[0]
        if "/" in head or re.search(r"\.(py|md|json|tex|rs|gz)$", head):
            if not any((base / head).is_file() for base in (ROOT, HERE.parent)):
                errors.append(f"{tid}: citation '{part}' names a file that does not exist")
            continue
        if any(part.lower() == d or part.lower().startswith(d + " ") or part.lower().startswith(d + ",")
               for d in documents):
            continue
        if reg.get("clauses") and clause.match(part):
            continue
        errors.append(f"{tid}: ambiguous citation '{part}': it names no registered source prefix (default 19)")
    return errors


# ------------------------------------------------------------------------------------------------ the term check


def term_errors(doc: dict) -> list[str]:
    errors, seen = [], set()
    levels = levels_of(doc)
    names = index(doc)
    rows = {t.get("id"): t for t in doc.get("terms", [])}
    for t in doc.get("terms", []):
        tid = t.get("id", "?")
        for f in FIELDS:
            if not isinstance(t.get(f), str) or not t[f].strip():
                errors.append(f"{tid}: missing '{f}'")
        if tid in seen:
            errors.append(f"{tid}: duplicate id")
        seen.add(tid)
        if t.get("level") not in levels:
            errors.append(f"{tid}: unknown level {t.get('level')!r}")
        if not (DIAGRAMS / f"{tid}.mmd").is_file():
            errors.append(f"{tid}: no diagram source diagrams/{tid}.mmd")
        if "learner_facing" in t and not isinstance(t["learner_facing"], bool):
            errors.append(f"{tid}: 'learner_facing' must be true or false")
        facing = t.get("learner_facing", True)
        forward = {f.get("term", "").lower(): f for f in t.get("forward", [])}
        above = set()
        for shown, name in references(t.get("definition", "")):
            target, why = resolve(names, name)
            if why:
                errors.append(f"{tid}: {why}")
                continue
            if target == tid:
                errors.append(f"{tid}: refers to itself")
                continue
            used = rows.get(target, {})
            if levels.get(used.get("level"), 99) > levels.get(t.get("level"), -1):
                # A forward reference is refused unless the row records it, with the reason, for review.
                above.add(used.get("term", "").lower())
                if used.get("term", "").lower() not in forward:
                    errors.append(f"{tid}: its definition uses '{used.get('term')}' ({used.get('level')}), above "
                                  f"its own level {t.get('level')}")
            if facing and used.get("learner_facing", True) is False:
                errors.append(f"{tid}: a learner-facing definition uses '{used.get('term')}', which is not "
                              "learner-facing")
        for term, f in forward.items():
            if term not in above:
                errors.append(f"{tid}: records a forward reference to '{f.get('term')}' that its definition does "
                              "not make")
            if not str(f.get("why", "")).strip():
                errors.append(f"{tid}: the forward reference to '{f.get('term')}' gives no reason")
        errors += citation_errors(doc, tid, t.get("source", ""))
    lower = [t.get("term", "").lower() for t in doc.get("terms", [])]
    for n in sorted({n for n in lower if lower.count(n) > 1}):
        errors.append(f"term '{n}' is defined twice")
    errors += homonym_errors(doc)
    return errors


def homonym_errors(doc: dict) -> list[str]:
    """EV5.18: each sense has its own row, and a bare word is never used where two senses can occur."""
    errors = []
    terms = {t.get("term", "").lower(): t for t in doc.get("terms", [])}
    names = sorted({n.lower() for t in doc.get("terms", []) for n in [t.get("term", "")] + t.get("aliases", [])},
                   key=len, reverse=True)
    for h in doc.get("homonyms", []):
        for sense in h["senses"]:
            if sense.lower() not in terms:
                errors.append(f"homonym '{h['word']}': the sense '{sense}' has no row of its own")
        if h.get("bare") and h["bare"] not in h["senses"]:
            errors.append(f"homonym '{h['word']}': its bare sense '{h['bare']}' is not one of its senses")
        if h.get("bare"):
            continue
        bare = h["word"].lower()
        word = re.compile(r"(?<![\w-])" + re.escape(bare) + r"(?:s|es)?(?![\w])")
        # Longer names that contain the word (its senses and other terms such as "jumble twist") are not bare uses.
        longer = [re.compile(r"(?<![\w-])" + re.escape(n) + r"(?:s|es)?(?![\w-])") for n in names
                  if n != bare and re.search(r"(?<![\w-])" + re.escape(bare), n)]
        own = {s.lower() for s in h["senses"]}
        for t in doc.get("terms", []):
            if t.get("term", "").lower() in own:
                continue
            text = re.sub(r"`[^`]*`", " ", REF.sub(" ", t.get("definition", ""))).lower()
            for pattern in longer:
                text = pattern.sub(" ", text)
            if word.search(text):
                errors.append(f"{t.get('id')}: uses the bare word '{h['word']}', which has more than one sense; "
                              "mark the sense")
    return errors


# ------------------------------------------------------------------------------------------------ the learning path


def manual_lines() -> list[tuple[str, int, str, int, str]]:
    """The manual's lines in reading order, each with its file, line number, text, and the ordinal and heading of
    the section (a `#` or `##` heading) it sits in."""
    lines, section, heading = [], 0, ""
    for name in MANUAL_ORDER:
        path = MANUAL / name
        if not path.is_file():
            continue
        for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            if line.startswith("#") and not line.startswith("###"):
                section += 1
                heading = line.lstrip("#").strip()
            lines.append((name, number, line.lower(), section, heading))
    return lines


def manual_pattern(word: str) -> re.Pattern:
    return re.compile(r"(?<![\w-])" + re.escape(word.lower()) + r"(?![\w-])")


def first_use(doc: dict) -> list[dict]:
    """The manual's first-use order (owner, 23 November 2026): for each row that names the manual's words for it in
    `manual`, where the manual first uses one of them (its files in reading order, then the line). Only declared
    words count, so an everyday word such as "open" is never read as a term."""
    found = []
    for t in doc.get("terms", []):
        patterns = [manual_pattern(w) for w in t.get("manual", [])]
        for position, (name, number, line, section, heading) in enumerate(manual_lines()):
            if any(p.search(line) for p in patterns):
                found.append({"id": t["id"], "at": f"{name}:{number}", "section": heading, "position": position,
                              "ordinal": section})
                break
    found.sort(key=lambda f: (f["position"], f["id"]))
    return [{"id": f["id"], "at": f["at"], "section": f["section"]} for f in found]


def manual_sections(doc: dict) -> dict[str, int]:
    """Each manual-introduced row's section ordinal."""
    ordinal = {}
    lines = manual_lines()
    for t in doc.get("terms", []):
        patterns = [manual_pattern(w) for w in t.get("manual", [])]
        for line in lines:
            if any(p.search(line[2]) for p in patterns):
                ordinal[t["id"]] = line[3]
                break
    return ordinal


def manual_errors(doc: dict) -> list[str]:
    """Each declared manual word occurs in the manual, and a definition the manual introduces uses no term that the
    manual introduces in a later section: the first-use analogue of the level rule (EV5.14). A row may record such a
    use in `first_use_forward`, with the reason, for the mathematician's review."""
    errors = []
    text = "\n".join(line[2] for line in manual_lines())
    for t in doc.get("terms", []):
        for w in t.get("manual", []):
            if not manual_pattern(w).search(text):
                errors.append(f"{t['id']}: the manual never uses its word '{w}'")
    sections = manual_sections(doc)
    names = index(doc)
    rows = {t["id"]: t for t in doc.get("terms", [])}
    for t in doc.get("terms", []):
        recorded = {f.get("term", "").lower(): f for f in t.get("first_use_forward", [])}
        later = set()
        if t["id"] in sections:
            for _, name in references(t.get("definition", "")):
                used, _ = resolve(names, name)
                if used in sections and sections[used] > sections[t["id"]]:
                    later.add(rows[used]["term"].lower())
                    if rows[used]["term"].lower() not in recorded:
                        errors.append(f"{t['id']}: its definition uses '{rows[used]['term']}', which the manual "
                                      f"introduces in a later section than '{t['term']}'")
        for term, f in recorded.items():
            if term not in later:
                errors.append(f"{t['id']}: records a first-use forward reference to '{f.get('term')}' that does not "
                              "occur")
            if not str(f.get("why", "")).strip():
                errors.append(f"{t['id']}: the first-use forward reference to '{f.get('term')}' gives no reason")
    return errors


def json_pointer(doc, pointer: str):
    for part in [p for p in pointer.split("/") if p != ""]:
        part = part.replace("~1", "/").replace("~0", "~")
        doc = doc[int(part)] if isinstance(doc, list) else doc[part]
    return doc


def path_errors(doc: dict, path: dict) -> list[str]:
    errors = []
    names = index(doc)
    rows = {t["id"]: t for t in doc.get("terms", [])}
    steps = {s["id"]: s for s in path.get("steps", [])}
    if len(steps) != len(path.get("steps", [])):
        errors.append("learning path: duplicate step id")
    # Prerequisites form a DAG.
    listed = [s["id"] for s in path.get("steps", [])]
    for s in steps.values():
        for a in s.get("after", []):
            if a not in steps:
                errors.append(f"{s['id']}: comes after unknown step {a}")
            elif listed.index(a) > listed.index(s["id"]):
                errors.append(f"{s['id']}: comes after {a}, which is listed later; list the steps in their order")
    state: dict[str, int] = {}

    def visit(sid: str, trail: list[str]) -> None:
        if state.get(sid) == 2:
            return
        if state.get(sid) == 1:
            errors.append("learning path: the steps' prerequisites form a cycle: " + " -> ".join(trail + [sid]))
            return
        state[sid] = 1
        for a in steps[sid].get("after", []):
            if a in steps:
                visit(a, trail + [sid])
        state[sid] = 2

    for sid in steps:
        visit(sid, [])
    if any("cycle" in e for e in errors):
        return errors

    def ancestors(sid: str) -> set[str]:
        out, todo = set(), list(steps[sid].get("after", []))
        while todo:
            a = todo.pop()
            if a in steps and a not in out:
                out.add(a)
                todo += steps[a].get("after", [])
        return out

    taught: dict[str, str] = {}
    concepts: dict[str, list[str]] = {}
    for s in steps.values():
        ids = []
        # `concepts` are EV5.6's; `closure` adds the terms their definitions need, so that each step's concepts are
        # taught before its task (CT-EV5-02).
        for name in s.get("concepts", []) + s.get("closure", []):
            tid, why = resolve(names, name)
            if why:
                errors.append(f"{s['id']}: {why}")
                continue
            if rows[tid].get("learner_facing", True) is False:
                errors.append(f"{s['id']}: '{name}' is not learner-facing and cannot be taught")
            if tid in taught:
                errors.append(f"{s['id']}: '{name}' is already taught at {taught[tid]}")
            taught.setdefault(tid, s["id"])
            ids.append(tid)
        concepts[s["id"]] = ids
    for s in steps.values():
        for w in s.get("waiting", []):
            tid, why = resolve(names, w.get("term", ""))
            if why:
                errors.append(f"{s['id']} waiting: {why}")
            elif not str(w.get("why", "")).strip():
                errors.append(f"{s['id']}: '{w.get('term')}' waits without a reason")
            elif tid in taught:
                errors.append(f"{s['id']}: '{w.get('term')}' waits but is taught at {taught[tid]}")
    order = first_use(doc)
    used_by_manual = {f["id"] for f in order}
    # The terms the path assumes: the solving glossary's own terms, and terms the manual uses (owner, 23 November
    # 2026: first use is the manual). Each is named, so nothing is assumed by accident.
    outside = set()
    for name in path.get("assumed", []):
        tid, why = resolve(names, name)
        if why:
            errors.append(f"assumed: {why}")
        elif rows[tid].get("origin") != "M6" and tid not in used_by_manual:
            errors.append(f"assumed: '{name}' is neither a solving-glossary term nor used by the manual")
        elif tid in taught:
            errors.append(f"assumed: '{name}' is also taught at {taught[tid]}")
        else:
            outside.add(tid)
    # A step revisits a concept an earlier step taught, or one the path takes from the manual (step 4 revisits Word's
    # named alphabet and identity, since the manual's chapter 1 shows jumbling words only).
    for s in steps.values():
        for name in s.get("revisits", []):
            tid, why = resolve(names, name)
            if why:
                errors.append(f"{s['id']} revisits: {why}")
            elif taught.get(tid) not in ancestors(s["id"]) and tid not in outside:
                errors.append(f"{s['id']} revisits '{name}', which no earlier step teaches")
    for sid, ids in concepts.items():
        known = set(ids)
        for a in ancestors(sid):
            known |= set(concepts.get(a, []))
        # A step may record that a concept uses a term a later step teaches (EV5.6 reads two record parts at step 1
        # and the whole record at step 5). A row's recorded forward reference to a higher level excuses the level rule
        # only: the path still teaches the term it uses first, or the step records the use with its reason.
        planned = {(f.get("concept", "").lower(), f.get("uses", "").lower()): f for f in steps[sid].get("forward", [])}
        seen = set()
        for tid in ids:
            for _, name in references(rows[tid]["definition"]):
                used, _ = resolve(names, name)
                if not used or used in known or used in outside:
                    continue
                key = (rows[tid]["term"].lower(), rows[used]["term"].lower())
                if key in planned:
                    seen.add(key)
                    continue
                errors.append(f"{sid}: '{rows[tid]['term']}' uses '{rows[used]['term']}', which no step up to "
                              f"{sid} teaches")
        for key, f in planned.items():
            if key not in seen:
                errors.append(f"{sid}: records that '{f.get('concept')}' uses '{f.get('uses')}' later, which does "
                              "not occur")
            if not str(f.get("why", "")).strip():
                errors.append(f"{sid}: the planned forward use of '{f.get('uses')}' gives no reason")
    # The manual's first-use order: a term the manual introduces earlier is not taught at a later step.
    rank = {f["id"]: i for i, f in enumerate(order)}
    place = {s["id"]: i for i, s in enumerate(path.get("steps", []))}
    for a in taught:
        for b in taught:
            if a in rank and b in rank and rank[a] < rank[b] and place[taught[a]] > place[taught[b]]:
                errors.append(f"'{rows[a]['term']}' comes before '{rows[b]['term']}' in the manual but is taught "
                              f"at {taught[a]}, after {taught[b]}")
    if path.get("first_use") != order:
        errors.append("learning-path.json: the first-use order differs from the manual; run build.py")
    # Fixtures read recorded instances only, and no answer is a word or a macro.
    for s in steps.values():
        fixture = s.get("fixture")
        if not fixture:
            errors.append(f"{s['id']}: no fixture task")
            continue
        known = set(concepts[s["id"]])
        for a in ancestors(s["id"]):
            known |= set(concepts.get(a, []))
        for name in fixture.get("concepts", []):
            tid, why = resolve(names, name)
            if why:
                errors.append(f"{s['id']} fixture: {why}")
            elif tid not in known and tid not in outside:
                errors.append(f"{s['id']} fixture: '{name}' is not taught before the task")
        kind = fixture.get("answer", {}).get("kind")
        if kind not in ANSWER_KINDS:
            errors.append(f"{s['id']} fixture: answer kind {kind!r} is not one of {', '.join(ANSWER_KINDS)}; a "
                          "fixture never chooses or outputs a word or a macro")
        for read in fixture.get("reads", []):
            target = ROOT / read["path"]
            if ".." in Path(read["path"]).parts or not target.is_file():
                errors.append(f"{s['id']} fixture: the recorded instance {read['path']} does not exist")
                continue
            # A pinned instance is read at its recorded digest. A live one (a draft table other work edits) says why,
            # and its expectations select rows by ID instead of position.
            if "live" in read:
                if not str(read["live"]).strip() or "sha256" in read:
                    errors.append(f"{s['id']} fixture: {read['path']} is live without a reason, or also pinned")
            elif sha(target) != read.get("sha256"):
                errors.append(f"{s['id']} fixture: {read['path']} differs from its recorded digest")
                continue
            for expect in read.get("expect", []):
                if "pointer" in expect:
                    try:
                        value = json_pointer(load(target) if target.suffix == ".json" else None, expect["pointer"])
                        if "where" in expect:
                            value = next(v for v in value
                                         if all(v.get(k) == w for k, w in expect["where"].items()))
                        if "field" in expect:
                            value = value[expect["field"]]
                    except (KeyError, IndexError, TypeError, ValueError, StopIteration, AttributeError):
                        errors.append(f"{s['id']} fixture: {read['path']} has no {expect['pointer']} "
                                      f"{expect.get('where', '')} {expect.get('field', '')}")
                        continue
                    if value != expect["value"]:
                        errors.append(f"{s['id']} fixture: {read['path']}{expect['pointer']} "
                                      f"{expect.get('field', '')} is {value!r}, not {expect['value']!r}")
                if "text" in expect and expect["text"] not in target.read_text(encoding="utf-8"):
                    errors.append(f"{s['id']} fixture: {read['path']} does not contain {expect['text']!r}")
    return errors


# ------------------------------------------------------------------------------------------------ the page


def anchor(tid: str) -> str:
    return f'<a id="{tid}"></a>'


def linked(doc: dict, text: str) -> str:
    names = index(doc)

    def one(m: re.Match) -> str:
        if m.group(2) == "~":
            return m.group(1)
        target, _ = resolve(names, m.group(2) or m.group(1))
        return f"[{m.group(1)}](#{target})" if target else m.group(0)

    return REF.sub(one, text)


def page(doc: dict, path: dict | None = None) -> str:
    out = ["# Glossary (M6)", "",
           "Status: **draft for stage 2.** Generated by `build.py` from `terms.json` and `learning-path.json`; edit the "
           "sources, not this page. One term per concept (charter M1). Each term enters at the learning level where it is "
           "first needed (charter M6, D7). The levels follow the steps of the [solving workflow](../solving-workflow.md) "
           "and are provisional until the owner decides the learning path; the UI locations are the planned surfaces of "
           "solving-workflow section 4, not a layout. Diagrams are rendered locally with the Mermaid CLI; sources and "
           "digests are in `diagrams/`. The language's entries come from chapter 04 section 6 of the language "
           "architecture (EV5.14, EV5.15, EV5.18). The mathematician reviewed every entry, step and key on 24 November "
           "2026 (EV5.14); each entry's review line says what they found.", ""]
    out += ["## Levels", "", "| Level | Name | Workflow step |", "|---|---|---|"]
    out += [f"| {lv['id']} | {lv['name']} | {lv['workflow']} |" for lv in doc["levels"]]
    if doc.get("homonyms"):
        out += ["", "## Words with more than one sense (EV5.18)", "", "| Word | Senses | Bare word means |",
                "|---|---|---|"]
        terms = {t["term"].lower(): t["id"] for t in doc["terms"]}
        for h in doc["homonyms"]:
            senses = ", ".join(f"[{s}](#{terms.get(s.lower(), '')})" for s in h["senses"])
            out.append(f"| {h['word']} | {senses} | {h.get('bare') or 'never used bare'} |")
    taught = {}
    if path:
        names = index(doc)
        for s in path.get("steps", []):
            for name in s.get("concepts", []) + s.get("closure", []):
                tid, _ = resolve(names, name)
                if tid:
                    taught.setdefault(tid, s["id"])
    for lv in doc["levels"]:
        out += ["", f"## {lv['id']}: {lv['name']}"]
        for t in sorted((t for t in doc["terms"] if t["level"] == lv["id"]), key=lambda t: t["term"].lower()):
            out += ["", f"### {anchor(t['id'])}{t['term']}", "", linked(doc, t["definition"]), "",
                    f"![{t['term']}](diagrams/{t['id']}.svg)", "",
                    f"- First needed: {lv['id']} ({lv['name']})"]
            if t.get("learner_facing", True) is False:
                out.append("- Not learner-facing")
            out.append(f"- In the UI: {t['ui']}" if t.get("ui") else "- In the UI: not placed yet")
            if t["id"] in taught:
                out.append(f"- Taught at learning-path step {taught[t['id']]}")
            out.append(f"- Source: {t['source']}")
            for f in t.get("forward", []):
                out.append(f"- Forward reference to {f['term']}: {f['why']}")
            for f in t.get("first_use_forward", []):
                out.append(f"- Uses {f['term']} before the manual introduces it: {f['why']}")
            if t.get("review"):
                out.append(f"- Review: {t['review']}")
    if path:
        out += ["", "## Learning path (EV5.6)", "", path.get("note", ""), "",
                "Assumed from the manual and the solving glossary: " + ", ".join(path.get("assumed", [])) + ".", ""]
        for s in path["steps"]:
            after = ", ".join(s.get("after", [])) or "none"
            out += [f"### {s['id']}: {s['name']}", "", f"- After: {after}",
                    "- Concepts: " + ", ".join(s["concepts"])]
            if s.get("closure"):
                out.append("- Also taught, because the concepts' definitions use them: " + ", ".join(s["closure"]))
            if s.get("revisits"):
                out.append("- Revisits: " + ", ".join(s["revisits"]))
            for w in s.get("waiting", []):
                out.append(f"- Waiting: {w['term']}: {w['why']}")
            for f in s.get("forward", []):
                out.append(f"- Used before it is taught: {f['concept']} uses {f['uses']}: {f['why']}")
            if s.get("note"):
                out.append(f"- Note: {s['note']}")
            out.append("- Command routes: " + s.get("routes", ""))
            f = s.get("fixture", {})
            if f:
                out += [f"- Fixture task: {f.get('task', '')}",
                        f"- Answer: {f.get('answer', {}).get('kind', '')}: {f.get('answer', {}).get('expected', '')}",
                        "- Reads: " + ", ".join(f"`{r['path']}`" for r in f.get("reads", []))]
            if s.get("review"):
                out.append(f"- Review: {s['review']}")
            out.append("")
        out += ["### The manual's first-use order", "",
                "Where the manual's chapters first use each term they introduce, in reading order (computed by "
                "`build.py` from each row's `manual` words; owner, 23 November 2026):", ""]
        rows = {t["id"]: t["term"] for t in doc["terms"]}
        out += [f"{i}. [{rows.get(f['id'], f['id'])}](#{f['id']}): `{f['at']}`, {f['section']}"
                for i, f in enumerate(path.get("first_use", []), 1)]
    return "\n".join(out).rstrip("\n") + "\n"


# ------------------------------------------------------------------------------------------------ rendering


def render(doc: dict, browser: str, mmdc: Path) -> None:
    version = subprocess.run([str(mmdc), "--version"], capture_output=True, text=True, check=True).stdout.strip()
    try:
        old = json.loads(MANIFEST.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        old = {}
    same = old.get("version") == version and old.get("config_sha256") == sha(CONFIG)
    entries = {}
    with tempfile.TemporaryDirectory() as tmp:
        cfg = Path(tmp) / "puppeteer.json"
        cfg.write_text(json.dumps({"executablePath": browser, "args": ["--no-sandbox"]}), encoding="utf-8")
        for t in doc["terms"]:
            src, svg = DIAGRAMS / f"{t['id']}.mmd", DIAGRAMS / f"{t['id']}.svg"
            known = old.get("diagrams", {}).get(t["id"], {})
            if same and svg.is_file() and known.get("source_sha256") == sha(src) \
                    and known.get("svg_sha256") == sha(svg):
                entries[t["id"]] = known
                continue
            subprocess.run([str(mmdc), "-q", "-p", str(cfg), "-c", str(CONFIG), "-i", str(src), "-o", str(svg)],
                           check=True)
            entries[t["id"]] = {"source_sha256": sha(src), "svg_sha256": sha(svg)}
    for stale in DIAGRAMS.glob("*.svg"):
        if stale.stem not in entries:
            stale.unlink()
    write(MANIFEST, json.dumps({"tool": "@mermaid-js/mermaid-cli", "version": version, "config_sha256": sha(CONFIG),
                                "command": COMMAND, "diagrams": entries}, indent=1, sort_keys=True) + "\n")


def manifest_errors(doc: dict) -> list[str]:
    errors = []
    try:
        man = json.loads(MANIFEST.read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        return [f"manifest: cannot read ({exc})"]
    for field in ("tool", "version", "command"):
        if not man.get(field):
            errors.append(f"manifest: no {field}")
    if man.get("config_sha256") != sha(CONFIG):
        errors.append("manifest: mermaid.json changed since rendering")
    ids = {t.get("id") for t in doc.get("terms", [])}
    if set(man.get("diagrams", {})) != ids:
        errors.append("manifest: diagram set differs from the terms")
    for tid, e in man.get("diagrams", {}).items():
        src, svg = DIAGRAMS / f"{tid}.mmd", DIAGRAMS / f"{tid}.svg"
        if not src.is_file() or sha(src) != e.get("source_sha256"):
            errors.append(f"{tid}: diagram source changed since rendering")
        if not svg.is_file() or sha(svg) != e.get("svg_sha256"):
            errors.append(f"{tid}: SVG missing or differs from the manifest")
    return errors


def check(doc: dict, path: dict | None) -> list[str]:
    errors = term_errors(doc) + manual_errors(doc) + manifest_errors(doc)
    if path is not None:
        errors += path_errors(doc, path)
    if not PAGE.is_file() or PAGE.read_text(encoding="utf-8") != page(doc, path):
        errors.append("glossary.md is not the output of terms.json and learning-path.json; run build.py")
    return errors


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--render", action="store_true")
    ap.add_argument("--browser")
    ap.add_argument("--mmdc", default=str(MMDC))
    ap.add_argument("--check", action="store_true")
    a = ap.parse_args(argv)
    doc = load()
    path = load(PATH) if PATH.is_file() else None
    if a.check:
        errors = check(doc, path)
        for e in errors:
            print(e)
        if not errors:
            steps = len(path["steps"]) if path else 0
            print(f"glossary: ok ({len(doc['terms'])} terms, {steps} learning-path steps)")
        return 1 if errors else 0
    errors = term_errors(doc)
    if errors:
        print("\n".join(errors))
        return 1
    if path is not None:
        path["first_use"] = first_use(doc)
        write(PATH, json.dumps(path, indent=1, ensure_ascii=False) + "\n")
    if a.render:
        if not a.browser:
            ap.error("--render needs --browser <path to a local Chromium>")
        render(doc, a.browser, Path(a.mmdc))
    write(PAGE, page(doc, path))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
