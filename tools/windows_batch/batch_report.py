"""Sanitised public batch records, with raw evidence kept in the private root."""
from __future__ import annotations

import importlib.util
import json
import platform
import struct
import sys
import tempfile
from pathlib import Path

if __package__:
    from .batch_interface import ERROR, FAIL, INTERRUPTED, STATUSES, clip, sanitizer
else:
    from batch_interface import ERROR, FAIL, INTERRUPTED, STATUSES, clip, sanitizer


def load_file(path, name):
    """Load a checkout-owned file without adding its directory to sys.path."""
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    try:
        spec.loader.exec_module(module)
    except BaseException:
        sys.modules.pop(name, None)
        raise
    return module


TOOLS = Path(__file__).resolve().parents[1]
# The module clip() uses, so the cut and the publication apply the same rules.
_sanitizer = sanitizer()
_source_digest = load_file(TOOLS / "repo_digest.py", "_windows_batch_source_digest")


def source_identity():
    try:
        source = _source_digest.source_identity(timeout=30)
        return {"head": source["head"], "changed_paths": len(source["paths"]),
                "digest": source["digest"]}
    except Exception as exc:
        return {"error": clip(f"{type(exc).__name__}: {exc}")}


def machine(elevated):
    return {"os": platform.system(), "os_release": platform.release(),
            "os_build": platform.version(), "python": platform.python_version(),
            "python_bits": struct.calcsize("P") * 8, "elevated": elevated}


def outcome(steps):
    statuses = {step["status"] for step in steps}
    if INTERRUPTED in statuses:
        return "interrupted"
    return "fail" if statuses.intersection({FAIL, ERROR}) else "pass"


def _cell(value):
    return " ".join(str(value).split()).replace("|", "\\|")


def summary(record):
    source = record["source"]
    source_line = (f"`{source['head']}` ({source['changed_paths']} changed paths)"
                   if "head" in source else "unavailable: " + source["error"])
    host = record["machine"]
    # A pass with skipped steps must not read as a pass of every check.
    tally = ", ".join(f"{count} {status}" for status in STATUSES
                      if (count := sum(step["status"] == status for step in record["steps"])))
    lines = [f"# Windows batch {record['started_utc'][:10]}: {record['outcome']} ({tally})", "",
             "- Source commit: " + source_line,
             f"- Windows build: {host['os_build']} ({host['os_release']})",
             f"- Python: {host['python']} ({host['python_bits']}-bit)",
             "- Administrator: " + ("yes" if host["elevated"] else "no"),
             "- Start (UTC): " + record["started_utc"],
             "- End (UTC): " + record["finished_utc"], "",
             "Actual Windows on fresh synthetic data. Renderer numbers hold only for the probe "
             "build they name; the native self-test and the migration probes are not performance evidence.",
             "", "| Step | Status | Counts | Reason |", "| --- | --- | --- | --- |"]
    for step in record["steps"]:
        counts = ", ".join(f"{key}={value}" for key, value in sorted(step["counts"].items()))
        cells = [step["name"], step["status"], counts, step["reason"]]
        lines.append("| " + " | ".join(_cell(cell) for cell in cells) + " |")
    for step in record["steps"]:
        if step["lines"]:
            lines.extend(["", "## " + step["name"], ""])
            lines.extend("- " + line for line in step["lines"])
    lines.extend(["", "Private records: " + record["private_records"] +
                  "; raw records are not committed."])
    return "\n".join(lines) + "\n"


def _json(record):
    return json.dumps(record, indent=1, sort_keys=True) + "\n"


def _portable_paths(value):
    # JSON doubles backslashes; a placeholder path would then look like a UNC
    # path to leaks(text). Format these already-sanitised paths with slashes.
    if isinstance(value, str):
        return value.replace("\\", "/") if any(root in value for root in
            ("<repo>", "<scratch>", "<temp>", "<home>", "<path>")) else value
    if isinstance(value, dict):
        return {_portable_paths(key): _portable_paths(item) for key, item in value.items()}
    if isinstance(value, list):
        return [_portable_paths(item) for item in value]
    return value


def publish(record, repo, private, scratch, say):
    roots = {"<repo>": repo, "<scratch>": scratch, "<temp>": tempfile.gettempdir(),
             "<home>": Path.home()}
    clean = _portable_paths(_sanitizer.sanitize(record, roots))
    json_text = _json(clean)
    summary_text = summary(clean)
    found = set(_sanitizer.leaks(json_text))
    found.update(_sanitizer.leaks(summary_text))
    found.update(_sanitizer.leaks_in(clean))
    if found:
        (private / "batch-unpublished.json").write_text(_json(record), encoding="utf-8", newline="\n")
        say("Results not published: " + ", ".join(sorted(found)))
        return None
    base = repo / "work/windows-batch"
    base.mkdir(parents=True, exist_ok=True)
    date = record["started_utc"][:10]
    number = 1
    while True:
        directory = base / (date if number == 1 else f"{date}-{number}")
        try:
            directory.mkdir()
            break
        except FileExistsError:
            number += 1
    (directory / "batch.json").write_text(json_text, encoding="utf-8", newline="\n")
    (directory / "summary.md").write_text(summary_text, encoding="utf-8", newline="\n")
    return directory
