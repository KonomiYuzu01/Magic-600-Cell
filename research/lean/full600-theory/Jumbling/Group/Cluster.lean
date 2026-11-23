import Jumbling.Group.Words

set_option linter.style.header false in
/-!
# R22B-03: cluster certificates and separated commutators

`PROOFS-B.md` section 3, R22B-03 (layer "group" of `ARCHON-HANDOFF.md`).

* Cluster lemma (`clusterLemma`, `clusterExactTest`): a word `w` with permutation `p` that fixes
  `P ⊆ H ∩ T` pointwise and maps `X ⊆ H \ P` onto `Q ⊆ T \ P` leaves every chamber of `P ∪ Q` a
  hole, and a set `S` of chambers is vacant after `w` exactly when `p⁻¹(S) ⊆ H`.
* Separated overlap (`support_chronoComm_subset`, `separatedOverlap`): for permutations `a`, `b`
  with supports `A`, `B` and `I = A ∩ B`, the chronological commutator `c = a·b·a⁻¹·b⁻¹` moves only
  points of `I ∪ a⁻¹(I) ∪ b⁻¹(I)`; when `a(I)` and `b(I)` miss `I`, these three sets are disjoint
  and `c` is the product over `r ∈ I` of the disjoint three-cycles `(r, b⁻¹(r), a⁻¹(r))`.

The instance at b00 (depth 40 stop) is a computation and is not formalized.
-/

namespace Jumbling

open Equiv Finset

variable {Ω : Type*}

/-- The chronological commutator `a·b·a⁻¹·b⁻¹`: `a` acts first, then `b`, `a⁻¹` and `b⁻¹`. -/
def chronoComm (a b : Perm Ω) : Perm Ω :=
  b⁻¹ * a⁻¹ * b * a

/-- The word `u·v·u⁻¹·v⁻¹` has the chronological commutator of the permutations of `u` and `v`. -/
theorem Alphabet.wordPerm_commutator {L : Type*} (A : Alphabet L Ω) (u v : List L) :
    A.wordPerm (u ++ v ++ A.invWord u ++ A.invWord v) =
      chronoComm (A.wordPerm u) (A.wordPerm v) := by
  simp [chronoComm, mul_assoc]

theorem chronoComm_apply (a b : Perm Ω) (x : Ω) : chronoComm a b x = b⁻¹ (a⁻¹ (b (a x))) := rfl

/-- In a list of pairwise disjoint permutations, the product acts at a point moved by one factor as
that factor. -/
theorem list_prod_apply_of_pairwise_disjoint {l : List (Perm Ω)} (hl : l.Pairwise Perm.Disjoint)
    {x : Ω} :
    ((∀ σ ∈ l, σ x = x) → l.prod x = x) ∧ (∀ σ ∈ l, σ x ≠ x → l.prod x = σ x) := by
  induction l with
  | nil => exact ⟨fun _ => rfl, fun σ hσ => absurd hσ List.not_mem_nil⟩
  | cons σ₀ l ih =>
    rw [List.pairwise_cons] at hl
    obtain ⟨ih₁, ih₂⟩ := ih hl.2
    refine ⟨fun h => ?_, fun σ hσ hx => ?_⟩
    · rw [List.prod_cons, Perm.mul_apply, ih₁ (fun τ hτ => h τ (List.mem_cons_of_mem σ₀ hτ)),
        h σ₀ List.mem_cons_self]
    · rw [List.prod_cons, Perm.mul_apply]
      rcases List.mem_cons.1 hσ with rfl | hσ
      · rw [ih₁ fun τ hτ => ((hl.1 τ hτ) x).resolve_left hx]
      · rw [ih₂ σ hσ hx]
        have hσx : σ (σ x) ≠ σ x := fun h => hx (σ.injective h)
        exact ((hl.1 σ hσ) (σ x)).resolve_right hσx

variable [DecidableEq Ω]

/-- R22B-03, cluster lemma. Let `p` be the permutation of a SAFE-ALL word, `H` the holes and `T` the
target. If `P ⊆ H ∩ T`, `Q ⊆ T \ P`, `p` fixes `P` pointwise and `p(X) = Q` for some `X ⊆ H \ P`,
then every chamber of `P ∪ Q` is a target chamber that is a hole after the word: the new holes
`p(H)` contain `p(P) ∪ p(X) = P ∪ Q`. -/
theorem clusterLemma (p : Perm Ω) {H T P Q X : Finset Ω} (hP : P ⊆ H ∩ T) (hQ : Q ⊆ T \ P)
    (hfix : ∀ x ∈ P, p x = x) (hX : X ⊆ H \ P) (hXQ : X.image p = Q) :
    P ∪ Q ⊆ T ∧ P ∪ Q ⊆ H.image p := by
  refine ⟨Finset.union_subset (hP.trans Finset.inter_subset_right)
    (hQ.trans Finset.sdiff_subset), Finset.union_subset ?_ ?_⟩
  · intro x hx
    exact Finset.mem_image.2 ⟨x, Finset.mem_of_mem_inter_left (hP hx), hfix x hx⟩
  · rw [← hXQ]
    exact Finset.image_subset_image (hX.trans Finset.sdiff_subset)

