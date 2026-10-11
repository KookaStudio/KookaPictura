#pragma once

#include <QtCore/QPointF>
#include <QtCore/QPointer>
#include <QtWidgets/QWidget>

#include <functional>
#include <optional>

class QContextMenuEvent;
class QMouseEvent;
class QPaintEvent;

namespace pictura {

class ImageView;

// The ruler units CS6's Units & Rulers pane and ruler context menu offer, in
// menu order. Points are PostScript (72 per inch) unless Traditional (72.27).
enum class RulerUnit { Pixels, Inches, Centimeters, Millimeters, Points, Picas, Percent };
inline constexpr int kRulerUnitCount = 7;
QString rulerUnitName(RulerUnit unit);

// One of the canvas's two rulers: the horizontal one sits above the canvas,
// the vertical one left of it. Both measure from the document's top-left
// corner in the ruler unit (inches by default, as in CS6), label distances
// from 0 without a sign, and follow the canvas pan and zoom (in its unrotated
// view frame). A right-click picks the unit and a double-click opens Units &
// Rulers; a left drag out of a ruler places a guide: the top ruler a
// horizontal one, the left ruler a vertical one.
class CanvasRuler : public QWidget {
    Q_OBJECT

public:
    static constexpr int kThickness = 16;

    // Labelled major step in ruler units (1, 2, or 5 × 10ⁿ, at least 50 screen
    // pixels apart; whole pixels for the pixel unit) and the minor ticks it is
    // divided into: eighths for a 1-inch step, else tenths, quarters, or fifths,
    // halved while they would be closer than 4 screen pixels.
    struct Scale {
        double major = 1.0;
        int subdivisions = 1;
    };
    static Scale scaleFor(RulerUnit unit, double screenPxPerUnit);

    CanvasRuler(Qt::Orientation orientation, QWidget* parent = nullptr);

    void setView(ImageView* view);
    void setUnit(RulerUnit unit);
    RulerUnit unit() const { return unit_; }
    void setTraditionalPoints(bool on);
    // The document resolution in pixels per inch, read at paint time.
    void setPpiProvider(std::function<double()> ppi);
    // Document pixels per ruler unit along this ruler.
    double pixelsPerUnit() const;
    // The cursor's document position, marked on the ruler.
    void setCursorPosition(const QPointF& imagePos);
    Qt::Orientation orientation() const { return orientation_; }

signals:
    // A guide drag was released over the canvas at `position` document pixels.
    void guideDropped(bool vertical, double position);
    // A unit was picked from the ruler's context menu.
    void unitChosen(pictura::RulerUnit unit);
    // A double-click asks for Edit > Preferences > Units & Rulers.
    void preferencesRequested();

protected:
    void paintEvent(QPaintEvent* event) override;
    void mousePressEvent(QMouseEvent* event) override;
    void mouseMoveEvent(QMouseEvent* event) override;
    void mouseReleaseEvent(QMouseEvent* event) override;
    void contextMenuEvent(QContextMenuEvent* event) override;
    void mouseDoubleClickEvent(QMouseEvent* event) override;

private:
    // The whole document pixel a guide dragged to `globalPos` lands on, or
    // nothing when the point is off the canvas.
    std::optional<double> guidePositionAt(const QPointF& globalPos) const;
    // The ruler coordinate of document position `position` along the ruler.
    double toRuler(double position) const;

    Qt::Orientation orientation_;
    RulerUnit unit_ = RulerUnit::Inches;
    bool traditionalPoints_ = false;
    std::function<double()> ppi_;
    QPointer<ImageView> view_;
    QPointF cursor_;
    bool hasCursor_ = false;
    bool dragging_ = false;
};

} // namespace pictura
