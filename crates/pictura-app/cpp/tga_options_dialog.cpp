#include "tga_options_dialog.h"

#include <QtWidgets/QButtonGroup>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {
constexpr int kDepths[] = {16, 24, 32};
}

TgaOptionsDialog::TgaOptionsDialog(const SaveFormatOptions& initial, QWidget* parent)
    : QDialog(parent)
{
    setObjectName(QStringLiteral("tgaOptionsDialog"));
    setWindowTitle(tr("Targa Options"));
    auto* root = new QVBoxLayout(this);
    auto* box = new QGroupBox(tr("Resolution"), this);
    auto* layout = new QVBoxLayout(box);
    auto* group = new QButtonGroup(box);
    for (int i = 0; i < 3; ++i) {
        depth_[i] = new QRadioButton(tr("%1 bits/pixel").arg(kDepths[i]), box);
        depth_[i]->setObjectName(QStringLiteral("tgaOptionsDepth%1").arg(kDepths[i]));
        group->addButton(depth_[i], kDepths[i]);
        if (kDepths[i] == initial.tgaDepth) {
            depth_[i]->setChecked(true);
        }
        layout->addWidget(depth_[i]);
    }
    if (!group->checkedButton()) {
        depth_[1]->setChecked(true);
    }
    root->addWidget(box);
    rle_ = new QCheckBox(tr("Compress (RLE)"), this);
    rle_->setObjectName(QStringLiteral("tgaOptionsRle"));
    rle_->setChecked(initial.tgaRle);
    root->addWidget(rle_);
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &TgaOptionsDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &TgaOptionsDialog::reject);
    root->addWidget(buttons);
    setMinimumWidth(320);
}

int TgaOptionsDialog::depth() const
{
    for (int i = 0; i < 3; ++i) {
        if (depth_[i]->isChecked()) {
            return kDepths[i];
        }
    }
    return 24;
}

bool TgaOptionsDialog::rle() const { return rle_->isChecked(); }

} // namespace pictura

#include "moc_tga_options_dialog.cpp"
