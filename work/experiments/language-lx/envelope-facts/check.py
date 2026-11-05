"""Rebuild PS1.38 from Git; --write refreshes only this experiment's small outputs."""

import sys
sys.dont_write_bytecode = True

from collections import Counter
import copy
import json
import platform
import zlib

from corpus import (BASE, HERE, IMPORT, PREFIX, POST_RENAME, canonical, git,
                    manifest, pack, read_blobs, require, scan, sha, tree)
from display import enumerate_model
from envelopes import build, recorded_word

CANDIDATE = ("corpus.py", "envelopes.py", "display.py", "check.py", "envelope_model.md", "CARD.md")


def pretty(value):
    return (json.dumps(value, sort_keys=True, ensure_ascii=True, indent=1,
                       allow_nan=False) + "\n").encode("utf-8")


def candidate():
    return [dict(path=PREFIX + name, sha256=sha((HERE / name).read_bytes()))
            for name in CANDIDATE]


def negative_checks(envelopes, records, rows):
    """Prevent accidental scope/claim manufacture and source-pointer loss."""
    require(len(envelopes) == len(records) == 497, "Lost source record")
    require(len({e["source"]["path"] for e in envelopes}) == 497, "Duplicate envelope")
    require(all(recorded_word(e)["word"] == "open" for e in envelopes), "Untyped claim upgraded")
    for envelope in envelopes:
        for group in envelope["facts"]:
            if group["role"] == "reachability":
                require(all(rows[o["row"]]["rule"] == "H1" for o in group["labels"]),
                        "H1 merged with another claim")
            for field, choices in group["table_candidates"].items():
                for choice in choices:
                    require(any(rows[o["row"]]["mapping"].get(field) == choice["value"]
                                and o["at"] in choice["at"] for o in group["labels"]),
                            "Candidate field has no source row")
    for mutation in ("typed", "edge", "leaf"):
        specimen = copy.deepcopy(next(e for e in envelopes if e["graph"]["leaves"]))
        if mutation == "typed":
            specimen["fields"]["scope"]["value"] = "invented"
        elif mutation == "edge":
            specimen["graph"]["edges"] = [["unknown", "unknown"]]
        else:
            specimen["graph"]["leaves"][0]["admission"] = "proved"
        try:
            recorded_word(specimen)
        except ValueError:
            pass
        else:
            raise ValueError("Invented support was not refused")


def select_samples(envelopes, rows):
    ordered = sorted(envelopes, key=lambda e: (len(canonical(e)), e["source"]["path"]))
    selected = []
    for stage in ("14", "15", "16", "17", "18", "19"):
        selected.append(next(e for e in ordered if e["stage"] == stage))
    for rule in ("K1", "R6", "P11", "H1"):
        matches = [e for e in ordered if e not in selected and any(
            rows[o["row"]]["rule"] == rule for g in e["facts"] for o in g["labels"])]
        selected.append(matches[0] if matches else next(e for e in ordered if e not in selected))
    require(len(selected) == 10, "Sample count changed")
    return dict(selection="smallest envelope per stage, then K1/R6/P11/H1 examples; not representative of sizes",
                envelopes=selected)


def metrics(per_record, dependency, man, blobs, envelopes):
    sums = Counter()
    numeric = [key for key, value in per_record[0].items() if isinstance(value, int) and not isinstance(value, bool)]
    stages = {}
    for record in per_record:
        for key in numeric:
            sums[key] += record[key]
        stage = record["path"].split("/")[3][:2]
        stages.setdefault(stage, Counter())["records"] += 1
        for key in numeric:
            stages[stage][key] += record[key]
    catalogue = canonical(dependency)
    # Input artifacts are stored exactly once, outside the per-record metadata.
    common_bytes = sum(len(blobs[r["path"]]) for r in man["inputs"])
    common_gzip = sum(len(pack(blobs[r["path"]])) for r in man["inputs"])
    manifest_bytes = canonical(man)
    envelope_stream = b"[" + b",".join(canonical(e) for e in envelopes) + b"]"
    sums.update(dependency_catalogue_canonical_bytes=len(catalogue),
                dependency_catalogue_gzip_bytes=len(pack(catalogue)),
                common_input_captured_bytes=common_bytes, common_input_gzip_bytes=common_gzip,
                manifest_canonical_bytes=len(manifest_bytes), manifest_gzip_bytes=len(pack(manifest_bytes)))
    sums["envelope_package_canonical_bytes"] = (sums["envelope_canonical_bytes"] + len(catalogue)
                                                 + common_bytes + len(manifest_bytes))
    sums["envelope_package_gzip_bytes"] = (sums["envelope_gzip_bytes"] + len(pack(catalogue))
                                            + common_gzip + len(pack(manifest_bytes)))
    sums["captured_source_plus_envelope_package_bytes"] = (sums["original_stored_bytes"]
                                                            + sums["envelope_package_gzip_bytes"])
    return dict(records=len(per_record), totals=dict(sums), stages=stages,
                envelope_array_sha256=sha(envelope_stream), envelope_array_bytes=len(envelope_stream),
                dependency_catalogue_sha256=sha(catalogue),
                complete_envelopes=sum(r["complete"] for r in per_record),
                incomplete_envelopes=sum(not r["complete"] for r in per_record))


