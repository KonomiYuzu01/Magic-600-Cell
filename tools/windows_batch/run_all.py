"""Run the checkout's Windows checks in order and publish a sanitised summary."""
from __future__ import annotations

import argparse
import contextlib
import ctypes
import dataclasses
import os
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
import time
from datetime import datetime, timezone
from pathlib import Path

# --list must not create import caches either.
sys.dont_write_bytecode = True
if __package__:
    from . import batch_guard as guard, batch_report as report
    from .batch_interface import (Completed, DEFAULT_OPTIONS, ERROR, EXIT_FAIL,
                                  EXIT_INTERRUPTED, EXIT_PASS, EXIT_USAGE, FORMAT,
                                  INTERRUPTED, MAX_LINES, SKIPPED, STATUSES,
                                  STEP_MODULES, StepResult, clip)
else:
    import batch_guard as guard
    import batch_report as report
    from batch_interface import (Completed, DEFAULT_OPTIONS, ERROR, EXIT_FAIL,
                                 EXIT_INTERRUPTED, EXIT_PASS, EXIT_USAGE, FORMAT,
                                 INTERRUPTED, MAX_LINES, SKIPPED, STATUSES,
                                 STEP_MODULES, StepResult, clip)

HERE = Path(__file__).resolve().parent
QUIET_QUESTION = "Stop them and press Enter to check again, or type skip to skip the captures"
NOT_RUN = "not run: the batch was interrupted"


def is_windows():
    return sys.platform == "win32"


def is_elevated():
    return bool(ctypes.windll.shell32.IsUserAnAdmin()) if is_windows() else False


def stop_tree(process):
    ended = False
    if is_windows():
        try:
            ended = subprocess.run(
                ["taskkill", "/PID", str(process.pid), "/T", "/F"],
                stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                timeout=5).returncode == 0
        except (OSError, subprocess.TimeoutExpired):
            pass
    if not ended:
        try:
            process.kill()
        except OSError:
            pass
    try:
        process.wait(timeout=10)
    except subprocess.TimeoutExpired:
        pass


@dataclasses.dataclass
class BatchContext:
    repo: Path
    private: Path
    scratch: Path
    elevated: bool
    options: dict
    input_fn: object = input
    out: object = sys.stdout
    _runs: int = dataclasses.field(default=0, init=False)

    def say(self, line):
        print(line, file=self.out, flush=True)

    def _input(self):
        try:
            return self.input_fn()
        except EOFError:
            raise KeyboardInterrupt from None

    def manual(self, line):
        self.say("\a")
        self.say(">>> " + " ".join(str(line).split()) + " Press Enter to continue.")
        self._input()

    def ask(self, question, default=""):
        self.out.write(f"??? {' '.join(str(question).split())} [{default}]: ")
        self.out.flush()
        return self._input().strip() or default

    def require_quiet(self):
        while True:
            try:
                busy = guard.heavy(guard.list_processes(), os.getpid())
            except Exception as exc:
                self.say("Cannot read process list: " + clip(exc))
                names = "unknown"
            else:
                if not busy:
                    return None
                for pid, name, why in busy:
                    self.say(f"  {pid} {name}: {why}")
                names = ", ".join(sorted({name for _, name, _ in busy}, key=str.casefold))
            if self.ask(QUIET_QUESTION).casefold() == "skip":
                return f"skipped by the owner: heavy work was running ({names})"

    def run(self, argv, *, timeout, cwd=None, env=None, interactive=False):
        self._runs += 1
        start = time.monotonic()
        process = subprocess.Popen(
            argv, cwd=self.repo if cwd is None else cwd, env=env, shell=False,
            stdin=None if interactive else subprocess.DEVNULL,
            stdout=None if interactive else subprocess.PIPE,
            stderr=None if interactive else subprocess.STDOUT)
        output = []
        read_errors = []
        reader = None
        if not interactive:
            log_path = self.private / f"{self._runs:02d}-{Path(argv[0]).stem}.log"

            def read_output():
                try:
                    with process.stdout, log_path.open("w", encoding="utf-8", newline="\n") as log:
                        for raw in iter(process.stdout.readline, b""):
                            line = raw.decode("utf-8", errors="replace").replace("\r\n", "\n")
                            output.append(line)
                            self.out.write(line)
                            self.out.flush()
                            log.write(line)
                            log.flush()
                except Exception as exc:
                    read_errors.append(exc)

            reader = threading.Thread(target=read_output, daemon=True)
            reader.start()
        timed_out = False
        try:
            returncode = process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            timed_out = True
            returncode = None
            stop_tree(process)
        except KeyboardInterrupt:
            stop_tree(process)
            raise
        finally:
            if reader is not None:
                reader.join(timeout=10)
        if reader is not None and reader.is_alive():
            raise RuntimeError("process output did not close")
        if read_errors:
            raise read_errors[0]
        return Completed(returncode, "".join(output), time.monotonic() - start, timed_out)


