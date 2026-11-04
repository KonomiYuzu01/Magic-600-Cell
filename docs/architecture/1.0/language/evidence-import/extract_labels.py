#!/usr/bin/env python3
# DT-L1 round 3: labels of research stages 14-19. Read-only on the corpus.
# Run from the repository root; reads Git blobs and rules.txt, refs.txt, ext.txt, rule.txt.
import collections as C,gzip,hashlib,json,os,re,subprocess as S,sys
R="research/jumbling/restoration"
ST=("14","15","16","17","18","19")
def git(*a,i=None):
    return S.run(("git",)+a,input=i,capture_output=True,check=True).stdout
def entries():
    e=[]
    for x in git("ls-tree","-r","-z","HEAD",R).split(b"\0"):
        if not x: continue
        m,p=x.split(b"\t",1); p=p.decode("utf-8")
        if p.split("/")[3][:2] in ST: e.append((p,m.split()[2].decode()))
    return sorted(e)
def blobs(e):
    o=git("cat-file","--batch",i="".join(h+"\n" for _,h in e).encode())
    i=0;r=[]
    for p,h in e:
        j=o.index(b"\n",i);n=int(o[i:j].split()[2]);r.append((p,h,o[j+1:j+1+n]));i=j+2+n
    return r
def load(p,d):
    if p.endswith(".gz"): d=gzip.decompress(d)
    if ".jsonl" in p: return [json.loads(x) for x in d.splitlines() if x.strip()]
    return [json.loads(d)]
def isrec(p):
    return p.endswith((".json",".json.gz",".jsonl",".jsonl.gz"))
KEY=re.compile(r"^(-?\d+|[0-9a-f]{8,}|[A-Za-z]?\d+(_\d+)*|[ds]\d+(-\w+)+)$")
def walk(v,fmt,path,out):
    if isinstance(v,dict):
        g=v.get("format")
        if isinstance(g,str) and g: fmt=g
        for k,x in v.items():
            walk(x,fmt,path+"."+("<k>" if KEY.match(k) else k),out)
    elif isinstance(v,list):
        for x in v: walk(x,fmt,path+"[]",out)
    else: out(fmt,path.lstrip("."),v)
L=r"\w*status|verdict|result|outcome|reason|stop|stop_reason|label|kind|classification"
L+=r"|\w*reachability|severity|all_predicates|profile_join|return|standing|gate|contract"
LEX=re.compile("^("+L+r"|evidence_kind|search|mode)$")
VAL=re.compile(r"^(PASS|FAIL|pass|fail|UNKNOWN|INVALID)\b|not_claimed")
BAD=re.compile(r"^[0-9a-f]{16,}$|^-?\d+$|^\d+:\d+|[\/]|\.(json|npz|md|gz)$")
P=r"scope|limits|not_claimed|evidence_class|explicitly_not_newly_verified|metric_scope"
P+=r"|source_review_scope|\w*note|\w*basis|full_state_replay_evidence|claim|proof_checks"
PRO=re.compile("^("+P+")$")
B=r"replay|fresh|recomputed|rechecked|checked|verified|audit|review|reexecuted|recertified"
B+=r"|reused|exhausted|complete|truncated|uncertain|applied|admissible|accepted|legal|claim"
B+=r"|established|impossible|hypothes|restored|not_a_|_pass$|^passed$|imported|new_search"
B+=r"|count_only|branching_used|cached|dirty|implementation|sound|contradiction|prior_proofs"
BL=re.compile(B+"|full_tree_info")
def last(q): return q.split(".")[-1].replace("[]","")
def cls(q,v):
    k=last(q)
    if isinstance(v,bool): return "B" if BL.search(k) else "P"
    if v is None: return "S" if LEX.match(k) else None
    if not isinstance(v,str): return None
    if PRO.match(k): return "T"
    if VAL.search(v) or LEX.match(k):
        return None if BAD.search(v) or len(v)>80 else "S"
def syn(p):
    a=p.split("/")[3:];t="/".join(a[1:])
    return "path:"+a[0][:2]+"/"+re.sub(r"\d+","#",re.sub(r"\.json(\.gz)?$","",t))
def scan(r,p,T,X,Y,Z):
    top=r.get("format") if isinstance(r,dict) else None
    sy=syn(p)
    def out(f,q,v):
        c=cls(q,v);F=f or sy;Z.add((F,bool(f)))
        if c in ("S","B"):
            t=T.setdefault((F,q,json.dumps(v)),[c,0,set(),bool(f)])
            t[1]+=1;t[2].add(p)
        elif c=="T": X[(F,q)]+=1
        elif c=="P": Y[(last(q),v)]+=1
    walk(r,top if isinstance(top,str) else None,"",out)