def summary_table(summary):
    lines = ["# PS1.38 summary", "",
             "Stored leaf slots are incomplete source fact candidates; admitted EVD leaves are zero.", "",
             "| Stage | Records | Source stored bytes | Source canonical bytes | Envelope canonical bytes | Envelope gzip bytes | Leaf slots | AND | OR | Edges |",
             "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |"]
    for stage, row in sorted(summary["stages"].items()):
        lines.append("| " + " | ".join(str(x) for x in [stage, row["records"], row["original_stored_bytes"],
                     row["original_canonical_bytes"], row["envelope_canonical_bytes"], row["envelope_gzip_bytes"],
                     row["leaf_slots"], row["AND"], row["OR"], row["edges"]]) + " |")
    row = summary["totals"]
    lines += ["", "| Measure | Bytes or count |", "| --- | ---: |"]
    lines += ["| " + key + " | " + str(value) + " |" for key, value in sorted(row.items())]
    lines += ["", "All 497 source members are included in both columns, even without any labels.",
              "Source stored bytes retain the original 417 JSON / 80 gzip mix. Canonical gzip columns",
              "compress every member separately at level 9, mtime 0. These are different boundaries.",
              "Package rows include the shared dependency catalogue, pinned common inputs and manifest once.",
              "The source-preserving row additionally includes every captured original member once.",
              "Unresolved reported digests are not fetched and their absent bytes are not counted.", ""]
    return "\n".join(lines).encode()


