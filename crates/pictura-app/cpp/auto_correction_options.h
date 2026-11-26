#pragma once

#include "api/layers_bridge.h"

#include <QtWidgets/QDialog>

#include <array>
#include <functional>

class QButtonGroup;
class QCheckBox;
class QDoubleSpinBox;

namespace pictura {

class ColorSwatchWidget;
struct Preferences;

AutoCorrectionSettings autoCorrectionSettings(const Preferences& preferences);
bool saveAutoCorrectionSettings(const AutoCorrectionSettings& settings);

class AutoCorrectionOptionsDialog : public QDialog {
public:
    AutoCorrectionOptionsDialog(const AutoCorrectionSettings& settings,
                                std::function<void(const AutoCorrectionSettings&)> preview,
                                QWidget* parent);

    AutoCorrectionSettings settings() const;
    bool saveAsDefaults() const;

private:
    QButtonGroup* algorithms_;
    QCheckBox* snap_;
    QDoubleSpinBox* shadowClip_;
    QDoubleSpinBox* highlightClip_;
    std::array<ColorSwatchWidget*, 3> targets_;
    QCheckBox* save_;
};

} // namespace pictura