/-- R22B-03, the exact test: a set `S` of chambers (for the cluster lemma, `S = P ∪ Q`) is vacant
after a word with permutation `p` exactly when `p⁻¹(S) ⊆ H`. -/
theorem clusterExactTest (p : Perm Ω) (H S : Finset Ω) :
    S ⊆ H.image p ↔ S.image ⇑p⁻¹ ⊆ H := by
  constructor
  · intro h y hy
    obtain ⟨x, hx, rfl⟩ := Finset.mem_image.1 hy
    exact (Alphabet.mem_image_perm p H x).1 (h hx)
  · intro h x hx
    exact (Alphabet.mem_image_perm p H x).2 (h (Finset.mem_image_of_mem _ hx))

section Support

variable [Fintype Ω]

theorem inv_apply_of_notMem_support {σ : Perm Ω} {x : Ω} (hx : x ∉ σ.support) : σ⁻¹ x = x := by
  rw [Perm.inv_eq_iff_eq, Perm.notMem_support.1 hx]

theorem apply_of_notMem_support {σ : Perm Ω} {x : Ω} (hx : x ∉ σ.support) : σ x = x :=
  Perm.notMem_support.1 hx

theorem inv_apply_mem_support {σ : Perm Ω} {x : Ω} : σ⁻¹ x ∈ σ.support ↔ x ∈ σ.support := by
  rw [← Perm.apply_mem_support, perm_apply_inv_self]

/-- R22B-03, separated overlap, first part: with `I = A ∩ B`, the chronological commutator of `a`
and `b` moves only points of `I ∪ a⁻¹(I) ∪ b⁻¹(I)`. -/
theorem support_chronoComm_subset (a b : Perm Ω) :
    (chronoComm a b).support ⊆ (a.support ∩ b.support) ∪ (a.support ∩ b.support).image ⇑a⁻¹ ∪
      (a.support ∩ b.support).image ⇑b⁻¹ := by
  intro x hx
  rw [Perm.mem_support] at hx
  by_contra hout
  simp only [Finset.mem_union, Finset.mem_inter, Finset.mem_image, not_or, not_exists,
    not_and] at hout
  obtain ⟨⟨hI, haI⟩, hbI⟩ := hout
  -- `a x ∉ I` and `b x ∉ I`.
  have haI' : ¬(a x ∈ a.support ∧ a x ∈ b.support) := fun h => haI (a x) h (by simp)
  have hbI' : ¬(b x ∈ a.support ∧ b x ∈ b.support) := fun h => hbI (b x) h (by simp)
  apply hx
  rw [chronoComm_apply]
  by_cases hA : x ∈ a.support
  · have hB : x ∉ b.support := fun hB => hI hA hB
    have haB : a x ∉ b.support := fun h => haI' ⟨Perm.apply_mem_support.2 hA, h⟩
    rw [apply_of_notMem_support haB, perm_inv_apply_self, inv_apply_of_notMem_support hB]
  · rw [apply_of_notMem_support hA]
    by_cases hB : x ∈ b.support
    · have hbA : b x ∉ a.support := fun h => hbI' ⟨h, Perm.apply_mem_support.2 hB⟩
      rw [inv_apply_of_notMem_support hbA, perm_inv_apply_self]
    · rw [apply_of_notMem_support hB, inv_apply_of_notMem_support hA,
        inv_apply_of_notMem_support hB]

