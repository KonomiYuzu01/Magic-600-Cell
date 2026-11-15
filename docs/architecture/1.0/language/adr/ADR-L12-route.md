# ADR-L12: implementation route and runtime placement

Status: **Accepted** (owner, 15 November 2026;
[decision](../../../../wiki/decisions/owner-decisions-2026-11-15-freeze.md)). **The owner chose on 14 November 2026:** the engine, the
session store, the command layer and the interpreter's host in Rust (LQ8, RTE.17); route RT1, an own interpreter over
the typed IR, with the runtime in a process of its own (LQ9, RTE.21); and capability separation (LQ21, RTE.24). This
record states those choices, what holds under every outcome, and the checks that can still reopen LQ9.

Rests on: partition section 12; [chapter 11](../architecture/11-route-and-spikes.md); [the engine-language
question](../../../../wiki/questions/engine-language-rust.md). Round table: DT-L1 E1, E4, R2-E3 and Round 3, engineer
advisor, item 4; explainer P-E6.

## Context

The kickoff asks for options and the evidence each needs, with no preset conclusion. Two choices are linked. The engine
language decides where the interpreter normally lives, because direct access to snapshots and kernels favours one host
(RTE.19). The route decides how much of the language the project builds and how much it borrows.

The form of flow control is linked too, because it changes what each route must build (LQ21, RTE.24). LX-1 to LX-9 ran
as far as they can now under the allowance (owner, 6 November 2026, LQ18). Their first-pass results and limits are
recorded under `work/experiments/language-lx/` (RTE.82). LX-5 and LX-6 ran decisively in the owner's window of
14 November 2026, and the owner chose LQ8, LQ9 and LQ21 that day. No result decides LQ13.

## Options

| Route | What it is | For | Against |
| --- | --- | --- | --- |
| RT1 Own language | Own parser, checker, typed IR and interpreter, normally in the engine language | Direct control of domain types, determinism, budgets, flow labels and messages | Largest effort; editor and debugger from scratch; interpreter speed unmeasured; the front end is hard to reverse (RTE.9) |
| RT2 Embedded runtime | An existing runtime with a domain library. Candidates: Luau, Lua 5.4, a JavaScript engine, CPython or a restricted subset, Rhai, a WebAssembly VM | Mature VM, docs and editors; faster start | Types, effects and evidence need a separate checker; run-time flow labels need a modified VM; sandboxing and determinism vary; host semantics leak; licences. A borrowed VM does not remove the front-end commitment (RTE.12) |
| RT3 Hybrid | Own syntax, checker and IR lowered to an existing VM, such as WebAssembly with fuel metering | Own discipline with a mature sandbox and metering | Two layers; source mapping; target limits; the front end stays hard to reverse (RTE.14) |
| RT4 Host-language library | The language as a typed library in the engine language or Python | Fastest start; all host tooling | No static effect discipline without a separate checker; hard sandbox; users learn the host language; tied to the engine choice; no typed IR by definition (RTE.6) |

First-pass candidate records supply scoped evidence and leads (RTE.11, RTE.82). The owner chose RT1 on them; the
other rows record the alternatives and bind nothing. The engine language options are in RTE.17, and placement options
are in RTE.21.

## Decision

These points hold under every outcome, including a reopened LQ9:

- **ADR-L12.1** Every route is a client of the same kernel and the same state lifecycles (RTE.2).
- **ADR-L12.2** A route change keeps the language-neutral data core and the meaning of saved work (RTE.3).
- **ADR-L12.3** Each candidate is judged against one obligation inventory, and a borrowed tool, library, VM fuel or
  sandbox counts only for the part demonstrated (RTE.4, RTE.5).
  The owner chose user shader programs (owner, 6 November 2026, LQ7) and free views everywhere
  (owner, 6 November 2026, LQ6). RPI.68 to RPI.83 give the draft vertex and fragment stage contract and free-view
  workload. The project's pipeline checks and compiles stage modules of 600 Cell Script to portable HLSL; raw shader
  upload has no path (RPI.68, RPI.78).
- **ADR-L12.4** Each placement meets KN7.12, KN7.13 and every rule of chapter 08, and a runtime fault ends its jobs,
  never the session (RTE.21, RTE.22).
- **ADR-L12.5** The portability boundary holds under every engine language (RTE.17).
- **ADR-L12.6** Evidence classes stay separate. Oracle equality is source evidence, and performance figures count only
  from the owner's machine (RTE.20, RTE.36).

