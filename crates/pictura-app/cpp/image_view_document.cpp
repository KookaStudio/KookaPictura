// ImageView's document: the image or the document size it presents, the
// region blits a paint delivers, and a filter dialog's reduced preview. Split
// from image_view.cpp to keep each translation unit small.

#include "image_view.h"

#include "pictura_debug_timing.h"

#include <QtGui/QPainter>

#include <algorithm>
#include <cstring>

namespace pictura {

void ImageView::replaceImage(const QImage& image)
{
    image_ = image;
    docSize_ = image.size();
    ++frameKey_;
    presentCache_.valid = false;
    update();
}

void ImageView::setDocument(const QSize& size)
{
    setImage(QImage());
    docSize_ = size;
    if (!docSize_.isEmpty()) {
        applyInitialView();
    }
}

void ImageView::replaceDocument(const QSize& size)
{
    replaceImage(QImage());
    docSize_ = size;
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

} // namespace pictura
