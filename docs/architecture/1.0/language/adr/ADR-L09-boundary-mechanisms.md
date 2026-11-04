# ADR-L09: boundary mechanisms, with the policy left to the joint setting

Status: **Proposed.** Not accepted. The owner accepts or rejects it ([`AGENTS.md`](../../../../../AGENTS.md), "Ask the
owner"). **This record fixes mechanisms only. It sets no boundary policy.** The owner and the integrator set that
together (owner, 3 November 2026).

Rests on: partition INV-3, INV-11, INV-12, section 8.2 and section 11; [chapter 07](../architecture/07-boundary.md);
[chapter 04](../architecture/04-environment.md) EV5; the owner's records of [2
October](../../../../wiki/decisions/owner-decisions-2026-10-02.md), [17
October](../../../../wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md) and [3
November](../../../../wiki/decisions/owner-decisions-2026-11-03.md). Round table: DT-L1 M5, M6, R2-E3, R2-E4, R3-M3,
R3-M4; explainer P-M5, as corrected.

## Context

The program must never solve the puzzle by itself. On 3 November the owner added a principle: a tool a solver develops
themselves, through the built-in base tools and while understanding its principle, does not count as the program
outputting a solution. The exact boundary is for the owner and the integrator to set together, and until then the
built-in tools keep the 2 October boundary.

A general language makes this hard. It can express any solving procedure, and a solver can type a word computed
elsewhere (BND.7). Whether the program chose a word is not a function of the word and the state: the identical pair can
be the solver's macro or the output of a search (BND.8). So no check on a word can settle the question, and "themselves"
and "understanding" are not checkable at all (BND.12).

What software can do is narrower: control what ships, how execution happens, which capabilities a profile grants, which
flows inside the sandbox reach a preview, and how records are labelled (BND.13). This record fixes those mechanisms.
Every option the owner might choose needs them.

## Decision

- **ADR-L09.1 Execution gate (BM1).** A commit needs an explicit user action on the shown preview, and the shell mints
  the token (BND.5, BND.29). A preview is prepared only inside a command the solver invokes (BND.30). This gate does not
  rest on the checker or the interpreter, so a bug in either cannot commit a word the solver did not see (BND.75).
- **ADR-L09.2 Provenance (BM2).** The host fills a provenance block, and a program has no operation that sets a field
  (BND.22). Every preview names the program, its module digest, its author class and what it may depend on, as far as
  the form of flow control records it (BND.31). Stored words, macros and workspace records keep the block (BND.32,
  PS1.32). A missing field reads `unknown`, never `own` (BND.21).
- **ADR-L09.3 Profiles (BM3).** There are three profiles: solving, tool authoring and research. A profile belongs to a
  run, the host sets it, and a program cannot raise it (BND.17, BND.34). Research output never becomes a preview in a
  solving session (BND.61).
- **ADR-L09.4 Labelled records (BM4).** A solve record states which assistance classes the system recorded. The host
  computes the summary from the journal and the run records, the summary names those inputs, and no program writes it
  (BND.35).
- **ADR-L09.5 Boundary classes (BM5).** Each built-in tool declares its class, and the review checks the tool's entry
  and rows against that class (BND.27, BND.37). A class grants nothing by itself.
- **ADR-L09.6 Policy as data.** The policy table is versioned data outside the kernel. No program can change it, and its
  identity enters every run record (INV-11, BND.28). So the joint setting writes a table and rebuilds no mechanism.
- **ADR-L09.7 Declassifiers are fixed.** A declassifier is a fixed kernel or library entry that the policy enables. No
  program and no library can declare one (BND.53).
- **ADR-L09.8 What records claim.** Author class records how bytes arrived, and it proves neither authorship nor
  understanding (BND.20, BND.79). EV5 supplies the means of understanding and cannot certify it (BND.72). Boundary
  mechanisms carry no evidence, and a claim's strength never grants a capability (BND.81).

## What this record does not decide

