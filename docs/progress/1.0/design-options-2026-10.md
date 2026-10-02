# Design options for ten open gaps (October 2026)

Status: **options for the owner to choose from, not decisions.** Nothing here changes the charter, a rule, an ADR or a gate. After the owner picks (for example "1B 2A 3C …"), the choices are recorded in the wiki owner decisions and in the charter draft; until then every option, threshold and number below is an unverified proposal.

Source: the owner's second round of feedback on the Taste Lab notes (2 October 2026), ten gaps in the owner's wording. Basis: [solving-workflow](solving-workflow.md), charter section 5 ([charter-2.0-draft](charter-2.0-draft.md); M6, D7, D8, A6 and anti-goals 1 to 13 are the main yardstick), [owner decisions of 2 October](../../wiki/decisions/owner-decisions-2026-10-02.md), the design track in [stage-2-experiment-protocol](stage-2-experiment-protocol.md) section 4, and the gate in [renderer-candidates](../../wiki/decisions/renderer-candidates.md).

## How this was made

Two independent analyses of the same packet, sealed before either was read:
- **Claude** wrote its answer first and recorded its SHA-256 digest (`4661ff51…0936`, 18:50 UTC).
- **Codex** (senior model, read-only wrapper call `20261002T184745Z-1996a78d`, source identity checked) answered the same packet without access to Claude's answer.

This page merges the two. Where both proposed the same idea, it appears once. Each gap ends with one recommendation and a line "Claude and Codex differ" where they did. Option letters are this page's own; they do not match the letters inside either sealed answer. Every option has five parts: approach, pros, costs, engineering and renderer, conflicts with recorded decisions.

Fixed for every option: rendering never changes mechanical state or relabels pieces; all 259,800 sticker slots and all 1,200 generators stay; the renderer gate (full detail, average at least 30 fps and p99 at most 33.3 ms on the RTX 4070 Laptop GPU) holds in the shipped look (E1); the human-solve boundary holds (the program never chooses, outputs or executes a solving macro by itself).

## Summary

| Gap | Options | Recommended | Claude / Codex sealed picks |
|---|---|---|---|
| 1 Rewards on real progress | 1A structure only · 1B structure + milestone unlocks · 1C on-demand achievement record · 1D opt-in mastery statistics | **1B** | Claude: layered, close to 1B · Codex: 1A |
| 2 Encoding beyond colour | 2A one channel per question · 2B focus and context · 2C stable ring atlas · 2D colour × pattern | **2A** | both 2A (differ on what texture carries) |
| 3 Themes: fixed vs variable | 3A fixed meaning, bounded tokens · 3B authored theme families · 3C scene looks on 3A | **3C** | Claude: 3C · Codex: 3A |
| 4 Collecting interests and nexus points | 4A annotated pairs and nexus cards · 4B nexus map · 4C embeddings first · 4D taste diary | **4A** | Claude: 4B · Codex: 4A |
| 5 Learning curve and key opening | 5A competence checks + soft key disclosure · 5B user-chosen chapters · 5C in-situ metrics · 5D hard gating | **5A** | both 5A |
| 6 Global, Local and other views | 6A two views + shared analysis dock · 6B add an Orbit view · 6C add an Effect view · 6D one canvas with lenses | **6B** | Claude: 6B · Codex: 6A |
| 7 Core vs explorer entry | 7A two doors, one workspace · 7B separate exploration workspace · 7C profiles · 7D no separation | **7A** | both 7A |
| 8 Role priority | 8A order of criteria · 8B constraints, then the core workflow · 8C domain stewardship · 8D experiment first | **8B** | Claude: 8B · Codex: 8A |
| 9 Text cap and term diagrams | 9A one cap + term cards · 9B budgets by context · 9C budgets by region · 9D no sentences at rest | **9A** | both 9A (30 vs 40 words) |
| 10 When sound comes | 10A full sound with the motion table in 2.3 · 10B audition in G5, minimal set in the slice · 10C sound slots in 2.3, sounds after the slice · 10D silent 1.0 | **10B** | Claude: 10C · Codex: 10B |

## Framing notes (both analyses)

- **Palette (gap 2).** The sweep that found 0 of 4,096 palettes with one colour per ring is a sampled search, not a proof of impossibility. And even a feasible ring colouring does not identify every sticker label: colour can carry a class, not identity.
- **Four things that are never merged (gaps 1 and 5):** learning attainment (the user showed a skill), legal execution (the turn was allowed), current progress (what is true now, after undo) and exact completion (the completion check passed). A reward or a level must say which one it is.
- **Key opening (gap 5)** may hide or explain keys; it must never remove keyboard access to a command (E5) or change what a key means without the user's action (solving workflow 4.3).
- **Views (gap 6).** The charter draft already gives the two questions (D8: Local, does this piece match its target here; Global, how do these two places relate). The draft is unsigned, so the questions are still the owner's to confirm.
- **Taste Lab phase 1b (gap 4)** still needs the owner's approval for its model, runtime and source domains. Approved class A uploads do not include class B images, benchmark works or session material.
- **Numbers (gaps 5 and 9)** such as word caps or learning thresholds are tested on tasks in 2.3 (does the user act correctly?), not accepted by counting words or by model agreement.

