# Required fake kernel rows

This is a specification and fixed fake data, **not an implementation or real
engine**. It does not register another model, alter `600-cell-Full`, or claim
geometry, reachability or engine oracle agreement. All decisions are **open**.
Every value below comes from `fixture.json`. No session file is opened.

The fake label array has four slots and starts [1,0,2,3]. Letter permutations
map source indexes to destination indexes: for each source s, write old[s] into
new[permutation[s]]. Apply letters chronologically. Words are never simplified.
The fake cap counts, floors, blocked flags and nu arrays are fixed independent
stimuli; they are not mathematical claims about these four slots or the real
puzzle. The public matrices and correction words are explicit constants, not
state reads. The input identity is always the fake identity, never a real engine
model digest. All derived arrays are immutable values.

| Row | Arguments | Exact result |
| --- | --- | --- |
| `caps` | none | `caps` in fixture order |
| `counts`, `floors`, `blocked`, `bits`, `nu2`, `nu5` | none | Corresponding fixture array, state-derived |
| `snapshot` | none | Copy of the four initial labels, state-derived |
| `word_check` | Word | Object with `word`, `end_labels`, `moved_slots` in increasing source order, `protected` and `coverage:{letters,complete:true}` |
| `score` | Word | Number of end labels unequal to their indexes, state-derived |
| `setup` | Target | Corresponding fixed ordered setup list; requires a manual named target in solving profile |
| `preview_outcome` | Receipt | `{code,protected}` for that proposal, state-derived; inaccessible to a previewing program under separation |
| `batch_check` | Words | Ordered word_check results, one word per declared cancellation chunk |

`word_check` composes the full permutation to find `moved_slots`. `protected`
means every protected source slot is fixed by that permutation, not merely that
the final label happens to match. A false check is a returned value, not a
refusal. A preview of such a word is refused by the fake kernel with
`KN2:protection.piece_would_move`. Successful previews have `code:null` and
`protected:true`. Literal `c` and `d` move protected slot 3 and supply the refusal
probes. Every frozen setup fixes buffer slot 3. This table is not permission to
derive arbitrary candidate words in the solving profile (BND.57).

The adapter must provide a preview request, workspace get/put and recorded pick.
Those are host-mediated rows, not direct kernel mutation. Their provenance rules:

- Build a block with version, form, source/module pins, author, producing entry,
  profile, dependencies, parts, pick and declassifier uses. The program supplies
  none of those fields. Runtime-specific IDs remain host envelope data.
- Under labels, snapshot and derived values, preview outcomes, refusals and
  state-dependent control have `state`; research results also have `research`.
  Unknown input provenance stays `unknown`. Join dependencies through all parts.
  Under separation retain `read_state` and enough stored profile/parts metadata
  to enforce later reads and previews; a new run does not clear provenance.
- Setup is the fixed named-target declassifier. Record the manual target, entry
  digest, table digest and its output. Pick records the producer/list digest,
  whether it was a fixed entry/object set, option digest and chosen index/content.
  Picking a computed singleton keeps its option-set labels (BND.55).
- A solving preview may use a manual word/target or the fixed named setup. There
  is no own-tool solving-preview grant. Research parts cannot reach a solving
  preview. A missing/unreadable stored block uses the active provenance refusal.
- Under labels a public receipt's outcome has `state`. Under separation the
  program receives only a fixed acknowledgement independent of success/refusal;
  it cannot obtain `preview_outcome`. The separate shell observer retains the
  full outcome. A probe must not branch on a hidden error through an exception.
- Tool-authoring/research `preview` records a capture with provenance and makes
  no engine preview. No profile exposes a commit token or commit operation.

Each permitted solving preview replaces the fake pending preview so that the
stop-channel stimulus of BND.46 is present. This is a frozen adversarial harness
condition, **not a decision on KN5.12**. A candidate that refuses replacement must
report that actual code/layer; it cannot silently shorten the case or claim the
label mechanism closed it. The candidate/form remains subject to the frozen
expected-code criterion. The fixture keeps an initial manual preview specifically
to make preservation and emission observable.

The rows' input-dependent costs are in `fixture.json`. Before a batch chunk,
charge that word's declared cost; cancellation between chunks records the true
completed prefix. Caches may return only validated values and never alter logical
cost or memory. The four independent budget probes specify exact used counters;
there is no host performance claim. Scalar arithmetic and module/loop metering
must follow the experiment-only schedule in `profile.json`, with a later
candidate's mapping pinned before comparison.

Planted clock/file/network/process traps are test resources named only by logical
IDs. The candidate must refuse before the host trap is called, and no marker may
reach any result, diagnostic, view, workspace or output. In this packet the traps
are JSON data only: no endpoint, file read, process launch or clock probe runs.

Fresh data are copied per case. Only explicitly sequential runs within a case
share workspace and pending preview. The session label array, revision, journal,
protection, preferences and recovery checkpoint never change. A validator of this
specification does not demonstrate enforcement, determinism, cancellation, cache
behavior or fault containment in a candidate.
