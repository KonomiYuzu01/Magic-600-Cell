"""Run a Codex plan check, candidate review or packet implementation through one audited path.

  python tools/agents/codex_review.py --kind plan --packet <file>
  python tools/agents/codex_review.py --kind review --packet <file>
  python tools/agents/codex_review.py --kind implement --packet <file>
  python tools/agents/codex_review.py --cleanup <call-id> [<call-id> ...]
  python tools/agents/codex_review.py --kind review --packet <file> --effort ultra
  python tools/agents/codex_review.py --kind review --packet <file> --model gpt-6-astra
  python tools/agents/codex_review.py --kind review --packet <file> --speed fast
  python tools/agents/codex_review.py --kind review --packet <file> --model gpt-6-astra --effort ultra --gate day7-go-no-go

The wrapper passes the model, effort and speed tier explicitly and accepts no
other Codex arguments. Plan checks and reviews use the read-only sandbox. An
implementation runs with the workspace-write sandbox in a new worktree that the
wrapper creates at work/worktrees/<call-id>/ on branch codex/<call-id>, never in
the main worktree; afterwards the wrapper rejects any change outside the packet's
allowed files and any commit, ref, config, hook or link change, stores the patch
and runs the packet's acceptance check there inside the Codex sandbox. It never merges: the integrator
reviews changes.patch and applies it. A run is invalid when the model, effort or
sandbox that Codex reports differs from the request, or when Codex drops the
requested speed tier. Outputs go to work/reviews/<call-id>/ and one private
ledger line is appended per call.

Exit codes: 0 valid result, 2 refused or invalid run, 3 timeout,
4 valid implementation whose acceptance check failed.
"""
from __future__ import annotations

import argparse
import fnmatch
import hashlib
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import uuid
from datetime import datetime, timezone
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from repo_digest import ROOT, _is_link, file_sha256, source_identity  # noqa: E402

SCHEMA_PATH = ROOT / "schemas" / "review-result.schema.json"
BRIEFING_PATH = ROOT / "docs" / "development-guide" / "AGENT_BRIEFING.md"
REVIEWS = ROOT / "work" / "reviews"
LEDGER = ROOT / "work" / "loop-memory" / "ledgers" / "codex.jsonl"
WORKTREES = ROOT / "work" / "worktrees"
CRITICAL_PATHS = ROOT / "tools" / "agents" / "critical_paths.json"

MODELS = ("gpt-6.1-sol", "gpt-6-astra")
DEFAULT_MODEL = "gpt-6.1-sol"
EFFORTS = ("high", "max", "ultra")
GATES = ("day7-go-no-go", "migration-format-freeze", "architecture-freeze")
# Every run sets the tier explicitly so an inherited user preference cannot change it.
SPEEDS = {"standard": "default", "fast": "fast"}
HEADER_RE = {
    "model": re.compile(r"^model:\s*(\S+)", re.M),
    "effort": re.compile(r"^reasoning effort:\s*(\S+)", re.M),
    "sandbox": re.compile(r"^sandbox:\s*(\S+)", re.M),
    "provider": re.compile(r"^provider:\s*(\S+)", re.M),
}
# Reviews run on the owner's ChatGPT subscription only; API-key or custom-provider routing could incur paid usage.
SUBSCRIPTION_OVERRIDES = ["-c", 'model_provider="openai"', "-c", 'forced_login_method="chatgpt"']
# Every call ignores ~/.codex/config.toml, which can set a full-access sandbox, writable roots, and MCP servers and
# plugins that run outside any sandbox. Sign-in still comes from CODEX_HOME; this needs the default file-backed
# credential store (auth.json), since a keyring selection in the ignored file would be dropped too. On Windows the
# sandbox backend must then be named, or Codex falls back to a mode that rejects every command, even file reads.
# The unelevated backend needs no administrator setup; it enforces filesystem boundaries but not a network firewall.
USER_CONFIG_ISOLATION = ["--ignore-user-config"] + (["-c", 'windows.sandbox="unelevated"'] if os.name == "nt" else [])
# Anchored to the CLI's own warning line: file contents Codex reads also appear in its log.
TIER_DROPPED_RE = re.compile(r"^warning: Configured service tier `[^`]+` is not advertised", re.M)

