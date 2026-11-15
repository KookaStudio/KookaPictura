#pragma once

#include <QtWidgets/QDialog>

class QRadioButton;

namespace pictura {

// BMP Advanced Modes: the three 16-bit layouts.
class BmpAdvancedModesDialog : public QDialog {
    Q_OBJECT
public:
    explicit BmpAdvancedModesDialog(int initial, QWidget* parent = nullptr);
    int layout() const; // 0 X1R5G5B5, 1 A1R5G5B5, 2 R5G6B5
private:
    QRadioButton* layouts_[3] = {};
};

} // namespace pictura
