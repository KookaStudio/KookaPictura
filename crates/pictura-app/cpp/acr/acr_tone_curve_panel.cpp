#include "acr_panels.h"
#include "acr_settings.h"
#include "panels/curve_widget.h"

#include <QtCore/QPointF>
#include <QtCore/QStringList>
#include <QtCore/QVector>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QTabWidget>
#include <QtWidgets/QVBoxLayout>

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
} // namespace

AcrToneCurvePanel::AcrToneCurvePanel(AcrSettings* settings, QWidget* parent)
    : AcrPanel(settings, parent)
{
    auto* column = new QVBoxLayout(this);
    column->setContentsMargins(0, 0, 0, 0);

    auto* subtabs = new QTabWidget(this);
    subtabs->setObjectName(QStringLiteral("acrToneCurveTabs"));

    auto* param = new QWidget(this);
    auto* paramColumn = new QVBoxLayout(param);
    paramColumn->setContentsMargins(0, 0, 0, 0);
    addSlider(paramColumn, acr::ParamShadows, tr("Shadows"), -100.0, 100.0, 0);
    addSlider(paramColumn, acr::ParamDarks, tr("Darks"), -100.0, 100.0, 0);
    addSlider(paramColumn, acr::ParamLights, tr("Lights"), -100.0, 100.0, 0);
    addSlider(paramColumn, acr::ParamHighlights, tr("Highlights"), -100.0, 100.0, 0);
    addSlider(paramColumn, acr::SplitShadow, tr("Shadow Split"), 0.0, 100.0, 0, 25.0);
    addSlider(paramColumn, acr::SplitMid, tr("Midtone Split"), 0.0, 100.0, 0, 50.0);
    addSlider(paramColumn, acr::SplitHighlight, tr("Highlight Split"), 0.0, 100.0, 0, 75.0);
    paramColumn->addStretch(1);
    subtabs->addTab(param, tr("Parametric"));

    auto* point = new QWidget(this);
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

} // namespace pictura
