"""LP5 piece R0: the research archive import and inventory (chapter 05 PS3.10 to PS3.17) over the Git-blob manifest.

The driver reads the tracked Git-blob manifest `work/experiments/language-lx/envelope-facts/MANIFEST.json`, copies every
member it names out of Git (`git cat-file`) into a fresh temporary directory under a digest name, verifies each blob id
and SHA-256, and grants those files to the Rust importer (`c600-archive-import`), which never runs Git, walks a checkout
or uses the network. The import table edition 1 and the absent-format discriminator edition 1 are data in
`rust/c600-archive/data/`; this driver regenerates both from the pinned label table and the Git blobs and compares them
byte for byte (`--write` rewrites them, for development only).

Tests named with a CT ID name their part. Everything here is source evidence (PHS.31); nothing is Windows, DirectX,
input, long-session or performance evidence, no producer runs and no search runs.
"""
from __future__ import annotations

import gzip
import hashlib
import io
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RUST = ROOT / "rust"
CRATE = RUST / "c600-archive"
TABLE_DATA = CRATE / "data" / "import-table-1.json"
DISCRIMINATOR_DATA = CRATE / "data" / "discriminator-1.json"
MANIFEST = "work/experiments/language-lx/envelope-facts/MANIFEST.json"
LABELS = "docs/architecture/1.0/language/evidence-import/corpus-labels.json"
RULE = "docs/architecture/1.0/language/evidence-import/rule.txt"
SCRIPT = "docs/architecture/1.0/language/evidence-import/extract_labels.py"
MATRIX = "docs/architecture/1.0/language/claim-matrix/matrix.json"
RESEARCH = "research/jumbling/restoration/"
S15 = RESEARCH + "15-lowering-machine/"
S16 = RESEARCH + "16-six-jams/"
S18 = RESEARCH + "18-tail-completion/"
S19 = RESEARCH + "19-height-depth-framework/"
STAGE13_STOP_TABLE = RESEARCH + "13-tutorial-runner/runs/dev/d1-acc-s7101/out4/tables/00113.json"
DIGEST_MISMATCH = "PS1:input.digest_mismatch"

# The pinned inputs (CX.3, EVD.49): the label table and the corpus identities it records.
LABELS_SHA256 = "6fcd45325a8789736534d53cd336297a0718c3c4386bb9da5af21fbb0ac13a68"
PRE_RENAME_CORPUS_SHA256 = "e2b388befa1671be67219b23c4a565c003c8ae960bd0aa69e07ddb030a669702"
POST_RENAME_CORPUS_SHA256 = "00693f6c3b4c1f04c30f0fd15744c21be155da50c28b9c66c4b4511797aac35d"


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical(value) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True, allow_nan=False).encode()


def git(*args: str, data: bytes | None = None) -> bytes:
    result = subprocess.run(["git", *args], input=data, cwd=ROOT, capture_output=True, check=True)
    return result.stdout


def head_blob(path: str) -> bytes:
    return git("cat-file", "blob", "HEAD:" + path)


def cat_blobs(oids: list[str]) -> list[bytes]:
    """The blobs of `oids`, each checked against its Git blob id."""
    out = git("cat-file", "--batch", data="".join(o + "\n" for o in oids).encode())
    at, blobs = 0, []
    for oid in oids:
        end = out.index(b"\n", at)
        got, kind, size = out[at:end].decode().split()
        size = int(size)
        raw = out[end + 1:end + 1 + size]
        if got != oid or kind != "blob" or hashlib.sha1(b"blob %d\0" % size + raw).hexdigest() != oid:
            raise AssertionError("Git blob identity differs: " + oid)
        blobs.append(raw)
        at = end + 2 + size
    return blobs


# --- the edition data ----------------------------------------------------------------------------------------------

def build_import_table() -> bytes:
    """PS3.10's import table, edition 1: the pinned label table converted into FP.21's rule shape."""
    raw = head_blob(LABELS)
    if sha(raw) != LABELS_SHA256:
        raise AssertionError("the pinned label table differs from its recorded digest")
    labels = json.loads(raw)
    rules, absent, external = [], [], []
    for r in labels["rows"]:
        facts = r["mapping"] or None
        if r["format_kind"] == "declared":
            rules.append({
                "selector": {"kind": "declared", "format": r["format"]},
                "field": r["field"], "value": r["value"], "class": r["cls"],
                "outcome": "facts" if facts else "recorded", "facts": facts, "prerequisites": [],
                "reason": f'{r["rule"]}: {r["note"]}' if r["rule"] else "no label rule",
                "source_row": {"code": r["code"], "count": r["count"], "files": r["files"]},
            })
        elif r["format_kind"] == "synthetic":
            absent.append({
                "source_format": r["format"], "field": r["field"], "value": r["value"], "class": r["cls"],
                "label_code": r["code"], "label_rule": r["rule"], "note": r["note"],
                "count": r["count"], "files": r["files"], "outcome": "recorded",
                "reason": "FP.22: a synthetic path format selects nothing; each occurrence stays recorded (EVD.53)",
            })
        else:
            external.append({
                "source": r["format"], "kind": r["format_kind"], "field": r["field"], "value": r["value"],
                "label_code": r["code"], "label_rule": r["rule"], "note": r["note"], "facts": facts,
            })
    rules.sort(key=lambda x: (canonical(x["selector"]), canonical(x["field"]), canonical(x["value"])))
    absent.sort(key=lambda x: (canonical(x["source_format"]), canonical(x["field"]), canonical(x["value"])))
    manifest = json.loads(head_blob(MANIFEST))
    doc = {
        "format": "c600-import-table", "version": 1, "edition": "c600-import-table-1",
        "status": ("LP5 piece R0 data of 23 November 2026: the conversion of the pinned label table into FP.21's rule "
                   "shape; it freezes nothing and edits no frozen text"),
        "corpus": {"role": "content", "sha256": labels["source"]["manifest_sha256"],
                   "post_rename": manifest["post_rename_manifest_sha256"]},
        "coverage": {"path": LABELS, "role": "bytes", "sha256": sha(raw)},
        "source_table": {"head": labels["source"]["head"], "totals": labels["totals"]},
        "enumeration": {
            "rule": {"path": RULE, "role": "source", "sha256": sha(head_blob(RULE))},
            "script": {"path": SCRIPT, "role": "source", "sha256": sha(head_blob(SCRIPT))},
            "distinct_value_limit": 32,
        },
        "fields": labels["fields"],
        "rules": rules,
        "absent_format_rows": absent,
        "external_rows": external,
    }
    return (json.dumps(doc, ensure_ascii=True, indent=1, sort_keys=True) + "\n").encode()


