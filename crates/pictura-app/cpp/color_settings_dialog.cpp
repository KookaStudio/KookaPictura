#include "color_settings_dialog.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QFormLayout>
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

void selectData(QComboBox* combo, int data) {
    const int index = combo->findData(data);
    combo->setCurrentIndex(index >= 0 ? index : 0);
}

} // namespace

ColorSettingsDialog::ColorSettingsDialog(const ColorSettingsValues& values, QWidget* parent)
    : QDialog(parent)
{
    buildUi(values);
}

ColorSettingsDialog::ColorSettingsDialog(int currentPolicy, QWidget* parent)
    : QDialog(parent)
{
    ColorSettingsValues values;
    values.rgbPolicy = currentPolicy;
    buildUi(values);
}

void ColorSettingsDialog::buildUi(const ColorSettingsValues& values)
{
    setObjectName(QStringLiteral("colorSettingsDialog"));
    setWindowTitle(tr("Color Settings"));

    auto* spaces = new QGroupBox(tr("Working Spaces"), this);
    auto* spacesForm = new QFormLayout(spaces);
    workingSpace_ = new QLabel(QStringLiteral("sRGB IEC61966-2.1"), spaces);
    workingSpace_->setObjectName(QStringLiteral("colorSettingsWorkingSpace"));
    spacesForm->addRow(tr("RGB:"), workingSpace_);
    cmykSpace_ = new QComboBox(spaces);
    cmykSpace_->setObjectName(QStringLiteral("colorSettingsCmykSpace"));
    cmykSpace_->addItem(tr("Photocraft Coated CMYK"), 0);
    selectData(cmykSpace_, values.workingCmyk);
    spacesForm->addRow(tr("CMYK:"), cmykSpace_);
    spacesForm->addRow(tr("Gray:"), new QLabel(QStringLiteral("Gray Gamma 2.2"), spaces));

    auto* policies = new QGroupBox(tr("Color Management Policies"), this);
    auto* policyForm = new QFormLayout(policies);
    rgbPolicy_ = new QComboBox(policies);
    rgbPolicy_->setObjectName(QStringLiteral("colorSettingsPolicy"));
    rgbPolicy_->addItem(tr("Off"), kOff);
    rgbPolicy_->addItem(tr("Preserve Embedded Profiles"), kPreserve);
    rgbPolicy_->addItem(tr("Convert to Working RGB"), kConvert);
    selectData(rgbPolicy_, values.rgbPolicy);
    policyForm->addRow(tr("RGB:"), rgbPolicy_);
    cmykPolicy_ = new QComboBox(policies);
    cmykPolicy_->setObjectName(QStringLiteral("colorSettingsCmykPolicy"));
    cmykPolicy_->addItem(tr("Off"), kOff);
    cmykPolicy_->addItem(tr("Preserve Embedded Profiles"), kPreserve);
    cmykPolicy_->addItem(tr("Convert to Working CMYK"), kConvert);
    selectData(cmykPolicy_, values.cmykPolicy);
    policyForm->addRow(tr("CMYK:"), cmykPolicy_);
    askMismatch_ = new QCheckBox(tr("Profile Mismatches: Ask When Opening"), policies);
    askMismatch_->setObjectName(QStringLiteral("colorSettingsAskMismatch"));
    askMismatch_->setChecked(values.askMismatchOpen);
    policyForm->addRow(QString(), askMismatch_);
    askMissing_ = new QCheckBox(tr("Missing Profiles: Ask When Opening"), policies);
    askMissing_->setObjectName(QStringLiteral("colorSettingsAskMissing"));
    askMissing_->setChecked(values.askMissingOpen);
    policyForm->addRow(QString(), askMissing_);

    auto* options = new QGroupBox(tr("Conversion Options"), this);
    auto* optionsForm = new QFormLayout(options);
    optionsForm->addRow(tr("Engine:"), new QLabel(QStringLiteral("Little CMS 2"), options));
    intent_ = new QComboBox(options);
    intent_->setObjectName(QStringLiteral("colorSettingsIntent"));
    intent_->addItem(tr("Perceptual"), 0);
    intent_->addItem(tr("Relative Colorimetric"), 1);
    intent_->addItem(tr("Saturation"), 2);
    intent_->addItem(tr("Absolute Colorimetric"), 3);
    selectData(intent_, values.renderIntent);
    optionsForm->addRow(tr("Intent:"), intent_);
    bpc_ = new QCheckBox(tr("Use Black Point Compensation"), options);
    bpc_->setObjectName(QStringLiteral("colorSettingsBpc"));
    bpc_->setChecked(values.blackPointCompensation);
    optionsForm->addRow(QString(), bpc_);
    dither_ = new QCheckBox(tr("Use Dither"), options);
    dither_->setObjectName(QStringLiteral("colorSettingsDither"));
    dither_->setChecked(values.useDither);
    optionsForm->addRow(QString(), dither_);

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
    setTextGamma(values.textGammaEnabled, values.textGamma);
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
    outer->addWidget(spaces);
    outer->addWidget(policies);
    outer->addWidget(options);
    outer->addWidget(advanced_);
    outer->addWidget(buttons);
    rgbPolicy_->setFocus();
}

