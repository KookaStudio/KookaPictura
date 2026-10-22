#include "system_info.h"

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QSet>
#include <QtCore/QThread>
#include <QtCore/QUrlQuery>
#include <QtCore/QSysInfo>
#include <QtGui/QGuiApplication>
#include <QtGui/QScreen>

#include <algorithm>
#include <cmath>

#ifdef Q_OS_LINUX
#include <unistd.h>
#endif

namespace pictura {

namespace {

constexpr int kIssueBodyCap = 6000;
constexpr int kCpuSampleMs = 200;

QString readTextFile(const QString& path)
{
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        return {};
    }
    return QString::fromUtf8(file.readAll());
}

QString valueOrUnavailable(const QString& value)
{
    return value.isEmpty() ? QStringLiteral("Unavailable") : value;
}

QString row(const QString& label, const QString& value)
{
    return label + QStringLiteral(": ") + valueOrUnavailable(value);
}

QString desktopName()
{
    QString desktop = qEnvironmentVariable("XDG_CURRENT_DESKTOP");
    if (desktop.isEmpty()) {
        desktop = qEnvironmentVariable("XDG_SESSION_DESKTOP");
    }
    if (desktop.isEmpty()) {
        desktop = qEnvironmentVariable("DESKTOP_SESSION");
    }
    const QString kde = qEnvironmentVariable("KDE_SESSION_VERSION");
    if (!kde.isEmpty()) {
        desktop += QStringLiteral(" (KDE ") + kde + QLatin1Char(')');
    }
    return desktop;
}

QString sessionName()
{
    QString session = qEnvironmentVariable("XDG_SESSION_TYPE");
    if (session.isEmpty()) {
        session = qEnvironmentVariable("QT_QPA_PLATFORM");
    }
    return session;
}

QString osType()
{
#ifdef Q_OS_LINUX
    return QStringLiteral("Linux");
#elif defined(Q_OS_MACOS)
    return QStringLiteral("macOS");
#elif defined(Q_OS_WIN)
    return QStringLiteral("Windows");
#else
    return QSysInfo::productType();
#endif
}

#ifdef Q_OS_LINUX

struct ProcessStat {
    quint64 cpuTicks = 0;
    quint64 startTicks = 0;
};

quint64 clockTicks()
{
    const long ticks = ::sysconf(_SC_CLK_TCK);
    return ticks > 0 ? static_cast<quint64>(ticks) : 100;
}

QString firstLine(const QString& text)
{
    const int end = text.indexOf(QLatin1Char('\n'));
    return end < 0 ? text : text.left(end);
}

QString systemCpuLine(const QString& stat)
{
    const QString line = firstLine(stat);
    return line.startsWith(QStringLiteral("cpu ")) ? line : QString();
}

quint64 systemCpuTotal(const QString& line, quint64* idle)
{
    const QStringList fields = line.split(QLatin1Char(' '), Qt::SkipEmptyParts);
    quint64 total = 0;
    for (int i = 1; i < fields.size(); ++i) {
        total += fields.at(i).toULongLong();
    }
    const quint64 idleTicks = fields.value(4).toULongLong() + fields.value(5).toULongLong();
    if (idle) {
        *idle = idleTicks;
    }
    return total;
}

ProcessStat parseProcessStat(const QString& text)
{
    ProcessStat stat;
    const int close = text.lastIndexOf(QLatin1Char(')'));
    if (close < 0) {
        return stat;
    }
    const QStringList fields =
        text.mid(close + 2).split(QLatin1Char(' '), Qt::SkipEmptyParts);
    // After the comm field, index 0 is state (field 3): utime is field 14,
    // stime 15, and starttime 22.
    if (fields.size() > 19) {
        stat.cpuTicks = fields.at(11).toULongLong() + fields.at(12).toULongLong();
        stat.startTicks = fields.at(19).toULongLong();
    }
    return stat;
}

double uptimeSeconds()
{
    const QStringList fields =
        readTextFile(QStringLiteral("/proc/uptime")).split(QLatin1Char(' '), Qt::SkipEmptyParts);
    return fields.value(0).toDouble();
}

QString percent(quint64 delta, quint64 total)
{
    if (total == 0) {
        return {};
    }
    return QString::number(100.0 * static_cast<double>(delta) / static_cast<double>(total), 'f',
                           1)
        + QStringLiteral(" %");
}

QString processMemoryFromStatus(const QString& status)
{
    const QStringList lines = status.split(QLatin1Char('\n'));
    for (const QString& line : lines) {
        if (line.startsWith(QStringLiteral("VmRSS:"))) {
            const QStringList fields =
                line.split(QLatin1Char(' '), Qt::SkipEmptyParts);
            return formatBytes(fields.value(1).toULongLong() * 1024);
        }
    }
    return {};
}

int physicalCoresFromSysfs()
{
    QSet<QString> cores;
    const QDir cpuDir(QStringLiteral("/sys/devices/system/cpu"));
    const QStringList entries = cpuDir.entryList(QDir::Dirs | QDir::NoDotAndDotDot);
    for (const QString& entry : entries) {
        bool numeric = false;
        if (!entry.startsWith(QStringLiteral("cpu"))) {
            continue;
        }
        entry.mid(3).toInt(&numeric);
        if (!numeric) {
            continue;
        }
        const QDir topology(cpuDir.filePath(entry + QStringLiteral("/topology")));
        const QString package = readTextFile(topology.filePath(QStringLiteral("physical_package_id"))).trimmed();
        const QString core = readTextFile(topology.filePath(QStringLiteral("core_id"))).trimmed();
        if (!package.isEmpty() && !core.isEmpty()) {
            cores.insert(package + QLatin1Char(':') + core);
        }
    }
    return cores.size();
}

QString cpuMaxClockFromSysfs()
{
    const QString khz = readTextFile(
        QStringLiteral("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq")).trimmed();
    const double mhz = khz.toDouble() / 1000.0;
    return mhz > 0 ? QString::number(mhz / 1000.0, 'f', 2) + QStringLiteral(" GHz") : QString();
}

#endif // Q_OS_LINUX

} // namespace

