#include "debug_toolset.h"

#include <QtCore/QDir>
#include <QtCore/QElapsedTimer>
#include <QtCore/QFile>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QRegularExpression>
#include <QtCore/QSet>

#include <unistd.h>

#include <algorithm>
#include <cmath>
#include <map>

namespace pictura {

namespace {

constexpr char kMissing[] = "not measured";

QString readText(const QString& path)
{
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        return {};
    }
    return QString::fromUtf8(file.readAll());
}

// First number after `key` at the start of a line (0 when absent).
quint64 keyedValue(const QString& text, const QString& key)
{
    for (const QString& line : text.split(QLatin1Char('\n'))) {
        if (line.startsWith(key)) {
            const QStringList fields = line.mid(key.size()).split(
                QRegularExpression(QStringLiteral("\\s+")), Qt::SkipEmptyParts);
            return fields.value(0).toULongLong();
        }
    }
    return 0;
}

double clockTicksPerSecond()
{
    const long ticks = ::sysconf(_SC_CLK_TCK);
    return ticks > 0 ? static_cast<double>(ticks) : 100.0;
}

struct DrmTotals {
    bool present = false;
    quint64 vramBytes = 0;
    quint64 engineNs = 0;
};

quint64 sizeWithUnit(const QString& value)
{
    const QStringList parts = value.split(QRegularExpression(QStringLiteral("\\s+")), Qt::SkipEmptyParts);
    quint64 scale = 1;
    const QString unit = parts.value(1);
    if (unit == QLatin1String("KiB")) {
        scale = 1024;
    } else if (unit == QLatin1String("MiB")) {
        scale = 1024ull * 1024;
    } else if (unit == QLatin1String("GiB")) {
        scale = 1024ull * 1024 * 1024;
    }
    return parts.value(0).toULongLong() * scale;
}

// Each DRM client is counted once however many descriptors name it.
DrmTotals drmTotals(const QStringList& infos)
{
    DrmTotals totals;
    QSet<QString> seen;
    for (int i = 0; i < infos.size(); ++i) {
        QString id = QStringLiteral("anonymous-%1").arg(i);
        quint64 vram = 0;
        quint64 engine = 0;
        for (const QString& line : infos.at(i).split(QLatin1Char('\n'))) {
            const int colon = line.indexOf(QLatin1Char(':'));
            if (colon < 0) {
                continue;
            }
            const QString key = line.left(colon);
            const QString value = line.mid(colon + 1).trimmed();
            if (key == QLatin1String("drm-client-id")) {
                id = value;
            } else if (key == QLatin1String("drm-memory-vram")) {
                vram = sizeWithUnit(value);
            } else if (key.startsWith(QLatin1String("drm-engine-"))
                       && !key.startsWith(QLatin1String("drm-engine-capacity-"))) {
                engine += value.split(QLatin1Char(' ')).value(0).toULongLong();
            }
        }
        if (seen.contains(id)) {
            continue;
        }
        seen.insert(id);
        totals.present = true;
        totals.vramBytes += vram;
        totals.engineNs += engine;
    }
    return totals;
}

QString jobKind(const QString& comm)
{
    if (comm.startsWith(QLatin1String("pictura-open"))) {
        return QStringLiteral("Open");
    }
    if (comm.startsWith(QLatin1String("pictura-save"))) {
        return QStringLiteral("Save");
    }
    if (comm.startsWith(QLatin1String("pictura-filter"))) {
        return QStringLiteral("Filter");
    }
    if (comm.startsWith(QLatin1String("pictura-doc"))) {
        return QStringLiteral("Document");
    }
    return {};
}

double percentile(QList<double> sorted, double fraction)
{
    std::sort(sorted.begin(), sorted.end());
    const double position = fraction * (sorted.size() - 1);
    const int low = static_cast<int>(std::floor(position));
    const int high = std::min<int>(low + 1, static_cast<int>(sorted.size()) - 1);
    return sorted.at(low) + (sorted.at(high) - sorted.at(low)) * (position - low);
}

quint64 sumTaskCounter(const DebugReadings& readings, const QString& key)
{
    quint64 total = 0;
    for (const DebugTaskReading& task : readings.tasks) {
        total += keyedValue(task.status, key);
    }
    return total;
}

} // namespace