INSTRUCTIONS = """You are the independent Codex reviewer for Magic 600 Cell.
Review only. Do not edit files, run write operations or perform any follow-up work after answering.
Read AGENTS.md and the files the packet names. Cite evidence as repository paths with line numbers.
Separate verified facts from inference. Mark each finding verified or unverified.
Severity: blocker (must not ship), major (should fix before commit), minor, nit.
Return only JSON that matches the provided output schema.
"""
KIND_FOCUS = {
    "plan": "Task: check this plan before implementation. Look for wrong assumptions, missing acceptance checks, unsafe order, scope creep and cheaper falsifying experiments.",
    "review": "Task: review the current candidate described in the packet and present in the working tree. Look for correctness, invariant, persistence, security and evidence-claim problems.",
    "implement": "",
}
IMPLEMENT_INSTRUCTIONS = """You are the Codex co-implementer for Magic 600 Cell. The current directory is a Git worktree created for this packet only.
Change only files whose repository-relative path matches one of the allowed globs in the contract below, and work only inside the current directory.
Do not commit, stage, stash, tag, branch, push or fetch; do not change any Git configuration, hook or the .git file; do not create symbolic links, junctions or nested repositories; do not use the network.
Read AGENTS.md and the files the packet names. Run the acceptance check before you finish. Stop when the stop condition holds and do no work beyond the packet.
Final message: the files you changed, what changed and why, the acceptance check result and anything left open.
"""
CALL_ID_RE = re.compile(r"^\d{8}T\d{6}Z-[0-9a-f]{8}$")
CONTRACT_RE = re.compile(r"^```implement-contract[ \t]*\r?\n(.*?)^```[ \t]*$", re.M | re.S)
ACCEPTANCE_TIMEOUT = 1800
SANDBOX_PROFILE = "magic600-implement"
SANDBOX_LINE_RE = re.compile(r"^sandbox:\s*(.+)$", re.M)
SANDBOX_ROOTS = {"workdir", "/tmp", "$TMPDIR"}  # as Codex reports them: "workspace-write [workdir, /tmp, $TMPDIR]"
# The Codex sandbox leaves the temp directory writable, so a repository there would not be protected.
SANDBOX_DEFAULT_WRITABLE = (Path(tempfile.gettempdir()),)
CHILD_GIT_CONFIG = {"protocol.allow": "never", "core.hooksPath": os.devnull}
# Every wrapper Git command runs without repository hooks or a filesystem monitor.
SAFE_GIT = ["-c", "core.hooksPath=" + os.devnull, "-c", "core.fsmonitor=false"]


class Refused(Exception):
    pass


def shown(path: Path) -> str:
    try:
        return path.relative_to(ROOT).as_posix()
    except ValueError:
        return str(path)


def validate_result(data, schema: dict, where: str = "$") -> list[str]:
    """Small validator for the subset of JSON Schema used by the review schema."""
    errors = []
    types = schema.get("type")
    if types is not None:
        allowed = types if isinstance(types, list) else [types]
        py = {"object": dict, "array": list, "string": str, "integer": int, "number": (int, float), "null": type(None), "boolean": bool}
        if not any(isinstance(data, py[t]) and not (t in ("integer", "number") and isinstance(data, bool)) for t in allowed):
            return [f"{where}: expected {allowed}"]
    if "enum" in schema and data not in schema["enum"]:
        errors.append(f"{where}: {data!r} not in {schema['enum']}")
    if isinstance(data, dict):
        props = schema.get("properties", {})
        for key in schema.get("required", []):
            if key not in data:
                errors.append(f"{where}: missing {key}")
        if schema.get("additionalProperties") is False:
            errors += [f"{where}: unexpected {k}" for k in data if k not in props]
        for key, sub in props.items():
            if key in data:
                errors += validate_result(data[key], sub, f"{where}.{key}")
    if isinstance(data, list) and "items" in schema:
        for i, item in enumerate(data):
            errors += validate_result(item, schema["items"], f"{where}[{i}]")
    return errors


def codex_command() -> list[str]:
    """The Codex executable; tests substitute a fake through MAGIC600_CODEX_CMD (a JSON list)."""
    override = os.environ.get("MAGIC600_CODEX_CMD")
    if override:
        cmd = json.loads(override)
        if not (isinstance(cmd, list) and cmd and all(isinstance(c, str) for c in cmd)):
            raise Refused("MAGIC600_CODEX_CMD must be a JSON list of strings")
        return cmd
    exe = shutil.which("codex")
    if not exe:
        raise Refused("codex CLI not found; run python tools/toolchain/bootstrap.py doctor")
    return [native_codex(Path(exe))]


def native_codex(exe: Path) -> str:
    """The native executable behind npm's codex.cmd: cmd.exe would reinterpret &, | and ^ in forwarded arguments."""
    if exe.suffix.lower() not in (".cmd", ".bat"):
        return str(exe)
    found = sorted((exe.parent / "node_modules" / "@openai" / "codex").glob("**/vendor/*/bin/codex.exe"))
    if not found:
        raise Refused(f"{exe.name} is a batch launcher and no native codex.exe was found beside it")
    return str(found[0])


def build_prompt(kind: str, packet_text: str, gate: str | None) -> str:
    parts = [IMPLEMENT_INSTRUCTIONS] if kind == "implement" else [INSTRUCTIONS, KIND_FOCUS[kind]]
    if gate:
        parts.append(f"This is the final ruling for the '{gate}' gate. State a clear go or no-go in the summary.")
    if BRIEFING_PATH.is_file():
        parts.append(f"Current agent briefing: docs/development-guide/AGENT_BRIEFING.md (sha256 {file_sha256(BRIEFING_PATH)}). Read it.")
    parts.append("----- PACKET -----\n" + packet_text)
    return "\n\n".join(parts)


