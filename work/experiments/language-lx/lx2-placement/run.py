"""Fault/control driver and per-row sandbox leads. No renderer or language VM."""
from __future__ import annotations

import argparse
from array import array
import json
import math
import os
import queue
import statistics
import struct
import subprocess
import sys
import threading

sys.dont_write_bytecode = True
from prepare import BUILD, EXE, HERE, canonical, cleanup, environment, prepare, sha, write_json


class EngineDied(Exception):
    pass


class Refused(Exception):
    pass


def read_exact(stream, size):
    parts = bytearray()
    while len(parts) < size:
        part = stream.read(size - len(parts))
        if not part:
            raise EngineDied()
        parts.extend(part)
    return bytes(parts)


class Control:
    def __init__(self, placement, name):
        self.journal = BUILD / (name + ".journal")
        self.stderr_path = BUILD / (name + ".engine.stderr")
        self.stderr = self.stderr_path.open("wb")
        self.process = subprocess.Popen(
            [str(EXE), "kernel", str(BUILD / "model.bin"), str(BUILD / "pose.bin"),
             str(HERE / "PROVENANCE.json"), str(self.journal), placement],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self.stderr,
            env=environment(), creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0)
        self.answers = queue.Queue()
        self.runtime_pid = None

        def reader():
            try:
                while True:
                    size = struct.unpack("<I", read_exact(self.process.stdout, 4))[0]
                    if not 1 <= size <= 2 * 1024 * 1024:
                        raise ValueError("Invalid control frame length")
                    self.answers.put(read_exact(self.process.stdout, size))
            except (EngineDied, OSError, ValueError) as error:
                self.answers.put(error)

        self.reader = threading.Thread(target=reader, daemon=True)
        self.reader.start()

    def call(self, op, **fields):
        payload = canonical(dict(op=op, **fields))
        try:
            self.process.stdin.write(struct.pack("<I", len(payload)) + payload)
            self.process.stdin.flush()
        except (BrokenPipeError, OSError):
            raise EngineDied() from None
        answer = self.answers.get(timeout=150 if op == "finish" else 30)
        if isinstance(answer, Exception):
            raise answer
        if answer[0] == 1:
            raise Refused(answer[1:].decode())
        if answer[0] != 0:
            raise ValueError("Invalid control response tag")
        return json.loads(answer[1:])

    def close(self):
        if self.process.poll() is None:
            try:
                self.call("shutdown")
                self.process.wait(timeout=10)
            except (EngineDied, OSError, queue.Empty, subprocess.TimeoutExpired):
                self.process.kill()
                self.process.wait(timeout=10)
                if self.runtime_pid and self.runtime_pid != self.process.pid:
                    try:
                        os.kill(self.runtime_pid, 9)
                    except OSError:
                        pass
        self.reader.join(timeout=5)
        self.process.stdin.close()
        self.process.stdout.close()
        self.stderr.close()


