#include "color_swatch_widget.h"

#include <QtGui/QPainter>
#include <QtGui/QPalette>

namespace pictura {

ColorSwatchWidget::ColorSwatchWidget(QWidget* parent)
    : QPushButton(parent)
{
    setObjectName(QStringLiteral("colorSwatch"));
}

void ColorSwatchWidget::setColor(const QColor& color)
{
    color_ = color.isValid() ? color.toRgb() : QColor();
    setProperty("color", color_);
    update();
}

void ColorSwatchWidget::setActive(bool active)
{
    if (active_ == active) {
        return;
    }
    active_ = active;
    update();
}

void ColorSwatchWidget::paintEvent(QPaintEvent*)
{
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing);
    const qreal inset = active_ ? 1.0 : 0.5;
    const QRectF frame = QRectF(rect()).adjusted(inset, inset, -inset, -inset);
    if (color_.isValid()) {
        painter.fillRect(frame, color_);
    } else {
        painter.fillRect(frame, palette().color(QPalette::Base));
        painter.setPen(palette().color(QPalette::Disabled, QPalette::WindowText));
        painter.drawLine(frame.topLeft(), frame.bottomRight());
    }
    QPen pen(active_ ? palette().color(QPalette::Highlight)
                     : palette().color(isEnabled() ? QPalette::Active : QPalette::Disabled,
                                       QPalette::Mid));
    pen.setWidth(active_ ? 2 : 1);    painter.setPen(pen);
    painter.setBrush(Qt::NoBrush);
    painter.drawRect(frame);
    if (hasFocus()) {
        QPen focus(palette().color(QPalette::Highlight));
        focus.setStyle(Qt::DotLine);
        painter.setPen(focus);
        painter.drawRect(frame.adjusted(2, 2, -2, -2));
    }
}

} // namespace pictura

#include "moc_color_swatch_widget.cpp"
