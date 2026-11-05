# LX-9 Windows result

> **Integrator note, 5 November 2026.** This report covers the Windows legs only. The Linux leg ran afterwards, and six
> requests chose a different exact rotation there. Read `LINUX-COMPARISON.md`, `AMENDMENT.md` and `followup/NOTE.md`.

No different exact choices were observed on these Windows requests. This is a finite-sample result only; the amendment decision remains open.

| CT-RTE-05 field | Record |
| --- | --- |
| LX card / experiment ID | LX-9, RTE.64 to RTE.66 / E-2.4-L9 |
| Owner allowance | `docs/wiki/decisions/owner-decisions-2026-11-05.md`; assigned implementation packet |
| Time box | what can be done now; one wrapper call of at most two hours |
| Evidence class | synthetic; actual Windows CPython/NumPy and Rust execution, no native input or performance claim |
| Frozen candidate digest | `CANDIDATE.json`: `a4d96e90952ca867c0dfe565a175c0edc5036d51aa4f44c0dd3886f401f0ca7e` |
| Frozen input manifest digest | `MANIFEST.json`: `656212223fd3ce5d35a68512ab793fc300e409683b0abb00198eef9c972c1677` |
| Step 1 input digest | `70e0180cc54625b53bd91565985a657b7e71ef92131a5db74615c22421ab87de` |
| Step 2 input digest | `2e24177cf609bbc3955834e5dfd563d2d2909b257ecf7d2297bcca0e19614902` |
| Commands and tool versions | Below; all detailed raw-output digests and controls in `WINDOWS.json` |
| Kill criteria | Different exact rotation stops an unqualified cross-runtime claim and prepares amendment evidence; target-only differences feed step 2; timeout leaves work open |
| Actual result | All 13,600 step-1 requests and all 2104 step-2 requests ran in all three primary Windows legs; acceptance passed |
| Decision with decider | **open**, owner; no route, language, syntax, LQ answer or contract change |

## Comparisons

| Step | Requests | Pair (left/right) | Target-bit differences | Exact rotation differences | Raw parameter differences |
| --- | --- | --- | --- | --- | --- |
| 1 | 13600 | reference/pure | 0 | 0 | 0 |
| 1 | 13600 | reference/rust | 888 | 0 | 0 |
| 1 | 13600 | pure/rust | 888 | 0 | 0 |
| 2 | 2104 | reference/pure | 0 | 0 | 0 |
| 2 | 2104 | reference/rust | 948 | 0 | 0 |
| 2 | 2104 | pure/rust | 948 | 0 | 0 |

Step 2 contains 1,216 certified near-bisector requests plus
888 unchanged requests carried from step 1.
The carried requests are not counted as near-bisector samples. All pairwise
differences are retained in `differences.tsv`, the request sets and the small
compressed result tables. Counts across pairs/steps must not be added as unique
requests. Exact keys are primitive projective integer quaternions; raw parameter
comparisons additionally control the tie order.

## Port and norm controls

| Step | Same-interpreter pure target differences | Same-interpreter pure parameter differences | Unfused Python target differences | Rust FMA target differences vs reference | Rust FMA rotation differences vs reference |
| --- | --- | --- | --- | --- | --- |
| 1 | 0 | 0 | 888 | 0 | 0 |
| 2 | 0 | 0 | 948 | 0 | 0 |

The bit-for-bit pure control runs inside the reference interpreter on every
request. NumPy's short dot products are reproduced with sequential FMA;
ordinary loop summation has the measured differences above. The primary Rust
candidate deliberately uses ordinary separate multiply/add; the extra Rust FMA
leg changes only the two norms. Its remaining differences, if any, are recorded
above and must not automatically be attributed to trigonometry. Both Rust modes'
exact parameter choices were independently re-scored with Python integers on
every emitted target, with zero mismatches. Equal target bits also required
identical raw parameter choices between all primary legs.

The reference uses the unchanged NumPy screening search with
`INPUT_SEARCH_MARGIN = 1e-12`; the ports exhaust their bounded candidate sets.
The frozen construction verifies the half-turn boundaries exactly in Q(sqrt(2))
and the angle bounds with Fraction arithmetic. Acceptance rechecks construction
and digests, includes every step-1 difference, and removes `build/` in `finally`.

## Reproduction

Run from the repository root. No network, installation or personal data is used.

```text
python work/experiments/language-lx/lx9-input-map/check.py
```

Tool versions: Windows standalone CPython 3.15.1;
reference CPython 3.14.7, NumPy 2.3.5;
rustc 1.98.1 (48a229cea 2026-09-01); target x86_64-pc-windows-msvc.
Rust command (the check supplies TMP and TEMP under its local build folder):

```text
rustc --edition=2024 -C opt-level=2 -C debuginfo=0 -C panic=abort work/experiments/language-lx/lx9-input-map/rust_input_map.rs -o work/experiments/language-lx/lx9-input-map/build/float-map.exe
```

Initial production was `python work/experiments/language-lx/lx9-input-map/make_requests.py`, then
`python work/experiments/language-lx/lx9-input-map/check.py --freeze-step2`. That one-time second command freezes
the dependent inclusion list and candidate sources. It refuses to overwrite a
frozen step 2. Normal acceptance verifies the frozen inputs and sources. Sources,
bit-encoded requests and compressed result tables are retained; each raw table
is under 2 MB, and its uncompressed digest is recorded. The extra Rust FMA raw
tables are regenerated by the check and removed; their digests are retained.
Elapsed acceptance time is a sandbox lead only, not a benchmark.

Run record: the first run's acceptance passed and was refused only because its check rewrote `elapsed_seconds_lead_only` in `WINDOWS.json`; this re-run preserves the same work with a check that writes nothing outside `build/` and removes it before exit; this run updated the source digest pin for `check.py` only.

## Not run here and open

Linux CPython 3.14.4 in WSL is **not run here**. From a Windows shell at the
repository root, the integrator must run exactly:

```text
wsl.exe --exec python3 -B work/experiments/language-lx/lx9-input-map/linux_leg.py
python -B work/experiments/language-lx/lx9-input-map/compare_legs.py
```

The Linux script requires only the Python standard library. It writes the same
result format plus a manifest, retains every new Linux step-1 difference by the
frozen inclusion rule, and runs those requests in step 2. The comparison uses
Windows step-1 exact choices for any newly included requests. It records counts
and all difference IDs and updates amendment evidence if Linux adds a rotation
difference. No cross-platform agreement is claimed before those commands run.

R2-E2 records no seed or axes. The 13,600 requests here reuse only its stated
construction, with the explicit replacements in `CARD.md`; they are **not** a
byte-identical reconstruction of its historical corpus. N=1/D=4 in step 1 and
N=1/D=4 or 16 in step 2 bound this result. Product-default N=16/D=1000, world-frame
input, the proposed 60-candidate menu-mode drag chooser, Linux and native input
remain open. The Rust source is an LX-9 candidate using the specified LX-6
toolchain, not evidence that an LX-6 engine implementation was executed.

Cannot show: equality for every request from a finite sample, input latency, or
an approved change to state contract A1. Replay of recorded rotations is a
different test. No state contract, model, product code, journal or user session
was changed. Integrator review and any EN1 Astra/owner amendment decision remain
outside this packet. No decision field is closed by a passing harness.
