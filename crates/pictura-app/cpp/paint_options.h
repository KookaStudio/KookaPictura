#pragma once

#include "tool_options.h"

#include <QtCore/QString>

#include <algorithm>
#include <functional>

namespace pictura {

// The brush tip's dynamics. The five legacy fields are Scattering and Shape
// Dynamics: `count` dabs per step (1-16), each offset up to `scatter` % of the
// diameter, shrunk by up to `sizeJitter` %, turned by up to ±`angleJitter`°,
// and flattened by up to `roundnessJitter` %. The rest are the Control model
// and the Color Dynamics, Transfer, and Brush Pose sets. Controls are 0 Off,
// 1 Fade, 2 Pen Pressure, 3 Pen Tilt, 4 Stylus Wheel, 5 Rotation,
// 6 Initial Direction, 7 Direction; each `*Fade` is the Fade step count.
struct BrushDynamics {
    int scatter = 0;
    int count = 1;
    int sizeJitter = 0;
    int angleJitter = 0;
    int roundnessJitter = 0;

    bool shapeEnabled = false;
    int sizeControl = 0;
    int sizeFade = 25;
    int minDiameter = 0;
    int angleControl = 0;
    int angleFade = 25;
    int roundnessControl = 0;
    int roundnessFade = 25;
    int minRoundness = 0;
    int tiltScale = 100;
    bool brushProjection = false;

    bool scatterEnabled = true;
    int scatterControl = 0;
    int scatterFade = 25;
    bool bothAxes = true;
    int countControl = 0;
    int countFade = 25;
    int countJitter = 0;

    bool colorEnabled = false;
    bool applyPerTip = false;
    int fgBgJitter = 0;
    int fgBgControl = 0;
    int fgBgFade = 25;
    int hueJitter = 0;
    int saturationJitter = 0;
    int brightnessJitter = 0;
    int purity = 0;

    bool transferEnabled = false;
    int opacityJitter = 0;
    int opacityControl = 0;
    int opacityFade = 25;
    int flowJitter = 0;
    int flowControl = 0;
    int flowFade = 25;

    bool poseEnabled = false;
    int poseTiltX = 0;
    int poseTiltY = 0;
    int poseRotation = 0;
    int posePressure = 100;
    bool overrideTiltX = false;
    bool overrideTiltY = false;
    bool overrideRotation = false;
    bool overridePressure = false;

    bool lockShape = false;
    bool lockScatter = false;
    bool lockColor = false;
    bool lockTransfer = false;
    bool lockPose = false;
    bool lockTexture = false;
    bool lockDual = false;

    bool operator==(const BrushDynamics& o) const
    {
        return scatter == o.scatter && count == o.count && sizeJitter == o.sizeJitter
            && angleJitter == o.angleJitter && roundnessJitter == o.roundnessJitter
            && shapeEnabled == o.shapeEnabled && sizeControl == o.sizeControl
            && sizeFade == o.sizeFade && minDiameter == o.minDiameter
            && angleControl == o.angleControl && angleFade == o.angleFade
            && roundnessControl == o.roundnessControl && roundnessFade == o.roundnessFade
            && minRoundness == o.minRoundness && tiltScale == o.tiltScale
            && brushProjection == o.brushProjection && scatterEnabled == o.scatterEnabled
            && scatterControl == o.scatterControl && scatterFade == o.scatterFade
            && bothAxes == o.bothAxes && countControl == o.countControl
            && countFade == o.countFade && countJitter == o.countJitter
            && colorEnabled == o.colorEnabled && applyPerTip == o.applyPerTip
            && fgBgJitter == o.fgBgJitter && fgBgControl == o.fgBgControl
            && fgBgFade == o.fgBgFade && hueJitter == o.hueJitter
            && saturationJitter == o.saturationJitter
            && brightnessJitter == o.brightnessJitter && purity == o.purity
            && transferEnabled == o.transferEnabled && opacityJitter == o.opacityJitter
            && opacityControl == o.opacityControl && opacityFade == o.opacityFade
            && flowJitter == o.flowJitter && flowControl == o.flowControl
            && flowFade == o.flowFade && poseEnabled == o.poseEnabled
            && poseTiltX == o.poseTiltX && poseTiltY == o.poseTiltY
            && poseRotation == o.poseRotation && posePressure == o.posePressure
            && overrideTiltX == o.overrideTiltX && overrideTiltY == o.overrideTiltY
            && overrideRotation == o.overrideRotation
            && overridePressure == o.overridePressure && lockShape == o.lockShape
            && lockScatter == o.lockScatter && lockColor == o.lockColor
            && lockTransfer == o.lockTransfer && lockPose == o.lockPose
            && lockTexture == o.lockTexture && lockDual == o.lockDual;
    }
    bool operator!=(const BrushDynamics& o) const { return !(*this == o); }

