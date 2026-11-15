"""Draft FR-B08 planning rules, not evidence admission or executable checkers.

Paths classify work to be done; they do not supply an absent research format.
Every family retains the source record's exact scope and historical check mode.
"""

RESEARCH = "research/jumbling/restoration/"
STAGES = {
    14: "14-blocking-structure", 15: "15-lowering-machine",
    16: "16-six-jams", 17: "17-routing-in-driver",
    18: "18-tail-completion", 19: "19-height-depth-framework",
}


def path(stage, suffix):
    return RESEARCH + STAGES[stage] + "/" + suffix


COMMON = ["assets/manifest.json", "assets/model.npz", "assets/primitives.npz",
          "research/jumbling/exact.py", "research/jumbling/witness.py", "research/jumbling/theory/trees.py",
          "research/jumbling/sim/__init__.py", "research/jumbling/sim/kernel.py", "research/jumbling/sim/state.py",
          "research/jumbling/sim/model.py", "research/jumbling/sim/regions.py",
          "research/jumbling/sim/kplus.py", "research/jumbling/sim/twists.py",
          "research/jumbling/fixtures/wj.py"]

# These are witnesses to producer coupling, NOT a proposed checker implementation.
COUPLING = [
    (path(15, "scripts/macro.py"), "import t3p as P", path(14, "scripts/t3p.py")),
    (path(14, "scripts/t3p.py"), "import blockmap as BM", path(14, "scripts/blockmap.py")),
    (path(14, "scripts/replay_word.py"), "import blockmap as BM", path(14, "scripts/blockmap.py")),
    (path(14, "scripts/blockmap.py"), "import restore as R", "research/jumbling/theory/restore.py"),
    (path(16, "scripts/common.py"), "import pstar", path(15, "scripts/pstar.py")),
    (path(18, "scripts/chunk_verify.py"), "import chunk_prep as C", path(18, "scripts/chunk_prep.py")),
    (RESEARCH + "13-tutorial-runner/scripts/check_run.py", "import restore", "research/jumbling/theory/restore.py"),
]


def family(assertion, checker, producer, policy, need=None, inputs=(), limits=""):
    return dict(asserts=assertion, planned_checker=checker, producer_paths=list(producer),
                input_policy=policy, missing_input=need, extra_inputs=list(inputs), limits=limits)


