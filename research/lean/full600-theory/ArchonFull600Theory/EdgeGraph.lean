import ArchonFull600Theory.Vertices
import Mathlib.Analysis.Convex.Exposed
import Mathlib.Analysis.Convex.Extreme
import Mathlib.Combinatorics.Enumerative.DoubleCounting
import Mathlib.Combinatorics.SimpleGraph.Clique
import Mathlib.Combinatorics.SimpleGraph.DegreeSum

set_option linter.style.header false in
/-!
# The edge graph and its cliques

The graph on the 120 vertices that joins two vertices when their inner product is `2 φ`, the
largest inner product of distinct vertices. Its cliques of sizes 1 to 4 are counted (120, 720,
1200 and 600; five triangles and five four-cliques at each edge, twenty four-cliques at each
vertex, and `(12, 30, 20)` cliques of sizes 2, 3 and 4 through each vertex), and the metric data of
Lemma 3 are proved: every vertex at distance 2 from the centre, the edge length `2 / φ`, and that
every vertex is exposed. `Facets` shows that these cliques span faces of `P` and that the
four-cliques are its facets.

The finite facts come from two Boolean certificates checked by `decide +kernel`: an adjacency
mask with one bit per ordered pair, compared against the exact inner products, and the common
neighbour counts of adjacent pairs and triangles. Double counting, proved for an arbitrary finite
graph, turns these into the counts.
-/

namespace ArchonFull600Theory

open scoped goldenRatio Quaternion RealInnerProductSpace
open Finset ZPhi Vec4

/-! ### Cliques in a finite graph -/

section Cliques

variable {α : Type*} [Fintype α] [DecidableEq α] (G : SimpleGraph α) [DecidableRel G.Adj]

/-- The common neighbours of every vertex of `b`. -/
def commonNbrs (b : Finset α) : Finset α := univ.filter fun x => ∀ y ∈ b, G.Adj x y

theorem disjoint_commonNbrs (b : Finset α) : Disjoint b (commonNbrs G b) := by
  rw [Finset.disjoint_left]
  intro x hxb hx
  simp only [commonNbrs, mem_filter, mem_univ, true_and] at hx
  exact G.loopless.irrefl x (hx x hxb)

/-- Cliques through a fixed clique `b` correspond to cliques among its common neighbours. -/
theorem card_cliques_through (b : Finset α) (hb : G.IsClique (b : Set α)) (k : ℕ) :
    #((G.cliqueFinset (#b + k)).filter (b ⊆ ·)) =
      #(((commonNbrs G b).powersetCard k).filter fun t : Finset α => G.IsClique (t : Set α)) := by
  refine Finset.card_nbij' (· \ b) (b ∪ ·) ?_ ?_ ?_ ?_
  · intro s hs
    simp only [coe_filter, Set.mem_ofPred_eq, SimpleGraph.mem_cliqueFinset_iff] at hs
    obtain ⟨hs, hbs⟩ := hs
    simp only [coe_filter, Set.mem_ofPred_eq, mem_powersetCard]
    refine ⟨⟨?_, ?_⟩, hs.1.subset (by simp)⟩
    · intro x hx
      rw [mem_sdiff] at hx
      simp only [commonNbrs, mem_filter, mem_univ, true_and]
      intro y hy
      exact hs.1 (by simpa using hx.1) (by simpa using hbs hy) (fun h => hx.2 (h ▸ hy))
    · rw [card_sdiff_of_subset hbs, hs.2]
      omega
  · intro t ht
    simp only [coe_filter, Set.mem_ofPred_eq, mem_powersetCard] at ht
    obtain ⟨⟨hsub, hcard⟩, ht⟩ := ht
    have hdisj : Disjoint b t := (disjoint_commonNbrs G b).mono_right hsub
    simp only [coe_filter, Set.mem_ofPred_eq, SimpleGraph.mem_cliqueFinset_iff]
    refine ⟨⟨?_, by rw [card_union_of_disjoint hdisj, hcard]⟩, subset_union_left⟩
    have : Std.Symm G.Adj := G.symm
    rw [coe_union, SimpleGraph.IsClique, Set.pairwise_union_of_symm]
    refine ⟨hb, ht, ?_⟩
    intro x hx y hy _
    have := hsub (by simpa using hy)
    simp only [commonNbrs, mem_filter, mem_univ, true_and] at this
    exact (this x (by simpa using hx)).symm
  · intro s hs
    simp only [coe_filter, Set.mem_ofPred_eq] at hs
    exact union_sdiff_of_subset hs.2
  · intro t ht
    simp only [coe_filter, Set.mem_ofPred_eq, mem_powersetCard] at ht
    exact union_sdiff_cancel_left ((disjoint_commonNbrs G b).mono_right ht.1.1)