def nested_formats(value, pointer=""):
    """Pointers of objects below the root that declare a non-empty string `format`."""
    found = []
    if isinstance(value, dict):
        if pointer and isinstance(value.get("format"), str) and value["format"]:
            found.append((pointer, value["format"]))
        for k, v in value.items():
            found += nested_formats(v, pointer + "/" + k.replace("~", "~0").replace("/", "~1"))
    elif isinstance(value, list):
        for n, v in enumerate(value):
            found += nested_formats(v, f"{pointer}/{n}")
    return found


def decoded(path: str, raw: bytes) -> bytes:
    return gzip.decompress(raw) if path.endswith(".gz") else raw


def build_discriminator(manifest_raw: bytes, records: dict[str, bytes]) -> bytes:
    """FP.22's discriminator edition 1: one rule per absent-format record whose own bytes declare the format of one
    member object; the rule is keyed by the manifest's and the record's stored digests and the member's pointer."""
    rules = []
    for path in sorted(records):
        value = json.loads(decoded(path, records[path]))
        if isinstance(value, dict) and isinstance(value.get("format"), str) and value["format"]:
            continue
        inner = nested_formats(value)
        if not inner:
            continue
        shallowest = min(p.count("/") for p, _ in inner)
        top = [(p, f) for p, f in inner if p.count("/") == shallowest]
        if len(top) != 1:
            raise AssertionError("an absent-format record declares several formats at one depth: " + path)
        pointer, fmt = top[0]
        rules.append({
            "selector": {"kind": "absent", "manifest": {"role": "bytes", "sha256": sha(manifest_raw)},
                         "source": {"role": "bytes", "sha256": sha(records[path])}, "pointer": pointer},
            "interpretation": {"inner_format": fmt},
            "prerequisites": ["the object at the pointer declares inner_format as its own string member format"],
            "reason": ("the record's own bytes declare the format of the member at the pointer; the record's top-level "
                       "format stays absent and this interpretation is kept beside it"),
            "source_name": path,
        })
    doc = {
        "format": "c600-absent-format-discriminator", "version": 1, "edition": "c600-absent-format-discriminator-1",
        "review": ("pending: written by LP5 piece R0 on 23 November 2026 and not yet reviewed by the integrator; an "
                   "interpretation produces no facts on its own (FP.22, EVD.53)"),
        "manifest": {"path": MANIFEST, "role": "bytes", "sha256": sha(manifest_raw)},
        "rules": rules,
    }
    return (json.dumps(doc, ensure_ascii=True, indent=1, sort_keys=True) + "\n").encode()


# --- the corpus and the importer ------------------------------------------------------------------------------------

class Corpus:
    """The Git-blob corpus, copied into a fresh directory under digest names."""

    def __init__(self, directory: Path):
        self.dir = directory
        self.objects = directory / "objects"
        self.objects.mkdir()
        self.manifest_raw = head_blob(MANIFEST)
        if self.manifest_raw != (ROOT / MANIFEST).read_bytes():
            raise AssertionError("the checkout's Git-blob manifest differs from its committed blob")
        self.manifest = json.loads(self.manifest_raw)
        self.entries = self.manifest["records"] + self.manifest["auxiliary_corpus"]
        blobs = cat_blobs([e["git_blob"] for e in self.entries])
        self.bytes: dict[str, bytes] = {}
        for e, raw in zip(self.entries, blobs):
            if sha(raw) != e["stored_sha256"] or len(raw) != e["stored_bytes"]:
                raise AssertionError("a member's SHA-256 or size differs from the manifest: " + e["path"])
            self.bytes[e["path"]] = raw
            self.put(raw)
        self.manifest_path = self.put(self.manifest_raw)
        self.counter = 0

    def put(self, raw: bytes) -> Path:
        path = self.objects / sha(raw)
        if not path.exists():
            path.write_bytes(raw)
        return path

    def grant(self, members: dict[str, bytes] | None = None, manifest: bytes | None = None,
              manifest_name: str = MANIFEST) -> Path:
        members = self.bytes if members is None else members
        manifest_path = self.manifest_path if manifest is None else self.put(manifest)
        doc = {"manifest": {"name": manifest_name, "path": str(manifest_path)},
               "members": [{"name": n, "path": str(self.put(b))} for n, b in sorted(members.items())]}
        self.counter += 1
        path = self.dir / f"grant-{self.counter}.json"
        path.write_text(json.dumps(doc), encoding="utf-8")
        return path


def importer_binary() -> Path:
    """The importer, built in release mode: unoptimised SHA-256 over the 59 MB corpus takes about 12 s a run."""
    out = subprocess.run(
        ["cargo", "build", "--release", "--locked", "--offline", "--quiet", "--message-format=json", "--manifest-path",
         str(RUST / "Cargo.toml"), "-p", "c600-archive", "--bin", "c600-archive-import"],
        cwd=ROOT, capture_output=True, check=True, text=True).stdout
    for line in out.splitlines():
        message = json.loads(line)
        if message.get("reason") == "compiler-artifact" and message.get("executable"):
            return Path(message["executable"])
    raise AssertionError("cargo built no c600-archive-import executable")


