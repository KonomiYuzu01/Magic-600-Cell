"""Regenerate and compare the draft FR-B08 inventory; never run research code."""

import sys
sys.dont_write_bytecode = True

from collections import Counter, defaultdict
import gzip
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import shutil

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
PREFIX = "docs/architecture/1.0/language/claim-matrix/"
ENVELOPE = "work/experiments/language-lx/envelope-facts/"
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(ROOT / ENVELOPE))

import corpus
import envelopes
from plan import COMMON, COUPLING, FAMILIES, RESEARCH, STAGES, classify, path

HEX = re.compile(r"[0-9a-f]{64}\Z")
FILE = re.compile(r"\.(?:json(?:\.gz)?|jsonl|npz|npy|py|md|txt|ia16(?:\.gz)?)\Z")


def require(ok, message):
    if not ok:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def encode(value):
    # One compact record per line, keeping the complete pointer table below 2 MB.
    return (json.dumps(value, ensure_ascii=True, sort_keys=True, indent=1) + "\n").encode()


def pointer_token(key):
    return str(key).replace("~", "~0").replace("/", "~1")


def safe_name(name):
    name = name.replace("\\", "/")
    return bool(name and not name.startswith("/") and ":" not in name
                and ".." not in PurePosixPath(name).parts and "\n" not in name)


class Repository:
    """Only named repository blobs; no producer import, filesystem crawl or fetch."""

    def __init__(self):
        self.tree = {}
        raw = corpus.git("ls-tree", "-r", "-z", "HEAD", "research/jumbling", "assets", ENVELOPE)
        for line in raw.split(b"\0"):
            if not line:
                continue
            meta, name = line.split(b"\t")
            mode, kind, oid = meta.decode().split()
            name = name.decode()
            if mode in ("100644", "100755") and kind == "blob":
                self.tree[name] = oid
        self.raw = {}
        self.files = []
        self.ids = {}

    def load(self, names):
        names = sorted(set(names) - self.raw.keys())
        require(all(n in self.tree for n in names),
                "Required repository inputs absent: " + ", ".join(n for n in names if n not in self.tree))
        self.raw.update(corpus.read_blobs([(n, self.tree[n]) for n in names]))

    def ref(self, name):
        require(safe_name(name), "Non-repository path")
        if name not in self.ids:
            require(name in self.raw, "Unloaded input: " + name)
            self.ids[name] = len(self.files)
            self.files.append(dict(path=name, sha256=sha(self.raw[name]), present=True,
                                   bytes=len(self.raw[name]),
                                   boundary="worktree_bytes" if name.startswith(PREFIX) else "git_blob_bytes"))
        return self.ids[name]

    def candidates(self, owner, name):
        name = name.replace("\\", "/")
        if not safe_name(name):
            return []
        if name.startswith("source/"):
            name = name[len("source/"):]
        stage_root = "/".join(owner.split("/")[:4])
        options = [name, str(PurePosixPath(owner).parent / name), stage_root + "/" + name,
                   stage_root + "/records/" + name]
        # Compression is an explicit decoded boundary, never a guessed hash match.
        options += [n + ".gz" for n in options if n.endswith(".json")]
        return sorted({n for n in options if n in self.tree})


def declared_references(value, at=""):
    """File-named hashes and explicit file fields; arbitrary hashes stay untyped."""
    if isinstance(value, dict):
        for key, child in value.items():
            p = at + "/" + pointer_token(key)
            if FILE.search(key.replace("\\", "/")) and isinstance(child, str) and HEX.fullmatch(child):
                yield p, key, child
            elif isinstance(child, str) and FILE.search(child.replace("\\", "/")) and not any(c.isspace() for c in child):
                # A path/sha pair binds the digest occurrence, not the path string.
                hkey = next((k for k in ("sha256", "file_sha256", "table_sha256")
                             if key in ("path", "file", "table") and isinstance(value.get(k), str)
                             and HEX.fullmatch(value[k])), None)
                yield (at + "/" + hkey if hkey else p), child, value[hkey] if hkey else None
            if isinstance(child, (dict, list)):
                yield from declared_references(child, p)
    elif isinstance(value, list):
        for i, child in enumerate(value):
            yield from declared_references(child, at + "/" + str(i))