## 1. Rewards on real progress, without a game interface

Question: how do rewards attach to real progress so that learning feels like clearing levels while the screen stays an expert work surface (anti-goal 7, A6)?

**1A. The structure is the reward (and nothing else).**
- Approach: after an exact progress check on the committed state, the puzzle itself changes how it is drawn: a seated piece's slot settles, a finished block's outline consolidates, a completed orbit sweeps once along its own structure. No badges, points, pop-ups or counters. Gains, losses and remaining residuals are shown together; no fictitious "percent complete".
- Pros: A6 exactly ("rewards come from the structure itself"); anti-goals 7 and 8; the reward teaches what changed mathematically.
- Costs: event rules need care (undo, redo, stale display); may feel too thin to give the "level cleared" feeling the owner asked for.
- Engineering and renderer: events come from authoritative results for the displayed committed revision; previews never award anything. A short highlight or material transition on a subset of slots after commit; never during a turn or picking; costed in H-06 and recorded in the motion table (H-03).
- Conflicts: none.

**1B. Structure, plus milestone unlocks of real content.** *(recommended)*
- Approach: 1A always. At milestones (first star, first block, position finish, first orbit, stage milestones) the user also gains real content: the matching theory-book page (S6) and glossary terms with their diagrams (M6), the solver's own macro saved as a template, and at larger milestones a new scene preset or theme. All rewards play after the commit, never block input (the next key cancels them), are revoked silently on undo, and quieten after repetition.
- Pros: gives the "cleared a level" feeling through content the user needs anyway, not decoration; ties rewards to the learning path (D7) and the language (M6).
- Costs: theory-book pages and term cards must exist and be reviewed by a person (anti-goal 3); content unlocking must never hide a command (E5: unlocks open explanations and presets, never functions).
- Engineering and renderer: an event log in the session store; the unlock state is presentation only. Same renderer cost as 1A.
- Conflicts: none, provided no command is locked.

**1C. On-demand achievement record.**
- Approach: restrained immediate feedback (1A), plus an optional illustrated record of structures built and skills demonstrated, opened by the user. Each entry shows its exact conditions and whether they still hold now.
- Pros: small advances stay visible after a long session without occupying the work screen.
- Costs: may feel delayed; must separate history from current state (an undone block must not look complete); risk of growing into a trophy room.
- Engineering and renderer: derived from verified history; comparisons rendered on demand and cancelled when work resumes. No second mechanical state, no automatic screenshot archive.
- Conflicts: none, if it stays a structural record rather than a collection.

**1D. Opt-in mastery statistics.**
- Approach: quiet local records after each orbit (stars used, median preparation time, undo and conflict rate, change against the previous orbit). Off by default.
- Pros: motivates experts; doubles as the learning-curve measure of gap 5.
- Costs: invites score-chasing; with two or three core users there is little to compare against.
- Engineering and renderer: a local metrics store, never uploaded; no renderer cost.
- Conflicts: none if local and switchable off.

Recommendation: **1B.** 1A alone keeps the screen clean but risks being too quiet for "a game with generous rewards" (charter, learning and reward); 1B adds weight through content that teaches, not through game decoration. 1C and 1D can be added later without changing 1B.
Claude and Codex differ: Codex recommended 1A, warning that richer rewards blur learning progress with puzzle progress. 1B keeps that separation explicit (see the ladder).
Cheapest test (G5): walk the ladder below against fixture snapshots (wrong orientation, a referenced block, mixed gain and loss, a permitted non-identity residual, a missing certificate, undo, an imported already-solved state). Any false completion rejects the event definition. Then the owner judges three short fixture clips for whether the small rewards feel earned.

### Reward ladder (first draft, merged)

Levels follow the steps of the solving workflow (D7). This is a teaching order, not a mandatory solve sequence. Every row names which kind of progress it is: **learning** (the user showed a skill on a fresh task) or **puzzle** (a condition is true in the current committed state). Every cue is optional, non-blocking, derived from the actual geometry, and revoked on undo.

