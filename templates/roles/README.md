# API team role cards

These cards are the integration contract for the external API team controller (`agent-control/`), which is not part of this repository. Nothing here claims that the controller runs. A role is a responsibility in the controller's state machine, not a permanently running agent.

Every role uses the seven-part packet in `templates/problem-packet.md`, returns results in the formats named on its card, and records each call as one line that follows `schemas/ledger-entry.schema.json`.

Providers:
- Default bindings: reviewer = Claude Opus; solver = Fable. These are independent of the local integrator and reviewer roles in `AGENTS.md`.
- The API team may also run on OpenAI models through the OpenAI API. Before any API-team run, the controller must show the owner the provider, the model for each role, the expected calls and the budget reservation, and wait for the owner's confirmation. A confirmation covers one run.
- Record the resolved provider and model for every call. Never assume an alias keeps pointing to the same model.

Budgets: paid API ceilings are cumulative (USD 100, then 150, then 300). Raising a ceiling needs owner approval. Exhausted subscription quota never switches to paid API automatically. Reserve budget before concurrent paid calls, then settle it. Record an unknown cost as `null`, never as zero.
