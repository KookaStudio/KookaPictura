#pragma once

// The Liquify dialog's preview: the active layer through the dialog's mesh on
// a checkerboard, over CS6's grey ground. A brush drag edits the mesh in the
// Rust session (`filter_tools/liquify.rs`) and repaints only the preview
// rectangle the session re-rendered; Hand drags (or Space) pan and Zoom clicks
// (Alt: out) zoom.

#include <QtCore/QList>
#include <QtCore/QPointF>
#include <QtCore/QTimer>
#include <QtGui/QImage>
#include <QtWidgets/QWidget>

#include <functional>

namespace pictura {

class LiquifyCanvas : public QWidget {
    Q_OBJECT

public:
    enum class Mode { Brush, Hand, Zoom };

    explicit LiquifyCanvas(QWidget* parent = nullptr);

    // Re-read the session's geometry and whole preview.
    void reload();
    // Re-fetch the whole preview after a whole-mesh edit.
    void refreshAll();

    // `actsWhileHeld`: the tool keeps working at Brush Rate while the brush
    // is held still.
    void setMode(Mode mode, bool actsWhileHeld);
    // Brush diameter in layer pixels, for the cursor ring.
    void setBrushSize(double size);
    void setStylusPressure(bool on);

    // Screen pixels per layer pixel.
    double zoom() const { return zoom_; }
    // Zoom about the widget centre, clamped to CS6's 6.25%–1600%.
    void setZoom(double zoom);
    void zoomBy(double factor);
    void fitInView();

    QPointF toLayer(const QPointF& pos) const;
    QPointF toWidget(const QPointF& layer) const;

    // Asked before each stroke with whether Alt is held, to set the brush.
    std::function<void(bool reverse)> beforeStroke;
    // After any brush edit.
    std::function<void()> edited;
    std::function<void()> zoomChanged;
    // The first stylus event, so the Stylus Pressure option can enable.
    std::function<void()> tabletSeen;

    // Test hooks: a stroke through `layerPoints`, and `ticks` Brush Rate ticks
    // at its last point.
    void strokeForTest(const QList<QPointF>& layerPoints, bool reverse = false, int ticks = 0);
    const QImage& previewImage() const { return preview_; }

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
    void clampOffset();
    double pressure() const;
    void beginStroke(const QPointF& layer, bool reverse, bool straightLine);
    void strokeTo(const QPointF& layer);
    void hold();
    // Paint the session's preview rectangle `rect` ([x, y, w, h]) into the
    // preview image.
    void applyPatch(const QList<int>& rect);

    QImage preview_;
    int layerWidth_ = 0;
    int layerHeight_ = 0;
    int scale_ = 1;
    double zoom_ = 1.0;
    QPointF offset_;
    bool fitted_ = false;

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
