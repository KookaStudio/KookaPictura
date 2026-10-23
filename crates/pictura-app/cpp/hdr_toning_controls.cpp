#include "hdr_toning_controls.h"

#include "panels/jump_slider.h"

#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QSlider>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QVBoxLayout>

#include <cmath>

namespace pictura {

namespace {

QHBoxLayout* labeledRow(const QString& text, QWidget* control)
{
    auto* row = new QHBoxLayout;
    auto* label = new QLabel(text);
    label->setFixedWidth(70);
    label->setAlignment(Qt::AlignRight | Qt::AlignVCenter);
    row->addWidget(label);
    row->addWidget(control, 1);
    return row;
}

} // namespace

HdrToningControls::HdrToningControls(QWidget* parent)
    : QWidget(parent)
{
    auto* outer = new QVBoxLayout(this);
    outer->setContentsMargins(0, 0, 0, 0);

    const auto makeIntRow = [this](const QString& label, const QString& name, int min, int max,
                                   int def, QSpinBox*& spin) -> QHBoxLayout* {
        auto* row = new QHBoxLayout;
        auto* caption = new QLabel(label);
        caption->setFixedWidth(70);
        caption->setAlignment(Qt::AlignRight | Qt::AlignVCenter);
        row->addWidget(caption);
        auto* slider = new JumpSlider(Qt::Horizontal);
        slider->setRange(min, max);
        slider->setValue(def);
        row->addWidget(slider, 1);
        spin = new QSpinBox;
        spin->setObjectName(name);
        spin->setRange(min, max);
        spin->setValue(def);
        spin->setFixedWidth(70);
        row->addWidget(spin);
        connect(slider, &QSlider::valueChanged, spin, &QSpinBox::setValue);
        connect(spin, qOverload<int>(&QSpinBox::valueChanged), slider, &QSlider::setValue);
        connect(spin, qOverload<int>(&QSpinBox::valueChanged), this, [this] {
            if (!loading_) {
                Q_EMIT edited();
            }
        });
        return row;
    };

    const auto makeDoubleRow = [this](const QString& label, const QString& name, double min,
                                      double max, double def, double step,
                                      QDoubleSpinBox*& spin) -> QHBoxLayout* {
        auto* row = new QHBoxLayout;
        auto* caption = new QLabel(label);
        caption->setFixedWidth(70);
        caption->setAlignment(Qt::AlignRight | Qt::AlignVCenter);
        row->addWidget(caption);
        auto* slider = new JumpSlider(Qt::Horizontal);
        const int scale = int(std::lround(1.0 / step));
        slider->setRange(int(std::lround(min * scale)), int(std::lround(max * scale)));
        slider->setValue(int(std::lround(def * scale)));
        row->addWidget(slider, 1);
        spin = new QDoubleSpinBox;
        spin->setObjectName(name);
        spin->setRange(min, max);
        spin->setValue(def);
        spin->setSingleStep(step);
        spin->setDecimals(2);
        spin->setFixedWidth(70);
        row->addWidget(spin);
        connect(slider, &QSlider::valueChanged, this,
                [spin, step](int v) { spin->setValue(step * v); });
        connect(spin, qOverload<double>(&QDoubleSpinBox::valueChanged), slider,
                [slider, scale](double v) { slider->setValue(int(std::lround(v * scale))); });
        connect(spin, qOverload<double>(&QDoubleSpinBox::valueChanged), this, [this] {
            if (!loading_) {
                Q_EMIT edited();
            }
        });
        return row;
    };

    auto* edge = new QGroupBox(QStringLiteral("Edge Glow"));
    auto* edgeLayout = new QVBoxLayout(edge);
    edgeLayout->addLayout(makeIntRow(QStringLiteral("Radius:"), QStringLiteral("hdrRadius"), 1, 500,
                                     187, radius_));
    edgeLayout->addLayout(makeDoubleRow(QStringLiteral("Strength:"), QStringLiteral("hdrStrength"),
                                        0.01, 4.0, 4.0, 0.01, strength_));
    outer->addWidget(edge);

    auto* tone = new QGroupBox(QStringLiteral("Tone and Detail"));
    auto* toneLayout = new QVBoxLayout(tone);
    toneLayout->addLayout(makeDoubleRow(QStringLiteral("Gamma:"), QStringLiteral("hdrGamma"), 0.01,
                                        9.99, 0.99, 0.01, gamma_));
    toneLayout->addLayout(makeDoubleRow(QStringLiteral("Exposure:"), QStringLiteral("hdrExposure"),
                                        -5.0, 5.0, 0.0, 0.01, exposure_));
    toneLayout->addLayout(makeIntRow(QStringLiteral("Detail:"), QStringLiteral("hdrDetail"), -100,
                                     300, 30, detail_));
    outer->addWidget(tone);

    auto* advanced = new QGroupBox(QStringLiteral("Advanced"));
    auto* advancedLayout = new QVBoxLayout(advanced);
    advancedLayout->addLayout(
        makeIntRow(QStringLiteral("Shadow:"), QStringLiteral("hdrShadow"), -100, 100, 0, shadow_));
    advancedLayout->addLayout(makeIntRow(QStringLiteral("Highlight:"),
                                         QStringLiteral("hdrHighlight"), -100, 100, 0, highlight_));
    advancedLayout->addLayout(makeIntRow(QStringLiteral("Vibrance:"), QStringLiteral("hdrVibrance"),
                                         -100, 100, 0, vibrance_));
    advancedLayout->addLayout(makeIntRow(QStringLiteral("Saturation:"),
                                         QStringLiteral("hdrSaturation"), -100, 100, 20,
                                         saturation_));
    outer->addWidget(advanced);
}

HdrToningControls::Values HdrToningControls::values() const
{
    Values result;
    result.radius = radius_->value();
    result.strength = strength_->value();
    result.gamma = gamma_->value();
    result.exposure = exposure_->value();
    result.detail = detail_->value();
    result.shadow = shadow_->value();
    result.highlight = highlight_->value();
    result.vibrance = vibrance_->value();
    result.saturation = saturation_->value();
    return result;
}

void HdrToningControls::setValues(const Values& values)
{
    loading_ = true;
    radius_->setValue(values.radius);
    strength_->setValue(values.strength);
    gamma_->setValue(values.gamma);
    exposure_->setValue(values.exposure);
    detail_->setValue(values.detail);
    shadow_->setValue(values.shadow);
    highlight_->setValue(values.highlight);
    vibrance_->setValue(values.vibrance);
    saturation_->setValue(values.saturation);
    loading_ = false;
}

} // namespace pictura

#include "moc_hdr_toning_controls.cpp"
