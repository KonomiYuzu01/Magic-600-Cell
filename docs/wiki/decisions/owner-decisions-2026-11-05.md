---
id: owner-decisions-2026-11-05
type: decision
status: verified
visibility: public
summary: Owner decisions of 5 November 2026 - the language walkthrough is confirmed as understood; the nine language experiments and the remaining work run as far as they can now, the questions page is then updated, and everything goes into one pull request; after the owner's answers the architecture is written and then frozen; Codex Astra took more of the experiment and architecture work while the owner's subscription reset lasted, and the work then returned to the original workflow; the reviewed draft stays uncommitted and is reviewed together with the final architecture. No language question is answered and no ADR is accepted by these messages.
related: [owner-decisions-2026-11-03, owner-decisions-2026-10-02, renderer-candidates]
supersedes: []
claims:
  - {id: walkthrough-confirmed, evidence_kind: decision, checked_at: 2026-11-05}
  - {id: spikes-run-what-can-run, evidence_kind: decision, checked_at: 2026-11-05}
  - {id: one-pull-request, evidence_kind: decision, checked_at: 2026-11-05}
  - {id: architecture-after-answers, evidence_kind: decision, checked_at: 2026-11-05}
  - {id: astra-takes-more, evidence_kind: decision, checked_at: 2026-11-05}
  - {id: original-workflow-restored, evidence_kind: decision, checked_at: 2026-11-05}
  - {id: draft-waits-for-final-candidate, evidence_kind: decision, checked_at: 2026-11-05}
---

# Owner decisions, 5 November 2026: experiments, one pull request, then the freeze

The owner gave these decisions in chat messages in Chinese on 5 November 2026. This page records them in English.

## Decisions

1. **The walkthrough is confirmed.** The integrator explained the language architecture in five batches: the one state
   and the preview, evidence, the boundary, persistence and the route. The owner confirmed each batch as understood.
   This is the understanding confirmation of PHS.5 and PHS.56 for LP0.
2. **Run what can run.** The nine language experiments LX-1 to LX-9 and the remaining work run as far as they can be
   done now. The questions page is then updated, and the owner answers it.
3. **One pull request.** All of this goes into one pull request. After the owner has answered, the architecture is
   written and then frozen.
4. **Astra takes more.** The integrator directs Codex Astra to do more of the experiment and architecture work, because
   a subscription reset of the owner's is about to expire. Decision 5 ended this the same day.
5. **Back to the original workflow.** The owner has used the reset, and the work returns to the original workflow.
6. **The draft waits for the final architecture.** The review of the draft text used both verification rounds and left
   three major findings in a draft reference checker. The owner chose not to commit the draft now: it is reviewed together
   with the final architecture.

## Integrator readings (the owner may change them)

- Decision 2 is the owner's allowance under LQ18 for the work that can run now. It sets no numeric time box, so each
  experiment record says "what can be done now" and a reached limit is no pass (RTE.35). It allows no installation and
  no source outside the allowlist (RTE.1, `AGENTS.md` "Ask the owner"), and no cost beyond subscription quota.
- These messages answer none of LQ1 to LQ21 beyond that allowance, and they accept no ADR.
- Decision 3 keeps the gates. Freezing the architecture is the `architecture-freeze` gate: an Astra ruling, then the
  owner's sign-off. RTE.32 and PHS.47 keep the charter's stage 2.5 exit in front of implementation until the owner
  decides LQ19, and no gate is relaxed (3 November).
- Decision 4 widens Astra's use for this task beyond the default assignment in `AGENTS.md`: Astra may implement
  experiment packets and write architecture text through the wrapper. Implementation stays confined to packet-owned
  worktrees, and nobody reviews their own work.
- Decision 5 ends that widening for all work not yet started, and `AGENTS.md` assigns the models again. Sol takes routine
  plan checks and reviews, scoped verification rounds and the separable parts under "Codex first". Astra keeps what
  `AGENTS.md` gives it: plan checks and full reviews of critical-path, behaviour, contract and design work, the gate
  rulings, escalations, joint attacks and milestone audits. The integrator writes the architecture. Work already done
  under decision 4 stands, with its reviews as recorded.
- Decision 6 grants no exception to the review rules. The integrator fixes the three findings in the final candidate, each
  with a stored control, and that candidate gets its own full review before any of the text is committed.

## Effect

- Unchanged: model identities, mathematical contracts, protection rules, review rules, evidence kinds, the renderer
  selection gate and the design-line hold.
- Next: the experiment records in [`work/experiments/language-lx/`](../../../work/experiments/language-lx/README.md).
