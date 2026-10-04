#include "smoke.h"
#include "l2.h"
#include <QtCore/QFileInfo>
#include <QtCore/QTimer>
#include <QtGui/QGuiApplication>
#include <QtGui/QScreen>
#include <QtGui/QSurfaceFormat>
#include <QtQuick/QQuickGraphicsConfiguration>
#include <QtQuick/QQuickGraphicsDevice>
#include <QtQuick/QSGRendererInterface>
#include <rhi/qrhi_platform.h>
#include <d3d12sdklayers.h>
#include <dxgi1_6.h>
#include <wrl/client.h>
#include <algorithm>
#include <charconv>
#include <cstring>
#include <iostream>
#include <set>
#include <stdexcept>
#include <string_view>

namespace {
sd::Options arguments(int argc, char** argv) {
    sd::Options options;
    std::set<std::string> seen;
    for (int i = 1; i < argc; ++i) {
        const std::string key = argv[i];
        if (!seen.insert(key).second) throw std::invalid_argument("duplicate argument");
        if (key == "--sd-debug-layer") { options.debugLayer = true; continue; }
        if (i + 1 >= argc) throw std::invalid_argument("missing argument value");
        const std::string value = argv[++i];
        const auto choice = [&](const auto& choices) {
            if (std::find(choices.begin(), choices.end(), value) == choices.end()) throw std::invalid_argument("invalid argument value");
            return value;
        };
        const auto integer = [&]() {
            std::uint32_t result = 0;
            auto parsed = std::from_chars(value.data(), value.data() + value.size(), result);
            if (parsed.ec != std::errc() || parsed.ptr != value.data() + value.size() || result > 2147483647u)
                throw std::invalid_argument("invalid integer");
            return result;
        };
        if (key == "--sd-out") options.out = QString::fromLocal8Bit(argv[i]);
        else if (key == "--sd-dll") options.dll = QString::fromLocal8Bit(argv[i]);
        else if (key == "--sd-device") options.device = choice(std::array<std::string, 3>{"qt", "from-rhi", "from-device"});
        else if (key == "--sd-route") options.route = choice(std::array<std::string, 4>{"rhi-upload", "import-copy", "export-copy", "import-direct"});
        else if (key == "--sd-queue") options.queue = choice(std::array<std::string, 2>{"same", "own"});
        else if (key == "--sd-handover") options.handover = choice(std::array<std::string, 2>{"tracked", "declared"});
        else if (key == "--sd-barriers") options.barriers = choice(std::array<std::string, 2>{"legacy", "match"});
        else if (key == "--sd-frames") options.frames = integer();
        else if (key == "--sd-resize-every") options.resizeEvery = integer();
        else if (key == "--sd-verify-every") options.verifyEvery = integer();
        else if (key == "--sd-device-loss-at") options.lossAt = integer();
        else if (key == "--sd-timeout-ms") options.timeoutMs = integer();
        else throw std::invalid_argument("unknown argument");
    }
    if (options.out.isEmpty() || options.dll.isEmpty() || !QFileInfo(options.out).isAbsolute() || !QFileInfo(options.dll).isAbsolute())
        throw std::invalid_argument("absolute --sd-out and --sd-dll are required");
    if (!options.frames || !options.verifyEvery || !options.timeoutMs || options.lossAt > options.frames
        || (options.handover == "declared" && options.route != "import-copy"))
        throw std::invalid_argument("invalid argument combination");
    return options;
}
struct Owned {
    ID3D12Device* device = nullptr;
    ID3D12CommandQueue* queue = nullptr;
    QRhi* rhi = nullptr;
    void release(sd::Harness& harness) {
        delete rhi; rhi = nullptr;
        if (queue) {
            const auto count = queue->Release(); queue = nullptr;
            harness.edit([&](sd::Stats& s) { s.queueRefcount = count; });
        }
        if (device) {
            const auto count = device->Release(); device = nullptr;
            harness.edit([&](sd::Stats& s) { s.deviceRefcount = count; });
        }
    }
};
void createOwned(sd::Harness& harness, Owned& owned) {
    using Microsoft::WRL::ComPtr;
    if (harness.options.device == "qt") return;
    ComPtr<IDXGIFactory6> factory;
    ComPtr<IDXGIAdapter1> adapter;
    if (harness.options.debugLayer) {
        ComPtr<ID3D12Debug> debug;
        if (FAILED(D3D12GetDebugInterface(IID_PPV_ARGS(&debug)))) throw std::runtime_error("debug layer unavailable");
        debug->EnableDebugLayer();
    }
    if (FAILED(CreateDXGIFactory2(0, IID_PPV_ARGS(&factory)))
        || FAILED(factory->EnumAdapterByGpuPreference(0, DXGI_GPU_PREFERENCE_HIGH_PERFORMANCE, IID_PPV_ARGS(&adapter)))
        || FAILED(D3D12CreateDevice(adapter.Get(), D3D_FEATURE_LEVEL_11_0, IID_PPV_ARGS(&owned.device))))
        throw std::runtime_error("device creation failed");
    harness.suppliedDevice = owned.device;
    if (harness.options.device == "from-device") return;
    D3D12_COMMAND_QUEUE_DESC descriptor{};
    descriptor.Type = D3D12_COMMAND_LIST_TYPE_DIRECT;
    if (FAILED(owned.device->CreateCommandQueue(&descriptor, IID_PPV_ARGS(&owned.queue)))
        || FAILED(owned.queue->SetName(L"SD harness queue"))) throw std::runtime_error("queue creation failed");
    harness.suppliedQueue = owned.queue;
    QRhiD3D12InitParams params{};
    params.enableDebugLayer = harness.options.debugLayer;
    QRhiD3D12NativeHandles handles{};
    handles.dev = owned.device; handles.commandQueue = owned.queue;
    owned.rhi = QRhi::create(QRhi::D3D12, &params, QRhi::Flags{}, &handles);
    if (!owned.rhi) { harness.fail("rhi-create-failed"); throw std::runtime_error("QRhi creation failed"); }
    const auto* actual = static_cast<const QRhiD3D12NativeHandles*>(owned.rhi->nativeHandles());
    ComPtr<IUnknown> expectedDevice, actualDevice;
    const bool deviceMatches = actual && actual->dev
        && SUCCEEDED(owned.device->QueryInterface(IID_PPV_ARGS(&expectedDevice)))
        && SUCCEEDED(static_cast<IUnknown*>(actual->dev)->QueryInterface(IID_PPV_ARGS(&actualDevice))) && expectedDevice.Get() == actualDevice.Get();
    const bool queueMatches = actual && actual->commandQueue == owned.queue;
    harness.edit([&](sd::Stats& s) { s.deviceMatches = deviceMatches; s.queueMatches = queueMatches; s.fallbackDetected = !deviceMatches || !queueMatches; });
    if (!deviceMatches || !queueMatches) { harness.fail("identity-mismatch"); throw std::runtime_error("QRhi identity mismatch"); }
    harness.suppliedRhi = owned.rhi;
}
}

