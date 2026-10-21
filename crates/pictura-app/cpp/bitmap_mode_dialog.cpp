#include "bitmap_mode_dialog.h"

#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

BitmapModeDialog::BitmapModeDialog(QWidget* parent) : QDialog(parent)
{
    setWindowTitle(tr("Bitmap"));
    auto* root = new QVBoxLayout(this);

    auto* methodBox = new QGroupBox(tr("Method"), this);
    auto* methodForm = new QFormLayout(methodBox);
    method_ = new QComboBox(methodBox);
    method_->setObjectName(QStringLiteral("bitmapMethod"));
    method_->addItem(tr("50% Threshold"), 0);
    method_->addItem(tr("Pattern Dither"), 1);
    method_->addItem(tr("Diffusion Dither"), 2);
    methodForm->addRow(tr("Use:"), method_);
    root->addWidget(methodBox);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    root->addWidget(buttons);
}

int BitmapModeDialog::method() const { return method_->currentData().toInt(); }

} // namespace pictura

#include "moc_bitmap_mode_dialog.cpp"
