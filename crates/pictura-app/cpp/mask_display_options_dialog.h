#pragma once

#include <QtGui/QColor>
#include <QtWidgets/QDialog>

class QSlider;
class QSpinBox;
class QToolButton;

namespace pictura {

// Layer Mask Display Options: the rubylith overlay's colour and opacity. The
// settings change only how a mask is displayed, never what it protects.
class MaskDisplayOptionsDialog : public QDialog {
    Q_OBJECT

public:
    MaskDisplayOptionsDialog(const QColor& color, int opacity, QWidget* parent = nullptr);

    QColor color() const { return color_; }
    int opacity() const;

    void setColor(const QColor& color);
    void setOpacity(int opacity);

private:
    void pickColor();

    QColor color_;
    QToolButton* swatch_ = nullptr;
    QSpinBox* opacity_ = nullptr;
    QSlider* opacitySlider_ = nullptr;
};

} // namespace pictura