@dataclasses.dataclass
class ImportErrorStep:
    name: str
    reason: str

    @property
    def title(self):
        return self.name + " (import unavailable)"

    def unavailable(self, ctx):
        return None

    def run(self, ctx):
        return StepResult(ERROR, self.reason)


def load_steps():
    steps = []
    for module_name in STEP_MODULES:
        try:
            module = report.load_file(HERE / (module_name + ".py"), "_windows_batch_" + module_name)
            steps.append(module.STEP)
        except Exception as exc:
            steps.append(ImportErrorStep(module_name.removeprefix("step_"),
                                         clip(f"{type(exc).__name__}: {exc}")))
    return steps


def normalize(result):
    status = result.status if result.status in STATUSES else ERROR
    reason = result.reason if result.status in STATUSES else "unknown status: " + str(result.status)
    return StepResult(status, clip(reason), counts=result.counts,
                      digests={name: value for name, value in result.digests.items()
                               if isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value)},
                      details=result.details, lines=[clip(line, 200) for line in result.lines[:MAX_LINES]])


def _scenes(text):
    if text == "none":
        return ()
    names = tuple(name.strip() for name in text.split(","))
    if len(set(names)) != len(names) or any(name not in {"w1", "w2", "w3", "w4"} for name in names):
        raise argparse.ArgumentTypeError("scenes must be w1 to w4 without repeats, or none")
    return names


def _runs(text):
    try:
        number = int(text)
    except ValueError:
        raise argparse.ArgumentTypeError("runs must be an integer from 1 to 10") from None
    if not 1 <= number <= 10:
        raise argparse.ArgumentTypeError("runs must be an integer from 1 to 10")
    return number


def _arguments(argv, out):
    parser = argparse.ArgumentParser(description=__doc__)
    names = tuple(module.removeprefix("step_") for module in STEP_MODULES)

    def only(text):
        selected = tuple(name.strip() for name in text.split(","))
        if any(name not in names for name in selected):
            raise argparse.ArgumentTypeError("only must name " + ", ".join(names))
        return selected

    parser.add_argument("--only", type=only, default=names, metavar="NAMES")
    parser.add_argument("--scenes", type=_scenes, default=DEFAULT_OPTIONS["scenes"], metavar="LIST")
    parser.add_argument("--runs", type=_runs, default=DEFAULT_OPTIONS["runs"], metavar="N")
    parser.add_argument("--no-faults", action="store_true")
    parser.add_argument("--rebuild-probe", action="store_true")
    parser.add_argument("--keep-scratch", action="store_true")
    parser.add_argument("--list", action="store_true")
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(out):
        return parser.parse_args(argv)


