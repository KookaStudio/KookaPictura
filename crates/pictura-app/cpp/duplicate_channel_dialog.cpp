#include "duplicate_channel_dialog.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

DuplicateChannelDialog::DuplicateChannelDialog(const QString& sourceName,
                                               const QList<SelectionDocument>& documents,
                                               QWidget* parent)
    : QDialog(parent)
    , documents_(documents)
{
    setObjectName(QStringLiteral("duplicateChannelDialog"));
    setWindowTitle(tr("Duplicate Channel"));

    auto* root = new QVBoxLayout(this);
    auto* form = new QFormLayout;

    name_ = new QLineEdit(this);
    name_->setObjectName(QStringLiteral("duplicateChannelName"));
    name_->setText(sourceName.isEmpty() ? tr("Alpha 1 copy") : tr("%1 copy").arg(sourceName));
    form->addRow(tr("As:"), name_);

    document_ = new QComboBox(this);
    document_->setObjectName(QStringLiteral("duplicateChannelDocument"));
    for (int i = 0; i < documents_.size(); ++i) {
        document_->addItem(documents_.at(i).label, i);
        if (documents_.at(i).active) {
            document_->setCurrentIndex(i);
        }
    }
    document_->addItem(tr("New"), int(documents_.size()));
    form->addRow(tr("Document:"), document_);

    invert_ = new QCheckBox(tr("Invert"), this);
    invert_->setObjectName(QStringLiteral("duplicateChannelInvert"));
    form->addRow(QString(), invert_);
    root->addLayout(form);

    buttons_ = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    buttons_->setObjectName(QStringLiteral("duplicateChannelButtons"));
    root->addWidget(buttons_);
    connect(buttons_, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons_, &QDialogButtonBox::rejected, this, &QDialog::reject);
    connect(name_, &QLineEdit::textChanged, this, [this]() {
        if (buttons_) {
            buttons_->button(QDialogButtonBox::Ok)->setEnabled(!name_->text().trimmed().isEmpty());
        }
    });
}

DuplicateChannelRequest DuplicateChannelDialog::request() const
{
    DuplicateChannelRequest request;
    const int index = document_->currentData().toInt();
    if (index >= 0 && index < documents_.size()) {
        request.document = documents_.at(index).view;
    } else {
        request.newDocument = true;
    }
    request.name = name_->text().trimmed();
    request.invert = invert_->isChecked();
    return request;
}

int DuplicateChannelDialog::documentCountForTest() const
{
    return document_->count();
}

int DuplicateChannelDialog::currentDocumentForTest() const
{
    return document_->currentIndex();
}

QString DuplicateChannelDialog::nameForTest() const
{
    return name_->text();
}

bool DuplicateChannelDialog::invertForTest() const
{
    return invert_->isChecked();
}

bool DuplicateChannelDialog::okEnabledForTest() const
{
    return buttons_ && buttons_->button(QDialogButtonBox::Ok)->isEnabled();
}

void DuplicateChannelDialog::setDocumentIndexForTest(int index)
{
    document_->setCurrentIndex(index);
}

void DuplicateChannelDialog::setNameForTest(const QString& name)
{
    name_->setText(name);
}

void DuplicateChannelDialog::setInvertForTest(bool invert)
{
    invert_->setChecked(invert);
}

} // namespace pictura

#include "moc_duplicate_channel_dialog.cpp"