QString timingText(const QString& metricsJson, bool edits)
{
    const QString missing = QString::fromLatin1(kMissing);
    if (metricsJson.isEmpty()) {
        return missing;
    }
    const QJsonObject root = QJsonDocument::fromJson(metricsJson.toUtf8()).object();
    const QJsonArray records = root.value(edits ? QStringLiteral("edits") : QStringLiteral("frames")).toArray();
    const QString key = edits ? QStringLiteral("wall_ms") : QStringLiteral("paint_ms");
    QList<double> values;
    for (const QJsonValue& record : records) {
        const QJsonValue value = record.toObject().value(key);
        if (value.isDouble()) {
            values.append(value.toDouble());
        }
    }
    if (values.isEmpty()) {
        return missing;
    }
    return QStringLiteral("p50 %1 ms   p95 %2 ms   last %3 ms (%4)")
        .arg(percentile(values, 0.5), 0, 'f', 1)
        .arg(percentile(values, 0.95), 0, 'f', 1)
        .arg(values.last(), 0, 'f', 1)
        .arg(values.size());
}

MemoryBudget parseMemoryBudget(const QString& json)
{
    MemoryBudget budget;
    const QJsonObject root = QJsonDocument::fromJson(json.toUtf8()).object();
    if (!root.contains(QStringLiteral("pressure"))) {
        return budget;
    }
    budget.present = true;
    const auto optional = [&](const QString& key) -> std::optional<quint64> {
        const QJsonValue value = root.value(key);
        return value.isDouble() ? std::optional<quint64>(static_cast<quint64>(value.toDouble()))
                                : std::nullopt;
    };
    budget.used = optional(QStringLiteral("used"));
    budget.ceiling = optional(QStringLiteral("ceiling"));
    budget.reserved = static_cast<quint64>(root.value(QStringLiteral("reserved")).toDouble());
    budget.pressure = root.value(QStringLiteral("pressure")).toString();
    const QJsonObject scratch = root.value(QStringLiteral("scratch")).toObject();
    budget.scratchUsed = static_cast<quint64>(scratch.value(QStringLiteral("used")).toDouble());
    budget.scratchAvailable = static_cast<quint64>(scratch.value(QStringLiteral("available")).toDouble());
    const QJsonObject tiles = root.value(QStringLiteral("tiles")).toObject();
    budget.packed = static_cast<quint64>(tiles.value(QStringLiteral("packed")).toDouble());
    budget.packedBytes = static_cast<quint64>(tiles.value(QStringLiteral("packed_bytes")).toDouble());
    budget.spilled = static_cast<quint64>(tiles.value(QStringLiteral("spilled")).toDouble());
    budget.backed = static_cast<quint64>(tiles.value(QStringLiteral("backed")).toDouble());
    return budget;
}

quint64 parseStatusKilobytes(const QString& status, const QString& key)
{
    return keyedValue(status, key + QLatin1Char(':')) * 1024;
}

int parseStatusThreads(const QString& status)
{
    return static_cast<int>(keyedValue(status, QStringLiteral("Threads:")));
}

quint64 parseProcessStatTicks(const QString& stat)
{
    const int close = stat.lastIndexOf(QLatin1Char(')'));
    if (close < 0) {
        return 0;
    }
    const QStringList fields = stat.mid(close + 2).split(QLatin1Char(' '), Qt::SkipEmptyParts);
    // Past the comm field index 0 is state (field 3): utime is field 14, stime 15.
    if (fields.size() < 13) {
        return 0;
    }
    return fields.at(11).toULongLong() + fields.at(12).toULongLong();
}

