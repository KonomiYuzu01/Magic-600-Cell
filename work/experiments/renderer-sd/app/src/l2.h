#pragma once
#include "smoke.h"
#include "module_observer.h"
#include <QtCore/QJsonObject>
#include <atomic>
#include <powersetting.h>

namespace sd {
struct L2Options {
    Options framework;
    QString mode, scene, out, runId, dll, inject;
    std::uint32_t traceMs = 192000, prerollMs = 4000;
    double turnMs = 190;
    QJsonObject declared;
    bool gpuValidation = false, enforce = true, noVram = false, halfTarget = false;
    L2Options() { framework.route = "import-direct"; }
};
void parseL2(int argc, char** argv, L2Options& options);
QString usableL2Out(int argc, char** argv);
void selectHighPerformanceAdapter();
void watchFrameFailures();
bool frameFailed();

// Written on the render thread; the GUI reads the report only after window deletion joins it.
class L2 {
public:
    explicit L2(L2Options options);
    L2Options options;
    HWND hwnd = nullptr;
    bool traceActive = false, traceCompleted = false;
    qint64 firstProduce = 0, traceStart = 0;
    int exitCode = 1;
    void configuration();
    void failure(const QString& reason, int code = 1);
    void outcome(int status, const QString& failedCheck);
    void status(int value, const QString& error);
    void abi(std::uint32_t version);
    void identity(const QByteArray& json);
    void loadedDll(const Native& native);
    bool injectModuleProbe(); // GUI timer only; waits for the render thread's trace-begin publication.
    void initialized(QRhi* rhi, const sa2_device_info& info);
    void layout(QSizeF size) { itemSize_ = size; }
    QSize displayed(QQuickWindow* window) const;
    bool finalSize(QQuickWindow* window);
    bool readyToTrace();
    void captureSizes(QQuickWindow* window, QSize target);
    void beginTrace(QQuickWindow* window, QSize target);
    bool sample(QQuickWindow* window, QSize target);
    bool endDue() const;
    void presented(bool inTrace);
    void debug(const struct sa2_debug_counts& counts, const QString& messages);
    void releaseConditions();
    bool write();
    static qint64 qpc();
private:
    QJsonObject record_, initialSizes_;
    QSizeF itemSize_;
    qint64 frequency_ = 0, foregroundWait_ = 0, nextSample_ = 0;
    std::uint64_t samples_ = 0, mains_ = 0, battery_ = 0, changedSizes_ = 0;
    std::uint64_t notVisible_ = 0, covered_ = 0, notForeground_ = 0, presents_ = 0;
    std::atomic<int> powerMode_{-1};
    int powerModeAtStart_ = -1;
    bool powerChanged_ = false, displayRequired_ = false;
    void* powerRegistration_ = nullptr;
    std::optional<int> lastStatus_;
    QString lastError_;
    QString moduleInjection_;
    std::atomic<bool> traceStarted_{false};
    DWORD guiThread_ = GetCurrentThreadId();
    bool moduleInjected_ = false;
    QJsonObject sizes(QQuickWindow* window, QSize target);
    void updateConditions();
    void files(const modules::Record& observed);
    static VOID WINAPI onPowerMode(EFFECTIVE_POWER_MODE mode, VOID* context);
};
}
