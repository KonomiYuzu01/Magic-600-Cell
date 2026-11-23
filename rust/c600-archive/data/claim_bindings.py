"""LP5 piece R1: writes `claim-bindings-1.json`, FR-B08's typed claim bindings, one row per claim group of the frozen
claim matrix (owner, 23 November 2026, LP5-4).

Run from the repository root: `python rust/c600-archive/data/claim_bindings.py` rewrites the edition next to this file;
`--check` compares instead and exits 1 on a difference. It reads, in place and read-only, the frozen claim matrix
(`docs/architecture/1.0/language/claim-matrix/matrix.json`), the files the matrix pins (each SHA-256 compared with its
pin), the jumbling model's identity (`assets/manifest.json` and c600-jumble's committed I_a binding), the two stage 18
`steps.json` records, the stage 13 stop table and q11's record. It runs no producer, no checker, no search and no Git,
and it writes nothing but the edition.

A row binds a claim group to a typed statement (its ID is the SHA-256 of the statement's canonical content), the scope
phrase, its stored inputs by digest (the matrix's `required_files`, plus the stage 18 chains' stop table, step members
and stored end; a composed or tail-replay group whose checker checks it records the intermediate start tables as
`derived`), and the KN6 sub-checkers that read it.
Which sub-checker reads which group, and how, is the table `ROUTES` below, written from each stage's sub-checker
documentation in `rust/c600-checkers/src/archive/` and the tests that run them: when a sub-checker is added or
changes its scope, change its rows there and rerun this script (the Rust test then asks for the new pin). The
matrix's own blockers are kept word for word; `resolved` names only what the binding resolves (the typed binding, and
the stage 18 start tables that piece R0 found rebuildable). Whether a named checker is registered is not data here:
the host answers it at preflight (`c600_archive::bindings`).

The script is deterministic and uses the Python standard library only; it reads no network and runs nothing.
"""
from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent / "claim-bindings-1.json"
MATRIX = "docs/architecture/1.0/language/claim-matrix/matrix.json"
MODEL_MANIFEST = "assets/manifest.json"
JUMBLE_BINDING = "rust/c600-jumble/data/corrections-I_a.json"
RESEARCH = "research/jumbling/restoration/"
S18 = RESEARCH + "18-tail-completion/results/"
STOP_TABLE = RESEARCH + "13-tutorial-runner/runs/dev/d1-acc-s7101/out4/tables/00113.json"
T6_END = S18 + "lattice-d1/run/start-table.json"
S19_MANIFEST = RESEARCH + "19-height-depth-framework/IMPORT-MANIFEST.json"

# LB5.14's reason atoms, in its order.
REASONS = ["claim_unmapped", "dependency_missing", "dependency_mismatch", "no_independent_checker", "unsupported_claim",
           "historical_check_incomplete"]

STATEMENT_FORMAT = "c600-claim-statement/1"

# What a row's `resolved` may name, with its basis.
RESOLUTIONS = {
    "typed_binding": "claim_unmapped: this row is the group's typed statement, scope and input binding, written from "
                     "the matrix's anchors (owner, 23 November 2026, LP5-4; FR-B08, CX.22).",
    "rebuilt_start_tables": "dependency_missing: the chain's start table is the stored stage 13 stop table, its step "
                            "words are stored and hash-checked, and every later step's start table is rebuilt by "
                            "exact replay of those words as a new derivation under the checking sub-checker's "
                            "identity, marked derived (owner, 23 November 2026, LP5-3; CX.17; LP5 R0 dependency "
                            "report, section 3). A matching profile is not a matching start, and nothing here fills "
                            "a historical gate.",
}

# Which KN6 sub-checker reads which group: (family, regex over the source path below `research/jumbling/restoration/`,
# pointers or None for any, sub-checker IDs, how the group's record enters it). The first matching row wins, and
# every row must match a group; a group no row matches names no checker, and its preflight keeps
# `no_independent_checker`. `checked_as` is `certificate` (the record is the checker's certificate), `input` (the
# record is an input of the check whose certificate is `via`), `table` (stage 19: the certificate states the table's
# `IMPORT-MANIFEST.json` entry) or `scope_only`: the sub-checker reads the record but checks no claim of the group (it
# answers KN6's `not_applicable` with the record's own bounds, since no KN6 checker of a search's coverage exists, or,
# for stage 16's reduced transport at 7307, checks only the recorded agreement with the unreduced case), so the group
# keeps `no_independent_checker`. Records that every sub-checker of a stage answers `not_applicable` (stage 14's
# finite searches, stage 18's static tails, stage 19's records outside its families) name no sub-checker.
STEP_MEMBER = r"18-tail-completion/results/(d1-tail|rule-stop)/[0-9]{2}-[0-9]+-[0-9]+/(unit|v|w|winv|x)\.json(\.gz)?$"
POSE_TABLES = (r"19-height-depth-framework/records/(irregularity-descent-source-end-table|profile-family-validation-"
               r"(input|end)-table|type-lex-independent-282-13-end-table|type-lex-recovery-transfer-joined-table|"
               r"type-lex-trimmed-macro-end-table)\.json\.gz$")
