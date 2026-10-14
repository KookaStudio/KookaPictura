#include "acr_panels.h"
#include "acr_settings.h"

#include <QtWidgets/QComboBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

AcrEffectsPanel::AcrEffectsPanel(AcrSettings* settings, QWidget* parent)
    : AcrPanel(settings, parent)
{
    auto* column = new QVBoxLayout(this);
    column->setContentsMargins(0, 0, 0, 0);

    addSection(column, tr("Dehaze"));
    addSlider(column, acr::Dehaze, tr("Amount"), -100, 100, 0, 0);

    addSection(column, tr("Grain"));
    addSlider(column, acr::GrainAmount, tr("Amount"), 0, 100, 0, 0);
    addSlider(column, acr::GrainSize, tr("Size"), 0, 100, 0, 25);
    addSlider(column, acr::GrainFrequency, tr("Roughness"), 0, 100, 0, 50);

    addSection(column, tr("Post Crop Vignetting"));
    auto* styleRow = new QHBoxLayout;
    styleRow->addWidget(new QLabel(tr("Style"), this));
    styleRow->addStretch(1);
    auto* styleCombo = new QComboBox(this);
    styleCombo->setObjectName(QStringLiteral("acrPcvStyle"));
    styleCombo->addItems({tr("Highlight Priority"), tr("Color Priority"), tr("Paint Overlay")});
    styleCombo->setCurrentIndex(qBound(0, settings_->intValue(acr::PcvStyle, 0), 2));
    styleRow->addWidget(styleCombo);
    column->addLayout(styleRow);
    connect(styleCombo, &QComboBox::currentIndexChanged, this, [this](int index) {
        settings_->setInt(acr::PcvStyle, index);
        emit changed();
    });

    AcrSlider* amount = addSlider(column, acr::PcvAmount, tr("Amount"), -100, 100, 0, 0);
    addSlider(column, acr::PcvMidpoint, tr("Midpoint"), 0, 100, 0, 50);
    addSlider(column, acr::PcvRoundness, tr("Roundness"), -100, 100, 0, 0);
    addSlider(column, acr::PcvFeather, tr("Feather"), 0, 100, 0, 50);
    addSlider(column, acr::PcvHighlights, tr("Highlights"), 0, 100, 0, 0);

    styleCombo->setEnabled(amount->value() != 0.0);
    connect(amount, &AcrSlider::valueChanged, styleCombo, [styleCombo, amount] {
        styleCombo->setEnabled(amount->value() != 0.0);
    });

    column->addStretch(1);
}

} // namespace pictura
