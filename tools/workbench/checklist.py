"""Progress checklist rules for docs/progress/status.json (schema 2).

Each step carries `items` ({id, title, weight, done, evidence}) and an optional
`weight`. A step's percentage is its done weight over its total weight; an item
counts as done only when `done` is true and its evidence has an accepted form.
A track's percentage is the step-weighted mean of its step percentages. Nothing
here estimates: no item, no percentage. Schema 1 documents have no checklist.

Pure functions, standard library only. `tools/workbench/statusline.py` mirrors
`step_percent` (it may not import repository code); tests compare both.
"""
from __future__ import annotations

import json
import re

ITEM_ID_RE = re.compile(r"^[a-z0-9][a-z0-9-]{0,39}$")
SHA_RE = re.compile(r"^[0-9a-f]{7,40}$")
PR_RE = re.compile(r"^https://github\.com/KonomiYuzu01/Magic-600-Cell/pull/[1-9][0-9]{0,5}$")
PATH_RE = re.compile(r"^[A-Za-z0-9_.-]+(?:/[A-Za-z0-9_.-]+)*$")
TRACKS = ("0.4.1", "stage-2")
STATUSES = ("not_started", "in_progress", "blocked", "done")


def evidence_kind(ref) -> str | None:
    """"sha", "pr" or "path" for an accepted evidence form, else None. Format only: whether
    the commit or the file exists is checked by progress.py and the tests."""
    if not isinstance(ref, str) or not ref or len(ref) > 200:
        return None
    if SHA_RE.fullmatch(ref):  # fullmatch: "$" alone accepts a trailing newline
        return "sha"
    if PR_RE.fullmatch(ref):
        return "pr"
    if PATH_RE.fullmatch(ref):
        parts = ref.split("/")  # not PurePosixPath: it drops "." parts
        if ".." not in parts and "." not in parts and parts[0].lower() != "work":  # Windows paths ignore case
            return "path"
    return None


def _positive(value) -> bool:
    return isinstance(value, int) and not isinstance(value, bool) and value > 0


def counted(item) -> bool:
    """An item counts as done only with `done` true and evidence of an accepted form."""
    return isinstance(item, dict) and item.get("done") is True and evidence_kind(item.get("evidence")) is not None


def step_percent(step) -> float | None:
    """Done weight over total weight in percent, or None when the step has no valid checklist."""
    items = step.get("items") if isinstance(step, dict) else None
    if not isinstance(items, list) or not items:
        return None
    total = done = 0
    for item in items:
        if not isinstance(item, dict) or not _positive(item.get("weight")):
            return None
        total += item["weight"]
        if counted(item):
            done += item["weight"]
    return 100.0 * done / total


def step_weight(step) -> int:
    w = step.get("weight", 1) if isinstance(step, dict) else 1
    return w if _positive(w) else 1


def track_percent(doc, track: str) -> float | None:
    """Step-weighted mean of the track's step percentages; None without a schema-2 checklist."""
    if not isinstance(doc, dict) or doc.get("schema") != 2 or not isinstance(doc.get("steps"), list):
        return None
    steps = [s for s in doc["steps"] if isinstance(s, dict) and s.get("track") == track]
    parts = [(step_percent(s), step_weight(s)) for s in steps]
    if not parts or any(p is None for p, _ in parts):
        return None
    return sum(p * w for p, w in parts) / sum(w for _, w in parts)


def remaining(step) -> list[dict]:
    """Items of `step` that do not count as done, in file order."""
    items = step.get("items") if isinstance(step, dict) else None
    if not isinstance(items, list):
        return []
    return [i for i in items if isinstance(i, dict) and not counted(i)]


def validate(doc) -> list[str]:
    """Cross-field rules of schema 2 that the JSON schema cannot express. The schema file
    (schemas/progress-status.schema.json) covers types and required fields."""
    problems: list[str] = []
    if not isinstance(doc, dict):
        return ["the document is not an object"]
    if doc.get("schema") != 2:
        return ["not a schema-2 document"]
    steps = doc.get("steps")
    if not isinstance(steps, list) or not steps:
        return ["no steps"]
    seen_steps = set()
    for s in steps:
        if not isinstance(s, dict) or not isinstance(s.get("id"), str):
            problems.append("a step is malformed")
            continue
        sid = s["id"]
        if sid in seen_steps:
            problems.append(f"{sid}: duplicate step id")
        seen_steps.add(sid)
        if s.get("track") not in TRACKS:
            problems.append(f"{sid}: unknown track")
        if s.get("status") not in STATUSES:
            problems.append(f"{sid}: unknown status")
        if "weight" in s and not _positive(s["weight"]):
            problems.append(f"{sid}: step weight must be a positive integer")
        items = s.get("items")
        if not isinstance(items, list) or not items:
            problems.append(f"{sid}: no checklist items")
            continue
        seen_items = set()
        for item in items:
            if not isinstance(item, dict):
                problems.append(f"{sid}: an item is malformed")
                continue
            iid = item.get("id")
            if not isinstance(iid, str) or not ITEM_ID_RE.fullmatch(iid):
                problems.append(f"{sid}: item id {iid!r} is invalid")
            elif iid in seen_items:
                problems.append(f"{sid}/{iid}: duplicate item id")
            seen_items.add(iid)
            if not isinstance(item.get("title"), str) or not item["title"].strip():
                problems.append(f"{sid}/{iid}: no title")
            if not _positive(item.get("weight")):
                problems.append(f"{sid}/{iid}: weight must be a positive integer")
            if not isinstance(item.get("done"), bool):
                problems.append(f"{sid}/{iid}: done must be true or false")
            if item.get("done") is True and evidence_kind(item.get("evidence")) is None:
                problems.append(f"{sid}/{iid}: done without accepted evidence")
        done = [i for i in items if isinstance(i, dict) and i.get("done") is True]
        if s.get("status") == "not_started" and done:
            problems.append(f"{sid}: not_started but has done items")
        if s.get("status") == "done" and len(done) != len(items):
            problems.append(f"{sid}: done but not every item is done")
    if doc.get("current") not in seen_steps:
        problems.append("current names no step")
    return problems


def dump(doc: dict) -> str:
    """The file layout of docs/progress/status.json: top-level fields one per line, one line per
    step and one line per item, so a checked item is a one-line diff. `steps` is written last."""
    head = [f"  {json.dumps(k)}: {json.dumps(v)}" for k, v in doc.items() if k != "steps"]
    steps = []
    for s in doc.get("steps", []):
        line = "    " + json.dumps({k: v for k, v in s.items() if k != "items"})
        if "items" in s:
            items = ",\n".join("      " + json.dumps(i) for i in s["items"])
            line = line[:-1] + ',\n     "items": [\n' + items + "\n     ]}"
        steps.append(line)
    return "{\n" + ",\n".join(head + ['  "steps": [\n' + ",\n".join(steps) + "\n  ]"]) + "\n}\n"