def strict_schema_file(tmpdir: Path) -> Path:
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    for key in ("$schema", "title", "description"):
        schema.pop(key, None)
    path = tmpdir / "schema.json"
    path.write_text(json.dumps(schema), encoding="utf-8")
    return path


def run_codex(cmd: list[str], prompt: str, timeout: float, cwd: Path | None = None,
              env: dict | None = None) -> tuple[int | None, str, str]:
    return run_bounded(cmd, prompt, timeout, cwd, env)


def run_bounded(cmd: list[str], prompt: str, timeout: float, cwd: Path | None = None,
                env: dict | None = None) -> tuple[int | None, str, str]:
    """Run a process tree with a deadline; the exit code is None after a timeout."""
    kwargs = {"start_new_session": True} if os.name != "nt" else {"creationflags": subprocess.CREATE_NEW_PROCESS_GROUP}
    proc = subprocess.Popen(cmd, cwd=cwd or ROOT, env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                            text=True, encoding="utf-8", errors="replace", **kwargs)
    try:
        out, err = proc.communicate(prompt, timeout=timeout)
        return proc.returncode, out, err
    except subprocess.TimeoutExpired:
        if os.name != "nt":
            os.killpg(proc.pid, signal.SIGKILL)
        else:
            subprocess.run(["taskkill", "/F", "/T", "/PID", str(proc.pid)], capture_output=True)
        out, err = proc.communicate()
        return None, out, err


def parse_header(text: str) -> dict:
    found = {}
    for key, rx in HEADER_RE.items():
        m = rx.search(text)
        found[key] = m.group(1) if m else None
    return found


def parse_contract(packet_text: str) -> dict:
    """The implement-contract block of an implementation packet; refuses an incomplete or unsafe contract."""
    blocks = CONTRACT_RE.findall(packet_text)
    if len(blocks) != 1:
        raise Refused("an implementation packet needs exactly one ```implement-contract block")
    try:
        contract = json.loads(blocks[0])
    except ValueError:
        raise Refused("the implement-contract block is not JSON")
    keys = {"allowed_files", "acceptance_check", "stop_condition"}
    if not isinstance(contract, dict) or set(contract) != keys:
        raise Refused("the contract needs exactly allowed_files, acceptance_check and stop_condition")
    globs, check, stop = contract["allowed_files"], contract["acceptance_check"], contract["stop_condition"]
    if not (isinstance(globs, list) and globs and all(isinstance(g, str) and g.strip() for g in globs)):
        raise Refused("allowed_files must be a non-empty list of globs")
    for g in globs:
        if (g.startswith("/") or "\\" in g or ":" in g or ".." in g.split("/")
                or any(fnmatch.fnmatchcase(p, g) for p in (".git", ".git/config", ".GIT"))):
            raise Refused(f"unsafe allowed_files glob: {g!r}")
    if not (isinstance(check, list) and check and all(isinstance(a, str) and a for a in check)):
        raise Refused("acceptance_check must be a non-empty argv list")
    if not (isinstance(stop, str) and stop.strip()):
        raise Refused("stop_condition must be a non-empty string")
    return contract


def git(cwd: Path, *args: str, env: dict | None = None) -> str:
    try:
        r = subprocess.run(["git", *SAFE_GIT, *args], cwd=cwd, env=env, capture_output=True, timeout=300)
    except (OSError, subprocess.SubprocessError) as exc:
        raise Refused(f"git {args[0]} failed: {type(exc).__name__}")
    if r.returncode != 0:
        raise Refused(f"git {args[0]} failed: {r.stderr.decode('utf-8', 'replace').strip()[:300]}")
    return r.stdout.decode("utf-8", "surrogateescape")


def path_digest(path: Path, skip: tuple[str, ...] = ()) -> str | None:
    """Digest of a file or directory tree that records links as links and never follows them."""
    if not os.path.lexists(path):
        return None
    if _is_link(path):
        return "link:" + os.readlink(path)
    if path.is_file():
        return file_sha256(path)
    entries = []
    for dirpath, dirs, files in os.walk(path):
        for name in dirs + files:
            p = Path(dirpath, name)
            rel = p.relative_to(path).as_posix()
            if rel in skip:
                continue
            if _is_link(p):
                entries.append(f"{rel}\0link\0{os.readlink(p)}")
            elif p.is_file():
                entries.append(f"{rel}\0{file_sha256(p)}")
            else:
                entries.append(f"{rel}\0dir")
        dirs[:] = [d for d in dirs if not _is_link(Path(dirpath, d))]
    return hashlib.sha256("\n".join(sorted(entries)).encode("utf-8", "surrogateescape")).hexdigest()


