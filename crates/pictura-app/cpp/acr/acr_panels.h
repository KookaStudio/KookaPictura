#pragma once

// The nine ACR tab panels. One `.cpp` per tab; each exposes the controls the
// reference screenshots list and writes them into the shared `AcrSettings`.

#include "acr_controls.h"

#include <optional>

class QImage;

namespace pictura {

class AcrParamCurveGraph;

// A camera raw source's as-shot white balance: with one, the Basic panel's
// Temperature is absolute kelvin and Tint runs ±150 (#400).
struct AcrAsShot {
    int temperature = 0;
    int tint = 0;
};

class AcrBasicPanel : public AcrPanel {
    Q_OBJECT

public:
    explicit AcrBasicPanel(AcrSettings* settings, QWidget* parent = nullptr,
                           std::optional<AcrAsShot> asShot = std::nullopt);

private:
    void resetBasic();

    std::optional<AcrAsShot> asShot_;
};

class AcrToneCurvePanel : public AcrPanel {
    Q_OBJECT

public:
    explicit AcrToneCurvePanel(AcrSettings* settings, QWidget* parent = nullptr);

    // Feed the preview image so the parametric graph can draw its histogram.
    void setPreviewImage(const QImage& image);

private:
    AcrParamCurveGraph* graph_ = nullptr;
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
