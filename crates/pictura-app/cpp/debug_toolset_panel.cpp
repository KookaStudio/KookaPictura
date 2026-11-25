#include "debug_toolset_panel.h"

#include "system_info.h"

#include <QtCore/QEvent>
#include <QtGui/QHelpEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPainterPath>
#include <QtGui/QWindow>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QSizeGrip>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QToolTip>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>
#include <utility>

#include <QtCore/QMap>

namespace pictura {

namespace {

// ponytail: shown until build numbers exist.
constexpr char kBuildPlaceholder[] = "n/a (placeholder)";
constexpr int kMargin = 8;

// Fixed on purpose: the owner asked for yellow with dark text whatever the theme.
constexpr char kPanelStyle[] = R"(
#debugToolsetPanel { background: #FFF9C4; border: 1px solid #8A7A1E; }
#debugToolsetPanel QLabel { color: #222222; background: transparent; }
#debugToolsetPanel QLabel[cell="title"] {
    font-weight: bold; background: #F0E07A; padding: 3px 6px;
}
#debugToolsetPanel QLabel[cell="label"] {
    font-weight: 600; padding: 2px 6px;
    border-top: 1px solid #D9CB6B; border-right: 1px solid #D9CB6B;
}
#debugToolsetPanel QLabel[cell="value"] { padding: 2px 6px; border-top: 1px solid #D9CB6B; }
#debugToolsetPanel QFrame[group="true"] { border: 1px solid #B8A63A; background: #FFFBD6; }
#debugToolsetPanel QLabel#debugToolsetTitle { font-weight: bold; }
#debugToolsetPanel QToolButton#debugToolsetClose {
    color: #222222; border: none; background: transparent; font-size: 16px; padding: 0 6px;
}
#debugToolsetPanel QToolButton#debugToolsetClose:hover { background: #E8D85A; }
)";

QString waiting()
{
    return QStringLiteral("…");
}

QString percentText(double value)
{
    return QStringLiteral("%1 %").arg(value, 0, 'f', 1);
}

QString countRateText(const std::optional<double>& perSecond)
{
    return perSecond ? QStringLiteral("%1/s").arg(*perSecond, 0, 'f', 1) : waiting();
}

QString rateText(const std::optional<double>& bytesPerSec)
{
    return bytesPerSec ? formatBytes(static_cast<quint64>(*bytesPerSec)) + QStringLiteral("/s")
                       : waiting();
}

// Six dots in two rows of three: the conventional "grab here" mark.
class GripIcon : public QWidget {
public:
    explicit GripIcon(QWidget* parent)
        : QWidget(parent)
    {
        setFixedSize(28, 14);
        setCursor(Qt::SizeAllCursor);
    }

protected:
    void paintEvent(QPaintEvent*) override
    {
        QPainter painter(this);
        painter.setRenderHint(QPainter::Antialiasing);
        painter.setPen(Qt::NoPen);
        painter.setBrush(QColor(0x55, 0x4A, 0x10));
        for (int row = 0; row < 2; ++row) {
            for (int col = 0; col < 3; ++col) {
                painter.drawEllipse(QPointF(5.0 + col * 9.0, 4.0 + row * 6.0), 1.8, 1.8);
            }
        }
    }
};

// Three diagonal strokes in the corner, dark enough to read on the yellow.
class ResizeGrip : public QSizeGrip {
public:
    explicit ResizeGrip(QWidget* parent)
        : QSizeGrip(parent)
    {
        setFixedSize(18, 18);
    }

protected:
    void paintEvent(QPaintEvent*) override
    {
        QPainter painter(this);
        painter.setRenderHint(QPainter::Antialiasing);
        painter.setPen(QPen(QColor(0x55, 0x4A, 0x10), 1.6, Qt::SolidLine, Qt::RoundCap));
        for (int i = 1; i <= 3; ++i) {
            const double offset = i * 5.0;
            painter.drawLine(QPointF(width() - 2.0, height() - offset),
                             QPointF(width() - offset, height() - 2.0));
        }
    }
};

} // namespace

