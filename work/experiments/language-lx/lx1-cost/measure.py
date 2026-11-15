"""Freeze inputs, then explicitly record LX-1 counts and sandbox timing leads."""

import argparse
from datetime import datetime, timezone
import statistics
import sys

sys.dont_write_bytecode = True

from support import (BUILD, CANDIDATES, HERE, MODES, PART_B, clean_build, count_all,
                     digest, freeze_inputs, read, require, sha, verify_pins, worker, write)


def metric(samples, denominator=1):
    if denominator in (None, 0):
        return "not available here"
    return f"{statistics.median(samples) / denominator / 1000:.3f} / {sorted(samples)[98] / denominator / 1000:.3f}"


def series_report(timings, counters):
    cells = {(c["candidate"], c["mode"], c["workload"]): c for c in counters["cells"]}
    text = ["# LX-1 sandbox time series", "", "Decision **open**; decider **owner**. Timing is a weak lead only.", "",
            "Raw ordered nanosecond samples (smoke, then three series of 100) are in `timings.json`.",
            f"Raw file SHA-256: `{sha((HERE / 'timings.json').read_bytes())}`.",
            "Regenerate: `python -B work/experiments/language-lx/lx1-cost/measure.py --record`.", "",
            "Timer: `time.perf_counter_ns`. Python is in-process within a fresh worker per candidate/mode.",
            "Each program gets a fresh synthetic host. RT2 Node reuses one child and creates a fresh VM context",
            "per program; RT3 starts a fresh Node child per program. One workload run executes its entire",
            "ordered list. Source reading, parsing/approval/compilation, execution and bridge work are timed;",
            "worker startup and final payload hashing/validation are excluded. Counter hooks/opcode tracing",
            "are disabled. Original logical metering and provenance overhead remain in every mode.", "",
            "Cells run sequentially on a machine shared with other work. D6 power source, Windows power mode,",
            "vendor mode, V-Sync and overlays are not controlled or recorded here. No GPU is used. Neither",
            "these series nor their smoke runs are decisive B4-12 performance evidence.", "",
            "Each entry below is **median / p99, microseconds**. Median is the mean of samples 50 and 51;",
            "p99 is nearest rank, sample 99 of 100 sorted samples. Per-step and per-kernel-call entries divide",
            "each whole-workload elapsed sample by its repeated count: amortized workload cost, not isolated",
            "instruction or kernel latency. No cross-route step ordering or total is meaningful.", ""]
    for candidate in CANDIDATES:
        text += ["## " + candidate, "", "| Workload | Mode | Series | Executed form | Elapsed | Per step | Per kernel call | Per coarse unit |",
                 "| --- | --- | --- | --- | --- | --- | --- | --- |"]
        for t in timings["cells"]:
            if t["candidate"] != candidate:
                continue
            count = cells[(candidate, t["mode"], t["workload"])]
            mode = t["mode"] + (" (incomplete wrapper attempt)" if t["status"] == "incomplete_wrapper_only_attempt" else "")
            if t["status"] == "not_run":
                text.append(f"| {t['workload']} | {mode} | not_run | {t['executed_form']} | {t['reason']} | not_run | not_run | not_run |")
                continue
            coarse = count["coarse_unit"]
            for i, samples in enumerate(t["series_ns"], 1):
                unit = (coarse["name"] + ": " + metric(samples, coarse["count"])) if coarse else "not applicable"
                text.append(f"| {t['workload']} | {mode} | {i} | {t['executed_form']} | {metric(samples)} | "
                            f"{metric(samples, count['steps'])} | {metric(samples, count['kernel_calls'])} | {unit} |")
        text.append("")
    (HERE / "SERIES.md").write_bytes(("\n".join(text) + "\n").encode("utf-8"))


