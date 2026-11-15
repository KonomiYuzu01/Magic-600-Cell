"""Offline LX-8 source-equivalence acceptance. Read-only; creates no artifacts."""
import sys

# Set before importing local modules, including the three candidate parsers.
sys.dont_write_bytecode = True

import ast
from collections import Counter
import copy
import hashlib
import importlib.util
import json
from pathlib import Path

from lower import lower
from semantics import FAMILIES, LoweringError, check_charges, check_type, semantic, split_type
from syntax import SYNTAX_CODES


ROOT = Path(__file__).resolve().parent
CANDIDATES = ("candidate-a", "candidate-b", "candidate-c")


class CheckFailure(Exception):
    pass


def need(condition, message):
    if not condition:
        raise CheckFailure(message)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def no_float(text):
    raise CheckFailure("a fixture contains a non-exact JSON number: " + text)


def read_json(data):
    return json.loads(data, parse_float=no_float, parse_constant=no_float)


def file_bytes(path):
    relative = Path(path)
    need(not relative.is_absolute() and ".." not in relative.parts, "nonlocal manifest path")
    target = ROOT / relative
    need(target.resolve().is_relative_to(ROOT), "manifest path escaped experiment")
    return target.read_bytes()


def frozen(entry):
    data = file_bytes(entry["path"])
    need(sha(data) == entry["sha256"], "frozen input changed: " + entry["path"])
    return data


