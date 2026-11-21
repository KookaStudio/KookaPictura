#include "find_replace_dialog.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

FindReplaceDialog::FindReplaceDialog(bool currentLayerIsType, QWidget* parent)
    : QDialog(parent)
    , currentLayerIsType_(currentLayerIsType)
{
    setObjectName(QStringLiteral("findReplaceDialog"));
    setWindowTitle(tr("Find And Replace Text"));
    setWindowModality(Qt::WindowModal);

    auto* layout = new QVBoxLayout(this);
    auto* form = new QFormLayout();
    find_ = new QLineEdit(this);
    find_->setObjectName(QStringLiteral("findReplaceFind"));
    replace_ = new QLineEdit(this);
    replace_->setObjectName(QStringLiteral("findReplaceReplace"));
    form->addRow(tr("Find What:"), find_);
    form->addRow(tr("Replace With:"), replace_);
    layout->addLayout(form);

    allLayers_ = new QCheckBox(tr("Search All Layers"), this);
    allLayers_->setObjectName(QStringLiteral("findReplaceAllLayers"));
    forward_ = new QCheckBox(tr("Forward"), this);
    forward_->setObjectName(QStringLiteral("findReplaceForward"));
    forward_->setChecked(true);
    caseSensitive_ = new QCheckBox(tr("Case Sensitive"), this);
    caseSensitive_->setObjectName(QStringLiteral("findReplaceCaseSensitive"));
    wholeWord_ = new QCheckBox(tr("Whole Word Only"), this);
    wholeWord_->setObjectName(QStringLiteral("findReplaceWholeWord"));
    for (QCheckBox* box : {allLayers_, forward_, caseSensitive_, wholeWord_}) {
        layout->addWidget(box);
    }

    auto* buttons = new QDialogButtonBox(this);
    replaceAll_ = buttons->addButton(tr("Replace All"), QDialogButtonBox::ActionRole);
    replaceAll_->setObjectName(QStringLiteral("findReplaceReplaceAll"));
    buttons->addButton(QDialogButtonBox::Close);
    layout->addWidget(buttons);

    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    connect(replaceAll_, &QPushButton::clicked, this, &FindReplaceDialog::replaceAllRequested);
    connect(find_, &QLineEdit::textChanged, this, [this]() { updateReplaceEnabled(); });
    connect(allLayers_, &QCheckBox::toggled, this, [this]() { updateReplaceEnabled(); });
    updateReplaceEnabled();
    find_->setFocus();
}

QString FindReplaceDialog::findText() const
{
    return find_->text();
}

QString FindReplaceDialog::replaceText() const
{
    return replace_->text();
}

bool FindReplaceDialog::searchAllLayers() const
{
    return allLayers_->isChecked();
}

bool FindReplaceDialog::forward() const
{
    return forward_->isChecked();
}

bool FindReplaceDialog::caseSensitive() const
{
    return caseSensitive_->isChecked();
}

bool FindReplaceDialog::wholeWordOnly() const
{
    return wholeWord_->isChecked();
}

void FindReplaceDialog::updateReplaceEnabled()
{
    replaceAll_->setEnabled(!find_->text().isEmpty()
                            && (allLayers_->isChecked() || currentLayerIsType_));
}

} // namespace pictura

#include "moc_find_replace_dialog.cpp"