| Level | Kind | Real progress event (exact check) | Reward | For experts |
|---|---|---|---|---|
| L0 First turn | learning | first intentional legal turn, then undo and recognising the restored state (S1) | the turned cap outlined once with the motion-table "settle"; first glossary card | ordinary turn feedback |
| L1 Reading the structure | learning | first isolation of a cap, an orbit and a piece's Home (S3) | theory-book page "Orbits"; terms *orbit*, *Home* with glyphs | none |
| L2 Target, piece, buffers | learning | on a fresh task, the user tells apart destination, current occupant and buffers A and B | the four relations drawn as linked markers | compact persistent markers |
| L3 Compose and check | learning | the user enters their own macro, names the setup target and reads its full effect (collateral, protection) correctly | a short graphical account of the operation; the macro enters the macro base as the solver's template. Permission alone earns nothing | normal analysis only |
| L4 First insertion | puzzle | after explicit execution, the declared insertion holds and earlier achievements stay intact | the piece settles into the block drawing; any remaining orientation error stays visible | a brief local marker |
| L5 Block built or extended | puzzle | the solver's block conditions all hold, including orientation and reference conditions | the block outline consolidates; protection shown separately; block lock offered | outline update and exact counts |
| L6 Position finish | puzzle | all non-buffer destinations hold their correct occupants | position marks clear; orientation and buffer residuals stay explicit; "Endgame" chapter opens | compact stage update, never the orbit cue |
| L7 Non-buffer orientation | puzzle | all non-buffer pieces exact | orientation glyphs clear in one pass, with the paired transfer into the buffer shown | before/after frame |
| L8 Buffer A, then buffer B | puzzle | A exact; then B confirmed exact or corrected by a macro the solver chose and executed (including the D5 and A5 cases) | each residual disappears; the correction saved as a template; "Residuals" page opens. A permitted non-identity residual is unfinished; a missing certificate stays a gap; an impossible residual gets a diagnosis, not a celebration | residual indicators only |
| L9 Orbit complete | puzzle | every identity, position, orientation and sticker of the orbit passes the completion check (S5, orbit) | signature moment: the orbit sweeps once along the stage order; its atlas entry fills; optional sound | one restrained cue, shortened after five orbits |
| L10 Stage milestones | puzzle | for example all collateral-free orbits; half the stage order | an atlas chapter; a new scene preset or theme becomes available | once |
| L11 Whole puzzle | puzzle | every label exact (S5, whole) | the full signature moment; an exportable solve record (session identity, commit log, setup-search use) as the record of a human solve | one quiet acknowledgement unless more was asked for |

Across all levels: undo and regression update the current status at once; major rewards never replay through undo and redo; legitimate preparatory moves are never punished; importing an already-solved state never counts as a demonstrated skill.

## 2. Graphic encoding beyond colour

Question: when colour alone cannot carry identity (in the sampled palette search no palette with one colour per ring passed the colour-vision-deficiency check, while 4 to 8 classes from a proper colouring did; this is a sampled result, not a proof), what do shape, texture, position and motion each carry?

**2A. One channel per question.** *(recommended)*
- Approach:
  - colour: a class from a proper colouring (4 to 8 classes, CVD-checked), a local separation aid, never identity;
  - position and layout: identity and relations (where it is, where it belongs; Local and Global keep target, piece and buffers in fixed screen places);
  - shape (glyphs drawn from the tetrahedral geometry, D3): role (target, occupant, buffer A, buffer B, reference, block member);
  - texture or pattern: status (protected, locked, unknown frame, unchecked prefix);
  - motion: change only (preview, collateral, commit, reward), with a static arrow as its fallback;
  - exact ring, cell and piece IDs on focus resolve any repeated colour or pattern.
- Pros: role and status never depend on hue, so CVD is safe; each channel answers one question; users do not memorise twenty arbitrary textures.
- Costs: a glyph set, pattern shaders and a written encoding table reviewed by the mathematician (M2); glyph and focus rules need dense-scene testing.
- Engineering and renderer: glyphs and patterns are overlays, never replacement geometry or relabelled state; drawn for the focus set (current orbit, block, buffers), not all 259,800 slots; procedural patterns are cheap; each feature's cost goes into H-06.
- Conflicts: none; it narrows G4 from "identity colours" to "class colours".

**2B. Focus and context.**
- Approach: outside the focus set the puzzle is drawn in desaturated classes; detailed encoding applies only inside the focus. Identity elsewhere is read by inspecting, or by a key that pulses all stickers of a chosen class, orbit or role.
- Pros: cheapest renderer path and least clutter; follows "one orbit at a time".
- Costs: the whole-puzzle state is less visible; protection conflicts must break through the dimming (solving workflow 4.3: a conflict is never hidden by a filter).
- Engineering and renderer: one dimming pass plus focus overlays.
- Conflicts: none, if conflicts always break through.

**2C. Stable ring atlas.**
- Approach: the twenty rings sit in a fixed, explicitly schematic index linked to the 3D view. Position in the index carries ring identity; shapes mark roles; textures compare current and Home; colour stays a local adjacency aid; entries expand to their cells.
- Pros: distant rings stay distinguishable without twenty unique swatches; identity survives camera movement.
- Costs: screen area and a learned index; neighbouring entries must not suggest adjacency or a move route.
- Engineering and renderer: a second, light projection of the same view model; selection through explicit commands; bounded highlights in the main renderer.
- Conflicts: none, if the atlas states its mathematical basis (M2) and never stands in for the geometry.

