#pragma once

#include "gradient_editor_dialog.h"

#include <QtWidgets/QDialog>

class QCheckBox;
class QComboBox;
class QToolButton;

namespace pictura {

class NumericField;

// CS6's Gradient Fill layer editor: the gradient sample (click for the
// Gradient Editor), Style, Angle, Scale, Reverse, and the present-but-disabled
// Dither and Align With Layer controls.
class GradientFillDialog : public QDialog {
    Q_OBJECT

public:
    explicit GradientFillDialog(const QString& payload, QWidget* parent = nullptr);

    QString payload() const;
    static bool getFill(const QString& initial, QWidget* parent, QString* out);

    QComboBox* styleForTest() const { return style_; }
    NumericField* angleForTest() const { return angle_; }
    NumericField* scaleForTest() const { return scale_; }
    QCheckBox* reverseForTest() const { return reverse_; }
    // Open the Gradient Editor on the current stops.
    void openEditor();

private:
    void updateSample();

    GradientSpec spec_;
    QToolButton* sample_ = nullptr;
    QComboBox* style_ = nullptr;
    NumericField* angle_ = nullptr;
    NumericField* scale_ = nullptr;
    QCheckBox* reverse_ = nullptr;
};

} // namespace pictura
