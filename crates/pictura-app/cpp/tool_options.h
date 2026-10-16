#pragma once

#include <QtCore/QPoint>
#include <QtCore/QPointF>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtGui/QColor>

#include <algorithm>
#include <array>
#include <functional>
#include <utility>

namespace pictura {

// How a selection edit combines with the existing selection (and the shape
// tools' mode buttons).
enum class SelectionMode { New, Add, Subtract, Intersect };

// The rectangular/elliptical marquee's constraint.
enum class MarqueeStyle { Normal, FixedRatio, FixedSize };

// The option groups a tool family owns. The concrete group is one of the
// classes below; `ToolController::optionsFor<T>` resolves it through the
// active handler's port.
class ToolOptions {
public:
    virtual ~ToolOptions() = default;
};

// The Color Replacement options bar: Mode 0 Hue … 3 Luminosity (2 Color),
// Sampling 0 Continuous / 1 Once / 2 Background Swatch, Limits
// 0 Discontiguous / 1 Contiguous / 2 Find Edges, Tolerance 0-100 %.
struct ColorReplaceOptions {
    int mode = 2;
    int sampling = 0;
    int limits = 1;
    int tolerance = 30;
    bool antialias = true;
};

// The Mixer Brush options bar (all 0-100 %) and its after-stroke toggles.
// Defaults are photorust's "Dry" preset; the spec leaves CS6's open. The
// reservoir is the paint on the brush (alpha 0: clean); it outlives strokes.
struct MixerOptions {
    int wet = 0;
    int load = 50;
    int mix = 0;
    int flow = 100;
    bool loadAfterStroke = false;
    bool cleanAfterStroke = false;
    QColor reservoir = Qt::black;
};

// The Clone Stamp and Pattern Stamp options bars. Sample 0 Current Layer /
// 1 Current And Below / 2 All Layers; Ignore Adjustment Layers applies to All
// Layers only. `pattern` indexes the built-in patterns (stamp_pattern_name).
struct StampOptions {
    bool cloneAligned = true;
    int cloneSample = 0;
    bool ignoreAdjustments = false;
    int pattern = 0;
    bool patternAligned = true;
};

// The brush tip's dynamics (Scattering and Shape Dynamics), set by the brush
// preset picker: `count` dabs per step (1-16), each offset up to `scatter` % of
// the diameter, shrunk by up to `sizeJitter` %, turned by up to ±`angleJitter`°,
// and flattened by up to `roundnessJitter` %.
struct BrushDynamics {
    int scatter = 0;
    int count = 1;
    int sizeJitter = 0;
    int angleJitter = 0;
    int roundnessJitter = 0;
};

// The Eraser options bar: Mode 0 Brush / 1 Pencil / 2 Block, and Erase To
// History (Alt held while pressing does the same for one stroke).
struct EraserOptions {
    int mode = 0;
    bool toHistory = false;
};

// The Background Eraser options bar: Sampling 0 Continuous / 1 Once /
// 2 Background Swatch, Limits 0 Discontiguous / 1 Contiguous / 2 Find Edges,
// Tolerance 0-100 %, Protect Foreground Color.
struct BackgroundEraseOptions {
    int sampling = 0;
    int limits = 1;
    int tolerance = 50;
    bool protectForeground = false;
};

// The Magic Eraser options bar: Tolerance 0-255 per channel (the Magic Wand's
// scale), Anti-alias, Contiguous, Sample All Layers, Opacity 0-100 %.
struct MagicEraseOptions {
    int tolerance = 32;
    bool antialias = true;
    bool contiguous = true;
    bool sampleAllLayers = false;
    int opacity = 100;
};

// The Art History Brush options bar: Style 0 Tight Short … 9 Loose Curl Long
// (art_history_style_name), Area in pixels, Tolerance 0-100 %. Tolerance
// starts at 0 (paint anywhere); the spec's unverified 100 would paint almost
// nowhere.
struct ArtHistoryOptions {
    int style = 0;
    int area = 50;
    int tolerance = 0;
};

// The Blur, Sharpen, and Smudge options bars (each tool keeps its own):
// Strength 1-100 %, Mode 0 Normal / 1 Darken / 2 Lighten / 3 Hue /
// 4 Saturation / 5 Color / 6 Luminosity, Sample All Layers, Sharpen's Protect
// Detail, and Smudge's Finger Painting.
struct RetouchOptions {
    int strength = 50;
    int mode = 0;
    bool sampleAllLayers = false;
    bool protectDetail = true;
    bool fingerPainting = false;
};

// The Dodge, Burn, and Sponge options bars (each tool keeps its own):
// `amount` 1-100 % is Dodge and Burn's Exposure and the Sponge's Flow. Dodge
// and Burn: Range 0 Shadows / 1 Midtones / 2 Highlights, Protect Tones.
// Sponge: Mode 0 Desaturate / 1 Saturate, Vibrance.
struct ToneOptions {
    int amount = 50;
    int range = 1;
    bool protectTones = true;
    int spongeMode = 0;
    bool vibrance = true;
};

// The Eyedropper options bar: Sample Size in pixels (odd, 1..101; 1 is the
// point sample), the Sample scope 0 Current Layer / 1 Current & Below /
// 2 All Layers / 3 All Layers No Adjustments / 4 Current & Below No
// Adjustments, and Show Sampling Ring.
struct EyedropperOptions {
    int sampleSize = 1;
    int scope = 2;
    bool ring = false;
};

// The Gradient options bar: `preset` indexes the built-in gradients
// (gradient_preset_name), Style 0 Linear / 1 Radial / 2 Angle / 3 Reflected /
// 4 Diamond, Mode a Brush mode, Opacity 0-100 %.
struct GradientOptions {
    int preset = 0;
    int style = 0;
    QString mode = QStringLiteral("normal");
    int opacity = 100;
    bool reverse = false;
    bool dither = false;
    bool transparency = true;
};

// The Paint Bucket options bar: Fill 0 Foreground / 1 Pattern (`pattern`
// indexes the built-in patterns), Mode a Brush mode, Opacity 0-100 %,
// Tolerance 0-255 per channel, Anti-alias, Contiguous, All Layers.
struct BucketOptions {
    int fill = 0;
    int pattern = 0;
    QString mode = QStringLiteral("normal");
    int opacity = 100;
    int tolerance = 32;
    bool antialias = true;
    bool contiguous = true;
    bool allLayers = false;
};

// The Pen tool group's options: the Pen's Auto Add/Delete and Rubber Band, and
// the Freeform Pen's Curve Fit (0.5-10 px; Douglas-Peucker tolerance). Path
// Selection's Show Bounding Box rides along.
struct PenOptions {
    bool autoAddDelete = true;
    bool rubberBand = false;
    double curveFit = 2.0;
    bool showBoundingBox = false;
};

// The shape tools' options bar, shared by the six tools: Mode 0 Shape /
// 1 Path / 2 Pixels, the Rounded Rectangle's corner Radius in pixels (0-1000),
// the Polygon's Sides (3-100), the Line's Weight (1-1000 px) and arrowheads
// (Start / End, Width 10-1000 % and Length 10-5000 % of the weight, Concavity
// -50-50 %), and the Custom Shape's index. The appearance: Fill (an invalid
// `fillColor` follows the foreground colour) and Stroke (colour, width px,
// Align 0 Inside / 1 Center / 2 Outside). The geometry gear: 0 Unconstrained /
// 1 Square, Circle, or Defined Proportions / 2 Fixed Size, and From Center;
// the Polygon's star and smoothing. Align Edges. `activeWidth` /
// `activeHeight` mirror the active shape layer's size (0 without one); the
// W / H fields edit it, `linkSize` keeping its proportions.
struct ShapeOptions {
    int mode = 0;
    double radius = 10.0;
    int sides = 5;
    double weight = 1.0;
    bool arrowStart = false;
    bool arrowEnd = false;
    double arrowWidth = 500.0;
    double arrowLength = 1000.0;
    double arrowConcavity = 0.0;
    int custom = 0;
    bool fillEnabled = true;
    QColor fillColor;
    bool strokeEnabled = false;
    QColor strokeColor = Qt::black;
    double strokeWidth = 3.0;
    int strokeAlign = 1;
    int geometry = 0;
    double fixedWidth = 100.0;
    double fixedHeight = 100.0;
    bool fromCenter = false;
    bool star = false;
    double indent = 50.0;
    bool smoothCorners = false;
    bool smoothIndents = false;
    bool alignEdges = true;
    double activeWidth = 0.0;
    double activeHeight = 0.0;
    bool linkSize = false;
};

// The Type tools' options bar, shared by all four (the tool is the
// orientation): the font family, its size in pixels, Anti-alias (None / Sharp /
// Crisp / Strong / Smooth, as the combo index), and the alignment 0 left / top,
// 1 right / bottom, 2 centre. The text colour is the foreground colour.
struct TypeOptions {
    QString family = QStringLiteral("Liberation Sans");
    double size = 24.0;
    int antialias = 1;
    int justification = 0;
    // The text colour: the foreground colour, or a reopened layer's colour.
    QColor color = Qt::black;
};

// One Clone Source panel slot: the Alt-clicked source point; the offset the
// first stroke measured (source minus destination) and the destination point
// it was measured at; and the source transform (W / H %, rotation in degrees
// counter-clockwise, flips).
struct CloneSource {
    bool hasSource = false;
    QPoint source;
    bool hasOffset = false;
    QPoint offset;
    QPoint anchor;
    double width = 100.0;
    double height = 100.0;
    double angle = 0.0;
    bool flipH = false;
    bool flipV = false;
};

// The Move tool options: Auto-Select on/off and 0 Group / 1 Layer, Show
// Transform Controls (on by default by owner decision; CS6 ships it off),
// Align To 0 Selection / 1 Canvas.
struct MoveOptions {
    bool autoSelectOn = false;
    int autoSelect = 0;
    bool showTransformControls = true;
    int alignTo = 0;
};

// The Spot Healing Brush Type (0 Proximity Match, 1 Create Texture,
// 2 Content-Aware) and Healing Brush Aligned (keep the sample offset across
// strokes).
struct HealingOptions {
    int spotType = 0;
    bool aligned = true;
};

// The Patch tool: Content-Aware rebuilds the selection in place and ignores
// the drag; Destination applies the selection where it is dragged;
// Transparent transfers texture only.
struct PatchOptions {
    bool contentAware = false;
    bool destination = false;
    bool transparent = false;
};

// The Content-Aware Move tool: Extend copies instead of moving; Adaptation is
// 0 Very Strict … 4 Very Loose (2 Medium, the default).
struct ContentAwareMoveOptions {
    bool extend = false;
    int adaptation = 2;
};

// The Red Eye tool: Pupil Size and Darken Amount, 0-100 % (CS6 defaults
// 50 / 50).
struct RedEyeOptions {
    int pupil = 50;
    int darken = 50;
};

// The magnetic lasso's detection settings: Width 1-256 px, Contrast 1-100 %,
// Frequency 0-100; CS6 defaults 10 / 10 / 57.
struct MagneticOptions {
    int width = 10;
    int contrast = 10;
    int frequency = 57;
};

// The clamping every `ToolOption<T>::set` applies. The generic template is the
// identity; the overloads below are the families whose fields have ranges.
template <typename T> T clampOptions(T value) { return value; }

inline PenOptions clampOptions(PenOptions options)
{
    options.curveFit = std::clamp(options.curveFit, 0.5, 10.0);
    return options;
}

inline TypeOptions clampOptions(TypeOptions options)
{
    options.size = std::clamp(options.size, 1.0, 1296.0);
    options.justification = std::clamp(options.justification, 0, 2);
    return options;
}

inline ShapeOptions clampOptions(ShapeOptions options)
{
    options.mode = std::clamp(options.mode, 0, 2);
    options.radius = std::clamp(options.radius, 0.0, 1000.0);
    options.sides = std::clamp(options.sides, 3, 100);
    options.weight = std::clamp(options.weight, 1.0, 1000.0);
    options.arrowWidth = std::clamp(options.arrowWidth, 10.0, 1000.0);
    options.arrowLength = std::clamp(options.arrowLength, 10.0, 5000.0);
    options.arrowConcavity = std::clamp(options.arrowConcavity, -50.0, 50.0);
    options.custom = std::max(options.custom, 0);
    options.strokeWidth = std::clamp(options.strokeWidth, 1.0, 250.0);
    options.strokeAlign = std::clamp(options.strokeAlign, 0, 2);
    options.geometry = std::clamp(options.geometry, 0, 2);
    options.fixedWidth = std::max(options.fixedWidth, 0.01);
    options.fixedHeight = std::max(options.fixedHeight, 0.01);
    options.indent = std::clamp(options.indent, 1.0, 99.0);
    return options;
}

inline EyedropperOptions clampOptions(EyedropperOptions options)
{
    const int size = std::clamp(options.sampleSize, 1, 101);
    options.sampleSize = size % 2 == 1 ? size : size + 1;
    options.scope = std::clamp(options.scope, 0, 4);
    return options;
}

inline RedEyeOptions clampOptions(RedEyeOptions options)
{
    options.pupil = std::clamp(options.pupil, 0, 100);
    options.darken = std::clamp(options.darken, 0, 100);
    return options;
}

inline HealingOptions clampOptions(HealingOptions options)
{
    options.spotType = std::clamp(options.spotType, 0, 2);
    return options;
}

inline ContentAwareMoveOptions clampOptions(ContentAwareMoveOptions options)
{
    options.adaptation = std::clamp(options.adaptation, 0, 4);
    return options;
}

inline MoveOptions clampOptions(MoveOptions options)
{
    options.autoSelect = options.autoSelect == 1 ? 1 : 0;
    options.alignTo = options.alignTo == 1 ? 1 : 0;
    return options;
}

// One family option value: `set` clamps, stores, and notifies. `update` is for
// computed state that must not notify (the crop pivot).
template <typename T>
class ToolOption : public ToolOptions {
public:
    const T& get() const { return value_; }

