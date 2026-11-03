#pragma once

#include "adjustment_dialog.h"

class QComboBox;

namespace pictura {

// Image > Adjustments > Exposure in CS6's layout: a Preset menu, then
// Exposure / Offset / Gamma Correction rows of label, slider, number field, and
// eyedropper, with OK / Cancel and Preview down the right. Ported from
// photorust's ExposureDialog.
// ponytail: the black / gray / white point eyedroppers stay disabled; the
// presets are the four whole-stop steps either side of Default.
class ExposureDialog : public AdjustmentDialog {
    Q_OBJECT

public:
    ExposureDialog(PictureView* view, const QByteArray& block, const QRect& visible,
                   QWidget* parent = nullptr);

    QWidget* controlForTest(const QString& key) const override;
    void reloadFromBlock() override;

private:
    void applyPreset(int index);
    void markCustom();

    QComboBox* preset_ = nullptr;
    bool applying_ = false;
};

} // namespace pictura
