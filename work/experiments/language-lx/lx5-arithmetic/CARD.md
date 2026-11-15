# LX-5 exact arithmetic: E-2.4-L5

Experiment code only. Decision: **open**. Decider: **owner**. No route, engine language, syntax, runtime placement,
dependency, licence or LQ is selected. Scope is this folder alone; no network, installation, package or crate.

This implements chapter 11's RTE.54/RTE.55 card under the allowance in
`docs/wiki/decisions/owner-decisions-2026-11-05.md`, decision 2, and the assigned implementation packet.
The card's question, method, kill criteria and limits are unchanged.

| Field | Frozen experiment card |
| --- | --- |
| Question | Which exact-arithmetic candidates meet KN3 at the recorded and synthetic sizes, with what dependency obligations and measured costs? |
| Decision it feeds | LQ8; arithmetic integration obligations for LQ9. Both remain open. |
| Hypothesis | Unbounded pose heights and recorded large lengths require arbitrary precision; no library speed or licence advantage is established. |
| Candidates | Node BigInt; .NET System.Numerics.BigInteger; own Rust base-2^32 signed limbs using the standard library. Python integers are the reference. |
| Method | Freeze operations, operands, Python reference answers, Q(sqrt(5)) fixtures and the RTE.33 size corpus. Construct large entries from the J1 negative control; compare exact canonical bytes; record dependency terms. |
| Time box | what can be done now; one wrapper call of at most two hours |
| Kill criteria | Any exact-result difference removes that candidate globally; keep operands and operation. Timeout leaves untested sizes open. A reached limit is no pass. Proposed copyleft dependencies go to the owner. |
| Evidence class | Source for recorded fixtures and inspected notices; synthetic for generated operands/entries/streams. Smoke timing is a lead on a shared sandboxed machine; no performance claim. |
| Cannot show | That synthetic entries occurred in a saved state, that the largest length was played, a general speed threshold, or owner approval of a dependency. |
| Product | Frozen fixtures and recipes; candidate sources; exact comparer; complete per-size/operation reports; smoke/full measurement entry point; licence inventory. |

The acceptance command is run from the repository root:

```text
python work/experiments/language-lx/lx5-arithmetic/check.py
```

It verifies a deterministic regeneration of the frozen fixtures, independently self-tests the Python reference and
comparer, compiles the candidates locally, compares all answers, runs Rust's additional self-test and invokes
`measure.py --smoke` in a separate process. It prints each candidate's status and the passing list. A completed
negative numeric experiment may exit 0 **with the differing candidate removed**; this is a complete report, not a
claim that every candidate passed. Missing execution, fixture drift, reference failure, malformed timing data or
build failure exits nonzero. No speed threshold applies. `build/` is removed in `finally`, including on failure.

`SOURCE-MANIFEST.json` freezes candidate/harness/card/licence identities before each run and is rechecked afterward.
`check-results.json` binds that snapshot, the fixture manifest, tool versions and executable digests. Raw candidate
answer files are temporary; their digests and the command above suffice to regenerate them. The canonical reference
is stored as deterministic gzip (level 9, mtime 0) to keep the checked-in corpus small; its decoded digest is also frozen.

The input protocol is ASCII TSV with LF endings. `value <id> <decimal>` defines an integer; then each
`case <id> <size> <operation> <m-or-dash> <operand-ids...>` uses those values. Only `word` takes a relative stream
filename. All candidates read the same file. `m` selects the first representative operation/size case for measurement.
An answer is `<case-id><TAB><canonical-result><LF>`, in frozen case order, without timing, host, path or process fields.
Decimal integers have no leading zero or plus sign. Hex is sign plus lowercase magnitude, no prefix, followed by
`|` and the decimal value parsed back. Rationals use `numerator,denominator`; Q(sqrt(5)) uses `a,b,d` for
`(a+b*sqrt(5))/d`. Denominators are positive, the tuple's common gcd is one, and zero normalizes to denominator one.
Division/remainder use Python's floor convention, right shift is arithmetic, and bit length uses absolute magnitude.

The frozen operation list is add, subtract, multiply, floor divide, remainder, gcd, compare, left/right shift,
bit length, decimal/hex round trip, normalized rational add/multiply/compare, Q(sqrt(5)) add/multiply/conjugate/norm/sign,
exact length sum and stream work. Q(sqrt(5)) sign uses signs and the integer comparison of a^2 with 5*b^2, never a float.
The corpus includes all quotient sign combinations, zero, exact division, limb boundaries, negative denominators,
shared factors and Pell near-cancellation cases with independently known norm +1 or -1. Undefined division by zero
is outside this frozen valid-operation corpus; this spike defines no product error contract.

