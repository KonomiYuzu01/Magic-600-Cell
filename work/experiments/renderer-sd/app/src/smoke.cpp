#include "smoke.h"
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QSaveFile>
#include <QtQuick/QSGRendererInterface>
#include <QtQuick/qsgtexture_platform.h>
#include <rhi/qrhi_platform.h>
#include <dxgi1_6.h>
#include <wrl/client.h>
#include <algorithm>
#include <cstring>
#include <iostream>
#include <stdexcept>

namespace sd {
using Microsoft::WRL::ComPtr;
namespace {
std::uint64_t handle(const void* pointer) { return reinterpret_cast<std::uint64_t>(pointer); }
bool sameDevice(IUnknown* a, IUnknown* b) {
    ComPtr<IUnknown> first, second;
    return a && b && SUCCEEDED(a->QueryInterface(IID_PPV_ARGS(&first)))
        && SUCCEEDED(b->QueryInterface(IID_PPV_ARGS(&second))) && first.Get() == second.Get();
}
int legacyState(int state) {
    switch (state) {
    case SA2_STATE_RENDER_TARGET: return D3D12_RESOURCE_STATE_RENDER_TARGET;
    case SA2_STATE_COPY_SOURCE: return D3D12_RESOURCE_STATE_COPY_SOURCE;
    case SA2_STATE_PIXEL_SHADER_RESOURCE: return D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE;
    default: return D3D12_RESOURCE_STATE_COMMON;
    }
}
QJsonValue number(std::uint64_t value) { return QJsonValue(static_cast<qint64>(value)); }
QJsonArray numbers(const auto& values) {
    QJsonArray result;
    for (auto value : values) result.append(QJsonValue(static_cast<qint64>(value)));
    return result;
}
void optionalBool(QJsonObject& object, const char* name, std::optional<bool> value) {
    if (value) object.insert(name, *value);
}
}

void Harness::edit(const std::function<void(Stats&)>& function) {
    std::lock_guard lock(mutex_);
    function(stats_); // Only plain observation data; no rendering or DLL calls under this lock.
}
Stats Harness::snapshot() { std::lock_guard lock(mutex_); return stats_; }
void Harness::fail(const std::string& reason) {
    edit([&](Stats& s) { s.reasons.insert(reason); if (!s.lossTriggered) s.stop = true; });
}
bool Harness::call(const char* name, int status) {
    if (snapshot().lossTriggered) edit([&](Stats& s) { s.lossStatuses.push_back({name, status}); });
    if (status == SA2_OK) return true;
    char error[1024]{};
    native.fn_sa2_last_error(context_, error, sizeof(error));
    edit([&](Stats& s) { s.failures.push_back({name, error, status}); });
    fail(status == SA2_E_DEVICE_REMOVED ? "device-removed" : "sa2-call-failed");
    return false;
}
sa2_config Harness::config() const {
    const int state = options.route == "import-direct" ? SA2_STATE_PIXEL_SHADER_RESOURCE : SA2_STATE_COPY_SOURCE;
    return {sizeof(sa2_config), options.route == "rhi-upload" || options.queue == "same" ? SA2_QUEUE_SAME : SA2_QUEUE_OWN,
        options.barriers == "legacy" ? SA2_BARRIERS_LEGACY : SA2_BARRIERS_MATCH_GODOT,
        state, options.handover == "declared" ? SA2_STATE_RENDER_TARGET : state, options.timeoutMs, 1};
}

void Harness::initialized() {
    try {
        // The threaded loop emits sceneGraphInitialized before the window publishes its QRhi.
        auto* rif = window->rendererInterface();
        rhi_ = static_cast<QRhi*>(rif->getResource(window, QSGRendererInterface::RhiResource));
        const auto previous = snapshot();
        edit([](Stats& s) { ++s.initialized; });
        if (previous.lossTriggered) {
            edit([&](Stats& s) {
                s.reasons.insert("loss-reinitialized");
                s.lossNewRhi = s.lossNewRhi || rhi_ != firstRhi_;
                if (s.lossNewRhi) s.reasons.insert("loss-new-rhi");
            });
            return; // Never attach a second context after the removal probe.
        }
        const auto api = rif->graphicsApi();
        const auto dpr = window->effectiveDevicePixelRatio();
        edit([&](Stats& s) { s.graphicsApi = int(api); s.dpr = dpr; });
        if (api != QSGRendererInterface::Direct3D12) { fail("backend-not-d3d12"); return; }
        if (dpr != 1.0) { fail("dpr-not-1"); return; }
        if (!rhi_) { fail("rhi-create-failed"); return; }
        firstRhi_ = rhi_;
        const auto* handles = static_cast<const QRhiD3D12NativeHandles*>(rhi_->nativeHandles());
        auto* device = static_cast<ID3D12Device*>(rif->getResource(window, QSGRendererInterface::DeviceResource));
        auto* queue = static_cast<ID3D12CommandQueue*>(rif->getResource(window, QSGRendererInterface::CommandQueueResource));
        if (!device || !queue || !handles) { fail("identity-mismatch"); return; }
        // COM identity only: two interface pointers to one device need not be equal.
        bool deviceMatches = sameDevice(device, static_cast<IUnknown*>(handles->dev));
        bool queueMatches = queue == handles->commandQueue;
        if (options.device != "qt") deviceMatches = deviceMatches && sameDevice(device, suppliedDevice);
        if (options.device == "from-rhi") {
            queueMatches = queueMatches && queue == suppliedQueue;
            edit([&](Stats& s) { s.queueMatches = queueMatches; s.rhiMatches = rhi_ == suppliedRhi; });
        }
        edit([&](Stats& s) {
            s.deviceMatches = deviceMatches;
            s.fallbackDetected = !deviceMatches || !queueMatches || (s.rhiMatches && !*s.rhiMatches);
        });
        if (!deviceMatches || !queueMatches || (options.device == "from-rhi" && rhi_ != suppliedRhi)) {
            fail("identity-mismatch"); return;
        }
        ComPtr<IDXGIFactory6> factory;
        ComPtr<IDXGIAdapter1> adapter;
        if (FAILED(CreateDXGIFactory2(0, IID_PPV_ARGS(&factory)))
            || FAILED(factory->EnumAdapterByLuid(device->GetAdapterLuid(), IID_PPV_ARGS(&adapter))))
            throw std::runtime_error("matching adapter lookup failed");
        DXGI_ADAPTER_DESC1 description{};
        if (FAILED(adapter->GetDesc1(&description))) throw std::runtime_error("adapter description failed");
        sa2_device_info info{};
        info.struct_size = sizeof(info);
        if (!call("sa2_probe", native.fn_sa2_probe(handle(device), handle(queue), handle(adapter.Get()), &info))) return;
        const auto adapterName = QString::fromWCharArray(description.Description).toStdString();
        edit([&](Stats& s) {
            s.info = info; s.adapter = adapterName;
            if (options.device == "from-device") s.qtQueueDeviceMatches = info.queue_device_matches != 0;
        });
        if (!info.queue_device_matches || !info.adapter_matches_device || info.queue_type != D3D12_COMMAND_LIST_TYPE_DIRECT) {
            fail("identity-mismatch"); return;
        }
        if (options.debugLayer && !info.debug_layer) { fail("debug-layer-unavailable"); return; }
        const auto settings = config();
        if (!call("sa2_attach", native.fn_sa2_attach(handle(device), handle(queue), &settings, &context_))) return;
        edit([](Stats& s) { s.attached = true; });
    } catch (const std::exception&) { fail("exception"); }
}

bool Harness::createRing(QSize size) {
    if (size.width() < SA2_MIN_TEXTURE_PX || size.height() < SA2_MIN_TEXTURE_PX) {
        fail("texture-create-failed"); return false;
    }
    ringSize_ = size;
    if (options.route == "rhi-upload") return true;
    const auto settings = config();
    for (unsigned k = 0; k < slots_.size(); ++k) {
        auto& slot = slots_[k];
        if (options.route == "export-copy") {
            slot.texture = rhi_->newTexture(QRhiTexture::RGBA8, size, 1,
                QRhiTexture::RenderTarget | QRhiTexture::UsedAsTransferSource);
            if (!slot.texture->create()) { fail("texture-create-failed"); return false; }
            slot.target = rhi_->newTextureRenderTarget(QRhiTextureRenderTargetDescription(QRhiColorAttachment(slot.texture)));
            slot.pass = slot.target->newCompatibleRenderPassDescriptor();
            slot.target->setRenderPassDescriptor(slot.pass);
            if (!slot.target->create()) { fail("texture-create-failed"); return false; }
            slot.resource = slot.texture->nativeTexture().object;
        } else {
            if (!call("sa2_create_texture", native.fn_sa2_create_texture(context_, size.width(), size.height(),
                settings.state_before_write, &slot.resource))) return false;
            if (options.route == "import-direct") {
                slot.sceneTexture = QNativeInterface::QSGD3D12Texture::fromNative(
                    reinterpret_cast<void*>(slot.resource), D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE, window, size);
                if (!slot.sceneTexture) { fail("texture-create-failed"); return false; }
            } else {
                slot.texture = rhi_->newTexture(QRhiTexture::RGBA8, size, 1, QRhiTexture::UsedAsTransferSource);
                if (!slot.texture->createFrom(QRhiTexture::NativeTexture{slot.resource, legacyState(settings.state_before_write)})) {
                    fail("texture-create-failed"); return false;
                }
            }
        }
        if (!call("sa2_register_slot", native.fn_sa2_register_slot(context_, k, slot.resource,
            size.width(), size.height(), options.route == "export-copy" ? 1 : 0))) return false;
        slot.registered = true;
    }
    warmupLeft_ = options.route == "export-copy" ? SA2_RING_SLOTS : 0;
    return true;
}

void Harness::destroyWrappers() {
    for (auto& slot : slots_) {
        delete slot.target; slot.target = nullptr;
        delete slot.pass; slot.pass = nullptr;
        delete slot.texture; slot.texture = nullptr;
        delete slot.sceneTexture; slot.sceneTexture = nullptr;
    }
}
bool Harness::releaseRing() {
    const int status = native.fn_sa2_drain(context_, options.timeoutMs);
    const bool confirmed = status == SA2_OK;
    edit([&](Stats& s) { s.drainResult = status; s.drainConfirmed = confirmed; });
    // A removed device makes release safe, but is never a confirmed drain.
    if (!call("sa2_drain", status) && status != SA2_E_DEVICE_REMOVED) return false;
    for (unsigned k = 0; k < slots_.size(); ++k) {
        auto& slot = slots_[k];
        if (slot.registered && call("sa2_unregister_slot", native.fn_sa2_unregister_slot(context_, k))) {
            slot.registered = false;
            edit([](Stats& s) { ++s.slotsUnregistered; });
        }
        if (slot.resource && options.route != "export-copy" && !slot.registered) {
            std::uint32_t count = 0;
            if (call("sa2_release_texture", native.fn_sa2_release_texture(context_, slot.resource, &count))) {
                slot.resource = 0;
                edit([&](Stats& s) { s.refcounts.push_back(count); });
                if (count != 0) fail("refcount-nonzero");
            }
        } else if (!slot.registered) slot.resource = 0;
    }
    return std::none_of(slots_.begin(), slots_.end(), [](const Slot& slot) { return slot.registered || slot.resource; });
}

void Harness::completeTransition() {
    if (!rebuilding_ || warmupLeft_) return;
    rebuilding_ = false;
    edit([&](Stats& s) {
        if (s.resizePending) {
            ++s.resizeCompleted;
            s.transitionFrames.push_back(frame_ - transitionStart_ + 1);
            s.resizePending = false;
        }
    });
}

void Harness::showSlot(unsigned slot) {
    auto* texture = slots_[slot].sceneTexture;
    if (!root_ || !texture) return;
    if (!node_) {
        node_ = new QSGSimpleTextureNode;
        node_->setOwnsTexture(false);
        node_->setFiltering(QSGTexture::Nearest);
        root_->appendChildNode(node_);
    }
    node_->setTexture(texture);
    node_->setRect(0, 0, itemSize_.width(), itemSize_.height());
    node_->markDirty(QSGNode::DirtyMaterial);
}
void Harness::setNode(QSGNode* root, QSize size) {
    root_ = root; itemSize_ = size;
    // Sync precedes beforeRendering, so choose the slot of the next render step.
    showSlot(unsigned((frame_ + 1) % SA2_RING_SLOTS));
}

void Harness::renderStep(QRhiCommandBuffer* cb, QRhiTexture* color, QRhiRenderTarget* target, QSize size) {
    currentEligible_ = false;
    try {
        // Completion callbacks have returned before this render-thread step.
        std::erase_if(readbacks_, [](const auto& ticket) { return ticket->completed; });
        auto observations = snapshot();
        if (observations.lossTriggered) {
            const bool newRhi = window->rhi() != firstRhi_;
            edit([&](Stats& s) {
                ++s.steps; ++s.lossResumed;
                s.lossNewRhi = s.lossNewRhi || newRhi;
                s.reasons.insert("loss-render-resumed");
                if (s.lossNewRhi) s.reasons.insert("loss-new-rhi");
            });
            observeLoss();
            return;
        }
        if (observations.stop || !context_ || cleanupStarted_ || observations.run >= options.frames) return;
        const auto dpr = window->effectiveDevicePixelRatio();
        edit([&](Stats& s) { s.dpr = dpr; });
        if (dpr != 1.0) { fail("dpr-not-1"); return; }
        ++frame_;
        edit([](Stats& s) { ++s.steps; s.frameEnded = false; });
        if (options.route != "rhi-upload"
            && !call("sa2_signal_godot_free", native.fn_sa2_signal_godot_free(context_, frame_ - 1))) return;
        if (options.route == "import-direct") size = itemSize_;
        if (ringSize_ != size) {
            const bool existed = !ringSize_.isEmpty();
            if (existed) {
                // No render pass is open here. finish submits partial work and completes callbacks.
                rhi_->finish();
                if (node_) { root_->removeChildNode(node_); delete node_; node_ = nullptr; }
                destroyWrappers();
                if (!releaseRing()) return;
                generation_ = (generation_ + 1) % 4096;
            }
            rebuilding_ = true;
            if (!observations.resizePending) transitionStart_ = frame_;
            if (!createRing(size)) return;
            // Qt allocates render targets without zeroing. The item's new color buffer (shown on this
            // step) and new export slots (read by warm-up) are cleared once before anything reads them,
            // with the optimized clear values Qt declares for its render targets (zero color, depth 1).
            const QRhiDepthStencilClearValue depthStencil(1.0f, 0);
            if (target) { cb->beginPass(target, Qt::transparent, depthStencil); cb->endPass(); }
            for (const auto& slot : slots_)
                if (slot.target) { cb->beginPass(slot.target, Qt::transparent, depthStencil); cb->endPass(); }
            edit([](Stats& s) { ++s.transition; ++s.skippedTransition; });
            if (options.route == "import-direct") showSlot(unsigned(frame_ % SA2_RING_SLOTS));
            completeTransition();
            return;
        }
        if (observations.resizePending && !rebuilding_) {
            edit([](Stats& s) { ++s.transition; ++s.skippedTransition; });
            return; // GUI resize is requested but the scenegraph has not synchronized it yet.
        }
        const unsigned k = unsigned(frame_ % SA2_RING_SLOTS);
        if (options.route == "export-copy") {
            for (const auto& slot : slots_)
                if (slot.texture->nativeTexture().object != slot.resource) { fail("resource-changed"); return; }
        }
        if (warmupLeft_) {
            if (!call("sa2_mark_shown", native.fn_sa2_mark_shown(context_, k, frame_))) return;
            auto* batch = rhi_->nextResourceUpdateBatch();
            batch->copyTexture(color, slots_[k].texture);
            cb->resourceUpdate(batch);
            --warmupLeft_;
            edit([](Stats& s) { ++s.warmup; ++s.skippedWarmup; });
            completeTransition();
            return;
        }
        currentFields_ = {static_cast<std::uint32_t>(frame_), k, generation_};
        currentSize_ = ringSize_;
        const bool baseline = options.route == "rhi-upload";
        if (!baseline) {
            if (!call("sa2_produce", native.fn_sa2_produce(context_, k, frame_, currentFields_.sequence, generation_))
                || !call("sa2_godot_wait_ready", native.fn_sa2_godot_wait_ready(context_, frame_))
                || !call("sa2_mark_shown", native.fn_sa2_mark_shown(context_, k, frame_))) return;
            if (options.handover == "declared") slots_[k].texture->setNativeLayout(D3D12_RESOURCE_STATE_RENDER_TARGET);
        }
        edit([](Stats& s) { ++s.run; });
        observations = snapshot();
        const bool verify = observations.run % options.verifyEvery == 0;
        if (verify && !baseline) {
            std::uint64_t mismatch = 0;
            edit([](Stats& s) { ++s.nativeRequested; });
            const int status = native.fn_sa2_verify_slot(context_, k, currentFields_.sequence, generation_, &mismatch);
            edit([&](Stats& s) { if (status == SA2_OK || status == SA2_E_VERIFY) ++s.nativeCompleted; s.nativeMismatch += mismatch; });
            if (!call("sa2_verify_slot", status)) { fail("verify-mismatch"); return; }
        }
        if (options.route == "import-direct") {
            showSlot(k);
        } else {
            auto* batch = rhi_->nextResourceUpdateBatch();
            if (baseline) {
                const auto pixels = expectedImage(size.width(), size.height(), currentFields_);
                const QByteArray bytes(reinterpret_cast<const char*>(pixels.data()), qsizetype(pixels.size() * 4));
                const QRhiTextureSubresourceUploadDescription subresource(bytes);
                batch->uploadTexture(color, QRhiTextureUploadDescription({QRhiTextureUploadEntry(0, 0, subresource)}));
            } else batch->copyTexture(color, slots_[k].texture);
            if (verify) readback(batch, color, false);
            cb->resourceUpdate(batch);
        }
        if (options.lossAt && observations.run == options.lossAt) {
            edit([](Stats& s) {
                s.lossTriggered = true; s.lossStarted = std::chrono::steady_clock::now();
                s.reasons.insert("loss-requested");
            });
            call("sa2_remove_device", native.fn_sa2_remove_device(context_));
            observeLoss();
            return;
        }
        auto* swapChain = window->swapChain();
        const QSize backBufferSize = swapChain ? swapChain->currentPixelSize() : QSize();
        currentEligible_ = backBufferSize.width() >= ringSize_.width() && backBufferSize.height() >= ringSize_.height()
            && !snapshot().resizePending;
        edit([&](Stats& s) { if (currentEligible_) ++s.eligible; else ++s.skippedTransition; });
    } catch (const std::exception&) { fail("exception"); }
}

void Harness::readback(QRhiResourceUpdateBatch* batch, QRhiTexture* texture, bool composite) {
    auto ticket = std::make_unique<Readback>();
    auto* saved = ticket.get();
    const Fields fields = currentFields_;
    const QSize ring = currentSize_;
    saved->result.completed = [this, saved, fields, ring, composite] {
        saved->completed = true;
        const auto& result = saved->result;
        const bool bgra = result.format == QRhiTexture::BGRA8;
        const bool formatOk = result.format == QRhiTexture::RGBA8 || bgra;
        const auto width = result.pixelSize.width(), height = result.pixelSize.height();
        if (composite) {
            Fields decoded{};
            const bool ok = formatOk && decode(result.data.constData(), result.data.size(), width, height,
                ring.width(), ring.height(), bgra, decoded) && decoded == fields;
            edit([&](Stats& s) {
                ++s.compositeCompleted;
                if (ok) ++s.compositeVerified; else ++s.compositeMismatch;
                s.backBufferFormat = formatOk ? (bgra ? "BGRA8" : "RGBA8") : "unsupported";
            });
            if (!ok) fail("decode-mismatch");
        } else {
            const auto wanted = expectedImage(ring.width(), ring.height(), fields);
            std::uint64_t mismatches = 0;
            if (!formatOk || result.pixelSize != ring || result.data.size() != qsizetype(wanted.size() * 4)) mismatches = wanted.size();
            else {
                const auto* bytes = reinterpret_cast<const std::uint8_t*>(result.data.constData());
                for (std::size_t i = 0; i < wanted.size(); ++i) {
                    Pixel actual{bytes[4 * i + (bgra ? 2 : 0)], bytes[4 * i + 1], bytes[4 * i + (bgra ? 0 : 2)], bytes[4 * i + 3]};
                    if (actual != wanted[i]) ++mismatches;
                }
            }
            edit([&](Stats& s) { ++s.textureCompleted; s.textureMismatch += mismatches; });
            if (mismatches) fail("verify-mismatch");
        }
    };
    edit([&](Stats& s) { if (composite) ++s.compositeRequested; else ++s.textureRequested; });
    batch->readBackTexture(texture ? QRhiReadbackDescription(texture) : QRhiReadbackDescription(), &saved->result);
    readbacks_.push_back(std::move(ticket)); // Keep every result alive through finish and QRhi destruction.
}

void Harness::afterRendering() {
    if (!currentEligible_ || cleanupStarted_ || snapshot().lossTriggered) return;
    try {
        auto* batch = rhi_->nextResourceUpdateBatch();
        readback(batch, nullptr, true);
        window->swapChain()->currentFrameCommandBuffer()->resourceUpdate(batch);
    } catch (const std::exception&) { fail("exception"); }
    currentEligible_ = false;
}
void Harness::afterFrameEnd() {
    const auto s = snapshot();
    if (s.lossTriggered) return;
    edit([](Stats& result) { result.frameEnded = true; });
    if (s.run >= options.frames) { edit([](Stats& result) { result.stop = true; }); return; }
    if (!s.stop && options.resizeEvery && s.run && s.run % options.resizeEvery == 0
        && !s.resizePending && s.resizeRequested < s.run / options.resizeEvery) {
        constexpr std::array<std::array<int, 2>, 4> sizes{{{1280, 720}, {1024, 640}, {1440, 810}, {800, 600}}};
        const auto size = sizes[(s.resizeRequested + 1) % sizes.size()];
        transitionStart_ = frame_ + 1;
        edit([&](Stats& result) {
            ++result.resizeRequested; result.resizePending = true; result.resizeRequest = true;
            result.resizeWidth = size[0]; result.resizeHeight = size[1];
        });
    }
}

void Harness::observeLoss() {
    if (!context_) return;
    int reason = 0;
    if (call("sa2_device_removed_reason", native.fn_sa2_device_removed_reason(context_, &reason)))
        edit([&](Stats& s) { s.removedReason = reason; s.reasons.insert("loss-reason-observed"); });
}
void Harness::teardown(QRhi* rhi) {
    if (cleanupStarted_) return;
    cleanupStarted_ = true;
    currentEligible_ = false;
    if (rhi) rhi->finish();
    // The renderer/node is being deleted on this thread while the QRhi is alive.
    node_ = nullptr;
    root_ = nullptr;
    destroyWrappers();
    edit([](Stats& s) { s.phase = "pending"; });
    writeResult(false); // Must exist before the DLL's possible TerminateProcess(exit 3).
    if (context_) {
        if (snapshot().lossTriggered) observeLoss();
        releaseRing();
        struct sa2_debug_counts counts{};
        counts.struct_size = sizeof(counts);
        call("sa2_debug_counts", native.fn_sa2_debug_counts(context_, &counts));
        char descriptions[8192]{};
        call("sa2_debug_messages", native.fn_sa2_debug_messages(context_, descriptions, sizeof(descriptions)));
        edit([&](Stats& s) { s.debug = counts; s.debugMessages = descriptions; });
        if (call("sa2_detach", native.fn_sa2_detach(context_))) {
            context_ = nullptr;
            edit([](Stats& s) { s.detached = true; });
        }
    }
    edit([](Stats& s) { s.renderDone = true; });
    rhi_ = nullptr;
}
void Harness::rendererGone(QRhi* rhi) { teardown(rhi); }
void Harness::invalidated() {
    edit([](Stats& s) { ++s.invalidated; if (s.lossTriggered) s.reasons.insert("loss-invalidated"); });
    // QQuickRhiItem's destructor normally cleaned first; the direct route cleans here.
    teardown(rhi_);
}
void Harness::sceneError(const QString& message) {
    const auto text = message.toStdString();
    edit([&](Stats& s) { ++s.graphErrors; s.sceneGraphError = text; });
    fail("qt-scenegraph-error");
}
GuiAction Harness::guiAction() {
    GuiAction result{};
    edit([&](Stats& s) {
        const bool lossExpired = s.lossTriggered && std::chrono::steady_clock::now() - s.lossStarted >= std::chrono::seconds(10);
        result = {s.lossTriggered ? lossExpired : s.stop, s.resizeRequest, s.renderDone, s.resizeWidth, s.resizeHeight};
        s.resizeRequest = false;
    });
    return result;
}

int Harness::finalCode() {
    if (options.probe()) return 0;
    return snapshot().reasons.empty() ? 0 : 1;
}
bool Harness::writeResult(bool final) {
    auto s = snapshot();
    if (final) {
        if (s.run != options.frames || s.compositeVerified != s.eligible) s.reasons.insert("unverified-frames");
        if (!s.run || s.eligible * 10 < s.run * 9) s.reasons.insert("eligible-coverage");
        if (s.compositeRequested != s.compositeCompleted || s.textureRequested != s.textureCompleted
            || s.nativeRequested != s.nativeCompleted) s.reasons.insert("readback-missing");
        if (s.nativeMismatch || s.textureMismatch) s.reasons.insert("verify-mismatch");
        if (s.compositeMismatch) s.reasons.insert("decode-mismatch");
        if (!s.failures.empty()) s.reasons.insert("sa2-call-failed");
        if (options.debugLayer && (s.debug.error || s.debug.corruption)) s.reasons.insert("debug-errors");
        if (!s.attached || !s.detached || !s.drainConfirmed) s.reasons.insert("teardown-incomplete");
        if (!s.deviceMatches.value_or(false) || s.fallbackDetected || !s.info.queue_device_matches
            || (options.device == "from-rhi" && (!s.queueMatches.value_or(false) || !s.rhiMatches.value_or(false)))
            || (options.device == "from-device" && !s.qtQueueDeviceMatches.value_or(false))) s.reasons.insert("identity-mismatch");
        if (s.resizeRequested != s.resizeCompleted) s.reasons.insert("resize-incomplete");
        if (std::any_of(s.refcounts.begin(), s.refcounts.end(), [](auto count) { return count != 0; })
            || (options.device != "qt" && (!s.deviceRefcount || *s.deviceRefcount != 0))
            || (options.device == "from-rhi" && (!s.queueRefcount || *s.queueRefcount != 0))) s.reasons.insert("refcount-nonzero");
        edit([&](Stats& result) { result.reasons = s.reasons; });
    }
    QJsonArray reasons, failures;
    for (const auto& reason : s.reasons) reasons.append(QString::fromStdString(reason));
    for (const auto& failure : s.failures) failures.append(QJsonObject{{"call", QString::fromStdString(failure.call)},
        {"status", failure.status}, {"last_error", QString::fromStdString(failure.error)}});
    QJsonObject identity{{"fallback_detected", s.fallbackDetected}};
    optionalBool(identity, "device_matches", s.deviceMatches); optionalBool(identity, "queue_matches", s.queueMatches);
    optionalBool(identity, "rhi_matches", s.rhiMatches); optionalBool(identity, "qt_queue_device_matches", s.qtQueueDeviceMatches);
    const auto settings = config();
    const auto text = [](const std::string& value) { return QString::fromStdString(value); };
    QJsonObject teardown{{"phase", text(s.phase)}, {"drain_confirmed", s.drainConfirmed}, {"detached", s.detached},
        {"refcount_after", numbers(s.refcounts)}, {"slots_unregistered", int(s.slotsUnregistered)}};
    if (s.drainResult) teardown.insert("drain_result", *s.drainResult);
    if (s.deviceRefcount) teardown.insert("device_refcount_after", number(*s.deviceRefcount));
    if (s.queueRefcount) teardown.insert("queue_refcount_after", number(*s.queueRefcount));
    QJsonArray lossStatuses;
    for (const auto& status : s.lossStatuses) lossStatuses.append(QJsonObject{{"call", QString::fromStdString(status.call)}, {"status", status.status}});
    QJsonObject loss{{"requested_at", int(options.lossAt)}, {"triggered", s.lossTriggered},
        {"scene_graph_initialized", int(s.initialized)}, {"scene_graph_invalidated", int(s.invalidated)},
        {"scene_graph_errors", int(s.graphErrors)}, {"render_steps_resumed", int(s.lossResumed)},
        {"new_rhi_seen", s.lossNewRhi}, {"statuses", lossStatuses}};
    if (s.removedReason) loss.insert("removed_reason", *s.removedReason);
    QJsonArray ids;
    for (unsigned i = 0; i < std::min(s.debug.distinct_id_count, 16u); ++i) ids.append(s.debug.ids[i]);
    const QString renderLoop = qEnvironmentVariable("QSG_RENDER_LOOP", "threaded");
    QJsonObject root{
        {"format", "magic600-sd-smoke-run-v1"},
        {"status", !final || options.probe() ? "recorded" : (s.reasons.empty() ? "pass" : "fail")},
        {"reasons", reasons}, {"sa2_failures", failures},
        {"config", QJsonObject{{"out", options.out}, {"dll", options.dll}, {"device", text(options.device)}, {"route", text(options.route)}, {"queue", text(options.queue)},
            {"handover", text(options.handover)}, {"barriers", text(options.barriers)}, {"frames", int(options.frames)},
            {"resize_every", int(options.resizeEvery)}, {"verify_every", int(options.verifyEvery)}, {"device_loss_at", int(options.lossAt)},
            {"timeout_ms", int(options.timeoutMs)}, {"render_loop", renderLoop}, {"debug_layer", options.debugLayer}}},
        {"qt", QJsonObject{{"version", qVersion()}, {"graphics_api", s.graphicsApi}, {"render_loop", renderLoop},
            {"effective_device_pixel_ratio", s.dpr}, {"back_buffer_format", text(s.backBufferFormat)}}},
        {"device", QJsonObject{{"route", text(options.device)}, {"struct_size", int(s.info.struct_size)}, {"adapter_name", text(s.adapter)}, {"queue_type", s.info.queue_type},
            {"queue_device_matches", bool(s.info.queue_device_matches)}, {"adapter_matches_device", bool(s.info.adapter_matches_device)},
            {"enhanced_barriers", bool(s.info.enhanced_barriers)}, {"debug_layer", bool(s.info.debug_layer)},
            {"max_feature_level", s.info.max_feature_level}, {"node_count", int(s.info.node_count)},
            {"vendor_id", int(s.info.vendor_id)}, {"device_id", int(s.info.device_id)}, {"umd_version", number(s.info.umd_version)}, {"identity", identity}}},
        {"dll", QJsonObject{{"abi_version", int(SA2_ABI_VERSION)}, {"resolved_barriers", settings.barrier_api == SA2_BARRIERS_LEGACY
            || !s.info.enhanced_barriers ? "legacy" : "enhanced"}, {"state_before", settings.state_before_write},
            {"state_after", settings.state_after_write}, {"initial_state", options.route == "export-copy" ? SA2_STATE_COMMON : settings.state_before_write}}},
        {"frames", QJsonObject{{"requested", int(options.frames)}, {"steps", number(s.steps)}, {"run", number(s.run)},
            {"warmup", number(s.warmup)}, {"transition", number(s.transition)}, {"eligible", number(s.eligible)},
            {"skipped_warmup", number(s.skippedWarmup)}, {"skipped_transition", number(s.skippedTransition)}}},
        {"verify", QJsonObject{{"native_requested", number(s.nativeRequested)}, {"native_completed", number(s.nativeCompleted)},
            {"native_mismatched_texels", number(s.nativeMismatch)}, {"texture_requested", number(s.textureRequested)},
            {"texture_completed", number(s.textureCompleted)}, {"texture_mismatched_texels", number(s.textureMismatch)},
            {"composite_requested", number(s.compositeRequested)}, {"composite_completed", number(s.compositeCompleted)},
            {"composite_verified", number(s.compositeVerified)}, {"composite_mismatches", number(s.compositeMismatch)}}},
        {"resize", QJsonObject{{"requested", int(s.resizeRequested)}, {"completed", int(s.resizeCompleted)},
            {"generation", int(generation_)}, {"transition_frames", numbers(s.transitionFrames)}}},
        {"device_loss", loss},
        {"debug", QJsonObject{{"error", number(s.debug.error)}, {"corruption", number(s.debug.corruption)},
            {"warning", number(s.debug.warning)}, {"info", number(s.debug.info)}, {"message", number(s.debug.message)},
            {"mismatching_clear_value", number(s.debug.mismatching_clear_value)}, {"mentioning_sa2", number(s.debug.mentioning_sa2)},
            {"distinct_id_count", int(s.debug.distinct_id_count)}, {"ids", ids}, {"messages", text(s.debugMessages)}}},
        {"teardown", teardown}, {"scene_graph_error", text(s.sceneGraphError)}};
    QSaveFile output(options.out);
    const auto bytes = QJsonDocument(root).toJson(QJsonDocument::Indented);
    if (!output.open(QIODevice::WriteOnly) || output.write(bytes) != bytes.size() || !output.commit()) {
        fail("result-write-failed");
        std::cerr << "sd: result write failed\n";
        return false;
    }
    return true;
}

class Renderer : public QQuickRhiItemRenderer {
public:
    explicit Renderer(Harness& harness) : harness_(harness) {}
    ~Renderer() override { harness_.rendererGone(rhi()); }
    void initialize(QRhiCommandBuffer*) override {}
    void synchronize(QQuickRhiItem*) override {}
    void render(QRhiCommandBuffer* cb) override {
        harness_.renderStep(cb, colorTexture(), renderTarget(), colorTexture()->pixelSize());
    }
private:
    Harness& harness_;
};
QQuickRhiItemRenderer* RhiItem::createRenderer() { return new Renderer(harness_); }
QSGNode* DirectItem::updatePaintNode(QSGNode* old, UpdatePaintNodeData*) {
    auto* root = old ? old : new QSGNode;
    harness_.setNode(root, QSize(qRound(width()), qRound(height())));
    return root;
}
}