def decoded(name, raw):
    return gzip.decompress(raw) if name.endswith(".gz") else raw


def digest_index(repo):
    index = defaultdict(list)
    for name, raw in sorted(repo.raw.items()):
        index[sha(raw)].append((name, "stored"))
        if name.endswith(".gz"):
            index[sha(decoded(name, raw))].append((name, "decoded"))
    return index


def dependency(repo, owner, at, name, expected, index, originals, manifest_name):
    result = dict(source=repo.ref(owner), at=at,
                  name=name.replace("\\", "/") if safe_name(name) else "source-located reference",
                  # A reported historical hash is addressed in its immutable source.
                  # It is not advertised as a resolved file digest in this matrix.
                  digest_at=at if expected else None)
    if expected in index:
        target, boundary = index[expected][0]
        result.update(present=True, historical_bytes_present=True, resolution="exact_" + boundary,
                      target=repo.ref(target))
    elif expected in originals:
        target, pointer = originals[expected]
        result.update(present=True, historical_bytes_present=False, resolution="public_correspondence",
                      target=repo.ref(target), correspondence=dict(file=repo.ref(manifest_name), at=pointer))
    else:
        candidates = repo.candidates(owner, name)
        target = candidates[0] if len(candidates) == 1 else None
        if expected is None and target:
            result.update(present=True, historical_bytes_present=None, resolution="path_selected_current_input",
                          target=repo.ref(target))
        else:
            result.update(present=False, historical_bytes_present=False,
                          resolution="dependency_mismatch" if target and expected else "dependency_missing",
                          target=None)
            if target:
                result["different_edition"] = repo.ref(target)
    return result


def closure(start, edges, by_source):
    found, pending, visited = set(), [start], set()
    while pending:
        source = pending.pop()
        if source in visited:
            continue
        visited.add(source)
        for i in by_source.get(source, []):
            found.add(i)
            if edges[i]["target"] is not None:
                pending.append(edges[i]["target"])
    return sorted(found)


def source_table(repo, name, value, values, table_by_digest, derived_states, family_id):
    """Locate archival start bindings; no geometry/digest recomputation is implied."""
    refs = {name}
    if not isinstance(value, dict):
        return refs, False
    v = value
    if v.get("format") == "macro-check/1":
        candidates = [name.replace("-check.json", ".json"), name.replace("-check.json", "-cert.json")]
        pair = next((p for p in candidates if p in values and values[p].get("format") == "macro-certificate/1"), None)
        if pair:
            refs.add(pair)
            v = values[pair]
    if v.get("format") == "t3p-check/1":
        suffix = name.rsplit("-", 1)[-1]
        pair = path(14, "results/s7304-" + suffix.replace(".json", "-certificate.json"))
        if pair in values:
            refs.add(pair)
            v = values[pair]
    table = v.get("table")
    if isinstance(table, str) and table in repo.raw:
        refs.add(table)
        return refs, True
    table_digest = v.get("table_digest")
    if table_digest in table_by_digest:
        table, catalogue = table_by_digest[table_digest]
        refs.update((table, catalogue))
        return refs, True
    if family_id == "relations":
        if value.get("format") == "t3p-locality/1":
            # The all-cap locality relation depends on the immutable model, not a state.
            return refs, True
        state = v.get("digest")
        if state in table_by_digest:
            refs.update(table_by_digest[state])
            return refs, True
        if state in derived_states:
            refs.update(derived_states[state])
            return refs, True
    # A named jam trace binds its start through the complete table/chain/macros.
    jam_name = path(16, "jams.json")
    seed = v.get("seed")
    jams = values[jam_name]
    if isinstance(seed, str) and seed in jams:
        table = jams[seed]["table"]
        if table in repo.raw:
            trace = v.get("trace", [])
            if trace and trace[0].get("digest") == jams[seed]["end_digest"]:
                refs.update((table, jam_name))
                return refs, True
    return refs, False


