#include "acr_panels.h"
#include "acr_settings.h"

#include <QtWidgets/QVBoxLayout>

namespace pictura {

AcrDetailPanel::AcrDetailPanel(AcrSettings* settings, QWidget* parent)
    : AcrPanel(settings, parent)
{
    auto* column = new QVBoxLayout(this);
    column->setContentsMargins(0, 0, 0, 0);

    addSection(column, tr("Sharpening"));
    addSlider(column, acr::SharpenAmount, tr("Amount"), 0.0, 150.0, 0, 25.0);
    addSlider(column, acr::SharpenRadius, tr("Radius"), 0.5, 3.0, 1, 1.0);
    addSlider(column, acr::SharpenDetail, tr("Detail"), 0.0, 100.0, 0, 25.0);
    addSlider(column, acr::SharpenMasking, tr("Masking"), 0.0, 100.0, 0, 0.0);

    addSection(column, tr("Noise Reduction"));
    addSlider(column, acr::NoiseLuminance, tr("Luminance"), 0.0, 100.0, 0, 0.0);
    addSlider(column, acr::NoiseLuminanceDetail, tr("Luminance Detail"), 0.0, 100.0, 0, 50.0);
    addSlider(column, acr::NoiseLuminanceContrast, tr("Luminance Contrast"), 0.0, 100.0, 0, 0.0);
    addSlider(column, acr::NoiseColor, tr("Color"), 0.0, 100.0, 0, 25.0);
    addSlider(column, acr::NoiseColorDetail, tr("Color Detail"), 0.0, 100.0, 0, 50.0);
    addSlider(column, acr::NoiseColorSmoothness, tr("Color Smoothness"), 0.0, 100.0, 0, 50.0);

    column->addStretch(1);
}

} // namespace pictura