**The policy.** Chapter 07 sets out four option sets: BS1 Provenance, BS2 Base tier, BS3 Understanding and BS4 Named
targets. Each keeps BM1 to BM5, the built-in tools' 2 October boundary and the 17 October exception, and the order
implies nothing. BND.67 says what each writes into the policy table, and BND.68 works five tools through them. None is
chosen here.

The integrator's leaning in BND.70 is for discussion only, and this record does not adopt it.

The joint setting answers the ten questions of BND.74. They cover, among others, whether the flow rule is on (BND.33),
which display sinks it checks (which also fills the gap of BND.59), which fix of BND.51 applies, whether the pick rule
of BND.55 stands, whether the practice copy counts as a solving session (BND.15), and the edges of BND.66.

Conditional on an owner question:

- **LQ2:** the option set, and whether to keep or change the interim reading (BND.69). The interim reading is the
  integrator's, and it extends the 2 October boundary to everything that ships, library functions included (BND.63,
  BND.64). The joint setting comes before LP6 (BND.71).
- **LQ21:** the form of flow control, which [ADR-L06](ADR-L06-effects-and-flow.md) holds for LP1. The form is how flows
  are seen, and the flow rule is whether they are refused (BND.41).
- **LQ14:** whether a competence check only guides, or gates a capability under BS3 (BND.73).
- **LQ1:** that profiles differ only in capabilities and labels (BND.16).

## Why

- Every option needs these five mechanisms, and they are the part software can do. Building them first means the owner
  can choose the policy with the facts in view, without any rebuild afterwards.
- Separating mechanism from policy keeps a mistaken judgement cheap to correct. A wrong row in a table is an edit, while
  a wrong rule inside the checker would be a migration of saved tools.
- A check on programs cannot replace them. For programs without limits, Rice's theorem rules out a general test, and a
  budget does not change the deeper fact of BND.8 (BND.9).
- Honest labels need no proof of intent. A record can say what the system recorded about a solve without claiming who
  understood what.

## Alternatives considered

| Alternative | Why it was turned down |
| --- | --- |
| Detecting solving by inspecting words or programs | Not a function of (word, state) (BND.8), and undecidable for programs in general (BND.9) |
| Writing the boundary into the checker or the kernel | The policy would stop being data, against INV-11, and every change would be a change to the trusted core |
| Choosing an option set now (LQ2's alternative) | The owner set this boundary as a joint decision, so the integrator does not finalise it. It stays open as LQ2's alternative |
| Certifying understanding by a test | A competence check shows that a person completed a task (BND.72) |
| Letting a module declare its own class or provenance | A declaration would be self-certification. The host fills both (BND.18, BND.22) |

## Consequences

- A solver's own tool has no solving preview until the boundary is set (BND.69). Nothing ships to a solving session
  before LP6, so this costs nothing yet (BND.59).
- Records can be evaded by hand. A retyped word leaves a record that says the solver typed it, and a program outside the
  runtime leaves no label (BND.13, BND.49, BND.76).
- No form of flow control closes every channel (BND.46, BND.77). A determined user can rebuild any search from base
  parts (BND.78).
- The owner of a local install can change the table or the program. The boundary describes what the system recorded
  about a solve and says what it cannot record (BND.82).
- Every built-in tool needs a class, a review and a principle record before it ships (INV-12, BND.27, BND.63). No tool
  has those yet.

## Evidence

Class: source. The limits in chapter 07, section 2, are arguments, not measurements, and no person has reviewed them.
CT-BND-01 to CT-BND-25 are requirements only. LX-3 has not run, so the false-refusal count of each form is unknown
(BND.80).

## What the owner accepts

Accepting this record accepts ADR-L09.1 to ADR-L09.8 and the [D] clauses they cite. It accepts no option set and answers
none of the questions of BND.74.

The owner may overrule these integrator additions: the term "solving session" (BND.14), what a previewing program
receives and the fixed acknowledgement under capability separation (BND.42, BND.43), and the absence of a
capability-dependent budget ceiling (BND.62). The rule for drafts (BND.58) and the capture of `propose` outside the
solving profile (BND.60) are also new, and they stand or fall with LQ2. The owner should also see BND.59: the interim
reading does not say what an own tool may read or show in a solving session.
