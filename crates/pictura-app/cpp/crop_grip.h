#pragma once

// The eight-handle box geometry shared by the Crop and Slice Select tools:
// which handle (or the body) a point grabs, where a drag takes the box, and the
// cursor for each. Ported from photorust's CanvasView gripAt/dragCrop/cropCursor.

#include <QtCore/QLineF>
#include <QtCore/QPointF>
#include <QtCore/QRectF>
#include <QtCore/Qt>

#include <algorithm>
#include <cmath>

namespace pictura {

enum class BoxGrip { None, Move, TopLeft, Top, TopRight, Right, BottomRight, Bottom, BottomLeft, Left };

// Screen-pixel margin outside the crop box that still belongs to the resize
// handles; just beyond it (and inside the canvas) a press rotates the content.
inline constexpr double kCropRotateMarginPx = 8.0;

// True when `p` (image space) is outside `box` by at least the rotate margin
// (so it does not fight the resize handles). Ignoring the canvas: with an active
// crop box the whole region outside it rotates, including past the canvas edge.
inline bool cropRotateZone(const QRectF& box, const QPointF& p, double zoom,
                           double marginPx = kCropRotateMarginPx)
{
    if (box.isNull()) {
        return false;
    }
    const double margin = marginPx / std::max(zoom, 1e-6);
    const QPointF clamped(std::clamp(p.x(), box.left(), box.right()),
                          std::clamp(p.y(), box.top(), box.bottom()));
    return std::hypot(p.x() - clamped.x(), p.y() - clamped.y()) > margin;
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

// The straighten angle (degrees) for a horizon line from `a` to `b`: the
// content rotation that levels it, i.e. the negative of the line's inclination.
// A degenerate line is a no-op.
inline double straightenAngle(const QPointF& a, const QPointF& b)
{
    if (std::hypot(b.x() - a.x(), b.y() - a.y()) < 1e-6) {
        return 0.0;
    }
    return -std::atan2(b.y() - a.y(), b.x() - a.x()) * 180.0 / M_PI;
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

// `start` moved by `delta` through `grip`, normalized.
inline QRectF dragBox(const QRectF& start, BoxGrip grip, const QPointF& delta)
{
    QRectF r = start;
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
// opposite `grip` so the edge under the pointer is the one that moves.
inline QRectF fitRatio(const QRectF& box, double ratio, BoxGrip grip)
{
    if (ratio <= 0.0 || box.width() <= 0.0 || box.height() <= 0.0) {
        return box;
    }
    double width = box.width();
    double height = box.height();
    if (width / height > ratio) {
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

} // namespace pictura
