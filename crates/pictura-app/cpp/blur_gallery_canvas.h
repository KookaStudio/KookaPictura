#pragma once

// The Blur Gallery's on-image pin surface: the proxy picture with one marker
// per pin. The dialog owns the pin list; this widget edits it — a press on a
// pin moves it or, on its blur ring, changes its amount, and a press on the
// picture adds a pin. The dialog handles `H` (hide the markers) and Delete.

#include <QtCore/QList>
#include <QtCore/QPointF>
#include <QtCore/QSize>
#include <QtGui/QImage>
#include <QtWidgets/QWidget>

namespace pictura {

// One gallery pin: a position as a fraction of the picture (outside allowed,
// so a pin can anchor a border) and a blur amount in document pixels.
struct BlurGalleryPin {
    QPointF pos{0.5, 0.5};
    double blur = 0.0;
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
    void dragTo(const QPointF& widget);

    QList<BlurGalleryPin>* pins_ = nullptr;
    QImage image_;
    QSize documentSize_;
    int selected_ = 0;
    bool uiVisible_ = true;
    enum class Drag { None, Move, Blur };
    Drag drag_ = Drag::None;
    QPointF dragStart_;
    double dragStartBlur_ = 0.0;
};

} // namespace pictura
