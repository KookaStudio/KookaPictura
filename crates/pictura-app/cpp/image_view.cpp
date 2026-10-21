#include "image_view.h"

#include "api/selection_api.h"
#include "icons.h"
#include "pictura_debug_timing.h"

#include <QtCore/QDebug>
#include <QtCore/QRect>
#include <QtCore/QStringList>
#include <QtCore/QTimer>
#include <QtCore/QtNumeric>
#include <QtGui/QContextMenuEvent>
#include <QtGui/QFontMetrics>
#include <QtGui/QKeyEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPixmap>
#include <QtGui/QWheelEvent>

#include <algorithm>
#include <cmath>
#include <cstring>

namespace pictura {

namespace {

// The modifier flag for a modifier key, or 0 for any other key.
int modifierFlag(int key)
{
    return key == Qt::Key_Alt     ? int(Qt::AltModifier)
        : key == Qt::Key_Shift    ? int(Qt::ShiftModifier)
        : key == Qt::Key_Control  ? int(Qt::ControlModifier)
        : key == Qt::Key_Meta     ? int(Qt::MetaModifier)
                                  : 0;
}

// 2x2-cell tile reused for every transparency fill. Built lazily on the GUI
// thread the first time a document is painted.
const QPixmap& transparencyTile()
{
    static const QPixmap tile = [] {
        const int cell = ImageView::transparencyCellSize();
        QPixmap pm(2 * cell, 2 * cell);
        pm.fill(ImageView::transparencyColorA());
        QPainter p(&pm);
        p.fillRect(cell, 0, cell, cell, ImageView::transparencyColorB());
        p.fillRect(0, cell, cell, cell, ImageView::transparencyColorB());
        return pm;
    }();
    return tile;
}
} // namespace

int ImageView::transparencyCellSize()
{
    return 8;
}

QColor ImageView::transparencyColorA()
{
    return QColor(255, 255, 255);
}

QColor ImageView::transparencyColorB()
{
    return QColor(204, 204, 204);
}

ImageView::ImageView(QWidget* parent)
    : QWidget(parent)
{
    setMinimumSize(320, 240);
    laidOutSize_ = size();
    setFocusPolicy(Qt::StrongFocus);
    setMouseTracking(true);
    // The workspace default pointer: the bare arrow the tool cursors are built
    // on. Panels, menus, and dialogs keep the system cursor.
    setCursor(pictura::cursor(QStringLiteral("cursor.workspace"), 2, 2));
    antsTimer_ = new QTimer(this);
    antsTimer_->setInterval(120);
    connect(antsTimer_, &QTimer::timeout, this, [this]() {
        antsPhase_ = (antsPhase_ + 1) % 8;
        update();
    });
}

void ImageView::setCanvasColor(const QColor& color)
{
    canvasColor_ = color;
    update();
}

void ImageView::setCanvasBackdrop(const QColor& color, bool opaque)
{
    if (canvasBackdrop_ == color && canvasBackdropOpaque_ == opaque) return;
    canvasBackdrop_ = color;
    canvasBackdropOpaque_ = opaque;
    update();
}

void ImageView::setPanEnabled(bool enabled)
{
    // A transient Space pan owns pan state while held: a tool-state refresh that
    // recomputes pan-enabled must not drop it mid-hold.
    if (spacePan_ && !enabled) {
        return;
    }
    panEnabled_ = enabled;
    if (!enabled) {
        panning_ = false;
    }
}

void ImageView::setFreePan(bool enabled)
{
    if (freePan_ == enabled) {
        return;
    }
    freePan_ = enabled;
    clampOffset();
    update();
    emit viewChanged();
}

void ImageView::setSnapToWorkspace(bool enabled)
{
    if (snapToWorkspace_ == enabled) {
        return;
    }
    snapToWorkspace_ = enabled;
    clampOffset();
    update();
    emit viewChanged();
}

bool ImageView::workspaceSnapLocked() const
{
    if (!snapToWorkspace_ || docSize_.isEmpty() || zoom_ <= 0.0) {
        return false;
    }
    return docSize_.width() * zoom_ <= width() && docSize_.height() * zoom_ <= height();
}

void ImageView::setSpacePan(bool on)
{
    if (spacePan_ == on) {
        return;
    }
    spacePan_ = on;
    if (on) {
        setPanEnabled(true);
        // Keep the closed hand if a middle-button pan is already in progress.
        setCursor(panning_ ? Qt::ClosedHandCursor : Qt::OpenHandCursor);
    } else {
        panning_ = false;
    }
    update();
}

void ImageView::setOverlayPolygon(const QPolygonF& polygon)
{
    overlayPolygon_ = polygon;
    update();
}

void ImageView::clearOverlay()
{
    overlayPolygon_.clear();
    update();
}

void ImageView::setSelectionContour(const selection_api::SelectionContour& contour)
{
    selectionContours_ = contour.loops;
    selectionContourOffset_ = QPointF();
    updateAntsTimer();
    update();
}

void ImageView::clearSelectionContour()
{
    selectionContours_.clear();
    selectionContourOffset_ = QPointF();
    updateAntsTimer();
    update();
}

void ImageView::setSelectionContourOffset(const QPointF& offset)
{
    if (selectionContourOffset_ == offset) {
        return;
    }
    selectionContourOffset_ = offset;
    update();
}

void ImageView::setSelectionEdgesVisible(bool on)
{
    selectionEdgesVisible_ = on;
    updateAntsTimer();
    update();
}

void ImageView::setSelectionPreview(const QList<QPolygonF>& loops, bool closed, bool solid)
{
    previewContours_ = loops;
    selectionPreviewClosed_ = closed;
    selectionPreviewSolid_ = solid;
    updateAntsTimer();
    update();
}

void ImageView::setSelectionPreviewOrigin(const QPointF& imagePos)
{
    previewOrigin_ = imagePos;
    previewOriginActive_ = true;
    update();
}

void ImageView::clearSelectionPreview()
{
    previewContours_.clear();
    previewOriginActive_ = false;
    selectionPreviewClosed_ = true;
    selectionPreviewSolid_ = false;
    updateAntsTimer();
    update();
}

void ImageView::updateAntsTimer()
{
    const bool run = (!selectionContours_.isEmpty() && selectionEdgesVisible_)
                     || !previewContours_.isEmpty() || !moveOutline_.isEmpty();
    const bool active = run && isVisible();
    if (active && !antsTimer_->isActive()) {
        antsTimer_->start();
    } else if (!active && antsTimer_->isActive()) {
        antsTimer_->stop();
    }
}

void ImageView::beginMovePreview(const QImage& base, const QImage& layer, const QPointF& layerPos,
                                 double opacity)
{
    moveBase_ = base;
    moveLayer_ = layer;
    moveLayerPos_ = layerPos;
    moveDelta_ = QPointF();
    moveOpacity_ = opacity;
    movePreviewActive_ = true;
    update();
}

void ImageView::setMovePreviewDelta(const QPointF& delta)
{
    moveDelta_ = delta;
    update();
}

void ImageView::endMovePreview()
{
    setMoveOutline(QRectF());
    movePreviewActive_ = false;
    moveBase_ = QImage();
    moveLayer_ = QImage();
    moveDelta_ = QPointF();
    moveOpacity_ = 1.0;
    update();
}

void ImageView::beginTransformPreview(const QImage& base, const QImage& layer,
                                      const QPointF& layerPos, double opacity)
{
    moveBase_ = base;
    moveLayer_ = layer;
    moveLayerPos_ = layerPos;
    moveDelta_ = QPointF();
    moveOpacity_ = opacity;
    movePreviewActive_ = true;
    transformActive_ = true;
    transformScaleX_ = 1.0;
    transformScaleY_ = 1.0;
    transformAngle_ = 0.0;
    transformDx_ = 0.0;
    transformDy_ = 0.0;
    transformProjective_ = QTransform();
    transformProjectiveCoefficients_ = {};
    transformProjectiveSource_ = QRectF();
    transformWarped_ = WarpedTransformPreview();
    transformWarpedStale_ = true;
    transformProjectiveActive_ = false;
    transformQuad_.clear();
    update();
}

void ImageView::setTransformPreviewProjective(const std::array<double, 9>& coefficients,
                                              const QRectF& sourceRect)
{
    transformProjective_ = QTransform(coefficients[0], coefficients[1], coefficients[2],
                                      coefficients[3], coefficients[4], coefficients[5],
                                      coefficients[6], coefficients[7], coefficients[8]);
    transformProjectiveCoefficients_ = coefficients;
    transformProjectiveSource_ = sourceRect;
    transformWarpedStale_ = true;
    transformProjectiveActive_ = true;
    update();
}

void ImageView::setTransformPreview(double scaleX, double scaleY, double angleRadians, double dx,
                                    double dy)
{
    transformProjectiveActive_ = false;
    transformScaleX_ = scaleX;
    transformScaleY_ = scaleY;
    transformAngle_ = angleRadians;
    transformDx_ = dx;
    transformDy_ = dy;
    update();
}

void ImageView::setTransformQuad(const QPolygonF& corners)
{
    transformQuad_ = corners;
    update();
}

void ImageView::clearTransformPreview()
{
    transformActive_ = false;
    transformProjective_ = QTransform();
    transformProjectiveCoefficients_ = {};
    transformProjectiveSource_ = QRectF();
    transformWarped_ = WarpedTransformPreview();
    transformWarpedStale_ = true;
    transformProjectiveActive_ = false;
    transformQuad_.clear();
    transformPivotVisible_ = false;
    transformWarp_ = transform_api::WarpOverlay();
    movePreviewActive_ = false;
    moveBase_ = QImage();
    moveLayer_ = QImage();
    moveDelta_ = QPointF();
    moveOpacity_ = 1.0;
    update();
}

void ImageView::setDragSizeHint(const QString& text, const QPointF& imagePos)
{
    dragSizeText_ = text;
    dragSizeImagePos_ = imagePos;
    dragSizeActive_ = !text.isEmpty();
    update();
}

void ImageView::clearDragSizeHint()
{
    dragSizeText_.clear();
    dragSizeImagePos_ = QPointF();
    dragSizeActive_ = false;
    update();
}

QPointF ImageView::widgetToImage(const QPointF& widgetPos) const
{
    return (viewRotation().inverted().map(widgetPos) - offset_) / zoom_;
}

QPointF ImageView::imageToWidget(const QPointF& imagePos) const
{
    return viewRotation().map(imagePos * zoom_ + offset_);
}

QTransform ImageView::viewRotation() const
{
    if (rotation_ == 0.0) {
        return QTransform();
    }
    const QPointF centre(width() / 2.0, height() / 2.0);
    return QTransform::fromTranslate(-centre.x(), -centre.y()) * QTransform().rotate(rotation_)
        * QTransform::fromTranslate(centre.x(), centre.y());
}

QRectF ImageView::viewRect() const
{
    return viewRotation().inverted().mapRect(QRectF(rect()));
}

void ImageView::setRotation(double degrees)
{
    double normalised = std::fmod(degrees, 360.0);
    if (normalised <= -180.0) {
        normalised += 360.0;
    } else if (normalised > 180.0) {
        normalised -= 360.0;
    }
    if (normalised == rotation_) {
        return;
    }
    rotation_ = normalised;
    presentCache_.valid = false;
    update();
    emit viewChanged();
}

void ImageView::setCompassVisible(bool visible)
{
    if (compassVisible_ != visible) {
        compassVisible_ = visible;
        update();
    }
}

// The compass in widget space: a ring at the canvas centre whose red needle
// points to the document's top edge however the canvas is turned.
void ImageView::paintCompass(QPainter& painter)
{
    painter.save();
    painter.resetTransform();
    painter.setClipping(false);
    painter.setRenderHint(QPainter::Antialiasing, true);
    const QPointF centre(width() / 2.0, height() / 2.0);
    const double radius = std::min(60.0, std::min(width(), height()) / 4.0);
    painter.setPen(QPen(QColor(255, 255, 255, 200), 2));
    painter.setBrush(QColor(0, 0, 0, 90));
    painter.drawEllipse(centre, radius, radius);
    const QTransform turn = QTransform().rotate(rotation_);
    const QPointF north = turn.map(QPointF(0.0, -radius * 0.85));
    const QPointF side = turn.map(QPointF(radius * 0.12, 0.0));
    painter.setPen(Qt::NoPen);
    painter.setBrush(QColor(0xe0, 0x20, 0x20));
    painter.drawPolygon(QPolygonF({centre + north, centre + side, centre - side}));
    painter.setBrush(QColor(255, 255, 255, 220));
    painter.drawPolygon(QPolygonF({centre - north, centre + side, centre - side}));
    painter.restore();
}

void ImageView::setPresentCacheEnabledForTest(bool enabled)
{
    presentCacheEnabledForTest_ = enabled;
}

void ImageView::setPresentLevelCropForTest(bool enabled)
{
    presentLevelCropForTest_ = enabled;
    presentCache_.valid = false;
}

void ImageView::setLevelProvider(LevelProvider provider)
{
    levelProvider_ = std::move(provider);
    presentCache_.valid = false;
}

int ImageView::presentLevelForZoom(double zoom, int levelCount)
{
    if (levelCount <= 0) {
        return 0;
    }
    int level = 0;
    while (level + 1 < levelCount && 1.0 / double(1ll << (level + 1)) >= zoom) {
        ++level;
    }
    return level;
}

QRectF ImageView::visibleDocumentRect() const
{
    if (docSize_.isEmpty() || zoom_ <= 0.0) {
        return QRectF();
    }
    const QRectF view = viewRect();
    const QPointF tl = (view.topLeft() - offset_) / zoom_;
    const QPointF br = (view.bottomRight() - offset_) / zoom_;
    const QRectF visible(QPointF(std::min(tl.x(), br.x()), std::min(tl.y(), br.y())),
                         QPointF(std::max(tl.x(), br.x()), std::max(tl.y(), br.y())));
    return visible.intersected(QRectF(0.0, 0.0, docSize_.width(), docSize_.height()));
}

const QImage* ImageView::presentCrop(QRect& docRect)
{
    docRect = QRect();
    // A live stroke patches the same level-0/pyramid the idle present crops (the
    // app's regional refresh folds every dab into the damage account), so a
    // stroke in progress presents through the level crop too, not the
    // full-resolution image.
    // No provider or no crop source: nothing to present, and no cache to touch.
    if (!presentLevelCropForTest_ || !levelProvider_.crop || docSize_.isEmpty()) {
        return nullptr;
    }
    const int levels = levelProvider_.levelCount ? levelProvider_.levelCount() : 0;
    if (levels <= 0) {
        return nullptr;
    }
    // While a large brush previews, the crop must come from the level the
    // preview patched, whatever the zoom would otherwise select: only that level
    // carries the in-progress pixels, and the commit's level-0 rebuild hands the
    // zoom-selected level back.
    int level = presentLevelForZoom(zoom_, levels);
    if (levelProvider_.previewLevel) {
        const int previewLevel = levelProvider_.previewLevel();
        if (previewLevel > 0 && previewLevel < levels) {
            level = previewLevel;
        } else if (previewLevel < 0) {
            // A GPU stroke presents level-0 regions, so crop level 0 while it
            // is live.
            level = 0;
        }
    }
    const QSize levelSize =
        levelProvider_.levelSize ? levelProvider_.levelSize(level) : QSize();
    if (levelSize.isEmpty()) {
        return nullptr;
    }
    const QRectF visible = visibleDocumentRect();
    if (visible.isEmpty()) {
        return nullptr;
    }
    const int scale = 1 << level;
    const double inv = 1.0 / double(scale);
    // One level-pixel margin so interpolated edge samples have neighbours.
    const int lx0 = std::max(0, int(std::floor(visible.left() * inv)) - 1);
    const int ly0 = std::max(0, int(std::floor(visible.top() * inv)) - 1);
    const int lx1 = std::min(levelSize.width(), int(std::ceil(visible.right() * inv)) + 1);
    const int ly1 = std::min(levelSize.height(), int(std::ceil(visible.bottom() * inv)) + 1);
    if (lx1 <= lx0 || ly1 <= ly0) {
        return nullptr;
    }
    const QRect rect(lx0 * scale, ly0 * scale, (lx1 - lx0) * scale, (ly1 - ly0) * scale);
    const qint64 key = frameKey_;
    const quint64 revision =
        levelProvider_.canvasRevision ? levelProvider_.canvasRevision() : quint64(0);
    if (presentCacheEnabledForTest_ && presentCache_.valid && presentCache_.key == key
        && presentCache_.revision == revision && presentCache_.level == level
        && presentCache_.docRect == rect && !presentCache_.crop.isNull()) {
        docRect = presentCache_.docRect;
        return &presentCache_.crop;
    }
    QImage crop = levelProvider_.crop(level, lx0, ly0, lx1 - lx0, ly1 - ly0);
    ++presentCache_.rebuilds;
    if (crop.isNull() || crop.width() <= 0 || crop.height() <= 0) {
        presentCache_.valid = false;
        return nullptr;
    }
    presentCache_.crop = crop;
    presentCache_.key = key;
    presentCache_.revision = revision;
    presentCache_.level = level;
    presentCache_.docRect = rect;
    presentCache_.valid = true;
    docRect = rect;
    return &presentCache_.crop;
}

void ImageView::paintEvent(QPaintEvent*)
{
    pictura::ScopedTimer paintTimer("cxx_paintEvent");
    QPainter painter(this);
    painter.fillRect(rect(), canvasColor_);
    if (docSize_.isEmpty()) {
        return;
    }
    // Everything below draws in the view frame; Rotate View turns it here.
    const QTransform rotation = viewRotation();
    painter.setTransform(rotation);

    // The canvas frame follows the box at every angle (backdrop pads past it).
    const bool straightening = cropStraighten_ != 0.0 && !cropBox_.isNull();
    const QRectF canvasDoc = cropBox_.isNull()
                                 ? QRectF(0.0, 0.0, docSize_.width(), docSize_.height())
                                 : cropCanvasImageRect();

    // Checkerboard in screen space, anchored to the document origin and
    // clipped to the document rect so it never spills onto the canvas.
    const QRectF docRect(offset_.x() + canvasDoc.x() * zoom_, offset_.y() + canvasDoc.y() * zoom_,
                         canvasDoc.width() * zoom_, canvasDoc.height() * zoom_);
    // One integer device rect for the document, rounded the way fillRect rounds,
    // so the checkerboard and the cached base share an exact boundary. Without
    // this the floor-sized present cache fell a pixel short at the right/bottom
    // edge and let the checkerboard show through an opaque canvas-sized layer.
    const QRect docDevice(qRound(docRect.left()), qRound(docRect.top()),
                          qRound(docRect.right()) - qRound(docRect.left()),
                          qRound(docRect.bottom()) - qRound(docRect.top()));
    const QRect checkerRect = docDevice.intersected(viewRect().toAlignedRect());
    if (!checkerRect.isEmpty()) {
        painter.setBrushOrigin(docDevice.topLeft());
        // A Background layer makes the canvas opaque: the area beyond the
        // content shows the background colour, not the transparency tile.
        painter.fillRect(checkerRect,
                         canvasBackdropOpaque_ ? QBrush(canvasBackdrop_)
                                               : QBrush(transparencyTile()));
    }

    painter.translate(offset_);
    painter.scale(zoom_, zoom_);
    // Crop the document content (base, preview, overlays) to the image rect.
    // The brush ring below is drawn outside this scope so it can render past
    // the document edge while staying inside the canvas widget.
    const QRectF docClip = canvasDoc;
    painter.save();
    painter.setClipRect(docClip);

    if (transformActive_ && !moveBase_.isNull()) {
        // Free Transform: the base is fixed; the layer is drawn under the live
        // similarity transform about its original centre.
        painter.drawImage(QPointF(0.0, 0.0), moveBase_);
        painter.setOpacity(moveOpacity_);
        painter.save();
        if (transformProjectiveActive_) {
            // The projective map is in the layer image's own coordinates. The
            // document-space form has a large translation and, for a strong
            // perspective, a vanishing line close to the image; Qt's raster
            // engine can drop the whole image in that form.
            painter.translate(moveLayerPos_);
            refreshTransformWarp();
            if (!transformWarped_.image.isNull()) {
                painter.drawImage(QRectF(transformWarped_.origin,
                                         QSizeF(transformWarped_.image.size())
                                             / transformWarped_.scale),
                                  transformWarped_.image);
            } else if (!transformWarped_.crossed) {
                painter.setTransform(transformProjective_, true);
                painter.drawImage(QPointF(0.0, 0.0), moveLayer_);
            }
        } else {
            painter.setTransform(transformPreviewMatrix(), true);
            painter.drawImage(moveLayerPos_, moveLayer_);
        }
        painter.restore();
        painter.setOpacity(1.0);
        presentCacheRebuiltLastPaint_ = false;
    } else if (movePreviewActive_ && !moveBase_.isNull()) {
        // Draw the base and moving layer straight through the pan/zoom
        // transform. A move preview is transient and its sources change every
        // drag, so a scaled present cache would be rebuilt on the first frame
        // of every drag (a full-size conversion copy: 16 MP ≈ 48 ms at 4000²
        // at 100%, 64 MP ≈ 4× that at 200%). Drawing directly is a steady
        // ~3–4 ms/frame at any zoom and removes the drag-start hitch.
        painter.drawImage(QPointF(0.0, 0.0), moveBase_);
        painter.setOpacity(moveOpacity_);
        painter.drawImage(moveLayerPos_ + moveDelta_, moveLayer_);
        painter.setOpacity(1.0);
        presentCacheRebuiltLastPaint_ = false;
    } else {
        const int before = presentCache_.rebuilds;
        QRect cropDoc;
        const QImage* crop = presentCrop(cropDoc);
        painter.save();
        painter.setTransform(rotation);
        painter.setRenderHint(QPainter::SmoothPixmapTransform,
                              rotation_ != 0.0 || cropStraighten_ != 0.0
                                  || smoothSamplingForZoom(zoom_));
        // Clip to the (possibly expanded) canvas before composing the straighten
        // rotation: the clip must stay axis-aligned in the view frame.
        painter.setClipRect(docDevice);
        // Straighten preview rotates the content about the session pivot while
        // the box stays axis-aligned; composed into the view transform.
        if (straightening) {
            QTransform about;
            const QPointF c = cropStraightenPivot_ * zoom_ + offset_;
            about.translate(c.x(), c.y());
            about.rotate(cropStraighten_);
            about.translate(-c.x(), -c.y());
            painter.setTransform(rotation * about);
        }
        if (!cropContentOffset_.isNull()) {
            // Modern content pan: shift the composite, pulled from image() at zoom.
            if (const QImage composite = image(); !composite.isNull()) {
                painter.drawImage(QRectF(offset_ + cropContentOffset_ * zoom_,
                                         QSizeF(composite.size()) * zoom_), composite);
            }
        } else if (singleChannel() >= 0 && crop) {
            const QRectF target(offset_.x() + cropDoc.x() * zoom_,
                                offset_.y() + cropDoc.y() * zoom_, cropDoc.width() * zoom_,
                                cropDoc.height() * zoom_);
            painter.drawImage(target, channelImage(*crop, crop->cacheKey()));
        } else if (singleChannel() >= 0 && !image_.isNull()) {
            const QRectF visible = visibleDocumentRect();
            painter.drawImage(QRectF(offset_.x() + visible.x() * zoom_,
                                     offset_.y() + visible.y() * zoom_, visible.width() * zoom_,
                                     visible.height() * zoom_),
                              channelImage(image_, image_.cacheKey()), visible);
        } else if (crop) {
            const QRectF target(offset_.x() + cropDoc.x() * zoom_,
                                offset_.y() + cropDoc.y() * zoom_, cropDoc.width() * zoom_,
                                cropDoc.height() * zoom_);
            painter.drawImage(target, *crop);
        } else {
            // No level crop (a live stroke, or no provider): draw only the
            // visible document, not the whole image resampled every paint. The
            // source rect bounds per-paint sampling to the viewport.
            const QRectF visible = visibleDocumentRect();
            if (!visible.isEmpty() && !image_.isNull()) {
                const QRectF target(offset_.x() + visible.x() * zoom_,
                                    offset_.y() + visible.y() * zoom_, visible.width() * zoom_,
                                    visible.height() * zoom_);
                painter.drawImage(target, image_, visible);
            } else if (!visible.isEmpty() && levelProvider_.crop) {
                // A document canvas: level 0, cropped to what is on screen.
                const QRect area = visible.toAlignedRect();
                const QImage part =
                    levelProvider_.crop(0, area.x(), area.y(), area.width(), area.height());
                painter.drawImage(QRectF(offset_.x() + area.x() * zoom_,
                                         offset_.y() + area.y() * zoom_,
                                         area.width() * zoom_, area.height() * zoom_),
                                  part);
            }
        }
        painter.restore();
        presentCacheRebuiltLastPaint_ = presentCache_.rebuilds != before;
    }
    paintFilterPreview(painter);
    paintMaskOverlay(painter);
    // Hidden channels (two still shown): zero them with one multiply.
    if (channelMask_ != 0x7 && singleChannel() < 0) {
        painter.setCompositionMode(QPainter::CompositionMode_Multiply);
        painter.fillRect(docClip, QColor(channelMask_ & 1 ? 255 : 0, channelMask_ & 2 ? 255 : 0,
                                         channelMask_ & 4 ? 255 : 0));
        painter.setCompositionMode(QPainter::CompositionMode_SourceOver);
    }
    painter.restore();

    if (brushOutlineActive_ && brushOutlineDiameter_ > 0.0 && !panning_ && !spacePan_) {
        // Cosmetic pens keep each ring 1 device px independent of zoom; the
        // radius is in image pixels, so the on-screen circle scales with zoom.
        // A white ring one image px outside the black one keeps the outline
        // legible on both light and dark documents. Drawn with the full-widget
        // clip, not the document clip, so the ring stays visible past the edge.
        const double radius = brushOutlineDiameter_ / 2.0;
        painter.setBrush(Qt::NoBrush);
        painter.setPen(QPen(Qt::white, 0));
        painter.drawEllipse(brushOutlineImagePos_, radius + 1.0, radius + 1.0);
        painter.setPen(QPen(Qt::black, 0));
        painter.drawEllipse(brushOutlineImagePos_, radius, radius);
    }

    painter.save();
    painter.setClipRect(docClip);

    if (!overlayPolygon_.isEmpty()) {
        painter.setBrush(Qt::NoBrush);
        painter.setPen(QPen(Qt::white, 0, Qt::DashLine));
        painter.drawPolygon(overlayPolygon_);
    }

    if (selectionEdgesVisible_ && !selectionContours_.isEmpty()) {
        // Marching ants: a solid white base under an animated black dashed pen,
        // so the edge reads against both light and dark pixels. Width 0 = a
        // cosmetic 1-px line independent of the pan/zoom transform.
        // ponytail: a dragged outline past the canvas edge is cut off rather
        // than closed along it; the release retrace draws the clipped mask.
        painter.save();
        painter.translate(selectionContourOffset_);
        painter.setBrush(Qt::NoBrush);
        painter.setPen(QPen(Qt::white, 0));
        for (const QPolygonF& loop : selectionContours_) {
            painter.drawPolygon(loop);
        }
        QPen ants(Qt::black, 0, Qt::DashLine);
        ants.setDashOffset(antsPhase_);
        painter.setPen(ants);
        for (const QPolygonF& loop : selectionContours_) {
            painter.drawPolygon(loop);
        }
        painter.restore();
    }

    if (!previewContours_.isEmpty()) {
        // Live tool rubber band, so the selection reads during the drag exactly
        // as it will once committed. An open preview (Polygonal Lasso) is a
        // polyline: no phantom closing edge until it commits. A solid preview
        // draws white then black with no dash offset.
        painter.setBrush(Qt::NoBrush);
        painter.setPen(QPen(Qt::white, 0));
        for (const QPolygonF& loop : previewContours_) {
            if (selectionPreviewClosed_) {
                painter.drawPolygon(loop);
            } else {
                painter.drawPolyline(loop);
            }
        }
        QPen secondPen(Qt::black, 0,
                       selectionPreviewSolid_ ? Qt::SolidLine : Qt::DashLine);
        if (!selectionPreviewSolid_) {
            secondPen.setDashOffset(antsPhase_);
        }
        painter.setPen(secondPen);
        for (const QPolygonF& loop : previewContours_) {
            if (selectionPreviewClosed_) {
                painter.drawPolygon(loop);
            } else {
                painter.drawPolyline(loop);
            }
        }
        if (previewOriginActive_) {
            // 5 screen px whatever the zoom; the painter is in image space.
            const double half = 2.5 / std::max(zoom_, 1e-6);
            painter.setPen(QPen(Qt::black, 0));
            painter.setBrush(Qt::white);
            painter.drawRect(QRectF(previewOrigin_.x() - half, previewOrigin_.y() - half,
                                    2 * half, 2 * half));
            painter.setBrush(Qt::NoBrush);
        }
    }

    painter.restore();

    // The transform box and handles stay visible past the document edge (CS6);
    // only the rasterised preview above is clipped to the canvas.
    paintTransformOverlay(painter);

    paintGuides(painter);
    paintSmartGuides(painter);
    paintCropGroupOverlays(painter);
    paintAnnotations(painter);
    paintPathOverlay(painter);
    paintTypeOverlay(painter);
    paintMoveOutline(painter);
    if (compassVisible_) {
        paintCompass(painter);
    }

    if (dragSizeActive_ && !dragSizeText_.isEmpty()) {
        painter.save();
        painter.resetTransform();
        painter.setClipping(false);
        const QPointF widgetPos = imageToWidget(dragSizeImagePos_);
        QFont font = painter.font();
        font.setPointSize(10);
        painter.setFont(font);
        const QFontMetrics fm(painter.font());
        // Multi-line readouts (the Move tool's ΔX over ΔY) size to every line.
        QRectF boxRect(widgetPos + QPointF(14, 14),
                       fm.boundingRect(QRect(0, 0, 4000, 4000), Qt::AlignLeft, dragSizeText_).size()
                           + QSize(10, 10));
        if (boxRect.right() > rect().right()) {
            boxRect.moveRight(rect().right());
        }
        if (boxRect.bottom() > rect().bottom()) {
            boxRect.moveBottom(rect().bottom());
        }
        boxRect.moveLeft(std::max(boxRect.left(), qreal(rect().left())));
        boxRect.moveTop(std::max(boxRect.top(), qreal(rect().top())));
        painter.setPen(Qt::NoPen);
        painter.setBrush(QColor(0, 0, 0, 200));
        painter.drawRoundedRect(boxRect, 3, 3);
        painter.setPen(Qt::white);
        painter.drawText(boxRect, Qt::AlignCenter, dragSizeText_);
        painter.restore();
    }

    if (pressTrace_.armed) {
        pressTrace_.armed = false;
        const qint64 totalNs = pressTrace_.clock.nsecsElapsed();
        const bool force = qEnvironmentVariableIsSet("PICTURA_PRESS_TRACE");
        if (force || totalNs > 16000000) {
            const qint64 handled = pressTrace_.handledNs < 0 ? 0 : pressTrace_.handledNs;
            if (pressTrace_.firstMoveNs < 0) {
                qWarning("[press-trace] no-move press_handler=%.1fms total=%.1fms",
                         handled / 1e6, totalNs / 1e6);
            } else {
                const double osPressToMove = double(pressTrace_.firstMoveHwMs - pressTrace_.pressHwMs);
                qWarning("[press-trace] press_handler=%.1fms os_press_to_move=%.0fms "
                         "after_handler_to_move=%.1fms move_to_paint=%.1fms total=%.1fms",
                         handled / 1e6, osPressToMove,
                         (pressTrace_.firstMoveNs - handled) / 1e6,
                         (totalNs - pressTrace_.firstMoveNs) / 1e6, totalNs / 1e6);
            }
        }
    }
}

void ImageView::wheelEvent(QWheelEvent* event)
{
    const QPoint delta = event->angleDelta();
    const Qt::KeyboardModifiers mods = event->modifiers();
    // Shared wheel precedence: a horizontal side-wheel pans horizontally;
    // Ctrl+Alt pans vertically; Alt (without Ctrl) pans horizontally; otherwise
    // the vertical wheel zooms at the cursor, with Shift doubling the step.
    if (delta.x() != 0) {
        panBy(QPointF(delta.x(), 0.0));
    } else if (mods.testFlag(Qt::ControlModifier) && mods.testFlag(Qt::AltModifier)) {
        panBy(QPointF(0.0, delta.y()));
    } else if (mods.testFlag(Qt::AltModifier)) {
        panBy(QPointF(delta.y(), 0.0));
    } else {
        const int step = (delta.y() != 0 && mods.testFlag(Qt::ShiftModifier)) ? delta.y() * 2
                                                                              : delta.y();
        zoomAt(viewRotation().inverted().map(event->position()), step);
    }
    event->accept();
}

void ImageView::mousePressEvent(QMouseEvent* event)
{
    // The workspace snap lock pins a fitting canvas to the centre, so no pan
    // may start; a press that would otherwise pan falls through unhandled.
    const bool panPress =
        event->button() == Qt::MiddleButton
        || (event->button() == Qt::LeftButton
            && (spacePan_
                || (panEnabled_ && !event->modifiers().testFlag(Qt::ControlModifier))));
    if (panPress && workspaceSnapLocked()) {
        QWidget::mousePressEvent(event);
        return;
    }
    if (event->button() == Qt::MiddleButton) {
        panning_ = true;
        userAdjusted_ = true;
        pendingSettle_ = false;
        last_ = event->position();
        setCursor(Qt::ClosedHandCursor);
        update();
    } else if (event->button() == Qt::LeftButton
               && (spacePan_
                   || (panEnabled_ && !event->modifiers().testFlag(Qt::ControlModifier)))) {
        // A Ctrl press goes to the controller, which pans if no guide is hit.
        beginPan(event->position());
        if (spacePan_) {
            setCursor(Qt::ClosedHandCursor);
        }
    } else {
        pressTrace_.clock.restart();
        pressTrace_.pressHwMs = event->timestamp();
        pressTrace_.handledNs = -1;
        pressTrace_.firstMoveNs = -1;
        pressTrace_.firstMoveHwMs = -1;
        pressTrace_.armed = true;
        emit mousePressed(widgetToImage(event->position()), event->button(),
                          event->modifiers());
        pressTrace_.handledNs = pressTrace_.clock.nsecsElapsed();
    }
    QWidget::mousePressEvent(event);
}

void ImageView::mouseMoveEvent(QMouseEvent* event)
{
    const Qt::MouseButtons buttons = event->buttons();
    if (panning_ && (buttons & (Qt::MiddleButton | Qt::LeftButton))) {
        // A screen-space drag moves the turned canvas along with the pointer.
        const QTransform unturn = QTransform().rotate(-rotation_);
        panBy(unturn.map(event->position() - last_));
        last_ = event->position();
    } else if (!(spacePan_ && buttons == Qt::NoButton)) {
        // While Space is held with no button down, the hover path is muted so a
        // tool cursor update cannot clobber the hand cursor; a button-held drag
        // still gets its moves.
        if (pressTrace_.armed && pressTrace_.firstMoveNs < 0) {
            pressTrace_.firstMoveNs = pressTrace_.clock.nsecsElapsed();
            pressTrace_.firstMoveHwMs = event->timestamp();
        }
        emit mouseMoved(widgetToImage(event->position()));
    }
    if (!transformWarp_.points.isEmpty()) {
        warpHover_ = widgetToImage(event->position());
        update();
    }
    QWidget::mouseMoveEvent(event);
}

void ImageView::contextMenuEvent(QContextMenuEvent* event)
{
    // The frame decides which tool menu (if any) applies at this point.
    emit contextMenuRequested(event->globalPos());
    event->accept();
}

void ImageView::mouseReleaseEvent(QMouseEvent* event)
{
    if (event->button() == Qt::MiddleButton || event->button() == Qt::LeftButton) {
        const bool wasPanning = panning_;
        panning_ = false;
        if (wasPanning && spacePan_ && event->button() == Qt::LeftButton) {
            setCursor(Qt::OpenHandCursor);
        }
    }
    if (event->button() == Qt::MiddleButton) {
        // A Space hold keeps the hand cursor, matching the left-button path.
        if (spacePan_) {
            setCursor(Qt::OpenHandCursor);
        } else {
            unsetCursor();
        }
    }
    emit mouseReleased(widgetToImage(event->position()));
    QWidget::mouseReleaseEvent(event);
}

void ImageView::resizeEvent(QResizeEvent* event)
{
    // A fresh document set before layout runs its automatic fit against a
    // stale viewport, so while the settle is pending every resize re-fits;
    // the show pass ends it. A load/fit also leaves userAdjusted_ false until
    // the first resize; any pan/zoom sets it true so a later tool-switch
    // resize keeps the view instead of snapping the zoom.
    if (pendingSettle_ && !docSize_.isEmpty()) {
        applyInitialView();
    } else if (!userAdjusted_ && !docSize_.isEmpty()) {
        applyInitialView();
    } else {
        offset_ += QRectF(QPointF(), QSizeF(event->size() - laidOutSize_)).center();
        clampOffset();
        emit viewChanged();
    }
    laidOutSize_ = event->size();
    QWidget::resizeEvent(event);
}

void ImageView::leaveEvent(QEvent* event)
{
    clearBrushOutline();
    QWidget::leaveEvent(event);
}

void ImageView::showEvent(QShowEvent* event)
{
    QWidget::showEvent(event);
    updateAntsTimer();
    // Qt assigns a never-shown widget its laid-out geometry silently, with no
    // resize event, so a fit computed before the show is stale (the canvas sat
    // at its grid minimum). The geometry is real by now: re-fit, then settle
    // once the show pass finished so a multi-pass layout still lands correctly.
    if (pendingSettle_ && !docSize_.isEmpty()) {
        applyInitialView();
        QMetaObject::invokeMethod(this, [this]() {
            if (pendingSettle_ && !docSize_.isEmpty()) {
                applyInitialView();
                pendingSettle_ = false;
            }
        }, Qt::QueuedConnection);
    }
}

void ImageView::keyPressEvent(QKeyEvent* event)
{
    // A modifier's own press may not carry its flag yet (platform-dependent).
    if (const int flag = modifierFlag(event->key())) {
        emit modifierKeyChanged(int(event->modifiers()) | flag);
    }
    if (transformActive_) {
        if (event->key() == Qt::Key_Return || event->key() == Qt::Key_Enter) {
            emit transformCommitRequested();
            event->accept();
            return;
        }
        if (event->key() == Qt::Key_Escape) {
            emit transformCancelRequested();
            event->accept();
            return;
        }
    }
    QWidget::keyPressEvent(event);
}

void ImageView::keyReleaseEvent(QKeyEvent* event)
{
    if (const int flag = modifierFlag(event->key())) {
        emit modifierKeyChanged(int(event->modifiers()) & ~flag);
    }
    QWidget::keyReleaseEvent(event);
}

void ImageView::hideEvent(QHideEvent* event)
{
    QWidget::hideEvent(event);
    updateAntsTimer();
}

void ImageView::setBrushOutline(double diameter, const QPointF& imagePos)
{
    if (diameter <= 0.0) {
        clearBrushOutline();
        return;
    }
    brushOutlineActive_ = true;
    brushOutlineDiameter_ = diameter;
    brushOutlineImagePos_ = imagePos;
    update();
}

void ImageView::clearBrushOutline()
{
    if (!brushOutlineActive_) {
        return;
    }
    brushOutlineActive_ = false;
    update();
}

} // namespace pictura
