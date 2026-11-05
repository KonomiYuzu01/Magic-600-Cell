# E-2.4-L3 part A: neutral inputs before candidates

- LX card: LX-3, chapter 11 RTE.44 to RTE.46; CT-RTE-05 and CT-RTE-06.
- Experiment ID: E-2.4-L3, part A; corpus `lx3-neutral-v2`.
- Owner allowance: `docs/wiki/decisions/owner-decisions-2026-11-05.md`, decision 2;
  the assigned corpus-only implementation packet. No new installation or cost.
- Time box: **what can be done now; one wrapper call of at most two hours**.
  A reached limit is no pass.
- Candidate identity and candidate digest: **none**. No candidate implementation,
  parser, interpreter, route, syntax or real engine was created or executed.
- Frozen input digests: `FREEZE.json` lists every corpus file, including the
  normalized manifest self hash; `keys.json` pins source documents, input files
  and neutral run keys. Candidate DET.1 bindings remain explicitly absent.
- Commands/tool versions: CPython 3.15.1, standard library only. Commands below.
- Evidence class: source/structure and synthetic fixture data only. The expected
  values and refusal predictions are specifications, not measured results.
- Kill criteria: malformed inputs, unknown/reserved/retired expected codes,
  missing family/category coverage, changed source/input/key digests or a freeze
  mismatch fail the check. A later candidate retains RTE.44's mismatch, planted
  access, unrefused case, wrong code, crash, hang and timeout failures; no omitted
  case becomes a pass.
- Decision: **open**. Decider: **owner**. LQ2, LQ9, LQ21 and BS1 to BS4 remain open.

## Version 2: same cases, new source pins

Version 1 was frozen against the text at `1e7d0074`. Commit `f73443b8` then changed 4 of its
pinned source documents (`07-boundary.md`, `11-route-and-spikes.md`, `13-errors.md`, `ecosystem-partition.md`), so its plain check
refused before any candidate existed. That edit closed text gaps: none of it changes an RTE.45 family,
an expected code or the denominator.

Version 2 changes three things and nothing else: the version string; the source pins, which now name the
text at `f73443b8`; and the plain check, which now reports a moved source document instead of refusing,
because the chapters keep changing. Case, fixture, policy, profile, notation and stub content is unchanged.
Codes and clauses are still checked against the live text, and any corpus file change still needs a new
version. No candidate had run when this was done, so no observed result influenced it. Version 1's manifest
`content_sha256` was `02c6236606e15a543719982ce729c654035e5bcef6f7b702f5be61b5aa0ce8bf`; its files stay in Git history.

## Contents and fixed scope

There are 20 must-refuse cases, two for each of all ten RTE.45 families, including
one generic F2 correction solve and one F5 solve. The harmless denominator is 35
tools, five in each requested category. Four planted no-grant resources, fourteen
budget/interruption/cache cases, and two approved-subset/byte-binding probes are
also frozen. The fixture is fixed fake data with no real engine or personal data.

The policy uses the interim reading. Dedicated flow codes remain reserved under
ERR.24, so interim policy denials use `LG3:capability.not_granted` with the gap
recorded. Unknown stored provenance and module substitution use their existing
specific codes. Stop/emission and accumulation cases remain must-refuse even
where the unmodified label forms are not expected to catch them. An early policy
denial does not establish that deeper propagation was exercised.

## Actual result

Input validation passed with exit 0 using `--validate`: 82 neutral run keys
recomputed, every expected code resolved to an active chapter 13 registry row,
all ten RTE.45 families had two cases, and the denominator was 35. Ten deliberately
invalid in-memory mutations were rejected, including missing family coverage,
reserved/unknown codes, a changed denominator, bad references, a granted planted
resource, an invalid precision and a setup that moved its buffer. The initial
freeze is produced and checked by the acceptance command below; its final stdout
is the acceptance record. No candidate or cross-platform result is claimed.

The check creates no build directory, bytecode, binaries, external temporary
files or executable candidate. Its negative schema checks mutate copies in
memory only. Initial keys and the initial freeze are its only generated files.

## Commands

Initial key preparation, before a freeze exists:

```text
python work/experiments/language-lx/lx3-sandbox-flow/corpus/check_corpus.py --prepare-keys
```

Structural verification run here (exit 0), before sealing:

```text
python work/experiments/language-lx/lx3-sandbox-flow/corpus/check_corpus.py --validate
```

Acceptance (recomputes the full manifest and checks an existing freeze without
overwriting it):

```text
python work/experiments/language-lx/lx3-sandbox-flow/corpus/check_corpus.py
```

## Open and not run here

The integrator's headless Linux corpus revalidation is **not run here**. Its exact
command, from the repository root, is:

```text
python work/experiments/language-lx/lx3-sandbox-flow/corpus/check_corpus.py
```

Candidate rendering, the actual stub, per-form refusal and false-refusal results,
ten Windows and ten headless Linux executions per full key, cache/interruption
enforcement, and the approved-subset variants are **not run here**. There is no
candidate runner yet, so no executable candidate command can honestly be named.
Part B must freeze those exact commands and the missing candidate bindings before
execution; the corpus cannot be changed to accommodate a result. This is the
intended boundary of part A, not unfinished candidate implementation in this run.

The packet's network and write boundary exclude nested review-wrapper calls;
integration and independent review remain with the integrator. No commit, index
change, fetch or other Git mutation was made.

This run cannot show **all-program safety, authorship, understanding, absence of
covert channels, product latency or an acceptable false-refusal threshold** (the
card's Cannot show row). It also cannot show actual Windows/DirectX, engine
correctness, a route pass, fault containment or exact cross-platform execution.
The BND.51 stop fix, display policy for own tools, LP1 meter and eventual active
flow code remain open. Fixture counting/preview-replacement conditions do not
decide those product contracts.
