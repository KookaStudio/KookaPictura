// Path Blur's half of the Blur Gallery canvas: each pin is a path drawn as
// the curve through its points, with a dot per point and an arrow past its
// end. As in the owner's CC walkthrough, a click on empty picture pins a
// path's first stop point and the curve then follows the pointer, each
// further click pinning another stop; a double-click, Enter, or Esc ends the
// path. Dragging a point moves it, pressing the curve inserts a point, and
// Ctrl-drag moves the whole path (inferred).

#include "blur_gallery_canvas.h"

#include "icons.h"

#include <QtCore/QLineF>
#include <QtGui/QPainter>
#include <QtGui/QPainterPath>
#include <QtGui/QPolygonF>

#include <algorithm>
#include <cmath>

namespace pictura {

namespace {

const double kPoint = 4.5;
const double kGrab = 5.0;
// As `pictura_filters::MAX_PATH_POINTS` and `MAX_FIELD_PINS`.
const int kMaxPoints = 32;
const int kMaxPaths = 16;
// Curve samples per span, as the engine's.
const int kSpanSegments = 16;
// A path ended after its first stop runs this far right (a fraction of the
// picture's width): the opening path's length.
const double kDefaultLength = 0.08;
const QColor kSelectedPath(0x2c, 0xa0, 0xdc);

// The pushpin with a plus: a click places a stop point. Its hotspot is the
// pin's tip.
QCursor pinCursor()
{
    return cursor(QStringLiteral("cursor.pinAdd"), 4, 20);
}
const QColor kPath(0x8c, 0xbc, 0xd0, 220);

// The uniform Catmull-Rom curve through `points`, the ends' outer control
// points mirrored, as `pictura_filters::blur::path` samples it.
QList<QPointF> curveThrough(const QList<QPointF>& points)
{
    const int last = int(points.size()) - 1;
    QList<QPointF> samples;
    if (last < 1) {
        return points;
    }
    const auto at = [&](int i) {
        if (i < 0) {
            return 2.0 * points[0] - points[1];
        }
        if (i > last) {
            return 2.0 * points[last] - points[last - 1];
        }
        return points[i];
    };
    for (int span = 0; span < last; ++span) {
        const QPointF p0 = at(span - 1), p1 = at(span), p2 = at(span + 1), p3 = at(span + 2);
        for (int k = 0; k < kSpanSegments; ++k) {
            const double t = double(k) / kSpanSegments;
            const double t2 = t * t, t3 = t2 * t;
            samples << 0.5
                           * (2.0 * p1 + (p2 - p0) * t + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
                              + (3.0 * p1 - p0 - 3.0 * p2 + p3) * t3);
        }
    }
    samples << points[last];
    return samples;
}

// The nearest point on segment `a`–`b` to `p`.
QPointF nearestOnSegment(const QPointF& p, const QPointF& a, const QPointF& b)
{
    const QPointF d = b - a;
    const double length2 = d.x() * d.x() + d.y() * d.y();
    if (length2 <= 0.0) {
        return a;
    }
    const QPointF r = p - a;
    const double t = std::clamp((r.x() * d.x() + r.y() * d.y()) / length2, 0.0, 1.0);
    return a + d * t;
}

} // namespace

int BlurGalleryCanvas::hitPoint(const QPointF& widget, int* point) const
{
    if (!pins_) {
        return -1;
    }
    // The selected path first, so its points win over another's on top.
    QList<int> order;
    if (selected_ >= 0 && selected_ < pins_->size()) {
        order << selected_;
    }
    for (int i = pins_->size() - 1; i >= 0; --i) {
        if (i != selected_) {
            order << i;
        }
    }
    for (const int i : order) {
        const QList<QPointF>& points = pins_->at(i).points;
        for (int k = 0; k < points.size(); ++k) {
            if (QLineF(widget, toWidget(points[k])).length() <= kPoint + kGrab) {
                *point = k;
                return i;
            }
        }
    }
    return -1;
}

int BlurGalleryCanvas::hitCurve(const QPointF& widget, int* span, QPointF* at) const
{
    if (!pins_) {
        return -1;
    }
    double best = kGrab + 1.0;
    int hit = -1;
    for (int i = 0; i < pins_->size(); ++i) {
        QList<QPointF> points;
        for (const QPointF& point : pins_->at(i).points) {
            points << toWidget(point);
        }
        const QList<QPointF> samples = curveThrough(points);
        for (int k = 0; k + 1 < samples.size(); ++k) {
            const QPointF nearest = nearestOnSegment(widget, samples[k], samples[k + 1]);
            const double distance = QLineF(widget, nearest).length();
            if (distance < best) {
                best = distance;
                hit = i;
                *span = std::min(k / kSpanSegments, int(points.size()) - 2);
                *at = nearest;
            }
        }
    }
    return hit;
}

void BlurGalleryCanvas::pressPath(const QPointF& widget, Qt::KeyboardModifiers modifiers)
{
    if (building_) {
        addStop(widget);
        return;
    }
    const bool whole = modifiers & Qt::ControlModifier;
    const auto select = [this](int path, int point) {
        selected_ = path;
        selectedPoint_ = point;
        emit pinSelected(path);
    };
    int point = -1;
    int path = hitPoint(widget, &point);
    if (path >= 0) {
        select(path, point);
        drag_ = whole ? Drag::Path : Drag::Point;
    } else {
        int span = 0;
        QPointF at;
        path = hitCurve(widget, &span, &at);
        if (path >= 0 && !whole && pins_->at(path).points.size() < kMaxPoints) {
            (*pins_)[path].points.insert(span + 1, toFraction(at));
            select(path, span + 1);
            drag_ = Drag::Point;
            emit pinsEdited();
        } else if (path >= 0) {
            select(path, -1);
            drag_ = Drag::Path;
        } else if (imageRect().contains(widget) && pins_->size() < kMaxPaths) {
            // A new path copies the selected one's end speeds. Its last point
            // floats with the pointer until the path ends.
            BlurGalleryPin pin;
            if (selected_ >= 0 && selected_ < pins_->size()) {
                pin = pins_->at(selected_);
            }
            pin.points = {toFraction(widget), toFraction(widget)};
            pins_->append(pin);
            selected_ = pins_->size() - 1;
            selectedPoint_ = 0;
            building_ = true;
            setCursor(pinCursor());
            emit pinAdded(selected_);
            emit pinsEdited();
            update();
            return;
        } else {
            return;
        }
    }
    dragStart_ = widget;
    dragPoints_ = pins_->at(selected_).points;
    update();
}

void BlurGalleryCanvas::dragPath(const QPointF& widget)
{
    if (!pins_ || selected_ < 0 || selected_ >= pins_->size()) {
        return;
    }
    QList<QPointF>& points = (*pins_)[selected_].points;
    if (drag_ == Drag::Path) {
        const QPointF delta = toFraction(widget) - toFraction(dragStart_);
        for (int k = 0; k < points.size() && k < dragPoints_.size(); ++k) {
            points[k] = dragPoints_[k] + delta;
        }
    } else if (selectedPoint_ >= 0 && selectedPoint_ < points.size()) {
        points[selectedPoint_] = toFraction(widget);
    }
}

void BlurGalleryCanvas::addStop(const QPointF& widget)
{
    QList<QPointF>& points = (*pins_)[selected_].points;
    // A click on the last stop ends the path there.
    const bool onLast = points.size() >= 2
                        && QLineF(widget, toWidget(points[points.size() - 2])).length()
                               <= kPoint + kGrab;
    if (onLast || points.size() >= kMaxPoints) {
        finishPath();
        return;
    }
    points.last() = toFraction(widget);
    points.append(toFraction(widget));
    selectedPoint_ = int(points.size()) - 2;
    emit pinsEdited();
    update();
}

void BlurGalleryCanvas::moveFloatingStop(const QPointF& widget)
{
    (*pins_)[selected_].points.last() = toFraction(widget);
    emit pinsEdited();
    update();
}

void BlurGalleryCanvas::finishPath()
{
    if (!building_) {
        return;
    }
    building_ = false;
    if (pins_ && selected_ >= 0 && selected_ < pins_->size()) {
        QList<QPointF>& points = (*pins_)[selected_].points;
        points.removeLast();
        if (points.size() < 2) {
            // Ended on its first stop: the opening path's length, heading right.
            points.append(points.first() + QPointF(kDefaultLength, 0.0));
        }
        selectedPoint_ = int(points.size()) - 1;
        emit pinSelected(selected_);
        emit pinsEdited();
    }
    update();
    emit dragFinished();
}

void BlurGalleryCanvas::updatePathCursor(const QPointF& widget)
{
    // Over a point the arrow (it drags); elsewhere the pin that places a stop.
    int point = -1;
    if (!building_ && hitPoint(widget, &point) >= 0) {
        unsetCursor();
    } else {
        setCursor(pinCursor());
    }
}

void BlurGalleryCanvas::paintPaths(QPainter& painter) const
{
    // Unselected paths first, the selected one last, so it reads on top.
    for (int pass = 0; pass < 2; ++pass) {
        for (int i = 0; i < pins_->size(); ++i) {
            const bool selected = i == selected_;
            if ((pass == 0) == selected) {
                continue;
            }
            QList<QPointF> points;
            for (const QPointF& point : pins_->at(i).points) {
                points << toWidget(point);
            }
            if (points.size() < 2) {
                continue;
            }
            const QList<QPointF> samples = curveThrough(points);
            QPainterPath curve(samples.first());
            for (int k = 1; k < samples.size(); ++k) {
                curve.lineTo(samples[k]);
            }
            const QColor colour = selected ? kSelectedPath : kPath;
            painter.setBrush(Qt::NoBrush);
            painter.setPen(QPen(QColor(0, 0, 0, 110), 3.4));
            painter.drawPath(curve);
            painter.setPen(QPen(colour, selected ? 2.0 : 1.6));
            painter.drawPath(curve);

            // The arrow sits just past the end point along the last tangent.
            const QPointF tip = samples.last();
            QPointF along = tip - samples[samples.size() - 2];
            const double length = std::hypot(along.x(), along.y());
            if (length > 0.0) {
                along /= length;
                const QPointF across(-along.y(), along.x());
                const QPointF base = tip + along * (kPoint + 1.0);
                const QPointF point = base + along * 8.0;
                painter.setPen(QPen(QColor(0, 0, 0, 140), 1.0));
                painter.setBrush(colour);
                painter.drawPolygon(
                    QPolygonF({point, base + across * 4.5, base - across * 4.5}));
            }
            for (int k = 0; k < points.size(); ++k) {
                const bool chosen = selected && k == selectedPoint_;
                painter.setPen(QPen(chosen ? QColor(255, 255, 255) : QColor(0, 0, 0, 170), 1.4));
                painter.setBrush(chosen ? QColor(0x20, 0x20, 0x20) : QColor(255, 255, 255));
                painter.drawEllipse(points[k], kPoint, kPoint);
            }
        }
    }
}

} // namespace pictura
