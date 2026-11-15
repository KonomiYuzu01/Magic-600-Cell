# candidate-c

Experiment only, extension `.lxc8`. Decision: **open**; decider: **owner**.
No language name, syntax recommendation or product proposal is made.

This candidate uses expression bodies, `bind ... in` expressions, and pipelines
that pass the left operand as the first argument of the next call. Spaces and
newlines separate tokens without delimiting a body. Top-level starters remain
in column 1 for draft recovery. Shared tokens and values are in `../ir.md`; the
grammar was constructed for this repository experiment.

The edition header is exactly `@edition("lx8-0");`. Edition and module directives
are distinct from function bodies. No dependency can change this header.

```text
file        = edition module { import | declaration }
edition     = '@' 'edition' '(' string ')' ';'
module      = '@' 'module' '(' name ',' names ')' ';'
import      = '@' 'use' '(' name ',' string ')' ';'
declaration = ('fn' | 'view') signature '=' body ';'
            | 'cmd' signature metadata '=' body ';'
signature   = name params ':' type '!' names '@' charges
params      = '(' [ name ':' type { ',' name ':' type } ] ')'
body        = { 'bind' name '<-' expr 'in' } expr
expr        = primary { '|>' name '(' [ expr { ',' expr } ] ')' }
primary     = atom | 'case' atom 'of' arm { '|' arm } 'end'
arm         = name '(' name ')' '=>' expr
names       = '[' [ name { ',' name } ] ']'
charges     = '[' natural ',' natural ',' natural ',' natural ']'
metadata    = 'with' '(' string ',' name ',' names ',' names ')'
```

The metadata positions are command row, kind, contexts, permits. The `!` list
declares effects; the `@` vector declares steps, kernel, memory and output
charges. `type` and `atom` use the shared grammar. No trailing commas are
accepted. Imports identify the digest of the frozen interface, not a path.

`x |> f(y) |> g(z)` lowers to `g(f(x,y),z)`: operands are still visited in the
order x, y, z. A pipeline never reorders arguments or turns a key into a witness.
The corpus includes both pipelines and nested calls, including zero-argument
calls. The `bind` prefix maps to the same ordered local bindings as candidate-a's
`let` and candidate-b's `:=`. `case` maps to `match.verdict`; the checker requires
yes, no and unknown arms, with payloads and common result types. Punctuation and
pipeline links add no operations or charges of their own.

For example, the frozen module-client body calls a pinned imported signature:

```text
fn length(witness: Word<Full,Retained>) : Cost<Full,Generators> ! [] @ [12, 0, 0, 0] =
    (witness |> helpers_length());
```

Recovery scans for column-1 `fn`, `cmd`, `view` or `@` after a local failure.
The next independent declaration remains available for navigation, with the
damaged declaration represented as a non-executable hole. No missing `in`,
parenthesis or `end` is inferred as an executable expression.

Known awkward spots, as structural facts: pipe insertion is limited to the
first argument; placing a result in another argument requires a nested call;
parentheses delimit a pipeline used inside another call; `|` separates verdict
arms while `|>` feeds a call; charge and directive punctuation both use `@` in
different positions. Comments are accepted but not retained. There is no
formatter or human observation.
