"""Small, explicitly synthetic type/signature catalogue. No kernel implementation."""
import re


IR_VERSION = "lx8-ir-0"
EFFECTS = {"snapshot", "view", "workspace", "preview", "file", "random", "budget", "clock"}
INDICES = {
    "model": {"Full", "Jumble"}, "level": {"Labels", "Poses", "Regions"},
    "claim": {"Witnessed", "Theorem", "Promised", "Refuted", "Open"},
    "alphabet": {"Retained", "MenuA"}, "unit": {"Generators", "MenuLetters", "UIActions"},
    "mode": {"Off", "On"}, "precision": {"P24", "P53", "P113"}, "dimension": {"D2", "D3"},
}
FAMILIES = {
    "identity": "Model Cell Grip Cap Generator Menu MenuLetter Piece Sticker Slot Position Pose StickerClass RegionClass Orbit Level",
    "state": "Snapshot DerivedState Difference",
    "words": "Word WordKey Recipe CompressedWord Cost",
    "operations": "Proposal Preview Receipt Outcome TypedError",
    "claims": "Claim Evidence Certificate Premise Scope Domain Goal Reachability",
    "representations": "Chart Projection Section Layout Atlas Action Embedding Relation Graph Tree CellComplex StateGraph",
    "interface": "View Panel Control Overlay DisplayPlan DisplayInstance SourceMap Command Binding Recording",
    "research": "SearchSpace SearchRecord Strategy Rank Budget RunRecord Branch",
}
TYPE_SLOTS = {}
for names, slots in [
    ("Int Rat Q5 Text SourceMap Provenance FileGrant RandomStream", ()),
    ("Model Cell Grip Cap Generator Menu Proposal Preview Receipt Outcome TypedError Command Binding Recording Rank RunRecord Branch", ("model",)),
    ("Piece Sticker Slot Position Pose StickerClass RegionClass Orbit Difference Claim Evidence Certificate Premise Scope Domain Goal Section Layout Atlas Action Embedding Relation Graph Tree StateGraph View Panel Control Overlay DisplayPlan DisplayInstance SearchSpace SearchRecord Strategy", ("model", "level")),
    ("Snapshot DerivedState", ("model", "level", "claim")),
    ("Word WordKey Recipe CompressedWord MenuLetter", ("model", "alphabet")),
    ("Cost Budget", ("model", "unit")), ("Reachability", ("model", "claim")),
    ("Chart", ("model", "level", "precision")),
    ("Projection", ("model", "level", "level")),
    ("CellComplex", ("model", "level", "dimension")),
    ("Revision", ("model", "mode")), ("Level", ("level",)),
    ("Approx", ("precision",)), ("Verdict", ("evidence",)),
]:
    for name in names.split():
        TYPE_SLOTS[name] = slots


class LoweringError(ValueError):
    """Experiment checker refusal, not a new product diagnostic code."""


def require(condition, message):
    if not condition:
        raise LoweringError(message)


def split_type(text):
    if "<" not in text:
        return text, []
    require(text.endswith(">"), "unclosed type")
    head, tail = text.split("<", 1)
    args, depth, begin = [], 0, 0
    for index, char in enumerate(tail[:-1]):
        if char == "<":
            depth += 1
        elif char == ">":
            depth -= 1
        elif char == "," and depth == 0:
            args.append(tail[begin:index])
            begin = index + 1
        require(depth >= 0, "malformed type")
    require(depth == 0, "malformed type")
    args.append(tail[begin:-1])
    return head, args


def check_type(text):
    head, args = split_type(text)
    require(head != "UserActionToken", "user-action tokens are shell-only and not language values")
    require(head in TYPE_SLOTS, "unknown type: " + head)
    slots = TYPE_SLOTS[head]
    require(len(args) == len(slots), "wrong type arity: " + text)
    for arg, slot in zip(args, slots):
        if slot == "evidence":
            check_type(arg)
            require(split_type(arg)[0] in {"Evidence", "Certificate"}, "verdict needs evidence payload")
        else:
            require(arg in INDICES[slot], "wrong " + slot + " index: " + text)
    if head in {"Word", "CompressedWord", "WordKey", "Recipe", "MenuLetter"}:
        require(args in (["Full", "Retained"], ["Jumble", "MenuA"]), "word alphabet/model mismatch")
    if head in {"Snapshot", "DerivedState"}:
        require(args[:2] in (["Full", "Labels"], ["Jumble", "Poses"]), "state model/level mismatch")
    return text


