"""Serves the Taste Lab folder on 127.0.0.1 for the local references page (owner design decision 4A).

    python tools/tastelab/refs/serve.py [--port N] [--no-open]

The page opens a folder of the owner's reference images through the browser's
file system access and keeps references.json in that folder. This server only
sends the page's own code: it listens on 127.0.0.1, answers GET and HEAD for
files under tools/tastelab/ (not through a link that leads elsewhere), lists no
directories and receives nothing.
"""
from __future__ import annotations

import argparse
import functools
import http.server
import sys
import webbrowser
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
HOST = "127.0.0.1"
PORT = 8613
CSP = ("default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' blob:; connect-src 'none'; "
       "base-uri 'none'; form-action 'none'; frame-ancestors 'none'")
# Explicit types: on Windows the registry can map .js to text/plain, which browsers refuse for modules.
TYPES = {".html": "text/html; charset=utf-8", ".js": "text/javascript; charset=utf-8",
         ".mjs": "text/javascript; charset=utf-8", ".css": "text/css; charset=utf-8",
         ".json": "application/json", ".svg": "image/svg+xml"}


class Handler(http.server.SimpleHTTPRequestHandler):
    extensions_map = {**http.server.SimpleHTTPRequestHandler.extensions_map, **TYPES}

    def do_GET(self):
        if self.path.split("?", 1)[0] == "/":
            self.send_response(302)
            self.send_header("Location", "/refs/")
            self.end_headers()
            return
        super().do_GET()

    def send_head(self):
        # A symbolic link or junction could lead outside the folder: serve only what resolves inside it.
        root = Path(self.directory).resolve()
        try:
            inside = Path(self.translate_path(self.path)).resolve().is_relative_to(root)
        except (OSError, ValueError):
            inside = False
        if not inside:
            self.send_error(404)
            return None
        return super().send_head()

    def end_headers(self):
        self.send_header("Content-Security-Policy", CSP)
        self.send_header("X-Content-Type-Options", "nosniff")
        self.send_header("Referrer-Policy", "no-referrer")
        self.send_header("Cache-Control", "no-store")
        super().end_headers()

    def list_directory(self, path):
        self.send_error(404)
        return None

    def log_message(self, format, *args):
        pass


class Server(http.server.ThreadingHTTPServer):
    # On Windows the option would let a second server bind the same port.
    allow_reuse_address = False
    daemon_threads = True


def make_server(port: int = PORT, root: Path = ROOT) -> Server:
    return Server((HOST, port), functools.partial(Handler, directory=str(root)))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Serve the Taste Lab references page on 127.0.0.1.")
    parser.add_argument("--port", type=int, default=PORT, help=f"port on 127.0.0.1 (default {PORT})")
    parser.add_argument("--no-open", action="store_true", help="do not open the page in the browser")
    args = parser.parse_args(argv)
    try:
        server = make_server(args.port)
    except OSError as err:
        print(f"Cannot listen on {HOST}:{args.port} ({err}). Choose another port with --port.", file=sys.stderr)
        return 2
    url = f"http://{HOST}:{server.server_address[1]}/refs/"
    print(f"Taste Lab references: {url}  (Ctrl+C stops the server)")
    if not args.no_open:
        webbrowser.open(url)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