Size inputs cover 2^64, 2^256, a 22,500-bit integer and a **synthetic** 6,775-decimal-digit integer (not the actual B11
value). The pose recipe extracts the largest-component entry from exact powers `(g H)^n` of the first two J1
negative-control journal matrices, in chronological column-vector order. It checks all twelve recorded heights before
extending to 70, 260, 3,022 and 22,504 bits. The X1 H6030 figures of 3,018/3,036 bits motivate additional tests; H6030 is
not replayed. `pose-recipe.json` records matrices, rounds, entries and the source digest. Every constructed pose entry
remains synthetic; the stale table-size sentence in KN3.2 does not erase the larger recorded H6030 witness.

Stream results are `length|sum-of-letter-codes|SHA256-of-raw-ia16|code:count,...`, with all nonzero counts in ascending
16-bit code order. This compares the full histogram, not merely a histogram hash. Each candidate reads 64 KiB chunks;
its digest and counters span the entire file. Per-chunk sums and per-letter counters use bounded native arithmetic
with no possible overflow at this frozen corpus's sizes; total length and code sums accumulate in arbitrary-precision
integers. Native timing counters are separate from exact arithmetic results and logical charges.

`word-recipes.json` binds all 53 compressed source files and their raw digests. They concatenate into **one**
13,231,266-letter stream without state resets; word 00014 is also consumed alone. Prefixes of word 00014 cover the
672,417 and additional 871,582 literal sizes, and are explicitly not the earlier E2/T6 words. The 10^7-letter stream is
xorshift32 with seed 24581 and letter `state % 36000`. The 166,788,405 size is length arithmetic only. No long word is
mechanically replayed and no compressed/composed word is silently replaced by a claimed legal execution.

Rust is own code with audit burden. It uses schoolbook multiplication, normalized Knuth Algorithm D division with a
linear one-limb path, and Euclidean gcd. Multi-limb division costs O((n-m+1)*m) 32-bit limb work for dividend/divisor
lengths n >= m >= 2, plus normalization; scratch space is O(n+m). Rust's self-test compares 512 seeded native i128
cases, three SHA-256 known vectors and 2,904 frozen Python random/limb-boundary cases, including 22,500-bit operands.
It does not turn this implementation into a dependency recommendation or a universal correctness proof.

Production and verification commands (run from the repository root):

```text
python work/experiments/language-lx/lx5-arithmetic/make_fixtures.py --write
python work/experiments/language-lx/lx5-arithmetic/make_fixtures.py
python work/experiments/language-lx/lx5-arithmetic/licences.py
python work/experiments/language-lx/lx5-arithmetic/check.py
python work/experiments/language-lx/lx5-arithmetic/measure.py --smoke
```

`--write` explicitly establishes a new input identity; ordinary acceptance never overwrites mismatching frozen data.
The runner invokes Roslyn's `csc.dll` directly with the installed .NET reference assemblies, and invokes `rustc`
directly with `--edition=2021 -O -C debuginfo=0`. It discovers the installed native linker with vswhere on Windows.
No cargo, NuGet, project restore, network or compiler server is used. Temporary, .NET home and compiler scratch
directories stay in this folder's `build/tmp/`; no personal state is opened. Source and text output use LF.

Measurement excludes startup, operand parsing and answer-file writes; it includes arithmetic and canonical result
formatting (word work also includes stream I/O). One warmup precedes each case in each series. Smoke is 1 x 3 samples;
full is 3 x 100. Median is the standard sample median; p99 is nearest rank `ceil(0.99*n)`, computed per series. The
first representative of each operation/size is measured, not every sign/control variation. Random cases are correctness
only. A smoke p99 is just the maximum of three observations, not tail-latency evidence. Full raw sample files are not
retained; their digests, per-series summaries and regeneration command are retained.

Full mode and Linux/WSL are **not run here**. Integrator commands, with matching installed tools and frozen files:

```text
python work/experiments/language-lx/lx5-arithmetic/measure.py
python3 work/experiments/language-lx/lx5-arithmetic/check.py
```

Before any decisive owner-machine run, record D6 power source, Windows power mode, vendor mode, driver V-Sync,
overlays and frame generation. `measure.py --conditions work/experiments/language-lx/lx5-arithmetic/measurement-conditions.json`
can attach a public JSON record of those fields; the file is deliberately not fabricated here. The measurement program
always labels its output as a lead; a performance claim still needs integrator review of conditions and evidence.
The smoke entry point is exercised here; full sampling itself remains untested. GPU, renderer and mechanical replay
are outside this card. Review before integration remains the integrator's responsibility.
