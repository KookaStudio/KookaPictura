#include "gif_options_dialog.h"

#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

GifOptionsDialog::GifOptionsDialog(const SaveFormatOptions& initial, QWidget* parent)
    : QDialog(parent)
{
    setObjectName(QStringLiteral("gifOptionsDialog"));
    setWindowTitle(tr("GIF Options"));
    auto* root = new QVBoxLayout(this);
    auto* box = new QGroupBox(tr("Row Order"), this);
    auto* layout = new QVBoxLayout(box);
    normal_ = new QRadioButton(tr("Normal"), box);
    normal_->setObjectName(QStringLiteral("gifOptionsNormal"));
    interlaced_ = new QRadioButton(tr("Interlaced"), box);
    interlaced_->setObjectName(QStringLiteral("gifOptionsInterlaced"));
    (initial.gifRowOrder == 1 ? interlaced_ : normal_)->setChecked(true);
    layout->addWidget(normal_);
    layout->addWidget(interlaced_);
    root->addWidget(box);
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &GifOptionsDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &GifOptionsDialog::reject);
    root->addWidget(buttons);
    setMinimumWidth(320);
}

int GifOptionsDialog::rowOrder() const { return interlaced_->isChecked() ? 1 : 0; }

} // namespace pictura

#include "moc_gif_options_dialog.cpp"