def result_report(counters, timings, manifest):
    text = ["# LX-1: work and boundary traffic on LX-3 prototypes", "",
            "**Lead only. The decisive LX-1 run waits for LQ8 and frozen experiment versions of the three",
            "LP3 tools. Neither exists. No figure here selects or rejects a route, answers an LQ or establishes E2.**", "",
            "## CT-RTE-05 record", "", "| Field | Record |", "| --- | --- |",
            "| LX card / experiment | LX-1, chapter 11 RTE.38-RTE.40 / E-2.4-L1 |",
            "| Owner allowance | `docs/wiki/decisions/owner-decisions-2026-11-05.md`, decision 2; experiment index: counters on LX-3 prototypes as a lead with the host named |",
            "| Time box | what can be done now; one wrapper call of at most two hours. A reached limit is no pass. |",
            "| Frozen workload/input manifest | `workloads.json` and `INPUTS.json`; one unchanged lx3-neutral-v2 input set; raw FREEZE SHA-256 `" + manifest["corpus_freeze_sha256"] + "` |",
            "| Candidate/source scope | Five original LX-3 candidates plus the explicit LX-1 labels-off variants. All original source, artifact, summary and manifest pins are verified. Local sources are separately pinned. |",
            f"| Input manifest SHA-256 | `{sha((HERE / 'INPUTS.json').read_bytes())}` |",
            f"| Workload file SHA-256 | `{sha((HERE / 'workloads.json').read_bytes())}` |",
            f"| Commands / tools | Commands below; CPython {manifest['versions']['python']}; Node {manifest['versions']['node']}; V8 {manifest['versions']['v8']}; Python standard library and Node built-ins only |",
            "| Host | Windows sandbox; CPython interpreter/host library, Node vm, and Node built-in WebAssembly engine. The experiment index names the machine as AMD Ryzen 9 7945HX / RTX 4070 Laptop GPU; no GPU work ran here. |",
            "| Evidence class | source and synthetic. All timing is a shared-machine sandbox lead, not performance or Windows/DirectX evidence. |",
            "| Kill criteria | Stop a run on output, identity, accounting or source-binding failure. Retain the failure. A slow observation rejects nothing. |",
            "| Actual result | 39 runnable workload/mode cells and six declared not_run cells; 416 program invocations in the count pass. All comparable exact summary hashes and all cross-mode observable result hashes agree. Each runnable cell has a smoke run and three series of 100. |",
            "| Decision / decider | **open / owner**; every route, host, placement, flow-control, syntax and LQ decision remains open |", "",
            "Original candidate raw `bindings.json` / `summary.json` SHA-256 (verified, not regenerated):", "",
            "| Candidate | Bindings | Summary |", "| --- | --- | --- |"]
    for candidate in CANDIDATES:
        folder = PART_B / "candidates" / candidate
        text.append(f"| {candidate} | `{sha((folder / 'bindings.json').read_bytes())}` | `{sha((folder / 'summary.json').read_bytes())}` |")
    text += ["", "## Workloads and missing real work", "",
             "Selection was written in `CARD.md` before measurement. `full` is A01-A10, A21-A24, A26-A29;",
             "`jumbling` is A31/A32/A34/A35; `representations` is A11-A20, each in ID order with a fresh host",
             "per program. A25 and A30 cannot run under the capability-separation basis of labels-off and are",
             "excluded. A33 is outside the category/row selection rule. No allowed program calls `bits`.", "",
             "The Full stand-in has no real full-state kernel or live view. Jumbling uses tiny constant nu arrays,",
             "two-by-two arithmetic, workspace storage and a synthetic research capture; it has no real pose",
             "state or large research workload. Representations uses tiny chart plans and literal explanations,",
             "with no renderer or interactive view. These are not frozen implementations of the three LP3 tools.", "",
             "## Local labels-off changes", "",
             "The unchanged static/runtime cells run their original adapters. Adapters were read first: these",
             "execution paths write no files, so originals can run read-only; no candidate copy or preparation",
             "script is needed. Python bytecode writes are disabled, including in child workers. All generated",
             "outputs live under LX-1's `build/` and are removed. No corpus, candidate or harness file is edited.", "",
             "| Prototype | Exact LX-1 labels-off variant |", "| --- | --- |",
             "| rt1-own | In-memory AST variant replaces every Flow construction and join in the checker and executor with one empty Flow singleton. Type/effect traversal and source dispatch remain; there is no label inference or propagation. The independent coarse read-state flag remains. |",
             "| rt2-py | Replaces Analysis initialization with a structural call/effect scan and empty annotations. The original AST approval, renderer, bytecodes and bridge execute; no static label pass runs. |",
             "| rt2-node | Replaces analyze with a reachable-operation/effect scan and empty annotations. Original approved JavaScript and unmodified V8 run through the original bridge. |",
             "| rt3-wasm | Skips label analysis; emits the existing constant-field path with every label field zero, no mask joins and no metadata imports. Keeps inert ABI slots and provenance handles. Local Wasm hashes are retained per program; these modules are not relabelled as the originals. |",
             "| rt4-pyhost | The capability cell already skips static analysis. Erases state/computed tag argument expressions and replaces Library knowledge/make label unions with empty labels while retaining read-state; original rendered Python bytecodes execute. |", "",
             "All five use a local Host that removes observational labels, including preview's label unions.",
             "Capabilities, profile/parts/declassifier provenance, coarse read-state, source approval and the",
             "original logical meter remain. Empty label fields and other scaffolding remain, so labels-off",
             "is not a claim about the cheapest possible implementation. Each local variant has a distinct",
             "binding identity. This experiment supplies no new enforcement or soundness evidence.", "",
             "RT2 Python/Node `run_time` stays not_run because RTE.12 requires a modified VM. RT4 `run_time`",
             "runs the original **incomplete wrapper-only attempt**, whose native branch-label failures remain",
             "documented in its LX-3 report. This workload does not repair or retest those negative cases.", "",
             "## Work and bytes (within each prototype only)", "",
             "**Largest limitation, applying to every figure below:** the fake kernel has four slots. Its",
             "state/nu rows return fixed small arrays; `word_check` does real permutation work only on those",
             "four toy slots. There is no full-model per-sticker kernel work. Figures measure prototype",
             "interpreter, checking, provenance and bridge overhead; they cannot establish batching benefits",
             "on the real model. The source's toy `word_check` qualifies the packet's description of constant rows.", "",
             "Canonical argument/result bytes use the same compact sorted-key UTF-8 JSON encoding of data",
             "for every prototype, separately in each direction. They exclude provenance and framing, and",
             "are a virtual encoding for in-process calls. Actual bridge bytes include LF, source/Wasm",
             "launch, every helper call and reentrant fuel messages over Node stdio. That wider bridge",
             "often transports handles rather than kernel data; its bytes are not the canonical-byte figure.", "",
             "The bench column is **continuation calls / sum of continuation weights**, not executed steps,",
             "DET.20's abstract budget or an LG4.8 charge proposal. Python steps are opcode trace events in",
             "rendered code objects only via sys.monitoring.INSTRUCTION, including their module setup;",
             "RESUME and CACHE emit no instruction events. Host/library/harness bytecodes and",
             "compiler work are excluded. V8/Wasm instruction counts are unavailable without a different",
             "engine/instrumentation. Their coarse callbacks/imports are named separately and never steps.", ""]
    for candidate in CANDIDATES:
        text += ["### " + candidate, "", "| Workload | Mode | Executed form | Steps | Coarse unit | Bench continuations | Kernel calls | Canonical args / results B | Actual bridge sent / received B |",
                 "| --- | --- | --- | --- | --- | --- | --- | --- | --- |"]
        for c in counters["cells"]:
            if c["candidate"] != candidate:
                continue
            mode = c["mode"] + (" (incomplete wrapper attempt)" if c["status"] == "incomplete_wrapper_only_attempt" else "")
            if c["status"] == "not_run":
                text.append(f"| {c['workload']} | {mode} | {c['executed_form']} | not_run | {c['reason']} | not_run | not_run | not_run | not_run |")
                continue
            steps = c["steps"] if c["steps"] is not None else "not available here"
            coarse = c["coarse_unit"]
            coarse_text = f"{coarse['count']} {coarse['name']}" if coarse else "not applicable"
            bridge = c["actual_bridge"]
            bridge_text = f"{bridge['python_to_node_bytes']} / {bridge['node_to_python_bytes']}" if bridge else "no transport"
            text.append(f"| {c['workload']} | {mode} | {c['executed_form']} | {steps} | {coarse_text} | "
                        f"{c['bench_continuation_calls']} / {c['bench_continuation_weight']} | {c['kernel_calls']} | "
                        f"{c['canonical_arguments_bytes']} / {c['canonical_results_bytes']} | {bridge_text} |")
        text.append("")
    text += ["## Kernel mix and size projections", "",
             "Only calls through `ctx.kernel_call` are counted here. Preview's internal fake check is a",
             "host operation outside that boundary. No selected case calls batch_check. Every runnable",
             "prototype/mode produced the following same mix; zeros are retained. Full details by row and",
             "direction, including per-program result hashes, are in `counters.json`.", "",
             "| Kernel row | full calls | jumbling calls | representations calls |", "| --- | --- | --- | --- |"]
    exemplar = {w: next(c for c in counters["cells"] if c["workload"] == w and c["status"] != "not_run")
                for w in ("full", "jumbling", "representations")}
    for c in counters["cells"]:
        if c["status"] != "not_run":
            require(c["kernel_calls_by_row"] == exemplar[c["workload"]]["kernel_calls_by_row"], "row mix differs")
    for row in sorted(exemplar["full"]["kernel_calls_by_row"]):
        values = [str(exemplar[w]["kernel_calls_by_row"][row]["calls"]) for w in exemplar]
        text.append("| " + row + " | " + " | ".join(values) + " |")
    text += ["", "**Projection, never measurement.** RTE.33 supplies sizes, not a mapping from nu arrays to poses.", "",
             "| Workload | snapshot calls x 1,039,200 label B | Pose bundle size | Pose projection |", "| --- | --- | --- | --- |"]
    for w, c in exemplar.items():
        p = c["projection"]
        text.append(f"| {w} | {p['snapshot_calls']} x 1,039,200 = {p['label_bytes']:,} | 869,952 B | not available here: no pose-transfer row |")
    text += ["", "LX-2 can use this per-row stand-in mix and the explicit label projection. It cannot infer",
             "a pose-transfer frequency from this corpus. No full-size payload was transferred by LX-1.", "",
             "## Timing and equality", "",
             "`SERIES.md` reports median and p99 of all three 100-run series per cell, including time per",
             "executed step where available and per kernel call. `timings.json` retains ordered raw samples",
             "and smoke runs. Times are whole-workload elapsed divided by counts, not isolated call latency.",
             "The timer and child lifecycle are documented there. D6 conditions were not controlled; other",
             "work shares the machine. All measurements are weak leads and no slow result rejects anything.", "",
             "For original static/runtime cells, each exact payload hash and full run key matches every",
             "corresponding stored summary repeat (256 program observations, 2,560 stored-repeat comparisons).",
             "All 416 count-pass observations match the frozen allowed-tool oracle. Each workload's observable",
             "result hash agrees across its runnable modes. Local labels-off exact hashes are deliberately",
             "not compared with old capability hashes; their new identity and removed labels differ.", "",
             "Cross-mode payload normalization is specified in CARD.md and `runner.py:result_payload`: it",
             "retains exact typed outputs, order, coverage, display, effects and protected session state,",
             "excluding form/source identities, budgets and provenance-dependent metadata. This is not a",
             "claim that full LX-3 metadata or hashes are identical across modes. Full hashes remain retained.", "",
             "## Effort and reuse (RTE.9)", "",
             "Only rt2-node's report gives elapsed agent hours, including waiting and acceptance. The other",
             "reports do not give hours; do not turn a wrapper's time limit into effort. These rows are",
             "leads and set no product estimate. LX-1 wrapper work is not separately metered by prototype.", "",
             "| Prototype | Agent hours | Slice built in its LX-3 report | Reused here / throwaway |", "| --- | --- | --- | --- |",
             "| rt1-own | not known | Own lexer/parser, type/effect/flow checker, typed IR interpreter and source binding | Original executor and synthetic bench reused; label erasure/count wrapper and experiment surface are throwaway |",
             "| rt2-py | not known | Mechanical Python renderer, approved AST subset, static checker and domain bridge | CPython and original artifacts reused; effect-only variant/opcode observer are throwaway |",
             "| rt2-node | about 1.2 agent-hours elapsed, as its LX-3 report records; includes waiting and acceptance | Canonical JS renderer/subset recognizer, static analysis and stdio handle bridge | Node/V8 and original JS reused; effect-only variant/pipe observer are throwaway |",
             "| rt3-wasm | not known | Own array surface, analysis, handwritten Wasm lowering and host-handle bridge | Node built-in Wasm and original compiler reused; zero-label lowering/import observer are throwaway |",
             "| rt4-pyhost | not known | Python renderer, subset checker and domain library; incomplete runtime wrapper attempt | CPython/library/programs reused; label-free library/opcode observer are throwaway |", "",
             "The shared policy, arithmetic, provenance, fake kernel, source maps and oracle remain the same.",
             "LX-1 adds fixed selection, observations, explicit local variants and repeatability/report drivers.",
             "No product implementation, renderer, VM modification, installation or external service was built.", "",
             "## Commands, remaining work and limits", "", "Executed from the repository root:", "", "```text",
             "python -B work/experiments/language-lx/lx1-cost/measure.py --freeze",
             "python -B work/experiments/language-lx/lx1-cost/measure.py --record",
             "python work/experiments/language-lx/lx1-cost/check.py", "```", "",
             "The check verifies corpus/raw/normalized freeze, each candidate's stored source/artifact",
             "pins and source maps, local source pins, and tool versions (also the stored Node/Python",
             "executable pins). It recomputes every count once under `build/`, compares stored counters,",
             "full summary hashes where comparable and normalized results across modes, and removes",
             "`build/` even on failure. It neither measures nor parses timing samples, refreshes pins nor",
             "rewrites reports. A byte snapshot checks retained-file immutability. There is no speed bar.", "",
             "**Not run here:** optional N=1/10/100/1000 word_check-loop versus batch_check scale contrast.",
             "Priority was the complete frozen-corpus slice. These adapters bind approval to frozen artifacts;",
             "new neutral scale programs need a separately scoped local approval/artifact variant, not edits",
             "to the corpus or existing candidates. No scale count or batching-speed result is claimed.", "",
             "**Not run here:** Linux, a selected LQ8 host, real/frozen LP3 tools, real pose traffic, a modified",
             "RT2/RT4 VM, foreground renderer, GPU/native input, long-session runs and decisive E2 timing.",
             "For the integrator's repeat of this same Windows lead outside the sandbox, use exactly:", "", "```text",
             "python work/experiments/language-lx/lx1-cost/check.py",
             "python -B work/experiments/language-lx/lx1-cost/measure.py --record", "```", "",
             "Preserve this observation before explicitly recording a new series. These commands remain",
             "prototype leads even on an idle machine; they do not become decisive by moving outside the",
             "sandbox. No truthful decisive-host/LP3/GPU command exists until those candidates and their",
             "inputs are supplied and frozen. Linux would need a new host/tool identity, not a bypass of pins.", "",
             "Independent integration review remains with the integrator. No nested review-wrapper call",
             "was made: this packet forbids its network use and writes outside the owned folder. No Git",
             "mutation was performed. No rule, source, corpus or candidate outside this folder was changed.", "",
             "Cannot show: product responsiveness, a route pass, equal instruction counts across routes,",
             "LP3's charter E2 result, real-kernel scaling or a product label-cost estimate. Nothing here",
             "resolves LQ8/LQ9/LQ21 or any other LQ. Negative/unavailable evidence remains explicit.", "",
             "Sources read: AGENTS.md and the current agent briefing; chapter 11 RTE.9/RTE.12/RTE.24 and",
             "RTE.30-RTE.40; the experiment index and owner allowance; all five LX-3 candidate reports,",
             "declarations, bindings, summaries and adapters; the named harness/corpus files; LX-2 RESULT.md",
             "for the two real sizes. Moving governance sources are named, never acceptance-pinned."]
    (HERE / "RESULT.md").write_bytes(("\n".join(text) + "\n").encode("utf-8"))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--freeze", action="store_true")
    group.add_argument("--record", action="store_true")
    args = parser.parse_args()
    if args.freeze:
        freeze_inputs()
        print("Frozen selection, inputs, original candidate files, local sources and tool versions; no measurement.")
        return
    require(not BUILD.exists(), "build must be absent before recording")
    manifest = verify_pins()
    BUILD.mkdir()
    try:
        counters = count_all()
        write(HERE / "counters.json", counters)
        timings = {"version": 1, "decision": "open", "decider": "owner", "timer": "time.perf_counter_ns",
                   "started_utc": datetime.now(timezone.utc).isoformat(), "versions": manifest["versions"],
                   "input_manifest_sha256": sha((HERE / "INPUTS.json").read_bytes()), "cells": []}
        for candidate in CANDIDATES:
            for mode in MODES:
                print("Recording", candidate, mode, flush=True)
                timings["cells"].extend(worker(candidate, mode, "timings"))
        timings["finished_utc"] = datetime.now(timezone.utc).isoformat()
        write(HERE / "timings.json", timings)
        series_report(timings, counters)
        result_report(counters, timings, manifest)
        print("Recorded 45 cells, including not_run; decisions open. Run check.py for acceptance.")
    finally:
        clean_build()


if __name__ == "__main__":
    main()
