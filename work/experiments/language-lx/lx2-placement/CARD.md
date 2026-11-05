# LX-2 headless placement card (E-2.4-L2)

This is experiment code, not a product kernel, runtime, language or session format.
LQ8, LQ9, LQ21 and every route and placement decision stay **open / owner**.
Time box: **what can be done now; one wrapper call of at most two hours**.
The packet prioritizes conditions 1 and 2 over timing. A reached limit is no pass.

## Sources read and scope

Owner allowance: `docs/wiki/decisions/owner-decisions-2026-11-05.md`, decision 2,
and the LX-2 headless implementation packet. Base: `ab046f4f8fecf201f0d1cb348b45b29adaead7fb`.
The following documents were read on 5 November 2026. They are references, not
digest-pinned build inputs; later document edits do not invalidate this check.

- `AGENTS.md` and `docs/development-guide/AGENT_BRIEFING.md`.
- `docs/architecture/1.0/language/architecture/11-route-and-spikes.md`:
  RTE.21, RTE.22, RTE.33, RTE.35, RTE.37, RTE.41 to RTE.43 and CT-RTE-05.
- The same architecture directory's `01-kernel.md` KN7.12/KN7.13,
  `08-one-state.md` ONE.70, and `09-determinism.md` DET.43.
- `docs/progress/1.0/design-table/topics/language-ecosystem.md`, DT-L1 R2-E3,
  and `docs/architecture/1.0/language/architecture/07-boundary.md` BND.22 to BND.25.
- `work/experiments/language-lx/README.md`, including its later-commit caveat.
- `work/experiments/language-lx/lx6-engine-port/RESULT.md`, `engine.rs`, `json.rs`,
  `sha256.rs`, `export_model.py`, `run_oracle.py`, `main.rs`, `measure.py`,
  `INPUTS.json` and `CANDIDATE.json`.

Question: on this frozen scripted Rust host, do an engine-process thread and a
separate runtime process preserve a real session through metered limits and
faults outside the meter? Shared failure and memory domains are the hypothesis;
there is no assumed winner. The shell placement is outside this packet.

## Kernel and runtime boundary

The Rust kernel reuses LX-6 through `#[path]`, without copying or changing its
mechanics, JSON or SHA-256 code. It retains all 259,800 labels and 1,200 legal
generators of `600-cell-Full`. `prepare.py` reuses LX-6's read-only NPY reader to
export into this folder's `build/`; it does not invoke the old exporter's chapter
pins or output path. The generated model must match LX-6's recorded model digest.
Python and Rust standard libraries suffice; no NumPy, crate, registry or install.

One fresh session owns labels, revision, immutable read snapshots, one pending
preview, undo history and an append-only journal. A journal row includes the
literal word, previous/resulting full-state hashes and the entire provenance
block. A commit or undo writes, flushes and calls `File::sync_all` before publishing
the state and replying with its receipt. No restart is counted as session survival.
Python separately replays the actual journal using the exported move arrays and
checks every row's chain, provenance and state hash. The initial protected set is
empty; this experiment adds no protection policy or model identity.

One `KernelPort` trait admits `read_labels`, `read_pose_bundle`, `check_word` and
`preview`. Direct calls run on the runtime's own thread; the second implementation
uses two anonymous pipes with a little-endian u32 length and byte payload. Replies
have an additional one-byte success/error tag. The separate runtime receives no
model, journal or session path. The Python driver uses a separate control pipe for
the solver's commit and undo; neither is a runtime capability. A runtime attempt
to commit is exercised and refused. The control pipe is a trusted fixture, not a
production shell token or authentication design.

Runtime reads retain an immutable snapshot admitted before the job starts. They
take no session lock while copying or transferring data. Preview computation is
outside the session lock; installation and commit recheck the base revision and
full-state hash. A second pending preview is refused with no journal effect.
No branch, import/reset, crash-recovery protocol, program checker or renderer is
implemented here. The native runtime is trusted experiment code, not a security
sandbox. Product `EngineProcess` is not used to launch this independent Rust fixture.

