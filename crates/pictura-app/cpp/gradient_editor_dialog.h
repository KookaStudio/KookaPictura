#pragma once

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

// One colour stop; `location` in the PSD's 0..4096. The colour's alpha is
// the gradient's opacity there.
struct GradientColorStop {
    int location;
    QColor color;
};
using GradientStops = QVector<GradientColorStop>;

// A Gradient Editor gradient: Solid stops at a smoothness, or Noise.
struct GradientSpec {
    enum Type { Solid, Noise };
    QString name;
    int type = Solid;
    GradientStops stops;
    int smoothness = 100;
    NoiseSettings noise;
};

// The bridge's `"location:RRGGBB[AA] …"` text and back (empty on a bad list).
QString formatGradientStops(const GradientStops& stops);
GradientStops parseGradientStops(const QString& text);
// `stops` across `rect`, left to right, over a checkerboard where they are
// part transparent.
void paintGradient(QPainter& painter, const QRect& rect, const GradientStops& stops);
// Built-in gradient `index` between `foreground` and `background`; empty out
// of range.
GradientStops gradientPreset(int index, const QColor& foreground, const QColor& background);
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

protected:
    void paintEvent(QPaintEvent* event) override;

private:
    GradientStops stops_;
};

// The Gradient Editor's ramp: the gradient with its colour stops under it.
// Click a stop to select it and drag it along; drag it well below the bar to
// delete it (two always remain); click under the bar to add a stop of the
// colour there; double-click a stop to pick its colour. CS6's opacity stops
// above the bar are drawn at 100% and do not edit.
class GradientStopBar : public QWidget {
    Q_OBJECT

public:
    explicit GradientStopBar(QWidget* parent = nullptr);

    void setStops(const GradientStops& stops);
    const GradientStops& stops() const { return stops_; }
    int selected() const { return selected_; }
    void select(int index);
    // The stop's colour or location, kept in order; the selection follows it.
    void setStopColor(int index, const QColor& color);
    void setStopLocation(int index, int location);
    void removeStop(int index);
    // A stop at `location` in the gradient's colour there, selected.
    int addStop(int location);

    QSize sizeHint() const override;

signals:
    void stopsChanged();
    void selectionChanged(int index);

protected:
    void paintEvent(QPaintEvent* event) override;
    void mousePressEvent(QMouseEvent* event) override;
    void mouseMoveEvent(QMouseEvent* event) override;
    void mouseReleaseEvent(QMouseEvent* event) override;
    void mouseDoubleClickEvent(QMouseEvent* event) override;

private:
    QRect barRect() const;
    int stopX(int index) const;
    int stopAt(const QPoint& pos) const;
    int locationAt(int x) const;
    void sortKeepingSelection();

    GradientStops stops_;
    int selected_ = -1;
    bool dragging_ = false;
    bool dragOff_ = false;
};

// CS6's Gradient Editor: the Presets grid, Name, Gradient Type, and either the
// Solid page (Smoothness, the ramp and its stops, and the Stops group's Color,
// Location, and Delete for the selected colour stop) or the Noise page, with
// OK / Cancel down the right. Every edit emits `gradientChanged`, so the
// caller can preview it. Ported from photorust's GradientEditorDialog.
// ponytail: no colour-stop midpoints; the opacity stop row, Load… / Save…,
// and New stay disabled, so an edited gradient is not kept as a preset.
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

signals:
    void gradientChanged();

private:
    void showSelection(int index);
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
    QColor foreground_;
    QColor background_;
    bool loading_ = false;
};

} // namespace pictura
