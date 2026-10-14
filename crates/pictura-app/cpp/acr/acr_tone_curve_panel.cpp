#include "acr_panels.h"
#include "acr_histogram.h"
#include "acr_settings.h"
#include "panels/curve_widget.h"
#include "theme.h"

#include <QtCore/QPointF>
#include <QtCore/QStringList>
#include <QtCore/QVector>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPalette>
#include <QtGui/QPolygonF>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QTabWidget>
#include <QtWidgets/QVBoxLayout>

#include <cmath>
#include <functional>

namespace pictura {

namespace {
QByteArray curveKeyForChannel(const QString& channel)
{
    if (channel == QLatin1String("Red")) {
        return acr::CurveRed;
    }
    if (channel == QLatin1String("Green")) {
        return acr::CurveGreen;
    }
    if (channel == QLatin1String("Blue")) {
        return acr::CurveBlue;
    }
    return acr::CurveRgb;
}

QList<int> curvePreset(const QString& preset)
{
    if (preset == QLatin1String("Medium Contrast")) {
        return {0, 0, 64, 48, 128, 140, 192, 205, 255, 255};
    }
    if (preset == QLatin1String("Strong Contrast")) {
        return {0, 0, 64, 32, 128, 160, 192, 224, 255, 255};
    }
    return {0, 0, 255, 255};
}

// The flat `[in, out, ...]` curve as the widget's `"x,y x,y …"` text.
QString curveText(const QList<int>& curve)
{
    QStringList pairs;
    for (int i = 0; i + 1 < curve.size(); i += 2) {
        pairs << QStringLiteral("%1,%2").arg(curve[i]).arg(curve[i + 1]);
    }
    return pairs.join(QLatin1Char(' '));
}

QList<int> curveFromText(const QString& text)
{
    QList<int> curve;
    for (const QString& pair : text.split(QLatin1Char(' '), Qt::SkipEmptyParts)) {
        const QStringList xy = pair.split(QLatin1Char(','));
        if (xy.size() == 2) {
            curve << xy[0].toInt() << xy[1].toInt();
        }
    }
    return curve;
}

void stylePage(QWidget* page)
{
    page->setAutoFillBackground(true);
    QPalette palette = page->palette();
    palette.setColor(QPalette::Window, Theme::panelColor());
    page->setPalette(palette);
}
} // namespace

// The parametric tone-curve graph: a luminance histogram, a diagonal, and three
// draggable region-split handles along the bottom. No curves of its own — the
// region sliders bend the render, the graph only sets the split points.
class AcrParamCurveGraph : public QWidget {
public:
    explicit AcrParamCurveGraph(QWidget* parent = nullptr) : QWidget(parent)
    {
        setMinimumHeight(180);
        setSizePolicy(QSizePolicy::Preferred, QSizePolicy::Fixed);
    }

    void setHistogram(const std::array<int, 256>& bins)
    {
        bins_ = bins;
        update();
    }

    void setRegions(double shadow, double mid, double highlight)
    {
        s_[0] = shadow;
        s_[1] = mid;
        s_[2] = highlight;
        update();
    }

    std::function<void(double, double, double)> regionsChanged;

protected:
    void paintEvent(QPaintEvent*) override
    {
        QPainter painter(this);
        painter.setRenderHint(QPainter::Antialiasing, true);
        const QRect plot = plotRect();
        painter.fillRect(plot, Theme::panelColor().darker(118));

        int maxBin = 1;
        for (int bin : bins_) {
            maxBin = qMax(maxBin, bin);
        }
        painter.setPen(Qt::NoPen);
        painter.setBrush(QColor(150, 150, 150, 170));
        for (int i = 0; i < 256; ++i) {
            const int h = bins_[i] * plot.height() / maxBin;
            if (h <= 0) {
                continue;
            }
            const int x0 = plot.left() + i * plot.width() / 256;
            const int x1 = plot.left() + (i + 1) * plot.width() / 256;
            painter.drawRect(QRect(x0, plot.bottom() - h, qMax(1, x1 - x0), h));
        }

        painter.setPen(QColor(90, 90, 90, 150));
        for (int i = 1; i < 4; ++i) {
            const int x = plot.left() + i * plot.width() / 4;
            const int y = plot.top() + i * plot.height() / 4;
            painter.drawLine(x, plot.top(), x, plot.bottom());
            painter.drawLine(plot.left(), y, plot.right(), y);
        }

        painter.setPen(QColor(220, 220, 220));
        painter.drawLine(plot.bottomLeft(), plot.topRight());

        for (double region : s_) {
            const int x = plot.left() + qRound(region * plot.width());
            QPolygonF handle;
            handle << QPointF(x, plot.bottom() + 1) << QPointF(x - 6, plot.bottom() + 11)
                   << QPointF(x + 6, plot.bottom() + 11);
            painter.setBrush(QColor(0xdc, 0xdc, 0xdc));
            painter.setPen(QColor(0x24, 0x24, 0x24));
            painter.drawPolygon(handle);
        }
    }

