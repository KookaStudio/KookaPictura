#pragma once

#include <QtCore/QPoint>

#include <QtCore/QPointF>
#include <QtCore/QString>
#include <QtCore/Qt>
#include <QtGui/QColor>

namespace pictura {

class ImageView;
class PictureView;
enum class SelectionMode;
enum class ToolId;
enum class MarqueeStyle;

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
// Defaults are photorust's "Dry" preset; the spec leaves CS6's open.
struct MixerOptions {
    int wet = 0;
    int load = 50;
    int mix = 0;
    int flow = 100;
    bool loadAfterStroke = false;
    bool cleanAfterStroke = false;
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

// The shared services a tool handler may use, implemented by `ToolController`.
// Kept minimal on purpose: add accessors only as a migrating tool needs them.
struct ToolContext {
    virtual ~ToolContext() = default;
    virtual PictureView* view() const = 0;
    virtual ImageView* canvas() const = 0;
    virtual void sampledForeground(const QColor& color) = 0;

    virtual QColor foreground() const = 0;
    virtual QColor background() const = 0;
    virtual int brushSize() const = 0;
    virtual int brushHardness() const = 0;
    virtual int brushOpacity() const = 0;
    virtual int brushFlow() const = 0;
    virtual QString brushMode() const = 0;
    // The Brush panel's Brush Tip Shape: Roundness 0-100 %, the painted angle
    // (Flip X / Flip Y folded in), and Spacing as a percentage of the size.
    virtual int brushRoundness() const = 0;
    virtual int brushAngle() const = 0;
    virtual int brushSpacing() const = 0;
    virtual BrushDynamics brushDynamics() const = 0;
    virtual bool autoErase() const = 0;
    virtual int tolerance() const = 0;
    virtual bool contiguous() const = 0;

    // Spot Healing Brush Type: 0 Proximity Match, 1 Create Texture,
    // 2 Content-Aware. Healing Brush Aligned: keep the sample offset across
    // strokes instead of re-anchoring it to each stroke's start.
    virtual int spotHealingType() const = 0;
    virtual bool healingAligned() const = 0;
    // Patch: Content-Aware rebuilds the selection in place and ignores the
    // drag; Destination applies the selection where it is dragged; Transparent
    // transfers texture only.
    virtual bool patchContentAware() const = 0;
    virtual bool patchDestination() const = 0;
    virtual bool patchTransparent() const = 0;
    // Content-Aware Move: Extend copies instead of moving; Adaptation is
    // 0 Very Strict … 4 Very Loose (2 Medium, the default).
    virtual bool contentAwareMoveExtend() const = 0;
    virtual int contentAwareAdaptation() const = 0;
    // Red Eye: Pupil Size and Darken Amount, 0-100 % (CS6 defaults 50 / 50).
    virtual int redEyePupil() const = 0;
    virtual int redEyeDarken() const = 0;
    virtual ColorReplaceOptions colorReplaceOptions() const = 0;
    virtual MixerOptions mixerOptions() const = 0;
    // The paint on the Mixer Brush (alpha 0: clean); it outlives each stroke.
    virtual QColor mixerReservoir() const = 0;
    virtual void setMixerReservoir(const QColor& color) = 0;
    virtual StampOptions stampOptions() const = 0;
    virtual EraserOptions eraserOptions() const = 0;
    virtual BackgroundEraseOptions backgroundEraseOptions() const = 0;
    virtual MagicEraseOptions magicEraseOptions() const = 0;
    virtual ArtHistoryOptions artHistoryOptions() const = 0;
    virtual GradientOptions gradientOptions() const = 0;
    // The Blur, Sharpen, or Smudge tool's options.
    virtual RetouchOptions retouchOptions(ToolId id) const = 0;
    // The Dodge, Burn, or Sponge tool's options.
    virtual ToneOptions toneOptions(ToolId id) const = 0;
    virtual BucketOptions bucketOptions() const = 0;
    virtual PenOptions penOptions() const = 0;
    virtual ShapeOptions shapeOptions() const = 0;
    // Mirror the active shape layer into the options (the bar re-reads them).
    virtual void setShapeOptions(const ShapeOptions& options) = 0;
    // Rotate View: the canvas's view rotation in degrees clockwise.
    virtual void setViewRotation(double degrees) = 0;
    virtual TypeOptions typeOptions() const = 0;
    virtual void setTypeOptions(const TypeOptions& options) = 0;
    // The Clone Source panel's active slot, read and written by the Clone Stamp.
    virtual CloneSource cloneSource() const = 0;
    virtual void setCloneSource(const CloneSource& source) = 0;