// A line chart of the last samples, newest at the right. The vertical range is
// the series' own span (padded, never narrower than minSpan), so a flat series
// reads flat and a spike reads as a spike.
class HistoryChart : public QWidget {
public:
    HistoryChart(const QString& title, const QColor& line, std::function<QString(double)> format,
                 double minSpan, QWidget* parent)
        : QWidget(parent)
        , title_(title)
        , line_(line)
        , format_(std::move(format))
        , minSpan_(minSpan)
    {
        setMinimumHeight(72);
        setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Fixed);
    }

    void setSeries(const QList<double>& series)
    {
        series_ = series;
        update();
    }
    const QList<double>& series() const { return series_; }
    QSize minimumSizeHint() const override { return {140, 72}; }

protected:
    void paintEvent(QPaintEvent*) override
    {
        QPainter painter(this);
        painter.setRenderHint(QPainter::Antialiasing);
        const QRectF frame = QRectF(rect()).adjusted(0.5, 0.5, -0.5, -0.5);
        painter.fillRect(frame, QColor(0xFF, 0xFE, 0xF0));
        painter.setPen(QColor(0xB8, 0xA6, 0x3A));
        painter.drawRect(frame);

        const QColor ink(0x22, 0x22, 0x22);
        QString caption = title_;
        if (!series_.isEmpty()) {
            const auto [low, high] = std::minmax_element(series_.cbegin(), series_.cend());
            caption += QStringLiteral("   now %1   low %2   peak %3")
                           .arg(format_(series_.last()), format_(*low), format_(*high));
        } else {
            caption += QStringLiteral("   …");
        }
        painter.setPen(ink);
        const QFontMetrics metrics(font());
        painter.drawText(QPointF(6.0, 3.0 + metrics.ascent()), caption);

        if (series_.size() < 2) {
            return;
        }
        const auto [lowIt, highIt] = std::minmax_element(series_.cbegin(), series_.cend());
        const double span = std::max(*highIt - *lowIt, minSpan_);
        const double centre = (*highIt + *lowIt) / 2.0;
        double lo = centre - span * 0.6;
        const double hi = centre + span * 0.6;
        lo = std::max(lo, std::min(0.0, *lowIt));
        const QRectF plot = frame.adjusted(4.0, metrics.height() + 8.0, -4.0, -4.0);
        const double step = plot.width() / (DebugToolsetPanel::kHistorySamples - 1);
        QPainterPath path;
        for (int i = 0; i < series_.size(); ++i) {
            const double x = plot.right() - (series_.size() - 1 - i) * step;
            const double y = plot.bottom() - (series_.at(i) - lo) / (hi - lo) * plot.height();
            if (i == 0) {
                path.moveTo(x, y);
            } else {
                path.lineTo(x, y);
            }
        }
        painter.setPen(QPen(line_, 1.6));
        painter.setBrush(Qt::NoBrush);
        painter.drawPath(path);
    }

private:
    QString title_;
    QColor line_;
    std::function<QString(double)> format_;
    double minSpan_;
    QList<double> series_;
};

// CPU per thread group, one coloured line each, with a legend; the lines share the
// panel's sample window and a common vertical scale from zero.
class GroupHistoryChart : public QWidget {
public:
    explicit GroupHistoryChart(QWidget* parent)
        : QWidget(parent)
    {
        setMinimumHeight(84);
        setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Fixed);
    }

    static const QList<std::pair<QString, QColor>>& groups()
    {
        static const QList<std::pair<QString, QColor>> all = {
            {QStringLiteral("GUI"), QColor(0x1A, 0x4F, 0x8B)},
            {QStringLiteral("Rayon workers"), QColor(0xA1, 0x26, 0x1C)},
            {QStringLiteral("Jobs"), QColor(0x2E, 0x7D, 0x32)},
            {QStringLiteral("Other"), QColor(0x6A, 0x4C, 0x93)},
        };
        return all;
    }

    void push(const QString& group, double percent)
    {
        QList<double>& series = series_[group];
        series.append(percent);
        while (series.size() > DebugToolsetPanel::kHistorySamples) {
            series.removeFirst();
        }
        update();
    }
    void clear()
    {
        series_.clear();
        update();
    }
    QList<double> series(const QString& group) const { return series_.value(group); }
    QSize minimumSizeHint() const override { return {140, 84}; }

