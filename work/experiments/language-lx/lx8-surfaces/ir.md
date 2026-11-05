# LX-8 experiment IR `lx8-ir-0`

Experiment E-2.4-L8 only. Decision: **open**; decider: **owner**. This document,
the catalogue and the JSON fixtures select no LG4 instruction set, tariff,
syntax, engine, route, model identity or LQ answer. There is no interpreter.

## Boundary and representation

`syntax.py` owns byte decoding, locations, tokens and shared value syntax.
Each candidate's `parser.py` owns its declaration, body and match grammar.
They return located parsed nodes; `lower.py` alone produces typed operations
using `semantics.py`. No parser reads a fixture filename or expected output.
The public entry is `parse(source_bytes)`; `lower(parsed, interfaces)` refuses
any diagnostics or recovery holes. Neither entry executes a body.

The stored JSON form is illustrated by every file in `fixtures/expected/`:

| Field | Meaning |
| --- | --- |
| `ir_version`, `edition` | Separate experiment IR and source-edition identities |
| `module`, `requires` | Module name and sorted required effect/capability names; requirements grant nothing |
| `imports` | Ordered alias and SHA-256 pairs for frozen, local interface bytes |
| `declarations` | Source-order function, command and view definitions |
| Declaration `params`, `result` | Ordered names and canonical indexed types, and the result type |
| `declared_effects`, `effects` | Checked function effect bound and computed body/callee summary |
| `charges` | Exact nonnegative decimal strings for steps, kernel, memory and output |
| `metadata` | Command row, kind, contexts and permitted primitive rows, otherwise null |
| `bindings`, `body` | Ordered immutable local bindings, then one result expression |
| Operation `op`, `type`, `effects`, `charges` | Typed expression and its effect/charge declaration |
| Operation payload | `value`, ordered `letters`, `name`, ordered call `args`, or verdict `subject`/ordered `arms` |
| `expanded_length` | Exact decimal length for literal and statically composed word constructors |
| `source` | SHA-256 of the exact input bytes and, on located nodes, a source span |

An operation tree evaluates its child operands in array order if an executor is
ever supplied. The experiment only preserves that order. Bindings are in source
order, with no reordering, cancellation of inverse letters, or dead-code removal.
Commands and views are definitions, never invocation or registration actions.

Canonical type strings have grammar `Name [ '<' Type { ',' Type } '>' ]` with
no whitespace. They are parsed and checked, not arbitrary annotations.
`semantics.py:TYPE_SLOTS` checks each constructor's arity and the kind of every
index. Whole types must match at arguments and results. Model, level, alphabet,
mode, reachability claim, precision, dimension and cost unit survive lowering.
There is no implicit projection, numeric conversion or word-key conversion.

`Full` denotes the existing 600-cell-Full concept; no assets are read or changed.
`Jumble` and `MenuA` are opaque synthetic fixture indices, not new or selected
puzzle/menu identities. `spin`, `unspin` and `identity` are symbolic fixture
letters with no assigned rotations. No admissibility or group relation is
asserted by their spelling. Parameters of state types are hypothetical host
inputs with explicit reachability indices, not created or validated states.

## Shared lexical and value rules

Source bytes must be strict UTF-8, LF, without BOM. The parser refuses malformed
bytes and CR rather than normalizing and hashing different bytes. Identifiers
are ASCII `[A-Za-z_][A-Za-z_0-9]*`; case matters. A `#` comment ends at LF outside
a string. Strings use JSON string escapes and may contain Unicode, but not raw
newlines, invalid escapes or unpaired surrogates. Keywords are reserved where
the grammar uses them; `q5`, `word` and `menu` are reserved as expression atoms.
Whitespace is not an operator. The candidate descriptions specify line rules.

The shared atom grammar is:

```text
atom      = signed_integer | rational | string | approximation
          | 'q5' '(' rational ',' rational ')'
          | 'word' '[' [ signed_integer { ',' signed_integer } ] ']'
          | 'menu' '[' [ menu_letter { ',' menu_letter } ] ']'
          | identifier [ '(' [ expr { ',' expr } ] ')' ] | '(' expr ')'
rational  = signed_integer '/' signed_integer
approximation = '~' unsigned_precision string
menu_letter   = nonnegative_integer ':' ('spin' | 'unspin' | 'identity')
```

Rational numerator, slash and denominator are one token, with no intervening
spaces. Signs and leading zeros are accepted. Exact integers are normalized
decimal strings; rational pairs have gcd one and a positive denominator; zero
denominators are refused. `q5(a/b,c/d)` means `(a/b) + (c/d) sqrt(5)` and retains
both normalized rational pairs. Decimal conversion uses integer chunks to avoid
CPython's ordinary decimal-string digit ceiling. No host float is involved.

`~24"0.125"`, `~53"0.125"` and `~113"0.125"` have distinct approximate types
and carry both decimal text and declared precision. Decimal text has an optional
sign, integer digits, optional fractional digits and optional signed exponent.
Nonfinite values and unmarked decimal fractions are refused. No rounding is
implemented or claimed. `approx53(exact_q5)` is an explicit conversion operation
in the IR only; there is no inverse conversion or approximate decision operation.

