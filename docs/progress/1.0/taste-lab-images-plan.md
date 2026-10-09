# Taste Lab phases 1b and 2: plan

Status: **revised after the plan check** (call 20261004T034034Z-e9249a21: six major findings, TLPLAN-01 to TLPLAN-06, all adopted); scoped re-checks 20261004T035543Z-e2ee8926, 20261004T040159Z-8f29d358 and 20261004T040603Z-765a793d: TLPLAN-07 resolved; TLPLAN-04 and TLPLAN-08 escalated to an independent diagnosis (call 20261004T041041Z-8a770c8c); its design was re-checked (call 20261004T042754Z-bc2fead9: TLPLAN-09 and TLPLAN-10, both about uploads), and the page now uploads nothing (section 6.2); scoped re-check 20261004T043921Z-9e30f06c passed. Approved scope: [owner-decisions-2026-10-03-image-library](../../wiki/decisions/owner-decisions-2026-10-03-image-library.md). The plan reuses the local image tool parked on 2 October 2026 (unreviewed). Three read-only Codex calls screened that code on 3 and 4 October 2026 (calls 20261003T234800Z-7e12cbe5, -fada12be and -fa24a79f). They found 20 problems, 12 of them major. Section 7 answers each one.

## 1. Goal and acceptance

**Phase 1b.** The owner rates class A images in the Images tab of the Taste Lab page. Embeddings group the images and suggest what to rate and what to compare. The owner's ratings and notes decide.

**Phase 2.** A local tool for class B private references. The owner fetches, rates and maps these images on their own computer only.

Phase 1b is accepted when:
1. The headless tests pass:
   - `python tests/test_tastelab.py`, which also runs the node tests of the new page modules;
   - `python tests/test_tastelab_images.py`, a new runner that runs the Python tests of the image tool with the Taste Lab environment. When that environment is missing it skips with a clear message, but a skip never counts toward acceptance: the suite must actually run.
   - The installer changes (section 8) also need the agent-rules checks of `AGENTS.md`: `test_agent_rules_sync.py`, `test_codex_review.py`, `test_stop_gate.py`, `test_bootstrap.py`, `test_wiki_lint.py` and `test_workbench.py`.
2. The model checks of section 5 pass with the installed weights.
3. The calibration gate of the content screen (section 5.3) passes with the real model before any bulk fetch. The owner gets its counts.
4. The first class A library holds at least 300 admitted images across the seed categories. Each image has its licence proof and its credit.
5. The Images tab passes the Artifact checks and one functional pass:
   - open a bundle of at most 20 images;
   - rate one image and read the rating back with `ArtifactData`;
   - export it.
6. No class B image, personal data, key or private path is in the page, its storage, a bundle or the repository. The bundle exporter and the page each refuse class B by construction, and tests with class B canaries show it.

Phase 2 is accepted when:
1. The class B adapters fetch within each source's terms and rate limit, under the Safebooru rules of section 4. Class B gets the strict content screen.
2. The local rating window rates class A and class B images, and the taste map and its report run locally.
3. Class B never enters any fitting step: the classifier, the selector, the taste-map clusters and axes, the term ranking, the proposals or the screen calibration. A model fitted on class A may score or place class B images, but only locally. A canary test instruments every fitting step. Class B never reaches a bundle, the page or the repository.
4. The headless tests pass. The Qt window test runs with the Taste Lab environment.

## 2. Scope

- **In:**
  - the five class A sources and the three class B sources of the owner decision;
  - CLIP ViT-B/16 in NumPy;
  - the Taste Lab environment and the two model files (section 8);
  - the local pipeline, store, bundle exporter and phase 2 views;
  - the Images tab.
- **Out:**
  - other sources or models, and PyTorch at run time;
  - uploading images or embeddings to claude.ai: the page reads bundles from the owner's computer (section 6.2);
  - ranking the owner's own folders (`score.py` is dropped);
  - the Look Lab;
  - performance claims.
- Images, ratings, notes, embeddings, bundles and reports stay under `work/loop-memory/tastelab/` (ignored by Git). Only code is committed.

## 3. Data flow and privacy boundary