def git_state(wt: Path, call_id: str, paths: dict) -> dict:
    """Everything outside the working files that an implementation must not change."""
    refs, others = {}, {}
    for line in git(wt, "for-each-ref", "--format=%(refname)%00%(objectname)%00%(symref)").splitlines():
        name, obj, sym = line.split("\0")
        other = name.startswith("refs/heads/codex/") and name != f"refs/heads/codex/{call_id}"
        (others if other else refs)[name] = f"{obj} {sym}".strip()
    return {
        "worktree git directory (commit, checkout, reset or per-worktree ref)": path_digest(paths["gitdir"], skip=("index",)),
        "git config": path_digest(paths["common"] / "config"),
        "git hooks": path_digest(paths["common"] / "hooks"),
        "git info (exclude, attributes)": path_digest(paths["common"] / "info"),
        "main worktree HEAD": path_digest(paths["root_gitdir"] / "HEAD"),
        "refs (branches, tags, remotes, stash)": refs,
        "other codex branches": others,
    }


def integrity_problems(wt: Path, call_id: str, paths: dict, before: dict) -> list[str]:
    """Git-state and link changes since `before`; the .git pointer is checked first so git cannot be redirected."""
    pointer = wt / ".git"
    if _is_link(pointer) or not pointer.is_file() or pointer.read_bytes() != paths["pointer"]:
        return ["the worktree .git file changed"]
    problems = []
    after = git_state(wt, call_id, paths)
    for key, value in before.items():
        if key == "other codex branches":
            # A parallel call creates its own branch together with its worktree, and cleanup deletes one; nothing else may move them.
            bad = [n for n, v in after[key].items() if (n in value and v != value[n])
                   or (n not in value and not (WORKTREES / n[len("refs/heads/codex/"):]).is_dir())]
            if bad:
                problems.append(f"{key} changed: {', '.join(sorted(bad))}")
        elif key.startswith("refs"):
            changed = set(value.items()) ^ set(after[key].items())
            if changed:
                problems.append(f"{key} changed: {', '.join(sorted({name for name, _ in changed}))}")
        elif after[key] != value:
            problems.append(f"{key} changed")
    for dirpath, dirs, files in os.walk(wt):
        for name in dirs + files:
            p = Path(dirpath, name)
            rel = p.relative_to(wt).as_posix()
            if rel.lower() == ".git":
                continue
            if _is_link(p):
                problems.append(f"link created: {rel}")
            elif name.lower() == ".git":
                problems.append(f"nested repository: {rel}")
        dirs[:] = [d for d in dirs if not _is_link(Path(dirpath, d)) and d.lower() != ".git"]
    return problems


def snapshot_env(index: Path) -> dict:
    # No system attributes file (Git for Windows maps documents to a textconv there); see snapshot_git.
    return dict(os.environ, GIT_INDEX_FILE=str(index), GIT_ATTR_NOSYSTEM="1")


def snapshot_git(base: str) -> list[str]:
    """Attributes come only from the base commit and the integrator's info/attributes, never from candidate
    files, and create_worktree refuses a base that declares a filter; so staging runs no candidate code."""
    return [f"--attr-source={base}", "-c", "core.attributesFile=" + os.devnull]


def snapshot_changes(wt: Path, base: str, index: Path) -> tuple[list[str], str]:
    """Stage the whole worktree, ignored files included, into a fresh index, so neither the worktree's own
    index flags nor ignore rules can hide a change. Returns the changed leaf paths and the tree ID."""
    env, opts = snapshot_env(index), snapshot_git(base)
    git(wt, *opts, "read-tree", base, env=env)
    git(wt, *opts, "add", "--all", "--force", "--", ".", env=env)
    names = git(wt, *opts, "-c", "core.quotePath=false", "diff", "--cached", "--no-renames", "--no-textconv",
                "--name-only", "-z", base, env=env)
    return sorted(filter(None, names.split("\0"))), git(wt, *opts, "write-tree", env=env).strip()


def worktree_patch(wt: Path, base: str, index: Path) -> bytes:
    r = subprocess.run(["git", *SAFE_GIT, *snapshot_git(base), "diff", "--cached", "--no-renames", "--binary", "--no-color",
                        "--no-ext-diff", "--no-textconv", base], cwd=wt, env=snapshot_env(index), capture_output=True, timeout=300)
    if r.returncode != 0:
        raise Refused("git diff failed")
    return r.stdout


def patch_tree(wt: Path, base: str, patch: Path, index: Path) -> str:
    """Tree ID of the base with the exported patch applied in a separate index; it must equal the snapshot."""
    env, opts = snapshot_env(index), snapshot_git(base)
    git(wt, *opts, "read-tree", base, env=env)
    if patch.stat().st_size:
        git(wt, *opts, "apply", "--cached", "--binary", str(patch), env=env)
    return git(wt, *opts, "write-tree", env=env).strip()


def child_env() -> dict:
    """Environment for Codex and the acceptance check: no bytecode caches, no Git transport (so no push or
    fetch, even to a local path) and no repository hooks."""
    env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1", GIT_CONFIG_COUNT=str(len(CHILD_GIT_CONFIG)))
    for i, (key, value) in enumerate(CHILD_GIT_CONFIG.items()):
        env[f"GIT_CONFIG_KEY_{i}"], env[f"GIT_CONFIG_VALUE_{i}"] = key, value
    return env


