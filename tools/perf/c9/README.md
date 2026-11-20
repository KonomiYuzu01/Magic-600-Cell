# C9 measurement tools

Headless, standard-library tools for LP3's on-screen line (C9) on route B (owner, 18 November 2026). They read logs
and captures, write evidence records and print a runbook; none of them starts a GPU device, a window or a capture,
except `presentmon_capture.py` without `--dry-run`, which needs an administrator shell, and `unattended.ps1`, the
owner's runner for U1 to U8, which starts all of them (its `-DryRun` starts nothing). Every log schema, rule and
choice marked DRAFT here is the integrator's draft (PHS.65) until the owner or a frozen text says otherwise.

| Tool | What it does |
| --- | --- |
| `key_to_frame.py` | Charter E2: key press to the core-confirmed present of the first frame carrying it |
| `cold_start.py` | Charter E3: process creation to the first correct frame, three cold runs |
| `presentmon_capture.py` | Builds, checks and (administrator only) runs a timed PresentMon 2.6.0.0 capture |
| `ui_skew.py` | Route B check B2: UI frame to 3D frame skew from `ui_frame` rows and PresentMon; no bar, numbers only |
| `lx4.py` | LX-4: the schedule (10,800 S/M/L samples), posted window messages (`post`, Windows only; `--dry-run` posts nothing) and the RTE.49/RTE.53 analysis |
| `lx2.py` | LX-2 in the shell (RTE.42 conditions 3 and 4): each metered fault's limit windows (its job runs to its end with no commit) and every fault's turn windows, with LX-4 echo samples (`post`, Windows only; `--dry-run` posts nothing), and the per-fault analysis (E2 over turn windows; echo and gate over a metered fault's limit windows, the others' turn windows) with conditions 1 and 2 from c600-lx2-faults' record |
| `runbook.py` | The owner-machine batch in order (labels A1 to A3, U1 to U8); `--dry-run` checks prerequisites without a GPU |
| `unattended.ps1` | Runs U1 to U8 with one command from an administrator PowerShell (`-SceneBuild`, `-StageBuild`, `-ProductBuild`, `-Steps`, `-Suffix`, `-Retries`, `-DryRun`); in a shell run the watcher, capture and poster start before the shell, and a run with owner input or a lost foreground is set aside and run again, at most twice |
| `foreground.py` | The shell-run watcher `unattended.ps1` starts: owner input (GetLastInputInfo), the shell's window, and with `--hold` topmost plus one SetForegroundWindow (no foreground-lock workaround) and a lost foreground |
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

The same log also carries, for B2, a `ui_frame` row per Godot frame (`ui_seq`, `qpc` before the draw, `frame_id`,
`drawn_qpc` after it) and
`ui_swap_chain` in the `run` row (null with the core's `swap_chain` set: `ui_skew.py` then takes the one other chain of
the process in the capture); for LX-4, an `lx4` row per endpoint (`sample`, `endpoint`, `frame_id`,
`buffer_sha256`) and `puzzle` and `window` in the `run` row. `lx4.py post` turns with a physical turn key
(`--turn-key`, default KeyJ) and sends each key message with the key's scan code (VK_BACK too), since the shell routes
by Godot's physical keycode. For LX-2, LX-2's module writes `lx2` rows (`event`, `qpc`, `window`, `fault`, `kind`: phase,
start, turn, kill, fault_end, child_exit, probe_refused, recovered, phase_end, error) and, like LX-4's, a `module` row
when it starts. `key_to_frame.py` skips those rows; `ui_skew.py`, `lx4.py` and `lx2.py` describe them.

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