    BrushDynamics clamped() const
    {
        BrushDynamics d = *this;
        d.scatter = std::clamp(d.scatter, 0, 1000);
        d.count = std::clamp(d.count, 1, 16);
        d.sizeJitter = std::clamp(d.sizeJitter, 0, 100);
        d.angleJitter = std::clamp(d.angleJitter, 0, 180);
        d.roundnessJitter = std::clamp(d.roundnessJitter, 0, 100);
        d.sizeFade = std::clamp(d.sizeFade, 1, 9999);
        d.minDiameter = std::clamp(d.minDiameter, 0, 100);
        d.angleFade = std::clamp(d.angleFade, 1, 9999);
        d.roundnessFade = std::clamp(d.roundnessFade, 1, 9999);
        d.minRoundness = std::clamp(d.minRoundness, 0, 100);
        d.tiltScale = std::clamp(d.tiltScale, 0, 100);
        d.scatterFade = std::clamp(d.scatterFade, 1, 9999);
        d.countFade = std::clamp(d.countFade, 1, 9999);
        d.countJitter = std::clamp(d.countJitter, 0, 100);
        d.fgBgJitter = std::clamp(d.fgBgJitter, 0, 100);
        d.fgBgFade = std::clamp(d.fgBgFade, 1, 9999);
        d.hueJitter = std::clamp(d.hueJitter, 0, 100);
        d.saturationJitter = std::clamp(d.saturationJitter, 0, 100);
        d.brightnessJitter = std::clamp(d.brightnessJitter, 0, 100);
        d.purity = std::clamp(d.purity, -100, 100);
        d.opacityJitter = std::clamp(d.opacityJitter, 0, 100);
        d.opacityFade = std::clamp(d.opacityFade, 1, 9999);
        d.flowJitter = std::clamp(d.flowJitter, 0, 100);
        d.flowFade = std::clamp(d.flowFade, 1, 9999);
        d.poseTiltX = std::clamp(d.poseTiltX, -90, 90);
        d.poseTiltY = std::clamp(d.poseTiltY, -90, 90);
        d.poseRotation = ((d.poseRotation % 360) + 360) % 360;
        d.posePressure = std::clamp(d.posePressure, 0, 100);
        return d;
    }
};

class PaintOptions : public ToolOptions {
public:
    int size() const { return size_; }
    void setSize(int size)
    {
        const int clamped = std::clamp(size, 1, 5000);
        if (clamped == size_) {
            return;
        }
        size_ = clamped;
        if (sizeChanged) {
            sizeChanged(size_);
        }
    }

    int hardness() const { return hardness_; }
    void setHardness(int hardness)
    {
        const int clamped = std::clamp(hardness, 0, 100);
        if (clamped != hardness_) {
            hardness_ = clamped;
            if (tipChanged) {
                tipChanged();
            }
        }
    }

    int grain() const { return grain_; }
    void setGrain(int grain)
    {
        const int clamped = std::clamp(grain, 0, 100);
        if (clamped != grain_) {
            grain_ = clamped;
            if (tipChanged) {
                tipChanged();
            }
        }
    }

    // Wet Edges: a built-up stroke paints lighter than its darker rim.
    bool wetEdges() const { return wetEdges_; }
    void setWetEdges(bool on)
    {
        if (on != wetEdges_) {
            wetEdges_ = on;
            if (tipChanged) {
                tipChanged();
            }
        }
    }

    int opacity() const { return opacity_; }
    void setOpacity(int opacity)
    {
        const int clamped = std::clamp(opacity, 0, 100);
        if (clamped != opacity_) {
            opacity_ = clamped;
            if (paintSettingsChanged) {
                paintSettingsChanged();
            }
        }
    }

