#include "layer_comp_options_dialog.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QPlainTextEdit>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

LayerCompOptionsDialog::LayerCompOptionsDialog(const QString& title, const QString& name,
                                               const QString& comment, int recordBits,
                                               QWidget* parent)
    : QDialog(parent)
{
    setWindowTitle(title);
    auto* layout = new QVBoxLayout(this);
    auto* form = new QFormLayout;

    name_ = new QLineEdit(name, this);
    name_->setObjectName(QStringLiteral("layerCompName"));
    form->addRow(tr("Name:"), name_);

    comment_ = new QPlainTextEdit(comment, this);
    comment_->setObjectName(QStringLiteral("layerCompComment"));
    comment_->setFixedHeight(72);
    form->addRow(tr("Comment:"), comment_);

    visibility_ = new QCheckBox(tr("Visibility"), this);
    visibility_->setObjectName(QStringLiteral("layerCompVisibility"));
    visibility_->setChecked((recordBits & 1) != 0);
    form->addRow(QString(), visibility_);

    position_ = new QCheckBox(tr("Position"), this);
    position_->setObjectName(QStringLiteral("layerCompPosition"));
    position_->setChecked((recordBits & 2) != 0);
    form->addRow(QString(), position_);

    appearance_ = new QCheckBox(tr("Appearance"), this);
    appearance_->setObjectName(QStringLiteral("layerCompAppearance"));
    appearance_->setChecked((recordBits & 4) != 0);
    form->addRow(QString(), appearance_);

    layout->addLayout(form);
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Cancel | QDialogButtonBox::Ok, this);
    layout->addWidget(buttons);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
}

QString LayerCompOptionsDialog::name() const
{
    return name_->text();
}

QString LayerCompOptionsDialog::comment() const
{
    return comment_->toPlainText();
}

int LayerCompOptionsDialog::recordBits() const
{
    return (visibility_->isChecked() ? 1 : 0) | (position_->isChecked() ? 2 : 0)
           | (appearance_->isChecked() ? 4 : 0);
}

} // namespace pictura

#include "moc_layer_comp_options_dialog.cpp"
