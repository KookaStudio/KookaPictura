#pragma once

#include "adjustment_dialog.h"

class QCheckBox;
class QComboBox;
class QLabel;

namespace pictura {

// Image > Adjustments > Channel Mixer in CS6's layout: the Output Channel
// menu, a Source Channels group whose rows sit over sliders ramping into their
// own colour, the sources' Total, Constant, and OK / Cancel / Preview down the
// right. Each output channel edits its own row. An RGB document has Red /
// Green / Blue sources and a Monochrome checkbox (the Gray output); a CMYK
// document has Cyan / Magenta / Yellow / Black sources and no Monochrome.
// Ported from photorust's ChannelMixerDialog.
class ChannelMixerDialog : public AdjustmentDialog {
    Q_OBJECT

public:
    ChannelMixerDialog(PictureView* view, const QByteArray& block, const QRect& visible,
                       QWidget* parent = nullptr);

    QWidget* controlForTest(const QString& key) const override;
    void reloadFromBlock() override;

protected:
    QString paramKey(const QString& key) const override;

private:
    void syncOutputs();
    void loadOutput();
    void updateTotal();
    void toggleMonochrome(bool on);

    QComboBox* output_ = nullptr;
    QLabel* total_ = nullptr;
    QCheckBox* monochrome_ = nullptr;
    QStringList sources_;
};

} // namespace pictura
