#pragma once

#include <QtWidgets/QDialog>

class QCheckBox;
class QRadioButton;

namespace pictura {

class PicturaMainWindow;

// Run Auto-Align Layers over `window`'s selected layers, with progress and
// Cancel, and keep the moved layers selected. False when nothing was
// committed. Defined in frame_menus_align.cpp.
bool runAutoAlign(PicturaMainWindow& window, int projection, bool vignette, bool geometric);

// The dialog's options, kept between runs in a session.
struct AutoAlignChoice {
    int projection = 0; // 0 Auto … 6 Lens Correction
    bool vignette = false;
    bool geometric = false;
};

// Edit > Auto-Align Layers: the seven CS6 projections and the Lens Correction
// sub-options.
class AutoAlignDialog : public QDialog {
public:
    explicit AutoAlignDialog(QWidget* parent = nullptr, const AutoAlignChoice& choice = {});

    int projection() const;
    bool vignette() const;
    bool geometric() const;
    AutoAlignChoice choice() const;

private:
    void syncLensOptions();

    QRadioButton* auto_ = nullptr;
    QRadioButton* perspective_ = nullptr;
    QRadioButton* cylindrical_ = nullptr;
    QRadioButton* spherical_ = nullptr;
    QRadioButton* collage_ = nullptr;
    QRadioButton* reposition_ = nullptr;
    QRadioButton* lens_ = nullptr;
    QCheckBox* vignette_ = nullptr;
    QCheckBox* geometric_ = nullptr;
};

} // namespace pictura