1. **Local fetch.** A source adapter searches its API and proposes candidates. `admit` keeps a candidate only when its own metadata proves an allowed licence and names a creator or credit.
2. **Local processing.**
   - The GET-only client downloads the image into memory.
   - `images` decodes it and checks its size.
   - `screen` discards unsafe content.
   - `embed` computes the CLIP vector.
   - `store` commits the image, its thumbnail and its metadata under the private data root.
3. **Bundle.** `bundle` writes class A images only: thumbnails, attribution and embeddings, in one folder under `work/loop-memory/tastelab/bundles/`.
4. **Open in the page.** In the Images tab the owner opens one bundle folder. The page validates the whole bundle and keeps it in this browser only. Nothing is uploaded: thumbnails and embeddings never leave the owner's computer and browser. Claude is not in this path.
5. **Ratings back to the computer.** Ratings, notes and pair notes stay in the page's owner-only storage. They come back through the page's export or `ArtifactData`, into `work/loop-memory/tastelab/`. The local tool imports the class A ratings by image id (section 6.1).
6. **Class B.** Class B goes through steps 1 and 2 locally and never reaches step 3. The bundle exporter selects tier A only. It checks every row again with `admit(row, "A")`. The page accepts only class A licences, and class B rows carry the licence `private-reference`.

## 4. Local tool (`tools/tastelab/`, Taste Lab environment)

| Module | Disposition | Work |
|---|---|---|
| `common.py` | keep, fix | Data root fixed to `<checkout>/work/loop-memory/tastelab/library/`. `--data` may only pick a folder inside `work/loop-memory/tastelab/`. Both the default and the explicit root pass the same guard (no links or junctions, no session markers) before any directory is created or the database opened (A-002, A-003). The SHA check uses a full-string match (A-006). |
| `net.py` | finish | GET only, from the adapter's hosts only. Redirects only to those hosts. No proxy or credential from the environment (no `trust_env`, an empty proxy handler). Size caps and the source's minimum interval. A replay transport for tests. |
| `sources.py` | finish | Adapters for wikimedia, openverse, met, aic and nasa (tier A), and archive, demozoo and safebooru (tier B, phase 2). Admission rules, hosts and intervals as in the parked docstring (terms checked on 1 October 2026). Every adapter is tested against recorded API responses, never live. |
| `seeds.py`, `seeds.default.yaml` | keep, fix | Malformed shapes raise `SeedError` with the field name (A-005). |
| `fetch.py` | finish | Pipeline of section 3. An item stays in memory until it is committed. Only counts of discarded items are kept. The calibration run of section 5.3. |
| `images.py` | keep | JPEG, PNG, WebP and GIF; pixel cap; 512-pixel JPEG thumbnail. |
| `clip.py`, `embed.py` | keep, fix | Section 5. |
| `screen.py` | keep, fix | Section 5.3. |
| `store.py` | keep, fix | Admission serialized by one write transaction that holds the database lock across its checks and its insert. A file goes in under a temporary name and is renamed only after the row commits. A journal table records file operations, and opening the store reconciles them (TL-B02, TL-B03). |
| `learn.py` | keep, fix | Trains on class A only (TL-B01), and so do the proposals. Becomes ready when the like or dislike count crosses its threshold (TL-B04). The selector is rebuilt when the set of eligible images or their embeddings change, not only their count (TL-B05). |
| `bundle.py` | new | Section 6.1. Replaces `library.py`. |
| `rate.py`, `qml/Rate.qml` | keep, fix (phase 2) | Ignore key auto-repeat (C-05). Recheck removal and blocking when a delayed pick completes (C-06). |
| `mapping.py`, `tastemap.py`, `explore.py` | keep, fix (phase 2) | Clusters, axis directions and term rankings are fitted on class A only; class B images are only placed on the class A map (TLPLAN-02). Rank-aware explained share (TL-B07). Liked and avoided terms never overlap, and short lists are shown as a ranking (TL-B08). |
| `library.py`, `score.py` | drop | Out of scope (section 2). |

The data root is the private folder of the checkout the tool runs in. The owner runs it from their main checkout.

## 5. Model and content screen

### 5.1 Weights and tokenizer

- `embed.load_embedder` checks the SHA-256 and the size of the weights file and of `merges.txt` against the lockfile before `clip.load` reads either. A mismatch refuses to load (C-01). The test changes one tensor byte in a fixture without changing its size and expects the refusal before `clip.load`.

