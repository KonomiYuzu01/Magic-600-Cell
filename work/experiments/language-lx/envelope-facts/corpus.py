"""PS1.38 experiment: immutable Git inputs and the existing label classifier."""

import ast
from collections import Counter, defaultdict
import gzip
import hashlib
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
PREFIX = "work/experiments/language-lx/envelope-facts/"
RESEARCH = "research/jumbling/restoration/"
IMPORT = "docs/architecture/1.0/language/evidence-import/"
ARCH = "docs/architecture/1.0/language/architecture/"
BASE = "1e7d0074df96cbeb05f55282cf8f6c03fbf4a4ff"
POST_RENAME = "00693f6c3b4c1f04c30f0fd15744c21be155da50c28b9c66c4b4511797aac35d"
INPUTS = [IMPORT + n for n in (
    "README.md", "extract_labels.py", "corpus-labels.json", "corpus-labels.md",
    "rules.txt", "refs.txt", "ext.txt", "rule.txt",
)] + [ARCH + "05-persistence.md", ARCH + "06-evidence.md",
      ARCH + "11-route-and-spikes.md", "docs/development-guide/AGENT_BRIEFING.md",
      "docs/wiki/decisions/owner-decisions-2026-11-05.md",
      "work/experiments/language-lx/README.md", "AGENTS.md"]


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def canonical(value):
    # Experiment profile only: retained JSON (PS1.2), no terminal LF.
    return json.dumps(value, sort_keys=True, separators=(",", ":"),
                      ensure_ascii=True, allow_nan=False).encode("utf-8")


def pack(data):
    return gzip.compress(data, compresslevel=9, mtime=0)