def implement_command(base_cmd: list[str], args, wt: Path, out_dir: Path) -> list[str]:
    cmd = [*base_cmd, "exec", *USER_CONFIG_ISOLATION, "-m", args.model, "-c", f'model_reasoning_effort="{args.effort}"',
           "-c", f'service_tier="{SPEEDS[args.speed]}"', *SUBSCRIPTION_OVERRIDES,
           "-c", "sandbox_workspace_write.writable_roots=[]", "-c", "sandbox_workspace_write.network_access=false"]
    for key, value in child_env().items():
        if key == "PYTHONDONTWRITEBYTECODE" or key.startswith("GIT_CONFIG_"):
            # Set explicitly: Codex's default shell policy drops variables whose names contain KEY.
            cmd += ["-c", f"shell_environment_policy.set.{key}={json.dumps(value)}"]
    return cmd + ["--sandbox", "workspace-write", "-C", str(wt), "-o", str(out_dir / "report.md"), "-"]


def sandbox_command(base_cmd: list[str], wt: Path, argv: list[str]) -> list[str]:
    """The acceptance check runs Codex-authored code, so it runs in the Codex sandbox: writes only in the
    worktree and the temp directory, and no network where the platform backend enforces that."""
    p = SANDBOX_PROFILE
    return [*base_cmd, "sandbox", "-P", p, "-c", f'permissions.{p}.extends=":workspace"',
            "-c", f"permissions.{p}.network.enabled=false", "-C", str(wt), "--", *argv]


def sandbox_env() -> dict:
    """The sandbox runner needs no sign-in, so it gets a Codex home with no config.toml: no user or profile layer
    can add writable roots to the acceptance profile. The home keeps only the runner's own sandbox state."""
    home = WORKTREES.parent / "codex-sandbox-home"
    home.mkdir(parents=True, exist_ok=True)
    if (home / "config.toml").exists():
        raise Refused(f"{shown(home)}/config.toml must not exist")
    return dict(child_env(), CODEX_HOME=str(home))


def run_implement(args, record: dict, call_id: str, out_dir: Path, base_cmd: list[str], packet_text: str,
                  contract: dict, wt: Path, base: str, paths: dict) -> int:
    meta = {"call_id": call_id, "kind": "implement", "gate": None, "requested": record["requested"],
            "resolved": record["resolved"], "sandbox_line": None, "packet_sha256": record["packet_sha256"], "base": base,
            "branch": f"codex/{call_id}", "worktree": shown(wt), "contract": contract, "main_uncommitted": [],
            "valid": False, "problems": [], "changed_files": [], "critical_paths_touched": [], "acceptance": None}

    def finish(outcome: str, exit_code: int, problems=()) -> int:
        record["outcome"] = outcome
        meta["problems"] = list(problems)
        (out_dir / "meta.json").write_text(json.dumps(meta, indent=2) + "\n", encoding="utf-8")
        if problems:
            print(("timeout: " if exit_code == 3 else "invalid run: ") + "; ".join(problems), file=sys.stderr)
        return exit_code

    try:
        return implement_steps(args, record, meta, finish, call_id, out_dir, base_cmd, packet_text, contract, wt, base, paths)
    except Exception as exc:  # always leave a terminal record, so the worktree stays removable with --cleanup
        return finish("invalid", 2, [f"wrapper error: {type(exc).__name__}: {exc}"])


