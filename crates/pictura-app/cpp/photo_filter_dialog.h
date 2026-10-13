#pragma once

#include "adjustment_dialog.h"

class QComboBox;
class QRadioButton;
class ColorSwatchWidget;

namespace pictura {

// Image > Adjustments > Photo Filter in CS6's layout: a Use group choosing a
// named Filter or a Color swatch, Density as a percent field over a slider,
// Preserve Luminosity, and OK / Cancel / Preview down the right. A named
// filter is its colour: the block stores only the colour, so a colour
// matching a filter reopens as that filter. While the Color swatch's picker is
// open, a canvas click samples the image as it was before the preview (#413).
// Ported from photorust's
// PhotoFilterDialog (its 20 filter colours).
// ponytail: a version-3 block (colour as CIE XYZ) keeps its colour; the Use
// group is disabled for it.
class PhotoFilterDialog : public AdjustmentDialog {
    Q_OBJECT

public:
    PhotoFilterDialog(PictureView* view, const QByteArray& block, const QRect& visible,
                      QWidget* parent = nullptr);

    QWidget* controlForTest(const QString& key) const override;

private:
    // Write the colour the selected source (filter or swatch) names.
    void applySource();

    QRadioButton* useFilter_ = nullptr;
    QRadioButton* useColor_ = nullptr;
    QComboBox* filter_ = nullptr;
    ColorSwatchWidget* color_ = nullptr;
};

} // namespace pictura