QString parseTaskComm(const QString& stat)
{
    const int open = stat.indexOf(QLatin1Char('('));
    const int close = stat.lastIndexOf(QLatin1Char(')'));
    if (open < 0 || close <= open) {
        return {};
    }
    return stat.mid(open + 1, close - open - 1);
}

quint64 parseKeyedCounter(const QString& text, const QString& key)
{
    return keyedValue(text, key);
}

bool parseProcessFaults(const QString& stat, quint64* minor, quint64* major)
{
    const int close = stat.lastIndexOf(QLatin1Char(')'));
    if (close < 0) {
        return false;
    }
    const QStringList fields = stat.mid(close + 2).split(QLatin1Char(' '), Qt::SkipEmptyParts);
    // Past the comm field index 7 is minflt (field 10) and 9 is majflt (field 12).
    if (fields.size() < 10) {
        return false;
    }
    *minor = fields.at(7).toULongLong();
    *major = fields.at(9).toULongLong();
    return true;
}

double parseTaskStartSeconds(const QString& stat)
{
    const int close = stat.lastIndexOf(QLatin1Char(')'));
    if (close < 0) {
        return 0.0;
    }
    const QStringList fields = stat.mid(close + 2).split(QLatin1Char(' '), Qt::SkipEmptyParts);
    // starttime is field 22: index 19 once state (field 3) is index 0.
    return fields.size() > 19 ? fields.at(19).toDouble() / clockTicksPerSecond() : 0.0;
}

QList<std::pair<quint64, quint64>> parsePerCpuTimes(const QString& stat)
{
    QList<std::pair<quint64, quint64>> cpus;
    for (const QString& line : stat.split(QLatin1Char('\n'))) {
        // "cpu " is the aggregate; "cpuN" lines are the individual CPUs.
        if (!line.startsWith(QStringLiteral("cpu")) || line.size() < 4
            || !line.at(3).isDigit()) {
            continue;
        }
        const QStringList fields = line.split(QLatin1Char(' '), Qt::SkipEmptyParts);
        quint64 total = 0;
        for (int i = 1; i < fields.size(); ++i) {
            total += fields.at(i).toULongLong();
        }
        const quint64 idle = fields.value(4).toULongLong() + fields.value(5).toULongLong();
        cpus.append({total, idle});
    }
    return cpus;
}

QString parseSwapLocation(const QString& swaps)
{
    QStringList paths;
    const QStringList lines = swaps.split(QLatin1Char('\n'));
    for (int i = 1; i < lines.size(); ++i) {
        const QString path = lines.at(i).split(QLatin1Char(' '), Qt::SkipEmptyParts).value(0);
        if (!path.isEmpty()) {
            paths.append(path);
        }
    }
    return paths.join(QStringLiteral(", "));
}

quint64 parseVmStatCounter(const QString& vmStat, const QString& key)
{
    return keyedValue(vmStat, key + QLatin1Char(' '));
}

bool parseLoadAverage(const QString& text, double* one, double* five, double* fifteen)
{
    const QStringList fields = text.split(QLatin1Char(' '), Qt::SkipEmptyParts);
    if (fields.size() < 3) {
        return false;
    }
    *one = fields.at(0).toDouble();
    *five = fields.at(1).toDouble();
    *fifteen = fields.at(2).toDouble();
    return true;
}

QString threadGroupName(const QString& comm, bool isMainThread)
{
    if (isMainThread) {
        return QStringLiteral("GUI");
    }
    if (comm.startsWith(QStringLiteral("pictura-rayon"))) {
        return QStringLiteral("Rayon workers");
    }
    for (const char* job : {"pictura-open", "pictura-save", "pictura-filter", "pictura-doc"}) {
        if (comm.startsWith(QLatin1String(job))) {
            return QStringLiteral("Jobs");
        }
    }
    return QStringLiteral("Other");
}

