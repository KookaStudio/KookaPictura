#include "brush_tip_preview.h"

#include <QtCore/QEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QPaintEvent>
#include <QtGui/QPainter>
#include <QtGui/QPolygonF>

#include <algorithm>

namespace pictura {

BrushTipPreview::BrushTipPreview(QWidget* parent)
    : QWidget(parent)
{
    setObjectName(QStringLiteral("brushTipPreview"));
    setFixedSize(kWidth, kHeight);
    setCursor(Qt::OpenHandCursor);
    if (parent) {
        parent->installEventFilter(this);
    }
    hide();
}

QSize BrushTipPreview::sizeHint() const
{
    return QSize(kWidth, kHeight);
}

void BrushTipPreview::paintEvent(QPaintEvent*)
{
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing, true);

    painter.fillRect(rect(), QColor(0xb3, 0xb3, 0xb3));

    const int ground = height() - 18;
    QPen dash(QColor(0xa0, 0xa0, 0xa0), 1, Qt::DashLine);
    dash.setDashPattern({3.0, 3.0});
    painter.setPen(dash);
    painter.drawLine(0, ground, width(), ground);

    const int cx = width() / 2;
    const int barrelW = qRound(width() * 0.12);
    const int barrelLeft = cx - barrelW / 2;
    const int barrelBottom = qRound(height() * 0.565);
    const int shoulder = qRound(height() * 0.74);
    const QRect barrel(barrelLeft, 0, barrelW, barrelBottom);

    painter.fillRect(barrel, QColor(0xeb, 0xeb, 0xeb));
    painter.setPen(QPen(QColor(0xce, 0xce, 0xce), 1));
    painter.drawLine(barrel.topLeft(), barrel.bottomLeft());
    painter.setPen(QPen(QColor(0xe1, 0xe1, 0xe1), 1));
    painter.drawLine(barrel.topRight(), barrel.bottomRight());

    QPolygonF nib;
    nib << QPointF(barrelLeft, barrelBottom)
        << QPointF(barrelLeft + barrelW, barrelBottom)
        << QPointF(barrelLeft + barrelW, shoulder) << QPointF(cx, ground)
        << QPointF(barrelLeft, shoulder);
    painter.setPen(Qt::NoPen);
    painter.setBrush(QColor(0x1d, 0x1d, 0x1d));
    painter.drawPolygon(nib);

    QPolygonF bevel;
    bevel << QPointF(cx, barrelBottom) << QPointF(barrelLeft + barrelW, barrelBottom)
          << QPointF(barrelLeft + barrelW, shoulder) << QPointF(cx, ground);
    painter.setBrush(QColor(0x34, 0x38, 0x3a));
    painter.drawPolygon(bevel);
}

void BrushTipPreview::mousePressEvent(QMouseEvent* event)
{
    if (event->button() == Qt::LeftButton) {
        dragging_ = true;
        dragOffset_ = event->position().toPoint();
        setCursor(Qt::ClosedHandCursor);
    }
    event->accept();
}

void BrushTipPreview::mouseMoveEvent(QMouseEvent* event)
{
    if (dragging_) {
        move(mapToParent(event->position().toPoint()) - dragOffset_);
        clampToParent();
    }
    event->accept();
}

void BrushTipPreview::mouseReleaseEvent(QMouseEvent* event)
{
    if (event->button() == Qt::LeftButton) {
        dragging_ = false;
        setCursor(Qt::OpenHandCursor);
    }
    event->accept();
}

bool BrushTipPreview::eventFilter(QObject* watched, QEvent* event)
{
    if (watched == parentWidget() && event->type() == QEvent::Resize) {
        clampToParent();
    }
    return QWidget::eventFilter(watched, event);
}

void BrushTipPreview::clampToParent()
{
    QWidget* host = parentWidget();
    if (!host) {
        return;
    }
    const int maxX = std::max(0, host->width() - width());
    const int maxY = std::max(0, host->height() - height());
    move(std::clamp(x(), 0, maxX), std::clamp(y(), 0, maxY));
}

} // namespace pictura

#include "moc_brush_tip_preview.cpp"
