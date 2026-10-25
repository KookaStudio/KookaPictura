#include "save_options_dialog.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

SaveOptionsDialog::SaveOptionsDialog(bool embedDefault, bool profileAvailable, QWidget* parent)
    : QDialog(parent)
{
    setObjectName(QStringLiteral("saveOptionsDialog"));
    setWindowTitle(tr("Save Options"));

    auto* root = new QVBoxLayout(this);
    embed_ = new QCheckBox(tr("Embed Color Profile"), this);
    embed_->setObjectName(QStringLiteral("saveOptionsEmbedProfile"));
    embed_->setChecked(profileAvailable && embedDefault);
    embed_->setEnabled(profileAvailable);
    root->addWidget(embed_);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &SaveOptionsDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &SaveOptionsDialog::reject);
    root->addWidget(buttons);
}

bool SaveOptionsDialog::embedProfile() const
{
    return embed_->isChecked();
}

void SaveOptionsDialog::setEmbedForTest(bool on)
{
    embed_->setChecked(on);
}

} // namespace pictura

#include "moc_save_options_dialog.cpp"
