"""Build partial facts envelopes; never guess a statement or an argument edge."""

from collections import defaultdict

from corpus import ARCH, IMPORT, BASE, canonical, pack, sha, require, pointer_value


MISSING = {
    "identity": "EF:missing.typed_identity",
    "statement_id": "EF:missing.statement_identity",
    "statement_form": "EF:missing.statement_form",
    "negation_pair": "EF:missing.negation_pair",
    "subject_level": "EF:missing.subject_level",
    "scope": "EF:missing.typed_scope",
    "scope_membership": "EF:missing.membership_binding",
    "search_record": "EF:missing.search_binding",
    "premise_classes": "EF:missing.premise_binding",
    "guards": "EF:missing.guard_binding",
    "covered": "EF:missing.leaf_coverage_binding",
    "check_dependencies": "EF:missing.check_binding_and_common_base",
    "reachability_reference": "EF:missing.reachability_binding",
}

FIELD_ROLES = {
    "identity": {"model_id", "model_sha256", "puzzle", "menu", "contract", "contract_revision"},
    "statement": {"statement", "claims", "claim", "theorem", "proof", "conclusion"},
    "inputs": {"input", "inputs", "input_sha256", "inputs_sha256", "table_digest",
               "start", "seed", "prefix", "prefix_word", "options", "config"},
    "search": {"search", "stop", "stop_reason", "exhausted", "family_exhausted", "complete",
               "bounds", "limits", "families", "alphabet"},
    "premises_and_guards": {"premises", "hypotheses", "guards", "base_family_premises"},
    "attribution": {"attribution", "author", "reproducer", "scope_setter", "review_class"},
    "engineering_evidence": {"evidence_kind", "evidence_class"},
    "checks": {"passed", "checks", "replay", "verification_status", "source_files_sha256",
               "source_sha256", "candidate_sha256", "certificate", "best_certificate"},
}


def absent(reason):
    return dict(value=None, reason=reason)


def source_fields(value):
    found = defaultdict(list)

    def visit(item, at):
        if isinstance(item, dict):
            for key, child in item.items():
                pointer = at + "/" + key.replace("~", "~0").replace("/", "~1")
                for role, names in FIELD_ROLES.items():
                    if key in names:
                        found[role].append(pointer)
                visit(child, pointer)
        elif isinstance(item, list):
            for i, child in enumerate(item):
                visit(child, at + "/" + str(i))
    visit(value, "")
    return dict(sorted(found.items()))


def combine(observations, rows, field):
    # A same-object grouping is a storage bundle, NOT proof of a shared claim.
    choices = defaultdict(list)
    for obs in observations:
        row = rows[obs["row"]]
        if field in row["mapping"]:
            choices[row["mapping"][field]].append(obs["at"])
    return [dict(value=value, at=locations) for value, locations in sorted(choices.items())]


def build_one(record, rows, edition):
    source = record["source"]
    value = record["value"]
    top_format = value.get("format") if isinstance(value, dict) else None
    groups = defaultdict(list)
    for obs in record["observations"]:
        row = rows[obs["row"]]
        # H1 must remain a separate, open reachability claim (EVD.52).
        role = "reachability" if row["rule"] == "H1" else "source_object"
        groups[(obs["at"].rsplit("/", 1)[0], role)].append(obs)
    facts, leaves = [], []
    for (at, role), observations in sorted(groups.items()):
        fields = {field: combine(observations, rows, field)
                  for field in ("scope", "basis", "check", "check_mode", "job_outcome")}
        fact = dict(at=at, role=role, labels=observations,
                    table_candidates={k: v for k, v in fields.items() if v})
        facts.append(fact)
        if fields["basis"]:
            # Slots hold source evidence, but cannot enter an EVD argument without a key.
            leaves.append(dict(fact_group=len(facts) - 1,
                               key=absent(MISSING["statement_id"]),
                               covered=absent(MISSING["covered"]),
                               admission="recorded_only"))
    field_refs = source_fields(value)
    missing = dict(MISSING)
    if not top_format:
        missing["format_dispatch"] = "EF:missing.absent_format_dispatch"
    if not record["observations"]:
        missing["mapped_labels"] = "EF:missing.no_label_rows"
    if not field_refs.get("attribution"):
        missing["attribution"] = "EF:missing.attribution"
    if not field_refs.get("engineering_evidence"):
        missing["engineering_evidence"] = "EF:missing.engineering_evidence_class"
    envelope = dict(
        format="ps1.38-partial-facts-experiment/1", payload="facts", edition=edition,
        source=source, stage=source["path"].split("/")[3][:2], source_commit=BASE,
        source_format=top_format if isinstance(top_format, str) else None,
        fields={k: absent(v) for k, v in sorted(missing.items())},
        source_field_refs=field_refs, facts=facts,
        scope_text=record["prose"], reported_digests=record["digests"],
        graph=dict(leaves=leaves, AND=[], OR=[], edges=[]),
    )
    # The facts variant has no stored derived word or truth/standing cache.
    require("claim" not in envelope and "word" not in envelope, "Stored judgement")
    require(sum(len(g["labels"]) for g in facts) == len(record["observations"]),
            "Lost label observations")
    for refs in field_refs.values():
        for pointer in refs:
            pointer_value(value, pointer)
    return envelope