**2D. Colour × pattern double coding.**
- Approach: combine hue with lightness or pattern to reach about 20 classes (for example 5 hues × 4 patterns), so every sticker shows a fine class at a glance.
- Pros: keeps glanceable fine classes.
- Costs: patterns on small stickers at full detail alias into noise (anti-goal 8 risk); legibility at distance unproven.
- Engineering and renderer: a per-pixel pattern on every slot, plus mipmapping and anti-aliasing work; a real H-06 cost.
- Conflicts: none.

Recommendation: **2A.** Both analyses picked it: it gives each visual channel one job and keeps the CVD-safe palette within what was found feasible.
Claude and Codex differ: on what texture carries. Claude: status (protected, locked, unknown, unchecked). Codex: a small number of compared structural groups, with status shown elsewhere. The G4 test decides.
Cheapest test (G4): static full-detail crops with repeated-colour rings, occlusion and small stickers; users identify target, A/B, ring identity and orientation under normal colour, CVD simulation, greyscale and motion off. Any critical misidentification rejects the encoding; only survivors get a renderer cost run.

## 3. Themes: what is fixed and what may vary

Question: several themes share one design language; which elements are fixed and which may vary?

**3A. Fixed meaning, bounded theme tokens.**
- Approach: fixed: geometry and projection, layout regions and anchors, command meanings and key bindings, glossary and glyph skeletons, the encoding table of gap 2, evidence and status distinctions, motion meanings and timing bands (H-03), minimum ΔE and contrast, type-scale ratios, hit areas. Variable: palette character (within checks), materials and light, background, frame and panel treatment, typefaces from an approved set with mathematical notation (D6), easing character within bands, sound set, celebration styling. Users tune a small validated parameter range.
- Pros: switching theme never requires relearning; one component system; checks run per theme.
- Costs: token variation may not give enough aesthetic range (anti-goal 10 risk); safe individual sliders do not make safe combinations.
- Engineering and renderer: three tiers of tokens (core, language, skin); combined settings are validated before use; every shipped theme passes the gate in its own look (E1), including p99 and VRAM.
- Conflicts: none.

**3B. Authored theme families on a common interaction skeleton.**
- Approach: commands, glyphs, comparison positions, accessibility alternatives and layout anchors fixed; each theme is an authored family (observatory, museum, watch, stationery) of frame surfaces, materials, typography and scene treatments, which may also change density within one region topology. Users tune accents, density and effect intensity within the family.
- Pros: the strongest aesthetic range (anti-goals 4, 10, 12, 13).
- Costs: more asset production and cross-theme review; each new theme is real design work; picking and hit areas verified per family.
- Engineering and renderer: shared components expose a finite set of replaceable treatments; no theme-specific command logic or arbitrary shader plug-ins; each family has its own costed presets.
- Conflicts: partly with "one design language" if densities or layouts drift apart.

**3C. Scene looks on the 3A invariants.** *(recommended)*
- Approach: 3A's fixed set. A theme defines a look per scene (solving: calm and dense; inspecting: analytic; celebrating: expressive). Users tune the skin within ranges checked live (ΔE, contrast, the cost meter).
- Pros: matches the owner's "a scene may have its own look" (charter section 5) and Taste Lab's scene context, while keeping one language.
- Costs: scenes × themes to verify; scene transitions must not move the layout or disturb work.
- Engineering and renderer: scene is a token layer switched by view state, not a new renderer path; each scene look is costed, and the most expensive one must pass the gate.
- Conflicts: none.

Recommendation: **3C.** It is 3A plus the owner's own scene requirement; the scene layer is where the expressive range comes from, without touching meaning.
Claude and Codex differ: Codex recommended 3A, citing the lower test load. 3C costs one more token layer and a scene-by-theme check.
Cheapest test (G2/G3): style one difficult screen in three markedly different token sets and in the three scenes. Reject a set that changes how status or picking is read, or that the owner cannot tell apart. Send survivors through H-01 and H-02 and cost their expensive combinations with H-06.

## 4. Collecting the owner's interests and nexus points

Question: how to collect the owner's interests and nexus points and use them in Taste Lab phase 1b and in the design track. The owner supplies the content; these are methods only.

**4A. Annotated pairs and nexus cards.** *(recommended)*
- Approach: the owner supplies a small set of liked and disliked references and marks the specific property, the intended feeling and any connection to another reference. Subject is kept apart from visual treatment. Each connection becomes a nexus card with three fields: the interest, its bridge to the project (symmetry, quaternions, Hopf fibres, the stage order, the instrument metaphor) and the product place it could take (theme, signature moment, theory-book figure, icon motif, celebration). Agents extract transferable attributes only (palette relations, rhythm, material, typography, structure, motion), never assets. In phase 1b, embeddings organise eligible class A images and suggest comparisons; the owner's ratings and notes decide.
- Pros: cheap, interpretable, keeps why a reference matters; the bridge field stops pastiche and lets the mathematician check that a visual does not misrepresent (M2).
- Costs: owner annotation time; a liked reference may not transfer to a usable interface.
- Engineering and renderer: cards map to G2 manifesto choices, G3 parameters and H-01 features; embeddings never become renderer requirements.
- Conflicts: none. The upload allowlist applies as recorded: no class B images, benchmark works or session material; phase 1b needs its own approval.

