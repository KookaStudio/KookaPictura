#pragma once

#include <QtWidgets/QDialog>

class QCheckBox;
class QRadioButton;

namespace pictura {

class PicturaMainWindow;

// Run Auto-Blend Layers over `window`'s selected layers, with progress and
// Cancel, and keep the processed layers selected. False when nothing was
// committed. Defined in frame_menus_align.cpp.
bool runAutoBlend(PicturaMainWindow& window, bool stackImages, bool seamless,
                  bool fillTransparent = false);

// The dialog's options, kept between runs in a session.
struct AutoBlendChoice {
    bool stackImages = false;
    bool seamless = false;
    bool fillTransparent = false;
};

// Edit > Auto-Blend Layers: the blend method, illustrated, Seamless Tones And
// Colors, and Content Aware Fill Transparent Areas (post-CS6, 8-bit only).
class AutoBlendDialog : public QDialog {
public:
    explicit AutoBlendDialog(bool allowFill = true, QWidget* parent = nullptr,
                             const AutoBlendChoice& choice = {});

    bool stackImages() const;
    bool seamless() const;
    bool fillTransparent() const;
    // The options to remember. Unlike fillTransparent(), this keeps a checked Fill
    // box while it is disabled, so a 16-bit run does not forget it.
    AutoBlendChoice choice() const;

private:
    QRadioButton* stack_ = nullptr;
    QCheckBox* seamless_ = nullptr;
    QCheckBox* fill_ = nullptr;
};

} // namespace pictura