Retained letters are signed integers in 1..1200. Menu grips in this fixture
vocabulary are 0..119. Literal order, adjacent inverses and identity letters
remain present. `word_key` returns a distinct key type and computes no key here.
`repeat` accepts a known-length word and a nonnegative exact integer literal;
`join` accepts matching, known-length compressed words. Only length arithmetic
is performed. Even the `3 * 10^80 + 4`-letter fixture is never expanded or replayed.

## Effects, declarations and charges

The catalogue has `snapshot`, `view`, `workspace`, `preview`, `file`, `random`,
`budget`, and the separately named clock requirement. Pure is the empty set.
Calls include argument and callee effects. An annotation may bound effects,
never erase them. Module requirements must cover declaration requirements.
Local functions are predeclared, so forward calls retain their summaries.
Imported signatures are accepted only from the supplied digest-keyed local
interface map; the checker verifies the interface file's bytes first.

Views reject workspace and preview effects, plus the file, random and clock
requirements outside this fixture's view subset. Commands name a dotted row,
kind, contexts and permitted primitive rows. A command's kind must admit its
effects and its permits must cover local transitive primitive requests. There is
no commit primitive, commit effect or commit command. `UserActionToken` is the
host-only exception to LG2.12's coverage: it is documented here and explicitly
rejected as a language type, rather than manufactured by a fixture.

All numerical charges are **invented fixture declarations**. Every ordinary
literal/control operation declares one step, a reference zero, selected kernel
stubs seven kernel units and output stubs one output unit. Calls preserve the
callee's declared vector; each source definition explicitly declares its own
vector. These are neither measurements, budget grants nor computed totals.
No metering, cache policy, allocation charge or stopping point is implemented.
LG4.8's granularity and input-dependent production tariffs remain open. Comparing
these fields tests their preservation, not their adequacy for a real executor.

Verdict matching requires exactly one `yes(evidence)`, `no(evidence)` and
`unknown(reason)` arm. Evidence payload types remain indexed; the unknown reason
is text. Arm result types must match, all possible effects are retained, and
arm order is preserved. There is no verdict-to-boolean conversion.

## Source association and recovery

Spans are half-open UTF-8 byte offsets plus 1-based line and Unicode scalar
column at the start. The source digest names the original bytes. This is an
experiment coordinate convention, not a selection of EV1/EV2's open convention.
An invalid UTF-8 diagnostic locates the undecodable bytes without replacement.
`semantic()` removes only the `source` fields (including different source
digests). No type, effect, import-interface digest, operand, arm order, precision,
word letter, expanded length or charge is excluded from equivalence.

Recovery discards a damaged top-level declaration through the next declaration
starter in column 1. It keeps a located hole and emits a primary registry code
plus `LG1:syntax.recovery_gap`; later independent declarations remain navigable.
The whole draft remains unlowerable. Missing/unsupported editions and invalid
byte inputs stop before declaration parsing; there is no default-edition retry.
Malformed headers/modules also stop because there is no established context.

The six codes are exactly chapter 13's `LG1:syntax.edition_missing`,
`edition_unsupported`, `invalid_encoding`, `invalid_literal`, `unexpected_token`
and `recovery_gap`. Explanations are not code identities. The small type checker
raises an internal `LoweringError` for semantic refusals; this invents no product
registry codes. No module store exists, so there is no draft installation path.

## Corpus and scope of proof

The 17 programs in `fixtures/MANIFEST.json` have three frozen source spellings
and independent expected typed JSON. `fixtures/build.py` authors the expected
operations, types, effects, values and charges, then spells them with three
small printers; it imports no parser, lowerer or semantic catalogue. It is a
corpus builder, not a general formatter. Acceptance reads the frozen files and
does not rebuild or bless new expected results. All three parses must match the
authored oracle and one another. Semantic mutation controls check that changing
an exact coefficient, precision, literal order or charge is observable, and
that several invalid type/effect/capability edits are refused.

The eight LG2.12 families are covered at the type-surface level, with every
listed constructor represented in a parameter, result or operation, except the
explicitly forbidden user-action token. Operations exercise representative
members; they do not implement all family semantics. Identity parameters in
program 03, representation parameters in 13 and research parameters in 16 are
also signature coverage. Outcomes remain opaque; no admissibility is decided.
Snapshot/check/display stubs are signatures, not engine, renderer or GUI calls.
The state-contract examples keep pose/label levels separate; they prove no
projection, reachability, checkpoint or solving claim. No search runs.

There is no full language checker, generic inference, formatter, package loader,
kernel, interpreter, runtime grant system, persistence, execution trace, streaming
cursor, cancellation, display taint discipline or BM2 implementation. LQ21 and
other open mechanisms stay open. This finite corpus proves only the recorded
syntax-to-IR equivalence and recovery cases for these frozen bytes. It cannot
show general equivalence, human learnability or a syntax preference.
