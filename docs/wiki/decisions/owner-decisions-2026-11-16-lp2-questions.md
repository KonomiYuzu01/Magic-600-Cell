---
id: owner-decisions-2026-11-16-lp2-questions
type: decision
status: verified
visibility: public
summary: Owner decisions of 16 November 2026 on three LP2 questions - M8's headless Linux leg runs as a GitHub CI job on ubuntu-24.04 (cargo test --locked --offline in rust/), installing nothing on the owner's machine; the jumbling engine stamps the state contract pin state-contract 2026-11-15 A1-A6, while J1's A1-A4 stamp is recorded as a known J1 defect and mapped only in the differential comparison; and one J1 file, research/jumbling/sim/accept.py, may change so that its process-pool workers receive their shared inputs as arguments, which lets J1's acceptance run on Windows with its results unchanged.
related: [owner-decisions-2026-11-16-en3-engine-half, owner-decisions-2026-11-16-time-saving, owner-decisions-2026-11-14-language-route, owner-decisions-2026-11-03]
supersedes: []
claims:
  - {id: lp2-linux-leg-on-ci, evidence_kind: decision, checked_at: 2026-11-16}
  - {id: lp2-contract-stamp-a6, evidence_kind: decision, checked_at: 2026-11-16}
  - {id: lp2-accept-py-exception, evidence_kind: decision, checked_at: 2026-11-16}
  - {id: linux-rust-workflow, evidence_kind: source, path: .github/workflows/linux-rust.yml, checked_at: 2026-11-16}
  - {id: engine-contract-stamp, evidence_kind: source, path: rust/c600-jumble/src/lib.rs, checked_at: 2026-11-16}
  - {id: accept-pool-inputs, evidence_kind: source, path: research/jumbling/sim/accept.py, checked_at: 2026-11-16}
---

# Owner decisions, 16 November 2026: three LP2 questions

The LP2 integrator put these questions on the one question card of the milestone, with the recommended option first,
as the [time-saving decision](owner-decisions-2026-11-16-time-saving.md) provides. The owner answered in chat in
Chinese on 16 November 2026, each time with the recommended option. This page records the answers in English.

## 1. M8's headless Linux leg runs on GitHub CI