def requirements(repo, source, family_id, values, table_by_digest, derived_states, stage_names):
    spec = FAMILIES[family_id]
    policy = spec["input_policy"]
    names = {source}
    missing = []
    if policy != "record":
        names.update(COMMON)
    names.update(spec["extra_inputs"])
    if policy == "needs":
        missing.append(spec["missing_input"])
    elif policy == "table":
        refs, found = source_table(repo, source, values[source], values, table_by_digest, derived_states, family_id)
        names.update(refs)
        if not found:
            missing.append("exact full start table and prefix/word binding for this source record")
    elif policy == "parts":
        parent = source.rsplit("/", 1)[0]
        if source.endswith("/steps.json"):
            names.update(n for n in stage_names if n.startswith(parent + "/"))
        else:
            names.update(n for n in stage_names if n.rsplit("/", 1)[0] == parent)
    elif policy == "lattice":
        names.update(n for n in stage_names if "/results/lattice-d1/" in n)
    for name in sorted(names):
        if name not in repo.raw:
            missing.append(name)
    status = "record_only" if policy == "record" else "needs_input" if missing else "self_contained"
    return dict(data_status=status, required_files=[repo.ref(n) for n in sorted(names) if n in repo.raw],
                missing_inputs=[dict(name=n, present=False, digest=None, source=repo.ref(source)) for n in missing])


def validate_accounting(matrix, expected):
    actual = set()
    records = {r["file"] for r in matrix["records"]}
    require(len(records) == len(matrix["records"]) == 497, "Record coverage differs")
    for row in matrix["groups"]:
        for p in row["leaves"]:
            key = row["source"], p
            require(key not in actual, "Duplicate leaf assignment")
            actual.add(key)
        if row["out_of_scope"]:
            require(bool(row["out_of_scope_reason"]), "Unreasoned exclusion")
    require(actual == expected, "Leaf coverage differs")
    require(len(actual) == matrix["counts"]["leaf_slots"] == 11467, "Leaf total differs")


def validate_files(matrix, repo):
    for item in matrix["files"]:
        require(item["present"] is True, "Digest catalogue contains an absent file")
        require(item["path"] in repo.raw and safe_name(item["path"]), "Unresolved file digest")
        require(item["sha256"] == sha(repo.raw[item["path"]]), "File digest mismatch: " + item["path"])
        require(item["bytes"] == len(repo.raw[item["path"]]), "File size mismatch")
    for edge in matrix["dependencies"]:
        if edge["present"]:
            require(isinstance(edge["target"], int) and 0 <= edge["target"] < len(matrix["files"]),
                    "Present dependency has no file")
        else:
            require(edge["target"] is None, "Absent dependency resolved as present")