def recorded_word(envelope):
    """EVD.36/51/53: an untyped scope/key remains recorded, hence row 7."""
    require(envelope["format"] == "ps1.38-partial-facts-experiment/1", "Unknown envelope")
    require(all(item["value"] is None and item["reason"].startswith("EF:missing.")
                for item in envelope["fields"].values()), "Unexpected typed envelope field")
    require(not any(envelope["graph"][kind] for kind in ("AND", "OR", "edges")),
            "Unjustified argument edge")
    require(all(leaf["admission"] == "recorded_only" for leaf in envelope["graph"]["leaves"]),
            "Unjustified support")
    return dict(word="open", source_labels="retained in facts", row=7)


def dependency_inventory(records, blobs, man, edition_data):
    # Equality of bytes is separately reported from semantic dependency identity.
    byte_index = defaultdict(list)
    for path, raw in blobs.items():
        byte_index[sha(raw)].append(dict(path=path, boundary="stored"))
    for record in records:
        src = record["source"]
        if src["decoded_sha256"] != src["stored_sha256"]:
            byte_index[src["decoded_sha256"]].append(dict(path=src["path"], boundary="decoded"))
    uses = defaultdict(list)
    for record in records:
        for item in record["digests"]:
            uses[item["digest"]].append((record["source"]["path"], item["at"]))
    dependencies = []
    for digest, locations in sorted(uses.items()):
        paths = sorted({p for p, _ in locations})
        dependencies.append(dict(digest=digest, occurrences=len(locations), records=len(paths),
                                 examples=[dict(path=p, at=a) for p, a in locations[:3]],
                                 byte_matches=byte_index.get(digest, []),
                                 semantic_role="untyped; no EVD edge or independence inferred"))
    return dict(format="ps1.38-dependency-inventory/1", edition=edition_data,
                common_inputs=man["inputs"], reported_digests=dependencies,
                common_input_ref_count=len(records),
                shared_reported_digests=sum(d["records"] > 1 for d in dependencies),
                unmatched_reported_digests=sum(not d["byte_matches"] for d in dependencies),
                matched_reported_digests=sum(bool(d["byte_matches"]) for d in dependencies),
                missing_is_not_dangling="Unresolved source assertions are opaque facts, never graph dependencies")


def build(records, rows, man, blobs, candidate):
    edition = dict(format="ps1.38-trial-edition/1", decision="open", decider="owner",
                   grade_order=dict(source=ARCH + "06-evidence.md", clause="EVD.19"),
                   invalid_pairs=dict(source=ARCH + "06-evidence.md", clause="EVD.21", value=[]),
                   threshold=dict(trial="U0 only; not an owner selection", clause="EVD.29"),
                   import_table_sha256=sha(blobs[IMPORT + "corpus-labels.json"]),
                   evidence_chapter_sha256=sha(blobs[ARCH + "06-evidence.md"]),
                   functions=candidate)
    edition_digest = sha(canonical(edition))
    envelopes = [build_one(r, rows, edition_digest) for r in records]
    dependency = dependency_inventory(records, blobs, man, edition)
    per_record = []
    for record, envelope in zip(records, envelopes):
        raw = canonical(envelope)
        stored = pack(raw)
        source = record["source"]
        original_canonical = canonical(record["value"])
        require(sha(original_canonical) == source["canonical_sha256"], "Source changed")
        per_record.append(dict(path=source["path"],
                               original_stored_bytes=source["stored_bytes"],
                               original_decoded_bytes=source["decoded_bytes"],
                               original_canonical_bytes=len(original_canonical),
                               original_canonical_gzip_bytes=len(pack(original_canonical)),
                               envelope_canonical_bytes=len(raw), envelope_gzip_bytes=len(stored),
                               envelope_sha256=sha(raw), envelope_gzip_sha256=sha(stored),
                               labels=len(record["observations"]),
                               mapped_labels=sum(rows[o["row"]]["code"] not in ("rec", "none")
                                                 for o in record["observations"]),
                               fact_groups=len(envelope["facts"]),
                               leaf_slots=len(envelope["graph"]["leaves"]), AND=0, OR=0, edges=0,
                               admitted_leaves=0, complete=False,
                               missing={k: v["reason"] for k, v in envelope["fields"].items()},
                               display=recorded_word(envelope)["word"]))
    return envelopes, dependency, per_record
