"""Author/freeze the synthetic corpus. Deliberately imports no parser or lowerer.

The expected types, effects, charges and values below are the fixture oracle.
The printers only spell this authored corpus; check.py never runs this builder.
"""
import hashlib
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
VERSION = "lx8-ir-0"
EDITION = "lx8-0"
CANDIDATES = {"candidate-a": "lxa8", "candidate-b": "lxb8", "candidate-c": "lxc8"}


def encoded(value):
    return (json.dumps(value, indent=2, ensure_ascii=False) + "\n").encode("utf-8")


def digest(data):
    return hashlib.sha256(data).hexdigest()


def put(path, data):
    target = ROOT / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(data if isinstance(data, bytes) else data.encode("utf-8"))
    return {"path": path, "sha256": digest(target.read_bytes())}


def cost(steps="1", kernel="0", memory="0", output="0"):
    return dict(zip(("steps", "kernel", "memory", "output"), (steps, kernel, memory, output)))


def op(opcode, type_, effects=(), charges=None, **fields):
    return {"op": opcode, "type": type_, "effects": sorted(effects),
            "charges": cost() if charges is None else charges, **fields}


def ref(name, type_, length=None):
    extra = {} if length is None else {"expanded_length": length}
    return op("ref", type_, charges=cost(steps="0"), name=name, **extra)


def literal(kind, type_, value):
    return op("literal." + kind, type_, value=value)


def word(letters, menu=False):
    return op("literal.word", "Word<Jumble,MenuA>" if menu else "Word<Full,Retained>",
              letters=letters, expanded_length=str(len(letters)))


def call(name, type_, args, effects=(), charges=None, length=None):
    all_effects = set(effects)
    for arg in args:
        all_effects.update(arg["effects"])
    extra = {} if length is None else {"expanded_length": length}
    return op("call", type_, all_effects, charges, target=name, args=args, **extra)


def function(name, params, result, body, effects=(), charges=None, bindings=(), kind="function", metadata=None):
    binding_rows = [{"name": name, "type": value["type"], "value": value} for name, value in bindings]
    actual = set(body["effects"])
    for binding in binding_rows:
        actual.update(binding["value"]["effects"])
    return {"kind": kind, "name": name, "params": [{"name": n, "type": t} for n, t in params],
            "result": result, "declared_effects": sorted(effects), "effects": sorted(actual),
            "charges": cost(steps="12") if charges is None else charges, "metadata": metadata,
            "bindings": binding_rows, "body": body}


def program(name, declarations, requires=(), imports=()):
    return {"ir_version": VERSION, "edition": EDITION, "module": name,
            "requires": sorted(requires), "imports": list(imports), "declarations": declarations}


