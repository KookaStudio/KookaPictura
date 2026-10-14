#include "acr_panels.h"
#include "acr_settings.h"

#include <QtGui/QColor>
#include <QtWidgets/QLabel>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {
QList<QColor> spectrumRamp()
{
    return {QColor(0xff, 0x00, 0x00), QColor(0xff, 0xff, 0x00), QColor(0x00, 0xff, 0x00),
            QColor(0x00, 0xff, 0xff), QColor(0x00, 0x00, 0xff), QColor(0xff, 0x00, 0xff),
            QColor(0xff, 0x00, 0x00)};
}

const QList<QColor> kPurpleRamp = {QColor(0x33, 0x33, 0x33), QColor(0x9b, 0x4f, 0xc0)};
const QList<QColor> kGreenRamp = {QColor(0x33, 0x33, 0x33), QColor(0x3f, 0xae, 0x4a)};
} // namespace

AcrLensPanel::AcrLensPanel(AcrSettings* settings, QWidget* parent)
    : AcrPanel(settings, parent)
{
    auto* column = new QVBoxLayout(this);
    column->setContentsMargins(0, 0, 0, 0);

    auto* banner = new QLabel(tr("Upright has moved to the new Transform tool."), this);
    banner->setWordWrap(true);
    column->addWidget(banner);

    addSection(column, tr("Distortion"));
    addSlider(column, acr::Distortion, tr("Amount"), -100, 100, 0, 0);

    addSection(column, tr("Defringe"));
    addSlider(column, acr::DefringePurpleAmount, tr("Purple Amount"), 0, 100, 0, 0, kPurpleRamp);
    addRangeSlider(column, acr::DefringePurpleHueLo, acr::DefringePurpleHueHi, tr("Purple Hue"), 0,
                   100, 0, 30, 70, spectrumRamp());
    addSlider(column, acr::DefringeGreenAmount, tr("Green Amount"), 0, 100, 0, 0, kGreenRamp);
    addRangeSlider(column, acr::DefringeGreenHueLo, acr::DefringeGreenHueHi, tr("Green Hue"), 0, 100,
                   0, 40, 60, spectrumRamp());

    addSection(column, tr("Vignetting"));
    addSlider(column, acr::LensVignetteAmount, tr("Amount"), -100, 100, 0, 0);
    addSlider(column, acr::LensVignetteMidpoint, tr("Midpoint"), 0, 100, 0, 50);

    column->addStretch(1);
}

} // namespace pictura
