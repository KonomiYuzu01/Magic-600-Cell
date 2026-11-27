# ADR-D02 Track J results (B32 and B33, reduced)

The reduced Track J of the ADR-D02 build (`docs/progress/1.0/adr-d02-build-framework.md`, B32 "J1", B33 "J0, the researcher
subagent, and the report", and section 7). These are product runs in sample mode: the shell plays a fixed session on its sample
state's fixture with injected keys and checks the screens it dumps. They are actual Windows/DirectX runs of the product shell, not
performance evidence, and their facts come from Look Lab's samples, not from the product's engine (that comes in K2).

## Result lines

Build: branch `claude/adr-d02-build` at d0fbc397 (27 November 2026). The System log showed no nvlddmkm 14/153 and no
LiveKernelEvent after either run.

```
M600_D02_J1_PASS operations=56 reached=56 boundary=0 observation=6/6
M600_D02_J0_PASS planted=20 caught_driver=17 caught_subagent=8 caught_any=18 replay=9/9 not_planted=5 reach=not_run
```

- **J1** (`python -B renderer/shell/track_j.py --reduced --only j1 --out <folder>`, on e4c50c2a, whose product is d0fbc397's):
  every typed row was reached through the palette, the boundary ledger held (no crossing), and the six observation rules of
  `jumbling-observation.md` that keys can show were observed. Its `not_run` list names the rows that have no typed operation yet.
- **J0** (`python -B renderer/shell/track_j.py --reduced --only j0 --defects <Fable's sealed file> --out <folder>`, then
  `--score <folder> --subagent <answers> --effort <effort>`):
  - Planted: 20 defects, Opus's ten in `tools/looklab/data/samples/jumble/defects.json` and Fable's ten in a sealed file outside
    the repository (sha256 `07c6d19a3e197b74e69f240be12d1c1ffe0cb92db1ed531ba731a13a7c421c2d`), written at checkpoint C5 by
    the `fable-solver` agent and never shown to any other agent. Five of section 7's classes cannot be planted as sample data and
    are listed as `not_planted`.
  - Caught: the driver caught 17 of 20 and the researcher subagent 8 of the 8 faulted runs in its packet; together 18 of 20, over
    the bar of 16 (0.8). Neither raised a false alarm on the two clean runs.
  - Replay: the nine words of seed 7001 replay through the host's engine to the samples' digests, 9 of 9.
  - Missed by both: one defect on a driver rule (`blocked-undefined`) that the packet's draw did not include, so only the driver
    could catch it; and one on a rule outside the driver's seventeen (Global's total N is the state's own seam count, never the
    sum of the caps' N_c), which also was not drawn into the packet. The first is a driver gap to close; the second is a rule to
    add to the driver.

## The researcher subagent

- Inputs: only the packet (ten unlabelled runs, `steps.json`, `rules.json`), copied alone to a fresh folder, and the palette's
  rows (`command-rows-d02.json`, `command-labels.json`), the key map (`keymap.json`) and `jumbling-observation.md`. It saw no
  sample, source, test, defect list, run folder or key.
- Model: the session's default model (Claude Opus 5.5), not a smaller model as section 7 says, because the owner's rule of
  27 November 2026 is that subagents never run on a smaller model. A stronger reader makes `caught_subagent` an upper bound
  for a weaker one.
- Effort (Opus's count) against section 7's cap: 0 of 2 scripts, 10 of 12 dumps, about 7 of 90 minutes, about 520 of 2,000
  words. It printed fields of the listed files with read-only one-line commands and wrote no key script.
- Reach: not run. The subagent's reach half (one key script over at most 30 display names) has no driver function in this build.

## What this does not show

The framework's list stands (section 7, "What it does not prove"): the aids' facts come from samples until K2; J2 to J4, the
target of ADR-D02.55 and the researcher agent of ADR-D02.51 are not run; keys are injected inside the shell; nothing here is
human, input-latency or four-view evidence.

## Records

The run folders, the packet, the answers, the effort count and the score are kept outside Git in
`work/adr-d02/track-j/` (`j0-run2-20261127/`, `researcher-pass-20261127/`); the sealed key stays in the run's `j0/sealed/`.
