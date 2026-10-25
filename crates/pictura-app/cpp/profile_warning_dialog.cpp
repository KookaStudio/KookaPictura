#include "profile_warning_dialog.h"

#include <QtWidgets/QButtonGroup>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

ProfileWarningDialog::ProfileWarningDialog(Kind kind, const QString& profileName, QWidget* parent)
    : QDialog(parent)
    , kind_(kind)
{
    setObjectName(QStringLiteral("profileWarningDialog"));
    setWindowTitle(kind == Mismatch ? tr("Embedded Profile Mismatch")
                                    : tr("Missing Profile"));

    auto* root = new QVBoxLayout(this);
    auto* message = new QLabel(
        kind == Mismatch
            ? tr("The document has an embedded profile that does not match the working space%1.")
                  .arg(profileName.isEmpty() ? QString() : QStringLiteral(" (%1)").arg(profileName))
            : tr("The document has no embedded color profile."),
        this);
    message->setWordWrap(true);
    root->addWidget(message);

    auto* group = new QButtonGroup(this);
    first_ = new QRadioButton(
        kind == Mismatch ? tr("Use the embedded profile")
                         : tr("Assign working space"),
        this);
    second_ = new QRadioButton(
        kind == Mismatch ? tr("Convert to working space")
                         : tr("Assign then convert"),
        this);
    third_ = new QRadioButton(
        kind == Mismatch ? tr("Discard the embedded profile")
                         : tr("Leave without a profile"),
        this);
    for (QRadioButton* radio : {first_, second_, third_}) {
        group->addButton(radio);
        root->addWidget(radio);
    }
    (kind == Mismatch ? first_ : third_)->setChecked(true);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &ProfileWarningDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &ProfileWarningDialog::reject);
    root->addWidget(buttons);
}

ProfileWarningDialog::Choice ProfileWarningDialog::choice() const
{
    if (kind_ == Mismatch) {
        if (second_->isChecked()) {
            return ConvertToWorking;
        }
        if (third_->isChecked()) {
            return DiscardEmbedded;
        }
        return UseEmbedded;
    }
    if (first_->isChecked()) {
        return AssignWorking;
    }
    if (second_->isChecked()) {
        return AssignThenConvert;
    }
    return LeaveUntagged;
}

void ProfileWarningDialog::setChoiceForTest(Choice choice)
{
    QRadioButton* target = third_;
    if (kind_ == Mismatch) {
        target = choice == ConvertToWorking ? second_ : choice == DiscardEmbedded ? third_ : first_;
    } else {
        target = choice == AssignWorking ? first_
                 : choice == AssignThenConvert ? second_
                                               : third_;
    }
    target->setChecked(true);
}

} // namespace pictura

#include "moc_profile_warning_dialog.cpp"
