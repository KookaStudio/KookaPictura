#pragma once

// The Vanishing Point dialog's preview: the document with the edited layer
// on CS6's grey ground, rendered by the Rust session
// (`filter_tools/vanishing_point*.rs`) and patched only where it changed.
// Over it sit every plane's outline and grid in its validity colour (blue
// valid, red no aspect ratio, yellow no camera), the active plane's nodes,
// the marquee and the floating selection's outline. Coordinates here are
// document pixels.
//
// Edit Plane: click a plane to select it, drag a corner to reshape, an edge
// node to resize in perspective, the inside to move; Ctrl-drag an edge node
// tears off a related plane, Alt-drag a related plane's far edge node turns
// it, and Ctrl-click cycles through overlapping planes. Create Plane: click
// four corners (Backspace drops the last, Esc all), then the dialog returns
// to Edit Plane; nodes still drag. Delete removes the active plane.
// Marquee: drag a rect (Shift: square), double-click a plane to select it,
// drag inside to move it, Alt-drag to float a copy, Ctrl-drag to fill it
// from under the pointer; a floating selection drags. Transform: corner
// handles scale (Shift: keep the aspect), just past one rotates, inside
// moves. Stamp: Alt-click a source, then paint. Brush paints; Eyedropper
// picks Brush Color. Hand (or Space) pans, Zoom (or holding X) zooms.

#include <QtCore/QLineF>
#include <QtCore/QList>
#include <QtCore/QPointF>
#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtWidgets/QWidget>

#include <functional>

namespace pictura {

class VpCanvas : public QWidget {
    Q_OBJECT

public:
    // In the dialog's tool order.
    enum class Mode { EditPlane, CreatePlane, Marquee, Stamp, Brush, Transform, Eyedropper, Hand, Zoom };

    explicit VpCanvas(QWidget* parent = nullptr);

    // Read the session's document size and preview, and open fitted.
    void reload();
    // Re-read the planes, the selections and the changed preview pixels
    // after a session edit.
    void refreshAll();

    void setMode(Mode mode);
    Mode mode() const { return mode_; }
    void setShowEdges(bool on);
    bool showEdges() const { return showEdges_; }

    double zoom() const { return zoom_; }
    // Zoom about the widget centre, clamped to 6.25%–1600%.
    void setZoom(double zoom);
    void zoomBy(double factor);
    void fitInView();

    QPointF toDocument(const QPointF& widget) const;
    QPointF toWidget(const QPointF& document) const;

    int pendingCorners() const { return static_cast<int>(pending_.size()); }
    void cancelPending();

    // The preview as drawn (document pixels), for tests.
    const QImage& previewImage() const { return view_; }

    static QColor validityColor(int validity);

    std::function<void()> zoomChanged;
    // A gesture changed the planes, the selections or the pixels.
    std::function<void()> planesChanged;
    // The fourth Create Plane click made a plane.
    std::function<void()> planeCreated;
    // The Eyedropper picked a colour.
    std::function<void(const QColor&)> colourPicked;
    // Something to tell the user on the help line.
    std::function<void(const QString&)> notice;

protected:
    void paintEvent(QPaintEvent* event) override;
    void resizeEvent(QResizeEvent* event) override;
    void mousePressEvent(QMouseEvent* event) override;
    void mouseDoubleClickEvent(QMouseEvent* event) override;
    void mouseMoveEvent(QMouseEvent* event) override;
    void mouseReleaseEvent(QMouseEvent* event) override;
    void wheelEvent(QWheelEvent* event) override;
    void keyPressEvent(QKeyEvent* event) override;
    void keyReleaseEvent(QKeyEvent* event) override;

private:
    struct Plane {
        QList<QPointF> corners;
        QList<QPointF> edgeNodes;
        QList<QLineF> grid;
        int validity = 0;
    };

    Mode effectiveMode() const;
    void updateCursor();
    void zoomAbout(double zoom, const QPointF& anchor);
    void viewChanged();
    void clampOffset();
    void changed();
    // Patch the cached preview where the session says it changed.
    void syncView();
    // Start a node or plane gesture at widget point `pos`; false over
    // nothing.
    bool beginPlaneGesture(const QPointF& pos, Qt::KeyboardModifiers modifiers);
    // Start a Marquee, Transform, Stamp or Brush gesture; false over nothing.
    bool beginToolGesture(Mode mode, const QPointF& doc, Qt::KeyboardModifiers modifiers);
    void addCorner(const QPointF& document);

    QImage view_;
    QList<Plane> planes_;
    QList<QPointF> floatOutline_;
    QList<QPointF> marqueeOutline_;
    QList<QPointF> floatCorners_;
    int active_ = -1;
    int docWidth_ = 0;
    int docHeight_ = 0;
    double zoom_ = 1.0;
    QPointF offset_;
    bool placed_ = false;
    bool showEdges_ = true;

    Mode mode_ = Mode::EditPlane;
    bool spaceHeld_ = false;
    // Holding X: the zoom to return to, or 0.
    double heldZoom_ = 0.0;

    enum class Drag { None, Pan, Plane, Tool };
    Drag drag_ = Drag::None;
    Mode dragMode_ = Mode::EditPlane;
    QPointF pressWidget_;
    QPointF panOffset_;
    QList<QPointF> pending_; // document
    QPointF hover_;
    bool hovering_ = false;
};

} // namespace pictura