    void set(const T& value)
    {
        value_ = clampOptions(value);
        if (changed) {
            changed();
        }
    }

    void update(const T& value) { value_ = value; }

    std::function<void()> changed;

private:
    T value_{};
};

// The named groups the options UI resolves through `optionsFor<T>`.
using PenGroup = ToolOption<PenOptions>;
using ShapeGroup = ToolOption<ShapeOptions>;
using TypeGroup = ToolOption<TypeOptions>;
using MoveGroup = ToolOption<MoveOptions>;
using HealingGroup = ToolOption<HealingOptions>;
using PatchGroup = ToolOption<PatchOptions>;
using ContentAwareMoveGroup = ToolOption<ContentAwareMoveOptions>;
using RedEyeGroup = ToolOption<RedEyeOptions>;
using EyedropperGroup = ToolOption<EyedropperOptions>;
using GradientGroup = ToolOption<GradientOptions>;
using BucketGroup = ToolOption<BucketOptions>;
using StampGroup = ToolOption<StampOptions>;
using EraserGroup = ToolOption<EraserOptions>;
using BackgroundEraseGroup = ToolOption<BackgroundEraseOptions>;
using MagicEraseGroup = ToolOption<MagicEraseOptions>;
using ArtHistoryGroup = ToolOption<ArtHistoryOptions>;
using RetouchGroup = ToolOption<RetouchOptions>;
using ToneGroup = ToolOption<ToneOptions>;
using ColorReplaceGroup = ToolOption<ColorReplaceOptions>;
using MixerGroup = ToolOption<MixerOptions>;

// The paint family's shared brush: tip shape, dynamics, and paint stroke. The
// size has its own notification (the hover ring and the size readouts); every
// other tip field shares `tipChanged`.
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

