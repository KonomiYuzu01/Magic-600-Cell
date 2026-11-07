# LP1 execution contract reference vectors

Status: **draft for the owner. Not accepted.** These are abstract accounting examples, not a language, a serialized
IR, a route implementation, measured tariffs or budget presets. The semantic rules are DET.54 to DET.65 in
[chapter 09](../architecture/09-determinism.md); LG4.8 retains the weighted-step alternative. Boundary examples
also consume LG2.20/LG2.22 and EV3.5. They are small rule models over declared inputs, not a source checker or renderer.

- **CV.1 [D]** `vectors.json` names the draft contract, declares every test tariff and supplies concrete `given`
  and `expected` objects. `check_vectors.py` recomputes the expected objects in memory using only the Python standard
  library. Run `python -B docs/architecture/1.0/language/contract-vectors/check_vectors.py`. The script has no update
  mode, writes no file, imports no local module and uses no clock, randomness, subprocess, network or third-party
  package. Its checks establish only agreement between these examples and this small reference model.

- **CV.2 [D]** Each accounting `given` is a finite abstract continuation trace and four explicit limits. Literal fixture
  operands describe the input to the listed continuation; they are not source expressions with an implicit parser
  or materialization schedule. A fixture's synthetic arrays, scratch bounds and zero-slot frames have their stated
  logical sizes; they are not real puzzle states or executable tool programs. Real captured input entry operations
  remain required by DET.57. All fixture meters start at zero and have no unlisted live owners. The trace's `at`
  names and nesting give its stop path; this is test notation, not a proposed source-coordinate or IR encoding.
  A supplied source range consumes HANDOFF H7 and is checked against its fixture bytes. A `source_map` associates
  operation paths with those ranges; a denied operation reports its own range, with the older `source` field as fallback.
  Completing a trace means completing its listed continuations. It adds no invisible top-level return; a complete
  program's return/output continuations must be listed too. Function and child fixtures include their returns.

- **CV.3 [D]** An expected `charges` row is `[path, steps, kernel_cost, memory_reserved, output_bytes, released]`.
  It includes the denied operation. A step failure has zero charges; a secondary failure keeps only its one step.
  `used.memory` is peak reservation/live use; `live_before_cleanup` is captured before a terminal unwind, and
  `live_after_cleanup` is zero. Child unwind releases abandoned owners and unowned reservations without adding an
  operation or refunding cumulative use. It therefore need not appear as a charged release row. The child ledger records effective caps
  after the parent's entry step, local use, stop and transferred payload; ancestors have already counted that use.
  `values`, `generators` and `coverage` are test observations of model state, not uncharged program output. Only an
  `emit` event publishes a fixture output unit.

- **CV.4 [D]** Test tariffs are deliberately small and unmeasured. A synthetic kernel costs `1 + 2*items`. With
  eight-bit test limbs, addition costs the sum of operand limb counts and multiplication their product; a zero
  operand has one bit/limb. Integer result reservations use the input bit bounds in DET.57 and release excess after
  calculation. Text comparison costs both scalar lengths. Decode and encode cost one unit per input/output byte.
  A generator initialization costs its specified hash-input bytes and a draw costs three units. A stream open costs
  one unit; each letter costs one expansion plus two replay units. Complete streams partition those charges into
  chunks of the declared semantic size. Transport partitions are deliberately absent from the accounting function.
  The model records no hardware cost, product tariff or chosen default allowance.

- **CV.5 [D]** The output examples explicitly use **PS1.2 retained JSON**, including ASCII escapes, compact
  separators and no terminal LF, as a test profile. They do not choose p3's new data-core profile. For example,
  the string U+00E9 has canonical fixture bytes `22 5c 75 30 30 65 39 22` and costs eight output bytes. A different
  recorded profile can give a different byte length and key, while H8's canonical-uncompressed-byte unit stays the
  same. There is no second production codec here. Terminal outcomes in fixtures carry no user output; a zero output
  limit still permits them. A product profile must also demonstrate DET.59's bounded reserved status capacity.
  Fixture `emit` is a fused size-known encoding/publication row; its operand descriptor supplies the canonical
  length. The model computes that descriptor in Python, while a product obtains it through earlier metered work.

- **CV.6 [D]** Every p4 tag from `closure/findings.json` must occur in a fixture's `covers`, as must p7-lang's S2-02,
  S2-03, S2-04 and X-H6 tags from `closure/reconciliation-findings.json`, and the LANG-01, MC-05, S4-01 and S4-02
  controls listed by the checker. The checker validates
  coverage and compares complete computed results with stored expectations, including every charge, value, child
  ledger and stop. Expectations have no refresh path. Additional traces test the final false loop test, root/child
  precedence, scratch reservation, invalid Unicode, partial output and both display branches. Generator vectors
  fix seed bytes, stream paths, initialization digests and draws independently of the meter. Zero and exact limits
  are intentional semantic boundaries, not recommended budgets.