    int flow() const { return flow_; }
    void setFlow(int flow)
    {
        const int clamped = std::clamp(flow, 0, 100);
        if (clamped != flow_) {
            flow_ = clamped;
            if (paintSettingsChanged) {
                paintSettingsChanged();
            }
        }
    }

    const QString& mode() const { return mode_; }
    void setMode(const QString& mode)
    {
        if (mode != mode_) {
            mode_ = mode;
            if (paintSettingsChanged) {
                paintSettingsChanged();
            }
        }
    }

    bool autoErase() const { return autoErase_; }
    void setAutoErase(bool on) { autoErase_ = on; }

    // Airbrush: while a stroke is open, keep dabbing at the last sample on a
    // timer even when the pointer is still. The Brush panel's Build-up row is
    // the same flag.
    bool airbrush() const { return airbrush_; }
    void setAirbrush(bool on)
    {
        if (on != airbrush_) {
            airbrush_ = on;
            if (paintSettingsChanged) {
                paintSettingsChanged();
            }
        }
    }

    // Always use Pressure for Size / Opacity: the options-bar pressure toggles.
    bool pressureSize() const { return pressureSize_; }
    void setPressureSize(bool on)
    {
        if (on != pressureSize_) {
            pressureSize_ = on;
            if (paintSettingsChanged) {
                paintSettingsChanged();
            }
        }
    }

    bool pressureOpacity() const { return pressureOpacity_; }
    void setPressureOpacity(bool on)
    {
        if (on != pressureOpacity_) {
            pressureOpacity_ = on;
            if (paintSettingsChanged) {
                paintSettingsChanged();
            }
        }
    }

    // Preview-only: a preset whose tip is drawn as a pen nib. The stroke
    // engine never reads it.
    bool penTip() const { return penTip_; }
    void setPenTip(bool on)
    {
        if (on != penTip_) {
            penTip_ = on;
            if (tipChanged) {
                tipChanged();
            }
        }
    }

    // A pencil preset's hard, aliased edge. The Brush handler ORs it with its
    // own Pencil flag.
    bool aliased() const { return aliased_; }
    void setAliased(bool on)
    {
        if (on != aliased_) {
            aliased_ = on;
            if (tipChanged) {
                tipChanged();
            }
        }
    }

    int roundness() const { return roundness_; }
    void setRoundness(int roundness)
    {
        const int clamped = std::clamp(roundness, 0, 100);
        if (clamped != roundness_) {
            roundness_ = clamped;
            if (tipChanged) {
                tipChanged();
            }
        }
    }

    int tipAngle() const { return tipAngle_; }
    void setTipAngle(int angle)
    {
        const int clamped = std::clamp(angle, -180, 180);
        if (clamped != tipAngle_) {
            tipAngle_ = clamped;
            if (tipChanged) {
                tipChanged();
            }
        }
    }

    // The painted angle: a single flip mirrors it, both flips cancel.
    int angle() const { return flipX_ != flipY_ ? -tipAngle_ : tipAngle_; }

    int spacing() const { return spacing_; }
    void setSpacing(int spacing)
    {
        const int clamped = std::clamp(spacing, 1, 1000);
        if (clamped != spacing_) {
            spacing_ = clamped;
            if (tipChanged) {
                tipChanged();
            }
        }
    }

    bool flipX() const { return flipX_; }
    bool flipY() const { return flipY_; }
    void setFlip(bool x, bool y)
    {
        if (x != flipX_ || y != flipY_) {
            flipX_ = x;
            flipY_ = y;
            if (tipChanged) {
                tipChanged();
            }
        }
    }

    const BrushDynamics& dynamics() const { return dynamics_; }
    void setDynamics(const BrushDynamics& dynamics)
    {
        const BrushDynamics clamped = dynamics.clamped();
        if (clamped != dynamics_) {
            dynamics_ = clamped;
            if (tipChanged) {
                tipChanged();
            }
        }
    }

    // Bristle Tip: a bundle of individual bristle tracks instead of one smooth
    // profile. Shape 0 Round / 1 Round Fan / 2 Round Curve / 3 Flat /
    // 4 Flat Fan / 5 Flat Curve; the rest are 0-100 %.
    bool bristleEnabled() const { return bristleEnabled_; }
    void setBristleEnabled(bool on)
    {
        if (on != bristleEnabled_) {
            bristleEnabled_ = on;
            if (tipChanged) {
                tipChanged();
            }
        }
    }

