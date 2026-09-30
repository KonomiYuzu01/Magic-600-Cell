"""Fixed workbench run plans, detached startup, and bounded local run readers."""
from __future__ import annotations

import hashlib
import json
import os
import re
import stat
import subprocess
import sys
import time
import uuid
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path

try:
    from . import paths
except ImportError:
    import paths

CODEX_KINDS = ("plan", "review", "implement")
CODEX_MODELS = ("gpt-6.1-sol", "gpt-6-astra")
CODEX_EFFORTS = ("high", "max", "ultra")
CODEX_SPEEDS = ("standard", "fast")
CODEX_MODEL_TIMEOUT = 3600
CODEX_ACCEPTANCE_TIMEOUT = 1800
CODEX_OVERHEAD = 1800
PACKET_RE = r"^[A-Za-z0-9][A-Za-z0-9._-]{0,99}\.md$"
RID_RE = r"^\d{8}T\d{6}Z-[0-9a-f]{8}$"
ID_RE = r"^[a-z0-9][a-z0-9-]{0,47}$"
REGISTRY_KEYS = {"schema", "runs"}
ENTRY_KEYS = {"id", "title", "steps", "cwd", "inputs", "outputs", "timeout_s", "windows_required"}
JSON_LIMIT = 256 * 1024
VENV_RE = r"^tools/\.venv/[a-z0-9-]+/(?:Scripts/python\.exe|bin/python)$"


class RunRefused(Exception):
    pass


@dataclass
class Registry:
    entries: list[dict]
    problems: list[str]
    sha256: str | None


def codex_deadline(kind: str) -> int:
    return CODEX_MODEL_TIMEOUT + (CODEX_ACCEPTANCE_TIMEOUT if kind == "implement" else 0) + CODEX_OVERHEAD


def entry_digest(entry: dict) -> str:
    return hashlib.sha256(json.dumps(entry, sort_keys=True, separators=(",", ":")).encode("utf-8")).hexdigest()


def _relative(value, *, work: bool = True) -> bool:
    if not isinstance(value, str) or not value or "\0" in value or value.startswith(("/", "\\")):
        return False
    parts = re.split(r"[/\\]", value)
    if ".." in parts or any(re.match(r"^[A-Za-z]:", p) for p in parts):
        return False
    parts = [p for p in parts if p not in ("", ".")]
    first = parts[0].lower() if parts else ""
    return first != ".git" and (work or first != "work")


def _python(exe: str) -> bool:
    return exe == "python" or re.fullmatch(VENV_RE, exe) is not None


def _validate_entry(entry, where: str, problems: list[str], ids: set[str]) -> None:
    def bad(reason):
        problems.append(f"{where}: {reason}")

    if not isinstance(entry, dict):
        bad("expected an object")
        return
    if set(entry) != ENTRY_KEYS:
        bad(f"missing keys {sorted(ENTRY_KEYS - set(entry))}; unknown keys {sorted(set(entry) - ENTRY_KEYS)}")
    rid = entry.get("id")
    if not isinstance(rid, str) or re.fullmatch(ID_RE, rid) is None or rid.startswith("codex-"):
        bad("invalid id (codex- is reserved)")
    elif rid in ids:
        bad("duplicate id")
    else:
        ids.add(rid)
    title = entry.get("title")
    if not isinstance(title, str) or not 1 <= len(title) <= 120:
        bad("title must be a non-empty string of at most 120 characters")
    if not _relative(entry.get("cwd"), work=False):
        bad("cwd must be a repository-relative path outside .git and work")
    for field in ("inputs", "outputs"):
        values = entry.get(field)
        if not isinstance(values, list) or len(values) > 50 or any(not _relative(v) for v in values):
            bad(f"{field} must contain at most 50 repository-relative globs")
    timeout = entry.get("timeout_s")
    if type(timeout) is not int or not 10 <= timeout <= 14400:
        bad("timeout_s must be an integer from 10 to 14400")
    if type(entry.get("windows_required")) is not bool:
        bad("windows_required must be a boolean")
    steps = entry.get("steps")
    if not isinstance(steps, list) or not 1 <= len(steps) <= 10:
        bad("steps must contain 1 to 10 argv lists")
        return
    for i, argv in enumerate(steps):
        if (not isinstance(argv, list) or not 1 <= len(argv) <= 32
                or any(not isinstance(s, str) or len(s) > 512 or "\0" in s for s in argv)):
            bad(f"step {i + 1}: expected 1 to 32 strings, at most 512 characters each, without NUL")
            continue
        exe = argv[0]
        if _python(exe):
            if len(argv) < 2 or not _relative(argv[1], work=False) or not argv[1].endswith(".py") or argv[1].startswith("-"):
                bad(f"step {i + 1}: Python requires a repository-relative .py script as its first argument")
        elif exe == "git":
            if len(argv) < 2 or argv[1] not in ("status", "diff", "log", "show"):
                bad(f"step {i + 1}: Git requires status, diff, log or show")
        else:
            bad(f"step {i + 1}: executable is not allowed")


