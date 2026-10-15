#include "acr_panels.h"
#include "acr_settings.h"

#include <QtGui/QColor>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {
QList<QColor> spectrumRamp()
{
    return {QColor(0xff, 0x00, 0x00), QColor(0xff, 0xff, 0x00), QColor(0x00, 0xff, 0x00),
            QColor(0x00, 0xff, 0xff), QColor(0x00, 0x00, 0xff), QColor(0xff, 0x00, 0xff),
            QColor(0xff, 0x00, 0x00)};
}
} // namespace

AcrSplitToningPanel::AcrSplitToningPanel(AcrSettings* settings, QWidget* parent)
    : AcrPanel(settings, parent)
{
    auto* column = new QVBoxLayout(this);
    column->setContentsMargins(0, 0, 0, 0);

    addSection(column, tr("Highlights"));
    addSlider(column, acr::ToningHighlightHue, tr("Hue"), 0, 360, 0, 0, spectrumRamp());
    addSlider(column, acr::ToningHighlightSat, tr("Saturation"), 0, 100, 0, 0);

    addSection(column, tr("Balance"));
    addSlider(column, acr::ToningBalance, tr("Balance"), -100, 100, 0, 0);

    addSection(column, tr("Shadows"));
    addSlider(column, acr::ToningShadowHue, tr("Hue"), 0, 360, 0, 0, spectrumRamp());
    addSlider(column, acr::ToningShadowSat, tr("Saturation"), 0, 100, 0, 0);

    column->addStretch(1);
}

} // namespace pictura
