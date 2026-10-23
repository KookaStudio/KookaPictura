#include "hdr_toning_dialog.h"

#include "dialogs.h"
#include "hdr_toning_controls.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_hdr_toning.cxxqt.h"

#include <QtCore/QSignalBlocker>
#include <QtCore/QTimer>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QVBoxLayout>

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

HdrToningControls::Values presetValues(const Preset& p)
{
    return HdrToningControls::Values{
        p.radius, p.strength, p.gamma, p.exposure, p.detail,
        p.shadow, p.highlight, p.vibrance, p.saturation,
    };
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

    controls_ = new HdrToningControls(this);
    controls_->setValues(presetValues(kPresets[0]));
    connect(controls_, &HdrToningControls::edited, this, &HdrToningDialog::edited);
    left->addWidget(controls_);
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
    controls_->setValues(presetValues(kPresets[index]));
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
    const auto values = controls_->values();
    const bool ok =
        hdr_toning_preview(*view_, values.radius, values.strength, values.gamma, values.exposure,
                           values.detail, values.shadow, values.highlight, values.vibrance,
                           values.saturation, visible_.x(), visible_.y(), visible_.width(),
                           visible_.height());
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
    const auto values = controls_->values();
    if (view_
        && hdr_toning_apply(*view_, values.radius, values.strength, values.gamma, values.exposure,
                            values.detail, values.shadow, values.highlight, values.vibrance,
                            values.saturation, QStringLiteral("HDR Toning"))) {
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
