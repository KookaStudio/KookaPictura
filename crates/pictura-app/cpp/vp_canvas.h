#pragma once

// The Vanishing Point dialog's preview: the document composite on CS6's grey
// ground, and over it every plane's outline and grid in its validity colour
// (blue valid, red no aspect ratio, yellow no camera) with the active
// plane's corner and edge nodes. The planes live in the Rust session
// (`filter_tools/vanishing_point.rs`); coordinates here are document pixels.
//
// Edit Plane: click a plane to select it, drag a corner to reshape, an edge
// node to resize in perspective, the inside to move; Ctrl-drag an edge node
// tears off a related plane, Alt-drag a related plane's far edge node turns
// it, and Ctrl-click cycles through overlapping planes. Create Plane: click
// four corners (Backspace drops the last, Esc all), then the dialog returns
// to Edit Plane; nodes still drag. Delete removes the active plane. Hand
// (or Space) pans, Zoom (or holding X) zooms.

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
    enum class Mode { EditPlane, CreatePlane, Hand, Zoom };

    explicit VpCanvas(QWidget* parent = nullptr);

    // Take the document composite and size, and open fitted to the view.
    void reload(const QImage& backdrop);
    // Re-read the planes after a session edit.
    void refreshPlanes();

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

    static QColor validityColor(int validity);

    std::function<void()> zoomChanged;
    // A gesture changed the planes or the active plane.
    std::function<void()> planesChanged;
    // The fourth Create Plane click made a plane.
    std::function<void()> planeCreated;

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
    // Start a node or plane gesture at widget point `pos`; false over
    // nothing.
    bool beginPlaneGesture(const QPointF& pos, Qt::KeyboardModifiers modifiers);
    void addCorner(const QPointF& document);

    QImage backdrop_;
    QList<Plane> planes_;
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

    enum class Drag { None, Pan, Plane };
    Drag drag_ = Drag::None;
    QPointF pressWidget_;
    QPointF panOffset_;
    QList<QPointF> pending_; // document
    QPointF hover_;
    bool hovering_ = false;
};

} // namespace pictura