def _plan(steps, context, source):
    context.say("Administrator: " + ("yes" if context.elevated else "no"))
    if source is None:
        context.say("Commit: not checked (--list)")
    elif "head" in source:
        context.say(f"Commit: {source['head'][:12]}; changed paths: {source['changed_paths']}")
    else:
        context.say("Commit unavailable: " + source["error"])
    entries = []
    cancel_at = None
    for index, step in enumerate(steps):
        ctx = dataclasses.replace(context, private=context.private / step.name,
                                  scratch=context.scratch / step.name)
        result = None
        if cancel_at is not None:
            result = StepResult(SKIPPED, NOT_RUN)
        else:
            try:
                reason = step.unavailable(ctx)
                if reason is not None:
                    result = StepResult(SKIPPED, clip(reason))
            except KeyboardInterrupt:
                cancel_at = index
                result = StepResult(INTERRUPTED, "interrupted while checking availability")
            except Exception as exc:
                result = StepResult(ERROR, clip(f"{type(exc).__name__}: {exc}"))
        action = "skip: " + result.reason if result is not None and result.status == SKIPPED else "run"
        context.say(f"  {step.name}: {action} ({clip(step.title)})")
        entries.append((step, ctx, result))
    return entries, cancel_at


def _utc(clock):
    value = clock()
    if value.tzinfo is None:
        value = value.replace(tzinfo=timezone.utc)
    return value.astimezone(timezone.utc)


class _InterruptHold:
    """The SIGINT handler while a batch runs.

    Ctrl+C raises KeyboardInterrupt only while the hold is released (the start prompt and the
    steps) and never in the batch frame itself. Otherwise it is held, and the next release()
    raises it, before the next step starts. The batch takes the hold with a plain attribute
    store, never a call: Python handles a pending signal at calls, so a call there could raise
    between a finished step and its recorded result."""

    def __init__(self):
        self.held = True
        self.pending = False

    def __call__(self, signum, frame):
        # Handled in the batch frame, the step or prompt has returned: its result is kept.
        if self.held or frame is None or frame.f_code is _run_batch.__code__:
            self.pending = True
        else:
            raise KeyboardInterrupt

    def release(self):
        """Let Ctrl+C interrupt again; one held until now interrupts at once."""
        self.held = False
        if self.pending:
            self.pending = False
            raise KeyboardInterrupt

    def install(self):
        """Become the SIGINT handler; returns the previous handler, or None if it cannot change."""
        try:
            if signal.getsignal(signal.SIGINT) is not None:
                return signal.signal(signal.SIGINT, self)
        except ValueError:  # only the main thread can change signal handlers
            pass
        return None


