#include "replace_color_dialog.h"

#include "color_picker_dialog.h"
#include "icons.h"
#include "image_view.h"
#include "panels/color_swatch_widget.h"
#include "panels/jump_slider.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_replace_color.cxxqt.h"

#include <QtWidgets/QButtonGroup>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QSlider>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include <QtCore/QSignalBlocker>

#include <cmath>

namespace pictura {

namespace {

// Longest side of the mask thumbnail, matching CS6's proportions.
constexpr int kMaskBox = 200;

// A flat swatch showing a colour, framed so white reads against the dialog.
ColorSwatchWidget* makeSwatch()
{
    auto* swatch = new ColorSwatchWidget;
    swatch->setFixedSize(34, 26);
    return swatch;
}

void setSwatchColor(ColorSwatchWidget* swatch, const QColor& color)
{
    if (!swatch) {
        return;
    }
    swatch->setColor(color);
}

} // namespace

ReplaceColorDialog::ReplaceColorDialog(PictureView* view, ToolController* tools,
                                       ImageView* canvas, QWidget* parent)
    : QDialog(parent)
    , view_(view)
    , tools_(tools)
    , canvas_(canvas)
{
    setWindowTitle(QStringLiteral("Replace Color"));
    setObjectName(QStringLiteral("replaceColorDialog"));
    buildUi();
    if (tools_) {
        samples_.append({QPoint(-1, -1), tools_->foreground()});
    }
    refreshSampler();
    refreshMask();
    refreshSwatches();
}

ReplaceColorDialog::~ReplaceColorDialog()
{
    cancelPreview();
    if (tools_) {
        tools_->setCanvasSampler({});
    }
}

void ReplaceColorDialog::buildUi()
{
    auto* outer = new QHBoxLayout(this);
    auto* left = new QVBoxLayout;

    // --- Eyedropper mode and the sampled colour ---------------------------
    auto* topRow = new QHBoxLayout;
    auto* group = new QButtonGroup(this);
    group->setExclusive(true);
    const auto makeDropper = [&](const QString& name, const QString& tip) {
        auto* button = new QToolButton(this);
        button->setObjectName(name);
        button->setCheckable(true);
        button->setIcon(icon(QStringLiteral("tool.eyedropper")));
        button->setToolTip(tip);
        group->addButton(button);
        topRow->addWidget(button);
        return button;
    };
    sampleButton_ = makeDropper(QStringLiteral("replaceColorSample"),
                                QStringLiteral("Sample the color to replace"));
    addButton_ = makeDropper(QStringLiteral("replaceColorAdd"),
                             QStringLiteral("Add to sample"));
    subtractButton_ = makeDropper(QStringLiteral("replaceColorSubtract"),
                                  QStringLiteral("Subtract from sample"));
    sampleButton_->setChecked(true);
    topRow->addStretch();
    topRow->addWidget(new QLabel(QStringLiteral("Color:"), this));
    colorSwatch_ = makeSwatch();
    topRow->addWidget(colorSwatch_);
    left->addLayout(topRow);

    localized_ = new QCheckBox(QStringLiteral("Localized Color Clusters"), this);
    localized_->setObjectName(QStringLiteral("replaceColorLocalized"));
    left->addWidget(localized_);

    // --- Fuzziness ---------------------------------------------------------
    auto* fuzzRow = new QHBoxLayout;
    fuzzRow->addWidget(new QLabel(QStringLiteral("Fuzziness:"), this));
    fuzzinessSlider_ = new JumpSlider(Qt::Horizontal, this);
    fuzzinessSlider_->setRange(0, 200);
    fuzzinessSlider_->setValue(40);
    fuzzRow->addWidget(fuzzinessSlider_, 1);
    fuzziness_ = new QSpinBox(this);
    fuzziness_->setObjectName(QStringLiteral("replaceColorFuzziness"));
    fuzziness_->setRange(0, 200);
    fuzziness_->setValue(40);
    fuzziness_->setFixedWidth(60);
    fuzzRow->addWidget(fuzziness_);
    left->addLayout(fuzzRow);

    // --- Mask thumbnail ----------------------------------------------------
    maskLabel_ = new QLabel(this);
    maskLabel_->setObjectName(QStringLiteral("replaceColorMaskPreview"));
    maskLabel_->setFixedSize(kMaskBox, kMaskBox * 3 / 5);
    maskLabel_->setAlignment(Qt::AlignCenter);
    left->addWidget(maskLabel_, 0, Qt::AlignHCenter);

    auto* modeRow = new QHBoxLayout;
    modeRow->addStretch();
    selectionButton_ = new QRadioButton(QStringLiteral("Selection"), this);
    selectionButton_->setObjectName(QStringLiteral("replaceColorSelection"));
    selectionButton_->setChecked(true);
    imageButton_ = new QRadioButton(QStringLiteral("Image"), this);
    imageButton_->setObjectName(QStringLiteral("replaceColorImage"));
    modeRow->addWidget(selectionButton_);
    modeRow->addWidget(imageButton_);
    modeRow->addStretch();
    left->addLayout(modeRow);

    // --- Replacement -------------------------------------------------------
    auto* grid = new QGridLayout;
    const auto makeRow = [&](int row, const QString& label, int min, int max, QSlider*& slider,
                             QSpinBox*& spin, const QString& name) {
        grid->addWidget(new QLabel(label, this), row, 0);
        slider = new JumpSlider(Qt::Horizontal, this);
        slider->setRange(min, max);
        grid->addWidget(slider, row, 1);
        spin = new QSpinBox(this);
        spin->setObjectName(name);
        spin->setRange(min, max);
        spin->setFixedWidth(60);
        grid->addWidget(spin, row, 2);
        connect(slider, &QSlider::valueChanged, spin, &QSpinBox::setValue);
        connect(spin, QOverload<int>::of(&QSpinBox::valueChanged), slider, &QSlider::setValue);
    };
    makeRow(0, QStringLiteral("Hue:"), -180, 180, hueSlider_, hueSpin_,
            QStringLiteral("replaceColorHue"));
    makeRow(1, QStringLiteral("Saturation:"), -100, 100, saturationSlider_, saturationSpin_,
            QStringLiteral("replaceColorSaturation"));
    makeRow(2, QStringLiteral("Lightness:"), -100, 100, lightnessSlider_, lightnessSpin_,
            QStringLiteral("replaceColorLightness"));

    auto* resultCol = new QVBoxLayout;
    resultSwatch_ = new ColorSwatchWidget(this);
    resultSwatch_->setObjectName(QStringLiteral("replaceColorResult"));
    resultSwatch_->setFixedSize(34, 26);
    resultSwatch_->setToolTip(QStringLiteral("Click to set the result color"));
    resultCol->addWidget(resultSwatch_);
    resultCol->addWidget(new QLabel(QStringLiteral("Result"), this));
    grid->addLayout(resultCol, 0, 3, 3, 1, Qt::AlignCenter);
    left->addLayout(grid);
    left->addStretch();
    outer->addLayout(left, 1);

    // --- Buttons ----------------------------------------------------------
    auto* btnCol = new QVBoxLayout;
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &ReplaceColorDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &ReplaceColorDialog::reject);
    btnCol->addWidget(buttons);
    preview_ = new QCheckBox(QStringLiteral("Preview"), this);
    preview_->setObjectName(QStringLiteral("replaceColorPreview"));
    preview_->setChecked(true);
    btnCol->addWidget(preview_);
    btnCol->addStretch();
    outer->addLayout(btnCol);

