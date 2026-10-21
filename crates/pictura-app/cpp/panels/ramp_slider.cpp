#include "ramp_slider.h"

#include <QtGui/QLinearGradient>
#include <QtGui/QPainter>
#include <QtGui/QPalette>

namespace pictura {

RampSlider::RampSlider(QWidget* parent)
    : JumpSlider(Qt::Horizontal, parent)
{
}

void RampSlider::setRamp(const QList<QColor>& stops)
{
    // The hue ramp is asked for again on every tick of a drag, so repaint only
    // when the colours actually moved.
    if (stops == stops_) {
        return;
    }
    stops_ = stops;
    update();
}

void RampSlider::paintEvent(QPaintEvent* event)
{
    if (stops_.size() < 2) {
        JumpSlider::paintEvent(event);
        return;
    }
    QStyleOptionSlider option;
    initStyleOption(&option);
    const QRect groove =
        style()->subControlRect(QStyle::CC_Slider, &option, QStyle::SC_SliderGroove, this);
    const QRect handle =
        style()->subControlRect(QStyle::CC_Slider, &option, QStyle::SC_SliderHandle, this);

    // Taller than the theme's 3px line, which is too thin to read a rainbow off,
    // and with no filled sub-page: on a ramp the colour is the information. The
    // ends stop half a handle in, where the handle's centre stops.
    const int inset = handle.width() / 2;
    const QRect band(groove.left() + inset, groove.center().y() - 3,
                     groove.width() - 2 * inset, 7);
    QLinearGradient ramp(band.topLeft(), band.topRight());
    for (int i = 0; i < stops_.size(); ++i) {
        ramp.setColorAt(qreal(i) / (stops_.size() - 1), stops_.at(i));
    }
    QPainter painter(this);
    painter.setOpacity(isEnabled() ? 1.0 : 0.4);
    painter.fillRect(band, ramp);
    painter.setPen(palette().color(QPalette::Shadow));
    painter.drawRect(band.adjusted(0, 0, -1, -1));
    painter.setOpacity(1.0);

    option.subControls = QStyle::SC_SliderHandle;
    style()->drawComplexControl(QStyle::CC_Slider, &option, &painter, this);
}

} // namespace pictura

#include "moc_ramp_slider.cpp"