def load_registry(main: Path) -> Registry:
    try:
        with (Path(main) / "tools" / "workbench" / "runs.json").open("rb") as f:
            raw = f.read(JSON_LIMIT + 1)
            digest = hashlib.sha256(raw)
            if len(raw) > JSON_LIMIT:
                # Preserve the file digest without retaining an oversized document.
                while chunk := f.read(1 << 20):
                    digest.update(chunk)
                return Registry([], ["registry is larger than 256 KiB"], digest.hexdigest())
    except OSError as exc:
        return Registry([], [f"cannot read the run registry: {exc}"], None)
    sha = digest.hexdigest()
    try:
        doc = json.loads(raw)
    except (ValueError, UnicodeError, RecursionError) as exc:
        return Registry([], [f"invalid registry JSON: {exc}"], sha)
    problems = []
    if not isinstance(doc, dict):
        return Registry([], ["registry must be an object"], sha)
    if set(doc) != REGISTRY_KEYS:
        problems.append("registry must have exactly schema and runs")
    if type(doc.get("schema")) is not int or doc["schema"] != 1:
        problems.append("registry schema must be 1")
    entries = doc.get("runs")
    if not isinstance(entries, list):
        problems.append("registry runs must be a list")
        entries = []
    ids = set()
    for i, entry in enumerate(entries):
        _validate_entry(entry, f"runs[{i}]", problems, ids)
    return Registry([] if problems else entries, problems, sha)


def _checkout(main: Path, checkout: Path) -> Path:
    checkout = Path(checkout).resolve()
    known = {os.path.normcase(str(p.resolve())) for p in paths.checkouts(main)}
    if os.path.normcase(str(checkout)) not in known:
        raise RunRefused("checkout is not a registered checkout of this repository")
    return checkout


def _path(checkout: Path, value: str) -> Path:
    p = checkout.joinpath(*re.split(r"[/\\]", value)).resolve()
    if not paths.inside(p, [checkout]):
        raise RunRefused(f"path is outside the checkout: {value}")
    return p


def _on_path(exe: str) -> Path:
    """The first native `exe` on PATH. Windows App Execution Aliases are skipped: Windows starts them
    outside the runner's job. Unlike shutil.which, the current directory is never searched."""
    name = exe + ".exe" if os.name == "nt" else exe
    aliases = False
    for directory in os.environ.get("PATH", "").split(os.pathsep):
        if not directory or not os.path.isabs(directory):
            continue
        candidate = Path(directory) / name
        try:
            if os.name == "nt" and candidate.lstat().st_reparse_tag == stat.IO_REPARSE_TAG_APPEXECLINK:
                aliases = True
                continue
        except OSError:
            continue
        if candidate.is_file() and (os.name == "nt" or os.access(candidate, os.X_OK)):
            return candidate.resolve()
    if aliases:
        raise RunRefused(f"{exe} on PATH is only a Windows App Execution Alias, which cannot inherit the runner's job; "
                         f"put a native {name} on PATH")
    raise RunRefused(f"executable not found: {exe}")


def step_path() -> str:
    """PATH for a step: the directories of the native python and git that the plans resolve come first, so a
    descendant that starts bare `python` or `git` (the wrapper's acceptance check, a Git hook, cmd.exe) finds them
    before a Windows App Execution Alias, which Windows would start outside the step's job."""
    first = []
    for exe in ("python", "git"):
        try:
            directory = str(_on_path(exe).parent)
        except RunRefused:
            continue
        if directory not in first:
            first.append(directory)
    return os.pathsep.join([*first, os.environ.get("PATH", "")])