def corpus(interface_digest):
    w, snapshot = "Word<Full,Retained>", "Snapshot<Full,Labels,Witnessed>"
    verdict, compressed = "Verdict<Certificate<Full,Labels>>", "CompressedWord<Full,Retained>"
    rows = []

    def add(key, features, ir):
        rows.append({"id": key, "covers": features, "ir": ir})

    # More than 4,300 decimal digits exercises CPython's ordinary conversion ceiling.
    huge = "9" * 5001
    phi = literal("q5", "Q5", {"one": ["1", "2"], "sqrt5": ["1", "2"]})
    add("01-exact-values", ["exact-integer", "exact-rational", "exact-q5"], program("exact_values", [
        function("constants", [], "Q5", literal("q5", "Q5", {"one": ["-3", "2"], "sqrt5": ["5", "7"]}),
                 bindings=[("large", literal("int", "Int", huge)),
                           ("fraction", literal("rat", "Rat", ["-3", "4"])),
                           ("phi", phi),
                           ("sum", call("exact_add", "Int", [ref("large", "Int"), literal("int", "Int", "7")]))])]))
    add("02-explicit-approx", ["explicit-approximation", "unicode-locations"], program("display_numbers", [
        function("scale", [], "Approx<P53>", call("approx53", "Approx<P53>", [ref("exact", "Q5")]),
                 bindings=[("exact", phi),
                           ("small", literal("approx", "Approx<P24>", {"decimal": "1.25e-1", "precision_bits": "24"})),
                           ("wide", literal("approx", "Approx<P113>", {"decimal": "-0.00000000000000000001", "precision_bits": "113"})),
                           ("label", literal("text", "Text", "scale of λ"))])]))
    identities = [
        ("model", "Model<Full>"), ("cell", "Cell<Full>"), ("grip", "Grip<Full>"), ("cap", "Cap<Full>"),
        ("generator", "Generator<Full>"), ("menu_source", "Menu<Jumble>"), ("letter", "MenuLetter<Jumble,MenuA>"),
        ("piece", "Piece<Full,Labels>"), ("sticker", "Sticker<Full,Labels>"), ("slot", "Slot<Full,Labels>"),
        ("position", "Position<Full,Labels>"), ("pose", "Pose<Jumble,Poses>"),
        ("sticker_class", "StickerClass<Jumble,Labels>"), ("region_class", "RegionClass<Jumble,Regions>"),
        ("orbit", "Orbit<Full,Labels>"), ("level", "Level<Labels>"),
    ]
    add("03-identities", ["identity"], program("orbit_identity", [
        function("describe", identities, "Text", call("describe_identity", "Text", [ref("piece", "Piece<Full,Labels>"), ref("position", "Position<Full,Labels>")]))]))
    state_params = [("before", snapshot), ("after", "DerivedState<Full,Labels,Witnessed>"),
                    ("jumble", "Snapshot<Jumble,Poses,Witnessed>"), ("derived", "DerivedState<Jumble,Poses,Promised>")]
    add("04-states", ["state", "reachability-indices"], program("state_difference", [
        function("compare", state_params, "Difference<Full,Labels>",
                 call("difference", "Difference<Full,Labels>", [ref(*state_params[0]), ref(*state_params[1])], ["snapshot"], cost(kernel="7")), ["snapshot"])], ["snapshot"]))
    letters = ["1", "-1", "2", "1200", "-1200", "1"]
    menu = [{"grip": "2", "rotation": "spin"}, {"grip": "2", "rotation": "unspin"}, {"grip": "5", "rotation": "identity"}]
    add("05-literal-words", ["words", "literal-order", "equality-key-separation", "menu-word"], program("authored_words", [
        function("witness", [], w, word(letters), bindings=[("key", call("word_key", "WordKey<Full,Retained>", [word(letters)]))]),
        function("menu_witness", [], "Word<Jumble,MenuA>", word(menu, True)),
        function("menu_cost", [], "Cost<Jumble,MenuLetters>", call("word_length", "Cost<Jumble,MenuLetters>", [call("menu_witness", "Word<Jumble,MenuA>", [], charges=cost(steps="12"))])),
        function("recipe_note", [("recipe", "Recipe<Full,Retained>"), ("count", "Cost<Full,Generators>")], "Text", literal("text", "Text", "User-authored recipe; no expansion executed.")),
    ]))
    times = "1" + "0" * 80
    long_length = "3" + "0" * 80
    total_length = "3" + "0" * 79 + "4"
    add("06-compressed-words", ["words", "compressed-words", "unbounded-length"], program("compressed_witness", [
        function("large_word", [], compressed,
                 call("join", compressed, [ref("long_word", compressed, long_length), ref("tail", compressed, "4")], length=total_length),
                 bindings=[("literal", word(["1", "-1", "7"])),
                           ("long_word", call("repeat", compressed, [ref("literal", w, "3"), literal("int", "Int", times)], length=long_length)),
                           ("tail", call("repeat", compressed, [word(["-7", "3"]), literal("int", "Int", "2")], length="4"))]),
        function("menu_repeat", [], "CompressedWord<Jumble,MenuA>",
                 call("repeat", "CompressedWord<Jumble,MenuA>", [word(menu, True), literal("int", "Int", "3")], length="9")),
    ]))
    match = op("match.verdict", "Text", subject=ref("verdict", verdict), arms=[
        {"tag": "yes", "binder": {"name": "proof", "type": "Certificate<Full,Labels>"}, "body": literal("text", "Text", "supported")},
        {"tag": "no", "binder": {"name": "counterexample", "type": "Certificate<Full,Labels>"}, "body": literal("text", "Text", "refuted")},
        {"tag": "unknown", "binder": {"name": "reason", "type": "Text"}, "body": ref("reason", "Text")},
    ])
    add("07-verdict-match", ["verdict-matching", "unknown-preserved"], program("protection_status", [function("status", [("verdict", verdict)], "Text", match)]))
    params = [("snapshot", snapshot), ("witness", w)]
    add("08-effect-functions", ["effect-functions", "transitive-effects"], program("protection_helpers", [
        function("delegated", params, verdict, call("probe", verdict, [ref(*p) for p in params], ["snapshot"], cost(steps="12")), ["snapshot"]),
        function("probe", params, verdict, call("check_word", verdict, [ref(*p) for p in params], ["snapshot"], cost(kernel="7")), ["snapshot"]),
    ], ["snapshot"]))
    add("09-modules", ["modules", "pinned-interface"], program("module_client", [
        function("length", [("witness", w)], "Cost<Full,Generators>", call("helpers_length", "Cost<Full,Generators>", [ref("witness", w)], charges=cost(steps="2")))
    ], imports=[{"name": "helpers", "digest": interface_digest}]))
    params = [("witness", w), ("scope", "Scope<Full,Labels>")]
    add("10-workspace-command", ["commands", "workspace-effect"], program("macro_workspace", [
        function("remember", params, "Receipt<Full>", call("save_word", "Receipt<Full>", [ref(*p) for p in params], ["workspace"], cost(kernel="7")),
                 ["workspace"], cost("2", "9", "0", "1"), kind="command",
                 metadata={"row": "macro.save", "kind": "workspace", "contexts": ["macro_base"], "permits": ["save_word"]})
    ], ["workspace"]))
    params = [("base", "Revision<Full,Off>"), ("witness", w), ("provenance", "Provenance")]
    add("11-preview-command", ["commands", "preview-without-commit"], program("explicit_preview", [
        function("prepare", params, "Receipt<Full>", call("prepare_preview", "Receipt<Full>", [ref(*p) for p in params], ["preview"], cost(kernel="7")),
                 ["preview"], kind="command", metadata={"row": "operation.preview", "kind": "query", "contexts": ["operation"], "permits": ["prepare_preview"]})
    ], ["preview"]))
    params = [("snapshot", snapshot), ("chart", "Chart<Full,Labels,P53>"), ("source", "SourceMap")]
    add("12-revision-view", ["views", "revision-bound-input", "interface"], program("orbit_display", [
        function("orbit_view", params, "View<Full,Labels>", call("publish", "View<Full,Labels>", [ref("plan", "DisplayPlan<Full,Labels>")], ["view"], cost(output="1")),
                 ["snapshot", "view"], kind="view", bindings=[
                     ("scale", literal("approx", "Approx<P53>", {"decimal": "0.5", "precision_bits": "53"})),
                     ("plan", call("make_plan", "DisplayPlan<Full,Labels>", [ref(*params[0]), ref("scale", "Approx<P53>"), ref(*params[1]), ref(*params[2])], ["snapshot"], cost(kernel="7")))])
    ], ["snapshot", "view"]))
    params = [("graph", "Graph<Full,Labels>"), ("layout", "Layout<Full,Labels>"),
              ("projection", "Projection<Jumble,Poses,Labels>"), ("complex", "CellComplex<Full,Labels,D2>"),
              ("section", "Section<Full,Labels>"), ("atlas", "Atlas<Full,Labels>"), ("action", "Action<Full,Labels>"),
              ("embedding", "Embedding<Full,Labels>"), ("tree", "Tree<Full,Labels>"), ("states", "StateGraph<Jumble,Poses>")]
    add("13-representations", ["representations"], program("representation_inputs", [
        function("relations", params, "Relation<Full,Labels>", call("representation", "Relation<Full,Labels>", [ref(*p) for p in params[:4]]))]))
    params = [("claim", "Claim<Full,Labels>"), ("premise", "Premise<Full,Labels>"), ("domain", "Domain<Full,Labels>"),
              ("reachability", "Reachability<Full,Theorem>"), ("evidence", "Evidence<Full,Labels>"),
              ("certificate", "Certificate<Full,Labels>"), ("goal", "Goal<Full,Labels>")]
    add("14-claims", ["claims"], program("claim_context", [
        function("scope", params, "Scope<Full,Labels>", call("claim_scope", "Scope<Full,Labels>", [ref(*p) for p in params[:4]]))]))
    params = [("outcome", "Outcome<Full>"), ("error", "TypedError<Full>"), ("proposal", "Proposal<Full>"), ("preview", "Preview<Full>"), ("receipt", "Receipt<Full>")]
    interface_params = [("panel", "Panel<Full,Labels>"), ("control", "Control<Full,Labels>"), ("overlay", "Overlay<Full,Labels>"),
                        ("instance", "DisplayInstance<Full,Labels>"), ("command", "Command<Full>"), ("binding", "Binding<Full>"), ("recording", "Recording<Full>")]
    add("15-operations-interface", ["operations", "interface", "shell-token-unavailable"], program("operation_records", [
        function("describe_outcome", params, "Text", call("outcome_text", "Text", [ref(*p) for p in params[:4]])),
        function("describe_controls", interface_params, "Text", call("interface_text", "Text", [ref(*p) for p in interface_params])),
    ]))
    params = [("space", "SearchSpace<Jumble,Poses>"), ("record", "SearchRecord<Jumble,Poses>"), ("strategy", "Strategy<Jumble,Poses>"),
              ("rank", "Rank<Jumble>"), ("budget", "Budget<Jumble,MenuLetters>"), ("run", "RunRecord<Jumble>"), ("branch", "Branch<Jumble>")]
    add("16-research-records", ["research", "budget-effect"], program("research_records", [
        function("summarize", params, "Text", call("research_note", "Text", [ref(*params[1]), ref(*params[4])], ["budget"]), ["budget"])
    ], ["budget"]))
    add("17-granted-effects", ["effect-functions", "explicit-grants"], program("granted_inputs", [
        function("note", [("grant", "FileGrant")], "Text", call("write_note", "Text", [ref("grant", "FileGrant"), literal("text", "Text", "Fixture note")], ["file"], cost(output="1")), ["file"]),
        function("sample", [("stream", "RandomStream")], "Int", call("seeded_sample", "Int", [ref("stream", "RandomStream"), literal("int", "Int", "17")], ["random"]), ["random"]),
    ], ["file", "random"]))
    return rows


