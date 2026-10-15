# Card 1: finite certificates that would make the retained group equal to its invariant bound

Open problem L1 of the jumbling theory, reduced to group theory. The goal is a theorem of the form "if these finitely many machine-checkable facts hold, then $G$ equals the kernel $\mathcal K$ below", with the facts chosen so that a computer can check each one by evaluating explicit words.

## Setting

$G$ is a finite group (the retained legal group of a twisty puzzle), given by $1{,}200$ generators, acting faithfully on a finite set of $259{,}800$ stickers. The stickers fall into $600$ fixed one-sticker centres (fixed by every generator) and $35$ position orbits $o=0,\dots,34$. Orbit $o$ has $N_o$ pieces with $k_o$ stickers each. By coherent transport of frames, the action of $G$ on orbit $o$ is a homomorphism $\rho_o$ into the wreath product
$$W_o=H_o\wr S_{N_o}=H_o^{N_o}\rtimes S_{N_o},$$
where $H_o$ is the frame group: a faithful permutation group of the $k_o$ stickers of one piece. A single $\rho_o$ need not be injective (an element can move one orbit and fix another), but together they give an injective homomorphism
$$G\hookrightarrow \mathcal W=\prod_{o=0}^{34}W_o .$$
Composition of elements of $G$ is chronological (the word $w_1w_2$ applies $w_1$ first).

| Orbit | $k_o$ | $N_o$ | $H_o$ | Orbit | $k_o$ | $N_o$ | $H_o$ |
|---|---:|---:|---|---|---:|---:|---|
| O00 | 2 | 1200 | $C_2$ | O18 | 1 | 7200 | 1 |
| O01 | 1 | 3600 | 1 | O19 | 1 | 7200 | 1 |
| O02 | 1 | 2400 | 1 | O20 | 2 | 7200 | 1 |
| O03 | 2 | 3600 | $C_2$ | O21 | 1 | 7200 | 1 |
| O04 | 1 | 7200 | 1 | O22 | 2 | 3600 | $C_2$ |
| O05 | 1 | 7200 | 1 | O23 | 2 | 7200 | 1 |
| O06 | 5 | 720 | $D_5$ | O24 | 1 | 7200 | 1 |
| O07 | 1 | 7200 | 1 | O25 | 2 | 3600 | $C_2$ |
| O08 | 2 | 3600 | $C_2$ | O26 | 1 | 7200 | 1 |
| O09 | 2 | 7200 | 1 | O27 | 1 | 7200 | 1 |
| O10 | 1 | 7200 | 1 | O28 | 5 | 1440 | $C_5$ |
| O11 | 2 | 3600 | $C_2$ | O29 | 1 | 2400 | 1 |
| O12 | 1 | 7200 | 1 | O30 | 2 | 7200 | 1 |
| O13 | 1 | 7200 | 1 | O31 | 1 | 7200 | 1 |
| O14 | 2 | 7200 | 1 | O32 | 2 | 3600 | $C_2$ |
| O15 | 1 | 2400 | 1 | O33 | 1 | 2400 | 1 |
| O16 | 2 | 7200 | 1 | O34 | 20 | 120 | $A_5$ |
| O17 | 5 | 1440 | $C_5$ | | | | |

For each orbit define $\nu_o:W_o\to \{\pm1\}\times H_o^{\mathrm{ab}}$, $(h_1,\dots,h_{N_o};\sigma)\mapsto(\operatorname{sgn}\sigma,\ \text{image of }h_1h_2\cdots h_{N_o}\text{ in }H_o^{\mathrm{ab}})$. ($H^{\mathrm{ab}}$ is the abelianisation: $C_2^{\mathrm{ab}}=C_2$, $C_5^{\mathrm{ab}}=C_5$, $D_5^{\mathrm{ab}}=C_2$, $A_5^{\mathrm{ab}}=1$.) Let $\mathcal K=\prod_o\ker\nu_o$.

## Given

- **F1 (proved).** Each $\nu_o$ is a homomorphism, and $G\subseteq\mathcal K$ (its value on each of the $1{,}200$ generators was computed exactly to be 0). The index of $\mathcal K$ in $\mathcal W$ is $2^{43}5^2$.
- **C (hypothesis, not a proved fact).** The retained theory assumes, and recorded controller searches support, the following; treat it as a hypothesis and keep it visible in every conclusion that uses it. For the nine orbits $\mathcal I=\{\mathrm{O01},\mathrm{O04},\mathrm{O05},\mathrm{O07},\mathrm{O18},\mathrm{O19},\mathrm{O27},\mathrm{O31},\mathrm{O33}\}$ (all with $k_o=1$), $G$ contains a pure three-cycle on that orbit (an element moving exactly three stickers, all in that orbit) and, for every other position $X$ of the orbit, an element fixing the cycle's first two positions and sending the third to $X$. Under C, $G$ contains $A_{N_o}$ acting on each of these orbits and fixing every other sticker (Lemma 9 of the retained theory).

## Questions

1. **The certificate list.** Give a short list of certificate types, each a property of one explicit element of $G$ or of an explicit finite family, that a computer can check by evaluating sticker permutations, and prove: if every certificate holds, then $G=\mathcal K$. Keep the list small. Prefer certificates of the kind in C (pure cycles plus setups), and state for each orbit type ($H_o=1$; $C_2$; $C_5$; $D_5$; $A_5$) exactly which elements must be exhibited. Pure frame twists (an element changing only the frames of one or two pieces of one orbit and fixing every other sticker) are allowed certificate types.
2. **Reduction.** Can the certificates for an orbit with $H_o\neq1$ be reduced to certificates for its positions plus one frame certificate? Prove the reduction you propose (for example: $A_{N}$ on positions together with one pure frame twist $(h,h^{-1})$ for a generator $h$ of $H$ gives $\ker\nu$ on that orbit; say what extra element $D_5$ and $A_5$ need).
3. **Independence across orbits.** Prove that pure certificates on different orbits combine to the product, so that per-orbit kernels multiply to $\mathcal K$, and state the order of $\mathcal K$ as a formula.
4. **What is not needed.** Identify any certificate type that becomes redundant once the others hold (for example, whether a pure three-cycle plus transitivity of $G$ on the orbit's positions already gives $A_{N_o}$ without a setup for every position, using primitivity or a Jordan-type theorem; state the exact theorem used and its hypotheses).

## Return

A proof blueprint with numbered lemmas and a final theorem "certificates $\Rightarrow G=\mathcal K$". Mark every fact that must be checked by computer as a certificate, with its exact statement. Do not claim that any certificate holds.
