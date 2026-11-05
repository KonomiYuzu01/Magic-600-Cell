"""One policy question, supplied with mechanism knowledge, never inferred from cases."""

from .common import load

DENIED = "LG3:capability.not_granted"
UNKNOWN = "PS1:capability.provenance_incompatible"
STATE_ROWS = frozenset(("counts", "floors", "blocked", "bits", "nu2", "nu5",
                        "snapshot", "word_check", "score", "preview_outcome", "batch_check"))
# These spellings bind the descriptive grant groups in the frozen policy to the
# finite operations of this bench. They confer no product capability.
GRANTS = ("read", "setup", "workspace", "preview", "display", "pure")


class Policy:
    def __init__(self):
        self.data = load("policy.json")

    def decide(self, operation, *, form, profile, author, boundary_class,
               invocation, provenance, labels, grants, read_state,
               resource=None, manual_target=False, fixed_entry=False):
        """Return None or the active registry code. All arguments are explicit.

        provenance is the host's flattened metadata (profiles, unknown, valid).
        labels/grants/read_state are the candidate's computed knowledge.
        Neither a case ID nor an expected result is accepted here.
        """
        if form not in self.data["forms"] or profile not in self.data["profiles"]:
            return DENIED
        if operation == "ambient":
            return DENIED if resource == "process" else "LG3:capability.ambient_read"
        if operation in ("dynamic", "commit"):
            return DENIED
        if operation == "preview_outcome" and form == "capability_separation":
            return DENIED
        group = ("read" if operation in STATE_ROWS or operation == "caps" else
                 "workspace" if operation.startswith("workspace_") else
                 "display" if operation in ("emit", "publish", "return") else
                 "pure" if operation in ("calc", "pick", "call", "stop", "tick", "retain", "if", "for") else operation)
        if group not in grants or group not in GRANTS:
            return DENIED
        if operation == "workspace_put" and invocation != "command":
            return DENIED
        if operation == "setup" and profile == "solving":
            if author != "built_in" or not manual_target or not fixed_entry:
                return DENIED
        if operation == "preview" and profile == "solving":
            if not provenance["valid"] or provenance["unknown"] or "unknown" in labels:
                return UNKNOWN
            if author != "built_in" and not self.data["solver_tool_solving_preview"]:
                return DENIED
            if "research" in provenance["profiles"] or "research" in labels:
                return DENIED
            if form == "capability_separation":
                if read_state:
                    return DENIED
            elif set(labels) & {"state", "computed"}:
                return DENIED
        if operation in STATE_ROWS and form == "capability_separation" and profile == "solving":
            if "preview" in grants:
                return DENIED
        if profile == "solving" and operation in ("emit", "publish", "return"):
            if author != "built_in" or boundary_class not in (
                    "tracking", "analysis", "checking", "view_aid", "setup_search_named_target"):
                return DENIED
        return None


def self_test():
    p = Policy()
    args = dict(form="static_labels", profile="solving", author="built_in",
                boundary_class="analysis", invocation="command",
                provenance={"valid": True, "unknown": False, "profiles": ["solving"]},
                labels=[], grants=list(GRANTS), read_state=False)
    assert p.decide("preview", **args) is None
    assert p.decide("preview", **dict(args, labels=["state"])) == DENIED
    assert p.decide("preview", **dict(args, labels=["unknown"])) == UNKNOWN
    assert p.decide("preview", **dict(args, author="own")) == DENIED
    assert p.decide("preview", **dict(args, form="capability_separation", read_state=True)) == DENIED
    assert p.decide("preview_outcome", **dict(args, form="capability_separation")) == DENIED
    assert p.decide("setup", **dict(args, manual_target=True, fixed_entry=True)) is None
    assert p.decide("setup", **args) == DENIED
    for resource in ("clock", "file", "network", "process"):
        assert p.decide("ambient", **args, resource=resource) is not None
