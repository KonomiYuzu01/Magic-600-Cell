# Theory round 22, line (d), round 6 (R22D-90, R22D-93, R22D-96): builds no state. It prints the K+ index of the menu
# element (320, 1) of I_a (asserted to lie in K+, so the letter is retained), checks that both products of (61, 15) and
# (61, 44) are the identity, and checks the number of pieces (177,120) and the largest degree in the cell graph of
# rim.cell_pairs (600), used in R22D-90's bound.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got the
# output in runs/round22/r22d96_retained_check.txt, the reviewer's line. Under a CPU minute.
# It writes no file. Research only, headless Linux; exact arithmetic.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d96_retained_check.py
import sys,json,resource
resource.setrlimit(resource.RLIMIT_CPU,(1190,1200))
sys.path[:0]=['research/jumbling/theory','research/jumbling','research/jumbling/fixtures']
import restore as R
import numpy as np
from exact import matmul
C=R.ctx; M=R.menu('I_a')
k=C.kplus.index_of_matrix(R.element(M,320,1)); assert k>=0
g,h=R.element(M,61,15),R.element(M,61,44)
assert R.to_tuple(matmul(h,g))==R.I4 and R.to_tuple(matmul(g,h))==R.I4
lo,up=R.rim.cell_pairs()
degree=np.bincount(lo,minlength=R.rim.NP+1)+np.bincount(up,minlength=R.rim.NP+1)
assert R.rim.NP==177120 and int(degree.max())==600
print(json.dumps(dict(retained_index_320_1=int(k),cycle_products_identity=True,pieces=R.rim.NP,max_cell_graph_degree=int(degree.max()))))
