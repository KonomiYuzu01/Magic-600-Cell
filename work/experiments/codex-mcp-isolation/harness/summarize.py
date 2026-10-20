"""Summarize every case in a work directory as sanitized JSON (no machine paths) on stdout.

Per case: the wrapper's result, the flags after `exec`, whether the stand-in plugin MCP server started (from its own
start log) and under which parent, the direct children of `codex exec` (from process snapshots, matched by parent
PID), the backend requests the mock received, and what Codex put into the plugin cache."""
import collections
import json
import re
import sys
from pathlib import Path

work = Path(sys.argv[1]).resolve()


def clean(text: str) -> str:
    text = text.replace(str(work), "<work>")
    return re.sub(r"/\S*/(codex|codex-code-mode-host)\b", r"<bin>/\1", text)


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace") if path.exists() else ""


cases = []
for out in sorted(p for p in (work / "results").iterdir() if p.is_dir()):
    wrapper = [clean(line) for line in (read(out / "wrapper.out") + read(out / "wrapper.err")).splitlines() if line.strip()]
    exit_line = next((line for line in wrapper if line.startswith("wrapper exit ")), "wrapper exit ?")
    flags = []
    for line in read(out / "argv.jsonl").splitlines():
        argv = json.loads(line)
        if "exec" in argv:
            rest = argv[argv.index("exec") + 1:]
            flags = [f"{a} {b}" for a, b in zip(rest, rest[1:]) if a == "--disable"]
    procs = {}
    for line in read(out / "ps.log").splitlines():
        m = re.match(r"\s*(\d+)\s+(\d+)\s+(.*)$", line)
        if m:
            procs[int(m[1])] = (int(m[2]), m[3])
    codex_exec = {pid for pid, (_, args) in procs.items() if re.match(r"\S*/codex exec ", args)}
    children = sorted({clean(args) for pid, (ppid, args) in procs.items() if ppid in codex_exec})
    markers = [json.loads(line) for line in read(out / "markers.jsonl").splitlines()]
    started = [m for m in markers if m["event"] == "started"]
    requests = collections.Counter()
    for line in read(out / "requests.jsonl").splitlines():
        r = json.loads(line)
        requests[f"{r['method']} {r['path']} {r['status']}"] += 1
    cases.append({
        "case": out.name,
        "flags_after_exec": flags,
        "wrapper": {"exit": int(exit_line.split()[-1]) if exit_line.split()[-1].isdigit() else None,
                    "result": next((line for line in wrapper if not line.startswith("wrapper exit")), "")},
        "stand_in_mcp_server_started": bool(started),
        "stand_in_parent_is_codex_exec": bool(started) and all(m["ppid"] in codex_exec for m in started),
        "stand_in_requests": [m["method"] for m in markers if m["event"] == "request"],
        "stand_in_received_codex_home": any(m.get("codex_home_passed") for m in started),
        "stand_in_could_read_auth_json": any(m.get("auth_json_readable") for m in started),
        "codex_exec_children": children,
        "backend_requests": dict(sorted(requests.items())),
        "plugin_cache": sorted(line for line in read(out / "plugin-cache.txt").splitlines() if line.count("/") >= 3),
    })
print(json.dumps({"codex_version": read(work / "codex-version.txt").strip(), "cases": cases}, indent=2))