def quoted(value):
    return json.dumps(value, ensure_ascii=False)


def names(values):
    return "[" + ", ".join(values) + "]"


def expression(node, surface):
    kind = node["op"]
    if kind == "ref":
        return node["name"]
    if kind == "literal.int":
        return node["value"]
    if kind == "literal.rat":
        # Three exact spellings with the same expected normalized rational.
        return {"a": "-6/8", "b": "6/-8", "c": "-0006/0008"}[surface]
    if kind == "literal.q5":
        value = node["value"]
        return "q5(" + "/".join(value["one"]) + ", " + "/".join(value["sqrt5"]) + ")"
    if kind == "literal.text":
        return quoted(node["value"])
    if kind == "literal.approx":
        return "~" + node["value"]["precision_bits"] + quoted(node["value"]["decimal"])
    if kind == "literal.word":
        if node["type"] == "Word<Full,Retained>":
            return "word" + names([("+" if not x.startswith("-") else "") + x for x in node["letters"]])
        return "menu" + names([x["grip"] + ":" + x["rotation"] for x in node["letters"]])
    if kind == "call":
        args = [expression(arg, surface) for arg in node["args"]]
        if surface == "c" and args:
            return "(" + args[0] + " |> " + node["target"] + "(" + ", ".join(args[1:]) + "))"
        return node["target"] + "(" + ", ".join(args) + ")"
    if kind == "match.verdict":
        subject = expression(node["subject"], surface)
        arms = [a["tag"] + "(" + a["binder"]["name"] + ") => " + expression(a["body"], surface) for a in node["arms"]]
        if surface == "a":
            return "match " + subject + " { " + "; ".join(arms) + "; }"
        if surface == "b":
            return "choose " + subject + ":\n" + "\n".join("        " + arm for arm in arms)
        return "case " + subject + " of " + " | ".join(arms) + " end"
    raise ValueError(kind)


