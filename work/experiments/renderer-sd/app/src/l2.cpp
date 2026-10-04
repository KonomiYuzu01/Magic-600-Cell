#include "l2.h"
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QProcessEnvironment>
#include <QtCore/QRegularExpression>
#include <dwmapi.h>
#include <dxgi1_6.h>
#include <psapi.h>
#include <wrl/client.h>
#include <algorithm>
#include <charconv>
#include <cmath>
#include <set>
#include <stdexcept>
#include <string_view>

namespace sd {
namespace {
const QJsonValue unknown(QJsonValue::Null);
QJsonValue number(std::uint64_t value) { return QJsonValue(static_cast<qint64>(value)); }
QJsonObject dimensions(QSize size) {
    if (size.isEmpty()) return {{"width", unknown}, {"height", unknown}};
    return {{"width", size.width()}, {"height", size.height()}};
}
QString executablePath() {
    wchar_t path[32768]{};
    const auto length = GetModuleFileNameW(nullptr, path, DWORD(std::size(path)));
    if (!length || length >= std::size(path)) throw std::runtime_error("executable path unavailable");
    return QDir::cleanPath(QString::fromWCharArray(path));
}
bool outputDirectory(const QString& path, bool allOutputs) {
    if (path.isEmpty() || !QFileInfo(path).isDir()) return false;
    const QDir directory(path);
    const QStringList names = allOutputs
        ? QStringList{"harness.json", "harness.json.tmp", "native.json", "trace.jsonl", "geometry.json"}
        : QStringList{"harness.json", "harness.json.tmp"};
    for (const auto& name : names) if (directory.exists(name)) return false;
    return true;
}
QString powerModeName(int mode) {
    switch (mode) {
    case EffectivePowerModeBatterySaver: return "battery_saver";
    case EffectivePowerModeBetterBattery: return "better_battery";
    case EffectivePowerModeBalanced: return "balanced";
    case EffectivePowerModeHighPerformance: return "high_performance";
    case EffectivePowerModeMaxPerformance: return "max_performance";
    case EffectivePowerModeGameMode: return "game_mode";
    case EffectivePowerModeMixedReality: return "mixed_reality";
    default: return "unknown";
    }
}
bool covered(HWND window) {
    RECT r{};
    if (!GetWindowRect(window, &r)) return true;
    const LONG w = r.right - r.left, h = r.bottom - r.top;
    const POINT points[] = {{r.left + w / 2, r.top + h / 2}, {r.left + w / 10, r.top + h / 10},
        {r.right - w / 10, r.top + h / 10}, {r.left + w / 10, r.bottom - h / 10}, {r.right - w / 10, r.bottom - h / 10}};
    for (const auto& point : points) {
        const HWND hit = WindowFromPoint(point);
        if (!hit || GetAncestor(hit, GA_ROOT) != window) return true;
    }
    return false;
}
}

QString usableL2Out(int argc, char** argv) {
    QString path;
    int count = 0;
    for (int i = 1; i < argc; ++i) {
        if (std::string_view(argv[i]) == "--l2-out") {
            ++count;
            if (i + 1 < argc) path = QString::fromLocal8Bit(argv[i + 1]);
        }
    }
    return count == 1 && outputDirectory(path, false) ? QFileInfo(path).absoluteFilePath() : QString();
}

void parseL2(int argc, char** argv, L2Options& options) {
    std::set<std::string> seen;
    const auto usage = [] { throw std::invalid_argument("invalid level 2 arguments"); };
    for (int i = 1; i < argc; ++i) {
        const std::string key = argv[i];
        if (key != "--l2-declare" && !seen.insert(key).second) usage();
        if (key == "--l2-no-vram") { options.noVram = true; continue; }
        if (key == "--l2-debug-half-target") { options.halfTarget = true; continue; }
        if (i + 1 >= argc || std::string_view(argv[i + 1]).starts_with("--")) usage();
        const std::string value = argv[++i];
        const auto text = QString::fromLocal8Bit(argv[i]);
        const auto choice = [&](std::initializer_list<std::string_view> choices) {
            if (std::find(choices.begin(), choices.end(), value) == choices.end()) usage();
            return value;
        };
        const auto integer = [&](std::uint32_t minimum, std::uint32_t maximum) {
            std::uint32_t result = 0;
            const auto parsed = std::from_chars(value.data(), value.data() + value.size(), result);
            if (parsed.ec != std::errc() || parsed.ptr != value.data() + value.size() || result < minimum || result > maximum) usage();
            return result;
        };
        if (key == "--l2-mode") options.mode = QString::fromStdString(choice({"run", "geometry"}));
        else if (key == "--l2-scene") options.scene = QString::fromStdString(choice({"w1", "w2", "w3", "w4"}));
        else if (key == "--l2-out") options.out = text;
        else if (key == "--l2-run-id") options.runId = text;
        else if (key == "--l2-dll") options.dll = text;
        else if (key == "--l2-trace-ms") options.traceMs = integer(1000, 3600000);
        else if (key == "--l2-preroll-ms") options.prerollMs = integer(0, 60000);
        else if (key == "--l2-turn-ms") {
            double result = 0;
            const auto parsed = std::from_chars(value.data(), value.data() + value.size(), result);
            if (parsed.ec != std::errc() || parsed.ptr != value.data() + value.size() || !std::isfinite(result) || result <= 0 || result > 10000) usage();
            options.turnMs = result;
        }
        else if (key == "--l2-inject") options.inject = QString::fromStdString(choice({"corrupt-label", "swap-same-colour", "delay-adoption", "stale-binding"}));
        else if (key == "--l2-gpu-validation") options.gpuValidation = integer(0, 1) == 1;
        else if (key == "--l2-conditions") options.enforce = choice({"enforce", "record"}) == "enforce";
        else if (key == "--l2-declare") {
            const auto split = text.indexOf('=');
            if (split <= 0 || split == text.size() - 1) usage();
            const auto name = text.left(split), declared = text.mid(split + 1);
            if (name == "overlays" || options.declared.contains(name)) usage();
            options.declared.insert(name, declared == "true" ? QJsonValue(true) : declared == "false" ? QJsonValue(false) : QJsonValue(declared));
        }
        else if (key == "--sd-device") options.framework.device = choice({"qt", "from-rhi", "from-device"});
        else if (key == "--sd-route") options.framework.route = choice({"import-copy", "export-copy", "import-direct"});
        else if (key == "--sd-queue") options.framework.queue = choice({"same", "own"});
        else if (key == "--sd-handover") options.framework.handover = choice({"tracked", "declared"});
        else if (key == "--sd-barriers") options.framework.barriers = choice({"legacy", "match"});
        else if (key == "--sd-timeout-ms") options.framework.timeoutMs = integer(5000, 5000);
        else usage(); // Frame counts, resize, verification, loss, drain injection and CPU upload have no scene meaning.
    }
    if (options.mode.isEmpty() || (options.mode == "run" && options.scene.isEmpty()) || options.runId.isEmpty()
        || !QRegularExpression("\\A[A-Za-z0-9][A-Za-z0-9._-]{0,127}\\z").match(options.runId).hasMatch()
        || !outputDirectory(options.out, true) || !QFileInfo(options.dll).isAbsolute() || !QFileInfo(options.dll).isFile()
        || QFileInfo(options.dll).fileName().compare("sa2_interop.dll", Qt::CaseInsensitive) != 0
        || (seen.contains("--l2-turn-ms") && options.scene != "w3")
        || (!options.inject.isEmpty() && options.scene != "w3" && options.scene != "w4")
        || (options.framework.handover == "declared" && options.framework.route != "import-copy")) usage();
    const auto renderLoop = qEnvironmentVariable("QSG_RENDER_LOOP", "threaded");
    if (renderLoop != "threaded" && renderLoop != "basic") usage();
    if (qEnvironmentVariableIsSet("M600_SA2_INJECT_UNCONFIRMED_DRAIN")) usage();
    options.out = QFileInfo(options.out).absoluteFilePath();
    options.dll = QFileInfo(options.dll).absoluteFilePath();
    options.framework.out = QDir(options.out).filePath("harness.json");
    options.framework.dll = options.dll;
    options.framework.debugLayer = options.gpuValidation;
}

void selectHighPerformanceAdapter() {
    using Microsoft::WRL::ComPtr;
    ComPtr<IDXGIFactory6> factory;
    ComPtr<IDXGIAdapter1> preferred;
    DXGI_ADAPTER_DESC1 wanted{};
    if (FAILED(CreateDXGIFactory2(0, IID_PPV_ARGS(&factory)))
        || FAILED(factory->EnumAdapterByGpuPreference(0, DXGI_GPU_PREFERENCE_HIGH_PERFORMANCE, IID_PPV_ARGS(&preferred)))
        || FAILED(preferred->GetDesc1(&wanted))) throw std::runtime_error("high-performance adapter unavailable");
    for (UINT index = 0;; ++index) {
        ComPtr<IDXGIAdapter1> adapter;
        if (factory->EnumAdapters1(index, &adapter) == DXGI_ERROR_NOT_FOUND) break;
        DXGI_ADAPTER_DESC1 actual{};
        if (!adapter || FAILED(adapter->GetDesc1(&actual))) throw std::runtime_error("adapter enumeration failed");
        if (actual.AdapterLuid.LowPart == wanted.AdapterLuid.LowPart && actual.AdapterLuid.HighPart == wanted.AdapterLuid.HighPart) {
            // Assumed for Qt 6.10.3's D3D12 backend; initialized() independently checks the actual device.
            qputenv("QT_D3D_ADAPTER_INDEX", QByteArray::number(index));
            return;
        }
    }
    throw std::runtime_error("adapter index unavailable");
}

namespace {
std::atomic<bool> frameFailed_{false};
QtMessageHandler previousHandler_ = nullptr;
void frameFailureHandler(QtMsgType type, const QMessageLogContext& context, const QString& message) {
    if (type != QtDebugMsg && type != QtInfoMsg && message.startsWith(QLatin1String("Failed to end frame"))) frameFailed_.store(true);
    if (previousHandler_) previousHandler_(type, context, message);
}
}
// Qt 6.10.3's render loops log this warning for every failed endFrame, a failed Present included, and still emit
// afterFrameEnd (FRAMEWORK-FACTS Q9), so it is the only sign of a frame that was not presented. Install it before
// QGuiApplication starts any thread; the previous handler, Qt's default one, still writes every message.
void watchFrameFailures() { previousHandler_ = qInstallMessageHandler(frameFailureHandler); }
bool frameFailed() { return frameFailed_.load(); }

L2::L2(L2Options value) : options(std::move(value)) {
    const auto optionalText = [](const QString& text) { return text.isEmpty() ? unknown : QJsonValue(text); };
    record_ = {{"format", "magic600-l2-harness-v1"}, {"candidate", "sd"}, {"mode", optionalText(options.mode)},
        {"run_id", optionalText(options.runId)}, {"scene", optionalText(options.scene)}, {"process_id", int(GetCurrentProcessId())},
        {"exit_code", 1}, {"reason", unknown},
        {"options", QJsonObject{{"trace_ms", int(options.traceMs)}, {"preroll_ms", int(options.prerollMs)}, {"turn_ms", options.turnMs},
            {"inject", optionalText(options.inject)}, {"declared", options.declared}, {"gpu_validation", options.gpuValidation},
            {"conditions", options.enforce ? "enforce" : "record"}, {"no_vram", options.noVram}, {"debug_half_target", options.halfTarget}}},
        {"configuration", QJsonObject{}}, {"files", QJsonObject{{"dll", optionalText(options.dll)}}},
        {"dll_identity", QJsonObject{{"dll", QJsonObject{{"file", unknown}, {"sha256", unknown}}}, {"shaders", unknown}}},
        {"dll_status", QJsonObject{{"abi_version", unknown}, {"last_status", unknown}, {"last_error", unknown}}}, {"qpc_frequency", unknown},
        {"sizes", QJsonObject{{"display", dimensions({})}, {"window", dimensions({})}, {"backbuffer", dimensions({})},
            {"displayed", dimensions({})}, {"target", dimensions({})}, {"samples", 0}, {"samples_changed", 0}}},
        {"scaling", QJsonObject{{"device_pixel_ratio", unknown}, {"item_width", unknown}, {"item_height", unknown}, {"texture_stretch", "none"}}},
        {"dpi_awareness", unknown},
        {"environment", QJsonObject{{"power_source", unknown}, {"power_mode", unknown}, {"presenting_adapter", unknown},
            {"presentation_interval", unknown}, {"declared", options.declared},
            {"display", QJsonObject{{"width", unknown}, {"height", unknown}, {"refresh_hz", unknown}}}, {"backbuffer", dimensions({})},
            {"power_samples", QJsonObject{{"samples", 0}, {"mains", 0}, {"battery", 0}}}, {"vsync", unknown}, {"adapter", unknown},
            {"driver", unknown}, {"msaa", unknown}, {"warp", unknown}}},
        {"window", QJsonObject{{"topmost", unknown}, {"display_required", unknown}, {"foreground_at_trace_start", unknown}, {"sample_period_ms", 100},
            {"samples", 0}, {"samples_not_visible", 0}, {"samples_covered", 0}, {"samples_not_foreground", 0},
            {"visible_throughout", unknown}, {"foreground_throughout", unknown}, {"presents", 0}}},
        {"debug", QJsonObject{{"enabled", options.gpuValidation}, {"debug_layer", unknown}, {"counts", unknown}, {"messages", unknown}}}};
    configuration();
}

void L2::configuration() {
    const auto& f = options.framework;
    QJsonObject config{{"framework", "qt"}, {"framework_version", qVersion()}, {"graphics_api", "d3d12"},
        {"window_mode", "fullscreen"}, {"vsync", "disabled"}, {"swap_interval", 0},
        {"render_loop", qEnvironmentVariable("QSG_RENDER_LOOP", "threaded")}, {"device", QString::fromStdString(f.device)},
        {"route", QString::fromStdString(f.route)}, {"queue", QString::fromStdString(f.queue)}, {"handover", QString::fromStdString(f.handover)},
        {"barriers", QString::fromStdString(f.barriers)}, {"warmup_frames", f.route == "export-copy" ? SA2_RING_SLOTS : 0}, {"timeout_ms", int(f.timeoutMs)}};
    const auto env = QProcessEnvironment::systemEnvironment();
    for (const auto& name : env.keys()) {
        if ((name.startsWith("QSG_") || name.startsWith("QT_")) && name != "QSG_RHI_DEBUG_LAYER") config.insert(name, env.value(name));
    }
    record_.insert("configuration", config);
}
void L2::failure(const QString& reason, int code) {
    if (record_["reason"].isNull() || exitCode == 2) record_.insert("reason", reason);
    if (exitCode != 3) exitCode = code;
}
void L2::outcome(int status, const QString& failedCheck) {
    if (status == SA2_E_CHECK_FAILED) { exitCode = 2; record_.insert("reason", failedCheck); }
    else if (status == SA2_OK && record_["reason"].isNull()) exitCode = 0;
    else failure("dll");
}
void L2::status(int value, const QString& error) {
    lastStatus_ = value; lastError_ = error;
}
void L2::abi(std::uint32_t version) {
    auto status = record_["dll_status"].toObject(); status.insert("abi_version", int(version)); record_.insert("dll_status", status);
}
void L2::identity(const QByteArray& json) {
    QJsonParseError error{};
    const auto document = QJsonDocument::fromJson(json, &error);
    if (error.error != QJsonParseError::NoError || !document.isObject() || !document.object()["dll"].isObject()
        || !document.object()["shaders"].isArray()) throw std::runtime_error("DLL identity malformed or truncated");
    record_.insert("dll_identity", document.object());
}
qint64 L2::qpc() {
    LARGE_INTEGER time{};
    if (!QueryPerformanceCounter(&time)) throw std::runtime_error("QPC unavailable");
    return time.QuadPart;
}
VOID WINAPI L2::onPowerMode(EFFECTIVE_POWER_MODE mode, VOID* context) { static_cast<L2*>(context)->powerMode_.store(int(mode)); }

void L2::initialized(QRhi* rhi, const sa2_device_info& info) {
    auto debug = record_["debug"].toObject(); debug.insert("debug_layer", info.debug_layer); record_.insert("debug", debug);
    LARGE_INTEGER frequency{};
    if (!QueryPerformanceFrequency(&frequency) || !frequency.QuadPart) throw std::runtime_error("QPC frequency unavailable");
    frequency_ = frequency.QuadPart; record_.insert("qpc_frequency", frequency_);
    const auto awareness = GetThreadDpiAwarenessContext();
    const bool perMonitorV2 = AreDpiAwarenessContextsEqual(awareness, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    record_.insert("dpi_awareness", perMonitorV2 ? "per-monitor-v2" : "other");
    if (!perMonitorV2) throw std::runtime_error("render thread must be per-monitor-v2 aware");
    displayRequired_ = SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED) != 0;
    auto window = record_["window"].toObject();
    window.insert("display_required", displayRequired_); window.insert("topmost", bool(GetWindowLongPtrW(hwnd, GWL_EXSTYLE) & WS_EX_TOPMOST));
    record_.insert("window", window);
    if (!displayRequired_) throw std::runtime_error("cannot keep display on");
    if (!window["topmost"].toBool()) throw std::runtime_error("window must be topmost");
    if (options.mode == "run"
        && FAILED(PowerRegisterForEffectivePowerModeNotifications(EFFECTIVE_POWER_MODE_V2, &L2::onPowerMode, this, &powerRegistration_))) powerRegistration_ = nullptr;
    auto environment = record_["environment"].toObject();
    const auto adapter = QString::fromUtf8(rhi->driverInfo().deviceName);
    const auto version = info.umd_version;
    const auto driver = QString("%1.%2.%3.%4").arg((version >> 48) & 65535).arg((version >> 32) & 65535).arg((version >> 16) & 65535).arg(version & 65535);
    environment.insert("adapter", adapter); environment.insert("driver", driver);
    environment.insert("presentation_interval", 0); environment.insert("vsync", false); environment.insert("msaa", 1);
    environment.insert("warp", info.vendor_id == 0x1414 && info.device_id == 0x8c);
    MONITORINFOEXW monitor{}; monitor.cbSize = sizeof(monitor);
    if (!GetMonitorInfoW(MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY), &monitor)) throw std::runtime_error("window monitor unavailable");
    bool drives = false;
    for (DWORD index = 0;; ++index) {
        DISPLAY_DEVICEW device{}; device.cb = sizeof(device);
        if (!EnumDisplayDevicesW(nullptr, index, &device, 0)) break;
        if (QString::fromWCharArray(device.DeviceName) == QString::fromWCharArray(monitor.szDevice)) {
            drives = QString::fromWCharArray(device.DeviceString) == adapter; break;
        }
    }
    environment.insert("presenting_adapter", adapter + ", driver " + driver
        + (drives ? ", drives the window's display" : ", display driven by another adapter"));
    record_.insert("environment", environment);
}

