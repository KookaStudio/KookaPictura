#include "acr_panels.h"
#include "acr_settings.h"

#include "theme.h"

#include <QtGui/QColor>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {
// A smooth hue wheel, red back to red, for the Hue tracks.
QList<QColor> hueRamp()
{
    QList<QColor> stops;
    for (int i = 0; i <= 12; ++i) {
        stops.append(QColor::fromHsv((i * 30) % 360, 255, 255));
    }
    return stops;
}

// A Saturation track runs from neutral grey to the hue its slider pairs with.
QList<QColor> saturationRamp(int hue)
{
    return {Theme::panelColor().darker(135), QColor::fromHsv(hue % 360, 255, 255)};
}
} // namespace

AcrSplitToningPanel::AcrSplitToningPanel(AcrSettings* settings, QWidget* parent)
    : AcrPanel(settings, parent)
{
    auto* column = new QVBoxLayout(this);
    column->setContentsMargins(0, 0, 0, 0);

    addSection(column, tr("Highlights"));
    auto* highlightHue =
        addSlider(column, acr::ToningHighlightHue, tr("Hue"), 0, 360, 0, 0, hueRamp());
    auto* highlightSat = addSlider(column, acr::ToningHighlightSat, tr("Saturation"), 0, 100, 0, 0);

    addSection(column, tr("Balance"));
    addSlider(column, acr::ToningBalance, tr("Balance"), -100, 100, 0, 0);

    addSection(column, tr("Shadows"));
    auto* shadowHue = addSlider(column, acr::ToningShadowHue, tr("Hue"), 0, 360, 0, 0, hueRamp());
    auto* shadowSat = addSlider(column, acr::ToningShadowSat, tr("Saturation"), 0, 100, 0, 0);

    const auto syncRamp = [](AcrSlider* hue, AcrSlider* sat) {
        sat->setRamp(saturationRamp(qRound(hue->value()) % 360));
    };
    connect(highlightHue, &AcrSlider::valueChanged, this,
            [highlightHue, highlightSat, syncRamp]() { syncRamp(highlightHue, highlightSat); });
    connect(shadowHue, &AcrSlider::valueChanged, this,
            [shadowHue, shadowSat, syncRamp]() { syncRamp(shadowHue, shadowSat); });
    syncRamp(highlightHue, highlightSat);
    syncRamp(shadowHue, shadowSat);

    column->addStretch(1);
}

} // namespace pictura
