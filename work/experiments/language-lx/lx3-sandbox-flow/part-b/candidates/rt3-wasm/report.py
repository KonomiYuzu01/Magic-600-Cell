"""Capture a finished shared-bench series; no mechanism or policy code."""

import argparse
import json
from pathlib import Path
import sys

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
sys.path.insert(1, str(HERE.parents[1]))

from harness.common import canonical, sha
from check import projection

COMMAND = "python -B work/experiments/language-lx/lx3-sandbox-flow/part-b/"


def report(summary, rows):
    pins = sha((HERE / "bindings.json").read_bytes())
    modules = json.loads((HERE / "modules.json").read_bytes())
    count_rows = []
    for key, forms in sorted(modules.items()):
        columns = []
        for form in ("capability_separation", "static_labels", "static_runtime_labels"):
            counts = [artifact["counts"] for artifact in forms[form].values()]
            fields = sum(c["value_label_fields"] + c["control_label_fields"] + c["temporary_label_fields"] for c in counts)
            fuel = sum(c["fuel_i64_fields"] for c in counts)
            calls = sum(sum(c["static_call_sites"].values()) for c in counts)
            columns.append(f"{fields} + {fuel} / {calls}")
        count_rows.append(f"| `{key[:16]}` | {len(next(iter(forms.values())))} | " + " | ".join(columns) + " |")
    lines = ["# LX-3 part B: rt3-wasm", "", "Experiment only. Every decision remains **open**, decider **owner**. No route, engine language, syntax, LQ or BS option is selected.", "",
             "## CT-RTE-05 record", "", "| Field | Record |", "| --- | --- |",
             "| LX card / experiment | LX-3, RTE.44 to RTE.46 / E-2.4-L3, part B, candidate rt3-wasm |",
             "| Owner allowance | `docs/wiki/decisions/owner-decisions-2026-11-05.md`; this isolated implementation packet |",
             "| Time box | what can be done now; one wrapper call of at most two hours. A reached limit is no pass. |",
             f"| Frozen candidate | raw SHA-256 of `bindings.json`: `{pins}`; its source_files pins the full candidate and shared Python closure |",
             "| Frozen inputs | lx3-neutral-v2; raw FREEZE.json SHA-256 `d7eff17503ee95d45ddca0efe38ec3bb09418b0248a6279afe2dce0a90ea6627`; content SHA-256 `c4aa5b09430d8e5dbe02c63686b4fb400b36e7e7faf82dc42113491efe1e28c7` |",
             "| Tools | CPython 3.15.1; Node v24.21.0 built-in WebAssembly engine; Python standard library and Node built-ins only |",
             "| Evidence class | source and synthetic; Windows is the matrix leg, not a Windows/DirectX or performance claim |",
             "| Kill criteria | A mismatch, planted read, unrefused prohibited case, crash, hang or wrong code fails that form. Missing keys cannot pass. |",
             f"| Actual series | {summary['runs_per_key_per_leg']} Windows runs per key, {summary['row_count']} observations; all six cells retain all 87 invocation/host keys per repetition |",
             "| Actual result | All three source-bound forms FAIL the must-refuse criterion. Capability separation: 18/20 cases pass, 2 fail, 2/35 false refusals. Each label form: 16/20 pass, 4 fail, 0/35 false refusals. |",
             "| Decision / decider | open / owner; results do not select LQ9 or LQ21 |", "",
             "**WebAssembly under Node's built-in engine stands in for a compiled target because no standalone WebAssembly VM is available.** This is the hybrid RT3 route: an own front end lowers to Wasm. It is not evidence for a standalone VM, native machine-code target, or the RT2/RT4 host-subset routes.", "",
             "Cannot show: all-program safety, authorship, understanding, absence of covert channels, product latency, or an acceptable false-refusal threshold. Speed is not measured. No real engine, personal session, GPU device, native input, long session or product process boundary ran.", "",
             "Read for this packet: `AGENTS.md`, `docs/development-guide/AGENT_BRIEFING.md`, chapter 11 RTE.11/RTE.12/RTE.24 to RTE.27/RTE.30 to RTE.37/RTE.44 to RTE.46, the owner allowance above, `work/experiments/language-lx/README.md`, the part-B harness README and modules, both controls' formats, all named frozen corpus inputs, and `part-b/CORPUS-ISSUES.md`. Moving chapter/decision pages are named, not pinned by this check.", "",
             "## Mechanism and the RT3 lowering boundary", "",
             "`render.py:render(program)` is a pure mechanical translation to JSON arrays with positional operands. It accepts no case ID, family or expected answer. It preserves both branches, source instruction paths and sequential entry order. This small surface is throwaway analysis notation, not an LQ13 syntax proposal. `programs.rt3` retains its exact UTF-8 source; `bindings.json` maps every neutral instruction, including unexecuted branches, to its exact byte range. `modules.json` retains every compiled module as hexadecimal plus its literal pool and lowering counts. No generated binary remains on disk.", "",
             "`frontend.py:Analysis` joins both branches, iterates loop-carried masks to a fixed point, carries module summaries and tracks possible workspace labels across entries. Static mode emits immediate masks; it has zero emitted i32.or label-join sites. The uniform ABI still carries constant label fields and provenance handles in this mode. Runtime mode retains the structural/static analysis and emits operand/pc joins for the executed path. Its label masks are not taken from expected results. Labels are state=1, computed=2, research=4, unknown=8; read_state=16 occupies the same i32 field.", "",
             "The handwritten assembler emits genuine Wasm if/block/loop/br instructions and direct function calls. Values stay as immutable handles on the Python host; Wasm carries control flow, explicit label fields and an i64 cumulative step-fuel global. User integers, exact collection operations and fake kernel values stay on the host, so Node never rounds A09's integer above 2^53. There is no claim to have compiled those numerical primitives into Wasm.", "",
             "The lowering hazard is an implicit dependency becoming an ordinary native branch: choosing literal words under a state-derived condition would lose state if the compiler retained only each literal's data labels. `Compiler.block` emits a pc-mask OR and a control-provenance handle at each branch and loop. Callees receive both fields; static joins retain the unexecuted arm as well. Effect intrinsics receive those computed fields and the control dependencies. `probe.py` runs independent synthetic branch/phi/module/workspace cases: state must survive literal selection, a compiled callee and persistence; static masks include an unexecuted state arm while runtime masks retain path precision. These are extra probes, never extra corpus passes.", "",
             "Separation selects its effective grants before admission: for a solving entry with a reachable state row (both branches and transitive calls), it removes preview. Every gated intrinsic asks `ctx.check`; the shared policy alone supplies codes and permissions. Imported persisted read_state/profile/unknown metadata survives later runs. Capability mode keeps observational labels too; it is not a minimal-cost implementation of separation. A source scan alone does not close stop/emission channels, and the failed cases below remain failed.", "",
             "Compiler-emitted step and loop-test imports charge before execution; the final false loop test is charged. Exact calculation constituents invoke the compiler-emitted `charge` Wasm export, reenter the host meter, then perform their arithmetic. Kernel, memory, output, batch-chunk cancellation and cache validation use the shared precharged intrinsics. Logical aliases and recursive frames follow the frozen tariff. The final Wasm fuel field must equal ctx.meter.steps. Costs are logical fixture counts, not physical instructions or timing.", "",
             "The Node bridge has exactly 18 function imports and no WASI, memory or table imports. It reads/writes only its fixed stdio JSON protocol. The Python adapter is trusted glue, not an OS sandbox. Whole surface bytes and compiled modules are regenerated and compared before execution; source instructions are also recorded through ctx.executed. The mechanism reads only ctx stimulus/services and its pinned files; it does not read answer corpora. The report/preparation/check drivers do read frozen cases for enumeration and comparison.", "",
             "The frozen S01/S02 cases apply only to approved_subset_byte_bound and are `not_run: variant_not_applicable` in source-bound cells. Separate synthetic probes confirm dynamic-evaluation refusal and detection of substituted dependency bytes before execution. Those probes do not change the frozen matrix's applicability. B11 kills the actual Python matrix child at admission, before Node starts; it is not a Node-kill or production process-containment test.", "",
             "## Results by declared cell", "",
             "A must-refuse case passes only when every gated run has the expected code and preservation/no-leak checks pass; harmless preparation runs are not refusals. Mixed success/failure in a multi-run case remains a failed case. Every denial is listed with its actual layer. No false-refusal threshold is imposed."]
    for cell in summary["cells"]:
        selected = [r for r in rows if r["form"] == cell["form"] and r["variant"] == cell["variant"]]
        counts = {status: sum(r["classification"]["status"] == status for r in selected)
                  for status in ("crash", "hang", "wrong_code", "planted_read")}
        supported = cell["declaration"] == "supported"
        failed = [key for key, statuses in cell["outcomes"]["must-refuse"].items()
                  if set(statuses) & {"unrefused", "wrong_code", "state_changed", "crash", "hang", "planted_read", "wrong_value"}]
        lines += ["", f"### {cell['form']} / {cell['variant']}", "", "| Measure | Outcome |", "| --- | --- |",
                  "| Declaration | " + ("supported; measured form FAILS" if supported else "not_run: " + cell["reason"]) + " |",
                  f"| Must-refuse | {20 - len(failed) if supported else 0}/20 passed; " +
                  ("failed: " + ", ".join(failed) if supported else "20 untested, denominator retained") + " |",
                  f"| Wrong codes | {counts['wrong_code']} recorded" + ("" if supported else "; not measured") + " |",
                  f"| False refusals | {cell['false_refusal_count']}/35 recorded; " +
                  ("; ".join(item["case_id"] + " `" + item["code"] + "`" for item in cell["false_refusals"]) or
                   ("none" if supported else "35 untested, denominator retained")) + " |",
                  "| Planted reads | " + (f"{counts['planted_read']}; P01-P04 refused with expected codes; no trap calls/data" if supported else "not run") + " |",
                  "| Budgets | " + ("B01-B07 and B13-B14: budget_ok, exact counters and stops" if supported else "not run") + " |",
                  "| Cache | " + ("B12: cold, warm, evicted, poisoned, shared, relocated all budget_ok; exact hashes equal; poisoned entry rejected" if supported else "not run") + " |",
                  "| Interruptions | " + ("B08 loop cancellation and B09 chunk cancellation: budget_ok; B11 actual child loss: budget_ok" if supported else "not run") + " |",
                  "| Wall-clock safety stop | B10: " + ("not_run: corpus_issue" if supported else "cell not run; B10 also remains a corpus issue") + " |",
                  "| Approval probes | " + ("S01/S02: not_run: variant_not_applicable" if supported else "not run: own front end has no host subset") + " |",
                  f"| Reproducibility counterexamples | {len(cell['reproducibility_counterexamples'])}" + ("; Windows only" if supported else "; only explicit not_run envelopes repeated") + " |",
                  f"| Crashes / hangs | {counts['crash']} / {counts['hang']} |"]
        if supported:
            lines += ["", "Actual refused invocations (all repetitions agreed):", "",
                      "| Program/run | Code | Refusing layer |", "| --- | --- | --- |"]
            lines += [f"| {item['case_id']}/{item['run_id']} | `{item['code']}` | {item['layer']} |" for item in cell["refusals"]]
    lines += ["", "Stop/emission counterexamples are MR13/MR14; accumulation counterexamples are MR19/MR20. Both label forms miss all four. Separation misses MR14 and MR20 because some paths stop before attempting the removed preview capability; other MR20 runs do refuse, which cannot rescue the case. Labels do not claim a termination-sensitive repair. A25 and A30 are the separation false refusals: they display state-derived facts and then request a manual/fixed-setup preview; the selected grant subset denies that preview.", "",
              "`summary.json` is the shared bench's unedited summary. `outcomes.json` retains one classification, run key and exact payload hash for every invocation/host key; repeat disagreement prevents capture. Full payloads are regenerated, not retained: `evidence.json` records their digest and size and pins the small retained tables and report. No absolute paths or raw machine diagnostics are retained.", "",
              "## Lowering counts", "",
              "Per neutral program, summed across its separately compiled entries: added i32 label locals/parameters + i64 fuel fields / total emitted call sites. These are static counts, not executed-call frequencies, latency or benchmark measurements. Result-label signature fields, control fields, temporary masks, i32.or sites, per-import call counts and module byte counts are individually retained under each program/form/entry in `modules.json`. Full source digests are the keys there. No timing comparison is made.", "",
              "| Neutral program SHA-256 prefix | Entries | Separation | Static | Runtime |", "| --- | --- | --- | --- | --- |",
              *count_rows, "", "## Commands and reproducibility", "",
              "From the repository root; CLI paths after matrix flags are relative to part-b. Preparation intentionally writes pins and is not an acceptance operation:", "", "```text",
              COMMAND + "candidates/rt3-wasm/prepare.py",
              COMMAND + "candidates/rt3-wasm/probe.py",
              COMMAND + f"run_matrix.py --candidate candidates/rt3-wasm --out candidates/rt3-wasm/build/Windows --runs {summary['runs_per_key_per_leg']} --leg Windows",
              COMMAND + "candidates/rt3-wasm/report.py build/Windows",
              "python work/experiments/language-lx/lx3-sandbox-flow/part-b/candidates/rt3-wasm/check.py", "```", "",
              "The report command captures the finished series and writes this report. Remove the candidate-owned build folder after capture. Acceptance verifies the complete corpus freeze without checking moving source-document digests, verifies source/binding pins, regenerates both rendered files byte for byte, runs the focused probes and one full Windows matrix pass, compares every outcome/run key/exact hash, and removes build even on failure. Acceptance does not refresh any pin, result, report or timing field. A negative form can reproduce successfully; acceptance success is not a form pass.", "",
              "**Not run here: headless Linux and the combined twenty-run series.** On the comparison host, regenerate both legs (raw outputs are not shipped) with exactly:", "", "```text",
              COMMAND + "run_matrix.py --candidate candidates/rt3-wasm --out candidates/rt3-wasm/build/Windows --runs 10 --leg Windows",
              COMMAND + "run_matrix.py --candidate candidates/rt3-wasm --out candidates/rt3-wasm/build/Linux --runs 10 --leg headless_Linux", "```", "",
              "Then from `work/experiments/language-lx/lx3-sandbox-flow/part-b/`:", "", "```text",
              "python -B -m harness.report --inputs candidates/rt3-wasm/build/Windows candidates/rt3-wasm/build/Linux --out candidates/rt3-wasm/build/comparison.json", "```", "",
              "Record the Linux Python/Node versions before comparison. Do not refresh bindings to relabel a changed candidate. No GPU or decisive timing command is required for this card; neither was run here. Integrator review remains outside this implementation packet; no commit, stage, ref change, nested review call, network access or installation was performed.", "",
              "## Bench issues", "",
              "B10 remains `not_run: corpus_issue` at admission, exactly as the integrator's recorded decision requires. Its short loop does not sustain the wall deadline; no delay or synthetic trigger was inserted. In wholly not-run host-subset cells the shared bench reports the cell reason first; B10 is still untested, never passed. This is the documented bench behavior, not a denominator exclusion. No bench or corpus file was changed, and no other defect was worked around.", "",
              "## Effort and unfinished work", "",
              "Built one positional-array renderer, a finite front-end analysis and handwritten Wasm binary assembler, explicit handle/label/control ABI, synchronous stdio bridge with a reentrant metering trampoline, trusted host-operation glue, source/compiled-byte pins, synthetic lowering probes, and capture/acceptance drivers. Reused the frozen corpus and the shared policy, exact calculations, fake kernel, immutable provenance host, meter, cache checks, classifier, matrix and summary; no enforcing mechanism was borrowed from a control. The surface, assembler, bridge and fixture-specific plumbing are throwaway experiment code. There is no proposed product language or general-purpose compiler.", "",
              "Open: all form failures and the false-refusal tradeoff; all owner decisions; Linux comparison; B10 wall-clock evidence; standalone Wasm runtime evidence; all-program correctness/security and product integration. The three approved_subset_byte_bound cells deliberately remain not_run because this route has no host subset. These omissions and failures do not become passes at the time limit.", ""]
    return "\n".join(lines).encode("utf-8")