### 5.2 Upstream equivalence (C-02)

- Recorded reference outputs prove equivalence:
  - token IDs, including Unicode, truncation and mixed lengths;
  - the preprocessed tensors of odd sizes, extreme aspect ratios and transparency;
  - the image and text embeddings of a fixed synthetic set.
- Tolerances: cosine similarity at least 0.9999 per embedding, and an absolute error of at most 1e-4 per preprocessed value.
- The fixture is bound to the weights checksum. It is produced as the owner chose on 9 October 2026 (section 9, option (a)).

### 5.3 Content screen

- Probe validation: the screen refuses to build when any probe vector has the wrong shape or count, a norm outside 1 ± 1e-3, or a non-finite value. When probes are invalid, every image is discarded at both thresholds (C-03).
- Calibration gate, as approved by the owner on 1 October 2026:
  - It uses lawful public-domain or CC0 images of adult subjects from met, aic, nasa and wikimedia. They are held in memory, never stored, and only counts are kept.
  - A group passes only when it has at least `MIN_GROUP` scorable images.
  - Images that cannot be scored are counted and reported, and they never count as caught (C-04).
  - The thresholds stay as approved.

## 6. Images tab

### 6.1 Bundle (`tastelab-images`, version 1)

- **Identity (TLPLAN-01).** An image is identified everywhere by its **image id**: the store's SHA-256 of the fetched original, the key of its row, embedding and ratings. Its thumbnail is a different file with its own SHA-256. The bundle carries both: the page checks the thumbnail file against its own hash and keys every row, rating and pair by the image id. A round-trip test runs a synthetic PNG through the thumbnail, the bundle, the page's validation and rating export (node), and the local rating import, and checks that the original's store row receives the rating.
- **Folder.** `work/loop-memory/tastelab/bundles/<id>/` holds `manifest.json` and one `<thumbnail sha256>.jpg` per image.
- **Manifest.** The manifest has the format, the version, the id and the creation time, and the model (id and weights SHA-256). Each item has:
  - the image id, and the thumbnail's SHA-256, width, height and bytes;
  - source, source id and title;
  - creator or credit;
  - page URL, licence and licence URL.

  The manifest also holds the embeddings: float16, 512 dimensions, unit norm, in item order, as base64.
- **Links (TLPLAN-03).** The page URL must be https on the source's own page hosts (for example `commons.wikimedia.org`, `www.metmuseum.org`, `www.artic.edu`, `images.nasa.gov`, `openverse.org`). The licence URL is checked against a fixed table that maps each allowed licence to its canonical https URL prefixes: `creativecommons.org/licenses/by/`, `…/by-sa/`, `creativecommons.org/publicdomain/zero/1.0/`, `creativecommons.org/publicdomain/mark/1.0/`, and the source's own open-access or media-use page for `public-domain` and `US-Gov-PD`. The table lives in one module shared by the exporter and the page, and tests feed one recorded admitted item from each class A source through it.
- **Exporter.**
  - It writes tier A only. Thumbnails are JPEG, at most 512 pixels and 200 KB each.
  - A bundle has at most 300 items. There are no local paths, ratings or notes.
  - It writes to a temporary folder and renames the folder when the bundle is complete, and it never overwrites a bundle (TL-B06).
- **Page validation** (`page/bundle.js`). One failure refuses the whole bundle before it is used. The page checks that:
  - the manifest has exactly the fields above;
  - the licence is in the tier A list, and the credit is not empty;
  - the page URL and the licence URL pass the link rules above;
  - each file's SHA-256 matches its name and its item's thumbnail hash, and its first bytes mark a JPEG;
  - image ids are unique and well formed;
  - sizes and the count are within the caps;
  - the embeddings have item count × 512 finite values, each vector of unit norm ± 1e-2.

### 6.2 Opening a bundle

- **Design: the page uploads nothing.** This replaces the upload designs of earlier revisions:
  - Four scoped re-checks and an independent diagnosis (calls 20261004T035543Z-e2ee8926, 20261004T040159Z-8f29d358, 20261004T040603Z-765a793d, 20261004T041041Z-8a770c8c and 20261004T042754Z-bc2fead9) found that every failure came from uploads the page could not recall or attribute:
    - unrecorded uploads;
    - uploads landing after a discard;
    - identical bytes in two imports;
    - deletion while documents still name an asset;
    - storage caps that block recovery.
  - Thumbnails and embeddings therefore stay on the owner's computer and in the owner's browser.
  - Only ratings, notes and pair notes go to the page's storage, as in phase 1a.
  - The approved scope allows class A thumbnails on claude.ai, but does not require them.
