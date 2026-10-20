#pragma once

#include <array>

#include <QtCore/QList>

#include <QtCore/QString>
#include <QtGui/QPolygon>

namespace pictura {
class PictureView;
}

namespace pictura::transform_api {

/// A layer's document-space rectangle; invalid when `path` does not resolve.
struct LayerRect {
    int left = 0;
    int top = 0;
    int right = 0;
    int bottom = 0;
    bool valid = false;

    int width() const { return right - left; }
    int height() const { return bottom - top; }
    /// `"left top right bottom"`, or empty when invalid.
    QString toString() const;
};

/// A transform quad's four document-space corners; empty without a session.
struct Quad {
    QPolygonF corners;
    bool isEmpty() const { return corners.isEmpty(); }
    /// `"x,y x,y x,y x,y"` (2-decimal corners), matching the old string form.
    QString toString() const;
};

/// The nine preview-map coefficients in `QTransform` constructor order.
struct Matrix {
    std::array<double, 9> coefficients{};
    bool valid = false;
};

/// One draggable point of a custom warp.
struct WarpPoint {
    enum Kind { Anchor = 0, Handle = 1, DraggedHandle = 2 };
    Kind kind = Anchor;
    QPointF pos;
    /// The anchor a handle belongs to (an anchor's own position).
    QPointF anchor;
};

/// Warp mode's overlay in document space: the patch boundaries, the
/// rule-of-thirds guides, and a custom warp's points. Empty when not warping.
struct WarpOverlay {
    QList<QPolygonF> grid;
    QList<QPolygonF> guides;
    QList<WarpPoint> points;
    bool isEmpty() const { return grid.isEmpty(); }
};

LayerRect layerRect(const PictureView& view, const QString& path);
/// A layer's ink box: the box round its non-transparent pixels, as Align
/// computes it. Invalid when `path` does not resolve or has no content.
LayerRect contentRect(const PictureView& view, const QString& path);
Quad quad(const PictureView& view);
Matrix previewMatrix(const PictureView& view);
Matrix previewLocalMatrix(const PictureView& view);
WarpOverlay warpOverlay(const PictureView& view);

} // namespace pictura::transform_api
