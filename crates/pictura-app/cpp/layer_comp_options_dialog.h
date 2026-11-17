#pragma once

#include <QtWidgets/QDialog>

class QCheckBox;
class QLineEdit;
class QPlainTextEdit;

namespace pictura {

class LayerCompOptionsDialog : public QDialog {
    Q_OBJECT

public:
    LayerCompOptionsDialog(const QString& title, const QString& name, const QString& comment,
                           int recordBits, QWidget* parent = nullptr);

    QString name() const;
    QString comment() const;
    int recordBits() const;

private:
    QLineEdit* name_ = nullptr;
    QPlainTextEdit* comment_ = nullptr;
    QCheckBox* visibility_ = nullptr;
    QCheckBox* position_ = nullptr;
    QCheckBox* appearance_ = nullptr;
};

} // namespace pictura
