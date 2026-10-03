"""Runs the Taste Lab JavaScript tests (core, geometry, references), checks the geometry fixture and the references server."""
from __future__ import annotations

import http.client
import importlib.util
import os
import shutil
import subprocess
import sys
import tempfile
import threading
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
NODE = shutil.which("node")
SUITES = ("tools/tastelab/core/tests", "tools/tastelab/page/tests", "tools/tastelab/refs/tests")


@unittest.skipIf(NODE is None, "Node.js is not installed; the Taste Lab JavaScript tests were not run")
class TasteLabTests(unittest.TestCase):
    def run_suite(self, folder: str) -> None:
        files = sorted((ROOT / folder).glob("*.test.mjs"))
        self.assertTrue(files, f"no tests in {folder}")
        result = subprocess.run([NODE, "--test", *[f.relative_to(ROOT).as_posix() for f in files]],
                                cwd=ROOT, capture_output=True, text=True, timeout=600)
        self.assertEqual(result.returncode, 0, result.stdout[-4000:] + result.stderr[-2000:])

    def test_core(self):
        self.run_suite(SUITES[0])

    def test_geometry(self):
        self.run_suite(SUITES[1])

    def test_references(self):
        self.run_suite(SUITES[2])


class GeometryFixtureTests(unittest.TestCase):
    def test_fixture_matches_its_builder(self):
        result = subprocess.run([sys.executable, "tools/tastelab/sim/geometry_fixture.py", "--check"],
                                cwd=ROOT, capture_output=True, text=True, timeout=600)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


def load_serve():
    spec = importlib.util.spec_from_file_location("tastelab_refs_serve", ROOT / "tools/tastelab/refs/serve.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def request(server, method: str, path: str):
    conn = http.client.HTTPConnection("127.0.0.1", server.server_address[1], timeout=10)
    try:
        conn.request(method, path)
        response = conn.getresponse()
        return response.status, {k.lower(): v for k, v in response.getheaders()}, response.read()
    finally:
        conn.close()


def make_dir_link(link: Path, target: Path) -> bool:
    """A directory symbolic link, or a junction where links need a privilege; False if neither works."""
    try:
        os.symlink(target, link, target_is_directory=True)
        return True
    except OSError:
        pass
    try:
        import _winapi
        _winapi.CreateJunction(str(target), str(link))
        return True
    except (ImportError, AttributeError, OSError):
        return False


class ReferencesServerTests(unittest.TestCase):
    """The local references page server: 127.0.0.1 only, the page's own files, nothing received."""

    @classmethod
    def setUpClass(cls):
        cls.serve = load_serve()
        cls.server = cls.serve.make_server(0)
        cls.thread = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.thread.start()

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()
        cls.thread.join(timeout=10)

    def request(self, method: str, path: str):
        return request(self.server, method, path)

    def test_listens_on_loopback_only(self):
        self.assertEqual(self.server.server_address[0], "127.0.0.1")

    def test_page_and_modules(self):
        status, headers, body = self.request("GET", "/refs/index.html")
        self.assertEqual(status, 200)
        self.assertTrue(headers["content-type"].startswith("text/html"))
        self.assertIn("connect-src 'none'", headers["content-security-policy"])
        self.assertIn(b"Taste Lab references", body)
        for path in ("/refs/app.js", "/core/color.js", "/core/space.js"):
            status, headers, _ = self.request("GET", path)
            self.assertEqual(status, 200, path)
            self.assertTrue(headers["content-type"].startswith("text/javascript"), path)

    def test_root_redirects_to_the_page(self):
        status, headers, _ = self.request("GET", "/")
        self.assertEqual(status, 302)
        self.assertEqual(headers["location"], "/refs/")

    def test_lists_no_directory_and_stays_inside(self):
        self.assertEqual(self.request("GET", "/core/")[0], 404)
        for path in ("/../../AGENTS.md", "/%2e%2e/%2e%2e/AGENTS.md", "/refs/..%2f..%2f..%2fAGENTS.md"):
            self.assertEqual(self.request("GET", path)[0], 404, path)

    def test_receives_nothing(self):
        for method in ("POST", "PUT", "DELETE"):
            self.assertEqual(self.request(method, "/refs/index.html")[0], 501, method)

    def test_refuses_links_that_lead_outside(self):
        with tempfile.TemporaryDirectory() as tmp:
            root, outside = Path(tmp, "root"), Path(tmp, "outside")
            root.mkdir()
            outside.mkdir()
            (root / "inside.css").write_text("body {}\n", encoding="utf-8")
            (outside / "secret.css").write_text("secret\n", encoding="utf-8")
            link = root / "link"
            if not make_dir_link(link, outside):
                self.skipTest("this system cannot create a directory link")
            server = self.serve.make_server(0, root)
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()
            try:
                for method in ("GET", "HEAD"):
                    self.assertEqual(request(server, method, "/inside.css")[0], 200, method)
                    self.assertEqual(request(server, method, "/link/secret.css")[0], 404, method)
            finally:
                server.shutdown()
                server.server_close()
                thread.join(timeout=10)
                # Remove the link itself, never what it points to.
                (os.rmdir if os.name == "nt" else os.unlink)(link)


if __name__ == "__main__":
    unittest.main()
