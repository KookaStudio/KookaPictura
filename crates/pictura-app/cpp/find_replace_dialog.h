#pragma once

#include <QtCore/QString>
#include <QtWidgets/QDialog>

class QCheckBox;
class QLineEdit;
class QPushButton;

namespace pictura {

// Edit > Find And Replace Text…: the CS6 fields and options. The dialog owns
// no document logic; Replace All asks the registrar, which records one state.
class FindReplaceDialog : public QDialog {
    Q_OBJECT

public:
    explicit FindReplaceDialog(bool currentLayerIsType, QWidget* parent = nullptr);

    QString findText() const;
    QString replaceText() const;
    bool searchAllLayers() const;
    bool forward() const;
    bool caseSensitive() const;
    bool wholeWordOnly() const;

signals:
    void replaceAllRequested();

private:
    void updateReplaceEnabled();

    QLineEdit* find_ = nullptr;
    QLineEdit* replace_ = nullptr;
    QCheckBox* allLayers_ = nullptr;
    QCheckBox* forward_ = nullptr;
    QCheckBox* caseSensitive_ = nullptr;
    QCheckBox* wholeWord_ = nullptr;
    QPushButton* replaceAll_ = nullptr;
    bool currentLayerIsType_ = false;
};

} // namespace pictura
