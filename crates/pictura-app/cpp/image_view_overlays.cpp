// ImageView overlays for the Crop tool group: the crop box, the Perspective
// Crop quad, and the slice map. Split from image_view.cpp to keep each translation unit small.
// Ported from photorust's CanvasView::paintCropQuad / paintSlices.

#include "image_view.h"

#include "crop_grip.h"

#include <QtGui/QFontMetrics>
#include <QtGui/QPainter>
#include <QtGui/QPainterPath>

#include <algorithm>
#include <cmath>

namespace pictura {

namespace {

// The rotate grid: square cells about kCropRotateGridPx on screen, fitted so
// the lines divide the box evenly.
constexpr double kCropRotateGridPx = 45.0;

void drawCropRotateGrid(QPainter& p, const QRectF& box)
{
    const int cols = std::max(2, int(std::lround(box.width() / kCropRotateGridPx)));
    const int rows = std::max(2, int(std::lround(box.height() / kCropRotateGridPx)));
    for (int i = 1; i < cols; ++i) {
        const double x = box.left() + box.width() * i / cols;
        p.drawLine(QPointF(x, box.top()), QPointF(x, box.bottom()));
    }
    for (int i = 1; i < rows; ++i) {
        const double y = box.top() + box.height() * i / rows;
        p.drawLine(QPointF(box.left(), y), QPointF(box.right(), y));
    }
}

// The active crop guide overlay: 0 Rule of Thirds, 1 Grid, 2 Diagonal,
// 3 Triangle, 4 Golden Ratio, 5 Golden Spiral.
void drawCropGuides(QPainter& p, const QRectF& box, int mode)
{
    const QPointF tl = box.topLeft();
    const QPointF tr = box.topRight();
    const QPointF bl = box.bottomLeft();
    const auto vline = [&](double fx) { p.drawLine(QPointF(fx, box.top()), QPointF(fx, box.bottom())); };
    const auto hline = [&](double fy) { p.drawLine(QPointF(box.left(), fy), QPointF(box.right(), fy)); };
    switch (mode) {
    case 1: // Grid: eighths.
        for (int i = 1; i < 8; ++i) {
            hline(box.top() + box.height() * i / 8.0);
            vline(box.left() + box.width() * i / 8.0);
        }
        break;
    case 2: // Diagonal.
        p.drawLine(tl, box.bottomRight());
        p.drawLine(tr, bl);
        break;
    case 3: { // Triangle: diagonals plus top-corner fans to the bottom centre.
        const QPointF bc(box.center().x(), box.bottom());
        p.drawLine(tl, box.bottomRight());
        p.drawLine(tr, bl);
        p.drawLine(tl, bc);
        p.drawLine(tr, bc);
        break;
    }
    case 4: // Golden Ratio lines.
        for (double f : {0.382, 0.618}) {
            hline(box.top() + box.height() * f);
            vline(box.left() + box.width() * f);
        }
        break;
    case 5: { // Golden Spiral about the box centre (an approximation of CS6's).
        p.save();
        p.setClipRect(box);
        constexpr double kPi = 3.14159265358979323846;
        const double k = std::log(1.6180339887498949) / (kPi / 2.0);
        const double span = 4.0 * kPi;
        const double rMin = std::exp(-k * span);
        const double scale = (std::min(box.width(), box.height()) * 0.5) / (1.0 - rMin);
        QPainterPath path;
        const int n = 320;
        for (int i = 0; i <= n; ++i) {
            const double theta = span * i / double(n);
            const double r = std::exp(-k * (span - theta)) * scale;
            const QPointF pt(box.center().x() + r * std::cos(theta),
                             box.center().y() + r * std::sin(theta));
            if (i == 0) {
                path.moveTo(pt);
            } else {
                path.lineTo(pt);
            }
        }
        p.setBrush(Qt::NoBrush);
        p.drawPath(path);
        p.restore();
        break;
    }
    default: // 0 Rule of Thirds.
        for (int i = 1; i <= 2; ++i) {
            hline(box.top() + box.height() * i / 3.0);
            vline(box.left() + box.width() * i / 3.0);
        }
        break;
    }
}

} // namespace

QTransform ImageView::transformPreviewMatrix() const
{
    const double cx = moveLayerPos_.x() + moveLayer_.width() / 2.0;
    const double cy = moveLayerPos_.y() + moveLayer_.height() / 2.0;
    QTransform matrix;
    matrix.translate(transformDx_, transformDy_);
    matrix.translate(cx, cy);
    matrix.rotate(transformAngle_ * 180.0 / M_PI);
    matrix.scale(transformScaleX_, transformScaleY_);
    matrix.translate(-cx, -cy);
    return matrix;
}

void ImageView::setTransformPivot(const QPointF& pivot, bool visible)
{
    transformPivot_ = pivot;
    transformPivotVisible_ = visible;
    update();
}

void ImageView::updateTransformPreviewLayer(const QImage& layer, const QPointF& layerPos)
{
    moveLayer_ = layer;
    moveLayerPos_ = layerPos;
    transformWarpedStale_ = true;
    update();
}

// The Free Transform reference point: a target glyph in image space (the
// painter is already scaled by the zoom).
void ImageView::paintTransformPivot(QPainter& painter)
{
    const double z = zoom_ > 0.0 ? zoom_ : 1.0;
    const double r = 6.0 / z;
    const QPointF p = transformPivot_;
    painter.setBrush(Qt::NoBrush);
    painter.setPen(QPen(Qt::white, 0, Qt::SolidLine));
    painter.drawEllipse(p, r, r);
    painter.drawLine(QPointF(p.x() - r, p.y()), QPointF(p.x() + r, p.y()));
    painter.drawLine(QPointF(p.x(), p.y() - r), QPointF(p.x(), p.y() + r));
    painter.setPen(QPen(Qt::black, 0, Qt::DashLine));
    painter.drawEllipse(p, r, r);
    painter.drawLine(QPointF(p.x() - r, p.y()), QPointF(p.x() + r, p.y()));
    painter.drawLine(QPointF(p.x(), p.y() - r), QPointF(p.x(), p.y() + r));
    painter.setBrush(Qt::white);
    painter.setPen(QPen(Qt::black, 0, Qt::SolidLine));
    painter.drawEllipse(p, 1.5 / z, 1.5 / z);
}

void ImageView::setTransformWarp(const transform_api::WarpOverlay& overlay)
{
    transformWarp_ = overlay;
    update();
}

// Warp mode's chrome in image space (photocraft's warp overlay): the patch
// boundaries solid, a single patch's rule-of-thirds guides faint, and for a
// custom warp its anchors always; a handle and its arm only near the pointer or
// while its anchor is dragged, so the grid does not bury the picture in dots.
void ImageView::paintWarpOverlay(QPainter& painter)
{
    const double z = zoom_ > 0.0 ? zoom_ : 1.0;
    // How near the pointer (screen px) a handle or its anchor reveals it.
    const double reveal = 56.0 / z;
    painter.setBrush(Qt::NoBrush);
    painter.setPen(QPen(QColor(255, 255, 255, 150), 0, Qt::SolidLine));
    for (const QPolygonF& guide : transformWarp_.guides) {
        painter.drawPolyline(guide);
    }
    for (const QPolygonF& line : transformWarp_.grid) {
        painter.setPen(QPen(Qt::white, 0, Qt::SolidLine));
        painter.drawPolyline(line);
        painter.setPen(QPen(Qt::black, 0, Qt::DashLine));
        painter.drawPolyline(line);
    }
    const auto near = [this, reveal](const QPointF& p) {
        return QLineF(p, warpHover_).length() <= reveal;
    };
    const double hs = 2.5 / z;
    const double hr = 2.75 / z;
    for (const transform_api::WarpPoint& point : transformWarp_.points) {
        if (point.kind == transform_api::WarpPoint::Anchor) {
            painter.setBrush(Qt::white);
            painter.setPen(QPen(Qt::black, 0, Qt::SolidLine));
            painter.drawRect(QRectF(point.pos.x() - hs, point.pos.y() - hs, 2.0 * hs, 2.0 * hs));
            continue;
        }
        if (point.kind != transform_api::WarpPoint::DraggedHandle && !near(point.pos)
            && !near(point.anchor)) {
            continue;
        }
        painter.setPen(QPen(Qt::white, 0, Qt::SolidLine));
        painter.drawLine(point.anchor, point.pos);
        painter.setBrush(Qt::white);
        painter.setPen(QPen(Qt::black, 0, Qt::SolidLine));
        painter.drawEllipse(point.pos, hr, hr);
    }
}

// The Free Transform box and its chrome in image space. Called outside the
// document clip so a layer dragged past the canvas edge keeps its full box.
void ImageView::paintTransformOverlay(QPainter& painter)
{
    if (transformActive_ && !transformWarp_.isEmpty()) {
        paintWarpOverlay(painter);
        return;
    }
    // A session's live quad, else the Move tool's idle controls box.
    const QPolygonF& quad = transformActive_ ? transformQuad_ : controlsBox_;
    if (quad.size() < 4) {
        return;
    }
    painter.setBrush(Qt::NoBrush);
    painter.setPen(QPen(Qt::white, 0, Qt::SolidLine));
    painter.drawPolygon(quad);
    painter.setPen(QPen(Qt::black, 0, Qt::DashLine));
    painter.drawPolygon(quad);

    // Eight scale handles at the corners and edge midpoints.
    const QPointF tl = quad.at(0);
    const QPointF tr = quad.at(1);
    const QPointF br = quad.at(2);
    const QPointF bl = quad.at(3);
    const QPointF handles[8] = {
        tl,
        tr,
        br,
        bl,
        (tl + tr) / 2.0,
        (tr + br) / 2.0,
        (br + bl) / 2.0,
        (bl + tl) / 2.0,
    };
    const double hs = 3.5 / (zoom_ > 0.0 ? zoom_ : 1.0);
    painter.setBrush(Qt::white);
    painter.setPen(QPen(Qt::black, 0, Qt::SolidLine));
    for (const QPointF& h : handles) {
        const QRectF box(h.x() - hs, h.y() - hs, 2.0 * hs, 2.0 * hs);
        painter.drawRect(box);
    }

    // Rotate affordance: a short stem and knob below the bottom edge.
    const QPointF bottomMid = (br + bl) / 2.0;
    const double stem = 22.0 / (zoom_ > 0.0 ? zoom_ : 1.0);
    const QPointF knob(bottomMid.x(), bottomMid.y() + stem);
    painter.setPen(QPen(Qt::white, 0, Qt::SolidLine));
    painter.drawLine(bottomMid, knob);
    painter.setBrush(Qt::white);
    painter.drawEllipse(knob, hs, hs);

    // Reference point: a target glyph (circle, crosshair, centre dot).
    if (transformPivotVisible_) {
        paintTransformPivot(painter);
    }
}

void ImageView::setCropBox(const QRectF& box){
    cropBox_ = box.normalized();
    update();
}

void ImageView::setCropPreview(bool preview)
{
    if (cropPreview_ == preview) {
        return;
    }
    cropPreview_ = preview;
    update();
}

void ImageView::setCropRotating(bool rotating)
{
    if (cropRotating_ == rotating) {
        return;
    }
    cropRotating_ = rotating;
    update();
}

void ImageView::setCropOverlay(int overlay)
{
    if (cropOverlay_ == overlay) {
        return;
    }
    cropOverlay_ = overlay;
    update();
}

void ImageView::setCropBoxRotates(bool rotates)
{
    if (cropBoxRotates_ == rotates) {
        return;
    }
    cropBoxRotates_ = rotates;
    update();
}

void ImageView::setCropShield(const CropShield& shield)
{
    cropShield_ = shield;
    update();
}

// Auto Adjust Opacity: the shield eases off while the box is being dragged so
// the content under it stays readable.
int ImageView::cropShieldAlpha() const
{
    if (!cropShield_.showOutside) {
        return 255;
    }
    if (!cropShield_.enabled) {
        return 0;
    }
    double opacity = cropShield_.opacity / 100.0;
    if (cropShield_.autoAdjust && cropShield_.dragging) {
        opacity *= 0.5;
    }
    return int(std::lround(opacity * 255.0));
}

void ImageView::centerOnImageRect(const QRectF& area)
{
    if (area.isNull() || docSize_.isEmpty()) {
        return;
    }
    panBy(QRectF(this->rect()).center() - imageToWidget(area.center()));
}

void ImageView::setCropContentOffset(const QPointF& offset)
{
    if (cropContentOffset_ == offset) {
        return;
    }
    cropContentOffset_ = offset;
    update();
}

void ImageView::clearCropBox()
{
    cropBox_ = QRectF();
    cropPreview_ = false;
    cropRotating_ = false;
    cropContentOffset_ = QPointF();
    cropStraighten_ = 0.0;
    cropStraightenPivot_ = QPointF();
    cropStraightenLine_ = QLineF();
    update();
}

void ImageView::setCropStraighten(double degrees, const QPointF& pivot)
{
    cropStraighten_ = degrees;
    cropStraightenPivot_ = pivot;
    update();
}

void ImageView::setCropStraightenLine(const QLineF& line)
{
    cropStraightenLine_ = line;
    update();
}

void ImageView::clearCropStraightenLine()
{
    cropStraightenLine_ = QLineF();
    update();
}

// The preview canvas: the image rect when straight, else the axis-aligned
// bounding box of the image rect rotated about the session pivot. Matches
// `rotate_document_in`'s canvas growth so the preview and the commit agree.
QRectF ImageView::cropCanvasImageRect() const
{
    // The content offset (Modern pan) moves the composite, so the frame follows
    // the shifted image rect.
    const QRectF image =
        QRectF(0.0, 0.0, docSize_.width(), docSize_.height()).translated(cropContentOffset_);
    if (docSize_.isEmpty()) {
        return image;
    }
    QRectF canvas = image;
    if (cropStraighten_ != 0.0 && !cropBox_.isNull() && !cropBoxRotates_) {
        const double rad = cropStraighten_ * M_PI / 180.0;
        const double c = std::cos(rad);
        const double s = std::sin(rad);
        const QPointF pivot = cropStraightenPivot_;
        double min_x = 0.0;
        double min_y = 0.0;
        double max_x = 0.0;
        double max_y = 0.0;
        bool first = true;
        for (const QPointF& corner : {image.topLeft(), image.topRight(), image.bottomRight(),
                                      image.bottomLeft()}) {
            const QPointF u = corner - pivot;
            const double qx = pivot.x() + c * u.x() - s * u.y();
            const double qy = pivot.y() + s * u.x() + c * u.y();
            if (first) {
                min_x = max_x = qx;
                min_y = max_y = qy;
                first = false;
            } else {
                min_x = std::min(min_x, qx);
                min_y = std::min(min_y, qy);
                max_x = std::max(max_x, qx);
                max_y = std::max(max_y, qy);
            }
        }
        canvas = QRectF(min_x, min_y, max_x - min_x, max_y - min_y);
    }
    // The canvas follows a crop box dragged beyond its edge, so the grown area
    // is visible and shaded instead of clipped at the original edge.
    if (!cropBox_.isNull()) {
        canvas = canvas.united(cropBox_);
    }
    return canvas;
}

QPolygonF ImageView::cropBoxWidgetForTest() const
{
    const auto toWidget = [this](const QPointF& p) { return p * zoom_ + offset_; };
    QPolygonF poly;
    if (!cropBox_.isNull()) {
        poly << toWidget(cropBox_.topLeft()) << toWidget(cropBox_.topRight())
             << toWidget(cropBox_.bottomRight()) << toWidget(cropBox_.bottomLeft());
    }
    return poly;
}

void ImageView::setPerspectiveCropQuad(const QPolygonF& quad, double gridSpacing)
{
    // 1–3 points stage the quad; the fourth completes it.
    perspectiveQuad_ = quad;
    perspectiveGridSpacing_ = gridSpacing;
    update();
}

void ImageView::clearPerspectiveCropQuad()
{
    perspectiveQuad_.clear();
    perspectiveGridSpacing_ = 0.0;
    update();
}

void ImageView::setSliceOverlay(const QList<SliceOverlay>& slices, const QRectF& dragging)
{
    sliceOverlay_ = slices;
    sliceDrag_ = dragging.normalized();
    update();
}

void ImageView::clearSliceOverlay()
{
    sliceOverlay_.clear();
    sliceDrag_ = QRectF();
    update();
}

void ImageView::setSamplingRing(double diameter, const QPointF& imagePos)
{
    if (diameter <= 0.0) {
        clearSamplingRing();
        return;
    }
    samplingRingActive_ = true;
    samplingRingDiameter_ = diameter;
    samplingRingImagePos_ = imagePos;
    update();
}

void ImageView::clearSamplingRing()
{
    if (!samplingRingActive_) {
        return;
    }
    samplingRingActive_ = false;
    update();
}

// Drawn in widget space so lines, handles, and badges keep their screen size at
// any zoom.
void ImageView::paintCropGroupOverlays(QPainter& painter)
{
    if (perspectiveQuad_.isEmpty() && sliceOverlay_.isEmpty() && sliceDrag_.isNull()
        && cropBox_.isNull() && cropStraightenLine_.isNull() && !samplingRingActive_) {
        return;
    }
    const auto toWidget = [this](const QPointF& p) { return p * zoom_ + offset_; };
    painter.save();
    painter.setTransform(viewRotation());
    painter.setClipping(false);

    if (!sliceOverlay_.isEmpty() || !sliceDrag_.isNull()) {
        // CS6 colours: user slices saturated blue and solid, auto slices grey
        // and dotted.
        const QColor userColor(0x2c, 0x6f, 0xd6);
        const QColor autoColor(0x8a, 0x8a, 0x8a);
        QFont badgeFont = painter.font();
        badgeFont.setPixelSize(9);
        painter.setFont(badgeFont);
        const QFontMetrics metrics(badgeFont);
        painter.setRenderHint(QPainter::Antialiasing, false);
        for (const SliceOverlay& slice : sliceOverlay_) {
            const QRectF box(toWidget(slice.rect.topLeft()), toWidget(slice.rect.bottomRight()));
            const QColor color = slice.user ? userColor : autoColor;
            painter.setPen(QPen(color, 1, slice.user ? Qt::SolidLine : Qt::DotLine));
            painter.setBrush(Qt::NoBrush);
            painter.drawRect(box);
            // The numbered badge sits inside the top-left corner, skipped on a
            // slice too small to hold it.
            const QString label = QStringLiteral("%1").arg(slice.number, 2, 10, QLatin1Char('0'));
            const QRectF badge(box.left() + 1, box.top() + 1,
                               metrics.horizontalAdvance(label) + 6, 12);
            if (box.width() >= badge.width() + 2 && box.height() >= badge.height() + 2) {
                painter.setPen(Qt::NoPen);
                painter.setBrush(color);
                painter.drawRect(badge);
                painter.setPen(Qt::white);
                painter.drawText(badge, Qt::AlignCenter, label);
            }
            if (slice.selected) {
                // CS6 marks the selected slice with orange handles.
                const QColor handle(0xf5, 0x9e, 0x0b);
                painter.setPen(QPen(handle, 1));
                painter.setBrush(handle);
                const QPointF c = box.center();
                for (const QPointF& h :
                     {box.topLeft(), QPointF(c.x(), box.top()), box.topRight(),
                      QPointF(box.right(), c.y()), box.bottomRight(), QPointF(c.x(), box.bottom()),
                      box.bottomLeft(), QPointF(box.left(), c.y())}) {
                    painter.drawRect(QRectF(h.x() - 2.5, h.y() - 2.5, 5, 5));
                }
            }
        }
        if (!sliceDrag_.isNull()) {
            painter.setPen(QPen(userColor, 1));
            painter.setBrush(Qt::NoBrush);
            painter.drawRect(QRectF(toWidget(sliceDrag_.topLeft()),
                                    toWidget(sliceDrag_.bottomRight())));
        }
    }

    if (!cropBox_.isNull()) {
        const QRectF box(toWidget(cropBox_.topLeft()), toWidget(cropBox_.bottomRight()));
        painter.setRenderHint(QPainter::Antialiasing, false);
        // The crop shield: what is about to be thrown away, dimmed. Only the
        // canvas outside the box is dimmed, not the workspace around it; under
        // straighten the canvas is the rotated composite's bounding box.
        const QRectF canvasDoc = cropCanvasImageRect();
        const QRectF canvas(toWidget(canvasDoc.topLeft()), toWidget(canvasDoc.bottomRight()));
        // Classic turns the box about the pivot over the fixed image.
        QTransform boxTurn;
        if (cropBoxRotates_ && cropStraighten_ != 0.0) {
            const QPointF pivot = toWidget(cropStraightenPivot_);
            boxTurn.translate(pivot.x(), pivot.y());
            boxTurn.rotate(-cropStraighten_);
            boxTurn.translate(-pivot.x(), -pivot.y());
        }
        QPainterPath shield;
        shield.addRect(canvas);
        if (boxTurn.isIdentity()) {
            shield.addRect(box);
        } else {
            QPainterPath turned;
            turned.addPolygon(boxTurn.map(QPolygonF(box)));
            turned.closeSubpath();
            shield = QPainterPath(shield).subtracted(turned);
        }
        painter.setPen(Qt::NoPen);
        if (const int alpha = cropShieldAlpha(); alpha > 0) {
            QColor tint = cropShield_.color.isValid() ? cropShield_.color : canvasColor_;
            tint.setAlpha(alpha);
            painter.setBrush(tint);
            painter.drawPath(shield);
        }
        painter.setBrush(Qt::NoBrush);
        painter.save();
        painter.setTransform(boxTurn, true);
        // The active guide overlay, chosen from the options-bar menu; the
        // preview box the tool starts with shows its handles but no guides, and
        // a rotation shows the dense rotate grid instead.
        if (cropRotating_) {
            painter.setPen(QPen(QColor(255, 255, 255, 110), 1));
            drawCropRotateGrid(painter, box);
        } else if (!cropPreview_) {
            painter.setPen(QPen(QColor(255, 255, 255, 90), 1));
            drawCropGuides(painter, box, cropOverlay_);
        }
        // CS6 handles sit inside the outline: L-bracket corners and thin edge
        // bars, drawn white over a dark shadow so they read on any content.
        // Shadows are clamped to the box so they never bleed onto the shield.
        const CropHandleGlyphs glyphs = cropHandleGlyphs(box);
        painter.setRenderHint(QPainter::Antialiasing, true);
        painter.setOpacity(0.8);
        painter.setBrush(Qt::NoBrush);
        painter.setPen(QPen(QColor(40, 40, 40, 200), kCropHandleBarThickPx + 2));
        for (const QLineF& s : glyphs.cornerStrokes) {
            painter.drawLine(s);
        }
        painter.setPen(Qt::NoPen);
        painter.setBrush(QColor(40, 40, 40, 200));
        for (const QRectF& bar : glyphs.edgeBars) {
            painter.drawRect(bar.adjusted(-1, -1, 1, 1).intersected(box));
        }
        painter.setPen(QPen(Qt::white, kCropHandleBarThickPx));
        painter.setBrush(Qt::NoBrush);
        for (const QLineF& s : glyphs.cornerStrokes) {
            painter.drawLine(s);
        }
        painter.setPen(Qt::NoPen);
        painter.setBrush(Qt::white);
        for (const QRectF& bar : glyphs.edgeBars) {
            painter.drawRect(bar);
        }
        painter.setOpacity(1.0);
        // The outline is drawn last so the handle shadows cannot break it.
        painter.setRenderHint(QPainter::Antialiasing, false);
        painter.setBrush(Qt::NoBrush);
        painter.setPen(QPen(QColor(255, 255, 255, 220), 1));
        painter.drawRect(box);
        painter.restore();
    }

    // The straighten line tool's live horizon: a plain ruled line from press to
    // the current pointer, drawn in widget space.
    if (!cropStraightenLine_.isNull()) {
        painter.setRenderHint(QPainter::Antialiasing, true);
        painter.setBrush(Qt::NoBrush);
        painter.setPen(QPen(QColor(255, 255, 255, 220), 1));
        painter.drawLine(QLineF(toWidget(cropStraightenLine_.p1()),
                                toWidget(cropStraightenLine_.p2())));
    }

    if (!perspectiveQuad_.isEmpty()) {
        QPolygonF quad;
        for (const QPointF& p : perspectiveQuad_) {
            quad << toWidget(p);
        }
        painter.setRenderHint(QPainter::Antialiasing, true);
        if (quad.size() == 4) {
            // Shade the canvas area outside the quad (not the workspace), as the
            // rectangular crop does.
            const QRectF canvasDoc = cropCanvasImageRect();
            const QRectF canvas(toWidget(canvasDoc.topLeft()), toWidget(canvasDoc.bottomRight()));
            QPainterPath shield;
            shield.addRect(canvas);
            QPainterPath inside;
            inside.addPolygon(quad);
            inside.closeSubpath();
            painter.setPen(Qt::NoPen);
            painter.setBrush(QColor(0, 0, 0, 150));
            painter.drawPath(shield.subtracted(inside));
            // An inch grid interpolated along the edges, so it follows the
            // perspective and keeps a fixed physical spacing as the quad grows.
            const double spacing = perspectiveGridSpacing_;
            if (spacing > 0.0) {
                const double e01 = std::hypot(quad[1].x() - quad[0].x(), quad[1].y() - quad[0].y());
                const double e32 = std::hypot(quad[2].x() - quad[3].x(), quad[2].y() - quad[3].y());
                const double e03 = std::hypot(quad[3].x() - quad[0].x(), quad[3].y() - quad[0].y());
                const double e12 = std::hypot(quad[2].x() - quad[1].x(), quad[2].y() - quad[1].y());
                const int cols = std::max(1, int(std::lround((e01 + e32) / 2.0 / spacing)));
                const int rows = std::max(1, int(std::lround((e03 + e12) / 2.0 / spacing)));
                painter.setBrush(Qt::NoBrush);
                painter.setPen(QPen(QColor(255, 255, 255, 90), 1));
                for (int i = 1; i < cols; ++i) {
                    const double t = double(i) / cols;
                    painter.drawLine(quad[0] + (quad[1] - quad[0]) * t,
                                     quad[3] + (quad[2] - quad[3]) * t);
                }
                for (int i = 1; i < rows; ++i) {
                    const double t = double(i) / rows;
                    painter.drawLine(quad[0] + (quad[3] - quad[0]) * t,
                                     quad[1] + (quad[2] - quad[1]) * t);
                }
            }
            painter.setPen(QPen(QColor(255, 255, 255, 220), 1));
            painter.drawPolygon(quad);
        } else {
            // Staging: connect the placed points and mark each one.
            painter.setBrush(Qt::NoBrush);
            painter.setPen(QPen(QColor(255, 255, 255, 180), 1, Qt::DashLine));
            painter.drawPolyline(quad);
        }
        // The corner handles.
        painter.setPen(QPen(QColor(40, 40, 40), 1));
        painter.setBrush(Qt::white);
        for (const QPointF& p : quad) {
            painter.drawRect(QRectF(p.x() - 3.5, p.y() - 3.5, 7, 7));
        }
    }
    if (samplingRingActive_) {
        // The eyedropper's sampling ring: a white/black pair so it reads on any
        // document, centred on the hovered pixel in widget space.
        const QPointF c = toWidget(samplingRingImagePos_);
        const double r = samplingRingDiameter_ * 0.5 * zoom_;
        painter.setRenderHint(QPainter::Antialiasing, true);
        painter.setBrush(Qt::NoBrush);
        painter.setPen(QPen(Qt::white, 0));
        painter.drawEllipse(c, r + 1.0, r + 1.0);
        painter.setPen(QPen(Qt::black, 0));
        painter.drawEllipse(c, r, r);
    }

    painter.restore();
}

} // namespace pictura
