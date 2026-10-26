#include "jpeg_options_dialog.h"

#include "pictura_app/src/cxxqt_object/save_format.cxxqt.h"

#include <QtCore/QTimer>
#include <QtGui/QImage>
#include <QtGui/QPixmap>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QColorDialog>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QSlider>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

// The shared numeric-field width the adjustment dialogs use.
constexpr int kFieldWidth = 76;

// The CS6 JPEG quality presets and the 0-12 value each sets.
struct QualityPreset {
    const char* name;
    int value;
};
constexpr QualityPreset kQualityPresets[] = {
    {"Low", 3}, {"Medium", 5}, {"High", 8}, {"Maximum", 10},
};

QString formatSize(qint64 bytes)
{
    if (bytes < 1024) {
        return QStringLiteral("%1 B").arg(bytes);
    }
    if (bytes < 1024 * 1024) {
        return QStringLiteral("%1K").arg(bytes / 1024.0, 0, 'f', 1);
    }
    return QStringLiteral("%1M").arg(bytes / (1024.0 * 1024.0), 0, 'f', 1);
}

} // namespace

JpegOptionsDialog::JpegOptionsDialog(const SaveFormatOptions& initial, PictureView* view,
                                     const QColor& foreground, const QColor& background,
                                     QWidget* parent)
    : QDialog(parent)
    , view_(view)
    , foreground_(foreground)
    , background_(background)
{
    setObjectName(QStringLiteral("jpegOptionsDialog"));
    setWindowTitle(tr("JPEG Options"));

    auto* root = new QVBoxLayout(this);

    auto* matteBox = new QGroupBox(tr("Matte"), this);
    auto* matteForm = new QFormLayout(matteBox);
    matte_ = new QComboBox(matteBox);
    matte_->setObjectName(QStringLiteral("jpegOptionsMatte"));
    matte_->addItems({tr("None"), tr("Foreground"), tr("Background"), tr("White"),
                      tr("Black"), tr("Other…")});
    matteForm->addRow(matte_);
    // ponytail: the matte is not offered yet; JPEG always flattens transparency
    // onto opaque white.
    matteBox->setEnabled(false);
    matteBox->setToolTip(tr("The JPEG matte is not available yet."));
    matte_->setToolTip(matteBox->toolTip());
    root->addWidget(matteBox);

    auto* imageBox = new QGroupBox(tr("Image Options"), this);
    auto* imageLayout = new QVBoxLayout(imageBox);
    auto* qualityRow = new QHBoxLayout();
    qualityRow->addWidget(new QLabel(tr("Quality:"), imageBox));
    quality_ = new QSpinBox(imageBox);
    quality_->setObjectName(QStringLiteral("jpegOptionsQuality"));
    quality_->setRange(0, 12);
    quality_->setValue(initial.jpegQuality);
    quality_->setFixedWidth(kFieldWidth);
    qualityRow->addWidget(quality_);
    preset_ = new QComboBox(imageBox);
    preset_->setObjectName(QStringLiteral("jpegOptionsQualityPreset"));
    for (const QualityPreset& preset : kQualityPresets) {
        preset_->addItem(tr(preset.name), preset.value);
    }
    preset_->setMinimumWidth(120);
    qualityRow->addWidget(preset_);
    qualityRow->addStretch(1);
    imageLayout->addLayout(qualityRow);
    slider_ = new QSlider(Qt::Horizontal, imageBox);
    slider_->setObjectName(QStringLiteral("jpegOptionsQualitySlider"));
    slider_->setRange(0, 12);
    slider_->setValue(initial.jpegQuality);
    imageLayout->addWidget(slider_);
    root->addWidget(imageBox);

    auto* formatBox = new QGroupBox(tr("Format Options"), this);
    auto* formatLayout = new QVBoxLayout(formatBox);
    baseline_ = new QRadioButton(tr("Baseline (\"Standard\")"), formatBox);
    baseline_->setObjectName(QStringLiteral("jpegOptionsBaseline"));
    baselineOptimized_ = new QRadioButton(tr("Baseline Optimized"), formatBox);
    baselineOptimized_->setObjectName(QStringLiteral("jpegOptionsBaselineOptimized"));
    progressive_ = new QRadioButton(tr("Progressive"), formatBox);
    progressive_->setObjectName(QStringLiteral("jpegOptionsProgressive"));
    scans_ = new QSpinBox(formatBox);
    scans_->setObjectName(QStringLiteral("jpegOptionsScans"));
    scans_->setRange(1, 100);
    scans_->setValue(initial.jpegScans);
    scans_->setFixedWidth(kFieldWidth);
    scans_->setEnabled(false);
    scans_->setToolTip(tr("The progressive scan count is written by the image "
                          "encoder, not chosen here."));
    auto* scansRow = new QHBoxLayout();
    scansRow->addWidget(new QLabel(tr("Scans:"), formatBox));
    scansRow->addWidget(scans_);
    scansRow->addStretch(1);
    formatLayout->addWidget(baseline_);
    formatLayout->addWidget(baselineOptimized_);
    formatLayout->addWidget(progressive_);
    formatLayout->addLayout(scansRow);
    root->addWidget(formatBox);

    previewBox_ = new QCheckBox(tr("Preview"), this);
    previewBox_->setObjectName(QStringLiteral("jpegOptionsPreview"));
    previewBox_->setChecked(initial.jpegPreview);
    size_ = new QLabel(this);
    size_->setObjectName(QStringLiteral("jpegOptionsSize"));
    previewImage_ = new QLabel(this);
    previewImage_->setObjectName(QStringLiteral("jpegOptionsPreviewImage"));
    previewImage_->setMinimumSize(160, 120);
    previewImage_->setAlignment(Qt::AlignCenter);
    root->addWidget(previewBox_);
    root->addWidget(size_);
    root->addWidget(previewImage_, 0, Qt::AlignHCenter);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &JpegOptionsDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &JpegOptionsDialog::reject);
    root->addWidget(buttons);

    switch (initial.jpegFormat) {
    case 1:
        baselineOptimized_->setChecked(true);
        break;
    case 2:
        progressive_->setChecked(true);
        break;
    default:
        baseline_->setChecked(true);
        break;
    }
    for (int i = 0; i < int(sizeof(kQualityPresets) / sizeof(kQualityPresets[0])); ++i) {
        if (kQualityPresets[i].value == initial.jpegQuality) {
            preset_->setCurrentIndex(i);
            break;
        }
    }
    if (initial.jpegMatte < 0) {
        matte_->setCurrentIndex(0);
    } else {
        const QColor matte(QRgb(initial.jpegMatte));
        const int index = matte == foreground_ ? 1
                          : matte == background_ ? 2
                          : matte == QColor(Qt::white) ? 3
                          : matte == QColor(Qt::black) ? 4
                                                       : 5;
        otherMatte_ = matte;
        matte_->setCurrentIndex(index);
    }

    debounce_ = new QTimer(this);
    debounce_->setSingleShot(true);
    debounce_->setInterval(150);
    connect(debounce_, &QTimer::timeout, this, &JpegOptionsDialog::refreshEstimate);

    connect(quality_, qOverload<int>(&QSpinBox::valueChanged), this,
            [this](int value) { syncQualityControls(value); });
    connect(slider_, &QSlider::valueChanged, this,
            [this](int value) { syncQualityControls(value); });
    connect(preset_, qOverload<int>(&QComboBox::currentIndexChanged), this,
            [this](int index) { applyPreset(index); });
    connect(matte_, qOverload<int>(&QComboBox::currentIndexChanged), this, [this](int index) {
        if (index == 5) {
            const QColor chosen = QColorDialog::getColor(otherMatte_, this, tr("Matte Color"));
            if (chosen.isValid()) {
                otherMatte_ = chosen;
            }
        }
        scheduleEstimate();
    });
    for (QRadioButton* button : {baseline_, baselineOptimized_, progressive_}) {
        connect(button, &QRadioButton::toggled, this, [this]() { scheduleEstimate(); });
    }
    connect(previewBox_, &QCheckBox::toggled, this, [this](bool on) {
        previewImage_->setVisible(on);
        refreshEstimate();
    });
    previewImage_->setVisible(previewBox_->isChecked());
    setMinimumWidth(460);
    refreshEstimate();
}