/-- The cliques with one more vertex than a clique `b` and containing it correspond to the common
neighbours of `b`. -/
theorem card_cliques_succ_through (b : Finset α) (hb : G.IsClique (b : Set α)) :
    #((G.cliqueFinset (#b + 1)).filter (b ⊆ ·)) = #(commonNbrs G b) := by
  rw [card_cliques_through G b hb 1, filter_true_of_mem, card_powersetCard, Nat.choose_one_right]
  intro t ht
  obtain ⟨x, rfl⟩ := card_eq_one.mp (mem_powersetCard.mp ht).2
  simp

theorem card_cliques_succ_of_mem {n n' : ℕ} (hn : n + 1 = n') {b : Finset α}
    (hb : b ∈ G.cliqueFinset n) : #((G.cliqueFinset n').filter (b ⊆ ·)) = #(commonNbrs G b) := by
  subst hn
  obtain ⟨hc, rfl⟩ := SimpleGraph.mem_cliqueFinset_iff.mp hb
  exact card_cliques_succ_through G b hc

/-- Double counting of nested cliques: an `(n + 1)`-clique containing `b` contains
`n + 1 - #b` cliques of size `n` that contain `b`. -/
theorem card_cliques_double_count (b : Finset α) {n : ℕ} :
    (n + 1 - #b) * #((G.cliqueFinset (n + 1)).filter (b ⊆ ·)) =
      ∑ c ∈ (G.cliqueFinset n).filter (b ⊆ ·), #((G.cliqueFinset (n + 1)).filter (c ⊆ ·)) := by
  set T := (G.cliqueFinset (n + 1)).filter (b ⊆ ·)
  set C := (G.cliqueFinset n).filter (b ⊆ ·)
  have hc : ∀ c ∈ C, (G.cliqueFinset (n + 1)).filter (c ⊆ ·) = T.filter (c ⊆ ·) := by
    intro c hc
    simp only [C, mem_filter] at hc
    rw [filter_filter]
    exact filter_congr fun x _ => ⟨fun h => ⟨hc.2.trans h, h⟩, fun h => h.2⟩
  have hx : ∀ x ∈ T, #(C.filter (· ⊆ x)) = n + 1 - #b := by
    intro x hx
    simp only [T, mem_filter, SimpleGraph.mem_cliqueFinset_iff, SimpleGraph.isNClique_iff] at hx
    obtain ⟨⟨hxc, hxn⟩, hbx⟩ := hx
    rw [← hxn, ← card_sdiff_of_subset hbx]
    symm
    refine card_bij (fun y _ => x.erase y) ?_ ?_ ?_
    · intro y hy
      rw [mem_sdiff] at hy
      simp only [C, mem_filter, SimpleGraph.mem_cliqueFinset_iff, SimpleGraph.isNClique_iff]
      refine ⟨⟨⟨hxc.subset (by simp), ?_⟩, ?_⟩, erase_subset y x⟩
      · rw [card_erase_of_mem hy.1, hxn]
        rfl
      · intro z hz
        exact mem_erase.mpr ⟨fun h => hy.2 (h ▸ hz), hbx hz⟩
    · intro y1 hy1 y2 hy2 h
      exact erase_injOn x (mem_sdiff.mp hy1).1 (mem_sdiff.mp hy2).1 h
    · intro c hc
      simp only [C, mem_filter, SimpleGraph.mem_cliqueFinset_iff,
        SimpleGraph.isNClique_iff] at hc
      obtain ⟨⟨⟨_, hcn⟩, hbc⟩, hcx⟩ := hc
      have h1 : #(x \ c) = 1 := by
        rw [card_sdiff_of_subset hcx, hxn, hcn]
        omega
      obtain ⟨y, hy⟩ := card_eq_one.mp h1
      have hyx : y ∈ x \ c := hy ▸ mem_singleton_self y
      rw [mem_sdiff] at hyx
      refine ⟨y, mem_sdiff.mpr ⟨hyx.1, fun h => hyx.2 (hbc h)⟩, ?_⟩
      symm
      apply eq_of_subset_of_card_le
      · intro z hz
        exact mem_erase.mpr ⟨fun h => hyx.2 (h ▸ hz), hcx hz⟩
      · rw [card_erase_of_mem hyx.1, hxn, hcn]
        rfl
  rw [sum_congr rfl fun c hc' => congrArg card (hc c hc')]
  have key := sum_card_bipartiteAbove_eq_sum_card_bipartiteBelow (s := C) (t := T)
    (r := fun c x => c ⊆ x)
  simp only [bipartiteAbove, bipartiteBelow] at key
  rw [key, sum_congr rfl hx, sum_const, smul_eq_mul, mul_comm]