- **Open.**
  - The owner opens one bundle folder in the Images tab, with a folder picker or by selecting the folder's files.
  - The selected files must sit directly in one folder, with unique names.
  - The page validates the whole bundle (6.1) and refuses it on any failure.
  - It reads nothing else from the computer.
- **Cache in this browser.**
  - The page keeps each validated bundle in this page's IndexedDB: its manifest text and its thumbnail bytes, keyed by bundle id. The bundle then opens without the folder next time.
  - On each load the page validates every cached bundle again with the same checks, and drops any bundle that fails.
  - The cache is only a convenience. When it is empty, blocked or cleared, the page asks for the folder again; nothing else depends on it.
  - **Close bundle** removes the bundle from this browser's cache only.
  - Another browser or device needs the folder once.
- **Shown images.**
  - The page shows the images of the open, validated bundles, each image once by image id, with suggestions from their embeddings (6.4).
  - An image whose bundle is not open in this browser is not shown, but its ratings and notes stay.
- **Storage writes.**
  - The page writes only `imageRatings/<image id>` and `imagePairs/<liked id>_<disliked id>`, one document each, on the owner's action, as in phase 1a.
  - No document names a bundle, a file or a path.
  - When a write fails, the page says so and keeps the answer in this tab, as phase 1a does.
- **Tests** (`page/library.js`, node, with a fake key-value cache and synthetic bundles):
  - open, cache, reload from the cache, and close;
  - a bundle changed after caching (its manifest or one thumbnail) is dropped on load, and the other bundles still open;
  - a cache that throws or is empty leads to a request for the folder and changes nothing else;
  - files in a nested folder, or repeated names, are refused;
  - an image in two open bundles is shown once, and closing one of the bundles keeps it while the other is open.

### 6.3 Rating

- One image at a time. Under the image: title, creator or credit, the source linked to the page URL, and the licence linked to the licence URL.
- Keys:
  - → like, ← dislike, ↓ skip; a skipped image returns after 50 further ratings;
  - W opens a one-line note of at most 140 characters, as in phase 1a;
  - Z undoes the session's last rating.

### 6.4 Suggestions (`page/images.js`, deterministic, node tests)

- **Before the model is ready.** The model is ready at 5 likes and 5 dislikes. Until then, the next image is the unrated one farthest from every rated image: the largest minimum cosine distance, with ties broken by bundle order.
- **When ready.**
  - Fit an L2-regularised logistic regression on the likes and dislikes: λ = 1, balanced class weights, a bias term, and Newton steps until the gradient norm is below 1e-6 or 50 steps have run. It runs in the worker.
  - Every third pick is the farthest-point pick. The others are the unrated image whose predicted probability is closest to 0.5.
  - The model refits after every answer.
- **Pairs.** Up to 10 pairs of one liked and one disliked image with the highest cosine similarity whose pair has no note yet. The owner opens a pair, sees both images with their attribution, and writes what differs in at most 280 characters. This is raw material for the annotated references (4A).

### 6.5 Storage, capabilities and export

- **Collections:** `imageRatings`, `imagePairs`. The owner-only rules of phase 1a apply. Bundles live only in the browser's cache (section 6.2).
- **Capabilities:** unchanged: `db`, `user` and `downloads`. No `assets`.
- **Republishing.** The page is republished only after the owner says so.
- **Export:** adds the bundles open in this browser (bundle id and item count), the ratings and the pair notes; no thumbnails and no embeddings.
- **Ratings back to the store.** `python tools/tastelab/fetch.py --import-ratings <export>` imports class A ratings and pair notes into the store by image id and reports unknown ids. It refuses a rating for an image the store holds as class B.

## 7. Screening findings

Every major finding is adopted, with the fix shown in sections 4 to 6. Each minor finding has a trivial, local fix and is fixed in the same packet. The TLPLAN findings come from the plan check of this plan (call 20261004T034034Z-e9249a21).

