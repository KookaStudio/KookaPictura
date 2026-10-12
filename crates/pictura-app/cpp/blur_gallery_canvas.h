#pragma once

// The Blur Gallery's on-image pin surface: the proxy picture with one marker
// per pin. The dialog owns the pin list; this widget edits it — a press on a
// pin moves it or, on its blur ring, changes its amount, and a press on the
// picture adds a pin. In Iris mode the selected pin also shows its outline
// with the radius handles (resize and rotate), the inner feather dots, and
// the roundness diamond. The dialog handles `H` (hide the markers) and Delete.

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
// the outline (`pictura_filters::IrisPin`).
struct BlurGalleryPin {
    QPointF pos{0.5, 0.5};
    double blur = 0.0;
    QSizeF radii{0.35, 0.35};
    double angle = 0.0;
    double squareness = 50.0;
    double feather = 0.65;
};

class BlurGalleryCanvas : public QWidget {
    Q_OBJECT

public:
    explicit BlurGalleryCanvas(QList<BlurGalleryPin>* pins, QWidget* parent = nullptr);

    void setImage(const QImage& image);
    const QImage& image() const { return image_; }
    // The document's size, so a blur-ring drag moves the amount by document
    // pixels however the proxy is fitted to the widget.
    void setDocumentSize(const QSize& size) { documentSize_ = size; }
    void setIris(bool on);
    bool iris() const { return iris_; }
    void setUiVisible(bool on);
    bool uiVisible() const { return uiVisible_; }

    int selected() const { return selected_; }
    void setSelected(int index);

signals:
    // A pin moved, was added, or changed its amount; the dialog re-renders.
    void pinsEdited();
    void pinSelected(int index);
    void pinAdded(int index);
    // Delete or Backspace on the canvas; the dialog removes the pin.
    void deleteRequested();

protected:
    void paintEvent(QPaintEvent* event) override;
    void mousePressEvent(QMouseEvent* event) override;
    void mouseMoveEvent(QMouseEvent* event) override;
    void mouseReleaseEvent(QMouseEvent* event) override;
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
    void dragTo(const QPointF& widget);

    QList<BlurGalleryPin>* pins_ = nullptr;
    QImage image_;
    QSize documentSize_;
    int selected_ = 0;
    bool uiVisible_ = true;
    bool iris_ = false;
    enum class Drag { None, Move, Blur, Radius, Roundness, Feather };
    Drag drag_ = Drag::None;
    // An Iris radius or feather drag: the outline axis (0 along the angle, 1
    // across it) and which end of it was grabbed.
    int dragAxis_ = 0;
    double dragSign_ = 1.0;
    QPointF dragStart_;
    double dragStartBlur_ = 0.0;
};

} // namespace pictura
