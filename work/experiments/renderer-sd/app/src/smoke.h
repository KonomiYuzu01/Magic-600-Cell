#pragma once
#include "code_layout.h"
#include "native_loader.h"
#include <d3d12.h>
#include <QtQuick/QQuickRhiItem>
#include <QtQuick/QQuickWindow>
#include <QtQuick/QSGSimpleTextureNode>
#include <rhi/qrhi.h>
#include <chrono>
#include <functional>
#include <memory>
#include <mutex>
#include <optional>
#include <set>
#include <string>
#include <vector>

namespace sd {
struct Options {
    QString out, dll;
    std::string device = "qt", route = "import-copy", queue = "same", handover = "tracked", barriers = "legacy";
    std::uint32_t frames = 1200, resizeEvery = 0, verifyEvery = 50, lossAt = 0, timeoutMs = 5000;
    bool debugLayer = false;
    bool probe() const { return handover == "declared" || barriers == "match" || lossAt != 0; }
};
struct CallFailure { std::string call, error; int status; };
struct CallStatus { std::string call; int status; };
struct Stats {
    std::set<std::string> reasons;
    std::vector<CallFailure> failures;
    sa2_device_info info{};
    struct sa2_debug_counts debug{};
    std::string adapter, debugMessages, backBufferFormat, sceneGraphError;
    std::optional<bool> deviceMatches, queueMatches, rhiMatches, qtQueueDeviceMatches;
    bool fallbackDetected = false, attached = false, detached = false, drainConfirmed = false;
    bool stop = false, renderDone = false, resizeRequest = false, resizePending = false, frameEnded = false;
    bool lossTriggered = false, lossNewRhi = false;
    int resizeWidth = 1280, resizeHeight = 720, graphicsApi = -1;
    double dpr = 0;
    std::uint64_t steps = 0, run = 0, warmup = 0, transition = 0, eligible = 0;
    std::uint64_t skippedWarmup = 0, skippedTransition = 0;
    std::uint64_t nativeRequested = 0, nativeCompleted = 0, nativeMismatch = 0;
    std::uint64_t textureRequested = 0, textureCompleted = 0, textureMismatch = 0;
    std::uint64_t compositeRequested = 0, compositeCompleted = 0, compositeVerified = 0, compositeMismatch = 0;
    unsigned resizeRequested = 0, resizeCompleted = 0, slotsUnregistered = 0;
    std::vector<std::uint64_t> transitionFrames;
    std::vector<std::uint32_t> refcounts;
    std::optional<std::uint32_t> deviceRefcount, queueRefcount;
    unsigned initialized = 0, invalidated = 0, graphErrors = 0, lossResumed = 0;
    std::vector<CallStatus> lossStatuses;
    std::optional<int> removedReason;
    std::optional<int> drainResult;
    std::string phase = "not-started";
    std::chrono::steady_clock::time_point lossStarted{};
};
struct GuiAction { bool close, resize, renderDone; int width, height; };

class Harness {
public:
    explicit Harness(Options options) : options(std::move(options)) {}
    Options options;
    Native native;
    QQuickWindow* window = nullptr;
    QRhi* suppliedRhi = nullptr;
    ID3D12Device* suppliedDevice = nullptr;
    ID3D12CommandQueue* suppliedQueue = nullptr;
    void initialized();
    void invalidated();
    void sceneError(const QString& message);
    void renderStep(QRhiCommandBuffer* cb, QRhiTexture* color, QRhiRenderTarget* target, QSize size);
    void afterRendering();
    void afterFrameEnd();
    void rendererGone(QRhi* rhi);
    void setNode(QSGNode* root, QSize size);
    GuiAction guiAction();
    void fail(const std::string& reason);
    void edit(const std::function<void(Stats&)>& function);
    Stats snapshot();
    bool writeResult(bool final);
    int finalCode();
private:
    struct Slot {
        std::uint64_t resource = 0;
        QRhiTexture* texture = nullptr;
        QRhiTextureRenderTarget* target = nullptr; // export-copy only: clears the new slot once
        QRhiRenderPassDescriptor* pass = nullptr;
        QSGTexture* sceneTexture = nullptr;
        bool registered = false;
    };
    struct Readback { QRhiReadbackResult result; bool completed = false; };
    std::mutex mutex_;
    Stats stats_;
    sa2_context* context_ = nullptr;
    QRhi* rhi_ = nullptr;
    QRhi* firstRhi_ = nullptr;
    std::array<Slot, SA2_RING_SLOTS> slots_{};
    QSize ringSize_, itemSize_;
    QSGNode* root_ = nullptr;
    QSGSimpleTextureNode* node_ = nullptr;
    std::uint64_t frame_ = 0, transitionStart_ = 0;
    std::uint32_t generation_ = 0;
    unsigned warmupLeft_ = 0;
    bool rebuilding_ = false, currentEligible_ = false, cleanupStarted_ = false;
    Fields currentFields_{};
    QSize currentSize_;
    std::vector<std::unique_ptr<Readback>> readbacks_;
    sa2_config config() const;
    bool call(const char* name, int status);
    bool createRing(QSize size);
    bool releaseRing();
    void destroyWrappers();
    void teardown(QRhi* rhi);
    void observeLoss();
    void readback(QRhiResourceUpdateBatch* batch, QRhiTexture* texture, bool composite);
    void completeTransition();
    void showSlot(unsigned slot);
};

class RhiItem : public QQuickRhiItem {
public:
    RhiItem(Harness& harness, QQuickItem* parent) : QQuickRhiItem(parent), harness_(harness) {}
    QQuickRhiItemRenderer* createRenderer() override;
private:
    Harness& harness_;
};
class DirectItem : public QQuickItem {
public:
    DirectItem(Harness& harness, QQuickItem* parent) : QQuickItem(parent), harness_(harness) { setFlag(ItemHasContents); }
protected:
    QSGNode* updatePaintNode(QSGNode* old, UpdatePaintNodeData*) override;
private:
    Harness& harness_;
};
}
