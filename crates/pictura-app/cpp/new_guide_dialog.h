#pragma once

#include <QtWidgets/QDialog>

class QDoubleSpinBox;
class QRadioButton;

namespace pictura {

// View > New Guide…: a horizontal or vertical guide at a position in pixels.
class NewGuideDialog : public QDialog {
    Q_OBJECT

public:
    explicit NewGuideDialog(QWidget* parent = nullptr);

    bool vertical() const;
    double position() const;
    void setGuide(bool vertical, double position);

private:
    QRadioButton* horizontal_ = nullptr;
    QRadioButton* vertical_ = nullptr;
    QDoubleSpinBox* position_ = nullptr;
};

} // namespace pictura