def ft(p): return re.search(r"(\.json\.gz|\.ia16\.gz|\.[^./]+)$",p).group(1)
def collect():
    bl=blobs(entries());T={};X=C.Counter();Y=C.Counter();Z=set();bad=[];n=C.Counter()
    for p,h,d in bl:
        n[ft(p)]+=1;n["bytes "+ft(p)]+=len(d)
        if not isrec(p): continue
        try: rs=load(p,d)
        except Exception as x: bad.append([p,repr(x)[:80]]);continue
        n["records"]+=len(rs)
        for r in rs:
            if not(isinstance(r,dict) and isinstance(r.get("format"),str)): n["no top format"]+=1
            scan(r,p,T,X,Y,Z)
    g=C.Counter(k[:2] for k,t in T.items() if t[0]=="S")
    over=sorted(k for k,c in g.items() if c>32)
    T={k:t for k,t in T.items() if k[:2] not in over}
    return bl,T,X,Y,Z,bad,n,over
D="docs/architecture/1.0/language/evidence-import/"
NM=[("claim",dict(P="proved",C="conditional",R="refuted",O="open")),
("scope",dict(I="one finite instance",S="declared search set",D="admitted domain",U="every reachable state")),
("basis",dict(T="theorem",K="certificate",X="exhaustive computation",E="exact computation",M="sample")),
("check",dict(N="none",p="producer",i="independent checker",m="model review")),
("check_mode",dict(t="reused through exact transport",a="fresh exact audit",r="actual replay",
 R="stated: not replayed")),
("job_outcome",dict(c="completed",x="cancelled",u="cut off",e="error",n="not stated"))]
def lines(n):
    with open(D+n,encoding="utf-8") as f:
        return [l.rstrip("\n").split("@@") for l in f if l.strip()]
def expand(c):
    if c in ("rec","none"): return {}
    return {k:m[x] for (k,m),x in zip(NM,c) if x!="-"}
def row(F,k,q,v,c,n,fl,ex,code,rid,ref):
    r=dict(format=F,format_kind=k,field=q,value=v,cls=c,count=n,files=fl,example_path=ex)
    r.update(code=code,rule=rid,note=ref.get(rid,""),mapping=expand(code))
    return r
def rows(T):
    R=[(re.compile(a),b,c) for a,b,c in lines("rules.txt")]
    ref=dict(lines("refs.txt"));o=[]
    for (F,q,v),t in sorted(T.items()):
        s=F+"|"+q+"="+v
        m=next((r for r in R if r[0].search(s)),None)
        k="declared" if t[3] else "synthetic"
        o.append(row(F,k,q,json.loads(v),t[0],t[1],len(t[2]),min(t[2]),
                     m[1] if m else "none",m[2] if m else "",ref))
    for F,q,v,code,rid in lines("ext.txt"):
        k="code" if F.startswith("code:") else "external"
        o.append(row(F,k,q,v,"E",0,0,"",code,rid,ref))
    return o
def twins(bl):
    by={}
    for p,h,b in bl: by.setdefault(p.lower(),[]).append((p,b))
    o=[]
    for v in sorted(by.values()):
        if len(v)<2: continue
        w=open(v[0][0],"rb").read() if os.path.exists(v[0][0]) else None
        o.append(dict(paths=[a for a,_ in v],sizes=[len(b) for _,b in v],
                      working_equals=[a for a,b in v if b==w]))
    return o
def diffs(bl):
    o=[]
    for p,h,b in bl:
        w=open(p,"rb").read() if os.path.exists(p) else None
        if w!=b: o.append(p)
    return o
def amb(o):
    g={}
    for r in o:
        if r["cls"]=="E": continue
        w=str(r["value"]).lower() if r["cls"]=="S" else last(r["field"])+"="+str(r["value"]).lower()
        t=g.setdefault(w,[set(),set(),0])
        t[0].add(r["code"]);t[1].add(r["format"]);t[2]+=r["count"]
    a=[dict(word=w,codes=sorted(t[0]),formats=len(t[1]),count=t[2])
       for w,t in g.items() if len(t[0])>1]
    return sorted(a,key=lambda x:(-len(x["codes"]),-x["formats"],x["word"]))