- **CV.7 [D]** These examples cannot establish RT1 to RT4 equivalence, native kernel bounds, generator quality,
  sandbox containment, floating predicate agreement or display-byte-size agreement. Those need matching executor
  evidence under RTE.71, DET.49 and DET.64. The frozen LX-3 analysis profile is a different experiment input; DET.65
  lists the differences. Chapter 05 supplies the actual output profile/terminal carrier; LG2.21 specifies form-2
  lowering, LG2.20 owns H6's isolation/reproduction rule, and EV1.10 consumes H7. Executor implementations and their
  checked mappings remain owed. Integration reviews the proposals, runs the combined closure check and obtains the required
  independent candidate review. No owner answer or freeze follows from this script's exit status.

- **CV.8 [D]** Form-2 fixtures explicitly name `lp1-exec-region-draft-1` from LG2.21, distinct from the default
  `lp1-exec-draft-1`. Fixture `display_region` expands to charged `display_enter`, its listed body and charged
  `display_exit`, with a logical scope but no implicit function frame. `display_branch` charges one step and runs
  only the selected body. These are already admitted abstract traces; the model does not establish type admission.
  The synthetic predicate is one fixed step. Each branch allocates a four-byte display scalar; the true branch adds
  one fixed step. Exit moves the scalar's owner without a second payload charge. The source text is a locator
  fixture, not proposed syntax. Calls and variable work need their ordinary operations; they are not hidden in a `tick`.

  `form2-equal-limit` completes entry, predicate, branch, two taken continuations and exit at six steps.
  `form2-one-short` has five steps and denies `region/display_exit`, with no charge for the exit and source range
  [0,48) at digest `2fff1652258c8381dd15ce9d9ce9cfd5008c3c3a37d2cd38ff0da007c74dc430`.
  Both use four bytes of logical memory, with four live bytes before terminal cleanup and zero after. The false
  branch uses five steps out of six. Completion of these region-only traces publishes nothing.
  The publication fixtures append CV.5's size-known `emit`: display scalar `1.25` costs four retained-JSON bytes.
  At limits (steps=7, kernel_cost=4, memory=8, output=4), used counters are (7,4,8,4). At output=3, the publication
  step is charged but all secondary charges are denied: used counters are (7,0,4,0), code is `KN7:budget.output`,
  and the stop is `publish`, range [48,55) in the same source. No output unit is emitted. LI.28 owns the corresponding
  `cancelled(budget)` receipt; LI.23/LI.25 own supersession. This meter supplies no second receipt mapping.

- **CV.9 [D]** `boundary_vectors` supplies semantic input descriptors separately from accounting traces.
  The exactness examples declare the value type, display-control dependence and qualifier erasure at an exact-use boundary; the
  reference model applies LG2.22's precedence and EV1.4's unchanged forwarding. It does not infer those descriptors
  from source. Direct and helper approximate conditions refuse with `LG2:type.exactness_violation`; a direct
  approximate scalar at exact addition uses `LG2:type.exact_required`. A display-dependent scalar still uses the
  former code. Host-counter reads, reclaimed remainders and budget-status branches by exact continuations are
  refused under LG2.20. Recorded deterministic counters remain the accounting traces' explicit expected fields.
  Pick examples compare declared RPI.24 bindings before source digests, consuming EV3.5/RM.10. The same instance at
  generation 7 after generation 8 refuses with `EV3:stale.display_pick`, including when source also changed.
  With current bindings, targeting different source bytes uses `EV3:stale.source_link`. Opening the producing
  bytes still succeeds when only the editable draft changed. These same RM.30 cases cover the registry rows under
  CT-ERR-12, including both unchanged-source and changed-source stale picks. These checks supply no actual pick or
  editor evidence.

- **CV.10 [D]** Encoding diagnostics use ERR.10's raw context, separately from H7/EV1.10 source ranges.
  `source-encoding-invalid-byte` rejects `61 FF 62` with byte interval [1,2); `source-encoding-truncated-scalar`
  rejects `61 F0 9F 98` with [1,4). Both retain `LG1:syntax.invalid_encoding` and diagnostic form, hash the exact raw
  bytes, omit `span` and produce neither replacement text nor a converted host range. Separate range vectors
  reject even [0,0) on those malformed documents; a valid scalar range remains accepted. The model uses strict
  UTF-8 decoding, not a language parser or an editor implementation.

- **CV.11 [D]** `child-decode-error-releases-reservation` has a 20-byte memory allowance, a 16-byte child frame and
  a four-byte reservation for decoding `FF`. The child ends with `LG4:input.text_scalar_invalid`, peak memory 20
  and `live_after_return = 0`; the parent's following 20-byte allocation succeeds. The child's admitted step and
  kernel charges remain. The model tracks unowned reservations in every active meter and releases them on unwind
  under DET.58; root cleanup uses that ledger too, without overwriting the live count.

- **CV.12 [D]** The two `full-straddlers-fixed-ceiling` vectors consume RM.37's full certified input descriptors:
  pieces 7 and 9, both point pairs, and a cap-map witness for piece 7 alone. They declare that producer budgets fit
  and that a fixed renderer ceiling is exceeded, without choosing a ceiling number or computing geometry. RM.12/RM.20
  give standalone refusal with A still pending, or failure on the original accepted scheduled B with A still
  superseded. Both use `EV3:resource_limit.display_plan_ceiling`, retain the full set without one-witness truncation,
  perform no new admission and install nothing. These are lifecycle rule examples, not renderer or production-limit
  evidence.
