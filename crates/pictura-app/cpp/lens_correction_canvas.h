#pragma once

// The Lens Correction dialog's preview: the active layer through the current
// correction (or, with Preview off, as it is) on a checkerboard over CS6's
// grey ground, rendered by the Rust session
// (`filter_tools/lens_correction.rs`) at the screen's resolution, with the
// alignment grid over it. Display coordinates are layer pixels; the
// correction keeps the layer's size.
//
// Remove Distortion drags toward the centre (barrel) or away from it
// (pincushion); Straighten drags a line the image is then levelled to; Move
// Grid drags the grid. Hand drags (or Space) pan, and Zoom clicks (Alt: out)
// zoom.

#include <QtCore/QPointF>
#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtWidgets/QWidget>

#include <functional>

namespace pictura {

class LensCorrectionCanvas : public QWidget {
    Q_OBJECT

public:
    enum class Mode { RemoveDistortion, Straighten, MoveGrid, Hand, Zoom };

    explicit LensCorrectionCanvas(QWidget* parent = nullptr);

    // Re-read the session's layer size and open at 100%, centred.
    void reload();
    // Re-render the view after a session edit.
    void refresh();

    void setMode(Mode mode);
    Mode mode() const { return mode_; }
    void setPreview(bool on);
    bool preview() const { return preview_; }

    void setShowGrid(bool on);
    bool gridShown() const { return showGrid_; }
    void setGridSize(int size);
    int gridSize() const { return gridSize_; }
    void setGridColor(const QColor& color);
    QColor gridColor() const { return gridColor_; }
    // Where the grid's lines cross, within one cell of the layer's origin.
    QPointF gridOffset() const { return gridOffset_; }

    // Screen pixels per display pixel.
    double zoom() const { return zoom_; }
    // Zoom about the widget centre, clamped to 6.25%–1600%.
    void setZoom(double zoom);
    void zoomBy(double factor);

    QPointF toDisplay(const QPointF& widget) const;
    QPointF toWidget(const QPointF& display) const;
    const QImage& previewImage() const { return view_; }
    // The grid lines drawn across the layer, `x` then `y` positions.
    QList<double> gridLinesForTest() const;

    std::function<void()> zoomChanged;
    // Remove Distortion: the drag's start value, and the dragged value.
    std::function<double()> distortionValue;
    std::function<void(double)> distortionDragged;
    // Straighten: the line dragged, in display pixels.
    std::function<void(const QPointF&, const QPointF&)> straightened;

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
    Mode effectiveMode() const;
    void updateCursor();
    void zoomAbout(double zoom, const QPointF& anchor);
    void viewChanged();
    void clampOffset();
    double radius(const QPointF& display) const;

    QImage view_;
    bool preview_ = true;
    bool showGrid_ = false;
    int gridSize_ = 64;
    QColor gridColor_ = QColor(128, 128, 128);
    QPointF gridOffset_;
    int layerWidth_ = 0;
    int layerHeight_ = 0;
    double zoom_ = 1.0;
    QPointF offset_;
    bool placed_ = false;

    Mode mode_ = Mode::RemoveDistortion;
    bool spaceHeld_ = false;

    enum class Drag { None, Pan, Distort, Line, Grid };
    Drag drag_ = Drag::None;
    QPointF pressWidget_;
    QPointF panOffset_;
    QPointF lineStart_; // display
    QPointF lineEnd_;   // display
    QPointF gridStart_;
    double distortStart_ = 0.0;
};

} // namespace pictura
