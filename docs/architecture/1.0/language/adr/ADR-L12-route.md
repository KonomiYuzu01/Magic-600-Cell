# ADR-L12: implementation route and runtime placement

Status: **Proposed.** Not accepted. **Held for LP1: it follows the evidence** (partition section 17, PHS.10). The owner
accepts or rejects it then ([`AGENTS.md`](../../../../../AGENTS.md), "Ask the owner"). **This draft chooses no route.**
It records the options, what holds under all of them, and how the choice is made.

Rests on: partition section 12; [chapter 11](../architecture/11-route-and-spikes.md); [the engine-language
question](../../../../wiki/questions/engine-language-rust.md). Round table: DT-L1 E1, E4, R2-E3 and Round 3, engineer
advisor, item 4; explainer P-E6.

## Context

The kickoff asks for options and the evidence each needs, with no preset conclusion. Two choices are linked. The engine
language decides where the interpreter normally lives, because direct access to snapshots and kernels favours one host
(RTE.19). The route decides how much of the language the project builds and how much it borrows.

The form of flow control is linked too, because it changes what each route must build (LQ21, RTE.24). None of the
evidence exists. No spike has run, and spikes run only if the owner allows them (LQ18).

## Options

| Route | What it is | For | Against |
| --- | --- | --- | --- |
| RT1 Own language | Own parser, checker, typed IR and interpreter, normally in the engine language | Direct control of domain types, determinism, budgets, flow labels and messages | Largest effort; editor and debugger from scratch; interpreter speed unmeasured; the front end is hard to reverse (RTE.9) |
| RT2 Embedded runtime | An existing runtime with a domain library. Candidates: Luau, Lua 5.4, a JavaScript engine, CPython or a restricted subset, Rhai, a WebAssembly VM | Mature VM, docs and editors; faster start | Types, effects and evidence need a separate checker; run-time flow labels need a modified VM; sandboxing and determinism vary; host semantics leak; licences. A borrowed VM does not remove the front-end commitment (RTE.12) |
| RT3 Hybrid | Own syntax, checker and IR lowered to an existing VM, such as WebAssembly with fuel metering | Own discipline with a mature sandbox and metering | Two layers; source mapping; target limits; the front end stays hard to reverse (RTE.14) |
| RT4 Host-language library | The language as a typed library in the engine language or Python | Fastest start; all host tooling | No static effect discipline without a separate checker; hard sandbox; users learn the host language; tied to the engine choice; no typed IR by definition (RTE.6) |

Candidate-specific licence, speed and enforcement results do not exist yet (RTE.11). The engine language options are in
RTE.17, and placement options are in RTE.21.

## Decision

No route, engine language or placement is decided here. These points hold under every outcome:

- **ADR-L12.1** Every route is a client of the same kernel and the same state lifecycles (RTE.2).
- **ADR-L12.2** A route change keeps the language-neutral data core and the meaning of saved work (RTE.3).
- **ADR-L12.3** Each candidate is judged against one obligation inventory, and a borrowed tool, library, VM fuel or
  sandbox counts only for the part demonstrated (RTE.4, RTE.5).
- **ADR-L12.4** Each placement meets KN7.12, KN7.13 and every rule of chapter 08, and a runtime fault ends its jobs,
  never the session (RTE.21, RTE.22).
- **ADR-L12.5** The portability boundary holds under every engine language (RTE.17).
- **ADR-L12.6** Evidence classes stay separate. Oracle equality is source evidence, and performance figures count only
  from the owner's machine (RTE.20, RTE.36).

The order of the choices is proposed under LQ8, LQ9, LQ18 and LQ21:

1. LX-5 and LX-6 feed LQ8, the engine language.
2. LX-3, LX-7, LX-8 and LX-9 do not wait for LQ8. LX-9's run on the engine candidate follows LX-6.
3. LX-1, LX-2 and LX-4 follow LQ8, because they depend on the interpreter's host.
4. LX-1 to LX-5 feed LQ9, the route and placement, decided with LQ21 inside LP1 (RTE.23).
5. LQ21 comes before LP2 builds the checker and the IR under any order of spikes (RTE.26).

