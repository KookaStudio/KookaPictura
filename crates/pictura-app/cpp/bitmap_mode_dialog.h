#pragma once

#include <QtCore/QJsonObject>
#include <QtWidgets/QDialog>

class QComboBox;
class QDoubleSpinBox;

namespace pictura {

class PictureView;

class BitmapModeDialog : public QDialog {
    Q_OBJECT

public:
    explicit BitmapModeDialog(PictureView& view, const QJsonObject& initial = {},
                              QWidget* parent = nullptr);

    int method() const;
    QJsonObject options() const;

private:
    QComboBox* method_ = nullptr;
    QDoubleSpinBox* frequency_ = nullptr;
    QComboBox* unit_ = nullptr;
    QDoubleSpinBox* angle_ = nullptr;
    QComboBox* shape_ = nullptr;
    QComboBox* pattern_ = nullptr;
};

} // namespace pictura
