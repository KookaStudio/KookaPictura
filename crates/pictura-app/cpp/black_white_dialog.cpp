#include "black_white_dialog.h"

#include "color_picker_dialog.h"
#include "panels/color_swatch_widget.h"
#include "panels/ramp_slider.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

#include <array>

namespace pictura {

namespace {

struct Channel {
    const char* label;
    const char* key;
    QColor color;
};

const Channel kChannels[] = {
    {"Reds", "reds", QColor(230, 40, 40)},     {"Yellows", "yellows", QColor(225, 210, 30)},
    {"Greens", "greens", QColor(40, 200, 60)}, {"Cyans", "cyans", QColor(30, 200, 225)},
    {"Blues", "blues", QColor(50, 70, 225)},   {"Magentas", "magentas", QColor(210, 40, 200)},
};

struct Preset {
    const char* name;
    std::array<int, 6> weights;
};

// CS6's Preset menu, in its order; the weights are photorust's table.
// ponytail: the names come from a CS6 capture but the weights are unverified
// (docs/04-image-ops/adjustments/black-white.md lists them as open); Neutral
// Density reads as Default.
constexpr Preset kDefault = {"Default", {40, 60, 40, 60, 20, 80}};
constexpr Preset kPresets[] = {
    {"Blue Filter", {25, 35, 25, 10, 300, 25}},
    {"Darker", {20, 40, 20, 40, 0, 60}},
    {"Green Filter", {25, 35, 300, 10, 20, 25}},
    {"High Contrast Blue Filter", {10, 0, 10, 0, 300, 10}},
    {"High Contrast Red Filter", {300, 0, 10, 0, 10, 10}},
    {"Infrared", {-70, 200, -50, -200, -150, 100}},
    {"Lighter", {60, 80, 60, 80, 40, 100}},
    {"Maximum Black", {0, 0, 0, 0, 0, 0}},
    {"Maximum White", {100, 100, 100, 100, 100, 100}},
    {"Neutral Density", {40, 60, 40, 60, 20, 80}},
    {"Red Filter", {300, 35, 25, 10, 20, 25}},
    {"Yellow Filter", {25, 300, 25, 10, 20, 25}},
};
const QString kCustom = QStringLiteral("Custom");

} // namespace

BlackWhiteDialog::BlackWhiteDialog(PictureView* view, const QByteArray& block,
                                   const QRect& visible, QWidget* parent)
    : AdjustmentDialog(view, block, visible, QStringLiteral("Black & White"), parent)
{
    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(14, 12, 14, 12);
    layout->setSpacing(6);

    auto* presetRow = new QHBoxLayout;
    presetRow->addWidget(new QLabel(QStringLiteral("Preset:"), this));
    preset_ = new QComboBox(this);
    preset_->setObjectName(QStringLiteral("blackWhitePreset"));
    preset_->addItem(QLatin1String(kDefault.name));
    preset_->insertSeparator(1);
    for (const Preset& preset : kPresets) {
        preset_->addItem(QLatin1String(preset.name));
    }
    preset_->insertSeparator(preset_->count());
    preset_->addItem(kCustom);
    presetRow->addWidget(preset_, 1);
    layout->addLayout(presetRow);
    layout->addSpacing(6);

    for (const Channel& channel : kChannels) {
        QSpinBox* spin =
            addRampRow(layout, QLatin1String(channel.label), QLatin1String(channel.key), -200, 300,
                       {Qt::black, channel.color}, QStringLiteral(" %"));
        connect(spin, &QSpinBox::valueChanged, this, &BlackWhiteDialog::markCustom);
        layout->addSpacing(4);
    }
    markCustom();

    auto* tintRow = new QHBoxLayout;
    auto* tint = new QCheckBox(QStringLiteral("Tint"), this);
    tint->setObjectName(QStringLiteral("tint"));
    tint->setChecked(param(QStringLiteral("tint")) != 0.0);
    color_ = swatch(QStringLiteral("tintColor"));
    const QColor opening = QColor::fromRgb(QRgb(uint(param(QStringLiteral("tintColor")))));
    tintRow->addWidget(tint);
    tintRow->addWidget(color_);
    tintRow->addStretch(1);
    layout->addLayout(tintRow);

    // CS6's tint Hue and Saturation edit the swatch's HSV hue and saturation;
    // its brightness stays put.
    const auto addTintRow = [this, layout](const QString& label, const QString& key, int max,
                                           const QString& suffix) {
        auto* head = new QHBoxLayout;
        head->addWidget(new QLabel(label, this));
        head->addStretch(1);
        auto* spin = new QSpinBox(this);
        spin->setObjectName(key);
        spin->setRange(0, max);
        spin->setSuffix(suffix);
        spin->setFixedWidth(kFieldWidth);
        head->addWidget(spin);
        layout->addLayout(head);
        auto* slider = new RampSlider(this);
        slider->setObjectName(key + QStringLiteral("Slider"));
        slider->setRange(0, max);
        layout->addWidget(slider);
        connect(slider, &QSlider::valueChanged, spin, &QSpinBox::setValue);
        connect(spin, &QSpinBox::valueChanged, slider, &QSlider::setValue);
        connect(spin, &QSpinBox::valueChanged, this, &BlackWhiteDialog::tintEdited);
        return spin;
    };
    hue_ = addTintRow(QStringLiteral("Hue"), QStringLiteral("tintHue"), 360, QStringLiteral("°"));
    saturation_ = addTintRow(QStringLiteral("Saturation"), QStringLiteral("tintSaturation"), 100,
                             QStringLiteral(" %"));
    QList<QColor> spectrum;
    for (int h = 0; h <= 360; h += 60) {
        spectrum << QColor::fromHsv(h % 360, 255, 255);
    }
    findChild<RampSlider*>(QStringLiteral("tintHueSlider"))->setRamp(spectrum);
    paintTint(opening);
    showTint(opening);
    const auto enableTint = [this](bool on) {
        color_->setEnabled(on);
        for (QSpinBox* spin : {hue_, saturation_}) {
            spin->setEnabled(on);
            findChild<QWidget*>(spin->objectName() + QStringLiteral("Slider"))->setEnabled(on);
        }
    };
    enableTint(tint->isChecked());

    layout->addSpacing(4);
    layout->addWidget(previewCheck());
    layout->addStretch(1);
    layout->addLayout(bottomButtons());

    connect(tint, &QCheckBox::toggled, this, [this, enableTint](bool on) {
        enableTint(on);
        setParam(QStringLiteral("tint"), on ? 1.0 : 0.0);
    });
    connect(color_, &QPushButton::clicked, this, [this]() {
        const QColor picked = ColorPickerDialog::getColor(
            color_->property("color").value<QColor>(), this, QStringLiteral("Tint Color"));
        if (picked.isValid()) {
            setTint(picked);
            showTint(picked);
        }
    });
    connect(preset_, &QComboBox::activated, this, &BlackWhiteDialog::applyPreset);
    setMinimumWidth(460);
}

void BlackWhiteDialog::applyPreset(int index)
{
    const QString name = preset_->itemText(index);
    const Preset* chosen = name == QLatin1String(kDefault.name) ? &kDefault : nullptr;
    for (const Preset& preset : kPresets) {
        if (name == QLatin1String(preset.name)) {
            chosen = &preset;
        }
    }
    if (!chosen) {
        return;
    }
    QList<QPair<QString, double>> edits;
    for (int i = 0; i < int(std::size(kChannels)); ++i) {
        edits << qMakePair(QLatin1String(kChannels[i].key), double(chosen->weights[i]));
    }
    setParams(edits);
    for (int i = 0; i < int(std::size(kChannels)); ++i) {
        auto* spin = findChild<QSpinBox*>(QLatin1String(kChannels[i].key));
        const QSignalBlocker block(spin);
        spin->setValue(chosen->weights[i]);
        findChild<QWidget*>(QLatin1String(kChannels[i].key) + QStringLiteral("Slider"))
            ->setProperty("value", chosen->weights[i]);
    }
}

// The menu names the preset the six weights match, else Custom.
void BlackWhiteDialog::markCustom()
{
    std::array<int, 6> current{};
    for (int i = 0; i < int(std::size(kChannels)); ++i) {
        current[i] = findChild<QSpinBox*>(QLatin1String(kChannels[i].key))->value();
    }
    QString name = kCustom;
    if (current == kDefault.weights) {
        name = QLatin1String(kDefault.name);
    } else {
        for (const Preset& preset : kPresets) {
            if (current == preset.weights) {
                name = QLatin1String(preset.name);
                break;
            }
        }
    }
    const QSignalBlocker block(preset_);
    preset_->setCurrentIndex(preset_->findText(name));
}

void BlackWhiteDialog::tintEdited()
{
    if (showingTint_) {
        return;
    }
    const QColor current = color_->property("color").value<QColor>();
    const QColor next = QColor::fromHsv(hue_->value() % 360,
                                        qRound(saturation_->value() * 255 / 100.0),
                                        current.value());
    setTint(next);
}

void BlackWhiteDialog::setTint(const QColor& color)
{
    paintTint(color);
    setParam(QStringLiteral("tintColor"), double(color.rgb() & 0xffffffu));
}

// The swatch, and the Saturation groove from gray to the swatch's full hue.
void BlackWhiteDialog::paintTint(const QColor& color)
{
    paintSwatch(color_, color);
    const int hue = qMax(0, color.hsvHue());
    findChild<RampSlider*>(QStringLiteral("tintSaturationSlider"))
        ->setRamp({QColor::fromHsv(hue, 0, color.value()), QColor::fromHsv(hue, 255, color.value())});
}

void BlackWhiteDialog::showTint(const QColor& color)
{
    showingTint_ = true;
    hue_->setValue(qMax(0, color.hsvHue()));
    saturation_->setValue(qRound(color.hsvSaturation() * 100 / 255.0));
    showingTint_ = false;
}

QWidget* BlackWhiteDialog::controlForTest(const QString& key) const
{
    if (key == QLatin1String("preset")) {
        return preset_;
    }
    return findChild<QWidget*>(key);
}

} // namespace pictura

#include "moc_black_white_dialog.cpp"
