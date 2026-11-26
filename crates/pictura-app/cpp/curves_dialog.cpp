#include "curves_dialog.h"

#include "api/document_bridge.h"
#include "panels/curves_controls.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

CurvesDialog::CurvesDialog(PictureView* view, const QByteArray& block, const QRect& visible,
                           QWidget* parent)
    : AdjustmentDialog(view, block, visible, QStringLiteral("Curves"), parent)
{
    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(14, 12, 14, 12);
    layout->setSpacing(8);
    addPresetRow(layout);
    curves_ = new CurvesControls(true, this);
    curves_->setCurveSource([this](int channel) { return curve(channel); });
    const QString mode = view ? image_mode(*view) : QStringLiteral("rgb");
    CurveHistograms histograms{};
    if (mode == QLatin1String("rgb") || mode == QLatin1String("grayscale")) {
        for (int i = 0; i < 4; ++i) {
            histograms[i] = histogram(i);
        }
    } else {
        histograms = curvesHistograms(view, mode);
    }
    curves_->setContext(mode, histograms);
    curves_->reload();
    layout->addWidget(curves_);
    layout->addWidget(previewCheck());
    layout->addLayout(bottomButtons({QStringLiteral("Auto")}));
    button(QStringLiteral("Auto"))->setEnabled(curves_->autoAvailable());

    connect(curves_, &CurvesControls::curveChanged, this, [this](int channel, const QString& points) {
        if (!setCurve(channel, points)) {
            curves_->reload();
        }
    });
    connect(curves_, &CurvesControls::grayPointPicked, this, [this](const QColor& color) {
        setBlock(curvesGrayPointBlock(block(), color));
    });
    connect(this, &QDialog::finished, curves_, &CurvesControls::disarm);
}

void CurvesDialog::setCanvasSampling(ToolController* tools)
{
    AdjustmentDialog::setCanvasSampling(tools);
    curves_->setCanvas(tools, [this](const QPoint& pos) { return originalColorAt(pos); });
}

void CurvesDialog::reloadFromBlock() { curves_->reload(); }

void CurvesDialog::applyAuto() { curves_->applyAuto(); }

QWidget* CurvesDialog::controlForTest(const QString& key) const
{
    return curves_->findChild<QWidget*>(key);
}

} // namespace pictura

#include "moc_curves_dialog.cpp"
