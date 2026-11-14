# The project's own language

This folder holds the architecture of the project's own language, 600 Cell Script, with source extension `.6cs`
(owner, 6 November 2026, decision 6). Under the
[owner's decision of 3 November 2026](../../../wiki/decisions/owner-decisions-2026-11-03.md), the engineering line
develops the language first, beginning with its complete ecosystem and architecture. Every document here is a draft for
the owner. The owner accepts ADRs and freezes the architecture.

Status: **the owner answered LQ1 to LQ21 on 6 November 2026 and chose LQ8, LQ9 and LQ21 on 14 November 2026. No ADR is
accepted and nothing is frozen.** The
[decision page](../../../wiki/decisions/owner-decisions-2026-11-06.md) records the answers; the chapters carry them.

| Document | What it is | Status |
| --- | --- | --- |
| [Ecosystem partition and owner questions](ecosystem-partition.md) | The components and their boundaries; what restoration and the research need; the option sets for the boundary of tools a solver develops; the implementation-route options; the phases; the owner's questions | Historical record of the questions, with a dated status note; the chapters carry the answers |
| [Architecture document](architecture/00-overview.md) | The full text: an overview, one chapter per layer and pillar, then the route, the phases and the error registry | Draft for the owner. Not accepted |
| [ADR drafts](adr/README.md) | Twelve records, one per architectural decision | All proposed. None accepted |
| [Evidence import table](evidence-import/README.md) | The versioned table that maps the research corpus's recorded labels by (format, field, value) | Draft |

The owner confirmed the direction of step 1, and the full text followed. The owner answered LQ1 to LQ21 on 6 November
2026 and chose LQ8, LQ9 and LQ21 on 14 November 2026: the engine and the interpreter's host in Rust, route RT1 with the
runtime in a process of its own, and capability separation (RTE.17, RTE.21, RTE.24). LQ13 waits for its evidence, and
LQ2's option set waits for the joint setting.

The round table for the language is [DT-L1](../../../progress/1.0/design-table/topics/language-ecosystem.md), with the
mathematician and the engineer advisor.