def declaration(decl, surface):
    params = ", ".join(p["name"] + ": " + p["type"] for p in decl["params"])
    effects = names(decl["declared_effects"])
    charges = names(list(decl["charges"].values()))
    name, result, kind = decl["name"], decl["result"], decl["kind"]
    meta = decl["metadata"]
    metadata = ""
    if meta:
        metadata = " row " + quoted(meta["row"]) + " kind " + meta["kind"] + " contexts " + names(meta["contexts"]) + " permits " + names(meta["permits"])
    if surface == "a":
        header = ("fn" if kind == "function" else kind) + " " + name + "(" + params + ") -> " + result + " effects " + effects + " charge " + charges + metadata + " {\n"
        bindings = "".join("    let " + b["name"] + " = " + expression(b["value"], surface) + ";\n" for b in decl["bindings"])
        return header + bindings + "    return " + expression(decl["body"], surface) + ";\n}\n"
    if surface == "b":
        header = kind + " " + name + "(" + params + ") returns " + result + " using " + effects + " charge " + charges + metadata + ":\n"
        bindings = "".join("    " + b["name"] + " := " + expression(b["value"], surface) + "\n" for b in decl["bindings"])
        return header + bindings + "    yield " + expression(decl["body"], surface) + "\n"
    if meta:
        metadata = " with (" + quoted(meta["row"]) + ", " + meta["kind"] + ", " + names(meta["contexts"]) + ", " + names(meta["permits"]) + ")"
    keyword = {"function": "fn", "command": "cmd", "view": "view"}[kind]
    header = keyword + " " + name + "(" + params + ") : " + result + " ! " + effects + " @ " + charges + metadata + " =\n"
    bindings = "".join("    bind " + b["name"] + " <- " + expression(b["value"], surface) + " in\n" for b in decl["bindings"])
    return header + bindings + "    " + expression(decl["body"], surface) + ";\n"