def result(summary, dependency, display, man, candidates, versions):
    t = summary["totals"]
    n = lambda key: f"{t[key]:,}"
    candidate_digest = sha(canonical(candidates))
    text = f"""# PS1.38 facts-envelope measurement

| CT-RTE-05 field | Record |
| --- | --- |
| LX card | PS1.38, supplementary evidence card E-2.4-LE; CARD.md |
| Experiment ID | E-2.4-LE / PS1.38 |
| Owner allowance | docs/wiki/decisions/owner-decisions-2026-11-05.md, decision 2; packet PS1.38 |
| Time box | what can be done now; one wrapper call of at most two hours |
| Frozen candidate | CANDIDATE.json; canonical inventory SHA-256 `{candidate_digest}` |
| Frozen inputs | Git `{BASE}`; MANIFEST.json; record inventory SHA-256 `{man['record_manifest_sha256']}` |
| Commands | `python {PREFIX}check.py --write`, then `python {PREFIX}check.py` |
| Tools | CPython {versions['python']}; {versions['git']}; zlib compile {versions['zlib_compile']}, runtime {versions['zlib_runtime']} |
| Evidence class | Source: corpus counts and serialization; synthetic: finite display cases. No new mathematical evidence. |
| Kill criteria | Wrong baseline, missing/duplicate record, label coverage mismatch, unsupported fact/edge, non-unique display result, stale artifact, or time limit: fail. |
| Actual result | 497 partial envelopes; 0 complete typed envelopes; every source record and 22,513 label occurrences accounted for. |
| Decision / decider | open / owner; no LQ answered and neither chapter accepted |
| Cannot show | A product migration format, lossless judgements, complete mathematical argument graphs, new verification/replay, an accepted display policy, performance, or any route/engine-language/syntax choice. |

## Corpus identity and equal inclusion

The source stored baseline is **497 records, 13,412,769 bytes**, comprising 417 `.json`
and 80 `.json.gz` members, no JSONL. Both the source and envelope columns include all
497 members, including lists, absent-format records, publication metadata and records
with no mapped label. No corpus file is read from the checkout or materialized by name.
Git `ls-tree` supplies paths and object IDs; `cat-file --batch` supplies verified bytes.

The full stage-14-to-19 manifest ({man['full_corpus_entries']} files, including non-records) is
`{man['full_corpus_manifest_sha256']}`. It **matches** the README's
post-rename digest at `8af824a4`; there is no corpus change to explain at this commit.
The older label table's `e2b388be...` digest predates the six certificate renames and
the stage-16 README note. It uses the same 497 record contents. The current manifest
has no case-colliding paths. Its digest uses the extractor's path-NUL-OID-LF encoding;
the separate record-only digest includes sizes and SHA-256 boundaries and is not comparable
to that full-corpus digest. The recorded label table stays frozen, including its old paths.

## Nodes, fields and missing semantics

The envelopes store **{n('fact_groups')} source-object fact bundles** assembled from
multiple rows where present, with **{n('leaf_slots')} leaf slots**, **0 AND nodes,
0 OR nodes and 0 edges**. These are incomplete leaf slots, not admitted support:
**0 leaves have a complete statement/scope key**. No artificial OR groups unrelated
claims, and no AND is inferred from a reused/check-mode label. Graph storage links
(row references, digest references) are counted separately from evidence edges.

All **497 records cannot form a complete typed envelope with these rules**. Every path
and every missing-field reason is listed in RECORDS.json. The common gaps include
statement identity and form, negation pair, subject level, typed identity/scope,
membership, search-to-statement binding, premise/guard classes, per-leaf covered set,
checker/tool/common-base bindings and reachability bindings. These are missing
*typed interpretations*, not claims that the raw records contain no relevant data.
Exact source pointers retain input tables, options, certificates, proof text, checks,
attribution and engineering classes where present; absent attribution stays absent.

The table has 387 triples: 298 mapped, 89 recorded with reasons, zero without a rule.
UNMAPPED.json lists all 89, with their original values, counts and reasons. The 24
external/code-declared rows are outside this corpus and create no envelopes. Prose,
payload booleans and numbers remain in pinned source bytes; prose scope strings also
remain beside the record's facts. All 22,513 label occurrences are verified against
exact JSON pointers and the frozen table's per-triple and per-file counts.

V14 remains a table interpretation of model review; it is never KN6 evidence. P13
and other mode labels add no transport edges. K1 gets no inherited checker without
a demonstrated covering relation. H1 rows have separate reachability bundles and
supply no support. A same-object bundle may describe several statements, so candidate
field values are retained as alternatives, never merged into a judgement. In particular
a failing guard and a completed job cannot turn into one fabricated contradiction.

## Bytes and sharing

| Boundary | Bytes |
| --- | ---: |
| Original members as stored (baseline) | {n('original_stored_bytes')} |
| Original members decoded, before canonicalization | {n('original_decoded_bytes')} |
| Original values, canonical uncompressed | {n('original_canonical_bytes')} |
| Original canonical values, gzip every member | {n('original_canonical_gzip_bytes')} |
| Envelope metadata, canonical uncompressed | {n('envelope_canonical_bytes')} |
| Envelope metadata, gzip every member | {n('envelope_gzip_bytes')} |
| Envelope package including common artifacts, canonical/captured bytes | {n('envelope_package_canonical_bytes')} |
| Envelope package including common artifacts, gzip bytes | {n('envelope_package_gzip_bytes')} |
| Captured original members plus gzip envelope package | {n('captured_source_plus_envelope_package_bytes')} |

SUMMARY.md gives stage totals and every component. Envelope-only bytes are **not** a
replacement for the records. The source-preserving figure keeps the captured inputs
once, so compression/formatting changes cannot erase their original digest boundaries.
These sizes measure this partial facts model; a future complete argument graph may be larger.

The import table, chapter/edition pins and other common input artifacts are referenced
497 times and counted once in the package. DEPENDENCIES.json accounts for reported
digest reuse: **{dependency['shared_reported_digests']:,} digests occur in multiple records**;
**{dependency['matched_reported_digests']:,}** have an exact stored/decoded byte match in the scoped
inputs, and **{dependency['unmatched_reported_digests']:,}** have no such match. No match is
invented from a filename. A reported digest can identify a state, an unavailable private
source edition or content; equal text is not automatically a shared evidence leaf or a
tool dependency. No independence or common failure-point claim is inferred from it.

Envelope-array canonical SHA-256: `{summary['envelope_array_sha256']}`
({summary['envelope_array_bytes']:,} bytes). Ten full envelopes are in SAMPLES.json;
all 497 canonical and compressed digests are in RECORDS.json. The full dependency
catalogue digest is `{summary['dependency_catalogue_sha256']}`.
The full arrays are regenerated in memory by the acceptance command, not committed.

## Display enumeration and open rules

DISPLAY.json records **{display['combinations']:,}** combinations of the explicitly
bounded synthetic facts model in envelope_model.md, including all five mode states.
Each has one word or one typed inconsistency. The output-stream SHA-256 is
`{display['outcomes_sha256']}`. The 504 EVD.32 node-level cases also run,
alongside {display['adversarial_checks']} fixed checks. Every actual partial envelope derives
`open` with its source labels retained, because a typed scope/key has not been bound.

- **Open collision:** `EF:collision.not_run_vs_budget`. With no standing support,
  incomplete search plus a deterministic/declared limit meets EVD.32 row 5, but
  EVD.47 says `not run` reads row 7. The probe reports a typed collision instead of
  resolving it; {display['counts'].get('EF:collision.not_run_vs_budget', 0):,} combinations expose it.
- **Open collision:** `EF:collision.existence_refutation_basis`. An exact-computation
  leaf for the negation of an existence statement can pass the equal-coverage I7 test
  and reach EVD.32 row 1, but EVD.42 requires a proof or complete certificate. The
  probe retains this as a typed collision ({display['counts'].get('EF:collision.existence_refutation_basis', 0):,} combinations);
  it does not invent a basis conversion or claim that the counterexample is sound.
- **Open job/support combinations:** `EF:collision.not_run_vs_support` and
  `EF:collision.negative_job_without_completion` occur in
  {display['counts'].get('EF:collision.not_run_vs_support', 0):,} and
  {display['counts'].get('EF:collision.negative_job_without_completion', 0):,} combinations respectively.
  In these fixtures the job belongs to the supporting leaf. Standing support reaches
  rows 1-4 despite a not-run job (EVD.47), or a search-record-free negative reaches
  row 1 despite its producing job failing to complete (EVD.43). I5 already catches
  search-backed negatives. These cases need a rule or an admission restriction;
  the probe preserves typed collisions. It makes no assertion about support from a
  different, already completed historical job. DISPLAY.json gives concrete tuples
  for every collision so the binding assumption is reviewable.
- **Open edition:** LQ3 has not chosen the threshold or invalid pairs. The synthetic
  run pins trial U0 and the chapter's initially empty invalid-pair table, not a product
  choice. A stricter threshold may change supported words. LQ15 remains open too.
- **Missing facts:** an exhaustive leaf without its search record, a cut-off search
  with no stop category, and a mixed-universal refutation without its required witness
  have experiment-local typed inconsistencies. They do not add chapter-06 error codes.
- **Resolved priority, not a collision:** a budget-limited search and an error/cancelled
  job can match rows 5 and 6; EVD.32 explicitly says first match wins. The probe uses row 5.
- **Unmapped semantics:** absent-format dispatch (PS3.12/EVD.53), source-to-statement
  bindings, transport relations, guard/premise classes and covered-set construction
  remain open. The source facts and typed reasons preserve these gaps.

## Validation and limits

The acceptance command rebuilds the Git manifest and all outputs, checks the frozen
baseline, per-label counts, per-record accounting, pointers, digests and absence of
invented support, exhausts the bounded display model, and compares every frozen artifact.
It writes no build directory, bytecode cache or binaries, even on failure. `--write`
refreshes only the listed experiment outputs. No timing result is claimed.

Not run here: full chapter-06 conformance on arbitrary graphs, general whole-record
statement interpretation, judgement-option mapping, historical mathematical checkers,
WSL, GPU/native device runs and decisive performance measurements. They are not required
by this source/synthetic packet, and no executable command for the unimplemented
semantic work is claimed. There is no required external hardware command for PS1.38.
The integrator must review this candidate and rerun exactly
`python {PREFIX}check.py` after applying it. This run does not accept chapters 05/06,
freeze a migration format, answer LQ3/LQ10, or select a route, engine language or syntax.
"""
    return text.encode("utf-8")


