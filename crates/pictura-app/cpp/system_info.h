#pragma once

#include <QtCore/QList>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtCore/QUrl>

namespace pictura {

struct MonitorInfo {
    QString name;
    QString model;
    QString manufacturer;
    QString resolution;
    QString physicalSize;
    QString dpi;
    QString refreshRate;
};

struct SystemInfo {
    QString appVersion;
    QString buildType;
    QString qtVersion;
    QString osType;
    QString osVersion;
    QString kernel;
    QString architecture;
    QString desktop;
    QString session;
    QString cpuModel;
    QString cpuPhysicalCores;
    QString cpuLogicalCores;
    QString cpuClock;
    QString gpuName;
    QString gpuType;
    QString gpuBackend;
    QString gpuDriver;
    QString gpuVram;
    QString memoryTotal;
    QString memoryAvailable;
    QString swapTotal;
    QString swapAvailable;
    QString systemCpu;
    QString processMemory;
    QString processCpu;
    QString processCpuAverage;
    QList<MonitorInfo> monitors;
};

// Collect the machine and process environment. `gpuFields` is the bridge's
// `gpuAdapterInfo()` list (name, device type, backend, driver, driver info);
// an empty list means no usable adapter.
SystemInfo collectSystemInfo(const QStringList& gpuFields);

// The plain-text report shared by the dialog, the clipboard, and the issue
// body.
QString formatSystemInfo(const SystemInfo& info);

// The repository's new-issue page with `report` in the body, capped so the
// URL stays browser-safe.
QUrl submitIssueUrl(const QString& report);

// Pure helpers, exposed for tests.
QString parseCpuModel(const QString& cpuInfo);
int parsePhysicalCores(const QString& cpuInfo);
QString parseCpuClock(const QString& cpuInfo);
quint64 parseMemInfoBytes(const QString& memInfo, const QString& key);
QString formatBytes(quint64 bytes);

} // namespace pictura
