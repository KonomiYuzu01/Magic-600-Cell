import Jumbling.Group.ProductAlt

set_option linter.style.header false in
/-!
# R22B-25: a finite classifier of the induced action, with a residual case

`PROOFS-B.md` section 3, round 5 of line (b), R22B-25 (layer "group" of `ARCHON-HANDOFF.md`). The
mathematical claims are formalized; the Schreier–Sims chain, the enumeration and their time limits
are algorithms and are not.

Setting: the group `K` that the SAFE generators induce on a target orbit `Δ` of `d` points, a
subgroup of `Sym(Δ)`. The abstract statements take a finite type `X` (the orbit) and
`K : Subgroup (Perm X)`, or a group acting on `X`; `vacancy_iff_restrict` links them to part one's
alphabet, with `K = restrict ...` the induced group of `Jumbling.ProductAlt`.

* `alternatingGroup_le_iff`, `alternatingGroup_le_iff_div_two`: `K` contains `Alt(d)` exactly when
  `|K| ≥ d!/2`, for every `d` (the source states it for `d ≥ 8`); `eq_alternatingGroup_iff`: the
  subgroup of order `d!/2` is `Alt(d)` (Mathlib's `Equiv.Perm.eq_alternatingGroup_of_index_eq_two`).
* Blocks: `blockRel a b` is the least `K`-invariant equivalence relation with `a ~ b`
  (`blockRel_le`); its classes are blocks (`isBlock_blockClass`); for a transitive action, `K` is
  primitive exactly when every `blockRel a b`, `b ≠ a`, is the total relation
  (`isPreprimitive_iff_blockRel`), and otherwise the class of `a` is a nontrivial block
  (`not_isPreprimitive_of_blockRel`). For the group of an alphabet, closing under the letters
  gives the same relation (`blockRel_act`, `blockRel_le_of_letters`).
* `eq_one_of_smul_eq_of_card_eq`, `bijective_smul_of_card_eq`: a transitive group of order `d` is
  regular, with one element for each image of a point.
* `vacancy_iff_restrict`: vacancy is decided by the group induced on any set that contains `T` and
  that every letter maps into itself; with the union of the target orbits
  (`vacancy_iff_targetOrbits`), orbits that miss `T` impose nothing, and for a regular orbit the
  test runs over its `d` elements.
* `vacancy_iff_blocks`: the imprimitive case, the block inequality criterion, with independent
  `Alt(B_i)` factors (independent `Sym(B_i)` factors for blocks of at most two points);
  `block_inequality_of_vacancy`: without the factors the inequalities stay necessary.
* `ResidualCase`: `Sym(12)` on the 66 two-element subsets of a 12-set is primitive, has order
  479,001,600 and does not contain `Alt(66)`.

The independent factors of the imprimitive case are a hypothesis (in the source they come from a
computation or from R22B-19); no result of R22B-15 or R22B-22 is used.
-/

open Equiv Equiv.Perm Subgroup MulAction
open scoped Nat Pointwise

namespace Jumbling

namespace Classifier

/-! ### Containing `Alt(d)` is an order condition -/

section Order

variable {X : Type*} [Fintype X] [DecidableEq X]

/-- R22B-25: a subgroup `K` of `Sym(d)` contains `Alt(d)` exactly when `2·|K| ≥ d!`. -/
theorem alternatingGroup_le_iff (K : Subgroup (Perm X)) :
    alternatingGroup X ≤ K ↔ (Fintype.card X)! ≤ 2 * Nat.card K := by
  have hK : 0 < Nat.card K := Nat.card_pos
  have hmul := K.card_mul_index
  rw [Nat.card_perm, Nat.card_eq_fintype_card (α := X)] at hmul
  constructor
  · intro h
    by_cases hX : Nontrivial X
    · have h1 : 2 * Nat.card (alternatingGroup X) = (Fintype.card X)! := by
        rw [two_mul_nat_card_alternatingGroup, Nat.card_perm, Nat.card_eq_fintype_card]
      have h2 := Subgroup.card_le_of_le h
      omega
    · rw [not_nontrivial_iff_subsingleton] at hX
      have : Fintype.card X ≤ 1 := Fintype.card_le_one_iff_subsingleton.2 hX
      interval_cases h : Fintype.card X <;> simp
  · intro h
    apply alternatingGroup_le_of_index_le_two
    by_contra hlt
    push Not at hlt
    have : Nat.card K * 3 ≤ Nat.card K * K.index := Nat.mul_le_mul_left _ hlt
    omega

/-- R22B-25, in the source's words: `K` contains `Alt(d)` exactly when `|K| ≥ d!/2`. -/
theorem alternatingGroup_le_iff_div_two (K : Subgroup (Perm X)) :
    alternatingGroup X ≤ K ↔ (Fintype.card X)! / 2 ≤ Nat.card K := by
  rw [alternatingGroup_le_iff]
  by_cases h2 : 2 ≤ Fintype.card X
  · have heven : 2 ∣ (Fintype.card X)! := Nat.dvd_factorial (by norm_num) h2
    obtain ⟨m, hm⟩ := heven
    rw [hm, Nat.mul_div_cancel_left _ (by norm_num)]
    omega
  · have : Fintype.card X ≤ 1 := by omega
    have hK : 0 < Nat.card K := Nat.card_pos
    interval_cases h : Fintype.card X <;> simp <;> omega

/-- R22B-25: `Alt(d)` is the only subgroup of index two, that is, of order `d!/2`. -/
theorem eq_alternatingGroup_iff [Nontrivial X] (K : Subgroup (Perm X)) :
    K = alternatingGroup X ↔ 2 * Nat.card K = (Fintype.card X)! := by
  have hmul := K.card_mul_index
  rw [Nat.card_perm, Nat.card_eq_fintype_card (α := X)] at hmul
  constructor
  · rintro rfl
    rw [two_mul_nat_card_alternatingGroup, Nat.card_perm, Nat.card_eq_fintype_card]
  · intro h
    apply eq_alternatingGroup_of_index_eq_two
    have hK : 0 < Nat.card K := Nat.card_pos
    have : Nat.card K * K.index = Nat.card K * 2 := by rw [hmul]; omega
    exact Nat.eq_of_mul_eq_mul_left hK this

end Order

/-! ### Blocks -/

section Blocks

variable {G X : Type*} [Group G] [MulAction G X]

variable (G) in
/-- R22B-25: the least `G`-invariant equivalence relation in which `a ~ b`: the equivalence
generated by the pairs `(g • a, g • b)`. -/
def blockRel (a b : X) : X → X → Prop :=
  Relation.EqvGen fun x y => ∃ g : G, g • a = x ∧ g • b = y

/-- `blockRel a b` is an equivalence relation. -/
theorem blockRel_equivalence (a b : X) : Equivalence (blockRel G a b) :=
  Relation.EqvGen.is_equivalence _

/-- R22B-25: `a ~ b`. -/
theorem blockRel_self (a b : X) : blockRel G a b a b :=
  Relation.EqvGen.rel _ _ ⟨1, one_smul G a, one_smul G b⟩

/-- R22B-25: `blockRel` is `G`-invariant. -/
theorem blockRel_smul {a b x y : X} (h : blockRel G a b x y) (g : G) :
    blockRel G a b (g • x) (g • y) := by
  induction h with
  | rel x y hxy =>
    obtain ⟨k, rfl, rfl⟩ := hxy
    exact Relation.EqvGen.rel _ _ ⟨g * k, mul_smul g k a, mul_smul g k b⟩
  | refl x => exact Relation.EqvGen.refl _
  | symm x y _ ih => exact Relation.EqvGen.symm _ _ ih
  | trans x y z _ _ ih₁ ih₂ => exact Relation.EqvGen.trans _ _ _ ih₁ ih₂

/-- R22B-25: `blockRel a b` is the least: it lies in every `G`-invariant equivalence relation in
which `a ~ b`. -/
theorem blockRel_le {r : X → X → Prop} (hr : Equivalence r) (hinv : ∀ g : G, ∀ x y, r x y →
    r (g • x) (g • y)) {a b : X} (hab : r a b) {x y : X} (h : blockRel G a b x y) : r x y := by
  induction h with
  | rel x y hxy =>
    obtain ⟨k, rfl, rfl⟩ := hxy
    exact hinv k a b hab
  | refl x => exact hr.refl x
  | symm x y _ ih => exact hr.symm ih
  | trans x y z _ _ ih₁ ih₂ => exact hr.trans ih₁ ih₂

variable (G) in
/-- The class of `x` under `blockRel a b`. -/
def blockClass (a b x : X) : Set X :=
  {y | blockRel G a b x y}

/-- `G` permutes the classes of `blockRel a b`. -/
theorem smul_blockClass (a b x : X) (g : G) :
    g • blockClass G a b x = blockClass G a b (g • x) := by
  ext y
  simp only [blockClass, Set.mem_smul_set_iff_inv_smul_mem, Set.mem_ofPred_eq]
  constructor
  · intro h
    simpa using blockRel_smul h g
  · intro h
    simpa using blockRel_smul h g⁻¹

/-- R22B-25: the classes of `blockRel a b` are blocks, so they form a block system. -/
theorem isBlock_blockClass (a b x : X) : IsBlock G (blockClass G a b x) := by
  rw [isBlock_iff_smul_eq_or_disjoint]
  intro g
  rw [smul_blockClass]
  by_cases h : blockRel G a b x (g • x)
  · left
    ext y
    simp only [blockClass, Set.mem_ofPred_eq]
    exact ⟨fun hy => (blockRel_equivalence a b).trans h hy,
      fun hy => (blockRel_equivalence a b).trans ((blockRel_equivalence a b).symm h) hy⟩
  · right
    rw [Set.disjoint_left]
    intro y hy hy'
    exact h ((blockRel_equivalence a b).trans hy' ((blockRel_equivalence a b).symm hy))

/-- R22B-25: for a transitive action, `G` is primitive exactly when, for a fixed `a`, every
`blockRel a b` with `b ≠ a` is the total relation. -/
theorem isPreprimitive_iff_blockRel [IsPretransitive G X] (a : X) :
    IsPreprimitive G X ↔ ∀ b, b ≠ a → ∀ x y, blockRel G a b x y := by
  constructor
  · intro hprim b hb x y
    have htriv := hprim.isTrivialBlock_of_isBlock (isBlock_blockClass (G := G) a b a)
    rcases htriv with hsub | huniv
    · exact absurd (hsub ((blockRel_equivalence a b).refl a) (blockRel_self a b)).symm hb
    · have hx : x ∈ blockClass G a b a := huniv ▸ Set.mem_univ x
      have hy : y ∈ blockClass G a b a := huniv ▸ Set.mem_univ y
      exact (blockRel_equivalence a b).trans ((blockRel_equivalence a b).symm hx) hy
  · intro h
    refine IsPreprimitive.of_isTrivialBlock_base a fun {B} haB hB => ?_
    by_cases hsub : B.Subsingleton
    · exact Or.inl hsub
    right
    obtain ⟨b, hbB, hba⟩ : ∃ b ∈ B, b ≠ a := by
      by_contra hne
      push Not at hne
      exact hsub fun x hx y hy => (hne x hx).trans (hne y hy).symm
    -- the translates of `B` give an invariant equivalence relation with `a ~ b`
    let r : X → X → Prop := fun x y => ∃ g : G, x ∈ g • B ∧ y ∈ g • B
    have hr : Equivalence r := by
      refine ⟨fun x => ?_, fun ⟨g, hx, hy⟩ => ⟨g, hy, hx⟩, fun ⟨g, hx, hy⟩ ⟨g', hy', hz⟩ => ?_⟩
      · obtain ⟨g, rfl⟩ := exists_smul_eq G a x
        exact ⟨g, Set.smul_mem_smul_set haB, Set.smul_mem_smul_set haB⟩
      · refine ⟨g, hx, ?_⟩
        rcases hB.smul_eq_smul_or_disjoint g g' with heq | hdisj
        · exact heq ▸ hz
        · exact absurd (Set.disjoint_left.1 hdisj hy hy') id
    have hinv : ∀ k : G, ∀ x y, r x y → r (k • x) (k • y) := by
      rintro k x y ⟨g, hx, hy⟩
      exact ⟨k * g, by rw [mul_smul]; exact Set.smul_mem_smul_set hx,
        by rw [mul_smul]; exact Set.smul_mem_smul_set hy⟩
    have hab : r a b := ⟨1, by simpa using haB, by simpa using hbB⟩
    ext y
    simp only [Set.mem_univ, iff_true]
    obtain ⟨g, hga, hgy⟩ := blockRel_le hr hinv hab (h b hba a y)
    rcases hB.smul_eq_smul_or_disjoint g 1 with heq | hdisj
    · rw [heq, one_smul] at hgy
      exact hgy
    · rw [one_smul] at hdisj
      exact absurd (Set.disjoint_left.1 hdisj hga haB) id

/-- R22B-25: a `blockRel a b`, `b ≠ a`, that is not total gives a nontrivial block, the class of
`a`, so the action is not primitive. -/
theorem not_isPreprimitive_of_blockRel {a b : X} (hb : b ≠ a) {x y : X}
    (hxy : ¬blockRel G a b x y) :
    IsBlock G (blockClass G a b a) ∧ ¬(blockClass G a b a).Subsingleton ∧
      blockClass G a b a ≠ Set.univ ∧ ¬IsPreprimitive G X := by
  have hns : ¬(blockClass G a b a).Subsingleton := fun hsub =>
    hb (hsub ((blockRel_equivalence a b).refl a) (blockRel_self a b)).symm
  have hnu : blockClass G a b a ≠ Set.univ := by
    intro huniv
    have hx : x ∈ blockClass G a b a := huniv ▸ Set.mem_univ x
    have hy : y ∈ blockClass G a b a := huniv ▸ Set.mem_univ y
    exact hxy ((blockRel_equivalence a b).trans ((blockRel_equivalence a b).symm hx) hy)
  refine ⟨isBlock_blockClass a b a, hns, hnu, fun hprim => ?_⟩
  rcases hprim.isTrivialBlock_of_isBlock (isBlock_blockClass (G := G) a b a) with h | h
  · exact hns h
  · exact hnu h

end Blocks

/-! ### Regular actions -/

/-- R22B-25: a transitive group of order `d` on `d` points is regular: only the identity fixes a
point. -/
theorem eq_one_of_smul_eq_of_card_eq {G X : Type*} [Group G] [MulAction G X] [Finite X]
    [IsPretransitive G X] (hcard : Nat.card G = Nat.card X) {g : G} {x : X} (hgx : g • x = x) :
    g = 1 := by
  have hX : 0 < Nat.card X := Nat.card_pos_iff.2 ⟨⟨x⟩, inferInstance⟩
  have hprod := Nat.card_congr (orbitProdStabilizerEquivGroup G x)
  rw [Nat.card_prod, orbit_eq_univ, Nat.card_univ, hcard] at hprod
  have h1 : Nat.card (stabilizer G x) = 1 := by
    have : Nat.card X * Nat.card (stabilizer G x) = Nat.card X * 1 := by rw [hprod, mul_one]
    exact Nat.eq_of_mul_eq_mul_left hX this
  have hbot := (Subgroup.card_eq_one).1 h1
  have hg : g ∈ stabilizer G x := hgx
  rw [hbot, Subgroup.mem_bot] at hg
  exact hg

/-- R22B-25: for a transitive group of order `d` on `d` points, `g ↦ g • a` is a bijection onto
the points, so the group has exactly one element for each image of `a`. -/
theorem bijective_smul_of_card_eq {G X : Type*} [Group G] [MulAction G X] [Finite X]
    [IsPretransitive G X] (hcard : Nat.card G = Nat.card X) (a : X) :
    Function.Bijective fun g : G => g • a := by
  refine ⟨fun g h hgh => ?_, fun x => exists_smul_eq G a x⟩
  have hgh : g • a = h • a := hgh
  have hfix : (h⁻¹ * g) • a = a := by rw [mul_smul, hgh, inv_smul_smul]
  have := eq_one_of_smul_eq_of_card_eq hcard hfix
  exact (inv_mul_eq_one.1 this).symm

/-! ### The imprimitive case: the block inequality criterion -/

section BlockCriterion

variable {X : Type*} [DecidableEq X]

/-- R22B-25, imprimitive case: the inequalities are necessary. If `k` vacates `T` (`T ⊆ k(H)`),
then for every block `B i` and its image `B j = k(B i)`, `|T ∩ B j| ≤ |H ∩ B i|`. No factor
hypothesis is needed. -/
theorem block_inequality_of_vacancy {ι : Type*} (B : ι → Finset X) {k : Perm X}
    {H T : Finset X} (hk : T ⊆ H.image k) {i j : ι} (hij : (B i).image k = B j) :
    (T ∩ B j).card ≤ (H ∩ B i).card := by
  calc (T ∩ B j).card ≤ (H.image k ∩ B j).card :=
        Finset.card_le_card (Finset.inter_subset_inter hk (Finset.Subset.refl _))
    _ = ((H ∩ B i).image k).card := by rw [← hij, Finset.image_inter _ _ k.injective]
    _ = (H ∩ B i).card := Finset.card_image_of_injective _ k.injective

/-- For `S, T ⊆ B` and `|T| ≤ |S|`, a permutation of `B` that is even when `|B| ≥ 3` maps a
subset of `S` onto `T`. -/
theorem exists_cover {B S T : Finset X} (hS : S ⊆ B) (hT : T ⊆ B) (hc : T.card ≤ S.card) :
    ∃ σ : Perm B, (3 ≤ B.card → σ ∈ alternatingGroup B) ∧
      ∀ t ∈ T, ∃ s : B, (s : X) ∈ S ∧ (σ s : X) = t := by
  by_cases h3 : 3 ≤ B.card
  · obtain ⟨σ, hσ, h⟩ := ProductAlt.exists_alt_cover h3 hS hT hc
    exact ⟨σ, fun _ => hσ, h⟩
  · obtain ⟨σ, h⟩ := ProductAlt.exists_perm_cover hS hT hc
    exact ⟨σ, fun h' => absurd h' h3, h⟩

/-- **R22B-25, imprimitive case.** Let `K` be a permutation group of `X` (the orbit `Δ`), with
blocks `B i` that cover `X` and that `K` permutes (`hblock`), and with independent factors
(`hfac`): every permutation of a block, even when the block has at least three points, is induced
by an element of `K` that fixes every other block pointwise. Then `K` vacates the targets `T`
(some `k ∈ K` with `T ⊆ k(H)`) exactly when some `k ∈ K` satisfies the block inequalities
`|T ∩ B j| ≤ |H ∩ B i|` whenever `k(B i) = B j` (in the source: `h_i ≥ t_κ(i)` for the block
permutation `κ` of `k`). The factors are a hypothesis (in the source a computation or R22B-19);
without them only `block_inequality_of_vacancy` holds. -/
theorem vacancy_iff_blocks {ι : Type*} [Finite ι] (K : Subgroup (Perm X)) (B : ι → Finset X)
    (hcov : ∀ x, ∃ i, x ∈ B i) (hblock : ∀ k ∈ K, ∀ i, ∃ j, (B i).image k = B j)
    (hfac : ∀ i, ∀ σ : Perm (B i), (3 ≤ (B i).card → σ ∈ alternatingGroup (B i)) →
      ∃ g ∈ K, (∀ x : B i, g x = σ x) ∧ ∀ j ≠ i, ∀ x ∈ B j, g x = x)
    (H T : Finset X) :
    (∃ k ∈ K, T ⊆ H.image k) ↔
      ∃ k ∈ K, ∀ i j, (B i).image k = B j → (T ∩ B j).card ≤ (H ∩ B i).card := by
  constructor
  · rintro ⟨k, hk, hT⟩
    exact ⟨k, hk, fun i j hij => block_inequality_of_vacancy B hT hij⟩
  · rintro ⟨k₀, hk₀, hineq⟩
    have := Fintype.ofFinite ι
    have hσ : ∀ j, ∃ σ : Perm (B j), (3 ≤ (B j).card → σ ∈ alternatingGroup (B j)) ∧
        ((T ∩ B j).card ≤ (H.image k₀ ∩ B j).card →
          ∀ t ∈ T ∩ B j, ∃ s : B j, (s : X) ∈ H.image k₀ ∩ B j ∧ (σ s : X) = t) := by
      intro j
      by_cases hc : (T ∩ B j).card ≤ (H.image k₀ ∩ B j).card
      · obtain ⟨σ, hσ, h⟩ := exists_cover Finset.inter_subset_right Finset.inter_subset_right hc
        exact ⟨σ, hσ, fun _ => h⟩
      · exact ⟨1, fun _ => one_mem _, fun h => absurd h hc⟩
    choose σ hσalt hσcov using hσ
    obtain ⟨p, hp, hpσ, -⟩ :=
      ProductAlt.exists_forall_of_factors σ Finset.univ fun j _ => hfac j (σ j) (hσalt j)
    refine ⟨p * k₀, mul_mem hp hk₀, fun t ht => ?_⟩
    obtain ⟨i, hi⟩ := hcov (k₀⁻¹ t)
    obtain ⟨j, hj⟩ := hblock k₀ hk₀ i
    have htj : t ∈ B j := by
      rw [← hj]
      exact Finset.mem_image.2 ⟨k₀⁻¹ t, hi, by simp⟩
    have hcard : (H.image k₀ ∩ B j).card = (H ∩ B i).card := by
      rw [← hj, ← Finset.image_inter _ _ k₀.injective,
        Finset.card_image_of_injective _ k₀.injective]
    obtain ⟨s, hs, hst⟩ :=
      hσcov j (hcard ▸ hineq i j hj) t (Finset.mem_inter.2 ⟨ht, htj⟩)
    obtain ⟨h, hh, hhs⟩ := Finset.mem_image.1 (Finset.mem_inter.1 hs).1
    exact Finset.mem_image.2 ⟨h, hh, by rw [Perm.mul_apply, hhs, hpσ j (Finset.mem_univ j) s, hst]⟩

end BlockCriterion

/-! ### Vacancy in part one's setting: only the target orbits matter -/

section Vacancy

open ProductAlt

variable {L Ω : Type*} [DecidableEq Ω] (A : Alphabet L Ω)

/-- **R22B-25, orbits that miss `T` impose nothing.** For any set `U` of chambers that contains the
targets and that every letter maps into itself (a union of orbits), a vacating word exists exactly
when the group `G|U` induced on `U` has an element that maps holes in `U` onto every target. So the
joint action on the union of the target orbits suffices (`vacancy_iff_targetOrbits`), several
orbits use one common element of their joint induced group, and a regular orbit is tested on its
`d` elements (`bijective_smul_of_card_eq`). -/
theorem vacancy_iff_restrict {U : Finset Ω} (hU : A.IsInvariant U) {H T : Finset Ω} (hT : T ⊆ U) :
    (∃ V : List L, T ⊆ H.image (A.wordPerm V)) ↔
      ∃ k ∈ (restrict (A.isStable_group hU)).range,
        T.subtype (· ∈ U) ⊆ (H.subtype (· ∈ U)).image k := by
  constructor
  · rintro ⟨V, hV⟩
    refine ⟨_, ⟨⟨A.wordPerm V, A.wordPerm_mem_group V⟩, rfl⟩, fun t ht => ?_⟩
    obtain ⟨x, hx, hxt⟩ := Finset.mem_image.1 (hV (Finset.mem_subtype.1 ht))
    have hxU : x ∈ U := (hU.wordPerm_mem_iff V).1 (hxt ▸ t.2)
    exact Finset.mem_image.2 ⟨⟨x, hxU⟩, Finset.mem_subtype.2 hx, Subtype.ext hxt⟩
  · rintro ⟨k, ⟨g, rfl⟩, hk⟩
    obtain ⟨V, hV⟩ := A.mem_group_iff.1 g.2
    refine ⟨V, fun t ht => ?_⟩
    have ht' : (⟨t, hT ht⟩ : U) ∈ T.subtype (· ∈ U) := Finset.mem_subtype.2 ht
    obtain ⟨s, hs, hst⟩ := Finset.mem_image.1 (hk ht')
    refine Finset.mem_image.2 ⟨s, Finset.mem_subtype.1 hs, ?_⟩
    rw [hV]
    exact congrArg Subtype.val hst

/-- The union of the orbits of the targets is invariant. -/
theorem isInvariant_targetOrbits [Fintype Ω] (T : Finset Ω) :
    A.IsInvariant (T.biUnion A.orbitOf) :=
  Alphabet.IsInvariant.biUnion fun t _ => (A.isOrbit_orbitOf t).invariant

/-- The targets lie in the union of their orbits. -/
theorem subset_targetOrbits [Fintype Ω] (T : Finset Ω) : T ⊆ T.biUnion A.orbitOf :=
  fun t ht => Finset.mem_biUnion.2 ⟨t, ht, A.mem_orbitOf_self t⟩

/-- **R22B-25.** Vacancy is decided by the joint action on the union of the target orbits: every
word in the restricted generators lifts to the same word, and orbits that miss `T` impose
nothing. -/
theorem vacancy_iff_targetOrbits [Fintype Ω] (H T : Finset Ω) :
    (∃ V : List L, T ⊆ H.image (A.wordPerm V)) ↔
      ∃ k ∈ (restrict (A.isStable_group (isInvariant_targetOrbits A T))).range,
        T.subtype (· ∈ T.biUnion A.orbitOf) ⊆
          (H.subtype (· ∈ T.biUnion A.orbitOf)).image k :=
  vacancy_iff_restrict A (isInvariant_targetOrbits A T) (subset_targetOrbits A T)

omit [DecidableEq Ω] in
/-- R22B-25: the block relation of the group of the alphabet is preserved by every letter. -/
theorem blockRel_act (l : L) {a b x y : Ω} (h : blockRel A.group a b x y) :
    blockRel A.group a b (A.act l x) (A.act l y) :=
  blockRel_smul h ⟨A.act l, Subgroup.subset_closure ⟨l, rfl⟩⟩

omit [DecidableEq Ω] in
/-- R22B-25, closing under the generators: `blockRel a b` of the group of the alphabet lies in
every equivalence relation with `a ~ b` that every letter preserves, so (with `blockRel_act`) it
is the least such relation: the closure computed with the letters is the block relation of the
group. -/
theorem blockRel_le_of_letters {r : Ω → Ω → Prop} (hr : Equivalence r)
    (hinv : ∀ l x y, r x y → r (A.act l x) (A.act l y)) {a b : Ω} (hab : r a b) {x y : Ω}
    (h : blockRel A.group a b x y) : r x y := by
  refine blockRel_le hr (fun g x y hxy => ?_) hab h
  obtain ⟨w, hw⟩ := A.mem_group_iff.1 g.2
  change r ((g : Perm Ω) x) ((g : Perm Ω) y)
  rw [← hw]
  clear hw
  induction w generalizing x y with
  | nil => simpa [Alphabet.wordPerm_nil] using hxy
  | cons l w ih =>
    rw [Alphabet.wordPerm_cons, Perm.mul_apply, Perm.mul_apply]
    exact ih _ _ (hinv l x y hxy)

end Vacancy

/-! ### The residual case: `Sym(12)` on the 66 pairs -/

namespace ResidualCase

/-- The two-element subsets of a 12-set: 66 points. -/
abbrev Pairs : Type := Set.powersetCard (Fin 12) 2

/-- `Sym(12)` acting on the pairs, as permutations of the 66 points. -/
noncomputable def toPairs : Perm (Fin 12) →* Perm Pairs :=
  MulAction.toPermHom (Perm (Fin 12)) Pairs

/-- The permutation group of the 66 pairs induced by `Sym(12)`. -/
noncomputable def K : Subgroup (Perm Pairs) :=
  toPairs.range

/-- R22B-25, residual case: there are 66 pairs. -/
theorem card_pairs : Nat.card Pairs = 66 := by
  rw [Set.powersetCard.card, Nat.card_eq_fintype_card, Fintype.card_fin]
  rfl

/-- `Sym(12)` acts faithfully on the pairs. -/
theorem toPairs_injective : Function.Injective toPairs := by
  have : FaithfulSMul (Perm (Fin 12)) Pairs :=
    Set.powersetCard.faithfulSMul (n := 2) (by norm_num)
      (by rw [ENat.card_eq_coe_fintype_card, Fintype.card_fin]; norm_num)
  exact MulAction.toPerm_injective

/-- R22B-25, residual case: the group has order `12!`. -/
theorem card_K_eq_factorial : Nat.card K = 12 ! := by
  rw [K, ← Nat.card_congr (MonoidHom.ofInjective toPairs_injective).toEquiv, Nat.card_perm,
    Nat.card_eq_fintype_card, Fintype.card_fin]

/-- **R22B-25, residual case**: the group has order 479,001,600. -/
theorem card_K : Nat.card K = 479001600 := by
  rw [card_K_eq_factorial]
  rfl

/-- R22B-25, residual case: `Sym(12)` acts primitively on the pairs (Mathlib's
`Set.powersetCard.isPreprimitive_perm`, since `12 ≠ 2·2`). -/
theorem isPreprimitive_perm : IsPreprimitive (Perm (Fin 12)) Pairs :=
  Set.powersetCard.isPreprimitive_perm (α := Fin 12) (n := 2) (by norm_num) (by simp) (by simp)

/-- **R22B-25, residual case**: the induced permutation group of the 66 pairs is primitive. -/
theorem isPreprimitive_K : IsPreprimitive K Pairs := by
  have := isPreprimitive_perm
  let F : Perm (Fin 12) → K := fun g => ⟨toPairs g, g, rfl⟩
  let f : Pairs →ₑ[F] Pairs := { toFun := id, map_smul' := fun _ _ => rfl }
  exact IsPreprimitive.of_surjective (f := f) fun x => ⟨x, rfl⟩

/-- **R22B-25, residual case**: the primitive group does not contain `Alt(66)`, since
`2·12! < 13! ≤ 66!` (no factorial is evaluated). -/
theorem not_alternatingGroup_le : ¬alternatingGroup Pairs ≤ K := by
  rw [alternatingGroup_le_iff, ← Nat.card_eq_fintype_card, card_pairs, card_K_eq_factorial]
  intro h
  have h1 : 13 ! ≤ 66 ! := Nat.factorial_le (by norm_num)
  have h2 : 13 ! = 13 * 12 ! := Nat.factorial_succ 12
  have h3 : 0 < 12 ! := Nat.factorial_pos 12
  omega

end ResidualCase

end Classifier

end Jumbling
