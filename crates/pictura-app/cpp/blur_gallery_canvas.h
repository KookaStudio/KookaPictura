#pragma once

// The Blur Gallery's on-image pin surface: the proxy picture with one marker
// per pin. The dialog owns the pin list; this widget edits it — a press on a
// pin moves it or, on its blur ring, changes its amount, and a press on the
// picture adds a pin. In Iris mode the selected pin also shows its outline
// with the radius handles (resize and rotate), the inner feather dots, and
// the roundness diamond; in Tilt-Shift mode its band's solid focus lines,
// dashed fade lines, and the rotation dots on the solid lines. In Path mode
// each pin is a path drawn as the curve through its points with an end
// arrow: clicks on empty picture pin a new path's stop points while its last
// point follows the pointer (double-click, Enter, or Esc ends it), a point
// drags, a press on the curve inserts one, and Ctrl-drag moves the path. The dialog handles
// `H` (hide the markers) and Delete.

#include <QtCore/QList>
#include <QtCore/QPointF>
#include <QtCore/QSize>
#include <QtCore/QSizeF>
#include <QtGui/QImage>
#include <QtWidgets/QWidget>

class QPainter;

namespace pictura {

// One gallery pin: a position as a fraction of the picture (outside allowed,
// so a pin can anchor a border) and a blur amount in document pixels. Iris
// Blur adds its outline: semi-axes as fractions of the picture's width, the
// rotation in degrees, squareness 0..100, and the sharp core as a fraction of
// the outline (`pictura_filters::IrisPin`). Tilt-Shift shares the angle and
// adds its band: the focus and fade extents above and below the centre line
// as fractions of the picture's height, and the distortion
// (`pictura_filters::TiltPin`). Path Blur uses only `points` (fractions of
// the picture, at least two) and the start and end `speeds` in document
// pixels (`pictura_filters::BlurPath`).
struct BlurGalleryPin {
    QPointF pos{0.5, 0.5};
    double blur = 0.0;
    QSizeF radii{0.35, 0.35};
    double angle = 0.0;
    double squareness = 50.0;
    double feather = 0.65;
    double focus[2] = {0.1, 0.1};
    double fade[2] = {0.15, 0.15};
    double distortion = 0.0;
    bool symmetric = false;
    QList<QPointF> points;
    double speeds[2] = {0.0, 0.0};
};

// The gallery effect the workspace edits.
enum class BlurGalleryMode { Field, Iris, TiltShift, Path };

class BlurGalleryCanvas : public QWidget {
    Q_OBJECT

public:
    explicit BlurGalleryCanvas(QList<BlurGalleryPin>* pins, QWidget* parent = nullptr);

    void setImage(const QImage& image);
    const QImage& image() const { return image_; }
    // The document's size, so a blur-ring drag moves the amount by document
    // pixels however the proxy is fitted to the widget.
    void setDocumentSize(const QSize& size) { documentSize_ = size; }
    void setMode(BlurGalleryMode mode);
    // Edit another effect's pin list (owned by the dialog), selecting
    // `selected`.
    void setPins(QList<BlurGalleryPin>* pins, int selected);
    BlurGalleryMode mode() const { return mode_; }
    void setUiVisible(bool on);
    bool uiVisible() const { return uiVisible_; }

    int selected() const { return selected_; }
    // Path mode: the selected path's selected point, or -1 for none.
    int selectedPoint() const { return selectedPoint_; }
    void setSelectedPoint(int point);
    // A pin, handle, or line drag, or a path being placed, is in progress;
    // the dialog previews a draft until `dragFinished`.
    bool dragging() const { return drag_ != Drag::None || building_; }
    // Path mode: a new path's stops are being placed.
    bool building() const { return building_; }
    // End the path being placed, dropping the point that follows the pointer.
    void finishPath();
    void setSelected(int index);

signals:
    // A pin moved, was added, or changed its amount; the dialog re-renders.
    void pinsEdited();
    // A pin (or, in Path mode, a path or one of its points) was selected.
    void pinSelected(int index);
    void pinAdded(int index);
    // The pointer released a drag; the dialog renders the full preview.
    void dragFinished();
    // Delete or Backspace on the canvas; the dialog removes the pin.
    void deleteRequested();

protected:
    void paintEvent(QPaintEvent* event) override;
    void mousePressEvent(QMouseEvent* event) override;
    void mouseMoveEvent(QMouseEvent* event) override;
    void mouseReleaseEvent(QMouseEvent* event) override;
    void mouseDoubleClickEvent(QMouseEvent* event) override;
    void keyPressEvent(QKeyEvent* event) override;
    QSize sizeHint() const override;

private:
    QRectF imageRect() const;
    QPointF toWidget(const QPointF& fraction) const;
    QPointF toFraction(const QPointF& widget) const;
    // The pin under `widget` and whether the press grabbed its blur ring.
    int hitPin(const QPointF& widget, bool* onRing) const;
    // Arms an Iris handle drag on the selected pin; false when none is hit.
    bool grabIrisHandle(const QPointF& widget);
    void dragIris(const QPointF& widget, BlurGalleryPin& pin) const;
    void paintIris(QPainter& painter, const BlurGalleryPin& pin) const;
    enum class Drag {
        None,
        Move,
        Blur,
        Radius,
        Roundness,
        Feather,
        Focus,
        Fade,
        Rotate,
        Point,
        Path
    };
    // The Tilt-Shift line or rotation dot of the selected pin under `widget`
    // (`*sign` -1 above the band, +1 below), or `None`.
    Drag hitTilt(const QPointF& widget, double* sign) const;
    // Arms a Tilt-Shift line or rotation-dot drag on the selected pin.
    bool grabTiltHandle(const QPointF& widget);
    void updateTiltCursor(Drag hit, const QPointF& widget);
    void dragTilt(const QPointF& widget, BlurGalleryPin& pin) const;
    void paintTilt(QPainter& painter, const BlurGalleryPin& pin) const;
    void paintTiltDots(QPainter& painter, const BlurGalleryPin& pin) const;
    void dragTo(const QPointF& widget);
    // Path mode (blur_gallery_canvas_path.cpp).
    void pressPath(const QPointF& widget, Qt::KeyboardModifiers modifiers);
    void dragPath(const QPointF& widget);
    void addStop(const QPointF& widget);
    void moveFloatingStop(const QPointF& widget);
    void updatePathCursor(const QPointF& widget);
    // The path and point under `widget`, or -1.
    int hitPoint(const QPointF& widget, int* point) const;
    // The path whose curve passes under `widget` and the span (from point
    // `*span` to the next) and curve position there, or -1.
    int hitCurve(const QPointF& widget, int* span, QPointF* at) const;
    void paintPaths(QPainter& painter) const;

    QList<BlurGalleryPin>* pins_ = nullptr;
    QImage image_;
    QSize documentSize_;
    int selected_ = 0;
    int selectedPoint_ = -1;
    bool building_ = false;
    bool uiVisible_ = true;
    BlurGalleryMode mode_ = BlurGalleryMode::Field;
    Drag drag_ = Drag::None;
    // An Iris radius or feather drag: the outline axis (0 along the angle, 1
    // across it) and which end of it was grabbed. A Tilt-Shift drag uses the
    // sign alone: -1 above the band, +1 below it.
    int dragAxis_ = 0;
    double dragSign_ = 1.0;
    QPointF dragStart_;
    double dragStartBlur_ = 0.0;
    // A whole-path drag: the points as the press found them.
    QList<QPointF> dragPoints_;
};

} // namespace pictura
