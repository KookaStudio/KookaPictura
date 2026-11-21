#pragma once

#include <QtCore/QList>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtWidgets/QDialog>

class QCheckBox;

namespace pictura {

// View > Show > Show Extra Options: the membership window that selects which
// extras View > Extras (Ctrl+H) hides.
class ShowExtrasDialog : public QDialog {
    Q_OBJECT

public:
    struct Entry {
        QString id;
        QString label;
    };

    ShowExtrasDialog(const QList<Entry>& entries, const QStringList& checked,
                     QWidget* parent = nullptr);

    QStringList selectedIds() const;

private:
    QList<Entry> entries_;
    QList<QCheckBox*> boxes_;
};

} // namespace pictura
