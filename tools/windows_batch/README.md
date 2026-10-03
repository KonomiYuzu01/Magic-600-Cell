# Windows verification batch

From the repository root, run:

```powershell
python tools/windows_batch/run_all.py
```

Use an administrator PowerShell to include the renderer captures, which need
PresentMon. In an ordinary PowerShell the renderer step is skipped with its
reason. The batch does not elevate itself or install anything.

The batch loads the step modules in this order: the native host self-test
(`native`), migration probes (`migration`), and S-B renderer captures
(`renderer`). An unavailable check is skipped. A module that cannot be imported
is recorded as an error; the remaining checks continue. Every check uses fresh
synthetic data.

The first prompt asks you to start the batch and keep its window open. Before
each manual step, one `>>>` line says what to do and waits for Enter. The
migration probes start, wait for and close 0.4 by themselves; leave 0.4 closed
while they run. Before every capture, the heavy-work guard lists process IDs
and names. Stop those programs and press Enter to recheck, or type `skip` to
skip the remaining captures. The batch does not stop unrelated programs itself.
Ctrl+C or EOF at a prompt, or Ctrl+C during a step, interrupts the batch;
completed results are still written and later steps are skipped. A Ctrl+C
pressed while the batch plans, records a result, publishes or removes its
scratch data is held until the next step would start, so a finished step always
keeps its result.

```powershell
python tools/windows_batch/run_all.py --list
python tools/windows_batch/run_all.py --only migration
python tools/windows_batch/run_all.py --only renderer --scenes w1,w2,w4 --runs 3
```

`--only` accepts comma-separated step names and preserves the order above;
repeated names run once. `--scenes` accepts unique names from `w1` to `w4`, or
`none` for no scene capture. Its default is `w1,w2,w4`. `--runs` is 1 to 10
(default 3). `--no-faults` omits injected-fault runs; `--rebuild-probe` requests a
probe rebuild. `--list` works on any OS and only reads availability; it does not
look up the commit, create files, start processes or prompt.

By default the renderer step captures W1, W2 and W4 (attribution scenes) and the
four 10-second W3 injected-fault runs. After every scene run, `run_scene.ps1`
asks you to type `yes` if you watched the whole run with nothing covering the
probe window; a run without that answer is discarded. Add `w3` to `--scenes`
only for an owner-attended gate capture. A fault run counts as caught only when
the probe applied the fault, its label check failed, the capture completed and
the gate refused the run for that label check. The overlay and vendor-mode
answers are published only when you keep the default; any other answer stays
in the private record.

Sanitised `batch.json` and a one-page `summary.md` go under
`work/windows-batch/<UTC date>/`, with `-2`, `-3`, etc. for another batch on that
date. Paths, account names and SIDs become placeholders. Only these public
results should be committed, after reviewing them:

```powershell
git add -f work/windows-batch/<dir>
```

Raw child output stays private under
`work/loop-memory/windows-batch/<UTC stamp>/<step>/`. If sanitisation still
detects private text, neither public file is written: the raw record is saved
there as `batch-unpublished.json` and the batch exits 2. Scratch data under the
system temporary directory (`m6wb-<UTC stamp>`, or with a PID suffix on a
collision) is removed after the batch; `--keep-scratch` retains it. Neither raw
records nor scratch data should be committed. A repeated private timestamp is
refused rather than overwriting raw records.

Exit codes: 0 for passing or skipped checks, 1 for a failed or errored check,
2 for usage/platform errors or a sanitisation leak, and 130 for interruption.
Renderer numbers apply only to their named probe build. The native self-test
and migration probes are not performance evidence.
