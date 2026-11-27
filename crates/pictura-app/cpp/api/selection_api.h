#pragma once

#include "tool_options.h"

#include <QtCore/QList>
#include <QtCore/QPointF>
#include <QtCore/QRect>
#include <QtCore/QSize>
#include <QtCore/QString>
#include <QtCore/QtGlobal>
#include <QtCore/QVector>
#include <QtGui/QPainterPath>
#include <QtGui/QPolygon>
#include <QtGui/QPolygonF>

namespace pictura {
class PictureView;
}

namespace pictura::selection_api {

/// The selected pixels' document-space bounding box; invalid without a selection.
struct SelectionBounds {
    int x = 0;
    int y = 0;
    int width = 0;
    int height = 0;
    bool valid = false;

    /// `"x y w h"` for the control server's wire format and logs; empty when invalid.
    QString toString() const;
};

/// The marching-ants outline: one loop of integer pixel corners per contour.
struct SelectionContour {
    QVector<QPolygonF> loops;
    /// The selection revision this contour was traced for; a consumer may skip
    /// applying a contour it already shows.
    quint64 revision = 0;
    bool isEmpty() const { return loops.isEmpty(); }
    int loopCount() const { return loops.size(); }
    /// Whether any loop carries `(x, y)` as a vertex.
    bool hasVertex(int x, int y) const;
};

/// One typed crossing each: bounds as `[x, y, w, h]` and contour loops as
/// point-count-prefixed integer corners; both empty without a selection.
SelectionBounds bounds(const PictureView& view);
SelectionContour contour(const PictureView& view);
QPainterPath committedContourPath(const PictureView& view);
QList<QPolygonF> rectPreviewContour(const PictureView& view, const QRect& rect,
                                   const QString& mode, double feather,
                                   const QPainterPath& committed);
QRect singleLineBand(const QSize& size, const QPointF& pos, bool row);

QRect marqueeDragRect(const QPointF& a, const QPointF& b, Qt::KeyboardModifiers mods,
                      bool mirror, MarqueeStyle style, double fixedRatioW, double fixedRatioH,
                      int fixedSizeW, int fixedSizeH);
bool withinRadius(const QPointF& a, const QPointF& b, double radius);

/// Engine-backed clipping for live selection-preview loops.
QList<QPolygonF> clipSelectionShapeToCanvas(const QList<QPolygonF>& loops, const QSize& canvas);

} // namespace pictura::selection_api
