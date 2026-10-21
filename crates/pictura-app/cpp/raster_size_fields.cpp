#include "raster_size_fields.h"

#include "image_size_dialog.h"

#include <QtCore/QSignalBlocker>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QLabel>

#include <algorithm>
#include <cmath>
#include <initializer_list>

namespace pictura {

namespace {

enum Unit { UnitPixels = 0, UnitInches, UnitCm, UnitMm, UnitPoints, UnitPicas };

const char* const kModes[][2] = {
    {"Grayscale", "grayscale"},
    {"RGB Color", "rgb"},
    {"CMYK Color", "cmyk"},
    {"Lab Color", "lab"},
};
constexpr int kDefaultMode = 1;

int channelsFor(const QString& mode)
{
    if (mode == QStringLiteral("grayscale")) {
        return 1;
    }
    return mode == QStringLiteral("cmyk") ? 4 : 3;
}

QComboBox* unitBox(QWidget* parent, const QString& name)
{
    auto* box = new QComboBox(parent);
    box->setObjectName(name);
    box->addItems({QStringLiteral("Pixels"), QStringLiteral("Inches"),
                   QStringLiteral("Centimeters"), QStringLiteral("Millimeters"),
                   QStringLiteral("Points"), QStringLiteral("Picas")});
    box->setCurrentIndex(UnitInches);
    return box;
}

QDoubleSpinBox* sizeBox(QWidget* parent, const QString& name)
{
    auto* box = new QDoubleSpinBox(parent);
    box->setObjectName(name);
    box->setRange(0.001, 300000.0);
    box->setDecimals(3);
    box->setKeyboardTracking(false);
    return box;
}

} // namespace

RasterSizeFields::RasterSizeFields(bool withBitDepth, QWidget* parent)
    : QGroupBox(parent)
{
    setObjectName(QStringLiteral("rasterSizeFields"));
    auto* grid = new QGridLayout(this);

    width_ = sizeBox(this, QStringLiteral("width"));
    height_ = sizeBox(this, QStringLiteral("height"));
    widthUnit_ = unitBox(this, QStringLiteral("widthUnit"));
    heightUnit_ = unitBox(this, QStringLiteral("heightUnit"));
    grid->addWidget(new QLabel(tr("Width:"), this), 0, 0, Qt::AlignRight);
    grid->addWidget(width_, 0, 1);
    grid->addWidget(widthUnit_, 0, 2);
    grid->addWidget(new QLabel(tr("Height:"), this), 1, 0, Qt::AlignRight);
    grid->addWidget(height_, 1, 1);
    grid->addWidget(heightUnit_, 1, 2);

    constrain_ = new QCheckBox(tr("Constrain Proportions"), this);
    constrain_->setObjectName(QStringLiteral("constrain"));
    constrain_->setChecked(true);
    grid->addWidget(constrain_, 2, 1, 1, 2);

    resolution_ = new QDoubleSpinBox(this);
    resolution_->setObjectName(QStringLiteral("resolution"));
    resolution_->setRange(1.0, 30000.0);
    resolution_->setDecimals(0);
    resolution_->setKeyboardTracking(false);
    resolutionUnit_ = new QComboBox(this);
    resolutionUnit_->setObjectName(QStringLiteral("resolutionUnit"));
    resolutionUnit_->addItems({QStringLiteral("Pixels/Inch"), QStringLiteral("Pixels/Centimeter")});
    grid->addWidget(new QLabel(tr("Resolution:"), this), 3, 0, Qt::AlignRight);
    grid->addWidget(resolution_, 3, 1);
    grid->addWidget(resolutionUnit_, 3, 2);

    mode_ = new QComboBox(this);
    mode_->setObjectName(QStringLiteral("mode"));
    for (const auto& entry : kModes) {
        mode_->addItem(tr(entry[0]), QString::fromLatin1(entry[1]));
    }
    mode_->setCurrentIndex(kDefaultMode);
    grid->addWidget(new QLabel(tr("Mode:"), this), 4, 0, Qt::AlignRight);
    grid->addWidget(mode_, 4, 1, 1, 2);

    if (withBitDepth) {
        depth_ = new QComboBox(this);
        depth_->setObjectName(QStringLiteral("depth"));
        depth_->addItem(tr("8 bit"), 8);
        depth_->addItem(tr("16 bit"), 16);
        grid->addWidget(new QLabel(tr("Bit Depth:"), this), 5, 0, Qt::AlignRight);
        grid->addWidget(depth_, 5, 1);
    }

    connect(width_, &QDoubleSpinBox::valueChanged, this, [this]() { widthEdited(); });
    connect(height_, &QDoubleSpinBox::valueChanged, this, [this]() { heightEdited(); });
    connect(resolution_, &QDoubleSpinBox::valueChanged, this, [this]() { resolutionEdited(); });
    for (QComboBox* box :
         std::initializer_list<QComboBox*>{widthUnit_, heightUnit_, resolutionUnit_, mode_, depth_}) {
        if (box) {
            connect(box, &QComboBox::currentIndexChanged, this, [this]() { sync(); });
        }
    }
    connect(constrain_, &QCheckBox::toggled, this, [this](bool on) {
        // Re-binding keeps Width and recomputes Height from the box's ratio.
        if (on && source_.width() > 0) {
            heightIn_ = widthIn_ * source_.height() / source_.width();
        }
        sync();
    });
    setSourceSize(source_);
}

void RasterSizeFields::setSourceSize(QSizeF points)
{
    if (points.width() <= 0 || points.height() <= 0) {
        return;
    }
    source_ = points;
    widthIn_ = points.width() / 72.0;
    heightIn_ = points.height() / 72.0;
    sync();
}

QSize RasterSizeFields::pixelSize() const
{
    return {std::max(1, int(std::lround(widthIn_ * ppi_))),
            std::max(1, int(std::lround(heightIn_ * ppi_)))};
}

bool RasterSizeFields::perCm() const
{
    return resolutionUnit_->currentIndex() == 1;
}

QString RasterSizeFields::mode() const
{
    return mode_->currentData().toString();
}

int RasterSizeFields::depth() const
{
    return depth_ ? depth_->currentData().toInt() : 8;
}

void RasterSizeFields::setSizeEnabled(bool on)
{
    for (QWidget* w : std::initializer_list<QWidget*>{width_, height_, widthUnit_, heightUnit_,
                                                      constrain_, resolution_, resolutionUnit_}) {
        w->setEnabled(on);
    }
}

double RasterSizeFields::inchesPerUnit(int unit) const
{
    switch (unit) {
    case UnitPixels:
        return 1.0 / ppi_;
    case UnitCm:
        return 1.0 / 2.54;
    case UnitMm:
        return 1.0 / 25.4;
    case UnitPoints:
        return 1.0 / 72.0;
    case UnitPicas:
        return 1.0 / 6.0;
    default:
        return 1.0;
    }
}

void RasterSizeFields::sync()
{
    updating_ = true;
    const QSignalBlocker w(width_);
    const QSignalBlocker h(height_);
    const QSignalBlocker r(resolution_);
    width_->setDecimals(widthUnit_->currentIndex() == UnitPixels ? 0 : 3);
    height_->setDecimals(heightUnit_->currentIndex() == UnitPixels ? 0 : 3);
    const QSize pixels = pixelSize();
    width_->setValue(widthUnit_->currentIndex() == UnitPixels
                         ? pixels.width()
                         : widthIn_ / inchesPerUnit(widthUnit_->currentIndex()));
    height_->setValue(heightUnit_->currentIndex() == UnitPixels
                          ? pixels.height()
                          : heightIn_ / inchesPerUnit(heightUnit_->currentIndex()));
    resolution_->setValue(perCm() ? ppi_ / 2.54 : ppi_);
    const double bytes = double(pixels.width()) * pixels.height() * channelsFor(mode())
                         * (depth() == 16 ? 2 : 1);
    setTitle(tr("Image Size: %1").arg(imageSizeSummary(bytes)));
    updating_ = false;
    emit changed();
}

void RasterSizeFields::widthEdited()
{
    if (updating_) {
        return;
    }
    widthIn_ = width_->value() * inchesPerUnit(widthUnit_->currentIndex());
    if (constrain_->isChecked() && source_.width() > 0) {
        heightIn_ = widthIn_ * source_.height() / source_.width();
    }
    sync();
}

void RasterSizeFields::heightEdited()
{
    if (updating_) {
        return;
    }
    heightIn_ = height_->value() * inchesPerUnit(heightUnit_->currentIndex());
    if (constrain_->isChecked() && source_.height() > 0) {
        widthIn_ = heightIn_ * source_.width() / source_.height();
    }
    sync();
}

void RasterSizeFields::resolutionEdited()
{
    if (updating_) {
        return;
    }
    ppi_ = perCm() ? resolution_->value() * 2.54 : resolution_->value();
    sync();
}

void RasterSizeFields::setFieldForTest(const QString& name, const QString& value)
{
    if (auto* spin = findChild<QDoubleSpinBox*>(name)) {
        spin->setValue(value.toDouble());
    } else if (auto* box = findChild<QComboBox*>(name)) {
        const int byText = box->findText(value);
        box->setCurrentIndex(byText >= 0 ? byText : value.toInt());
    } else if (auto* check = findChild<QCheckBox*>(name)) {
        check->setChecked(value == QStringLiteral("1"));
    }
}

QString RasterSizeFields::fieldForTest(const QString& name) const
{
    if (auto* spin = findChild<QDoubleSpinBox*>(name)) {
        return QString::number(spin->value(), 'f', spin->decimals());
    }
    if (auto* box = findChild<QComboBox*>(name)) {
        return box->currentText();
    }
    if (auto* check = findChild<QCheckBox*>(name)) {
        return check->isChecked() ? QStringLiteral("1") : QStringLiteral("0");
    }
    return name == QStringLiteral("title") ? title() : QString();
}

} // namespace pictura

#include "moc_raster_size_fields.cpp"