ROUTES = [
    ("14/t3p", r"14-blocking-structure/results/s73\d\d-t3p-(chain|check-w[35]|scan\.certificate)\.json$|"
               r"14-blocking-structure/results/s7304-w[35]-certificate\.json$", None, ["s14.t3p"], "certificate"),
    ("14/literal", r"14-blocking-structure/results/s73\d\d-.*replay\.json$", None, ["s14.literal"], "certificate"),
    ("14/relations", r"14-blocking-structure/results/locality-summary\.json$", None, ["s14.locality"],
     "certificate"),
    ("14/relations", r"14-blocking-structure/results/s73\d\d-(chain-end-)?levels(-w[35])?\.json$", None,
     ["s14.levels"], "certificate"),
    ("14/relations", r"14-blocking-structure/results/s7304-(coheld|end-caps|jam|moves-522)\.json$", None,
     ["s14.blocks"], "certificate"),
    # Piece R4's stage 15: every `macro-certificate/1` and `macro-check/1` record (R01 to R17, the whole descents, the
    # P* and DIS-1 certificates); the clean-pair censuses (`pstar-census/1`, census-7304-prep among them, which keeps
    # the matrix's missing start table and prefix binding: this edition binds neither); the P* stop and run records,
    # scope only.
    ("15/macro", r"15-lowering-machine/results/(certificates/R(0[1-9]|1[0-7])(-check)?|runs/s73\d\d-descent(-check)?|"
                 r"runs/s73\d\d-pstar\.certificate|runs/dis1-r0[4-6]\.certificate)\.json$", None, ["s15.macro"],
     "certificate"),
    ("15/relations", r"15-lowering-machine/results/census-73\d\d(-prep)?\.json$", None, ["s15.pstar"], "certificate"),
    ("15/selectors", r"15-lowering-machine/results/runs/[A-Za-z0-9-]+\.json$", None, ["s15.pstar"], "scope_only"),
    # Piece R4's stage 16: the jam descents and P** whole descents with their check records; the literal traces
    # (SJ-E1 among them, LB5.11); the unit mobility records; the transport, routing, T1, two-carrier and P** run
    # records, scope only.
    ("16/macro", r"16-six-jams/results/(certificates/(I-73\d\d|O-D\dc?|O-SJ-D\d|SJ-D\d)(-check)?|"
                 r"runs/PX-73\d\d(-D\d|-OD\d)?-(cert|check))\.json$", None, ["s16.macro"], "certificate"),
    ("16/literal", r"16-six-jams/results/certificates/(I-73\d\d|SJ-E1|T2-cx)-trace\.json$", None, ["s16.release"],
     "certificate"),
    ("16/relations", r"16-six-jams/results/mobility/73\d\d\.json$", None, ["s16.mobility"], "certificate"),
    ("16/reductions", r"16-six-jams/results/(dis|routing|t1|transport)/[A-Za-z0-9-]+\.json$", None,
     ["s16.reductions"], "scope_only"),
    ("16/selectors", r"16-six-jams/results/runs/px-[A-Za-z0-9-]+\.json$", None, ["s16.reductions"], "scope_only"),
    # Piece R4's stage 17: the chosen descents of P*** and RR-7304 with their check records; the RR release searches,
    # P*** runs and transport stops, scope only, and the family-R stop at 7304 under P*'s unbeamed scope.
    ("17/macro", r"17-routing-in-driver/results/certificates/(PX3-73\d\d(p|-D\d|-OD\d)?|RR-7304)(-check)?\.json$",
     None, ["s17.macro"], "certificate"),
    ("17/rr", r"17-routing-in-driver/results/stop-7304-438/r\.json$", None, ["s15.pstar"], "scope_only"),
    ("17/rr", r"17-routing-in-driver/results/(rr-7304-23-41|runs/pxrr-[A-Za-z0-9-]+|stop-7304-438/[a-z0-9/-]+)\.json$",
     None, ["s17.rr"], "scope_only"),
    ("18/expansion", r"18-tail-completion/results/(d1-tail|rule-stop)/steps\.json$", None, ["s18.expansion"],
     "certificate"),
    ("18/expansion", STEP_MEMBER, None, ["s18.expansion"], "input"),
    ("18/composed", r"18-tail-completion/results/(d1-tail|rule-stop)/steps\.json$", None, ["s18.composed"],
     "certificate"),
    # Gate 3: s18.tail_replay replays T6's steps 1 to 3 and answers `not_applicable` for a record whose first step is
    # one it never replays, as the rule's first stage is; that group keeps the matrix's blockers whole.
    ("18/tail_replay", r"18-tail-completion/results/rule-stop/steps\.json$", None, ["s18.tail_replay"], "scope_only"),
    ("18/tail_replay", r"18-tail-completion/results/d1-tail/steps\.json$", None, ["s18.tail_replay"], "certificate"),
    ("18/lattice_run", r"18-tail-completion/results/lattice-d1/run/run\.json$", None,
     ["s18.lattice_run", "s18.lattice_replay"], "certificate"),
    ("18/lattice_run", r"18-tail-completion/results/lattice-d1/run/.*", None, ["s18.lattice_run"], "input"),
    ("19/matching", POSE_TABLES, None, ["s19.pose-table"], "table"),
    ("19/matching", r"19-height-depth-framework/records/profile-family-validation-matching-certificate\.json$", None,
     ["s19.gamma-matching"], "certificate"),
    ("19/matching", r"19-height-depth-framework/records/profile-family-validation-results\.json\.gz$", None,
     ["s19.gamma-witness"], "certificate"),
    ("19/frontier", POSE_TABLES, None, ["s19.pose-table"], "table"),
    ("19/frontier", r"19-height-depth-framework/records/type-lex-trimmed-macro\.json$", None, ["s19.frontier"],
     "certificate"),
    ("19/frontier", r"19-height-depth-framework/records/type-lex-trimmed-independent-results\.json$", [""],
     ["s19.frontier"], "certificate"),
    ("19/ranks", r"19-height-depth-framework/records/type-height-weight-results\.json$", None, ["s19.ranks"],
     "certificate"),
    ("19/ranks", r"19-height-depth-framework/records/type-lex-trimmed-independent-results\.json$",
     ["/one_fixed_L3", "/one_fixed_S", "/generic_mixed_rank_proof_check"], ["s19.ranks"], "certificate"),
]

