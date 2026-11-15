"""Lower the three parsed surfaces to one experiment IR. Nothing executes."""
import re

from syntax import EDITION, decimal, integer
from semantics import (IR_VERSION, PRIMITIVES, WORD_OPERATIONS, LoweringError, charge,
                       check_charges, check_effects, check_type, require, split_type)


def union(*groups):
    return sorted(set().union(*groups))


class Lowerer:
    def __init__(self, parsed, interfaces):
        self.parsed, self.interfaces = parsed, interfaces
        self.signatures = dict(PRIMITIVES)
        self.declarations = {}
        self.used_calls = {}

    def source(self, node):
        return {"sha256": self.parsed["digest"], **node["source"]}

    def operation(self, node, op, type_, effects=(), charges=None, **fields):
        check_type(type_)
        return {"op": op, "type": type_, "effects": sorted(effects),
                "charges": charge() if charges is None else charges,
                **fields, "source": self.source(node)}

    def expression(self, node, env, calls):
        kind = node["kind"]
        types = {"int": "Int", "rat": "Rat", "q5": "Q5", "text": "Text"}
        if kind in types:
            return self.operation(node, "literal." + kind, types[kind], value=node["value"])
        if kind == "approx":
            return self.operation(node, "literal.approx", "Approx<P" + node["value"]["precision_bits"] + ">",
                                  value=node["value"])
        if kind in {"word", "menu"}:
            type_ = "Word<Full,Retained>" if kind == "word" else "Word<Jumble,MenuA>"
            return self.operation(node, "literal.word", type_, letters=node["value"],
                                  expanded_length=decimal(len(node["value"])))
        if kind == "ref":
            name = node["name"]
            require(name in env, "unbound name: " + name)
            value = env[name]
            extra = {"expanded_length": value["expanded_length"]} if "expanded_length" in value else {}
            return self.operation(node, "ref", value["type"], charges=charge(steps="0"), name=name, **extra)
        if kind == "match":
            subject = self.expression(node["subject"], env, calls)
            head, args = split_type(subject["type"])
            require(head == "Verdict", "match requires a verdict, never an approximate boolean")
            tags = [arm["tag"] for arm in node["arms"]]
            require(len(tags) == 3 and set(tags) == {"yes", "no", "unknown"}, "verdict match must cover yes, no, unknown once")
            arms = []
            for arm in node["arms"]:
                require(arm["binder"] not in env, "case binder shadows a name")
                payload = "Text" if arm["tag"] == "unknown" else args[0]
                arm_env = {**env, arm["binder"]: {"type": payload}}
                body = self.expression(arm["body"], arm_env, calls)
                arms.append({"tag": arm["tag"], "binder": {"name": arm["binder"], "type": payload}, "body": body})
            require(len({arm["body"]["type"] for arm in arms}) == 1, "case result types differ")
            return self.operation(node, "match.verdict", arms[0]["body"]["type"],
                                  union(subject["effects"], *(arm["body"]["effects"] for arm in arms)),
                                  subject=subject, arms=arms)
        require(kind == "call", "recovery holes and unknown expressions cannot lower")
        args = [self.expression(arg, env, calls) for arg in node["args"]]
        name = node["name"]
        extra = {}
        if name in WORD_OPERATIONS:
            result, extra = self.word_call(name, args)
            signature = {"result": result, "effects": [], "charges": charge()}
        else:
            require(name in self.signatures, "unknown function or forbidden operation: " + name)
            signature = self.signatures[name]
            require([arg["type"] for arg in args] == signature["params"], "argument types differ for " + name)
            calls.add(name)
        return self.operation(node, "call", signature["result"],
                              union(signature["effects"], *(arg["effects"] for arg in args)),
                              signature["charges"], target=name, args=args, **extra)

    def word_call(self, name, args):
        require(len(args) == (2 if name in {"repeat", "join"} else 1), "wrong word-operation arity")
        head, indices = split_type(args[0]["type"])
        require(head in {"Word", "CompressedWord"}, "word operations cannot consume equality keys")
        suffix = "<" + ",".join(indices) + ">"
        if name == "word_key":
            require(head == "Word", "fixture key input must be literal")
            return "WordKey" + suffix, {}
        if name == "word_length":
            unit = "Generators" if indices[0] == "Full" else "MenuLetters"
            return "Cost<" + indices[0] + "," + unit + ">", {}
        if name == "repeat":
            require(args[1]["type"] == "Int" and args[1]["op"] == "literal.int", "fixture repeat count must be an exact literal")
            count = integer(args[1]["value"])
            require(count >= 0, "negative repeat count")
            require("expanded_length" in args[0], "fixture compression needs a known length")
            return "CompressedWord" + suffix, {"expanded_length": decimal(integer(args[0]["expanded_length"]) * count)}
        require(args[0]["type"] == args[1]["type"] and head == "CompressedWord", "join requires equal compressed alphabets")
        require(all("expanded_length" in arg for arg in args), "fixture join needs known lengths")
        return "CompressedWord" + suffix, {"expanded_length": decimal(sum(integer(arg["expanded_length"]) for arg in args))}

    def prepare(self):
        parsed = self.parsed
        require(not parsed["diagnostics"] and not parsed["holes"], "broken drafts cannot lower")
        require(parsed["edition"] == EDITION and parsed["module"] is not None, "missing or unsupported edition/module")
        check_effects(parsed["requires"])
        aliases = set()
        for imported in parsed["imports"]:
            alias, digest = imported["name"], imported["digest"]
            require(alias not in aliases, "duplicate module alias")
            aliases.add(alias)
            require(re.fullmatch("[0-9a-f]{64}", digest) is not None and digest in self.interfaces, "dependency interface is not pinned")
            interface = self.interfaces[digest]
            require(interface["ir_version"] == IR_VERSION, "unsupported interface IR")
            for export, signature in interface["exports"].items():
                name = alias + "_" + export
                require(name not in self.signatures, "duplicate imported name")
                for type_ in [*signature["params"], signature["result"]]:
                    check_type(type_)
                check_effects(signature["effects"])
                check_charges(signature["charges"])
                self.signatures[name] = signature
        for declaration in parsed["declarations"]:
            name = declaration["name"]
            require(name not in self.signatures and name not in self.declarations and name not in WORD_OPERATIONS,
                    "duplicate declaration: " + name)
            self.declarations[name] = declaration
            params = declaration["params"]
            require(len({p["name"] for p in params}) == len(params), "duplicate parameter")
            for type_ in [*(p["type"] for p in params), declaration["result"]]:
                check_type(type_)
            effects = check_effects(declaration["effects"])
            require(set(effects) <= set(parsed["requires"]), "module lacks a required capability")
            check_charges(declaration["charges"])
            if declaration["kind"] == "function":
                self.signatures[name] = {"params": [p["type"] for p in params], "result": declaration["result"],
                                         "effects": effects, "charges": declaration["charges"]}

    def declaration(self, declaration):
        env = {p["name"]: {"type": p["type"]} for p in declaration["params"]}
        bindings, effects, calls = [], set(), set()
        for binding in declaration["bindings"]:
            require(binding["name"] not in env, "duplicate binding")
            value = self.expression(binding["value"], env, calls)
            bindings.append({"name": binding["name"], "type": value["type"], "value": value})
            env[binding["name"]] = value
            effects.update(value["effects"])
        body = self.expression(declaration["body"], env, calls)
        effects.update(body["effects"])
        require(body["type"] == declaration["result"], "return type differs: " + declaration["name"])
        declared_effects = check_effects(declaration["effects"])
        require(effects <= set(declared_effects), "annotation erases a callee effect")
        kind, metadata = declaration["kind"], declaration["metadata"]
        if kind == "view":
            require(not set(declared_effects) & {"workspace", "preview", "file", "random", "clock"}, "effect forbidden in a view")
            require(split_type(body["type"])[0] in {"View", "DisplayPlan"}, "view needs display output")
        if kind == "command":
            require(metadata["kind"] in {"query", "workspace", "view", "file"}, "unsupported or committing command")
            allowed = {"query": {"snapshot", "preview", "budget"}, "workspace": {"snapshot", "workspace", "budget"},
                       "view": {"snapshot", "view", "budget"}, "file": {"file", "snapshot", "budget"}}
            require(set(declared_effects) <= allowed[metadata["kind"]], "command kind cannot grant these effects")
            require(metadata["contexts"] and len(set(metadata["contexts"])) == len(metadata["contexts"]), "invalid command contexts")
            require(re.fullmatch(r"[a-z][a-z0-9_]*(?:\.[a-z][a-z0-9_]*)+", metadata["row"]) is not None, "invalid command row")
        self.used_calls[declaration["name"]] = calls
        return {"kind": kind, "name": declaration["name"], "params": declaration["params"],
                "result": declaration["result"], "declared_effects": declared_effects, "effects": sorted(effects),
                "charges": declaration["charges"], "metadata": metadata, "bindings": bindings,
                "body": body, "source": self.source(declaration)}

    def requests(self, name, seen):
        if name in seen:
            return set()
        if name in PRIMITIVES:
            return {name} if PRIMITIVES[name]["effects"] else set()
        seen.add(name)
        result = set()
        for callee in self.used_calls.get(name, ()):
            result.update(self.requests(callee, seen))
        return result

    def run(self):
        self.prepare()
        declarations = [self.declaration(d) for d in self.parsed["declarations"]]
        for declaration in declarations:
            if declaration["kind"] == "command":
                require(self.requests(declaration["name"], set()) <= set(declaration["metadata"]["permits"]),
                        "command does not declare every transitive kernel row")
        return {"ir_version": IR_VERSION, "edition": self.parsed["edition"],
                "module": self.parsed["module"], "requires": sorted(self.parsed["requires"]),
                "imports": [{"name": x["name"], "digest": x["digest"], "source": self.source(x)} for x in self.parsed["imports"]],
                "declarations": declarations, "source": {"sha256": self.parsed["digest"]}}


def lower(parsed, interfaces=None):
    return Lowerer(parsed, {} if interfaces is None else interfaces).run()
