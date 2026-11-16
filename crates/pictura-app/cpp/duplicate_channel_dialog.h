#pragma once

#include <QtCore/QList>
#include <QtCore/QString>
#include <QtWidgets/QDialog>

#include "save_load_selection_dialog.h"

class QCheckBox;
class QComboBox;
class QDialogButtonBox;
class QLineEdit;

namespace pictura {

struct DuplicateChannelRequest {
    PictureView* document = nullptr;
    bool newDocument = false;
    QString name;
    bool invert = false;
};

// Channels panel > Duplicate Channel… (PAN-002): name the copy, choose a
// same-size destination document or New, and optionally invert. The frame
// performs the cross-document transfer and records the destination history.
class DuplicateChannelDialog : public QDialog {
    Q_OBJECT

public:
    DuplicateChannelDialog(const QString& sourceName,
                           const QList<SelectionDocument>& documents,
                           QWidget* parent = nullptr);

    DuplicateChannelRequest request() const;

    int documentCountForTest() const;
    int currentDocumentForTest() const;
    QString nameForTest() const;
    bool invertForTest() const;
    bool okEnabledForTest() const;
    void setDocumentIndexForTest(int index);
    void setNameForTest(const QString& name);
    void setInvertForTest(bool invert);

private:
    QList<SelectionDocument> documents_;
    QComboBox* document_ = nullptr;
    QLineEdit* name_ = nullptr;
    QCheckBox* invert_ = nullptr;
    QDialogButtonBox* buttons_ = nullptr;
};

} // namespace pictura
