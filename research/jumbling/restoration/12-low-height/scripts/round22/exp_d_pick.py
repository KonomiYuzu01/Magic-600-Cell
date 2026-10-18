# Round 22, line (c), experiment D: from a transport_census.py record, the unit's strict zero-Bad word (written as a
# step word) or else its entry transport word as the core of a closure ({"P": [], "Q": word}).
# Usage: exp_d_pick.py <census json> <unit name> <step out> <core out>
import json
import sys
from pathlib import Path

for out in sys.argv[3:5]:
    Path(out).unlink(missing_ok=True)  # never leave the output of an earlier call
rec = json.loads(Path(sys.argv[1]).read_text())
res = rec['units'][sys.argv[2]]
zb, sb = res.get('zero_bad'), res.get('small_bad')
print(json.dumps({'unit': sys.argv[2], 'result': res.get('result'),
                  'zero_bad': {k: v for k, v in zb.items() if k != 'word'} if isinstance(zb, dict) else zb,
                  'small_bad': {k: v for k, v in sb.items() if k != 'word'} if isinstance(sb, dict) else sb}))
if isinstance(zb, dict) and zb['strict']:
    Path(sys.argv[3]).write_text(json.dumps({'word': zb['word']}) + '\n')
elif isinstance(sb, dict) and sb['W_ok']:
    Path(sys.argv[4]).write_text(json.dumps({'P': [], 'Q': sb['word']}) + '\n')
