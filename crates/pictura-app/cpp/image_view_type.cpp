// ImageView overlay for the Type tools: the text being typed and its caret,
// or for the Type Mask tools the red quick-mask tint with the letters cut out.
// The caret blinks on a 1.5 s cycle; point type marks its placement point and
// the baseline under each typed line, paragraph type its box and handles.

#include "image_view.h"

#include <QtCore/QTimer>
#include <QtGui/QPainter>
#include <QtGui/QRegion>

namespace pictura {

int ImageView::typeCaretBlinkIntervalForTest() const
{
    return typeCaretBlink_ ? typeCaretBlink_->interval() : kTypeCaretBlinkMs;
}

void ImageView::setTypeOverlay(const TypeOverlay& overlay)
{
    typeOverlay_ = overlay;
    if (!typeCaretBlink_) {
        typeCaretBlink_ = new QTimer(this);
        typeCaretBlink_->setInterval(kTypeCaretBlinkMs);
        connect(typeCaretBlink_, &QTimer::timeout, this, [this]() {
            typeCaretShown_ = !typeCaretShown_;
            update();
        });
    }
    // An edit shows the caret at once, so it never vanishes under a keystroke.
    typeCaretShown_ = true;
    typeCaretBlink_->start();
    update();
}

void ImageView::clearTypeOverlay()
{
    if (typeCaretBlink_) {
        typeCaretBlink_->stop();
    }
    typeCaretShown_ = true;
    if (!typeOverlay_.active) {
        return;
    }
    typeOverlay_ = TypeOverlay();
    update();
}

void ImageView::paintTypeOverlay(QPainter& painter)
{
    const TypeOverlay& o = typeOverlay_;
    if (!o.active) {
        return;
    }
    const QTransform toWidget = QTransform::fromScale(zoom_, zoom_)
        * QTransform::fromTranslate(offset_.x(), offset_.y());
    const QTransform toDevice = toWidget * viewRotation();
    painter.save();
    painter.setClipping(false);
    painter.setTransform(toDevice);
    const QRect textRect(o.topLeft, o.image.size());
    if (o.mask) {
        QRegion tinted(QRect(QPoint(0, 0), o.canvas));
        if (!o.image.isNull()) {
            tinted -= textRect;
        }
        painter.setClipRegion(tinted);
        painter.fillRect(QRect(QPoint(0, 0), o.canvas), QColor(255, 0, 0, 128));
        painter.setClipping(false);
    }
    if (!o.image.isNull()) {
        painter.drawImage(o.topLeft, o.image);
    }
    painter.setPen(Qt::NoPen);
    painter.setBrush(o.mask ? QColor(255, 255, 255, 110) : QColor(51, 153, 255, 110));
    for (const QPolygonF& quad : o.selection) {
        painter.drawPolygon(quad);
    }
    // The caret, guides, and box stay one device pixel wide at any zoom.
    const QColor ink = o.mask || o.lightInk ? Qt::white : Qt::black;
    painter.setRenderHint(QPainter::Antialiasing, false);
    painter.setBrush(Qt::NoBrush);
    painter.setPen(QPen(ink, 0));
    for (const QLineF& line : o.baselines) {
        painter.drawLine(line);
    }
    if (typeCaretShown_) {
        painter.drawLine(o.caret);
    }
    if (!o.box.isEmpty()) {
        // White under black dashes, as the marquee and transform boxes draw.
        painter.setPen(QPen(Qt::white, 0, Qt::SolidLine));
        painter.drawPolygon(o.box);
        painter.setPen(QPen(Qt::black, 0, Qt::DashLine));
        painter.drawPolygon(o.box);
    }

    // Markers are drawn in device pixels so they keep their size.
    painter.setTransform(QTransform());
    if (o.hasPivot) {
        // The reference point: a small ring with a cross, as Free Transform's.
        const QPointF p = toDevice.map(o.pivot);
        painter.setBrush(Qt::NoBrush);
        painter.setPen(QPen(Qt::white, 3));
        painter.drawEllipse(p, 4.0, 4.0);
        painter.setPen(QPen(Qt::black, 1));
        painter.drawEllipse(p, 4.0, 4.0);
        painter.drawLine(QPointF(p.x() - 7.0, p.y()), QPointF(p.x() + 7.0, p.y()));
        painter.drawLine(QPointF(p.x(), p.y() - 7.0), QPointF(p.x(), p.y() + 7.0));
    }
    if (o.hasPlacement) {
        const QPointF p = toDevice.map(o.placement);
        painter.fillRect(QRectF(p.x() - 1.5, p.y() - 1.5, 3.0, 3.0), ink);
    }
    if (o.box.size() >= 4) {
        const QPolygonF box = toDevice.map(o.box);
        painter.setPen(QPen(Qt::black, 0, Qt::SolidLine));
        painter.setBrush(Qt::white);
        for (int i = 0; i < 4; ++i) {
            const QPointF corner = box.at(i);
            const QPointF mid = (corner + box.at((i + 1) % 4)) / 2.0;
            for (const QPointF& h : {corner, mid}) {
                painter.drawRect(QRectF(h.x() - 3.0, h.y() - 3.0, 6.0, 6.0));
            }
        }
        if (o.overflow) {
            // The bottom-right corner (box order: TL, TR, BR, BL) gets a plus.
            const QPointF c = box.at(2);
            painter.setPen(QPen(QColor(200, 0, 0), 0));
            painter.drawLine(QPointF(c.x() - 2.0, c.y()), QPointF(c.x() + 2.0, c.y()));
            painter.drawLine(QPointF(c.x(), c.y() - 2.0), QPointF(c.x(), c.y() + 2.0));
        }
    }
    painter.restore();
}

} // namespace pictura