    connect(sampleButton_, &QToolButton::clicked, this, [this] { mode_ = PickMode::Replace; });
    connect(addButton_, &QToolButton::clicked, this, [this] { mode_ = PickMode::Add; });
    connect(subtractButton_, &QToolButton::clicked, this, [this] { mode_ = PickMode::Subtract; });
    connect(fuzzinessSlider_, &QSlider::valueChanged, fuzziness_, &QSpinBox::setValue);
    connect(fuzziness_, QOverload<int>::of(&QSpinBox::valueChanged), fuzzinessSlider_,
            &QSlider::setValue);
    connect(fuzziness_, QOverload<int>::of(&QSpinBox::valueChanged), this,
            [this] { applyChange(true); });
    connect(localized_, &QCheckBox::toggled, this, [this] { applyChange(true); });
    // A slider move changes only the shift, not the mask.
    connect(hueSlider_, &QSlider::valueChanged, this, [this] { applyChange(false); });
    connect(saturationSlider_, &QSlider::valueChanged, this, [this] { applyChange(false); });
    connect(lightnessSlider_, &QSlider::valueChanged, this, [this] { applyChange(false); });
    connect(imageButton_, &QRadioButton::toggled, this, [this] { refreshMask(); });
    connect(resultSwatch_, &QPushButton::clicked, this, [this] {
        if (samples_.isEmpty()) {
            return;
        }
        const QColor picked =
            ColorPickerDialog::getColor(resultForTest(), this, QStringLiteral("Result Color"));
        if (picked.isValid()) {
            pickResult(picked);
        }
    });
    connect(preview_, &QCheckBox::toggled, this, [this](bool on) {
        if (on) {
            applyPreview();
        } else {
            cancelPreview();
        }
    });
}

QString ReplaceColorDialog::samplesString() const
{
    QStringList parts;
    parts.reserve(samples_.size());
    for (const Sample& s : samples_) {
        parts << QStringLiteral("%1,%2,%3,%4,%5")
                     .arg(s.pos.x())
                     .arg(s.pos.y())
                     .arg(s.color.red())
                     .arg(s.color.green())
                     .arg(s.color.blue());
    }
    return parts.join(QLatin1Char(';'));
}

