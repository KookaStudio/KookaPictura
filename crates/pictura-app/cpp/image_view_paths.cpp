// ImageView overlay for the Pen tool group and the path selection tools: the
// Work Path's curve, anchors, direction handles, and bounding box. Ported from photorust's
// CanvasView::paintPathOverlay.

#include "image_view.h"

#include "path_overlay.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <QtGui/QPainter>

namespace pictura {

void ImageView::setPathOverlay(const PathOverlay& overlay)
{
    pathOverlay_ = overlay;
    update();
}

void showWorkPathOverlay(ImageView& canvas, PictureView* view)
{
    if (!view || !view->has_document()) {
        canvas.clearPathOverlay();
        return;
    }
    canvas.setPathOverlay(workPathOverlay(*view, -1));
}

void ImageView::clearPathOverlay()
{
    if (pathOverlay_.curve.isEmpty() && pathOverlay_.anchors.isEmpty()
        && pathOverlay_.preview.isEmpty()) {
        return;
    }
    pathOverlay_ = PathOverlay();
    update();
}

// Drawn in widget space so lines and glyphs keep their screen size at any zoom.
void ImageView::paintPathOverlay(QPainter& painter)
{
    const PathOverlay& o = pathOverlay_;
    if (o.curve.isEmpty() && o.anchors.isEmpty() && o.preview.isEmpty()) {
        return;
    }
    const QTransform toWidget = QTransform::fromScale(zoom_, zoom_)
        * QTransform::fromTranslate(offset_.x(), offset_.y());
    const QColor blue(0x2c, 0x6f, 0xd6);
    painter.save();
    painter.setTransform(viewRotation());
    painter.setClipping(false);
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.setBrush(Qt::NoBrush);

    // White under blue reads over light and dark pixels alike.
    const QPainterPath curve = toWidget.map(o.curve);
    painter.setPen(QPen(Qt::white, 2.6));
    painter.drawPath(curve);
    painter.setPen(QPen(blue, 1.1));
    painter.drawPath(curve);
    painter.setPen(QPen(blue, 1.0, Qt::DashLine));
    painter.drawPath(toWidget.map(o.preview));
    if (!o.bounds.isNull()) {
        painter.setPen(QPen(blue, 1.0));
        painter.drawRect(toWidget.mapRect(o.bounds));
    }

    painter.setPen(QPen(blue, 1.0));
    for (const QLineF& line : o.handles) {
        painter.drawLine(toWidget.map(line));
    }
    painter.setBrush(Qt::white);
    for (const QLineF& line : o.handles) {
        painter.drawEllipse(toWidget.map(line.p2()), 2.6, 2.6);
    }

    // Hollow squares, the active anchor (or a selected component's every
    // anchor) solid, as CS6 draws them.
    painter.setPen(QPen(blue, 1.2));
    for (int i = 0; i < o.anchors.size(); ++i) {
        const QPointF p = toWidget.map(o.anchors.at(i));
        painter.setBrush(o.anchorsSolid || i == o.activeAnchor ? QBrush(blue)
                                                                 : QBrush(Qt::white));
        painter.drawRect(QRectF(p.x() - 2.5, p.y() - 2.5, 5.0, 5.0));
    }
    painter.restore();
}

} // namespace pictura
