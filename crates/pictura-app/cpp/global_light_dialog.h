#pragma once

#include <QtWidgets/QDialog>

class QSpinBox;

namespace pictura {

// Layer > Layer Style > Global Light…: the document-wide Angle and Altitude
// that every effect with Use Global Light reads. Values only; the caller
// applies them on accept.
class GlobalLightDialog : public QDialog {
    Q_OBJECT

public:
    explicit GlobalLightDialog(int angle, int altitude, QWidget* parent = nullptr);

    int angle() const;
    int altitude() const;
    bool dirty() const { return dirty_; }

private:
    QSpinBox* angle_ = nullptr;
    QSpinBox* altitude_ = nullptr;
    bool dirty_ = false;
};

} // namespace pictura