# The certificate a `checked_as: input` record enters (stage 18): its chain's `steps.json`, or the lattice run.
def via_for(path: str, checked_as: str) -> str | None:
    if checked_as == "input":
        m = re.match(r"(.*18-tail-completion/results/(?:d1-tail|rule-stop)/)[^/]+/[^/]+$", path)
        if m:
            return m.group(1) + "steps.json"
        if "/lattice-d1/run/" in path:
            return S18 + "lattice-d1/run/run.json"
        raise SystemExit(f"no certificate for the input record {path}")
    if checked_as == "table":
        return S19_MANIFEST
    return None


# LB5.17's captured string, kept word for word with the group that preflights it: (family, source path) -> pointer.
LABELS = {("19/q11", RESEARCH + "19-height-depth-framework/records/type-lex-trimmed-independent-results.json"): "/status"}

# Stage 18's chains (piece R0's dependency report, section 3): the record, and its stored end table if any.
CHAINS = {S18 + "d1-tail/steps.json": T6_END, S18 + "rule-stop/steps.json": None}


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canon(value) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True)


def read(path: str) -> bytes:
    return (ROOT / path).read_bytes()


def pointer_get(doc, pointer: str):
    for part in pointer.split("/")[1:]:
        part = part.replace("~1", "/").replace("~0", "~")
        doc = doc[int(part)] if isinstance(doc, list) else doc[part]
    return doc


def role_of(path: str, source: str) -> str:
    if path == source:
        return "claim_record"
    if path.startswith("assets/"):
        return "model"
    if path.startswith("research/jumbling/") and not path.startswith(RESEARCH):
        return "convention"
    if "/tables/" in path and path.startswith(RESEARCH + "13-tutorial-runner/"):
        return "start_table"
    if path.endswith("16-six-jams/jams.json") or path.endswith("14-blocking-structure/results/sources.json"):
        return "catalogue"
    if re.search(STEP_MEMBER, path):
        return "step_member"
    return "record"


