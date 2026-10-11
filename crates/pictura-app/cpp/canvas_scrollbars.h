#pragma once

#include <QtWidgets/QWidget>

#include <functional>

class QGridLayout;
class QScrollBar;
class QResizeEvent;

namespace pictura {

class CanvasRuler;
enum class RulerUnit;
class ImageView;

// Thin host around an ImageView adding horizontal/vertical workspace
// scrollbars and the two rulers (View > Rulers). The bars are a pure projection
// of the canvas offset and zoom; ImageView::offset() stays the single source of
// truth and dragging a bar calls ImageView::setOffset, which clamps through the
// shared reveal range.
class CanvasScrollBars : public QWidget {
    Q_OBJECT

public:
    explicit CanvasScrollBars(QWidget* parent = nullptr);

    void setView(ImageView* view);
    ImageView* view() const { return view_; }

    void setRulersVisible(bool on);
    void setRulerUnit(RulerUnit unit, bool traditionalPoints);
    // The document resolution (ppi) the rulers convert with.
    void setPpiProvider(const std::function<double()>& ppi);
    CanvasRuler* horizontalRuler() const { return hruler_; }
    CanvasRuler* verticalRuler() const { return vruler_; }

    QScrollBar* horizontalBarForTest() const { return hbar_; }
    QScrollBar* verticalBarForTest() const { return vbar_; }

signals:
    // Either ruler dropped a guide on the canvas.
    void guideDropped(bool vertical, double position);
    // Either ruler's context menu picked a unit.
    void rulerUnitChosen(pictura::RulerUnit unit);
    // A ruler was double-clicked: open Units & Rulers.
    void rulerPreferencesRequested();

protected:
    void resizeEvent(QResizeEvent* event) override;

private:
    void syncFromView();

    ImageView* view_ = nullptr;
    QScrollBar* hbar_ = nullptr;
    QScrollBar* vbar_ = nullptr;
    QGridLayout* grid_ = nullptr;
    CanvasRuler* hruler_ = nullptr;
    CanvasRuler* vruler_ = nullptr;
    QWidget* rulerCorner_ = nullptr;
    bool syncing_ = false;
};

} // namespace pictura
