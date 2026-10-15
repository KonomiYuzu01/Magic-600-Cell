# 03. The retained group equals its invariant bound, and a lattice-phase restorer (13 October 2026)

This stage answers items 2 and 3 of the owner's list. The main document is [report.md](report.md), which gives the
theorems, proofs, data tables and limits.
- The proofs and constructions are the integrator's, with no external review.
- The two checkers share no code with the construction code.
- Everything is an exact headless computation on Linux, on the retained model and the exact jumbling simulator.

## Item 2: card 1's certificates, and G = 𝒦

[Card 1](../../theory/cards/card-1-retained-group.md) reduces the question "is the retained group the whole kernel
of its invariants?" to explicit certificates. This stage constructs all of them.
- **The certificates.** Explicit words for all 35 position certificates P_o and all 11 frame certificates F_o. They
  form one straight-line program of 4,788 nodes.
- **The independent checker** `checker/check_certs.py` passes every check:
  - R1, rigidity;
  - R2, K⁺ closure (order 7,200);
  - H, group types, 35 of 35;
  - P, 35 of 35;
  - F, 11 of 11, A5 orbits with explicit isomorphism tables;
  - F1: ν = 0 for all 1,200 generators in the same coordinates.
- **Tamper self-test.** 4 of 4 tampered certificates are rejected (`certificates/selftest.log`).
- **Consequence: G = 𝒦, without hypothesis C.** This uses the accepted card-1 theorem: certificates and F1 imply
  G = 𝒦.
  - The order of the retained group is |G| = ∏_o N_o! |H_o|^{N_o} / (2 |H_o^ab|), a number with 598,061 decimal
    digits.
  - Its index in the full wreath product is 2^43 · 5^2.

## Item 3: the lattice-phase reference restorer

The restorer reads only the current configuration, as a pose table or as sticker labels; it reads no log or history.
It works in four steps:
1. check that every piece is on the lattice;
2. compute the invariant ν: 35 sign bits, 8 C2 frame bits and 2 C5 frame values;
3. solve the correction equations over F2 and F5 with I_a correction macros;
4. solve the retained part with the retained model's cascade.

**Theorem R** (section 3.2 of the report): the restorer either solves the configuration or returns an exact failure
certificate. The certificate holds ν, the library rank and the residual.

Results at this stage:
- **The library.** It had 13 macros:
  - 11 conjugated mirror half-turns of 5 or 9 twists, w9 among them;
  - one pure frame macro of 228 twists (O11);
  - one 332-twist macro (f11 plus the O12 sign).
- **Restored.** 8 of 8 inputs were restored to the identity: 2 random elements of 𝒦 (not uniform on 𝒦: report
  section 3.4), 1 random word, 2 with w9 and 3 mixed with library macros. The independent checker
  `checker/verify_restore.py` passes all of them in 130 s.
- **Outside the span.** For the reachable states E3, I1 and I2 the restorer returned failure certificates: their ν
  lies outside the span of this library. Stages 04 and 09 close this:
  - stage 04 extends the library to span the whole lattice correction group 𝒜_J = C2^34 × C5^2;
  - stage 09 restores E3, I1 and I2 to the solved state.
- **Card 5.** Questions 3.2, 3.3 and 3.4 are answered "no" (report section 3.5). Question 3.1 was open here and is
  answered in stage 04.

## Replay

`$ROOT` is the repository root and `$J` is `research/jumbling`. Decompress the `.gz` files first.

    python3 -I checker/check_certs.py $ROOT certificates/certificates.json <report.json>
    python3 -I checker/selftest.py $ROOT certificates/certificates.json <work dir>
    python3 -I scripts/restore_lattice.py scripts $ROOT $J restore/library-final.json <out dir> <inputs...>
    python3 -I checker/verify_restore.py $ROOT $J restore/library-final.json <out dir> <report.json>

Section 4 of the report has the full commands, and `restore/commands.txt` the arguments of every run.

## Files

| Path | What it is |
| --- | --- |
| `report.md` | Theorems, proofs, tables and limits |
| `certificates/` | The certificate program `certificates.json`, its check report, construction log, seed survey and tamper self-test |
| `checker/` | `check_certs.py`, `selftest.py`, `verify_restore.py`: independent of the construction code |
| `scripts/` | Construction, library searches and the restorer `restore_lattice.py` |
| `restore/` | The library, restore summary, solutions, verify report and the searches that built the library |

Not stored: the start-label arrays (`.npy`), which the restorer rebuilds from the inputs, and the Chinese delivery
note. Data files of 256 KiB or more are stored gzipped; see `IMPORT-MANIFEST.json`.

## Limits

- The cascade is machine-scale (about 1.9 to 2.3 × 10^8 twists). It is a reference restorer and a correctness
  criterion, not a method for people.
- Not run: Windows, DirectX, the native host regression and performance measurements.
