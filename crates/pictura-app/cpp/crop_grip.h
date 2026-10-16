#pragma once

// The eight-handle box geometry shared by the Crop and Slice Select tools:
// which handle (or the body) a point grabs, where a drag takes the box, and the
// cursor for each. Ported from photorust's CanvasView gripAt/dragCrop/cropCursor.

#include <QtCore/QLineF>
#include <QtCore/QPointF>
#include <QtCore/QRectF>
#include <QtCore/Qt>

#include <algorithm>
#include <array>
#include <cmath>

namespace pictura {

enum class BoxGrip { None, Move, TopLeft, Top, TopRight, Right, BottomRight, Bottom, BottomLeft, Left };

// Screen-pixel margins outside the crop box: within `kCropRotateMarginPx` a
// press grabs a resize handle; from there out to `kCropFarMarginPx` it rotates
// the content; beyond the far margin it applies the crop.
inline constexpr double kCropRotateMarginPx = 8.0;
inline constexpr double kCropFarMarginPx = 100.0;

// The image-space distance from `p` to the nearest point of `box`.
inline double boxDistance(const QRectF& box, const QPointF& p)
{
    const QPointF clamped(std::clamp(p.x(), box.left(), box.right()),
                          std::clamp(p.y(), box.top(), box.bottom()));
    return std::hypot(p.x() - clamped.x(), p.y() - clamped.y());
}

// True when `p` (image space) is beyond the rotate margin but within the far
// margin: the rotate band. Ignoring the canvas, so the band extends past the
// canvas edge.
inline bool cropRotateZone(const QRectF& box, const QPointF& p, double zoom,
                           double marginPx = kCropRotateMarginPx,
                           double outerPx = kCropFarMarginPx)
{
    if (box.isNull()) {
        return false;
    }
    const double d = boxDistance(box, p) * std::max(zoom, 1e-6);
    return d > marginPx && d <= outerPx;
}

// True when `p` (image space) is beyond the far margin from `box`; there a press
// applies the crop rather than rotating.
inline bool cropFarZone(const QRectF& box, const QPointF& p, double zoom,
                        double outerPx = kCropFarMarginPx)
{
    return !box.isNull() && boxDistance(box, p) * std::max(zoom, 1e-6) > outerPx;
}

// The eight rotate-cursor directions (degrees, screen y-down), one per box
// corner (diagonal) and edge (axis). `delta` is the pointer relative to the box
// centre; the nearest zone wins, with the four corner zones widened by 7.5° on
// each side so their variant is easy to hit on a small box.
inline double cropRotateZoneAngle(const QPointF& delta)
{
    if (delta.isNull()) {
        return 0.0;
    }
    double deg = std::atan2(delta.y(), delta.x()) * 180.0 / M_PI;
    if (deg < 0.0) {
        deg += 360.0;
    }
    double best = 0.0;
    double bestScore = 1e9;
    for (int k = 0; k < 8; ++k) {
        const double axis = k * 45.0;
        double diff = std::abs(deg - axis);
        if (diff > 180.0) {
            diff = 360.0 - diff;
        }
        const double score = diff - (k % 2 == 1 ? 7.5 : 0.0);
        if (score < bestScore) {
            bestScore = score;
            best = axis;
        }
    }
    return best;
}

// The straighten angle (degrees) for a reference line from `a` to `b`: the
// content rotation that levels it against the nearer of the horizontal and the
// vertical, i.e. the negative of its inclination from that axis, in (-45, 45].
// The drawing direction does not matter. A degenerate line is a no-op.
inline double straightenAngle(const QPointF& a, const QPointF& b)
{
    if (std::hypot(b.x() - a.x(), b.y() - a.y()) < 1e-6) {
        return 0.0;
    }
    double angle = -std::atan2(b.y() - a.y(), b.x() - a.x()) * 180.0 / M_PI;
    while (angle > 45.0) {
        angle -= 90.0;
    }
    while (angle <= -45.0) {
        angle += 90.0;
    }
    return angle;
}

// `box` scaled about its centre (never grown) so all four corners fall inside
// `content` rotated `degrees` about `pivot`, the rotation the crop preview
// draws (QTransform::rotate, clockwise on screen). A box whose centre lies
// outside the rotated content is returned unchanged.
inline QRectF fitBoxInRotated(const QRectF& box, const QRectF& content, double degrees,
                              const QPointF& pivot)
{
    const double rad = degrees * M_PI / 180.0;
    const double c = std::cos(rad);
    const double s = std::sin(rad);
    // Into the unrotated content frame: the inverse rotation about the pivot.
    const auto unrotate = [c, s](const QPointF& v) {
        return QPointF(c * v.x() + s * v.y(), -s * v.x() + c * v.y());
    };
    const QPointF centre = pivot + unrotate(box.center() - pivot);
    if (!content.contains(centre)) {
        return box;
    }
    double scale = 1.0;
    const QPointF half(box.width() / 2.0, box.height() / 2.0);
    for (const QPointF& corner : {QPointF(-half.x(), -half.y()), QPointF(half.x(), -half.y()),
                                  QPointF(half.x(), half.y()), QPointF(-half.x(), half.y())}) {
        const QPointF v = unrotate(corner);
        // The largest t with centre + t*v still inside, per axis.
        if (v.x() > 1e-12) scale = std::min(scale, (content.right() - centre.x()) / v.x());
        if (v.x() < -1e-12) scale = std::min(scale, (content.left() - centre.x()) / v.x());
        if (v.y() > 1e-12) scale = std::min(scale, (content.bottom() - centre.y()) / v.y());
        if (v.y() < -1e-12) scale = std::min(scale, (content.top() - centre.y()) / v.y());
    }
    QRectF fitted(0.0, 0.0, box.width() * scale, box.height() * scale);
    fitted.moveCenter(box.center());
    return fitted;
}

// The handle of `box` (image space) under `p`, within `grabPx` screen pixels at
// `zoom`; corners win over edges, and the body is Move.
inline BoxGrip boxGripAt(const QRectF& box, const QPointF& p, double zoom, double grabPx = 6.0)
{
    const double grab = grabPx / std::max(zoom, 1e-6);
    if (p.x() < box.left() - grab || p.x() > box.right() + grab || p.y() < box.top() - grab
        || p.y() > box.bottom() + grab) {
        return BoxGrip::None;
    }
    const bool left = std::abs(p.x() - box.left()) <= grab;
    const bool right = std::abs(p.x() - box.right()) <= grab;
    const bool top = std::abs(p.y() - box.top()) <= grab;
    const bool bottom = std::abs(p.y() - box.bottom()) <= grab;
    if (left && top) return BoxGrip::TopLeft;
    if (right && top) return BoxGrip::TopRight;
    if (left && bottom) return BoxGrip::BottomLeft;
    if (right && bottom) return BoxGrip::BottomRight;
    if (left) return BoxGrip::Left;
    if (right) return BoxGrip::Right;
    if (top) return BoxGrip::Top;
    if (bottom) return BoxGrip::Bottom;
    return box.contains(p) ? BoxGrip::Move : BoxGrip::None;
}

// `start` moved by `delta` through `grip`, normalized. With `mirror`, the
// dragged edge(s) move and the opposite edge(s) move just as far the other way,
// so the box scales about its centre (Alt-drag).
inline QRectF dragBox(const QRectF& start, BoxGrip grip, const QPointF& delta, bool mirror = false)
{
    QRectF r = start;
    if (mirror && grip != BoxGrip::None && grip != BoxGrip::Move) {
        const bool left = grip == BoxGrip::TopLeft || grip == BoxGrip::Left
            || grip == BoxGrip::BottomLeft;
        const bool right = grip == BoxGrip::TopRight || grip == BoxGrip::Right
            || grip == BoxGrip::BottomRight;
        const bool top = grip == BoxGrip::TopLeft || grip == BoxGrip::Top
            || grip == BoxGrip::TopRight;
        const bool bottom = grip == BoxGrip::BottomLeft || grip == BoxGrip::Bottom
            || grip == BoxGrip::BottomRight;
        r.setLeft(start.left() + (left ? delta.x() : right ? -delta.x() : 0.0));
        r.setRight(start.right() + (right ? delta.x() : left ? -delta.x() : 0.0));
        r.setTop(start.top() + (top ? delta.y() : bottom ? -delta.y() : 0.0));
        r.setBottom(start.bottom() + (bottom ? delta.y() : top ? -delta.y() : 0.0));
        return r.normalized();
    }
    switch (grip) {
    case BoxGrip::None: break;
    case BoxGrip::Move: r.translate(delta); break;
    case BoxGrip::TopLeft: r.setTopLeft(r.topLeft() + delta); break;
    case BoxGrip::Top: r.setTop(r.top() + delta.y()); break;
    case BoxGrip::TopRight: r.setTopRight(r.topRight() + delta); break;
    case BoxGrip::Right: r.setRight(r.right() + delta.x()); break;
    case BoxGrip::BottomRight: r.setBottomRight(r.bottomRight() + delta); break;
    case BoxGrip::Bottom: r.setBottom(r.bottom() + delta.y()); break;
    case BoxGrip::BottomLeft: r.setBottomLeft(r.bottomLeft() + delta); break;
    case BoxGrip::Left: r.setLeft(r.left() + delta.x()); break;
    }
    return r.normalized();
}

// Fit `ratio` (width / height, > 0) inside `box`, pivoting on the corner
// opposite `grip` so the edge under the pointer is the one that moves. An edge
// grip has only one dimension to drive, so the other follows from it; a corner
// keeps the larger fitting dimension.
inline QRectF fitRatio(const QRectF& box, double ratio, BoxGrip grip)
{
    if (ratio <= 0.0 || box.width() <= 0.0 || box.height() <= 0.0) {
        return box;
    }
    double width = box.width();
    double height = box.height();
    const bool horizontalEdge = grip == BoxGrip::Left || grip == BoxGrip::Right;
    const bool verticalEdge = grip == BoxGrip::Top || grip == BoxGrip::Bottom;
    if (horizontalEdge) {
        height = width / ratio;
    } else if (verticalEdge) {
        width = height * ratio;
    } else if (width / height > ratio) {
        width = height * ratio;
    } else {
        height = width / ratio;
    }
    const bool anchorLeft =
        grip != BoxGrip::TopLeft && grip != BoxGrip::Left && grip != BoxGrip::BottomLeft;
    const bool anchorTop =
        grip != BoxGrip::TopLeft && grip != BoxGrip::Top && grip != BoxGrip::TopRight;
    return QRectF(anchorLeft ? box.left() : box.right() - width,
                  anchorTop ? box.top() : box.bottom() - height, width, height);
}

// The move / resize cursor for a handle; `fallback` for None.
inline Qt::CursorShape boxGripCursor(BoxGrip grip, Qt::CursorShape fallback)
{
    switch (grip) {
    case BoxGrip::Move: return Qt::SizeAllCursor;
    case BoxGrip::TopLeft:
    case BoxGrip::BottomRight: return Qt::SizeFDiagCursor;
    case BoxGrip::TopRight:
    case BoxGrip::BottomLeft: return Qt::SizeBDiagCursor;
    case BoxGrip::Left:
    case BoxGrip::Right: return Qt::SizeHorCursor;
    case BoxGrip::Top:
    case BoxGrip::Bottom: return Qt::SizeVerCursor;
    case BoxGrip::None: break;
    }
    return fallback;
}

// The drawn handle glyphs of an active crop box, in widget space: four L-shaped
// corner brackets and four thin edge bars, all inset inside the box outline.
// Sizes are fixed screen pixels, clamped so a box smaller than a glyph does not
// produce degenerate strokes.
inline constexpr double kCropCornerArmPx = 18.0;
inline constexpr double kCropHandleBarLenPx = 22.0;
inline constexpr double kCropHandleBarThickPx = 4.0;
inline constexpr double kCropHandleInsetPx = 1.0;

struct CropHandleGlyphs {
    // Two strokes per corner, in top-left, top-right, bottom-right, bottom-left
    // order; each pair meets at the inset corner and runs inward along the edges.
    std::array<QLineF, 8> cornerStrokes{};
    // One bar per edge, in top, right, bottom, left order, hugging the inside.
    std::array<QRectF, 4> edgeBars{};
};

inline CropHandleGlyphs cropHandleGlyphs(const QRectF& box)
{
    CropHandleGlyphs g;
    if (box.isNull() || box.width() <= 0.0 || box.height() <= 0.0) {
        return g;
    }
    const double in = std::min(kCropHandleInsetPx, std::min(box.width(), box.height()) / 4.0);
    const double arm = std::min(kCropCornerArmPx, std::min(box.width(), box.height()) / 2.0 - in);
    const double thick = std::min(kCropHandleBarThickPx, std::min(box.width(), box.height()) / 4.0);
    const double lenX = std::min(kCropHandleBarLenPx, std::max(0.0, box.width() - 2.0 * in));
    const double lenY = std::min(kCropHandleBarLenPx, std::max(0.0, box.height() - 2.0 * in));
    // The bracket strokes are drawn as thick as the edge bars, so their centre
    // line sits half a thickness in from the corner to keep them inside the box.
    const double ci = std::min(std::max(kCropHandleInsetPx, kCropHandleBarThickPx / 2.0),
                               std::min(box.width(), box.height()) / 4.0);
    const double cl = box.left() + ci;
    const double ct = box.top() + ci;
    const double cr = box.right() - ci;
    const double cb = box.bottom() - ci;
    const QPointF c = box.center();
    g.cornerStrokes = {
        QLineF(cl, ct, cl, ct + arm), QLineF(cl, ct, cl + arm, ct),   // top-left
        QLineF(cr, ct, cr, ct + arm), QLineF(cr, ct, cr - arm, ct),   // top-right
        QLineF(cr, cb, cr, cb - arm), QLineF(cr, cb, cr - arm, cb),   // bottom-right
        QLineF(cl, cb, cl, cb - arm), QLineF(cl, cb, cl + arm, cb),   // bottom-left
    };
    g.edgeBars = {
        QRectF(c.x() - lenX / 2, box.top() + in, lenX, thick),          // top
        QRectF(box.right() - in - thick, c.y() - lenY / 2, thick, lenY), // right
        QRectF(c.x() - lenX / 2, box.bottom() - in - thick, lenX, thick), // bottom
        QRectF(box.left() + in, c.y() - lenY / 2, thick, lenY),         // left
    };
    return g;
}

} // namespace pictura