`PROVENANCE.json` is an explicitly synthetic, structured sizing assumption:
2,836 canonical JSON bytes, including version/form/checker/policy pins, author,
program/module/entry, profile, dependencies, 32 component provenance digests and
a pick record. These are fixture references, not verified real modules or a
measured distribution of deployed block sizes. The full preview request is 2,887
bytes. The block uses one flow-label carrier to exercise DT-L1 R2-E3's transfer
obligation; it selects no LQ21 form and proves no flow-policy enforcement.

## Fixed commands and fault levels

`SCENARIO.json` freezes the words and command order. Each trial starts solved,
previews and durably commits `[1,2,-3]`, then the runtime reads labels and poses,
checks `[7,-11,1200]` and leaves that word as the engine's pending preview. A barrier
holds the runtime while the driver verifies the prior receipt and preview. The
driver flushes the release acknowledgement before starting the fault. The control
runs the identical prefix and then completes without a fault.

After observing completion or death, the same requested tail is: commit the
runtime preview, undo it, preview `[600,-1200,23,1]`, commit that preview, inspect
the still-open session. If the engine died, the unavailable tail is explicitly
recorded as unexecuted, never silently retried. No engine restart hides a failure.
The complete control has four journal rows and a non-solved final state.

| Level | Fault | Actual injection |
| --- | --- | --- |
| Runtime meter | Allocation | Allocate/touch 64 KiB chunks up to 1 MiB; refuse the next charge with `budget.memory_bytes` |
| Runtime meter | Recursion | Recurse through 64 logical levels; refuse level 65 with `budget.recursion` |
| Runtime meter | No-kernel-call loop | Charge each iteration of an endless loop; after 100,000 instructions refuse the next with `budget.instructions` |
| Outside the meter | Allocation | A thread-local allocator ceiling admits 8 MiB of native payload allocations, then returns null for a real 1 MiB allocation; Rust's allocation-failure path aborts the host |
| Outside the meter | Recursion | Unmetered recursion with live 4 KiB frames overflows the runtime thread's 128 KiB stack |
| Outside the meter | No-kernel-call loop | Native endless loop ignores the logical meter; the host observes it with a 1,000 ms watchdog |
| Outside the meter | Forced kill | OS termination of the runtime's actual process: `Child::kill` for a separate host, `Popen.kill` for the shared host |

Forced kill has no metered substitute. Ordinary cancellation is not offered as
evidence that forced kill survived. Both placements use the same runtime thread
stack and scripts. The native allocation ceiling is an injection device, not a
proposed logical-memory rule or an OS-wide low-memory test. It bounds machine
pressure without containing Rust's process abort.

For the native loop, the separate host is killed and reaped by the kernel, which
returns a typed incomplete runtime error. Rust std has no thread-kill operation:
the direct host reports that the thread is still running, with no typed terminal
outcome. Its session is inspected and its command tail attempted while that loop
continues; process exit afterwards is fixture cleanup, not fault recovery.

Condition 1 compares complete journal bytes and the independently replayed full
state hash with the placement's control. Live state is also compared whenever the
engine survives. Condition 2 requires a terminated job, a typed outcome and the
same open session. Pending-preview and earlier-receipt survival are additional
recorded checks from RTE.22. On engine death, offline journal replay establishes
only durable content, never a live session. Kill criteria: any failed condition,
especially inability to survive forced kill, fails that placement's tested slice;
continue the remaining cases to preserve every counterexample and its control.
Outcome codes are stable experiment tags, not a claim of language error-registry
conformance. Scripts without a language front end exercise the placement boundary only.

## Transfer and timing protocol

