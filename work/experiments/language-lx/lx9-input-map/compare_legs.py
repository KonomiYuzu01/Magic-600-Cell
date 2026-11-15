"""Compare a later Linux pure-Python run against the frozen Windows evidence."""
import sys
sys.dont_write_bytecode = True

import gzip
import json

from common import (HERE, differences, digest, json_bytes, read_requests, read_results,
                    request_bytes, rotation_key, verify_candidate, verify_manifest)

PRIMARY = ("reference", "pure", "rust")


def load_windows():
    verify_manifest()
    verify_candidate()
    summary = json.loads((HERE / "WINDOWS.json").read_bytes())
    assert summary["input_manifest_sha256"] == digest((HERE / "MANIFEST.json").read_bytes())
    assert summary["candidate_manifest_sha256"] == digest((HERE / "CANDIDATE.json").read_bytes())
    rows = {}
    for step in (1, 2):
        rows[step] = {}
        for leg in PRIMARY:
            name = f"windows-{leg}-step{step}.tsv.gz"
            archive = summary["archives"][name]
            data = (HERE / name).read_bytes()
            assert digest(data) == archive["sha256"]
            raw = gzip.decompress(data)
            assert digest(raw) == archive["raw_sha256"]
            rows[step][leg] = read_results(raw)
            assert set(rows[step][leg]) == {r[0] for r in read_requests(HERE / f"step{step}.tsv")}
    return summary, rows


def linux_inclusion(windows, linux_step1, manifest):
    ids = set(manifest["step2_included_step1_ids"])
    for leg in PRIMARY:
        diff = differences(windows[1][leg], linux_step1)
        ids.update(diff["target"])
        ids.update(diff["rotation"])
    return sorted(ids)


def main():
    windows_summary, windows = load_windows()
    manifest = verify_manifest()
    metadata = json.loads((HERE / "LINUX.json").read_bytes())
    linux_manifest = json.loads((HERE / "LINUX-MANIFEST.json").read_bytes())
    assert metadata["platform"] == "linux"
    assert metadata["input_manifest_sha256"] == windows_summary["input_manifest_sha256"]
    assert metadata["candidate_manifest_sha256"] == windows_summary["candidate_manifest_sha256"]
    assert metadata["linux_manifest_sha256"] == digest((HERE / "LINUX-MANIFEST.json").read_bytes())
    linux = {}
    for step in (1, 2):
        packed = (HERE / f"linux-pure-step{step}.tsv.gz").read_bytes()
        assert digest(packed) == metadata["results"][str(step)]["sha256"]
        raw = gzip.decompress(packed)
        assert digest(raw) == metadata["results"][str(step)]["raw_sha256"]
        linux[step] = read_results(raw)
    ids = linux_inclusion(windows, linux[1], manifest)
    assert ids == linux_manifest["step2_included_step1_ids"]
    step1 = {r[0]: r for r in read_requests(HERE / "step1.tsv")}
    expected = read_requests(HERE / "bisectors.tsv") + [step1[k] for k in ids]
    data = request_bytes(expected)
    assert data == (HERE / "linux-step2.tsv").read_bytes()
    assert digest(data) == linux_manifest["step2_sha256"]
    assert set(linux[2]) == {r[0] for r in expected}
    for name in ids:
        assert linux[1][name] == linux[2][name]
        for leg in PRIMARY:
            windows[2][leg][name] = windows[1][leg][name]
    comparison = {"schema": "lx9-cross-platform-v1", "decision": "open", "decider": "owner",
                  "evidence_class": "synthetic", "linux_python": metadata["python"],
                  "windows_summary_sha256": digest((HERE / "WINDOWS.json").read_bytes()),
                  "linux_metadata_sha256": digest((HERE / "LINUX.json").read_bytes()),
                  "new_step1_inclusions": sorted(set(ids) - set(manifest["step2_included_step1_ids"])),
                  "steps": {}}
    lines = ["step\tid\twindows_leg\ttarget_bits_differ\trotation_differs\tparameters_differ\n"]
    table, examples = [], []
    for step in (1, 2):
        comparison["steps"][str(step)] = {}
        for leg in PRIMARY:
            diff = differences(windows[step][leg], linux[step])
            assert set(diff["parameters"]) <= set(diff["target"]), "equal targets but inconsistent exact chooser"
            counts = {k: len(v) for k, v in diff.items()}
            comparison["steps"][str(step)][leg] = counts
            table.append(f"| {step} | {len(linux[step])} | {leg}/Linux | {counts['target']} | {counts['rotation']} |")
            sets = {k: set(v) for k, v in diff.items()}
            for name in sorted(set.union(*sets.values())):
                lines.append(f"{step}\t{name}\t{leg}\t{int(name in sets['target'])}\t"
                             f"{int(name in sets['rotation'])}\t{int(name in sets['parameters'])}\n")
            for name in diff["rotation"][:3]:
                examples.append(f"- Step {step}, {name}, {leg}/Linux: "
                                f"`{rotation_key(windows[step][leg][name][1])}` / "
                                f"`{rotation_key(linux[step][name][1])}`.")
    (HERE / "linux-differences.tsv").write_bytes("".join(lines).encode())
    comparison["difference_table_sha256"] = digest((HERE / "linux-differences.tsv").read_bytes())
    (HERE / "COMPARISON.json").write_bytes(json_bytes(comparison))
    note = f"""# LX-9 Linux follow-up

Decision: **open**; decider: owner. Evidence: synthetic. Linux CPython {metadata['python']}.
Candidate and input identities are checked against the Windows manifests.

| Step | Requests | Pair | Target-bit differences | Exact rotation differences |
| --- | --- | --- | --- | --- |
{chr(10).join(table)}

Additional step-1 requests carried into step 2: {len(comparison['new_step1_inclusions'])}.
Every difference is retained in `linux-differences.tsv` and the result tables.
No difference is a finite-sample result only; any different rotation triggers
unaccepted amendment evidence. This cannot show agreement for every request,
input latency, or an approved A1 change. Menu-mode drags remain untested.
"""
    (HERE / "LINUX-COMPARISON.md").write_bytes(note.encode())
    if examples:
        path = HERE / "AMENDMENT.md"
        current = path.read_text(encoding="utf-8") if path.exists() else "# LX-9 amendment evidence — unaccepted\n"
        current = current.split("\n## Linux follow-up\n")[0]
        current += "\n## Linux follow-up\n\nDecision: **open**, owner after EN1's Astra check.\n\n"
        current += "\n".join(examples) + "\n\n"
        current += ("All evidence and input/source digests are in COMPARISON.json, LINUX.json, "
                    "LINUX-MANIFEST.json and linux-differences.tsv. Fresh-request agreement is stopped. "
                    "The unaccepted KN3.4/RTE.66 path is a dyadic Cayley request read exactly and "
                    "60 menu candidates compared exactly in Q(sqrt(5)), then EN1 Astra review and "
                    "the owner. No contract, format or product change is approved or implemented.\n")
        path.write_bytes(current.encode())
    print(note)


if __name__ == "__main__":
    main()
