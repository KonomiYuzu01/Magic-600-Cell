# Export the state at the end of a census record (stage 11) as a pose table, for the scripts that start from a table
# (eb4_close.py, eb4_replay.py and the driver gdrive.py). Prints the record, the table's digest and the off count.
# Usage (from the repository root):
#   python3 -I -B export_table.py <census record json> <out table json>
import sys
from pathlib import Path

REPO = Path.cwd()
sys.path[:0] = [str(REPO / 'research/jumbling/theory'), str(REPO / 'research/jumbling'),
                str(REPO / 'research/jumbling/fixtures'), str(Path(__file__).resolve().parent)]
import buffer as B  # noqa: E402
from screen import load  # noqa: E402

st, M, src = load('record', sys.argv[1])
doc = B.export_pose_table(st, sys.argv[2])
print(src, doc['digest'], int(st.off_lattice_count()))