def run_import(binary: Path, grant: Path, directory: Path, bundle_request: dict | None = None):
    report = directory / (grant.stem + "-report.json")
    args = [str(binary), str(grant), str(report)]
    bundle = None
    if bundle_request is not None:
        request = directory / (grant.stem + "-bundle-request.json")
        request.write_text(json.dumps(bundle_request), encoding="utf-8")
        bundle = directory / (grant.stem + "-bundle.zip")
        args += [str(request), str(bundle)]
    done = subprocess.run(args, cwd=directory, capture_output=True, text=True)
    if done.returncode != 0:
        raise AssertionError("the importer refused: " + done.stderr.strip()[-400:])
    result = json.loads(report.read_bytes())
    return (result, bundle.read_bytes()) if bundle is not None else result


def synthetic_format(path: str) -> str:
    """The label table's synthetic format of a leaf with no format (rule.txt), used only to compare with its rows."""
    parts = path.split("/")[3:]
    rest = "/".join(parts[1:])
    return "path:" + parts[0][:2] + "/" + re.sub(r"\d+", "#", re.sub(r"\.json(\.gz)?$", "", rest))


def member_checks(report, name):
    member = next(m for m in report["members"] if m["name"] == name)
    return member, [(c["check"], c["against"].split(" ")[0], c["result"]) for c in member["integrity"]["checks"]]


def failed(report):
    """Every failed (member, check, against) and every failed manifest self-check."""
    out = {(m["name"], c["check"], c["against"].split(" ")[0])
           for m in report["members"] for c in m["integrity"]["checks"] if c["result"] == "fail"}
    out |= {("<manifest>", c["check"], "") for c in report["manifest"]["checks"] if c["result"] == "fail"}
    return out


def repin_record(manifest: dict, name: str, stored: bytes | None = None, decoded_bytes: bytes | None = None):
    """Re-pins one corpus manifest entry and the manifest's own record digest."""
    for e in manifest["records"] + manifest["auxiliary_corpus"]:
        if e["path"] == name:
            if stored is not None:
                e["stored_sha256"], e["stored_bytes"] = sha(stored), len(stored)
            if decoded_bytes is not None:
                e["decoded_sha256"], e["decoded_bytes"] = sha(decoded_bytes), len(decoded_bytes)
                value = json.loads(decoded_bytes)
                e["canonical_sha256"], e["canonical_bytes"] = sha(canonical(value)), len(canonical(value))
    manifest["record_manifest_sha256"] = sha(canonical(manifest["records"]))


def dump(manifest: dict) -> bytes:
    return json.dumps(manifest, indent=1, sort_keys=True).encode()


def first_object(value):
    """The first object, depth first, of a decoded record (a mutation target)."""
    if isinstance(value, dict):
        return value
    for item in value:
        found = first_object(item)
        if found is not None:
            return found
    return None


