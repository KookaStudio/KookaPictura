#include "eps_options_dialog.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

EpsOptionsDialog::EpsOptionsDialog(const SaveFormatOptions& initial, QWidget* parent)
    : QDialog(parent)
{
    setObjectName(QStringLiteral("epsOptionsDialog"));
    setWindowTitle(tr("Photoshop EPS Options"));
    auto* root = new QVBoxLayout(this);
    auto* form = new QFormLayout();
    preview_ = new QComboBox(this);
    preview_->setObjectName(QStringLiteral("epsOptionsPreview"));
    preview_->addItem(tr("None"), 0);
    preview_->addItem(tr("TIFF 1-bit"), 1);
    preview_->addItem(tr("TIFF 8-bit"), 2);
    preview_->setCurrentIndex(initial.epsPreview);
    form->addRow(tr("Preview:"), preview_);
    encoding_ = new QComboBox(this);
    encoding_->setObjectName(QStringLiteral("epsOptionsEncoding"));
    encoding_->addItem(tr("ASCII"), 0);
    encoding_->addItem(tr("ASCII85"), 1);
    encoding_->addItem(tr("Binary"), 2);
    encoding_->addItem(tr("JPEG"), 3);
    encoding_->setCurrentIndex(initial.epsEncoding);
    form->addRow(tr("Encoding:"), encoding_);
    quality_ = new QSpinBox(this);
    quality_->setObjectName(QStringLiteral("epsOptionsJpegQuality"));
    quality_->setRange(0, 12);
    quality_->setValue(initial.epsJpegQuality);
    form->addRow(tr("JPEG Quality:"), quality_);
    root->addLayout(form);
    halftone_ = new QCheckBox(tr("Include Halftone Screen"), this);
    halftone_->setObjectName(QStringLiteral("epsOptionsHalftone"));
    halftone_->setChecked(initial.epsHalftone);
    transfer_ = new QCheckBox(tr("Include Transfer Function"), this);
    transfer_->setObjectName(QStringLiteral("epsOptionsTransfer"));
    transfer_->setChecked(initial.epsTransfer);
    transparentWhites_ = new QCheckBox(tr("Transparent Whites"), this);
    transparentWhites_->setObjectName(QStringLiteral("epsOptionsTransparentWhites"));
    transparentWhites_->setChecked(initial.epsTransparentWhites);
    psColorManagement_ = new QCheckBox(tr("PostScript Color Management"), this);
    psColorManagement_->setObjectName(QStringLiteral("epsOptionsPsColorManagement"));
    psColorManagement_->setChecked(initial.epsPsColorManagement);
    vectorData_ = new QCheckBox(tr("Include Vector Data"), this);
    vectorData_->setObjectName(QStringLiteral("epsOptionsVectorData"));
    vectorData_->setChecked(initial.epsVectorData);
    interpolation_ = new QCheckBox(tr("Image Interpolation"), this);
    interpolation_->setObjectName(QStringLiteral("epsOptionsInterpolation"));
    interpolation_->setChecked(initial.epsInterpolation);
    for (QCheckBox* box : {halftone_, transfer_, transparentWhites_, psColorManagement_, vectorData_, interpolation_}) {
        root->addWidget(box);
    }
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &EpsOptionsDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &EpsOptionsDialog::reject);
    root->addWidget(buttons);
    setMinimumWidth(420);
}

int EpsOptionsDialog::preview() const { return preview_->currentData().toInt(); }
int EpsOptionsDialog::encoding() const { return encoding_->currentData().toInt(); }
int EpsOptionsDialog::jpegQuality() const { return quality_->value(); }
bool EpsOptionsDialog::halftone() const { return halftone_->isChecked(); }
bool EpsOptionsDialog::transfer() const { return transfer_->isChecked(); }
bool EpsOptionsDialog::transparentWhites() const { return transparentWhites_->isChecked(); }
bool EpsOptionsDialog::psColorManagement() const { return psColorManagement_->isChecked(); }
bool EpsOptionsDialog::vectorData() const { return vectorData_->isChecked(); }
bool EpsOptionsDialog::interpolation() const { return interpolation_->isChecked(); }

} // namespace pictura

#include "moc_eps_options_dialog.cpp"