int level2Main(int argc, char** argv) {
    sd::L2Options options;
    options.out = sd::usableL2Out(argc, argv);
    try { sd::parseL2(argc, argv, options); }
    catch (const std::exception&) {
        // The independent pre-scan keeps a usable output directory even if parsing failed earlier.
        options.out = sd::usableL2Out(argc, argv);
        sd::L2 result(options); result.failure("usage"); result.write();
        std::cerr << "sd l2: invalid arguments\n";
        return 1;
    }
    sd::L2 result(options);
    if (!qEnvironmentVariableIsSet("QSG_RENDER_LOOP")) qputenv("QSG_RENDER_LOOP", "threaded");
    for (const auto* name : {"QSG_INFO", "QSG_RENDER_TIMING", "QSG_RHI_PROFILE", "QSG_VISUALIZE", "QSG_RENDERER_DEBUG", "QT_DEBUG_PLUGINS"}) qunsetenv(name);
    qputenv("QT_LOGGING_RULES", "qt.rhi.general=true;qt.scenegraph.general=true");
    sd::watchFrameFailures();
    QGuiApplication app(argc, argv);
    app.setQuitOnLastWindowClosed(false);
    sd::Harness harness(options.framework, &result);
    Owned owned;
    std::unique_ptr<QQuickWindow> window;
    try {
        if (std::strcmp(qVersion(), "6.10.3") != 0) throw std::runtime_error("Qt version mismatch");
        sd::selectHighPerformanceAdapter();
        if (options.gpuValidation) {
            Microsoft::WRL::ComPtr<ID3D12Debug1> debug;
            if (FAILED(D3D12GetDebugInterface(IID_PPV_ARGS(&debug)))) throw std::runtime_error("GPU validation unavailable");
            // The debug layer only, as level 1 and the DLL self-test: GPU-based validation would slow frames past the W3/W4 turns.
            debug->EnableDebugLayer();
        }
        qunsetenv("QSG_RHI_DEBUG_LAYER"); // Validation is an option, never a configuration/environment identity part.
        result.configuration();
        harness.native.load(options.dll.toStdWString().c_str());
        createOwned(harness, owned);
        QQuickWindow::setGraphicsApi(QSGRendererInterface::Direct3D12);
        QSurfaceFormat surface = QSurfaceFormat::defaultFormat(); surface.setSwapInterval(0);
        QSurfaceFormat::setDefaultFormat(surface);
        window = std::make_unique<QQuickWindow>(); harness.window = window.get();
        window->setFormat(surface);
        window->setPersistentSceneGraph(false); window->setPersistentGraphics(false);
        QQuickGraphicsConfiguration graphics; graphics.setDebugLayer(options.gpuValidation);
        window->setGraphicsConfiguration(graphics);
        if (owned.rhi) window->setGraphicsDevice(QQuickGraphicsDevice::fromRhi(owned.rhi));
        else if (owned.device) window->setGraphicsDevice(QQuickGraphicsDevice::fromDeviceAndContext(owned.device, nullptr));
        auto* screen = QGuiApplication::primaryScreen();
        if (!screen) throw std::runtime_error("primary screen unavailable");
        window->setScreen(screen); window->setFlags(Qt::Window | Qt::FramelessWindowHint | Qt::WindowStaysOnTopHint);
        window->setGeometry(screen->geometry()); window->setCursor(Qt::BlankCursor);
        QQuickItem* item = nullptr;
        if (options.framework.route == "import-direct") item = new sd::DirectItem(harness, window->contentItem());
        else {
            auto* rhiItem = new sd::RhiItem(harness, window->contentItem());
            if (options.halfTarget) {
                const auto physical = screen->size() * screen->devicePixelRatio();
                rhiItem->setFixedColorBufferWidth(physical.width() / 2); rhiItem->setFixedColorBufferHeight(physical.height() / 2);
            }
            item = rhiItem;
        }
        item->setPosition(QPointF(0, 0)); item->setSize(QSizeF(window->width(), window->height()));
        const auto resizeItem = [&] { item->setSize(QSizeF(window->width(), window->height())); item->update(); };
        QObject::connect(window.get(), &QQuickWindow::widthChanged, &app, resizeItem);
        QObject::connect(window.get(), &QQuickWindow::heightChanged, &app, resizeItem);
        result.hwnd = reinterpret_cast<HWND>(window->winId());
        if (!SetWindowPos(result.hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE))
            throw std::runtime_error("topmost window unavailable");
        QObject::connect(window.get(), &QQuickWindow::sceneGraphInitialized, window.get(), [&] { harness.initialized(); }, Qt::DirectConnection);
        QObject::connect(window.get(), &QQuickWindow::sceneGraphInvalidated, window.get(), [&] { harness.invalidated(); }, Qt::DirectConnection);
        QObject::connect(window.get(), &QQuickWindow::sceneGraphError, window.get(),
            [&](QQuickWindow::SceneGraphError, const QString& message) { harness.sceneError(message); }, Qt::DirectConnection);
        if (options.framework.route == "import-direct") QObject::connect(window.get(), &QQuickWindow::beforeRendering, window.get(),
            [&] { harness.renderStep(nullptr, nullptr, nullptr, {}); }, Qt::DirectConnection);
        QObject::connect(window.get(), &QQuickWindow::afterFrameEnd, window.get(), [&] { harness.afterFrameEnd(); }, Qt::DirectConnection);
        bool stopping = false;
        QObject::connect(window.get(), &QQuickWindow::frameSwapped, &app, [&] {
            if (!stopping && !harness.snapshot().stop) { item->update(); window->update(); }
        }, Qt::QueuedConnection);
        QTimer timer;
        QObject::connect(&timer, &QTimer::timeout, &app, [&] {
            const auto action = harness.guiAction();
            if (!stopping && (action.close || action.renderDone)) {
                stopping = true; window->close(); window->releaseResources();
                window.reset(); harness.window = nullptr;
                owned.release(harness); app.exit();
            }
        });
        timer.start(10);
        std::cout << "sd l2: starting\n";
        window->showFullScreen();
        SetForegroundWindow(result.hwnd); // Exactly one request; no foreground-lock workaround.
        item->update(); window->update();
        app.exec();
        if (!stopping) result.failure("interrupted");
    } catch (const std::exception&) { harness.fail("startup"); }
    // The join happens before reading the render-thread report or releasing supplied graphics objects.
    window.reset(); harness.window = nullptr; owned.release(harness);
    if (harness.native.fn_sa2_abi_version) result.abi(harness.native.fn_sa2_abi_version());
    const auto stats = harness.snapshot();
    if (!stats.reasons.empty()) result.failure(QString::fromStdString(*stats.reasons.begin()));
    const bool written = result.write();
    std::cout << "sd l2: exit " << (written ? result.exitCode : 1) << '\n';
    return written ? result.exitCode : 1;
}