    int bristleShape() const { return bristleShape_; }
    void setBristleShape(int shape)
    {
        const int clamped = std::clamp(shape, 0, 5);
        if (clamped != bristleShape_) {
            bristleShape_ = clamped;
            if (tipChanged) {
                tipChanged();
            }
        }
    }

    int bristleDensity() const { return bristleDensity_; }
    void setBristleDensity(int density)
    {
        const int clamped = std::clamp(density, 0, 100);
        if (clamped != bristleDensity_) {
            bristleDensity_ = clamped;
            if (tipChanged) {
                tipChanged();
            }
        }
    }

    int bristleLength() const { return bristleLength_; }
    void setBristleLength(int length)
    {
        const int clamped = std::clamp(length, 0, 100);
        if (clamped != bristleLength_) {
            bristleLength_ = clamped;
            if (tipChanged) {
                tipChanged();
            }
        }
    }

    int bristleThickness() const { return bristleThickness_; }
    void setBristleThickness(int thickness)
    {
        const int clamped = std::clamp(thickness, 0, 100);
        if (clamped != bristleThickness_) {
            bristleThickness_ = clamped;
            if (tipChanged) {
                tipChanged();
            }
        }
    }

    int bristleStiffness() const { return bristleStiffness_; }
    void setBristleStiffness(int stiffness)
    {
        const int clamped = std::clamp(stiffness, 0, 100);
        if (clamped != bristleStiffness_) {
            bristleStiffness_ = clamped;
            if (tipChanged) {
                tipChanged();
            }
        }
    }

    // Restore persisted values without notifying (startup). The callbacks stay
    // this object's; only the data members are replaced.
    void apply(const PaintOptions& other)
    {
        size_ = std::clamp(other.size_, 1, 5000);
        hardness_ = std::clamp(other.hardness_, 0, 100);
        grain_ = std::clamp(other.grain_, 0, 100);
        wetEdges_ = other.wetEdges_;
        opacity_ = std::clamp(other.opacity_, 0, 100);
        flow_ = std::clamp(other.flow_, 0, 100);
        mode_ = other.mode_;
        autoErase_ = other.autoErase_;
        airbrush_ = other.airbrush_;
        pressureSize_ = other.pressureSize_;
        pressureOpacity_ = other.pressureOpacity_;
        penTip_ = other.penTip_;
        aliased_ = other.aliased_;
        roundness_ = std::clamp(other.roundness_, 0, 100);
        tipAngle_ = std::clamp(other.tipAngle_, -180, 180);
        spacing_ = std::clamp(other.spacing_, 1, 1000);
        flipX_ = other.flipX_;
        flipY_ = other.flipY_;
        dynamics_ = other.dynamics_.clamped();
        bristleEnabled_ = other.bristleEnabled_;
        bristleShape_ = std::clamp(other.bristleShape_, 0, 5);
        bristleDensity_ = std::clamp(other.bristleDensity_, 0, 100);
        bristleLength_ = std::clamp(other.bristleLength_, 0, 100);
        bristleThickness_ = std::clamp(other.bristleThickness_, 0, 100);
        bristleStiffness_ = std::clamp(other.bristleStiffness_, 0, 100);
    }

    std::function<void(int)> sizeChanged;
    std::function<void()> tipChanged;
    std::function<void()> paintSettingsChanged;

private:
    int size_ = 12;
    int hardness_ = 100;
    int grain_ = 0;
    bool wetEdges_ = false;
    int opacity_ = 100;
    int flow_ = 100;
    QString mode_ = QStringLiteral("normal");
    bool autoErase_ = false;
    bool airbrush_ = false;
    bool pressureSize_ = false;
    bool pressureOpacity_ = false;
    bool penTip_ = false;
    bool aliased_ = false;
    int roundness_ = 100;
    int tipAngle_ = 0;
    int spacing_ = 25;
    bool flipX_ = false;
    bool flipY_ = false;
    BrushDynamics dynamics_;
    bool bristleEnabled_ = false;
    int bristleShape_ = 0;
    int bristleDensity_ = 50;
    int bristleLength_ = 50;
    int bristleThickness_ = 50;
    int bristleStiffness_ = 50;
};

} // namespace pictura