def _executable(checkout: Path, exe: str) -> str:
    if exe in ("python", "git"):
        p = _on_path(exe)
    else:
        platform_part = "/Scripts/python.exe" if os.name == "nt" else "/bin/python"
        if not exe.endswith(platform_part):
            raise RunRefused("venv interpreter is for a different operating system")
        p = _path(checkout, exe)
    if not p.is_file():
        raise RunRefused(f"executable not found: {exe}")
    if os.name == "nt" and p.suffix.lower() != ".exe":
        raise RunRefused("Windows executables must resolve to .exe files")
    if os.name == "nt" and p.lstat().st_reparse_tag == stat.IO_REPARSE_TAG_APPEXECLINK:
        raise RunRefused("Windows App Execution Aliases cannot inherit the runner's job; use a native .exe")
    return str(p)


def plan_run(main: Path, checkout: Path, run_id: str) -> dict:
    registry = load_registry(main)
    if registry.problems:
        raise RunRefused("; ".join(registry.problems))
    entry = next((e for e in registry.entries if e["id"] == run_id), None)
    if entry is None:
        raise RunRefused(f"unknown run id: {run_id}")
    checkout = _checkout(main, checkout)
    if entry["windows_required"] and os.name != "nt":
        raise RunRefused("this run requires Windows")
    cwd = _path(checkout, entry["cwd"])
    if not cwd.is_dir():
        raise RunRefused("cwd is not an existing directory inside the checkout")
    steps, display = [], []
    for argv in entry["steps"]:
        step = [_executable(checkout, argv[0]), *argv[1:]]
        if _python(argv[0]):
            script = _path(checkout, argv[1])
            if not script.is_file():
                raise RunRefused(f"script not found: {argv[1]}")
            step[1] = str(script)
        steps.append(step)
        display.append([argv[0], *step[1:]])
    return {"kind": "registry", "run": run_id, "title": entry["title"], "checkout": str(checkout),
            "cwd": str(cwd), "steps": steps, "display": display, "timeout_s": entry["timeout_s"],
            "digest": entry_digest(entry), "windows_required": entry["windows_required"]}


def plan_codex(main: Path, checkout: Path, kind: str, packet: str, model: str, effort: str, speed: str) -> dict:
    for value, allowed, field in ((kind, CODEX_KINDS, "kind"), (model, CODEX_MODELS, "model"),
                                  (effort, CODEX_EFFORTS, "effort"), (speed, CODEX_SPEEDS, "speed")):
        if value not in allowed:
            raise RunRefused(f"Codex {field} is not allowed")
    checkout = _checkout(main, checkout)
    if not isinstance(packet, str) or re.fullmatch(PACKET_RE, packet) is None:
        raise RunRefused("packet must be a bare .md file name")
    if paths.packet_file(checkout, packet) is None:
        raise RunRefused("packet is missing or outside the packet directory")
    wrapper = _path(checkout, "tools/agents/codex_review.py")
    if not wrapper.is_file():
        raise RunRefused("Codex review wrapper is missing")
    args = ["tools/agents/codex_review.py", "--kind", kind, "--packet", "work/reviews/packets/" + packet,
            "--model", model, "--effort", effort, "--speed", speed]
    return {"kind": "codex", "run": "codex-" + kind, "title": "Codex " + kind,
            "checkout": str(checkout), "cwd": str(checkout), "steps": [[_executable(checkout, "python"), *args]],
            "display": [["python", *args]], "timeout_s": None, "digest": None, "windows_required": False}


def packets(checkout: Path) -> list[str]:
    directory = Path(checkout) / "work" / "reviews" / "packets"
    try:
        files = [f for p in directory.iterdir() if re.fullmatch(PACKET_RE, p.name)
                 and (f := paths.packet_file(checkout, p.name)) is not None]
        return [f.name for f in sorted(files, key=lambda f: (f.stat().st_mtime_ns, f.name), reverse=True)]
    except OSError:
        return []


def _iso() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def _timestamp(value) -> float | None:
    try:
        return datetime.fromisoformat(value.replace("Z", "+00:00")).timestamp() if isinstance(value, str) else None
    except (ValueError, OverflowError, OSError):
        return None