def negative_checks(matrix, expected, repo):
    def refuses(action, message):
        try:
            action()
        except ValueError:
            return
        raise ValueError("Negative control accepted: " + message)

    row = next(r for r in matrix["groups"] if r["leaves"])
    leaf = row["leaves"].pop()
    refuses(lambda: validate_accounting(matrix, expected), "missing leaf")
    row["leaves"].append(leaf)
    row["leaves"].append(leaf)
    refuses(lambda: validate_accounting(matrix, expected), "duplicate leaf")
    row["leaves"].pop()
    old = matrix["files"][0]["sha256"]
    matrix["files"][0]["sha256"] = "0" * 64
    refuses(lambda: validate_files(matrix, repo), "altered digest")
    matrix["files"][0]["sha256"] = old
    matching = [g for g in matrix["groups"] if g["family"] == "19/matching"]
    require(matching and all(g["data_status"] == "self_contained" and g["fresh_status"] == "needs_input"
                            for g in matching), "Matching witness/checker distinction lost")
    q11 = next(g for g in matrix["groups"] if g["family"] == "19/q11")
    require(q11["data_status"] == "needs_input", "q11 lower bound promoted")
    require(any(e["name"].endswith("type-lex-successor-base-cache.npz") and not e["present"]
                for e in matrix["dependencies"]), "Missing successor cache disappeared")
    require(any(e["resolution"] == "public_correspondence" and not e["historical_bytes_present"]
                for e in matrix["dependencies"]), "Public bytes confused with historical bytes")
    gates = [g for g in matrix["groups"] if g["family"] == "18/tail_replay"]
    require(sum(len(g["historically_incomplete"]) for g in gates) == 4, "Unfinished gate 3 lost")
    require(all(g["fresh_status"] != "self_contained" for g in matrix["groups"]), "Invented independent checker")
    chain_stops = [g for g in matrix["groups"] if g["family"] == "14/finite_search"
                   and "/stop" in g["leaves"]]
    require(len(chain_stops) == 6 and all(g["data_status"] == "needs_input" for g in chain_stops),
            "Replay promoted chain-stop completeness")
    return 10