**4B. Nexus map.**
- Approach: the owner arranges references into a small map, names the connections and marks indispensable anchors, disliked combinations and scene preferences; each edge passes through a bridge to the project's structure or mathematics. Embeddings later help retrieval but never define edges or reduce the map to an average.
- Pros: captures relations between interests that isolated choices miss; a durable shared vocabulary for the design track.
- Costs: more up-front work; can over-formalise and suppress the surprising combinations the owner wants.
- Engineering and renderer: map nodes become traceable design hypotheses and bounded preset families, never assets inserted into the product.
- Conflicts: none.

**4C. Embeddings first.**
- Approach: Taste Lab 1b swipes a broad class A corpus seeded with the owner's references; clusters of liked images are described by agents and named by the owner; clusters seed themes.
- Pros: finds latent taste cheaply per item.
- Costs: needs the 1b approval (allowlist, about 1 GB model, new source domains); embeddings capture surface style and subject; corpus bias toward mainstream taste (anti-goal 13).
- Engineering and renderer: none at runtime; the Taste Lab session owns the tooling.
- Conflicts: needs an owner approval not yet given (phase 1b).

**4D. Taste diary.**
- Approach: short daily notes with one reference; a weekly agent summary into the cards.
- Pros: light; already in the protocol's early days.
- Costs: slow to converge.
- Engineering and renderer: none.
- Conflicts: none.

Recommendation: **4A**, with the bridge field. It is the cheapest method that keeps the reason behind each reference, and the bridge is what turns a liked reference into a nexus point rather than decoration.
Claude and Codex differ: Claude's sealed pick was the map (4B); Codex picked annotated pairs first. The merged 4A takes Codex's order (pairs first) and Claude's bridge. 4B follows only if the test shows it finds what the cards miss.
Cheapest test: the owner annotates six pairs (same subject with different treatment, different subjects with similar treatment); agents make two local sketches from the annotations; the owner rejects any inferred attribute they cannot recognise. Then one time-boxed paper map from the same references: keep 4B only if it yields a combination the owner values that the cards missed.

## 5. Measuring the learning curve and opening keys step by step

Question: how to measure the learning curve, and how to open key bindings level by level.

**5A. Competence checks with soft key disclosure.** *(recommended)*
- Approach: measure on workflow tasks matching the ladder (first turn and undo; target and buffer identification; composing a draft; reading a full effect; explicit execution; keeping a block; endgame diagnosis), each with equivalent fixture variants on fresh isolated data. Record independent completion, critical mistakes, help use, preparation time, context switches and recovery after an interruption, and transfer to a new target. Report time separately from correctness; never judge by speed alone. Keys open by level (L0 grip, twist and undo; L1 inspect and isolate; L2 macro entry and star; L3 setup search and work sheets; L4 blocks and locks; L5 endgame; L6 custom key maps). "Open" means shown in the on-screen key map and hints; the command palette always offers everything (E5). An expert can open all at once or import a key map.
- Pros: measures usable skill, not clicks or tutorial time; keys never move.
- Costs: two or three core users give within-person observations, not population claims; fixtures for each level must be built.
- Engineering and renderer: the command table is complete from the start; all 1,200 generators reachable; key-set changes are explicit; key mappings never follow the camera silently; coaching overlays never capture working input.
- Conflicts: none.

**5B. User-chosen chapters and a key reference.**
- Approach: a compact chapter selector and an always-available illustrated key reference; the user chooses which key groups get explanations; nothing infers or enforces a level. Measures as 5A.
- Pros: predictable and cheap; suits experienced hypercubers who need only some concepts.
- Costs: users may skip essential ideas (for example draft versus live turns); self-assessment can exceed competence.
- Engineering and renderer: presentation-only reference layers and saved preferences.
- Conflicts: none, if the chapters still form a complete path (D7).

**5C. In-situ metrics on real solving.**
- Approach: the same soft disclosure; measure on actual solving (preparation time per star, context switches, undo rate), locally only, no extra tasks.
- Pros: no practice tasks to build or sit through.
- Costs: noisy and confounded by orbit difficulty; cannot tell a slow learner from a hard orbit.
- Engineering and renderer: a local metrics store (shared with 1D).
- Conflicts: none if local only.

**5D. Hard gating.**
- Approach: keys are disabled until a level check passes.
- Pros: the clearest "level cleared" feel.
- Costs: blocks experts and returning users.
- Engineering and renderer: per-command enable state.
- Conflicts: **E5** (every action reachable from the keyboard) and the core-user priority; listed for completeness.