def source(ir, surface):
    if surface == "a":
        header = 'edition "lx8-0";\nmodule ' + ir["module"] + " requires " + names(ir["requires"]) + ";\n"
        imports = "".join("import " + i["name"] + " " + quoted(i["digest"]) + ";\n" for i in ir["imports"])
    elif surface == "b":
        header = 'edition: "lx8-0"\nmodule ' + ir["module"] + " requires " + names(ir["requires"]) + "\n"
        imports = "".join("use " + i["name"] + " from " + quoted(i["digest"]) + "\n" for i in ir["imports"])
    else:
        header = '@edition("lx8-0");\n@module(' + ir["module"] + ", " + names(ir["requires"]) + ");\n"
        imports = "".join("@use(" + i["name"] + ", " + quoted(i["digest"]) + ");\n" for i in ir["imports"])
    return header + imports + "\n" + "\n".join(declaration(d, surface) for d in ir["declarations"])


def recovery(surface):
    bad = function("broken", [], "Int", literal("int", "Int", "123"))
    good = function("recovered", [], "Int", literal("int", "Int", "99"))
    base = source(program("draft", [bad, good]), surface)
    failures = [
        ("missing-edition", "\n".join(base.split("\n")[1:]), "edition_missing", []),
        ("unsupported-edition", base.replace("lx8-0", "lx8-future"), "edition_unsupported", []),
    ]
    for name, replacement, code in [
        ("zero-denominator", "1/0", "invalid_literal"),
        ("approx-precision", '~17"0.5"', "invalid_literal"),
        ("approx-nonfinite", '~53"NaN"', "invalid_literal"),
        ("unmarked-decimal", "0.5", "invalid_literal"),
        ("q5-coefficient", "q5(1, 1/2)", "invalid_literal"),
        ("zero-generator", "word[0]", "invalid_literal"),
        ("missing-expression", "", "unexpected_token"),
        ("missing-delimiter", "exact_add(1, 2", "unexpected_token"),
        ("unknown-character", "$", "unexpected_token"),
        ("bad-string-escape", '"\\q"', "invalid_literal"),
    ]:
        failures.append((name, base.replace("123", replacement), code, ["recovered"]))
    # A damaged header and missing surface-specific block boundary.
    failures.append(("parameter-colon", base.replace("broken()", "broken(x Int)"), "unexpected_token", ["recovered"]))
    if surface == "a":
        damaged = base.replace(";\n}\n", ";\n", 1)
    elif surface == "b":
        damaged = base.replace("    yield 123", "  yield 123")
    else:
        damaged = base.replace("123;", "bind x <- 123 99;")
    failures.append(("block-boundary", damaged, "unexpected_token", ["recovered"]))
    return failures, base


