#pragma once

#include "api/type_api.h"

#include <QtWidgets/QDialog>

class QButtonGroup;
class QComboBox;
class QSlider;
class QSpinBox;

namespace pictura {

// `Type > Warp Text…` (docs/03-tools/type-tools.md): Style (None and the
// fifteen presets), Horizontal / Vertical, then Bend, Horizontal Distortion
// and Vertical Distortion as -100…+100 % sliders. Style None disables the
// rest, as CS6 does. Every user change emits `changed` so the frame can
// preview it on the canvas; the frame commits on accept and restores on
// reject.
class WarpTextDialog : public QDialog {
    Q_OBJECT

public:
    explicit WarpTextDialog(const type_api::WarpSetting& initial, QWidget* parent = nullptr);

    type_api::WarpSetting setting() const;

signals:
    void changed();

private:
    void styleChosen();
    void syncEnabled();

    QComboBox* style_ = nullptr;
    QButtonGroup* orientation_ = nullptr;
    // Bend, Horizontal Distortion, Vertical Distortion.
    QSpinBox* values_[3] = {};
    QSlider* sliders_[3] = {};
    // The style before the latest choice: picking a preset from None starts
    // Bend at +50.
    QString lastStyle_;
};

} // namespace pictura