FAMILIES = {
    "input_catalogue": family(
        "Retain source tables, units and publication metadata as inputs, without asserting a mathematical result.",
        "archive/source binding", (), "record", limits="No evidence leaf or mathematical checking promise is removed."),
    "t3p": family(
        "Check the supplied T3P rounds, P0-P4, chronology, moved sets and full collateral effects.",
        "finite T3P predicates and literal replay", (path(14, "scripts/t3p.py"),), "table",
        limits="A round certificate does not certify completeness of the search which found it."),
    "relations": family(
        "Recompute the declared finite locality, holder, co-held and greatest-fixed-point relations.",
        "exact relation and finite fixed-point predicates",
        tuple(path(14, "scripts/" + n + ".py") for n in ("blockmap", "coheld", "levels", "locality")),
        "table", limits="Optional label caches are recomputed; a finite histogram is not a global theorem."),
    "literal": family(
        "Replay each supplied literal letter from the actual start and recompute all claimed prefix observables.",
        "literal replay with exact full-state observables", (path(14, "scripts/replay_word.py"),), "table",
        limits="No search-completeness claim; SJ-E1 is a release, not a descending terminal-jumble macro."),
    "finite_search": family(
        "Check the declared finite search, plateau or excursion closure and its exact family restrictions.",
        "finite coverage/closure certificate checker",
        (path(14, "scripts/t3p.py"), path(15, "scripts/pstar2.py")), "needs",
        "complete coverage certificate: alphabet, bounds, every admitted edge, pruning lemmas and stop coverage",
        limits="A beam, budget or missing history gives no completed negative; a positive word has a separate replay obligation."),
    "macro": family(
        "Check Theorem MC claims, each support, F/D/delta, excursion, prefix and end digest of the supplied macros.",
        "MC certificate predicates and literal replay", (path(15, "scripts/macro.py"),), "table",
        limits="Only descending macros ending in a jumble fit macro-certificate/1; mutate the start table and each claim."),
    "selectors": family(
        "Check only recorded P*, P** or P*** choices and finite coverage under their declared options.",
        "selector trace and finite coverage predicates",
        (path(15, "scripts/pstar.py"), path(15, "scripts/pstar2.py"), path(16, "scripts/pxrun.py")), "needs",
        "complete per-family coverage and reduction certificate for this recorded run",
        limits="Chosen macro replay cannot certify a beam's completeness, termination or global availability."),
    "reductions": family(
        "Check the named jam, transport, T1, dynamic-target or alphabet-restricted gate in its finite scope.",
        "finite reduction hypotheses and exhaustive-family checker",
        tuple(path(16, "scripts/" + n + ".py") for n in ("transport", "routing", "release", "t1")), "needs",
        "complete finite coverage certificate and matching reduction premises for this gate",
        limits="Local commuting reduction is not a trace normal form; missing helper producers do not prove a negative."),
    "rr": family(
        "Check RR carrier/helper/tail canonicalization and each recorded shard's stated coverage.",
        "RR tail/reduction and disjoint-shard coverage checker",
        (path(17, "scripts/rr.py"), path(17, "scripts/pxrr.py"), path(17, "scripts/test_rr.py")), "needs",
        "complete matching RR shard set, tail bounds and reduction certificate",
        limits="Keep brute-force gate agreement, run negatives and chosen-word replay separate."),
    "expansion": family(
        "Check exact length and expanded hash of the stored T6 or rule word parts (gate 1 only).",
        "streamed word expansion/hash checker", (path(18, "scripts/replay_tail.py"),), "parts",
        limits="No geometry check or literal replay follows from expansion; both recorded forms retain their own end."),
    "composed": family(
        "Check the composed strict-prime contract at the actual chained starts (gate 2 only).",
        "composed-contract predicate checker", (path(18, "scripts/replay_tail.py"),), "needs",
        "actual preceding end tables and complete composed-contract inputs at every join",
        limits="A matching profile is not a matching start; dropping the inverse changes the end."),
    "tail_replay": family(
        "Check complete letter replay, span coverage and boundary-digest chaining of each tail step (gate 3).",
        "literal replay and chunk-chain checker",
        (RESEARCH + "12-low-height/scripts/round22/stepchk.py",
         path(18, "scripts/chunk_verify.py")), "needs",
        "complete actual-start tables and literal/chunk coverage for this step",
        limits="T6 steps 1-3 have recorded replay; T6 step 4 and all three rule stages keep historical_check_incomplete."),
    "static_tail": family(
        "Check finite whole-cap exclusions, WC-PLAN flows, group factors, Schreier/Jordan witnesses and entry predicates.",
        "finite flow/group/plan certificate checker",
        (path(18, "scripts/wholecap.py"), path(18, "scripts/wc_prep.py"), path(18, "scripts/wc_t1.py")), "needs",
        "all action/group witnesses, hole demands and exact endpoint joins for the supplied static plan",
        limits="Static existence is neither a constructed word nor gate-3 replay; entry premises remain explicit."),
    "lattice_run": family(
        "Check the supplied lattice run's flat words, chronological full states and recorded contracts.",
        "lattice literal replay and run-contract checker",
        (RESEARCH + "13-tutorial-runner/scripts/check_run.py",), "lattice",
        limits="Retain r22b10/1, r22b10/2, runner-run/3 and literal-lattice scopes; 13,231,266 stored letters are not rerun here."),
    "matching": family(
        "Check the supplied full regional bijection, exact gauges and nine-letter forward/inverse witness.",
        "full-region matching certificate and literal replay", (), "fixed",
        inputs=tuple(path(19, "records/" + n) for n in (
            "irregularity-descent-source-end-table.json.gz", "profile-family-validation-input-table.json.gz",
            "profile-family-validation-end-table.json.gz", "profile-family-validation-matching-certificate.json",
            "profile-family-validation-results.json.gz")),
        limits="Check all physical rows and collateral; synthetic reachability stays unknown and sticker-goal transport is not asserted."),
    "frontier": family(
        "Check supplied compact/cleanup words, full backgrounds, prefix heights and finite frontier progress.",
        "full-table literal replay and typed q/height predicates", (), "fixed",
        inputs=tuple(path(19, "records/" + n) for n in (
            "irregularity-descent-source-end-table.json.gz", "type-lex-trimmed-macro.json",
            "type-lex-trimmed-macro-end-table.json.gz", "type-lex-independent-282-13-end-table.json.gz",
            "type-lex-recovery-transfer-joined-table.json.gz", "type-lex-recovery-transfer-full-replay.json")),
        limits="Rebuild intermediate tables from the actual words; the q16 frontier is not a restored sticker state."),
    "q11": family(
        "Check exact distance 3 to q11=0 for the twelve stated post-A inputs, including the lower bound.",
        "complete holder/cut and bounded distance certificate checker", (), "needs",
        "type-lex-successor-base-cache.npz and complete successor audit, or a new independently rebuilt holder/cut certificate",
        inputs=(path(19, "records/type-lex-trimmed-independent-results.json"),),
        limits="The upper witness alone is insufficient; the historical cache audit cannot be replayed from the archive."),
    "ranks": family(
        "Recompute finite R3, S and R_L weights and inequalities for explicit type counts and stored block vectors.",
        "exact integer rank-instance checker", (), "fixed",
        inputs=tuple(path(19, "records/" + n) for n in (
            "type-height-weight-results.json", "type-lex-trimmed-independent-results.json",
            "type-lex-recovery-transfer-full-replay.json")),
        limits="New derivation from supplied vectors, not reproduction of absent type-height-weight-inputs.json; global selector/closure premises persist and D4 uses S."),
    "indexed_reviews": family(
        "Retain each indexed reference and its stated reused/audit/replay coverage without manufacturing independent replays.",
        "reference/transport closure checker", (), "needs",
        "each cited reference endpoint, exact transport and independently checkable source certificate",
        limits="73 indices are not 73 distinct fibres or 73 fresh replays; review receipts stay reviews."),
    "controlled_bound": family(
        "Check exact recurrence arithmetic and supplied transition instances for C_36 and upper-bound induction.",
        "exact bound-instance and recurrence checker", (), "needs",
        "explicit recurrence parameters and independently bound transition/induction premises",
        limits="A finite arithmetic audit cannot establish macro availability or invariant-domain closure."),
    "pump": family(
        "Check finite guarded/prefix-complete decoder, guard and return-distance certificates against full backgrounds.",
        "exact prefix/guard/decoder certificate checker", (), "needs",
        "complete seed/background tables, decoder branches and stage-10 pump/guard premise bindings",
        limits="A recognized pose alone is not the full input family; preserve induction and metric hypotheses."),
    "high": family(
        "Check finite regional seed, prefix, support and height obligations for the guarded no-overshoot family.",
        "regional seed/height certificate checker", (), "needs",
        "complete guarded pump inputs and reachable-lattice sticker theorem bindings",
        limits="No-overshoot records state no new suffix replay; the reachability promise and family restriction remain."),
    "profiles": family(
        "Check native/b07/entry core words, Bad sets, capacities, group/flow witnesses and exact profile joins.",
        "prepared-core and finite profile/flow certificate checker", (), "needs",
        "unbundled action/group arrays or independent replacements, every actual endpoint and preparation witness",
        limits="Equal profiles cannot justify replaying a word at a different endpoint; bound/existence and literal replay are separate."),
    "lattice_bound": family(
        "Check finite B10/B11 cost arithmetic, correction ranks and scoped lattice height certificates.",
        "exact lattice cost/rank and height-instance checker", (), "needs",
        "complete pinned seed, correction and group certificates plus full per-word height inputs",
        limits="B11's 6,775 digits describe a symbolic bound, not an expanded word or measured execution."),
    "admission": family(
        "Check D[k] membership only by complete finite legal enumeration and the three exact branch contracts.",
        "finite D[k] admission checker", (), "needs",
        "exact k/input and complete branch certificates for a new finite membership request",
        limits="Unavailable is not OUTSIDE_CERTIFIED_DOMAIN(k); a cut-off is unknown and reachability remains separate."),
    "finite_extra": family(
        "Check the specifically recorded sigma/tau finite obstruction, continuation, transport or bounded-depth claim.",
        "finite cut/word/coverage certificate checker", (), "needs",
        "full named start/background, literal words and complete obstruction/coverage witnesses for this field",
        limits="Separate positive words from finite exclusions and metric lemmas; absent histories or classifiers do not prove a negative."),
    "shortening": family(
        "Check each recorded seed rewrite/window join and its exact finite search restriction.",
        "local word-equivalence and bounded-window certificate checker", (), "needs",
        "complete input seed/background and per-window join/coverage certificate",
        limits="Shortening a supplied word proves no optimality outside the recorded windows."),
    "global_open": family(
        "Retain unresolved universal availability, entrance, low-tail and quantitative restoration obligations.",
        "not_implemented: no global certificate supplied", (), "record",
        limits="unsupported_claim for a global certificate request; producer execution cannot replace a missing proof."),
}


