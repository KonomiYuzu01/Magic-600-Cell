---
id: owner-decisions-2026-10-03-image-library
type: decision
status: verified
visibility: public
summary: Owner decision of 3 October 2026 - the Taste Lab image library starts now, in parallel with the close of phase 1a - class A sources, the embedding model and its runtime (phase 1b), class B private references kept local (phase 2), and the annotated references and nexus cards (4A); the review and installation gates are unchanged.
related: [owner-decisions-2026-10-02, owner-decisions-2026-10-02-design, owner-decisions-2026-10-03-taste-lab]
supersedes: []
claims:
  - {id: phase-1b-approved, evidence_kind: decision, checked_at: 2026-10-03}
  - {id: phase-2-approved, evidence_kind: decision, checked_at: 2026-10-03}
  - {id: references-start, evidence_kind: decision, checked_at: 2026-10-03}
  - {id: gates-unchanged, evidence_kind: decision, checked_at: 2026-10-03}
  - {id: plan, evidence_kind: source, path: docs/progress/1.0/taste-lab-plan.md, checked_at: 2026-10-03}
---

# Owner decision, 3 October 2026: Taste Lab image library

The [Taste Lab plan](../../progress/1.0/taste-lab-plan.md) (section 2) kept phase 1b, the class A image swipe, and phase 2, a local tool for class B images, behind an owner approval ([owner-decisions-2026-10-02](owner-decisions-2026-10-02.md)). The owner asked when the image library would start. The integrator answered with three items that needed approval and two related items, and the owner approved all five to start now and run in parallel with the close of phase 1a. The integrator stated its reading of the five items back to the owner. The owner's reply was in Chinese; this page records it in English.

## Approved

1. **Class A sources on the allowlist**: Wikimedia Commons, Openverse, the Metropolitan Museum of Art, the Art Institute of Chicago and NASA. An item is admitted only when its own metadata proves an allowed open licence and names its creator or credit.
2. **Embedding model**: CLIP ViT-B/16 in a NumPy implementation (about 572 MB of weights), checked against a pinned upstream checksum before use. No PyTorch.
3. **Runtime**: NumPy and SciPy pinned in the lockfile for the Taste Lab environment.
4. **Phase 2, class B private references**: the Internet Archive, Demozoo and Safebooru (general rating only, with excluded tags), at each source's rate limit. They stay on the owner's computer: never committed, uploaded, published or shown on claude.ai, and never used as texture, asset, trace or training data.
5. **Annotated references and nexus cards** ([design decision 4A](owner-decisions-2026-10-02-design.md)): the owner's liked and disliked reference pairs and nexus cards start now. Agents extract transferable attributes only, never assets.

## Unchanged

- What may go to claude.ai stays as approved on 2 October 2026: the page code, thumbnails of class A images with source, licence and author, their embeddings as numbers, and the owner's choices, ratings and notes in the page's private storage. Class B images, anything from the owner's sessions, keys and benchmark works never go there.
- The installer and lockfile changes are critical paths: a plan check and an Astra review come first, and automatic installation starts only after the owner runs `python tools/toolchain/bootstrap.py approve` in a terminal. The fetch code runs only after its review. Only code is committed; data copies stay in `work/loop-memory/`.
- Phase 1a closes as planned ([owner-decisions-2026-10-03-taste-lab](owner-decisions-2026-10-03-taste-lab.md)), and the owner rules on its acceptance result.
