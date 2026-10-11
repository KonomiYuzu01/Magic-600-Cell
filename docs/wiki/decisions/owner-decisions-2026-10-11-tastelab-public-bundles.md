---
id: owner-decisions-2026-10-11-tastelab-public-bundles
type: decision
status: verified
visibility: public
summary: Owner decision of 11 October 2026 - the class A image bundles are published with the public Taste Lab page, which loads them by itself, so images can be rated on any device; each image keeps its credit and licence link; class B images and every rating, note and export stay private.
related: [owner-decisions-2026-10-03-image-library, owner-decisions-2026-10-10-tastelab-rating]
supersedes: []
claims:
  - {id: class-a-bundles-public, evidence_kind: decision, checked_at: 2026-10-11}
  - {id: page-loads-hosted-bundles, evidence_kind: decision, checked_at: 2026-10-11}
  - {id: class-b-and-answers-private, evidence_kind: decision, checked_at: 2026-10-11}
---

# Owner decision, 11 October 2026: class A bundles on the public Taste Lab page

The owner asked for the existing class A bundles to be loaded into the public Taste Lab page. The integrator offered two ways: publish the bundles with the page, or keep them on this computer and import them all with one folder selection. The integrator stated that publishing would end the practice of keeping images on this computer. The owner chose to publish. The owner's message and answer were in Chinese; this page records them in English.

## Decision

1. **Publish the class A bundles.** The bundles written by the Taste Lab bundle writer go into the owner's public page repository (`KonomiYuzu01/taste-lab`), served by GitHub Pages. Each bundle holds JPEG thumbnails, a manifest with title, credit, source page and licence link per image, and the image embeddings. At this decision there are 20 bundles with 5,862 images, about 240 MiB.
2. **The page loads them by itself.** The page finds the bundles on its own site and imports them through the same validation as a folder it is given. Images can then be rated on any device.

## Requirements

- **Class A only.** Each published image must be a class A library image with an allowed open licence (CC BY, CC BY-SA, CC0, public domain), as the [image library decision](owner-decisions-2026-10-03-image-library.md) admits them. At this decision no bundle item has a non-commercial or no-derivatives licence.
- **Attribution.** Each image is shown with its title, credit, source link and licence link. The published repository states that the images keep their own licences, separate from the page's licence.
- **Unchanged.** Class B images are never published, uploaded or shown on claude.ai. Image ratings, notes, pair notes and exports stay in the browser or on this computer; nothing is uploaded. The main repository still commits code only, never bundles.
- **Process.** Loading hosted bundles is a behaviour change to the page: it gets a plan check, an implementation in isolated runs and one review of the finished candidate. Pushing the bundles to the public repository needs the owner's explicit approval to push.