Recommendation: **5A.** Both analyses picked it.
Claude and Codex differ: Claude proposed "learned" as time within a factor of the owner's own baseline; Codex warned that speed penalises careful reasoning and proposed transfer to a new task as the criterion. The merged 5A uses transfer and correctness, with time reported separately.
Cheapest test (G6, then the living test): counterbalanced equivalent fixtures, then a different target and an interruption on a later day. Trial acceptance: first legal turn within 30 seconds (S1), no critical state or protection misunderstanding, successful transfer to the new task, keyboard reachability without coaching.

## 6. Global and Local, and whether other views are needed

Question: Global and Local are redesigned from scratch; which single question does each answer, and are other views needed?

Both analyses keep the charter draft's two questions: **Local**, does this piece match its target here (current against Home frame, with buffer context); **Global**, how do these two places relate (actual structure, incidence, navigation). Inspecting, assigning a target, assigning a grip and moving the camera stay separate actions in every option. Preview is a state of every view, not a view.

**6A. Two spatial views and a shared analysis dock.**
- Approach: Local and Global only; a docked panel answers complete-operation questions (cycles, collateral, protection, residuals). The dock is not a third spatial world.
- Pros: fewest surfaces; follows D8 exactly.
- Costs: the dock gets crowded in long macro analysis; residual progress has no home of its own.
- Engineering and renderer: both views read one target, piece, buffer and protection context; shared geometry; combined viewport and compositing cost measured.
- Conflicts: none.

**6B. Add an Orbit view.** *(recommended)*
- Approach: 6A plus an Orbit view answering "what is left in this orbit?": a residual map of positions, buffers, orientation, blocks and protection; zoomed out it shows all 35 orbits in stage order. The operation effect stays in the dock.
- Pros: these are the three questions of the loop's first steps (choose a target, find the piece, relate places); the solving workflow (section 6) already makes the per-orbit residual drive the stage display; the Orbit view is also where the reward ladder's orbit and stage levels live.
- Costs: one more view to design and keep linked.
- Engineering and renderer: the Orbit view is a diagram from the session's residual data, not a third continuously rendered 3D scene; low renderer cost.
- Conflicts: none.

**6C. Add an Effect view.**
- Approach: 6A or 6B plus a full Effect view: "what will this complete operation change and preserve?", with all-orbit net effects, protection results and a chronological witness, clearly separating net safety from intermediate motion.
- Pros: difficult macros and endgames get room; collateral is harder to overlook (S4).
- Costs: another switch in the fast loop; a stale review must never survive an edit.
- Engineering and renderer: derived from the same analysed operation and revision; changing roles, target, macro or protection invalidates it; diagrams and an explicitly requested witness replay rather than a third live 3D scene.
- Conflicts: none; it analyses solver-supplied operations only.

**6D. One canvas with lenses.**
- Approach: one central 3D view whose lens (Local, Global, Orbit) changes projection and filters with animated transitions.
- Pros: the frame never stands apart from the work (anti-goal 11); camera continuity.
- Costs: re-projection transitions in the renderer; hard to show two places at once.
- Engineering and renderer: transition cost inside the gate; one viewport.
- Conflicts: none.

Recommendation: **6B.** The per-orbit residual is the question a solver asks most often between operations, and it has no home in 6A.
Claude and Codex differ: Codex recommended 6A and would put residuals in the dock. On the Effect view (6C), Claude listed it as a plain alternative; Codex would adopt it only if it improves correct reading of effects enough to justify the extra switching. This page suggests Codex's condition.
Cheapest test (G1): storyboard one complete insertion and one residual correction, including hidden collateral and a reference-only buffer. Users answer the view questions and name every protected effect without re-entering context. Compare 6A and 6B on "what is left in this orbit?" questions; reject any layout that hides an operation's review status.

## 7. Entries for core solvers and exploring users

Question: how to separate the entry of core solvers from that of explorers; is a separate exploration mode needed?

**7A. Two doors into one workspace.** *(recommended)*
- Approach: the first screen offers "Continue solving" and "Explore the 600-cell". Returning solvers resume their context directly; explorers get a small curated full-model scene in a sandbox session with optional explanations (S1, S3, S6). Same tools, keys and command meanings.
- Pros: one product to maintain; an explorer who gets interested continues on the learning path.
- Costs: the explore door needs designed tours; fixture and session identity must be unmistakable.
- Engineering and renderer: exploration uses separate synthetic session data with the complete model, all generators and normal validation; entering it never resets a personal solve. Same renderer and view model.
- Conflicts: none.

**7B. A separate exploration workspace.**
- Approach: an explicitly selected workspace with structural chapters, theory links and curated scenes, beside the solving workspace. Two variants: (i) same command palette and rules (Codex); (ii) a reduced UI without the macro base and protection, for the lowest barrier (Claude).
- Pros: a coherent route through caps, orbits, frames and fibres without crowding the core layout.
- Costs: more navigation and curation; risks becoming a second product whose habits do not transfer; variant (ii) means two UIs to maintain.
- Engineering and renderer: shared engine, commands and renderer; isolated fixture sessions and preferences.
- Conflicts: none as a scope option, but it must not slow the core workflow.

