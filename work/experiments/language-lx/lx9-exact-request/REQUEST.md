# Option B experiment request

Decision **open**, owner. This is a small independent reference, not an A1,
KN3.4, RTE.65, journal or product-format amendment. It uses one common cap
frame; choosing a grip, model/frame identity, UI gesture and operation is the
caller's context. No state, labels, protections or session are touched.

## Bytes and validation

The version prefix is `LX9B1`. ASCII only, exactly one final LF, no whitespace
or extra fields, at most 2048 bytes. Each dyadic token `m:k` means m/2^k:

- m is a canonical decimal integer, with at most 1024 magnitude bits.
- 0<=k<=1074; m must be odd if k>0. Zero is only `0:0`.
- No plus sign, leading zero, negative-zero spelling, decimal float, NaN or
  infinity is admitted on this wire. The binary64 ingress reads raw bits,
  refuses non-finite values, reads finite values exactly and canonicalises
  both signed zeros to `0:0`. These are prototype size limits, not an owner
  decision or a claim of sufficient kernel admission bounds.

```text
LX9B1|C|m0:k0|m1:k1|m2:k2
LX9B1|H|x:0|y:0|z:0
LX9B1|M|menu-set-sha256|menu-order-sha256|index
```

C is the exact Cayley vector omega. R=(I-W)^-1(I+W), equivalently
I+2(W+W^2)/(1+omega.omega). A finite Cayley vector cannot give a half-turn:
its projective quaternion scalar is nonzero. H therefore explicitly names
the half-turn 2aa^T/(a.a)-I. Its wire axis is a nonzero primitive integer
triple, with first nonzero component positive. Encoding a dyadic H axis
clears denominators, divides the gcd and fixes that sign; normalized integer
components must still meet the size bound. Scaled/sign-reversed axes have
the same bytes. Wire parsing refuses noncanonical equivalents.

M is a direct exact menu letter: both digests and a canonical index 0..59 are
required. This branch matters because most I_a elements have irrational
Cayley coordinates and cannot be expressed by a dyadic C request. Tests of
all 60 exact menu targets use their exact matrices and M letters; they do
not pretend that all 60 are dyadic C requests.

## Two free-mode meanings, both tested

`free(data, quantise=False)` returns the exact C/H rotation directly.
`free(data, quantise=True, n=1, dmax=16)` uses the same target but searches A1's
bounded rational candidate grid, with the denominator/numerator tie order.
H candidates are eligible only within one degree of a half-turn. For a C
request this is tested without atan using its rational quaternion scalar
squared <=(1-cos(1 degree))/2 and rational interval bounds; H is eligible
exactly. An unresolved finite precision cap raises an error, never a choice.
Bounds are explicit call arguments. M is refused in free mode.

The fixture omega=(3/8,0,0) separates the variants: direct returns exactly
3/8, while the N=1,D=16 grid returns 1/3. Other fixtures cover exact identity,
an A4 third-turn, half-turns, mixed directions and both sides of the one-degree
half-turn eligibility window. The independent scorer constructs projective
quaternions and compares exact squared inner products instead of matrices.

## The ordered repository menu

The source is `wj.make_menu(ctx,'I_a')`, using `theory/groups.py` and
`TwistMenu`. The captured public record is the `menu` member of
`research/jumbling/fixtures/wj-I_a.json`; `MENU.json` retains every label and
every exact 4x4 matrix in its recorded order. `differential/menu_order.py`
names these same identities:

```text
set:   fddc8532b9184bd1b413c1032b5a7a305dd9fd1f10385d87ba6ec4fd06eb958c
order: 5a0429d2946fbf58b495d8f300e24a3fa164b1d9518ce4a5c3f4ef0a40625c3c
```

The set hash is compact JSON of exact matrix keys sorted lexicographically;
the order hash uses those keys in list order. Labels are not part of either
identity, matching the repository encoding. The checker recomputes both by
two serialization routes, verifies all rotations, recovers the common pole
from the group average, converts to the A1 cap frame and checks exact lifting
back to all original 4x4 matrices. The other route uses a different column
of the fixed-line projector and reconstructs projective quaternions.

For a C/H **direct** menu request, exact non-membership is refused. M names
an exact member under both identities. For a C/H **drag**, every ordered
candidate is scored by squared Frobenius distance, equivalently maximising
trace(R^T M). The smallest index wins every exact tie. `bound_request` returns
the selected index, complete tie list, exact squared distance, both menu
identities, operation and request digest. A bare index is not the bound output.
The oracle instead maximises squared normalized quaternion dot products in
Q(sqrt(5)). The primary scalar signs use the rational field norm; the oracle
compares against an integer square root independently. Neither uses a float
tolerance.

The quarter-turn C=(1,0,0) is an exact non-member and has four nearest menu
indices: 15,29,40,43. Direct refuses it; drag chooses 15. Reversing the menu
preserves the set hash but changes the order hash to
`1a87a4209a933f47a568eac9e881697614bf53e1f7411e817f5764a3a5f92a78`.
The reversed drag chooses index 16, the old index 43, so the rotation changes.
An old M request is refused under the reordered binding. This distinguishes
an order identity from the existing order-insensitive set identity.

`menu_self.tsv` checks all 60 self-selections. `option_b.json` checks eleven
C/H inputs through both free variants and both menu operations, including
exact ties and half-turns. Reconstructed copies of identical request bytes
give identical bound choices under the same operation, variant, bounds and
menu identities. `validation.tsv` covers malformed, noncanonical, overbound
and non-finite inputs as well as signed zero, subnormal and largest finite
binary64 ingress.

What stays outside: **identical gestures do not imply identical omega bytes
unless the producer of omega is specified too**. This prototype does not
specify that producer, world-frame projection, a production parser budget,
unbounded precision costs or a migration/replay format. The two free-mode
meanings are alternatives for the owner; neither is silently selected.
