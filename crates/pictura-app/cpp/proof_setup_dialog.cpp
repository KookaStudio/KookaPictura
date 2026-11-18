#include "proof_setup_dialog.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

ProofSetupDialog::ProofSetupDialog(const ProofSetupValues& values, QWidget* parent)
    : QDialog(parent)
{
    setObjectName(QStringLiteral("proofSetupDialog"));
    setWindowTitle(tr("Proof Setup"));

    auto* layout = new QVBoxLayout(this);
    auto* form = new QFormLayout();

    device_ = new QComboBox(this);
    device_->setObjectName(QStringLiteral("proofSetupDevice"));
    device_->addItems({tr("Working CMYK"), tr("Working Gray"), tr("sRGB IEC61966-2.1"),
                       tr("Adobe RGB (1998)"), tr("ProPhoto RGB")});
    device_->setCurrentIndex(qBound(0, values.device, device_->count() - 1));
    form->addRow(tr("Device to Simulate:"), device_);

    intent_ = new QComboBox(this);
    intent_->setObjectName(QStringLiteral("proofSetupIntent"));
    intent_->addItems({tr("Perceptual"), tr("Relative Colorimetric"), tr("Saturation"),
                       tr("Absolute Colorimetric")});
    intent_->setCurrentIndex(qBound(0, values.intent, intent_->count() - 1));
    form->addRow(tr("Rendering Intent:"), intent_);

    bpc_ = new QCheckBox(tr("Black Point Compensation"), this);
    bpc_->setObjectName(QStringLiteral("proofSetupBpc"));
    bpc_->setChecked(values.bpc);
    form->addRow(bpc_);

    paper_ = new QCheckBox(tr("Simulate Paper Color"), this);
    paper_->setObjectName(QStringLiteral("proofSetupPaper"));
    paper_->setChecked(values.simulatePaper);
    form->addRow(paper_);

    black_ = new QCheckBox(tr("Simulate Black Ink"), this);
    black_->setObjectName(QStringLiteral("proofSetupBlack"));
    black_->setChecked(values.simulateBlack);
    form->addRow(black_);
    layout->addLayout(form);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    layout->addWidget(buttons);
}

ProofSetupValues ProofSetupDialog::values() const
{
    ProofSetupValues values;
    values.device = device_ ? device_->currentIndex() : 0;
    values.intent = intent_ ? intent_->currentIndex() : 1;
    values.bpc = bpc_ && bpc_->isChecked();
    values.simulatePaper = paper_ && paper_->isChecked();
    values.simulateBlack = black_ && black_->isChecked();
    return values;
}

void ProofSetupDialog::addPrintProfiles(const QStringList& profiles, int selected)
{
    device_->addItems(profiles);
    device_->setCurrentIndex(qBound(0, selected, device_->count() - 1));
}

void ProofSetupDialog::setDeviceForTest(int device)
{
    device_->setCurrentIndex(qBound(0, device, device_->count() - 1));
}

void ProofSetupDialog::setIntentForTest(int intent)
{
    intent_->setCurrentIndex(qBound(0, intent, intent_->count() - 1));
}

void ProofSetupDialog::setSimulateBlackForTest(bool on)
{
    black_->setChecked(on);
}

} // namespace pictura

#include "moc_proof_setup_dialog.cpp"
