# C9 measurement tools

Headless, standard-library tools for LP3's on-screen line (C9) on route B (owner, 18 November 2026). They read logs
and captures, write evidence records and print a runbook; none of them starts a GPU device, a window or a capture,
except `presentmon_capture.py` without `--dry-run`, which needs an administrator shell. Every log schema, rule and
choice marked DRAFT here is the integrator's draft (PHS.65) until the owner or a frozen text says otherwise.

| Tool | What it does |
| --- | --- |
| `key_to_frame.py` | Charter E2: key press to the core-confirmed present of the first frame carrying it |
| `cold_start.py` | Charter E3: process creation to the first correct frame, three cold runs |
| `presentmon_capture.py` | Builds, checks and (administrator only) runs a timed PresentMon 2.6.0.0 capture |
| `runbook.py` | The owner-machine batch in order; `--dry-run` checks prerequisites without a GPU |
| `evidence.py` | The shared record format `magic600-c9-measure-record-v1` and log reading |

## Logs (DRAFT)

**E2, `events.jsonl`, format `magic600-c9-shell-events-v1`**, written by the product shell, one object per line, QPC
values in integer ticks:

- `run`: `run_id`, `build_identity`, `qpc_frequency`, `synthetic`, `foreground_throughout`, `correctness`
  (`pass`/`fail`), and for a PresentMon cross-check `process_id` and `swap_chain`;
- `key`: `seq` (0, 1, ...), `qpc` (the shell's key-down receipt), `frame_id` (first frame whose preview carries the
  press, or null);
- `frame`: `frame_id`, `qpc` at submission, `device_epoch`, `surface`;
- `presented`: one `sa2_presented` row: `frame_id`, `device_epoch`, `surface`, `sequence`, `display_qpc`
  (SyncQPCTime of the core's own frame statistics).

**E3, `startup.jsonl`, format `magic600-c9-startup-v1`**: `run` (with `cold`: `new_process`,
`no_product_process_at_launch`, `fresh_session_copy`, `seconds_since_previous_exit`), `process_start` (the launcher's
QPC before process creation), `session_opened`, `runtime_loaded`, `first_correct_frame` (`frame_id`, `display_qpc`,
`check`).

## Bars

- **E2** (charter E2, RTE.43, V11.1): "A turn at full detail responds within 100 ms (the B4-12 M1 interval) on the
  target machine." Nearest-rank p95 at most 100 ms over at least 100 confirmed keys of valid runs of one build; the
  pooled p95 and every run's p95 must meet it. A key without a confirmed frame gets no time and invalidates its run.
- **E3** (owner, 18 November 2026): 3 s, cold start, with an existing session and runtime loaded, to the first
  correct frame, in each of three cold runs. A run that never reaches a passing first correct frame fails the bar.
  Cold (DRAFT): new process, none running, fresh session copy, at least 20 s since the last exit, no reboot.

## Records

Each analysis writes one record (`--out`, never replacing a file): evidence kind `performance` (never `source`), build
identity, input names with SHA-256, tool and helper digests, a path-free command, the bar, the result and a verdict.
A record built from a synthetic input (its header says so, `--synthetic`, or an input under `tests/`) has the verdict
`synthetic` and never passes a bar. A record states what its logs show; it does not establish the machine, the
conditions or the build (PHS.64), and it is never a phase pass on its own (PHS.66).

Tests: `python tests/test_c9_measure.py`.