QString parseCpuModel(const QString& cpuInfo)
{
    QString model;
    QString hardware;
    QString processor;
    for (const QString& line : cpuInfo.split(QLatin1Char('\n'))) {
        const int colon = line.indexOf(QLatin1Char(':'));
        if (colon < 0) {
            continue;
        }
        const QString key = line.left(colon).trimmed();
        const QString value = line.mid(colon + 1).trimmed();
        if (key == QStringLiteral("model name") && model.isEmpty()) {
            model = value;
        } else if (key == QStringLiteral("Hardware") && hardware.isEmpty()) {
            hardware = value;
        } else if (key == QStringLiteral("Processor") && processor.isEmpty()) {
            processor = value;
        }
    }
    return !model.isEmpty() ? model : (!hardware.isEmpty() ? hardware : processor);
}

int parsePhysicalCores(const QString& cpuInfo)
{
    QSet<QString> cores;
    QString package;
    QString core;
    int cpuCores = 0;
    const auto flush = [&] {
        if (!package.isEmpty() && !core.isEmpty()) {
            cores.insert(package + QLatin1Char(':') + core);
        }
        package.clear();
        core.clear();
    };
    for (const QString& line : cpuInfo.split(QLatin1Char('\n'))) {
        if (line.trimmed().isEmpty()) {
            flush();
            continue;
        }
        const int colon = line.indexOf(QLatin1Char(':'));
        if (colon < 0) {
            continue;
        }
        const QString key = line.left(colon).trimmed();
        const QString value = line.mid(colon + 1).trimmed();
        if (key == QStringLiteral("physical id")) {
            package = value;
        } else if (key == QStringLiteral("core id")) {
            core = value;
        } else if (key == QStringLiteral("cpu cores") && cpuCores == 0) {
            cpuCores = value.toInt();
        }
    }
    flush();
    return cores.isEmpty() ? cpuCores : cores.size();
}

QString parseCpuClock(const QString& cpuInfo)
{
    double maxMhz = 0;
    for (const QString& line : cpuInfo.split(QLatin1Char('\n'))) {
        const int colon = line.indexOf(QLatin1Char(':'));
        if (colon < 0) {
            continue;
        }
        if (line.left(colon).trimmed() == QStringLiteral("cpu MHz")) {
            maxMhz = std::max(maxMhz, line.mid(colon + 1).trimmed().toDouble());
        }
    }
    return maxMhz > 0 ? QString::number(maxMhz / 1000.0, 'f', 2) + QStringLiteral(" GHz")
                      : QString();
}

