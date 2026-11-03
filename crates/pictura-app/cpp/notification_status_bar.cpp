#include "notification_status_bar.h"

#include <QtGui/QPainter>
#include <QtGui/QPaintEvent>

namespace pictura {

NotificationStatusBar::NotificationStatusBar(QWidget* parent)
    : QStatusBar(parent)
{
}

void NotificationStatusBar::paintEvent(QPaintEvent* event)
{
    QStatusBar::paintEvent(event);

    const QString message = currentMessage();
    if (message.isEmpty()) {
        return;
    }

    const QRect rect = messageRect();
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.setPen(QPen(QColor(0xd4, 0xb1, 0x06), 1));
    painter.setBrush(QColor(0xff, 0xf3, 0xb0));
    painter.drawRoundedRect(rect.adjusted(0, 2, 0, -2), 4, 4);
    painter.setPen(QColor(0x1a, 0x1a, 0x1a));
    painter.drawText(rect, Qt::AlignLeading | Qt::AlignVCenter | Qt::TextSingleLine, message);
}

QRect NotificationStatusBar::messageRect() const
{
    int left = 6;
    int right = width() - 12;
    for (QWidget* child : findChildren<QWidget*>()) {
        if (!child->isVisible()) {
            continue;
        }
        const int x = child->geometry().x();
        if (x > 0) {
            right = qMin(right, x - 2);
        }
    }
    return QRect(left, 0, qMax(0, right - left), height());
}

} // namespace pictura

#include "moc_notification_status_bar.cpp"
