#pragma once

#include <QtWidgets/QDialog>

class QCheckBox;
class QRadioButton;

namespace pictura {

class PicturaMainWindow;

// Run Auto-Blend Layers over `window`'s selected layers, with progress and
// Cancel, and keep the processed layers selected. False when nothing was
// committed. Defined in frame_menus_align.cpp.
bool runAutoBlend(PicturaMainWindow& window, bool stackImages, bool seamless);

// Edit > Auto-Blend Layers: the blend objective and Seamless Tones And Colors.
class AutoBlendDialog : public QDialog {
public:
    explicit AutoBlendDialog(QWidget* parent = nullptr);

    bool stackImages() const;
    bool seamless() const;

private:
    QRadioButton* stack_ = nullptr;
    QCheckBox* seamless_ = nullptr;
};

} // namespace pictura