    int opacity() const { return opacity_; }
    void setOpacity(int opacity) { opacity_ = std::clamp(opacity, 0, 100); }

    int flow() const { return flow_; }
    void setFlow(int flow) { flow_ = std::clamp(flow, 0, 100); }

    const QString& mode() const { return mode_; }
    void setMode(const QString& mode) { mode_ = mode; }

    bool autoErase() const { return autoErase_; }
    void setAutoErase(bool on) { autoErase_ = on; }

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
        const BrushDynamics clamped{
            std::clamp(dynamics.scatter, 0, 1000), std::clamp(dynamics.count, 1, 16),
            std::clamp(dynamics.sizeJitter, 0, 100), std::clamp(dynamics.angleJitter, 0, 180),
            std::clamp(dynamics.roundnessJitter, 0, 100)};
        const BrushDynamics& old = dynamics_;
        if (clamped.scatter != old.scatter || clamped.count != old.count
            || clamped.sizeJitter != old.sizeJitter || clamped.angleJitter != old.angleJitter
            || clamped.roundnessJitter != old.roundnessJitter) {
            dynamics_ = clamped;
            if (tipChanged) {
                tipChanged();
            }
        }
    }

    std::function<void(int)> sizeChanged;
    std::function<void()> tipChanged;

private:
    int size_ = 12;
    int hardness_ = 100;
    int opacity_ = 100;
    int flow_ = 100;
    QString mode_ = QStringLiteral("normal");
    bool autoErase_ = false;
    int roundness_ = 100;
    int tipAngle_ = 0;
    int spacing_ = 25;
    bool flipX_ = false;
    bool flipY_ = false;
    BrushDynamics dynamics_;
};

