#include "bmp_options_dialog.h"

#include <QtWidgets/QButtonGroup>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

constexpr int kDepths[] = {1, 4, 8, 16, 24, 32};

} // namespace

BmpOptionsDialog::BmpOptionsDialog(const SaveFormatOptions& initial, QWidget* parent)
    : QDialog(parent)
{
    setObjectName(QStringLiteral("bmpOptionsDialog"));
    setWindowTitle(tr("BMP Options"));

    auto* root = new QVBoxLayout(this);

    // ponytail: Qt's BMP writer exposes no file format, depth, RLE, or flip
    // option, so every control below is shown disabled.
    auto* formatBox = new QGroupBox(tr("File Format"), this);
    auto* formatLayout = new QVBoxLayout(formatBox);
    windows_ = new QRadioButton(tr("Windows"), formatBox);
    windows_->setObjectName(QStringLiteral("bmpOptionsWindows"));
    os2_ = new QRadioButton(tr("OS/2"), formatBox);
    os2_->setObjectName(QStringLiteral("bmpOptionsOs2"));
    (initial.bmpFormat == 1 ? os2_ : windows_)->setChecked(true);
    formatLayout->addWidget(windows_);
    formatLayout->addWidget(os2_);
    root->addWidget(formatBox);

    auto* depthBox = new QGroupBox(tr("Depth"), this);
    auto* depthLayout = new QVBoxLayout(depthBox);
    auto* depthGroup = new QButtonGroup(depthBox);
    for (int i = 0; i < 6; ++i) {
        depth_[i] = new QRadioButton(tr("%1 Bit").arg(kDepths[i]), depthBox);
        depth_[i]->setObjectName(QStringLiteral("bmpOptionsDepth%1").arg(kDepths[i]));
        depthGroup->addButton(depth_[i], kDepths[i]);
        if (kDepths[i] == initial.bmpDepth) {
            depth_[i]->setChecked(true);
        }
        depthLayout->addWidget(depth_[i]);
    }
    root->addWidget(depthBox);

    rle_ = new QCheckBox(tr("Compress (RLE)"), this);
    rle_->setObjectName(QStringLiteral("bmpOptionsRle"));
    rle_->setChecked(initial.bmpRle);
    flip_ = new QCheckBox(tr("Flip row order"), this);
    flip_->setObjectName(QStringLiteral("bmpOptionsFlip"));
    flip_->setChecked(initial.bmpFlip);
    root->addWidget(rle_);
    root->addWidget(flip_);

    auto* advanced = new QPushButton(tr("Advanced Modes"), this);
    advanced->setObjectName(QStringLiteral("bmpOptionsAdvanced"));
    auto* advancedRow = new QHBoxLayout();
    advancedRow->addWidget(advanced);
    advancedRow->addStretch();
    root->addLayout(advancedRow);

    for (QWidget* widget :
         {static_cast<QWidget*>(windows_), static_cast<QWidget*>(os2_),
          static_cast<QWidget*>(rle_), static_cast<QWidget*>(flip_),
          static_cast<QWidget*>(advanced)}) {
        widget->setEnabled(false);
        widget->setToolTip(tr("This BMP option is not written yet."));
    }
    for (QRadioButton* button : depth_) {
        button->setEnabled(false);
        button->setToolTip(tr("This BMP option is not written yet."));
    }

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &BmpOptionsDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &BmpOptionsDialog::reject);
    root->addWidget(buttons);
    setMinimumWidth(420);
}

int BmpOptionsDialog::format() const
{
    return os2_->isChecked() ? 1 : 0;
}

int BmpOptionsDialog::depth() const
{
    for (int i = 0; i < 6; ++i) {
        if (depth_[i]->isChecked()) {
            return kDepths[i];
        }
    }
    return 24;
}

bool BmpOptionsDialog::rle() const
{
    return rle_->isChecked();
}

bool BmpOptionsDialog::flip() const
{
    return flip_->isChecked();
}

} // namespace pictura

#include "moc_bmp_options_dialog.cpp"
