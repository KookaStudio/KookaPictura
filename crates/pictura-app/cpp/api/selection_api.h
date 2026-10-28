#pragma once

#include <QtCore/QString>
#include <QtCore/QtGlobal>
#include <QtCore/QVector>
#include <QtGui/QPolygon>

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

} // namespace pictura::selection_api
