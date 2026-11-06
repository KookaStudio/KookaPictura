#include "notification_status_bar.h"

#include <QtGui/QPainter>
#include <QtGui/QPaintEvent>
#include <QtWidgets/QStyle>
#include <QtWidgets/QStyleOption>

namespace pictura {

NotificationStatusBar::NotificationStatusBar(QWidget* parent)
    : QStatusBar(parent)
{
}

void NotificationStatusBar::paintEvent(QPaintEvent* event)
{
    Q_UNUSED(event);

    // The banner owns the message: draw the themed panel, then the pill and its
    // text once. The base paintEvent would draw the message text a second time
    // across the whole bar, including the readouts the message is replacing.
    QPainter painter(this);
    QStyleOption option;
    option.initFrom(this);
    style()->drawPrimitive(QStyle::PE_PanelStatusBar, &option, &painter, this);

    const QString message = currentMessage();
    if (message.isEmpty()) {
        return;
    }

    const QRect rect = messageRect();
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.setPen(QPen(QColor(0xd4, 0xb1, 0x06), 1));
    painter.setBrush(QColor(0xff, 0xf3, 0xb0));
    painter.drawRoundedRect(rect.adjusted(0, 2, 0, -2), 4, 4);
    painter.setPen(QColor(0x20, 0x18, 0x00));
    painter.drawText(rect.adjusted(6, 0, -6, 0),
                     Qt::AlignLeading | Qt::AlignVCenter | Qt::TextSingleLine, message);
}

QRect NotificationStatusBar::messageRect() const
{
    int left = 6;
    int right = width() - 12;
    // Direct children only: a readout's internal widget has its own coordinate
    // space and would collapse the banner.
    for (QWidget* child : findChildren<QWidget*>(Qt::FindDirectChildrenOnly)) {
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