QSize L2::displayed(QQuickWindow* window) const {
    const auto ratio = window->effectiveDevicePixelRatio();
    return {qRound(itemSize_.width() * ratio), qRound(itemSize_.height() * ratio)};
}
QJsonObject L2::sizes(QQuickWindow* window, QSize target) {
    MONITORINFOEXW monitor{}; monitor.cbSize = sizeof(monitor);
    DEVMODEW mode{}; mode.dmSize = sizeof(mode);
    RECT client{};
    if (!GetMonitorInfoW(MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY), &monitor)
        || !EnumDisplaySettingsW(monitor.szDevice, ENUM_CURRENT_SETTINGS, &mode) || !GetClientRect(hwnd, &client))
        throw std::runtime_error("window sizes unavailable");
    const auto* swapChain = window->swapChain(); // Only this render-thread path reads the swap chain.
    const auto backbuffer = swapChain ? swapChain->currentPixelSize() : QSize();
    auto environment = record_["environment"].toObject();
    environment.insert("display", QJsonObject{{"width", int(mode.dmPelsWidth)}, {"height", int(mode.dmPelsHeight)}, {"refresh_hz", int(mode.dmDisplayFrequency)}});
    environment.insert("backbuffer", dimensions(backbuffer)); record_.insert("environment", environment);
    return {{"display", dimensions({int(mode.dmPelsWidth), int(mode.dmPelsHeight)})},
        {"window", dimensions({client.right - client.left, client.bottom - client.top})}, {"backbuffer", dimensions(backbuffer)},
        {"displayed", dimensions(displayed(window))}, {"target", dimensions(target)}};
}
bool L2::finalSize(QQuickWindow* window) {
    const auto current = sizes(window, displayed(window));
    for (const auto* key : {"window", "backbuffer", "displayed"}) if (current[key] != current["display"]) return false;
    return !displayed(window).isEmpty();
}
bool L2::readyToTrace() {
    const auto now = qpc();
    if (!firstProduce || (now - firstProduce) * 1000 < qint64(options.prerollMs) * frequency_) return false;
    if (GetForegroundWindow() == hwnd) return true;
    if (!foregroundWait_) foregroundWait_ = now;
    if (now - foregroundWait_ < 5 * frequency_) return false;
    auto window = record_["window"].toObject(); window.insert("foreground_at_trace_start", false); record_.insert("window", window);
    if (options.enforce) failure("foreground", 3);
    return !options.enforce;
}
void L2::beginTrace(QQuickWindow* window, QSize target) {
    traceActive = true; traceStart = qpc(); nextSample_ = traceStart;
    captureSizes(window, target);
    auto facts = record_["window"].toObject(); facts.insert("foreground_at_trace_start", GetForegroundWindow() == hwnd); record_.insert("window", facts);
}
void L2::captureSizes(QQuickWindow* window, QSize target) {
    initialSizes_ = sizes(window, target);
    auto facts = initialSizes_; facts.insert("samples", number(samples_)); facts.insert("samples_changed", number(changedSizes_));
    record_.insert("sizes", facts);
    record_.insert("scaling", QJsonObject{{"device_pixel_ratio", window->effectiveDevicePixelRatio()},
        {"item_width", itemSize_.width()}, {"item_height", itemSize_.height()}, {"texture_stretch", "none"}});
}
bool L2::sample(QQuickWindow* window, QSize target) {
    const auto now = qpc();
    if (!traceActive || now < nextSample_) return true;
    nextSample_ = now + frequency_ / 10;
    BOOL cloaked = FALSE;
    const bool hidden = SUCCEEDED(DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &cloaked, sizeof(cloaked))) && cloaked;
    const bool isCovered = covered(hwnd);
    const bool visible = IsWindowVisible(hwnd) && !IsIconic(hwnd) && !hidden && !isCovered;
    const bool foreground = GetForegroundWindow() == hwnd;
    const int mode = powerMode_.load();
    if (!samples_) powerModeAtStart_ = mode; else if (mode != powerModeAtStart_) powerChanged_ = true;
    ++samples_;
    if (isCovered) ++covered_;
    if (!visible) ++notVisible_;
    if (!foreground) ++notForeground_;
    SYSTEM_POWER_STATUS power{};
    if (GetSystemPowerStatus(&power)) { if (power.ACLineStatus == 1) ++mains_; else if (power.ACLineStatus == 0) ++battery_; }
    if (sizes(window, target) != initialSizes_) ++changedSizes_;
    updateConditions();
    if (options.enforce && (!visible || !foreground)) { failure(!visible ? "visibility" : "foreground", 3); return false; }
    return true;
}
void L2::updateConditions() {
    auto environment = record_["environment"].toObject();
    const auto source = samples_ && mains_ == samples_ ? "mains" : samples_ && battery_ == samples_ ? "battery"
        : mains_ && battery_ && mains_ + battery_ == samples_ ? "changed" : "unknown";
    environment.insert("power_source", source); environment.insert("power_mode", powerChanged_ ? QString("changed") : powerModeName(powerModeAtStart_));
    environment.insert("power_samples", QJsonObject{{"samples", number(samples_)}, {"mains", number(mains_)}, {"battery", number(battery_)}});
    record_.insert("environment", environment);
    auto window = record_["window"].toObject();
    window.insert("samples", number(samples_)); window.insert("samples_not_visible", number(notVisible_));
    window.insert("samples_covered", number(covered_)); window.insert("samples_not_foreground", number(notForeground_));
    window.insert("visible_throughout", samples_ ? QJsonValue(notVisible_ == 0) : unknown);
    window.insert("foreground_throughout", samples_ ? QJsonValue(notForeground_ == 0) : unknown);
    window.insert("presents", number(presents_)); record_.insert("window", window);
    auto sizes = record_["sizes"].toObject(); sizes.insert("samples", number(samples_)); sizes.insert("samples_changed", number(changedSizes_)); record_.insert("sizes", sizes);
}
bool L2::endDue() const { return traceActive && (qpc() - traceStart) * 1000 >= qint64(options.traceMs) * frequency_; }
void L2::presented(bool inTrace) { if (inTrace) ++presents_; }
void L2::debug(const struct sa2_debug_counts& counts, const QString& messages) {
    QJsonArray ids;
    for (const auto id : counts.ids) ids.append(id);
    auto debug = record_["debug"].toObject();
    debug.insert("counts", QJsonObject{{"struct_size", int(counts.struct_size)}, {"distinct_id_count", int(counts.distinct_id_count)},
        {"corruption", number(counts.corruption)}, {"error", number(counts.error)}, {"warning", number(counts.warning)},
        {"info", number(counts.info)}, {"message", number(counts.message)}, {"mismatching_clear_value", number(counts.mismatching_clear_value)},
        {"mentioning_sa2", number(counts.mentioning_sa2)}, {"ids", ids}});
    debug.insert("messages", messages); record_.insert("debug", debug);
}
void L2::releaseConditions() {
    if (powerRegistration_) { PowerUnregisterFromEffectivePowerModeNotifications(powerRegistration_); powerRegistration_ = nullptr; }
    if (displayRequired_) { SetThreadExecutionState(ES_CONTINUOUS); displayRequired_ = false; }
}
void L2::files() {
    const auto exe = executablePath();
    const auto directory = QFileInfo(exe).absolutePath() + '/';
    auto files = record_["files"].toObject(); files.insert("qt:exe", exe);
    std::vector<HMODULE> modules(256);
    DWORD required = 0;
    if (!EnumProcessModules(GetCurrentProcess(), modules.data(), DWORD(modules.size() * sizeof(HMODULE)), &required)) throw std::runtime_error("module enumeration failed");
    if (required > modules.size() * sizeof(HMODULE)) {
        modules.resize(required / sizeof(HMODULE));
        if (!EnumProcessModules(GetCurrentProcess(), modules.data(), required, &required) || required > modules.size() * sizeof(HMODULE))
            throw std::runtime_error("module enumeration changed");
    }
    modules.resize(required / sizeof(HMODULE));
    for (const auto module : modules) {
        wchar_t path[32768]{};
        const auto length = GetModuleFileNameW(module, path, DWORD(std::size(path)));
        if (!length || length >= std::size(path)) throw std::runtime_error("module path unavailable");
        const auto file = QDir::cleanPath(QString::fromWCharArray(path));
        if (file.compare(options.dll, Qt::CaseInsensitive) == 0 || file.compare(exe, Qt::CaseInsensitive) == 0) continue;
        if (file.startsWith(directory, Qt::CaseInsensitive)) {
            const auto key = "qt:" + QFileInfo(file).fileName();
            if (files.contains(key) && files[key].toString().compare(file, Qt::CaseInsensitive) != 0) throw std::runtime_error("duplicate module basename");
            files.insert(key, file);
        }
    }
    record_.insert("files", files);
}
bool L2::write() {
    if (!outputDirectory(options.out, false)) return false;
    try { files(); } catch (const std::exception&) { failure("framework-modules"); }
    if (samples_ || presents_) updateConditions();
    if (lastStatus_) {
        auto status = record_["dll_status"].toObject(); status.insert("last_status", *lastStatus_); status.insert("last_error", lastError_);
        record_.insert("dll_status", status);
    }
    record_.insert("exit_code", exitCode);
    if (exitCode == 0) record_.insert("reason", unknown);
    const QDir directory(options.out);
    const auto temporary = directory.filePath("harness.json.tmp"), destination = directory.filePath("harness.json");
    QFile output(temporary);
    if (!output.open(QIODevice::WriteOnly | QIODevice::NewOnly)) return false;
    const auto bytes = QJsonDocument(record_).toJson(QJsonDocument::Indented);
    const bool written = output.write(bytes) == bytes.size() && output.flush();
    output.close();
    if (written && MoveFileExW(reinterpret_cast<LPCWSTR>(temporary.utf16()), reinterpret_cast<LPCWSTR>(destination.utf16()), MOVEFILE_WRITE_THROUGH)) return true;
    QFile::remove(temporary); // Only the file this invocation created; MoveFileEx never replaces a destination.
    return false;
}
}
