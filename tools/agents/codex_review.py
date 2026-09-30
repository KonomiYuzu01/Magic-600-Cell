"""Run a read-only Codex plan check or candidate review through one audited path.

  python tools/agents/codex_review.py --kind plan --packet <file>
  python tools/agents/codex_review.py --kind review --packet <file>
  python tools/agents/codex_review.py --kind review --packet <file> --effort ultra
  python tools/agents/codex_review.py --kind review --packet <file> --model gpt-6-astra
  python tools/agents/codex_review.py --kind review --packet <file> --speed fast
  python tools/agents/codex_review.py --kind review --packet <file> --model gpt-6-astra --effort ultra --gate day7-go-no-go

The wrapper passes the model, effort and speed tier explicitly, always uses the
read-only sandbox and accepts no other Codex arguments. A run is invalid when
the model, effort or sandbox that Codex reports differs from the request, or
when Codex drops the requested speed tier. Outputs go to work/reviews/<call-id>/
and one private ledger line is appended per call.

Exit codes: 0 valid result, 2 refused or invalid run, 3 timeout.
"""
from __future__ import annotations

import argparse
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
from repo_digest import ROOT, file_sha256, source_identity  # noqa: E402

SCHEMA_PATH = ROOT / "schemas" / "review-result.schema.json"
BRIEFING_PATH = ROOT / "docs" / "development-guide" / "AGENT_BRIEFING.md"
REVIEWS = ROOT / "work" / "reviews"
LEDGER = ROOT / "work" / "loop-memory" / "ledgers" / "codex.jsonl"

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
}


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
    return [exe]


def build_prompt(kind: str, packet_text: str, gate: str | None) -> str:
    parts = [INSTRUCTIONS, KIND_FOCUS[kind]]
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


def run_codex(cmd: list[str], prompt: str, timeout: float) -> tuple[int | None, str, str]:
    kwargs = {"start_new_session": True} if os.name != "nt" else {"creationflags": subprocess.CREATE_NEW_PROCESS_GROUP}
    proc = subprocess.Popen(cmd, cwd=ROOT, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
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


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--kind", required=True, choices=sorted(KIND_FOCUS))
    parser.add_argument("--packet", required=True, type=Path)
    parser.add_argument("--model", default=DEFAULT_MODEL, choices=MODELS)
    parser.add_argument("--effort", default="max", choices=EFFORTS)
    parser.add_argument("--speed", choices=sorted(SPEEDS), default="standard", help="fast for scoped verification rounds, plan re-checks and other latency-sensitive calls; never for gate rulings")
    parser.add_argument("--gate", choices=GATES)
    parser.add_argument("--timeout", type=int, default=3600, help="seconds (60-7200)")
    args = parser.parse_args(argv)

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
        packet = args.packet.resolve()
        if not packet.is_file():
            raise Refused(f"packet not found: {args.packet}")
        packet_text = packet.read_text(encoding="utf-8")
        record["packet_sha256"] = hashlib.sha256(packet_text.encode()).hexdigest()
        try:
            identity = source_identity()
            record["source_identity"] = identity["digest"]
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
        with tempfile.TemporaryDirectory() as td:
            cmd = [*base_cmd, "exec", "-m", args.model, "-c", f'model_reasoning_effort="{args.effort}"',
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
