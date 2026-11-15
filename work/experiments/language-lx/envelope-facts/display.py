"""Bounded synthetic probe of chapter 06, not a general evidence engine.

The six graph shapes and finite field alphabets below define the admitted probe
model. Real corpus envelopes stay recorded until a typed statement can be bound.
"""

from collections import Counter
import hashlib
from itertools import product

from corpus import canonical, require, sha

BASIS = ("theorem", "certificate", "exhaustive", "exact", "sample")
CHECK = ("none", "producer", "independent", "model", "human", "formal")
GRADES = (None,) + tuple(product(BASIS, CHECK))
SHAPES = ("empty", "positive", "negative", "both", "premise", "failed")
SCOPES = ("one_instance", "search_set", "admitted_domain", "every_reachable")
FORMS = (None, "finite", "universal", "existence", "mixed_forall", "mixed_exists")
COVERED = ("absent", "same", "narrower", "domain")
SEARCH = ("absent", "closed", "budget", "limit", "wall", "resource", "other")
JOBS = (None, "completed", "cancelled", "cut_off", "error", "not_run", "not_stated")
MODES = (None, "transport", "audit", "replay", "not_replayed")
DOMAINS = dict(shape=SHAPES, grade=GRADES, scope=SCOPES, form=FORMS,
               covered=COVERED, search=SEARCH, job=JOBS,
               membership=(False, True), mode=MODES)

OUTCOMES = (
    "word:open", "word:finite instance", "word:proved", "word:conditional",
    "word:refuted", "word:not found within budget", "word:incomplete",
    "EVD:evidence.inconsistent_fields:I1", "EVD:evidence.inconsistent_fields:I2",
    "EVD:evidence.inconsistent_fields:I3", "EVD:evidence.inconsistent_fields:I4",
    "EVD:evidence.negative_without_completion:I5",
    "EVD:evidence.inconsistent_fields:I6", "EVD:evidence.inconsistent_fields:I7a",
    "EVD:evidence.inconsistent_fields:I7b", "EVD:evidence.inconsistent_fields:I7c",
    "EF:inconsistency.missing_search_record", "EF:inconsistency.unknown_cutoff_reason",
    "EF:inconsistency.missing_mixed_refutation_witness",
    "EF:collision.not_run_vs_budget",
    "EF:collision.existence_refutation_basis",
    "EF:collision.not_run_vs_support",
    "EF:collision.negative_job_without_completion",
)
CODE = {name: i for i, name in enumerate(OUTCOMES)}


def row(s0, s1, n0, scope, search, job):
    if s0 and n0:
        return "EVD:evidence.inconsistent_fields:I4"
    if n0:
        return "word:refuted"
    if s0:
        return "word:finite instance" if scope in SCOPES[:2] else "word:proved"
    if s1:
        return "word:conditional"
    if search in ("budget", "limit"):
        if job == "not_run":
            # EVD.32 row 5 versus EVD.47's explicit row-7 instruction. Do not choose.
            return "EF:collision.not_run_vs_budget"
        return "word:not found within budget"
    if job in ("cancelled", "error") or search in ("wall", "resource"):
        return "word:incomplete"
    if job == "cut_off":
        if search == "absent":
            return "word:incomplete"
        return "EF:inconsistency.unknown_cutoff_reason"
    return "word:open"


def negate(form):
    return {None: None, "finite": "finite", "universal": "existence",
            "existence": "universal", "mixed_forall": "mixed_exists",
            "mixed_exists": "mixed_forall"}[form]


