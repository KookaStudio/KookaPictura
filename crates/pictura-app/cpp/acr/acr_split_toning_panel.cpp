#include "acr_panels.h"
#include "acr_settings.h"

#include <QtWidgets/QVBoxLayout>

namespace pictura {

AcrSplitToningPanel::AcrSplitToningPanel(AcrSettings* settings, QWidget* parent)
    : AcrPanel(settings, parent)
{
    auto* column = new QVBoxLayout(this);
    column->setContentsMargins(0, 0, 0, 0);

    addSection(column, tr("Highlights"));
    addSlider(column, acr::ToningHighlightHue, tr("Hue"), 0, 360, 0, 0);
    addSlider(column, acr::ToningHighlightSat, tr("Saturation"), 0, 100, 0, 0);

    addSection(column, tr("Balance"));
    addSlider(column, acr::ToningBalance, tr("Balance"), -100, 100, 0, 0);

    addSection(column, tr("Shadows"));
    addSlider(column, acr::ToningShadowHue, tr("Hue"), 0, 360, 0, 0);
    addSlider(column, acr::ToningShadowSat, tr("Saturation"), 0, 100, 0, 0);

    column->addStretch(1);
}

} // namespace pictura