/-- R22B-03, separated overlap. Let `a` and `b` be permutations with supports `A` and `B`,
`I = A ∩ B`, and `c = a·b·a⁻¹·b⁻¹` (chronological). If `a(I) ∩ I = ∅` and `b(I) ∩ I = ∅`, then
`I`, `a⁻¹(I)` and `b⁻¹(I)` are pairwise disjoint, `c` sends each `r ∈ I` to `b⁻¹(r)`, `b⁻¹(r)` to
`a⁻¹(r)` and `a⁻¹(r)` to `r`, and `c` is the product over `r ∈ I` of the pairwise disjoint
three-cycles `(r, b⁻¹(r), a⁻¹(r))`. -/
theorem separatedOverlap (a b : Perm Ω) (ha : Disjoint ((a.support ∩ b.support).image a)
    (a.support ∩ b.support)) (hb : Disjoint ((a.support ∩ b.support).image b)
    (a.support ∩ b.support)) :
    let I := a.support ∩ b.support
    (Disjoint I (I.image ⇑a⁻¹) ∧ Disjoint I (I.image ⇑b⁻¹) ∧
      Disjoint (I.image ⇑a⁻¹) (I.image ⇑b⁻¹)) ∧
    (∀ r ∈ I, chronoComm a b r = b⁻¹ r ∧ chronoComm a b (b⁻¹ r) = a⁻¹ r ∧
      chronoComm a b (a⁻¹ r) = r) ∧
    (I.toList.map fun r => cycle3 r (b⁻¹ r) (a⁻¹ r)).Pairwise Perm.Disjoint ∧
    chronoComm a b = (I.toList.map fun r => cycle3 r (b⁻¹ r) (a⁻¹ r)).prod := by
  intro I
  -- Basic facts: for `r ∈ I`, `a r`, `b r`, `a⁻¹ r` and `b⁻¹ r` lie outside `I`.
  have haI : ∀ r ∈ I, a r ∉ I := fun r hr h =>
    Finset.disjoint_left.1 ha (Finset.mem_image_of_mem _ hr) h
  have hbI : ∀ r ∈ I, b r ∉ I := fun r hr h =>
    Finset.disjoint_left.1 hb (Finset.mem_image_of_mem _ hr) h
  have hainvI : ∀ r ∈ I, a⁻¹ r ∉ I := fun r hr h => by
    have := haI _ h
    rw [perm_apply_inv_self] at this
    exact this hr
  have hbinvI : ∀ r ∈ I, b⁻¹ r ∉ I := fun r hr h => by
    have := hbI _ h
    rw [perm_apply_inv_self] at this
    exact this hr
  have hmemI : ∀ {r}, r ∈ I ↔ r ∈ a.support ∧ r ∈ b.support := fun {r} => Finset.mem_inter
  -- `a⁻¹(I) ⊆ A \ B` and `b⁻¹(I) ⊆ B \ A`.
  have hainvA : ∀ r ∈ I, a⁻¹ r ∈ a.support ∧ a⁻¹ r ∉ b.support := fun r hr =>
    ⟨inv_apply_mem_support.2 (hmemI.1 hr).1,
      fun h => hainvI r hr (hmemI.2 ⟨inv_apply_mem_support.2 (hmemI.1 hr).1, h⟩)⟩
  have hbinvB : ∀ r ∈ I, b⁻¹ r ∈ b.support ∧ b⁻¹ r ∉ a.support := fun r hr =>
    ⟨inv_apply_mem_support.2 (hmemI.1 hr).2,
      fun h => hbinvI r hr (hmemI.2 ⟨h, inv_apply_mem_support.2 (hmemI.1 hr).2⟩)⟩
  have hdisj : Disjoint I (I.image ⇑a⁻¹) ∧ Disjoint I (I.image ⇑b⁻¹) ∧
      Disjoint (I.image ⇑a⁻¹) (I.image ⇑b⁻¹) := by
    refine ⟨Finset.disjoint_left.2 ?_, Finset.disjoint_left.2 ?_, Finset.disjoint_left.2 ?_⟩
    · intro x hx hx'
      obtain ⟨r, hr, rfl⟩ := Finset.mem_image.1 hx'
      exact hainvI r hr hx
    · intro x hx hx'
      obtain ⟨r, hr, rfl⟩ := Finset.mem_image.1 hx'
      exact hbinvI r hr hx
    · intro x hx hx'
      obtain ⟨r, hr, rfl⟩ := Finset.mem_image.1 hx
      obtain ⟨r', hr', hr'eq⟩ := Finset.mem_image.1 hx'
      exact (hbinvB r' hr').2 (hr'eq ▸ (hainvA r hr).1)
  -- The three values of `c`.
  have hval : ∀ r ∈ I, chronoComm a b r = b⁻¹ r ∧ chronoComm a b (b⁻¹ r) = a⁻¹ r ∧
      chronoComm a b (a⁻¹ r) = r := by
    intro r hr
    have hrA := (hmemI.1 hr).1
    have harB : a r ∉ b.support := fun h => haI r hr (hmemI.2 ⟨Perm.apply_mem_support.2 hrA, h⟩)
    have hbrA : b r ∉ a.support := fun h =>
      hbI r hr (hmemI.2 ⟨h, Perm.apply_mem_support.2 (hmemI.1 hr).2⟩)
    refine ⟨?_, ?_, ?_⟩
    · rw [chronoComm_apply, apply_of_notMem_support harB, perm_inv_apply_self]
    · rw [chronoComm_apply, apply_of_notMem_support (hbinvB r hr).2, perm_apply_inv_self,
        inv_apply_of_notMem_support (hainvA r hr).2]
    · rw [chronoComm_apply, perm_apply_inv_self, inv_apply_of_notMem_support hbrA,
        perm_inv_apply_self]
  -- Distinctness inside one triple.
  have hdist : ∀ r ∈ I, r ≠ b⁻¹ r ∧ r ≠ a⁻¹ r ∧ b⁻¹ r ≠ a⁻¹ r := fun r hr =>
    ⟨fun h => hbinvI r hr (h ▸ hr), fun h => hainvI r hr (h ▸ hr),
      fun h => (hbinvB r hr).2 (h ▸ (hainvA r hr).1)⟩
  -- The triple of `r` is moved by its three-cycle and nothing else is.
  let f : Ω → Perm Ω := fun r => cycle3 r (b⁻¹ r) (a⁻¹ r)
  have hf_out : ∀ r ∈ I, ∀ x, x ≠ r → x ≠ b⁻¹ r → x ≠ a⁻¹ r → f r x = x :=
    fun r _ x h1 h2 h3 => cycle3_apply_of_ne h1 h2 h3
  have hpair : (I.toList.map f).Pairwise Perm.Disjoint := by
    rw [List.pairwise_map]
    refine (Finset.nodup_toList I).imp_of_mem ?_
    intro r r' hr hr' hne x
    rw [Finset.mem_toList] at hr hr'
    by_cases h1 : x = r ∨ x = b⁻¹ r ∨ x = a⁻¹ r
    · right
      refine hf_out r' hr' x ?_ ?_ ?_
      · rintro rfl
        rcases h1 with h | h | h
        · exact hne h.symm
        · exact hbinvI r hr (h ▸ hr')
        · exact hainvI r hr (h ▸ hr')
      · intro hx
        rcases h1 with h | h | h
        · exact hbinvI r' hr' (hx ▸ h ▸ hr)
        · exact hne (b⁻¹.injective (h.symm.trans hx))
        · exact (hbinvB r' hr').2 (hx ▸ h ▸ (hainvA r hr).1)
      · intro hx
        rcases h1 with h | h | h
        · exact hainvI r' hr' (hx ▸ h ▸ hr)
        · exact (hbinvB r hr).2 (h ▸ hx ▸ (hainvA r' hr').1)
        · exact hne (a⁻¹.injective (h.symm.trans hx))
    · left
      simp only [not_or] at h1
      exact hf_out r hr x h1.1 h1.2.1 h1.2.2
  refine ⟨hdisj, hval, hpair, ?_⟩
  have hprod := fun x => list_prod_apply_of_pairwise_disjoint hpair (x := x)
  ext x
  by_cases hx : x ∈ I ∨ x ∈ I.image ⇑a⁻¹ ∨ x ∈ I.image ⇑b⁻¹
  · rcases hx with hx | hx | hx
    · have hfx : f x x = b⁻¹ x := cycle3_apply_left (hdist x hx).1 (hdist x hx).2.1
      rw [(hval x hx).1, (hprod x).2 (f x) (List.mem_map.2 ⟨x, Finset.mem_toList.2 hx, rfl⟩)
        (by rw [hfx]; exact (hdist x hx).1.symm), hfx]
    · obtain ⟨r, hr, rfl⟩ := Finset.mem_image.1 hx
      have hfx : f r (a⁻¹ r) = r := cycle3_apply_right _ _ _
      rw [(hval r hr).2.2, (hprod _).2 (f r) (List.mem_map.2 ⟨r, Finset.mem_toList.2 hr, rfl⟩)
        (by rw [hfx]; exact (hdist r hr).2.1), hfx]
    · obtain ⟨r, hr, rfl⟩ := Finset.mem_image.1 hx
      have hfx : f r (b⁻¹ r) = a⁻¹ r := cycle3_apply_mid (hdist r hr).2.1 (hdist r hr).2.2
      rw [(hval r hr).2.1, (hprod _).2 (f r) (List.mem_map.2 ⟨r, Finset.mem_toList.2 hr, rfl⟩)
        (by rw [hfx]; exact (hdist r hr).2.2.symm), hfx]
  · simp only [not_or] at hx
    have hc : chronoComm a b x = x := by
      by_contra h
      have := support_chronoComm_subset a b (Perm.mem_support.2 h)
      simp only [Finset.mem_union] at this
      tauto
    rw [hc, (hprod x).1]
    intro σ hσ
    obtain ⟨r, hr, rfl⟩ := List.mem_map.1 hσ
    rw [Finset.mem_toList] at hr
    refine hf_out r hr x (fun h => hx.1 (h ▸ hr)) (fun h => hx.2.2 ?_) (fun h => hx.2.1 ?_)
    · exact h ▸ Finset.mem_image_of_mem _ hr
    · exact h ▸ Finset.mem_image_of_mem _ hr

end Support

end Jumbling
