"""Hermetic HTTPS stand-in for the ChatGPT backend that Codex 0.159.0 talks to when signed in with ChatGPT.

Serves an account whose installed plugins include codex-security from the openai-curated-remote
marketplace, the plugin bundle, and a one-message SSE model reply. Logs every request it receives."""
import json
import ssl
import sys
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlparse

PORT, CERTS, BUNDLE, LOG = int(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3]), Path(sys.argv[4])
ORIGIN = f"https://127.0.0.1:{PORT}"
PLUGIN_ID = "plugins~Plugin_00000000000000000000000000000000"
REVIEW = {"verdict": "pass", "summary": "mock reply", "findings": []}


def plugin_item(enabled: bool | None = True) -> dict:
    item = {"id": PLUGIN_ID, "name": "codex-security", "scope": "GLOBAL", "discoverability": None,
            "installation_policy": "AVAILABLE", "authentication_policy": "ON_USE",
            "release": {"version": "0.1.32", "display_name": "Codex Security", "description": "stand-in",
                        "bundle_download_url": f"{ORIGIN}/bundle/codex-security.tar.gz", "interface": {}}}
    if enabled is not None:
        item["enabled"] = enabled
    return item


def sse(events: list[dict]) -> bytes:
    return "".join(f"event: {e['type']}\ndata: {json.dumps(e)}\n\n" for e in events).encode()


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *_):
        pass

    def record(self, status: int, body_len: int) -> None:
        u = urlparse(self.path)
        with LOG.open("a", encoding="utf-8") as f:
            f.write(json.dumps({"t": time.time(), "method": self.command, "path": u.path,
                                "query": sorted(parse_qs(u.query)), "status": status,
                                "authorization": "authorization" in {k.lower() for k in self.headers},
                                "request_bytes": int(self.headers.get("content-length") or 0),
                                "response_bytes": body_len}) + "\n")

    def send(self, status: int, body: bytes, ctype: str = "application/json") -> None:
        self.send_response(status)
        self.send_header("content-type", ctype)
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)
        self.record(status, len(body))

    def handle_any(self) -> None:
        length = int(self.headers.get("content-length") or 0)
        if length:
            self.rfile.read(length)
        path = urlparse(self.path).path
        if path == "/backend-api/ps/plugins/installed":
            body = {"plugins": [plugin_item()], "pagination": {"limit": 200, "next_page_token": None}}
            return self.send(200, json.dumps(body).encode())
        if path == f"/backend-api/ps/plugins/{PLUGIN_ID}":
            return self.send(200, json.dumps(plugin_item(None)).encode())
        if path.startswith("/backend-api/ps/plugins"):
            return self.send(200, json.dumps({"plugins": [], "pagination": {"limit": 200, "next_page_token": None}}).encode())
        if path == "/backend-api/wham/accounts/check":
            body = {"accounts": [{"id": "account-123", "workspace_backend_origin": ORIGIN,
                                  "account_routing_override": "NO_CONSTRAINT"}]}
            return self.send(200, json.dumps(body).encode())
        if path == "/bundle/codex-security.tar.gz":
            return self.send(200, BUNDLE.read_bytes(), "application/gzip")
        if path == "/backend-api/codex/responses" and self.command == "POST":
            events = [{"type": "response.created", "response": {"id": "resp-1"}},
                      {"type": "response.output_item.done", "item": {
                          "type": "message", "role": "assistant", "id": "msg-1",
                          "content": [{"type": "output_text", "text": json.dumps(REVIEW)}]}},
                      {"type": "response.completed", "response": {"id": "resp-1", "usage": {
                          "input_tokens": 1, "input_tokens_details": None, "output_tokens": 1,
                          "output_tokens_details": None, "total_tokens": 2}}}]
            return self.send(200, sse(events), "text/event-stream")
        return self.send(404, b'{"detail":"not found"}')

    do_GET = do_POST = do_PUT = do_DELETE = do_PATCH = handle_any


ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
ctx.load_cert_chain(CERTS / "srv.pem", CERTS / "srv.key")
server = ThreadingHTTPServer(("127.0.0.1", PORT), Handler)
server.socket = ctx.wrap_socket(server.socket, server_side=True)
print(f"mock backend on {ORIGIN}", flush=True)
server.serve_forever()
