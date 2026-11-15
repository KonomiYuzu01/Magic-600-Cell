"""LX-4 deterministic acceptance. No timing comparisons or surviving writes."""
import sys
sys.dont_write_bytecode = True

import json

from common import (ROOT, LX8, BUILD, PARSERS, canonical, compile_rust, load,
                    lower, put, python_front, rust_run, scratch, semantic, sha)
from edit_script import apply, dependents, make_script
from generate import generate, graph, export_interface, write_build


def need(ok, message):
    if not ok:
        raise ValueError(message)


def frozen():
    manifest = json.loads((ROOT / "MANIFEST.json").read_bytes())
    need(manifest["decision"] == "open" and manifest["decider"] == "owner", "decision changed")
    for path, digest in manifest["sources"].items():
        need(sha((ROOT / path).read_bytes()) == digest, "source changed: " + path)
    for path, digest in manifest["upstream"].items():
        need(sha((ROOT.parent / path).read_bytes()) == digest, "upstream changed: " + path)
    for field, value in (("candidate_sha256", manifest["sources"]), ("input_sha256", manifest["upstream"]),
                         ("module_set_sha256", manifest["modules"])):
        need(sha(canonical(value)) == manifest[field], "aggregate changed: " + field)
    need(sha((ROOT / "EDITS.json").read_bytes()) == manifest["edit_sha256"], "edit script changed")
    expected = set(manifest["sources"]) | {"MANIFEST.json", "RESULT.md", "SERIES.md", "timings.json"}
    for path in ROOT.rglob("*"):
        if path.is_file() and BUILD not in path.parents:
            need(path.relative_to(ROOT).as_posix() in expected, "unexpected artifact")
            data = path.read_bytes()
            need(b"\r" not in data and not data.startswith(b"\xef\xbb\xbf"), "non-LF artifact: " + path.name)
            data.decode("utf-8")
    import semantics
    catalogue = {name: getattr(semantics, name) for name in
                 ("IR_VERSION", "PRIMITIVES", "TYPE_SLOTS", "INDICES", "EFFECTS", "WORD_OPERATIONS")}
    def clean(value):
        if isinstance(value, dict):
            return {key: clean(item) for key, item in value.items()}
        if isinstance(value, set):
            return sorted(value)
        if isinstance(value, tuple):
            return list(value)
        return value
    need(json.loads((ROOT / "catalogue.json").read_bytes()) == clean(catalogue), "catalogue differs from LX-8")
    return manifest


def compare(exe, cases, interfaces):
    jobs, expected = [], []
    decoded = {key: json.loads(value) for key, value in interfaces.items()}
    for i, (label, data, surface) in enumerate(cases):
        path = BUILD / "cases" / f"{i:04}.src"
        put(path, data)
        jobs.append({"source": path.relative_to(ROOT).as_posix(), "surface": surface})
        expected.append(python_front(data, decoded, surface))
    actual = rust_run(exe, "check", jobs, interfaces)
    need(len(actual) == len(expected), "Rust result count differs")
    oracle = load("lx4_oracle_difference", LX8 / "check.py")
    for (label, _, _), py, rs in zip(cases, expected, actual):
        need(py == rs, label + ": Python/Rust differ at " + oracle.difference(py, rs))
    return expected


def corpus(exe):
    oracle = load("lx4_oracle", LX8 / "check.py")
    oracle.main()  # All 17 x 3 golden IRs, 48 broken drafts and 45 mutations.
    manifest = json.loads((LX8 / "fixtures/MANIFEST.json").read_bytes())
    support = manifest["interface"]
    interfaces = {support["sha256"]: (LX8 / support["path"]).read_bytes()}
    cases = []
    for row in manifest["programs"]:
        for candidate, source in row["sources"].items():
            cases.append((candidate + "/" + row["id"], (LX8 / source["path"]).read_bytes(), candidate[-1]))
    for row in manifest["recovery"] + manifest["byte_recovery"]:
        data = (LX8 / row["path"]).read_bytes()
        if row in manifest["byte_recovery"]:
            data = bytes.fromhex(json.loads(data)["source_hex"])
        label = row["candidate"] + "/" + row["id"]
        cases.extend([(label, data, row["candidate"][-1]),
                      (label + " shifted", "# shifted λ\n".encode() + data, row["candidate"][-1])])
    compare(exe, cases, interfaces)
    print("PASS: Rust/Python exact parsed trees, IR, diagnostic messages/spans and refusals: 51 programs, 48 drafts, 48 shifted drafts.")
    # Raw-source mutations exercise the checker independently of the parser's
    # recovery path, including exact values, effects, authority and imports.
    edits = [
        ("01-exact-values", b"q5(-3/2, 5/7)", b"q5(-3/2, 6/7)", False),
        ("02-explicit-approx", b'~24"1.25e-1"', b'~53"1.25e-1"', False),
        ("03-identities", b"Piece<Full,Labels>", b"Piece<Jumble,Labels>", True),
        ("03-identities", b"Piece<Full,Labels>", b"UserActionToken<Full>", True),
        ("05-literal-words", b"word[+1, -1, +2, +1200, -1200, +1]", b"word[-1, +1, +2, +1200, -1200, +1]", False),
        ("07-verdict-match", b"unknown(reason)", b"no(reason)", True),
        ("08-effect-functions", b"[snapshot]", b"[]", True),
        ("09-modules", b"Cost<Full,Generators>", b"Cost<Full,UIActions>", True),
        ("10-workspace-command", b"[2, 9, 0, 1]", b"[3, 9, 0, 1]", False),
        ("10-workspace-command", b"permits [save_word]", b"permits []", True),
        ("11-preview-command", b"Revision<Full,Off>", b"Revision<Full,On>", True),
        ("12-revision-view", b"[snapshot, view]", b"[snapshot, view, workspace]", True),
        ("17-granted-effects", b"[file, random]", b"[]", True),
        ("09-modules", support["sha256"].encode(), b"0" * 64, True),
    ]
    programs = {row["id"]: row for row in manifest["programs"]}
    mutations = []
    originals = []
    for name, old, new, refuses in edits:
        data = (LX8 / programs[name]["sources"]["candidate-a"]["path"]).read_bytes()
        need(old in data, "mutation did not apply: " + name)
        originals.append(data)
        mutations.append((name + " mutation", data.replace(old, new), "a"))
    results = compare(exe, mutations, interfaces)
    for result, original, (_, _, _, refuses) in zip(results, originals, edits):
        need(not result["parsed"]["diagnostics"], "semantic control became a syntax error")
        need((result["error"] is not None) == refuses, "semantic control outcome differs")
        if not refuses:
            before = python_front(original, {k: json.loads(v) for k, v in interfaces.items()})
            need(semantic(before["ir"]) != semantic(result["ir"]), "semantic edit disappeared")
    print("PASS: 14 additional cross-host source mutations (4 changed IRs, 10 semantic refusals).")