protected:
    void paintEvent(QPaintEvent*) override
    {
        QPainter painter(this);
        painter.setRenderHint(QPainter::Antialiasing);
        const QRectF frame = QRectF(rect()).adjusted(0.5, 0.5, -0.5, -0.5);
        painter.fillRect(frame, QColor(0xFF, 0xFE, 0xF0));
        painter.setPen(QColor(0xB8, 0xA6, 0x3A));
        painter.drawRect(frame);

        const QFontMetrics metrics(font());
        double x = 6.0;
        const double baseline = 3.0 + metrics.ascent();
        painter.setPen(QColor(0x22, 0x22, 0x22));
        const QString title = QStringLiteral("CPU by group");
        painter.drawText(QPointF(x, baseline), title);
        x += metrics.horizontalAdvance(title) + 10.0;
        double peak = 10.0;
        for (const auto& [name, color] : groups()) {
            painter.setPen(Qt::NoPen);
            painter.setBrush(color);
            painter.drawRect(QRectF(x, baseline - 8.0, 8.0, 8.0));
            painter.setPen(QColor(0x22, 0x22, 0x22));
            // The legend shortens the one long name so it fits the narrower column.
            const QString label = name == QLatin1String("Rayon workers") ? QStringLiteral("Rayon") : name;
            painter.drawText(QPointF(x + 11.0, baseline), label);
            x += 11.0 + metrics.horizontalAdvance(label) + 8.0;
            for (double value : series_.value(name)) {
                peak = std::max(peak, value);
            }
        }
        const QRectF plot = frame.adjusted(4.0, metrics.height() + 8.0, -4.0, -4.0);
        const double step = plot.width() / (DebugToolsetPanel::kHistorySamples - 1);
        painter.setBrush(Qt::NoBrush);
        for (const auto& [name, color] : groups()) {
            const QList<double> series = series_.value(name);
            if (series.size() < 2) {
                continue;
            }
            QPainterPath path;
            for (int i = 0; i < series.size(); ++i) {
                const double px = plot.right() - (series.size() - 1 - i) * step;
                const double py = plot.bottom() - series.at(i) / (peak * 1.1) * plot.height();
                if (i == 0) {
                    path.moveTo(px, py);
                } else {
                    path.lineTo(px, py);
                }
            }
            painter.setPen(QPen(color, 1.6));
            painter.drawPath(path);
        }
    }

private:
    QMap<QString, QList<double>> series_;
};

CpuHeatmap::CpuHeatmap(QWidget* parent)
    : QWidget(parent)
{
    setObjectName(QStringLiteral("debugToolsetCpuHeatmap"));
    setSizePolicy(QSizePolicy::Preferred, QSizePolicy::Fixed);
}

void CpuHeatmap::setLoads(const QList<double>& percents)
{
    if (percents.size() != loads_.size()) {
        updateGeometry();
    }
    loads_ = percents;
    update();
}

int CpuHeatmap::rowCount() const
{
    return loads_.size() > 8 ? 2 : 1;
}

int CpuHeatmap::columnCount() const
{
    const int rows = rowCount();
    return std::max(1, (static_cast<int>(loads_.size()) + rows - 1) / rows);
}

QRect CpuHeatmap::cellRect(int index) const
{
    const int columns = columnCount();
    return QRect(kGap + (index % columns) * (kCell + kGap), kGap + (index / columns) * (kCell + kGap),
                 kCell, kCell);
}

QColor CpuHeatmap::colorFor(double percent)
{
    static const QColor cold(0x3B, 0x6F, 0xB6);
    static const QColor mid(0xF2, 0xB1, 0x34);
    static const QColor hot(0xC0, 0x28, 0x1C);
    const double t = std::clamp(percent, 0.0, 100.0) / 100.0;
    const auto blend = [](const QColor& a, const QColor& b, double f) {
        return QColor::fromRgbF(a.redF() + (b.redF() - a.redF()) * f,
                                a.greenF() + (b.greenF() - a.greenF()) * f,
                                a.blueF() + (b.blueF() - a.blueF()) * f);
    };
    return t < 0.5 ? blend(cold, mid, t * 2.0) : blend(mid, hot, (t - 0.5) * 2.0);
}

