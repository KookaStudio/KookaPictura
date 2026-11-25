#pragma once

#include "debug_toolset.h"

#include <QtCore/QHash>
#include <QtCore/QList>
#include <QtCore/QPoint>
#include <QtGui/QColor>
#include <QtCore/QTimer>
#include <QtWidgets/QFrame>

#include <functional>
#include <memory>

class QGridLayout;
class QLabel;
class QVBoxLayout;

namespace pictura {

class HistoryChart;
class GroupHistoryChart;

// One square per logical CPU, cold (blue) to hot (red) by usage, no numbers.
// One row of squares for 8 CPUs or fewer, two rows above that.
class CpuHeatmap : public QWidget {
    Q_OBJECT

public:
    static constexpr int kCell = 22;
    static constexpr int kGap = 2;

    explicit CpuHeatmap(QWidget* parent = nullptr);

    // Percent per CPU; empty shows the waiting mark.
    void setLoads(const QList<double>& percents);
    const QList<double>& loads() const { return loads_; }
    int rowCount() const;
    int columnCount() const;
    QRect cellRect(int index) const;
    static QColor colorFor(double percent);

    QSize sizeHint() const override;
    QSize minimumSizeHint() const override { return sizeHint(); }

protected:
    void paintEvent(QPaintEvent*) override;
    bool event(QEvent* event) override;

private:
    int cellAt(const QPoint& point) const;

    QList<double> loads_;
};

// Help > Developer > Pictura Metrics: a small floating tool window over the main
// window. Inactive means no timer, no source read and no retained sample or history.
class DebugToolsetPanel : public QFrame {
    Q_OBJECT

public:
    static constexpr int kRefreshMs = 2000;
    static constexpr int kHistorySamples = 60;

    explicit DebugToolsetPanel(QWidget* window);

    // The last-GPU-time source: a `metrics_snapshot` JSON, or empty.
    void setMetricsProvider(std::function<QString()> provider) { metrics_ = std::move(provider); }

    // The memory governor's status report (`memory_status_json`), or empty.
    void setMemoryProvider(std::function<QString()> provider) { memory_ = std::move(provider); }

    void setActive(bool active);
    bool isActive() const { return refresh_.isActive(); }
    bool holdsSample() const { return sampler_ && sampler_->hasPrevious(); }
    int refreshIntervalMs() const { return refresh_.interval(); }

    // Take one reading now (the timer calls this).
    void refreshNow();

    // The value text of a row: ram, swap, cpu, threads, gpu, busiest, os,
    // architecture, desktop, session, memory, swapUsed, swapLocation, swapIo,
    // load, build.
    QString rowText(const QString& key) const;
    // The samples a chart holds, oldest first: "ram" (bytes) or "cpu" (percent).
    QList<double> history(const QString& key) const;
    // CPU percent samples of one thread group: GUI, Rayon workers, Jobs, Other.
    QList<double> groupHistory(const QString& group) const;
    QWidget* header() const { return header_; }
    const CpuHeatmap* cpuHeatmap() const { return heat_; }

signals:
    // The panel turned itself off (its close button).
    void deactivated();

protected:
    void mousePressEvent(QMouseEvent* event) override;
    void mouseMoveEvent(QMouseEvent* event) override;
    void mouseReleaseEvent(QMouseEvent* event) override;

private:
    QGridLayout* addGroup(QVBoxLayout* column, const QString& key, const QString& title);
    void addRow(QGridLayout* grid, const QString& key, const QString& label,
                QWidget* content = nullptr);
    HistoryChart* addChart(QGridLayout* grid, const QString& key, const QColor& line,
                           std::function<QString(double)> format, double minSpan);
    void buildHeader(QVBoxLayout* column);
    void placeAtDefault();
    void apply(const DebugSnapshot& s);

    QWidget* header_ = nullptr;
    CpuHeatmap* heat_ = nullptr;
    QHash<QString, QLabel*> rows_;
    QHash<QString, HistoryChart*> charts_;
    QTimer refresh_;
    std::unique_ptr<DebugSampler> sampler_;
    std::function<QString()> metrics_;
    std::function<QString()> memory_;
    GroupHistoryChart* groupChart_ = nullptr;
    QString os_, architecture_, desktop_, session_;
    bool dragging_ = false;
    QPoint dragOffset_;
    bool placed_ = false;
};

} // namespace pictura
