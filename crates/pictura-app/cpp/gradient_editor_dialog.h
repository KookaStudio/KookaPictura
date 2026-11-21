#pragma once

#include <QtCore/QPair>
#include <QtCore/QString>
#include <QtCore/QVector>
#include <QtGui/QColor>
#include <QtWidgets/QDialog>
#include <QtWidgets/QWidget>

#include "gradient_noise_page.h"

class QComboBox;
class QLineEdit;
class QPainter;
class QPushButton;
class QSpinBox;
class QStackedWidget;
class QToolButton;

namespace pictura {

class ColorSwatchWidget;
struct PresetRow;

// One colour stop; `location` in the PSD's 0..4096. `midpoint` is the percent
// of the segment to the next stop at which the blend is half way.
struct GradientColorStop {
    int location;
    QColor color;
    int midpoint = 50;
};
using GradientStops = QVector<GradientColorStop>;

// One opacity stop; `location` in 0..4096, `opacity` a percent, `midpoint` the
// percent of the segment to the next stop at which the blend is half way.
struct GradientOpacityStop {
    int location;
    int opacity = 100;
    int midpoint = 50;
};
using GradientOpacityStops = QVector<GradientOpacityStop>;

// A Gradient Editor gradient: Solid stops (colour and opacity) at a smoothness,
// or Noise.
struct GradientSpec {
    enum Type { Solid, Noise };
    QString name;
    int type = Solid;
    GradientStops stops;
    GradientOpacityStops opacityStops;
    int smoothness = 100;
    NoiseSettings noise;
};

// The bridge's `"location:RRGGBB[AA] …"` text and back (empty on a bad list).
QString formatGradientStops(const GradientStops& stops);
GradientStops parseGradientStops(const QString& text);
// The Gradient tool's stored spec as JSON and back (a default spec on bad
// text). Midpoints and opacity stops are preserved.
QString formatGradientSpec(const GradientSpec& spec);
GradientSpec parseGradientSpec(const QString& text);
// The bridge's `"at:RRGGBB[AA][:mid] …|at:opacity[:mid] …"` ramp text.
QString formatGradientSpecStops(const GradientSpec& spec);
// Interpolation helpers shared by the ramp editor and the baked solid ramp.
QColor gradientColorAt(const GradientStops& stops, int location);
int gradientOpacityAt(const GradientOpacityStops& stops, int location);
GradientOpacityStops gradientOpacityFromColors(const GradientStops& stops);
// The solid ramp with midpoints and opacity stops applied, without Smoothness.
GradientStops bakedSolidStops(const GradientSpec& spec);
// `stops` across `rect`, left to right, over a checkerboard where they are
// part transparent.
void paintGradient(QPainter& painter, const QRect& rect, const GradientStops& stops);
// Built-in gradient `index` between `foreground` and `background`; empty out
// of range.
GradientStops gradientPreset(int index, const QColor& foreground, const QColor& background);
GradientStops libraryRamp(const PresetRow& row, const QColor& foreground, const QColor& background);
GradientStops noiseStops(const NoiseSettings& noise);
// The ramp `spec` draws and maps: Solid stops with the smoothness baked in,
// or the Noise gradient.
GradientStops resolvedStops(const GradientSpec& spec);

// A plain strip showing a gradient.
class GradientSwatch : public QWidget {
    Q_OBJECT

public:
    explicit GradientSwatch(QWidget* parent = nullptr);
    void setStops(const GradientStops& stops);
    QSize sizeHint() const override;

signals:
    void clicked();

protected:
    void paintEvent(QPaintEvent* event) override;
    void mousePressEvent(QMouseEvent* event) override;

private:
    GradientStops stops_;
};

// The Gradient Editor's ramp: colour stops under the bar, opacity stops above
// it, and a draggable midpoint diamond between each adjacent pair. Click a
// stop to select it and drag it along; drag it well off its side of the bar to
// delete it (two always remain); click an empty part of a row to add a stop
// there; double-click a colour stop to pick its colour.
class GradientStopBar : public QWidget {
    Q_OBJECT

public:
    enum class StopKind { Color, Opacity };

    explicit GradientStopBar(QWidget* parent = nullptr);