| Finding | Severity | Disposition | Where |
|---|---|---|---|
| A-001 downloader inherits proxies | major | adopt | section 8 |
| A-002 default data root bypasses the guard | major | adopt | `common.py` |
| A-003 data overrides escape the private tree | major | adopt | `common.py` |
| A-004 requirement pins and hashes | minor | adopt | section 8 |
| A-005 malformed seeds escape `SeedError` | minor | adopt | `seeds.py` |
| A-006 SHA guard accepts a newline | minor | adopt | `common.py` |
| TL-B01 class B enters training | major | adopt | `learn.py` |
| TL-B02 concurrent admission | major | adopt | `store.py` |
| TL-B03 interrupted file operations | major | adopt | `store.py` |
| TL-B04 readiness does not trigger training | major | adopt | `learn.py` |
| TL-B05 selector invalidation by count | major | adopt | `learn.py` |
| TL-B06 interrupted export | minor | adopt | `bundle.py` |
| TL-B07 rank-deficient axes | minor | adopt | `mapping.py` |
| TL-B08 contradictory terms | minor | adopt | `mapping.py` |
| C-01 weights loaded without checksum | major | adopt | section 5.1 |
| C-02 no upstream equivalence | major | adopt; route needs an owner decision | sections 5.2 and 9 |
| C-03 invalid probes admit an image | major | adopt | section 5.3 |
| C-04 calibration without positive scores | major | adopt | section 5.3 |
| C-05 key auto-repeat | minor | adopt | `Rate.qml`, `rate.py` |
| C-06 delayed pick and removal | minor | adopt | `rate.py` |
| TLPLAN-01 image identity and thumbnail hash | major | adopt | section 6.1 |
| TLPLAN-02 class B in taste-map fitting | major | adopt | section 1, `mapping.py` |
| TLPLAN-03 licence links and source hosts | major | adopt | section 6.1 |
| TLPLAN-04 recoverable bundle import, including unrecorded uploads | major | adopt | section 6.2 (nothing is uploaded) |
| TLPLAN-05 no unreviewed base commit | major | adopt | section 10 |
| TLPLAN-06 installer checks in acceptance | major | adopt | section 1 |
| TLPLAN-07 runtime files in the packets | major | adopt | section 10 |
| TLPLAN-08 cleanup can delete another import's identical upload | major | adopt | section 6.2 (nothing is uploaded) |
| TLPLAN-09 removal deletes assets that documents still name | major | adopt | section 6.2 (nothing is uploaded) |
| TLPLAN-10 the document cap can block resume and discard | major | adopt | section 6.2 (no import documents) |

## 8. Installer and environment (critical paths)

- **Environment `tastelab`** (Windows x64, CPython 3.14.7).
  - Packages: NumPy and SciPy (named by the owner); Pillow (decoding); regex and ftfy (the CLIP tokenizer); PyYAML (seeds); scikit-learn (the phase 2 model); PySide6-Essentials (the phase 2 window); marimo (the taste map).
  - Pins: exact versions with hashes in `tools/python/tastelab.txt`, compiled from `tastelab.in`.
- **Model files.** `clip-vit-b-16` (`open_clip_model.safetensors`, 598,516,980 bytes) and `clip-vit-b-16-merges` (`merges.txt`, 524,657 bytes) come from the pinned Hugging Face commit of `laion/CLIP-ViT-B-16-laion2B-s34B-b88K`, with SHA-256 pinned.
- **Downloader.**
  - It follows redirects only to the declared hosts.
  - It uses no proxy or credential from the environment (A-001).
  - It refuses requirement lines that are not exact pins with a well-formed hash (A-004).
- **Install gate.** Installation runs only after the review and the owner's `python tools/toolchain/bootstrap.py approve`.

## 9. Owner decision: option (a), 9 October 2026

How the reference outputs of section 5.2 are produced. Smart App Control blocks PyTorch's unsigned DLLs on the owner's computer. The owner chose option (a) on 9 October 2026.

- **(a) Chosen.** A Linux cloud session runs the upstream OpenCLIP implementation once on the CPU.
  - It uses the same pinned weights and a fixed synthetic test set: generated images and captions, no owner data.
  - The outputs become a committed test fixture of about 100 KB.
  - Nothing private leaves the computer.
