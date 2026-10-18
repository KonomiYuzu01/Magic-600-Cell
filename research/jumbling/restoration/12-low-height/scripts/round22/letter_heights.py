# Theory round 22, before coverage round 7: the integrator's check of the I_a menu on the tree model. For each of the
# 600 caps and each of the 60 menu items it takes the exact multiplier restore.element(M, c, i), factors it into its
# left and right quaternions (trees.factors) and takes their tree distances (trees.dist), and it counts the pairs of
# distances by kind of item (jumble or retained). ALL_CAPS gives the counts and the CPU time. Every multiplier acts on
# each tree by an isometry, so one letter changes a piece's left height by at most the largest left distance listed.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/letter_heights.py
import sys,time
from collections import Counter
sys.path[:0]=['research/jumbling/theory','research/jumbling']
import restore as RS,trees as T
M=RS.menu('I_a');J=set(RS.jumble_items(M));T0=time.process_time();tot=Counter()
for c in range(600):
 for i in range(len(M.items)):
  l,r=T.factors(RS.element(M,c,i));tot[('jumble' if i in J else 'retained',T.dist(l),T.dist(r))]+=1
print('ALL_CAPS',sorted(tot.items()),'CPU',round(time.process_time()-T0,1),flush=True)