class JournalOracle:
    """Independent Python replay of the exported source-to-destination arrays."""
    def __init__(self):
        with (BUILD / "model.bin").open("rb") as stream:
            assert stream.read(8) == b"LX6MOD1\0"
            size = struct.unpack("<I", stream.read(4))[0]
            self.model = json.loads(stream.read(size))
            vectors = []
            for _ in range(5):
                count = struct.unpack("<I", stream.read(4))[0]
                values = array("i")
                values.frombytes(stream.read(count * 4))
                if sys.byteorder != "little":
                    values.byteswap()
                vectors.append(values)
        self.offsets, self.source, self.destination = vectors[2:]
        assert len(self.offsets) == 1201

    @staticmethod
    def state_hash(labels):
        if sys.byteorder == "little":
            return sha(labels.tobytes())
        little = labels[:]
        little.byteswap()
        return sha(little.tobytes())

    def inspect(self, raw, provenance):
        labels = array("i", range(259_800))
        state_hash = self.state_hash(labels)
        rows, errors = [], []
        if raw and not raw.endswith(b"\n"):
            errors.append("incomplete journal row")
        for revision, line in enumerate(raw.splitlines(keepends=True), 1):
            try:
                row = json.loads(line)
                if set(row) != {"revision", "kind", "word", "previous_hash", "state_hash", "provenance"}:
                    raise ValueError("journal fields")
                if row["revision"] != revision or row["previous_hash"] != state_hash:
                    raise ValueError("journal chain")
                if row["provenance"] != provenance or row["kind"] not in ("commit", "undo"):
                    raise ValueError("journal provenance or kind")
                if not row["word"]:
                    raise ValueError("empty journal word")
                for move in row["word"]:
                    if type(move) is not int or not 1 <= abs(move) <= 1200:
                        raise ValueError("illegal journal letter")
                    start, end = self.offsets[abs(move) - 1:abs(move) + 1]
                    source, destination = self.source[start:end], self.destination[start:end]
                    if move < 0:
                        source, destination = destination, source
                    scratch = [labels[s] for s in source]
                    for target, value in zip(destination, scratch):
                        labels[target] = value
                state_hash = self.state_hash(labels)
                if row["state_hash"] != state_hash:
                    raise ValueError("journal state hash")
                summary = {k: row[k] for k in ("revision", "kind", "word", "previous_hash", "state_hash")}
                summary.update(row_sha256=sha(line), provenance_sha256=sha(canonical(row["provenance"])))
                rows.append(summary)
            except (ValueError, KeyError, TypeError):
                errors.append("invalid journal row " + str(revision))
                break
        return dict(bytes=len(raw), sha256=sha(raw), rows=rows, errors=errors,
                    valid=not errors, replay_state_hash=state_hash if not errors else None)


