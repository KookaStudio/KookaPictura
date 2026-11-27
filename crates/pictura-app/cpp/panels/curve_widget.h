#pragma once

#include <QtCore/QPointF>
#include <QtCore/QRectF>
#include <QtCore/QString>
#include <QtCore/QVector>
#include <QtWidgets/QWidget>

#include <array>
#include <cstdint>

class QKeyEvent;
class QMouseEvent;
class QPaintEvent;

namespace pictura {

// The Curves editor on a dark face: a histogram, a 4x4 grid, a baseline
// diagonal, and the monotone cubic-spline curve through its control points.
// Clicking adds a point (up to the `curv` block's 14) and selects it; dragging
// moves it; dragging a non-endpoint off the face, Ctrl-clicking it, or Delete
// removes it. Optional ramps down the left and along the bottom show the
// output and input tones. Each change emits `curveChanged`.
class CurveWidget : public QWidget {
    Q_OBJECT

public:
    static constexpr int kMaxPoints = 14;

    explicit CurveWidget(QWidget* parent = nullptr);

    void setBins(const std::array<int, 256>& bins);
    void setShowHistogram(bool v)
    {
        showHisto_ = v;
        update();
    }
    void setShowBaseline(bool v)
    {
        showBaseline_ = v;
        update();
    }
    void setShowRamps(bool v)
    {
        showRamps_ = v;
        update();
    }
    // Draw and LUT the points as straight segments instead of the spline. The
    // contour editor uses this so its curve matches the engine's piecewise
    // linear lookup table.
    void setLinearInterpolation(bool linear);

    void resetCurve();
    void setPoints(const QVector<QPointF>& pts);
    QVector<QPointF> points() const { return points_; }
    // As the `curv` block stores them: whole levels, strictly increasing
    // inputs, `"x,y x,y …"`.
    QString pointsText() const;
    // Load `"x,y x,y …"` without emitting `curveChanged`.
    void setPointsText(const QString& text);

    // The selected point's index, or -1.
    int selected() const { return selected_; }
    // Move the selected point to (`input`, `output`) in 0..255, its input
    // kept strictly between its neighbours'; endpoints keep their input.
    void moveSelected(int input, int output);
    bool selectTone(int input);
    void selectFirstPoint();
    int outputAt(int input) const;
    void setEndpoint(bool white, int input);

    void buildLut(uint8_t lut[256]) const;
    const float (&normalizedSamples() const)[256] { return curve_; }

    QSize sizeHint() const override;

signals:
    void curveChanged();
    void selectionChanged();

protected:
    void paintEvent(QPaintEvent* event) override;
    void mousePressEvent(QMouseEvent* event) override;
    void mouseMoveEvent(QMouseEvent* event) override;
    void mouseReleaseEvent(QMouseEvent* event) override;
    void keyPressEvent(QKeyEvent* event) override;

private:
    void interpolate();
    void select(int index);
    void removeAt(int index);
    // The square the curve is drawn in, right of / above the ramps.
    QRectF plot() const;
    QPointF toWidget(QPointF p) const;
    QPointF fromWidget(QPointF p) const;

    QVector<QPointF> points_;
    float curve_[256]{};
    int dragging_ = -1;
    int selected_ = -1;
    std::array<int, 256> histo_{};
    bool showHisto_ = true;
    bool showBaseline_ = true;
    bool showRamps_ = false;
    bool linear_ = false;
};

} // namespace pictura