**Question.** The LP2 plan's M8 row asks for PHS.19's acceptance on Windows and headless Linux. The owner's machine
has no Rust toolchain for Linux, and the standing instruction was to install nothing new. The options were a GitHub
CI job on ubuntu-24.04 that runs `cargo test --locked --offline` in `rust/` (recommended: the repository is public, so
the job costs nothing, and nothing is installed on the owner's machine); a pinned Rust installed in WSL, which is
outside the allowlist and needs the owner's approval; or leaving the Linux leg to LP3.

**Answer.** The CI job: "yes, run it as soon as possible".

**Carried out.** `.github/workflows/linux-rust.yml` runs on every pull request, on every push to `main` and by hand.
It checks out the committed bytes without credentials, prints the runner's Rust toolchain and runs
`cargo test --locked --offline` in `rust/`. The approved crates are vendored, so the tests need no network after the
checkout, and the runner's preinstalled Rust is used, as on the Windows shards. A pass is headless Linux evidence
only: it is not Windows/DirectX, input, long-session or performance evidence. The headless Linux legs of CT-DET-02,
CT-DET-15 and CT-PS1-01 now run there against the same pinned bytes as the Windows shards, so CT-PS1-01 counts as
whole in section 15.3's index (63 whole). The Python comparisons of the Rust candidate with J1
(`tests/test_jumbling_rust_*.py`) still run only on the Windows shards of the headless checks.

## 2. The engine stamps the state contract pin A1 to A6

**Question.** JI.3 names the contract pin `state-contract 2026-11-15 A1-A6`, but J1 and the Rust candidate engine
`c600-jumble` stamped `state-contract 2026-10-09 A1-A4`. The options were to change the engine's stamp to A1 to A6,
record J1's A1 to A4 as a known J1 defect and map it in the differential comparison (recommended), or to keep A1 to A4
until the research restarts.

**Answer.** "Go with A1 to A6."

**Carried out.**

- `CONTRACT_REVISION` in `rust/c600-jumble/src/lib.rs` is now `state-contract 2026-11-15 A1-A6`. The jumbling identity
  block (KN1.4), the repair list's parameters and the adapter's model report carry it. The kernel crate stamps no
  contract revision.
- J1 is not edited for this. Its stamp `state-contract 2026-10-09 A1-A4` (`research/jumbling/sim/model.py`) is a
  known J1 defect. The state contract itself already says that J1 implements A1 to A4 and stays the reference for
  sections 1 to 4 and for menu letters under A1 to A6.
- Only the differential comparison maps J1's stamp to the engine's: `engine_stamp` in
  `tests/test_jumbling_rust_m8_observations.py`, which leaves every other value unchanged, so a change of either stamp
  still fails the comparison. At the candidate, KN1.4 refuses J1's stamp like any other foreign stamp
  (`StampTests` in `tests/test_jumbling_rust_attacks.py`).
- No frozen file under `docs/architecture/1.0/language/` names either stamp, so no frozen text changes (PHS.52).

## 3. One J1 file changes so that J1's acceptance runs on Windows

**Question.** `research/jumbling/sim/accept.py` exits with code 1 on Windows. Its process-pool workers read module
globals that only a forked worker inherits, and Windows starts its workers by spawning fresh processes. The options
were to allow a change to that one file so that the workers receive their inputs as arguments, with unchanged
results (recommended: the change is small and touches research code only), or to record a known failure until the
research restarts. The card spoke of one global table; in fact the workers read four module globals: the retained
slot moves, the shared input of the independent recomputation, the comparison state and the menu controls. The scope,
one file with unchanged results, is the same.

**Answer.** "Yes."

**The exception.** The research stays stopped ([3 November 2026](owner-decisions-2026-11-03.md)). This answer allows
a change to `research/jumbling/sim/accept.py` alone. No other J1 file changes. J1's other known defects, which the M5
attack tests found (in `word_check.py`, `sim/api.py`, `pose_entries` and the API scan), stay as expected failures.

**Carried out.** A pool initializer, `_share`, receives the four inputs as arguments. The comparison state travels as
its journal and is replayed in the worker (`State.replay`), and the menus travel as their records
(`TwistMenu.from_record`), because a J1 state holds a cache that cannot be unpickled. A forked worker keeps what it
inherited, so nothing changes under fork.

**Result.** On 16 November 2026, on the owner's Windows machine with the pinned engine environment (CPython 3.14.7,
NumPy 2.3.5), `python -I research/jumbling/sim/accept.py` exited with 0 and all 32 summary flags were true. Every
result field equals the recorded `research/jumbling/sim/acceptance.json` (a cloud run, CPython 3.13.16, NumPy 2.5.3),
apart from the timings, the machine block and two angles reported in degrees that differ in the last digit
(62.96430821058771 against 62.96430821058772, and 72.37255861806287 against 72.37255861806288). Both angles come from
sections that use no process pool (`section_twists` and `section_input_map`), and both are floating-point reports,
not exact results. The recorded `acceptance.json` is unchanged, and the Windows run's output is not committed. This is
source and synthetic-geometry evidence from a headless run, not Windows/DirectX or performance evidence.

## A fourth item: notice only

The free rotation mode on 600-cell-Full went to the owner as a notice, not a question. Section 11, point 3 of the
LP2 plan already makes free mode an internal entry with no product command, so the drafted refusal stays and nothing
changes.

## What stays open

The frozen-text gaps of PHS.52 are not covered by these answers. They are on the same decision card, and
`rust/README.md` keeps them as drafts until the owner answers.
