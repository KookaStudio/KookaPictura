#include "png_options_dialog.h"

#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

PngOptionsDialog::PngOptionsDialog(const SaveFormatOptions& initial, QWidget* parent)
    : QDialog(parent)
{
    setObjectName(QStringLiteral("pngOptionsDialog"));
    setWindowTitle(tr("PNG Options"));

    auto* root = new QVBoxLayout(this);

    auto* compressionBox = new QGroupBox(tr("Compression"), this);
    auto* compressionLayout = new QVBoxLayout(compressionBox);
    fast_ = new QRadioButton(tr("None / Fast"), compressionBox);
    fast_->setObjectName(QStringLiteral("pngOptionsFast"));
    slow_ = new QRadioButton(tr("Smallest / Slow"), compressionBox);
    slow_->setObjectName(QStringLiteral("pngOptionsSlow"));
    (initial.pngCompression >= 50 ? slow_ : fast_)->setChecked(true);
    compressionLayout->addWidget(fast_);
    compressionLayout->addWidget(slow_);
    root->addWidget(compressionBox);

    auto* interlaceBox = new QGroupBox(tr("Interlace"), this);
    auto* interlaceLayout = new QVBoxLayout(interlaceBox);
    none_ = new QRadioButton(tr("None"), interlaceBox);
    none_->setObjectName(QStringLiteral("pngOptionsInterlaceNone"));
    interlaced_ = new QRadioButton(tr("Interlaced"), interlaceBox);
    interlaced_->setObjectName(QStringLiteral("pngOptionsInterlaced"));
    (initial.pngInterlace == 1 ? interlaced_ : none_)->setChecked(true);
    // ponytail: Qt's PNG writer exposes no interlace option.
    for (QRadioButton* button : {none_, interlaced_}) {
        button->setEnabled(false);
        button->setToolTip(tr("PNG interlacing is not written yet."));
    }
    interlaceLayout->addWidget(none_);
    interlaceLayout->addWidget(interlaced_);
    root->addWidget(interlaceBox);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &PngOptionsDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &PngOptionsDialog::reject);
    root->addWidget(buttons);
    setMinimumWidth(420);
}

int PngOptionsDialog::compression() const
{
    return slow_->isChecked() ? 100 : 0;
}

int PngOptionsDialog::interlace() const
{
    return interlaced_->isChecked() ? 1 : 0;
}

} // namespace pictura

#include "moc_png_options_dialog.cpp"