def _write_record(directory: Path, record: dict) -> None:
    directory.mkdir(parents=True, exist_ok=True)
    target = directory / (record["rid"] + ".json")
    tmp = target.with_suffix(".json.tmp")
    tmp.write_text(json.dumps(record, ensure_ascii=True) + "\n", encoding="utf-8")
    for i in range(5):
        try:
            os.replace(tmp, target)
            return
        except PermissionError:
            if i == 4:
                raise
            time.sleep(0.05)


def _entry_for_plan(main: Path, plan: dict) -> tuple[dict, str | None]:
    if plan["kind"] == "codex":
        return {"inputs": ["tools/agents/codex_review.py", plan["steps"][0][5]], "outputs": []}, None
    registry = load_registry(main)
    entry = next((e for e in registry.entries if e["id"] == plan["run"]), None)
    if entry is None or entry_digest(entry) != plan["digest"]:
        raise RunRefused("the registry entry changed after it was shown")
    return entry, registry.sha256


def _new_record(rid: str, plan: dict, registry_sha: str | None) -> dict:
    created = _iso()
    python = next((step[0] for step, shown in zip(plan["steps"], plan["display"]) if _python(shown[0])), None)
    return {**plan, "schema": 1, "rid": rid, "registry_sha256": registry_sha, "python": python,
            "created": created, "started": None, "ended": None, "alive_at": created, "status": "starting",
            "reason": None, "exits": [], "call_id": None, "inputs": {"files": {}, "incomplete": False},
            "outputs": [], "log_dropped": 0, "lock": "rid-" + rid + ".lock", "entry_lock": None}


def start(main: Path, checkout: Path, *, run_id: str | None = None, expect: str | None = None,
          codex: dict | None = None) -> str:
    if (run_id is None) == (codex is None) or (codex is not None and expect is not None):
        raise RunRefused("choose exactly one registered run or Codex call")
    if codex is not None:
        if not isinstance(codex, dict) or set(codex) != {"kind", "packet", "model", "effort", "speed"}:
            raise RunRefused("Codex requires exactly kind, packet, model, effort and speed")
        plan = plan_codex(main, checkout, **codex)
        options = ["--codex", *[arg for k in ("kind", "packet", "model", "effort", "speed") for arg in ("--" + k, codex[k])]]
    else:
        plan = plan_run(main, checkout, run_id)
        options = ["--run", run_id, "--expect", plan["digest"] if expect is None else expect]
    _, registry_sha = _entry_for_plan(main, plan)
    rid = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + uuid.uuid4().hex[:8]
    directory = paths.data_root(main) / "runs"
    record = _new_record(rid, plan, registry_sha)
    _write_record(directory, record)
    python = Path(sys.executable)
    if python.name.lower() == "pythonw.exe" and python.with_name("python.exe").is_file():
        python = python.with_name("python.exe")
    argv = [str(python), str(Path(__file__).resolve().with_name("runner.py")), "--main", str(Path(main).resolve()),
            "--checkout", plan["checkout"], "--rid", rid, *options]
    detach = {"creationflags": subprocess.CREATE_NO_WINDOW | subprocess.CREATE_NEW_PROCESS_GROUP} if os.name == "nt" else {"start_new_session": True}
    try:
        subprocess.Popen(argv, cwd=str(main), stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                         stderr=subprocess.DEVNULL, **detach)
    except OSError as exc:
        record.update(status="refused", reason=str(exc), ended=_iso())
        _write_record(directory, record)
        raise RunRefused(f"cannot start the runner: {exc}") from exc
    return rid


def cancel(data: Path, rid: str) -> None:
    if not isinstance(rid, str) or re.fullmatch(RID_RE, rid) is None:
        raise RunRefused("invalid run id")
    directory = Path(data) / "runs"
    directory.mkdir(parents=True, exist_ok=True)
    (directory / (rid + ".cancel")).touch()


def _take_lock(path: Path) -> int:
    fd = os.open(path, os.O_RDWR | os.O_CREAT, 0o600)
    try:
        if os.name == "nt":
            import msvcrt
            os.lseek(fd, 0, 0)
            msvcrt.locking(fd, msvcrt.LK_NBLCK, 1)
        else:
            import fcntl
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BaseException:
        os.close(fd)
        raise
    return fd


def _release_lock(fd: int) -> None:
    try:
        if os.name == "nt":
            import msvcrt
            os.lseek(fd, 0, 0)
            msvcrt.locking(fd, msvcrt.LK_UNLCK, 1)
        else:
            import fcntl
            fcntl.flock(fd, fcntl.LOCK_UN)
    finally:
        os.close(fd)