    void setStops(const GradientStops& stops);
    // Derive opacity stops from the colour stops' alpha when `opacity` is empty.
    void setGradient(const GradientStops& stops, const GradientOpacityStops& opacity);
    const GradientStops& stops() const { return stops_; }
    const GradientOpacityStops& opacityStops() const { return opacityStops_; }
    int selected() const { return selected_; }
    int selectedOpacity() const { return selectedOpacity_; }
    StopKind selectedKind() const { return kind_; }
    void select(int index);
    void selectOpacity(int index);
    // The stop's colour or location, kept in order; the selection follows it.
    void setStopColor(int index, const QColor& color);
    void setStopLocation(int index, int location);
    void setStopMidpoint(int index, int percent);
    void setOpacityStopValue(int index, int percent);
    void setOpacityStopLocation(int index, int location);
    void setOpacityStopMidpoint(int index, int percent);
    // Keep the colour locations strictly rising: a moved or added stop is
    // clamped to its neighbours. The layer-style call site enables this.
    void setStrictLocations(bool strict) { strict_ = strict; }
    void removeStop(int index);
    void removeOpacityStop(int index);
    // A stop at `location` in the ramp's colour (or opacity) there, selected.
    int addStop(int location);
    int addOpacityStop(int location);

    QSize sizeHint() const override;

signals:
    void stopsChanged();
    void opacityStopsChanged();
    void selectionChanged(int index);
    void opacitySelectionChanged(int index);

protected:
    void paintEvent(QPaintEvent* event) override;
    void mousePressEvent(QMouseEvent* event) override;
    void mouseMoveEvent(QMouseEvent* event) override;
    void mouseReleaseEvent(QMouseEvent* event) override;
    void mouseDoubleClickEvent(QMouseEvent* event) override;

private:
    enum class DragKind { None, Color, Opacity, ColorMidpoint, OpacityMidpoint };

    QRect barRect() const;
    int colorStopX(int index) const;
    int opacityStopX(int index) const;
    int colorStopAt(const QPoint& pos) const;
    int opacityStopAt(const QPoint& pos) const;
    // The midpoint handle under `pos`, as (kind, left stop index), or (-1,-1).
    QPair<StopKind, int> midpointAt(const QPoint& pos) const;
    int locationAt(int x) const;
    void sortKeepingSelection();
    void sortKeepingOpacitySelection();
    void deriveOpacityStops();

    GradientStops stops_;
    GradientOpacityStops opacityStops_;
    int selected_ = -1;
    int selectedOpacity_ = -1;
    StopKind kind_ = StopKind::Color;
    DragKind dragging_ = DragKind::None;
    bool dragOff_ = false;
    bool strict_ = false;
};

// CS6's Gradient Editor: the Presets grid, Name, Gradient Type, and either the
// Solid page (Smoothness, the ramp with colour and opacity stops and midpoints,
// and the Stops group) or the Noise page, with OK / Cancel down the right.
// Every edit emits `gradientChanged`, so the caller can preview it. Ported from
// photorust's GradientEditorDialog.
// ponytail: Load… / Save… and New stay disabled, so an edited gradient is not
// kept as a preset.
class GradientEditorDialog : public QDialog {
    Q_OBJECT

public:
    GradientEditorDialog(const GradientSpec& spec, const QColor& foreground,
                         const QColor& background, QWidget* parent = nullptr);

    GradientSpec spec() const;
    GradientStopBar* bar() const { return bar_; }
    GradientNoisePage* noisePage() const { return noise_; }
    QComboBox* typeCombo() const { return type_; }
    PercentField* smoothnessField() const { return smoothness_; }
    // Load preset `index`, as clicking its swatch does.
    void choosePreset(int index);
    // Hide the Noise type and Smoothness control; the layer-style call site
    // writes only a solid gradient.
    void setSolidOnly(bool solidOnly);

signals:
    void gradientChanged();

private:
    void showSelection();
    void edited();

    GradientStopBar* bar_ = nullptr;
    GradientNoisePage* noise_ = nullptr;
    QStackedWidget* pages_ = nullptr;
    QComboBox* type_ = nullptr;
    PercentField* smoothness_ = nullptr;
    QLineEdit* name_ = nullptr;
    ColorSwatchWidget* color_ = nullptr;
    QSpinBox* location_ = nullptr;
    QPushButton* delete_ = nullptr;
    QSpinBox* opacity_ = nullptr;
    QSpinBox* opacityLocation_ = nullptr;
    QPushButton* opacityDelete_ = nullptr;
    QSpinBox* midpoint_ = nullptr;
    QColor foreground_;
    QColor background_;
    bool loading_ = false;
};

} // namespace pictura
