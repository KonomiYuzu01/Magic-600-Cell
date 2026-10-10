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
    for(const char* name:{"wj_probe.exe","draw_vs.dxil","draw_ps.dxil","count_vs.dxil","geometry_cs.dxil","lattice_cs.dxil","overlay_vs.dxil","overlay_ps.dxil"}){
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
Assets::Assets(const Options& options):opt(options){
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
    slotPiece=arrayFile<uint32_t>(root/"assets/slot_piece.u32",Slots);
    cameras=readJson(root/"work/experiments/renderer-sb/cameras.json");require(cameras.at("format").string()=="magic600-sb-cameras-v1"&&cameras.at("cameras").array().size()==3,"bad camera contract");
    require(cameras.at("aspect").number()==1.6,"bad geometry-check aspect");
    for(const auto& camera:cameras.at("cameras").array()){auto q=identity();require(!camera.at("name").string().empty(),"missing camera name");for(const auto& r:camera.at("rotations").array()){require(r.array().size()==3,"bad rotation");rotate(q,int(r.array()[0].integer()),int(r.array()[1].integer()),r.array()[2].number());}}
    straddle.resize(Pieces);
    poses[0].index.resize(Pieces);poses[0].lattice={0};
    auto id=identity();poses[0].rows.assign(id.begin(),id.end());poses[1]=poses[0];
    if(opt.scene=="wj"){
        auto path=root/("research/jumbling/fixtures/wj-"+opt.menu+".json");auto raw=bytes(path);fixture=readJson(path);
        auto ref=readJson(root/("work/experiments/renderer-wj/ref_"+opt.menu+"_index.json"));
        require(sha256(raw)==ref.at("inputs").at("research/jumbling/fixtures/wj-"+opt.menu+".json").string(),"fixture hash differs from immutable reference");
        auto motion=readJson(opt.data/opt.menu/"motion.json");const auto& sw=fixture.at("sweep");
        require(motion.at("dimension").integer()==4&&motion.at("fixture_sha256").string()==sha256(raw),"unsupported motion or stale fixture export");
        auto movingRaw=bytes(opt.data/opt.menu/"moving.i32");
        require(sha256(movingRaw)==sw.at("moved_sha256").string(),"moving set hash mismatch");
        auto pieces=arrayFile<int32_t>(opt.data/opt.menu/"moving.i32",size_t(sw.at("moved").integer()));
        std::vector<bool> inside(Pieces);int32_t last=-1;
        for(auto p:pieces){require(p>last&&p<int32_t(Pieces),"invalid moving piece ids");inside[size_t(p)]=true;last=p;}
        flags.assign(Slots,0);moving.clear();for(uint32_t s=0;s<Slots;++s)if(inside[slotPiece[s]]){flags[s]=1;moving.push_back(s);}
        planeU=vector4(sw.at("plane_u"));planeV=vector4(sw.at("plane_v"));angle=sw.at("angle_deg").number()*std::acos(-1.0)/180;
        poses[0]=poseState("sweep-before");poses[1]=poseState("sweep-after");
        for(const char* stage:{"start","mid","end","sweep-before","sweep-after"})maxPoses=std::max(maxPoses,poseState(stage).count());
        oracle[1]=oracle[0];oracleHash[1]=oracleHash[0];
        const auto& survey=fixture.at("end_survey");auto status=survey.at("status").string();require(status.size()==Cells,"incomplete grip survey");
        for(uint32_t c=0;c<Cells;++c){
            OverlayPoint p{};for(uint32_t r=0;r<4;++r)for(uint32_t col=0;col<4;++col)p.world[r]+=frames[c*16+col*4+r]*normal[col]/radius;
            require(status[c]=='a'||status[c]=='b',"unknown grip status");p.colour=status[c]=='a'?std::array<float,4>{.2f,1,.3f,1}:std::array<float,4>{1,.2f,.2f,1};grips.push_back(p);
        }
        for(const auto& cert:survey.at("certificates").array()){
            auto piece=cert.at("piece").integer();require(piece>=0&&piece<Pieces,"invalid certificate piece");straddle[size_t(piece)]=1;
            for(const char* key:{"point_below_float","point_above_float"}){auto world=vector4(cert.at(key));for(auto& x:world)x/=radius;
                certificates.push_back({world,std::string(key)=="point_below_float"?std::array<float,4>{.7f,.2f,1,1}:std::array<float,4>{1,.85f,.1f,1}});}
        }
    }
    for(auto& state:poses){auto raw=state.raw();for(size_t i=0;i<3;++i)state.hashes[i]=sha256(raw[i]);}
}
void Assets::move(std::vector<uint32_t>& labels,uint64_t completedTurn) const {
    const auto& from=completedTurn%2?invSrc:src;const auto& to=completedTurn%2?invDst:dst;
    std::vector<uint32_t> saved(from.size());for(size_t i=0;i<from.size();++i)saved[i]=labels[from[i]];
    for(size_t i=0;i<from.size();++i)labels[to[i]]=saved[i];
}
std::vector<uint32_t> Assets::samples() const {
    std::vector<uint32_t> out;for(uint32_t g=0;g<Cells*Vertices;g+=4099)out.push_back(g);
    for(uint32_t slot:moving)out.push_back(slot/Stickers*Vertices+offsets[slot%Stickers]);
    if(opt.scene=="wj"){
        std::set<uint32_t> highlighted;
        for(const auto& row:fixture.at("stages").at("end").at("samples").array())highlighted.insert(uint32_t(row.at("piece").integer()));
        for(uint32_t slot=0;slot<Slots;++slot)if(highlighted.contains(slotPiece[slot]))out.push_back(slot/Stickers*Vertices+offsets[slot%Stickers]);
    }
    std::sort(out.begin(),out.end());out.erase(std::unique(out.begin(),out.end()),out.end());return out;
}

PoseState Assets::poseState(const std::string& stage) const {
    bool control=stage.starts_with("lattice-");auto directory=control?opt.data/stage:opt.data/opt.menu/stage;
    auto header=readJson(directory/"header.json");
    require(header.at("format").string()=="magic600-jumbling-render/1"&&header.at("dimension").integer()==4&&header.at("pieces").integer()==Pieces,"unsupported pose header");
    auto count=header.at("poses").integer();require(count>0&&count<=Pieces,"invalid pose table size");
    const auto& files=header.at("files");
    if(!control){
        const auto& ref=stage.starts_with("sweep-")?fixture.at("sweep").at(stage.substr(6)):fixture.at("stages").at(stage);
        require(header.at("digest").string()==ref.at("digest").string()&&files.dump()==ref.at("arrays").dump(),"export differs from fixture stage");
        for(const char* field:{"model_identity","menu_identity","contract_revision"})require(header.at(field).dump()==fixture.at("journal").at(field).dump(),"pose header identity mismatch");
        auto expectedRevision=stage.starts_with("sweep-")?fixture.at("sweep").at("record").integer()+(stage=="sweep-after"):ref.at("records").integer();
        require(header.at("revision").integer()==expectedRevision,"export revision differs from stage");
    }
    PoseState result;result.index=arrayFile<int32_t>(directory/"pose_index.i32",Pieces);
    result.rows=arrayFile<float>(directory/"poses.f32",size_t(count)*16);result.lattice=arrayFile<int32_t>(directory/"pose_lattice.i32",size_t(count));
    const char* names[]={"pose_index.i32","poses.f32","pose_lattice.i32"};auto raw=result.raw();
    for(size_t i=0;i<3;++i){result.hashes[i]=files.at(names[i]).at("sha256").string();require(sha256(raw[i])==result.hashes[i],"pose array hash mismatch");}
    for(auto pid:result.index)require(pid>=0&&pid<count,"pose index outside table");
    auto id=identity();require(std::equal(id.begin(),id.end(),result.rows.begin()),"pose zero must be identity");
    for(auto x:result.rows)require(std::isfinite(x),"nonfinite pose entry");
    for(auto k:result.lattice)require(k>=-1&&k<7200,"invalid lattice index");
    return result;
}
Options options(int argc,char** argv){
    Options opt;
    for(int i=1;i<argc;++i){std::string arg=argv[i];auto next=[&](){require(i+1<argc,"missing value for "+arg);return std::string(argv[++i]);};
        auto num=[&](){auto s=next();size_t n=0;double v=std::stod(s,&n);require(n==s.size()&&std::isfinite(v),"invalid numeric option");return v;};
        if(arg=="--scene")opt.scene=next();else if(arg=="--menu")opt.menu=next();else if(arg=="--data")opt.data=fs::u8path(next());
        else if(arg=="--out")opt.out=fs::u8path(next());else if(arg=="--run-id")opt.runId=next();
        else if(arg=="--duration")opt.duration=num();else if(arg=="--preroll")opt.preroll=num();else if(arg=="--turn-ms")opt.turnMs=num();
        else if(arg=="--vsync")opt.vsync=true;else if(arg=="--warp")opt.warp=true;else if(arg=="--debug-layer")opt.debug=true;
        else if(arg=="--geometry-check")opt.geometry=true;else if(arg=="--lattice-check")opt.lattice=true;else if(arg=="--selftest")opt.selftest=true;
        else if(arg=="--inject")opt.inject=next();else if(arg=="--overlay")opt.overlay=next();
        else if(arg=="--declare"){auto d=next();auto eq=d.find('=');require(eq!=std::string::npos&&eq>0,"--declare expects key=value");auto v=d.substr(eq+1);opt.declared[d.substr(0,eq)]=v=="true"?Json(true):v=="false"?Json(false):Json(v);}
        else throw std::runtime_error("unknown option: "+arg);
    }
    require(opt.scene=="wj"||opt.scene=="w3","--scene must be wj or w3");
    require(opt.menu=="S4"||opt.menu=="I_a"||opt.menu=="I_b","unknown W-J menu");
    require(opt.scene!="wj"||!opt.data.empty(),"W-J requires --data from prepare_wj.py");
    require(opt.duration>0&&opt.duration<=86400&&opt.preroll>=0&&opt.turnMs>0&&opt.turnMs<=10000,"invalid duration, preroll or turn-ms");
    require(opt.inject.empty()||opt.inject=="corrupt-index"||opt.inject=="swap-same-colour"||opt.inject=="stale-pose"||opt.inject=="delay-adoption"||opt.inject=="stale-binding","unknown injected fault");
    require(opt.inject.empty()||opt.scene=="wj","pose faults require W-J");
    require(opt.overlay=="none"||opt.overlay=="grips"||opt.overlay=="certificates"||opt.overlay=="straddles"||opt.overlay=="angle","unknown overlay");
    require(opt.overlay=="none"||opt.scene=="wj","overlays require W-J");
    require(!(opt.geometry&&opt.lattice),"choose one geometry check");
    require(opt.lattice?opt.scene=="wj":true,"lattice check requires W-J data");
    require((!opt.geometry&&!opt.lattice)||opt.inject.empty(),"geometry checks cannot inject faults");
    return opt;
}
int64_t turnTicks(double ms,int64_t frequency){auto ticks=int64_t(std::llround(ms*double(frequency)/1000));require(ticks>0,"turn-ms below QPC resolution");return ticks;}
Turn turnAt(int64_t now,int64_t start,int64_t duration,double angle){require(now>=start&&duration>0,"bad turn clock");int64_t dt=now-start;uint64_t t=uint64_t(dt/duration);double p=double(dt%duration)/double(duration);return {t,p,(t%2?-1:1)*angle*p*p*(3-2*p)};}
Json Trace::json() const{return Json::Object{{"frame",frame},{"qpc",qpc},{"revision",revision},{"turn",turn},{"phase",phase},{"camera",camera}};}

void injectPose(PoseState& state,const PoseState& previous,const Assets& a,const std::string& fault){
    if(fault=="corrupt-index"){auto& pid=state.index[0];pid=(pid+1)%int32_t(state.count());}
    if(fault=="swap-same-colour"){
        std::vector<int32_t> colour(Pieces,-1);std::vector<bool> mixed(Pieces);
        for(uint32_t slot=0;slot<Slots;++slot){auto piece=a.slotPiece[slot];int32_t c=int32_t(a.oracle[0][slot]/Stickers);
            if(colour[piece]<0)colour[piece]=c;else if(colour[piece]!=c)mixed[piece]=true;}
        std::array<int32_t,Cells> first;first.fill(-1);bool swapped=false;
        for(uint32_t piece=0;piece<Pieces&&!swapped;++piece)if(!mixed[piece]){
            auto& previousPiece=first[size_t(colour[piece])];
            if(previousPiece<0)previousPiece=int32_t(piece);
            else if(state.index[size_t(previousPiece)]!=state.index[piece]){std::swap(state.index[size_t(previousPiece)],state.index[piece]);swapped=true;}
        }
        require(swapped,"no same-colour pieces with distinct pose indices");
    }
    if(fault=="stale-pose"){
        bool replaced=false;
        for(uint32_t p=0;p<Pieces;++p){size_t current=size_t(state.index[p])*16,old=size_t(previous.index[p])*16;
            if(!std::equal(state.rows.begin()+current,state.rows.begin()+current+16,previous.rows.begin()+old)){
                std::copy_n(previous.rows.begin()+old,16,state.rows.begin()+current);replaced=true;break;}}
        require(replaced,"no stale table entry to inject");
    }
}
static Json metadata(const LabelRecords& records){
    uint64_t missing=0,late=0,binding=0;std::map<uint64_t,Use> first;std::map<uint64_t,Upload> uploads;
    std::map<uint64_t,std::vector<const Copy*>> copies;std::set<uint64_t> required;
    for(const auto& u:records.uploads)if(!uploads.emplace(u.revision,u).second)++missing;
    for(const auto& u:records.uses){required.insert(u.requiredRevision);first.emplace(u.requiredRevision,u);if(u.revision!=u.requiredRevision)++late;if(u.resource!=u.expectedResource)++binding;}
    for(const auto& c:records.copies){copies[c.revision].push_back(&c);if(c.resource!=c.expectedResource)++binding;}
    for(auto r:required){auto f=first.find(r);auto up=uploads.find(r);auto c=copies.find(r);
        if(up==uploads.end()||c==copies.end()||c->second.size()!=1){++missing;continue;}
        const auto& copy=*c->second.front();
        if(f->second.frame!=up->second.frame)++late;
        if(copy.frame!=f->second.frame||copy.resource!=f->second.resource||copy.expectedResource!=up->second.resource)++binding;
    }
    for(const auto& [r,c]:copies)if(!required.contains(r))++missing;
    Json::Array revisions;for(auto r:required)revisions.emplace_back(r);
    return Json::Object{{"missing",missing},{"late_adoptions",late},{"binding_mismatches",binding},{"required",revisions}};
}
Json checkLabels(const Assets& a,const LabelRecords& records,const std::function<std::span<const uint32_t>(const Copy&)>& copied){
    Json result=metadata(records);uint64_t mismatches=0,hashFailures=0;
    for(const auto& c:records.copies){auto actual=copied(c);const auto& expected=a.oracle[c.revision%2];
        if(actual.size()!=Slots){++mismatches;continue;}
        for(size_t i=0;i<Slots;++i)if(actual[i]!=expected[i])++mismatches;
        if(sha256(std::span(reinterpret_cast<const uint8_t*>(actual.data()),actual.size_bytes()))!=a.oracleHash[c.revision%2])++hashFailures;
    }
    bool pass=!(mismatches||hashFailures||result.at("missing").integer()||result.at("late_adoptions").integer()||result.at("binding_mismatches").integer());
    result["status"]=pass?"pass":"fail";result["copies"]=uint64_t(records.copies.size());result["revisions"]=uint64_t(result.at("required").array().size());
    result["mismatches"]=mismatches;result["hash_failures"]=hashFailures;return result;
}
Json checkPoses(const Assets& a,const LabelRecords& records,const std::function<PoseBytes(const Copy&)>& copied){
    Json result=metadata(records);uint64_t mismatches=0,hashFailures=0;std::set<uint64_t> matched;Json::Array checks;
    for(const auto& c:records.copies){auto actual=copied(c);const auto& expected=a.poses[c.revision%2];auto oracle=expected.raw();bool equal=c.poses==expected.count();Json::Array arrays;
        for(size_t j=0;j<3;++j){auto hash=sha256(actual[j]);bool sameHash=hash==expected.hashes[j],sameElements=actual[j].size()==oracle[j].size();
            uint64_t differences=0;if(sameElements){for(size_t i=0;i<actual[j].size();i+=4)if(std::memcmp(actual[j].data()+i,oracle[j].data()+i,4)!=0)++differences;sameElements=differences==0;}
            else ++differences;
            mismatches+=differences;if(!sameHash)++hashFailures;equal&=sameHash&&sameElements;
            arrays.emplace_back(Json::Object{{"sha256",hash},{"expected_sha256",expected.hashes[j]},{"sha256_equal",sameHash},{"element_equal",sameElements},{"element_mismatches",differences}});
        }
        if(equal)matched.insert(c.revision);
        checks.emplace_back(Json::Object{{"revision",c.revision},{"frame",c.frame},{"resource",c.resource},{"pose_count",c.poses},{"arrays",arrays},{"status",equal?"pass":"fail"}});
    }
    bool pass=!(mismatches||hashFailures||result.at("missing").integer()||result.at("late_adoptions").integer()||result.at("binding_mismatches").integer());
    Json::Array revisions;for(auto r:matched)revisions.emplace_back(r);
    if(Json(revisions).dump()!=result.at("required").dump())pass=false;
    result["status"]=pass?"pass":"fail";result["revisions"]=revisions;result["mismatches"]=mismatches;result["hash_failures"]=hashFailures;result["checks"]=checks;return result;
}
Json runJson(const Options& opt,int64_t frequency,int64_t start,int64_t stop,const std::string& build,const Json& environment,const Json& labels,double vram){
    Json run=Json::Object{{"format","magic600-renderer-run-v1"},{"run_id",opt.runId},{"candidate","s-b"},{"scene",opt.scene},{"overlay",opt.overlay},
        {"qpc_frequency",frequency},{"markers",Json::Object{{"trace_start_qpc",start},{"trace_stop_qpc",stop}}},
        {"presentmon",Json::Object{{"process_id",GetCurrentProcessId()},{"swap_chain","FILL-FROM-CSV"}}},
        {"build",Json::Object{{"build_identity",build}}},{"environment",environment},{"turn_ms",opt.turnMs},{"label_check",labels},{"vram_peak_mb",vram},
        {"camera",Json::Object{{"plane",Json::Array{0,3}},{"step_rad",0.002},{"per","frame"}}}};
    if(opt.scene=="wj")run["fixture"]=opt.menu=="S4"?"wj-s4":opt.menu=="I_a"?"wj-i-a":"wj-i-b";
    if(!opt.inject.empty())run["injected_fault"]=opt.inject;
    return run;
}
void selftest(const Assets& a){
    require(sha256(std::span<const uint8_t>())=="e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","SHA-256 empty input");
    constexpr int64_t freq=10000000,start=123456000000;auto duration=turnTicks(190,freq);
    for(uint64_t t=0;t<30;++t){auto first=turnAt(start+int64_t(t)*duration,start,duration,a.angle);auto mid=turnAt(start+int64_t(t)*duration+duration/2,start,duration,a.angle);
        require(first.index==t&&first.phase==0&&first.theta==0&&mid.phase==.5,"turn arithmetic");require(std::abs(mid.theta-(t%2?-1:1)*a.angle*.5)<1e-12,"smoothstep/inverse");}
    require(a.opt.scene=="wj","pose selftest requires W-J exports");
    for(const std::string fault:{"","corrupt-index","swap-same-colour","stale-pose","delay-adoption","stale-binding"}){
        LabelRecords records;auto bad=a.poses[0];injectPose(bad,a.poses[1],a,fault);
        for(uint64_t revision=0;revision<24;++revision){uint64_t frame=revision*10,id=revision%3+1;bool at=revision==20;
            records.uploads.push_back({frame,revision,id});
            if(at&&fault=="delay-adoption")records.uses.push_back({frame,revision,revision-1,id%3+1,id});
            auto adopted=frame+(at&&fault=="delay-adoption");auto bound=at&&fault=="stale-binding"?id%3+1:id;
            records.uses.push_back({adopted,revision,revision,bound,id});records.copies.push_back({adopted,revision,bound,id,revision,a.poses[revision%2].count()});
        }
        auto result=checkPoses(a,records,[&](const Copy& c){return c.revision==20?bad.raw():a.poses[c.revision%2].raw();});
        require(result.at("status").string()==(fault.empty()?"pass":"fail"),"pose negative control "+fault);
        auto labels=checkLabels(a,records,[&](const Copy& c){return std::span<const uint32_t>(a.oracle[c.revision%2]);});
        if(fault=="stale-pose")require(labels.at("status").string()=="pass","label-only check must miss stale transform");
        std::cout<<"CPU pose control "<<(fault.empty()?"clean":fault)<<": "<<result.at("status").string()<<'\n';
    }
    require(buildIdentity().size()==64,"build identity");std::cout<<"selftest: pass (CPU only, no GPU fault evidence)\n";
}
}
int main(int argc,char** argv){
    try{auto opt=sb::options(argc,argv);sb::Assets a(opt);if(opt.selftest){sb::selftest(a);return 0;}return sb::runGpu(a,opt);}
    catch(const std::exception& error){std::cerr<<"wj_probe: "<<error.what()<<'\n';return 1;}
}
