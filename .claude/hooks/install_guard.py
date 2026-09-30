"""PreToolUse hook for Bash: allow allowlisted installs only for the owner-approved installer revision.

An exact `python tools/toolchain/bootstrap.py install <id>`, `install --profile <p>`
or `install-skill <id>` command, run from the project root, is allowed without a
prompt when the digest of the installer and its lockfiles matches the owner's
approval record. Otherwise it falls back to asking the owner. Every other command
gets no decision from this hook.

The hook never imports or runs repository code: the digest is computed here, so
an unapproved installer cannot execute before approval is checked. The digest
algorithm must match tools/repo_digest.py (tests enforce this).
"""
from __future__ import annotations

import hashlib
import json
import os
import re
import sys
from pathlib import Path

ROOT = Path(os.environ.get("CLAUDE_PROJECT_DIR") or Path(__file__).resolve().parents[2])
APPROVAL_PATH = ROOT / "work" / "loop-memory" / "approvals" / "toolchain.json"
INSTALL_RE = re.compile(
    r"^python3? tools/toolchain/bootstrap\.py "
    r"(?:install (?:--profile )?[a-z0-9][a-z0-9-]{0,63}|install-skill [a-z0-9][a-z0-9-]{0,63})$"
)


def _sha256(path: Path) -> str:
    data = path.read_bytes()
    if b"\0" not in data:
        data = data.replace(b"\r\n", b"\n")
    return hashlib.sha256(data).hexdigest()


def approval_inputs(root: Path = ROOT) -> list[str]:
    spec = json.loads((root / "tools" / "toolchain" / "approval_inputs.json").read_text(encoding="utf-8"))
    for rel in [*spec["files"], *spec["globs"]]:
        parts = Path(rel).parts
        if not isinstance(rel, str) or not rel or Path(rel).is_absolute() or Path(rel).drive or ".." in parts or "\\" in rel or ":" in rel:
            raise ValueError(f"approval input {rel!r} must be a repository-relative path")
    inputs = set(spec["files"])
    for pattern in spec["globs"]:
        inputs.update(p.relative_to(root).as_posix() for p in root.glob(pattern))
    return sorted(inputs)


def approval_digest(root: Path = ROOT) -> str:
    h = hashlib.sha256()
    for rel in approval_inputs(root):
        p = root / rel
        h.update(f"{rel}\0{_sha256(p) if p.is_file() else 'missing'}\n".encode())
    return h.hexdigest()


def _is_link(p: Path) -> bool:
    """True for a symlink or a Windows junction (realpath resolves both, on every supported Python)."""
    real = os.path.normcase(os.path.realpath(p))
    return p.is_symlink() or real != os.path.normcase(os.path.join(os.path.realpath(p.parent), p.name))


def linked_inputs(root: Path = ROOT) -> list[str]:
    """Approval inputs that are, or sit below, a symlink or junction inside the project."""
    base = root.resolve()
    linked = []
    for rel in approval_inputs(root):
        probe = base
        for part in Path(rel).parts:
            probe = probe / part
            if _is_link(probe) or (probe.exists() and probe.resolve() != probe):
                linked.append(rel)
                break
    return linked


def approval_state() -> str:
    try:
        if linked_inputs():
            return "blocked (an approved input is a link)"
        record = json.loads(APPROVAL_PATH.read_text(encoding="utf-8"))
        return "approved" if isinstance(record, dict) and record.get("digest") == approval_digest() else "stale"
    except (OSError, ValueError, KeyError, TypeError):
        return "not-approved"


def decide(command: str, cwd: str | None = None) -> dict | None:
    command = command.strip()
    if "bootstrap.py" not in command or not INSTALL_RE.match(command):
        return None  # not an exact install command: normal permission rules apply
    try:
        same_root = cwd is not None and Path(cwd).resolve() == ROOT.resolve()
    except OSError:
        same_root = False
    state = approval_state() if same_root else "bound to another directory"
    if state == "approved":
        decision, reason = "allow", "Installer and lockfiles match the owner-approved revision."
    else:
        decision = "ask"
        reason = (f"Automatic installation is {state}. Installs run without a prompt only from the project root and only "
                  "for the owner-approved installer revision; the owner can review and run "
                  "`python tools/toolchain/bootstrap.py approve` in a terminal.")
    return {"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": decision,
                                   "permissionDecisionReason": reason}}


def main() -> int:
    try:
        event = json.loads(sys.stdin.read() or "{}")
    except ValueError:
        return 0
    if event.get("tool_name") != "Bash":
        return 0
    result = decide(str((event.get("tool_input") or {}).get("command", "")), event.get("cwd"))
    if result:
        print(json.dumps(result))
    return 0


if __name__ == "__main__":
    sys.exit(main())
