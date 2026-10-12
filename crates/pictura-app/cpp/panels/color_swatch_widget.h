#pragma once

#include <QtGui/QColor>
#include <QtWidgets/QPushButton>

namespace pictura {

// The one shared colour-swatch control: a push button that paints its data
// colour (or an empty cell when the colour is invalid) inside the theme's
// frame, with an optional active frame. The fill is data, so it is painted
// rather than styled through a per-widget stylesheet.
class ColorSwatchWidget : public QPushButton {
    Q_OBJECT

public:
    explicit ColorSwatchWidget(QWidget* parent = nullptr);

    void setColor(const QColor& color);
    QColor color() const { return color_; }
    void setActive(bool active);
    bool isActive() const { return active_; }

protected:
    void paintEvent(QPaintEvent* event) override;

private:
    QColor color_;
    bool active_ = false;
};

} // namespace pictura
