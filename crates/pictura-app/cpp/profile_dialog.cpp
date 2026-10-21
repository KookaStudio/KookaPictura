#include "profile_dialog.h"

#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

constexpr int kSrgb = 0;
constexpr int kAdobeRgb = 1;
constexpr int kProPhotoRgb = 2;

} // namespace

ProfileDialog::ProfileDialog(bool convert, QWidget* parent) : QDialog(parent)
{
    setObjectName(QStringLiteral("profileDialog"));
    setWindowTitle(convert ? tr("Convert to Profile") : tr("Assign Profile"));

    combo_ = new QComboBox(this);
    combo_->setObjectName(QStringLiteral("profileDialogCombo"));
    if (convert) {
        combo_->addItem(tr("sRGB IEC61966-2.1"), kSrgb);
        combo_->addItem(tr("Adobe RGB (1998)"), kAdobeRgb);
        combo_->addItem(tr("Pro Photo RGB"), kProPhotoRgb);
    } else {
        combo_->addItem(tr("Don't Color Manage"), kSrgb);
        combo_->addItem(tr("Adobe RGB (1998)"), kAdobeRgb);
        combo_->addItem(tr("Pro Photo RGB"), kProPhotoRgb);
    }

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &ProfileDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &ProfileDialog::reject);

    auto* outer = new QVBoxLayout(this);
    outer->addWidget(combo_);
    outer->addWidget(buttons);
    combo_->setFocus();
}

int ProfileDialog::profileIndex() const
{
    return combo_->currentData().toInt();
}

QStringList ProfileDialog::choicesForTest() const
{
    QStringList out;
    for (int i = 0; i < combo_->count(); ++i) {
        out.append(combo_->itemText(i));
    }
    return out;
}

void ProfileDialog::setChoiceForTest(int index)
{
    if (index >= 0 && index < combo_->count()) {
        combo_->setCurrentIndex(index);
    }
}

} // namespace pictura

#include "moc_profile_dialog.cpp"
