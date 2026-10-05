#include "probe.h"
#include <windows.h>
#include <bcrypt.h>
#include <algorithm>
#include <cstring>
#include <fstream>
#include <iostream>
#include <numeric>
#include <set>

namespace sb {
void require(bool ok,const std::string& reason){if(!ok)throw std::runtime_error(reason);}
std::vector<uint8_t> bytes(const fs::path& path){
    std::ifstream in(path,std::ios::binary|std::ios::ate);
    require(bool(in),"cannot read "+path.filename().string());
    auto n=in.tellg(); require(n>=0,"cannot size input");
    std::vector<uint8_t> data(static_cast<size_t>(n)); in.seekg(0);
    in.read(reinterpret_cast<char*>(data.data()),std::streamsize(data.size()));
    require(bool(in),"short read: "+path.filename().string());return data;
}
std::string hex(std::span<const uint8_t> data){
    const char* digits="0123456789abcdef";std::string out;
    for(uint8_t b:data){out+=digits[b>>4];out+=digits[b&15];}return out;
}
std::string sha256(std::span<const uint8_t> data){
    BCRYPT_ALG_HANDLE alg=nullptr;
    require(BCryptOpenAlgorithmProvider(&alg,BCRYPT_SHA256_ALGORITHM,nullptr,0)>=0,"BCrypt SHA-256 unavailable");
    std::array<uint8_t,32> digest{};
    NTSTATUS status=data.size()>ULONG_MAX?NTSTATUS(-1):BCryptHash(alg,nullptr,0,const_cast<PUCHAR>(data.data()),ULONG(data.size()),digest.data(),ULONG(digest.size()));
    BCryptCloseAlgorithmProvider(alg,0);require(status>=0,"BCryptHash failed");return hex(digest);
}
fs::path exeDir(){std::vector<wchar_t> buf(32768);DWORD n=GetModuleFileNameW(nullptr,buf.data(),DWORD(buf.size()));require(n>0&&n<buf.size(),"cannot locate executable");return fs::path(std::wstring(buf.data(),n)).parent_path();}
fs::path repoRoot(){return fs::path(M600_REPO_ROOT);}
std::string buildIdentity(){
    // Hash the exact executable and every compiled shader, in this fixed order.
    std::vector<uint8_t> all;
    for(const char* name:{"sb_probe.exe","draw_vs.dxil","draw_ps.dxil","count_vs.dxil","geometry_cs.dxil","fog_ps.dxil","outline_vs.dxil","outline_ps.dxil",
        "transparency_vs.dxil","transparency_ps.dxil","sort_keys_cs.dxil","sort_cs.dxil","sort_prefix_cs.dxil","sort_blocks_cs.dxil","sort_indices_cs.dxil","post_vs.dxil","dof_ps.dxil","ao_ps.dxil","ao_blur_ps.dxil","ao_composite_ps.dxil"}){
        auto b=bytes(exeDir()/name);all.insert(all.end(),b.begin(),b.end());
    }
    return sha256(all);
}
Json readJson(const fs::path& path){auto b=bytes(path);return Json::parse(std::string(b.begin(),b.end()));}
void writeText(const fs::path& path,const std::string& text){std::ofstream out(path,std::ios::binary);require(bool(out),"cannot create "+path.filename().string());out.write(text.data(),std::streamsize(text.size()));require(bool(out),"cannot write "+path.filename().string());}
template<class T> std::vector<T> arrayFile(const fs::path& path,size_t count){auto b=bytes(path);require(b.size()==count*sizeof(T),"wrong asset length: "+path.filename().string());std::vector<T> out(count);std::memcpy(out.data(),b.data(),b.size());return out;}
std::vector<uint32_t> uints(const Json& j,uint32_t limit){std::vector<uint32_t> out;for(const auto& v:j.array()){auto n=v.integer();require(n>=0&&uint64_t(n)<limit,"index outside full model");out.push_back(uint32_t(n));}return out;}
std::array<float,4> vector4(const Json& j){require(j.array().size()==4,"expected four components");std::array<float,4> out{};for(int i=0;i<4;++i)out[i]=float(j.array()[i].number());return out;}
Matrix identity(){Matrix q{};for(int i=0;i<4;++i)q[i*5]=1;return q;}
void rotate(Matrix& q,int a,int b,double theta){require(a>=0&&a<4&&b>=0&&b<4&&a!=b,"bad camera rotation plane");double c=std::cos(theta),s=std::sin(theta);for(int j=0;j<4;++j){float x=q[a*4+j],y=q[b*4+j];q[a*4+j]=float(c*x-s*y);q[b*4+j]=float(s*x+c*y);}}
Assets::Assets(){
    auto root=repoRoot();auto manifest=readJson(root/"assets/manifest.json");
    require(manifest.at("puzzle").string()=="600-cell-Full","wrong model profile");
    for(const auto& [name,digest]:manifest.at("files").object())
        require(sha256(bytes(root/name))==digest.string(),"asset SHA-256 mismatch: "+fs::path(name).filename().string());
    auto mesh=readJson(root/"assets/mesh.json");
    require(mesh.at("base_vertices").integer()==Vertices&&mesh.at("base_stickers").integer()==Stickers&&mesh.at("cells").integer()==Cells&&mesh.at("slots").integer()==Slots,"not the full-detail mesh");
    offsets=uints(mesh.at("offsets"),Vertices+1);normal=vector4(mesh.at("normal"));radius=float(mesh.at("normal_length").number());
    require(offsets.size()==Stickers+1&&offsets.front()==0&&offsets.back()==Vertices&&radius>0,"bad mesh ranges");
    vertices=arrayFile<float>(root/"assets/mesh_vertices.f32",Vertices*4);
    local=arrayFile<uint32_t>(root/"assets/mesh_sticker.u32",Vertices);
    centers=arrayFile<float>(root/"assets/mesh_centers.f32",Stickers*4);
    frames=arrayFile<float>(root/"assets/cell_frames.f32",Cells*16);
    for(uint32_t l=0;l<Stickers;++l){require(offsets[l]<offsets[l+1]&&(offsets[l+1]-offsets[l])%3==0,"bad triangle range");for(uint32_t vi=offsets[l];vi<offsets[l+1];++vi)require(local[vi]==l,"sticker/range mismatch");}
    for(const auto* values:{&vertices,&centers,&frames})for(float f:*values)require(std::isfinite(f),"nonfinite geometry asset");
    auto turn=readJson(root/"work/experiments/renderer-sb/workload/turn.json");
    require(turn.at("format").string()=="magic600-sb-turn-v1"&&turn.at("model_id").string()==manifest.at("model_id").string(),"wrong turn identity");
    planeU=vector4(turn.at("plane_u"));planeV=vector4(turn.at("plane_v"));angle=turn.at("angle").number();
    moving=uints(turn.at("moving_slots"),Slots);src=uints(turn.at("move_src"),Slots);dst=uints(turn.at("move_dst"),Slots);
    invSrc=uints(turn.at("inverse_src"),Slots);invDst=uints(turn.at("inverse_dst"),Slots);
    require(src.size()==4600&&src.size()==dst.size()&&src.size()==invSrc.size()&&src.size()==invDst.size()&&moving.size()==4605,"unexpected turn size");
    require(std::set<uint32_t>(src.begin(),src.end()).size()==src.size()&&std::set<uint32_t>(dst.begin(),dst.end()).size()==dst.size(),"turn is not a permutation");
    flags.resize(Slots);for(auto slot:moving)flags[slot]=1;
    oracle[0].resize(Slots);std::iota(oracle[0].begin(),oracle[0].end(),0u);oracle[1]=oracle[0];move(oracle[1],0);
    auto restored=oracle[1];move(restored,1);require(restored==oracle[0],"turn inverse does not restore labels");
    oracleHash[0]=turn.at("labels").at("revision_even_sha256").string();oracleHash[1]=turn.at("labels").at("revision_odd_sha256").string();
    for(int i=0;i<2;++i)require(sha256(std::span(reinterpret_cast<const uint8_t*>(oracle[i].data()),size_t(LabelBytes)))==oracleHash[i],"oracle label SHA-256 mismatch");
    cameras=readJson(root/"work/experiments/renderer-sb/cameras.json");require(cameras.at("format").string()=="magic600-sb-cameras-v1"&&cameras.at("cameras").array().size()==3,"bad camera contract");
    require(cameras.at("aspect").number()==1.6,"bad geometry-check aspect");
    for(const auto& camera:cameras.at("cameras").array()){auto q=identity();require(!camera.at("name").string().empty(),"missing camera name");for(const auto& r:camera.at("rotations").array()){require(r.array().size()==3,"bad rotation");rotate(q,int(r.array()[0].integer()),int(r.array()[1].integer()),r.array()[2].number());}}
}
void Assets::move(std::vector<uint32_t>& labels,uint64_t completedTurn) const {
    const auto& from=completedTurn%2?invSrc:src;const auto& to=completedTurn%2?invDst:dst;
    std::vector<uint32_t> saved(from.size());for(size_t i=0;i<from.size();++i)saved[i]=labels[from[i]];
    for(size_t i=0;i<from.size();++i)labels[to[i]]=saved[i];
}
std::vector<uint32_t> Assets::samples() const {
    std::vector<uint32_t> out;for(uint32_t g=0;g<Cells*Vertices;g+=4099)out.push_back(g);
    for(uint32_t slot:moving)out.push_back(slot/Stickers*Vertices+offsets[slot%Stickers]);
    std::sort(out.begin(),out.end());out.erase(std::unique(out.begin(),out.end()),out.end());return out;
}
Options options(int argc,char** argv){
    Options opt;
    bool featureSeen=false,turnFramesSeen=false,cycleFramesSeen=false;
    for(int i=1;i<argc;++i){std::string arg=argv[i];auto next=[&](){require(i+1<argc,"missing value for "+arg);return std::string(argv[++i]);};
        auto num=[&](){std::string s=next();size_t n=0;double v=std::stod(s,&n);require(n==s.size()&&std::isfinite(v),"invalid numeric option");return v;};
        auto frames=[&](){auto s=next();require(!s.empty()&&std::all_of(s.begin(),s.end(),[](char c){return c>='0'&&c<='9';}),"frame count must be a positive integer");size_t n=0;auto v=std::stoull(s,&n);require(n==s.size(),"invalid frame count");return uint64_t(v);};
        if(arg=="--scene")opt.scene=next();else if(arg=="--out")opt.out=fs::u8path(next());else if(arg=="--run-id")opt.runId=next();
        else if(arg=="--duration")opt.duration=num();else if(arg=="--preroll")opt.preroll=num();else if(arg=="--turn-ms")opt.turnMs=num();
        else if(arg=="--vsync")opt.vsync=true;else if(arg=="--warp")opt.warp=true;else if(arg=="--debug-layer")opt.debug=true;
        else if(arg=="--geometry-check")opt.geometry=true;else if(arg=="--selftest")opt.selftest=true;
        else if(arg=="--msaa"){auto n=num();require(n==1||n==4,"--msaa must be 1 or 4");opt.msaa=unsigned(n);}
        else if(arg=="--inject")opt.inject=next();
        else if(arg=="--snapshot")opt.snapshot=fs::u8path(next());
        else if(arg=="--turn-frames"){opt.turnFrames=frames();turnFramesSeen=true;}
        else if(arg=="--cycle-frames"){opt.cycleFrames=frames();cycleFramesSeen=true;}
        else if(arg=="--declare"){auto d=next();auto eq=d.find('=');require(eq!=std::string::npos&&eq>0,"--declare expects key=value");auto v=d.substr(eq+1);opt.declared[d.substr(0,eq)]=v=="true"?Json(true):v=="false"?Json(false):Json(v);}
        else if(arg=="--feature"){auto name=next();require(name=="none"||name=="no-gaps"||name=="outlines"||name=="transparency"||name=="fog"||name=="dof"||name=="ao"||name=="msaa4","unknown feature: "+name);require(!featureSeen||name==opt.feature,"only one feature may run at once");opt.feature=name;featureSeen=true;}
        else throw std::runtime_error("unknown option: "+arg);
    }
    require(opt.scene=="w1"||opt.scene=="w2"||opt.scene=="w3"||opt.scene=="w4"||opt.scene=="w3f","--scene must be w1, w2, w3, w4 or w3f");
    require(opt.scene=="w3f"||!(turnFramesSeen||cycleFramesSeen),"--turn-frames and --cycle-frames require w3f");
    // run.json stores both counts as JSON numbers (doubles); n <= 2^53 and n >= 2T keep both exact.
    require(opt.turnFrames>=2&&opt.turnFrames<=UINT64_MAX/2&&opt.cycleFrames>0&&opt.cycleFrames<=(uint64_t(1)<<53)&&opt.cycleFrames%(2*opt.turnFrames)==0,"cycle-frames must be a positive multiple of 2T, at most 2^53, with T >= 2");
    if(opt.msaa==4){require(opt.feature=="none"||opt.feature=="msaa4","--msaa 4 cannot be combined with another feature");opt.feature="msaa4";}
    if(opt.feature=="msaa4")opt.msaa=4;
    require(opt.feature=="none"||(!opt.geometry&&opt.inject.empty()),"features cannot be combined with --geometry-check or --inject");
    require(opt.duration>0&&opt.preroll>=0&&opt.turnMs>0&&opt.turnMs<=10000,"invalid duration, preroll or turn-ms");
    require(opt.inject.empty()||opt.inject=="corrupt-label"||opt.inject=="swap-same-colour"||opt.inject=="delay-adoption"||opt.inject=="stale-binding","unknown injected fault");
    require(opt.inject.empty()||opt.scene=="w3"||opt.scene=="w4","label injection requires W3 or W4");
    require(featureImplemented(opt.feature),"feature "+opt.feature+" is not implemented");
    return opt;
}
int64_t turnTicks(double ms,int64_t frequency){auto ticks=int64_t(std::llround(ms*double(frequency)/1000));require(ticks>0,"turn-ms is below QPC resolution");return ticks;}
Turn turnAt(int64_t now,int64_t start,int64_t duration,double angle){require(now>=start&&duration>0,"bad turn clock");int64_t dt=now-start;uint64_t t=uint64_t(dt/duration);double p=double(dt%duration)/double(duration);return {t,p,(t%2?-1:1)*angle*p*p*(3-2*p)};}
bool featureImplemented(const std::string& name){return name=="none"||name=="no-gaps"||name=="fog"||name=="msaa4"||name=="outlines"||name=="transparency"||name=="dof"||name=="ao";}
FrameState frameState(uint64_t k,uint64_t t,uint64_t n,double angle){
    require(t>=2&&t<=UINT64_MAX/2&&n>0&&n%(2*t)==0,"invalid frame sequence");
    uint64_t index=k/t;double p=double(k%t)/double(t);
    return {k%n+1,{index,p,(index%2?-1:1)*angle*p*p*(3-2*p)}};
}
Json effectCheck(std::span<const uint8_t> off,std::span<const uint8_t> on){
    require(!off.empty()&&off.size()==on.size()&&off.size()%4==0,"invalid effect images");
    uint64_t changed=0,pixels=off.size()/4;
    for(size_t i=0;i<off.size();i+=4){bool diff=false;for(size_t c=0;c<4;++c)diff|=std::abs(int(off[i+c])-int(on[i+c]))>2;if(diff)++changed;}
    return Json::Object{{"changed_pixels",changed},{"pixels",pixels},{"status",double(changed)>double(pixels)*0.001?"pass":"fail"}};
}
Json sortCheck(std::span<const SortKey> keys,uint32_t count){
    bool permutation=keys.size()==count,ordered=keys.size()==count;std::vector<bool> seen(count);
    for(size_t i=0;i<keys.size();++i){auto key=keys[i];if(key.id>=count)permutation=false;else if(seen[key.id])permutation=false;else seen[key.id]=true;
        if(!std::isfinite(key.depth)||(i&&keys[i-1].depth<key.depth))ordered=false;}
    return Json::Object{{"status",permutation&&ordered?"pass":"fail"},{"permutation",permutation},{"back_to_front",ordered},{"stickers",count}};
}
Json Trace::json() const {return Json::Object{{"frame",frame},{"qpc",qpc},{"revision",revision},{"turn",animated?Json(turn):Json()},{"phase",animated?Json(phase):Json()},{"camera",camera}};}
void injectLabels(std::vector<uint32_t>& labels,const std::string& fault){
    if(fault=="corrupt-label")labels[0]^=1;
    if(fault=="swap-same-colour"){
        size_t b=1;while(b<labels.size()&&(labels[b]/Stickers!=labels[0]/Stickers||labels[b]==labels[0]))++b;
        require(b<labels.size(),"cannot find equal-colour distinct labels");std::swap(labels[0],labels[b]);
    }
}
Json checkLabels(const Assets& a,const LabelRecords& records,const std::function<std::span<const uint32_t>(const Copy&)>& copied){
    uint64_t mismatches=0,late=0,missing=0,binding=0;std::map<uint64_t,Use> first;
    std::map<uint64_t,std::vector<const Copy*>> copies;std::map<uint64_t,Upload> uploads;std::set<uint64_t> required;
    for(const auto& u:records.uploads){if(!uploads.emplace(u.revision,u).second)++missing;}
    for(const auto& u:records.uses){if(u.requiredRevision)required.insert(u.requiredRevision);first.emplace(u.revision,u);if(u.revision!=u.requiredRevision)++late;if(u.resource!=u.expectedResource)++binding;}
    if(!required.empty()){
        // Whole turns skipped by a slow render are missing, even if the last upload is correct.
        for(uint64_t r=1;r<=*required.rbegin();++r)if(!required.contains(r))++missing;
    }
    for(const auto& c:records.copies){
        copies[c.revision].push_back(&c);if(c.resource!=c.expectedResource)++binding;
        auto values=copied(c);const auto& oracle=a.oracle[c.revision%2];
        if(values.size()!=Slots){++missing;continue;}
        for(size_t i=0;i<Slots;++i)if(values[i]!=oracle[i])++mismatches;
    }
    for(uint64_t r:required){
        auto f=first.find(r);auto up=uploads.find(r);auto c=copies.find(r);
        if(f==first.end()||up==uploads.end()||c==copies.end()||c->second.size()!=1){++missing;continue;}
        const auto& copy=*c->second.front();
        if(f->second.frame>up->second.frame||f->second.frame<up->second.frame)++late;
        if(copy.frame!=f->second.frame||copy.resource!=f->second.resource||copy.expectedResource!=up->second.resource)++binding;
    }
    for(const auto& [r,c]:copies)if(!required.contains(r))++missing;
    bool pass=!(mismatches||late||missing||binding);
    return Json::Object{{"status",pass?"pass":"fail"},{"copies",uint64_t(records.copies.size())},{"revisions",uint64_t(required.size())},
        {"mismatches",mismatches+binding},{"binding_mismatches",binding},{"late_adoptions",late},{"missing",missing},
        {"oracle_sha256",Json::Object{{"even",a.oracleHash[0]},{"odd",a.oracleHash[1]}}}};
}
Json runJson(const Options& opt,int64_t frequency,int64_t start,int64_t stop,const std::string& build,const Json& environment,const Json& labels,double vram){
    bool rotating=opt.scene=="w2"||opt.scene=="w3"||opt.scene=="w3f";
    Json run=Json::Object{{"format","magic600-renderer-run-v1"},{"run_id",opt.runId},{"candidate","s-b"},{"scene",opt.scene},{"feature",opt.feature},
        {"qpc_frequency",frequency},{"markers",Json::Object{{"trace_start_qpc",start},{"trace_stop_qpc",stop}}},
        {"presentmon",Json::Object{{"process_id",GetCurrentProcessId()},{"swap_chain","FILL-FROM-CSV"}}},
        {"build",Json::Object{{"build_identity",build}}},{"environment",environment},{"turn_ms",opt.turnMs},
        {"camera",Json::Object{{"plane",rotating?Json(Json::Array{0,3}):Json()},{"step_rad",rotating?0.002:0},{"per",rotating?"frame":"none"}}},
        {"window",Json::Object{{"display_required",true}}}};
    if(opt.scene=="w3"||opt.scene=="w4"||opt.scene=="w3f")run["label_check"]=labels;
    if(opt.scene=="w3f"){run["turn_frames"]=opt.turnFrames;run["cycle_frames"]=opt.cycleFrames;}
    run["vram_peak_mb"]=vram;
    if(!opt.inject.empty())run["injected_fault"]=opt.inject;
    return run;
}
void selftest(const Assets& a){
    auto edges=edgeMasks(a);
    std::cout<<"selftest: edge counts: features="<<edges.featureEdges<<", diagonals="<<edges.diagonals<<", open="<<edges.openEdges<<", multiple="<<edges.multipleEdges<<", degenerate="<<edges.degenerate<<", duplicates="<<edges.duplicates<<'\n';
    require(edges.featureEdges==3277&&edges.diagonals==5546&&edges.openEdges==819&&edges.multipleEdges==31&&edges.degenerate==3948&&edges.duplicates==761&&edges.nearZero==562,"real-asset feature edges and diagonals");
    require(edges.triangles.size()==Vertices/3&&edges.perSticker.size()==Stickers&&std::all_of(edges.perSticker.begin(),edges.perSticker.end(),[](uint32_t n){return n>=1&&n<=32;}),"every sticker has feature edges");
    auto parse=[](std::initializer_list<const char*> args){std::vector<std::string> s{"sb_probe"};for(auto arg:args)s.emplace_back(arg);std::vector<char*> av;for(auto& arg:s)av.push_back(arg.data());return options(int(av.size()),av.data());};
    for(const char* name:{"none","no-gaps","fog","msaa4","outlines","transparency","dof","ao"})
        for(const char* scene:{"w1","w2","w3","w4","w3f"})require(parse({"--scene",scene,"--feature",name}).feature==name,"feature accepted in every scene");
    parse({"--scene","w3f"});
    auto refuses=[&](std::initializer_list<const char*> args){bool refused=false;try{parse(args);}catch(const std::exception&){refused=true;}require(refused,"invalid options accepted");};
    require(parse({"--msaa","4"}).feature=="msaa4"&&parse({"--feature","msaa4"}).msaa==4,"MSAA synonym");
    parse({"--msaa","4","--feature","none"});parse({"--msaa","4","--feature","msaa4"});
    refuses({"--feature","fog","--feature","no-gaps"});refuses({"--feature","bogus"});
    for(const char* name:{"no-gaps","outlines","transparency","fog","dof","ao","msaa4"}){
        refuses({"--feature",name,"--geometry-check"});refuses({"--feature",name,"--inject","corrupt-label"});
        if(std::string(name)!="msaa4"){refuses({"--feature",name,"--msaa","4"});refuses({"--msaa","4","--feature",name});}
        for(const char* other:{"no-gaps","outlines","transparency","fog","dof","ao","msaa4"})if(std::string(name)!=other)refuses({"--feature",name,"--feature",other});
    }
    for(const char* scene:{"w1","w2","w3","w4"}){refuses({"--scene",scene,"--turn-frames","157"});refuses({"--scene",scene,"--cycle-frames","3140"});}
    for(const char* value:{"0","1","-2","2.5","18446744073709551615"})refuses({"--scene","w3f","--turn-frames",value});
    for(const char* value:{"0","1","3139","-3140","3140.5"})refuses({"--scene","w3f","--cycle-frames",value});
    refuses({"--scene","w3f","--inject","corrupt-label"});
    parse({"--scene","w3f","--turn-frames","2","--cycle-frames","4"});
    refuses({"--scene","w3f","--turn-frames","9007199254740993","--cycle-frames","18014398509481986"});
    refuses({"--scene","w3f","--turn-frames","2","--cycle-frames","9007199254740996"});
    for(uint64_t k=0;k<6280;++k){auto s=frameState(k,157,3140,a.angle);
        require(s.camera==k%3140+1&&s.turn.index==k/157&&std::abs(s.turn.phase-double(k%157)/157)<1e-12,"w3f sequence across two cycles");
        require(s.turn.phase==0?s.turn.theta==0:(s.turn.index%2?s.turn.theta<0:s.turn.theta>0),"w3f direction");
    }
    require(frameState(3139,157,3140,a.angle).camera==3140&&frameState(3140,157,3140,a.angle).camera==1&&frameState(3140,157,3140,a.angle).turn.index==20,"w3f cycle reset and turn continuity");
    auto half=frameState(1,2,4,1);require(half.camera==2&&half.turn.index==0&&half.turn.phase==0.5&&half.turn.theta==0.5,"w3f half turn");
    require(frameState(3,2,4,1).turn.theta==-0.5,"w3f inverse half turn");
    std::vector<uint8_t> off(4000),on=off;on[0]=2;require(effectCheck(off,on).at("status").string()=="fail","effect threshold is strictly above 2/255");
    on[0]=3;require(effectCheck(off,on).at("status").string()=="fail","effect requires more than 0.1 percent");on[4]=3;
    require(effectCheck(off,on).at("changed_pixels").integer()==2&&effectCheck(off,on).at("status").string()=="pass","effect comparison");
    std::vector<SortKey> keys{{8,2},{6,0},{6,1}};require(sortCheck(keys,3).at("status").string()=="pass","sorted permutation fixture");
    keys[2].id=0;require(sortCheck(keys,3).at("status").string()=="fail","sort duplicate id");keys[2]={9,1};require(sortCheck(keys,3).at("status").string()=="fail","sort ascending pair");
    keys[2]={6,3};require(sortCheck(keys,3).at("status").string()=="fail","sort out-of-range id");keys.pop_back();require(sortCheck(keys,3).at("status").string()=="fail","sort missing id");
    keys={{8,2},{6,0},{NAN,1}};require(sortCheck(keys,3).at("status").string()=="fail","sort nonfinite depth");
    require(sha256(std::span<const uint8_t>())=="e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","SHA-256 empty fixture");
    require(buildIdentity().size()==64,"executable and all compiled shaders form the build identity");
    constexpr int64_t freq=10000000,start=123456000000;
    auto d=turnTicks(190,freq);
    for(uint64_t t=0;t<30;++t){
        auto first=turnAt(start+int64_t(t)*d,start,d,a.angle);auto last=turnAt(start+int64_t(t+1)*d-1,start,d,a.angle);auto mid=turnAt(start+int64_t(t)*d+d/2,start,d,a.angle);
        require(first.index==t&&first.phase==0&&first.theta==0&&last.index==t&&last.phase<1&&mid.phase==0.5,"turn boundary/phase arithmetic");
        require(std::abs(mid.theta-(t%2?-1:1)*a.angle*0.5)<1e-12,"turn sign/smoothstep arithmetic");
        require(turnAt(start+int64_t(t+1)*d,start,d,a.angle).index==t+1,"revision boundary arithmetic");
    }
    for(const std::string fault:{"","corrupt-label","swap-same-colour","delay-adoption","stale-binding"}){
        LabelRecords r;std::vector<uint32_t> bad=a.oracle[0];injectLabels(bad,fault);
        for(uint64_t revision=1;revision<=24;++revision){uint64_t frame=revision*20,id=revision%3+1;bool atFault=revision==20;
            r.uploads.push_back({frame,revision,id});
            if(atFault&&fault=="delay-adoption")r.uses.push_back({frame,revision,revision-1,3,3});
            auto adopted=frame+(atFault&&fault=="delay-adoption"?1:0);auto bound=atFault&&fault=="stale-binding"?id%3+1:id;
            r.uses.push_back({adopted,revision,revision,bound,id});r.copies.push_back({adopted,revision,bound,id,revision-1});
        }
        auto result=checkLabels(a,r,[&](const Copy& c)->std::span<const uint32_t>{return c.revision==20&&(fault=="corrupt-label"||fault=="swap-same-colour")?std::span<const uint32_t>(bad):std::span<const uint32_t>(a.oracle[c.revision%2]);});
        require(result.at("status").string()==(fault.empty()?"pass":"fail"),"label checker did not detect "+fault);
        if(fault=="swap-same-colour")require(result.at("mismatches").integer()==2,"same-colour swap must change two integer labels");
    }
    // Independently exercise missing/duplicate copies, skipped revisions and a binding
    // fault whose bytes happen to equal the oracle.
    LabelRecords missing;missing.uploads={{1,1,1}};missing.uses={{1,1,1,1,1}};
    require(checkLabels(a,missing,[&](const Copy&)->std::span<const uint32_t>{return a.oracle[1];}).at("status").string()=="fail","missing copy accepted");
    Options opt;opt.runId="selftest-synthetic-200s";
    Json environment=Json::Object{{"power_source","mains"},{"declared",opt.declared},{"power_mode","max_performance"},{"presenting_adapter","synthetic fixture"},{"presentation_interval",0},{"display",Json::Object{{"width",2560},{"height",1600},{"refresh_hz",60}}},{"backbuffer",Json::Object{{"width",2560},{"height",1600}}},{"vsync",false},{"tearing",true},{"adapter","synthetic fixture"},{"driver","fixture"}};
    Json label=Json::Object{{"status","pass"},{"copies",1052},{"revisions",1052},{"mismatches",0},{"late_adoptions",0},{"missing",0},{"oracle_sha256",Json::Object{{"even",a.oracleHash[0]},{"odd",a.oracleHash[1]}}}};
    auto run=Json::parse(runJson(opt,freq,start,start+200*freq,std::string(64,'a'),environment,label,1).dump());
    require(run.at("format").string()=="magic600-renderer-run-v1"&&run.at("candidate").string()=="s-b"&&run.at("qpc_frequency").integer()==freq&&run.at("markers").at("trace_stop_qpc").integer()==start+200*freq,"run JSON shape");
    require(run.at("presentmon").at("process_id").integer()>0&&run.at("build").at("build_identity").string().size()==64&&run.at("label_check").at("status").string()=="pass","run metadata shape");
    require(run.at("feature").string()=="none"&&run.at("camera").at("plane").array()[1].integer()==3&&run.at("camera").at("step_rad").number()==0.002&&run.at("camera").at("per").string()=="frame"&&std::get<bool>(run.at("window").at("display_required").value),"feature/camera/display run shape");
    opt.scene="w3f";opt.feature="fog";auto cost=runJson(opt,freq,start,start+200*freq,std::string(64,'a'),environment,label,1);
    require(cost.at("scene").string()=="w3f"&&cost.at("feature").string()=="fog"&&cost.at("turn_frames").integer()==157&&cost.at("cycle_frames").integer()==3140&&cost.at("camera").at("per").string()=="frame","w3f run shape");
    auto largest=Json::parse(runJson(parse({"--scene","w3f","--turn-frames","4503599627370496","--cycle-frames","9007199254740992"}),freq,start,start+200*freq,"fixture",environment,label,1).dump());
    require(largest.at("turn_frames").number()==4503599627370496.0&&largest.at("cycle_frames").number()==9007199254740992.0,"largest accepted frame counts are exact in run.json");
    for(const char* scene:{"w1","w2","w3","w4","w3f"}){
        opt.scene=scene;auto record=Json::parse(runJson(opt,freq,start,start+200*freq,"fixture",environment,label,1).dump());const auto& camera=record.at("camera");
        bool rotating=opt.scene=="w2"||opt.scene=="w3"||opt.scene=="w3f";
        require(camera.at("per").string()==(rotating?"frame":"none")&&camera.at("step_rad").number()==(rotating?0.002:0),"camera metadata in every scene");
        require(rotating?(camera.at("plane").array()[0].integer()==0&&camera.at("plane").array()[1].integer()==3):camera.at("plane").null(),"camera plane in every scene");
    }
    auto staticTrace=Trace{}.json();require(staticTrace.at("camera").integer()==0&&staticTrace.at("turn").null()&&staticTrace.at("phase").null(),"stationary trace shape");
    std::ostringstream trace;
    for(uint64_t frame=0;frame<20000;++frame){auto qpc=start+int64_t(frame)*100000;auto t=turnAt(qpc,start,d,a.angle);trace<<Trace{frame,qpc,t.index,true,t.index,t.phase,frame+1}.json().dump()<<'\n';}
    std::istringstream lines(trace.str());std::string line;uint64_t frame=0;int64_t last=start-1;uint64_t previousTurn=0;
    while(std::getline(lines,line)){auto e=Json::parse(line);auto q=e.at("qpc").integer(),rev=e.at("revision").integer(),turn=e.at("turn").integer();double phase=e.at("phase").number();
        require(e.at("frame").integer()==int64_t(frame++)&&q>last&&rev==turn&&phase>=0&&phase<1,"trace JSON shape");
        require(e.at("camera").integer()==int64_t(frame),"trace camera shape");
        require(uint64_t(turn)==previousTurn||uint64_t(turn)==previousTurn+1,"trace turn gap");
        require(std::abs(double(q)-phase*double(d)-double(start+turn*d))<1,"trace phase off QPC clock");last=q;previousTurn=uint64_t(turn);
    }
    require(frame==20000&&last>=start+190*freq,"synthetic trace too short");
    std::cout<<"selftest: assets and oracle digests: ok\nselftest: executable and 19 shaders in build identity: ok\nselftest: clean labels and four injected faults: ok\nselftest: turn arithmetic and synthetic 200 s run/trace shape: ok\nselftest: feature options, w3f, camera/display metadata, effect/sort helpers: ok\nselftest: ok\n";
}
}

int main(int argc,char** argv){
    try{auto opt=sb::options(argc,argv);sb::Assets a;if(opt.selftest){sb::selftest(a);return 0;}return sb::runGpu(a,opt);}
    catch(const std::exception& e){std::cerr<<"sb_probe: "<<e.what()<<'\n';return 1;}
}
