#include "acr_panels.h"
#include "acr_settings.h"

#include <QtGui/QColor>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {
struct BasicKey {
    const QByteArray& key;
    bool isDouble;
};

const QList<QColor> kTemperatureRamp = {QColor(0x2b5d8a), QColor(0xf2d06b)};
const QList<QColor> kTintRamp = {QColor(0x3fae4a), QColor(0xd83f9e)};
const QList<QColor> kToneRamp = {QColor(0x33, 0x33, 0x33), QColor(0xdd, 0xdd, 0xdd)};
} // namespace

AcrBasicPanel::AcrBasicPanel(AcrSettings* settings, QWidget* parent,
                             std::optional<AcrAsShot> asShot)
    : AcrPanel(settings, parent), asShot_(asShot)
{
    auto* column = new QVBoxLayout(this);
    column->setContentsMargins(0, 0, 0, 0);

    // ponytail: the ACR wire codes for the WB enum; only `Cst ` (custom) is
    // verified against a fixture. An unknown stored code is preserved.
    addEnumCombo(column, acr::WBal, tr("White Balance"),
                 {{tr("As Shot"), QByteArrayLiteral("AsSh")},
                  {tr("Auto"), QByteArrayLiteral("Auto")},
                  {tr("Daylight"), QByteArrayLiteral("Dayl")},
                  {tr("Cloudy"), QByteArrayLiteral("Clou")},
                  {tr("Shade"), QByteArrayLiteral("Shad")},
                  {tr("Tungsten"), QByteArrayLiteral("Tung")},
                  {tr("Fluorescent"), QByteArrayLiteral("Fluo")},
                  {tr("Flash"), QByteArrayLiteral("Flsh")},
                  {tr("Custom"), QByteArrayLiteral("Cst ")}});
    if (asShot_) {
        // ponytail: the colour stage's locus is stated for 1000–15000 K, so
        // settings above 15000 K render as 15000 K.
        addSlider(column, acr::Temp, tr("Temperature"), 2000.0, 50000.0, 0,
                  asShot_->temperature, kTemperatureRamp)
            ->setReciprocalScale(true);
        addSlider(column, acr::Tint, tr("Tint"), -150.0, 150.0, 0, asShot_->tint, kTintRamp);
    } else {
        addSlider(column, acr::Temp, tr("Temperature"), -100.0, 100.0, 0, 0, kTemperatureRamp);
        addSlider(column, acr::Tint, tr("Tint"), -100.0, 100.0, 0, 0, kTintRamp);
    }

    addSeparator(column);

    auto* links = new QHBoxLayout;
    links->addStretch(1);
    auto* autoButton = new QPushButton(tr("Auto"), this);
    autoButton->setFlat(true);
    auto* defaultButton = new QPushButton(tr("Default"), this);
    defaultButton->setFlat(true);
    links->addWidget(autoButton);
    links->addWidget(defaultButton);
    column->addLayout(links);
    connect(autoButton, &QPushButton::clicked, this, &AcrBasicPanel::resetBasic);
    connect(defaultButton, &QPushButton::clicked, this, &AcrBasicPanel::resetBasic);

    addSlider(column, acr::Exposure, tr("Exposure"), -5.0, 5.0, 2, 0, kToneRamp);
    addSlider(column, acr::Contrast, tr("Contrast"), -100.0, 100.0, 0);
    addSlider(column, acr::Highlights, tr("Highlights"), -100.0, 100.0, 0);
    addSlider(column, acr::Shadows, tr("Shadows"), -100.0, 100.0, 0);
    addSlider(column, acr::Whites, tr("Whites"), -100.0, 100.0, 0);
    addSlider(column, acr::Blacks, tr("Blacks"), -100.0, 100.0, 0);

    addSeparator(column);

    addSlider(column, acr::Clarity, tr("Clarity"), -100.0, 100.0, 0);
    addSlider(column, acr::Vibrance, tr("Vibrance"), -100.0, 100.0, 0);
    addSlider(column, acr::Saturation, tr("Saturation"), -100.0, 100.0, 0);
    column->addStretch(1);
}

void AcrBasicPanel::resetBasic()
{
    // ponytail: ACR's Auto computes values we do not model; both links reset the
    // Basic controls to their defaults.
    const BasicKey keys[11] = {
        {acr::Temp, false},      {acr::Tint, false},     {acr::Exposure, true},
        {acr::Contrast, false},  {acr::Highlights, false}, {acr::Shadows, false},
        {acr::Whites, false},    {acr::Blacks, false},   {acr::Clarity, false},
        {acr::Vibrance, false},  {acr::Saturation, false},
    };
    // A raw source's white balance resets to the as-shot values, not 0.
    for (const BasicKey& entry : keys) {
        int value = 0;
        if (asShot_ && entry.key == acr::Temp) {
            value = asShot_->temperature;
        } else if (asShot_ && entry.key == acr::Tint) {
            value = asShot_->tint;
        }
        if (entry.isDouble) {
            settings_->setDouble(entry.key, value);
        } else {
            settings_->setInt(entry.key, value);
        }
        if (auto* slider = findChild<AcrSlider*>(QString::fromLatin1(entry.key))) {
            slider->setValue(value);
        }
    }
    emit changed();
}

} // namespace pictura

#include "moc_acr_panels.cpp"
