// ImageView's document: the image or the document size it presents, the
// zoom/pan view state over it, the region blits a paint delivers, and a
// filter dialog's reduced preview. Split from image_view.cpp to keep each
// translation unit small.

#include "image_view.h"

#include "canvas_range.h"
#include "pictura_debug_timing.h"

#include <QtGui/QPainter>

#include <algorithm>
#include <cmath>
#include <cstring>

namespace pictura {

namespace {
constexpr double kMinZoom = 0.01;
constexpr double kMaxZoom = 32.0;
} // namespace

void ImageView::replaceImage(const QImage& image)
{
    image_ = image;
    docSize_ = image.size();
    ++frameKey_;
    presentCache_.valid = false;
    // An empty image is the first half of a document-size replacement; leave the
    // offset for replaceDocument to clamp once the new size is known.
    if (!docSize_.isEmpty()) {
        clampOffset();
    }
    update();
}

void ImageView::setDocument(const QSize& size)
{
    setImage(QImage());
    docSize_ = size;
    if (!docSize_.isEmpty()) {
        pendingSettle_ = true;
        applyInitialView();
    }
}

void ImageView::replaceDocument(const QSize& size)
{
    replaceImage(QImage());
    docSize_ = size;
    clampOffset();
    update();
    emit viewChanged();
}

bool ImageView::smoothSamplingForZoom(double zoom)
{
    return zoom < 2.0;
}

void ImageView::setFilterPreview(const QImage& picture, const QRect& area)
{
    filterPreview_ = picture;
    filterPreviewArea_ = area;
    update();
}

void ImageView::clearFilterPreview()
{
    if (filterPreview_.isNull()) {
        return;
    }
    filterPreview_ = QImage();
    filterPreviewArea_ = QRect();
    update();
}

// Drawn in document space under the document clip, over the presented base.
void ImageView::paintFilterPreview(QPainter& painter)
{
    if (filterPreview_.isNull() || filterPreviewArea_.isEmpty()) {
        return;
    }
    painter.save();
    painter.setRenderHint(QPainter::SmoothPixmapTransform, true);
    painter.drawImage(QRectF(filterPreviewArea_), filterPreview_);
    painter.restore();
}

QImage ImageView::image() const
{
    if (!image_.isNull() || docSize_.isEmpty() || !levelProvider_.crop) {
        return image_;
    }
    return levelProvider_.crop(0, 0, 0, docSize_.width(), docSize_.height());
}

void ImageView::blitRegion(const QImage& region, int x, int y)
{
    pictura::ScopedTimer blitTimer("cxx_blitRegion");
    if (region.isNull() || region.width() <= 0 || region.height() <= 0 || docSize_.isEmpty()) {
        return;
    }
    // A document canvas holds no image: the region already landed in the
    // pyramid the next paint crops.
    if (image_.isNull()) {
        presentCache_.valid = false;
        update();
        return;
    }
    // Copy the region rows straight into the canvas instead of constructing a
    // QPainter on the full-resolution image: a painter on a shared buffer
    // detaches the whole image. Own the buffer once, then every later dab is an
    // in-place row copy. CompositionMode_Source semantics (replace, clipped to
    // the image rect) are reproduced by the clamped memcpy below.
    if (!image_.isDetached()) {
        pictura::ScopedTimer detachTimer("cxx_blit_detach_image");
        image_ = image_.copy();
    }
    QImage src = region;
    if (src.format() != image_.format()) {
        src = src.convertToFormat(image_.format());
    }
    const int x0 = std::max(0, x);
    const int y0 = std::max(0, y);
    const int x1 = std::min(docSize_.width(), x + src.width());
    const int y1 = std::min(docSize_.height(), y + src.height());
    if (x1 > x0 && y1 > y0) {
        const int bpp = std::max(1, image_.depth() / 8);
        const int rowBytes = (x1 - x0) * bpp;
        const int sx = x0 - x;
        const int sy = y0 - y;
        for (int row = 0; row < y1 - y0; ++row) {
            std::memcpy(image_.scanLine(y0 + row) + x0 * bpp,
                        src.constScanLine(sy + row) + sx * bpp, rowBytes);
        }
    }
    // The cached level crop no longer matches the patched image; the next paint
    // re-crops the updated pyramid level.
    presentCache_.valid = false;
    update();
}

void ImageView::setImage(const QImage& image)
{
    image_ = image;
    docSize_ = image.size();
    ++frameKey_;
    presentCache_.valid = false;
    userAdjusted_ = false;
    pendingSettle_ = !docSize_.isEmpty();
    if (docSize_.isEmpty()) {
        zoom_ = 1.0;
        offset_ = QPointF();
        update();
        emit viewChanged();
        return;
    }
    applyInitialView();
}

void ImageView::applyInitialView()
{
    if (docSize_.isEmpty()) {
        return;
    }
    if (docSize_.width() > width() || docSize_.height() > height()) {
        const double fit = std::min(double(width()) / docSize_.width(),
                                    double(height()) / docSize_.height())
                           * 0.95;
        zoom_ = std::clamp(fit, kMinZoom, kMaxZoom);
    } else {
        zoom_ = 1.0;
    }
    centreImage();
    clampOffset();
    emit zoomChanged(zoom_);
    emit viewChanged();
    update();
    userAdjusted_ = true;
}

void ImageView::centreImage()
{
    offset_ = QPointF((width() - docSize_.width() * zoom_) / 2.0,
                      (height() - docSize_.height() * zoom_) / 2.0);
    clampOffset();
}

void ImageView::clampOffset()
{
    if (docSize_.isEmpty()) {
        offset_ = QPointF();
        return;
    }
    const OffsetRange range = offsetRangeFor(
        QSizeF(docSize_.width(), docSize_.height()), zoom_, QSizeF(width(), height()), freePan_,
        kCanvasRevealMarginPx, snapToWorkspace_);
    offset_.setX(std::clamp(offset_.x(), range.minX, range.maxX));
    offset_.setY(std::clamp(offset_.y(), range.minY, range.maxY));
}

void ImageView::zoomAt(const QPointF& cursor, int angleDelta)
{
    const double factor = std::pow(1.0015, angleDelta);
    // Zoom-in keeps the point under the cursor fixed; zoom-out steps at the
    // canvas centre so the image does not drift toward (and off) the pointer.
    const QPointF anchor = angleDelta < 0 ? QPointF(width() / 2.0, height() / 2.0) : cursor;
    setZoom(zoom_ * factor, anchor);
}

void ImageView::panBy(const QPointF& delta)
{
    userAdjusted_ = true;
    pendingSettle_ = false;
    offset_ += delta;
    clampOffset();
    update();
    emit viewChanged();
}

void ImageView::setOffset(const QPointF& offset)
{
    userAdjusted_ = true;
    pendingSettle_ = false;
    offset_ = offset;
    clampOffset();
    update();
    emit viewChanged();
}

void ImageView::zoomIn()
{
    setZoom(zoom_ * 1.2, QPointF(width() / 2.0, height() / 2.0));
}

void ImageView::zoomOut()
{
    setZoom(zoom_ / 1.2, QPointF(width() / 2.0, height() / 2.0));
}

void ImageView::setZoom(double zoom, const QPointF& anchor)
{
    userAdjusted_ = true;
    pendingSettle_ = false;
    const double target = std::clamp(zoom, kMinZoom, kMaxZoom);
    const double factor = (zoom_ > 0.0) ? target / zoom_ : 1.0;
    offset_ = anchor - (anchor - offset_) * factor;
    zoom_ = target;
    clampOffset();
    emit zoomChanged(zoom_);
    emit viewChanged();
    update();
}

void ImageView::fitOnScreen()
{
    userAdjusted_ = false;
    if (docSize_.isEmpty()) {
        return;
    }
    const double fit = std::min(double(width()) / docSize_.width(),
                                double(height()) / docSize_.height())
                       * 0.95;
    zoom_ = std::clamp(fit, kMinZoom, kMaxZoom);
    offset_ = QPointF((width() - docSize_.width() * zoom_) / 2.0,
                      (height() - docSize_.height() * zoom_) / 2.0);
    clampOffset();
    emit zoomChanged(zoom_);
    emit viewChanged();
    update();
}

void ImageView::actualPixels()
{
    // 100% is an explicit user choice: keep it when the workspace scrollbars
    // appear and resize the canvas instead of refitting on the next resize.
    userAdjusted_ = true;
    pendingSettle_ = false;
    zoom_ = 1.0;
    offset_ = docSize_.isEmpty()
                  ? QPointF()
                  : QPointF((width() - docSize_.width()) / 2.0,
                            (height() - docSize_.height()) / 2.0);
    clampOffset();
    emit zoomChanged(zoom_);
    emit viewChanged();
    update();
}

} // namespace pictura