def evaluate(shape, grade, scope, form, covered, search, job, membership, mode):
    # mode is deliberately unread (EVD.4/47). Mode names do not construct edges.
    if form == "finite" and scope != "one_instance":
        return "EVD:evidence.inconsistent_fields:I7c"
    has_leaf = grade is not None and shape not in ("empty", "failed")
    positive = has_leaf and shape in ("positive", "both", "premise")
    negative = has_leaf and shape in ("negative", "both")
    basis, check = grade if grade else (None, None)
    if has_leaf and basis in ("exhaustive", "exact"):
        kind = scope if covered == "same" else "admitted_domain" if covered == "domain" else "one_instance"
        if covered != "absent" and kind not in SCOPES[:2]:
            return "EVD:evidence.inconsistent_fields:I7b"
    if has_leaf and (negative or basis == "exhaustive"):
        if search != "absent" and (search != "closed" or job in ("cancelled", "cut_off", "error", "not_run")):
            return "EVD:evidence.negative_without_completion:I5"
        if basis == "exhaustive" and search == "absent":
            return "EF:inconsistency.missing_search_record"
    if negative and form == "universal" and not membership:
        return "EVD:evidence.inconsistent_fields:I6"
    if negative and form == "mixed_forall":
        return "EF:inconsistency.missing_mixed_refutation_witness"

    def support(target_form, exists, is_negative):
        if not exists:
            return False, None
        if basis in ("certificate", "exhaustive", "exact") and target_form in (None, "universal", "mixed_forall"):
            if covered == "absent":
                return False, None  # EVD.41 absence is inert, not I7a.
            if covered != "same":
                if target_form is None:
                    return False, None
                return False, "EVD:evidence.inconsistent_fields:I7a"
        if (is_negative or basis == "exhaustive") and search != "absent" and job in (None, "not_stated"):
            return False, None
        return basis != "sample" and check != "none", None  # trial U0

    p, error = support(form, positive, False)
    if error:
        return error
    n, error = support(negate(form), negative, True)
    if error:
        return error
    if (p or n) and job == "not_run":
        return "EF:collision.not_run_vs_support"
    if n and job in ("cancelled", "cut_off", "error"):
        # I5 catches search-backed leaves above. EVD.43 also speaks about a
        # negative produced by this job when no SearchRecord was stored.
        return "EF:collision.negative_job_without_completion"
    if n and form in ("existence", "mixed_exists") and basis == "exact":
        # Row 1/I7 allow this equal-coverage leaf, but EVD.42 asks for a proof
        # or complete certificate. No conversion from this basis is specified.
        return "EF:collision.existence_refutation_basis"
    return row(p and shape != "premise", p, n, scope, search, job)


def phrase(scope, guards=()):
    text = {"one_instance": "for the pinned finite input",
            "search_set": "for the declared family, alphabet and bounds",
            "admitted_domain": "on the named admitted domain",
            "every_reachable": "for every reachable state of the named puzzle"}[scope]
    for kind, name in guards:
        text += "; " + {"reachability_promise": "for reachable inputs",
                         "open_hypothesis": "given " + name,
                         "model_assumption": "under " + name}[kind]
    return text


def probes():
    """Fixed adversarial checks, separate from exhaustive totality counting."""
    base = ("positive", ("exact", "producer"), "one_instance", "finite",
            "same", "absent", "completed", True, None)

    def outcome(**changes):
        args = dict(zip(DOMAINS, base))
        args.update(changes)
        return evaluate(**args)

    checks = [
        (outcome(), "word:finite instance"),
        (outcome(shape="failed"), "word:open"),
        (outcome(grade=("sample", "formal")), "word:open"),
        (outcome(grade=("theorem", "none")), "word:open"),
        (outcome(shape="both"), "EVD:evidence.inconsistent_fields:I4"),
        (outcome(grade=("exhaustive", "producer"), search="closed", job="not_stated"), "word:open"),
        (outcome(grade=("exhaustive", "producer"), search="closed"), "word:finite instance"),
        (outcome(grade=("exhaustive", "producer"), search="budget"), "EVD:evidence.negative_without_completion:I5"),
        (outcome(shape="negative", form="universal", membership=False), "EVD:evidence.inconsistent_fields:I6"),
        (outcome(form="universal", covered="narrower"), "EVD:evidence.inconsistent_fields:I7a"),
        (outcome(form="universal", covered="absent"), "word:open"),
        (outcome(form=None, covered="narrower"), "word:open"),
        (outcome(covered="domain"), "EVD:evidence.inconsistent_fields:I7b"),
        (outcome(scope="search_set"), "EVD:evidence.inconsistent_fields:I7c"),
        (outcome(shape="empty", search="budget", job="not_run"), "EF:collision.not_run_vs_budget"),
        (outcome(shape="empty", search="limit", job="error"), "word:not found within budget"),
        (outcome(shape="empty", search="wall", job="cut_off"), "word:incomplete"),
        (outcome(shape="premise", grade=("theorem", "model")), "word:conditional"),
        (outcome(shape="negative", form="existence", covered="absent"), "word:open"),
        (outcome(shape="negative", form="existence"), "EF:collision.existence_refutation_basis"),
        (outcome(job="not_run"), "EF:collision.not_run_vs_support"),
        (outcome(shape="negative", grade=("theorem", "producer"), job="error"),
         "EF:collision.negative_job_without_completion"),
    ]
    require(all(actual == expected for actual, expected in checks), "Display fixture failed")
    for mode in MODES:
        require(outcome(mode=mode) == outcome(), "Mode changed display")
    for kind, expected in (("reachability_promise", "for reachable inputs"),
                           ("open_hypothesis", "given H"), ("model_assumption", "under H")):
        require(expected in phrase("admitted_domain", [(kind, "H")]), "Lost guard phrase")
    require(all(phrase(scope) for scope in SCOPES), "Lost scope phrase")
    return len(checks) + len(MODES) + 4


