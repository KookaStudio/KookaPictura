#include "selftest_canvas.h"
#include "selftest_report.h"

#include "frame.h"
#include "image_view.h"

#include "api/view_bridge.h"

#include <QtCore/QPoint>
#include <QtCore/QPointF>
#include <QtCore/QRect>
#include <QtCore/QString>
#include <QtCore/Qt>
#include <QtCore/QtNumeric>
#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>

namespace {

// Device-space document rect, rounded the way the canvas rounds its doc rect.
QRect canvasDeviceRect(const pictura::ImageView* view)
{
    const QPointF offset = view->offset();
    const double zoom = view->zoom();
    const double w = view->image().width() * zoom;
    const double h = view->image().height() * zoom;
    return QRect(qRound(offset.x()), qRound(offset.y()),
                 qRound(offset.x() + w) - qRound(offset.x()),
                 qRound(offset.y() + h) - qRound(offset.y()));
}

} // namespace

int pictura::runCanvasChecks(pictura::PicturaMainWindow& frame)
{
        // present_cache_edge (295): an opaque, canvas-sized document at a
        // fractional zoom must not leak the transparency checkerboard through a
        // one-pixel seam at the document's right/bottom edge, and the cached and
        // direct present paths must agree pixel for pixel.
        const bool ceCreated = frame.newDocument(QStringLiteral("CacheEdge"), 101, 77,
                                                 QStringLiteral("rgb"), 8,
                                                 QStringLiteral("white"));
        pictura::PictureView* ceView = frame.activeView();
        pictura::ImageView* ceCanvas = frame.imageView();
        if (!ceCreated || !ceView || !ceCanvas) {
            return pictura::selfTest().fail(295, "present cache fixture");
        }
        const int ceDoc = frame.activeDocumentIndex();
        const QString ceFill = ceView->add_solid_fill(0xffff0000u);
        if (ceFill.isEmpty()) {
            return pictura::selfTest().fail(295, "present cache fill");
        }
        if (ceCanvas->width() <= 0 || ceCanvas->height() <= 0) {
            frame.resize(800, 600);
            QApplication::processEvents();
        }

        const QSize ceSize = ceCanvas->size();
        const double ceSrcW = ceCanvas->image().width();
        const double ceSrcH = ceCanvas->image().height();
        const QColor ceRed(255, 0, 0);
        const auto ceInBounds = [&ceSize](const QPoint& p) {
            return p.x() >= 0 && p.y() >= 0 && p.x() < ceSize.width()
                   && p.y() < ceSize.height();
        };

        // Sweep fractional zooms until one makes the floor-sized cache fall
        // short of the checkerboard's rounded doc rect, then assert the edge is
        // covered and both present paths agree. Finding no such zoom is itself a
        // failure, so the check cannot pass without exercising the seam.
        bool ceGapped = false;
        bool ceEdgeRed = true;
        bool ceIdentical = true;
        for (int step = 0; step < 40 && !ceGapped; ++step) {
            const double zoom = 0.5 + step * 0.005;
            ceCanvas->setZoom(zoom, QPointF(ceSize.width() / 2.0, ceSize.height() / 2.0));
            const QPointF offset = ceCanvas->offset();
            const QRect device = canvasDeviceRect(ceCanvas);
            const bool gapRight = qRound(offset.x() + ceSrcW * zoom)
                                  > qRound(offset.x()) + int(ceSrcW * zoom);
            const bool gapBottom = qRound(offset.y() + ceSrcH * zoom)
                                   > qRound(offset.y()) + int(ceSrcH * zoom);
            if (!gapRight && !gapBottom) {
                continue;
            }
            ceGapped = true;

            QImage cached(ceSize, QImage::Format_ARGB32);
            cached.fill(Qt::transparent);
            ceCanvas->render(&cached);
            const QPoint rightEdge(device.right(), device.center().y());
            const QPoint bottomEdge(device.center().x(), device.bottom());
            if (gapRight
                && (!ceInBounds(rightEdge)
                    || cached.pixelColor(rightEdge).rgb() != ceRed.rgb())) {
                ceEdgeRed = false;
            }
            if (gapBottom
                && (!ceInBounds(bottomEdge)
                    || cached.pixelColor(bottomEdge).rgb() != ceRed.rgb())) {
                ceEdgeRed = false;
            }

            ceCanvas->setPresentCacheEnabledForTest(false);
            QImage direct(ceSize, QImage::Format_ARGB32);
            direct.fill(Qt::transparent);
            ceCanvas->render(&direct);
            ceCanvas->setPresentCacheEnabledForTest(true);
            if (cached != direct) {
                ceIdentical = false;
            }
        }

        ST_BEGIN("present_cache_edge");
        ST_PASS("present_cache_edge gapped=%d edge_red=%d identical=%d", ceGapped ? 1 : 0,
                ceEdgeRed ? 1 : 0, ceIdentical ? 1 : 0);
        if (!ceGapped || !ceEdgeRed || !ceIdentical) {
            return pictura::selfTest().fail(295, "present cache edge seam");
        }
        frame.closeDocument(ceDoc, false);
        return 0;
}