    void mousePressEvent(QMouseEvent* event) override
    {
        if (event->button() != Qt::LeftButton) {
            return;
        }
        const int x = event->position().toPoint().x();
        const QRect plot = plotRect();
        int best = -1;
        int bestDistance = 12;
        for (int i = 0; i < 3; ++i) {
            const int distance = qAbs(x - (plot.left() + qRound(s_[i] * plot.width())));
            if (distance <= bestDistance) {
                bestDistance = distance;
                best = i;
            }
        }
        dragging_ = best;
        if (best >= 0) {
            mouseMoveEvent(event);
        }
    }

    void mouseMoveEvent(QMouseEvent* event) override
    {
        if (dragging_ < 0) {
            return;
        }
        const QRect plot = plotRect();
        const double raw = qBound(0.02, double(event->position().toPoint().x() - plot.left())
                                            / qMax(1, plot.width()),
                                  0.98);
        s_[dragging_] = raw;
        if (dragging_ == 0) {
            s_[0] = qMin(s_[0], s_[1] - 0.02);
        } else if (dragging_ == 2) {
            s_[2] = qMax(s_[2], s_[1] + 0.02);
        } else {
            s_[1] = qBound(s_[0] + 0.02, s_[1], s_[2] - 0.02);
        }
        update();
        if (regionsChanged) {
            regionsChanged(s_[0], s_[1], s_[2]);
        }
    }

    void mouseReleaseEvent(QMouseEvent*) override { dragging_ = -1; }

private:
    QRect plotRect() const { return rect().adjusted(2, 4, -2, -14); }

