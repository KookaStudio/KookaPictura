#include "dcs_options_dialog.h"

#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

DcsOptionsDialog::DcsOptionsDialog(const SaveFormatOptions& initial, QWidget* parent)
    : QDialog(parent)
{
    setObjectName(QStringLiteral("dcsOptionsDialog"));
    setWindowTitle(tr("DCS Format"));
    auto* root = new QVBoxLayout(this);
    auto* form = new QFormLayout();
    version_ = new QComboBox(this);
    version_->setObjectName(QStringLiteral("dcsOptionsVersion"));
    version_->addItem(tr("DCS 1.0"), 1);
    version_->addItem(tr("DCS 2.0"), 2);
    version_->setCurrentIndex(initial.dcsVersion == 1 ? 0 : 1);
    form->addRow(tr("Format:"), version_);
    preview_ = new QComboBox(this);
    preview_->setObjectName(QStringLiteral("dcsOptionsPreview"));
    preview_->addItem(tr("None"), 0);
    preview_->addItem(tr("TIFF 1-bit"), 1);
    preview_->addItem(tr("TIFF 8-bit"), 2);
    preview_->setCurrentIndex(initial.dcsPreview);
    form->addRow(tr("Preview:"), preview_);
    root->addLayout(form);
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &DcsOptionsDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &DcsOptionsDialog::reject);
    root->addWidget(buttons);
    setMinimumWidth(320);
}

int DcsOptionsDialog::version() const { return version_->currentData().toInt(); }
int DcsOptionsDialog::preview() const { return preview_->currentData().toInt(); }

} // namespace pictura

#include "moc_dcs_options_dialog.cpp"
