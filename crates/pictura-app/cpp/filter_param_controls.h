#pragma once

// One row of controls per filter parameter — slider + value box, checkbox,
// menu, colour swatch, placement pad — shared by the single-filter dialog and
// the Filter Gallery. Split out of filter_preview_dialog.cpp.

#include <QtCore/QList>
#include <QtCore/QObject>
#include <QtGui/QImage>

#include "filter_commands.h"

class QCheckBox;
class QComboBox;
class QDoubleSpinBox;
class QListWidget;
class QButtonGroup;
class QPushButton;
class QSlider;
class QSpinBox;
class QWidget;

namespace pictura {

class ColorSwatchWidget;

class FilterParamControls : public QObject {
    Q_OBJECT

public:
    // Builds one row widget per entry of `params`, parented to `parent`, each
    // on its spec's default. `padImage` backs a Placement pad (Lens Flare).
    FilterParamControls(const QList<FilterParamSpec>& params, QWidget* parent,
                        const QImage& padImage = {});

    // The row widgets, in parameter order, for the caller to lay out.
    QList<QWidget*> rows() const;

    // The parameter slot values in mapping order.
    QList<double> values() const;

    // values() with every pixel-sized slider (a "pixels" or "px" unit) scaled
    // by `scale`, held to its range and rounded on an integer slider, for a
    // preview on a copy reduced by `scale`.
    QList<double> scaledValues(double scale) const;

    // Set the controls from slot values; slots past the end keep their value.
    // Silent: emits no changed().
    void setValues(const QList<double>& values);

signals:
    // A value changed. A slider drag holds this until release.
    void changed();
    // A value changed mid-drag, before `changed`. Cheap consumers (the dialog's
    // own preview pane) update; the canvas waits for the release.
    void changedLive();

private:
    struct Control {
        FilterParamSpec spec;
        QWidget* row = nullptr;
        QDoubleSpinBox* spin = nullptr;
        QSlider* slider = nullptr;
        QComboBox* combo = nullptr;
        QCheckBox* box = nullptr;
        ColorSwatchWidget* colorButton = nullptr;
        QDoubleSpinBox* x = nullptr;
        QDoubleSpinBox* y = nullptr;
        QWidget* pad = nullptr;    // Placement crosshair pad
        QWidget* center = nullptr; // Radial Blur centre pad
        QWidget* shear = nullptr;  // Shear curve box
        QButtonGroup* group = nullptr; // Radio group
        double hidden = 0.0;           // a Hidden slot's value
        QList<QSpinBox*> cells;        // Matrix cells, row by row
        QListWidget* shapes = nullptr; // Shape grid
    };

    void addControl(const FilterParamSpec& spec);
    double controlValue(const Control& control, int slot) const;

    QWidget* parent_ = nullptr;
    QImage padImage_;
    QList<Control> controls_;
    bool sliderDragging_ = false;
};

} // namespace pictura