def generate():
    repo = Repository()
    envelope_names = [ENVELOPE + n for n in ("corpus.py", "envelopes.py", "MANIFEST.json", "RECORDS.json",
                                            "DEPENDENCIES.json", "CANDIDATE.json")]
    repo.load(envelope_names)
    man = json.loads(repo.raw[ENVELOPE + "MANIFEST.json"])
    frozen = {r["path"]: r["git_blob"] for section in ("records", "auxiliary_corpus") for r in man[section]}
    current = {n: oid for n, oid in repo.tree.items()
               if n.startswith(RESEARCH) and n.split("/")[3][:2] in {str(s) for s in STAGES}}
    require(current == frozen, "Stage 14-19 corpus differs from the envelope inventory")
    required = set(frozen) | set(COMMON)
    for f in FAMILIES.values():
        required.update(f["extra_inputs"])
        required.update(f["producer_paths"])
    required.update(n for edge in COUPLING for n in (edge[0], edge[2]))
    # The plan names historical source paths explicitly; fail on typos.
    repo.load(required)
    for item in json.loads(repo.raw[ENVELOPE + "CANDIDATE.json"]):
        if item["path"] in envelope_names:
            require(sha(repo.raw[item["path"]]) == item["sha256"], "Envelope extractor pin differs")
            local = (ROOT / item["path"]).read_bytes().replace(b"\r\n", b"\n")
            require(local == repo.raw[item["path"]].replace(b"\r\n", b"\n"), "Local extractor differs")
    # Use only the frozen label extractor/table needed by scan(), not moving chapters.
    table_inputs = [r for r in man["inputs"] if r["path"] in
                    (corpus.IMPORT + "extract_labels.py", corpus.IMPORT + "corpus-labels.json")]
    scan_blobs = dict(repo.raw)
    scan_blobs.update(corpus.read_blobs([(r["path"], r["git_blob"]) for r in table_inputs]))
    for r in man["records"]:
        require(sha(repo.raw[r["path"]]) == r["stored_sha256"], "Envelope source digest differs")
    print("Frozen research records resolved; reusing envelope extraction", flush=True)
    records, labels, _ = corpus.scan(man, scan_blobs)
    values = {r["source"]["path"]: r["value"] for r in records}
    declared = {n: sorted(set(declared_references(v))) for n, v in values.items()}
    candidates = set()
    for n, refs in declared.items():
        for _, name, _ in refs:
            candidates.update(repo.candidates(n, name))
    repo.load(candidates)
    # Catalogues are data bindings: state digests are not treated as file hashes.
    table_by_digest = {}
    sources_name = path(14, "results/sources.json")
    for r in values[sources_name]:
        if r["table"] in repo.raw:
            require(sha(repo.raw[r["table"]]) == r["table_sha256"], "Source-table file digest differs")
            table_by_digest[r["digest"]] = (r["table"], sources_name)
    jams_name = path(16, "jams.json")
    for r in values[jams_name].values():
        if r["table"] in repo.raw:
            table_by_digest[r["table_digest"]] = (r["table"], jams_name)
    derived_states = {r["end_digest"]: (r["table"], jams_name)
                      for r in values[jams_name].values() if r["table"] in repo.raw}
    for name, v in values.items():
        if not isinstance(v, dict) or not isinstance(v.get("table"), str) or v["table"] not in repo.raw:
            continue
        if "word" not in v:
            continue
        end = v.get("end_digest")
        if isinstance(v.get("rows"), list) and v["rows"] and isinstance(v["rows"][-1], dict):
            end = v["rows"][-1].get("digest", end)
        if isinstance(end, str) and HEX.fullmatch(end):
            derived_states[end] = (v["table"], name)
    manifest_name = path(19, "IMPORT-MANIFEST.json")
    originals = {}
    for i, item in enumerate(values[manifest_name]["files"]):
        name = path(19, item["path"])
        require(sha(repo.raw[name]) == item["sha256"], "Public stored digest differs")
        require(sha(decoded(name, repo.raw[name])) == item["uncompressed_sha256"], "Public decoded digest differs")
        originals[item["original_sha256"]] = (name, "/files/" + str(i))
    index = digest_index(repo)
    for n in ("check_matrix.py", "plan.py"):
        repo.raw[PREFIX + n] = (HERE / n).read_bytes()
    provenance = [repo.ref(n) for n in envelope_names + [PREFIX + "check_matrix.py", PREFIX + "plan.py"]]
    # Frozen table bytes are addressed through MANIFEST.json's input descriptors.
    # No moving architecture, decision or briefing digest is a validation input.
    source_walk = []
    for source, token, target in COUPLING:
        lines = repo.raw[source].decode().splitlines()
        line = next((i + 1 for i, s in enumerate(lines) if token in s), None)
        require(line is not None, "Source coupling witness changed: " + source)
        source_walk.append(dict(source=repo.ref(source), line=line, imports=repo.ref(target)))
    edges = []
    by_source = defaultdict(list)
    for owner, refs in declared.items():
        if owner == manifest_name:
            continue  # publication descriptors are validated above, not claim inputs
        for at, name, expected in refs:
            by_source[repo.ref(owner)].append(len(edges))
            edges.append(dependency(repo, owner, at, name, expected, index, originals, manifest_name))
    saved_records = {r["path"]: r for r in json.loads(repo.raw[ENVELOPE + "RECORDS.json"])}
    edition = sha(corpus.canonical(json.loads(repo.raw[ENVELOPE + "DEPENDENCIES.json"])["edition"]))
    groups, record_rows, expected_leaves = [], [], set()
    fact_groups = 0
    for record in records:
        source = record["source"]["path"]
        source_id = repo.ref(source)
        stage = int(source.split("/")[3][:2])
        relative = source.split("/", 4)[4]
        envelope = envelopes.build_one(record, labels, edition)
        require(sha(corpus.canonical(envelope)) == saved_records[source]["envelope_sha256"],
                "Regenerated envelope differs: " + source)
        fact_groups += len(envelope["facts"])
        by_family = defaultdict(lambda: dict(leaves=[], anchors=[]))
        by_family[classify(stage, relative, record["value"], "")]["anchors"].append("")
        for leaf in envelope["graph"]["leaves"]:
            p = envelope["facts"][leaf["fact_group"]]["at"]
            require((source_id, p) not in expected_leaves, "Duplicate envelope leaf identity")
            expected_leaves.add((source_id, p))
            by_family[classify(stage, relative, record["value"], p)]["leaves"].append(p)
        if source == path(19, "records/type-lex-trimmed-independent-results.json"):
            by_family["ranks"]["anchors"] += ["/one_fixed_L3", "/one_fixed_S", "/generic_mixed_rank_proof_check"]
        if stage == 18 and relative in ("results/d1-tail/steps.json", "results/rule-stop/steps.json"):
            anchors = ["/steps/" + str(i) for i in range(len(record["value"]["steps"]))]
            by_family["composed"]["anchors"] += anchors
            by_family["tail_replay"]["anchors"] += anchors
        transitive = closure(source_id, edges, by_source)
        source_edges = by_source[source_id]
        declared_digest_ats = {edges[i]["at"] for i in source_edges if edges[i]["digest_at"] is not None}
        digest_counts = Counter()
        for d in record["digests"]:
            category = ("declared_file_reference" if d["at"] in declared_digest_ats else
                        "exact_byte_match_not_typed_dependency" if d["digest"] in index else
                        "source_correspondence_not_typed_dependency" if d["digest"] in originals else
                        "untyped_state_content_or_unresolved")
            digest_counts[category] += 1
        record_rows.append(dict(file=source_id, stage=stage, source_format=envelope["source_format"],
                                fact_groups=len(envelope["facts"]), leaf_slots=len(envelope["graph"]["leaves"]),
                                reported_digest_occurrences=dict(sorted(digest_counts.items())),
                                dependencies=source_edges, transitive_dependencies=transitive))
        for fid, members in sorted(by_family.items()):
            for p in members["anchors"]:
                corpus.pointer_value(record["value"], p)
            req = requirements(repo, source, fid, values, table_by_digest, derived_states, frozen)
            reasons = ["claim_unmapped", "no_independent_checker"]
            if req["data_status"] == "needs_input":
                reasons.append("dependency_missing")
            if req["data_status"] == "record_only":
                reasons = ["unsupported_claim"]
            incomplete = []
            if fid == "tail_replay":
                incomplete = members["anchors"][3:] if "/d1-tail/" in source else members["anchors"]
                if incomplete:
                    reasons.append("historical_check_incomplete")
            historical_reasons = {"claim_unmapped", "no_independent_checker"}
            for i in transitive:
                edge = edges[i]
                if not edge["present"]:
                    historical_reasons.add(edge["resolution"])
                elif edge["resolution"] == "public_correspondence":
                    historical_reasons.add("dependency_missing")
            if incomplete:
                historical_reasons.add("historical_check_incomplete")
            # A complete-data plan still awaits its named, separated and mutation-gated checker.
            groups.append(dict(source=source_id, family=str(stage) + "/" + fid, **members, **req,
                               fresh_status="record_only" if req["data_status"] == "record_only" else "needs_input",
                               outcome=dict(kind="unavailable", reasons=sorted(reasons)),
                               historical_outcome=dict(kind="unavailable", reasons=sorted(historical_reasons)),
                               historical_reproduction="record_only" if any(
                                   not edges[i]["historical_bytes_present"] for i in transitive) else "not_qualified",
                               historically_incomplete=incomplete, out_of_scope=False, out_of_scope_reason=None))
    # An explicit promised family with no envelope leaf must not vanish from the plan.
    global_source = path(19, "README.md")
    groups.append(dict(source=repo.ref(global_source), family="19/global_open", leaves=[],
                       anchors=["Remaining obligations (prose)"], data_status="record_only", required_files=[repo.ref(global_source)],
                       missing_inputs=[], fresh_status="record_only", outcome=dict(kind="unavailable", reasons=["unsupported_claim"]),
                       historical_outcome=dict(kind="unavailable", reasons=["unsupported_claim"]),
                       historical_reproduction="record_only", historically_incomplete=[], out_of_scope=False, out_of_scope_reason=None))
    families = []
    for full_id in sorted({g["family"] for g in groups}):
        stage, fid = full_id.split("/")
        spec = FAMILIES[fid]
        members = [g for g in groups if g["family"] == full_id]
        families.append(dict(id=full_id, stage=int(stage), claim_family=fid, asserts=spec["asserts"],
                             scope="Exact source/pointer scope only; EVD statement/key/covered bindings remain untyped.",
                             producer=dict(source_walk_files=[repo.ref(n) for n in spec["producer_paths"]],
                                           identity="Historical identity stays at source; these paths are checking/construction leads, not reconstructed attribution.",
                                           status="historical_not_archived" if not spec["producer_paths"] else "research_path_not_KN6_qualified"),
                             checker=dict(name=spec["planned_checker"], status="not_implemented", source_digest=None,
                                          common_base_files=[repo.ref(n) for n in COMMON],
                                          gate="separate producer/search code; wrong-start and per-claim mutation failures required"),
                             limits=spec["limits"], records=len({g["source"] for g in members}), groups=len(members),
                             leaf_slots=sum(len(g["leaves"]) for g in members),
                             data_status_counts=dict(sorted(Counter(g["data_status"] for g in members).items())),
                             fresh_status_counts=dict(sorted(Counter(g["fresh_status"] for g in members).items()))))
    matrix = dict(format="fr-b08-claim-matrix-draft/1", status="draft; not accepted or frozen; no route, language, syntax or LQ selected",
                  evidence="source inventory only; no mathematical checker or research producer executed",
                  provenance_files=provenance, files=repo.files, source_walk=source_walk,
                  families=families, records=record_rows, groups=groups, dependencies=edges,
                  counts=dict(records=len(records), fact_groups=fact_groups, leaf_slots=len(expected_leaves),
                              families=len(families), groups=len(groups), out_of_scope_leaves=0,
                              admitted_support_leaves=0, ready_fresh_checks=0,
                              dependencies=len(edges), dependency_resolutions=dict(sorted(Counter(e["resolution"] for e in edges).items()))))
    require(fact_groups == 15210, "Fact-bundle total differs")
    validate_accounting(matrix, expected_leaves)
    validate_files(matrix, repo)
    matrix["counts"]["negative_controls"] = negative_checks(matrix, expected_leaves, repo)
    return matrix


