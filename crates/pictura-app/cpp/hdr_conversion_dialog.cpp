#include "hdr_conversion_dialog.h"
#include "dialogs.h"

#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QFormLayout>

namespace pictura {

HdrConversionDialog::HdrConversionDialog(QWidget* parent)
    : QDialog(parent)
{
    setWindowTitle(QStringLiteral("HDR Conversion"));

    // ponytail: the ranges beyond Exposure 0 / Gamma 1.0 are inferred; CS6's
    // documented method is Exposure and Gamma, nothing else.
    exposureSpin_ = new QDoubleSpinBox(this);
    exposureSpin_->setRange(-20.0, 20.0);
    exposureSpin_->setDecimals(2);
    exposureSpin_->setSingleStep(0.1);
    exposureSpin_->setValue(0.0);

    gammaSpin_ = new QDoubleSpinBox(this);
    gammaSpin_->setRange(0.01, 10.0);
    gammaSpin_->setDecimals(2);
    gammaSpin_->setSingleStep(0.1);
    gammaSpin_->setValue(1.0);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &HdrConversionDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &HdrConversionDialog::reject);

    auto* form = new QFormLayout(this);
    form->addRow(QStringLiteral("Exposure (EV):"), exposureSpin_);
    form->addRow(QStringLiteral("Gamma:"), gammaSpin_);
    form->addRow(buttons);
}

HdrConversionSpec HdrConversionDialog::spec() const
{
    HdrConversionSpec result;
    result.exposure_ev = exposureSpin_->value();
    result.gamma = gammaSpin_->value();
    return result;
}

bool HdrConversionDialog::get(QWidget* parent, HdrConversionSpec* out)
{
    HdrConversionDialog dialog(parent);
    if (runDialog(dialog, parent) != QDialog::Accepted) {
        return false;
    }
    *out = dialog.spec();
    return true;
}

} // namespace pictura

#include "moc_hdr_conversion_dialog.cpp"
