#pragma once

#include "adjustment_dialog.h"

class QButtonGroup;
class QLabel;

namespace pictura {

// Image > Adjustments > Color Balance in the newer single-column layout: Tone
// radios (Shadows, Midtones, Highlights), the Cyan · Red, Magenta · Green, and
// Yellow · Blue rows over colour-ramp sliders, Preserve Luminosity, Preview,
// and Cancel / OK. Each tone edits its own triple. A read-only row above the
// bars names the channel changes in the document's mode: R/G/B for RGB, A/B
// for Lab, C/M/Y for CMYK. Ported from photorust's ColorBalanceDialog.
class ColorBalanceDialog : public AdjustmentDialog {
    Q_OBJECT

public:
    ColorBalanceDialog(PictureView* view, const QByteArray& block, const QRect& visible,
                       QWidget* parent = nullptr);

    QWidget* controlForTest(const QString& key) const override;

protected:
    QString paramKey(const QString& key) const override;

private:
    void loadTone();
    void updateReadout();

    QButtonGroup* tone_ = nullptr;
    QLabel* readout_ = nullptr;
    // The document's visible mode as `channels_mode` names it: rgb, lab, cmyk.
    QString mode_;
};

} // namespace pictura
