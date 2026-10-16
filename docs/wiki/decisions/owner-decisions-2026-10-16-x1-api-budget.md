---
id: owner-decisions-2026-10-16-x1-api-budget
type: decision
status: verified
visibility: public
summary: Owner decision of 16 October 2026 - one unattended run on open problem X1 on the owner's own API credit, with a budget of USD 200; this raises the paid API ceiling for that run only.
related: [owner-decisions-2026-10-15-jumbling-restored]
supersedes: []
claims:
  - {id: x1-run-budget-usd-200, evidence_kind: decision, checked_at: 2026-10-16}
---

# Owner decision, 16 October 2026: an API run on X1 with a budget of USD 200

The owner gave this decision in a chat message in Chinese; this page records it in English.

## Decision

- The owner spends up to USD 200 of their own API credit on one unattended run that attacks open problem X1: is the reachable set R(I_a) finite, or equivalently, are left heights bounded ([open problems](../../../research/jumbling/theory/open-problems.md))?
- This approves paid API spending above the first ceiling of USD 100 in `AGENTS.md` "Records and budgets", for this run only. It does not change the ceilings and does not authorize any other paid run.

## How the run is bounded

- **Who runs it.** The owner starts and watches the run on their own machine, with a kit that the integrator prepared. The kit is not in the repository.
- **Spend.** It is limited in four ways:
  - the run wraps up at an estimated USD 165 and stops at USD 176;
  - the client cap is USD 182;
  - the API workspace has a monthly spend limit of USD 195;
  - automatic reload of the credit is off.
- **Inputs.** The run reads an export of commit f281b08 (`research/` and `assets/`). The export is read-only, and the run's shell has no network access.

## Effect

- **Results.** They come back to the integrator, who checks every claim and every witness independently before anything enters the repository. Each result is labelled proved, certificate or open.
- **Cost record.** The billed spend is recorded from the Console figure, apart from subscription usage.
- **Unchanged.** Other project work stays on subscription quota, and the rules for API-team runs stay as they are.