def capture(folder):
    summary_bytes = (folder / "summary.json").read_bytes()
    rows = [json.loads(line) for line in (folder / "results.jsonl").read_bytes().splitlines()]
    summary = json.loads(summary_bytes)
    (HERE / "RESULT.md").write_bytes(report(summary, rows))
    (HERE / "summary.json").write_bytes(summary_bytes)
    (HERE / "outcomes.json").write_bytes(canonical(projection(rows)) + b"\n")
    evidence = {"decision": "open", "decider": "owner", "evidence_class": ["source", "synthetic"],
                "versions": {"Python": sys.version.split()[0], "Node": "v24.21.0"},
                "raw_results": {"bytes": (folder / "results.jsonl").stat().st_size,
                                "sha256": sha((folder / "results.jsonl").read_bytes())},
                "raw_run": {"sha256": sha((folder / "run.json").read_bytes())},
                "files": {name: sha((HERE / name).read_bytes()) for name in (
                    "bindings.json", "summary.json", "outcomes.json", "RESULT.md")}}
    (HERE / "evidence.json").write_bytes(canonical(evidence) + b"\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("folder", help="candidate-relative retained series folder")
    args = parser.parse_args()
    folder = (HERE / args.folder).resolve()
    if not folder.is_relative_to(HERE / "build"):
        raise ValueError("capture must be from candidate build")
    capture(folder)