quint64 parseMemInfoBytes(const QString& memInfo, const QString& key)
{
    const QString prefix = key + QLatin1Char(':');
    for (const QString& line : memInfo.split(QLatin1Char('\n'))) {
        if (!line.startsWith(prefix)) {
            continue;
        }
        const QStringList fields =
            line.mid(prefix.size()).split(QLatin1Char(' '), Qt::SkipEmptyParts);
        return fields.value(0).toULongLong() * 1024;
    }
    return 0;
}

QString formatBytes(quint64 bytes)
{
    static const char* const units[] = {"B", "KB", "MB", "GB", "TB", "PB"};
    if (bytes < 1024) {
        return QString::number(bytes) + QStringLiteral(" B");
    }
    double value = static_cast<double>(bytes);
    int unit = 0;
    while (value >= 1024.0 && unit < 5) {
        value /= 1024.0;
        ++unit;
    }
    return QString::number(value, 'f', 1) + QLatin1Char(' ') + QString::fromLatin1(units[unit]);
}

SystemInfo collectSystemInfo(const QStringList& gpuFields)
{
    SystemInfo info;
#ifdef PICTURA_VERSION
    info.appVersion = QStringLiteral(PICTURA_VERSION);
#else
    info.appVersion = QStringLiteral("unknown");
#endif
#ifdef PICTURA_BUILD_TYPE
    info.buildType = QStringLiteral(PICTURA_BUILD_TYPE);
#else
    info.buildType = QStringLiteral("unknown");
#endif
    if (info.buildType.isEmpty()) {
        info.buildType = QStringLiteral("unknown");
    }
    info.qtVersion = QString::fromLatin1(qVersion());
    info.osType = osType();
    info.osVersion = QSysInfo::prettyProductName();
    QString kernel = QSysInfo::kernelType();
    if (kernel.compare(QStringLiteral("linux"), Qt::CaseInsensitive) == 0) {
        kernel = QStringLiteral("Linux");
    }
    info.kernel = kernel + QLatin1Char(' ') + QSysInfo::kernelVersion();
    info.architecture = QSysInfo::currentCpuArchitecture();
    info.desktop = desktopName();
    info.session = sessionName();
    const int logicalCores = QThread::idealThreadCount();
    if (logicalCores > 0) {
        info.cpuLogicalCores = QString::number(logicalCores);
    }

#ifdef Q_OS_LINUX
    const QString cpuInfo = readTextFile(QStringLiteral("/proc/cpuinfo"));
    info.cpuModel = parseCpuModel(cpuInfo);
    int physical = physicalCoresFromSysfs();
    if (physical <= 0) {
        physical = parsePhysicalCores(cpuInfo);
    }
    if (physical > 0) {
        info.cpuPhysicalCores = QString::number(physical);
    }
    info.cpuClock = parseCpuClock(cpuInfo);
    const QString maxClock = cpuMaxClockFromSysfs();
    if (!info.cpuClock.isEmpty() && !maxClock.isEmpty()) {
        info.cpuClock += QStringLiteral(" (max ") + maxClock + QLatin1Char(')');
    } else if (!maxClock.isEmpty()) {
        info.cpuClock = maxClock;
    }

    const QString memInfo = readTextFile(QStringLiteral("/proc/meminfo"));
    if (!memInfo.isEmpty()) {
        info.memoryTotal = formatBytes(parseMemInfoBytes(memInfo, QStringLiteral("MemTotal")));
        quint64 available = parseMemInfoBytes(memInfo, QStringLiteral("MemAvailable"));
        if (available == 0) {
            available = parseMemInfoBytes(memInfo, QStringLiteral("MemFree"))
                + parseMemInfoBytes(memInfo, QStringLiteral("Buffers"))
                + parseMemInfoBytes(memInfo, QStringLiteral("Cached"));
        }
        info.memoryAvailable = formatBytes(available);
        info.swapTotal = formatBytes(parseMemInfoBytes(memInfo, QStringLiteral("SwapTotal")));
        info.swapAvailable = formatBytes(parseMemInfoBytes(memInfo, QStringLiteral("SwapFree")));
    }

    const QString statBefore = readTextFile(QStringLiteral("/proc/stat"));
    const QString processBefore = readTextFile(QStringLiteral("/proc/self/stat"));
    QThread::msleep(kCpuSampleMs);
    const QString statAfter = readTextFile(QStringLiteral("/proc/stat"));
    const QString processAfter = readTextFile(QStringLiteral("/proc/self/stat"));

    quint64 idleBefore = 0;
    quint64 idleAfter = 0;
    const quint64 totalBefore = systemCpuTotal(systemCpuLine(statBefore), &idleBefore);
    const quint64 totalAfter = systemCpuTotal(systemCpuLine(statAfter), &idleAfter);
    if (totalAfter > totalBefore) {
        info.systemCpu = percent((totalAfter - totalBefore) - (idleAfter - idleBefore),
                                 totalAfter - totalBefore);
    }

    const ProcessStat processStatBefore = parseProcessStat(processBefore);
    const ProcessStat processStatAfter = parseProcessStat(processAfter);
    if (processStatAfter.cpuTicks > processStatBefore.cpuTicks) {
        const double elapsed = static_cast<double>(kCpuSampleMs) / 1000.0;
        const double usage =
            100.0 * static_cast<double>(processStatAfter.cpuTicks - processStatBefore.cpuTicks)
            / static_cast<double>(clockTicks()) / elapsed;
        info.processCpu = QString::number(usage, 'f', 1) + QStringLiteral(" %");
    }
    const double uptime = uptimeSeconds();
    if (uptime > 0 && processStatAfter.startTicks > 0) {
        const double elapsed =
            uptime - static_cast<double>(processStatAfter.startTicks)
            / static_cast<double>(clockTicks());
        if (elapsed > 0) {
            const double usage =
                100.0 * static_cast<double>(processStatAfter.cpuTicks)
                / static_cast<double>(clockTicks()) / elapsed;
            info.processCpuAverage = QString::number(usage, 'f', 1) + QStringLiteral(" %");
        }
    }
    info.processMemory = processMemoryFromStatus(readTextFile(QStringLiteral("/proc/self/status")));
#endif // Q_OS_LINUX

    if (gpuFields.size() >= 5) {
        info.gpuName = gpuFields.at(0);
        info.gpuType = gpuFields.at(1);
        info.gpuBackend = gpuFields.at(2);
        info.gpuDriver = gpuFields.at(3);
        if (!gpuFields.at(4).isEmpty()) {
            info.gpuDriver += QLatin1Char(' ') + gpuFields.at(4);
        }
    }
    info.gpuVram = QStringLiteral("Unavailable");

    for (const QScreen* screen : QGuiApplication::screens()) {
        MonitorInfo monitor;
        monitor.name = screen->name();
        monitor.model = screen->model();
        monitor.manufacturer = screen->manufacturer();
        const QRect geometry = screen->geometry();
        monitor.resolution =
            QString::number(geometry.width()) + QStringLiteral(" x ") + QString::number(geometry.height());
        const QSizeF physical = screen->physicalSize();
        if (physical.width() > 0 && physical.height() > 0) {
            const double diagonalMm =
                std::sqrt(physical.width() * physical.width() + physical.height() * physical.height());
            monitor.physicalSize =
                QString::number(diagonalMm / 25.4, 'f', 1) + QStringLiteral(" in");
        }
        monitor.dpi = QString::number(screen->physicalDotsPerInch(), 'f', 0) + QStringLiteral(" DPI");
        monitor.refreshRate =
            QString::number(screen->refreshRate(), 'f', 0) + QStringLiteral(" Hz");
        info.monitors.append(monitor);
    }

    return info;
}