QString gpuRowText(const QString& metricsJson)
{
    const QString missing = QString::fromLatin1(kMissing);
    if (metricsJson.isEmpty()) {
        return missing;
    }
    const QJsonObject root = QJsonDocument::fromJson(metricsJson.toUtf8()).object();
    const QJsonArray frames = root.value(QStringLiteral("frames")).toArray();
    for (int i = frames.size() - 1; i >= 0; --i) {
        const QJsonValue gpu = frames.at(i).toObject().value(QStringLiteral("gpu_composite_ms"));
        if (gpu.isDouble()) {
            return QStringLiteral("%1 ms (last frame)").arg(gpu.toDouble(), 0, 'f', 2);
        }
    }
    const QJsonArray edits = root.value(QStringLiteral("edits")).toArray();
    for (int i = edits.size() - 1; i >= 0; --i) {
        const QJsonValue gpu = edits.at(i).toObject().value(QStringLiteral("gpu_ms"));
        if (gpu.isDouble()) {
            return QStringLiteral("%1 ms (last edit)").arg(gpu.toDouble(), 0, 'f', 2);
        }
    }
    return missing;
}

DebugReadings readDebugSources()
{
    static const QElapsedTimer clock = [] {
        QElapsedTimer timer;
        timer.start();
        return timer;
    }();
    DebugReadings r;
    r.pid = static_cast<int>(::getpid());
    r.monotonicNs = clock.nsecsElapsed();
    r.status = readText(QStringLiteral("/proc/self/status"));
    r.processStat = readText(QStringLiteral("/proc/self/stat"));
    r.memInfo = readText(QStringLiteral("/proc/meminfo"));
    r.swaps = readText(QStringLiteral("/proc/swaps"));
    r.vmStat = readText(QStringLiteral("/proc/vmstat"));
    r.cpuStat = readText(QStringLiteral("/proc/stat"));
    r.loadAvg = readText(QStringLiteral("/proc/loadavg"));
    r.io = readText(QStringLiteral("/proc/self/io"));
    r.uptimeSeconds = readText(QStringLiteral("/proc/uptime")).split(QLatin1Char(' ')).value(0).toDouble();
    const QDir fdDir(QStringLiteral("/proc/self/fd"));
    const QStringList fds = fdDir.entryList(QDir::AllEntries | QDir::System | QDir::NoDotAndDotDot);
    // The listing itself holds one descriptor open.
    r.openFds = std::max(0, static_cast<int>(fds.size()) - 1);
    for (const QString& fd : fds) {
        if (QFile::symLinkTarget(fdDir.filePath(fd)).startsWith(QLatin1String("/dev/dri/"))) {
            r.drmFdinfo.append(readText(QStringLiteral("/proc/self/fdinfo/") + fd));
        }
    }
    const QDir drm(QStringLiteral("/sys/class/drm"));
    const QStringList cards = drm.entryList({QStringLiteral("card*")}, QDir::Dirs | QDir::NoDotAndDotDot);
    static const QRegularExpression cardName(QStringLiteral("^card\\d+$"));
    for (const QString& card : cards) {
        if (!cardName.match(card).hasMatch()) {
            continue;
        }
        const QString device = drm.filePath(card) + QStringLiteral("/device/");
        r.vramTotal = readText(device + QStringLiteral("mem_info_vram_total"));
        if (r.vramTotal.isEmpty()) {
            continue;
        }
        r.vramUsed = readText(device + QStringLiteral("mem_info_vram_used"));
        r.gpuBusy = readText(device + QStringLiteral("gpu_busy_percent"));
        break;
    }
    const QDir taskDir(QStringLiteral("/proc/self/task"));
    for (const QString& entry : taskDir.entryList(QDir::Dirs | QDir::NoDotAndDotDot)) {
        bool ok = false;
        const int tid = entry.toInt(&ok);
        if (ok) {
            r.tasks.append({tid, readText(taskDir.filePath(entry) + QStringLiteral("/stat")),
                            readText(taskDir.filePath(entry) + QStringLiteral("/status"))});
        }
    }
    return r;
}