QSize CpuHeatmap::sizeHint() const
{
    if (loads_.isEmpty()) {
        return {kCell * 2, kCell + 2 * kGap};
    }
    return {kGap + columnCount() * (kCell + kGap), kGap + rowCount() * (kCell + kGap)};
}

int CpuHeatmap::cellAt(const QPoint& point) const
{
    for (int i = 0; i < loads_.size(); ++i) {
        if (cellRect(i).contains(point)) {
            return i;
        }
    }
    return -1;
}

bool CpuHeatmap::event(QEvent* event)
{
    if (event->type() == QEvent::ToolTip) {
        const auto* help = static_cast<QHelpEvent*>(event);
        const int cell = cellAt(help->pos());
        if (cell >= 0) {
            QToolTip::showText(help->globalPos(),
                               tr("CPU %1: %2 %").arg(cell).arg(loads_.at(cell), 0, 'f', 1), this);
        } else {
            QToolTip::hideText();
        }
        return true;
    }
    return QWidget::event(event);
}

void CpuHeatmap::paintEvent(QPaintEvent*)
{
    QPainter painter(this);
    if (loads_.isEmpty()) {
        painter.setPen(QColor(0x22, 0x22, 0x22));
        painter.drawText(rect().adjusted(kGap, 0, 0, 0), Qt::AlignVCenter | Qt::AlignLeft, waiting());
        return;
    }
    painter.setPen(QColor(0x55, 0x4A, 0x10));
    for (int i = 0; i < loads_.size(); ++i) {
        painter.setBrush(colorFor(loads_.at(i)));
        painter.drawRect(cellRect(i).adjusted(0, 0, -1, -1));
    }
}

