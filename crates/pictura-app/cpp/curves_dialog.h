#pragma once

#include "adjustment_dialog.h"

namespace pictura {

class CurvesControls;

class CurvesDialog : public AdjustmentDialog {
    Q_OBJECT

public:
    CurvesDialog(PictureView* view, const QByteArray& block, const QRect& visible,
                 QWidget* parent = nullptr);

    QWidget* controlForTest(const QString& key) const override;
    void reloadFromBlock() override;
    void setCanvasSampling(ToolController* tools) override;

private:
    void applyAuto() override;
    CurvesControls* curves_ = nullptr;
};

} // namespace pictura