void ReplaceColorDialog::sampleAt(const QPointF& imagePos)
{
    if (!view_) {
        return;
    }
    const QPoint p = imagePos.toPoint();
    if (p.x() < 0 || p.y() < 0 || p.x() >= view_->document_width()
        || p.y() >= view_->document_height()) {
        return;
    }
    const QColor color = QColor::fromRgb(view_->sample_argb(p.x(), p.y()));
    switch (mode_) {
    case PickMode::Replace:
        samples_.clear();
        samples_.append({p, color});
        break;
    case PickMode::Add:
        samples_.append({p, color});
        break;
    case PickMode::Subtract: {
        // Drop the samples this click falls within, so clicking a colour an
        // earlier sample pulled in takes it back out again.
        const int tolerance = fuzziness_->value();
        for (int i = samples_.size() - 1; i >= 0; --i) {
            const QColor& c = samples_[i].color;
            const int distance =
                qMax(qMax(qAbs(c.red() - color.red()), qAbs(c.green() - color.green())),
                     qAbs(c.blue() - color.blue()));
            if (distance <= tolerance) {
                samples_.removeAt(i);
            }
        }
        break;
    }
    }
    applyChange(true);
}

void ReplaceColorDialog::applyChange(bool maskDirty)
{
    if (maskDirty) {
        refreshMask();
    }
    refreshSwatches();
    if (preview_->isChecked()) {
        applyPreview();
    }
}

void ReplaceColorDialog::applyPreview()
{
    if (!view_) {
        return;
    }
    const bool neutral = hueSpin_->value() == 0 && saturationSpin_->value() == 0
        && lightnessSpin_->value() == 0;
    // The visible section moves as the user pans or zooms, so read it live
    // rather than the rect captured when the dialog opened.
    const QRect visible =
        canvas_ ? canvas_->visibleDocumentRect().toAlignedRect() : QRect();
    if (samples_.isEmpty() || neutral
        || !image_replace_color_preview(*view_, samplesString(), fuzziness_->value(),
                                        localized_->isChecked(), hueSpin_->value(),
                                        saturationSpin_->value(), lightnessSpin_->value(),
                                        visible.x(), visible.y(), visible.width(),
                                        visible.height())) {
        cancelPreview();
        return;
    }
    previewing_ = true;
}

void ReplaceColorDialog::cancelPreview()
{
    if (view_ && previewing_) {
        filter_preview_cancel(*view_);
    }
    previewing_ = false;
}

void ReplaceColorDialog::refreshSampler()
{
    if (tools_) {
        tools_->setCanvasSampler([this](const QPointF& p) { sampleAt(p); });
    }
}

void ReplaceColorDialog::refreshMask()
{
    if (!view_ || !maskLabel_) {
        return;
    }
    QImage image;
    if (imageButton_->isChecked()) {
        image = view_->image();
    } else {
        image = image_replace_color_mask(*view_, samplesString(), fuzziness_->value(),
                                         localized_->isChecked(), kMaskBox);
    }
    if (!image.isNull()) {
        mask_ = image;
        maskLabel_->setPixmap(QPixmap::fromImage(image).scaled(
            maskLabel_->size(), Qt::KeepAspectRatio, Qt::SmoothTransformation));
    } else {
        mask_ = QImage();
        maskLabel_->clear();
    }
}

// "Color" is the most recently sampled colour; "Result" is that colour put
// through the same HSL shift the image will get.
QColor ReplaceColorDialog::resultForTest() const
{
    if (samples_.isEmpty()) {
        return {};
    }
    return QColor::fromRgb(image_replace_color_result(
        samples_.back().color.rgb() & 0xffffffu, hueSpin_->value(), saturationSpin_->value(),
        lightnessSpin_->value()));
}

void ReplaceColorDialog::refreshSwatches()
{
    setSwatchColor(colorSwatch_, samples_.isEmpty() ? QColor() : samples_.back().color);
    resultSwatch_->setColor(resultForTest());
}

void ReplaceColorDialog::pickResult(const QColor& result)
{
    if (samples_.isEmpty() || !result.isValid()) {
        return;
    }
    const ::rust::Vec<std::int32_t> values =
        image_replace_color_shift_for(samples_.back().color.rgb() & 0xffffffu, result.rgb() & 0xffffffu);
    QSlider* sliders[] = {hueSlider_, saturationSlider_, lightnessSlider_};
    QSpinBox* spins[] = {hueSpin_, saturationSpin_, lightnessSpin_};
    for (int i = 0; i < 3; ++i) {
        const QSignalBlocker blockSlider(sliders[i]);
        const QSignalBlocker blockSpin(spins[i]);
        sliders[i]->setValue(values[i]);
        spins[i]->setValue(values[i]);
    }
    applyChange(false);
}

void ReplaceColorDialog::accept()
{
    // The commit re-applies to the whole layer from the pre-preview pixels.
    if (view_ && !samples_.isEmpty()
        && image_replace_color_apply(*view_, samplesString(), fuzziness_->value(),
                                     localized_->isChecked(), hueSpin_->value(),
                                     saturationSpin_->value(), lightnessSpin_->value())) {
        previewing_ = false;
    } else {
        cancelPreview();
    }
    QDialog::accept();
}

void ReplaceColorDialog::reject()
{
    cancelPreview();
    QDialog::reject();
}

void ReplaceColorDialog::hideEvent(QHideEvent* event)
{
    // The canvas must not keep sampling once this window is gone.
    if (tools_) {
        tools_->setCanvasSampler({});
    }
    QDialog::hideEvent(event);
}

} // namespace pictura

#include "moc_replace_color_dialog.cpp"