DebugToolsetPanel::DebugToolsetPanel(QWidget* window)
    : QFrame(window, Qt::Tool | Qt::FramelessWindowHint | Qt::WindowStaysOnTopHint)
{
    setObjectName(QStringLiteral("debugToolsetPanel"));
    setAttribute(Qt::WA_ShowWithoutActivating);
    setAttribute(Qt::WA_StyledBackground);
    setStyleSheet(QString::fromLatin1(kPanelStyle));
    hide();

    auto* column = new QVBoxLayout(this);
    column->setContentsMargins(kMargin, kMargin, kMargin, kMargin);
    column->setSpacing(kMargin);
    buildHeader(column);

    auto* columns = new QHBoxLayout;
    columns->setSpacing(kMargin);
    auto* left = new QVBoxLayout;
    left->setSpacing(kMargin);
    auto* right = new QVBoxLayout;
    right->setSpacing(kMargin);
    columns->addLayout(left, 1);
    columns->addLayout(right, 1);
    column->addLayout(columns);

    QGridLayout* grid = addGroup(left, QStringLiteral("usage"), tr("Pictura Resource Usage"));
    addRow(grid, QStringLiteral("ram"), tr("RAM"));
    addRow(grid, QStringLiteral("ramSplit"), tr("RAM split"));
    addRow(grid, QStringLiteral("swap"), tr("Swap"));
    addRow(grid, QStringLiteral("cpu"), tr("CPU"));
    addRow(grid, QStringLiteral("threads"), tr("Threads"));
    addRow(grid, QStringLiteral("gpu"), tr("GPU frame"));
    addRow(grid, QStringLiteral("gpuEngine"), tr("GPU busy"));
    addRow(grid, QStringLiteral("gpuMemory"), tr("GPU memory"));
    addRow(grid, QStringLiteral("busiest"), tr("Busiest"));
    addChart(grid, QStringLiteral("ram"), QColor(0x1A, 0x4F, 0x8B),
             [](double bytes) { return formatBytes(static_cast<quint64>(std::max(bytes, 0.0))); },
             8.0 * 1024 * 1024);
    addChart(grid, QStringLiteral("cpu"), QColor(0xA1, 0x26, 0x1C),
             [](double percent) { return percentText(percent); }, 10.0);
    groupChart_ = new GroupHistoryChart(grid->parentWidget());
    groupChart_->setObjectName(QStringLiteral("debugToolsetChart_groups"));
    grid->addWidget(groupChart_, grid->rowCount(), 0, 1, 2);

    grid = addGroup(right, QStringLiteral("budget"), tr("Pictura Memory Budget"));
    addRow(grid, QStringLiteral("governor"), tr("Governor"));
    addRow(grid, QStringLiteral("scratch"), tr("Scratch disk"));
    addRow(grid, QStringLiteral("tiles"), tr("Tiles"));

    grid = addGroup(right, QStringLiteral("activity"), tr("Pictura Activity"));
    addRow(grid, QStringLiteral("files"), tr("Open files"));
    addRow(grid, QStringLiteral("disk"), tr("Disk I/O"));
    addRow(grid, QStringLiteral("faults"), tr("Page faults"));
    addRow(grid, QStringLiteral("ctx"), tr("Context switches"));
    addRow(grid, QStringLiteral("jobs"), tr("Jobs running"));
    addRow(grid, QStringLiteral("edits"), tr("Edit time"));
    addRow(grid, QStringLiteral("paint"), tr("Paint time"));

    grid = addGroup(right, QStringLiteral("platform"), tr("Platform"));
    addRow(grid, QStringLiteral("os"), tr("OS"));
    addRow(grid, QStringLiteral("architecture"), tr("Architecture"));
    addRow(grid, QStringLiteral("desktop"), tr("Desktop"));
    addRow(grid, QStringLiteral("session"), tr("Session"));
    addRow(grid, QStringLiteral("memory"), tr("Memory"));
    addRow(grid, QStringLiteral("swapUsed"), tr("Swap used"));
    addRow(grid, QStringLiteral("swapLocation"), tr("Swap location"));
    addRow(grid, QStringLiteral("swapIo"), tr("Swap I/O"));
    heat_ = new CpuHeatmap(grid->parentWidget());
    addRow(grid, QStringLiteral("cpus"), tr("CPUs"), heat_);
    addRow(grid, QStringLiteral("gpuSystem"), tr("GPU (system)"));
    addRow(grid, QStringLiteral("load"), tr("Load"));

    grid = addGroup(left, QStringLiteral("version"), tr("Version"));
    addRow(grid, QStringLiteral("build"), tr("Build"));
    rows_.value(QStringLiteral("build"))->setText(QString::fromLatin1(kBuildPlaceholder));

    left->addStretch(1);
    right->addStretch(1);

    auto* gripRow = new QHBoxLayout;
    gripRow->setContentsMargins(0, 0, 0, 0);
    gripRow->addStretch(1);
    gripRow->addWidget(new ResizeGrip(this), 0, Qt::AlignRight | Qt::AlignBottom);
    column->addLayout(gripRow);

    refresh_.setInterval(kRefreshMs);
    connect(&refresh_, &QTimer::timeout, this, &DebugToolsetPanel::refreshNow);
}

void DebugToolsetPanel::buildHeader(QVBoxLayout* column)
{
    header_ = new QWidget(this);
    header_->setObjectName(QStringLiteral("debugToolsetHeader"));
    header_->setCursor(Qt::SizeAllCursor);
    auto* row = new QHBoxLayout(header_);
    row->setContentsMargins(0, 0, 0, 0);

    auto* title = new QLabel(tr("Pictura Metrics"), header_);
    title->setObjectName(QStringLiteral("debugToolsetTitle"));
    title->setSizePolicy(QSizePolicy::Ignored, QSizePolicy::Preferred);
    row->addWidget(title, 1);

    auto* icon = new GripIcon(header_);
    icon->setObjectName(QStringLiteral("debugToolsetDragIcon"));
    row->addWidget(icon, 0, Qt::AlignCenter);

    auto* right = new QWidget(header_);
    right->setSizePolicy(QSizePolicy::Ignored, QSizePolicy::Preferred);
    auto* rightRow = new QHBoxLayout(right);
    rightRow->setContentsMargins(0, 0, 0, 0);
    rightRow->addStretch(1);
    auto* close = new QToolButton(right);
    close->setObjectName(QStringLiteral("debugToolsetClose"));
    close->setText(QStringLiteral("×"));
    close->setToolTip(tr("Close the debug toolset and stop collecting metrics"));
    close->setCursor(Qt::ArrowCursor);
    connect(close, &QToolButton::clicked, this, [this] {
        setActive(false);
        emit deactivated();
    });
    rightRow->addWidget(close);
    row->addWidget(right, 1);
    column->addWidget(header_);
}

