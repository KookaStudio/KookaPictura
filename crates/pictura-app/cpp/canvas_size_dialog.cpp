#include "canvas_size_dialog.h"

#include "color_picker_dialog.h"
#include "image_size_dialog.h"
#include "panels/color_swatch_widget.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust/image_size.cxxqt.h"

#include <QtCore/QSignalBlocker>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPainterPath>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include <cmath>

namespace pictura {

namespace {

// Width/Height units, in CS6's order.
enum Unit { UnitPercent = 0, UnitPixels, UnitInches, UnitCm, UnitMm, UnitPoints };

constexpr int kCell = 26;

const QString kForeground = QStringLiteral("Foreground");
const QString kBackground = QStringLiteral("Background");
const QString kWhite = QStringLiteral("White");
const QString kBlack = QStringLiteral("Black");
const QString kGray = QStringLiteral("Gray");
const QString kOther = QStringLiteral("Other…");

} // namespace

// --- AnchorSelector ---------------------------------------------------------

AnchorSelector::AnchorSelector(QWidget* parent)
    : QWidget(parent)
{
    setObjectName(QStringLiteral("canvasSizeAnchor"));
    setCursor(Qt::PointingHandCursor);
    setFixedSize(sizeHint());
}

QSize AnchorSelector::sizeHint() const
{
    return {kCell * 3, kCell * 3};
}

void AnchorSelector::setAnchor(int x, int y)
{
    x_ = qBound(0, x, 2);
    y_ = qBound(0, y, 2);
    update();
}

QString AnchorSelector::anchorName() const
{
    static const char* const rows[] = {"top", "center", "bottom"};
    static const char* const columns[] = {"left", "center", "right"};
    if (x_ == 1 && y_ == 1) {
        return QStringLiteral("center");
    }
    return QStringLiteral("%1-%2").arg(QLatin1String(rows[y_]), QLatin1String(columns[x_]));
}

QRect AnchorSelector::cellRect(int cx, int cy) const
{
    return {cx * kCell, cy * kCell, kCell, kCell};
}

void AnchorSelector::paintEvent(QPaintEvent*)
{
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing);
    const QColor ink = palette().color(QPalette::WindowText);
    const QColor frame = palette().color(QPalette::Mid);
    for (int cy = 0; cy < 3; ++cy) {
        for (int cx = 0; cx < 3; ++cx) {
            const QRect cell = cellRect(cx, cy).adjusted(1, 1, -1, -1);
            painter.setPen(QPen(frame, 1.0));
            painter.setBrush(Qt::NoBrush);
            painter.drawRect(cell);
            const QPointF centre = QRectF(cell).center();
            if (cx == x_ && cy == y_) {
                // The anchored square holds the image.
                painter.setPen(Qt::NoPen);
                painter.setBrush(ink);
                painter.drawRect(QRectF(centre.x() - 4, centre.y() - 4, 8, 8));
                continue;
            }
            // The squares next to the anchor point away from it, the way the
            // canvas grows.
            const int dx = cx - x_;
            const int dy = cy - y_;
            if (std::abs(dx) > 1 || std::abs(dy) > 1) {
                continue;
            }
            const double length = std::hypot(double(dx), double(dy));
            const QPointF dir(dx / length, dy / length);
            const QPointF tip = centre + dir * 7.0;
            const QPointF tail = centre - dir * 6.0;
            const QPointF normal(-dir.y(), dir.x());
            painter.setPen(QPen(ink, 1.4, Qt::SolidLine, Qt::RoundCap));
            painter.drawLine(tail, tip - dir * 3.0);
            QPainterPath head;
            head.moveTo(tip);
            head.lineTo(tip - dir * 5.0 + normal * 3.0);
            head.lineTo(tip - dir * 5.0 - normal * 3.0);
            head.closeSubpath();
            painter.setPen(Qt::NoPen);
            painter.setBrush(ink);
            painter.drawPath(head);
        }
    }
}

void AnchorSelector::mousePressEvent(QMouseEvent* event)
{
    const int cx = int(event->position().x()) / kCell;
    const int cy = int(event->position().y()) / kCell;
    if (cx >= 0 && cx <= 2 && cy >= 0 && cy <= 2) {
        setAnchor(cx, cy);
    }
}

// --- CanvasSizeDialog -------------------------------------------------------

CanvasSizeDialog::CanvasSizeDialog(PictureView* view, const QColor& foreground,
                                   const QColor& background, QWidget* parent)
    : QDialog(parent)
    , foreground_(foreground)
    , background_(background)
{
    setObjectName(QStringLiteral("canvasSizeDialog"));
    setWindowTitle(QStringLiteral("Canvas Size"));
    if (view) {
        pixelWidth_ = qMax(1, int(view->document_width()));
        pixelHeight_ = qMax(1, int(view->document_height()));
        resolution_ = document_ppi(*view);
    }
    bytesPerPixel_ = bytesPerPixel(view);
    buildUi();
    updateSizes();
}