def _run_batch(args, repo, selected, options, elevated, interrupts, input_fn, out, clock):
    started = _utc(clock)
    stamp = started.strftime("%Y%m%dT%H%M%SZ")
    private = repo / "work/loop-memory/windows-batch" / stamp
    scratch = Path(tempfile.gettempdir()) / ("m6wb-" + stamp)
    if scratch.exists():
        scratch = scratch.with_name(scratch.name + "-" + str(os.getpid()))
    context = BatchContext(repo, private, scratch, elevated, options, input_fn, out)
    try:
        private.mkdir(parents=True)
        scratch.mkdir()
    except OSError as exc:
        context.say("Cannot create batch directories: " + clip(f"{type(exc).__name__}: {exc}"))
        return EXIT_USAGE
    source = {"error": "not checked: the batch was interrupted"}
    results = []
    interrupted = False
    pending = "interrupted before the batch started"

    def add(step, result):
        result = normalize(result)
        results.append(dict(name=step.name, title=clip(step.title), **dataclasses.asdict(result)))
        return result

    # Only the start prompt and the steps run with the hold released. Ctrl+C there, or EOF at
    # a prompt, ends the loop; the results collected until then are published.
    try:
        source = report.source_identity()
        entries, cancel_at = _plan(selected, context, source)
        interrupted = cancel_at is not None
        if not interrupted:
            interrupts.release()
            context.manual("Start the batch. Keep this window open; the batch asks before every manual step.")
            interrupts.held = True
            pending = "interrupted between steps"
            for step, ctx, planned in entries:
                if planned is None:
                    interrupts.release()  # a Ctrl+C held since the previous step stops the batch here
                    try:
                        ctx.private.mkdir()
                        ctx.scratch.mkdir()
                        planned = step.run(ctx)
                    except KeyboardInterrupt:
                        interrupts.held = True
                        planned = StepResult(INTERRUPTED, "interrupted by the owner")
                    except Exception as exc:
                        interrupts.held = True
                        planned = StepResult(ERROR, clip(f"{type(exc).__name__}: {exc}"))
                    else:
                        interrupts.held = True
                result = add(step, planned)
                context.say(f"{step.name}: {result.status} - " +
                            (result.reason or (result.lines[0] if result.lines else "")))
                if result.status == INTERRUPTED:
                    break
    except KeyboardInterrupt:
        interrupts.held = True
        interrupted = True
    for step in selected[len(results):]:
        # The first step without a result records the interruption; the rest did not run.
        first = not any(entry["status"] == INTERRUPTED for entry in results)
        add(step, StepResult(INTERRUPTED, pending) if first else StepResult(SKIPPED, NOT_RUN))
    record = {"format": FORMAT, "started_utc": started.strftime("%Y-%m-%dT%H:%M:%SZ"),
              "finished_utc": _utc(clock).strftime("%Y-%m-%dT%H:%M:%SZ"),
              "outcome": "interrupted" if interrupted else report.outcome(results),
              "source": source, "machine": report.machine(elevated),
              "options": dict(options, only=[step.name for step in selected]), "steps": results,
              "private_records": f"work/loop-memory/windows-batch/{stamp}/ (not committed)"}
    try:
        directory = report.publish(record, repo, private, scratch, context.say)
    finally:
        if not args.keep_scratch:
            try:
                shutil.rmtree(scratch)
            except OSError:
                if scratch.exists():
                    context.say("Scratch remains: " + str(scratch))
    if directory is None:
        return EXIT_USAGE
    context.say("Summary: " + str(directory / "summary.md"))
    context.say("git add -f " + directory.relative_to(repo).as_posix())
    return {"pass": EXIT_PASS, "fail": EXIT_FAIL, "interrupted": EXIT_INTERRUPTED}[record["outcome"]]


def main(argv=None, *, repo=None, steps=None, input_fn=input, out=sys.stdout, clock=None) -> int:
    try:
        args = _arguments(argv, out)
    except SystemExit as exc:
        return int(exc.code)
    if not args.list and not is_windows():
        print("Windows is required; use --list to inspect the plan on another OS.", file=out, flush=True)
        return EXIT_USAGE
    repo = Path(repo) if repo is not None else HERE.parents[1]
    by_name = {step.name: step for step in (load_steps() if steps is None else steps)}
    selected = [by_name[name] for module in STEP_MODULES
                if (name := module.removeprefix("step_")) in args.only and name in by_name]
    options = dict(DEFAULT_OPTIONS, scenes=tuple(args.scenes), runs=args.runs,
                   faults=not args.no_faults, rebuild_probe=args.rebuild_probe)
    elevated = is_elevated()
    if args.list:
        # Only availability reads: no source-identity subprocess, temp lookup or mkdir.
        preview = repo / "work/loop-memory/windows-batch/list"
        context = BatchContext(repo, preview, preview, elevated, options, input_fn, out)
        _plan(selected, context, None)
        return EXIT_PASS
    # From here until main returns, Ctrl+C is held except during the start prompt and the steps.
    interrupts = _InterruptHold()
    previous = interrupts.install()
    try:
        return _run_batch(args, repo, selected, options, elevated, interrupts, input_fn, out,
                          clock or (lambda: datetime.now(timezone.utc)))
    finally:
        interrupts.held = True
        if previous is not None:
            signal.signal(signal.SIGINT, previous)


if __name__ == "__main__":
    sys.exit(main())