| Spike | Feeds | Rule |
| --- | --- | --- |
| LX-1 Interpreter cost | LQ9, LQ21 | RTE.38 to RTE.40. A lead only: the decisive check is LP3's end-to-end measurement |
| LX-2 Placement and faults | LQ9 | RTE.41 to RTE.43 |
| LX-3 Sandbox, determinism, refusals | LQ9, LQ21 | RTE.44 to RTE.46 |
| LX-4 Editor latency | LQ9 | RTE.47 to RTE.53. The bars are proposals, and none is a measured result |
| LX-5 Exact arithmetic | LQ8 | RTE.54, RTE.55. A copyleft dependency needs the owner |
| LX-6 Engine language | LQ8 | RTE.56 to RTE.58 |
| LX-7 User charts | LQ7 | RTE.59 to RTE.61 |
| LX-8 Syntax learnability | LQ13 | RTE.62, RTE.63. No rule exists until the owner sets the protocol |
| LX-9 Input-map determinism | INV-10 | RTE.64 to RTE.66 |

## What this record does not decide

Open whatever the owner answers:

- How RT4 meets the saved-program contract (RTE.6).
- The binding, ABI and buffer-lifetime protocol between host and kernel (RTE.7).

Conditional on an owner question:

- **LQ8:** the engine language. The alternatives are deciding Rust now, or keeping the exact Python and NumPy reference
  as the 1.0 engine. Neither supplies a passing spike (RTE.18).
- **LQ9:** the route and placement. The alternatives decide now for an own interpreter or an embedded runtime, and
  neither waives the rules above (RTE.23).
- **LQ21:** flow labels or capability separation, with [ADR-L06](ADR-L06-effects-and-flow.md).
- **LQ15:** whether a proof assistant has any place in 1.0 (RTE.28). It is a separate question from the route.
- **LQ18:** whether the spikes may run before the freeze. Under its alternative they run after LP1's gates and before
  LP2, and the frozen interfaces must hold under every route and both forms (RTE.29, RTE.31).

## Why

- Choosing first and measuring later would turn the first guess into a saved-work dependency. The front end is the hard
  part to reverse, so it deserves the most evidence.
- Each route's cost is unknown. So the comparison needs the same inventory and the same fixed inputs.
- Headless timing can lead and cannot decide. Whether a key feels immediate is measured on the owner's machine.

## Alternatives considered

| Alternative | Why it was turned down |
| --- | --- |
| Choosing a route now | The kickoff asks for no preset conclusion, and no evidence exists. It stays open under LQ9 |
| Deciding the route before the engine language | The interpreter's host normally follows the engine, so the route's evidence would have to be collected twice |
| A proof assistant as the language | A separate question (LQ15). It would add a research front end and an adapter to the common identities |

## Consequences

- Nothing can be built until the choices are made, and LP1 carries that weight.
- Spikes cost effort and owner time. LX-4's protocol alone is about three hours of foreground time at one second per
  sample (RTE.52), and that is source arithmetic.
- A spike can fail to separate the routes. Its report still counts, and negative results stay on record (RTE.37).
- Charter stage 2.5's order stays until the owner changes it (RTE.32).

## Evidence

None yet. No spike has run. The wiki question proposes Rust, and its spike has not run either. CT-RTE-01 to CT-RTE-20
are requirements on candidates and spike records only.

## What the owner accepts

Nothing at LP0. At LP1, with the spike reports, this record gains a route, an engine language and a placement, and the
owner accepts or rejects it with [ADR-L07](ADR-L07-typed-ir.md) and the form of flow control. If the owner answers LQ8,
LQ9 or LQ21 sooner, the record changes to say so, and the rules of the Decision section still apply.
