#pragma once
#include "json.h"
#include <array>
#include <filesystem>
#include <functional>
#include <span>

namespace sb {
namespace fs = std::filesystem;
constexpr uint32_t Vertices=30480, Stickers=433, Cells=600, Slots=259800;
constexpr uint64_t LabelBytes=uint64_t(Slots)*4;
using Matrix=std::array<float,16>;
void require(bool ok, const std::string& reason);
std::vector<uint8_t> bytes(const fs::path& path);
std::string sha256(std::span<const uint8_t> data);
std::string buildIdentity();
Json readJson(const fs::path& path);
void writeText(const fs::path& path, const std::string& text);
fs::path repoRoot();
fs::path exeDir();
Matrix identity();
void rotate(Matrix& q, int a, int b, double theta);

struct Assets {
    std::vector<float> vertices, centers, frames;
    std::vector<uint32_t> local, offsets, flags, moving, src, dst, invSrc, invDst;
    std::array<float,4> normal{}, planeU{}, planeV{};
    float radius=0;
    double angle=0;
    std::array<std::vector<uint32_t>,2> oracle;
    std::array<std::string,2> oracleHash;
    Json cameras;
    Assets();
    void move(std::vector<uint32_t>& labels, uint64_t completedTurn) const;
    std::vector<uint32_t> samples() const;
};
struct Options {
    std::string scene="w3", runId, inject;
    fs::path out="sb-run";
    double duration=192, preroll=4, turnMs=190;
    bool vsync=false, warp=false, debug=false, geometry=false, selftest=false;
    unsigned msaa=1;
    // run_scene.ps1 replaces the overlays placeholder with the operator's post-run confirmation.
    Json::Object declared{{"frame_generation",false},{"upscaling",false},{"overlays","OPERATOR-CONFIRMATION-PENDING"}};
};
Options options(int argc,char** argv);
struct Turn { uint64_t index; double phase,theta; };
int64_t turnTicks(double ms, int64_t frequency);
Turn turnAt(int64_t now,int64_t start,int64_t duration,double angle);
struct Trace {
    uint64_t frame=0; int64_t qpc=0; uint64_t revision=0;
    bool animated=false; uint64_t turn=0; double phase=0;
    Json json() const;
};
struct Upload { uint64_t frame,revision,resource; };
struct Use { uint64_t frame,requiredRevision,revision,resource,expectedResource; };
struct Copy { uint64_t frame,revision,resource,expectedResource,slot; };
struct LabelRecords { std::vector<Upload> uploads; std::vector<Use> uses; std::vector<Copy> copies; };
Json checkLabels(const Assets& assets,const LabelRecords& records,
                 const std::function<std::span<const uint32_t>(const Copy&)>& copied);
void injectLabels(std::vector<uint32_t>& labels,const std::string& fault);
Json runJson(const Options& opt,int64_t frequency,int64_t start,int64_t stop,
             const std::string& build,const Json& environment,const Json& labels,double vram);
void selftest(const Assets& assets);
int runGpu(const Assets& assets,const Options& opt);
}
