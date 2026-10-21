#include "hdr_toning_dialog.h"

#include "dialogs.h"
#include "panels/jump_slider.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_hdr_toning.cxxqt.h"

#include <QtCore/QSignalBlocker>
#include <QtCore/QTimer>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QSlider>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QVBoxLayout>

#include <cmath>
#include <iterator>

namespace pictura {

namespace {

// The 17 CS6 Local Adaptation presets, in the dialog's order.
struct Preset {
    const char* name;
    int radius;
    double strength;
    double gamma;
    double exposure;
    int detail;
    int shadow;
    int highlight;
    int vibrance;
    int saturation;
};

const Preset kPresets[] = {
    {"Default", 187, 4.00, 0.99, 0.00, 30, 0, 0, 0, 20},
    {"City Twilight", 383, 1.14, 4.43, 0.68, 98, 27, 21, 63, -3},
    {"Flat", 200, 1.00, 1.00, 0.00, 0, 0, 0, 0, 0},
    {"Monochromatic Artistic", 101, 3.45, 2.02, -1.20, 288, 100, -100, -100, -100},
    {"Monochromatic High Contrast", 240, 2.06, 1.80, 0.88, 200, -60, 50, -100, -100},
    {"Monochromatic Low Contrast", 1, 0.10, 0.80, 0.00, -100, -10, 40, -100, -100},
    {"Monochromatic", 100, 2.00, 1.00, -0.50, 80, 0, 0, -100, -100},
    {"More Saturated", 288, 1.75, 0.27, -0.15, 110, 0, -90, 59, 100},
    {"Photorealistic High Contrast", 25, 1.67, 1.00, 0.00, 60, -60, -60, 20, 10},
    {"Photorealistic Low Contrast", 50, 1.00, 1.50, 0.00, 20, 20, 20, 10, 10},
    {"Photorealistic", 25, 1.67, 1.26, 0.00, 46, -50, -50, 30, 10},
    {"RCS", 76, 2.21, 4.32, 0.75, 54, -30, -77, 100, -22},
    {"Saturated", 187, 4.00, 0.99, 0.00, 30, 0, 0, 50, 60},
    {"ScottS", 176, 0.46, 0.75, 0.30, 300, -100, -100, 22, 26},
    {"Surrealistic High Contrast", 126, 4.00, 1.62, -0.15, 270, -40, -40, 100, 0},
    {"Surrealistic Low Contrast", 300, 4.00, 3.00, -1.00, -81, 40, 40, 35, 55},
    {"Surrealistic", 50, 3.00, 0.30, 0.00, 80, 0, 0, 20, 20},
};

constexpr int kPresetCount = int(std::size(kPresets));

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

HdrToningDialog::HdrToningDialog(PictureView* view, const QRect& visible, QWidget* parent)
    : QDialog(parent)
    , view_(view)
    , visible_(visible)
{
    setObjectName(QStringLiteral("hdrToningDialog"));
    setWindowTitle(QStringLiteral("HDR Toning"));
    setFixedWidth(480);

    auto* outer = new QHBoxLayout(this);
    auto* left = new QVBoxLayout;

    preset_ = new QComboBox;
    preset_->setObjectName(QStringLiteral("hdrPreset"));
    for (const Preset& preset : kPresets) {
        preset_->addItem(QString::fromUtf8(preset.name));
    }
    preset_->addItem(QStringLiteral("Custom"));
    preset_->setCurrentIndex(0);
    preset_->setMinimumWidth(200);
    left->addLayout(labeledRow(QStringLiteral("Preset:"), preset_));

    auto* method = new QComboBox;
    method->addItem(QStringLiteral("Local Adaptation"));
    method->setEnabled(false);
    left->addLayout(labeledRow(QStringLiteral("Method:"), method));

    // An int slider + spin row; `spin` reports the value and drives `edited`.
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
        connect(spin, qOverload<int>(&QSpinBox::valueChanged), this, [this] { edited(); });
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
        connect(spin, qOverload<double>(&QDoubleSpinBox::valueChanged), this,
                [this] { edited(); });
        return row;
    };

    auto* edge = new QGroupBox(QStringLiteral("Edge Glow"));
    auto* edgeLayout = new QVBoxLayout(edge);
    edgeLayout->addLayout(makeIntRow(QStringLiteral("Radius:"), QStringLiteral("hdrRadius"), 1, 500,
                                     187, radius_));
    edgeLayout->addLayout(makeDoubleRow(QStringLiteral("Strength:"), QStringLiteral("hdrStrength"),
                                        0.01, 4.0, 4.0, 0.01, strength_));
    left->addWidget(edge);