**7C. Profiles at first launch.**
- Approach: "Solver" or "Explorer" sets defaults (disclosure level, panels, rewards), switchable later.
- Pros: cheap.
- Costs: a choice made before the user knows the product.
- Engineering and renderer: a preference set only.
- Conflicts: none.

**7D. No separation.**
- Approach: the learning path is the only entry.
- Pros: one path.
- Costs: explorers find it solving-centred; core users repeat onboarding.
- Engineering and renderer: none.
- Conflicts: none.

Recommendation: **7A.** Both analyses picked it.
Cheapest test (G1): first-launch and returning-user storyboards. The explorer reaches an intentional legal turn within 30 seconds and can say the full model is present; the returning solver recovers target, draft and protection without an onboarding detour.

## 8. Default priority when the four roles disagree

Question: the default order when the engineer, mathematician, front-end designer and interaction artist disagree. The owner rules in the end; this only sets the default.

Common to all options: binding inputs are not ranked, they are filters. An option that breaks the mathematical contract, protection, the human-solve boundary, the renderer gate, data safety or an owner design requirement is out before any ranking. No professional vote relaxes a gate.

**8A. Order of criteria.**
- Approach: among admissible options: correctness and truthful representation, then dependable input, recovery and performance, then core workflow and learning, then expressive preference. Mandatory art and design requirements are admission conditions, not last-place preferences.
- Pros: predictable reasons; stops taste from excusing incorrectness and convenience from excusing an unusable product.
- Costs: puts "performance beyond the gate" above the core workflow and look; art tends to get what is left (anti-goals 4, 6, 13).
- Engineering and renderer: disputes produce costed alternatives through H-01, H-06 and H-09.
- Conflicts: charter section 5 (target users): "the core users' workflow decides trade-offs". 8A would rank dependable input, recovery and performance beyond the gate above the core workflow, so choosing it changes that line.

**8B. Constraints, then the core workflow.** *(recommended)*
- Approach: among admissible options: core solver workflow efficiency (designer), then legibility and delight (artist), then cost beyond the gates (engineer), then notation preference beyond correctness (mathematician). Unresolved cases go to a Look Lab or living-test comparison before the owner.
- Pros: follows the charter's "the core users' workflow decides trade-offs"; correctness and performance are already protected by the filter, so the ranking only orders what is left.
- Costs: an engineer's unmeasured "risk" claim must be turned into a measurement, or it loses.
- Engineering and renderer: an engineering objection counts once it is measured (H-06, H-09).
- Conflicts: none.

**8C. Domain stewardship.**
- Approach: after the filter, the relevant role leads: mathematician for concepts and truth conditions, engineer for measured feasibility, designer for task structure and discoverability, artist for expressive treatment; cross-domain disputes go to the core workflow's measured outcome, then the owner with two alternatives.
- Pros: each role improves its domain without a blanket hierarchy.
- Costs: most real conflicts are cross-domain; boundaries get contested.
- Engineering and renderer: uses the existing handoffs.
- Conflicts: none as a proposal workflow.

**8D. Experiment first.**
- Approach: every conflict becomes an experiment card (Look Lab A/B with the cost meter, living test).
- Pros: evidence rather than argument.
- Costs: slow, and spends the owner's time.
- Engineering and renderer: none extra.
- Conflicts: none.

Recommendation: **8B.** Once the binding inputs filter out unsafe options, the charter already says the core workflow decides; 8B writes that down.
Claude and Codex differ: Codex recommended 8A (criteria, with dependable input and performance above workflow and look). Both agree on the filter and that roles never hold a veto; they differ only on where performance beyond the gate and expressive quality rank.
Cheapest test: apply both orders to three sample conflicts (translucent material versus picking; shorter terms versus mathematical precision; reward motion versus typing continuity). Each role must cite a constraint or propose a falsifying observation. Reject an order under which an unsupported preference gets a veto.

## 9. Free text per screen and pairing terms with diagrams

Question: a cap on free text per screen, and how terms are paired with diagrams.

Common to all options: every glossary term has a term card (glyph, one-line definition, one mini diagram rendered from the real geometry, M4), shown on first use and on hover; a term appears in the UI only from the level where it is first needed (M6). Labels, numbers, exact IDs, mathematical notation, glossary terms and key hints are not free text. Error, protection and stale-state messages may exceed a cap (E6); the theory book and help are exempt.