def dependent_inputs(modules, interfaces, script):
    row = next(r for r in script["edits"] if r["module"] == "W00" and r["kind"] == "signature")
    changed = apply(modules["W00"], row)
    new_interface = export_interface("Text")
    old_digest, new_digest = sha(export_interface()), sha(new_interface)
    updated = {**interfaces, new_digest: new_interface}
    # The changed module remains valid. Dependencies use its newly pinned
    # signature, never the old signature under a new source digest.
    ir = lower(PARSERS["a"](changed), {k: json.loads(v) for k, v in updated.items()})
    exported = next(d for d in ir["declarations"] if d["name"] == "exported")
    need(exported["result"] == "Text", "exported edit not checked")
    edges = graph()
    affected = dependents(edges, "W00")
    data = {}
    for name in affected:
        source = modules[name]
        if "W00" in edges[name]:
            old = f'import W00 "{old_digest}";'.encode()
            new = f'import W00 "{new_digest}";'.encode()
            need(old in source, "dependent import is not pinned")
            source = source.replace(old, new)
        data[name] = source
    return data, updated


def generated(exe, manifest):
    modules, interfaces, entries = generate()
    need(entries == manifest["modules"], "regenerated module digests/metadata differ")
    need({k: sha(v) for k, v in interfaces.items()} == manifest["interfaces"], "interface set differs")
    need(modules["S"] == (ROOT / "sample.lxa8").read_bytes(), "small sample differs")
    need(sum(e["lines"] for e in entries if e["size"] == "W") == 20000, "wrong W size")
    need(len([e for e in entries if e["size"] == "W"]) == 50, "wrong W count")
    need(len(set(entries[0]["fragments"])) == 17, "S omits a seed program")
    write_build(modules)
    for name, data in modules.items():
        need((BUILD / "modules" / (name + ".lxa8")).read_bytes() == data, "generated file differs")
    results = compare(exe, [(name, data, "a") for name, data in modules.items()], interfaces)
    need(all(r["error"] is None for r in results), "generated module is invalid")
    for result in results:
        exported = next(d for d in result["ir"]["declarations"] if d["name"] == "exported")
        signature = {"params": [p["type"] for p in exported["params"]], "result": exported["result"],
                     "effects": exported["declared_effects"], "charges": exported["charges"]}
        need(signature == json.loads(export_interface())["exports"]["exported"], "synthetic interface does not match its module")
    script = json.loads((ROOT / "EDITS.json").read_bytes())
    need(script == make_script(modules), "seeded edit replay differs")
    edits = [(r["module"] + "/" + r["kind"], apply(modules[r["module"]], r), "a") for r in script["edits"]]
    results = compare(exe, edits, interfaces)
    for row, result in zip(script["edits"], results):
        need((result["error"] is None) == (row["kind"] in {"body", "signature"}), "wrong edit outcome")
        if row["kind"] in {"completion", "syntax_break"}:
            need(result["parsed"]["holes"] and result["ir"] is None, "recovered edit became executable")
        try:
            apply(modules[row["module"]] + b"\n", row)
        except ValueError:
            pass
        else:
            raise ValueError("stale edit accepted")
    dependent, updated = dependent_inputs(modules, interfaces, script)
    results = compare(exe, [("dependent/" + k, v, "a") for k, v in dependent.items()], updated)
    direct = sum("W00" in graph()[name] for name in dependent)
    need(len(dependent) == 49 and sum(r["error"] is not None for r in results) == direct, "dependent invalidation did not check the new signature")
    print(f"PASS: 53 generated modules (26,200 lines), 16 edits, stale-edit refusals; 49 W dependents, {direct} expected type refusals.")


def main():
    manifest = frozen()
    with scratch():
        exe = compile_rust()
        corpus(exe)  # Equivalence comes before generated input work or measurement.
        generated(exe, manifest)
    print("PASS: build removed; experiment folder byte-identical. No timing comparison. Decision: open; decider: owner.")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, KeyError, AssertionError) as error:
        print("FAIL:", error)
        sys.exit(1)
