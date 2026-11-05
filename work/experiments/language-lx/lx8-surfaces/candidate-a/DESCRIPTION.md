# candidate-a

Experiment only, extension `.lxa8`. Decision: **open**; decider: **owner**.
No language name, syntax recommendation or product proposal is made.

This candidate uses declaration keywords, braces, local `let` statements and
explicit `return`. Newlines and spaces separate tokens but do not delimit
statements. Top-level declaration starters must be in column 1 so a broken
draft can recover. Shared tokens, literals and source coordinates are specified
in `../ir.md`; the grammar was constructed for this repository experiment.

The edition header is exactly `edition "lx8-0";`. It selects this experiment
edition only. Unsupported editions are refused before declarations are parsed.

```text
file        = edition module { import | declaration }
edition     = 'edition' string ';'
module      = 'module' name 'requires' names ';'
import      = 'import' name string ';'
declaration = ('fn' | 'view') signature block
            | 'command' signature command_metadata block
signature   = name params '->' type 'effects' names 'charge' charges
params      = '(' [ name ':' type { ',' name ':' type } ] ')'
block       = '{' { 'let' name '=' expr ';' } 'return' expr ';' '}'
expr        = atom | 'match' atom '{' arm ';' { arm ';' } '}'
arm         = name '(' name ')' '=>' expr
names       = '[' [ name { ',' name } ] ']'
charges     = '[' natural ',' natural ',' natural ',' natural ']'
command_metadata = 'row' string 'kind' name 'contexts' names 'permits' names
```

`type` and `atom` use the shared grammar; recursive call arguments are `expr`.
Exactly the three verdict arms are required by lowering. No trailing commas are
accepted. This subset requires all bindings before the single return. Imports
name a digest of the shared frozen interface, not a filesystem path.

For example, from the frozen effect-bearing function fixture:

```text
edition "lx8-0";
module protection_helpers requires [snapshot];
fn probe(snapshot: Snapshot<Full,Labels,Witnessed>, witness: Word<Full,Retained>) -> Verdict<Certificate<Full,Labels>> effects [snapshot] charge [12, 0, 0, 0] {
    return check_word(snapshot, witness);
}
```

Lowering preserves declaration and binding order. A call becomes a typed `call`
with ordered arguments; a literal has its distinct exact/approximate/word kind;
`match` becomes `match.verdict` with all payloads and arms. Definition effects,
charges and command metadata enter the shared IR unchanged after checking.
Braces, semicolons and `return` add no operations. See `../ir.md` for the single
lowerer and the exact equivalence exclusions.

Recovery recognizes `fn`, `command`, `view` and `import` in column 1. A missing
brace can discard the damaged declaration and still find the next function.
Headers and module declarations must parse before local recovery is enabled.

Known awkward spots, as structural facts: a return inside a match still needs
the enclosing return's semicolon; arm semicolons and block delimiters are
separate; long indexed signatures remain long lines in these fixtures; braces
do not remove the column-1 recovery constraint. Comments are accepted but not
retained in the parsed tree. There is no formatter or human observation.
