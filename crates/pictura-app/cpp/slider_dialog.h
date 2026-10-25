#pragma once

#include "adjustment_dialog.h"

#include <QtCore/QVector>

namespace pictura {

// An Image > Adjustments dialog of stacked value rows in CS6's layout: each
// row a label with its number field above a full-width slider, Preview under
// them, and OK / Cancel (plus any extra buttons) down the right. Brightness/
// Contrast and Vibrance use it. Ported from photorust's
// BrightnessContrastDialog / VibranceDialog.
// ponytail: Brightness/Contrast's Auto stays disabled and there is no Use
// Legacy check (the engine decodes the modern curve only).
class SliderDialog : public AdjustmentDialog {
    Q_OBJECT

public:
    struct Row {
        QString label;
        QString key;
    };

    SliderDialog(PictureView* view, const QByteArray& block, const QRect& visible,
                 const QString& title, const QVector<Row>& rows,
                 const QStringList& disabledButtons = {}, QWidget* parent = nullptr);

    QWidget* controlForTest(const QString& key) const override;
};

} // namespace pictura