def build() -> bytes:
    matrix_bytes = read(MATRIX)
    m = json.loads(matrix_bytes)
    files = m["files"]
    for f in files:
        if sha(read(f["path"])) != f["sha256"]:
            raise SystemExit(f"{f['path']} does not have its pinned SHA-256")

    model_id = json.loads(read(MODEL_MANIFEST))["model_id"]
    binding = json.loads(read(JUMBLE_BINDING))["binding"]
    if binding["model_id"] != model_id:
        raise SystemExit("the jumbling binding names another model")
    identity = {"puzzle": "jumbling", "model_id": model_id, "menu_id": binding["menu_identity"],
                "menu_order": binding["menu_order"], "contract_rev": binding["contract_revision"]}

    catalogue: dict[str, dict] = {}

    def pin(path: str) -> str:
        if path not in catalogue:
            data = read(path)
            catalogue[path] = {"bytes": len(data), "path": path, "sha256": sha(data)}
        return path

    families = []
    for fam in m["families"]:
        families.append({"asserts": fam["asserts"], "id": fam["id"], "limits": fam["limits"],
                         "planned_checker": fam["checker"]["name"], "stage": fam["stage"]})

    rows = []
    seen_ids = set()
    used_routes = set()
    for index, g in enumerate(m["groups"]):
        source = files[g["source"]]["path"]
        rel = source[len(RESEARCH):] if source.startswith(RESEARCH) else source
        pointers = g["anchors"] if g["anchors"] else g["leaves"]
        family = g["family"]
        stage = int(family.split("/")[0])
        group_id = f"{family}:{source}#{','.join(pointers)}"
        if group_id in seen_ids:
            raise SystemExit(f"two groups have the ID {group_id}")
        seen_ids.add(group_id)

        record_only = g["data_status"] == "record_only"
        form = None if record_only else "finite"
        content = {"covers": "one_instance", "family": family, "form": form, "pointers": pointers,
                   "source": {"path": source, "sha256": files[g["source"]]["sha256"]}, "statement": STATEMENT_FORMAT}
        statement = {"covers": "one_instance", "form": form, "id": sha(canon(content).encode("ascii")),
                     "input": files[g["source"]]["sha256"]}

        checkers, checked_as = [], None
        if not record_only:
            for n, (fam, pattern, ptrs, ids, how) in enumerate(ROUTES):
                if fam == family and re.fullmatch(pattern, rel) and (ptrs is None or ptrs == pointers):
                    checkers, checked_as = list(ids), how
                    used_routes.add(n)
                    break
        via = via_for(source, checked_as) if checked_as else None

        inputs = [[pin(files[i]["path"]), role_of(files[i]["path"], source)] for i in g["required_files"]]
        derived = []
        resolved = {}
        if "claim_unmapped" in g["outcome"]["reasons"]:
            resolved["claim_unmapped"] = "typed_binding"
        # Stage 18's chains: every gate's sub-checker reads the stored stop table as the chain's start, the step members
        # and the stored end table where there is one (expansion through its `steps.json`, whose inputs its members
        # share). Only a sub-checker that checks the claim rebuilds the intermediate start tables (composed and tail
        # replay), so only its row records them as derived and resolves `dependency_missing` by them.
        chain_path = source if source in CHAINS else via if via in CHAINS else None
        if family in ("18/expansion", "18/composed", "18/tail_replay") and checked_as and chain_path:
            chain = json.loads(read(chain_path))
            stop = json.loads(read(STOP_TABLE))
            if chain["start_digest"] != chain["steps"][0]["start_digest"] or stop.get("digest") != chain["start_digest"]:
                raise SystemExit(f"{chain_path} does not start at the stored stop table")
            inputs.append([pin(STOP_TABLE), "start_table"])
            base = chain_path.rsplit("/", 1)[0]
            for step in chain["steps"]:
                for name in ("x.json.gz", "w.json", "v.json.gz", "winv.json", "unit.json"):
                    member = f"{base}/{step['dir']}/{name}"
                    if (ROOT / member).exists():
                        inputs.append([pin(member), "step_member"])
            if CHAINS[chain_path]:
                inputs.append([pin(CHAINS[chain_path]), "end_table"])
            if family != "18/expansion" and checked_as != "scope_only":
                for k, step in enumerate(chain["steps"][1:], start=1):
                    if step["start_digest"] != chain["steps"][k - 1]["end_digest"]:
                        raise SystemExit(f"{chain_path}: step {k} does not start at the end of step {k - 1}")
                    derived.append({"digest": step["start_digest"], "pointer": f"/steps/{k}", "role": "start_table",
                                    "rule": "rebuilt_start_tables"})
                if "dependency_missing" in g["outcome"]["reasons"]:
                    resolved["dependency_missing"] = "rebuilt_start_tables"
        if via:
            pin(via)
        # One role per file: the first named wins (the matrix's required files, the claim record among them, come
        # first), then path order.
        roles: dict[str, str] = {}
        for path, role in inputs:
            roles.setdefault(path, role)
        inputs = [[path, roles[path]] for path in sorted(roles)]

        missing = []
        for mi in g["missing_inputs"]:
            if mi["present"] or mi["digest"] is not None or files[mi["source"]]["path"] != source:
                raise SystemExit(f"{group_id}: an unexpected missing input")
            cites = sorted(d["at"] for d in m["dependencies"]
                           if d["source"] == g["source"] and d["resolution"] == "dependency_missing"
                           and d["name"] in mi["name"])
            missing.append({"cites": cites, "name": mi["name"]})

        label = None
        key = (family, source)
        if key in LABELS:
            doc = json.loads(read(source))
            label = {"pointer": LABELS[key], "text": pointer_get(doc, LABELS[key])}

        rows.append({
            "anchors": g["anchors"],
            "checked_as": checked_as,
            "checkers": checkers,
            "data_status": g["data_status"],
            "derived": derived,
            "family": family,
            "group": group_id,
            "historically_incomplete": g["historically_incomplete"],
            "index": index,
            "inputs": inputs,
            "label": label,
            "matrix_reasons": g["outcome"]["reasons"],
            "missing": missing,
            "pointers": pointers,
            "resolved": resolved,
            "scope_phrase": "on one instance",
            "source": pin(source),
            "stage": stage,
            "statement": statement,
            "via": via,
        })

    unused = [ROUTES[n][:2] for n in range(len(ROUTES)) if n not in used_routes]
    if unused:
        raise SystemExit(f"routes that match no group: {unused}")

    ordered = sorted(catalogue)
    position = {p: i for i, p in enumerate(ordered)}
    for row in rows:
        row["source"] = position[row["source"]]
        row["via"] = position[row["via"]] if row["via"] else None
        row["inputs"] = [[position[p], role] for p, role in row["inputs"]]

    edition = {
        "edition": "c600-claim-bindings-1",
        "format": "c600-claim-bindings",
        "identity": identity,
        "matrix": {"format": m["format"], "groups": len(m["groups"]), "path": MATRIX, "sha256": sha(matrix_bytes)},
        "note": "FR-B08's typed claim bindings, one row per claim group of the frozen claim matrix, written by LP5 "
                "piece R1 on 23 November 2026 (owner, LP5-4). A row's statement is the claim its record states at "
                "its pointers, on that one record; its scope names the record's bytes. The matrix's blockers stay "
                "word for word in matrix_reasons and missing; resolved names only what this binding resolves. A "
                "checker named with checked_as scope_only reads the record but checks no claim of the group, which "
                "keeps no_independent_checker. Preflight adds no evidence leaf and runs no check (LB5.14, EVD.46).",
        "reasons": REASONS,
        "resolutions": RESOLUTIONS,
        "statement_format": STATEMENT_FORMAT,
        "version": 1,
        "families": families,
        "files": [catalogue[p] for p in ordered],
        "rows": rows,
    }
    parts = []
    for key in sorted(edition):
        value = edition[key]
        if key in ("families", "files", "rows"):
            body = ",\n".join(canon(item) for item in value)
            parts.append(f"{canon(key)}:[\n{body}\n]")
        else:
            parts.append(f"{canon(key)}:{canon(value)}")
    return ("{\n" + ",\n".join(parts) + "\n}\n").encode("ascii")


def main() -> int:
    data = build()
    if "--check" in sys.argv[1:]:
        if OUT.read_bytes() != data:
            print("claim-bindings-1.json differs from its build", file=sys.stderr)
            return 1
        print(f"claim-bindings-1.json matches its build ({sha(data)})")
        return 0
    OUT.write_bytes(data)
    print(f"wrote {OUT.name}: {len(data)} bytes, SHA-256 {sha(data)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
