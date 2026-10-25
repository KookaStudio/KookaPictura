#pragma once

// Opt-in paint-path timing for the Qt frontend, enabled by the same
// `PICTURA_PAINT_TIMING=1` env var the Rust side reads. A no-op otherwise.
// Header-only so it can be included wherever a phase needs instrumenting.

#include <QtCore/QByteArray>
#include <QtCore/QString>
#include <QtCore/QtGlobal>

#include <chrono>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <locale>
#include <sstream>

namespace pictura {

inline int paintTimingLevel()
{
    static const int level = [] {
        const char* value = std::getenv("PICTURA_PAINT_TIMING");
        return value ? std::atoi(value) : 0;
    }();
    return level;
}

/// Logs its elapsed wall time under `label` on destruction.
class ScopedTimer {
public:
    explicit ScopedTimer(const char* label)
        : label_(label)
        , start_(std::chrono::steady_clock::now())
    {
    }

    ~ScopedTimer()
    {
        if (paintTimingLevel() <= 0) {
            return;
        }
        const double ms =
            std::chrono::duration<double, std::milli>(std::chrono::steady_clock::now() - start_)
                .count();
        // Classic locale so the decimal separator is always '.', regardless of
        // the user's locale.
        std::ostringstream line;
        line.imbue(std::locale::classic());
        line.setf(std::ios::fixed);
        line.precision(2);
        line << ms;
        std::fprintf(stderr, "[paint-timing]   %-30s %10s ms\n", label_, line.str().c_str());
    }

    ScopedTimer(const ScopedTimer&) = delete;
    ScopedTimer& operator=(const ScopedTimer&) = delete;

private:
    const char* label_;
    std::chrono::steady_clock::time_point start_;
};

/// Whether `PICTURA_METRICS` is a non-zero integer, read once and parsed like
/// the Rust side (surrounding whitespace allowed, nothing else).
inline bool metricsEnabled()
{
    static const bool on = [] {
        bool ok = false;
        const qlonglong value = qgetenv("PICTURA_METRICS").trimmed().toLongLong(&ok);
        return ok && value != 0;
    }();
    return on;
}

/// Records its elapsed time under `label` into the Rust metrics totals on
/// destruction, when `PICTURA_METRICS` is on; reads no clock otherwise. `View`
/// is the bridge `PictureView`, templated so this header stays bridge-free.
template <typename View>
class MetricsTimer {
public:
    MetricsTimer(const View* view, const char* label)
        : view_(metricsEnabled() ? view : nullptr)
        , label_(label)
    {
        if (view_) {
            start_ = std::chrono::steady_clock::now();
        }
    }

    ~MetricsTimer()
    {
        if (!view_) {
            return;
        }
        const auto micros = std::chrono::duration_cast<std::chrono::microseconds>(
            std::chrono::steady_clock::now() - start_)
                                .count();
        view_->record_metric_phase(QString::fromLatin1(label_),
                                   static_cast<std::int64_t>(micros));
    }

    MetricsTimer(const MetricsTimer&) = delete;
    MetricsTimer& operator=(const MetricsTimer&) = delete;

private:
    const View* view_;
    const char* label_;
    std::chrono::steady_clock::time_point start_;
};

} // namespace pictura
