#include "cineon_options_dialog.h"

#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

CineonOptionsDialog::CineonOptionsDialog(const SaveFormatOptions&, QWidget* parent)
    : QDialog(parent)
{
    setObjectName(QStringLiteral("cineonOptionsDialog"));
    setWindowTitle(tr("Cineon Options"));
    auto* root = new QVBoxLayout(this);
    auto* label = new QLabel(tr("10-bit RGB output (fixed)"), this);
    label->setObjectName(QStringLiteral("cineonOptionsBits"));
    root->addWidget(label);
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &CineonOptionsDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &CineonOptionsDialog::reject);
    root->addWidget(buttons);
    setMinimumWidth(300);
}

} // namespace pictura

#include "moc_cineon_options_dialog.cpp"