void JpegOptionsDialog::syncQualityControls(int value)
{
    if (quality_->value() != value) {
        quality_->setValue(value);
    }
    if (slider_->value() != value) {
        slider_->setValue(value);
    }
    for (int i = 0; i < preset_->count(); ++i) {
        if (preset_->itemData(i).toInt() == value) {
            preset_->setCurrentIndex(i);
            break;
        }
    }
    scheduleEstimate();
}

void JpegOptionsDialog::applyPreset(int index)
{
    if (index < 0) {
        return;
    }
    syncQualityControls(preset_->itemData(index).toInt());
}

int JpegOptionsDialog::matteColour() const
{
    switch (matte_->currentIndex()) {
    case 1:
        return int(foreground_.rgb() & 0xFFFFFF);
    case 2:
        return int(background_.rgb() & 0xFFFFFF);
    case 3:
        return 0xFFFFFF;
    case 4:
        return 0x000000;
    case 5:
        return int(otherMatte_.rgb() & 0xFFFFFF);
    default:
        return -1;
    }
}

int JpegOptionsDialog::quality() const
{
    return quality_->value();
}

int JpegOptionsDialog::format() const
{
    if (progressive_->isChecked()) {
        return 2;
    }
    return baselineOptimized_->isChecked() ? 1 : 0;
}

int JpegOptionsDialog::scans() const
{
    return scans_->value();
}

int JpegOptionsDialog::matte() const
{
    return matteColour();
}

bool JpegOptionsDialog::preview() const
{
    return previewBox_->isChecked();
}

QString JpegOptionsDialog::sizeTextForTest() const
{
    return size_->text();
}

bool JpegOptionsDialog::previewPaneVisibleForTest() const
{
    return previewImage_->isVisibleTo(const_cast<JpegOptionsDialog*>(this));
}

void JpegOptionsDialog::refreshEstimateForTest()
{
    refreshEstimate();
}

void JpegOptionsDialog::scheduleEstimate()
{
    debounce_->start();
}

void JpegOptionsDialog::refreshEstimate()
{
    if (!view_) {
        return;
    }
    const ::rust::Vec<uint8_t> bytes = raster_preview_bytes(
        *view_, QStringLiteral("JPG"), jpegQualityPercent(quality()), -1,
        baselineOptimized_->isChecked() || progressive_->isChecked(),
        progressive_->isChecked(), matteColour());
    const qint64 count = qint64(bytes.size());
    lastSize_ = count;
    size_->setText(formatSize(count));
    if (!previewBox_->isChecked() || count == 0) {
        previewImage_->clear();
        return;
    }
    const QByteArray data(reinterpret_cast<const char*>(bytes.data()), int(bytes.size()));
    const QImage image = QImage::fromData(data, "JPG");
    if (image.isNull()) {
        previewImage_->clear();
        return;
    }
    previewImage_->setPixmap(QPixmap::fromImage(
        image.scaled(200, 200, Qt::KeepAspectRatio, Qt::SmoothTransformation)));
}

} // namespace pictura

#include "moc_jpeg_options_dialog.cpp"