def run(write=False):
    entries = tree("HEAD")
    require(entries == tree(BASE), "Frozen scoped Git inputs changed")
    blobs = read_blobs(entries)
    man = manifest(entries, blobs)
    require(man["full_corpus_manifest_sha256"] == POST_RENAME, "Post-rename corpus identity changed")
    require(len({r["path"].casefold() for r in man["records"]}) == 497, "Case-colliding current names")
    require(sha(blobs["docs/development-guide/AGENT_BRIEFING.md"]) ==
            "d7412a7b77a79c57836909503e39ce6004ed04b90c256114b1d3f52b700a381c", "Briefing differs from packet")
    print("Git baseline verified: 497 records, 13,412,769 stored bytes", flush=True)
    records, rows, table = scan(man, blobs)
    candidates = candidate()
    envelopes, dependency, per_record = build(records, rows, man, blobs, candidates)
    negative_checks(envelopes, records, rows)
    summary = metrics(per_record, dependency, man, blobs, envelopes)
    print("Envelopes built; enumerating the bounded display model", flush=True)
    display = enumerate_model()
    versions = dict(python=platform.python_version(), git=git("--version").decode().strip(),
                    zlib_compile=zlib.ZLIB_VERSION, zlib_runtime=zlib.ZLIB_RUNTIME_VERSION)
    # Exact stored-byte comparison is deliberately pinned to the recorded compressor.
    versions_path = HERE / "VERSIONS.json"
    if not write:
        require(versions_path.exists() and json.loads(versions_path.read_bytes()) == versions,
                "Tool versions differ; compare canonical boundaries before regenerating compressed evidence")
    shared = sorted((d for d in dependency["reported_digests"] if d["records"] > 1),
                    key=lambda d: (-d["records"], d["digest"]))
    dependency_report = {k: v for k, v in dependency.items() if k != "reported_digests"}
    dependency_report.update(reported_digest_count=len(dependency["reported_digests"]),
                             full_catalogue_sha256=sha(canonical(dependency)),
                             shared_digest_columns=["sha256", "occurrences", "records", "byte_matches"],
                             all_shared_digests=[[d["digest"], d["occurrences"], d["records"], len(d["byte_matches"])]
                                                 for d in sorted(shared, key=lambda d: d["digest"])],
                             shared_digest_examples=shared[:40],
                             full_catalogue_regeneration="check.py builds it in memory; all occurrences remain in envelopes")
    outputs = {
        "MANIFEST.json": pretty(man), "CANDIDATE.json": pretty(candidates),
        "VERSIONS.json": pretty(versions), "RECORDS.json": pretty(per_record),
        "SUMMARY.json": pretty(summary), "SUMMARY.md": summary_table(summary),
        "DEPENDENCIES.json": pretty(dependency_report), "DISPLAY.json": pretty(display),
        "UNMAPPED.json": pretty(dict(rows=[dict(row=i, **r) for i, r in enumerate(rows) if r["code"] in ("rec", "none")],
                                      excluded_external_rows=24, no_rule=0,
                                      prose_occurrences=sum(len(r["prose"]) for r in records),
                                      payload_boolean_occurrences=sum(r["payload"].get("boolean", 0) for r in records),
                                      numeric_occurrences=sum(r["payload"].get("number", 0) for r in records))),
        "SAMPLES.json": pretty(select_samples(envelopes, rows)),
    }
    outputs["RESULT.md"] = result(summary, dependency, display, man, candidates, versions)
    for name, data in outputs.items():
        require(len(data) < 2_000_000, "Artifact exceeds packet limit: " + name)
        require(b"\r" not in data, "Non-LF output: " + name)
        if write:
            (HERE / name).write_bytes(data)
        else:
            require((HERE / name).is_file() and (HERE / name).read_bytes() == data, "Stale artifact: " + name)
    print(f"PASS: 497 envelopes; {display['combinations']:,} combinations; 504 node cases; all artifacts match", flush=True)


if __name__ == "__main__":
    try:
        require(sys.argv[1:] in ([], ["--write"]), "Usage: check.py [--write]")
        run(sys.argv[1:] == ["--write"])
    except (ValueError, OSError, json.JSONDecodeError) as error:
        # Keep diagnostics public and repository-relative; no machine traceback.
        print("FAIL: " + (str(error) if isinstance(error, ValueError) else type(error).__name__), file=sys.stderr)
        sys.exit(1)
