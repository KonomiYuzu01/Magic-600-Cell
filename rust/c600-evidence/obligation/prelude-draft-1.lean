/-
  Magic 600 Cell: translation prelude for exported proof obligations, draft 1.

  Vocabulary: c600-obligation-vocabulary-draft-1 (PS1.46).
  Symbol table: symbols-draft-1.json beside this file. Each row of the table names one declaration below, with its
  arity; the sort `Piece` names the type `C600.Piece`.

  This file lives in the repository outside the shipped software (LB5.18, RTE.28). The translator in
  rust/c600-evidence/src/obligation/ holds only this file's SHA-256 as a pin and never compiles or reads it. Lean is
  neither shipped nor on the allowlist, and no project check runs this file (owner, 23 November 2026, LP5-2): it has
  not been elaborated by Lean.

  The declarations are opaque. They fix the names and the types the exported theorems use and state nothing about
  the puzzle. Draft 1 holds exactly the rows of the format packet's `obligations` fixture; a later table version
  brings its own prelude version and pin.

  An exported obligation is one theorem stated with no proof (`by sorry`). Read it after this prelude.
-/

namespace C600

/-- The sort `Piece`. -/
axiom Piece : Type

/-- The type of `orbit`. It is no sort of the table. -/
axiom Orbit : Type

/-- `orbit`, arity 0. -/
axiom orbit : Orbit

/-- `base`, arity 0. -/
axiom base : Piece

/-- `member`, arity 2. -/
axiom member : Orbit → Piece → Prop

/-- `grade`, arity 1. -/
axiom grade : Piece → Nat

end C600