ColorSettingsValues ColorSettingsDialog::values() const
{
    ColorSettingsValues values;
    values.rgbPolicy = rgbPolicy_->currentData().toInt();
    values.cmykPolicy = cmykPolicy_->currentData().toInt();
    values.workingCmyk = cmykSpace_->currentData().toInt();
    values.renderIntent = intent_->currentData().toInt();
    values.blackPointCompensation = bpc_->isChecked();
    values.useDither = dither_->isChecked();
    values.askMismatchOpen = askMismatch_->isChecked();
    values.askMissingOpen = askMissing_->isChecked();
    values.textGammaEnabled = blendTextGamma_->isChecked();
    values.textGamma = textGammaValue_->value();
    return values;
}

int ColorSettingsDialog::policyCode() const
{
    return rgbPolicy_->currentData().toInt();
}

int ColorSettingsDialog::cmykPolicyCode() const
{
    return cmykPolicy_->currentData().toInt();
}

int ColorSettingsDialog::workingCmyk() const
{
    return cmykSpace_->currentData().toInt();
}

int ColorSettingsDialog::renderIntent() const
{
    return intent_->currentData().toInt();
}

bool ColorSettingsDialog::blackPointCompensation() const
{
    return bpc_->isChecked();
}

bool ColorSettingsDialog::useDither() const
{
    return dither_->isChecked();
}

bool ColorSettingsDialog::askMismatchOpen() const
{
    return askMismatch_->isChecked();
}

bool ColorSettingsDialog::askMissingOpen() const
{
    return askMissing_->isChecked();
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
    for (int i = 0; i < rgbPolicy_->count(); ++i) {
        out.append(rgbPolicy_->itemText(i));
    }
    return out;
}

QStringList ColorSettingsDialog::cmykPolicyChoicesForTest() const
{
    QStringList out;
    for (int i = 0; i < cmykPolicy_->count(); ++i) {
        out.append(cmykPolicy_->itemText(i));
    }
    return out;
}

QStringList ColorSettingsDialog::intentChoicesForTest() const
{
    QStringList out;
    for (int i = 0; i < intent_->count(); ++i) {
        out.append(intent_->itemText(i));
    }
    return out;
}

QString ColorSettingsDialog::workingSpaceForTest() const
{
    return workingSpace_->text();
}

QString ColorSettingsDialog::cmykSpaceForTest() const
{
    return cmykSpace_->currentText();
}

void ColorSettingsDialog::setPolicyForTest(int index)
{
    if (index >= 0 && index < rgbPolicy_->count()) {
        rgbPolicy_->setCurrentIndex(index);
    }
}

void ColorSettingsDialog::setCmykPolicyForTest(int index)
{
    if (index >= 0 && index < cmykPolicy_->count()) {
        cmykPolicy_->setCurrentIndex(index);
    }
}

void ColorSettingsDialog::setIntentForTest(int code)
{
    selectData(intent_, code);
}

void ColorSettingsDialog::setAskMismatchForTest(bool on)
{
    askMismatch_->setChecked(on);
}

void ColorSettingsDialog::setAskMissingForTest(bool on)
{
    askMissing_->setChecked(on);
}

} // namespace pictura

#include "moc_color_settings_dialog.cpp"
