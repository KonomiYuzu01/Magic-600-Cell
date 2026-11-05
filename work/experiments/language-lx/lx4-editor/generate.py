"""Seeded LX-8 stand-in modules; regenerate from the three LP3 tools when frozen."""
import sys
sys.dont_write_bytecode = True

import argparse
import json
import random

from common import ROOT, LX8, BUILD, PARSERS, canonical, encoded, put, sha, scratch
from syntax import lex

SEED = 2026110547
SIZES = {"S": 200, "M": 1000, "L": 5000, "W": 400}
REQUIRES = "budget, file, preview, random, snapshot, view, workspace"


def seed_fragments():
    manifest = json.loads((LX8 / "fixtures/MANIFEST.json").read_bytes())
    fragments = []
    for row in manifest["programs"]:
        data = (LX8 / row["sources"]["candidate-a"]["path"]).read_bytes()
        parsed = PARSERS["a"](data)
        assert not parsed["diagnostics"]
        text = "\n".join(data[d["source"]["start"]:d["source"]["end"]].decode("utf-8")
                         for d in parsed["declarations"]) + "\n"
        fragments.append((row["id"], text, [d["name"] for d in parsed["declarations"]]))
    return fragments, manifest["interface"]


def rename(text, names, prefix):
    # Token locations keep strings, comments, parameter names and metadata intact.
    mapping = {name: prefix + name for name in names}
    data = text.encode("utf-8")
    for token in reversed(lex(text, False)):
        if token.kind == "ID" and token.text in mapping:
            data = data[:token.start] + mapping[token.text].encode() + data[token.end:]
    return data.decode("utf-8")


def export_interface(result="Int"):
    return encoded({"ir_version": "lx8-ir-0", "exports": {"exported": {
        "params": [], "result": result, "effects": [],
        "charges": {"steps": "1", "kernel": "0", "memory": "0", "output": "0"}}}})


def graph():
    rng = random.Random(SEED)
    edges = {}
    for i in range(50):
        if i == 0:
            dependencies = []
        elif i < 10:
            dependencies = [0]  # shared base utility
        elif i < 30:
            dependencies = sorted(rng.sample(range(10), 3))
        elif i < 45:
            dependencies = sorted(rng.sample(range(10, 30), 3))
        else:
            dependencies = sorted(rng.sample(range(30, 45), 3))
        edges[f"W{i:02}"] = [f"W{d:02}" for d in dependencies]
    return edges


def generate():
    fragments, support = seed_fragments()
    interfaces = {support["sha256"]: (LX8 / support["path"]).read_bytes()}
    public = export_interface()
    public_digest = sha(public)
    interfaces[public_digest] = public
    edges = graph()
    modules, entries = {}, []
    for size in SIZES:
        for index in range(50 if size == "W" else 1):
            name = f"W{index:02}" if size == "W" else size
            dependencies = edges.get(name, [])
            text = f'edition "lx8-0";\nmodule {name} requires [{REQUIRES}];\n'
            text += f'import helpers "{support["sha256"]}";\n'
            text += "".join(f'import {d} "{public_digest}";\n' for d in dependencies)
            text += "fn exported() -> Int effects [] charge [1, 0, 0, 0] {\n    return 1;\n}\n"
            for d in dependencies:
                text += (f"fn use_{d}() -> Int effects [] charge [1, 0, 0, 0] {{\n"
                         f"    return {d}_exported();\n}}\n")
            rng = random.Random(SEED + ord(size) * 100 + index)
            order = list(range(len(fragments)))
            rng.shuffle(order)
            used, serial = [], 0
            while True:
                fixture, fragment, names = fragments[order[serial % len(order)]]
                part = rename(fragment, names, f"f{serial:04}_")
                if text.count("\n") + part.count("\n") > SIZES[size]:
                    break
                text += part
                used.append(fixture)
                serial += 1
            while text.count("\n") < SIZES[size]:
                text += "# reserved source line\n"
            data = text.encode("utf-8")
            modules[name] = data
            entries.append({"name": name, "size": size, "lines": data.count(b"\n"),
                            "bytes": len(data), "sha256": sha(data), "imports": dependencies,
                            "fragments": used})
    return modules, interfaces, entries


def write_build(modules):
    for name, data in modules.items():
        put(BUILD / "modules" / (name + ".lxa8"), data)


def freeze():
    from edit_script import make_script
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
    put(ROOT / "catalogue.json", clean(catalogue))
    modules, interfaces, entries = generate()
    edits = make_script(modules)
    put(ROOT / "EDITS.json", edits)
    put(ROOT / "sample.lxa8", modules["S"])
    # Pin experiment inputs, never moving architecture/decision documents.
    upstream = {p.relative_to(ROOT.parent).as_posix(): sha(p.read_bytes())
                for p in LX8.rglob("*") if p.is_file()}
    own = {p.name: sha(p.read_bytes()) for p in ROOT.iterdir()
           if p.is_file() and p.name not in {"MANIFEST.json", "RESULT.md", "SERIES.md", "timings.json"}}
    manifest = {"schema": "lx4-headless-0", "experiment": "E-2.4-L4", "decision": "open",
                "decider": "owner", "seed": SEED, "surface": "candidate-a", "sources": own,
                "upstream": upstream, "modules": entries,
                "interfaces": {key: sha(value) for key, value in interfaces.items()},
                "candidate_sha256": sha(canonical(own)), "input_sha256": sha(canonical(upstream)),
                "module_set_sha256": sha(canonical(entries)), "edit_sha256": sha(encoded(edits))}
    put(ROOT / "MANIFEST.json", manifest)
    print("Frozen 53 modules, 26,200 lines; seed", SEED)
    print("Candidate SHA-256:", manifest["candidate_sha256"])


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--freeze", action="store_true", help="deliberately create a new input/source identity")
    args = parser.parse_args()
    if args.freeze:
        freeze()
    else:
        with scratch():
            modules, _, entries = generate()
            write_build(modules)
            print(sha(canonical(entries)))
            print("Generated the module set under build/; removed it on exit.")