The owner chose the engine language, the route and the placement (owner, 14 November 2026, LQ8, LQ9 and LQ21):

- **ADR-L12.7** The engine, the session store, the command layer and the interpreter's host are written in Rust
  (RTE.17). LP2 chooses the big-integer implementation on LX-5's rows, and the engine port adds the net-effect star
  cache before any net-effect comparison (RTE.18). A product dependency needs the owner's approval (RTE.70).
- **ADR-L12.8** The route is RT1: an own parser, checker, typed IR and interpreter (RTE.8, RTE.21).
  [ADR-L07](ADR-L07-typed-ir.md) applies.
- **ADR-L12.9** The runtime runs in a process of its own. The engine-process and shell-process placements are not
  admitted (RTE.21).
- **ADR-L12.10** The form of flow control is capability separation ([ADR-L06](ADR-L06-effects-and-flow.md), RTE.24).
- **ADR-L12.11** The decisive LX-1, LX-2 and LX-4 runs and LX-3's remaining checks are LP3 acceptance checks with their
  bars unchanged (RTE.38 to RTE.53). A failing LX-1, LX-2 or LX-4 run reopens LQ9 with its evidence (RTE.21); a failing
  LX-3 check goes to the owner (RTE.24).

Proof obligations export to Lean (owner, 6 November 2026, LQ15). Lean is neither shipped nor a runtime dependency;
formal proofs come after 1.0 (owner, 6 November 2026, decision 9). 1.0 has no proof-record import or writer for
`formal proof` (EVD.47, EVD.64, LB5.18, RTE.28).

LQ8, LQ9 and LQ21 were answered in this order of evidence (RTE.18, RTE.23, RTE.26):

1. LX-5 and LX-6 fed LQ8, the engine language; they ran decisively on 14 November 2026.
2. LX-3, LX-7, LX-8 and LX-9 do not wait for LQ8. LX-9's run on the engine candidate follows LX-6.
3. LX-1, LX-2 and LX-4 follow LQ8, because they depend on the interpreter's host. Their decisive runs are LP3
   acceptance checks.
4. LX-1 to LX-5 fed LQ9, the route and placement, chosen with LQ21 on their leads (RTE.23).
5. LQ21 was answered before LP2 builds the checker and the IR (RTE.26).

| Spike | Feeds | Rule |
| --- | --- | --- |
| LX-1 Interpreter cost | LP3 acceptance for LQ9 and LQ21 | RTE.38 to RTE.40. A lead only: the decisive check is LP3's end-to-end measurement |
| LX-2 Placement and faults | LP3 acceptance for LQ9 | RTE.41 to RTE.43 |
| LX-3 Sandbox, determinism, refusals | LQ9, LQ21; LP3 acceptance | RTE.44 to RTE.46 |
| LX-4 Editor latency | LP3 acceptance for LQ9 | RTE.47 to RTE.53. The bars are proposals, and none is a measured result |
| LX-5 Exact arithmetic | LQ8; LP2's big-integer choice | RTE.54, RTE.55. A copyleft dependency needs the owner |
| LX-6 Engine language | LQ8 | RTE.56 to RTE.58 |
| LX-7 User stages | LQ7's safety and renderer contract | RTE.59 to RTE.61, RTE.83. Earlier compute-subset results do not establish the vertex and fragment drawing pipeline |
| LX-8 Syntax learnability | LQ13 | RTE.62, RTE.63. No rule exists until the owner sets the protocol |
| LX-9 Input-map determinism | INV-10 | RTE.64 to RTE.66 |

## What this record does not decide

The owner answered LQ1 to LQ21 on 6 November 2026 and chose LQ8, LQ9 and LQ21 on 14 November 2026: the engine and the
interpreter's host in Rust, route RT1 with the runtime in a process of its own, and capability separation (RTE.17,
RTE.21, RTE.24). LQ13 waits for its evidence, and LQ2's option set waits for the joint setting.

Open mechanism:

- How the runtime process reaches snapshots: transfer or a separately specified read-only sharing mechanism, with
  immutable buffer lifetimes and exact-value transfer. LP1's interfaces specify it (RTE.7, RTE.19).

Waiting owner question:

- **LQ13:** syntax, on LX-8 before LP2 builds the parser (RTE.62, RTE.63).

## Why