QGridLayout* DebugToolsetPanel::addGroup(QVBoxLayout* column, const QString& key,
                                         const QString& title)
{
    auto* group = new QFrame(this);
    group->setObjectName(QStringLiteral("debugToolsetGroup_") + key);
    group->setProperty("group", true);
    auto* grid = new QGridLayout(group);
    grid->setContentsMargins(0, 0, 0, 0);
    grid->setSpacing(0);
    grid->setColumnStretch(1, 1);
    grid->setColumnMinimumWidth(0, 130);
    auto* heading = new QLabel(title, group);
    heading->setObjectName(QStringLiteral("debugToolsetGroupTitle_") + key);
    heading->setProperty("cell", QStringLiteral("title"));
    grid->addWidget(heading, 0, 0, 1, 2);
    column->addWidget(group);
    return grid;
}

void DebugToolsetPanel::addRow(QGridLayout* grid, const QString& key, const QString& label,
                               QWidget* content)
{
    QWidget* group = grid->parentWidget();
    const int row = grid->rowCount();
    auto* name = new QLabel(label, group);
    name->setProperty("cell", QStringLiteral("label"));
    if (content) {
        grid->addWidget(name, row, 0);
        grid->addWidget(content, row, 1, Qt::AlignLeft | Qt::AlignVCenter);
        return;
    }
    auto* value = new QLabel(waiting(), group);
    value->setObjectName(QStringLiteral("debugToolsetRow_") + key);
    value->setProperty("cell", QStringLiteral("value"));
    value->setTextInteractionFlags(Qt::NoTextInteraction);
    rows_.insert(key, value);
    grid->addWidget(name, row, 0);
    grid->addWidget(value, row, 1);
}

HistoryChart* DebugToolsetPanel::addChart(QGridLayout* grid, const QString& key,
                                          const QColor& line,
                                          std::function<QString(double)> format, double minSpan)
{
    const QString title = key == QStringLiteral("ram") ? tr("RAM") : tr("CPU");
    auto* chart = new HistoryChart(title, line, std::move(format), minSpan, grid->parentWidget());
    chart->setObjectName(QStringLiteral("debugToolsetChart_") + key);
    grid->addWidget(chart, grid->rowCount(), 0, 1, 2);
    charts_.insert(key, chart);
    return chart;
}

QString DebugToolsetPanel::rowText(const QString& key) const
{
    const QLabel* row = rows_.value(key);
    return row ? row->text() : QString();
}

QList<double> DebugToolsetPanel::history(const QString& key) const
{
    const HistoryChart* chart = charts_.value(key);
    return chart ? chart->series() : QList<double>();
}

QList<double> DebugToolsetPanel::groupHistory(const QString& group) const
{
    return groupChart_->series(group);
}

void DebugToolsetPanel::setActive(bool active)
{
    if (active == isActive()) {
        return;
    }
    if (!active) {
        refresh_.stop();
        sampler_.reset();
        for (HistoryChart* chart : std::as_const(charts_)) {
            chart->setSeries({});
        }
        heat_->setLoads({});
        groupChart_->clear();
        hide();
        return;
    }
    // Read once per enable; none of these change while the app runs.
    const SystemInfo info = collectSystemInfo({});
    os_ = QStringLiteral("%1 %2").arg(info.osType, info.osVersion).trimmed();
    architecture_ = info.architecture;
    desktop_ = info.desktop;
    session_ = info.session;
    sampler_ = std::make_unique<DebugSampler>();
    if (!placed_) {
        adjustSize();
        placeAtDefault();
        placed_ = true;
    }
    show();
    raise();
    refreshNow();
    refresh_.start();
}

