# LX-5 dependency and licence inventory

Decision: **open**. Decider: **owner**. This is an experiment inventory, not permission to adopt or redistribute a dependency.
No package, crate, source download or installation was used. No runtime or third-party binary is included in the deliverable.

| Component actually used | Terms and evidence | Outstanding obligation |
| --- | --- | --- |
| Node 24.21.0, built-in BigInt, fs and crypto | Node core is generally MIT; V8 and other bundled components have their own notices. This is **unverified recollection for this exact distribution**: the inspected installation root has no standalone Node core licence. npm/node-gyp licences do not establish the Node binary's terms. | Obtain and verify the matching distribution's full licence/third-party notice bundle before any dependency decision or redistribution. |
| .NET SDK 10.0.401 / runtime 10.0.12; System.Numerics.BigInteger | The installed root ships **Microsoft Software License Terms: Microsoft .NET Library** in LICENSE.txt and a separate ThirdPartyNotices.txt. Both were inspected locally and their exact byte digests are in LICENCE-EVIDENCE.json. Upstream .NET runtime sources are generally MIT, but that recollection is not a verification of the installed binary's complete terms. | Preserve applicable distribution terms and component notices; verify the selected runtime package/source revision before adoption. No blanket MIT label is inferred from BigInteger's namespace. |
| Rust 1.98.1 standard library and compiler | The shipped COPYRIGHT-library.html explicitly states **Apache-2.0 OR MIT**, subject to the exceptions and dependency notices listed there. COPYRIGHT.html covers the compiler and its dependencies. Both document digests are captured. | Preserve the selected licence and the applicable exception/dependency notices, including the target runtime/linker dependencies; this spike is no complete redistribution audit. |
| Own Rust limb and SHA-256 code | Written for this experiment, no third-party crate. There is no imported bigint implementation. Repository licensing applies to the new source; this document grants no new licence. | The arithmetic and hash code have an **audit and maintenance burden**. Passing this finite corpus does not make either a vetted general-purpose library. |
| Python 3.15.1 standard library reference/harness | Python is generally under PSF terms with bundled component notices; this is background knowledge, not a new distribution audit. Integer arithmetic, fractions, hashlib, gzip and other standard modules only; no NumPy. | Retain the notices of any Python distribution later selected. |
| Roslyn and the installed Windows native linker | Build tools already on the machine. Roslyn is invoked directly through the SDK; rustc uses the installed MSVC linker. No NuGet restore, cargo registry or downloaded build tool. | Tool/distribution rights are separate from the experiment's source licence. No tool binaries are redistributed. |

The following crates were **not fetched, built, inspected or timed**. All licence entries below are **unverified recollection**, with no pinned release or dependency graph. They remain an **open owner question**, not a comparison result.

| Unfetched source | Unverified licence recollection | Owner question |
| --- | --- | --- |
| num-bigint | MIT OR Apache-2.0 | Allow a specific source/version and verify its licences and transitive dependencies? |
| dashu | MIT OR Apache-2.0 | Allow a specific source/version and verify its licences and transitive dependencies? |
| ibig | MIT OR Apache-2.0 | Allow a specific source/version and verify its licences and transitive dependencies? |
| malachite | LGPL-3.0 family; exact expression/version unverified | Copyleft dependency needs the owner's explicit decision, then source and obligation verification. |
| rug | LGPL-3.0 family; exact expression/version unverified; GMP/MPFR/MPC dependencies carry additional terms | Copyleft dependency needs the owner's explicit decision, then native dependency, linking and redistribution verification. |

`LICENCE-EVIDENCE.json` records only public document labels, byte lengths, SHA-256 digests and tool versions, not private installation paths. Regenerate it on the matching installed distributions with:

```text
python work/experiments/language-lx/lx5-arithmetic/licences.py
```

The evidence record is an identity anchor for the documents inspected here, not a substitute for those complete notices. Nothing in this inventory resolves LQ8 or authorizes a copyleft dependency.
