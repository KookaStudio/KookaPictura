#pragma once

// The Liquify dialog's preview: the active layer through the dialog's mesh on
// a checkerboard, over CS6's grey ground. The view is rendered by the Rust
// session (`filter_tools/liquify.rs`) at the screen's resolution, so a
// zoomed-in preview is smooth and matches the commit; a brush drag edits the
// mesh and re-renders only the rectangle it changed. View Options can hide
// the picture, draw the mesh through the distortion, and show a backdrop of
// other layers in front of, behind, or blended with it. Hand drags (or Space)
// pan, Zoom clicks (Alt: out) zoom, and a right click with Zoom offers CS6's
// zoom levels.

#include <QtCore/QList>
#include <QtCore/QPointF>
#include <QtCore/QTimer>
#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtGui/QPainterPath>
#include <QtWidgets/QWidget>

#include <functional>

namespace pictura {

class LiquifyCanvas : public QWidget {
    Q_OBJECT

public:
    enum class Mode { Brush, Hand, Zoom };
    enum class BackdropMode { InFront, Behind, Blend };

    explicit LiquifyCanvas(QWidget* parent = nullptr);

    // Re-read the session's layer size and open at 100%, centred.
    void reload();
    // Re-render the whole view after a whole-mesh edit.
    void refreshAll();

    // `actsWhileHeld`: the tool keeps working at Brush Rate while the brush
    // is held still.
    void setMode(Mode mode, bool actsWhileHeld);
    // Brush diameter in layer pixels, for the cursor ring.
    void setBrushSize(double size);
    void setStylusPressure(bool on);

    // View Options. `spacing` is the mesh's cell in layer pixels.
    void setShowImage(bool show);
    void setMesh(bool show, double spacing, const QColor& color);
    // A null `image` hides the backdrop; it is in the layer's frame.
    void setBackdrop(const QImage& image, BackdropMode mode, double opacity);
    bool meshShown() const { return showMesh_; }
    // The traced mesh in widget coordinates, for tests.
    QPainterPath meshPathForTest();

    // Screen pixels per layer pixel.
    double zoom() const { return zoom_; }
    // Zoom about the widget centre, clamped to CS6's 6.25%–1600%.
    void setZoom(double zoom);
    void zoomBy(double factor);
    // The whole layer in the frame, never past 100%.
    void fitInView();
    // The whole layer filling the frame, magnifying a small one.
    void fitOnScreen();
    // CS6's Zoom tool menu, owned by `owner`: preset levels, Actual Pixels,
    // Fit in View, Fit on Screen (separators as null-text actions).
    QList<QAction*> zoomMenuActions(QObject* owner);
    void showZoomMenu(const QPoint& globalPos);

    QPointF toLayer(const QPointF& pos) const;
    QPointF toWidget(const QPointF& layer) const;

    // Asked before each stroke with whether Alt is held, to set the brush.
    std::function<void(bool reverse)> beforeStroke;
    std::function<void()> zoomChanged;
    // The first stylus event, so the Stylus Pressure option can enable.
    std::function<void()> tabletSeen;

    // Test hooks: a stroke through `layerPoints`, and `ticks` Brush Rate ticks
    // at its last point.
    void strokeForTest(const QList<QPointF>& layerPoints, bool reverse = false, int ticks = 0);
    const QImage& previewImage() const { return view_; }

protected:
    void paintEvent(QPaintEvent* event) override;
    void resizeEvent(QResizeEvent* event) override;
    void mousePressEvent(QMouseEvent* event) override;
    void mouseMoveEvent(QMouseEvent* event) override;
    void mouseReleaseEvent(QMouseEvent* event) override;
    void wheelEvent(QWheelEvent* event) override;
    void tabletEvent(QTabletEvent* event) override;
    void keyPressEvent(QKeyEvent* event) override;
    void keyReleaseEvent(QKeyEvent* event) override;
    void leaveEvent(QEvent* event) override;

private:
    Mode effectiveMode() const;
    void updateCursor();
    void zoomAbout(double zoom, const QPointF& anchor);
    void fit(bool capAtActualPixels);
    void viewChanged();
    void clampOffset();
    double pressure() const;
    void beginStroke(const QPointF& layer, bool reverse, bool straightLine);
    void strokeTo(const QPointF& layer);
    void hold();
    // Re-render the part of the view over layer rectangle `rect`
    // ([x, y, w, h]).
    void applyPatch(const QList<int>& rect);

    void rebuildMesh();

    QImage view_;
    bool showImage_ = true;
    bool showMesh_ = false;
    double meshSpacing_ = 32.0;
    QColor meshColor_ = QColor(128, 128, 128);
    // The mesh in layer coordinates, rebuilt when the mesh or view changes.
    QPainterPath meshPath_;
    bool meshDirty_ = true;
    QImage backdrop_;
    BackdropMode backdropMode_ = BackdropMode::InFront;
    double backdropOpacity_ = 0.5;
    int layerWidth_ = 0;
    int layerHeight_ = 0;
    double zoom_ = 1.0;
    QPointF offset_;
    bool placed_ = false;

    Mode mode_ = Mode::Brush;
    bool actsWhileHeld_ = false;
    double brushSize_ = 100.0;
    bool stylus_ = false;
    double tabletPressure_ = 1.0;
    bool tabletDown_ = false;
    bool spaceHeld_ = false;

    bool stroking_ = false;
    bool panning_ = false;
    QPointF panAnchor_;
    QPointF panOffset_;
    QPointF lastLayer_;
    bool hasLastClick_ = false;
    QPointF hover_;
    bool hovering_ = false;
    QTimer holdTimer_;
};

} // namespace pictura
