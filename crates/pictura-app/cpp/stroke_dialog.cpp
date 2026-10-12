#include "stroke_dialog.h"

#include "color_picker_dialog.h"
#include "panels/color_swatch_widget.h"

#include <QtCore/QEvent>
#include <QtWidgets/QButtonGroup>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

StrokeDialog::StrokeDialog(const QColor& initial, QWidget* parent)
    : QDialog(parent)
    , color_(initial)
{
    setWindowTitle(tr("Stroke"));
    setFixedSize(400, 330);

    auto* outer = new QHBoxLayout(this);
    auto* left = new QVBoxLayout;

    auto* strokeGroup = new QGroupBox(tr("Stroke"), this);
    auto* strokeLayout = new QVBoxLayout(strokeGroup);

    auto* widthRow = new QHBoxLayout;
    auto* widthLabel = new QLabel(tr("Width:"), strokeGroup);
    widthLabel->setFixedWidth(50);
    widthLabel->setAlignment(Qt::AlignRight | Qt::AlignVCenter);
    widthRow->addWidget(widthLabel);
    width_ = new QSpinBox(strokeGroup);
    width_->setObjectName(QStringLiteral("strokeWidth"));
    width_->setRange(1, 250);
    width_->setValue(1);
    width_->setSuffix(QStringLiteral(" px"));
    width_->setFixedWidth(80);
    widthRow->addWidget(width_);
    widthRow->addStretch();
    strokeLayout->addLayout(widthRow);

    auto* colorRow = new QHBoxLayout;
    auto* colorLabel = new QLabel(tr("Color:"), strokeGroup);
    colorLabel->setFixedWidth(50);
    colorLabel->setAlignment(Qt::AlignRight | Qt::AlignVCenter);
    colorRow->addWidget(colorLabel);
    colorSwatch_ = new ColorSwatchWidget(strokeGroup);
    colorSwatch_->setObjectName(QStringLiteral("strokeColor"));
    colorSwatch_->setFixedSize(40, 24);
    colorSwatch_->setCursor(Qt::PointingHandCursor);
    colorSwatch_->installEventFilter(this);
    colorRow->addWidget(colorSwatch_);
    colorRow->addStretch();
    strokeLayout->addLayout(colorRow);
    left->addWidget(strokeGroup);

    auto* locationGroup = new QGroupBox(tr("Location"), this);
    auto* locationLayout = new QHBoxLayout(locationGroup);
    inside_ = new QRadioButton(tr("Inside"), locationGroup);
    inside_->setObjectName(QStringLiteral("strokeInside"));
    center_ = new QRadioButton(tr("Center"), locationGroup);
    center_->setChecked(true);
    outside_ = new QRadioButton(tr("Outside"), locationGroup);
    auto* group = new QButtonGroup(this);
    group->addButton(inside_, 0);
    group->addButton(center_, 1);
    group->addButton(outside_, 2);
    locationLayout->addWidget(inside_);
    locationLayout->addWidget(center_);
    locationLayout->addWidget(outside_);
    left->addWidget(locationGroup);

    auto* blendGroup = new QGroupBox(tr("Blending"), this);
    auto* blendLayout = new QVBoxLayout(blendGroup);
    auto* modeRow = new QHBoxLayout;
    auto* modeLabel = new QLabel(tr("Mode:"), blendGroup);
    modeLabel->setFixedWidth(55);
    modeLabel->setAlignment(Qt::AlignRight | Qt::AlignVCenter);
    modeRow->addWidget(modeLabel);
    mode_ = new QComboBox(blendGroup);
    mode_->setObjectName(QStringLiteral("strokeMode"));
    mode_->addItem(QStringLiteral("Normal"), QStringLiteral("normal"));
    mode_->addItem(QStringLiteral("Dissolve"), QStringLiteral("dissolve"));
    mode_->addItem(QStringLiteral("Behind"), QStringLiteral("behind"));
    mode_->addItem(QStringLiteral("Clear"), QStringLiteral("clear"));
    mode_->setMinimumWidth(140);
    modeRow->addWidget(mode_, 1);
    blendLayout->addLayout(modeRow);

    auto* opacityRow = new QHBoxLayout;
    auto* opacityLabel = new QLabel(tr("Opacity:"), blendGroup);
    opacityLabel->setFixedWidth(55);
    opacityLabel->setAlignment(Qt::AlignRight | Qt::AlignVCenter);
    opacityRow->addWidget(opacityLabel);
    opacity_ = new QSpinBox(blendGroup);
    opacity_->setObjectName(QStringLiteral("strokeOpacity"));
    opacity_->setRange(1, 100);
    opacity_->setValue(100);
    opacity_->setSuffix(QStringLiteral(" %"));
    opacity_->setFixedWidth(70);
    opacityRow->addWidget(opacity_);
    opacityRow->addStretch();
    blendLayout->addLayout(opacityRow);
    left->addWidget(blendGroup);

    preserveTransparency_ = new QCheckBox(tr("Preserve Transparency"), this);
    preserveTransparency_->setObjectName(QStringLiteral("strokePreserveTransparency"));
    left->addWidget(preserveTransparency_);
    left->addStretch();
    outer->addLayout(left, 1);

    auto* buttons = new QVBoxLayout;
    auto* ok = new QPushButton(tr("OK"), this);
    ok->setDefault(true);
    ok->setFixedWidth(80);
    auto* cancel = new QPushButton(tr("Cancel"), this);
    cancel->setFixedWidth(80);
    buttons->addWidget(ok);
    buttons->addWidget(cancel);
    buttons->addStretch();
    outer->addLayout(buttons);

    connect(ok, &QPushButton::clicked, this, &QDialog::accept);
    connect(cancel, &QPushButton::clicked, this, &QDialog::reject);
    updateColorSwatch();
}

int StrokeDialog::strokeWidth() const { return width_->value(); }

QColor StrokeDialog::strokeColor() const { return color_; }

int StrokeDialog::location() const
{
    if (inside_->isChecked()) {
        return 0;
    }
    if (outside_->isChecked()) {
        return 2;
    }
    return 1;
}

QString StrokeDialog::blendMode() const { return mode_->currentData().toString(); }

int StrokeDialog::opacity() const { return opacity_->value(); }

bool StrokeDialog::preserveTransparency() const { return preserveTransparency_->isChecked(); }

void StrokeDialog::openColorPicker()
{
    const QColor picked = ColorPickerDialog::getColor(color_, this, tr("Stroke Color"));
    if (picked.isValid()) {
        color_ = picked;
        updateColorSwatch();
    }
}

void StrokeDialog::updateColorSwatch()
{
    colorSwatch_->setColor(color_);
}

bool StrokeDialog::eventFilter(QObject* obj, QEvent* event)
{
    if (obj == colorSwatch_ && event->type() == QEvent::MouseButtonRelease) {
        openColorPicker();
        return true;
    }
    return QDialog::eventFilter(obj, event);
}

} // namespace pictura
