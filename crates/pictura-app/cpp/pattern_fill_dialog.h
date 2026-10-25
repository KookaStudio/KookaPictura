#pragma once

#include <QtWidgets/QDialog>

class QCheckBox;
class QLabel;
class QListWidget;
class QPushButton;

namespace pictura {

class NumericField;
class PictureView;

// CS6's Pattern Fill layer editor: the document's pattern picker, Scale, Snap
// To Origin, and Link With Layer. The pattern library is the open document's
// embedded patterns; with none the dialog is an empty state.
class PatternFillDialog : public QDialog {
    Q_OBJECT

public:
    PatternFillDialog(PictureView& view, const QString& payload, QWidget* parent = nullptr);

    QString payload() const;
    static bool getFill(PictureView& view, const QString& initial, QWidget* parent,
                        QString* out);

    QListWidget* listForTest() const { return list_; }
    NumericField* scaleForTest() const { return scale_; }
    QCheckBox* linkForTest() const { return link_; }
    QPushButton* snapForTest() const { return snap_; }
    bool emptyForTest() const { return empty_; }

private:
    PictureView& view_;
    QString id_;
    int originX_ = 0;
    int originY_ = 0;
    bool empty_ = false;
    QListWidget* list_ = nullptr;
    NumericField* scale_ = nullptr;
    QCheckBox* link_ = nullptr;
    QPushButton* snap_ = nullptr;
    QLabel* emptyLabel_ = nullptr;
};

} // namespace pictura