theorem card_cliques_double_count_const (b : Finset α) {n n' c0 : ℕ} (hn' : n + 1 = n')
    (h : ∀ c ∈ (G.cliqueFinset n).filter (b ⊆ ·), #(commonNbrs G c) = c0) :
    (n' - #b) * #((G.cliqueFinset n').filter (b ⊆ ·)) =
      #((G.cliqueFinset n).filter (b ⊆ ·)) * c0 := by
  subst hn'
  rw [card_cliques_double_count G b, sum_congr rfl fun c hc =>
    (card_cliques_succ_of_mem G rfl (mem_filter.mp hc).1).trans (h c hc), sum_const, smul_eq_mul]

theorem filter_empty_subset (n : ℕ) : (G.cliqueFinset n).filter (∅ ⊆ ·) = G.cliqueFinset n :=
  filter_true_of_mem fun _ _ => empty_subset _

theorem card_cliqueFinset_succ {n n' c0 : ℕ} (hn' : n + 1 = n')
    (h : ∀ c ∈ G.cliqueFinset n, #(commonNbrs G c) = c0) :
    n' * #(G.cliqueFinset n') = #(G.cliqueFinset n) * c0 := by
  have := card_cliques_double_count_const G ∅ hn' fun c hc => h c (mem_filter.mp hc).1
  rwa [card_empty, Nat.sub_zero, filter_empty_subset, filter_empty_subset] at this

theorem card_cliqueFinset_one : #(G.cliqueFinset 1) = Fintype.card α := by
  have h := card_cliques_succ_of_mem G (n := 0) rfl (b := ∅)
    (SimpleGraph.mem_cliqueFinset_iff.mpr (SimpleGraph.isNClique_zero.mpr rfl))
  have hu : commonNbrs G ∅ = univ := by
    ext x
    simp [commonNbrs]
  rw [filter_empty_subset, hu, card_univ] at h
  exact h

theorem card_commonNbrs_singleton (v : α) : #(commonNbrs G {v}) = G.degree v := by
  rw [← SimpleGraph.card_neighborFinset_eq_degree]
  congr 1
  ext x
  simp [commonNbrs, G.adj_comm]

theorem singleton_mem_cliqueFinset_one (v : α) : {v} ∈ G.cliqueFinset 1 :=
  SimpleGraph.mem_cliqueFinset_iff.mpr (SimpleGraph.isNClique_one.mpr ⟨v, rfl⟩)

/-- Global clique counts of a finite graph whose vertices, edges and triangles have `d`, `k` and
`m` common neighbours. -/
theorem cliqueCounts_of_commonNbrs {d k m : ℕ} (hd : ∀ v, #(commonNbrs G {v}) = d)
    (hk : ∀ e ∈ G.cliqueFinset 2, #(commonNbrs G e) = k)
    (hm : ∀ t ∈ G.cliqueFinset 3, #(commonNbrs G t) = m) :
    #(G.cliqueFinset 1) = Fintype.card α ∧ 2 * #(G.cliqueFinset 2) = Fintype.card α * d ∧
      3 * #(G.cliqueFinset 3) = #(G.cliqueFinset 2) * k ∧
      4 * #(G.cliqueFinset 4) = #(G.cliqueFinset 3) * m ∧
      2 * #G.edgeFinset = Fintype.card α * d := by
  have h1 : ∀ c ∈ G.cliqueFinset 1, #(commonNbrs G c) = d := by
    intro c hc
    obtain ⟨v, rfl⟩ := SimpleGraph.isNClique_one.mp (SimpleGraph.mem_cliqueFinset_iff.mp hc)
    exact hd v
  have e2 := card_cliqueFinset_succ G (n := 1) rfl h1
  rw [card_cliqueFinset_one] at e2
  refine ⟨card_cliqueFinset_one G, e2, card_cliqueFinset_succ G rfl hk,
    card_cliqueFinset_succ G rfl hm, ?_⟩
  rw [← SimpleGraph.sum_degrees_eq_twice_card_edges,
    sum_congr rfl fun v _ => (card_commonNbrs_singleton G v).symm.trans (hd v), sum_const,
    card_univ, smul_eq_mul]

/-- Local clique counts of the same graph: at a vertex, at an edge and at a triangle. -/
theorem localCliqueCounts_of_commonNbrs {d k m : ℕ} (hd : ∀ v, #(commonNbrs G {v}) = d)
    (hk : ∀ e ∈ G.cliqueFinset 2, #(commonNbrs G e) = k)
    (hm : ∀ t ∈ G.cliqueFinset 3, #(commonNbrs G t) = m) :
    (∀ v, #((G.cliqueFinset 2).filter (v ∈ ·)) = d ∧
      2 * #((G.cliqueFinset 3).filter (v ∈ ·)) = d * k ∧
      3 * #((G.cliqueFinset 4).filter (v ∈ ·)) = #((G.cliqueFinset 3).filter (v ∈ ·)) * m) ∧
    (∀ e ∈ G.cliqueFinset 2, #((G.cliqueFinset 3).filter (e ⊆ ·)) = k ∧
      2 * #((G.cliqueFinset 4).filter (e ⊆ ·)) = k * m) ∧
    (∀ t ∈ G.cliqueFinset 3, #((G.cliqueFinset 4).filter (t ⊆ ·)) = m) := by
  have ht : ∀ t ∈ G.cliqueFinset 3, #((G.cliqueFinset 4).filter (t ⊆ ·)) = m := fun t h =>
    (card_cliques_succ_of_mem G rfl h).trans (hm t h)
  have he : ∀ e ∈ G.cliqueFinset 2, #((G.cliqueFinset 3).filter (e ⊆ ·)) = k := fun e h =>
    (card_cliques_succ_of_mem G rfl h).trans (hk e h)
  refine ⟨fun v => ?_, fun e h => ⟨he e h, ?_⟩, ht⟩
  · have hv : ∀ n, (G.cliqueFinset n).filter (v ∈ ·) = (G.cliqueFinset n).filter ({v} ⊆ ·) :=
      fun n => filter_congr fun _ _ => singleton_subset_iff.symm
    have c2 : #((G.cliqueFinset 2).filter ({v} ⊆ ·)) = d :=
      (card_cliques_succ_of_mem G rfl (singleton_mem_cliqueFinset_one G v)).trans (hd v)
    have c3 := card_cliques_double_count_const G {v} (n := 2) (n' := 3) rfl
      fun c hc => hk c (mem_filter.mp hc).1
    have c4 := card_cliques_double_count_const G {v} (n := 3) (n' := 4) rfl
      fun c hc => hm c (mem_filter.mp hc).1
    rw [card_singleton] at c3 c4
    rw [c2] at c3
    rw [hv, hv, hv, c2]
    exact ⟨rfl, c3, c4⟩
  · have hcard := (SimpleGraph.mem_cliqueFinset_iff.mp h).2
    have c4 := card_cliques_double_count_const G e (n := 3) (n' := 4) rfl
      fun c hc => hm c (mem_filter.mp hc).1
    rw [hcard, he e h] at c4
    exact c4

end Cliques

/-! ### Boolean certificates -/

/-- The exact value `2 φ`, the inner product of adjacent vertices. -/
def twoPhi : ZPhi := ⟨0, 2⟩

set_option linter.style.longLine false in
/-- The adjacency mask: bit `120 i + j` is set when vertices `i` and `j` have inner product
`2 φ`. `pairRow` certifies it. -/
def adjMask : ℕ := 0xb3811000000002000000000c0002011e40a000000004000000000a0008006d2240000000010000000008800404d8188000000040000000000a000109700d0000000100000000000c000103a8160000000080000000000880010ac1061000000200000000000c00040d82092000000800000000000a0002074410c00000040000000000088008060b604000000008000000000808080c15c02000000010000000000808020a26a0100000002000000000080804080300108000003201000000000c02040600210000001c00800000000a0802050040400000290200000000088401480080100004c00800000000a00101900100200009400400000003000100e00200080003801000000004400102600408000002810010000000c004089008040000090200200000050002050810020000044400400000022008020c2000020004094000000000808804124000010008148000000002020208218000040002222000000004040400000200128100810010020000c000200001000c4081020020040005000080000080212200440040010002200040004000c018001800804000000a0010008001400c003001010000000c0010002001801400280200800000088010000802010218400402000003000040001008020124800808000000a000200004040400c300100400000440008000001408404202200008000404008000004810802401400010000080802000002220201804800020000202004000000802000001021092010000c0200000040400000201210c00800500800000020100000400c0610200022040000001400000084080d0080000a010000001800000108101600400030010000000c00000202201881000044010000002200000400c0a01001030004000000810000080180c0200205000200000040800010014060400411000800000000c00020020140a40004040800000001200040040481180002020200000002100080080222420010100401009202100100080200000030002000810c0120020004040000000a008002006100c0040002010000004400400800d04080080001400000005000100401608100100001800000000c00101001882200200000c0000000220010001a010c004000022000000000c040002c0218008000081000000000a02000460414010000040800000000888040040a20120000000c00000020208080081140440000001200000040402020022480280000002100000008084000000200108181001202000000302000000400210240800c010000000a800000010040442200210080000044400000400080181800c04000000500100001000100300c0140800000030010000080020028140180200000110010000200040840021201080000000c4000080008048001280210000000502000040010030000c40404000000228000000820020240440800100002028000001040040180281000400010102000002080080420122000200004044000000001003200008000180303000200000000801c0001000024060500080000000200290000400042051100040000000804c0000010008148005001000000040940000020010190000c010000001003800000080200e0002201000000001281000800040260003004000000002902000400080890000a0200000000444400020010050800440800000040040900000220020c1010080000008008140000014004120404020000002002220000048008210202040000000000020000008110b300030200000000000400000040a11e00050800000000000100000022406d0011040000000000400000001884d80005010000000001000000000d09700003010000000000800000001603a8001101000000000200000000061ac1000304000000000800000000092d8200050200000000040000000010c74400110800000000000800000060460b010108000000000010000000c02c15010102000000000020000000a01a26010104fff0000000000000000000000000008a005140c00030200000000000000051008a2210008440000000000000000000904200099000422040000000002481248120004810000000000000000000608080142801014020000000000001082100224400848010000000000000000000380001c0e00007000000007e001c000003800000000000000000080108444002221010800000000000040210128001480806000000000000000000048100081212424800000000020404290000942009000000000000000000084400022108a51000000000000000030200040c0518a0000000000000000000000000000fff00000040c40c00000040c40c0000000000002212210000002212210000000000008128120000008128120000000000001c01c00000001c01c000000000000000000040c40c00000040c40c0000000000002212210000002212210000000000008128120000008128120000000000001c01c00000001c01c0000000

/-- Bit `120 i + j` of the adjacency mask. -/
def adjBit (i j : ℕ) : Bool := Nat.testBit adjMask (120 * i + j)

/-- Row `i` of the pair certificate: the mask records exactly the pairs with inner product `2 φ`,
and every other vertex `j ≠ i` has an inner product with `i` below `2 φ` by the exact sign test. -/
def pairRow (i : ℕ) : Bool :=
  (List.range 120).all fun j =>
    (adjBit i j == decide (dot (vertexN i) (vertexN j) = twoPhi)) &&
      (i == j || decide (dot (vertexN i) (vertexN j) = twoPhi) ||
        positive (sub twoPhi (dot (vertexN i) (vertexN j))))

/-- Rows `lo` to `lo + 14` of the pair certificate, checked as one kernel computation. -/
def pairBlock (lo : ℕ) : Bool := (List.range' lo 15).all pairRow

theorem pairBlock_0 : pairBlock 0 = true := by decide +kernel
theorem pairBlock_15 : pairBlock 15 = true := by decide +kernel
theorem pairBlock_30 : pairBlock 30 = true := by decide +kernel
theorem pairBlock_45 : pairBlock 45 = true := by decide +kernel
theorem pairBlock_60 : pairBlock 60 = true := by decide +kernel
theorem pairBlock_75 : pairBlock 75 = true := by decide +kernel
theorem pairBlock_90 : pairBlock 90 = true := by decide +kernel
theorem pairBlock_105 : pairBlock 105 = true := by decide +kernel

theorem pairRow_eq_true (i : ℕ) (hi : i < 120) : pairRow i = true := by
  have key : ∀ lo, pairBlock lo = true → lo ≤ i → i < lo + 15 → pairRow i = true :=
    fun lo h h1 h2 => List.all_eq_true.mp h i (List.mem_range'_1.mpr ⟨h1, h2⟩)
  rcases (by omega : i < 15 ∨ (15 ≤ i ∧ i < 30) ∨ (30 ≤ i ∧ i < 45) ∨ (45 ≤ i ∧ i < 60) ∨
    (60 ≤ i ∧ i < 75) ∨ (75 ≤ i ∧ i < 90) ∨ (90 ≤ i ∧ i < 105) ∨ (105 ≤ i ∧ i < 120)) with
    h | h | h | h | h | h | h | h
  · exact key 0 pairBlock_0 (Nat.zero_le _) h
  · exact key 15 pairBlock_15 h.1 h.2
  · exact key 30 pairBlock_30 h.1 h.2
  · exact key 45 pairBlock_45 h.1 h.2
  · exact key 60 pairBlock_60 h.1 h.2
  · exact key 75 pairBlock_75 h.1 h.2
  · exact key 90 pairBlock_90 h.1 h.2
  · exact key 105 pairBlock_105 h.1 h.2

/-- The neighbours of vertex `i` in the mask. -/
def maskNbrs (i : ℕ) : List ℕ := (List.range 120).filter fun x => adjBit x i

/-- The count certificate: every vertex has 12 neighbours, every adjacent pair has 5 common
neighbours, and every triangle has 2 common neighbours. -/
def countCheck : Bool :=
  (List.range 120).all fun i =>
    (maskNbrs i).length == 12 &&
      (maskNbrs i).all fun j =>
        ((maskNbrs i).countP fun x => adjBit x j) == 5 &&
          (maskNbrs i).all fun k =>
            !adjBit k j || ((maskNbrs i).countP fun x => adjBit x j && adjBit x k) == 2

theorem countCheck_eq_true : countCheck = true := by decide +kernel

theorem all_range_eq_true {n : ℕ} {p : ℕ → Bool} :
    (List.range n).all p = true ↔ ∀ i < n, p i = true := by
  simp [List.all_eq_true, List.mem_range]

theorem pairCheck_spec (i j : Fin 120) :
    (adjBit i j = true ↔ dot (vertex i) (vertex j) = twoPhi) ∧
      (i ≠ j → dot (vertex i) (vertex j) = twoPhi ∨
        positive (sub twoPhi (dot (vertex i) (vertex j))) = true) := by
  have h' := all_range_eq_true.mp (pairRow_eq_true i i.2) j j.2
  simp only [Bool.and_eq_true, beq_iff_eq, Bool.or_eq_true, decide_eq_true_eq] at h'
  obtain ⟨h1, h2⟩ := h'
  refine ⟨?_, fun hij => ?_⟩
  · rw [h1, decide_eq_true_iff]
    rfl
  · rcases h2 with (h2 | h2) | h2
    · exact absurd (Fin.ext h2) hij
    · exact Or.inl h2
    · exact Or.inr h2

/-- The card of a filter over `Fin 120` is a list count over `0, …, 119`. -/
theorem card_filter_fin (p : ℕ → Bool) :
    #(univ.filter fun x : Fin 120 => p x = true) = (List.range 120).countP p := by
  rw [card_filter, Fin.sum_univ_eq_sum_range (fun x => if p x = true then 1 else 0) 120,
    ← card_filter, List.countP_eq_length_filter, card_def, filter_val, range_val,
    Multiset.range, Multiset.filter_coe, Multiset.coe_card]
  simp

/-! ### The edge graph -/

/-- The edge graph: two vertices are adjacent when their inner product is `2 φ`. -/
def edgeGraph : SimpleGraph (Fin 120) where
  Adj i j := dot (vertex i) (vertex j) = twoPhi
  symm := ⟨fun i j h => by rwa [dot_comm]⟩
  loopless := ⟨fun i h => by
    rw [vertexTable_dot_self _ (vertex_mem i)] at h
    exact absurd h (by decide)⟩

instance : DecidableRel edgeGraph.Adj := fun i j =>
  inferInstanceAs (Decidable (dot (vertex i) (vertex j) = twoPhi))

theorem adjBit_iff (i j : Fin 120) : adjBit i j = true ↔ edgeGraph.Adj i j :=
  (pairCheck_spec i j).1

theorem toReal_twoPhi : toReal twoPhi = 2 * φ := by simp [twoPhi, toReal]

/-- Adjacent vertices are the pairs with inner product `2 φ`. -/
theorem edgeGraph_adj_iff (i j : Fin 120) :
    edgeGraph.Adj i j ↔ ⟪vertexR i, vertexR j⟫ = 2 * φ := by
  change dot (vertex i) (vertex j) = twoPhi ↔ _
  rw [inner_vertexR, ← toReal_twoPhi, toReal_injective.eq_iff]

/-- The largest inner product of distinct vertices is `2 φ`. -/
theorem inner_vertexR_le (i j : Fin 120) (hij : i ≠ j) : ⟪vertexR i, vertexR j⟫ ≤ 2 * φ := by
  rcases (pairCheck_spec i j).2 hij with h | h
  · rw [inner_vertexR, h, toReal_twoPhi]
  · rw [positive_iff, toReal_sub, toReal_twoPhi] at h
    rw [inner_vertexR]
    linarith

theorem inner_vertexR_lt (i j : Fin 120) (hij : i ≠ j) (h : ¬edgeGraph.Adj i j) :
    ⟪vertexR i, vertexR j⟫ < 2 * φ :=
  lt_of_le_of_ne (inner_vertexR_le i j hij) fun h' => h ((edgeGraph_adj_iff i j).mpr h')

/-! ### Counts from the certificates -/

theorem commonNbrs_card_eq_count (b : Finset (Fin 120)) (p : ℕ → Bool)
    (hp : ∀ x : Fin 120, (∀ y ∈ b, edgeGraph.Adj x y) ↔ p x = true) :
    #(commonNbrs edgeGraph b) = (List.range 120).countP p := by
  refine Eq.trans ?_ (card_filter_fin p)
  unfold commonNbrs
  congr 1
  ext x
  simp only [mem_filter, mem_univ, true_and]
  exact hp x

theorem countCheck_spec (i : Fin 120) :
    (List.range 120).countP (fun x => adjBit x i) = 12 ∧
      ∀ j : Fin 120, edgeGraph.Adj j i →
        (List.range 120).countP (fun x => adjBit x j && adjBit x i) = 5 ∧
          ∀ k : Fin 120, edgeGraph.Adj k i → edgeGraph.Adj k j →
            (List.range 120).countP (fun x => (adjBit x j && adjBit x k) && adjBit x i) = 2 := by
  have h := countCheck_eq_true
  rw [countCheck, all_range_eq_true] at h
  have h' := h i i.2
  simp only [Bool.and_eq_true, beq_iff_eq, List.all_eq_true] at h'
  obtain ⟨h12, hj⟩ := h'
  have hmem : ∀ j : Fin 120, edgeGraph.Adj j i → (j : ℕ) ∈ maskNbrs i := by
    intro j hj
    simp only [maskNbrs, List.mem_filter, List.mem_range, j.2, true_and]
    exact (adjBit_iff j i).mpr hj
  refine ⟨by rw [← h12, maskNbrs, List.countP_eq_length_filter], fun j hji => ?_⟩
  obtain ⟨h5, hk⟩ := hj j (hmem j hji)
  refine ⟨by rw [← h5, maskNbrs, List.countP_filter], fun k hki hkj => ?_⟩
  have h2 := hk k (hmem k hki)
  simp only [Bool.or_eq_true, Bool.not_eq_true', beq_iff_eq] at h2
  rcases h2 with h2 | h2
  · exact absurd ((adjBit_iff k j).mpr hkj) (by rw [h2]; decide)
  · rw [← h2, maskNbrs, List.countP_filter]

/-- Every vertex has 12 neighbours. -/
theorem card_commonNbrs_vertex (v : Fin 120) : #(commonNbrs edgeGraph {v}) = 12 := by
  rw [commonNbrs_card_eq_count {v} (fun x => adjBit x v) (fun x => by simp [adjBit_iff])]
  exact (countCheck_spec v).1

/-- Every edge has 5 common neighbours. -/
theorem card_commonNbrs_edge (e : Finset (Fin 120)) (he : e ∈ edgeGraph.cliqueFinset 2) :
    #(commonNbrs edgeGraph e) = 5 := by
  rw [SimpleGraph.mem_cliqueFinset_iff] at he
  obtain ⟨i, j, hij, rfl⟩ := card_eq_two.mp he.2
  have hadj : edgeGraph.Adj i j := he.1 (by simp) (by simp) hij
  rw [commonNbrs_card_eq_count {i, j} (fun x => adjBit x j && adjBit x i)
    (fun x => by simp [adjBit_iff, and_comm])]
  exact ((countCheck_spec i).2 j hadj.symm).1

/-- Every triangle has 2 common neighbours. -/
theorem card_commonNbrs_triangle (t : Finset (Fin 120)) (ht : t ∈ edgeGraph.cliqueFinset 3) :
    #(commonNbrs edgeGraph t) = 2 := by
  rw [SimpleGraph.mem_cliqueFinset_iff, SimpleGraph.is3Clique_iff] at ht
  obtain ⟨i, j, k, hij, hik, hjk, rfl⟩ := ht
  rw [commonNbrs_card_eq_count {i, j, k} (fun x => (adjBit x j && adjBit x k) && adjBit x i)
    (fun x => by simp only [mem_insert, mem_singleton, forall_eq_or_imp, forall_eq,
      Bool.and_eq_true, adjBit_iff]; tauto)]
  exact ((countCheck_spec i).2 j hij.symm).2 k hik.symm hjk.symm

/-! ### Lemma 3, counts (N1) -/

/-- **Clique counts.** The edge graph has 720 edges, and its cliques of sizes 1 to 4 number 120,
720, 1200 and 600. These are the graph's vertices, edges, triangles and four-cliques; `Facets`
identifies the four-cliques with the facets of `P`. -/
theorem faceCounts :
    #edgeGraph.edgeFinset = 720 ∧ #(edgeGraph.cliqueFinset 1) = 120 ∧
      #(edgeGraph.cliqueFinset 2) = 720 ∧ #(edgeGraph.cliqueFinset 3) = 1200 ∧
      #(edgeGraph.cliqueFinset 4) = 600 := by
  obtain ⟨h1, h2, h3, h4, hE⟩ := cliqueCounts_of_commonNbrs edgeGraph card_commonNbrs_vertex
    card_commonNbrs_edge card_commonNbrs_triangle
  rw [Fintype.card_fin] at h1 h2 hE
  omega

/-- **Vertex links.** Each vertex has 12 neighbours and lies in 12 edges, 30 triangles and 20
four-cliques of the edge graph, the numbers of vertices, edges and triangles of its link. -/
theorem vertexLinkCounts (v : Fin 120) :
    edgeGraph.degree v = 12 ∧ #((edgeGraph.cliqueFinset 2).filter (v ∈ ·)) = 12 ∧
      #((edgeGraph.cliqueFinset 3).filter (v ∈ ·)) = 30 ∧
      #((edgeGraph.cliqueFinset 4).filter (v ∈ ·)) = 20 := by
  obtain ⟨h2, h3, h4⟩ := (localCliqueCounts_of_commonNbrs edgeGraph card_commonNbrs_vertex
    card_commonNbrs_edge card_commonNbrs_triangle).1 v
  refine ⟨(card_commonNbrs_singleton edgeGraph v).symm.trans (card_commonNbrs_vertex v), h2,
    by omega, by omega⟩

/-- **Incidences.** Five triangles and five tetrahedra meet at each edge, twenty tetrahedra at
each vertex, and each triangle lies in exactly two tetrahedra. -/
theorem incidenceCounts :
    (∀ e ∈ edgeGraph.cliqueFinset 2, #((edgeGraph.cliqueFinset 3).filter (e ⊆ ·)) = 5 ∧
      #((edgeGraph.cliqueFinset 4).filter (e ⊆ ·)) = 5) ∧
      (∀ v, #((edgeGraph.cliqueFinset 4).filter (v ∈ ·)) = 20) ∧
      (∀ t ∈ edgeGraph.cliqueFinset 3, #((edgeGraph.cliqueFinset 4).filter (t ⊆ ·)) = 2) := by
  obtain ⟨-, he, ht⟩ := localCliqueCounts_of_commonNbrs edgeGraph card_commonNbrs_vertex
    card_commonNbrs_edge card_commonNbrs_triangle
  refine ⟨fun e h => ⟨(he e h).1, by have := (he e h).2; omega⟩,
    fun v => (vertexLinkCounts v).2.2.2, ht⟩

/-- The Euler characteristic of the boundary complex is 0. -/
theorem eulerCharacteristic :
    (#(edgeGraph.cliqueFinset 1) : ℤ) - #(edgeGraph.cliqueFinset 2) +
      #(edgeGraph.cliqueFinset 3) - #(edgeGraph.cliqueFinset 4) = 0 := by
  obtain ⟨_, h1, h2, h3, h4⟩ := faceCounts
  rw [h1, h2, h3, h4]
  norm_num

/-! ### Lemma 3, metric data -/

theorem dist_vertexR_sq (i j : Fin 120) :
    dist (vertexR i) (vertexR j) ^ 2 = 8 - 2 * ⟪vertexR i, vertexR j⟫ := by
  rw [dist_eq_norm, norm_sub_sq_real, norm_vertexR, norm_vertexR]
  ring

theorem two_div_goldenRatio_sq : (2 / φ) ^ 2 = 8 - 4 * φ := by
  rw [div_eq_mul_inv, mul_pow, goldenRatioIdentities.2.2]
  ring

/-- **Edge length.** Adjacent vertices are at distance `2 / φ`; distinct non-adjacent vertices
are farther apart. So the edges are exactly the closest pairs. -/
theorem edgeLength (i j : Fin 120) (hij : i ≠ j) :
    2 / φ ≤ dist (vertexR i) (vertexR j) ∧
      (dist (vertexR i) (vertexR j) = 2 / φ ↔ edgeGraph.Adj i j) := by
  have hpos : 0 < 2 / φ := div_pos two_pos Real.goldenRatio_pos
  have hsq := dist_vertexR_sq i j
  have hle := inner_vertexR_le i j hij
  have key : ∀ d : ℝ, 0 ≤ d → (d = 2 / φ ↔ d ^ 2 = (2 / φ) ^ 2) := fun d hd =>
    ⟨fun h => by rw [h], fun h => (pow_left_inj₀ hd hpos.le two_ne_zero).mp h⟩
  refine ⟨?_, ?_⟩
  · apply le_of_pow_le_pow_left₀ two_ne_zero dist_nonneg
    rw [hsq, two_div_goldenRatio_sq]
    linarith
  · rw [key _ dist_nonneg, hsq, two_div_goldenRatio_sq, edgeGraph_adj_iff]
    constructor <;> intro h <;> linarith

/-- **Radius and vertices.** Every vertex has norm 2, so all vertices lie at distance 2 from the
centre `0`; and the exposed points and the extreme points of the convex hull `P` of `V` are exactly
the 120 points of `V`. -/
theorem exposedVertices :
    (∀ q ∈ vertexSet, ‖q‖ = 2) ∧ (convexHull ℝ vertexSet).exposedPoints ℝ = vertexSet ∧
      (convexHull ℝ vertexSet).extremePoints ℝ = vertexSet ∧
      ((convexHull ℝ vertexSet).extremePoints ℝ).ncard = 120 := by
  have hball : convexHull ℝ vertexSet ⊆ Metric.closedBall 0 2 :=
    convexHull_min (fun q hq => by simp [vertexNorm q hq]) (convex_closedBall 0 2)
  have hexp : vertexSet ⊆ (convexHull ℝ vertexSet).exposedPoints ℝ := by
    intro v hv
    have hv2 := vertexNorm v hv
    refine ⟨subset_convexHull ℝ _ hv, innerSL ℝ v, fun y hy => ?_⟩
    have hy2 : ‖y‖ ≤ 2 := by simpa using hball hy
    have hvv : ⟪v, v⟫ = 4 := by rw [real_inner_self_eq_norm_sq, hv2]; norm_num
    have hcs : ⟪v, y⟫ ≤ 2 * ‖y‖ := by
      have := real_inner_le_norm v y
      rwa [hv2] at this
    rw [innerSL_apply_apply, innerSL_apply_apply, hvv]
    refine ⟨by linarith, fun h => ?_⟩
    have hy2' : ‖y‖ = 2 := by linarith
    have heq : ⟪v, y⟫ = ‖v‖ * ‖y‖ := by rw [hv2, hy2']; linarith
    rw [inner_eq_norm_mul_iff_real, hv2, hy2'] at heq
    exact (smul_right_injective ℍ two_ne_zero heq).symm
  have hext : (convexHull ℝ vertexSet).extremePoints ℝ = vertexSet :=
    extremePoints_convexHull_subset.antisymm (hexp.trans exposedPoints_subset_extremePoints)
  refine ⟨vertexNorm, ?_, hext, by rw [hext, vertexCount]⟩
  exact (exposedPoints_subset_extremePoints.trans hext.subset).antisymm hexp

end ArchonFull600Theory