double CanvasSizeDialog::unitScale(int unit) const
{
    switch (unit) {
    case UnitInches:
        return resolution_;
    case UnitCm:
        return resolution_ / 2.54;
    case UnitMm:
        return resolution_ / 25.4;
    case UnitPoints:
        return resolution_ / 72.0;
    default:
        return 1.0;
    }
}

int CanvasSizeDialog::toPixels(const QDoubleSpinBox* field, int unit, int base) const
{
    // A percentage is of the current size either way; Relative decides
    // whether the result is added to it or replaces it.
    return unit == UnitPercent ? qRound(base * field->value() / 100.0)
                               : qRound(field->value() * unitScale(unit));
}

int CanvasSizeDialog::resultWidth() const
{
    const int v = toPixels(width_, widthUnit_->currentIndex(), pixelWidth_);
    return qMax(1, relative_->isChecked() ? pixelWidth_ + v : v);
}

int CanvasSizeDialog::resultHeight() const
{
    const int v = toPixels(height_, heightUnit_->currentIndex(), pixelHeight_);
    return qMax(1, relative_->isChecked() ? pixelHeight_ + v : v);
}

QString CanvasSizeDialog::anchorName() const
{
    return anchor_->anchorName();
}

QColor CanvasSizeDialog::extensionColor() const
{
    const QString choice = extension_->currentText();
    if (choice == kForeground) {
        return foreground_;
    }
    if (choice == kBackground) {
        return background_;
    }
    if (choice == kWhite) {
        return Qt::white;
    }
    if (choice == kBlack) {
        return Qt::black;
    }
    if (choice == kGray) {
        return QColor(128, 128, 128);
    }
    return customColor_;
}

void CanvasSizeDialog::buildUi()
{
    auto* outer = new QHBoxLayout(this);
    outer->setContentsMargins(12, 12, 12, 12);
    outer->setSpacing(14);
    auto* left = new QVBoxLayout;

    auto* currentBox = new QGroupBox(this);
    auto* currentGrid = new QGridLayout(currentBox);
    currentSize_ = new QLabel(this);
    currentSize_->setObjectName(QStringLiteral("canvasSizeCurrent"));
    currentGrid->addWidget(currentSize_, 0, 0, 1, 2);
    currentGrid->addWidget(new QLabel(QStringLiteral("Width:"), this), 1, 0, Qt::AlignRight);
    currentWidth_ = new QLabel(this);
    currentGrid->addWidget(currentWidth_, 1, 1);
    currentGrid->addWidget(new QLabel(QStringLiteral("Height:"), this), 2, 0, Qt::AlignRight);
    currentHeight_ = new QLabel(this);
    currentGrid->addWidget(currentHeight_, 2, 1);
    currentGrid->setColumnStretch(1, 1);
    left->addWidget(currentBox);

    auto* newBox = new QGroupBox(this);
    auto* newGrid = new QGridLayout(newBox);
    newSize_ = new QLabel(this);
    newSize_->setObjectName(QStringLiteral("canvasSizeNew"));
    newGrid->addWidget(newSize_, 0, 0, 1, 3);
    const auto addRow = [&](int row, const QString& label, const QString& name,
                            QDoubleSpinBox*& field, QComboBox*& unit) {
        newGrid->addWidget(new QLabel(label, this), row, 0, Qt::AlignRight);
        field = new QDoubleSpinBox(this);
        field->setObjectName(name);
        field->setRange(-300000.0, 300000.0);
        field->setDecimals(0);
        field->setMinimumWidth(90);
        newGrid->addWidget(field, row, 1);
        unit = new QComboBox(this);
        unit->setObjectName(name + QStringLiteral("Unit"));
        unit->addItems({QStringLiteral("Percent"), QStringLiteral("Pixels"),
                        QStringLiteral("Inches"), QStringLiteral("Centimeters"),
                        QStringLiteral("Millimeters"), QStringLiteral("Points")});
        newGrid->addWidget(unit, row, 2);
    };
    addRow(1, QStringLiteral("Width:"), QStringLiteral("canvasSizeWidth"), width_, widthUnit_);
    addRow(2, QStringLiteral("Height:"), QStringLiteral("canvasSizeHeight"), height_, heightUnit_);
    relative_ = new QCheckBox(QStringLiteral("Relative"), this);
    relative_->setObjectName(QStringLiteral("canvasSizeRelative"));
    newGrid->addWidget(relative_, 3, 1);
    newGrid->addWidget(new QLabel(QStringLiteral("Anchor:"), this), 4, 0,
                       Qt::AlignRight | Qt::AlignTop);
    anchor_ = new AnchorSelector(this);
    newGrid->addWidget(anchor_, 4, 1, 1, 2, Qt::AlignLeft);
    left->addWidget(newBox);

    auto* extensionRow = new QHBoxLayout;
    extensionRow->addWidget(new QLabel(QStringLiteral("Canvas extension color:"), this));
    extension_ = new QComboBox(this);
    extension_->setObjectName(QStringLiteral("canvasSizeExtension"));
    extension_->addItems({kForeground, kBackground});
    extension_->insertSeparator(extension_->count());
    extension_->addItems({kWhite, kBlack, kGray});
    extension_->insertSeparator(extension_->count());
    extension_->addItem(kOther);
    extension_->setCurrentIndex(extension_->findText(kBackground));
    extensionRow->addWidget(extension_, 1);
    // A button, not a readout: in CS6 the swatch opens the colour picker
    // whatever the menu says.
    swatch_ = new ColorSwatchWidget(this);
    swatch_->setObjectName(QStringLiteral("canvasSizeSwatch"));
    swatch_->setFixedSize(24, 22);
    swatch_->setToolTip(QStringLiteral("Choose the canvas extension color"));
    extensionRow->addWidget(swatch_);
    left->addLayout(extensionRow);
    left->addStretch();
    outer->addLayout(left, 1);

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
    connect(width_, &QDoubleSpinBox::valueChanged, this, &CanvasSizeDialog::updateSizes);
    connect(height_, &QDoubleSpinBox::valueChanged, this, &CanvasSizeDialog::updateSizes);
    connect(widthUnit_, &QComboBox::currentIndexChanged, this,
            [this](int unit) { changeUnit(width_, unit, widthUnitPrevious_, pixelWidth_); });
    connect(heightUnit_, &QComboBox::currentIndexChanged, this,
            [this](int unit) { changeUnit(height_, unit, heightUnitPrevious_, pixelHeight_); });
    connect(relative_, &QCheckBox::toggled, this, &CanvasSizeDialog::relativeToggled);
    connect(extension_, &QComboBox::currentIndexChanged, this, &CanvasSizeDialog::extensionChosen);
    connect(swatch_, &QPushButton::clicked, this, &CanvasSizeDialog::pickExtensionColor);

    {
        const QSignalBlocker bw(widthUnit_);
        const QSignalBlocker bh(heightUnit_);
        widthUnit_->setCurrentIndex(UnitPixels);
        heightUnit_->setCurrentIndex(UnitPixels);
    }
    widthUnitPrevious_ = UnitPixels;
    heightUnitPrevious_ = UnitPixels;
    width_->setValue(pixelWidth_);
    height_->setValue(pixelHeight_);
    updateSwatch();
}

