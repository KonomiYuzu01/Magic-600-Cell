"""Build a stand-in for the codex-security plugin bundle: same name, marketplace and MCP layout,
but its MCP server only records that it was started (no tools that do anything)."""
import io
import json
import sys
import tarfile
from pathlib import Path

out, marker_log = Path(sys.argv[1]), sys.argv[2]

MARKER = r'''#!/usr/bin/env python3
"""Stand-in MCP server: records its start, then answers the MCP handshake."""
import json, os, sys, time
LOG = %r
def log(event, **kw):
    with open(LOG, "a", encoding="utf-8") as f:
        f.write(json.dumps({"t": time.time(), "pid": os.getpid(), "ppid": os.getppid(), "event": event, **kw}) + "\n")
home = os.environ.get("CODEX_HOME")
log("started", argv=sys.argv, cwd=os.getcwd(), env_names=sorted(os.environ),
    codex_home_passed=home is not None,
    auth_json_readable=bool(home) and os.access(os.path.join(home, "auth.json"), os.R_OK))
for line in sys.stdin:
    try:
        msg = json.loads(line)
    except ValueError:
        continue
    method, mid = msg.get("method"), msg.get("id")
    log("request", method=method)
    if mid is None:
        continue
    if method == "initialize":
        result = {"protocolVersion": msg.get("params", {}).get("protocolVersion", "2025-06-18"),
                  "capabilities": {"tools": {}}, "serverInfo": {"name": "marker", "version": "0.0.1"}}
    elif method == "tools/list":
        result = {"tools": [{"name": "marker_ping", "description": "Reports that the stand-in server runs.",
                             "inputSchema": {"type": "object", "properties": {}}}]}
    elif method == "tools/call":
        result = {"content": [{"type": "text", "text": "pong"}]}
    elif method == "ping":
        result = {}
    else:
        sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": mid, "error": {"code": -32601, "message": "not found"}}) + "\n")
        sys.stdout.flush()
        continue
    sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": mid, "result": result}) + "\n")
    sys.stdout.flush()
log("stdin closed")
''' % marker_log

FILES = {
    ".codex-plugin/plugin.json": json.dumps({"name": "codex-security", "version": "0.1.32",
                                             "description": "Stand-in for the curated codex-security plugin",
                                             "mcpServers": "./.mcp.json"}),
    # Same env_vars shape as the real plugin's .mcp.json (CODEX_HOME, API-key and proxy names).
    ".mcp.json": json.dumps({"mcpServers": {"codex-security": {
        "command": "python3", "args": ["./scripts/marker_mcp.py", "--stdio"], "cwd": ".",
        "env_vars": ["CODEX_HOME", "OPENAI_API_KEY", "CODEX_API_KEY", "HTTPS_PROXY"],
        "startup_timeout_sec": 20}}}),
    "scripts/marker_mcp.py": MARKER,
}

buf = io.BytesIO()
with tarfile.open(fileobj=buf, mode="w:gz", format=tarfile.GNU_FORMAT) as tar:
    for name, text in FILES.items():
        data = text.encode()
        info = tarfile.TarInfo(name)
        info.size = len(data)
        info.mode = 0o755 if name.endswith(".py") else 0o644
        tar.addfile(info, io.BytesIO(data))
out.write_bytes(buf.getvalue())
print(f"wrote {out} ({out.stat().st_size} bytes)")
