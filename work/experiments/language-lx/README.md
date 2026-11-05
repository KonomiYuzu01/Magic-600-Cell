# Language experiments LX-1 to LX-9

Experiment code and records for the C600 Script spikes of `docs/architecture/1.0/language/architecture/11-route-and-spikes.md`
(chapter 11). Nothing here is product code. Nothing here selects a route, an engine language or a syntax.

Authority: [owner decisions of 5 November 2026](../../../docs/wiki/decisions/owner-decisions-2026-11-05.md), the owner's
allowance under LQ18 for what can run now. It sets no numeric time box, allows no installation and no source outside the
allowlist, and uses subscription quota only. A reached limit is no pass (RTE.35).

## Common rules

- Each experiment owns one folder here and leaves the products of RTE.37: frozen card, inputs or a digest manifest,
  editable source, production commands, tool versions, raw observations and a report (`RESULT.md`) with negative results.
- A report names its evidence class (source, synthetic, windows, performance, human) and exact source and build scope.
  Headless or sandboxed timing is a lead. Decisive timing comes only from the owner's machine under the B4-12 method
  (one smoke run, then three series of 100 samples) with its D6 conditions recorded exactly: power source, Windows power
  mode, vendor performance mode, driver V-Sync and overlays.
- `RESULT.md` carries CT-RTE-05's fields: LX card, experiment ID, owner allowance, time box, frozen candidate and input
  digests, commands and tool versions, evidence class, kill criteria, actual result, and the decision with its decider
  (adopted, rejected or open). Decisions in this round stay open for the owner.
- Fresh isolated data only. No personal session, no network, no third-party dependency, no installation.
- Tools in use are already on the owner's machine and sit outside `tools/toolchain.lock.json`, so every report pins
  their versions. Using an installed tool is not an install.

## Cards

The card of each experiment is its table and decision rule in chapter 11, unchanged, plus the additions below (RTE.35).
Time box for every row: "what can be done now", at most one wrapper call of two hours per packet.

| Experiment ID | LX | Card in chapter 11 | Allowed now | Waits for | Evidence class now |
| --- | --- | --- | --- | --- | --- |
| E-2.4-L1 | LX-1 | RTE.38 to RTE.40 | Counters on the LX-3 prototypes, as a lead with the host named | LQ8 for the decisive host (RTE.29) | source; timing a lead |
| E-2.4-L2 | LX-2 | RTE.41 to RTE.43 | Conditions 1 and 2, kernel round trips and transfers, headless, as a lead | LQ8, LX-1's mix, a renderer candidate and a foreground window for conditions 3 and 4 and first-frame startup | source; timing a lead |
| E-2.4-L3 | LX-3 | RTE.44 to RTE.46 | All forms the available hosts allow | Luau, Lua, Rhai and WebAssembly candidates need a source outside the allowlist | source, synthetic where inputs are constructed |
| E-2.4-L4 | LX-4 | RTE.47 to RTE.53 | Module generator, seeded edit script, headless front-end lead | LQ8, an editor path, a renderer candidate, PresentMon and an agreed foreground window | source; timing a lead |
| E-2.4-L5 | LX-5 | RTE.54, RTE.55 | Candidates already on this machine | Third-party big-integer crates need the owner's yes | source; performance only under the B4-12 method |
| E-2.4-L6 | LX-6 | RTE.56 to RTE.58 | Rust port on the standard library, O01 to O07 | Nothing for correctness | source; performance only under the B4-12 method |
| E-2.4-L7 | LX-7 | RTE.59 to RTE.61 | Subset candidate, reference fixtures, generated stage, no-window named-device comparison | RPI.36's metric before any judgement; a foreground window for W3 | source and synthetic; windows for the device comparison |
| E-2.4-L8 | LX-8 | RTE.62, RTE.63 | Equivalent candidates and source/IR fixtures | The owner's protocol for any human run | source |
| E-2.4-L9 | LX-9 | RTE.64 to RTE.66 | Both steps on Windows CPython, Linux CPython and the Rust candidate's float path | Nothing | synthetic |
| E-2.4-LE | PS1.38 | PS1.35 to PS1.38 | Facts-option envelopes for the stage 14 to 19 corpus | LQ3 and chapter 06 for the judgement option | source |

Folders: `lx1-cost/`, `lx2-placement/`, `lx3-sandbox-flow/`, `lx4-editor/`, `lx5-arithmetic/`, `lx6-engine-port/`,
`lx7-charts/`, `lx8-surfaces/`, `lx9-input-map/`, `envelope-facts/`.

## Checks at a later commit

Some experiments pin chapters, cards or decision pages by digest, so their `check.py` refuses to run once those documents
move: an old result cannot be relabelled. `verify_frozen.py` reproduces such a check at the commit it was frozen on. It makes a
temporary local clone, lays the committed folder over it, runs the check there, fails if the check changed a file of the
folder, and removes the clone:

```text
python work/experiments/language-lx/verify_frozen.py <folder>
```

The frozen commit must stay reachable, so the branch that carries these experiments merges with a merge commit, never a
squash. `envelope-facts/` also reads Git objects at that commit.

## Machine

Owner's laptop: AMD Ryzen 9 7945HX, 32 logical processors, about 32 GB RAM, RTX 4070 Laptop GPU (8 GB), Windows 11 Home
10.0.26200. Linux leg: WSL2 Ubuntu, CPython 3.14.4, no NumPy. Reports record the machine state of each run.
