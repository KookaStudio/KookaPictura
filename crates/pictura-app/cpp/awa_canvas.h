#pragma once

// The Adaptive Wide Angle workspace's preview: the active layer through the
// solved correction (or, with Preview off, as it is) on a checkerboard over
// CS6's grey ground, rendered by the Rust session
// (`filter_tools/adaptive_wide_angle.rs`) at the screen's resolution. Over it
// sit the constraints in their state colours, the selected constraint's
// handles, and Show Mesh's solved grid. "Display" coordinates are output
// pixels with Preview on and layer pixels with it off; the frame is the
// layer's size either way.
//
// Constraint drags a line (Shift: Horizontal or Vertical by the drag's
// dominant axis); Polygon clicks corners and closes on the first (E drops
// the last corner, Esc the polygon); Move drags the result. A click on a
// constraint selects it; its handles drag its points; Delete removes it and
// a right click offers its orientations. Hand drags (or Space) pan, Zoom
// clicks (Alt: out) zoom, and a right click with Zoom offers CS6's levels.

#include <QtCore/QList>
#include <QtCore/QPointF>
#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtGui/QPainterPath>
#include <QtWidgets/QWidget>

#include <functional>

class QAction;
class QMenu;

namespace pictura {

class AwaCanvas : public QWidget {
    Q_OBJECT

public:
    enum class Mode { Constraint, Polygon, Move, Hand, Zoom };

    explicit AwaCanvas(QWidget* parent = nullptr);

    // Re-read the session's layer size and open at 100%, centred.
    void reload();
    // Re-render the view and re-read the constraints after a session edit.
    void refreshAll();

    void setMode(Mode mode);
    Mode mode() const { return mode_; }
    void setPreview(bool on);
    bool preview() const { return preview_; }
    void setShowConstraints(bool on);
    void setShowMesh(bool on);
    bool meshShown() const { return showMesh_; }

    // Screen pixels per display pixel.
    double zoom() const { return zoom_; }
    // Zoom about the widget centre, clamped to 6.25%–1600%.
    void setZoom(double zoom);
    void zoomBy(double factor);
    void fitInView();
    void fitOnScreen();
    QList<QAction*> zoomMenuActions(QObject* owner);
    void showZoomMenu(const QPoint& globalPos);

    QPointF toDisplay(const QPointF& widget) const;
    QPointF toWidget(const QPointF& display) const;

    // The selected constraint, or -1.
    int selected() const { return selected_; }
    void select(int index);
    // The constraint under widget point `pos`, or -1.
    int constraintAt(const QPointF& pos) const;
    // A constraint's orientation menu: Unfixed, Horizontal, Vertical,
    // Arbitrary (lines only), Delete. Owned by `owner`.
    QMenu* constraintMenu(int index, QWidget* owner);

    // The colour a constraint state (0 Unfixed … 4 invalid) draws in.
    static QColor stateColor(int state);
    QColor constraintColor(int index) const;

    // Polygon corners clicked so far.
    int pendingCorners() const { return static_cast<int>(corners_.size()); }
    void removeLastCorner();
    void cancelPolygon();

    // The drawn mesh in widget coordinates, for tests.
    QPainterPath meshPathForTest() const;
    const QImage& previewImage() const { return view_; }

    std::function<void()> zoomChanged;
    // The display point under the pointer, for the Detail view.
    std::function<void(const QPointF& display)> hoverMoved;

protected:
    void paintEvent(QPaintEvent* event) override;
    void resizeEvent(QResizeEvent* event) override;
    void mousePressEvent(QMouseEvent* event) override;
    void mouseMoveEvent(QMouseEvent* event) override;
    void mouseReleaseEvent(QMouseEvent* event) override;
    void wheelEvent(QWheelEvent* event) override;
    void keyPressEvent(QKeyEvent* event) override;
    void keyReleaseEvent(QKeyEvent* event) override;

private:
    struct Shape {
        QPainterPath path; // display coordinates
        QList<QPointF> points;
        int state = 0;
        bool polygon = false;
    };

    Mode effectiveMode() const;
    void updateCursor();
    void zoomAbout(double zoom, const QPointF& anchor);
    void fit(bool capAtActualPixels);
    void viewChanged();
    void clampOffset();
    void readShapes();
    // The layer point widget point `pos` shows, if any.
    bool layerAt(const QPointF& pos, QPointF* layer) const;
    int handleAt(const QPointF& pos) const;
    QPainterPath arcPath(const QPointF& a, const QPointF& b) const;
    void addCorner(const QPointF& pos);

    QImage view_;
    QList<Shape> shapes_;
    QPainterPath mesh_;
    bool preview_ = true;
    bool showConstraints_ = true;
    bool showMesh_ = false;
    int layerWidth_ = 0;
    int layerHeight_ = 0;
    double zoom_ = 1.0;
    QPointF offset_;
    bool placed_ = false;

    Mode mode_ = Mode::Constraint;
    bool spaceHeld_ = false;
    int selected_ = -1;

    enum class Drag { None, Pan, Line, Handle, Move };
    Drag drag_ = Drag::None;
    QPointF pressWidget_;
    QPointF panOffset_;
    QPointF lineStart_; // layer
    QPointF lineEnd_;   // layer
    bool lineEndValid_ = false;
    int handle_ = -1;
    QList<double> moveStart_;
    QList<QPointF> corners_;        // layer
    QList<QPointF> cornerDisplay_;  // display, where each was clicked
    QPointF hover_;
    bool hovering_ = false;
};

// CS6's Detail pane: the corrected picture at 100% around the pointer.
class AwaDetail : public QWidget {
    Q_OBJECT

public:
    explicit AwaDetail(QWidget* parent = nullptr);

    void showAt(const QPointF& display, bool preview);
    const QImage& image() const { return image_; }

protected:
    void paintEvent(QPaintEvent* event) override;

private:
    QImage image_;
};

} // namespace pictura
