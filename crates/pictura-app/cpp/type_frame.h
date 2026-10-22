#pragma once

#include <QtCore/QPointF>
#include <QtCore/QString>
#include <QtCore/QVector>
#include <QtGui/QPolygonF>

#include <array>
#include <optional>

namespace pictura {

// The path a `type_api::pathAt` hit carries: kind, closed, start, segment count,
// then eight numbers per cubic segment. `typePathPointString` encodes those
// segments as eight space-separated numbers each, `;` between segments;
// `typePathPolygon` rebuilds the outline for the overlay.
QString typePathPointString(const QVector<double>& hit);
QPolygonF typePathPolygon(const QString& points);

// The transform frame a Type session edits: where the text sits (`origin`),
// its linear transform (`matrix`, `xx, xy, yx, yy` as `TySh` orders it:
// x' = xx·u + yx·v, y' = xy·u + yy·v), paragraph type's box, the box point type
// shows while Ctrl is held, and the reference point; plus the gestures on that
// box, as CS6's paragraph box offers them. Boxes are left, top, right, bottom
// in the text's own frame (relative to the origin, before the matrix).
class TypeFrame {
public:
    enum class Gesture { None, Resize, Scale, Rotate, Pivot };

    QPointF origin;
    std::array<double, 4> matrix{1.0, 0.0, 0.0, 1.0};
    bool paragraph = false;
    // 0 none, 1 on-path, 2 area. A path session has no box gestures; area type
    // shows the closed path instead of a rectangle.
    int pathKind = 0;
    bool pathClosed = false;
    double pathStart = 0.0;
    QString pathPoints;
    QPolygonF pathPolygon;
    std::array<double, 4> box{0.0, 0.0, 0.0, 0.0};
    // Point type's text bounds while Ctrl shows its box.
    std::optional<std::array<double, 4>> pointBox;

    // The box the handles sit on: the paragraph box, else the Ctrl box; none.
    const std::array<double, 4>* frameBox() const;
    QPointF map(double u, double v) const;
    // `pos` in the text's frame under `matrix` about `origin`; `pos` itself
    // for a singular matrix.
    QPointF unmap(const QPointF& pos) const;
    // The frame box's corners (top-left, top-right, bottom-right, bottom-left)
    // in document pixels; empty without a box.
    QPolygonF polygon() const;
    QPointF pivotPoint() const;
    void movePivotBy(const QPointF& delta);
    // Back to the box centre (a new session).
    void resetPivot();

    // The handle within five screen pixels (TL, T, TR, R, BR, B, BL, L), or -1.
    int handleAt(const QPointF& pos, double zoom) const;
    // Within six screen pixels of the reference point.
    bool onPivot(const QPointF& pos, double zoom) const;
    // Just outside the box (within 24 screen pixels, off its handles): rotate.
    bool inRotateZone(const QPointF& pos, double zoom) const;

    // Start `gesture` from `press` (`handle` for Resize / Scale); `drag`
    // updates the frame from the pointer; `end` finishes it.
    void begin(Gesture gesture, int handle, const QPointF& press);
    void drag(const QPointF& pos);
    void end() { gesture_ = Gesture::None; }
    Gesture gesture() const { return gesture_; }

    // The frame box's size in document pixels (its edges under the matrix).
    double width() const;
    double height() const;
    // The rotation in degrees, clockwise on screen positive.
    double angleDegrees() const;

private:
    Gesture gesture_ = Gesture::None;
    int handle_ = -1;
    QPointF press_;
    QPointF origin0_;
    std::array<double, 4> matrix0_{};
    std::array<double, 4> box0_{};
    QPointF pivot0_;
    std::optional<QPointF> pivot_;
};

} // namespace pictura