The evidence on record for each choice:

- **Rust.** On the same inputs and with equal output digests, the port is 2.3 to 7.2 times faster than NumPy at
  applying a generator, applying a long word and composing macros. The fourth metric, the net effect, compares a
  NumPy reference that has a warmed star cache with a port that has none (RTE.18, RTE.82). Rust keeps the engine,
  store and command layer free of Windows-only code (RTE.17).
- **RT1.** It gives direct control of domain types, exactness, effects, determinism, budgets and messages, and its
  own checker infers effects over the source that runs (RTE.4, RTE.8). Its cost is the largest implementation effort,
  including the editor and debugger services (RTE.8).
- **A process of its own.** On LX-2's stand-in the separate process met conditions 1 and 2 for seven injected faults,
  while the shared-host native allocation, recursion, loop and forced-kill cases failed (RTE.82). These are leads;
  LX-2's decisive run is an LP3 acceptance check.
- The front end is the hard part to reverse (RTE.9). Choosing on leads keeps the decisive runs as acceptance checks,
  so a failure reopens LQ9 with its evidence instead of passing unnoticed.
- Headless timing can lead and cannot decide. Whether a key feels immediate is measured on the owner's machine.

## Alternatives considered

| Alternative | Why it was turned down |
| --- | --- |
| Waiting for decisive LX-1, LX-2 and LX-4 runs before choosing | They need the Rust host, experiment versions of the LP3 tools and a renderer candidate. The owner first chose to wait (owner, 6 November 2026, LQ9), then chose on the leads and made the runs LP3 acceptance checks (owner, 14 November 2026, LQ9) |
| RT2, RT3 or RT4 | Not selected (owner, 14 November 2026, LQ9). The Options table records their trade-offs |
| Keeping Python and NumPy as the 1.0 engine | Not selected (owner, 14 November 2026, LQ8). The decisive LX-6 run found the Rust port faster on three of four hot-path metrics (RTE.18) |
| The runtime inside the engine process or the shell process | Not admitted (owner, 14 November 2026, LQ9; RTE.21) |
| Deciding the route before the engine language | The interpreter's host normally follows the engine, so the route's evidence would have to be collected twice |
| A proof assistant as the language | Rejected alternative of LQ15 (owner, 6 November 2026). It would add a research front end and an adapter to the common identities |

## Consequences

- LP2 to LP5 wait for both LP1 freeze rulings and the owner's sign-off (owner, 6 November 2026, decision 7). Experiment
  code is allowed before them (owner, 6 November 2026, LQ18).
- The project builds the parser, checker, IR, interpreter, editor services and debugger itself (RTE.4, RTE.8).
- Engine and runtime exchange snapshots across a process boundary, not a language boundary (RTE.7, RTE.19).
- A failing LX-1, LX-2 or LX-4 acceptance run at LP3 reopens LQ9 and can change the route after LP2's work (RTE.21).
- Spikes cost effort and owner time. LX-4's protocol alone is about three hours of foreground time at one second per
  sample (RTE.52), and that is source arithmetic.
- A spike can fail to separate the routes. Its report still counts, and negative results stay on record (RTE.37).
- The amended charter order in RTE.32 keeps every gate: the design-system ADR conditions LP4's editor look and LP6,
  after the language's development is complete (owner, 6 November 2026, decision 7).

## Evidence

First-pass LX-1 to LX-9 records are the `RESULT.md` files under `work/experiments/language-lx/` (RTE.82). Source,
synthetic and actual Windows/DirectX evidence stay separate under RTE.36; function timings and headless work establish
no performance, native-input or foreground-window pass. LX-7's earlier compute-subset record does not establish the
new drawing-stage contract (RTE.83). LX-5's and LX-6's decisive timing is recorded in
`work/experiments/language-lx/quiet-window/2026-11-14T1818Z/RESULT.md`. No result decides LQ13. CT-RTE-01 to
CT-RTE-23, with CT-RTE-20 withdrawn, are requirements on candidates and spike records only.

## What the owner accepts

The owner answered LQ8, LQ9 and LQ21 on 14 November 2026, so this record names the engine language, the route and the
placement (ADR-L12.7 to ADR-L12.11). After both LP1 rulings the owner accepts or rejects it with
[ADR-L07](ADR-L07-typed-ir.md) and [ADR-L06](ADR-L06-effects-and-flow.md). The rules of the Decision section still
apply.
