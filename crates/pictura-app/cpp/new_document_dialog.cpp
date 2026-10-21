#include "new_document_dialog.h"

#include "color_picker_dialog.h"
#include "dialogs.h"

#include <QtGui/QClipboard>
#include <QtGui/QGuiApplication>
#include <QtGui/QImage>
#include <QtGui/QStandardItemModel>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

#include <cmath>
#include <iterator>
#include <vector>

namespace pictura {

namespace {

enum Unit { Pixels = 0, Inches, Centimeters, Millimeters, Points, Picas };

struct SizePreset {
    const char* name;
    double width;
    double height;
    int unit;
    int ppi;
};

// CS6's Document Type menu and the sizes each offers, all kept in the units
// their labels use so a preset cannot drift from its name.
struct DocumentType {
    const char* name;
    std::vector<SizePreset> sizes;
};

const DocumentType kTypes[] = {
    {"Custom", {}},
    {"Clipboard", {}},
    {"Default Photoshop Size", {{"Default Photoshop Size", 1280, 800, Pixels, 72}}},
    {"U.S. Paper",
     {{"Letter", 8.5, 11, Inches, 300},
      {"Legal", 8.5, 14, Inches, 300},
      {"Tabloid", 11, 17, Inches, 300}}},
    {"International Paper",
     {{"A4", 210, 297, Millimeters, 300},
      {"A3", 297, 420, Millimeters, 300},
      {"A5", 148, 210, Millimeters, 300},
      {"B5", 176, 250, Millimeters, 300}}},
    {"Photo",
     {{"Portrait, 2 x 3", 2, 3, Inches, 300},
      {"Portrait, 4 x 6", 4, 6, Inches, 300},
      {"Portrait, 5 x 7", 5, 7, Inches, 300},
      {"Portrait, 8 x 10", 8, 10, Inches, 300},
      {"Landscape, 3 x 2", 3, 2, Inches, 300},
      {"Landscape, 6 x 4", 6, 4, Inches, 300},
      {"Landscape, 7 x 5", 7, 5, Inches, 300},
      {"Landscape, 10 x 8", 10, 8, Inches, 300}}},
    {"Web",
     {{"640 x 480", 640, 480, Pixels, 72},
      {"800 x 600", 800, 600, Pixels, 72},
      {"1024 x 768", 1024, 768, Pixels, 72},
      {"1152 x 864", 1152, 864, Pixels, 72},
      {"1280 x 1024", 1280, 1024, Pixels, 72},
      {"1366 x 768", 1366, 768, Pixels, 72},
      {"1440 x 900", 1440, 900, Pixels, 72},
      {"1600 x 1200", 1600, 1200, Pixels, 72},
      {"1920 x 1080", 1920, 1080, Pixels, 72}}},
    {"Mobile & Devices",
     {{"320 x 480", 320, 480, Pixels, 72},
      {"640 x 960", 640, 960, Pixels, 72},
      {"640 x 1136", 640, 1136, Pixels, 72},
      {"768 x 1024", 768, 1024, Pixels, 72},
      {"1536 x 2048", 1536, 2048, Pixels, 72}}},
    {"Film & Video",
     {{"HDTV 1080p/29.97", 1920, 1080, Pixels, 72},
      {"HDV/HDTV 720p/29.97", 1280, 720, Pixels, 72},
      {"NTSC DV", 720, 480, Pixels, 72},
      {"PAL D1/DV", 720, 576, Pixels, 72}}},
};
constexpr int kCustomType = 0;
constexpr int kClipboardType = 1;
constexpr int kDefaultType = 2;

struct Mode {
    const char* label;
    const char* key;
    int channels;
};
const Mode kModes[] = {
    {"Bitmap", "bitmap", 1}, {"Grayscale", "grayscale", 1}, {"RGB Color", "rgb", 3},
    {"CMYK Color", "cmyk", 4}, {"Lab Color", "lab", 3},
};
constexpr int kDepths[] = {1, 8, 16, 32};

const QString kWhite = QStringLiteral("White");
const QString kBlack = QStringLiteral("Black");
const QString kBackground = QStringLiteral("Background Color");
const QString kTransparent = QStringLiteral("Transparent");
const QString kCustom = QStringLiteral("Custom…");

constexpr int kLargeSide = 30000;

QString formatSize(double bytes)
{
    if (bytes < 1024.0) {
        return QStringLiteral("%1 bytes").arg(qRound(bytes));
    }
    if (bytes < 1024.0 * 1024.0) {
        return QStringLiteral("%1K").arg(bytes / 1024.0, 0, 'f', 1);
    }
    if (bytes < 1024.0 * 1024.0 * 1024.0) {
        return QStringLiteral("%1M").arg(bytes / (1024.0 * 1024.0), 0, 'f', 2);
    }
    return QStringLiteral("%1G").arg(bytes / (1024.0 * 1024.0 * 1024.0), 0, 'f', 2);
}

// Enable only the entries `allowed` keeps.
template <typename Allowed>
void enableItems(QComboBox* combo, Allowed allowed)
{
    auto* model = qobject_cast<QStandardItemModel*>(combo->model());
    for (int i = 0; model && i < combo->count(); ++i) {
        if (QStandardItem* item = model->item(i)) {
            item->setEnabled(allowed(i));
        }
    }
}

} // namespace

NewDocumentDialog::NewDocumentDialog(const QString& name, const QColor& background,
                                     QWidget* parent)
    : QDialog(parent)
    , background_(background)
{
    setObjectName(QStringLiteral("newDocumentDialog"));
    setWindowTitle(QStringLiteral("New"));
    buildUi();
    name_->setText(name);
    name_->selectAll();
    // CS6 preloads the clipboard image's dimensions: open on Clipboard when
    // one is available, otherwise on the default size.
    documentType_->setCurrentIndex(clipboardHasImage_ ? kClipboardType : kDefaultType);
    updateImageSize();
}

void NewDocumentDialog::buildUi()
{
    auto* outer = new QHBoxLayout(this);
    outer->setContentsMargins(12, 12, 12, 12);
    outer->setSpacing(14);
    auto* left = new QVBoxLayout;
    auto* grid = new QGridLayout;
    grid->setHorizontalSpacing(8);
    grid->setVerticalSpacing(8);
    grid->setColumnStretch(1, 1);
    int row = 0;
    const auto label = [this, grid, &row](const QString& text) {
        grid->addWidget(new QLabel(text, this), row, 0);
    };

    label(QStringLiteral("Name:"));
    name_ = new QLineEdit(this);
    name_->setObjectName(QStringLiteral("newDocName"));
    name_->setMinimumWidth(260);
    grid->addWidget(name_, row++, 1, 1, 2);

    label(QStringLiteral("Document Type:"));
    documentType_ = new QComboBox(this);
    documentType_->setObjectName(QStringLiteral("newDocType"));
    for (const DocumentType& type : kTypes) {
        documentType_->addItem(QLatin1String(type.name));
    }
    const QImage clip = QGuiApplication::clipboard()->image();
    clipboardHasImage_ = !clip.isNull();
    if (clip.isNull()) {
        enableItems(documentType_, [](int i) { return i != kClipboardType; });
    }
    grid->addWidget(documentType_, row++, 1, 1, 2);

    label(QStringLiteral("Size:"));
    size_ = new QComboBox(this);
    size_->setObjectName(QStringLiteral("newDocSize"));
    grid->addWidget(size_, row++, 1, 1, 2);

    const auto number = [this](const QString& name) {
        auto* spin = new QDoubleSpinBox(this);
        spin->setObjectName(name);
        spin->setRange(0.01, 300000.0);
        spin->setKeyboardTracking(false);
        spin->setMinimumWidth(110);
        return spin;
    };
    label(QStringLiteral("Width:"));
    width_ = number(QStringLiteral("newDocWidth"));
    grid->addWidget(width_, row, 1);
    unitCombo_ = new QComboBox(this);
    unitCombo_->setObjectName(QStringLiteral("newDocUnit"));
    unitCombo_->addItems({QStringLiteral("Pixels"), QStringLiteral("Inches"),
                          QStringLiteral("Centimeters"), QStringLiteral("Millimeters"),
                          QStringLiteral("Points"), QStringLiteral("Picas")});
    grid->addWidget(unitCombo_, row++, 2);
    label(QStringLiteral("Height:"));
    height_ = number(QStringLiteral("newDocHeight"));
    grid->addWidget(height_, row++, 1);

    label(QStringLiteral("Resolution:"));
    resolution_ = number(QStringLiteral("newDocResolution"));
    resolution_->setDecimals(0);
    resolution_->setRange(1.0, 10000.0);
    resolution_->setValue(72.0);
    grid->addWidget(resolution_, row, 1);
    resolutionUnit_ = new QComboBox(this);
    resolutionUnit_->setObjectName(QStringLiteral("newDocResolutionUnit"));
    resolutionUnit_->addItems({QStringLiteral("Pixels/Inch"), QStringLiteral("Pixels/Centimeter")});
    grid->addWidget(resolutionUnit_, row++, 2);

    label(QStringLiteral("Color Mode:"));
    mode_ = new QComboBox(this);
    mode_->setObjectName(QStringLiteral("newDocMode"));
    for (const Mode& mode : kModes) {
        mode_->addItem(QLatin1String(mode.label), QLatin1String(mode.key));
    }
    mode_->setCurrentIndex(2);
    grid->addWidget(mode_, row, 1);
    depth_ = new QComboBox(this);
    depth_->setObjectName(QStringLiteral("newDocDepth"));
    for (int bits : kDepths) {
        depth_->addItem(QStringLiteral("%1 bit").arg(bits), bits);
    }
    depth_->setCurrentIndex(1);
    grid->addWidget(depth_, row++, 2);

    label(QStringLiteral("Background Contents:"));
    contents_ = new QComboBox(this);
    contents_->setObjectName(QStringLiteral("newDocBackground"));
    contents_->addItems({kWhite, kBlack, kBackground, kTransparent, kCustom});
    grid->addWidget(contents_, row++, 1);
    left->addLayout(grid);
    left->addSpacing(8);

    auto* advanced = new QGroupBox(QStringLiteral("Advanced"), this);
    auto* advancedGrid = new QGridLayout(advanced);
    advancedGrid->setColumnStretch(1, 1);
    advancedGrid->addWidget(new QLabel(QStringLiteral("Color Profile:"), advanced), 0, 0);
    auto* profile = new QComboBox(advanced);
    profile->setObjectName(QStringLiteral("newDocProfile"));
    profile->addItem(QStringLiteral("Working RGB:  sRGB IEC61966-2.1"));
    profile->setEnabled(false);
    profile->setToolTip(QStringLiteral("New documents are created untagged in sRGB."));
    advancedGrid->addWidget(profile, 0, 1);
    advancedGrid->addWidget(new QLabel(QStringLiteral("Pixel Aspect Ratio:"), advanced), 1, 0);
    auto* aspect = new QComboBox(advanced);
    aspect->setObjectName(QStringLiteral("newDocAspect"));
    aspect->addItem(QStringLiteral("Square Pixels"));
    aspect->setEnabled(false);
    aspect->setToolTip(QStringLiteral("Only square pixels are supported."));
    advancedGrid->addWidget(aspect, 1, 1);
    left->addWidget(advanced);
    left->addStretch();
    outer->addLayout(left, 1);

    auto* right = new QVBoxLayout;
    const auto button = [this, right](const QString& text, bool enabled) {
        auto* b = new QPushButton(text, this);
        b->setMinimumWidth(110);
        b->setEnabled(enabled);
        right->addWidget(b);
        return b;
    };
    QPushButton* ok = button(QStringLiteral("OK"), true);
    ok->setDefault(true);
    QPushButton* cancel = button(QStringLiteral("Cancel"), true);
    right->addSpacing(16);
    button(QStringLiteral("Save Preset…"), false);
    button(QStringLiteral("Delete Preset…"), false);
    right->addStretch();
    auto* sizeHeader = new QLabel(QStringLiteral("Image Size:"), this);
    QFont bold = sizeHeader->font();
    bold.setBold(true);
    sizeHeader->setFont(bold);
    right->addWidget(sizeHeader);
    imageSize_ = new QLabel(this);
    imageSize_->setObjectName(QStringLiteral("newDocImageSize"));
    right->addWidget(imageSize_);
    outer->addLayout(right);

    connect(ok, &QPushButton::clicked, this, &NewDocumentDialog::accept);
    connect(cancel, &QPushButton::clicked, this, &QDialog::reject);
    connect(documentType_, &QComboBox::currentIndexChanged, this,
            &NewDocumentDialog::documentTypeChosen);
    connect(size_, &QComboBox::currentIndexChanged, this, &NewDocumentDialog::sizeChosen);
    connect(unitCombo_, &QComboBox::currentIndexChanged, this, &NewDocumentDialog::unitChosen);
    connect(resolutionUnit_, &QComboBox::currentIndexChanged, this,
            &NewDocumentDialog::resolutionUnitChosen);
    connect(width_, &QDoubleSpinBox::valueChanged, this, &NewDocumentDialog::dimensionEdited);
    connect(height_, &QDoubleSpinBox::valueChanged, this, &NewDocumentDialog::dimensionEdited);
    connect(resolution_, &QDoubleSpinBox::valueChanged, this, &NewDocumentDialog::dimensionEdited);
    connect(mode_, &QComboBox::currentIndexChanged, this, &NewDocumentDialog::modeChosen);
    connect(depth_, &QComboBox::currentIndexChanged, this, &NewDocumentDialog::updateImageSize);
    connect(contents_, &QComboBox::activated, this, &NewDocumentDialog::backgroundChosen);
    modeChosen();
}

double NewDocumentDialog::ppi() const
{
    return resolutionUnit_->currentIndex() == 1 ? resolution_->value() * 2.54
                                                : resolution_->value();
}

double NewDocumentDialog::pixelsPerUnit(int unit) const
{
    const double dpi = ppi();
    switch (unit) {
    case Inches:
        return dpi;
    case Centimeters:
        return dpi / 2.54;
    case Millimeters:
        return dpi / 25.4;
    case Points:
        return dpi / 72.0;
    case Picas:
        return dpi / 6.0;
    default:
        return 1.0;
    }
}

int NewDocumentDialog::widthPixels() const
{
    return qMax(1, qRound(pixelWidth_));
}

int NewDocumentDialog::heightPixels() const
{
    return qMax(1, qRound(pixelHeight_));
}

void NewDocumentDialog::setPixels(double width, double height, int unit)
{
    updating_ = true;
    unit_ = unit;
    unitCombo_->setCurrentIndex(unit);
    width_->setDecimals(unit == Pixels ? 0 : 2);
    height_->setDecimals(unit == Pixels ? 0 : 2);
    width_->setValue(width);
    height_->setValue(height);
    pixelWidth_ = width * pixelsPerUnit(unit);
    pixelHeight_ = height * pixelsPerUnit(unit);
    updating_ = false;
    updateImageSize();
}

void NewDocumentDialog::documentTypeChosen(int index)
{
    if (updating_) {
        return;
    }
    updating_ = true;
    size_->clear();
    if (index >= 0 && index < int(std::size(kTypes))) {
        for (const SizePreset& preset : kTypes[index].sizes) {
            size_->addItem(QLatin1String(preset.name));
        }
    }
    size_->setEnabled(size_->count() > 0);
    updating_ = false;
    if (index == kClipboardType) {
        const QImage clip = QGuiApplication::clipboard()->image();
        if (!clip.isNull()) {
            setPixels(clip.width(), clip.height(), Pixels);
        }
        return;
    }
    if (size_->count() > 0) {
        sizeChosen(0);
    }
}

void NewDocumentDialog::sizeChosen(int index)
{
    const int type = documentType_->currentIndex();
    if (updating_ || type < 0 || type >= int(std::size(kTypes)) || index < 0
        || index >= int(kTypes[type].sizes.size())) {
        return;
    }
    const SizePreset& preset = kTypes[type].sizes[std::size_t(index)];
    updating_ = true;
    resolutionUnit_->setCurrentIndex(0);
    resolution_->setValue(preset.ppi);
    updating_ = false;
    setPixels(preset.width, preset.height, preset.unit);
}

void NewDocumentDialog::unitChosen(int unit)
{
    if (updating_) {
        return;
    }
    // Restate the exact pixels, not the rounded field, in the new unit.
    const double per = pixelsPerUnit(unit);
    const double width = pixelWidth_;
    const double height = pixelHeight_;
    setPixels(width / per, height / per, unit);
    pixelWidth_ = width;
    pixelHeight_ = height;
    updateImageSize();
}

void NewDocumentDialog::resolutionUnitChosen(int unit)
{
    if (updating_) {
        return;
    }
    // The same resolution restated per centimeter or per inch.
    updating_ = true;
    // The field still holds the old unit: pixels/inch when switching to
    // per-centimeter, pixels/cm when switching back.
    const double ppiValue = unit == 1 ? resolution_->value() : resolution_->value() * 2.54;
    resolution_->setDecimals(unit == 1 ? 3 : 0);
    resolution_->setValue(unit == 1 ? ppiValue / 2.54 : ppiValue);
    updating_ = false;
    updateImageSize();
}

void NewDocumentDialog::dimensionEdited()
{
    if (updating_) {
        return;
    }
    if (documentType_->currentIndex() != kCustomType) {
        updating_ = true;
        documentType_->setCurrentIndex(kCustomType);
        size_->clear();
        size_->setEnabled(false);
        updating_ = false;
    }
    pixelWidth_ = width_->value() * pixelsPerUnit(unit_);
    pixelHeight_ = height_->value() * pixelsPerUnit(unit_);
    updateImageSize();
}

void NewDocumentDialog::modeChosen()
{
    // Bitmap is 1-bit only and 1-bit is Bitmap's alone; 32-bit is for RGB and
    // Grayscale.
    const QString mode = mode_->currentData().toString();
    const auto allowed = [&mode](int bits) {
        if (mode == QLatin1String("bitmap")) {
            return bits == 1;
        }
        return bits == 8 || bits == 16
            || (bits == 32 && (mode == QLatin1String("rgb") || mode == QLatin1String("grayscale")));
    };
    enableItems(depth_, [this, &allowed](int i) { return allowed(depth_->itemData(i).toInt()); });
    if (!allowed(depth_->currentData().toInt())) {
        depth_->setCurrentIndex(depth_->findData(mode == QLatin1String("bitmap") ? 1 : 8));
    }
    updateImageSize();
}

void NewDocumentDialog::backgroundChosen()
{
    if (contents_->currentText() != kCustom) {
        return;
    }
    const QColor picked = ColorPickerDialog::getColor(
        custom_, this, QStringLiteral("Color Picker (Custom Background Color)"));
    if (picked.isValid()) {
        custom_ = picked;
    } else {
        contents_->setCurrentIndex(contents_->findText(kWhite));
    }
}

void NewDocumentDialog::updateImageSize()
{
    const int index = qBound(0, mode_->currentIndex(), int(std::size(kModes)) - 1);
    const double bits = double(kModes[index].channels) * depth_->currentData().toInt();
    imageSize_->setText(formatSize(double(widthPixels()) * heightPixels() * bits / 8.0));
}

NewDocumentSpec NewDocumentDialog::spec() const
{
    NewDocumentSpec result;
    result.name = name_->text();
    result.width = widthPixels();
    result.height = heightPixels();
    result.mode = mode_->currentData().toString();
    result.depth = depth_->currentData().toInt();
    const QString contents = contents_->currentText();
    result.transparent = contents == kTransparent;
    result.fill = contents == kBlack        ? QColor(Qt::black)
                  : contents == kBackground ? background_
                  : contents == kCustom     ? custom_
                                            : QColor(Qt::white);
    result.ppi = ppi();
    result.perCm = resolutionUnit_->currentIndex() == 1;
    return result;
}

void NewDocumentDialog::accept()
{
    if (widthPixels() > kLargeSide || heightPixels() > kLargeSide) {
        QMessageBox box(QMessageBox::Warning, QStringLiteral("New"),
                        QStringLiteral("Documents greater than 30,000 pixels in either dimension "
                                       "may not be compatible with other applications."),
                        QMessageBox::NoButton, this);
        QPushButton* proceed = box.addButton(QStringLiteral("Continue"), QMessageBox::AcceptRole);
        box.addButton(QMessageBox::Cancel);
        runDialog(box, this);
        if (box.clickedButton() != proceed) {
            return;
        }
    }
    QDialog::accept();
}

QWidget* NewDocumentDialog::controlForTest(const QString& name) const
{
    return findChild<QWidget*>(name);
}

bool NewDocumentDialog::get(QWidget* parent, const QString& name, const QColor& background,
                            NewDocumentSpec* out)
{
    NewDocumentDialog dialog(name, background, parent);
    if (runDialog(dialog, parent) != QDialog::Accepted) {
        return false;
    }
    *out = dialog.spec();
    return true;
}

} // namespace pictura

#include "moc_new_document_dialog.cpp"