Run all fault trials first. Then, for each placement, one smoke run calls every
runtime row once, followed by three series of 100 calls per row. Each call is timed
inside the runtime around `KernelPort::call`: direct kernel work and a full copied
reply, or JSON request encoding, both framed pipe transfers, kernel work and reply
decoding. Request construction, reply validation/hashing, process startup and
benchmark-only pending-preview reset are outside the timer. No process is started
per sample. No shared-memory or zero-copy optimization is claimed.

| Row | Request JSON bytes | Reply payload bytes | Contents |
| --- | ---: | ---: | --- |
| `read_labels` | 21 | 1,039,200 | All 259,800 little-endian int32 labels of the immutable full-model snapshot |
| `read_pose_bundle` | 26 | 869,952 | Synthetic pose transfer fixture described below |
| `check_word` | 40 | 126 | Three-letter word, complete net digest and moved-slot count |
| `preview` | 2,887 | 347 | Word plus full provenance; real mechanical preview and acknowledgement |

The synthetic pose fixture has 708,480 bytes of pose IDs, 131,136 bytes of 2,049
4x4 f32 display matrices, 8,196 bytes of lattice indices and one 22,140-byte mask.
It is not a jumbling pose calculation or a picture. All bytes are transferred and
checked against the generated fixture digest. The label and pose lengths exclude
the four-byte framing and one-byte reply tag. Preview measurements include full
state hashing and computation, so this table is not an isolated IPC overhead test.

`TIMINGS.json` retains all 2,408 raw nanosecond samples and each series' median,
nearest-rank p95, nearest-rank p99 and largest. `RESULT.md` reports the 24 full-series
rows in milliseconds. Repeated response bytes, lengths and digests are checked;
timing values are never compared on acceptance. Commit and undo remain real,
durable control commands in the fault trials, not program-call timing rows. No
LX-1 call mix is available: the mix and any weighted conclusion remain **open**.

All timing here is a **sandbox lead**. Other runs share the machine. D6 power source,
Windows power mode, vendor mode, V-Sync and overlay conditions were not established.
There is no renderer, foreground window, native-input observation, E2, E3, long-session
or performance-gate result. The card's Cannot show row remains: containment for
untested hosts or faults, a general process-isolation proof, or an accepted E3 budget.

## Reproduction and evidence

`CANDIDATE.json` freezes every source/input digest, the generated model and pose
digests, compiler flags, tool versions and executable digest. `MANIFEST.json` pins
only this folder and reused files under `work/experiments/`; it pins no chapter,
card or decision page. `RESULTS.json` contains every correctness field, journal
row and full-journal digests, command completions, fault observations and controls.
Changing an interior journal hash and truncating a journal are negative controls
for the independent verifier. Raw journals, model, pose fixture and executables
are regenerated in `build/`, never left in the patch.

Acceptance, from the repository root:

```text
python work/experiments/language-lx/lx2-placement/check.py
```

It rebuilds and reruns every trial and measurement, compares all correctness
fields exactly with stored results, excludes timing samples/statistics from the
comparison, removes `build/` even on failure, and verifies the complete folder's
byte snapshot. Exit 0 means reproduction, including failed placements. It writes
no updated report, stored timing or refreshed pin. The packet stops after two
consecutive passes. Integration review belongs to the integrator; this no-network,
folder-only packet starts no nested review wrapper.

Canonical correctness excludes timing samples/statistics, process/thread IDs,
absolute temporary paths and raw stderr text. The native failure signatures are
retained as `allocator_exhaustion` or `stack_overflow`; a driver-requested OS kill
and the two different watchdog observations have separate tags. No correctness
flag, outcome, journal digest, state hash or transfer digest is excluded. Build
and tool metadata are retained and compared separately in `CANDIDATE.json`.

To capture another platform or machine run, use a separate checkout/copy and a new
output filename with `run.py --output NAME.json`. It preserves that new combined
source/build, correctness and timing record, then removes `build/`. It refuses to
overwrite an existing output. A later native harness is required for the excluded
conditions; this headless command cannot acquire those results.