// The selection family's shared options: combine mode, marquee constraint,
// tolerance, wand settings, and the magnetic lasso's detection fields.
class SelectionOptions : public ToolOptions {
public:
    SelectionMode combineMode() const { return combineMode_; }
    void setCombineMode(SelectionMode mode) { combineMode_ = mode; }

    MarqueeStyle marqueeStyle() const { return marqueeStyle_; }
    void setMarqueeStyle(MarqueeStyle style) { marqueeStyle_ = style; }

    double feather() const { return feather_; }
    void setFeather(double feather)
    {
        feather_ = feather > 0.0 ? std::min(feather, 250.0) : 0.0;
    }

    double fixedRatioWidth() const { return fixedRatioW_; }
    double fixedRatioHeight() const { return fixedRatioH_; }
    void setFixedRatio(double width, double height)
    {
        fixedRatioW_ = width > 0.0 ? width : 1.0;
        fixedRatioH_ = height > 0.0 ? height : 1.0;
    }

    int fixedSizeWidth() const { return fixedSizeW_; }
    int fixedSizeHeight() const { return fixedSizeH_; }
    void setFixedSize(int width, int height)
    {
        fixedSizeW_ = std::max(width, 1);
        fixedSizeH_ = std::max(height, 1);
    }

    int tolerance() const { return tolerance_; }
    void setTolerance(int tolerance) { tolerance_ = std::clamp(tolerance, 0, 255); }

