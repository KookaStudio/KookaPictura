#pragma once

#include <QtCore/QList>
#include <QtCore/QString>
#include <QtCore/QStringList>

#include <optional>

namespace pictura {

// Raw text of every source one refresh reads, so tests feed fixtures and the
// sampler never touches /proc itself.
struct DebugTaskReading {
    int tid = 0;
    QString stat;
    QString status;
};

struct DebugReadings {
    QString status;
    QString processStat;
    QString memInfo;
    QString swaps;
    QString vmStat;
    QString cpuStat;
    QString loadAvg;
    QList<DebugTaskReading> tasks;
    QString io;
    int openFds = 0;
    double uptimeSeconds = 0.0;
    // /proc/self/fdinfo text of every open /dev/dri/* descriptor.
    QStringList drmFdinfo;
    // Sysfs text of the first GPU with video-memory counters.
    QString gpuBusy;
    QString vramUsed;
    QString vramTotal;
    int pid = 0;
    qint64 monotonicNs = 0;
};

struct JobInFlight {
    QString kind;
    double seconds = 0.0;
};

// This process's share of the GPU, from its DRM clients.
struct ProcessGpu {
    bool present = false;
    quint64 vramBytes = 0;
    std::optional<double> enginePercent;
};

struct SystemGpu {
    bool present = false;
    std::optional<double> busyPercent;
    quint64 vramUsed = 0;
    quint64 vramTotal = 0;
};

// The memory governor's status report (`memory_status_json`).
struct MemoryBudget {
    bool present = false;
    std::optional<quint64> used;
    std::optional<quint64> ceiling;
    quint64 reserved = 0;
    QString pressure;
    quint64 scratchUsed = 0;
    quint64 scratchAvailable = 0;
    quint64 packed = 0;
    quint64 packedBytes = 0;
    quint64 spilled = 0;
    quint64 backed = 0;
};

struct ThreadGroupUse {
    QString group;
    int threads = 0;
    double cpuPercent = 0.0;
};

// Rates are empty until a second sample exists.
struct DebugSnapshot {
    quint64 processRss = 0;
    quint64 processSwap = 0;
    int processThreads = 0;
    std::optional<double> processCpu;
    std::optional<QList<ThreadGroupUse>> groups;
    quint64 memTotal = 0;
    quint64 memUsed = 0;
    quint64 memCached = 0;
    quint64 swapTotal = 0;
    quint64 swapUsed = 0;
    QString swapLocation;
    std::optional<double> swapInBytesPerSec;
    std::optional<double> swapOutBytesPerSec;
    std::optional<QList<double>> cpuPercents;
    double load1 = 0.0;
    double load5 = 0.0;
    double load15 = 0.0;
    int openFds = 0;
    quint64 rssAnon = 0;
    quint64 rssFile = 0;
    std::optional<double> minorFaultsPerSec;
    std::optional<double> majorFaultsPerSec;
    std::optional<double> ioReadBytesPerSec;
    std::optional<double> ioWriteBytesPerSec;
    std::optional<double> ctxVoluntaryPerSec;
    std::optional<double> ctxInvoluntaryPerSec;
    QList<JobInFlight> jobs;
    ProcessGpu gpu;
    SystemGpu systemGpu;
};

// Read the live /proc sources for the calling process.
DebugReadings readDebugSources();

// Turns successive readings into snapshots; the previous reading is the only
// state, so dropping the sampler drops everything it retained.
class DebugSampler {
public:
    DebugSnapshot sample(const DebugReadings& readings);
    bool hasPrevious() const { return hasPrevious_; }

private:
    DebugReadings previous_;
    bool hasPrevious_ = false;
};

// "GUI", "Rayon workers", "Jobs" or "Other" for one task.
QString threadGroupName(const QString& comm, bool isMainThread);

// The last GPU time Pictura measured out of a `metrics_snapshot` JSON, or
// "not measured" when the snapshot is empty or has none.
QString gpuRowText(const QString& metricsJson);

// "p50 25.0 ms   p95 38.5 ms   last 40.0 ms (4)" of the metrics snapshot's recent
// edits (`edits` true, `wall_ms`) or canvas paints (`frames`, `paint_ms`), or
// "not measured" when there are none.
QString timingText(const QString& metricsJson, bool edits);

MemoryBudget parseMemoryBudget(const QString& json);

// Pure parsers, exposed for tests.
quint64 parseKeyedCounter(const QString& text, const QString& key);
bool parseProcessFaults(const QString& stat, quint64* minor, quint64* major);
double parseTaskStartSeconds(const QString& stat);
quint64 parseStatusKilobytes(const QString& status, const QString& key);
int parseStatusThreads(const QString& status);
quint64 parseProcessStatTicks(const QString& stat);
QString parseTaskComm(const QString& stat);
QList<std::pair<quint64, quint64>> parsePerCpuTimes(const QString& stat);
QString parseSwapLocation(const QString& swaps);
quint64 parseVmStatCounter(const QString& vmStat, const QString& key);
bool parseLoadAverage(const QString& text, double* one, double* five, double* fifteen);

} // namespace pictura
