#pragma once

#include <QtCore/QString>
#include <QtCore/QtTypes>

namespace pictura {

class PictureView;

// The governor and Efficiency figures the status bar and the Performance page
// read (#636), from `memory_status_json()`; -1 for a figure it cannot read.
struct MemoryReading {
    qint64 used = -1;
    qint64 ceiling = -1;
    qint64 computeMs = 0;
    qint64 ioMs = 0;
};

MemoryReading readMemory();

// CS6's Scratch Sizes form: `312.5M/2.04G`, an unknown side as `—`.
QString formatScratchSizes(qint64 used, qint64 available);

// Efficiency over the interval since the last reading in which time was
// measured: 100 % until something is, then the last value while idle.
class EfficiencyMeter {
public:
    int update(qint64 computeMs, qint64 ioMs);
    int percent() const { return percent_; }

private:
    qint64 compute_ = 0;
    qint64 io_ = 0;
    int percent_ = 100;
};

// The status bar's readout for `key` ("sizes", "dimensions",
// "measurementScale", "scratchSizes", "efficiency"). The document readouts
// show `—` without a document; the memory ones need none.
QString statusReadoutText(const QString& key, PictureView* view, EfficiencyMeter& efficiency);

// Whether `key` reads the governor and so goes stale without an event.
bool statusReadoutIsLive(const QString& key);

} // namespace pictura