def classify(stage, relative, value, pointer):
    """One family per extracted slot; unknown paths fail instead of disappearing."""
    fmt = value.get("format") if isinstance(value, dict) else None
    name = relative.rsplit("/", 1)[-1]
    if fmt in ("macro-certificate/1", "macro-check/1"):
        return "macro"
    if stage == 14:
        if name in ("sources.json", "states.json"):
            return "input_catalogue"
        if "replay" in name:
            return "literal"
        if fmt == "t3p-chain/1" and pointer.startswith("/stop"):
            return "finite_search"
        if fmt in ("t3p-certificate/1", "t3p-check/1", "t3p-chain/1"):
            return "t3p"
        if fmt in ("t3p-scan/1", "t3p-pairs/1", "t3p-plus/1") or "peel" in name or "retained" in name:
            return "finite_search"
        if fmt in ("t3p-locality/1", "t3p-levels/1") or name in (
                "s7304-coheld.json", "s7304-end-caps.json", "s7304-jam.json", "s7304-moves-522.json"):
            return "relations"
    if stage == 15:
        if fmt == "pstar-census/1":
            return "relations"
        if fmt == "t3p-pairs/1":
            return "finite_search"
        if relative.startswith("results/runs/"):
            return "selectors"
    if stage == 16:
        if name == "jams.json":
            return "input_catalogue"
        if name.endswith("-trace.json"):
            return "literal"
        if relative.startswith("results/mobility/"):
            return "relations"
        if relative.startswith("results/runs/"):
            return "selectors"
        if relative.startswith(("results/dis/", "results/routing/", "results/t1/", "results/transport/")):
            return "reductions"
    if stage == 17:
        return "rr"
    if stage == 18:
        if relative.startswith(("results/d1-tail/", "results/rule-stop/")):
            return "expansion"
        if relative.startswith("results/lattice-d1/"):
            return "lattice_run"
        if relative.startswith("results/stop-units/"):
            return "static_tail"
    if stage == 19:
        if relative in ("IMPORT-MANIFEST.json", "PUBLICATION-CHECK.json"):
            return "input_catalogue"
        if name.startswith("profile-family-validation-") or name == "irregularity-descent-source-end-table.json.gz":
            return "matching"
        if name.startswith("profile-family-review-"):
            return "indexed_reviews"
        if name == "type-lex-trimmed-independent-results.json":
            if pointer.startswith("/q11_depth_certificate"):
                return "q11"
            if pointer.startswith(("/one_fixed_", "/generic_mixed_rank_proof_check")):
                return "ranks"
            return "frontier"
        if name.startswith(("type-height-weight-", "type-weight-", "type-lex-independent-weight-")):
            return "ranks"
        if name.startswith("type-lex-"):
            return "frontier"
        if name.startswith("controlled-type-height-") or name == "upper-bound-induction-evidence.json":
            return "controlled_bound"
        if name.startswith(("guarded-pump-", "prefix-catalogue-", "prefix-complete-")):
            return "pump"
        if name.startswith(("regional-high-", "high-no-overshoot-")):
            return "high"
        if name.startswith(("lattice-", "tail-coverage-lattice-")):
            return "lattice_bound"
        if name.startswith(("tail-coverage-", "entry-profile-", "profile-tail-", "native-profile-", "b07-profile-")) or relative.startswith(("records/entry-profile-input/", "records/tail-coverage-input/")):
            return "profiles"
        if name.startswith("continuous-framework-"):
            return "admission"
        if name.startswith("seed-"):
            return "shortening"
        if name.startswith(("sigma", "tau", "px3-")):
            return "finite_extra"
    raise ValueError("Unmapped record: " + path(stage, relative))
