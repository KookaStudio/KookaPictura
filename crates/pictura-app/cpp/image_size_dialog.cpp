#include "image_size_dialog.h"
#include <QtCore/QCoreApplication>

#include "api/document_api.h"
#include "api/document_bridge.h"
#include "api/paint_bridge.h"
#include "canvas_ruler.h"
#include "dialogs.h"
#include "memory_refusal.h"
#include "preferences.h"

#include <QtCore/QSignalBlocker>
#include <QtGui/QPainter>

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>
#include <cmath>
#include <iterator>
#include <limits>

namespace pictura {

namespace {

// A test driver answers the Image Size dialog and its budget warning instead of
// running them modally.
std::function<bool(QDialog&)> imageSizeDialogDriver;

// Width/Height units, in CS6's order. Pixels and Percent stand apart from the
// physical units, which all convert through the resolution.
enum Unit {
    UnitPixels = 0, UnitInches, UnitCm, UnitMm, UnitPoints, UnitPercent, UnitPicas, UnitColumns
};

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
    {"Automatic", "automatic"},
    {"Preserve Details (enlargement)", "bicubic"},
    {"Bicubic Smoother (enlargement)", "bicubic-smoother"},
    {"Bicubic Sharper (reduction)", "bicubic-sharper"},
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

void addUnits(QComboBox* box, bool columns)
{
    box->addItems({QStringLiteral("Pixels"), QStringLiteral("Inches"),
                   QStringLiteral("Centimeters"), QStringLiteral("Millimeters"),
                   QStringLiteral("Points"),
                   QCoreApplication::translate("PicturaUi", "Percent"),
                   ImageSizeDialog::tr("Picas")});
    if (columns) {
        box->addItem(ImageSizeDialog::tr("Columns"));
    }
}

bool isPhysical(int unit)
{
    return unit != UnitPixels && unit != UnitPercent;
}

double inchesPerUnit(int unit, bool traditionalPoints)
{
    switch (unit) {
    case UnitCm:
        return 1.0 / 2.54;
    case UnitMm:
        return 1.0 / 25.4;
    case UnitPoints:
        return pixelsPerRulerUnit(RulerUnit::Points, 1.0, traditionalPoints, 0.0);
    case UnitPicas:
        return pixelsPerRulerUnit(RulerUnit::Picas, 1.0, traditionalPoints, 0.0);
    default:
        return 1.0;
    }
}

int roundedPixelCount(double pixels)
{
    return qRound(std::clamp(pixels, 1.0, double(std::numeric_limits<int>::max())));
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

bool imageSizeFitsBudget(PictureView& view, int width, int height)
{
    return image_size_fits_budget(view, width, height);
}

QString imageSizeBudgetMessage(int width, int height)
{
    const QString memory = ImageSizeDialog::tr(
                               "Not enough memory to resize to %1 × %2 pixels. Close other "
                               "documents or free memory and try again.")
                               .arg(width)
                               .arg(height);
    const QString scratchFull = ImageSizeDialog::tr(
                                    "Could not resize to %1 × %2 pixels: the scratch disks are "
                                    "full. Free space on the scratch disks, then try again.")
                                    .arg(width)
                                    .arg(height);
    const QString unreadable = ImageSizeDialog::tr(
                                   "Could not resize to %1 × %2 pixels: image data could not be "
                                   "read back from the scratch disk. Check the scratch disks, "
                                   "then try again.")
                                   .arg(width)
                                   .arg(height);
    const QString message = refusalMessage(last_refusal(), memory, scratchFull, unreadable);
    return message.isEmpty() ? memory : message;
}

void setImageSizeDialogDriverForTest(std::function<bool(QDialog&)> driver)
{
    imageSizeDialogDriver = std::move(driver);
}

bool answerImageSizeDialog(QDialog& dialog, QWidget* parent)
{
    return imageSizeDialogDriver ? imageSizeDialogDriver(dialog)
                                 : runDialog(dialog, parent) == QDialog::Accepted;
}

void showImageSizeBudgetWarning(QWidget* parent, int width, int height)
{
    QMessageBox warning(QMessageBox::Warning, ImageSizeDialog::tr("Image Size"),
                        imageSizeBudgetMessage(width, height), QMessageBox::Ok, parent);
    if (imageSizeDialogDriver) {
        imageSizeDialogDriver(warning);
    } else {
        warning.exec();
    }
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
    const auto prefs = loadPreferences();
    traditionalPoints_ = prefs.traditionalPoints;
    if (std::isfinite(prefs.columnWidthInches) && prefs.columnWidthInches > 0.0) {
        columnWidthInches_ = prefs.columnWidthInches;
    }
    if (std::isfinite(prefs.columnGutterInches) && prefs.columnGutterInches >= 0.0) {
        columnGutterInches_ = prefs.columnGutterInches;
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
    return resampleMode_->currentData().toString();
}

bool ImageSizeDialog::scaleStylesForResize() const
{
    return resample_->isChecked() && chain_->isChecked() && scaleStyles_->isChecked();
}

double ImageSizeDialog::enteredInches(double size, int unit) const
{
    if (unit == UnitColumns) {
        return size * columnWidthInches_ + std::max(0.0, size - 1.0) * columnGutterInches_;
    }
    return size * inchesPerUnit(unit, traditionalPoints_);
}

double ImageSizeDialog::displayedSize(int pixels, int unit, int originalPixels) const
{
    if (unit == UnitPixels) {
        return pixels;
    }
    if (unit == UnitPercent) {
        return 100.0 * pixels / originalPixels;
    }
    const double inches = pixels / resolution_;
    if (unit == UnitColumns) {
        if (inches < columnWidthInches_) {
            return inches / columnWidthInches_;
        }
        return (inches + columnGutterInches_) / (columnWidthInches_ + columnGutterInches_);
    }
    return inches / inchesPerUnit(unit, traditionalPoints_);
}

int ImageSizeDialog::enteredPixels(double size, int unit, int originalPixels) const
{
    if (unit == UnitPixels) {
        return roundedPixelCount(size);
    }
    if (unit == UnitPercent) {
        return roundedPixelCount(originalPixels * size / 100.0);
    }
    return roundedPixelCount(enteredInches(size, unit) * resolution_);
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
        // The preview box is small, so a reduced level is enough (#1274).
        const QImage image = pictura::preview_image(*view_, kPreviewBox);
        if (!image.isNull()) {
            preview_->setPixmap(QPixmap::fromImage(image).scaled(
                preview_->size() - QSize(4, 4), Qt::KeepAspectRatio, Qt::SmoothTransformation));
        }
    }
    outer->addWidget(preview_, 0, Qt::AlignTop);

    auto* right = new QVBoxLayout;
    auto* summaryRow = new QHBoxLayout;
    summaryRow->addWidget(new QLabel(QCoreApplication::translate("PicturaUi", "Image Size:"), this));
    summary_ = new QLabel(this);
    summary_->setObjectName(QStringLiteral("imageSizeSummary"));
    summaryRow->addWidget(summary_);
    summaryRow->addStretch();
    right->addLayout(summaryRow);
    auto* dimensionsRow = new QHBoxLayout;
    dimensionsRow->addWidget(new QLabel(QCoreApplication::translate("PicturaUi", "Dimensions:"), this));
    dimensions_ = new QLabel(this);
    dimensions_->setObjectName(QStringLiteral("imageSizeDimensions"));
    dimensionsRow->addWidget(dimensions_);
    dimensionsRow->addStretch();
    right->addLayout(dimensionsRow);

    auto* fitRow = new QHBoxLayout;
    fitRow->addWidget(new QLabel(QCoreApplication::translate("PicturaUi", "Fit To:"), this));
    fitTo_ = new QComboBox(this);
    fitTo_->setObjectName(QStringLiteral("imageSizeFitTo"));
    fitTo_->addItem(QCoreApplication::translate("PicturaUi", "Original Size"), kFitOriginal);
    fitTo_->addItem(QCoreApplication::translate("PicturaUi", "Custom"), kFitCustom);
    fitTo_->addItem(QCoreApplication::translate("PicturaUi", "Auto Resolution…"), kFitAutoResolution);
    fitTo_->insertSeparator(fitTo_->count());
    for (int i = 0; i < int(std::size(kFitPresets)); ++i) {
        fitTo_->addItem(QString::fromUtf8(kFitPresets[i].label), i);
        if (std::find(std::begin(kFitGroupBreaks), std::end(kFitGroupBreaks), i)
            != std::end(kFitGroupBreaks)) {
            fitTo_->insertSeparator(fitTo_->count());
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
    grid->addWidget(new QLabel(QCoreApplication::translate("PicturaUi", "Width:"), this), 0, 1, Qt::AlignRight);
    width_ = field(QStringLiteral("imageSizeWidth"), 0.001);
    grid->addWidget(width_, 0, 2);
    widthUnit_ = new QComboBox(this);
    widthUnit_->setObjectName(QStringLiteral("imageSizeWidthUnit"));
    addUnits(widthUnit_, true);
    grid->addWidget(widthUnit_, 0, 3);
    grid->addWidget(new QLabel(QCoreApplication::translate("PicturaUi", "Height:"), this), 1, 1, Qt::AlignRight);
    height_ = field(QStringLiteral("imageSizeHeight"), 0.001);
    grid->addWidget(height_, 1, 2);
    heightUnit_ = new QComboBox(this);
    heightUnit_->setObjectName(QStringLiteral("imageSizeHeightUnit"));
    addUnits(heightUnit_, false);
    grid->addWidget(heightUnit_, 1, 3);
    chain_ = new ChainToggle(this);
    grid->addWidget(chain_, 0, 0, 2, 1);
    grid->addWidget(new QLabel(QCoreApplication::translate("PicturaUi", "Resolution:"), this), 2, 1, Qt::AlignRight);
    resolutionField_ = field(QStringLiteral("imageSizeResolution"), 0.01);
    resolutionField_->setDecimals(2);
    grid->addWidget(resolutionField_, 2, 2);
    resolutionUnit_ = new QComboBox(this);
    resolutionUnit_->setObjectName(QStringLiteral("imageSizeResolutionUnit"));
    resolutionUnit_->addItems({QStringLiteral("Pixels/Inch"), QStringLiteral("Pixels/Centimeter")});
    grid->addWidget(resolutionUnit_, 2, 3);
    right->addLayout(grid);

    auto* resampleRow = new QHBoxLayout;
    resample_ = new QCheckBox(QCoreApplication::translate("PicturaUi", "Resample:"), this);
    resample_->setObjectName(QStringLiteral("imageSizeResample"));
    resample_->setChecked(true);
    resampleRow->addWidget(resample_);
    resampleMode_ = new QComboBox(this);
    resampleMode_->setObjectName(QStringLiteral("imageSizeResampleMode"));
    for (const ResampleEntry& entry : kResample) {
        resampleMode_->addItem(QLatin1String(entry.label), QString::fromLatin1(entry.kind));
    }
    resampleRow->addWidget(resampleMode_, 1);
    right->addLayout(resampleRow);
    scaleStyles_ = new QCheckBox(tr("Scale Styles"), this);
    scaleStyles_->setObjectName(QStringLiteral("imageSizeScaleStyles"));
    scaleStyles_->setChecked(true);
    right->addWidget(scaleStyles_);
    right->addStretch();
    outer->addLayout(right, 1);

    auto* buttons = new QVBoxLayout;
    auto* ok = new QPushButton(QCoreApplication::translate("PicturaUi", "OK"), this);
    ok->setDefault(true);
    auto* cancel = new QPushButton(QCoreApplication::translate("PicturaUi", "Cancel"), this);
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
    connect(chain_, &QToolButton::toggled, this, &ImageSizeDialog::updateResampleControls);
    connect(fitTo_, &QComboBox::currentIndexChanged, this, &ImageSizeDialog::fitToChosen);
}

void ImageSizeDialog::syncFields()
{
    updating_ = true;
    const int wUnit = widthUnit_->currentIndex();
    const int hUnit = heightUnit_->currentIndex();
    const auto sync = [this](QDoubleSpinBox* field, int unit, int pixels, int original) {
        field->setDecimals(unit == UnitPixels ? 0 : 4);
        field->setMinimum(unit == UnitPixels ? 1.0 : 0.0001);
        field->setValue(displayedSize(pixels, unit, original));
        field->setReadOnly(!resample_->isChecked() && !isPhysical(unit));
    };
    sync(width_, wUnit, pixelWidth_, originalWidth_);
    sync(height_, hUnit, pixelHeight_, originalHeight_);
    resolutionField_->setValue(resultPerCm() ? resolution_ / 2.54 : resolution_);
    updating_ = false;
    updateSummary();
}

void ImageSizeDialog::updateSummary()
{
    summary_->setText(imageSizeSummary(bytesPerPixel_ * pixelWidth_ * pixelHeight_));
    dimensions_->setText(QCoreApplication::translate("PicturaUi", "%1 px × %2 px").arg(pixelWidth_).arg(pixelHeight_));
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
            resolution_ = pixelWidth_ / enteredInches(width_->value(), unit);
        }
        syncFields();
        return;
    }
    pixelWidth_ = enteredPixels(width_->value(), unit, originalWidth_);
    if (chain_->isChecked()) {
        pixelHeight_ = roundedPixelCount(double(pixelWidth_) * originalHeight_ / originalWidth_);
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
            resolution_ = pixelHeight_ / enteredInches(height_->value(), unit);
        }
        syncFields();
        return;
    }
    pixelHeight_ = enteredPixels(height_->value(), unit, originalHeight_);
    if (chain_->isChecked()) {
        pixelWidth_ = roundedPixelCount(double(pixelHeight_) * originalWidth_ / originalHeight_);
    }
    syncFields();
}

void ImageSizeDialog::resolutionEdited()
{
    if (updating_) {
        return;
    }
    const double entered = resolutionField_->value();
    setResolutionPpi(resultPerCm() ? entered * 2.54 : entered);
}

void ImageSizeDialog::setResolutionPpi(double ppi)
{
    markCustom();
    // With Resample on, a physical Width/Height keeps its printed size, so
    // its pixel count follows the resolution; in pixels it stays put. Chained,
    // either physical axis carries the other with it.
    if (resample_->isChecked()) {
        const double scale = ppi / resolution_;
        const bool chained = chain_->isChecked();
        const bool physicalWidth = isPhysical(widthUnit_->currentIndex());
        const bool physicalHeight = isPhysical(heightUnit_->currentIndex());
        if (physicalWidth || (chained && physicalHeight)) {
            pixelWidth_ = roundedPixelCount(pixelWidth_ * scale);
        }
        if (physicalHeight || (chained && physicalWidth)) {
            pixelHeight_ = roundedPixelCount(pixelHeight_ * scale);
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
    if (data == kFitAutoResolution) {
        showAutoResolution();
        return;
    }
    lastFitIndex_ = index;
    if (data != kFitCustom) {
        applyFitPreset(data);
    }
}

void ImageSizeDialog::showAutoResolution()
{
    {
        const QSignalBlocker blocked(fitTo_);
        fitTo_->setCurrentIndex(lastFitIndex_);
    }
    QDialog dialog(this);
    dialog.setObjectName(QStringLiteral("imageSizeAutoResolution"));
    dialog.setWindowTitle(tr("Auto Resolution"));
    auto* layout = new QFormLayout(&dialog);
    auto* screen = new QDoubleSpinBox(&dialog);
    screen->setObjectName(QStringLiteral("imageSizeAutoScreen"));
    screen->setDecimals(2);
    screen->setRange(0.01, 150000.0);
    screen->setValue(150.0);
    auto* unit = new QComboBox(&dialog);
    unit->setObjectName(QStringLiteral("imageSizeAutoScreenUnit"));
    unit->addItems({tr("Lines/Inch"), tr("Lines/Centimeter")});
    auto* screenRow = new QHBoxLayout;
    screenRow->addWidget(screen);
    screenRow->addWidget(unit);
    layout->addRow(tr("Screen:"), screenRow);

    auto* quality = new QComboBox(&dialog);
    quality->setObjectName(QStringLiteral("imageSizeAutoQuality"));
    quality->addItems({tr("Draft"), tr("Good"), tr("Best")});
    quality->setCurrentIndex(1);
    layout->addRow(tr("Quality:"), quality);
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    layout->addRow(buttons);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    connect(unit, &QComboBox::currentIndexChanged, &dialog, [screen](int index) {
        const double converted = index == 1 ? screen->value() / 2.54 : screen->value() * 2.54;
        screen->setMaximum(index == 1 ? 150000.0 / 2.54 : 150000.0);
        screen->setValue(converted);
    });

    if (answerImageSizeDialog(dialog, this)) {
        const double frequency = screen->value() * (unit->currentIndex() == 1 ? 2.54 : 1.0);
        const int choice = quality->currentIndex();
        const double ppi = frequency * (1.0 + 0.5 * choice);
        setResolutionPpi(choice == 0 ? std::max(72.0, ppi) : ppi);
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
        updateResampleControls();
    }
    syncFields();
}

void ImageSizeDialog::markCustom()
{
    const int custom = fitTo_->findData(kFitCustom);
    if (custom >= 0 && fitTo_->currentIndex() != custom) {
        const QSignalBlocker block(fitTo_);
        fitTo_->setCurrentIndex(custom);
        lastFitIndex_ = custom;
    }
}

void ImageSizeDialog::updateResampleControls()
{
    const bool resample = resample_->isChecked();
    resampleMode_->setEnabled(resample);
    chain_->setEnabled(resample);
    scaleStyles_->setEnabled(resample && chain_->isChecked());
}

void ImageSizeDialog::resampleToggled(bool on)
{
    updateResampleControls();
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

#include "moc_image_size_dialog.cpp"
