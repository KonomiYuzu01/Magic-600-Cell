"""Check, install and diagnose the pinned Magic 600 Cell development tools.

  python tools/toolchain/bootstrap.py check [--profile P]      fast presence check (no subprocesses)
  python tools/toolchain/bootstrap.py doctor                   full report with probes
  python tools/toolchain/bootstrap.py install <id>             install one allowlisted tool
  python tools/toolchain/bootstrap.py install --profile <P>    install every installable tool in a profile
  python tools/toolchain/bootstrap.py install-skill <id>       install a pinned third-party skill
  python tools/toolchain/bootstrap.py pin <id>                 record a winget version and installer hash (Windows)
  python tools/toolchain/bootstrap.py approve                  owner only: approve this installer and lockfile revision

Only entries in tools/toolchain.lock.json with status "verified" and installable true
are installed. Unknown arguments are rejected before any download or write.
"""
from __future__ import annotations

import argparse
import importlib.util
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
import types
from datetime import datetime, timezone
from pathlib import Path

def _is_link(p: Path) -> bool:
    """True for a symlink or a Windows junction (realpath resolves both, on every supported Python)."""
    real = os.path.normcase(os.path.realpath(p))
    return p.is_symlink() or real != os.path.normcase(os.path.join(os.path.realpath(p.parent), p.name))


_HERE = Path(os.path.abspath(__file__))
for _p in (_HERE, _HERE.parent, _HERE.parents[1]):
    # Never follow a linked installer to helpers outside the approved tree.
    if _is_link(_p):
        sys.exit(f"refused: {_p.name} is a link; run the installer from the repository itself")
def _load_source(name: str, path: Path):
    """Import a helper from its source text only; cached bytecode is never trusted."""
    module = types.ModuleType(name)
    module.__file__ = str(path)
    sys.modules[name] = module
    exec(compile(path.read_bytes(), str(path), "exec"), module.__dict__)
    return module


_digest = _load_source("repo_digest", _HERE.parents[1] / "repo_digest.py")
ROOT, files_digest, tree_digest = _digest.ROOT, _digest.files_digest, _digest.tree_digest

LOCK_PATH = ROOT / "tools" / "toolchain.lock.json"
SKILLS_LOCK_PATH = ROOT / "tools" / "skills.lock.json"
APPROVAL_PATH = ROOT / "work" / "loop-memory" / "approvals" / "toolchain.json"
LEDGER_PATH = ROOT / "work" / "loop-memory" / "ledgers" / "installs.jsonl"
PROBE_TIMEOUT = 20
INSTALL_TIMEOUT = 1800
ID_RE = re.compile(r"^[a-z0-9][a-z0-9-]{0,63}$")
PLATFORM = {"win32": "windows", "darwin": "macos"}.get(sys.platform, "linux")
REQUIRED_KEYS = {"id", "purpose", "profiles", "method", "status", "installable", "platforms"}
METHODS = {"external", "pip-hashed", "uv-venv-hashed", "npm-ci", "winget"}


class Refused(Exception):
    """The request is outside the allowlist or its authorization."""


def approval_inputs() -> list[str]:
    """Files whose exact content the owner approves (tools/toolchain/approval_inputs.json)."""
    spec = json.loads((ROOT / "tools" / "toolchain" / "approval_inputs.json").read_text(encoding="utf-8"))
    for rel in [*spec["files"], *spec["globs"]]:
        parts = Path(rel).parts
        if not isinstance(rel, str) or not rel or Path(rel).is_absolute() or Path(rel).drive or ".." in parts or "\\" in rel or ":" in rel:
            raise ValueError(f"approval input {rel!r} must be a repository-relative path")
    inputs = set(spec["files"])
    for pattern in spec["globs"]:
        inputs.update(p.relative_to(ROOT).as_posix() for p in ROOT.glob(pattern))
    return sorted(inputs)


def approval_digest() -> str:
    return files_digest(approval_inputs())


def linked_inputs() -> list[str]:
    """Approval inputs that are, or sit below, a symlink or junction (never approvable)."""
    linked = []
    try:
        inputs = approval_inputs()
    except (ValueError, TypeError, KeyError) as exc:
        raise Refused(f"approval_inputs.json is invalid: {exc}")
    for rel in inputs:
        try:
            safe_dest(rel)
        except Refused:
            linked.append(rel)
    return linked