class Lp5ArchiveTests(unittest.TestCase):
    tmp: tempfile.TemporaryDirectory
    corpus: Corpus
    binary: Path
    report: dict

    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory(prefix="lp5-r0-")
        cls.dir = Path(cls.tmp.name)
        cls.corpus = Corpus(cls.dir)
        cls.binary = importer_binary()
        cls.report = run_import(cls.binary, cls.corpus.grant(), cls.dir)
        cls.labels = json.loads(head_blob(LABELS))

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def import_(self, **grant):
        return run_import(self.binary, self.corpus.grant(**grant), self.dir)

    # --- the editions ----------------------------------------------------------------------------------------------

    def test_editions_are_the_conversion_of_the_pinned_tables(self):
        records = {e["path"]: self.corpus.bytes[e["path"]] for e in self.corpus.manifest["records"]}
        self.assertEqual(TABLE_DATA.read_bytes(), build_import_table(), "regenerate with --write")
        self.assertEqual(DISCRIMINATOR_DATA.read_bytes(), build_discriminator(self.corpus.manifest_raw, records),
                         "regenerate with --write")
        table = json.loads(TABLE_DATA.read_bytes())
        totals = self.labels["totals"]
        self.assertEqual(len(table["rules"]) + len(table["absent_format_rows"]), totals["triples"])
        self.assertEqual(len(table["external_rows"]), totals["external_rows"])
        self.assertEqual(self.report["table"]["sha256"], sha(TABLE_DATA.read_bytes()))
        self.assertEqual(self.report["discriminator"]["sha256"], sha(DISCRIMINATOR_DATA.read_bytes()))

    # --- the inventory ---------------------------------------------------------------------------------------------

    def test_corpus_import_inventory(self):
        r, t = self.report, self.report["totals"]
        self.assertEqual((t["records"], t["auxiliary"], t["present"], t["missing"]), (497, 135, 632, 0))
        self.assertEqual((t["integrity_pass"], t["integrity_fail"], t["unparsed"], t["unlisted"]), (632, 0, 0, 0))
        self.assertIsNone(r["refusal"])
        self.assertTrue(all(c["result"] == "pass" for c in r["manifest"]["checks"]), r["manifest"]["checks"])
        self.assertEqual(r["manifest"]["sha256"], sha(self.corpus.manifest_raw))
        # PS3.11: 156 of the 417 JSON files have no top-level format; with the 76 compressed records, 232 in all.
        absent = [m for m in r["members"] if m["recognition"]["kind"] == "absent"]
        self.assertEqual(len(absent), 232)
        self.assertEqual(sum(not m["name"].endswith(".gz") for m in absent), 156)
        self.assertEqual(sum(m["role"] == "record" and not m["name"].endswith(".gz") for m in r["members"]), 417)
        self.assertTrue(all(m["recognition"]["format"] is None for m in absent))
        # The plan's per-stage counts of absent-format records (section 3).
        per_stage = {}
        for m in absent:
            per_stage[m["name"].split("/")[3][:2]] = per_stage.get(m["name"].split("/")[3][:2], 0) + 1
        self.assertEqual(per_stage, {"14": 20, "15": 9, "16": 61, "18": 97, "19": 45})
        # PS3.12: ten records are discriminated by their own nested format, pending review; 222 stay recorded.
        kinds = [m["recognition"]["discrimination"]["kind"] for m in absent]
        self.assertEqual((kinds.count("discriminated"), kinds.count("recorded")), (10, 222))
        inner = sorted({m["recognition"]["discrimination"].get("inner_format") for m in absent} - {None})
        self.assertEqual(inner, ["literal-lattice/1", "pstar-run/1"])
        # Every member's recheck is not run, and nothing ran a producer.
        self.assertTrue(all(m["recheck"]["result"] == "not_run" for m in r["members"]))
        self.assertEqual(t["producers_run"], 0)

    def test_every_occurrence_is_mapped_or_kept_word_for_word(self):
        r = self.report
        names = [m["name"] for m in r["members"]]
        rows = {(x["format"], x["field"], canonical(x["value"])): x for x in self.labels["rows"] if x["cls"] in "SB"}
        got: dict = {}
        for g in r["occurrences"]:
            self.assertIn(g["outcome"], ("facts", "recorded", "unmapped"))
            self.assertEqual(len(g["pointers"]), len(set(g["pointers"])))
            if g["outcome"] != "facts":
                self.assertTrue(g["reason"])
            fmt = g["format"] if g["format"] is not None else synthetic_format(names[g["member"]])
            entry = got.setdefault((fmt, g["field"], canonical(g["value"])), [0, set(), set()])
            entry[0] += len(g["pointers"])
            entry[1].add(names[g["member"]])
            entry[2].add(g["outcome"] if g["format"] is not None else "absent")
        self.assertEqual(set(got), set(rows), "the enumeration differs from the pinned label table")
        for key, (count, files, outcomes) in got.items():
            row = rows[key]
            self.assertEqual((count, len(files)), (row["count"], row["files"]), key)
            if row["format_kind"] == "synthetic":
                self.assertEqual(outcomes, {"absent"}, key)
            else:
                self.assertEqual(outcomes, {"facts" if row["mapping"] else "recorded"}, key)
        t = r["totals"]
        self.assertEqual(t["occurrences"], self.labels["totals"]["occurrences"])
        self.assertEqual(t["occurrences_unmapped"], 0)
        self.assertEqual(t["occurrences"], t["occurrences_facts"] + t["occurrences_recorded"])
        mapped = sum(x["count"] for x in self.labels["rows"] if x["format_kind"] == "declared" and x["mapping"])
        self.assertEqual(t["occurrences_facts"], mapped)
        # Prose fields and payload booleans, as the label table counts them.
        prose = {}
        for p in r["prose"]:
            fmt = p["format"] if p["format"] is not None else synthetic_format(names[p["member"]])
            prose[(fmt, p["field"])] = prose.get((fmt, p["field"]), 0) + p["count"]
        self.assertEqual(prose, {(a, b): c for a, b, c in self.labels["prose_fields"]})
        payload = {(p["key"], str(p["value"]).lower()): p["count"] for p in r["payload_booleans"]}
        self.assertEqual(payload, {(a, b): c for a, b, c in self.labels["payload_booleans"]})

    # --- CT-PS3-08 -------------------------------------------------------------------------------------------------

    def stage19_compressed(self):
        rows = json.loads(self.corpus.bytes[S19 + "IMPORT-MANIFEST.json"])["files"]
        row = next(x for x in rows if x["path"].endswith(".json.gz") and x["uncompressed_bytes"] > 4096)
        return S19 + row["path"], row

    def test_ct_ps3_08_tampered_gzip_decoded_and_manifest_digests_fail_separately(self):
        """CT-PS3-08 (part: the corpus, with stage 19's IMPORT-MANIFEST and the Git-blob manifest)."""
        self.assertEqual(failed(self.report), set())
        name, row = self.stage19_compressed()
        stage = S19 + "IMPORT-MANIFEST.json"
        original = self.corpus.bytes[name]

        # A tampered gzip stream: one byte of the compressed body flipped. Its stored digests fail against both
        # manifests and its stream fails; nothing else does.
        tampered = bytearray(original)
        tampered[len(tampered) // 2] ^= 0x55
        r = self.import_(members={**self.corpus.bytes, name: bytes(tampered)})
        self.assertEqual(r["refusal"], DIGEST_MISMATCH)
        got = failed(r)
        self.assertEqual({(m, c) for m, c, _ in got}, {(name, "stored_sha256"), (name, "gzip_stream")})
        self.assertEqual({a for m, c, a in got if c == "stored_sha256"}, {MANIFEST, stage})
        # The same stream with every stored pin moved to it: only the stream check fails.
        manifest, stage_doc = json.loads(self.corpus.manifest_raw), json.loads(self.corpus.bytes[stage])
        for x in stage_doc["files"]:
            if x["path"] == row["path"]:
                x["sha256"], x["stored_bytes"] = sha(bytes(tampered)), len(tampered)
        stage_raw = json.dumps(stage_doc, indent=1).encode()
        repin_record(manifest, name, stored=bytes(tampered))
        repin_record(manifest, stage, stored=stage_raw, decoded_bytes=stage_raw)
        r = self.import_(members={**self.corpus.bytes, name: bytes(tampered), stage: stage_raw}, manifest=dump(manifest))
        self.assertEqual({(m, c) for m, c, _ in failed(r)}, {(name, "gzip_stream")})

        # A tampered decoded content, recompressed, with the stored pins moved: only the decoded checks fail, against
        # both manifests, and the canonical digest that derives from the decoded content.
        content = json.loads(gzip.decompress(original))
        first_object(content)["changed"] = "a tampered decoded content"
        restored = gzip.compress(json.dumps(content).encode(), compresslevel=9, mtime=0)
        manifest, stage_doc = json.loads(self.corpus.manifest_raw), json.loads(self.corpus.bytes[stage])
        for x in stage_doc["files"]:
            if x["path"] == row["path"]:
                x["sha256"], x["stored_bytes"] = sha(restored), len(restored)
        stage_raw = json.dumps(stage_doc, indent=1).encode()
        repin_record(manifest, name, stored=restored)
        repin_record(manifest, stage, stored=stage_raw, decoded_bytes=stage_raw)
        r = self.import_(members={**self.corpus.bytes, name: restored, stage: stage_raw}, manifest=dump(manifest))
        got = failed(r)
        self.assertEqual({(m, c) for m, c, _ in got},
                         {(name, "decoded_sha256"), (name, "decoded_bytes"), (name, "canonical_sha256")})
        self.assertEqual({a for m, c, a in got if c == "decoded_sha256"}, {MANIFEST, stage})

        # A tampered manifest digest: one row's uncompressed_sha256 in IMPORT-MANIFEST. Against the unchanged
        # Git-blob manifest the stage manifest's own bytes fail; the row fails against the member it names; the
        # member's own bytes still pass against the Git-blob manifest.
        stage_doc = json.loads(self.corpus.bytes[stage])
        for x in stage_doc["files"]:
            if x["path"] == row["path"]:
                x["uncompressed_sha256"] = "0" * 64
        stage_raw = json.dumps(stage_doc, indent=1).encode()
        r = self.import_(members={**self.corpus.bytes, stage: stage_raw})
        got = failed(r)
        self.assertIn((stage, "stored_sha256", MANIFEST), got)
        self.assertIn((name, "decoded_sha256", stage), got)
        self.assertNotIn((name, "decoded_sha256", MANIFEST), got)
        self.assertNotIn((name, "stored_sha256", MANIFEST), got)
        # And a tampered digest inside the Git-blob manifest: its own record digest fails, and so does the member.
        manifest = json.loads(self.corpus.manifest_raw)
        victim = manifest["records"][0]
        victim["stored_sha256"] = "f" * 64
        r = self.import_(manifest=dump(manifest))
        got = failed(r)
        self.assertIn(("<manifest>", "record_manifest_sha256", ""), got)
        self.assertEqual({(m, c) for m, c, _ in got if m != "<manifest>"}, {(victim["path"], "stored_sha256")})

    def test_ct_ps3_08_source_edition_digest_is_not_taken_for_public_bytes(self):
        """CT-PS3-08 (part: IMPORT-MANIFEST's original digests stay source-edition metadata)."""
        stage = S19 + "IMPORT-MANIFEST.json"
        report_rows = next(s for s in self.report["stage_manifests"])["rows"]
        self.assertTrue(all(x["original_bytes"].startswith("absent") for x in report_rows))
        self.assertEqual([x["original_sha256"] for x in report_rows],
                         [x["original_sha256"] for x in json.loads(self.corpus.bytes[stage])["files"]])
        # Every original digest changed: no check result moves, so none of them is ever compared with bytes.
        stage_doc = json.loads(self.corpus.bytes[stage])
        for x in stage_doc["files"]:
            x["original_sha256"] = "0" * 64
        stage_raw = json.dumps(stage_doc, indent=1).encode()
        manifest = json.loads(self.corpus.manifest_raw)
        repin_record(manifest, stage, stored=stage_raw, decoded_bytes=stage_raw)
        r = self.import_(members={**self.corpus.bytes, stage: stage_raw}, manifest=dump(manifest))
        self.assertEqual(failed(r), set())
        # A row whose public digest is replaced by its original digest fails; the original is never accepted.
        name, row = self.stage19_compressed()
        stage_doc = json.loads(self.corpus.bytes[stage])
        for x in stage_doc["files"]:
            if x["path"] == row["path"]:
                self.assertNotEqual(x["original_sha256"], x["sha256"])
                x["sha256"] = x["original_sha256"]
        stage_raw = json.dumps(stage_doc, indent=1).encode()
        manifest = json.loads(self.corpus.manifest_raw)
        repin_record(manifest, stage, stored=stage_raw, decoded_bytes=stage_raw)
        r = self.import_(members={**self.corpus.bytes, stage: stage_raw}, manifest=dump(manifest))
        self.assertEqual(failed(r), {(name, "stored_sha256", stage)})

    # --- CT-PS3-09 -------------------------------------------------------------------------------------------------

    def test_ct_ps3_09_case_only_pairs_stay_distinct_by_digest(self):
        """CT-PS3-09 (part: the six stage 16 pairs under the names they had before the 4 November 2026 rename)."""
        self.assertEqual(self.report["case_only_names"], [])
        manifest = json.loads(self.corpus.manifest_raw)
        members = dict(self.corpus.bytes)
        renamed = {}
        for pair in self.labels["case_pairs"]:
            upper = next(p for p in pair["paths"] if "/PX-" in p)
            now = upper[:-len(".json")] + "-cert.json"
            self.assertIn(now, members)
            self.assertEqual(len(members[now]), pair["sizes"][pair["paths"].index(upper)])
            renamed[now] = upper
        self.assertEqual(len(renamed), 6)
        for e in manifest["records"] + manifest["auxiliary_corpus"]:
            if e["path"] in renamed:
                members[renamed[e["path"]]] = members.pop(e["path"])
                e["path"] = renamed[e["path"]]
        manifest["records"].sort(key=lambda e: e["path"])
        manifest["record_manifest_sha256"] = sha(canonical(manifest["records"]))
        everything = sorted(manifest["records"] + manifest["auxiliary_corpus"], key=lambda e: e["path"])
        full = sha("".join(e["path"] + "\0" + e["git_blob"] + "\n" for e in everything).encode())
        # Not the label table's pre-rename corpus digest: the stage 16 README also changed with the rename (its
        # rename note), so only the six names are restored here.
        self.assertNotEqual(full, PRE_RENAME_CORPUS_SHA256)
        self.assertEqual(self.corpus.manifest["full_corpus_manifest_sha256"], POST_RENAME_CORPUS_SHA256)
        manifest["full_corpus_manifest_sha256"] = full
        r = self.import_(members=members, manifest=dump(manifest))
        groups = r["case_only_names"]
        self.assertEqual(len(groups), 6)
        for g in groups:
            self.assertTrue(g["distinct_digests"], g)
            self.assertEqual(len(g["names"]), 2)
            self.assertEqual(len({n["name"].lower() for n in g["names"]}), 1)
        self.assertEqual((r["totals"]["records"], r["totals"]["present"], r["totals"]["integrity_fail"]), (497, 632, 0))
        self.assertIsNone(r["refusal"])

    def test_ct_ps3_09_missing_member_is_reported_and_stays_counted(self):
        """CT-PS3-09 (part: a member listed by the Git-blob manifest and not granted)."""
        name = S16 + "jams.json"
        members = {n: b for n, b in self.corpus.bytes.items() if n != name}
        r = self.import_(members=members)
        member = next(m for m in r["members"] if m["name"] == name)
        self.assertEqual((member["present"], member["integrity"]["result"], member["recognition"]["kind"]),
                         (False, "not_available", "not_available"))
        self.assertEqual((r["totals"]["records"], r["totals"]["present"], r["totals"]["missing"]), (497, 631, 1))
        self.assertIsNone(r["refusal"])
        # A missing word member is reported the same way and keeps its run record's reference.
        word = S18 + "results/lattice-d1/run/words/00007.ia16.gz"
        r = self.import_(members={n: b for n, b in self.corpus.bytes.items() if n != word})
        member = next(m for m in r["members"] if m["name"] == word)
        self.assertEqual((member["present"], member["recognition"]["kind"]), (False, "referenced"))
        self.assertEqual(r["totals"]["auxiliary"], 135)

    # --- CT-PS3-10 -------------------------------------------------------------------------------------------------

    def test_ct_ps3_10_bundle_declares_missing_dependencies_and_runs_nothing(self):
        """CT-PS3-10 (part: a bundle of stage 19's q11 record and a stage 14 record with a producer source as data)."""
        q11 = S19 + "records/type-lex-trimmed-independent-results.json"
        reference = S19 + "records/type-lex-trimmed-macro.json"
        producer = RESEARCH + "14-blocking-structure/scripts/replay_word.py"
        request = {"envelopes": [
            {"name": q11, "dependencies": [
                {"name": reference, "sha256": sha(self.corpus.bytes[reference]), "member": reference,
                 "reason": "a stored witness record the claim cites"},
                {"name": "records/type-lex-successor-base-cache.npz", "sha256": None, "member": None,
                 "reason": "absent from the stage 19 archive (LB5.17)"}]},
            {"name": RESEARCH + "14-blocking-structure/results/s7302-t3p-chain-replay.json", "dependencies": [
                {"name": producer, "sha256": sha(self.corpus.bytes[producer]), "member": producer,
                 "reason": "the historical replay program, carried as source text only"}]},
        ]}
        report, raw = run_import(self.binary, self.corpus.grant(), self.dir, request)
        self.assertIsNone(report["refusal"])
        z = zipfile.ZipFile(io.BytesIO(raw))
        infos = z.infolist()
        self.assertEqual(infos[0].filename, "manifest.json")
        names = [i.filename for i in infos[1:]]
        self.assertEqual(names, sorted(names))
        for i in infos:
            self.assertEqual((i.compress_type, i.date_time, i.extra, i.comment), (zipfile.ZIP_STORED, (1980, 1, 1, 0, 0, 0), b"", b""))
        for n in names:
            self.assertEqual(n, "objects/" + sha(z.read(n)))
        manifest = json.loads(z.read("manifest.json"))
        payload = manifest["payload"]
        self.assertEqual(manifest["format"], "c600-research-bundle")
        self.assertEqual(payload["checkers"], [])
        self.assertEqual(payload["missing"], [])
        self.assertEqual([m["name"] for m in manifest["extensions"]["missing_without_digest"]],
                         ["records/type-lex-successor-base-cache.npz"])
        self.assertIn("asserts no replay", manifest["extensions"]["replay"])
        self.assertEqual({m["name"] for m in payload["members"]}, set(names))
        self.assertEqual(payload["table"]["content"]["sha256"], sha(TABLE_DATA.read_bytes()))
        q11_source = next(s for s in payload["sources"] if s["name"] == q11)
        self.assertIsNotNone(q11_source["original"])
        self.assertNotEqual(q11_source["original"]["sha256"], q11_source["stored"]["sha256"])
        self.assertEqual(report["totals"]["producers_run"], 0)

    # --- CT-DET-24 -------------------------------------------------------------------------------------------------

    def test_ct_det_24_timed_stop_and_data_only_archive_keep_their_limits(self):
        """CT-DET-24 (part: import of an archived timed stop and of stage 19's data-only archive; the KN6-checked
        witnesses of the definition's last clause wait for R8 and the stage checkers)."""
        table = json.loads(TABLE_DATA.read_bytes())
        facts = lambda g: table["rules"][g["rule"]]["facts"] if g["outcome"] == "facts" else None
        names = [m["name"] for m in self.report["members"]]
        stop = S15 + "results/runs/s7305-pstar.json"
        groups = [g for g in self.report["occurrences"] if names[g["member"]] == stop]
        timed = [facts(g) for g in groups if g["field"] in ("run.status", "run.reason")]
        self.assertEqual(timed, [{"claim": "open", "check": "producer", "job_outcome": "cut off"}] * 2)
        member = next(m for m in self.report["members"] if m["name"] == stop)
        self.assertEqual(member["recognition"]["discrimination"]["inner_format"], "pstar-run/1")
        # Stage 19 holds no executable producer or checker: its members are data, nothing is run, and no import
        # result is a fresh replay.
        s19 = [m for m in self.report["members"] if m["name"].startswith(S19)]
        self.assertEqual(len(s19), 113)
        self.assertFalse(any(m["name"].endswith(".py") for m in s19))
        self.assertTrue(all(m["recheck"]["result"] == "not_run" for m in s19))
        self.assertEqual(self.report["totals"]["producers_run"], 0)

    # --- CT-EVD-12 -------------------------------------------------------------------------------------------------

    def test_ct_evd_12_import_half(self):
        """CT-EVD-12 (part: the import half; the records of E1 to E3 give, through the import table, the grade and job
        facts that rust/c600-evidence/tests/examples.rs transcribes. The forms of E1 to E3 come from the test's own
        statements, since the label table has no producer for `form` (chapter 06, open items; plan reading C2). Not
        run here: building the words from these facts, which waits for R1's typed bindings)."""
        table = json.loads(TABLE_DATA.read_bytes())
        names = [m["name"] for m in self.report["members"]]

        def facts(path, field):
            out = [table["rules"][g["rule"]]["facts"] for g in self.report["occurrences"]
                   if names[g["member"]] == path and g["field"] == field and g["outcome"] == "facts"]
            self.assertTrue(out, (path, field))
            return out[0]

        def member(path):
            return next(m for m in self.report["members"] if m["name"] == path)

        # E1: R01's certificate is a stored macro-certificate/1 object with no label occurrence (CX.1), so its basis is
        # the certificate kind itself; its check record gives a2's producer check by actual replay.
        r01 = S15 + "results/certificates/R01.json"
        self.assertEqual(member(r01)["recognition"], {"kind": "declared", "format": "macro-certificate/1"})
        self.assertEqual(member(r01)["interpretation"]["occurrences"], 0)
        check = S15 + "results/certificates/R01-check.json"
        self.assertEqual(facts(check, "replay.passed"), {"check": "producer", "check_mode": "actual replay"})
        self.assertEqual(facts(check, "passed"), {"check": "producer"})
        # E2: the stage 19 receipt of the controlled bound gives (theorem, model review) with claim conditional.
        receipt = S19 + "records/controlled-type-height-independent-review-receipt.json"
        self.assertEqual(facts(receipt, "status"),
                         {"claim": "conditional", "basis": "theorem", "check": "model review"})
        # E3: the 7302 stop's run reads a completed producer job whose family-R search is not exhausted (open); the
        # exhaustive families E, L, M and T read declared-set exhaustive computations.
        stop = S15 + "results/runs/s7302-pstar2.json"
        self.assertEqual(facts(stop, "segments[].run.status"),
                         {"claim": "open", "scope": "declared search set", "basis": "exact computation",
                          "check": "producer", "job_outcome": "completed"})
        self.assertEqual(facts(stop, "segments[].run.steps[].families.R.exhausted"), {"claim": "open"})
        self.assertEqual(facts(stop, "segments[].run.steps[].families.E.exhausted"),
                         {"scope": "declared search set", "basis": "exhaustive computation", "check": "producer",
                          "job_outcome": "completed"})

    # --- CT-LB5-06 -------------------------------------------------------------------------------------------------

    def test_ct_lb5_06_manifest_part(self):
        """CT-LB5-06 (part: stage 19's stored and uncompressed manifest digests; the reference tables, certificates
        and preflight parts belong to R6 and R1)."""
        [stage] = self.report["stage_manifests"]
        self.assertEqual(self.report["members"][stage["member"]]["name"], S19 + "IMPORT-MANIFEST.json")
        self.assertEqual(stage["fields"]["format"], "sanitized-research-archive/1")
        self.assertEqual(stage["fields"]["source_commit"], "a2ebb4cccd6b60d7527c001ef3b2cbc56b179249")
        self.assertEqual(len(stage["rows"]), 109)
        self.assertTrue(all(x["status"] == "checked" for x in stage["rows"]))
        self.assertEqual(sorted(stage["not_listed"]), sorted(
            S19 + n for n in ("PUBLICATION-CHECK.json", "PUBLICATION.md", "README.md")))
        doc = json.loads(self.corpus.bytes[S19 + "IMPORT-MANIFEST.json"])
        compressed = 0
        for x in doc["files"]:
            member, checks = member_checks(self.report, S19 + x["path"])
            mine = {c for c, a, res in checks if a == S19 + "IMPORT-MANIFEST.json" and res == "pass"}
            self.assertTrue({"stored_sha256", "stored_bytes", "decoded_sha256", "decoded_bytes"} <= mine, x["path"])
            if x["sha256"] != x["uncompressed_sha256"]:
                compressed += 1
                self.assertIn("gzip_stream", mine)
        self.assertEqual(compressed, 15)

    # --- word members ----------------------------------------------------------------------------------------------

    def test_ia16_word_members_import_with_stored_and_decoded_digests(self):
        words = [m for m in self.report["members"] if m["recognition"]["kind"] == "referenced"]
        self.assertEqual(len(words), 53)
        for m in words:
            self.assertEqual(m["recognition"]["encoding"], "ia16")
            got = {(c["check"], c["against"].split(" ")[0]) for c in m["integrity"]["checks"] if c["result"] == "pass"}
            run = S18 + "results/lattice-d1/run/run.json"
            self.assertTrue({("stored_sha256", MANIFEST), ("gzip_stream", run), ("decoded_sha256", run),
                             ("decoded_bytes", run)} <= got, m["name"])

    # --- no producer, process or network ---------------------------------------------------------------------------

    def test_nothing_runs_a_producer_or_reaches_git_processes_or_the_network(self):
        banned = re.compile(r'std::process|Command::|std::net|TcpStream|UdpSocket|read_dir|walkdir|"git"')
        for path in sorted((CRATE / "src").rglob("*.rs")):
            code = "\n".join(line for line in path.read_text(encoding="utf-8").splitlines()
                             if not line.lstrip().startswith("//"))
            self.assertIsNone(banned.search(code), path.name)
        manifest = (CRATE / "Cargo.toml").read_text(encoding="utf-8")
        deps = re.findall(r"^(c600-[a-z0-9-]+)\s*=", manifest, re.M)
        self.assertEqual(sorted(deps), ["c600-digest", "c600-evidence", "c600-kernel"])
        kernel_engines = re.compile(r"c600_kernel::(engine|model|exact|structure|request|parallel)")
        for path in sorted((CRATE / "src").rglob("*.rs")):
            self.assertIsNone(kernel_engines.search(path.read_text(encoding="utf-8")), path.name)

    # --- stage 18 start tables (the dependency report) -------------------------------------------------------------

    def test_stage18_start_tables_are_stored_or_rebuildable_from_stored_words(self):
        """The evidence behind docs/progress/1.0/lp5-r0-dependency-report.md, section 3. Read-only: digests of stored
        bytes, no replay and no producer."""
        stop = json.loads(head_blob(STAGE13_STOP_TABLE))
        self.assertEqual(stop["digest"], pose_table_digest(stop))
        t6 = json.loads(self.corpus.bytes[S18 + "results/d1-tail/steps.json"])
        rule = json.loads(self.corpus.bytes[S18 + "results/rule-stop/steps.json"])
        self.assertEqual(t6["start_digest"], stop["digest"])
        self.assertEqual(rule["start_digest"], stop["digest"])
        for run, record, copies in (("d1-tail", t6, 2), ("rule-stop", rule, 1)):
            previous = record["start_digest"]
            for step in record["steps"]:
                self.assertEqual(step["start_digest"], previous)
                previous = step["end_digest"]
                base = S18 + f"results/{run}/{step['dir']}/"
                words = {"w": "w.json", "v": "v.json.gz", "winv": "winv.json"}
                if step["X"] > 0:
                    words["x"] = "x.json.gz"
                for key, file in words.items():
                    value = json.loads(decoded(file, self.corpus.bytes[base + file]))
                    hashed = value if key == "winv" else value["word"]
                    self.assertEqual(sha(json.dumps(hashed).encode()), step[f"{key}_sha256"], base + file)
                w, v = (json.loads(decoded(f, self.corpus.bytes[base + f]))["word"] for f in ("w.json", "v.json.gz"))
                self.assertEqual((len(w), len(v)), (1, step["V"]))
                self.assertEqual(step["letters"], step["X"] + 2 + copies * step["V"])
        # T6's end table is stored: the lattice run's start table carries T6 step 4's end digest.
        lattice = json.loads(self.corpus.bytes[S18 + "results/lattice-d1/run/start-table.json"])
        self.assertEqual(lattice["digest"], t6["steps"][-1]["end_digest"])
        self.assertEqual(lattice["digest"], pose_table_digest(lattice))
        # No stored table carries another step boundary of either chain.
        boundaries = {s["end_digest"] for s in t6["steps"][:-1] + rule["steps"]}
        stored = set()
        for path, raw in self.corpus.bytes.items():
            if path.startswith(S18) and path.endswith((".json", ".json.gz")):
                value = json.loads(decoded(path, raw))
                if isinstance(value, dict) and "pose_id" in value:
                    stored.add(value.get("digest"))
        self.assertFalse(boundaries & stored)


def pose_table_digest(table: dict) -> str:
    """The simulator's configuration digest (research/jumbling/sim/state.py, `digest`), recomputed from a stored pose
    table: SHA-256 over the sorted (pose matrix JSON, int32 little-endian piece indices) of every non-identity pose."""
    members: dict[int, list[int]] = {}
    for piece, pid in enumerate(table["pose_id"]):
        if pid != 0:
            members.setdefault(pid, []).append(piece)
    items = sorted((json.dumps(table["poses"][pid]), b"".join(i.to_bytes(4, "little") for i in pieces))
                   for pid, pieces in members.items())
    h = hashlib.sha256()
    for key, data in items:
        h.update(key.encode())
        h.update(data)
    return h.hexdigest()


def write_editions() -> None:
    manifest = json.loads(head_blob(MANIFEST))
    entries = manifest["records"]
    records = dict(zip([e["path"] for e in entries], cat_blobs([e["git_blob"] for e in entries])))
    TABLE_DATA.parent.mkdir(parents=True, exist_ok=True)
    TABLE_DATA.write_bytes(build_import_table())
    DISCRIMINATOR_DATA.write_bytes(build_discriminator(head_blob(MANIFEST), records))
    print("wrote", TABLE_DATA.relative_to(ROOT).as_posix(), DISCRIMINATOR_DATA.relative_to(ROOT).as_posix())


if __name__ == "__main__":
    if sys.argv[1:] == ["--write"]:
        write_editions()
    else:
        unittest.main()