def enumerate_model():
    fixture_count = probes()
    node_counts = Counter()
    states = [(s0, s1, n0) for s0, s1, n0 in product((False, True), repeat=3) if not s0 or s1]
    for (s0, s1, n0), scope, search, job in product(states, SCOPES, ("absent", "closed", "budget"), JOBS):
        node_counts[row(s0, s1, n0, scope, search, job)] += 1
    require(sum(node_counts.values()) == 504, "CT-EVD-01 denominator changed")
    counts = Counter()
    digest = hashlib.sha256()
    chunk = bytearray()
    core_domains = list(DOMAINS.values())[:-1]
    for fields in product(*core_domains):
        first = None
        for mode in MODES:
            result = evaluate(*fields, mode)
            require(result in CODE, "No unique typed outcome")
            if first is None:
                first = result
            require(result == first, "Mode supplied evidence")
            counts[result] += 1
            chunk.append(CODE[result])
        if len(chunk) >= 65536:
            digest.update(chunk)
            chunk.clear()
    digest.update(chunk)
    expected = 1
    for values in DOMAINS.values():
        expected *= len(values)
    require(sum(counts.values()) == expected, "Incomplete Cartesian enumeration")
    return dict(format="ps1.38-display-probe/1", evidence_class="synthetic",
                decision="open", decider="owner", threshold="trial U0; no LQ3 answer",
                domains=DOMAINS, domain_order=list(DOMAINS), outcomes=OUTCOMES,
                combinations=expected, outcomes_sha256=digest.hexdigest(), counts=dict(counts),
                digest_encoding="one outcome-index byte per lexicographic product tuple; mode last",
                node_level_cases=504, node_level_counts=dict(node_counts),
                adversarial_checks=fixture_count,
                scope_phrase_templates={scope: phrase(scope) for scope in SCOPES},
                collisions=["EF:collision.not_run_vs_budget: EVD.32 row 5 versus EVD.47 row 7",
                            "EF:collision.existence_refutation_basis: EVD.32 row 1/I7 versus EVD.42",
                            "EF:collision.not_run_vs_support: EVD.32 rows 1-4 versus EVD.47 row 7",
                            "EF:collision.negative_job_without_completion: EVD.32 row 1 versus EVD.43"],
                collision_examples=[
                    dict(facts=dict(zip(DOMAINS, ("empty", None, "one_instance", "finite",
                                                  "absent", "budget", "not_run", False, None))),
                         outcome="EF:collision.not_run_vs_budget"),
                    dict(facts=dict(zip(DOMAINS, ("negative", ("exact", "producer"), "one_instance",
                                                  "existence", "same", "absent", "completed", True, None))),
                         outcome="EF:collision.existence_refutation_basis"),
                    dict(facts=dict(zip(DOMAINS, ("positive", ("exact", "producer"), "one_instance",
                                                  "finite", "same", "absent", "not_run", True, None))),
                         outcome="EF:collision.not_run_vs_support"),
                    dict(facts=dict(zip(DOMAINS, ("negative", ("theorem", "producer"), "one_instance",
                                                  "finite", "absent", "absent", "error", True, None))),
                         outcome="EF:collision.negative_job_without_completion")],
                limitations=["bounded shapes only; no arbitrary EVD graph conformance",
                             "typed EF outcomes diagnose missing facts/open rules, not new chapter-06 error codes",
                             "source payload, source IDs and names are opaque to the word function",
                             "evidence-threshold and invalid-pair decisions remain open"])