def approval_state() -> str:
    try:
        if linked_inputs():
            return "blocked (an approved input is a link)"
    except Refused:
        return "blocked (invalid approval inputs)"
    try:
        record = json.loads(APPROVAL_PATH.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return "not-approved"
    return "approved" if isinstance(record, dict) and record.get("digest") == approval_digest() else "stale"


def load_lock() -> dict:
    lock = json.loads(LOCK_PATH.read_text(encoding="utf-8"))
    ids = set()
    for entry in lock["tools"]:
        missing = REQUIRED_KEYS - entry.keys()
        if missing:
            raise SystemExit(f"lockfile: {entry.get('id')} lacks {sorted(missing)}")
        if entry["method"] not in METHODS:
            raise SystemExit(f"lockfile: {entry['id']} has unknown method {entry['method']}")
        if entry["id"] in ids or not ID_RE.match(entry["id"]):
            raise SystemExit(f"lockfile: bad or duplicate id {entry['id']}")
        ids.add(entry["id"])
    return lock


def entry_for(lock: dict, tool_id: str) -> dict:
    for entry in lock["tools"]:
        if entry["id"] == tool_id:
            return entry
    raise Refused(f"{tool_id}: not in the allowlist")


def safe_dest(rel: str) -> Path:
    """Resolve an install destination inside the repository; refuse symlinks or junctions on the way."""
    root = ROOT.resolve()
    dest = root / rel
    if dest.resolve() != dest or root not in dest.parents:
        raise Refused(f"{rel}: destination leaves the repository or passes through a link")
    probe = root
    for part in Path(rel).parts:
        probe = probe / part
        if probe.is_symlink() or (probe.exists() and probe.resolve() != probe):
            raise Refused(f"{rel}: destination passes through a link")
    return dest


def _bin_dir(venv: Path) -> Path:
    return venv / ("Scripts" if PLATFORM == "windows" else "bin")


def resolve_executable(entry: dict, name: str) -> str | None:
    """Expand {python}, {venv} and {prefix} placeholders to a concrete executable path."""
    if name == "{python}":
        return sys.executable
    if name.startswith("{venv}/"):
        root = safe_dest(entry["venv"])
        venv_contained(entry)  # the interpreter and scripts it runs must stay inside the environment
        base = _bin_dir(root) / name[len("{venv}/"):]
    elif name.startswith("{prefix}/"):
        root = safe_dest(entry["prefix"])
        base = root / name[len("{prefix}/"):]
    else:
        found = shutil.which(name)
        if found is None and PLATFORM == "windows":
            # winget puts command-line tools here; a terminal opened before the install lacks it on PATH.
            links = Path(os.environ.get("LOCALAPPDATA", "")) / "Microsoft" / "WinGet" / "Links"
            if os.environ.get("LOCALAPPDATA") and links.is_dir():
                found = shutil.which(name, path=str(links))
        return found
    for suffix in ([".exe", ".cmd", ""] if PLATFORM == "windows" else [""]):
        candidate = base.with_name(base.name + suffix)
        if os.path.lexists(candidate):
            real = candidate.resolve()
            if root.resolve() not in real.parents:
                raise Refused(f"{name}: executable leads outside {root.relative_to(ROOT).as_posix()}")
            if real.is_file():
                return str(candidate)
    return None


def present(entry: dict) -> bool:
    """Cheap presence test without subprocesses (used by check and the SessionStart hook)."""
    probe = entry.get("probe")
    if not probe:
        return False
    if probe[0] == "{python}" and len(probe) > 2 and probe[1] == "-m":
        return importlib.util.find_spec(probe[2]) is not None
    try:
        return resolve_executable(entry, probe[0]) is not None
    except Refused:
        return False


def run_probe(entry: dict) -> tuple[bool, str]:
    probe = entry.get("probe")
    if not probe:
        return False, "no probe defined"
    try:
        exe = resolve_executable(entry, probe[0])
    except Refused as exc:
        return False, f"refused: {exc}"
    if exe is None:
        if entry["method"] == "winget" and PLATFORM == "windows" and entry.get("version"):
            # GUI applications such as draw.io add no command to PATH; winget's own record decides.
            return winget_installed(entry)
        return False, "missing"
    try:
        r = subprocess.run([exe, *probe[1:]], capture_output=True, text=True, timeout=PROBE_TIMEOUT, env=probe_env())
    except (OSError, subprocess.TimeoutExpired) as exc:
        return False, f"probe failed: {type(exc).__name__}"
    output = (r.stdout + r.stderr).strip().splitlines()
    first = output[0] if output else ""
    if r.returncode != 0:
        return False, f"probe exit {r.returncode}: {first[:120]}"
    expect = entry.get("expect")
    if expect and not re.search(expect, r.stdout + r.stderr):
        return False, f"unexpected version: {first[:120]}"
    version = entry.get("version")
    if entry["method"] in EXACT_VERSION_METHODS and version:
        # The tool reports its own version first; later versions belong to dependencies.
        # The whole token is compared, so 0.12.20rc1, 0.12.20.dev1 or 0.12.20+local never match 0.12.20.
        reported = re.search(r"(?<![\w.])v?(\d[\w.+-]*)", r.stdout + r.stderr)
        if not reported or reported.group(1).rstrip(".") != version:
            return False, f"version is not exactly {version}: {first[:120]}"
    return True, first[:120]


# Methods whose probe prints the pinned version. uv-venv-hashed tools are compared exactly
# through the environment's installed distributions instead (venv_conflicts); a winget GUI
# tool without a command on PATH is compared through winget's record (winget_installed).
EXACT_VERSION_METHODS = {"pip-hashed", "npm-ci", "winget"}


def _norm(name: str) -> str:
    return re.sub(r"[-_.]+", "-", name).lower()


def venv_contained(entry: dict) -> list[Path]:
    """Refuse an environment that contains links other than the standard ones; return its site-packages.

    Allowed: lib64 -> lib at the top level, and interpreter links (python*) in bin/Scripts that point
    to another interpreter link there or to the interpreter running this installer. Every other
    symlink or junction is refused, and the walk never descends into one.
    """
    venv = safe_dest(entry["venv"])
    interpreter = Path(os.path.realpath(sys.executable))
    bindir = _bin_dir(venv)
    stack = [venv]
    while stack:
        directory = stack.pop()
        try:
            children = list(os.scandir(directory))
        except FileNotFoundError:
            continue
        for child in children:
            p = Path(child.path)
            if not _is_link(p):
                if child.is_dir(follow_symlinks=False):
                    stack.append(p)
                continue
            rel = p.relative_to(venv).as_posix()
            if rel == "lib64" and os.readlink(p) in ("lib", "lib/"):
                continue
            if p.parent == bindir and p.name.lower().startswith("python"):
                real = p.resolve()
                if real == interpreter or (real.parent == bindir.resolve() and real.name.lower().startswith("python")):
                    continue
            raise Refused(f"{entry['venv']}: {rel} is a link; the environment must not contain links")
    return list(venv.glob("lib/python*/site-packages")) + list(venv.glob("Lib/site-packages"))


def venv_conflicts(entry: dict) -> list[str]:
    """Offline diagnostic: installed distributions that differ from the pins (markers are not evaluated).

    The authoritative check before a sync is sync_removals, which asks uv itself."""
    pins = {}
    for line in _require_file(entry["requirements"]).read_text(encoding="utf-8").splitlines():
        m = re.match(r"^([A-Za-z0-9][A-Za-z0-9._-]*)==([^\s;\\]+)", line)
        if m:
            pins[_norm(m.group(1))] = m.group(2)
    site = venv_contained(entry)
    conflicts = []
    for sp in site:
        for dist in [*sp.glob("*.dist-info"), *sp.glob("*.egg-info")]:
            name, _, version = dist.name.rsplit(".", 1)[0].rpartition("-")
            if not name:
                conflicts.append(dist.name)  # unparseable metadata: never assume it is pinned
            elif pins.get(_norm(name)) != version:
                conflicts.append(f"{name} {version}")
    return sorted(conflicts)


# Package sources are fixed here; ambient settings cannot redirect them.
PYPI_INDEX = "https://pypi.org/simple"
NPM_REGISTRY = "https://registry.npmjs.org/"
# Every package-manager variable is dropped except these transport settings (proxy, TLS roots);
# code-injection variables are always dropped.
_PM_PREFIX_RE = re.compile(r"^(?:PIP|UV|NPM_CONFIG|YARN)_", re.I)
_PM_KEEP = {"UV_NATIVE_TLS", "UV_SYSTEM_CERTS", "NPM_CONFIG_HTTPS_PROXY", "NPM_CONFIG_PROXY", "NPM_CONFIG_NOPROXY", "NPM_CONFIG_CAFILE"}
_INJECT = {"PYTHONPATH", "PYTHONHOME", "PYTHONSTARTUP", "PYTHONUSERBASE", "NODE_OPTIONS", "NODE_PATH"}


def probe_env() -> dict:
    """Environment for probes: code-injection variables removed."""
    return {k: v for k, v in os.environ.items() if k.upper() not in _INJECT}


def install_env() -> dict:
    """Environment for package managers: only allowlisted transport settings survive; user config files are ignored."""
    env = {k: v for k, v in os.environ.items()
           if k.upper() not in _INJECT and (not _PM_PREFIX_RE.match(k) or k.upper() in _PM_KEEP)}
    empty = Path(tempfile.mkdtemp(prefix="magic600-npmrc-"))  # npm refuses one file for both user and global config
    for name in ("user", "global"):
        (empty / name).write_text("", encoding="utf-8")
    env.update({
        "PIP_CONFIG_FILE": os.devnull,
        "UV_NO_CONFIG": "1",
        "UV_PYTHON_DOWNLOADS": "never",
        "npm_config_userconfig": str(empty / "user"),
        "npm_config_globalconfig": str(empty / "global"),
        "npm_config_registry": NPM_REGISTRY,
    })
    return env


def git_env(empty: Path) -> dict:
    """Environment for Git downloads: no system, global or environment config, no templates, no hooks."""
    env = {k: v for k, v in install_env().items()
           if not k.upper().startswith("GIT_") or k.upper() in ("GIT_SSL_CAINFO", "GIT_TERMINAL_PROMPT")}
    env.update({"GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": os.devnull, "GIT_TEMPLATE_DIR": str(empty),
                "GIT_TERMINAL_PROMPT": "0"})
    return env


def git_cmd(empty: Path, *args: str) -> list[str]:
    return ["git", "-c", f"core.hooksPath={empty}", "-c", "protocol.allow=never", "-c", "protocol.https.allow=always",
            "-c", "core.symlinks=false", *args]


def _run(cmd: list[str], cwd: Path = ROOT, env: dict | None = None) -> None:
    print("+ " + " ".join(cmd), flush=True)
    r = subprocess.run(cmd, cwd=cwd, timeout=INSTALL_TIMEOUT, env=env if env is not None else install_env())
    if r.returncode != 0:
        raise Refused(f"command failed with exit {r.returncode}")


def _require_file(rel: str) -> Path:
    p = (ROOT / rel).resolve()
    if ROOT.resolve() not in p.parents or not p.is_file():
        raise Refused(f"{rel}: missing or outside the repository")
    return p


# pin, the hash check and install select the same installer (user scope, x64).
WINGET_SELECTION = ["--scope", "user", "--architecture", "x64"]


def winget_show(winget_id: str, version: str | None) -> dict:
    exe = shutil.which("winget")
    if exe is None:
        raise Refused("winget is not available")
    cmd = [exe, "show", "--id", winget_id, "--exact", *WINGET_SELECTION, "--disable-interactivity", "--accept-source-agreements"]
    if version:
        cmd += ["--version", version]
    r = subprocess.run(cmd, capture_output=True, text=True, timeout=120)
    if r.returncode != 0:
        raise Refused(f"winget show {winget_id} failed with exit {r.returncode}")
    found_version = re.search(r"^Version:\s*(\S+)", r.stdout, re.M)
    sha = re.search(r"Installer SHA256:\s*([0-9A-Fa-f]{64})", r.stdout)
    if not found_version or not sha:
        raise Refused(f"winget show {winget_id}: version or installer hash not reported")
    return {"version": found_version.group(1), "installer_sha256": sha.group(1).lower()}


APPROVED_SOURCES = {"pip-hashed": PYPI_INDEX, "uv-venv-hashed": PYPI_INDEX, "npm-ci": NPM_REGISTRY}


def sync_removals(entry: dict, sync: list[str]) -> list[str]:
    """Distributions the sync would uninstall or replace, as reported by uv's own dry run (markers included)."""
    venv_contained(entry)
    env = {k: v for k, v in install_env().items() if k.upper() not in ("CLICOLOR_FORCE", "FORCE_COLOR")}
    env["NO_COLOR"] = "1"
    r = subprocess.run([*sync[:5], "--dry-run", "--color", "never", *sync[5:]], capture_output=True, text=True,
                       timeout=INSTALL_TIMEOUT, env=env)
    if r.returncode != 0:
        raise Refused(f"{entry['id']}: uv dry run failed with exit {r.returncode}")
    plain = re.sub(r"\x1b\[[0-9;?]*[ -/]*[@-~]", "", r.stdout + r.stderr)  # never trust color settings
    return re.findall(r"^\s*-\s+(\S+)", plain, re.M)


# winget's APPINSTALLER_CLI_ERROR_NO_APPLICATIONS_FOUND (0x8A150014): the only exit that means "not installed".
WINGET_NO_MATCH = {0x8A150014, 0x8A150014 - 2**32}


def winget_listing(entry: dict) -> str | None:
    """The installed version winget records for this package, or None when winget confirms it is not installed.

    Any other outcome (winget missing, a failed or unreadable listing) is refused, so an unknown state
    never leads to an install."""
    exe = shutil.which("winget")
    if exe is None:
        raise Refused("winget is not available")
    try:
        r = subprocess.run([exe, "list", "--id", entry["winget_id"], "--exact", "--disable-interactivity",
                            "--accept-source-agreements"], capture_output=True, text=True, timeout=120, env=probe_env())
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise Refused(f"winget list failed: {type(exc).__name__}")
    if r.returncode in WINGET_NO_MATCH:
        return None
    if r.returncode != 0:
        raise Refused(f"winget list {entry['winget_id']} failed with exit {r.returncode}")
    # Table columns: Name, Id, Version, [Available], Source. The installed version follows the Id;
    # an Available column must never be read as the installed version.
    versions = []
    for line in r.stdout.splitlines():
        tokens = [t.lower() for t in line.split()]
        if entry["winget_id"].lower() in tokens:
            i = tokens.index(entry["winget_id"].lower())
            versions.append(line.split()[i + 1] if i + 1 < len(tokens) else "")
    if len(versions) != 1 or not versions[0]:
        raise Refused(f"winget list {entry['winget_id']}: installed version could not be read")
    return versions[0]


def winget_installed(entry: dict) -> tuple[bool, str]:
    """Whether winget reports the pinned version of this package as installed."""
    try:
        installed = winget_listing(entry)
    except Refused as exc:
        return False, f"unknown ({exc})"
    if installed is None:
        return False, "missing"
    # MSI packages often register the version with trailing ".0" parts (31.5.3 is listed as 31.5.3.0).
    if not re.fullmatch(rf"{re.escape(entry['version'])}(?:\.0)*", installed):
        return False, f"installed, but version {installed} is not {entry['version']}"
    return True, f"installed through winget: {entry['version']}"


def install_entry(entry: dict) -> None:
    method = entry["method"]
    source = APPROVED_SOURCES.get(method)
    if source and source.split("/")[2] not in entry.get("network_hosts", []):
        raise Refused(f"{entry['id']}: {source} is not among the entry's approved network hosts")
    if method == "pip-hashed":
        req = _require_file(entry["requirements"])
        cert = ["--cert", os.environ["PIP_CERT"]] if os.environ.get("PIP_CERT") else []  # transport only; --isolated ignores PIP_*
        _run([sys.executable, "-m", "pip", "install", "--isolated", "--user", "--require-hashes", "--no-deps",
              "--index-url", PYPI_INDEX, *cert, "-r", str(req)])
    elif method == "uv-venv-hashed":
        req = _require_file(entry["requirements"])
        venv = safe_dest(entry["venv"])
        if not (_bin_dir(venv)).is_dir():
            _run([sys.executable, "-m", "uv", "venv", "--no-config", str(venv), "--python", sys.executable])
        sync = [sys.executable, "-m", "uv", "pip", "sync", "--no-config", "--require-hashes", "--index-url", PYPI_INDEX,
                "--python", str(venv), str(req)]
        removals = sync_removals(entry, sync)
        if removals:
            raise Refused(f"{entry['id']}: syncing {entry['venv']} would replace or remove {', '.join(removals[:5])}; "
                          "replacing an installed tool needs owner approval")
        _run(sync)
    elif method == "npm-ci":
        prefix = safe_dest(entry["prefix"])
        safe_dest(entry["prefix"] + "/node_modules")  # npm ci empties it: never through a link
        for rel in ("package.json", "package-lock.json"):
            safe_dest(f"{entry['prefix']}/{rel}")
            _require_file(f"{entry['prefix']}/{rel}")
        for extra in ("npm-shrinkwrap.json", ".npmrc"):
            # npm would prefer or load these, and they are not approval inputs.
            if os.path.lexists(prefix / extra):
                raise Refused(f"{entry['id']}: unapproved {extra} in {entry['prefix']}; installation blocked")
        npm = shutil.which("npm")
        if npm is None:
            raise Refused("npm is not available; install Node.js first")
        _run([npm, "ci", f"--prefix={prefix}", "--workspaces=false", "--ignore-scripts", "--no-audit", "--no-fund",
              f"--registry={NPM_REGISTRY}"], cwd=prefix)
    elif method == "winget":
        if PLATFORM != "windows":
            raise Refused(f"{entry['id']}: winget entries install on Windows only")
        pinned = winget_show(entry["winget_id"], entry["version"])
        if pinned["installer_sha256"] != entry["installer_sha256"]:
            raise Refused(f"{entry['id']}: installer hash differs from the lockfile; installation blocked until verified again")
        # Dependencies are never installed implicitly; a required dependency becomes its own verified entry.
        _run([shutil.which("winget"), "install", "--id", entry["winget_id"], "--exact", "--version", entry["version"],
              *WINGET_SELECTION, "--skip-dependencies", "--silent", "--disable-interactivity",
              "--accept-source-agreements", "--accept-package-agreements"])
    else:
        raise Refused(f"{entry['id']}: {method} tools are installed by the owner")


def ledger(record: dict) -> None:
    LEDGER_PATH.parent.mkdir(parents=True, exist_ok=True)
    record = {"time": datetime.now(timezone.utc).isoformat(timespec="seconds"), **record}
    with LEDGER_PATH.open("a", encoding="utf-8") as f:
        f.write(json.dumps(record) + "\n")


def install(lock: dict, tool_id: str, done: set) -> str:
    if tool_id in done:
        return "done"
    entry = entry_for(lock, tool_id)
    if entry["status"] != "verified" or not entry["installable"]:
        raise Refused(f"{tool_id}: entry is {entry['status']} and not installable; ask the owner")
    if PLATFORM not in entry["platforms"]:
        raise Refused(f"{tool_id}: not available on {PLATFORM}")
    for dep in entry.get("requires", []):
        dep_entry = entry_for(lock, dep)
        if dep_entry["installable"]:
            install(lock, dep, done)
        elif not run_probe(dep_entry)[0]:  # owner-installed dependency must already work
            raise Refused(f"{tool_id}: requires {dep}, which the owner installs")
    ok, detail = run_probe(entry)
    if ok and entry["method"] == "uv-venv-hashed" and venv_conflicts(entry):
        raise Refused(f"{tool_id}: present, but {entry['venv']} differs from its pinned requirements; "
                      "replacing an installed tool needs owner approval")
    if ok:
        done.add(tool_id)
        print(f"{tool_id}: already present ({detail})")
        return "present"
    if present(entry) or (entry["method"] == "winget" and winget_listing(entry) is not None):
        # Installed but at another version or broken: replacing an installed tool needs the owner.
        # A GUI package such as draw.io adds no command to PATH, so winget's own record counts as present.
        raise Refused(f"{tool_id}: present but the probe failed ({detail}); replacing it needs owner approval")
    if entry["method"] == "uv-venv-hashed":
        # A sync pins, replaces and removes packages across the shared environment: refuse
        # before it would change anything already installed there.
        conflicts = venv_conflicts(entry)
        if conflicts:
            raise Refused(f"{tool_id}: syncing {entry['venv']} would replace or remove {', '.join(conflicts[:5])}; "
                          "replacing an installed tool needs owner approval")
    started = time.monotonic()
    install_entry(entry)
    ok, detail = run_probe(entry)
    ledger({"tool": tool_id, "version": entry.get("version"), "method": entry["method"], "result": "ok" if ok else "probe-failed",
            "approval": approval_state(), "seconds": round(time.monotonic() - started, 1)})
    if not ok:
        raise Refused(f"{tool_id}: installed but the probe failed ({detail})")
    done.add(tool_id)
    print(f"{tool_id}: installed ({detail})")
    return "installed"


def refuse_git_rewrites(url: str, env: dict) -> None:
    """Refuse when the Git configuration used for the download would rewrite the pinned URL to another source."""
    r = subprocess.run(["git", "config", "--get-regexp", r"^url\..*\.(insteadof|pushinsteadof)$"],
                       capture_output=True, text=True, timeout=20, env=env, cwd=env["GIT_TEMPLATE_DIR"])
    for line in r.stdout.splitlines():
        key, _, prefix = line.partition(" ")
        if key.lower().endswith(".insteadof") and prefix and url.startswith(prefix):
            raise Refused(f"Git configuration rewrites {prefix} to another source; installation blocked")


def install_skill(skill_id: str) -> None:
    lock = json.loads(SKILLS_LOCK_PATH.read_text(encoding="utf-8"))
    entry = lock["skills"].get(skill_id)
    if entry is None or entry["origin"] != "third-party":
        raise Refused(f"{skill_id}: not a pinned third-party skill")
    if not ID_RE.match(skill_id):
        raise Refused(f"{skill_id}: bad skill id")
    target = safe_dest(f".agents/skills/{skill_id}")
    safe_dest(f".claude/skills/{skill_id}")  # sync writes here next; refuse links before any download
    try:
        present_ok = target.is_dir() and tree_digest(target) == entry["digest"]
    except ValueError as exc:
        raise Refused(f"{skill_id}: {exc}")
    if present_ok:
        print(f"{skill_id}: already present at the pinned digest")
    else:
        if not re.fullmatch(r"https://github\.com/[\w.-]+/[\w.-]+", entry["repo"]) or not re.fullmatch(r"[0-9a-f]{40}", entry["commit"]):
            raise Refused(f"{skill_id}: lockfile source is not a pinned GitHub commit")
        with tempfile.TemporaryDirectory() as td:
            empty = Path(td) / "empty"
            empty.mkdir()
            env = git_env(empty)
            refuse_git_rewrites(entry["repo"], env)
            checkout = Path(td) / "src"
            _run(git_cmd(empty, "clone", "--quiet", "--no-checkout", f"--template={empty}", entry["repo"], str(checkout)), env=env)
            _run(git_cmd(empty, "-C", str(checkout), "checkout", "--quiet", entry["commit"]), env=env)
            staged = Path(td) / "skill"
            shutil.copytree(checkout / entry["subdir"], staged, symlinks=True)
            for dest, src in entry.get("extra_files", {}).items():
                shutil.copyfile(checkout / src, staged / dest)
            if any(_is_link(p) for p in [staged, *staged.rglob("*")]) or tree_digest(staged) != entry["digest"]:
                raise Refused(f"{skill_id}: content digest differs from the lockfile; installation blocked")
            if target.exists():
                shutil.rmtree(target)
            shutil.copytree(staged, target)
        ledger({"skill": skill_id, "commit": entry["commit"], "result": "ok", "approval": approval_state()})
        print(f"{skill_id}: installed at {entry['commit'][:12]}")
    _run([sys.executable, str(ROOT / "tools" / "skills" / "sync.py")])


def pin(lock: dict, tool_id: str) -> None:
    entry = entry_for(lock, tool_id)
    if entry["method"] != "winget":
        raise Refused(f"{tool_id}: only winget entries are pinned this way")
    pinned = winget_show(entry["winget_id"], entry.get("version"))
    entry.update(pinned, status="verified", installable=True)
    LOCK_PATH.write_text(json.dumps(lock, indent=2) + "\n", encoding="utf-8")
    print(f"{tool_id}: pinned {pinned['version']} ({pinned['installer_sha256'][:12]}). "
          "The lockfile changed, so installation asks again until the owner approves this revision.")


def approve() -> None:
    if not sys.stdin.isatty():
        raise Refused("approve must be run by the owner in an interactive terminal")
    linked = linked_inputs()
    if linked:
        raise Refused("approval inputs must be regular files, not links: " + ", ".join(linked))
    digest = approval_digest()
    print("Approving automatic installation for this exact revision of:")
    for rel in approval_inputs():
        print(f"  {rel}")
    answer = input(f"Type 'approve {digest[:12]}' to confirm: ").strip()
    if answer != f"approve {digest[:12]}":
        raise Refused("not confirmed")
    APPROVAL_PATH.parent.mkdir(parents=True, exist_ok=True)
    APPROVAL_PATH.write_text(json.dumps({"digest": digest, "approved_at": datetime.now(timezone.utc).isoformat(timespec="seconds")}, indent=2) + "\n", encoding="utf-8")
    print("approved")


def check(lock: dict, profile: str) -> int:
    missing = [e["id"] for e in lock["tools"] if profile in e["profiles"] and PLATFORM in e["platforms"] and not present(e)]
    print(f"toolchain {profile}: " + ("ok" if not missing else "missing " + ", ".join(missing)))
    return 0


def doctor(lock: dict) -> int:
    bits = 64 if sys.maxsize > 2**32 else 32
    print(f"python: {sys.version.split()[0]} {bits}-bit ({sys.implementation.name}); platform {PLATFORM}")
    print(f"automatic install approval: {approval_state()}")
    for entry in lock["tools"]:
        if PLATFORM not in entry["platforms"]:
            continue
        ok, detail = run_probe(entry) if entry.get("probe") else (False, "no probe")
        if ok and entry["method"] == "uv-venv-hashed":
            conflicts = venv_conflicts(entry)
            if conflicts:
                ok, detail = False, "environment differs from its pins: " + ", ".join(conflicts[:3])
        flag = "ok " if ok else ("-- " if entry["installable"] else "!! ")
        print(f"  {flag}{entry['id']:<12} [{','.join(entry['profiles'])}] {entry['status']}: {detail}")
    codex = shutil.which("codex")
    if codex:
        try:
            r = subprocess.run([codex, "login", "status"], capture_output=True, text=True, timeout=PROBE_TIMEOUT)
            print(f"codex login: {'ready' if r.returncode == 0 else 'not-ready'}")
        except (OSError, subprocess.TimeoutExpired):
            print("codex login: unknown")
    sync = subprocess.run([sys.executable, str(ROOT / "tools" / "skills" / "sync.py"), "--check"], capture_output=True, text=True)
    print((sync.stdout.strip().splitlines() or ["skills: unknown"])[-1])
    print("owner-installed only: " + ", ".join(lock["owner_required"]))
    return 0


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    p_check = sub.add_parser("check")
    p_check.add_argument("--profile", default="planning")
    sub.add_parser("doctor")
    p_install = sub.add_parser("install")
    group = p_install.add_mutually_exclusive_group(required=True)
    group.add_argument("tool_id", nargs="?")
    group.add_argument("--profile")
    p_skill = sub.add_parser("install-skill")
    p_skill.add_argument("skill_id")
    p_pin = sub.add_parser("pin")
    p_pin.add_argument("tool_id")
    sub.add_parser("approve")
    args = parser.parse_args(argv)

    lock = load_lock()
    try:
        for name in ("tool_id", "skill_id", "profile"):
            value = getattr(args, name, None)
            if value is not None and not ID_RE.match(value):
                raise Refused(f"invalid {name}: {value!r}")
        if args.command == "check":
            return check(lock, args.profile)
        if args.command == "doctor":
            return doctor(lock)
        if args.command == "install":
            done: set = set()
            if args.profile:
                if args.profile not in lock["profiles"]:
                    raise Refused(f"unknown profile {args.profile}")
                chosen = [e["id"] for e in lock["tools"] if args.profile in e["profiles"]]
                skipped = []
                for tool_id in chosen:
                    entry = entry_for(lock, tool_id)
                    if entry["installable"] and entry["status"] == "verified" and PLATFORM in entry["platforms"]:
                        install(lock, tool_id, done)
                    else:
                        skipped.append(tool_id)
                if skipped:
                    print("not installed automatically (owner-installed, unverified or other platform): " + ", ".join(skipped))
            else:
                install(lock, args.tool_id, done)
            return 0
        if args.command == "install-skill":
            install_skill(args.skill_id)
            return 0
        if args.command == "pin":
            pin(lock, args.tool_id)
            return 0
        if args.command == "approve":
            approve()
            return 0
    except Refused as exc:
        print(f"refused: {exc}", file=sys.stderr)
        return 2
    return 1


if __name__ == "__main__":
    sys.exit(main())