    bool contiguous() const { return contiguous_; }
    void setContiguous(bool on) { contiguous_ = on; }

    bool antiAlias() const { return antiAlias_; }
    bool sampleAllLayers() const { return sampleAllLayers_; }

    int magneticWidth() const { return magneticWidth_; }
    void setMagneticWidth(int width)
    {
        const int clamped = std::clamp(width, 1, 256);
        if (clamped != magneticWidth_) {
            magneticWidth_ = clamped;
            if (magneticWidthChanged) {
                magneticWidthChanged(clamped);
            }
        }
    }

    int magneticContrast() const { return magneticContrast_; }
    void setMagneticContrast(int contrast) { magneticContrast_ = std::clamp(contrast, 1, 100); }

    int magneticFrequency() const { return magneticFrequency_; }
    void setMagneticFrequency(int frequency)
    {
        magneticFrequency_ = std::clamp(frequency, 0, 100);
    }

    std::function<void(int)> magneticWidthChanged;

private:
    SelectionMode combineMode_ = SelectionMode::New;
    MarqueeStyle marqueeStyle_ = MarqueeStyle::Normal;
    double feather_ = 0.0;
    double fixedRatioW_ = 1.0;
    double fixedRatioH_ = 1.0;
    int fixedSizeW_ = 100;
    int fixedSizeH_ = 100;
    int tolerance_ = 32;
    bool contiguous_ = true;
    bool antiAlias_ = true;
    bool sampleAllLayers_ = true;
    int magneticWidth_ = 10;
    int magneticContrast_ = 10;
    int magneticFrequency_ = 57;
};

// The Crop and Perspective Crop tools' options. `angle` has its own
// notification (the straighten preview) and `straightenMode` its own (the
// spirit-level arming); every other field shares `optionsChanged`.
class CropOptions : public ToolOptions {
public:
    double ratio() const { return ratio_; }
    void setRatio(double ratio)
    {
        ratio_ = ratio > 0.0 ? ratio : 0.0;
        if (optionsChanged) {
            optionsChanged();
        }
    }

