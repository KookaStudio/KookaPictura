// ImageView's present cache: the cached level crop a document canvas draws, its
// damage patch, and the reuse of that crop when only the visible rectangle
// changed at the same level (a pan, or a zoom step inside the level's band).
// Split from image_view.cpp to keep each translation unit small.

#include "image_view.h"

#include <algorithm>
#include <cstring>

namespace pictura {

void ImageView::dropPresentCache()
{
    presentCache_.valid = false;
    presentCache_.level = -1;
    presentCache_.crop = QImage();
    presentCache_.levelRect = QRect();
}

// Reuse the cached crop when the visible rectangle changed at the cached level
// (a pan, or a zoom step that stays in the band): the overlap is already the
// right level's pixels, so copy it and crop only the newly exposed band(s). The
// union of the overlap and the bands covers `sourceRect` exactly, so every
// pixel of the new crop comes from the level provider.
bool ImageView::patchPresentCache(int level, const QRect& docRect, const QRect& sourceRect)
{
    const QRect oldLevel = presentCache_.levelRect;
    const QRect overlap = sourceRect.intersected(oldLevel);
    if (overlap.isEmpty()) {
        return false;
    }
    QImage next(sourceRect.size(), presentCache_.crop.format());
    if (next.isNull()) {
        return false;
    }
    const int bpp = std::max(1, next.depth() / 8);
    const QPoint from = overlap.topLeft() - oldLevel.topLeft();
    const QPoint to = overlap.topLeft() - sourceRect.topLeft();
    for (int row = 0; row < overlap.height(); ++row) {
        std::memcpy(next.scanLine(to.y() + row) + to.x() * bpp,
                    presentCache_.crop.constScanLine(from.y() + row) + from.x() * bpp,
                    overlap.width() * bpp);
    }
    // Left and right full-height bands, then top and bottom over the shared
    // columns. At most two are non-empty for a translation; a zoom-out uses up
    // to four, and a zoom-in none.
    const int x0 = std::max(sourceRect.left(), oldLevel.left());
    const int x1 = std::min(sourceRect.right(), oldLevel.right());
    const QRect bands[4] = {
        QRect(sourceRect.left(), sourceRect.top(), oldLevel.left() - sourceRect.left(),
              sourceRect.height()),
        QRect(oldLevel.right(), sourceRect.top(), sourceRect.right() - oldLevel.right(),
              sourceRect.height()),
        QRect(x0, sourceRect.top(), x1 - x0, oldLevel.top() - sourceRect.top()),
        QRect(x0, oldLevel.bottom(), x1 - x0, sourceRect.bottom() - oldLevel.bottom()),
    };
    for (const QRect& band : bands) {
        const QRect area = band.intersected(sourceRect);
        if (area.width() <= 0 || area.height() <= 0) {
            continue;
        }
        const QImage part =
            levelProvider_.crop(level, area.x(), area.y(), area.width(), area.height());
        if (part.isNull() || part.size() != area.size() || part.format() != next.format()) {
            return false;
        }
        const QPoint dst = area.topLeft() - sourceRect.topLeft();
        for (int row = 0; row < area.height(); ++row) {
            std::memcpy(next.scanLine(dst.y() + row) + dst.x() * bpp, part.constScanLine(row),
                        area.width() * bpp);
        }
    }
    presentCache_.crop = next;
    presentCache_.docRect = docRect;
    presentCache_.levelRect = sourceRect;
    return true;
}

const QImage* ImageView::presentCrop(int level, const QRect& docRect, const QRect& sourceRect)
{
    if (!levelProvider_.crop) {
        return nullptr;
    }
    const qint64 key = frameKey_;
    const quint64 revision =
        levelProvider_.canvasRevision ? levelProvider_.canvasRevision() : quint64(0);
    if (presentCacheEnabledForTest_ && presentCache_.valid && presentCache_.key == key
        && presentCache_.revision == revision && presentCache_.level == level
        && !presentCache_.crop.isNull()) {
        if (presentCache_.docRect == docRect) {
            return &presentCache_.crop;
        }
        if (patchPresentCache(level, docRect, sourceRect)) {
            return &presentCache_.crop;
        }
    }
    ++presentCache_.rebuilds;
    QImage crop = levelProvider_.crop(level, sourceRect.x(), sourceRect.y(), sourceRect.width(),
                                      sourceRect.height());
    if (crop.isNull() || crop.width() <= 0 || crop.height() <= 0) {
        presentCache_.valid = false;
        return nullptr;
    }
    presentCache_.crop = crop;
    presentCache_.key = key;
    presentCache_.revision = revision;
    presentCache_.level = level;
    presentCache_.docRect = docRect;
    presentCache_.levelRect = sourceRect;
    presentCache_.valid = presentCacheEnabledForTest_;
    return &presentCache_.crop;
}

} // namespace pictura
