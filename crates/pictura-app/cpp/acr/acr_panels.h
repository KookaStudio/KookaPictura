#pragma once

// The nine ACR tab panels. One `.cpp` per tab; each exposes the controls the
// reference screenshots list and writes them into the shared `AcrSettings`.

#include "acr_controls.h"

namespace pictura {

class AcrBasicPanel : public AcrPanel {
    Q_OBJECT

public:
    explicit AcrBasicPanel(AcrSettings* settings, QWidget* parent = nullptr);

private:
    void resetBasic();
};

class AcrToneCurvePanel : public AcrPanel {
    Q_OBJECT

public:
    explicit AcrToneCurvePanel(AcrSettings* settings, QWidget* parent = nullptr);
};

class AcrDetailPanel : public AcrPanel {
    Q_OBJECT

public:
    explicit AcrDetailPanel(AcrSettings* settings, QWidget* parent = nullptr);
};

class AcrHslPanel : public AcrPanel {
    Q_OBJECT

public:
    explicit AcrHslPanel(AcrSettings* settings, QWidget* parent = nullptr);
};

class AcrSplitToningPanel : public AcrPanel {
    Q_OBJECT

public:
    explicit AcrSplitToningPanel(AcrSettings* settings, QWidget* parent = nullptr);
};

class AcrLensPanel : public AcrPanel {
    Q_OBJECT

public:
    explicit AcrLensPanel(AcrSettings* settings, QWidget* parent = nullptr);
};

class AcrEffectsPanel : public AcrPanel {
    Q_OBJECT

public:
    explicit AcrEffectsPanel(AcrSettings* settings, QWidget* parent = nullptr);
};

class AcrCalibrationPanel : public AcrPanel {
    Q_OBJECT

public:
    explicit AcrCalibrationPanel(AcrSettings* settings, QWidget* parent = nullptr);
};

class AcrPresetsPanel : public AcrPanel {
    Q_OBJECT

public:
    explicit AcrPresetsPanel(AcrSettings* settings, QWidget* parent = nullptr);
};

} // namespace pictura
