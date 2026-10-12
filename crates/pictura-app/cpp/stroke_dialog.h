#pragma once

#include <QtGui/QColor>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialog>
#include <QtWidgets/QLabel>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QSpinBox>

namespace pictura {

class ColorSwatchWidget;

// Photoshop's Edit > Stroke: Width, Color, Location (Inside / Center /
// Outside), a Blending group (Mode, Opacity), and Preserve Transparency.
// Ported from photorust's StrokeDialog.
class StrokeDialog : public QDialog {
    Q_OBJECT

public:
    explicit StrokeDialog(const QColor& initial, QWidget* parent = nullptr);

    int strokeWidth() const;
    QColor strokeColor() const;
    // 0 Inside, 1 Center, 2 Outside.
    int location() const;
    QString blendMode() const;
    int opacity() const;
    bool preserveTransparency() const;

protected:
    bool eventFilter(QObject* obj, QEvent* event) override;

private:
    void openColorPicker();
    void updateColorSwatch();

    QSpinBox* width_ = nullptr;
    ColorSwatchWidget* colorSwatch_ = nullptr;
    QColor color_;
    QRadioButton* inside_ = nullptr;
    QRadioButton* center_ = nullptr;
    QRadioButton* outside_ = nullptr;
    QComboBox* mode_ = nullptr;
    QSpinBox* opacity_ = nullptr;
    QCheckBox* preserveTransparency_ = nullptr;
};

} // namespace pictura
