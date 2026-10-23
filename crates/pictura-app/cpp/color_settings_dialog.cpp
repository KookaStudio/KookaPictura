#include "color_settings_dialog.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

constexpr int kPreserve = 0;
constexpr int kConvert = 1;
constexpr int kOff = 2;

} // namespace

ColorSettingsDialog::ColorSettingsDialog(int currentPolicy, QWidget* parent) : QDialog(parent)
{
    setObjectName(QStringLiteral("colorSettingsDialog"));
    setWindowTitle(tr("Color Settings"));

    workingSpace_ = new QLabel(QStringLiteral("sRGB IEC61966-2.1"), this);
    workingSpace_->setObjectName(QStringLiteral("colorSettingsWorkingSpace"));

    combo_ = new QComboBox(this);
    combo_->setObjectName(QStringLiteral("colorSettingsPolicy"));
    combo_->addItem(tr("Off"), kOff);
    combo_->addItem(tr("Preserve Embedded Profiles"), kPreserve);
    combo_->addItem(tr("Convert to Working RGB"), kConvert);
    const int index = combo_->findData(currentPolicy);
    combo_->setCurrentIndex(index >= 0 ? index : combo_->findData(kPreserve));

    advanced_ = new QGroupBox(tr("Advanced Controls"), this);
    advanced_->setObjectName(QStringLiteral("colorSettingsAdvanced"));
    blendTextGamma_ = new QCheckBox(tr("Blend Text Colors Using Gamma:"), advanced_);
    blendTextGamma_->setObjectName(QStringLiteral("colorSettingsBlendTextGamma"));
    textGammaValue_ = new QDoubleSpinBox(advanced_);
    textGammaValue_->setObjectName(QStringLiteral("colorSettingsTextGamma"));
    textGammaValue_->setRange(1.0, 2.2);
    textGammaValue_->setDecimals(2);
    textGammaValue_->setSingleStep(0.01);
    connect(blendTextGamma_, &QCheckBox::toggled, textGammaValue_, &QWidget::setEnabled);
    auto* gammaRow = new QHBoxLayout(advanced_);
    gammaRow->addWidget(blendTextGamma_);
    gammaRow->addWidget(textGammaValue_);
    gammaRow->addStretch();
    setTextGamma(true, 1.45);
    advanced_->hide();

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    moreOptions_ = buttons->addButton(tr("More Options"), QDialogButtonBox::ActionRole);
    moreOptions_->setObjectName(QStringLiteral("colorSettingsMoreOptions"));
    connect(moreOptions_, &QPushButton::clicked, this, [this]() {
        const bool show = advanced_->isHidden();
        advanced_->setVisible(show);
        moreOptions_->setText(show ? tr("Fewer Options") : tr("More Options"));
        adjustSize();
    });
    connect(buttons, &QDialogButtonBox::accepted, this, &ColorSettingsDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &ColorSettingsDialog::reject);

    auto* outer = new QVBoxLayout(this);
    outer->addWidget(workingSpace_);
    outer->addWidget(combo_);
    outer->addWidget(advanced_);
    outer->addWidget(buttons);
    combo_->setFocus();
}

int ColorSettingsDialog::policyCode() const
{
    return combo_->currentData().toInt();
}

void ColorSettingsDialog::setTextGamma(bool enabled, double gamma)
{
    blendTextGamma_->setChecked(enabled);
    textGammaValue_->setValue(gamma);
    textGammaValue_->setEnabled(enabled);
}

bool ColorSettingsDialog::textGammaEnabled() const
{
    return blendTextGamma_->isChecked();
}

double ColorSettingsDialog::textGamma() const
{
    return textGammaValue_->value();
}

QStringList ColorSettingsDialog::policyChoicesForTest() const
{
    QStringList out;
    for (int i = 0; i < combo_->count(); ++i) {
        out.append(combo_->itemText(i));
    }
    return out;
}

QString ColorSettingsDialog::workingSpaceForTest() const
{
    return workingSpace_->text();
}

void ColorSettingsDialog::setPolicyForTest(int index)
{
    if (index >= 0 && index < combo_->count()) {
        combo_->setCurrentIndex(index);
    }
}

} // namespace pictura

#include "moc_color_settings_dialog.cpp"
