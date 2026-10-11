#include "new_guide_dialog.h"

#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

NewGuideDialog::NewGuideDialog(QWidget* parent)
    : QDialog(parent)
{
    setObjectName(QStringLiteral("newGuideDialog"));
    setWindowTitle(tr("New Guide"));
    auto* layout = new QVBoxLayout(this);
    auto* orientation = new QGroupBox(tr("Orientation"), this);
    auto* orientationLayout = new QVBoxLayout(orientation);
    horizontal_ = new QRadioButton(tr("Horizontal"), orientation);
    vertical_ = new QRadioButton(tr("Vertical"), orientation);
    horizontal_->setChecked(true);
    orientationLayout->addWidget(horizontal_);
    orientationLayout->addWidget(vertical_);
    layout->addWidget(orientation);

    auto* form = new QFormLayout;
    position_ = new QDoubleSpinBox(this);
    position_->setObjectName(QStringLiteral("newGuidePosition"));
    position_->setRange(-300000.0, 300000.0);
    position_->setDecimals(0);
    position_->setSuffix(tr(" px"));
    form->addRow(tr("Position:"), position_);
    layout->addLayout(form);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    layout->addWidget(buttons);
}

bool NewGuideDialog::vertical() const
{
    return vertical_->isChecked();
}

double NewGuideDialog::position() const
{
    return position_->value();
}

void NewGuideDialog::setGuide(bool vertical, double position)
{
    (vertical ? vertical_ : horizontal_)->setChecked(true);
    position_->setValue(position);
}

} // namespace pictura
