"""How the workbench opens files and tools: each in its own application, never by running a file.

`plan_open` and `plan_command` decide (pure functions, tested headless); `run` carries out a
decision. Policy:
- Private locations (`work/` of every checkout, the Claude Code projects directory) open only
  as plain text in Notepad, never through a default association or a browser.
- Project files open in their default application for inert document types; HTML and SVG
  (active content, such as Marp and Mermaid outputs) open in the browser, only from project
  locations. Scripts, executables and shortcuts are never opened.
- Commands run in a new console with a fixed argument list: git diff, bootstrap doctor, a marimo
  notebook, and `claude --resume` only for a session whose client has ended it.
"""
from __future__ import annotations

import os
import shutil
import subprocess
import sys
import webbrowser
from pathlib import Path

try:
    from . import paths
except ImportError:
    import paths  # type: ignore[no-redef]

PRIVATE_TEXT = {".md", ".txt", ".json", ".jsonl", ".log", ".csv"}
PROJECT_DOCS = {".md", ".txt", ".json", ".csv", ".png", ".pdf", ".drawio", ".mmd", ".typ"}
ACTIVE = {".html", ".htm", ".svg"}


class LaunchRefused(Exception):
    pass


def notepad() -> str:
    return str(Path(os.environ.get("SystemRoot", r"C:\Windows")) / "notepad.exe")


def private_roots(main: Path, projects: Path) -> list[Path]:
    return [c / "work" for c in paths.checkouts(main)] + [Path(projects)]


def plan_open(path, main: Path, projects: Path) -> tuple[str, object]:
    """("notepad", argv) | ("default", path) | ("browser", path), or LaunchRefused."""
    p = Path(path)
    if not p.is_file():
        raise LaunchRefused(f"not a file: {p.name}")
    ext = p.suffix.lower()
    if paths.inside(p, private_roots(main, projects)):
        if ext in PRIVATE_TEXT:
            return "notepad", [notepad(), str(p.resolve())]
        raise LaunchRefused("private files open only as plain text")
    if not paths.inside(p, paths.checkouts(main)):
        raise LaunchRefused("outside the repository")
    if ext in PROJECT_DOCS:
        return "default", str(p.resolve())
    if ext in ACTIVE:
        return "browser", str(p.resolve())
    raise LaunchRefused(f"{ext or 'this file type'} is not opened by the workbench")


def plan_command(kind: str, main: Path, *, checkout=None, notebook=None, sid=None, cwd=None,
                 session_status=None, remote=False) -> tuple[list[str], str]:
    """(argv, working directory) for a tool command, or LaunchRefused."""
    main = Path(main)
    roots = paths.checkouts(main)
    if kind == "git-diff":
        if checkout is None or not any(Path(checkout).resolve() == r for r in roots):
            raise LaunchRefused("not a checkout of this repository")
        return ["git", "-C", str(Path(checkout).resolve()), "--no-pager", "diff"], str(main)
    if kind == "doctor":
        return ["python", "tools/toolchain/bootstrap.py", "doctor"], str(main)
    if kind == "marimo":
        nb = Path(notebook) if notebook else None
        if nb is None or nb.suffix.lower() != ".py" or not nb.is_file() or not paths.inside(nb, roots) \
                or paths.inside(nb, [r / "work" for r in roots]):
            raise LaunchRefused("marimo opens only project notebooks")
        exe = main / "tools" / ".venv" / "planning" / ("Scripts/marimo.exe" if os.name == "nt" else "bin/marimo")
        return [str(exe), "edit", str(nb.resolve())], str(main)
    if kind == "resume":
        if not paths.valid_sid(sid):
            raise LaunchRefused("invalid session id")
        if session_status != "finished":
            # Only SessionEnd with no newer activity shows that the owning client let go of the transcript.
            raise LaunchRefused("the session may still be open in another client; bring Claude Code forward instead")
        argv = ["claude", "--resume", sid] + (["--remote-control"] if remote else [])
        return argv, str(cwd or main)
    raise LaunchRefused(f"unknown command {kind}")


def resume_command_text(sid: str, remote: bool = False) -> str:
    return f"claude --resume {sid}" + (" --remote-control" if remote else "")


def run(plan: tuple[str, object]) -> None:  # pragma: no cover - touches the desktop
    how, target = plan
    if how == "notepad":
        subprocess.Popen(target)
    elif how == "default":
        os.startfile(target)  # type: ignore[attr-defined]
    elif how == "browser":
        webbrowser.open(Path(target).as_uri())
    else:
        raise LaunchRefused(how)


KEEP_OPEN = ("git", "python")  # console tools whose output the owner reads after they exit
# Runs the argument list directly (never through a shell), then waits so the console stays open.
KEEP_OPEN_SCRIPT = (
    "import subprocess, sys\n"
    "try:\n"
    "    code = subprocess.call(sys.argv[1:])\n"
    "except OSError as exc:\n"
    "    print(exc)\n"
    "    code = 1\n"
    "input(f'\\n[exit {code}] Press Enter to close this window.')\n"
)


def console_python() -> str:
    """A console interpreter even when the workbench itself runs under pythonw."""
    exe = Path(sys.executable)
    console = exe.with_name("python.exe") if exe.name.lower() == "pythonw.exe" else exe
    return str(console if console.exists() else exe)


def console_argv(argv: list[str]) -> list[str]:
    """The process argument list for a tool command. No shell ever parses it, so characters
    such as & or % in a path stay part of that one argument."""
    full = [shutil.which(argv[0]) or argv[0], *argv[1:]]
    if argv[0] in KEEP_OPEN:
        return [console_python(), "-c", KEEP_OPEN_SCRIPT, *full]
    return full


def run_command(argv: list[str], cwd: str) -> None:  # pragma: no cover - touches the desktop
    subprocess.Popen(console_argv(argv), cwd=cwd, creationflags=getattr(subprocess, "CREATE_NEW_CONSOLE", 0))


def bring_claude_forward() -> None:  # pragma: no cover - touches the desktop
    os.startfile("claude://")  # type: ignore[attr-defined]
