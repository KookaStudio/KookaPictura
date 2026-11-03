#pragma once

#include "adjustment_dialog.h"

class QComboBox;
class QLabel;

namespace pictura {

// Image > Adjustments > Channel Mixer in CS6's layout: the Output Channel
// menu, a Source Channels group whose Red / Green / Blue rows sit over sliders
// ramping into their own colour, the sources' Total, Constant, and OK /
// Cancel / Preview down the right. Each output channel edits its own row.
// Ported from photorust's ChannelMixerDialog.
// ponytail: no Monochrome check (the engine keeps a block's monochrome flag as
// it is; a monochrome block offers its one Gray output).
class ChannelMixerDialog : public AdjustmentDialog {
    Q_OBJECT

public:
    ChannelMixerDialog(PictureView* view, const QByteArray& block, const QRect& visible,
                       QWidget* parent = nullptr);

    QWidget* controlForTest(const QString& key) const override;
    void reloadFromBlock() override { loadOutput(); }

protected:
    QString paramKey(const QString& key) const override;

private:
    void loadOutput();
    void updateTotal();

    QComboBox* output_ = nullptr;
    QLabel* total_ = nullptr;
};

} // namespace pictura
