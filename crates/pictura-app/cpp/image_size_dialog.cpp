#include "image_size_dialog.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust/image_mode.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust/image_size.cxxqt.h"

#include <QtCore/QSignalBlocker>
#include <QtGui/QPainter>
#include <QtGui/QStandardItemModel>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>
#include <cmath>
#include <iterator>

namespace pictura {

namespace {

// Width/Height units, in CS6's order. Pixels and Percent stand apart from the
// physical units, which all convert through the resolution.
enum Unit { UnitPixels = 0, UnitInches, UnitCm, UnitMm, UnitPoints, UnitPercent };

constexpr int kPreviewBox = 220;

// One Fit To entry, kept as the physical size and resolution its label names
// so a preset cannot drift from its label.
struct FitPreset {
    const char* label;
    double width;
    double height;
    int unit;
    double dpi;
};

const FitPreset kFitPresets[] = {
    {"960 x 640 px 144 ppi", 960, 640, UnitPixels, 144},
    {"1024 x 768 px 72 ppi", 1024, 768, UnitPixels, 72},
    {"1136 x 640 px 144 ppi", 1136, 640, UnitPixels, 144},
    {"1366 x 768 px 72 ppi", 1366, 768, UnitPixels, 72},
    {"A4 210 x 297 mm 300 dpi", 210, 297, UnitMm, 300},
    {"A6 105 x 148 mm 300 dpi", 105, 148, UnitMm, 300},
    {"Legal 8.5 x 14 in 300 dpi", 8.5, 14, UnitInches, 300},
    {"Letter 8.5 x 11 in 300 dpi", 8.5, 11, UnitInches, 300},
    {"4 x 6 in 300 dpi", 4, 6, UnitInches, 300},
    {"5 x 7 in 300 dpi", 5, 7, UnitInches, 300},
    {"8 x 10 in 300 dpi", 8, 10, UnitInches, 300},
    {"11 x 14 in 300 dpi", 11, 14, UnitInches, 300},
};
// Fit To draws a separator after these preset indices.
constexpr int kFitGroupBreaks[] = {3, 7};

constexpr int kFitOriginal = -1;
constexpr int kFitAutoResolution = -2;
constexpr int kFitCustom = -3;

// CS6's Resample menu and the engine interpolator each entry uses.
struct ResampleEntry {
    const char* label;
    const char* kind;
};
const ResampleEntry kResample[] = {
    {"Automatic", "bicubic"},
    {"Preserve Details (enlargement)", "bicubic"},
    {"Bicubic Smoother (enlargement)", "bicubic"},
    {"Bicubic Sharper (reduction)", "bicubic"},
    {"Bicubic (smooth gradients)", "bicubic"},
    {"Nearest Neighbor (hard edges)", "nearest"},
    {"Bilinear", "bilinear"},
};

int presetPixels(double size, int unit, double dpi)
{
    switch (unit) {
    case UnitMm:
        return qRound(size / 25.4 * dpi);
    case UnitInches:
        return qRound(size * dpi);
    default:
        return qRound(size);
    }
}

// The chain toggle with the bracket CS6 draws joining the two rows it binds:
// linked links when constrained, pulled apart when not.
class ChainToggle : public QToolButton {
public:
    explicit ChainToggle(QWidget* parent)
        : QToolButton(parent)
    {
        setObjectName(QStringLiteral("imageSizeConstrain"));
        setCheckable(true);
        setChecked(true);
        setAutoRaise(true);
        setFocusPolicy(Qt::NoFocus);
        setFixedWidth(30);
        setToolTip(QStringLiteral("Constrain aspect ratio"));
    }

protected:
    void paintEvent(QPaintEvent*) override
    {
        QPainter painter(this);
        painter.setRenderHint(QPainter::Antialiasing);
        const bool linked = isChecked();
        QColor ink = palette().color(QPalette::WindowText);
        ink.setAlpha(linked ? 235 : 120);
        const int spine = width() - 9;
        const int cy = height() / 2;
        // Two link outlines on the spine, touching when linked.
        painter.setPen(QPen(ink, 1.4));
        painter.setBrush(Qt::NoBrush);
        const double gap = linked ? -2.0 : 2.5;
        painter.drawRoundedRect(QRectF(spine - 3, cy - 9 - gap / 2, 6, 9), 3, 3);
        painter.drawRoundedRect(QRectF(spine - 3, cy + gap / 2, 6, 9), 3, 3);
        QColor line = ink;
        line.setAlpha(linked ? 200 : 90);
        painter.setPen(QPen(line, 1.0));
        const int right = width() - 1;
        const int top = height() / 4;
        const int bottom = height() - height() / 4;
        painter.drawLine(spine, top, right, top);
        painter.drawLine(spine, bottom, right, bottom);
        painter.drawLine(spine, top, spine, cy - 11);
        painter.drawLine(spine, cy + 11, spine, bottom);
    }
};

void addUnits(QComboBox* box, bool percent)
{
    box->addItems({QStringLiteral("Pixels"), QStringLiteral("Inches"),
                   QStringLiteral("Centimeters"), QStringLiteral("Millimeters"),
                   QStringLiteral("Points")});
    if (percent) {
        box->addItem(QStringLiteral("Percent"));
    }
}

bool isPhysical(int unit)
{
    return unit == UnitInches || unit == UnitCm || unit == UnitMm || unit == UnitPoints;
}

double inchesPerUnit(int unit)
{
    switch (unit) {
    case UnitCm:
        return 1.0 / 2.54;
    case UnitMm:
        return 1.0 / 25.4;
    case UnitPoints:
        return 1.0 / 72.0;
    default:
        return 1.0;
    }
}

} // namespace

QString imageSizeSummary(double bytes)
{
    if (bytes >= 1024.0 * 1024.0) {
        return QStringLiteral("%1M").arg(bytes / (1024.0 * 1024.0), 0, 'f', 2);
    }
    return QStringLiteral("%1K").arg(bytes / 1024.0, 0, 'f', 1);
}

double bytesPerPixel(PictureView* view)
{
    if (!view) {
        return 3.0;
    }
    // The stored mode, not the RGB/Gray working one: CMYK keeps four planes and
    // Bitmap one bit.
    const QString mode(image_mode(*view));
    int channels = 3;
    if (mode == QLatin1String("cmyk")) {
        channels = 4;
    } else if (mode == QLatin1String("grayscale") || mode == QLatin1String("bitmap")
               || mode == QLatin1String("duotone") || mode == QLatin1String("indexed")) {
        channels = 1;
    }
    return channels * qMax(1, int(image_depth_bits(*view))) / 8.0;
}

ImageSizeDialog::ImageSizeDialog(PictureView* view, QWidget* parent)
    : QDialog(parent)
    , view_(view)
{
    setObjectName(QStringLiteral("imageSizeDialog"));
    setWindowTitle(QStringLiteral("Image Size"));
    if (view_) {
        pixelWidth_ = qMax(1, int(view_->document_width()));
        pixelHeight_ = qMax(1, int(view_->document_height()));
        resolution_ = document_ppi(*view_);
    }
    bytesPerPixel_ = bytesPerPixel(view_);
    originalWidth_ = pixelWidth_;
    originalHeight_ = pixelHeight_;
    originalResolution_ = resolution_;
    buildUi();
    if (view_ && document_ppi_per_cm(*view_)) {
        const QSignalBlocker block(resolutionUnit_);
        resolutionUnit_->setCurrentIndex(1);
    }
    syncFields();
}

bool ImageSizeDialog::resultPerCm() const
{
    return resolutionUnit_->currentIndex() == 1;
}

QString ImageSizeDialog::resampleKind() const
{
    if (!resample_->isChecked()) {
        return {};
    }
    return QLatin1String(kResample[qBound(0, resampleMode_->currentIndex(),
                                          int(std::size(kResample)) - 1)]
                             .kind);
}

double ImageSizeDialog::unitScale(int unit) const
{
    return isPhysical(unit) ? resolution_ * inchesPerUnit(unit) : 1.0;
}

void ImageSizeDialog::buildUi()
{
    auto* outer = new QHBoxLayout(this);
    outer->setContentsMargins(12, 12, 12, 12);
    outer->setSpacing(14);

    preview_ = new QLabel(this);
    preview_->setObjectName(QStringLiteral("imageSizePreview"));
    preview_->setFixedSize(kPreviewBox, kPreviewBox);
    preview_->setAlignment(Qt::AlignCenter);
    if (view_) {
        const QImage image = view_->image();
        if (!image.isNull()) {
            preview_->setPixmap(QPixmap::fromImage(image).scaled(
                preview_->size() - QSize(4, 4), Qt::KeepAspectRatio, Qt::SmoothTransformation));
        }
    }
    outer->addWidget(preview_, 0, Qt::AlignTop);

    auto* right = new QVBoxLayout;
    auto* summaryRow = new QHBoxLayout;
    summaryRow->addWidget(new QLabel(QStringLiteral("Image Size:"), this));
    summary_ = new QLabel(this);
    summary_->setObjectName(QStringLiteral("imageSizeSummary"));
    summaryRow->addWidget(summary_);
    summaryRow->addStretch();
    right->addLayout(summaryRow);
    auto* dimensionsRow = new QHBoxLayout;
    dimensionsRow->addWidget(new QLabel(QStringLiteral("Dimensions:"), this));
    dimensions_ = new QLabel(this);
    dimensions_->setObjectName(QStringLiteral("imageSizeDimensions"));
    dimensionsRow->addWidget(dimensions_);
    dimensionsRow->addStretch();
    right->addLayout(dimensionsRow);

    auto* fitRow = new QHBoxLayout;
    fitRow->addWidget(new QLabel(QStringLiteral("Fit To:"), this));
    fitTo_ = new QComboBox(this);
    fitTo_->setObjectName(QStringLiteral("imageSizeFitTo"));
    fitTo_->addItem(QStringLiteral("Original Size"), kFitOriginal);
    fitTo_->addItem(QStringLiteral("Custom"), kFitCustom);
    fitTo_->addItem(QStringLiteral("Auto Resolution…"), kFitAutoResolution);
    fitTo_->insertSeparator(fitTo_->count());
    for (int i = 0; i < int(std::size(kFitPresets)); ++i) {
        fitTo_->addItem(QString::fromUtf8(kFitPresets[i].label), i);
        if (std::find(std::begin(kFitGroupBreaks), std::end(kFitGroupBreaks), i)
            != std::end(kFitGroupBreaks)) {
            fitTo_->insertSeparator(fitTo_->count());
        }
    }
    if (auto* model = qobject_cast<QStandardItemModel*>(fitTo_->model())) {
        if (QStandardItem* item = model->item(fitTo_->findData(kFitAutoResolution))) {
            item->setEnabled(false);
        }
    }
    fitRow->addWidget(fitTo_, 1);
    right->addLayout(fitRow);

    auto* grid = new QGridLayout;
    const auto field = [this](const QString& name, double min) {
        auto* spin = new QDoubleSpinBox(this);
        spin->setObjectName(name);
        spin->setRange(min, 300000.0);
        spin->setKeyboardTracking(false);
        spin->setMinimumWidth(90);
        return spin;
    };
    grid->addWidget(new QLabel(QStringLiteral("Width:"), this), 0, 1, Qt::AlignRight);
    width_ = field(QStringLiteral("imageSizeWidth"), 0.001);
    grid->addWidget(width_, 0, 2);
    widthUnit_ = new QComboBox(this);
    widthUnit_->setObjectName(QStringLiteral("imageSizeWidthUnit"));
    addUnits(widthUnit_, true);
    grid->addWidget(widthUnit_, 0, 3);
    grid->addWidget(new QLabel(QStringLiteral("Height:"), this), 1, 1, Qt::AlignRight);
    height_ = field(QStringLiteral("imageSizeHeight"), 0.001);
    grid->addWidget(height_, 1, 2);
    heightUnit_ = new QComboBox(this);
    heightUnit_->setObjectName(QStringLiteral("imageSizeHeightUnit"));
    addUnits(heightUnit_, true);
    grid->addWidget(heightUnit_, 1, 3);
    chain_ = new ChainToggle(this);
    grid->addWidget(chain_, 0, 0, 2, 1);
    grid->addWidget(new QLabel(QStringLiteral("Resolution:"), this), 2, 1, Qt::AlignRight);
    resolutionField_ = field(QStringLiteral("imageSizeResolution"), 1.0);
    resolutionField_->setDecimals(0);
    grid->addWidget(resolutionField_, 2, 2);
    resolutionUnit_ = new QComboBox(this);
    resolutionUnit_->setObjectName(QStringLiteral("imageSizeResolutionUnit"));
    resolutionUnit_->addItems({QStringLiteral("Pixels/Inch"), QStringLiteral("Pixels/Centimeter")});
    grid->addWidget(resolutionUnit_, 2, 3);
    right->addLayout(grid);

    auto* resampleRow = new QHBoxLayout;
    resample_ = new QCheckBox(QStringLiteral("Resample:"), this);
    resample_->setObjectName(QStringLiteral("imageSizeResample"));
    resample_->setChecked(true);
    resampleRow->addWidget(resample_);
    resampleMode_ = new QComboBox(this);
    resampleMode_->setObjectName(QStringLiteral("imageSizeResampleMode"));
    for (const ResampleEntry& entry : kResample) {
        resampleMode_->addItem(QLatin1String(entry.label));
    }
    resampleRow->addWidget(resampleMode_, 1);
    right->addLayout(resampleRow);
    right->addStretch();
    outer->addLayout(right, 1);

    auto* buttons = new QVBoxLayout;
    auto* ok = new QPushButton(QStringLiteral("OK"), this);
    ok->setDefault(true);
    auto* cancel = new QPushButton(QStringLiteral("Cancel"), this);
    for (QPushButton* b : {ok, cancel}) {
        b->setMinimumWidth(84);
        buttons->addWidget(b);
    }
    buttons->addStretch();
    outer->addLayout(buttons);

    connect(ok, &QPushButton::clicked, this, &QDialog::accept);
    connect(cancel, &QPushButton::clicked, this, &QDialog::reject);
    connect(width_, &QDoubleSpinBox::valueChanged, this, &ImageSizeDialog::widthEdited);
    connect(height_, &QDoubleSpinBox::valueChanged, this, &ImageSizeDialog::heightEdited);
    connect(resolutionField_, &QDoubleSpinBox::valueChanged, this,
            &ImageSizeDialog::resolutionEdited);
    for (QComboBox* unit : {widthUnit_, heightUnit_, resolutionUnit_}) {
        connect(unit, &QComboBox::currentIndexChanged, this, &ImageSizeDialog::syncFields);
    }
    connect(resample_, &QCheckBox::toggled, this, &ImageSizeDialog::resampleToggled);
    connect(fitTo_, &QComboBox::currentIndexChanged, this, &ImageSizeDialog::fitToChosen);
}

void ImageSizeDialog::syncFields()
{
    updating_ = true;
    const int wUnit = widthUnit_->currentIndex();
    const int hUnit = heightUnit_->currentIndex();
    width_->setDecimals(wUnit == UnitPixels ? 0 : 2);
    height_->setDecimals(hUnit == UnitPixels ? 0 : 2);
    width_->setValue(wUnit == UnitPercent ? 100.0 * pixelWidth_ / originalWidth_
                                          : pixelWidth_ / unitScale(wUnit));
    height_->setValue(hUnit == UnitPercent ? 100.0 * pixelHeight_ / originalHeight_
                                           : pixelHeight_ / unitScale(hUnit));
    resolutionField_->setValue(resultPerCm() ? resolution_ / 2.54 : resolution_);
    updating_ = false;
    updateSummary();
}

void ImageSizeDialog::updateSummary()
{
    summary_->setText(imageSizeSummary(bytesPerPixel_ * pixelWidth_ * pixelHeight_));
    dimensions_->setText(QStringLiteral("%1 px × %2 px").arg(pixelWidth_).arg(pixelHeight_));
}

void ImageSizeDialog::widthEdited()
{
    if (updating_) {
        return;
    }
    markCustom();
    const int unit = widthUnit_->currentIndex();
    if (!resample_->isChecked()) {
        // The pixels are fixed, so a new printed width is a new resolution.
        if (isPhysical(unit) && width_->value() > 0.0) {
            resolution_ = pixelWidth_ / (width_->value() * inchesPerUnit(unit));
        }
        syncFields();
        return;
    }
    pixelWidth_ = qMax(1, unit == UnitPercent ? qRound(originalWidth_ * width_->value() / 100.0)
                                              : qRound(width_->value() * unitScale(unit)));
    if (chain_->isChecked()) {
        pixelHeight_ = qMax(1, qRound(double(pixelWidth_) * originalHeight_ / originalWidth_));
    }
    syncFields();
}

void ImageSizeDialog::heightEdited()
{
    if (updating_) {
        return;
    }
    markCustom();
    const int unit = heightUnit_->currentIndex();
    if (!resample_->isChecked()) {
        if (isPhysical(unit) && height_->value() > 0.0) {
            resolution_ = pixelHeight_ / (height_->value() * inchesPerUnit(unit));
        }
        syncFields();
        return;
    }
    pixelHeight_ = qMax(1, unit == UnitPercent
                               ? qRound(originalHeight_ * height_->value() / 100.0)
                               : qRound(height_->value() * unitScale(unit)));
    if (chain_->isChecked()) {
        pixelWidth_ = qMax(1, qRound(double(pixelHeight_) * originalWidth_ / originalHeight_));
    }
    syncFields();
}

void ImageSizeDialog::resolutionEdited()
{
    if (updating_) {
        return;
    }
    markCustom();
    const double entered = resolutionField_->value();
    const double ppi = resultPerCm() ? entered * 2.54 : entered;
    // With Resample on, a physical Width/Height keeps its printed size, so
    // its pixel count follows the resolution; in pixels it stays put. Chained,
    // either physical axis carries the other with it.
    if (resample_->isChecked()) {
        const double scale = ppi / resolution_;
        const bool chained = chain_->isChecked();
        const bool physicalWidth = isPhysical(widthUnit_->currentIndex());
        const bool physicalHeight = isPhysical(heightUnit_->currentIndex());
        if (physicalWidth || (chained && physicalHeight)) {
            pixelWidth_ = qMax(1, qRound(pixelWidth_ * scale));
        }
        if (physicalHeight || (chained && physicalWidth)) {
            pixelHeight_ = qMax(1, qRound(pixelHeight_ * scale));
        }
    }
    resolution_ = ppi;
    syncFields();
}

void ImageSizeDialog::fitToChosen(int index)
{
    if (updating_) {
        return;
    }
    const int data = fitTo_->itemData(index).toInt();
    if (data != kFitCustom && data != kFitAutoResolution) {
        applyFitPreset(data);
    }
}

void ImageSizeDialog::applyFitPreset(int preset)
{
    if (preset == kFitOriginal) {
        pixelWidth_ = originalWidth_;
        pixelHeight_ = originalHeight_;
        resolution_ = originalResolution_;
        syncFields();
        return;
    }
    if (preset < 0 || preset >= int(std::size(kFitPresets))) {
        return;
    }
    const FitPreset& fit = kFitPresets[preset];
    int w = presetPixels(fit.width, fit.unit, fit.dpi);
    int h = presetPixels(fit.height, fit.unit, fit.dpi);
    // Presets are listed one way round; an image the other way takes the
    // preset on its side rather than being turned.
    if ((pixelWidth_ > pixelHeight_) != (w > h)) {
        std::swap(w, h);
    }
    pixelWidth_ = qMax(1, w);
    pixelHeight_ = qMax(1, h);
    resolution_ = fit.dpi;
    if (!resample_->isChecked()) {
        const QSignalBlocker block(resample_);
        resample_->setChecked(true);
        resampleMode_->setEnabled(true);
    }
    syncFields();
}

void ImageSizeDialog::markCustom()
{
    const int custom = fitTo_->findData(kFitCustom);
    if (custom >= 0 && fitTo_->currentIndex() != custom) {
        const QSignalBlocker block(fitTo_);
        fitTo_->setCurrentIndex(custom);
    }
}

void ImageSizeDialog::resampleToggled(bool on)
{
    resampleMode_->setEnabled(on);
    if (!on) {
        // From here only the print size can change, never the pixels.
        pixelWidth_ = originalWidth_;
        pixelHeight_ = originalHeight_;
    }
    syncFields();
}

QWidget* ImageSizeDialog::controlForTest(const QString& name) const
{
    return findChild<QWidget*>(name);
}

} // namespace pictura
