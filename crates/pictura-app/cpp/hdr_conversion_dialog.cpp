#include "hdr_conversion_dialog.h"

#include "dialogs.h"
#include "hdr_toning_controls.h"

#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

QLabel* note(const QString& text, QWidget* parent)
{
    auto* label = new QLabel(text, parent);
    label->setAlignment(Qt::AlignCenter);
    label->setWordWrap(true);
    return label;
}

} // namespace

HdrConversionDialog::HdrConversionDialog(QWidget* parent)
    : QDialog(parent)
{
    setWindowTitle(QStringLiteral("HDR Conversion"));

    method_ = new QComboBox(this);
    method_->setObjectName(QStringLiteral("hdrConversionMethod"));
    method_->addItem(QStringLiteral("Local Adaptation"));
    method_->addItem(QStringLiteral("Equalize Histogram"));
    method_->addItem(QStringLiteral("Exposure and Gamma"));
    method_->addItem(QStringLiteral("Highlight Compression"));
    method_->setCurrentIndex(0);

    pages_ = new QStackedWidget(this);
    pages_->setObjectName(QStringLiteral("hdrConversionPages"));

    local_ = new HdrToningControls(pages_);
    local_->setValues(HdrToningControls::Values{});
    pages_->addWidget(local_);
    pages_->addWidget(note(QStringLiteral("Automatic: compresses dynamic range while preserving contrast."),
                           pages_));

    auto* exposurePage = new QWidget(pages_);
    auto* exposureForm = new QFormLayout(exposurePage);
    exposureSpin_ = new QDoubleSpinBox(exposurePage);
    exposureSpin_->setObjectName(QStringLiteral("hdrConversionExposure"));
    exposureSpin_->setRange(-20.0, 20.0);
    exposureSpin_->setDecimals(2);
    exposureSpin_->setSingleStep(0.1);
    exposureSpin_->setValue(0.0);
    gammaSpin_ = new QDoubleSpinBox(exposurePage);
    gammaSpin_->setObjectName(QStringLiteral("hdrConversionGamma"));
    gammaSpin_->setRange(0.01, 10.0);
    gammaSpin_->setDecimals(2);
    gammaSpin_->setSingleStep(0.1);
    gammaSpin_->setValue(1.0);
    exposureForm->addRow(QStringLiteral("Exposure (EV):"), exposureSpin_);
    exposureForm->addRow(QStringLiteral("Gamma:"), gammaSpin_);
    pages_->addWidget(exposurePage);

    pages_->addWidget(note(QStringLiteral("Automatic: compresses highlights into the output range."),
                           pages_));
    pages_->setCurrentIndex(0);

    connect(method_, qOverload<int>(&QComboBox::currentIndexChanged), pages_,
            &QStackedWidget::setCurrentIndex);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &HdrConversionDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &HdrConversionDialog::reject);

    auto* outer = new QVBoxLayout(this);
    auto* methodRow = new QFormLayout;
    methodRow->addRow(QStringLiteral("Method:"), method_);
    outer->addLayout(methodRow);
    outer->addWidget(pages_, 1);
    outer->addWidget(buttons);
}

HdrConversionSpec HdrConversionDialog::spec() const
{
    HdrConversionSpec result;
    result.method = HdrConversionMethod(method_->currentIndex());
    result.exposure_ev = exposureSpin_->value();
    result.gamma = gammaSpin_->value();
    const auto local = local_->values();
    result.radius = local.radius;
    result.strength = local.strength;
    result.local_gamma = local.gamma;
    result.local_exposure = local.exposure;
    result.detail = local.detail;
    result.shadow = local.shadow;
    result.highlight = local.highlight;
    result.vibrance = local.vibrance;
    result.saturation = local.saturation;
    return result;
}

QWidget* HdrConversionDialog::controlForTest(const QString& name) const
{
    return findChild<QWidget*>(name);
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