def trial(placement, fault, scenario, provenance, oracle):
    name = placement + "-" + fault
    control = Control(placement, name)
    try:
        initial = control.call("status")
        baseline = control.call("preview", word=scenario["baseline_word"], provenance=provenance)
        receipt = control.call("commit", preview=baseline)
        prefix_bytes = control.journal.read_bytes()
        assert receipt["status"] == "committed" and receipt["journal_row_sha256"] == sha(prefix_bytes)
        spec = dict(mode="fault", fault=fault, word=scenario["runtime_word"], provenance=provenance)
        ready = control.call("start", spec=spec)
        control.runtime_pid = ready["runtime_pid"]
        armed = control.call("status")
        assert ready["prefix"]["labels_sha256"] == armed["state_hash"] == receipt["state_hash"]
        assert ready["prefix"]["pose_sha256"] == sha((BUILD / "pose.bin").read_bytes())
        assert ready["prefix"]["preview"] == armed["pending"]
        assert armed["receipts"] == [receipt]
        try:
            control.call("preview", word=scenario["final_word"], provenance=provenance)
            raise ValueError("Second pending preview was admitted")
        except Refused as error:
            assert str(error) == "pending preview exists"
        assert control.journal.read_bytes() == prefix_bytes
        control.call("release")
        session_open = True
        before_tail = after = None
        completion = None
        tail_receipts = []
        tail_commands = ["commit runtime preview", "undo", "preview final_word", "commit final preview"]
        completed_tail = []
        try:
            if fault == "forced_kill":
                if placement == "inside_engine":
                    control.process.kill()
                    control.process.wait(timeout=10)
                    raise EngineDied()
                control.call("kill_runtime")
            completion = control.call("finish")
            before_tail = control.call("status")
            tail_receipts.append(control.call("commit", preview=armed["pending"]))
            completed_tail.append(tail_commands[0])
            tail_receipts.append(control.call("undo"))
            completed_tail.append(tail_commands[1])
            final_preview = control.call("preview", word=scenario["final_word"], provenance=provenance)
            completed_tail.append(tail_commands[2])
            tail_receipts.append(control.call("commit", preview=final_preview))
            completed_tail.append(tail_commands[3])
            after = control.call("status")
        except EngineDied:
            control.process.wait(timeout=10)
            assert control.process.returncode != 0
            session_open = False
        raw = control.journal.read_bytes()
        journal = oracle.inspect(raw, provenance)
        live_state_matches_journal = after["state_hash"] == journal["replay_state_hash"] if after else None
        receipts_match_returns = after["receipts"] == [receipt] + tail_receipts if after else None
        pending_retained = before_tail is not None and before_tail["pending"] == armed["pending"]
        receipt_retained = before_tail is not None and before_tail["receipts"] == [receipt]
        outcome = completion["outcome"] if completion else None
        codes = {"completed", "budget.memory_bytes", "budget.recursion", "budget.instructions",
                 "runtime.died", "runtime.watchdog_killed"}
        typed = (isinstance(outcome, dict) and outcome.get("code") in codes
                 and outcome.get("status") == ("ok" if outcome["code"] == "completed" else "error"))
        job_terminated = completion["job_terminated"] if completion else not session_open
        record = dict(placement=placement, fault=fault, initial=initial, runtime_prefix=ready["prefix"],
                      durable_receipt_before_fault=receipt, durable_prefix_intact=raw.startswith(prefix_bytes),
                      pending_preview_retained=pending_retained, live_receipt_retained=receipt_retained,
                      job_terminated=job_terminated, typed_outcome=typed,
                      outcome=outcome, live_state_matches_journal=live_state_matches_journal,
                      receipts_match_returns=receipts_match_returns,
                      observation=completion["observation"] if completion else "engine_process_died",
                      session_open=session_open, final_live_state=after, journal=journal,
                      completed_tail_commands=completed_tail,
                      unexecuted_tail_commands=tail_commands[len(completed_tail):],
                      condition_2=typed and job_terminated and session_open)
    finally:
        control.close()
    stderr_file = (control.journal.with_suffix(".runtime.stderr") if placement == "own_process"
                   else control.stderr_path)
    stderr = stderr_file.read_bytes() if stderr_file.exists() else b""
    record["native_failure_observed"] = (
        "allocator_exhaustion" if b"memory allocation of 1048576 bytes failed" in stderr else
        "stack_overflow" if b"has overflowed its stack" in stderr else
        "os_forced_kill" if fault == "forced_kill" else
        ("watchdog_kill" if placement == "own_process" else "watchdog_expired_thread_running")
        if fault == "loop_native" else "none")
    if fault == "allocation_native":
        assert record["native_failure_observed"] == "allocator_exhaustion"
    if fault == "recursion_native":
        assert record["native_failure_observed"] == "stack_overflow"
    return record, raw


def measurements(placement, scenario, provenance):
    control = Control(placement, "measure-" + placement)
    try:
        ready = control.call("start", spec=dict(mode="benchmark", fault="control",
                             word=scenario["runtime_word"], provenance=provenance))
        control.runtime_pid = ready["runtime_pid"]
        control.call("release")
        result = control.call("finish")
        assert result["job_terminated"] and result["outcome"]["code"] == "measured"
        rows = result["outcome"]["rows"]
        assert len(rows) == 16
        correctness = []
        for row in rows:
            samples = row["samples_ns"]
            assert len(samples) == (1 if row["series"] == 0 else 100)
            assert all(type(n) is int and n > 0 for n in samples)
            if row["row"] in ("read_labels", "read_pose_bundle"):
                assert row["reply_bytes"] == (1_039_200 if row["row"] == "read_labels" else 869_952)
                expected_hash = (JournalOracle.state_hash(array("i", range(259_800)))
                                 if row["row"] == "read_labels" else sha((BUILD / "pose.bin").read_bytes()))
                assert row["reply_sha256"] == expected_hash
            ordered = sorted(samples)
            row["summary_ns"] = dict(median=statistics.median(samples), p95=ordered[math.ceil(.95 * len(samples)) - 1],
                                     p99=ordered[math.ceil(.99 * len(samples)) - 1], largest=max(samples))
            correctness.append({k: v for k, v in row.items() if k not in ("samples_ns", "summary_ns")})
            correctness[-1]["samples"] = len(samples)
        assert control.journal.read_bytes() == b""
        return dict(placement=placement, rows=rows), dict(placement=placement, rows=correctness)
    finally:
        control.close()