def main():
    interface = {"ir_version": VERSION, "exports": {"length": {
        "params": ["Word<Full,Retained>"], "result": "Cost<Full,Generators>", "effects": [], "charges": cost(steps="2")}}}
    support = put("fixtures/support-interface.json", encoded(interface))
    manifest = {"experiment": "E-2.4-L8", "decision": "open", "decider": "owner", "schema": "lx8-manifest-0",
                "interface": support, "programs": [], "recovery": [], "byte_recovery": []}
    for row in corpus(support["sha256"]):
        golden = put("fixtures/expected/" + row["id"] + ".json", encoded(row["ir"]))
        sources = {}
        for candidate, extension in CANDIDATES.items():
            sources[candidate] = put(candidate + "/fixtures/" + row["id"] + "." + extension, source(row["ir"], candidate[-1]))
        manifest["programs"].append({"id": row["id"], "covers": row["covers"], "expected": golden, "sources": sources})
    for candidate, extension in CANDIDATES.items():
        failures, base = recovery(candidate[-1])
        for name, draft, code, recovered in failures:
            entry = put(candidate + "/recovery/" + name + "." + extension, draft)
            manifest["recovery"].append({"candidate": candidate, "id": name, **entry,
                                         "codes": ["LG1:syntax." + code] + (["LG1:syntax.recovery_gap"] if recovered else []),
                                         "recovered": recovered})
        for name, data in [("invalid-utf8", base.encode("utf-8") + b"\xff"), ("unpinned-crlf", base.replace("\n", "\r\n").encode("utf-8"))]:
            # Broken bytes are frozen as hex in public UTF-8/LF text, never repaired on read.
            entry = put(candidate + "/recovery/" + name + ".json", encoded({"source_hex": data.hex()}))
            manifest["byte_recovery"].append({"candidate": candidate, "id": name, **entry,
                                              "codes": ["LG1:syntax.invalid_encoding"], "recovered": []})
    candidate_files = ["syntax.py", "semantics.py", "lower.py", "check.py", "fixtures/build.py", "ir.md", "CARD.md", "PROTOCOL-NEEDS.md"]
    candidate_files += [candidate + "/" + leaf for candidate in CANDIDATES for leaf in ("parser.py", "DESCRIPTION.md")]
    manifest["candidate_files"] = {path: digest((ROOT / path).read_bytes()) for path in candidate_files if (ROOT / path).is_file()}
    inputs = {support["path"]: support["sha256"]}
    for row in manifest["programs"]:
        for item in [row["expected"], *row["sources"].values()]:
            inputs[item["path"]] = item["sha256"]
    for row in [*manifest["recovery"], *manifest["byte_recovery"]]:
        inputs[row["path"]] = row["sha256"]
    manifest["candidate_sha256"] = digest(json.dumps(manifest["candidate_files"], sort_keys=True, separators=(",", ":")).encode())
    manifest["input_sha256"] = digest(json.dumps(inputs, sort_keys=True, separators=(",", ":")).encode())
    put("fixtures/MANIFEST.json", encoded(manifest))
    print(f"Frozen {len(manifest['programs'])} programs per candidate and 16 broken drafts per candidate.")
    print("Candidate SHA-256: " + manifest["candidate_sha256"])
    print("Input SHA-256: " + manifest["input_sha256"])


if __name__ == "__main__":
    main()