def load_parser(candidate):
    spec = importlib.util.spec_from_file_location(candidate.replace("-", "_"), ROOT / candidate / "parser.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.parse


def walk(value):
    if isinstance(value, dict):
        yield value
        for child in value.values():
            yield from walk(child)
    elif isinstance(value, list):
        for child in value:
            yield from walk(child)
    else:
        need(not isinstance(value, float), "host float in parsed or lowered values")


def locations(value, data):
    for node in walk(value):
        location = node.get("source")
        if location is None:
            continue
        if "sha256" in location:
            need(location["sha256"] == sha(data), "wrong source digest")
        if "start" not in location:
            continue
        start, end = location["start"], location["end"]
        need(0 <= start <= end <= len(data), "source span outside input")
        prefix = data[:start].decode("utf-8")
        need(location["line"] == prefix.count("\n") + 1, "wrong source line")
        need(location["column"] == len(prefix.rsplit("\n", 1)[-1]) + 1, "wrong Unicode source column")


def difference(a, b, path="IR"):
    if type(a) is not type(b):
        return path + " (kind)"
    if isinstance(a, dict):
        if a.keys() != b.keys():
            return path + " (fields)"
        for key in a:
            if a[key] != b[key]:
                return difference(a[key], b[key], path + "." + key)
    elif isinstance(a, list):
        if len(a) != len(b):
            return path + " (length)"
        for index, (left, right) in enumerate(zip(a, b)):
            if left != right:
                return difference(left, right, path + f"[{index}]")
    return path


def refuses(parsed, interfaces, label):
    try:
        lower(parsed, interfaces)
    except LoweringError:
        return
    raise CheckFailure(label + ": invalid input lowered")


def collect_types(ir):
    found = set()

    def collect(text):
        head, args = split_type(text)
        found.add(head)
        for arg in args:
            if "<" in arg:
                collect(arg)

    for node in walk(ir):
        for key in ("type", "result"):
            if key in node:
                check_type(node[key])
                collect(node[key])
        if "charges" in node:
            check_charges(node["charges"])
        if "op" in node:
            need(node["effects"] == sorted(set(node["effects"])), "noncanonical effect set")
    return found


def verify_manifest(manifest):
    need(manifest["decision"] == "open" and manifest["decider"] == "owner", "decision is not open for owner")
    required = {"syntax.py", "semantics.py", "lower.py", "check.py", "fixtures/build.py", "ir.md", "CARD.md", "PROTOCOL-NEEDS.md"}
    required |= {candidate + "/" + leaf for candidate in CANDIDATES for leaf in ("parser.py", "DESCRIPTION.md")}
    need(set(manifest["candidate_files"]) == required, "candidate freeze is incomplete")
    for path, expected in manifest["candidate_files"].items():
        need(sha(file_bytes(path)) == expected, "frozen candidate changed: " + path)
    canonical = lambda value: json.dumps(value, sort_keys=True, separators=(",", ":")).encode()
    need(sha(canonical(manifest["candidate_files"])) == manifest["candidate_sha256"], "candidate aggregate differs")
    entries = [manifest["interface"], *manifest["recovery"], *manifest["byte_recovery"]]
    for row in manifest["programs"]:
        need(set(row["sources"]) == set(CANDIDATES), row["id"] + ": missing candidate")
        entries += [row["expected"], *row["sources"].values()]
    inputs = {item["path"]: item["sha256"] for item in entries}
    need(len(inputs) == len(entries), "duplicate frozen input path")
    need(sha(canonical(inputs)) == manifest["input_sha256"], "input aggregate differs")
    # Every committed artifact is UTF-8/LF text, including hex-encoded negative bytes.
    expected_paths = set(inputs) | required | {"fixtures/MANIFEST.json", "RESULT.md"}
    actual_paths = {p.relative_to(ROOT).as_posix() for p in ROOT.rglob("*") if p.is_file()}
    need(actual_paths <= expected_paths, "unexpected artifacts: " + ", ".join(sorted(actual_paths - expected_paths)))
    need(not (ROOT / "build").exists(), "unexpected build directory")
    for path in actual_paths:
        data = file_bytes(path)
        need(b"\r" not in data and not data.startswith(b"\xef\xbb\xbf"), "text is not pinned LF: " + path)
        data.decode("utf-8", errors="strict")
    for path in ("syntax.py", "lower.py", "semantics.py"):
        tree = ast.parse(file_bytes(path))
        for node in ast.walk(tree):
            need(not (isinstance(node, ast.Constant) and isinstance(node.value, float)), "float constant on lowering path")
            need(not (isinstance(node, ast.Call) and isinstance(node.func, ast.Name) and node.func.id == "float"), "float conversion on lowering path")


def verify_recovery(row, parse, data, interfaces):
    first, second = parse(data), parse(data)
    need(first == second, row["id"] + ": unstable recovery")
    codes = [d["code"] for d in first["diagnostics"]]
    need(codes == row["codes"] and set(codes) <= SYNTAX_CODES, row["candidate"] + "/" + row["id"] + ": wrong diagnostic codes " + repr(codes))
    need([d["name"] for d in first["declarations"]] == row["recovered"], row["id"] + ": lost independent declaration")
    for declaration in first["declarations"]:
        need(declaration["body"]["kind"] == "int" and declaration["body"]["value"] == "99", "recovered declaration meaning changed")
    locations(first, data)
    refuses(first, interfaces, row["id"])
    # A changed explanation must neither change the stable code nor make a hole executable.
    changed = copy.deepcopy(first)
    for diagnostic in changed["diagnostics"]:
        diagnostic["message"] = "Alternate wording for this same diagnostic."
    refuses(changed, interfaces, row["id"] + " wording")
    prefix = "# shifted λ\n".encode("utf-8")
    shifted = parse(prefix + data)
    need([d["code"] for d in shifted["diagnostics"]] == codes, row["id"] + ": location affected code")
    for before, after in zip(first["diagnostics"], shifted["diagnostics"]):
        old, new = before["source"], after["source"]
        need(new["start"] == old["start"] + len(prefix) and new["end"] == old["end"] + len(prefix)
             and new["line"] == old["line"] + 1 and new["column"] == old["column"], "shifted diagnostic lost its location")
    if row["id"] == "zero-denominator":
        span = first["diagnostics"][0]["source"]
        need(data[span["start"]:span["end"]] == b"1/0", "literal diagnostic points to wrong token")
        hole = first["holes"][0]["source"]
        damaged = data[hole["start"]:hole["end"]]
        twice = data[:hole["start"]] + damaged + damaged.replace(b"broken", b"also_broken") + data[hole["end"]:]
        recovered = parse(twice)
        need([d["code"] for d in recovered["diagnostics"]] == codes * 2, "multiple independent errors not recovered")
        need([d["name"] for d in recovered["declarations"]] == ["recovered"], "multiple errors lost suffix")
        refuses(recovered, interfaces, "two recovery holes")
    return set(codes)


def mutation_checks(programs, parsers, interfaces, goldens):
    # Valid semantic edits must differ; invalid edits must be refused. None may disappear.
    cases = [
        ("01-exact-values", b"q5(-3/2, 5/7)", b"q5(-3/2, 6/7)", "different"),
        ("02-explicit-approx", b'~24"1.25e-1"', b'~53"1.25e-1"', "different"),
        ("03-identities", b"Piece<Full,Labels>", b"Piece<Jumble,Labels>", "refuse"),
        ("03-identities", b"Piece<Full,Labels>", b"UserActionToken<Full>", "refuse"),
        ("05-literal-words", b"word[+1, -1, +2, +1200, -1200, +1]", b"word[-1, +1, +2, +1200, -1200, +1]", "different"),
        ("07-verdict-match", b"unknown(reason)", b"no(reason)", "refuse"),
        ("08-effect-functions", b"[snapshot]", b"[]", "refuse"),
        ("09-modules", b"Cost<Full,Generators>", b"Cost<Full,UIActions>", "refuse"),
        ("10-workspace-command", b"[2, 9, 0, 1]", b"[3, 9, 0, 1]", "different"),
        ("11-preview-command", b"Revision<Full,Off>", b"Revision<Full,On>", "refuse"),
        ("17-granted-effects", b"[file, random]", b"[]", "refuse"),
    ]
    count = 0
    for candidate, parse in parsers.items():
        for key, old, new, outcome in cases:
            original = frozen(programs[key]["sources"][candidate])
            need(old in original, "mutation did not apply: " + key)
            changed = parse(original.replace(old, new))
            need(not changed["diagnostics"], "semantic mutation was only a syntax failure: " + key)
            if outcome == "refuse":
                refuses(changed, interfaces, key + " mutation")
            else:
                need(semantic(lower(changed, interfaces)) != goldens[key], "semantic edit erased: " + key)
            count += 1
        parsed = parse(frozen(programs["05-literal-words"]["sources"][candidate]))
        # Replace a word operand by its typed equality key; signatures must not accept it.
        key_node = parsed["declarations"][0]["bindings"][0]["value"]
        parsed["declarations"][0]["body"] = {"kind": "call", "name": "word_length", "args": [key_node], "source": key_node["source"]}
        refuses(parsed, interfaces, "equality key as word")
        count += 1
        parsed = parse(frozen(programs["12-revision-view"]["sources"][candidate]))
        parsed["requires"].append("workspace")
        parsed["declarations"][0]["effects"].append("workspace")
        refuses(parsed, interfaces, "workspace-capable view")
        count += 1
        parsed = parse(frozen(programs["09-modules"]["sources"][candidate]))
        parsed["imports"][0]["digest"] = "0" * 64
        refuses(parsed, interfaces, "unavailable dependency")
        count += 1
        parsed = parse(frozen(programs["10-workspace-command"]["sources"][candidate]))
        parsed["declarations"][0]["metadata"]["permits"] = []
        refuses(parsed, interfaces, "missing command row permission")
        count += 1
    return count


def main():
    manifest = read_json(file_bytes("fixtures/MANIFEST.json"))
    verify_manifest(manifest)
    parsers = {name: load_parser(name) for name in CANDIDATES}
    interface = manifest["interface"]
    interfaces = {interface["sha256"]: read_json(frozen(interface))}
    types, features, effects, goldens, programs = set(), set(), set(), {}, {}
    parsed_count = 0
    for row in manifest["programs"]:
        need(row["id"] not in programs, "duplicate program ID")
        programs[row["id"]] = row
        features.update(row["covers"])
        golden = read_json(frozen(row["expected"]))
        goldens[row["id"]] = golden
        types.update(collect_types(golden))
        comparisons = []
        for candidate, parse in parsers.items():
            data = frozen(row["sources"][candidate])
            parsed = parse(data)
            need(not parsed["diagnostics"], candidate + "/" + row["id"] + ": valid fixture has diagnostics " + repr(parsed["diagnostics"]))
            ir = lower(parsed, interfaces)
            locations(parsed, data)
            locations(ir, data)
            actual = semantic(ir)
            need(actual == golden, candidate + "/" + row["id"] + ": expected mismatch at " + difference(actual, golden))
            comparisons.append(actual)
            parsed_count += 1
        need(comparisons[0] == comparisons[1] == comparisons[2], row["id"] + ": cross-candidate mismatch")
        for declaration in golden["declarations"]:
            effects.update(declaration["effects"])
    need(len(programs) >= 12, "fewer than twelve programs")
    for family, members in FAMILIES.items():
        need(set(members.split()) <= types, "type family incomplete: " + family + " " + repr(set(members.split()) - types))
    required = {"exact-integer", "exact-rational", "exact-q5", "explicit-approximation", "verdict-matching", "literal-order", "compressed-words", "modules", "commands", "views", "effect-functions"}
    need(required <= features, "LG1.8 coverage missing")
    need({"snapshot", "view", "workspace", "preview", "file", "random", "budget"} <= effects, "effect family missing")
    codes, counts = set(), Counter()
    for row in manifest["recovery"] + manifest["byte_recovery"]:
        data = frozen(row)
        if row in manifest["byte_recovery"]:
            data = bytes.fromhex(read_json(data)["source_hex"])
        codes.update(verify_recovery(row, parsers[row["candidate"]], data, interfaces))
        counts[row["candidate"]] += 1
    need(set(counts) == set(CANDIDATES) and all(n >= 10 for n in counts.values()), "recovery corpus too small")
    need(codes == SYNTAX_CODES, "registry syntax-code coverage incomplete")
    mutations = mutation_checks(programs, parsers, interfaces, goldens)
    print(f"PASS: {len(programs)} programs x 3 candidates = {parsed_count} parses and golden/cross-candidate IR comparisons.")
    print(f"PASS: {sum(counts.values())} broken drafts ({dict(counts)}); all 6 LG1:syntax.* codes; recovery locations and non-lowering.")
    print(f"PASS: {len(FAMILIES)} domain families; exact literals; effects; word order/length; {mutations} semantic mutation controls.")
    print("Candidate SHA-256: " + manifest["candidate_sha256"])
    print("Input SHA-256: " + manifest["input_sha256"])
    print("Evidence: source only. Human run: not run. Decision: open; decider: owner.")


if __name__ == "__main__":
    try:
        main()
    except (CheckFailure, LoweringError, ValueError, OSError, KeyError) as error:
        print("FAIL: " + str(error))
        sys.exit(1)