QString formatSystemInfo(const SystemInfo& info)
{
    QString report;
    report += QStringLiteral("Kooka Pictura System Info\n");
    report += QStringLiteral("=========================\n\n");

    report += QStringLiteral("Application\n-----------\n");
    report += row(QStringLiteral("Version"), info.appVersion) + QLatin1Char('\n');
    report += row(QStringLiteral("Build"), info.buildType) + QLatin1Char('\n');
    report += row(QStringLiteral("Qt"), info.qtVersion) + QLatin1Char('\n');

    report += QStringLiteral("\nSystem\n------\n");
    report += row(QStringLiteral("OS"), info.osType) + QLatin1Char('\n');
    report += row(QStringLiteral("OS version"), info.osVersion) + QLatin1Char('\n');
    report += row(QStringLiteral("Kernel"), info.kernel) + QLatin1Char('\n');
    report += row(QStringLiteral("Architecture"), info.architecture) + QLatin1Char('\n');
    report += row(QStringLiteral("Desktop"), info.desktop) + QLatin1Char('\n');
    report += row(QStringLiteral("Session"), info.session) + QLatin1Char('\n');

    report += QStringLiteral("\nCPU\n---\n");
    report += row(QStringLiteral("Model"), info.cpuModel) + QLatin1Char('\n');
    report += row(QStringLiteral("Physical cores"), info.cpuPhysicalCores) + QLatin1Char('\n');
    report += row(QStringLiteral("Logical cores"), info.cpuLogicalCores) + QLatin1Char('\n');
    report += row(QStringLiteral("Clock"), info.cpuClock) + QLatin1Char('\n');

    report += QStringLiteral("\nMemory\n------\n");
    report += row(QStringLiteral("RAM total"), info.memoryTotal) + QLatin1Char('\n');
    report += row(QStringLiteral("RAM available"), info.memoryAvailable) + QLatin1Char('\n');
    report += row(QStringLiteral("Swap total"), info.swapTotal) + QLatin1Char('\n');
    report += row(QStringLiteral("Swap available"), info.swapAvailable) + QLatin1Char('\n');
    report += row(QStringLiteral("System CPU"), info.systemCpu) + QLatin1Char('\n');
    report += row(QStringLiteral("Pictura RAM"), info.processMemory) + QLatin1Char('\n');
    report += row(QStringLiteral("Pictura CPU"), info.processCpu) + QLatin1Char('\n');
    report += row(QStringLiteral("Pictura CPU average"), info.processCpuAverage)
        + QLatin1Char('\n');

    report += QStringLiteral("\nGPU\n---\n");
    report += row(QStringLiteral("Adapter"), info.gpuName) + QLatin1Char('\n');
    report += row(QStringLiteral("Type"), info.gpuType) + QLatin1Char('\n');
    report += row(QStringLiteral("Backend"), info.gpuBackend) + QLatin1Char('\n');
    report += row(QStringLiteral("Driver"), info.gpuDriver) + QLatin1Char('\n');
    report += row(QStringLiteral("VRAM"), info.gpuVram) + QLatin1Char('\n');

    report += QStringLiteral("\nDisplays\n--------\n");
    if (info.monitors.isEmpty()) {
        report += QStringLiteral("Connected: 0\n");
    } else {
        int index = 1;
        for (const MonitorInfo& monitor : info.monitors) {
            QString label = monitor.name;
            const QString model = monitor.manufacturer.isEmpty()
                ? monitor.model
                : monitor.manufacturer + QLatin1Char(' ') + monitor.model;
            if (!model.trimmed().isEmpty()) {
                label = label.isEmpty() ? model : label + QStringLiteral(" (") + model + QLatin1Char(')');
            }
            report += QStringLiteral("Display ") + QString::number(index) + QStringLiteral(": ")
                + valueOrUnavailable(label) + QLatin1Char('\n');
            report += QStringLiteral("  Resolution: ") + valueOrUnavailable(monitor.resolution)
                + QLatin1Char('\n');
            report += QStringLiteral("  Physical size: ") + valueOrUnavailable(monitor.physicalSize)
                + QLatin1Char('\n');
            report += QStringLiteral("  DPI: ") + valueOrUnavailable(monitor.dpi) + QLatin1Char('\n');
            report += QStringLiteral("  Refresh rate: ") + valueOrUnavailable(monitor.refreshRate)
                + QLatin1Char('\n');
            ++index;
        }
    }
    return report;
}

QUrl submitIssueUrl(const QString& report)
{
    QString body = QStringLiteral("### System Info\n\n```\n") + report + QStringLiteral("\n```\n");
    if (body.size() > kIssueBodyCap) {
        const QString marker = QStringLiteral("\n[System info truncated]\n");
        body = body.left(kIssueBodyCap - marker.size()) + marker;
    }
    QUrl url(QStringLiteral("https://github.com/KookaStudio/KookaPictura/issues/new"));
    QUrlQuery query;
    query.addQueryItem(QStringLiteral("body"), body);
    url.setQuery(query);
    return url;
}

} // namespace pictura
