#pragma once

// The Vanishing Point dialog's tool options, one page per tool, pushed to the
// Rust session as they change: Grid Size and Angle for the plane tools;
// Feather, Opacity, Heal and Move Mode for the Marquee; Diameter, Hardness,
// Opacity, Heal and Aligned for the Stamp; the same plus Brush Color for the
// Brush; Flip and Flop for Transform; Brush Color for the Eyedropper.
// ponytail: the Stamp and Brush ranges copy the main brush tools; CS6's own
// defaults are not tabulated.

#include <QtGui/QColor>
#include <QtWidgets/QStackedWidget>

#include <functional>

class QCheckBox;
class QComboBox;
class QSpinBox;
class QToolButton;

namespace pictura {

class VpOptionsBar : public QStackedWidget {
    Q_OBJECT

public:
    explicit VpOptionsBar(QWidget* parent = nullptr);

    // Show the page for the dialog's tool `tool`.
    void showFor(int tool);

    QSpinBox* gridSize() const { return gridSize_; }
    QSpinBox* angle() const { return angle_; }

    QColor colour() const { return colour_; }
    void setColour(const QColor& colour);
    // `[` / `]`: step the Stamp's or Brush's Diameter for `tool`.
    void stepDiameter(int tool, int direction);

    // Transform's Flip (`horizontal`) and Flop buttons.
    std::function<void(bool horizontal)> mirrored;

private:
    void pushMarquee();
    void pushPaint(bool brush);
    void pickColour();

    QSpinBox* gridSize_ = nullptr;
    QSpinBox* angle_ = nullptr;
    QSpinBox* feather_ = nullptr;
    QSpinBox* marqueeOpacity_ = nullptr;
    QComboBox* marqueeHeal_ = nullptr;
    QComboBox* moveMode_ = nullptr;
    struct Paint {
        QSpinBox* diameter = nullptr;
        QSpinBox* hardness = nullptr;
        QSpinBox* opacity = nullptr;
        QComboBox* heal = nullptr;
    };
    Paint stamp_;
    Paint brush_;
    QList<QToolButton*> swatches_;
    QColor colour_ = Qt::black;
    QWidget* planesPage_ = nullptr;
    QWidget* marqueePage_ = nullptr;
    QWidget* stampPage_ = nullptr;
    QWidget* brushPage_ = nullptr;
    QWidget* transformPage_ = nullptr;
    QWidget* eyedropperPage_ = nullptr;
    QWidget* emptyPage_ = nullptr;
};

} // namespace pictura