    bool deletePixels() const { return deletePixels_; }
    void setDeletePixels(bool on)
    {
        deletePixels_ = on;
        if (optionsChanged) {
            optionsChanged();
        }
    }

    double angle() const { return angle_; }
    void setAngle(double degrees)
    {
        double a = degrees;
        while (a > 180.0) {
            a -= 360.0;
        }
        while (a < -180.0) {
            a += 360.0;
        }
        angle_ = a;
        if (angleChanged) {
            angleChanged();
        }
    }

    // The pivot the straighten preview rotates about (image space); computed
    // when the angle is set and held fixed afterwards.
    QPointF pivot() const { return pivot_; }
    void setPivot(const QPointF& pivot) { pivot_ = pivot; }

    bool classicMode() const { return classicMode_; }
    void setClassicMode(bool on)
    {
        if (classicMode_ == on) {
            return;
        }
        classicMode_ = on;
        if (optionsChanged) {
            optionsChanged();
        }
    }

    // The guide overlays, in grid-menu order; the index is gridOverlay().
    static const QStringList& overlayNames()
    {
        static const QStringList names = {
            QStringLiteral("Rule of Thirds"), QStringLiteral("Grid"),
            QStringLiteral("Diagonal"),       QStringLiteral("Triangle"),
            QStringLiteral("Golden Ratio"),   QStringLiteral("Golden Spiral")};
        return names;
    }

    // The icon id of overlay `index` (same order as overlayNames()).
    static QString overlayIcon(int index)
    {
        static const QStringList ids = {
            QStringLiteral("crop.overlay.ruleOfThirds"), QStringLiteral("crop.overlay.grid"),
            QStringLiteral("crop.overlay.diagonal"),     QStringLiteral("crop.overlay.triangle"),
            QStringLiteral("crop.overlay.goldenRatio"),  QStringLiteral("crop.overlay.goldenSpiral")};
        return ids.value(index, ids.first());
    }

    int gridOverlay() const { return gridOverlay_; }
    void setGridOverlay(int index)
    {
        if (gridOverlay_ == index) {
            return;
        }
        gridOverlay_ = index;
        if (optionsChanged) {
            optionsChanged();
        }
    }

    bool straightenMode() const { return straightenMode_; }
    void setStraightenMode(bool on)
    {
        straightenMode_ = on;
        if (straightenChanged) {
            straightenChanged();
        }
    }

    // Restore persisted values without notifying (startup).
    void apply(bool classic, bool deletePixels, double ratio, int grid)
    {
        classicMode_ = classic;
        deletePixels_ = deletePixels;
        ratio_ = ratio > 0.0 ? ratio : 0.0;
        gridOverlay_ = grid;
    }

    std::function<void()> optionsChanged;
    std::function<void()> angleChanged;
    std::function<void()> straightenChanged;

private:
    double ratio_ = 0.0;
    bool deletePixels_ = true;
    double angle_ = 0.0;
    QPointF pivot_;
    bool classicMode_ = true;
    int gridOverlay_ = 0;
    bool straightenMode_ = false;
};

// The Clone Source panel's five slots; each keeps its own source.
class CloneSourceOptions : public ToolOptions {
public:
    const CloneSource& active() const { return slots_[slot_]; }
    void setActive(const CloneSource& source)
    {
        slots_[slot_] = source;
        if (changed) {
            changed();
        }
    }

    int slot() const { return slot_; }
    void setSlot(int slot)
    {
        const int clamped = std::clamp(slot, 0, int(slots_.size()) - 1);
        if (clamped != slot_) {
            slot_ = clamped;
            if (changed) {
                changed();
            }
        }
    }

    void setSlotSource(int slot, const CloneSource& source)
    {
        slots_[std::clamp(slot, 0, int(slots_.size()) - 1)] = source;
        if (changed) {
            changed();
        }
    }

    std::function<void()> changed;

private:
    std::array<CloneSource, 5> slots_{};
    int slot_ = 0;
};

} // namespace pictura
