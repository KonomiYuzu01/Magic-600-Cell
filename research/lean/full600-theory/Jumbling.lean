import Jumbling.Model.Pieces
import Jumbling.Model.Steps
import Jumbling.Model.ClassCell
import Jumbling.Model.CleanPowers
import Jumbling.Model.Transport
import Jumbling.Group.Words
import Jumbling.Group.Schreier
import Jumbling.Group.Cluster
import Jumbling.Group.Certified
import Jumbling.Group.PowerSeed
import Jumbling.Group.Anchors
import Jumbling.Group.ShortSeeds

/-!
# Jumbling restoration, round 22: first Lean batches

Formalizations of results of `research/jumbling/restoration/12-low-height/ARCHON-HANDOFF.md`
(statements in `PROOFS-B.md` of the same folder). The library lives in the Full600 Lake project so
that it shares the pinned toolchain and Mathlib and can use the permutation lemmas of
`ArchonFull600Theory.Basic`.

* `Jumbling.Model.*`: the model layer (scope A).
* `Jumbling.Group.*`: the group-theory layer (scope B).
-/