def git(*args, data=None):
    result = subprocess.run(["git", *args], input=data, cwd=ROOT,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    require(result.returncode == 0, "Git read failed: " + args[0])
    return result.stdout


def tree(ref):
    entries = []
    for line in git("ls-tree", "-r", "-z", ref, RESEARCH, *INPUTS).split(b"\0"):
        if not line:
            continue
        meta, path = line.split(b"\t", 1)
        path = path.decode("utf-8")
        mode, kind, oid = meta.decode("ascii").split()
        stage = path.split("/")[3][:2] if path.startswith(RESEARCH) else None
        if stage in ("14", "15", "16", "17", "18", "19") or path in INPUTS:
            require(kind == "blob" and mode in ("100644", "100755"),
                    "Unexpected non-file input: " + path)
            entries.append((path, oid))
    return sorted(entries)


def read_blobs(entries):
    data = git("cat-file", "--batch", data="".join(
        oid + "\n" for _, oid in entries).encode("ascii"))
    offset = 0
    result = {}
    for path, expected in entries:
        end = data.index(b"\n", offset)
        oid, kind, size = data[offset:end].decode("ascii").split()
        size = int(size)
        raw = data[end + 1:end + 1 + size]
        require(oid == expected and kind == "blob" and len(raw) == size,
                "Bad batch header: " + path)
        require(hashlib.sha1(b"blob " + str(size).encode() + b"\0" + raw).hexdigest()
                == oid, "Git blob digest mismatch: " + path)
        require(data[end + 1 + size:end + 2 + size] == b"\n", "Bad batch framing")
        offset = end + 2 + size
        result[path] = raw
    require(offset == len(data), "Trailing batch output")
    return result


def is_record(path):
    return path.endswith((".json", ".json.gz", ".jsonl", ".jsonl.gz"))


def manifest(entries, blobs):
    research = [(p, h) for p, h in entries if p.startswith(RESEARCH)]
    full_digest = sha("".join(p + "\0" + h + "\n" for p, h in research).encode())
    records = []
    for path, oid in research:
        if not is_record(path):
            continue
        require(".jsonl" not in path, "Unexpected JSONL baseline member")
        raw = blobs[path]
        decoded = gzip.decompress(raw) if path.endswith(".gz") else raw
        value = json.loads(decoded)
        records.append(dict(path=path, git_blob=oid, stored_bytes=len(raw),
                            stored_sha256=sha(raw), decoded_bytes=len(decoded),
                            decoded_sha256=sha(decoded),
                            canonical_bytes=len(canonical(value)),
                            canonical_sha256=sha(canonical(value))))
    require(len(records) == 497, "Record count differs from 497")
    require(sum(r["stored_bytes"] for r in records) == 13412769,
            "Stored bytes differ from baseline")
    require(sum(r["path"].endswith(".json.gz") for r in records) == 80,
            "Compressed-file count differs from 80")
    return dict(format="ps1.38-git-manifest/1", frozen_commit=BASE,
                full_corpus_manifest_sha256=full_digest,
                full_corpus_entries=len(research),
                full_corpus_manifest_encoding="sorted UTF-8 path + NUL + Git blob OID + LF",
                post_rename_manifest_sha256=POST_RENAME,
                post_rename_matches=full_digest == POST_RENAME,
                record_manifest_sha256=sha(canonical(records)), records=records,
                auxiliary_corpus=[dict(path=p, git_blob=h, stored_bytes=len(blobs[p]),
                                       stored_sha256=sha(blobs[p]))
                                  for p, h in research if not is_record(p)],
                inputs=[dict(path=p, git_blob=h, bytes=len(blobs[p]), sha256=sha(blobs[p]))
                        for p, h in entries if p in INPUTS])


def classifier(source):
    """Reuse only the audited classifier definitions, never its I/O or main()."""
    parsed = ast.parse(source, filename=IMPORT + "extract_labels.py")
    names = {"KEY", "L", "LEX", "VAL", "BAD", "P", "PRO", "B", "BL"}
    nodes = []
    for node in parsed.body:
        if isinstance(node, ast.FunctionDef) and node.name in ("cls", "last", "syn"):
            nodes.append(node)
        elif isinstance(node, (ast.Assign, ast.AugAssign)):
            targets = node.targets if isinstance(node, ast.Assign) else [node.target]
            if all(isinstance(t, ast.Name) and t.id in names for t in targets):
                nodes.append(node)
    import re
    namespace = {"re": re}
    exec(compile(ast.Module(body=nodes, type_ignores=[]),
                 IMPORT + "extract_labels.py", "exec"), namespace)
    return namespace


def walk(value, fmt, field, pointer, namespace):
    if isinstance(value, dict):
        if isinstance(value.get("format"), str) and value["format"]:
            fmt = value["format"]
        for key, child in value.items():
            normalized = "<k>" if namespace["KEY"].match(key) else key
            escaped = key.replace("~", "~0").replace("/", "~1")
            yield from walk(child, fmt, field + "." + normalized,
                            pointer + "/" + escaped, namespace)
    elif isinstance(value, list):
        for index, child in enumerate(value):
            yield from walk(child, fmt, field + "[]", pointer + "/" + str(index), namespace)
    else:
        yield fmt, field.lstrip("."), pointer, value


def pointer_value(value, pointer):
    for token in pointer.split("/")[1:]:
        token = token.replace("~1", "/").replace("~0", "~")
        value = value[int(token)] if isinstance(value, list) else value[token]
    return value


def scan(man, blobs):
    namespace = classifier(blobs[IMPORT + "extract_labels.py"])
    table = json.loads(blobs[IMPORT + "corpus-labels.json"])
    rows = [r for r in table["rows"] if r["cls"] in ("S", "B")]
    lookup = {(r["format"], r["field"], canonical(r["value"])): i
              for i, r in enumerate(rows)}
    require(len(rows) == len(lookup) == 387, "Label table changed")
    require(sum(r["code"] not in ("rec", "none") for r in rows) == 298,
            "Mapped table count changed")
    require(sum(r["code"] == "rec" for r in rows) == 89
            and not any(r["code"] == "none" for r in rows), "Unmapped table count changed")
    require(len(table["rows"]) - len(rows) == 24, "External-row count changed")
    over = {tuple(x) for x in table["over_limit"]}
    counts, files, values = Counter(), defaultdict(set), defaultdict(set)
    records = []
    for item in man["records"]:
        path = item["path"]
        raw = blobs[path]
        value = json.loads(gzip.decompress(raw) if path.endswith(".gz") else raw)
        observations, prose, digest_fields = [], [], []
        payload = Counter()
        for fmt, field, pointer, atom in walk(value, None, "", "", namespace):
            cls = namespace["cls"](field, atom)
            fmt = fmt or namespace["syn"](path)
            if cls == "S":
                values[(fmt, field)].add(canonical(atom))
            if cls in ("S", "B") and (fmt, field) not in over:
                key = (fmt, field, canonical(atom))
                require(key in lookup, "Uncatalogued label at " + path + pointer)
                rid = lookup[key]
                observations.append(dict(at=pointer, row=rid))
                counts[rid] += 1
                files[rid].add(path)
            elif cls == "T":
                prose.append(dict(at=pointer, value=atom))
            elif cls == "P":
                payload["boolean"] += 1
            elif isinstance(atom, (int, float)) and not isinstance(atom, bool):
                payload["number"] += 1
            # A bare SHA-256 is a reported digest, not yet a content dependency.
            if isinstance(atom, str) and len(atom) == 64 and all(c in "0123456789abcdef" for c in atom):
                digest_fields.append(dict(at=pointer, digest=atom,
                                          role="untyped_source_digest"))
        require(all(pointer_value(value, o["at"]) == rows[o["row"]]["value"]
                    for o in observations), "Pointer does not preserve label")
        records.append(dict(source=item, value=value, observations=observations,
                            prose=prose, digests=digest_fields, payload=dict(payload)))
    require({key for key, vs in values.items() if len(vs) > 32} == over,
            "Label cardinality exclusion changed")
    for rid, row in enumerate(rows):
        require(counts[rid] == row["count"] and len(files[rid]) == row["files"],
                "Label coverage mismatch at table row " + str(rid))
    require(sum(counts.values()) == 22513, "Label occurrence count differs")
    return records, rows, table
