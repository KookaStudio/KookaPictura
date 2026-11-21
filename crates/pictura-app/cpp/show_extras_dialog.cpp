#include "show_extras_dialog.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

ShowExtrasDialog::ShowExtrasDialog(const QList<Entry>& entries, const QStringList& checked,
                                   QWidget* parent)
    : QDialog(parent), entries_(entries)
{
    setWindowTitle(tr("Show Extra Options"));
    setObjectName(QStringLiteral("showExtrasDialog"));
    auto* layout = new QVBoxLayout(this);
    auto* note = new QLabel(
        tr("Choose which extras View > Extras (Ctrl+H) hides and shows."), this);
    note->setWordWrap(true);
    layout->addWidget(note);
    for (const Entry& entry : entries_) {
        auto* box = new QCheckBox(entry.label, this);
        box->setObjectName(QStringLiteral("extrasOption_") + entry.id);
        box->setChecked(checked.contains(entry.id));
        boxes_.append(box);
        layout->addWidget(box);
    }
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    layout->addWidget(buttons);
}

QStringList ShowExtrasDialog::selectedIds() const
{
    QStringList ids;
    for (int i = 0; i < entries_.size() && i < boxes_.size(); ++i) {
        if (boxes_.at(i)->isChecked()) {
            ids.append(entries_.at(i).id);
        }
    }
    return ids;
}

} // namespace pictura

#include "moc_show_extras_dialog.cpp"
