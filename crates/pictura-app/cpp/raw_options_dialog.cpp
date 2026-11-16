#include "raw_options_dialog.h"

#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

RawOptionsDialog::RawOptionsDialog(const SaveFormatOptions& initial, QWidget* parent)
    : QDialog(parent)
{
    setObjectName(QStringLiteral("rawOptionsDialog"));
    setWindowTitle(tr("Photoshop Raw Options"));
    auto* root = new QVBoxLayout(this);
    auto* form = new QFormLayout();
    header_ = new QSpinBox(this);
    header_->setObjectName(QStringLiteral("rawOptionsHeader"));
    header_->setRange(0, 1 << 20);
    header_->setValue(initial.rawHeader);
    form->addRow(tr("Header:"), header_);
    interleave_ = new QComboBox(this);
    interleave_->setObjectName(QStringLiteral("rawOptionsInterleave"));
    interleave_->addItem(tr("Interleaved"), 0);
    interleave_->addItem(tr("Non-interleaved"), 1);
    interleave_->setCurrentIndex(initial.rawInterleave == 1 ? 1 : 0);
    form->addRow(tr("Interleave:"), interleave_);
    auto* fileType = new QLineEdit(this);
    fileType->setObjectName(QStringLiteral("rawOptionsFileType"));
    fileType->setEnabled(false);
    fileType->setToolTip(tr("File Type is macOS-only."));
    form->addRow(tr("File Type:"), fileType);
    auto* fileCreator = new QLineEdit(this);
    fileCreator->setObjectName(QStringLiteral("rawOptionsFileCreator"));
    fileCreator->setEnabled(false);
    fileCreator->setToolTip(tr("File Creator is macOS-only."));
    form->addRow(tr("File Creator:"), fileCreator);
    root->addLayout(form);
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &RawOptionsDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &RawOptionsDialog::reject);
    root->addWidget(buttons);
    setMinimumWidth(360);
}

int RawOptionsDialog::header() const { return header_->value(); }
int RawOptionsDialog::interleave() const { return interleave_->currentData().toInt(); }

} // namespace pictura

#include "moc_raw_options_dialog.cpp"