    auto* tone = new QGroupBox(QStringLiteral("Tone and Detail"));
    auto* toneLayout = new QVBoxLayout(tone);
    toneLayout->addLayout(makeDoubleRow(QStringLiteral("Gamma:"), QStringLiteral("hdrGamma"), 0.01,
                                        9.99, 0.99, 0.01, gamma_));
    toneLayout->addLayout(makeDoubleRow(QStringLiteral("Exposure:"), QStringLiteral("hdrExposure"),
                                        -5.0, 5.0, 0.0, 0.01, exposure_));
    toneLayout->addLayout(makeIntRow(QStringLiteral("Detail:"), QStringLiteral("hdrDetail"), -100,
                                     300, 30, detail_));
    left->addWidget(tone);

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
    left->addWidget(advanced);
    left->addStretch();
    outer->addLayout(left, 1);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &HdrToningDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &HdrToningDialog::reject);
    preview_ = new QCheckBox(QStringLiteral("Preview"));
    preview_->setObjectName(QStringLiteral("hdrPreview"));
    preview_->setChecked(true);
    auto* right = new QVBoxLayout;
    right->addWidget(buttons);
    right->addWidget(preview_);
    right->addStretch();
    outer->addLayout(right);

    settle_ = new QTimer(this);
    settle_->setSingleShot(true);
    settle_->setInterval(40);
    connect(settle_, &QTimer::timeout, this, &HdrToningDialog::preview);
    connect(preset_, qOverload<int>(&QComboBox::currentIndexChanged), this,
            &HdrToningDialog::loadPreset);
    connect(preview_, &QCheckBox::toggled, this, [this](bool on) {
        if (on) {
            preview();
        } else {
            cancelPreview();
        }
    });
    preview();
}

HdrToningDialog::~HdrToningDialog()
{
    cancelPreview();
}

int HdrToningDialog::presetCount() const
{
    return preset_ ? preset_->count() : 0;
}

QWidget* HdrToningDialog::controlForTest(const QString& name) const
{
    return findChild<QWidget*>(name);
}

void HdrToningDialog::loadPreset(int index)
{
    if (index < 0 || index >= kPresetCount) {
        return;
    }
    loading_ = true;
    const Preset& p = kPresets[index];
    radius_->setValue(p.radius);
    strength_->setValue(p.strength);
    gamma_->setValue(p.gamma);
    exposure_->setValue(p.exposure);
    detail_->setValue(p.detail);
    shadow_->setValue(p.shadow);
    highlight_->setValue(p.highlight);
    vibrance_->setValue(p.vibrance);
    saturation_->setValue(p.saturation);
    loading_ = false;
    if (preview_->isChecked()) {
        preview();
    }
}

void HdrToningDialog::edited()
{
    if (loading_) {
        return;
    }
    if (preset_->currentIndex() < kPresetCount) {
        const QSignalBlocker block(preset_);
        preset_->setCurrentIndex(kPresetCount);
    }
    if (preview_->isChecked()) {
        settle_->start();
    }
}

void HdrToningDialog::preview()
{
    if (!view_) {
        return;
    }
    const bool ok =
        hdr_toning_preview(*view_, radius_->value(), strength_->value(), gamma_->value(),
                           exposure_->value(), detail_->value(), shadow_->value(),
                           highlight_->value(), vibrance_->value(), saturation_->value(),
                           visible_.x(), visible_.y(), visible_.width(), visible_.height());
    previewing_ = ok || previewing_;
}

void HdrToningDialog::cancelPreview()
{
    if (view_ && previewing_) {
        filter_preview_cancel(*view_);
    }
    previewing_ = false;
}

void HdrToningDialog::accept()
{
    settle_->stop();
    if (view_
        && hdr_toning_apply(*view_, radius_->value(), strength_->value(), gamma_->value(),
                            exposure_->value(), detail_->value(), shadow_->value(),
                            highlight_->value(), vibrance_->value(), saturation_->value(),
                            QStringLiteral("HDR Toning"))) {
        previewing_ = false;
        QDialog::accept();
        return;
    }
    // A refused apply must not report success: reject so `get()` is false and
    // the caller reports the refusal instead of refreshing as if it landed.
    cancelPreview();
    QDialog::reject();
}

void HdrToningDialog::reject()
{
    settle_->stop();
    cancelPreview();
    QDialog::reject();
}

bool HdrToningDialog::get(QWidget* parent, PictureView* view, const QRect& visible)
{
    if (!view) {
        return false;
    }
    HdrToningDialog dialog(view, visible, parent);
    return runDialog(dialog, parent) == QDialog::Accepted;
}

} // namespace pictura

#include "moc_hdr_toning_dialog.cpp"