def totals(o,Z):
    c=[r for r in o if r["cls"] in "SB"]
    t=dict(formats_declared=sum(1 for _,d in Z if d),formats_synthetic=sum(1 for _,d in Z if not d))
    t.update(triples=len(c),strings=sum(r["cls"]=="S" for r in c))
    t.update(booleans=sum(r["cls"]=="B" for r in c))
    t.update(mapped=sum(r["code"] not in ("rec","none") for r in c))
    t.update(recorded=sum(r["code"]=="rec" for r in c),no_rule=sum(r["code"]=="none" for r in c))
    t.update(occurrences=sum(r["count"] for r in c),external_rows=len(o)-len(c))
    return t
def usage(o):
    g={}
    for r in o:
        if r["cls"]=="E": continue
        t=g.setdefault((r["rule"],r["code"]),[0,0,r["note"]]);t[0]+=1;t[1]+=r["count"]
    return [[a,b]+t for (a,b),t in sorted(g.items())]
def tb(h,rs):
    f=lambda r:"| "+" | ".join(str(x) for x in r)+" |"
    return [f(h),f(["---"]*len(h))]+[f(r) for r in rs]+[""]
def doc():
    bl,T,X,Y,Z,bad,n,over=collect();o=rows(T)
    man="".join(p+"\0"+h+"\n" for p,h,_ in bl).encode()
    s=dict(head=git("rev-parse","HEAD").decode().strip(),root=R,stages=list(ST))
    s.update(manifest_sha256=hashlib.sha256(man).hexdigest(),parse_failures=bad)
    s.update(counts=dict(sorted(n.items())))
    d=dict(source=s,rule=open(D+"rule.txt",encoding="utf-8").read())
    d.update(totals=totals(o,Z),rows=o,fields={k:m for k,m in NM},ambiguous=amb(o))
    d.update(rule_usage=usage(o),case_pairs=twins(bl),working_tree_differs=diffs(bl))
    d.update(over_limit=over,prose_fields=[[a,b,c] for (a,b),c in sorted(X.items())])
    d["payload_booleans"]=sorted([k,str(v).lower(),c] for (k,v),c in Y.items())
    return d
def md(d):
    t=d["totals"];s=d["source"];L=["# Corpus labels, DT-L1 round 3",""]
    L+=["Generated by `extract_labels.py`; do not edit. Git HEAD `"+s["head"]+"`.",""]
    L+=["Corpus manifest SHA-256 `"+s["manifest_sha256"]+"`. Full table: `corpus-labels.json`.",""]
    L+=["## Totals",""]+tb(["Measure","Value"],sorted(t.items()))
    L+=["## Corpus",""]+tb(["Item","Value"],sorted(s["counts"].items()))
    L+=["Parse failures: "+str(len(s["parse_failures"])),""]
    L+=["## Rule",""]+d["rule"].splitlines()+[""]
    L+=["## Codes","","Six positions: "+", ".join(d["fields"])+". - unset; rec recorded.",""]
    for k,m in sorted(d["fields"].items()): L+=["- "+k+": "+str(m)]
    L+=["","## Rule usage",""]+tb(["Rule","Code","Rows","Occurrences","Note"],d["rule_usage"])
    a=[[x["word"]," ".join(x["codes"]),x["formats"],x["count"]] for x in d["ambiguous"]]
    L+=["## Same word, different mapping",""]+tb(["Word","Codes","Formats","Count"],a)
    e=[[r["format"],r["field"],r["value"],r["code"],r["note"]] for r in d["rows"] if r["cls"]=="E"]
    L+=["## External and code-declared rows",""]+tb(["Source","Field","Value","Code","Note"],e)
    c=[[" and ".join(x["paths"]),x["sizes"],x["working_equals"]] for x in d["case_pairs"]]
    L+=["## Case pairs at HEAD",""]+tb(["Paths","Sizes","Working file equals"],c)
    p=C.Counter()
    for a,b,n in d["prose_fields"]: p[last(b)]+=n
    L+=["## Prose fields kept word for word",""]+tb(["Last key","Leaves"],sorted(p.items()))
    b=d["payload_booleans"];k=len({x[0] for x in b})
    L+=["Payload booleans: "+str(k)+" keys, "+str(sum(x[2] for x in b))+" leaves.",""]
    return "\n".join(L)
def main():
    d=doc()
    with open(D+"corpus-labels.json","w",encoding="utf-8",newline="\n") as f:
        json.dump(d,f,ensure_ascii=False,indent=1,sort_keys=True)
    with open(D+"corpus-labels.md","w",encoding="utf-8",newline="\n") as f: f.write(md(d))
    print(d["totals"],d["source"]["head"],len(d["ambiguous"]))
main()
