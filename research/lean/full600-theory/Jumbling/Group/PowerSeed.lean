import Jumbling.Group.Certified

set_option linter.style.header false in
/-!
# R22B-12: when a power of a word is a seed

`PROOFS-B.md` section 3, R22B-12 (layer "group" of `ARCHON-HANDOFF.md`). `Δ` is the current target
orbit, `E` the union of the earlier target orbits, and `q` the permutation of a SAFE word. The
length of the cycle of `q` through `x` is its minimal period `λ(x)`.

* `powerSeed_iff`: some positive power of `q` acts on `Δ` as a three-cycle and fixes `E` pointwise
  if and only if, on `E ∪ Δ`, `q` has exactly one cycle whose length is divisible by 3, and that
  cycle has length 3 and lies in `Δ` (`PowerSeedCriterion`).
* `powerSeed_of_criterion`, `otherLcm_dvd_of_seed`: then `M`, the least common multiple of the
  lengths of the other cycles on `E ∪ Δ` (`otherLcm`, 1 if there are none), is prime to 3, `q^M` is
  a seed, and every admissible exponent is a multiple of `M`; `Alphabet.powerSeed_word`: `q^M` is
  the seed word `w^M` of `|w|·M` letters.
* `powerSeed_bound`, `Alphabet.powerSeed_commutator`: if the other cycle lengths use at most `c`
  distinct primes and every orbit in `E ∪ Δ` has at most `D` chambers, then `M ≤ D^c`, and for a
  commutator `[x, y]`, `[x, yz]` or `[x, yzu]` the seed has at most `8·D^c` letters.
* `powerSeed_counterexample`: if `q` has a 3-cycle on an earlier orbit, no power of `q` is a seed.

`Ω` is finite (`[Finite Ω]`) where the statement needs every chamber to lie on a finite cycle.
-/

namespace Jumbling

open Equiv Finset Function

variable {Ω : Type*} [DecidableEq Ω]

/-- A permutation that acts on `Δ` as a three-cycle of distinct chambers of `Δ` and fixes `E`
pointwise: the permutation of a seed (H2 of R22B-01). -/
def IsSeedPerm (σ : Perm Ω) (Δ E : Finset Ω) : Prop :=
  ∃ a b c : Ω, a ∈ Δ ∧ b ∈ Δ ∧ c ∈ Δ ∧ a ≠ b ∧ a ≠ c ∧ b ≠ c ∧
    (∀ x ∈ Δ, σ x = cycle3 a b c x) ∧ ∀ x ∈ E, σ x = x

/-- R22B-12: a word is a seed (`Alphabet.IsSeed`) exactly when its permutation is a seed
permutation. -/
theorem Alphabet.isSeedPerm_iff {L : Type*} (A : Alphabet L Ω) (Δ E : Finset Ω) (C : List L) :
    IsSeedPerm (A.wordPerm C) Δ E ↔ ∃ a b c, A.IsSeed Δ E a b c C := by
  constructor
  · rintro ⟨a, b, c, ha, hb, hc, hab, hac, hbc, hacts, hfix⟩
    exact ⟨a, b, c, ⟨ha, hb, hc, hab, hac, hbc, hacts, hfix⟩⟩
  · rintro ⟨a, b, c, h⟩
    exact ⟨a, b, c, h.ha, h.hb, h.hc, h.hab, h.hac, h.hbc, h.acts, h.fixes⟩

omit [DecidableEq Ω] in
theorem pow_apply_eq_self_iff (q : Perm Ω) (m : ℕ) (x : Ω) :
    (q ^ m) x = x ↔ minimalPeriod q x ∣ m := by
  rw [← isPeriodicPt_iff_minimalPeriod_dvd, IsPeriodicPt, IsFixedPt, Perm.iterate_eq_pow]

omit [DecidableEq Ω] in
theorem perm_minimalPeriod_pos [Finite Ω] (q : Perm Ω) (x : Ω) : 0 < minimalPeriod q x := by
  refine IsPeriodicPt.minimalPeriod_pos (orderOf_pos q) ?_
  change (⇑q)^[orderOf q] x = x
  rw [Perm.iterate_eq_pow, pow_orderOf_eq_one, Perm.one_apply]

