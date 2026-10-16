#include "mask_display_options_dialog.h"

#include "panels/jump_slider.h"

#include <QtGui/QPixmap>
#include <QtWidgets/QColorDialog>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

MaskDisplayOptionsDialog::MaskDisplayOptionsDialog(const QColor& color, int opacity,
                                                   QWidget* parent)
    : QDialog(parent)
{
    setWindowTitle(tr("Layer Mask Display Options"));
    setObjectName(QStringLiteral("maskDisplayOptionsDialog"));
    auto* root = new QVBoxLayout(this);

    auto* colorRow = new QHBoxLayout;
    colorRow->addWidget(new QLabel(tr("Color:"), this));
    swatch_ = new QToolButton(this);
    swatch_->setObjectName(QStringLiteral("maskDisplayColor"));
    swatch_->setIconSize(QSize(40, 18));
    swatch_->setToolTip(tr("Click to choose the overlay color"));
    colorRow->addWidget(swatch_);
    colorRow->addStretch(1);
    root->addLayout(colorRow);

    auto* opacityRow = new QHBoxLayout;
    opacityRow->addWidget(new QLabel(tr("Opacity:"), this));
    opacitySlider_ = new JumpSlider(Qt::Horizontal, this);
    opacitySlider_->setObjectName(QStringLiteral("maskDisplayOpacitySlider"));
    opacitySlider_->setRange(0, 100);
    opacityRow->addWidget(opacitySlider_, 1);
    opacity_ = new QSpinBox(this);
    opacity_->setObjectName(QStringLiteral("maskDisplayOpacity"));
    opacity_->setRange(0, 100);
    opacity_->setSuffix(QStringLiteral(" %"));
    opacityRow->addWidget(opacity_);
    root->addLayout(opacityRow);
    connect(opacitySlider_, &QSlider::valueChanged, opacity_, &QSpinBox::setValue);
    connect(opacity_, &QSpinBox::valueChanged, opacitySlider_, &QSlider::setValue);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    root->addWidget(buttons);

    connect(swatch_, &QToolButton::clicked, this, &MaskDisplayOptionsDialog::pickColor);
    setColor(color);
    setOpacity(opacity);
}

int MaskDisplayOptionsDialog::opacity() const
{
    return opacity_->value();
}

void MaskDisplayOptionsDialog::setColor(const QColor& color)
{
    color_ = color.isValid() ? color : QColor(Qt::red);
    QPixmap chip(swatch_->iconSize());
    chip.fill(color_);
    swatch_->setIcon(QIcon(chip));
}

void MaskDisplayOptionsDialog::setOpacity(int opacity)
{
    opacity_->setValue(qBound(0, opacity, 100));
}

void MaskDisplayOptionsDialog::pickColor()
{
    const QColor picked = QColorDialog::getColor(color_, this, tr("Select Overlay Color"));
    if (picked.isValid()) {
        setColor(picked);
    }
}

} // namespace pictura