    virtual MarqueeStyle marqueeStyle() const = 0;
    virtual double fixedRatioWidth() const = 0;
    virtual double fixedRatioHeight() const = 0;
    virtual int fixedSizeWidth() const = 0;
    virtual int fixedSizeHeight() const = 0;
    virtual double feather() const = 0;
    // Crop options: aspect ratio (width / height; 0 = unconstrained) and
    // whether a crop discards the pixels outside the canvas.
    virtual double cropRatio() const = 0;
    virtual bool cropDeletePixels() const = 0;
    // The crop straighten angle in degrees (positive clockwise), 0 when unset.
    virtual double cropAngle() const = 0;
    virtual void setCropAngle(double degrees) = 0;
    // The pivot the straighten preview rotates about (image space).
    virtual QPointF cropPivot() const = 0;
    // A modal tool session's history changed; the frame re-evaluates commands.
    virtual void notifyToolSessionChanged() = 0;
    // Classic crop (default): dragging inside moves the crop box, the content
    // stays. Modern (PS CC): dragging inside moves the content under the fixed
    // box. Persisted with the other crop options.
    virtual bool cropClassicMode() const = 0;
    virtual void setCropClassicMode(bool on) = 0;
    // The crop grid overlay index (0 Rule of Thirds, 1 Grid, ...). Persisted.
    virtual int cropGridOverlay() const = 0;
    virtual void setCropGridOverlay(int index) = 0;
    // The Crop straighten line tool: while armed, a canvas drag draws a
    // horizon line whose inclination sets the straighten angle.
    virtual bool cropStraightenMode() const = 0;
    virtual void setCropStraightenMode(bool on) = 0;
    // A crop option changed; the frame persists the session.
    virtual void notifyCropOptionsChanged() = 0;
    // Re-resolve the canvas cursor from the tool's current state and the live
    // pointer (a handler's `hoverCursor`).
    virtual void refreshCursor() = 0;
    // True while the Crop tool holds an active (not preview) box.
    virtual bool cropActive() const = 0;
    // The options bar's W/H fields.
    virtual bool resizeCrop(double width, double height) = 0;
    virtual double cropWidth() const = 0;
    virtual double cropHeight() const = 0;
    virtual int magneticWidth() const = 0;
    virtual int magneticContrast() const = 0;
    virtual int magneticFrequency() const = 0;

    // Eyedropper options: Sample Size (odd pixels, 1..101), the Sample scope,
    // and the Show Sampling Ring toggle.
    virtual EyedropperOptions eyedropperOptions() const = 0;

    // Move options: Auto-Select 0 Group / 1 Layer, Show Transform Controls,
    // Align To 0 Selection / 1 Canvas.
    virtual int moveAutoSelect() const = 0;
    virtual bool moveShowTransformControls() const = 0;
    virtual int moveAlignTo() const = 0;

    // The controller's selection-move service: start a mask/content translate
    // from `imagePos`. The Move handler and the selection pre-block share it.
    virtual void beginContentMove(PictureView* v, const QPointF& imagePos, bool duplicate) = 0;

    virtual bool dragging() const = 0;
    virtual void setDragging(bool dragging) = 0;
    virtual bool dragCommitted() const = 0;
    virtual void setDragCommitted(bool committed) = 0;
    virtual SelectionMode dragMode() const = 0;
    virtual void setDragMode(SelectionMode mode) = 0;
    virtual SelectionMode resolveSelectionMode(Qt::KeyboardModifiers mods,
                                               bool hasExistingSelection) const = 0;

    // Annotations: re-read the document's color samplers and notes onto the
    // canvas; the note shown in the Notes panel (-1 none); the Ruler's line
    // changed, so its readouts must update.
    virtual void refreshAnnotations() = 0;
    virtual int currentNote() const = 0;
    virtual void setCurrentNote(int index) = 0;
    virtual void notifyRulerChanged() = 0;
    // A Count group or mark changed; the options bar re-reads its state.
    virtual void notifyCountChanged() = 0;

    virtual void refused(const QString& message) = 0;
    // A tool created the layer at `path`; the Layers panel selects it.
    virtual void notifyLayerCreated(const QString& path) = 0;
    // A Type tool started or stopped taking keystrokes.
    virtual void notifyTextEditing(bool active) = 0;
    virtual void emitSelectionCommitted() = 0;
};

} // namespace pictura
