#pragma once

#include "adjustment_dialog.h"

class QComboBox;

namespace pictura {

// Image > Adjustments > Black & White in the newer single-column layout: the
// Preset menu, the six colour rows (percent fields over black-to-colour ramp
// sliders), Tint with its colour swatch and the Hue / Saturation rows that
// edit it (enabled with Tint), Preview, and Cancel / OK. Ported from
// photorust's BlackWhiteDialog.
// ponytail: no Auto, preset Load / Save, or on-canvas scrubby targeting.
class BlackWhiteDialog : public AdjustmentDialog {
    Q_OBJECT

public:
    BlackWhiteDialog(PictureView* view, const QByteArray& block, const QRect& visible,
                     QWidget* parent = nullptr);

    QWidget* controlForTest(const QString& key) const override;

private:
    void applyPreset(int index);
    void markCustom();
    void tintEdited();
    void setTint(const QColor& color);
    void paintTint(const QColor& color);
    // Point the Hue / Saturation rows at `color` without editing the block.
    void showTint(const QColor& color);

    QComboBox* preset_ = nullptr;
    ColorSwatchWidget* color_ = nullptr;
    QSpinBox* hue_ = nullptr;
    QSpinBox* saturation_ = nullptr;
    bool showingTint_ = false;
};

} // namespace pictura