def implement_steps(args, record, meta, finish, call_id, out_dir, base_cmd, packet_text, contract, wt, base, paths) -> int:
    globs = contract["allowed_files"]
    meta["main_uncommitted"] = [line[3:] for line in git(ROOT, "status", "--porcelain=v1").splitlines() if len(line) > 3]
    before = git_state(wt, call_id, paths)
    code, out, err = run_codex(implement_command(base_cmd, args, wt, out_dir), build_prompt("implement", packet_text, None),
                               args.timeout, cwd=wt, env=child_env())
    (out_dir / "codex.log").write_text(err, encoding="utf-8")
    header = parse_header(err + "\n" + out)
    record["resolved"] = meta["resolved"] = header
    line = SANDBOX_LINE_RE.search(err + "\n" + out)
    meta["sandbox_line"] = line.group(1).strip() if line else None
    if code is None:
        return finish("timeout", 3, [f"timeout after {args.timeout}s; see {shown(out_dir)}/codex.log"])
    problems = []
    if code != 0:
        problems.append(f"codex exited {code}")
    for key, want in (("model", args.model), ("effort", args.effort), ("sandbox", "workspace-write"), ("provider", "openai")):
        if header[key] != want:
            problems.append(f"{key} {header[key]} != {want}")
    roots = re.fullmatch(r"workspace-write \[([^\]]*)\]", meta["sandbox_line"] or "")
    if not roots or not {r.strip() for r in roots.group(1).split(",")} <= SANDBOX_ROOTS:
        problems.append(f"writable roots {meta['sandbox_line']!r} are not limited to the worktree and the temp directory")
    if TIER_DROPPED_RE.search(err):
        problems.append(f"speed tier {args.speed} was dropped by codex")
    with tempfile.TemporaryDirectory() as td:
        problems += integrity_problems(wt, call_id, paths, before)
        if any(p.startswith("the worktree .git file") for p in problems):
            return finish("invalid", 2, problems)
        try:
            changed, tree = snapshot_changes(wt, base, Path(td) / "index")
        except Refused as exc:
            return finish("invalid", 2, problems + [str(exc)])
        meta["changed_files"] = changed
        critical = json.loads(CRITICAL_PATHS.read_text(encoding="utf-8"))["patterns"] if CRITICAL_PATHS.is_file() else []
        meta["critical_paths_touched"] = [c for c in changed if any(fnmatch.fnmatchcase(c, p) for p in critical)]
        outside = [c for c in changed if not any(fnmatch.fnmatchcase(c, g) for g in globs)]
        if outside:
            problems.append("changed outside the allowed files: " + ", ".join(outside))
        # Snapshots read attributes from the base only, so a changed .gitattributes could make the exported bytes
        # differ from the bytes the acceptance check sees.
        attributes = [c for c in changed if c.rsplit("/", 1)[-1].lower() == ".gitattributes"]  # Windows: any case
        if attributes:
            problems.append("attribute files belong to the integrator: " + ", ".join(attributes))
        if problems:
            return finish("invalid", 2, problems)
        patch = worktree_patch(wt, base, Path(td) / "index")
        (out_dir / "changes.patch").write_bytes(patch)
        if patch_tree(wt, base, out_dir / "changes.patch", Path(td) / "check-index") != tree:
            return finish("invalid", 2, ["changes.patch applied to the base does not reproduce the candidate"])
    # The worktree now holds exactly base + changes.patch, so the acceptance check tests the exported candidate.
    argv = list(contract["acceptance_check"])
    if argv[0] == "python":
        argv[0] = sys.executable
    try:
        acc_code, acc_out, acc_err = run_bounded(sandbox_command(base_cmd, wt, argv), "", ACCEPTANCE_TIMEOUT,
                                                 cwd=wt, env=sandbox_env())
    except OSError as exc:
        acc_code, acc_out, acc_err = None, "", f"cannot start the acceptance check: {exc}\n"
    (out_dir / "acceptance.log").write_text(acc_out + ("\n" if acc_out and acc_err else "") + acc_err, encoding="utf-8")
    meta["acceptance"] = {"argv": contract["acceptance_check"], "exit_code": acc_code, "log": "acceptance.log"}
    problems = integrity_problems(wt, call_id, paths, before)
    if not any(p.startswith("the worktree .git file") for p in problems):
        with tempfile.TemporaryDirectory() as td:
            if snapshot_changes(wt, base, Path(td) / "index")[1] != tree:
                problems.append("the worktree changed during the acceptance check (outputs belong outside the worktree)")
    if problems:
        return finish("invalid", 2, [f"after the acceptance check: {p}" for p in problems])
    meta["valid"] = True
    passed = acc_code == 0
    print(f"implement {call_id}: {len(changed)} changed file(s); acceptance {'passed' if passed else f'failed ({acc_code})'}")
    if meta["critical_paths_touched"]:
        print("critical paths touched (also needs the other Codex model's review): " + ", ".join(meta["critical_paths_touched"]))
    print(f"patch: {shown(out_dir / 'changes.patch')}; worktree: {shown(wt)}")
    return finish("implemented", 0) if passed else finish("acceptance_failed", 4)


def create_worktree(call_id: str) -> tuple[Path, str, dict]:
    for writable in SANDBOX_DEFAULT_WRITABLE:
        a, b = os.path.normcase(os.path.realpath(ROOT)), os.path.normcase(os.path.realpath(writable))
        if os.path.splitdrive(a)[0] == os.path.splitdrive(b)[0] and os.path.commonpath([a, b]) == b:
            raise Refused(f"the repository is inside {writable}, which the Codex sandbox leaves writable")
    base = git(ROOT, "rev-parse", "--verify", "HEAD^{commit}").strip()
    common = Path(git(ROOT, "rev-parse", "--path-format=absolute", "--git-common-dir").strip())
    attributes = [git(ROOT, "show", f"{base}:{rel}") for rel in git(ROOT, "ls-tree", "-r", "--name-only", "-z", base).split("\0")
                  if rel == ".gitattributes" or rel.endswith("/.gitattributes")]
    if (common / "info" / "attributes").is_file():
        attributes.append((common / "info" / "attributes").read_text(encoding="utf-8", errors="replace"))
    if any(re.search(r"(^|\s)filter=", text, re.M) for text in attributes):
        raise Refused("the base declares a Git filter attribute; implementation snapshots never run filters")
    wt = WORKTREES / call_id
    WORKTREES.mkdir(parents=True, exist_ok=True)
    git(ROOT, "worktree", "add", "--quiet", "-b", f"codex/{call_id}", str(wt), base)
    paths = {"pointer": (wt / ".git").read_bytes(),
             "gitdir": Path(git(wt, "rev-parse", "--absolute-git-dir").strip()),
             "common": Path(git(wt, "rev-parse", "--path-format=absolute", "--git-common-dir").strip()),
             "root_gitdir": Path(git(ROOT, "rev-parse", "--absolute-git-dir").strip())}
    return wt, base, paths