void CanvasSizeDialog::changeUnit(QDoubleSpinBox* field, int unit, int& previous, int base)
{
    const int pixels = toPixels(field, previous, base);
    previous = unit;
    const QSignalBlocker block(field);
    field->setDecimals(unit == UnitPixels ? 0 : 2);
    field->setValue(unit == UnitPercent ? (base > 0 ? pixels * 100.0 / base : 0.0)
                                        : pixels / unitScale(unit));
    updateSizes();
}

void CanvasSizeDialog::relativeToggled(bool on)
{
    // Relative counts from the current size, so the fields start at "no
    // change"; back to absolute they restate the current canvas.
    const QSignalBlocker bw(width_);
    const QSignalBlocker bh(height_);
    const auto restore = [this, on](QDoubleSpinBox* field, int unit, int base) {
        field->setValue(on ? 0.0 : unit == UnitPercent ? 100.0 : base / unitScale(unit));
    };
    restore(width_, widthUnit_->currentIndex(), pixelWidth_);
    restore(height_, heightUnit_->currentIndex(), pixelHeight_);
    updateSizes();
}

void CanvasSizeDialog::extensionChosen()
{
    if (extension_->currentText() == kOther) {
        pickExtensionColor();
        return;
    }
    updateSwatch();
}

void CanvasSizeDialog::pickExtensionColor()
{
    const QColor picked = ColorPickerDialog::getColor(extensionColor(), this,
                                                      QStringLiteral("Canvas Extension Color"));
    if (picked.isValid()) {
        customColor_ = picked;
        const QSignalBlocker block(extension_);
        extension_->setCurrentIndex(extension_->findText(kOther));
    }
    updateSwatch();
}

void CanvasSizeDialog::updateSwatch()
{
    swatch_->setColor(extensionColor());
}

void CanvasSizeDialog::updateSizes()
{
    currentSize_->setText(QStringLiteral("Current Size: %1")
                              .arg(imageSizeSummary(bytesPerPixel_ * pixelWidth_ * pixelHeight_)));
    currentWidth_->setText(QStringLiteral("%1 Pixels").arg(pixelWidth_));
    currentHeight_->setText(QStringLiteral("%1 Pixels").arg(pixelHeight_));
    newSize_->setText(QStringLiteral("New Size: %1")
                          .arg(imageSizeSummary(bytesPerPixel_ * resultWidth() * resultHeight())));
}

QWidget* CanvasSizeDialog::controlForTest(const QString& name) const
{
    return findChild<QWidget*>(name);
}

} // namespace pictura
