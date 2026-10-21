#include "angle_dial.h"

#include <QtCore/QPointF>
#include <QtCore/QRectF>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPaintEvent>
#include <QtGui/QPalette>
#include <QtMath>

#include <cmath>

namespace pictura {

AngleDial::AngleDial(QWidget* parent)
    : QWidget(parent)
{
    setFixedSize(34, 34);
    setCursor(Qt::CrossCursor);
}

void AngleDial::setAngle(double degrees)
{
    const double wrapped = std::fmod(std::fmod(degrees, 360.0) + 360.0, 360.0);
    if (qFuzzyCompare(wrapped + 1.0, angle_ + 1.0)) {
        return;
    }
    angle_ = wrapped;
    update();
}

void AngleDial::paintEvent(QPaintEvent*)
{
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing, true);

    const QRectF face = QRectF(rect()).adjusted(2, 2, -2, -2);
    // Chrome follows the palette so all four brightness levels repaint it;
    // the ratios hold the original contrast (face above the pane, rim and
    // hand on the text tone).
    const QColor windowText = palette().color(QPalette::WindowText);
    painter.setPen(QPen(windowText.darker(165), 1.0));
    painter.setBrush(palette().color(QPalette::Button));
    painter.drawEllipse(face);

    const QPointF centre = face.center();
    const double radians = qDegreesToRadians(angle_);
    const QPointF tip(centre.x() + std::cos(radians) * face.width() / 2.2,
                      centre.y() - std::sin(radians) * face.height() / 2.2);
    painter.setPen(QPen(windowText, 1.4));
    painter.drawLine(centre, tip);
    painter.setPen(Qt::NoPen);
    painter.setBrush(windowText);
    painter.drawEllipse(centre, 1.6, 1.6);
}

void AngleDial::mousePressEvent(QMouseEvent* event)
{
    aim(event->position());
}

void AngleDial::mouseMoveEvent(QMouseEvent* event)
{
    aim(event->position());
}

void AngleDial::aim(const QPointF& pos)
{
    const QPointF centre = QRectF(rect()).center();
    const double degrees =
        qRadiansToDegrees(std::atan2(centre.y() - pos.y(), pos.x() - centre.x()));
    setAngle(degrees);
    emit angleChanged(angle_);
}

} // namespace pictura

#include "moc_angle_dial.cpp"