**9A. One cap per screen, plus term cards.** *(recommended)*
- Approach: at rest, a normal working screen shows at most **30 to 40** words of free sentences (the number is set by the G6 test), concentrated in at most one short coaching card. Tooltips one sentence (about 15 words); error messages in at most about 25 words. A lint counts strings per view from the UI string catalogue.
- Pros: one simple, checkable editorial rule against generated clutter, without banning mathematical language.
- Costs: word counts do not measure understanding; exceptions can become loopholes; a designer could meet the cap by replacing a needed word with an ambiguous icon.
- Engineering and renderer: reusable term and diagram components, human-reviewed wording, scalable vector graphics; text equivalents and keyboard access kept; tested at high DPI and with longer strings.
- Conflicts: none, if the cap stays below truthful evidence and actionable errors.

**9B. Budgets by context.**
- Approach: different caps by context: about 20 words while solving, 50 while inspecting, 80 in an explicitly opened learning panel.
- Pros: a dense expert workspace with room to explain on request.
- Costs: more rules; users may keep opening the panel if the solving view leaves out needed context.
- Engineering and renderer: the user opens inspection or learning content explicitly; no automatic context detection.
- Conflicts: none.

**9C. Budgets by region.**
- Approach: status line at most one line, panels at most three lines, dialogs at most two sentences.
- Pros: flexible.
- Costs: harder to check by script; total text can still pile up.
- Engineering and renderer: per-region layout limits.
- Conflicts: none.

**9D. No sentences at rest.**
- Approach: only labels, numbers and glyphs; sentences behind an "explain" key.
- Pros: strongest against anti-goal 2.
- Costs: errors still need text (E6); harder for explorers.
- Engineering and renderer: an explain overlay.
- Conflicts: none, given the E6 exception.

Recommendation: **9A.** Both analyses picked it.
Claude and Codex differ: on the number (Claude 30 words, Codex 40). The merged option leaves 30 to 40 to the test.
Cheapest test (G6): apply the cap to three screens (normal preparation, protection conflict, stale display). Users explain buffer, frame and residual and the permitted next action. Reject any cut that reduces correctness; keep the number only if the screens get clearer.

## 10. In which stage sound is done

Question: in which stage is sound designed and built? Sound is optional, can be switched off, and is designed together with the motion table if adopted (A4).

**10A. Full sound with the motion table in 2.3.**
- Approach: sound for turn, undo, invalid action and solved is designed in G5 and frozen with H-03.
- Pros: A4 literally ("designed together with the motion table").
- Costs: owner time in an already full stage; audio tooling and sound licences now.
- Engineering and renderer: an audio path in the vertical slice.
- Conflicts: none.

**10B. Audition in G5, a minimal set in the slice.** *(recommended)*
- Approach: H-03 gets optional sound semantics beside its motion rows. In G5 the owner compares a few locally produced clips, silent against sounded. If the owner adopts sound, only the chosen minimal cues (for example "solved" and "invalid action") go into the vertical slice. Event timing, mute behaviour and the visual equivalent of every cue freeze at 2.5; the full sound set follows after the slice.
- Pros: sound and motion are designed together (A4) while the expensive work waits until it has shown value.
- Costs: a little owner time in G5; even good sounds can tire with repetition.
- Engineering and renderer: sound consumes presentation events, never drives mechanics, never signals success for a rejected or stale result; behind a portable audio interface; never blocks the render or input path.
- Conflicts: none.

**10C. Sound slots in 2.3, sounds after the slice.**
- Approach: G5 gives each motion-table event a sound slot; the 2.5 freeze includes a small audio interface; sounds are designed with the reward ladder after the slice.
- Pros: no stage-2 time on audio.
- Costs: "together" in A4 is only met by the slots; the motion table may need an ADR update when sound arrives.
- Engineering and renderer: an interface only.
- Conflicts: partly with A4, mitigated by the slots.

**10D. Silent 1.0.**
- Approach: visual-only rewards in 1.0; sound recorded as deferred and revisited after sustained use.
- Pros: smallest scope; no audio fatigue risk.
- Costs: S2 and S5 lose an element the owner named; later adoption reopens the motion table.
- Engineering and renderer: none.
- Conflicts: none, sound being optional, but it is an owner choice.

Recommendation: **10B.** The silent-against-sounded clip comparison costs minutes and settles whether sound is worth any slice time; it keeps A4's "together" without a full sound programme in stage 2.
Claude and Codex differ: Claude's sealed pick was 10C (slots now, sounds after the slice). Codex's reason (a cheap audition now meets A4 and avoids reopening the motion table later) was judged stronger, and the recommendation changed to 10B.
Cheapest test (G5): one short clip, silent and sounded. Only if the owner prefers sound, test the minimal cues in the five-day living test with rapid turns, undo, a rejected execution and mute. Silent operation must stay fully understandable.

## Next step

The owner replies with one letter per gap (for example "1B 2A 3C 4A 5A 6B 7A 8B 9A 10B"), or changes an option. Claude then records the choices in a dated owner-decision page, updates charter section 5, and lists the tests above as stage 2.3 work. Until then this page stays a set of options.
