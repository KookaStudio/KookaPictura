#include "tiff_options_dialog.h"

#include <QtWidgets/QButtonGroup>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

// Qt's TIFF compression enum (0 None, 1 LZW, 5 JPEG) plus placeholder ids for
// the shown-but-unhonored RLE and ZIP choices.
constexpr int kNone = 0;
constexpr int kLzw = 1;
constexpr int kRlePlaceholder = 2;
constexpr int kZipPlaceholder = 3;
constexpr int kJpeg = 5;

void disableWithReason(QWidget* widget)
{
    widget->setEnabled(false);
    widget->setToolTip(QObject::tr("This TIFF option is not written yet."));
}

} // namespace

TiffOptionsDialog::TiffOptionsDialog(const SaveFormatOptions& initial, QWidget* parent)
    : QDialog(parent)
{
    setObjectName(QStringLiteral("tiffOptionsDialog"));
    setWindowTitle(tr("TIFF Options"));

    auto* root = new QVBoxLayout(this);
    auto* columns = new QHBoxLayout();
    root->addLayout(columns);

    auto* left = new QVBoxLayout();
    auto* right = new QVBoxLayout();
    columns->addLayout(left);
    columns->addLayout(right);

    auto* compressionBox = new QGroupBox(tr("Image Compression"), this);
    auto* compressionLayout = new QVBoxLayout(compressionBox);
    compression_ = new QButtonGroup(compressionBox);
    const struct {
        const char* name;
        int id;
        bool enabled;
    } compressions[] = {
        {"None", kNone, true},
        {"RLE", kRlePlaceholder, false},
        {"LZW", kLzw, true},
        {"ZIP", kZipPlaceholder, false},
        {"JPEG", kJpeg, true},
    };
    for (const auto& entry : compressions) {
        auto* button = new QRadioButton(tr(entry.name), compressionBox);
        button->setObjectName(QStringLiteral("tiffOptionsCompression%1")
                                  .arg(QString::fromLatin1(entry.name)));
        compression_->addButton(button, entry.id);
        if (!entry.enabled) {
            disableWithReason(button);
        }
        if (entry.id == initial.tiffCompression) {
            button->setChecked(true);
        }
        compressionLayout->addWidget(button);
    }
    if (!compression_->checkedButton()) {
        compression_->button(kLzw)->setChecked(true);
    }
    auto* qualityRow = new QFormLayout();
    jpegQuality_ = new QSpinBox(compressionBox);
    jpegQuality_->setObjectName(QStringLiteral("tiffOptionsJpegQuality"));
    jpegQuality_->setRange(0, 12);
    jpegQuality_->setValue(initial.tiffJpegQuality);
    qualityRow->addRow(tr("Quality:"), jpegQuality_);
    compressionLayout->addLayout(qualityRow);
    left->addWidget(compressionBox);

    auto* pixelBox = new QGroupBox(tr("Pixel Order"), this);
    auto* pixelLayout = new QVBoxLayout(pixelBox);
    pixelOrder_ = new QButtonGroup(pixelBox);
    for (const auto& entry :
         {std::pair<const char*, int>{"Interleaved (RGBRGB)", 0},
          std::pair<const char*, int>{"Per Channel (RRGGBB)", 1}}) {
        auto* button = new QRadioButton(tr(entry.first), pixelBox);
        button->setObjectName(QStringLiteral("tiffOptionsPixelOrder%1").arg(entry.second));
        pixelOrder_->addButton(button, entry.second);
        disableWithReason(button);
        if (entry.second == initial.tiffPixelOrder) {
            button->setChecked(true);
        }
        pixelLayout->addWidget(button);
    }
    right->addWidget(pixelBox);

    auto* byteBox = new QGroupBox(tr("Byte Order"), this);
    auto* byteLayout = new QVBoxLayout(byteBox);
    byteOrder_ = new QButtonGroup(byteBox);
    for (const auto& entry :
         {std::pair<const char*, int>{"IBM PC", 0}, std::pair<const char*, int>{"Macintosh", 1}}) {
        auto* button = new QRadioButton(tr(entry.first), byteBox);
        button->setObjectName(QStringLiteral("tiffOptionsByteOrder%1").arg(entry.second));
        byteOrder_->addButton(button, entry.second);
        disableWithReason(button);
        if (entry.second == initial.tiffByteOrder) {
            button->setChecked(true);
        }
        byteLayout->addWidget(button);
    }
    right->addWidget(byteBox);

    auto* layerBox = new QGroupBox(tr("Layer Compression"), this);
    auto* layerLayout = new QVBoxLayout(layerBox);
    layerCompression_ = new QButtonGroup(layerBox);
    for (const auto& entry :
         {std::pair<const char*, int>{"RLE (faster saves, bigger files)", 0},
          std::pair<const char*, int>{"ZIP (slower saves, smaller files)", 1},
          std::pair<const char*, int>{"Discard Layers and Save a Copy", 2}}) {
        auto* button = new QRadioButton(tr(entry.first), layerBox);
        button->setObjectName(QStringLiteral("tiffOptionsLayer%1").arg(entry.second));
        layerCompression_->addButton(button, entry.second);
        disableWithReason(button);
        if (entry.second == initial.tiffLayerCompression) {
            button->setChecked(true);
        }
        layerLayout->addWidget(button);
    }
    right->addWidget(layerBox);

    pyramid_ = new QCheckBox(tr("Save Image Pyramid"), this);
    pyramid_->setObjectName(QStringLiteral("tiffOptionsPyramid"));
    pyramid_->setChecked(initial.tiffPyramid);
    disableWithReason(pyramid_);
    left->addWidget(pyramid_);

    transparency_ = new QCheckBox(tr("Save Transparency"), this);
    transparency_->setObjectName(QStringLiteral("tiffOptionsTransparency"));
    transparency_->setChecked(initial.tiffTransparency);
    left->addWidget(transparency_);
    left->addStretch(1);
    right->addStretch(1);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &TiffOptionsDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &TiffOptionsDialog::reject);
    root->addWidget(buttons);
    setMinimumWidth(520);

    const auto syncQuality = [this]() {
        jpegQuality_->setEnabled(compression_->checkedId() == kJpeg);
    };
    connect(compression_, &QButtonGroup::idToggled, this, [syncQuality](int, bool) { syncQuality(); });
    syncQuality();
}

int TiffOptionsDialog::compression() const
{
    const int id = compression_->checkedId();
    return id == kLzw || id == kJpeg ? id : kNone;
}

int TiffOptionsDialog::jpegQuality() const
{
    return jpegQuality_->value();
}

int TiffOptionsDialog::pixelOrder() const
{
    return pixelOrder_->checkedId() == 1 ? 1 : 0;
}

int TiffOptionsDialog::byteOrder() const
{
    return byteOrder_->checkedId() == 1 ? 1 : 0;
}

int TiffOptionsDialog::layerCompression() const
{
    const int id = layerCompression_->checkedId();
    return id >= 0 ? id : 0;
}

bool TiffOptionsDialog::pyramid() const
{
    return pyramid_->isChecked();
}

bool TiffOptionsDialog::transparency() const
{
    return transparency_->isChecked();
}

} // namespace pictura

#include "moc_tiff_options_dialog.cpp"
