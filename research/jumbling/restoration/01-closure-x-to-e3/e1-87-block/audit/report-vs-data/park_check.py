import json, sys, glob, os, re
d = sys.argv[1]
prefix = sys.argv[2]  # e.g. park-E1-87
logs = sorted(glob.glob(os.path.join(d, prefix + "-*.log")), key=lambda p: int(re.findall(r"-(\d+)\.log$", p)[0]))
print("logs", len(logs))
tot_c = 0; tot_sec = 0; hits = []; recs = 0; maxsec = 0
for p in logs:
    k = int(re.findall(r"-(\d+)\.log$", p)[0])
    lines = [json.loads(l) for l in open(p) if l.strip()]
    cand = [l["candidates"] for l in lines if "candidates" in l]
    secs = [l.get("sec") for l in lines if "sec" in l]
    last = secs[-1] if secs else None
    c = cand[0] if cand else None
    tot_c += c or 0
    tot_sec += last or 0
    tested = [l for l in lines if "n" in l and "of" in l]
    recs += len(tested)
    h = [l for l in tested if l.get("T_on") == 87 and l.get("prot_exact") and l.get("badW") == 87]
    badq = [l for l in lines if "badQ" in l]
    print(k, "candidates", c, "tested", len(tested), "hits87", len(h), "last sec", last,
          "badQ", badq[0]["badQ"] if badq else None, "triple", badq[0].get("triple") if badq else None,
          "a,e,c", (badq[0].get("a"), badq[0].get("e"), badq[0].get("c")) if badq else None,
          "other", [l for l in lines if not any(x in l for x in ("n", "candidates", "badQ", "off", "Y"))][:2])
    hits += [(k, l["n"]) for l in h]
print("total candidates", tot_c, "tested records", recs, "sum of last sec", round(tot_sec, 1))
print("hits", len(hits), hits)
# jsonl check
jl = sorted(glob.glob(os.path.join(d, prefix + "-*.jsonl")))
nrec = 0; nh = 0
for p in jl:
    for l in open(p):
        if not l.strip(): continue
        r = json.loads(l); nrec += 1
        if r.get("T_on") == 87 and r.get("prot_exact") and r.get("badW") == 87: nh += 1
print("jsonl records", nrec, "jsonl hits", nh)
