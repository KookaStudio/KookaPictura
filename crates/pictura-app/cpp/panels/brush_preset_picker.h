#pragma once

#include <QtCore/QPoint>
#include <QtGui/QPixmap>
#include <QtWidgets/QWidget>

#include "tool_options.h"

class QLabel;
class QListWidget;
class QSlider;
class QSpinBox;

namespace pictura {

class ToolController;
struct PaintTip;

// The Size slider's travel: the first half runs 1-100 px evenly, where most
// painting happens; the second climbs geometrically from 100 to 5000 px.
namespace brush_size_scale {
constexpr int kSteps = 1000;
int sizeAt(int position);
int positionOf(int size);
} // namespace brush_size_scale

// CS6's Brush Preset picker, the popup behind the options bar's brush tip
// button: a preview of the tip, Size and Hardness (a number and a slider each),
// the current preset's name, and the default brush set. A preset is a whole
// tip — size, hardness, roundness, angle, spacing, and the dynamics (scatter,
// count, jitters) — written into the controller every paint tool reads.
// Thumbnails are painted by the engine (`brush_dab_preview`), so they cannot
// drift from what the brush lays down.
// ponytail: no cog menu (New Brush Preset, libraries), no Use Sample Size, and
// the sampled-image tips (chalk, charcoal, spatter, grass) are approximated
// with scatter and jitter, as photorust does.
class BrushPresetPicker : public QWidget {
    Q_OBJECT

public:
    explicit BrushPresetPicker(ToolController* controller, QWidget* parent = nullptr);

    // Re-read the controller and show below `anchor`, kept on screen.
    void popUpUnder(QWidget* anchor);

    // Re-read the controller and show at `globalPos`, kept on screen (the
    // canvas right-click path).
    void popUpAt(const QPoint& globalPos);

    // The controller's current tip at `edge` px square, for the bar's button.
    static QPixmap tipIcon(const PaintTip& tip, int edge);

    int presetCount() const;
    // Self-test hook: pick preset `index` as a click on the grid would.
    void choosePresetForTest(int index);

protected:
    bool eventFilter(QObject* watched, QEvent* event) override;

private:
    void sync();
    void applyPreset(int index);
    void buildGrid();
    // Place the overlay with its top-left at `globalTopLeft`, clamped to the
    // parent rect (the overlay is an in-window child, not a top-level window).
    void moveWithinParent(const QPoint& globalTopLeft);

    ToolController* controller_ = nullptr;
    PaintOptions* paint_ = nullptr;
    bool updating_ = false;
    QLabel* preview_ = nullptr;
    QSpinBox* size_ = nullptr;
    QSlider* sizeSlider_ = nullptr;
    QSpinBox* hardness_ = nullptr;
    QSlider* hardnessSlider_ = nullptr;
    QLabel* current_ = nullptr;
    QListWidget* grid_ = nullptr;
};

} // namespace pictura