DebugSnapshot DebugSampler::sample(const DebugReadings& now)
{
    DebugSnapshot s;
    s.processRss = parseStatusKilobytes(now.status, QStringLiteral("VmRSS"));
    s.processSwap = parseStatusKilobytes(now.status, QStringLiteral("VmSwap"));
    s.processThreads = parseStatusThreads(now.status);

    s.memTotal = parseStatusKilobytes(now.memInfo, QStringLiteral("MemTotal"));
    const quint64 available = parseStatusKilobytes(now.memInfo, QStringLiteral("MemAvailable"));
    s.memUsed = s.memTotal > available ? s.memTotal - available : 0;
    s.memCached = parseStatusKilobytes(now.memInfo, QStringLiteral("Cached"));
    s.swapTotal = parseStatusKilobytes(now.memInfo, QStringLiteral("SwapTotal"));
    const quint64 swapFree = parseStatusKilobytes(now.memInfo, QStringLiteral("SwapFree"));
    s.swapUsed = s.swapTotal > swapFree ? s.swapTotal - swapFree : 0;
    s.swapLocation = parseSwapLocation(now.swaps);
    parseLoadAverage(now.loadAvg, &s.load1, &s.load5, &s.load15);

    s.openFds = now.openFds;
    s.rssAnon = parseStatusKilobytes(now.status, QStringLiteral("RssAnon"));
    s.rssFile = parseStatusKilobytes(now.status, QStringLiteral("RssFile"));
    for (const DebugTaskReading& task : now.tasks) {
        const QString kind = jobKind(parseTaskComm(task.stat));
        if (!kind.isEmpty()) {
            s.jobs.append({kind, std::max(0.0, now.uptimeSeconds - parseTaskStartSeconds(task.stat))});
        }
    }
    std::stable_sort(s.jobs.begin(), s.jobs.end(),
                     [](const JobInFlight& a, const JobInFlight& b) { return a.seconds > b.seconds; });
    const DrmTotals drm = drmTotals(now.drmFdinfo);
    s.gpu.present = drm.present;
    s.gpu.vramBytes = drm.vramBytes;
    s.systemGpu.vramTotal = now.vramTotal.trimmed().toULongLong();
    s.systemGpu.present = s.systemGpu.vramTotal > 0;
    s.systemGpu.vramUsed = now.vramUsed.trimmed().toULongLong();
    if (!now.gpuBusy.trimmed().isEmpty()) {
        s.systemGpu.busyPercent = now.gpuBusy.trimmed().toDouble();
    }

    if (!hasPrevious_) {
        previous_ = now;
        hasPrevious_ = true;
        return s;
    }

    const double seconds = static_cast<double>(now.monotonicNs - previous_.monotonicNs) / 1e9;
    if (seconds > 0.0) {
        const double perTick = 100.0 / clockTicksPerSecond() / seconds;
        const quint64 before = parseProcessStatTicks(previous_.processStat);
        const quint64 after = parseProcessStatTicks(now.processStat);
        s.processCpu = static_cast<double>(after >= before ? after - before : 0) * perTick;

        std::map<int, quint64> previousTicks;
        for (const DebugTaskReading& task : previous_.tasks) {
            previousTicks[task.tid] = parseProcessStatTicks(task.stat);
        }
        std::map<QString, ThreadGroupUse> byGroup;
        for (const DebugTaskReading& task : now.tasks) {
            const QString group = threadGroupName(parseTaskComm(task.stat), task.tid == now.pid);
            ThreadGroupUse& use = byGroup[group];
            use.group = group;
            use.threads += 1;
            const quint64 ticks = parseProcessStatTicks(task.stat);
            const auto old = previousTicks.find(task.tid);
            if (old != previousTicks.end() && ticks >= old->second) {
                use.cpuPercent += static_cast<double>(ticks - old->second) * perTick;
            }
        }
        QList<ThreadGroupUse> groups;
        for (const auto& entry : byGroup) {
            groups.append(entry.second);
        }
        std::stable_sort(groups.begin(), groups.end(),
                         [](const ThreadGroupUse& a, const ThreadGroupUse& b) {
                             return a.cpuPercent > b.cpuPercent;
                         });
        s.groups = groups;

        const quint64 pageBytes = static_cast<quint64>(std::max<long>(::sysconf(_SC_PAGESIZE), 1));
        const auto rate = [&](const QString& key) {
            const quint64 a = parseVmStatCounter(previous_.vmStat, key);
            const quint64 b = parseVmStatCounter(now.vmStat, key);
            return static_cast<double>(b >= a ? b - a : 0) * static_cast<double>(pageBytes) / seconds;
        };
        s.swapInBytesPerSec = rate(QStringLiteral("pswpin"));
        s.swapOutBytesPerSec = rate(QStringLiteral("pswpout"));

        const auto perSecond = [seconds](quint64 before, quint64 after) {
            return static_cast<double>(after >= before ? after - before : 0) / seconds;
        };
        quint64 minorBefore = 0, majorBefore = 0, minorAfter = 0, majorAfter = 0;
        if (parseProcessFaults(previous_.processStat, &minorBefore, &majorBefore)
            && parseProcessFaults(now.processStat, &minorAfter, &majorAfter)) {
            s.minorFaultsPerSec = perSecond(minorBefore, minorAfter);
            s.majorFaultsPerSec = perSecond(majorBefore, majorAfter);
        }
        if (!now.io.isEmpty() && !previous_.io.isEmpty()) {
            s.ioReadBytesPerSec = perSecond(parseKeyedCounter(previous_.io, QStringLiteral("read_bytes:")),
                                            parseKeyedCounter(now.io, QStringLiteral("read_bytes:")));
            s.ioWriteBytesPerSec = perSecond(parseKeyedCounter(previous_.io, QStringLiteral("write_bytes:")),
                                             parseKeyedCounter(now.io, QStringLiteral("write_bytes:")));
        }
        const QString voluntary = QStringLiteral("voluntary_ctxt_switches:");
        const QString involuntary = QStringLiteral("nonvoluntary_ctxt_switches:");
        s.ctxVoluntaryPerSec = perSecond(sumTaskCounter(previous_, voluntary), sumTaskCounter(now, voluntary));
        s.ctxInvoluntaryPerSec =
            perSecond(sumTaskCounter(previous_, involuntary), sumTaskCounter(now, involuntary));
        const DrmTotals drmBefore = drmTotals(previous_.drmFdinfo);
        if (drm.present && drmBefore.present) {
            const quint64 delta = drm.engineNs >= drmBefore.engineNs ? drm.engineNs - drmBefore.engineNs : 0;
            s.gpu.enginePercent = 100.0 * static_cast<double>(delta) / (seconds * 1e9);
        }
    }

    const auto before = parsePerCpuTimes(previous_.cpuStat);
    const auto after = parsePerCpuTimes(now.cpuStat);
    if (!after.isEmpty() && before.size() == after.size()) {
        QList<double> percents;
        for (int i = 0; i < after.size(); ++i) {
            const quint64 total = after.at(i).first - before.at(i).first;
            const quint64 idle = after.at(i).second - before.at(i).second;
            percents.append(total == 0 ? 0.0
                                       : 100.0 * static_cast<double>(total - std::min(idle, total))
                                             / static_cast<double>(total));
        }
        s.cpuPercents = percents;
    }
    previous_ = now;
    return s;
}

} // namespace pictura
