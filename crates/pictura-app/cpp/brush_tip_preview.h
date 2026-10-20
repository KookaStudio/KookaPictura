#pragma once

#include <QtCore/QPoint>
#include <QtCore/QSize>
#include <QtWidgets/QWidget>

class QEvent;
class QMouseEvent;
class QPaintEvent;

namespace pictura {

// The 2D pen-tip model Photoshop's Brush Preview shows on the canvas while a
// pencil brush is selected. A child of the document's ImageView, hidden until
// the frame syncs a pen-tip preset. Dragging it never reaches the canvas, so it
// cannot start a stroke.
class BrushTipPreview : public QWidget {
    Q_OBJECT

public:
    static constexpr int kWidth = 150;
    static constexpr int kHeight = 176;
    static constexpr int kMargin = 8;

    explicit BrushTipPreview(QWidget* parent = nullptr);

    QSize sizeHint() const override;

protected:
    void paintEvent(QPaintEvent* event) override;
    void mousePressEvent(QMouseEvent* event) override;
    void mouseMoveEvent(QMouseEvent* event) override;
    void mouseReleaseEvent(QMouseEvent* event) override;
    bool eventFilter(QObject* watched, QEvent* event) override;

private:
    void clampToParent();

    bool dragging_ = false;
    QPoint dragOffset_;
};

} // namespace pictura
