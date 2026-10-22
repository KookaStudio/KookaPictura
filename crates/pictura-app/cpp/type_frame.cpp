// A Type session's transform frame and its box gestures: resize (paragraph
// type's box reflows the text), scale (Ctrl, or point type's box: the text
// itself scales), rotate (just outside the box, about the reference point), and
// the reference point's own drag.
//
// ponytail: no Shift constraints (proportional scale, 15° rotation steps) and
// no skew; Free Transform on the committed layer has them.

#include "type_frame.h"

#include <QtCore/QStringList>

#include <QtCore/QLineF>
#include <QtCore/QRectF>
#include <QtCore/QtMath>

#include <algorithm>
#include <cmath>

namespace pictura {

void TypeFrame::load(const TypeSetting& s)
{
    origin = QPointF(s.x, s.y);
    matrix = {s.xx, s.xy, s.yx, s.yy};
    paragraph = s.boxed;
    box = {s.box_left, s.box_top, s.box_right, s.box_bottom};
    pathKind = s.path_kind;
    pathClosed = s.path_closed;
    pathStart = s.path_start;
    pathPoints = s.path_points;
    pathPolygon = typePathPolygon(s.path_points);
    if (pathKind == 2) {
        paragraph = true;
        const QRectF bounds = pathPolygon.boundingRect();
        origin = bounds.topLeft();
        box = {0.0, 0.0, bounds.width(), bounds.height()};
    }
    warpStyle = s.warp_style;
    warpValues = {s.warp_bend, s.warp_h, s.warp_v};
    warpVertical = s.warp_vertical;
}

void TypeFrame::store(TypeSetting& s) const
{
    s.x = origin.x();
    s.y = origin.y();
    s.xx = matrix[0];
    s.xy = matrix[1];
    s.yx = matrix[2];
    s.yy = matrix[3];
    s.boxed = paragraph;
    s.box_left = box[0];
    s.box_top = box[1];
    s.box_right = box[2];
    s.box_bottom = box[3];
    s.path_kind = pathKind;
    s.path_closed = pathClosed;
    s.path_start = pathStart;
    s.path_points = pathPoints;
    s.warp_style = warpStyle;
    s.warp_bend = warpValues[0];
    s.warp_h = warpValues[1];
    s.warp_v = warpValues[2];
    s.warp_vertical = warpVertical;
}

QString typePathPointString(const QVector<double>& hit)
{
    QStringList segments;
    const int count = hit.size() >= 4 ? int(hit.at(3)) : 0;
    for (int i = 0; i < count; ++i) {
        const int base = 4 + i * 8;
        if (base + 7 >= hit.size()) {
            break;
        }
        QStringList values;
        for (int j = 0; j < 8; ++j) {
            values << QString::number(hit.at(base + j), 'g', 17);
        }
        segments << values.join(QLatin1Char(' '));
    }
    return segments.join(QLatin1Char(';'));
}

QPolygonF typePathPolygon(const QString& points)
{
    QPolygonF polygon;
    const QStringList segments = points.split(QLatin1Char(';'), Qt::SkipEmptyParts);
    for (int i = 0; i < segments.size(); ++i) {
        const QStringList values = segments.at(i).split(QLatin1Char(' '), Qt::SkipEmptyParts);
        if (values.size() != 8) {
            continue;
        }
        if (i == 0) {
            polygon << QPointF(values.at(0).toDouble(), values.at(1).toDouble());
        }
        polygon << QPointF(values.at(6).toDouble(), values.at(7).toDouble());
    }
    return polygon;
}

namespace {

// The handles in overlay order (TL, T, TR, R, BR, B, BL, L) as the edge each
// moves across and down: -1 left / top, 1 right / bottom, 0 neither.
constexpr std::array<std::array<int, 2>, 8> kHandles{
    {{-1, -1}, {0, -1}, {1, -1}, {1, 0}, {1, 1}, {0, 1}, {-1, 1}, {-1, 0}}};

double side(int sign, double low, double high)
{
    return sign < 0 ? low : sign > 0 ? high : (low + high) / 2.0;
}

double distanceToSegment(const QPointF& p, const QPointF& a, const QPointF& b)
{
    const QPointF d = b - a;
    const double length2 = d.x() * d.x() + d.y() * d.y();
    const double t = length2 > 0.0
        ? std::clamp(((p.x() - a.x()) * d.x() + (p.y() - a.y()) * d.y()) / length2, 0.0, 1.0)
        : 0.0;
    return QLineF(p, a + t * d).length();
}

} // namespace

const std::array<double, 4>* TypeFrame::frameBox() const
{
    if (paragraph) {
        return &box;
    }
    return pointBox ? &*pointBox : nullptr;
}

QPointF TypeFrame::map(double u, double v) const
{
    return QPointF(origin.x() + matrix[0] * u + matrix[2] * v,
                   origin.y() + matrix[1] * u + matrix[3] * v);
}

QPointF TypeFrame::unmap(const QPointF& pos) const
{
    const double det = matrix[0] * matrix[3] - matrix[2] * matrix[1];
    if (qFuzzyIsNull(det)) {
        return pos;
    }
    const QPointF d = pos - origin;
    return QPointF((matrix[3] * d.x() - matrix[2] * d.y()) / det,
                   (matrix[0] * d.y() - matrix[1] * d.x()) / det);
}

QPolygonF TypeFrame::polygon() const
{
    const std::array<double, 4>* b = frameBox();
    if (!b) {
        return QPolygonF();
    }
    const auto& r = *b;
    return QPolygonF({map(r[0], r[1]), map(r[2], r[1]), map(r[2], r[3]), map(r[0], r[3])});
}

QPointF TypeFrame::pivotPoint() const
{
    if (pivot_) {
        return *pivot_;
    }
    const std::array<double, 4>* b = frameBox();
    return b ? map(((*b)[0] + (*b)[2]) / 2.0, ((*b)[1] + (*b)[3]) / 2.0) : origin;
}

void TypeFrame::movePivotBy(const QPointF& delta)
{
    if (pivot_) {
        *pivot_ += delta;
    }
}

void TypeFrame::resetPivot()
{
    pivot_.reset();
}

int TypeFrame::handleAt(const QPointF& pos, double zoom) const
{
    const std::array<double, 4>* b = frameBox();
    if (!b) {
        return -1;
    }
    for (int i = 0; i < int(kHandles.size()); ++i) {
        const QPointF h = map(side(kHandles[i][0], (*b)[0], (*b)[2]),
                              side(kHandles[i][1], (*b)[1], (*b)[3]));
        if (QLineF(pos, h).length() * zoom <= 5.0) {
            return i;
        }
    }
    return -1;
}

bool TypeFrame::onPivot(const QPointF& pos, double zoom) const
{
    return frameBox() && QLineF(pos, pivotPoint()).length() * zoom <= 6.0;
}

bool TypeFrame::inRotateZone(const QPointF& pos, double zoom) const
{
    const QPolygonF quad = polygon();
    if (quad.size() < 4 || quad.containsPoint(pos, Qt::OddEvenFill) || handleAt(pos, zoom) >= 0) {
        return false;
    }
    double nearest = -1.0;
    for (int i = 0; i < 4; ++i) {
        const double d = distanceToSegment(pos, quad.at(i), quad.at((i + 1) % 4));
        nearest = nearest < 0.0 ? d : std::min(nearest, d);
    }
    return nearest * zoom <= 24.0;
}

void TypeFrame::begin(Gesture gesture, int handle, const QPointF& press)
{
    gesture_ = gesture;
    handle_ = handle;
    press_ = press;
    origin0_ = origin;
    matrix0_ = matrix;
    if (const std::array<double, 4>* b = frameBox()) {
        box0_ = *b;
    }
    pivot0_ = pivotPoint();
}

void TypeFrame::drag(const QPointF& pos)
{
    switch (gesture_) {
    case Gesture::None:
        return;
    case Gesture::Pivot:
        pivot_ = pivot0_ + (pos - press_);
        return;
    case Gesture::Rotate: {
        const QPointF a = press_ - pivot0_;
        const QPointF b = pos - pivot0_;
        const double t = std::atan2(b.y(), b.x()) - std::atan2(a.y(), a.x());
        const double c = std::cos(t);
        const double s = std::sin(t);
        const auto& m = matrix0_;
        matrix = {c * m[0] - s * m[1], s * m[0] + c * m[1], c * m[2] - s * m[3],
                  s * m[2] + c * m[3]};
        const QPointF o = origin0_ - pivot0_;
        origin = pivot0_ + QPointF(c * o.x() - s * o.y(), s * o.x() + c * o.y());
        pivot_ = pivot0_;
        return;
    }
    case Gesture::Resize:
    case Gesture::Scale:
        break;
    }
    if (handle_ < 0 || handle_ >= int(kHandles.size())) {
        return;
    }
    // The pointer in the frame the gesture started in.
    TypeFrame start = *this;
    start.origin = origin0_;
    start.matrix = matrix0_;
    const QPointF p = start.unmap(pos);
    const auto [sx, sy] = kHandles[handle_];
    const auto& b0 = box0_;
    if (gesture_ == Gesture::Resize) {
        box = b0;
        if (sx < 0) {
            box[0] = std::min(p.x(), b0[2] - 1.0);
        } else if (sx > 0) {
            box[2] = std::max(p.x(), b0[0] + 1.0);
        }
        if (sy < 0) {
            box[1] = std::min(p.y(), b0[3] - 1.0);
        } else if (sy > 0) {
            box[3] = std::max(p.y(), b0[1] + 1.0);
        }
        return;
    }
    // Scale about the opposite edge (or corner): the frame's columns stretch,
    // and the origin moves so that anchor stays put.
    const auto factor = [](int sign, double low, double high, double at) {
        if (sign == 0) {
            return 1.0;
        }
        const double anchor = sign < 0 ? high : low;
        const double grip = sign < 0 ? low : high;
        const double span = grip - anchor;
        return qFuzzyIsNull(span) ? 1.0 : std::max((at - anchor) / span, 0.01);
    };
    const double kx = factor(sx, b0[0], b0[2], p.x());
    const double ky = factor(sy, b0[1], b0[3], p.y());
    const double fx = sx < 0 ? b0[2] : b0[0];
    const double fy = sy < 0 ? b0[3] : b0[1];
    const auto& m = matrix0_;
    matrix = {m[0] * kx, m[1] * kx, m[2] * ky, m[3] * ky};
    const QPointF fixed0(m[0] * fx + m[2] * fy, m[1] * fx + m[3] * fy);
    const QPointF fixed(matrix[0] * fx + matrix[2] * fy, matrix[1] * fx + matrix[3] * fy);
    origin = origin0_ + fixed0 - fixed;
}

double TypeFrame::width() const
{
    const std::array<double, 4>* b = frameBox();
    return b ? std::hypot(matrix[0], matrix[1]) * ((*b)[2] - (*b)[0]) : 0.0;
}

double TypeFrame::height() const
{
    const std::array<double, 4>* b = frameBox();
    return b ? std::hypot(matrix[2], matrix[3]) * ((*b)[3] - (*b)[1]) : 0.0;
}

double TypeFrame::angleDegrees() const
{
    return qRadiansToDegrees(std::atan2(matrix[1], matrix[0]));
}

} // namespace pictura
