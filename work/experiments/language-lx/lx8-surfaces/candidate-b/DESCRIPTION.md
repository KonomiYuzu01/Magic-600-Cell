# candidate-b

Experiment only, extension `.lxb8`. Decision: **open**; decider: **owner**.
No language name, syntax recommendation or product proposal is made.

This candidate uses line endings and fixed indentation to delimit function
bodies and verdict arms. The edition header is exactly `edition: "lx8-0"` on
its own line. Shared token, literal and byte rules are in `../ir.md`; the grammar
was constructed for this repository experiment.

Top-level declarations start in column 1. Body statements start with four ASCII
spaces, and verdict arms with eight. Tabs are refused, including elsewhere
outside strings/comments. Blank/comment-only lines may separate statements;
they are not body terminators. The next top-level declaration ends a body.
Every statement, including the final one, requires LF. There is no implicit
multiline continuation inside parentheses or type annotations.

```text
file        = edition module { import | declaration }
edition     = 'edition' ':' string LF
module      = 'module' name 'requires' names LF
import      = 'use' name 'from' string LF
declaration = ('function' | 'view') signature ':' LF body
            | 'command' signature command_metadata ':' LF body
signature   = name params 'returns' type 'using' names 'charge' charges
params      = '(' [ name ':' type { ',' name ':' type } ] ')'
body        = { INDENT4 name ':=' expr LF } INDENT4 'yield' expr LF
expr        = atom | 'choose' atom ':' LF cases
cases       = INDENT8 arm { LF INDENT8 arm }
arm         = name '(' name ')' '=>' expr
names       = '[' [ name { ',' name } ] ']'
charges     = '[' natural ',' natural ',' natural ',' natural ']'
command_metadata = 'row' string 'kind' name 'contexts' names 'permits' names
```

`type` and `atom` use the shared grammar. `choose` is supported as the whole
value of a binding or yield; its arms in this corpus are single-line expressions.
It does not introduce another indentation level for nested matches. Lowering
requires exactly one arm for each verdict alternative. No trailing commas are
accepted. All bindings precede the final yield. Imports name frozen interface
digests rather than paths.

For example, the frozen verdict fixture has this body:

```text
function status(verdict: Verdict<Certificate<Full,Labels>>) returns Text using [] charge [12, 0, 0, 0]:
    yield choose verdict:
        yes(proof) => "supported"
        no(counterexample) => "refuted"
        unknown(reason) => reason
```

Lowering maps `:=` lines to ordered immutable bindings and `yield` to the result
expression. `choose` maps to the same `match.verdict` as the other candidates.
Newlines and indentation add no operations. Calls, source associations, declared
charges and inferred effect requirements use the single lowerer in `../ir.md`.

Recovery scans for `function`, `command`, `view` or `use` in column 1 after a
damaged declaration, retaining both a located hole and the later declarations.
It never supplies a value for the missing or malformed body. Edition/byte errors
stop before declarations; malformed module context does not get guessed.

Known awkward spots, as structural facts: shifting a line's indentation can
change whether it belongs to a declaration; long signatures cannot wrap; the
fixed eight-space match form does not express arbitrary nested match layout;
an absent final LF is an error. Comments are accepted but not retained. There
is no formatter or human observation.