- **(b)** No upstream comparison. Rely on the checksum, the calibration gate and a synthetic zero-shot test. C-02 then stays open and needs an owner exception.

The bulk fetch waits for the fixture and for its comparison to pass. The owner starts the cloud session; Claude then runs the comparison locally.

## 10. Process and packets

No unreviewed code is committed (TLPLAN-05). The parked snapshot in `work/reviews/inputs/tastelab-1b-parked/` (ignored by Git) is a read-only input that each packet names explicitly. Each packet writes complete files in its own worktree, starting from that snapshot and applying the fixes of sections 4 to 6. Packets are chosen so that each one is complete in its dependencies and needs no other packet's uncommitted code.

1. Plan check of this plan (Astra, max effort, standard tier), then a scoped re-check of the changes it causes.
2. Commit this plan alone on the branch `claude/tastelab-1b` from `main`.
3. **Recorded API responses.** Claude records a few real responses with plain HTTP GETs: at most two search or item requests per source, metadata only, at each source's interval and with the tool's User-Agent. This is not the fetch code, which runs only after its review. The responses stay private in `work/loop-memory/tastelab/recorded/` and are given to K2 as an input. The committed tests use synthetic responses with the same field structure and no third-party text.
4. **Wave 1, phase 1b.** Codex packets through `--kind implement`, run in parallel. Nobody commits while they run.

   | Packet | Files | Findings |
   |---|---|---|
   | K1 installer (critical) | `tools/toolchain/bootstrap.py`, `tools/toolchain.lock.json`, `tools/python/tastelab.in`, `tools/python/tastelab.txt`, `tests/test_bootstrap.py` | A-001, A-004 |
   | K2 image library | `__init__.py`, `common.py`, `net.py`, `sources.py` (all eight adapters), `seeds.py`, `seeds.default.yaml`, `probes.default.yaml`, `images.py`, `clip.py`, `embed.py`, `screen.py`, `store.py`, `fetch.py`, `bundle.py`, `licences.py`, their tests and the runner `tests/test_tastelab_images.py` | A-002, A-003, A-005, A-006, C-01, C-03, C-04, TL-B02, TL-B03, TL-B06, TLPLAN-01, TLPLAN-03 |
   | K3 page logic | `page/bundle.js`, `page/licences.js`, `page/images.js`, `page/library.js` and their node tests | TLPLAN-01, TLPLAN-03, TLPLAN-04 |

   - Each packet's file list includes every runtime file that its modules and tests read (TLPLAN-07). Before the run, the integrator checks the list against the snapshot's imports and file reads.
   - K2's acceptance check runs with the existing local Taste Lab environment. After the owner's `approve`, the environment is rebuilt from the reviewed lockfile and the suite runs again before acceptance.
   - The licence table exists twice, as `licences.py` and `page/licences.js`. After integration, a test checks that the two tables are identical.
   - Claude regenerates `tastelab.txt` before K1, because that needs the network.
   - After K3 is committed, packet K5 writes the Images tab (`index.html`, `app.js`, `page/images-ui.js`, `page/images-worker.js`, `page/idb-cache.js` and their node tests) through `--kind implement`, and Claude reviews it.
5. Claude reviews and applies every patch, and then the checks of section 1 run on the integrated candidate.
6. One full review of the finished phase 1b candidate: Astra, in two shards (installer and model; pipeline and page). K1 is a Codex-authored critical change, so a Sol review covers it too. At most two scoped verification rounds follow. Commit, pull request and merge follow `AGENTS.md`.
7. The owner runs `approve`. Then come the install, the model checks, the calibration gate, the first class A fetch and the first bundle.
8. Publish the page after the owner's go-ahead, then the functional pass.
9. **Wave 2, phase 2,** from the merged phase 1b. One packet, K4: `learn.py`, `rate.py`, `qml/Rate.qml`, `mapping.py`, `tastemap.py`, `explore.py` and their tests (TL-B01, TL-B04, TL-B05, TLPLAN-02, C-05, C-06, TL-B07, TL-B08). The same review rules apply. Then the class B fetch, the rating window and the taste map run, all locally.
10. C-02: the reference fixture of section 5.2, produced under option (a) of section 9. A Codex packet writes the comparison, the cloud script and their tests; the owner's cloud session adds the fixture. The bulk fetch waits for it.
