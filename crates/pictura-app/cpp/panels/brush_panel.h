#pragma once

#include <QtGui/QImage>
#include <QtWidgets/QWidget>

#include "tool_options.h"

class QCheckBox;
class QLabel;

namespace pictura {

class NumericField;
class ToolController;
class TipShapeView;

// Window > Brush (F5): the Brush Tip Shape page. A row of standard round tips,
// Size, Flip X / Flip Y, Angle, Roundness, Hardness, and Spacing, over the
// controller's brush fields that every painting tool shares, with the engine's
// stroke preview underneath. The brush dynamics are listed but not wired.
// ponytail: round tips only (no sampled, bristle, or erodible tips), no Brush
// Presets panel, and Spacing is always on (no velocity spacing).
class BrushPanel : public QWidget {
    Q_OBJECT

public:
    explicit BrushPanel(QWidget* parent = nullptr);

    void setController(ToolController* controller);
    // Re-read the controller's tip into the fields and redraw the preview.
    void refresh();

    // Self-test hook: the preview's current pixels.
    QImage previewForTest() const;

private:
    ToolController* controller_ = nullptr;
    PaintOptions* paint_ = nullptr;
    NumericField* size_ = nullptr;
    NumericField* angle_ = nullptr;
    NumericField* roundness_ = nullptr;
    NumericField* hardness_ = nullptr;
    NumericField* spacing_ = nullptr;
    QCheckBox* flipX_ = nullptr;
    QCheckBox* flipY_ = nullptr;
    TipShapeView* shape_ = nullptr;
    QLabel* preview_ = nullptr;
};

} // namespace pictura