    std::array<int, 256> bins_{};
    double s_[3] = {0.25, 0.5, 0.75};
    int dragging_ = -1;
};

AcrToneCurvePanel::AcrToneCurvePanel(AcrSettings* settings, QWidget* parent)
    : AcrPanel(settings, parent)
{
    auto* column = new QVBoxLayout(this);
    column->setContentsMargins(0, 0, 0, 0);

    auto* subtabs = new QTabWidget(this);
    subtabs->setObjectName(QStringLiteral("acrToneCurveTabs"));

    auto* param = new QWidget(this);
    stylePage(param);
    auto* paramColumn = new QVBoxLayout(param);
    paramColumn->setContentsMargins(0, 0, 0, 0);

    graph_ = new AcrParamCurveGraph(param);
    graph_->setObjectName(QStringLiteral("acrCurveGraph"));
    graph_->setRegions(settings_->intValue(acr::SplitShadow, 25) / 100.0,
                       settings_->intValue(acr::SplitMid, 50) / 100.0,
                       settings_->intValue(acr::SplitHighlight, 75) / 100.0);
    paramColumn->addWidget(graph_);

    addSlider(paramColumn, acr::ParamHighlights, tr("Highlights"), -100.0, 100.0, 0);
    addSlider(paramColumn, acr::ParamLights, tr("Lights"), -100.0, 100.0, 0);
    addSlider(paramColumn, acr::ParamDarks, tr("Darks"), -100.0, 100.0, 0);
    addSlider(paramColumn, acr::ParamShadows, tr("Shadows"), -100.0, 100.0, 0);

    auto* splitShadow = addSlider(paramColumn, acr::SplitShadow, tr("Shadow Split"), 0.0, 100.0, 0, 25.0);
    auto* splitMid = addSlider(paramColumn, acr::SplitMid, tr("Midtone Split"), 0.0, 100.0, 0, 50.0);
    auto* splitHighlight =
        addSlider(paramColumn, acr::SplitHighlight, tr("Highlight Split"), 0.0, 100.0, 0, 75.0);

    graph_->regionsChanged = [this, splitShadow, splitMid, splitHighlight](double a, double b,
                                                                           double c) {
        settings_->setInt(acr::SplitShadow, qRound(a * 100.0));
        settings_->setInt(acr::SplitMid, qRound(b * 100.0));
        settings_->setInt(acr::SplitHighlight, qRound(c * 100.0));
        splitShadow->setValue(a * 100.0);
        splitMid->setValue(b * 100.0);
        splitHighlight->setValue(c * 100.0);
        emit changed();
    };
    auto syncGraph = [this, splitShadow, splitMid, splitHighlight]() {
        graph_->setRegions(splitShadow->value() / 100.0, splitMid->value() / 100.0,
                           splitHighlight->value() / 100.0);
    };
    connect(splitShadow, &AcrSlider::valueChanged, this, [syncGraph]() { syncGraph(); });
    connect(splitMid, &AcrSlider::valueChanged, this, [syncGraph]() { syncGraph(); });
    connect(splitHighlight, &AcrSlider::valueChanged, this, [syncGraph]() { syncGraph(); });

    paramColumn->addStretch(1);
    subtabs->addTab(param, tr("Parametric"));

    auto* point = new QWidget(this);
    stylePage(point);
    auto* pointColumn = new QVBoxLayout(point);
    pointColumn->setContentsMargins(0, 0, 0, 0);
    auto* channel = new QComboBox(this);
    channel->setObjectName(QStringLiteral("acrChannel"));
    channel->addItems({tr("RGB"), tr("Red"), tr("Green"), tr("Blue")});
    pointColumn->addWidget(channel);
    auto* curve = new QComboBox(this);
    curve->setObjectName(QStringLiteral("acrCurve"));
    curve->addItems({tr("Linear"), tr("Medium Contrast"), tr("Strong Contrast")});
    pointColumn->addWidget(curve);
    auto* editor = new CurveWidget(this);
    editor->setObjectName(QStringLiteral("acrCurveWidget"));
    editor->setMinimumHeight(160);
    pointColumn->addWidget(editor);
    auto* input = new QLabel(tr("Input: —"), this);
    auto* output = new QLabel(tr("Output: —"), this);
    pointColumn->addWidget(input);
    pointColumn->addWidget(output);
    pointColumn->addStretch(1);
    subtabs->addTab(point, tr("Point"));

    column->addWidget(subtabs);

    // `setPointsText` is signal-blocked, so prefilling never writes back.
    auto reload = [this, channel, editor]() {
        const QList<int> stored = settings_->curveValue(curveKeyForChannel(channel->currentText()));
        editor->setPointsText(curveText(stored.isEmpty() ? QList<int>({0, 0, 255, 255}) : stored));
    };
    reload();

    connect(channel, &QComboBox::currentTextChanged, this, [reload](const QString&) { reload(); });
    connect(curve, &QComboBox::currentTextChanged, this,
            [this, channel, curve, editor](const QString& preset) {
                const QList<int> points = curvePreset(preset);
                editor->setPointsText(curveText(points));
                settings_->setCurve(curveKeyForChannel(channel->currentText()), points);
                emit changed();
            });
    connect(editor, &CurveWidget::curveChanged, this, [this, channel, editor]() {
        settings_->setCurve(curveKeyForChannel(channel->currentText()),
                            curveFromText(editor->pointsText()));
        emit changed();
    });
    connect(editor, &CurveWidget::selectionChanged, this, [editor, input, output]() {
        const QVector<QPointF> points = editor->points();
        const int selected = editor->selected();
        if (selected >= 0 && selected < points.size()) {
            input->setText(QStringLiteral("Input: %1").arg(qRound(points[selected].x() * 255.0)));
            output->setText(QStringLiteral("Output: %1").arg(qRound(points[selected].y() * 255.0)));
        } else {
            input->setText(QStringLiteral("Input: —"));
            output->setText(QStringLiteral("Output: —"));
        }
    });
}

void AcrToneCurvePanel::setPreviewImage(const QImage& image)
{
    if (graph_) {
        graph_->setHistogram(acrLumaBins(image));
    }
}

} // namespace pictura
