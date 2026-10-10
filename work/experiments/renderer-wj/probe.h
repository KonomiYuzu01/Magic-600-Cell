#pragma once
#include "json.h"
#include <array>
#include <filesystem>
#include <functional>
#include <span>

namespace sb {
namespace fs = std::filesystem;
constexpr uint32_t Vertices=30480, Stickers=433, Cells=600, Slots=259800, Pieces=177120;
constexpr uint64_t LabelBytes=uint64_t(Slots)*4, IndexBytes=uint64_t(Pieces)*4;
using Matrix=std::array<float,16>;
void require(bool ok,const std::string& reason);
std::vector<uint8_t> bytes(const fs::path& path);
std::string sha256(std::span<const uint8_t> data);
std::string buildIdentity();
Json readJson(const fs::path& path);
void writeText(const fs::path& path,const std::string& text);
fs::path repoRoot();
fs::path exeDir();
Matrix identity();
void rotate(Matrix& q,int a,int b,double theta);
struct Options {
    std::string scene="wj", menu="S4",runId,inject,overlay="none";
    fs::path out="wj-run",data;
    double duration=192,preroll=4,turnMs=190;
    bool vsync=false,warp=false,debug=false,geometry=false,lattice=false,selftest=false;
    Json::Object declared{{"frame_generation",false},{"upscaling",false},{"overlays","OPERATOR-CONFIRMATION-PENDING"}};
};
Options options(int argc,char** argv);
struct PoseState {
    std::vector<int32_t> index,lattice;
    std::vector<float> rows;
    std::array<std::string,3> hashes;
    uint32_t count() const {return uint32_t(lattice.size());}
    std::array<std::span<const uint8_t>,3> raw() const {
        return {std::span(reinterpret_cast<const uint8_t*>(index.data()),index.size()*4),
                std::span(reinterpret_cast<const uint8_t*>(rows.data()),rows.size()*4),
                std::span(reinterpret_cast<const uint8_t*>(lattice.data()),lattice.size()*4)};
    }
};
struct OverlayPoint {std::array<float,4> world,colour;};
struct Assets {
    std::vector<float> vertices,centers,frames;
    std::vector<uint32_t> local,offsets,flags,moving,src,dst,invSrc,invDst,slotPiece,straddle;
    std::vector<OverlayPoint> grips,certificates;
    std::array<float,4> normal{},planeU{},planeV{};
    float radius=0;
    double angle=0;
    std::array<std::vector<uint32_t>,2> oracle;
    std::array<std::string,2> oracleHash;
    std::array<PoseState,2> poses;
    uint32_t maxPoses=1;
    Json cameras,fixture;
    Options opt;
    explicit Assets(const Options& options);
    PoseState poseState(const std::string& stage) const;
    void move(std::vector<uint32_t>& labels,uint64_t completedTurn) const;
    std::vector<uint32_t> samples() const;
};
struct Turn {uint64_t index;double phase,theta;};
int64_t turnTicks(double ms,int64_t frequency);
Turn turnAt(int64_t now,int64_t start,int64_t duration,double angle);
struct Trace {
    uint64_t frame=0;int64_t qpc=0;uint64_t revision=0;
    bool animated=false;uint64_t turn=0;double phase=0;uint64_t camera=0;
    Json json() const;
};
struct Upload {uint64_t frame,revision,resource;};
struct Use {uint64_t frame,requiredRevision,revision,resource,expectedResource;};
struct Copy {uint64_t frame,revision,resource,expectedResource,slot;uint32_t poses=1;};
struct LabelRecords {std::vector<Upload> uploads;std::vector<Use> uses;std::vector<Copy> copies;};
Json checkLabels(const Assets& assets,const LabelRecords& records,
                 const std::function<std::span<const uint32_t>(const Copy&)>& copied);
using PoseBytes=std::array<std::span<const uint8_t>,3>;
Json checkPoses(const Assets& assets,const LabelRecords& records,
                const std::function<PoseBytes(const Copy&)>& copied);
void injectPose(PoseState& state,const PoseState& previous,const Assets& assets,const std::string& fault);
Json runJson(const Options& opt,int64_t frequency,int64_t start,int64_t stop,
             const std::string& build,const Json& environment,const Json& labels,double vram);
void selftest(const Assets& assets);
int runGpu(const Assets& assets,const Options& opt);
}
