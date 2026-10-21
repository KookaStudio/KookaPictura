#include "duplicate_image_dialog.h"

#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

DuplicateImageDialog::DuplicateImageDialog(const QString& sourceName,
                                           const QString& suggestedName, QWidget* parent)
    : QDialog(parent)
{
    setWindowTitle(tr("Duplicate Image"));

    auto* root = new QVBoxLayout(this);
    auto* grid = new QGridLayout;

    grid->addWidget(new QLabel(tr("Duplicate:"), this), 0, 0, Qt::AlignRight);
    grid->addWidget(new QLabel(sourceName, this), 0, 1);

    grid->addWidget(new QLabel(tr("As:"), this), 1, 0, Qt::AlignRight);
    name_ = new QLineEdit(suggestedName, this);
    name_->setObjectName(QStringLiteral("duplicateName"));
    name_->setMinimumWidth(240);
    grid->addWidget(name_, 1, 1);

    merged_ = new QCheckBox(tr("Duplicate Merged Layers Only"), this);
    merged_->setObjectName(QStringLiteral("duplicateMerged"));
    grid->addWidget(merged_, 2, 0, 1, 2);

    root->addLayout(grid);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    root->addWidget(buttons);

    name_->setFocus();
    name_->selectAll();
}

QString DuplicateImageDialog::copyName() const { return name_->text().trimmed(); }

bool DuplicateImageDialog::mergedOnly() const { return merged_->isChecked(); }

} // namespace pictura

#include "moc_duplicate_image_dialog.cpp"