def remove_links(tree: Path) -> None:
    """Remove every symlink and junction below a tree first, so deleting the tree can never reach their targets."""
    for dirpath, dirs, files in os.walk(tree):
        for name in dirs + files:
            p = Path(dirpath, name)
            if _is_link(p):
                try:
                    os.unlink(p)
                except OSError:
                    os.rmdir(p)
        dirs[:] = [d for d in dirs if os.path.lexists(Path(dirpath, d))]


def cleanup(call_ids: list[str]) -> int:
    """Remove finished implementation worktrees and their branches; review outputs stay."""
    code = 0
    for cid in call_ids:
        try:
            if not CALL_ID_RE.match(cid):
                raise Refused(f"not a call ID: {cid!r}")
            try:
                meta = json.loads((REVIEWS / cid / "meta.json").read_text(encoding="utf-8"))
            except (OSError, ValueError):
                raise Refused(f"{cid}: no finished implementation run")
            if not isinstance(meta, dict) or meta.get("kind") != "implement":
                raise Refused(f"{cid}: not an implementation run")
            wt = WORKTREES / cid
            for p in (ROOT / "work", WORKTREES, wt):
                if _is_link(p):
                    raise Refused(f"{shown(p)} is a link; nothing removed")
            registered = git(ROOT, "worktree", "list", "--porcelain", "-z").split("\0\0")
            entry = next((e for e in registered if e.startswith("worktree ")
                          and os.path.normcase(os.path.realpath(e.split("\0")[0][len("worktree "):])) == os.path.normcase(os.path.realpath(wt))), None)
            if os.path.lexists(wt):
                if entry is None or f"branch refs/heads/codex/{cid}" not in entry.split("\0"):
                    raise Refused(f"{shown(wt)} is not the registered worktree of codex/{cid}; nothing removed")
                remove_links(wt)
                git(ROOT, "worktree", "remove", "--force", "--force", str(wt))
            elif entry is not None:
                raise Refused(f"{shown(wt)} is registered but missing; check it, then run git worktree prune yourself")
            if git(ROOT, "branch", "--list", f"codex/{cid}").strip():
                git(ROOT, "branch", "-D", f"codex/{cid}")
            print(f"removed {cid}")
        except Refused as exc:
            print(f"refused: {exc}", file=sys.stderr)
            code = 2
    return code


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--kind", choices=sorted(KIND_FOCUS))
    parser.add_argument("--packet", type=Path)
    parser.add_argument("--cleanup", nargs="+", metavar="CALL_ID", help="remove finished implementation worktrees and their codex/<call-id> branches")
    parser.add_argument("--model", default=DEFAULT_MODEL, choices=MODELS)
    parser.add_argument("--effort", default="max", choices=EFFORTS)
    parser.add_argument("--speed", choices=sorted(SPEEDS), default="standard", help="fast for scoped verification rounds, plan re-checks and other latency-sensitive calls; never for gate rulings")
    parser.add_argument("--gate", choices=GATES)
    parser.add_argument("--timeout", type=int, default=3600, help="seconds (60-7200)")
    args = parser.parse_args(argv)
    if args.cleanup:
        if args.kind or args.packet:
            parser.error("--cleanup takes only call IDs")
        return cleanup(args.cleanup)
    if not (args.kind and args.packet):
        parser.error("--kind and --packet are required")

    call_id = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + uuid.uuid4().hex[:8]
    out_dir = REVIEWS / call_id
    record = {"call_id": call_id, "agent": "codex", "kind": args.kind, "channel": "subscription", "gate": args.gate,
              "requested": {"model": args.model, "effort": args.effort, "speed": args.speed},
              "resolved": {"model": None, "effort": None, "sandbox": None},
              "packet_sha256": None, "source_identity": None, "outcome": "error",
              "cost": {"status": "subscription", "usd": None}}
    called = False  # the ledger records model calls only, not requests refused before a call
    try:
        if args.gate and args.model != "gpt-6-astra":
            raise Refused("every --gate uses gpt-6-astra")
        if args.gate and args.effort != "ultra":
            raise Refused("gate rulings use --effort ultra")
        if args.speed == "fast" and args.gate:
            raise Refused("gate rulings use the standard tier")
        if not 60 <= args.timeout <= 7200:
            raise Refused("--timeout must be between 60 and 7200 seconds")
        if args.kind == "implement" and args.gate:
            raise Refused("an implementation is never a gate ruling")
        packet = args.packet.resolve()
        if not packet.is_file():
            raise Refused(f"packet not found: {args.packet}")
        packet_text = packet.read_text(encoding="utf-8")
        record["packet_sha256"] = hashlib.sha256(packet_text.encode()).hexdigest()
        contract = parse_contract(packet_text) if args.kind == "implement" else None
        try:
            identity = source_identity() if args.kind != "implement" else None
            record["source_identity"] = identity and identity["digest"]
        except (OSError, subprocess.SubprocessError):
            identity = None
            if args.kind == "review":
                raise Refused("cannot identify the current candidate (git failed); a review needs a source identity")
        base_cmd = codex_command()
        try:
            login = subprocess.run([*base_cmd, "login", "status"], capture_output=True, text=True, timeout=60)
        except (OSError, subprocess.SubprocessError) as exc:
            raise Refused(f"cannot check the Codex login: {type(exc).__name__}")
        if login.returncode != 0 or "chatgpt" not in (login.stdout + login.stderr).lower():
            raise Refused("Codex is not signed in with a ChatGPT subscription; paid API routing needs owner approval")
        out_dir.mkdir(parents=True, exist_ok=True)
        (out_dir / "packet.md").write_text(packet_text, encoding="utf-8")
        if args.kind == "implement":
            wt, base, paths = create_worktree(call_id)
            record["source_identity"] = base
            called = True
            return run_implement(args, record, call_id, out_dir, base_cmd, packet_text, contract, wt, base, paths)
        with tempfile.TemporaryDirectory() as td:
            cmd = [*base_cmd, "exec", *USER_CONFIG_ISOLATION, "-m", args.model, "-c", f'model_reasoning_effort="{args.effort}"',
                   "-c", f'service_tier="{SPEEDS[args.speed]}"', *SUBSCRIPTION_OVERRIDES]
            cmd += ["--sandbox", "read-only", "--output-schema", str(strict_schema_file(Path(td))),
                    "-o", str(out_dir / "review.json"), "-"]
            called = True
            code, out, err = run_codex(cmd, build_prompt(args.kind, packet_text, args.gate), args.timeout)
        (out_dir / "codex.log").write_text(err, encoding="utf-8")
        if code is None:
            record["outcome"] = "timeout"
            print(f"timeout after {args.timeout}s; see {shown(out_dir)}/codex.log", file=sys.stderr)
            return 3
        header = parse_header(err + "\n" + out)
        record["resolved"] = header
        problems = []
        if code != 0:
            problems.append(f"codex exited {code}")
        if header["model"] != args.model:
            problems.append(f"model {header['model']} != {args.model}")
        if header["effort"] != args.effort:
            problems.append(f"effort {header['effort']} != {args.effort}")
        if header["sandbox"] != "read-only":
            problems.append(f"sandbox {header['sandbox']} != read-only")
        if header["provider"] != "openai":
            problems.append(f"provider {header['provider']} != openai")
        if TIER_DROPPED_RE.search(err):
            problems.append(f"speed tier {args.speed} was dropped by codex")
        result = None
        review_path = out_dir / "review.json"
        if not problems:
            try:
                result = json.loads(review_path.read_text(encoding="utf-8"))
            except (OSError, ValueError):
                problems.append("review.json missing or not JSON")
            else:
                problems += validate_result(result, json.loads(SCHEMA_PATH.read_text(encoding="utf-8")))
                if not problems and result["verdict"] == "pass" and any(
                        f["severity"] in ("blocker", "major") for f in result["findings"]):
                    problems.append("verdict pass contradicts blocking findings")
        meta = {"call_id": call_id, "kind": args.kind, "gate": args.gate, "requested": record["requested"],
                "resolved": header, "packet_sha256": record["packet_sha256"],
                "source_identity": identity, "valid": not problems, "problems": problems,
                "verdict": result.get("verdict") if isinstance(result, dict) and not problems else None}
        (out_dir / "meta.json").write_text(json.dumps(meta, indent=2) + "\n", encoding="utf-8")
        if problems:
            record["outcome"] = "invalid"
            print("invalid run: " + "; ".join(problems), file=sys.stderr)
            return 2
        record["outcome"] = result["verdict"]
        blocking = [f["id"] for f in result["findings"] if f["severity"] in ("blocker", "major")]
        print(f"{args.kind} {call_id}: {result['verdict']}; {len(result['findings'])} finding(s), blocking: {', '.join(blocking) or 'none'}")
        print(f"result: {shown(review_path)}")
        return 0
    except Refused as exc:
        record["outcome"] = "invalid"
        print(f"refused: {exc}", file=sys.stderr)
        return 2
    finally:
        if called:
            write_ledger(record)


def write_ledger(record: dict) -> None:
    LEDGER.parent.mkdir(parents=True, exist_ok=True)
    record["time"] = datetime.now(timezone.utc).isoformat(timespec="seconds")
    with LEDGER.open("a", encoding="utf-8") as f:
        f.write(json.dumps(record) + "\n")


if __name__ == "__main__":
    sys.exit(main())