def charge(steps="1", kernel="0", memory="0", output="0"):
    return dict(zip(("steps", "kernel", "memory", "output"), (steps, kernel, memory, output)))


def signature(params, result, effects=(), charges=None):
    return {"params": params, "result": result, "effects": list(effects),
            "charges": charge() if charges is None else charges}


SNAPSHOT = "Snapshot<Full,Labels,Witnessed>"
WORD = "Word<Full,Retained>"
VERDICT = "Verdict<Certificate<Full,Labels>>"
PRIMITIVES = {
    "exact_add": signature(["Int", "Int"], "Int"),
    "approx53": signature(["Q5"], "Approx<P53>"),
    "describe_identity": signature(["Piece<Full,Labels>", "Position<Full,Labels>"], "Text"),
    "difference": signature([SNAPSHOT, "DerivedState<Full,Labels,Witnessed>"], "Difference<Full,Labels>", ["snapshot"], charge(kernel="7")),
    "check_goal": signature([SNAPSHOT, "Goal<Full,Labels>"], VERDICT, ["snapshot"], charge(kernel="7")),
    "check_word": signature([SNAPSHOT, WORD], VERDICT, ["snapshot"], charge(kernel="7")),
    "save_word": signature([WORD, "Scope<Full,Labels>"], "Receipt<Full>", ["workspace"], charge(kernel="7")),
    "prepare_preview": signature(["Revision<Full,Off>", WORD, "Provenance"], "Receipt<Full>", ["preview"], charge(kernel="7")),
    "make_plan": signature([SNAPSHOT, "Approx<P53>", "Chart<Full,Labels,P53>", "SourceMap"], "DisplayPlan<Full,Labels>", ["snapshot"], charge(kernel="7")),
    "publish": signature(["DisplayPlan<Full,Labels>"], "View<Full,Labels>", ["view"], charge(output="1")),
    "representation": signature(["Graph<Full,Labels>", "Layout<Full,Labels>", "Projection<Jumble,Poses,Labels>", "CellComplex<Full,Labels,D2>"], "Relation<Full,Labels>"),
    "claim_scope": signature(["Claim<Full,Labels>", "Premise<Full,Labels>", "Domain<Full,Labels>", "Reachability<Full,Theorem>"], "Scope<Full,Labels>"),
    "outcome_text": signature(["Outcome<Full>", "TypedError<Full>", "Proposal<Full>", "Preview<Full>"], "Text"),
    "interface_text": signature(["Panel<Full,Labels>", "Control<Full,Labels>", "Overlay<Full,Labels>", "DisplayInstance<Full,Labels>", "Command<Full>", "Binding<Full>", "Recording<Full>"], "Text"),
    "research_note": signature(["SearchRecord<Jumble,Poses>", "Budget<Jumble,MenuLetters>"], "Text", ["budget"]),
    "write_note": signature(["FileGrant", "Text"], "Text", ["file"], charge(output="1")),
    "seeded_sample": signature(["RandomStream", "Int"], "Int", ["random"]),
}
WORD_OPERATIONS = {"word_length", "word_key", "repeat", "join"}


def check_effects(effects):
    require(len(effects) == len(set(effects)), "duplicate effect")
    require(set(effects) <= EFFECTS, "unknown effect (commit is never an effect)")
    return sorted(effects)


def check_charges(charges):
    require(set(charges) == {"steps", "kernel", "memory", "output"}, "wrong charge dimensions")
    require(all(isinstance(n, str) and re.fullmatch(r"0|[1-9][0-9]*", n) for n in charges.values()),
            "charges must be exact nonnegative decimal strings")


def semantic(ir):
    """Erase only source associations; retain every semantic field and array order."""
    if isinstance(ir, dict):
        return {key: semantic(value) for key, value in ir.items() if key != "source"}
    if isinstance(ir, list):
        return [semantic(value) for value in ir]
    return ir