int main(int argc, char** argv) {
    for (int i = 1; i < argc; ++i) if (std::string_view(argv[i]).starts_with("--l2-")) return level2Main(argc, argv);
    sd::Options options;
    try { options = arguments(argc, argv); }
    catch (const std::exception& error) { std::cerr << "sd: " << error.what() << '\n'; return 2; }
    QGuiApplication app(argc, argv);
    app.setQuitOnLastWindowClosed(false);
    sd::Harness harness(options);
    Owned owned;
    std::unique_ptr<QQuickWindow> window;
    try {
        if (std::strcmp(qVersion(), "6.10.3") != 0) { harness.fail("qt-version"); throw std::runtime_error("Qt version mismatch"); }
        harness.native.load(options.dll.toStdWString().c_str());
        // All our GUI-thread native D3D12 setup precedes creation of the window.
        createOwned(harness, owned);
        QQuickWindow::setGraphicsApi(QSGRendererInterface::Direct3D12);
        window = std::make_unique<QQuickWindow>();
        harness.window = window.get();
        window->setPersistentSceneGraph(false);
        window->setPersistentGraphics(false);
        if (owned.rhi) window->setGraphicsDevice(QQuickGraphicsDevice::fromRhi(owned.rhi));
        else if (owned.device) window->setGraphicsDevice(QQuickGraphicsDevice::fromDeviceAndContext(owned.device, nullptr));
        window->resize(1280, 720); window->setPosition(40, 40);
        QQuickItem* item = options.route == "import-direct"
            ? static_cast<QQuickItem*>(new sd::DirectItem(harness, window->contentItem()))
            : static_cast<QQuickItem*>(new sd::RhiItem(harness, window->contentItem()));
        item->setPosition(QPointF(0, 0)); item->setSize(QSizeF(1280, 720));
        const auto resizeItem = [&] { item->setSize(QSizeF(window->width(), window->height())); item->update(); };
        QObject::connect(window.get(), &QQuickWindow::widthChanged, &app, resizeItem);
        QObject::connect(window.get(), &QQuickWindow::heightChanged, &app, resizeItem);
        QObject::connect(window.get(), &QQuickWindow::sceneGraphInitialized, window.get(), [&] { harness.initialized(); }, Qt::DirectConnection);
        QObject::connect(window.get(), &QQuickWindow::sceneGraphInvalidated, window.get(), [&] { harness.invalidated(); }, Qt::DirectConnection);
        QObject::connect(window.get(), &QQuickWindow::sceneGraphError, window.get(),
            [&](QQuickWindow::SceneGraphError, const QString& message) { harness.sceneError(message); }, Qt::DirectConnection);
        if (options.route == "import-direct")
            QObject::connect(window.get(), &QQuickWindow::beforeRendering, window.get(),
                [&] { harness.renderStep(nullptr, nullptr, nullptr, {}); }, Qt::DirectConnection);
        QObject::connect(window.get(), &QQuickWindow::afterRendering, window.get(), [&] { harness.afterRendering(); }, Qt::DirectConnection);
        QObject::connect(window.get(), &QQuickWindow::afterFrameEnd, window.get(), [&] { harness.afterFrameEnd(); }, Qt::DirectConnection);
        bool stopping = false, finalized = false;
        QObject::connect(window.get(), &QQuickWindow::frameSwapped, &app, [&] {
            if (!stopping) { item->update(); window->update(); }
        }, Qt::QueuedConnection);
        QTimer timer;
        QObject::connect(&timer, &QTimer::timeout, &app, [&] {
            const auto action = harness.guiAction();
            if (!stopping && action.resize) {
                window->resize(action.width, action.height);
                resizeItem(); window->update();
            }
            if (!stopping && action.close) {
                stopping = true;
                window->close();
                window->releaseResources();
            }
            if (stopping && !finalized) {
                finalized = true;
                // The basic loop releases nothing on hide. Window deletion runs any remaining teardown and
                // joins the render thread before the application's QRhi is destroyed.
                window.reset(); harness.window = nullptr;
                owned.release(harness);
                harness.edit([](sd::Stats& s) { s.phase = "complete"; });
                const bool written = harness.writeResult(true);
                app.exit(written ? harness.finalCode() : 1);
            }
        });
        timer.start(10);
        window->show(); item->update(); window->update();
        return app.exec();
    } catch (const std::exception&) {
        harness.fail("exception");
        window.reset(); harness.window = nullptr;
        owned.release(harness);
        harness.edit([](sd::Stats& s) { s.phase = "complete"; });
        harness.writeResult(true);
        std::cerr << "sd: startup failed\n";
        return 1;
    }
}
