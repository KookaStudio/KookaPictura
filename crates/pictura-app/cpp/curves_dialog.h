#pragma once

#include "adjustment_dialog.h"

class QComboBox;
class QSpinBox;

namespace pictura {

class CurveWidget;

// Image > Adjustments > Curves: a Channel menu, the curve over the channel's
// histogram between its output and input ramps, Input / Output fields for the
// selected point, a usage hint, Preview, and Auto / Cancel / OK along the
// bottom.
// Each channel (RGB, Red, Green, Blue) edits its own curve. Ported from
// photorust's CurvesDialog, laid out after the newer single-column Curves page.
// ponytail: no eyedroppers or Show options; the histogram
// is of the canvas composite, not the active layer alone.
class CurvesDialog : public AdjustmentDialog {
    Q_OBJECT

public:
    CurvesDialog(PictureView* view, const QByteArray& block, const QRect& visible,
                 QWidget* parent = nullptr);

    QWidget* controlForTest(const QString& key) const override;
    void reloadFromBlock() override { loadChannel(); }

private:
    void loadChannel();
    // Mirror the selected point into the Input / Output fields.
    void showSelected();
    void applyAuto() override;

    QComboBox* channel_ = nullptr;
    CurveWidget* curve_ = nullptr;
    QSpinBox* input_ = nullptr;
    QSpinBox* output_ = nullptr;
};

} // namespace pictura