def _lock_free(path: Path) -> bool:
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        fd = _take_lock(path)
    except OSError:
        return False
    _release_lock(fd)
    return True


def _entry_lock_name(checkout: Path, run_id: str) -> str:
    key = os.path.normcase(os.path.realpath(checkout)) + "\0" + run_id
    return hashlib.sha256(key.encode("utf-8")).hexdigest()[:24] + ".lock"


def _record_ok(record, stem: str) -> bool:
    """Every field the app and the flag rules read has the type the runner writes."""
    def optional_text(key):
        return record.get(key) is None or isinstance(record[key], str)

    return (isinstance(record, dict) and record.get("schema") == 1
            and isinstance(record.get("rid"), str) and re.fullmatch(RID_RE, record["rid"]) is not None and record["rid"] == stem
            and record.get("status") in ("starting", "running", "passed", "failed", "timed_out", "cancelled", "refused")
            and record.get("kind") in ("registry", "codex")
            and isinstance(record.get("run"), str) and record["run"] != ""
            and isinstance(record.get("title"), str) and isinstance(record.get("checkout"), str)
            and optional_text("reason") and optional_text("call_id")
            and all(record.get(key) is None or _timestamp(record[key]) is not None for key in ("created", "started", "ended", "alive_at"))
            and isinstance(record.get("exits"), list)
            and all(code is None or (isinstance(code, int) and not isinstance(code, bool)) for code in record["exits"])
            and isinstance(record.get("outputs", []), list))


def recent(data: Path, now: float | None = None, limit: int = 50, window: float | None = None) -> list[dict]:
    """Run records, newest first: the newest `limit` files and, with `window`, every older one written in the
    last `window` seconds. A running record is rewritten at each heartbeat and an ended one at its end, so the
    window keeps every live or recently ended run visible to the flag rules, whatever the display limit."""
    now = time.time() if now is None else now
    recent.malformed = 0
    directory = Path(data) / "runs"
    found = []
    try:
        with os.scandir(directory) as it:
            entries = sorted((e for e in it if e.name.endswith(".json")), key=lambda e: e.name, reverse=True)
    except OSError:
        return found
    files = []
    for index, entry in enumerate(entries):
        try:
            if index < limit or (window is not None and now - entry.stat().st_mtime <= window):
                files.append(Path(entry.path))
        except OSError:
            continue
    for file in files:
        try:
            with file.open("rb") as f:
                raw = f.read(JSON_LIMIT + 1)
            if len(raw) > JSON_LIMIT:
                raise ValueError("oversized record")
            record = json.loads(raw)
            if not _record_ok(record, file.stem):
                raise ValueError("invalid record")
        except (OSError, ValueError, UnicodeError, RecursionError):
            recent.malformed += 1
            continue
        started, ended = _timestamp(record.get("started")), _timestamp(record.get("ended"))
        record["duration_s"] = (now if ended is None else ended) - started if started is not None else None
        if record["status"] in ("running", "starting"):
            free = _lock_free(directory / "locks" / ("rid-" + record["rid"] + ".lock"))
            if record["status"] == "running":
                if free:
                    record.update(status="interrupted", reason="the runner ended without a final record")
                else:
                    alive = _timestamp(record.get("alive_at"))
                    if alive is not None and now - alive > 30:
                        record["stale_heartbeat_s"] = now - alive
            else:
                created = _timestamp(record.get("created"))
                if free and created is not None and now - created > 30:
                    record.update(status="interrupted", reason="the runner did not start")
        found.append(record)
    return found


recent.malformed = 0


class LogTail:
    def __init__(self, path: Path):
        self.path = Path(path)
        self.offset = 0
        self.partial = b""

    def poll(self, max_read: int = 1 << 20) -> list[str]:
        try:
            with self.path.open("rb") as f:
                if os.fstat(f.fileno()).st_size < self.offset:
                    self.offset, self.partial = 0, b""
                f.seek(self.offset)
                chunk = f.read(max(0, max_read))
                self.offset += len(chunk)
        except OSError:
            return []
        lines = (self.partial + chunk).split(b"\n")
        self.partial = lines.pop()
        return [line.rstrip(b"\r").decode("utf-8", errors="replace") for line in lines]