def count_table(matrix):
    lines = ["| Stage/family | Records | Groups | Leaves | Data self-contained | Data needs input | Record only |",
             "| --- | ---: | ---: | ---: | ---: | ---: | ---: |"]
    for f in matrix["families"]:
        c = f["data_status_counts"]
        lines.append("| " + " | ".join(str(x) for x in (f["id"], f["records"], f["groups"], f["leaf_slots"],
                     c.get("self_contained", 0), c.get("needs_input", 0), c.get("record_only", 0))) + " |")
    return "\n".join(lines)


def main():
    require(sys.argv[1:] in ([], ["--write"]), "Usage: check_matrix.py [--write]")
    build = HERE / "build"
    require(not build.exists(), "build/ already exists; inspect it before running")
    build.mkdir()
    try:
        matrix = generate()
        raw = encode(matrix)
        require(len(raw) < 2_000_000, "Matrix exceeds 2 MB")
        require(b"\r" not in raw, "Non-LF output")
        fresh = build / "matrix.json"
        fresh.write_bytes(raw)
        if sys.argv[1:] == ["--write"]:
            (HERE / "matrix.json").write_bytes(raw)
            print(count_table(matrix))
        else:
            require((HERE / "matrix.json").read_bytes() == fresh.read_bytes(), "Stored matrix differs")
            readme = (HERE / "README.md").read_text(encoding="utf-8")
            require(count_table(matrix) in readme, "README family counts differ")
        c = matrix["counts"]
        print(f"PASS: {c['records']} records; {c['fact_groups']} fact bundles; {c['leaf_slots']} leaves; "
              f"{c['families']} families; {c['out_of_scope_leaves']} excluded; all file digests resolved; "
              f"{c['negative_controls']} negative controls; no research checks run")
    finally:
        require(build.resolve() == HERE.resolve() / "build" and not build.is_symlink(), "Unsafe build cleanup")
        shutil.rmtree(build)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, KeyError, json.JSONDecodeError) as error:
        print("FAIL: " + (str(error) if isinstance(error, ValueError) else type(error).__name__), file=sys.stderr)
        sys.exit(1)
