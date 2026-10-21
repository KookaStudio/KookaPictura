#include "color_settings_dialog.h"

#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QLabel>
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

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &ColorSettingsDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &ColorSettingsDialog::reject);

    auto* outer = new QVBoxLayout(this);
    outer->addWidget(workingSpace_);
    outer->addWidget(combo_);
    outer->addWidget(buttons);
    combo_->setFocus();
}

int ColorSettingsDialog::policyCode() const
{
    return combo_->currentData().toInt();
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
