#include "color_range_dialog.h"

#include "icons.h"
#include "panels/jump_slider.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/color_range.cxxqt.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QSlider>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>

namespace pictura {

namespace {

// Longest side of the preview mask.
constexpr int kPreviewSize = 220;

} // namespace

ColorRangeDialog::ColorRangeDialog(PictureView* view, ToolController* tools, QWidget* parent)
    : QDialog(parent)
    , view_(view)
    , tools_(tools)
{
    setWindowTitle(QStringLiteral("Color Range"));
    setObjectName(QStringLiteral("colorRangeDialog"));
    auto* root = new QVBoxLayout(this);
    auto* form = new QGridLayout();

    form->addWidget(new QLabel(QStringLiteral("Select:"), this), 0, 0);
    select_ = new QComboBox(this);
    select_->setObjectName(QStringLiteral("colorRangeSelect"));
    const QStringList names = {
        QStringLiteral("Sampled Colors"), QStringLiteral("Reds"),     QStringLiteral("Yellows"),
        QStringLiteral("Greens"),         QStringLiteral("Cyans"),    QStringLiteral("Blues"),
        QStringLiteral("Magentas"),       QStringLiteral("Highlights"), QStringLiteral("Midtones"),
        QStringLiteral("Shadows")};
    for (int i = 0; i < names.size(); ++i) {
        select_->addItem(names.at(i), i);
        // CS6 rules off after Sampled Colors and between colours and tones.
        if (i == 0 || i == 6) {
            select_->insertSeparator(select_->count());
        }
    }
    form->addWidget(select_, 0, 1, 1, 2);

    form->addWidget(new QLabel(QStringLiteral("Fuzziness:"), this), 1, 0);
    fuzziness_ = new QSpinBox(this);
    fuzziness_->setObjectName(QStringLiteral("colorRangeFuzziness"));
    fuzziness_->setRange(0, 200);
    fuzziness_->setValue(40);
    form->addWidget(fuzziness_, 1, 1);
    fuzzinessSlider_ = new JumpSlider(Qt::Horizontal, this);
    fuzzinessSlider_->setRange(0, 200);
    fuzzinessSlider_->setValue(40);
    form->addWidget(fuzzinessSlider_, 2, 0, 1, 3);

    form->addWidget(new QLabel(QStringLiteral("Color:"), this), 3, 0);
    swatch_ = new QLabel(this);
    swatch_->setFixedSize(44, 18);
    swatch_->setFrameShape(QFrame::Box);
    swatch_->setAutoFillBackground(true);
    form->addWidget(swatch_, 3, 1);
    eyedropper_ = new QToolButton(this);
    eyedropper_->setObjectName(QStringLiteral("colorRangeEyedropper"));
    eyedropper_->setCheckable(true);
    eyedropper_->setChecked(true);
    eyedropper_->setIcon(icon(QStringLiteral("tool.eyedropper")));
    eyedropper_->setToolTip(QStringLiteral("Click the image to sample the color to match"));
    form->addWidget(eyedropper_, 3, 2);
    root->addLayout(form);

    invert_ = new QCheckBox(QStringLiteral("Invert"), this);
    invert_->setObjectName(QStringLiteral("colorRangeInvert"));
    root->addWidget(invert_);

    previewLabel_ = new QLabel(this);
    previewLabel_->setAlignment(Qt::AlignCenter);
    previewLabel_->setMinimumSize(kPreviewSize, kPreviewSize / 2);
    previewLabel_->setFrameShape(QFrame::Box);
    root->addWidget(previewLabel_, 1);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    root->addWidget(buttons);

    // Start on the foreground colour, usually the one just sampled (as CS6).
    if (tools_) {
        sampled_ = tools_->foreground();
    }
    connect(select_, &QComboBox::currentIndexChanged, this, [this]() {
        refreshEnabled();
        refreshPreview();
    });
    connect(fuzzinessSlider_, &QSlider::valueChanged, fuzziness_, &QSpinBox::setValue);
    connect(fuzziness_, &QSpinBox::valueChanged, fuzzinessSlider_, &QSlider::setValue);
    connect(fuzziness_, &QSpinBox::valueChanged, this, [this]() { refreshPreview(); });
    connect(invert_, &QCheckBox::toggled, this, [this]() { refreshPreview(); });
    connect(eyedropper_, &QToolButton::toggled, this, [this]() { refreshSampler(); });
    refreshEnabled();
    refreshPreview();
}

ColorRangeDialog::~ColorRangeDialog()
{
    if (tools_) {
        tools_->setCanvasSampler({});
    }
}

int ColorRangeDialog::range() const
{
    return select_->currentData().toInt();
}

int ColorRangeDialog::fuzziness() const
{
    return fuzziness_->value();
}

bool ColorRangeDialog::inverted() const
{
    return invert_->isChecked();
}

void ColorRangeDialog::sampleAt(const QPointF& imagePos)
{
    if (!view_ || range() != 0) {
        return;
    }
    const QPoint p = imagePos.toPoint();
    if (p.x() < 0 || p.y() < 0 || p.x() >= view_->document_width() || p.y() >= view_->document_height()) {
        return;
    }
    sampled_ = QColor::fromRgb(view_->sample_argb(p.x(), p.y()));
    refreshEnabled();
    refreshPreview();
}

void ColorRangeDialog::hideEvent(QHideEvent* event)
{
    // The canvas must not keep sampling once this window is gone.
    if (tools_) {
        tools_->setCanvasSampler({});
    }
    QDialog::hideEvent(event);
}

void ColorRangeDialog::refreshSampler()
{
    if (!tools_) {
        return;
    }
    if (eyedropper_->isChecked() && eyedropper_->isEnabled()) {
        tools_->setCanvasSampler([this](const QPointF& p) { sampleAt(p); });
    } else {
        tools_->setCanvasSampler({});
    }
}

void ColorRangeDialog::refreshEnabled()
{
    // A band is defined by its hue or tone: nothing to sample (CS6 greys the
    // eyedroppers out the same way).
    const bool sampled = range() == 0;
    swatch_->setEnabled(sampled);
    eyedropper_->setEnabled(sampled);
    QPalette palette = swatch_->palette();
    palette.setColor(QPalette::Window, sampled ? sampled_ : QColor(Qt::transparent));
    swatch_->setPalette(palette);
    refreshSampler();
}

void ColorRangeDialog::refreshPreview()
{
    if (!view_) {
        return;
    }
    int width = 0;
    int height = 0;
    const ::rust::Vec<uint8_t> coverage = color_range_preview(
        *view_, range(), sampled_.rgb() & 0xffffffu, fuzziness(), inverted(), kPreviewSize, width,
        height);
    if (width <= 0 || height <= 0 || int(coverage.size()) != width * height) {
        preview_ = QImage();
        previewLabel_->clear();
        return;
    }
    QImage image(width, height, QImage::Format_Grayscale8);
    for (int y = 0; y < height; ++y) {
        std::copy_n(coverage.data() + size_t(y) * size_t(width), width, image.scanLine(y));
    }
    preview_ = image;
    previewLabel_->setPixmap(QPixmap::fromImage(image));
}

} // namespace pictura

#include "moc_color_range_dialog.cpp"
