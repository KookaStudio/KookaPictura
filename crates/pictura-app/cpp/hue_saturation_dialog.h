#pragma once

#include "adjustment_dialog.h"

class QComboBox;

namespace pictura {

class SpectrumBar;

// Image > Adjustments > Hue/Saturation in CS6's layout: a Preset menu, the
// colour-range menu (Master, Reds … Magentas), Hue / Saturation / Lightness
// rows of label and number field over a slider, Colorize and Preview, the two
// spectrum bars (the lower one shifted by Hue), and OK / Cancel down the
// right. Each range edits its own record. Ported from photorust's
// HueSaturationDialog.
// ponytail: Colorize is shown disabled. `hue2` byte 2 stays 1 for a normal
// layer (psd-tools' `enable`, as Kooka has always written it; Adobe's spec
// reads it as the Colorize flag), which leaves Colorize no flag to store in.
// Photoshop's Cyanotype and Sepia presets need Colorize and are left out; no
// range-band sliders or eyedroppers.
class HueSaturationDialog : public AdjustmentDialog {
    Q_OBJECT

public:
    HueSaturationDialog(PictureView* view, const QByteArray& block, const QRect& visible,
                        QWidget* parent = nullptr);

    QWidget* controlForTest(const QString& key) const override;
    void reloadFromBlock() override;

protected:
    QString paramKey(const QString& key) const override;

private:
    void loadRange();
    void applyPreset(int index);
    void markCustom();

    QComboBox* preset_ = nullptr;
    QComboBox* range_ = nullptr;
    SpectrumBar* shifted_ = nullptr;
    bool loading_ = false;
};

} // namespace pictura