omit [DecidableEq Ω] in
theorem pow_apply_mod (q : Perm Ω) {p : Ω} (hp : minimalPeriod q p = 3) (n : ℕ) :
    (q ^ n) p = (q ^ (n % 3)) p := by
  rw [← Perm.iterate_eq_pow, ← Perm.iterate_eq_pow, ← hp, iterate_mod_minimalPeriod_eq]

omit [DecidableEq Ω] in
/-- A point with `q x ≠ x` and `q (q (q x)) = x` lies on a cycle of length 3. -/
theorem minimalPeriod_eq_three (q : Perm Ω) {x : Ω} (h1 : q x ≠ x) (h3 : q (q (q x)) = x) :
    minimalPeriod q x = 3 := by
  have : Fact (Nat.Prime 3) := ⟨Nat.prime_three⟩
  exact minimalPeriod_eq_prime (by simpa [IsPeriodicPt, IsFixedPt] using h3) h1

omit [DecidableEq Ω] in
/-- A cycle of `q` through `x` lies in any set that contains `x` and that `q` maps into itself, so
its length is at most the size of that set. -/
theorem minimalPeriod_le_card (q : Perm Ω) {O : Finset Ω} (hO : ∀ y ∈ O, q y ∈ O) {x : Ω}
    (hx : x ∈ O) : minimalPeriod q x ≤ O.card := by
  have hmaps : ∀ k, (⇑q)^[k] x ∈ O := by
    intro k
    induction k with
    | zero => exact hx
    | succ k ih => rw [iterate_succ_apply']; exact hO _ ih
  have := Finset.card_le_card_of_injOn (s := Finset.range (minimalPeriod q x))
    (fun k => (⇑q)^[k] x) (fun k _ => hmaps k)
    (by rw [Finset.coe_range]; exact iterate_injOn_Iio_minimalPeriod)
  simpa using this

/-- The anchors of a seed power: if `q^m` acts on `Δ` (a set that `q` maps into itself) as the
three-cycle `(a b c)`, then `q` itself cycles `a, b, c` with length 3. -/
theorem seed_anchor_period (q : Perm Ω) {Δ : Finset Ω} (hΔ : ∀ x ∈ Δ, q x ∈ Δ) {m : ℕ}
    {a b c : Ω} (ha : a ∈ Δ) (hb : b ∈ Δ) (hc : c ∈ Δ) (hab : a ≠ b) (hac : a ≠ c) (hbc : b ≠ c)
    (hact : ∀ x ∈ Δ, (q ^ m) x = cycle3 a b c x) :
    ((q a = b ∧ q b = c ∧ q c = a) ∨ (q a = c ∧ q c = b ∧ q b = a)) ∧
      minimalPeriod q a = 3 ∧ minimalPeriod q b = 3 ∧ minimalPeriod q c = 3 := by
  have hcomm : ∀ x, q ((q ^ m) x) = (q ^ m) (q x) := fun x => by
    rw [← Perm.mul_apply, ← Perm.mul_apply, (Commute.self_pow q m).eq]
  have hσa : (q ^ m) a = b := by rw [hact a ha, cycle3_apply_left hab hac]
  have hσb : (q ^ m) b = c := by rw [hact b hb, cycle3_apply_mid hac hbc]
  have hσc : (q ^ m) c = a := by rw [hact c hc, cycle3_apply_right]
  have hcyc : (q a = b ∧ q b = c ∧ q c = a) ∨ (q a = c ∧ q c = b ∧ q b = a) := by
    by_cases h1 : q a = a
    · have : (q ^ m) a = a := Perm.pow_apply_eq_self_of_apply_eq_self h1 m
      exact absurd (hσa.symm.trans this) (Ne.symm hab)
    by_cases h2 : q a = b
    · left
      have hqb : q b = c := by rw [← hσa, hcomm, h2, hσb]
      refine ⟨h2, hqb, ?_⟩
      rw [← hσb, hcomm, hqb, hσc]
    by_cases h3 : q a = c
    · right
      have hqb : q b = a := by rw [← hσa, hcomm, h3, hσc]
      refine ⟨h3, ?_, hqb⟩
      rw [← hσb, hcomm, hqb, hσa]
    exfalso
    have hfix : (q ^ m) (q a) = q a := by
      rw [hact _ (hΔ a ha), cycle3_apply_of_ne h1 h2 h3]
    rw [← hcomm, hσa] at hfix
    exact hab (q.injective hfix).symm
  refine ⟨hcyc, ?_⟩
  rcases hcyc with ⟨h1, h2, h3⟩ | ⟨h1, h2, h3⟩
  · refine ⟨minimalPeriod_eq_three q (by rw [h1]; exact Ne.symm hab) (by rw [h1, h2, h3]),
      minimalPeriod_eq_three q (by rw [h2]; exact Ne.symm hbc) (by rw [h2, h3, h1]),
      minimalPeriod_eq_three q (by rw [h3]; exact hac) (by rw [h3, h1, h2])⟩
  · refine ⟨minimalPeriod_eq_three q (by rw [h1]; exact Ne.symm hac) (by rw [h1, h2, h3]),
      minimalPeriod_eq_three q (by rw [h3]; exact hab) (by rw [h3, h1, h2]),
      minimalPeriod_eq_three q (by rw [h2]; exact hbc) (by rw [h2, h3, h1])⟩

/-- The criterion of R22B-12: on `E ∪ Δ`, `q` has exactly one cycle whose length is divisible by 3,
and that cycle has length 3 and lies in `Δ`. Equivalently, some `p ∈ Δ` lies on a cycle of length 3
and every chamber of `E ∪ Δ` whose cycle length is divisible by 3 lies on the cycle of `p`. -/
def PowerSeedCriterion (q : Perm Ω) (Δ E : Finset Ω) : Prop :=
  ∃ p ∈ Δ, minimalPeriod q p = 3 ∧
    ∀ x ∈ E ∪ Δ, 3 ∣ minimalPeriod q x → x = p ∨ x = q p ∨ x = q (q p)

/-- `M` of R22B-12: the least common multiple of the lengths of the cycles of `q` on `E ∪ Δ` whose
length is not divisible by 3 (1 if there are none). -/
noncomputable def otherLcm (q : Perm Ω) (Δ E : Finset Ω) : ℕ :=
  ((E ∪ Δ).filter fun x => ¬3 ∣ minimalPeriod q x).lcm fun x => minimalPeriod q x

omit [DecidableEq Ω] in
theorem prime_dvd_finset_lcm {β : Type*} {p : ℕ} (hp : p.Prime) {s : Finset β} {f : β → ℕ}
    (h : p ∣ s.lcm f) : ∃ b ∈ s, p ∣ f b := by
  classical
  induction s using Finset.induction_on with
  | empty =>
    rw [Finset.lcm_empty, Nat.dvd_one] at h
    exact absurd h hp.ne_one
  | insert a s ha ih =>
    rw [Finset.lcm_insert] at h
    have : p ∣ f a * s.lcm f := h.trans (lcm_dvd (dvd_mul_right _ _) (dvd_mul_left _ _))
    rcases hp.dvd_mul.1 this with h1 | h1
    · exact ⟨a, Finset.mem_insert_self a s, h1⟩
    · obtain ⟨b, hb, hb'⟩ := ih h1
      exact ⟨b, Finset.mem_insert_of_mem hb, hb'⟩

/-- R22B-12, the direction from a seed power to the criterion. -/
theorem criterion_of_seed (q : Perm Ω) {Δ E : Finset Ω} (hΔ : ∀ x ∈ Δ, q x ∈ Δ) {m : ℕ}
    (hseed : IsSeedPerm (q ^ m) Δ E) : PowerSeedCriterion q Δ E := by
  obtain ⟨a, b, c, ha, hb, hc, hab, hac, hbc, hact, hfixE⟩ := hseed
  obtain ⟨hcyc, ha3, -, -⟩ := seed_anchor_period q hΔ ha hb hc hab hac hbc hact
  have h3m : ¬3 ∣ m := by
    intro h
    have := (pow_apply_eq_self_iff q m a).2 (ha3 ▸ h)
    rw [hact a ha, cycle3_apply_left hab hac] at this
    exact hab this.symm
  refine ⟨a, ha, ha3, fun x hx h3x => ?_⟩
  by_contra hne
  simp only [not_or] at hne
  obtain ⟨hxa, hxqa, hxqqa⟩ := hne
  have hxb : x ≠ b := by
    rcases hcyc with ⟨h1, -, -⟩ | ⟨h1, h2, -⟩
    · rw [← h1]; exact hxqa
    · rw [← h2, ← h1]; exact hxqqa
  have hxc : x ≠ c := by
    rcases hcyc with ⟨h1, h2, -⟩ | ⟨h1, -, -⟩
    · rw [← h2, ← h1]; exact hxqqa
    · rw [← h1]; exact hxqa
  have hfix : (q ^ m) x = x := by
    rcases Finset.mem_union.1 hx with hxE | hxΔ
    · exact hfixE x hxE
    · rw [hact x hxΔ, cycle3_apply_of_ne hxa hxb hxc]
  exact h3m (h3x.trans ((pow_apply_eq_self_iff q m x).1 hfix))

/-- R22B-12: every admissible exponent is a multiple of `M`. -/
theorem otherLcm_dvd_of_seed (q : Perm Ω) {Δ E : Finset Ω} (hΔ : ∀ x ∈ Δ, q x ∈ Δ) {m : ℕ}
    (hseed : IsSeedPerm (q ^ m) Δ E) : otherLcm q Δ E ∣ m := by
  obtain ⟨a, b, c, ha, hb, hc, hab, hac, hbc, hact, hfixE⟩ := hseed
  obtain ⟨-, ha3, hb3, hc3⟩ := seed_anchor_period q hΔ ha hb hc hab hac hbc hact
  refine Finset.lcm_dvd fun x hx => ?_
  obtain ⟨hx, h3x⟩ := Finset.mem_filter.1 hx
  have hne : ∀ y, minimalPeriod q y = 3 → x ≠ y := fun y hy h => h3x (by rw [h, hy])
  rw [← pow_apply_eq_self_iff]
  rcases Finset.mem_union.1 hx with hxE | hxΔ
  · exact hfixE x hxE
  · rw [hact x hxΔ, cycle3_apply_of_ne (hne a ha3) (hne b hb3) (hne c hc3)]

/-- R22B-12, the direction from the criterion: `M` is prime to 3 and positive, and `q^M` acts on
`Δ` as a three-cycle and fixes `E` pointwise. -/
theorem powerSeed_of_criterion [Finite Ω] (q : Perm Ω) {Δ E : Finset Ω}
    (hΔ : ∀ x ∈ Δ, q x ∈ Δ) (hdisj : Disjoint Δ E) (hcrit : PowerSeedCriterion q Δ E) :
    ¬3 ∣ otherLcm q Δ E ∧ 0 < otherLcm q Δ E ∧ IsSeedPerm (q ^ otherLcm q Δ E) Δ E := by
  obtain ⟨p, hpΔ, hp3, huniq⟩ := hcrit
  set M := otherLcm q Δ E with hM
  have h3M : ¬3 ∣ M := by
    intro h
    obtain ⟨x, hx, h3x⟩ := prime_dvd_finset_lcm Nat.prime_three h
    exact (Finset.mem_filter.1 hx).2 h3x
  have hMpos : 0 < M := by
    rw [Nat.pos_iff_ne_zero, hM, otherLcm, Ne, Finset.lcm_eq_zero_iff]
    rintro ⟨x, -, hx⟩
    exact (perm_minimalPeriod_pos q x).ne' hx
  have hfix : ∀ x ∈ E ∪ Δ, x ≠ p → x ≠ q p → x ≠ q (q p) → (q ^ M) x = x := by
    intro x hx h1 h2 h3
    have hn3 : ¬3 ∣ minimalPeriod q x := fun h => by
      rcases huniq x hx h with h | h | h <;> contradiction
    rw [pow_apply_eq_self_iff]
    exact Finset.dvd_lcm (Finset.mem_filter.2 ⟨hx, hn3⟩)
  have hq1 : (q ^ 1) p = q p := by rw [pow_one]
  have hq2 : (q ^ 2) p = q (q p) := by rw [pow_two, Perm.mul_apply]
  have hq0 : (q ^ 0) p = p := by rw [pow_zero, Perm.one_apply]
  have hshift : ∀ j, (q ^ M) ((q ^ j) p) = (q ^ ((M + j) % 3)) p := fun j => by
    rw [← Perm.mul_apply, ← pow_add, pow_apply_mod q hp3]
  have hinj := iterate_injOn_Iio_minimalPeriod (f := ⇑q) (x := p)
  rw [hp3] at hinj
  have hne : ∀ i j : ℕ, i < 3 → j < 3 → i ≠ j → (q ^ i) p ≠ (q ^ j) p := fun i j hi hj hij h =>
    hij (hinj hi hj (by simpa only [Perm.iterate_eq_pow] using h))
  have h01 : p ≠ q p := by
    have := hne 0 1 (by norm_num) (by norm_num) (by norm_num)
    rwa [hq0, hq1] at this
  have h02 : p ≠ q (q p) := by
    have := hne 0 2 (by norm_num) (by norm_num) (by norm_num)
    rwa [hq0, hq2] at this
  have h12 : q p ≠ q (q p) := by
    have := hne 1 2 (by norm_num) (by norm_num) (by norm_num)
    rwa [hq1, hq2] at this
  have hp1Δ : q p ∈ Δ := hΔ p hpΔ
  have hp2Δ : q (q p) ∈ Δ := hΔ _ hp1Δ
  have hfixE : ∀ x ∈ E, (q ^ M) x = x := fun x hx => hfix x (Finset.mem_union_left _ hx)
    (fun h => Finset.disjoint_left.1 hdisj (h ▸ hpΔ) hx)
    (fun h => Finset.disjoint_left.1 hdisj (h ▸ hp1Δ) hx)
    (fun h => Finset.disjoint_left.1 hdisj (h ▸ hp2Δ) hx)
  refine ⟨h3M, hMpos, ?_⟩
  have hmod : M % 3 = 1 ∨ M % 3 = 2 := by omega
  rcases hmod with hm | hm
  · have e0 : (q ^ M) p = q p := by
      have h := hshift 0
      rw [hq0, show (M + 0) % 3 = 1 by omega, hq1] at h
      exact h
    have e1 : (q ^ M) (q p) = q (q p) := by
      have h := hshift 1
      rw [hq1, show (M + 1) % 3 = 2 by omega, hq2] at h
      exact h
    have e2 : (q ^ M) (q (q p)) = p := by
      have h := hshift 2
      rw [hq2, show (M + 2) % 3 = 0 by omega, hq0] at h
      exact h
    refine ⟨p, q p, q (q p), hpΔ, hp1Δ, hp2Δ, h01, h02, h12, fun x hx => ?_, hfixE⟩
    by_cases hx0 : x = p
    · rw [hx0, e0, cycle3_apply_left h01 h02]
    by_cases hx1 : x = q p
    · rw [hx1, e1, cycle3_apply_mid h02 h12]
    by_cases hx2 : x = q (q p)
    · rw [hx2, e2, cycle3_apply_right]
    rw [hfix x (Finset.mem_union_right _ hx) hx0 hx1 hx2, cycle3_apply_of_ne hx0 hx1 hx2]
  · have e0 : (q ^ M) p = q (q p) := by
      have h := hshift 0
      rw [hq0, show (M + 0) % 3 = 2 by omega, hq2] at h
      exact h
    have e1 : (q ^ M) (q p) = p := by
      have h := hshift 1
      rw [hq1, show (M + 1) % 3 = 0 by omega, hq0] at h
      exact h
    have e2 : (q ^ M) (q (q p)) = q p := by
      have h := hshift 2
      rw [hq2, show (M + 2) % 3 = 1 by omega, hq1] at h
      exact h
    refine ⟨p, q (q p), q p, hpΔ, hp2Δ, hp1Δ, h02, h01, Ne.symm h12, fun x hx => ?_, hfixE⟩
    by_cases hx0 : x = p
    · rw [hx0, e0, cycle3_apply_left h02 h01]
    by_cases hx2 : x = q (q p)
    · rw [hx2, e2, cycle3_apply_mid h01 (Ne.symm h12)]
    by_cases hx1 : x = q p
    · rw [hx1, e1, cycle3_apply_right]
    rw [hfix x (Finset.mem_union_right _ hx) hx0 hx1 hx2, cycle3_apply_of_ne hx0 hx2 hx1]

/-- R22B-12, when a power of a word is a seed. Let `Δ` be the current target orbit (`q` maps it
into itself) and `E` the union of the earlier ones, disjoint from `Δ`. Some positive power of `q`
acts on `Δ` as a three-cycle and fixes `E` pointwise if and only if the criterion holds. Then `M`
is prime to 3, `q^M` is a seed, and every admissible exponent is a multiple of `M`. -/
theorem powerSeed_iff [Finite Ω] (q : Perm Ω) {Δ E : Finset Ω} (hΔ : ∀ x ∈ Δ, q x ∈ Δ)
    (hdisj : Disjoint Δ E) :
    ((∃ m, 0 < m ∧ IsSeedPerm (q ^ m) Δ E) ↔ PowerSeedCriterion q Δ E) ∧
    (PowerSeedCriterion q Δ E →
      ¬3 ∣ otherLcm q Δ E ∧ IsSeedPerm (q ^ otherLcm q Δ E) Δ E) ∧
    (∀ m, IsSeedPerm (q ^ m) Δ E → otherLcm q Δ E ∣ m) := by
  refine ⟨⟨fun ⟨m, _, h⟩ => criterion_of_seed q hΔ h, fun h => ?_⟩, fun h => ?_,
    fun m h => otherLcm_dvd_of_seed q hΔ h⟩
  · obtain ⟨-, hpos, hseed⟩ := powerSeed_of_criterion q hΔ hdisj h
    exact ⟨_, hpos, hseed⟩
  · obtain ⟨h3, -, hseed⟩ := powerSeed_of_criterion q hΔ hdisj h
    exact ⟨h3, hseed⟩

/-- R22B-12, counterexample: if `q` has a cycle of length 3 on an earlier orbit (whether or not
it has one on `Δ`), every power that kills the earlier cycle kills the anchors' cycle too, so no
power of `q` is a seed. -/
theorem powerSeed_counterexample (q : Perm Ω) {Δ E : Finset Ω} (hΔ : ∀ x ∈ Δ, q x ∈ Δ) {p' : Ω}
    (hp'E : p' ∈ E) (hp'3 : minimalPeriod q p' = 3) (m : ℕ) : ¬IsSeedPerm (q ^ m) Δ E := by
  rintro ⟨a, b, c, ha, hb, hc, hab, hac, hbc, hact, hfixE⟩
  obtain ⟨-, ha3, -, -⟩ := seed_anchor_period q hΔ ha hb hc hab hac hbc hact
  have h3m : 3 ∣ m := hp'3 ▸ (pow_apply_eq_self_iff q m p').1 (hfixE p' hp'E)
  have := (pow_apply_eq_self_iff q m a).2 (ha3 ▸ h3m)
  rw [hact a ha, cycle3_apply_left hab hac] at this
  exact hab this.symm

/-! ### The bound -/

/-- If every element of a finite set `S` of positive numbers is at most `D ≥ 1` and the elements use
at most `c` distinct primes, every prime power dividing the least common multiple is at most `D`, so
the least common multiple is at most `D^c`. -/
theorem lcm_le_pow (S : Finset ℕ) {D c : ℕ} (hD : 1 ≤ D) (hS : ∀ n ∈ S, 0 < n ∧ n ≤ D)
    (hc : (S.biUnion Nat.primeFactors).card ≤ c) : S.lcm id ≤ D ^ c := by
  set P := S.biUnion Nat.primeFactors with hP
  let g : ℕ → ℕ := fun p => S.sup fun n => p ^ n.factorization p
  have hg : ∀ p, g p ≤ D := fun p => Finset.sup_le fun n hn =>
    (Nat.le_of_dvd (hS n hn).1 (Nat.ordProj_dvd n p)).trans (hS n hn).2
  have hgpos : ∀ p ∈ P, 0 < g p := by
    intro p hp
    obtain ⟨n, hn, hpn⟩ := Finset.mem_biUnion.1 hp
    exact lt_of_lt_of_le (pow_pos (Nat.prime_of_mem_primeFactors hpn).pos _)
      (Finset.le_sup (f := fun n => p ^ n.factorization p) hn)
  have hK : ∏ p ∈ P, g p ≤ D ^ c := by
    calc ∏ p ∈ P, g p ≤ D ^ P.card := Finset.prod_le_pow_card P g D fun p _ => hg p
      _ ≤ D ^ c := Nat.pow_le_pow_right hD hc
  have hdvd : ∀ n ∈ S, n ∣ ∏ p ∈ P, g p := by
    intro n hn
    have hn0 : n ≠ 0 := (hS n hn).1.ne'
    conv_lhs => rw [← Nat.prod_factorization_pow_eq_self hn0]
    rw [Finsupp.prod, Nat.support_factorization]
    refine (Finset.prod_dvd_prod_of_dvd _ _ fun p hp => ?_).trans
      (Finset.prod_dvd_prod_of_subset _ _ _ (Finset.subset_biUnion_of_mem _ hn))
    obtain ⟨n', hn', hgn'⟩ := Finset.exists_mem_eq_sup S ⟨n, hn⟩
      (fun n => p ^ n.factorization p)
    change p ^ n.factorization p ∣ g p
    rw [show g p = p ^ n'.factorization p from hgn']
    apply Nat.pow_dvd_pow
    have hle : p ^ n.factorization p ≤ p ^ n'.factorization p :=
      hgn' ▸ Finset.le_sup (f := fun n => p ^ n.factorization p) hn
    exact (Nat.pow_le_pow_iff_right (Nat.prime_of_mem_primeFactors hp).one_lt).1 hle
  exact (Nat.le_of_dvd (Finset.prod_pos hgpos) (Finset.lcm_dvd fun n hn => hdvd n hn)).trans hK

/-- R22B-12, the bound on `M`: if the other cycle lengths use at most `c` distinct primes and every
chamber of `E ∪ Δ` lies in a set of at most `D` chambers that `q` maps into itself (its orbit, since
`q` is a word), then `M ≤ D^c`. -/
theorem powerSeed_bound [Finite Ω] (q : Perm Ω) (Δ E : Finset Ω) {D c : ℕ} (hD : 1 ≤ D)
    (horb : ∀ x ∈ E ∪ Δ, ∃ O : Finset Ω, x ∈ O ∧ O.card ≤ D ∧ ∀ y ∈ O, q y ∈ O)
    (hc : (((E ∪ Δ).filter fun x => ¬3 ∣ minimalPeriod q x).biUnion
      fun x => (minimalPeriod q x).primeFactors).card ≤ c) :
    otherLcm q Δ E ≤ D ^ c := by
  set S' := (E ∪ Δ).filter fun x => ¬3 ∣ minimalPeriod q x
  have heq : otherLcm q Δ E = (S'.image fun x => minimalPeriod q x).lcm id := by
    rw [Finset.lcm_image]
    rfl
  rw [heq]
  refine lcm_le_pow _ hD (fun n hn => ?_) (by rwa [Finset.image_biUnion])
  obtain ⟨x, hx, rfl⟩ := Finset.mem_image.1 hn
  obtain ⟨O, hxO, hOD, hO⟩ := horb x (Finset.mem_filter.1 hx).1
  exact ⟨perm_minimalPeriod_pos q x, (minimalPeriod_le_card q hO hxO).trans hOD⟩

namespace Alphabet

variable {L : Type*} (A : Alphabet L Ω)

omit [DecidableEq Ω] in
theorem wordPerm_replicate (w : List L) (n : ℕ) :
    A.wordPerm (List.replicate n w).flatten = A.wordPerm w ^ n := by
  induction n with
  | zero => rfl
  | succ n ih =>
    rw [List.replicate_succ, List.flatten_cons, wordPerm_append, ih, pow_succ]

omit [DecidableEq Ω] in
theorem length_replicate_flatten (w : List L) (n : ℕ) :
    (List.replicate n w).flatten.length = w.length * n := by
  induction n with
  | zero => rfl
  | succ n ih => rw [List.replicate_succ, List.flatten_cons, List.length_append, ih]; ring

/-- R22B-12 for words: under the criterion, `w^M` (the word `w` repeated `M` times) is a seed of
`|w|·M` letters. -/
theorem powerSeed_word [Finite Ω] (w : List L) {Δ E : Finset Ω} (hΔ : A.IsInvariant Δ)
    (hdisj : Disjoint Δ E) (hcrit : PowerSeedCriterion (A.wordPerm w) Δ E) :
    ∃ a b c, A.IsSeed Δ E a b c (List.replicate (otherLcm (A.wordPerm w) Δ E) w).flatten ∧
      (List.replicate (otherLcm (A.wordPerm w) Δ E) w).flatten.length =
        w.length * otherLcm (A.wordPerm w) Δ E := by
  have hΔ' : ∀ x ∈ Δ, A.wordPerm w x ∈ Δ := fun x hx => hΔ.wordPerm_mem w hx
  obtain ⟨-, -, hseed⟩ := powerSeed_of_criterion (A.wordPerm w) hΔ' hdisj hcrit
  rw [← wordPerm_replicate, isSeedPerm_iff] at hseed
  exact ⟨_, _, _, hseed.choose_spec.choose_spec.choose_spec, length_replicate_flatten _ _⟩

/-- The chronological commutator word `[l, v] = l·v·l⁻¹·v⁻¹` of a letter and a word. -/
def commWord (l : L) (v : List L) : List L :=
  l :: v ++ [A.inv l] ++ A.invWord v

omit [DecidableEq Ω] in
theorem length_commWord (l : L) (v : List L) : (A.commWord l v).length = 2 * v.length + 2 := by
  simp only [commWord, List.length_append, List.length_cons, List.length_nil, length_invWord]
  ring

/-- R22B-12, the bound for commutators: if `q` is `[x, y]`, `[x, yz]` or `[x, yzu]` for SAFE
letters (4, 6 or 8 letters), the other cycle lengths use at most `c` distinct primes and every
orbit in `E ∪ Δ` has at most `D` chambers, then under the criterion `q^M` is a seed of at most
`8·D^c` letters. -/
theorem powerSeed_commutator [Finite Ω] (l : L) (v : List L) (hv : v.length ≤ 3)
    {Δ E : Finset Ω} (hΔ : A.IsInvariant Δ) (hdisj : Disjoint Δ E) {D c : ℕ} (hD : 1 ≤ D)
    (horb : ∀ x ∈ E ∪ Δ, ∃ O : Finset Ω, x ∈ O ∧ O.card ≤ D ∧ A.IsInvariant O)
    (hc : (((E ∪ Δ).filter fun x => ¬3 ∣ minimalPeriod (A.wordPerm (A.commWord l v)) x).biUnion
      fun x => (minimalPeriod (A.wordPerm (A.commWord l v)) x).primeFactors).card ≤ c)
    (hcrit : PowerSeedCriterion (A.wordPerm (A.commWord l v)) Δ E) :
    ∃ (a b c' : Ω) (C : List L), A.IsSeed Δ E a b c' C ∧ C.length ≤ 8 * D ^ c := by
  obtain ⟨a, b, c', hseed, hlen⟩ := A.powerSeed_word (A.commWord l v) hΔ hdisj hcrit
  refine ⟨a, b, c', _, hseed, ?_⟩
  have hM := powerSeed_bound (A.wordPerm (A.commWord l v)) Δ E hD (fun x hx => by
    obtain ⟨O, hxO, hOD, hO⟩ := horb x hx
    exact ⟨O, hxO, hOD, fun y hy => hO.wordPerm_mem _ hy⟩) hc
  rw [hlen, length_commWord]
  calc (2 * v.length + 2) * otherLcm (A.wordPerm (A.commWord l v)) Δ E
      ≤ 8 * otherLcm (A.wordPerm (A.commWord l v)) Δ E := Nat.mul_le_mul_right _ (by omega)
    _ ≤ 8 * D ^ c := Nat.mul_le_mul_left _ hM

end Alphabet

end Jumbling
