"""CT-LG1-02 (whole, with `c600-syntax`'s `ct_lg1_02_*` and `ct_err_05_*` tests in `syntax.rs`) and CT-PS1-05 (part,
with `c600-run`'s `ct_ps1_05_pinned_bytes_survive_checkout`; not run: a reader of captured proof sources, which has no
owner yet): pinned source bytes survive LF and CRLF checkouts (R-03; chapter 02, LG1.2; chapter 05, PS1.8).

Git writes a file at checkout after the line-ending conversion that the checkout's settings and the file's attributes
select. `git cat-file --filters` applies the same conversion to the indexed blob and writes nothing, so this test
reads every `.6cs` source and every file the format packet pins as an LF, a CRLF and an `input` checkout would write
it, and requires the indexed bytes each time: `.6cs` sources without a CR byte, the canonical layout fixture with the
SHA-256 that `syntax.rs` pins, and the packet's files with their pins. The Rust tests parse the checkout's own bytes
and refuse CRLF, a byte-order mark and malformed UTF-8. A repository without the no-conversion rule fails the same
check (the negative control). Git must be installed; the test changes no repository state.
"""
from __future__ import annotations

import hashlib
import json
import re
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PACKET = "docs/architecture/1.0/language/format-packet/candidate.json"
FIXTURE = "rust/c600-syntax/tests/fixtures/canonical.6cs"
SYNTAX_TESTS = ROOT / "rust" / "c600-syntax" / "tests" / "syntax.rs"

# The checkout settings: LF line ends, CRLF line ends, and conversion on commit only.
SETTINGS = {
    "LF": ["core.autocrlf=false", "core.eol=lf"],
    "CRLF": ["core.autocrlf=true", "core.eol=crlf"],
    "input": ["core.autocrlf=input"],
}


def git(root: Path, settings: list[str], *args: str) -> bytes:
    command = ["git", "-C", str(root)]
    for setting in settings:
        command += ["-c", setting]
    return subprocess.run([*command, *args], check=True, capture_output=True).stdout


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def checkout_problems(root: Path, expected: dict[str, str | None]) -> list[str]:
    """What an LF, CRLF or `input` checkout of each indexed path writes other than its indexed bytes, and indexed bytes
    that differ from a given SHA-256 or hold a CR byte in a `.6cs` source."""
    problems = []
    for path, digest in sorted(expected.items()):
        indexed = git(root, [], "cat-file", "blob", f":{path}")
        if digest is not None and sha256(indexed) != digest:
            problems.append(f"{path}: the indexed bytes do not have the pinned SHA-256")
        if path.endswith(".6cs") and b"\r" in indexed:
            problems.append(f"{path}: a CR byte in a source file")
        for name, settings in SETTINGS.items():
            if git(root, settings, "cat-file", "--filters", f":{path}") != indexed:
                problems.append(f"{path}: a {name} checkout writes other bytes")
    return problems


def sources() -> dict[str, str | None]:
    """Every tracked `.6cs` source, with the canonical layout fixture's SHA-256 as `syntax.rs` pins it."""
    listed = git(ROOT, [], "ls-files", "-z", "--", "*.6cs").decode("utf-8").split("\0")
    pinned = re.search(r'const PINNED_SHA256: &str = "([0-9a-f]{64})";', SYNTAX_TESTS.read_text(encoding="utf-8"))
    if pinned is None:
        raise AssertionError("syntax.rs pins no SHA-256 for the canonical layout fixture")
    return {path: pinned.group(1) if path == FIXTURE else None for path in listed if path}


class SourceCheckoutTests(unittest.TestCase):
    def test_ct_lg1_02_sources_keep_their_pinned_bytes_in_lf_and_crlf_checkouts(self):
        """CT-LG1-02 (whole, with `syntax.rs`): each `.6cs` source reads the same from LF, CRLF and `input`
        checkouts, and the canonical layout fixture keeps the SHA-256 that the parser's test pins."""
        expected = sources()
        self.assertIn(FIXTURE, expected)
        self.assertEqual(checkout_problems(ROOT, expected), [])

    def test_ct_ps1_05_pinned_packet_files_keep_their_bytes_in_lf_and_crlf_checkouts(self):
        """CT-PS1-05 (part, above): each file the format packet pins reads the same from LF, CRLF and `input`
        checkouts, with its pinned SHA-256."""
        candidate = json.loads((ROOT / PACKET).read_text(encoding="utf-8"))["candidate"]
        expected = {pin["path"]: pin["sha256"] for pin in candidate}
        self.assertEqual(len(expected), 7)
        self.assertEqual(checkout_problems(ROOT, expected), [])

    def test_negative_control_a_repository_without_the_rule_fails(self):
        """The check reports conversion: a CRLF checkout converts a source when no attribute forbids it, a `text
        eol=crlf` attribute converts it in every checkout, and `* -text` keeps it."""
        source = b'edition "6cs-draft-1";\nmodule m;\n'
        cases = [
            (None, ["a CRLF checkout writes other bytes"]),
            (b"*.6cs text eol=crlf\n", [f"a {name} checkout writes other bytes" for name in SETTINGS]),
            (b"* -text\n", []),
        ]
        for attributes, found in cases:
            with tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                git(root, [], "init", "-q")
                (root / "m.6cs").write_bytes(source)
                names = ["m.6cs"]
                if attributes is not None:
                    (root / ".gitattributes").write_bytes(attributes)
                    names.append(".gitattributes")
                git(root, ["core.autocrlf=false"], "add", "--", *names)
                problems = checkout_problems(root, {"m.6cs": sha256(source)})
                self.assertEqual(problems, [f"m.6cs: {text}" for text in found], attributes)
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            git(root, [], "init", "-q")
            (root / "m.6cs").write_bytes(source.replace(b"\n", b"\r\n"))
            (root / ".gitattributes").write_bytes(b"* -text\n")
            git(root, ["core.autocrlf=false"], "add", "--", "m.6cs", ".gitattributes")
            found = ["m.6cs: the indexed bytes do not have the pinned SHA-256", "m.6cs: a CR byte in a source file"]
            self.assertEqual(checkout_problems(root, {"m.6cs": sha256(source)}), found)


if __name__ == "__main__":
    unittest.main()
