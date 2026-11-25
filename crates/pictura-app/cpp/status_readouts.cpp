#include "status_readouts.h"

#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>

#include <algorithm>

#include "api/paint_bridge.h"

namespace pictura {

MemoryReading readMemory()
{
    const QJsonObject json = QJsonDocument::fromJson(memory_status_json().toUtf8()).object();
    const auto bytes = [](const QJsonValue& value) {
        return value.isDouble() ? qint64(value.toDouble()) : qint64(-1);
    };
    const QJsonObject efficiency = json.value(QStringLiteral("efficiency")).toObject();
    MemoryReading reading;
    reading.used = bytes(json.value(QStringLiteral("used")));
    reading.ceiling = bytes(json.value(QStringLiteral("ceiling")));
    reading.computeMs = qint64(efficiency.value(QStringLiteral("compute_ms")).toDouble());
    reading.ioMs = qint64(efficiency.value(QStringLiteral("io_ms")).toDouble());
    return reading;
}

QString formatScratchSizes(qint64 used, qint64 available)
{
    const auto side = [](qint64 bytes) {
        constexpr double kMiB = 1024.0 * 1024.0;
        constexpr double kGiB = kMiB * 1024.0;
        if (bytes < 0) {
            return QStringLiteral("—");
        }
        if (bytes >= qint64(kGiB)) {
            return QString::number(double(bytes) / kGiB, 'f', 2) + QLatin1Char('G');
        }
        return QString::number(double(bytes) / kMiB, 'f', 1) + QLatin1Char('M');
    };
    return side(used) + QLatin1Char('/') + side(available);
}

int EfficiencyMeter::update(qint64 computeMs, qint64 ioMs)
{
    const qint64 compute = std::max<qint64>(0, computeMs - compute_);
    const qint64 io = std::max<qint64>(0, ioMs - io_);
    if (compute + io > 0) {
        percent_ = int(compute * 100 / (compute + io));
        compute_ = computeMs;
        io_ = ioMs;
    }
    return percent_;
}

QString statusReadoutText(const QString& key, PictureView* view, EfficiencyMeter& efficiency)
{
    if (key == QStringLiteral("scratchSizes")) {
        const MemoryReading reading = readMemory();
        return formatScratchSizes(reading.used, reading.ceiling);
    }
    if (key == QStringLiteral("efficiency")) {
        const MemoryReading reading = readMemory();
        return QStringLiteral("%1%").arg(efficiency.update(reading.computeMs, reading.ioMs));
    }
    if (!view || !view->has_document()) {
        return QStringLiteral("—");
    }
    const int width = view->document_width();
    const int height = view->document_height();
    if (key == QStringLiteral("measurementScale")) {
        return measurement_scale_text(*view);
    }
    if (key == QStringLiteral("dimensions")) {
        return QStringLiteral("W %1  H %2").arg(width).arg(height);
    }
    // ponytail: 72 ppi is the PSD default; wire to document resolution once
    // the codec surfaces it.
    return QStringLiteral("%1 px x %2 px (72 ppi)").arg(width).arg(height);
}

bool statusReadoutIsLive(const QString& key)
{
    return key == QStringLiteral("scratchSizes") || key == QStringLiteral("efficiency");
}

} // namespace pictura