void DebugToolsetPanel::refreshNow()
{
    if (!sampler_) {
        return;
    }
    apply(sampler_->sample(readDebugSources()));
}

void DebugToolsetPanel::apply(const DebugSnapshot& s)
{
    const auto set = [this](const QString& key, const QString& text) {
        if (QLabel* row = rows_.value(key)) {
            row->setText(text);
        }
    };
    const auto push = [this](const QString& key, double sample) {
        HistoryChart* chart = charts_.value(key);
        QList<double> series = chart->series();
        series.append(sample);
        while (series.size() > kHistorySamples) {
            series.removeFirst();
        }
        chart->setSeries(series);
    };
    set(QStringLiteral("ram"), formatBytes(s.processRss));
    set(QStringLiteral("swap"), formatBytes(s.processSwap));
    set(QStringLiteral("cpu"), s.processCpu ? percentText(*s.processCpu) : waiting());
    set(QStringLiteral("threads"), QString::number(s.processThreads));
    const QString metricsJson = metrics_ ? metrics_() : QString();
    set(QStringLiteral("gpu"), gpuRowText(metricsJson));
    set(QStringLiteral("ramSplit"), tr("anon %1 / file %2")
                                        .arg(formatBytes(s.rssAnon), formatBytes(s.rssFile)));
    set(QStringLiteral("gpuEngine"),
        !s.gpu.present ? tr("n/a")
        : s.gpu.enginePercent ? percentText(*s.gpu.enginePercent)
                              : waiting());
    set(QStringLiteral("gpuMemory"), s.gpu.present ? formatBytes(s.gpu.vramBytes) : tr("n/a"));

    const MemoryBudget budget = parseMemoryBudget(memory_ ? memory_() : QString());
    if (budget.present) {
        const auto bytesOrNa = [](const std::optional<quint64>& value) {
            return value ? formatBytes(*value) : QStringLiteral("n/a");
        };
        set(QStringLiteral("governor"),
            tr("%1 of %2, %3, reserved %4")
                .arg(bytesOrNa(budget.used), bytesOrNa(budget.ceiling), budget.pressure,
                     formatBytes(budget.reserved)));
        set(QStringLiteral("scratch"), tr("%1 used, %2 free")
                                           .arg(formatBytes(budget.scratchUsed),
                                                formatBytes(budget.scratchAvailable)));
        set(QStringLiteral("tiles"), tr("packed %1 (%2), spilled %3, backed %4")
                                         .arg(budget.packed)
                                         .arg(formatBytes(budget.packedBytes))
                                         .arg(budget.spilled)
                                         .arg(budget.backed));
    } else {
        for (const char* key : {"governor", "scratch", "tiles"}) {
            set(QString::fromLatin1(key), tr("n/a"));
        }
    }

    set(QStringLiteral("files"), QString::number(s.openFds));
    set(QStringLiteral("disk"), tr("read %1, write %2")
                                    .arg(rateText(s.ioReadBytesPerSec), rateText(s.ioWriteBytesPerSec)));
    set(QStringLiteral("faults"), tr("minor %1, major %2")
                                      .arg(countRateText(s.minorFaultsPerSec),
                                           countRateText(s.majorFaultsPerSec)));
    set(QStringLiteral("ctx"), tr("voluntary %1, involuntary %2")
                                   .arg(countRateText(s.ctxVoluntaryPerSec),
                                        countRateText(s.ctxInvoluntaryPerSec)));
    QStringList jobLines;
    for (const JobInFlight& job : s.jobs) {
        jobLines << tr("%1 %2 s").arg(job.kind).arg(static_cast<int>(job.seconds));
    }
    set(QStringLiteral("jobs"), jobLines.isEmpty() ? tr("none") : jobLines.join(QLatin1Char('\n')));
    set(QStringLiteral("edits"), timingText(metricsJson, true));
    set(QStringLiteral("paint"), timingText(metricsJson, false));
    set(QStringLiteral("gpuSystem"),
        s.systemGpu.present
            ? tr("busy %1, VRAM %2 of %3")
                  .arg(s.systemGpu.busyPercent ? percentText(*s.systemGpu.busyPercent) : tr("n/a"),
                       formatBytes(s.systemGpu.vramUsed), formatBytes(s.systemGpu.vramTotal))
            : tr("n/a"));
    if (s.groups) {
        for (const auto& [name, color] : GroupHistoryChart::groups()) {
            double percent = 0.0;
            for (const ThreadGroupUse& use : *s.groups) {
                if (use.group == name) {
                    percent = use.cpuPercent;
                }
            }
            groupChart_->push(name, percent);
        }
    }
    push(QStringLiteral("ram"), static_cast<double>(s.processRss));
    if (s.processCpu) {
        push(QStringLiteral("cpu"), *s.processCpu);
    }
    if (s.groups) {
        QStringList lines;
        for (int i = 0; i < std::min<int>(3, s.groups->size()); ++i) {
            const ThreadGroupUse& use = s.groups->at(i);
            lines << QStringLiteral("%1 %2 (%3)").arg(use.group, percentText(use.cpuPercent),
                                                      QString::number(use.threads));
        }
        set(QStringLiteral("busiest"), lines.join(QLatin1Char('\n')));
    } else {
        set(QStringLiteral("busiest"), waiting());
    }

    set(QStringLiteral("os"), os_);
    set(QStringLiteral("architecture"), architecture_);
    set(QStringLiteral("desktop"), desktop_);
    set(QStringLiteral("session"), session_);
    set(QStringLiteral("memory"),
        tr("%1 total, %2 used, %3 cached")
            .arg(formatBytes(s.memTotal), formatBytes(s.memUsed), formatBytes(s.memCached)));
    set(QStringLiteral("swapUsed"),
        tr("%1 of %2").arg(formatBytes(s.swapUsed), formatBytes(s.swapTotal)));
    set(QStringLiteral("swapLocation"),
        s.swapLocation.isEmpty() ? tr("none") : s.swapLocation);
    set(QStringLiteral("swapIo"),
        tr("in %1, out %2").arg(rateText(s.swapInBytesPerSec), rateText(s.swapOutBytesPerSec)));
    heat_->setLoads(s.cpuPercents ? *s.cpuPercents : QList<double>());
    set(QStringLiteral("load"),
        QStringLiteral("%1  %2  %3")
            .arg(s.load1, 0, 'f', 2)
            .arg(s.load5, 0, 'f', 2)
            .arg(s.load15, 0, 'f', 2));
}

void DebugToolsetPanel::placeAtDefault()
{
    const QWidget* window = parentWidget();
    move(window->mapToGlobal(QPoint(window->width() - width() - 2 * kMargin, 3 * kMargin)));
}

void DebugToolsetPanel::mousePressEvent(QMouseEvent* event)
{
    if (event->button() == Qt::LeftButton
        && header_->geometry().contains(event->position().toPoint())) {
        dragOffset_ = event->globalPosition().toPoint() - frameGeometry().topLeft();
        // The window manager moves the window when it can (Wayland needs this);
        // otherwise follow the pointer by hand.
        dragging_ = !(windowHandle() && windowHandle()->startSystemMove());
        event->accept();
        return;
    }
    QFrame::mousePressEvent(event);
}

void DebugToolsetPanel::mouseMoveEvent(QMouseEvent* event)
{
    if (dragging_) {
        move(event->globalPosition().toPoint() - dragOffset_);
        event->accept();
        return;
    }
    QFrame::mouseMoveEvent(event);
}

void DebugToolsetPanel::mouseReleaseEvent(QMouseEvent* event)
{
    dragging_ = false;
    QFrame::mouseReleaseEvent(event);
}

} // namespace pictura

#include "moc_debug_toolset_panel.cpp"
