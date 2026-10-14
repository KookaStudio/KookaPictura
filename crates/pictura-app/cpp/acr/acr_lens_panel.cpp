#include "acr_panels.h"
#include "acr_settings.h"

#include <QtWidgets/QLabel>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

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
    addSlider(column, acr::DefringePurpleAmount, tr("Purple Amount"), 0, 100, 0, 0);
    addSlider(column, acr::DefringePurpleHueHi, tr("Purple Hue"), 0, 100, 0, 0);
    addSlider(column, acr::DefringeGreenAmount, tr("Green Amount"), 0, 100, 0, 0);
    addSlider(column, acr::DefringeGreenHueHi, tr("Green Hue"), 0, 100, 0, 0);

    addSection(column, tr("Vignetting"));
    addSlider(column, acr::LensVignetteAmount, tr("Amount"), -100, 100, 0, 0);
    addSlider(column, acr::LensVignetteMidpoint, tr("Midpoint"), 0, 100, 0, 50);

    column->addStretch(1);
}

} // namespace pictura