def collect(candidate):
    scenario = json.loads((HERE / "SCENARIO.json").read_bytes())
    provenance = json.loads((HERE / "PROVENANCE.json").read_bytes())
    oracle = JournalOracle()
    runs, timing, transfer = [], [], []
    negative_controls = None
    for placement in scenario["placements"]:
        control_raw = control_record = None
        for fault in scenario["faults"]:
            print(placement + ": " + fault, flush=True)
            record, raw = trial(placement, fault, scenario, provenance, oracle)
            if fault == "control":
                control_raw, control_record = raw, record
                assert len(record["journal"]["rows"]) == 4 and record["condition_2"]
                broken = json.loads(raw.splitlines()[1])
                broken["state_hash"] = "0" * 64
                lines = raw.splitlines(keepends=True)
                lines[1] = canonical(broken) + b"\n"
                negative_controls = dict(
                    changed_interior_hash_rejected=not oracle.inspect(b"".join(lines), provenance)["valid"],
                    truncated_journal_rejected=not oracle.inspect(raw[:-10], provenance)["valid"])
                assert all(negative_controls.values())
            record["control"] = placement + "/control"
            record["journal_equal_control"] = raw == control_raw
            after_hash = (record["final_live_state"]["state_hash"] if record["final_live_state"]
                          else record["journal"]["replay_state_hash"])
            record["hash_equal_control"] = after_hash == control_record["journal"]["replay_state_hash"]
            record["condition_1"] = record["journal"]["valid"] and record["journal_equal_control"] and record["hash_equal_control"]
            record["conditions_1_and_2"] = record["condition_1"] and record["condition_2"]
            runs.append(record)
    for placement in scenario["placements"]:
        print(placement + ": smoke + three series of 100 per row (sandbox lead)", flush=True)
        measured, verified = measurements(placement, scenario, provenance)
        timing.append(measured)
        transfer.append(verified)
    assert transfer[0]["rows"] == transfer[1]["rows"]
    assert runs[0]["journal"] == runs[8]["journal"]
    correctness = dict(format="LX2-placement-results-1", experiment="E-2.4-L2",
        candidate_sha256=candidate["candidate_sha256"], model_id=oracle.model["model_id"],
        slots=259_800, generators=1200, provenance_bytes=len(canonical(provenance)),
        provenance_sha256=sha(canonical(provenance)), runs=runs,
        transfer_checks=transfer, journal_negative_controls=negative_controls,
        slice_results={placement: all(r["conditions_1_and_2"] and r["pending_preview_retained"]
                                     and r["live_receipt_retained"] and r["receipts_match_returns"]
                                     and r["live_state_matches_journal"]
                                     for r in runs if r["placement"] == placement)
                       for placement in scenario["placements"]},
        not_run_here=["RTE.42 condition 3", "RTE.42 condition 4", "first interactive frame startup", "shell placement"],
        decision="open", decider="owner")
    return dict(candidate=candidate, correctness=correctness,
                timing=dict(evidence="sandbox lead", unit="ns", call_mix="open", placements=timing))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", default="OBSERVED.json", help="new JSON filename inside this experiment")
    args = parser.parse_args()
    if args.output != os.path.basename(args.output) or not args.output.endswith(".json"):
        parser.error("output must be a JSON filename, not a path")
    output = HERE / args.output
    if output.exists():
        parser.error("output already exists; preserve it and use another filename")
    try:
        result = collect(prepare())
        write_json(output, result)
        print("Observed conditions 1 and 2: " + json.dumps(result["correctness"]["slice_results"]), flush=True)
    finally:
        cleanup()


if __name__ == "__main__":
    main()
