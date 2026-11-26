import sys, re, statistics as st
lines = open(sys.argv[1], encoding="utf-8", errors="replace").read().splitlines()
skip = int(sys.argv[2]) if len(sys.argv) > 2 else 5
key = -1
data = {}
order = []
for l in lines:
    m = re.match(r"TIMING (\S+) (\S+)", l)
    if not m: continue
    tag, v = m.group(1), m.group(2)
    if tag == "key":
        key = int(v); continue
    if key < skip: continue
    try: v = float(v)
    except: continue
    if tag not in data: order.append(tag); data[tag] = {}
    data[tag].setdefault(key, []).append(v)
n = len({k for t in data for k in data[t]})
print(f"keys measured: {n}")
for tag in order:
    per = [sum(v) for v in data[tag].values()]
    per.sort()
    calls = sum(len(v) for v in data[tag].values()) / max(1, len(data[tag]))
    p = lambda q: per[round((len(per)-1)*q)]
    print(f"{tag:34s} calls/key={calls:4.1f}  p50={p(0.5):8.3f}  p90={p(0.9):8.3f}  max={p(1):8.3f}")
