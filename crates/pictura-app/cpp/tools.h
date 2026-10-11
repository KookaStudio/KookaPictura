#pragma once

#include <QtCore/QElapsedTimer>
#include <QtCore/QList>
#include <QtCore/QMetaType>
#include <QtCore/QObject>
#include <QtCore/QPointF>
#include <QtCore/QRect>
#include <QtCore/QString>
#include <QtCore/Qt>
#include <QtGui/QColor>
#include <QtGui/QImage>

#include <functional>
#include <QtGui/QPolygonF>

#include "tool_context.h"
#include "tool_registry.h"

#include <algorithm>
#include <array>
#include <functional>
#include <utility>

namespace pictura {

class ImageView;
class PictureView;

// The 71-tool CS6 catalogue in frozen table order. The 10 implemented tools
// keep their M19 names; the enum order matches `kToolTable` in tools.cpp.
enum class ToolId {
    Move,
    Marquee,
    EllipticalMarquee,
    Lasso,
    PolygonalLasso,
    MagneticLasso,
    MagicWand,
    QuickSelection,
    Crop,
    PerspectiveCrop,
    Slice,
    SliceSelect,
    Eyedropper,
    ColorSampler,
    Ruler,
    Note,
    Count,
    SpotHealingBrush,
    HealingBrush,
    Patch,
    ContentAwareMove,
    RedEye,
    Brush,
    Pencil,
    ColorReplacement,
    MixerBrush,
    CloneStamp,
    PatternStamp,
    HistoryBrush,
    ArtHistoryBrush,
    Eraser,
    BackgroundEraser,
    MagicEraser,
    Gradient,
    PaintBucket,
    Blur,
    Sharpen,
    Smudge,
    Dodge,
    Burn,
    Sponge,
    Pen,
    FreeformPen,
    AddAnchorPoint,
    DeleteAnchorPoint,
    ConvertPoint,
    HorizontalType,
    VerticalType,
    HorizontalTypeMask,
    VerticalTypeMask,
    PathSelection,
    DirectSelection,
    Rectangle,
    RoundedRectangle,
    Ellipse,
    Polygon,
    Line,
    CustomShape,
    ObjectRotate,
    ObjectRoll,
    ObjectPan,
    ObjectSlide,
    ObjectScale,
    CameraRotate,
    CameraRoll,
    CameraPan,
    CameraWalk,
    CameraZoom,
    Hand,
    RotateView,
    Zoom
};
enum class SelectionMode { New, Add, Subtract, Intersect };
enum class MarqueeStyle { Normal, FixedRatio, FixedSize };

struct ToolInfo {
    ToolId id;
    const char* name;
    const char* label;
    QChar shortcut;
    Qt::CursorShape cursor;
    const char* hint;
    int group;
    bool implemented;
    int hotspotX;
    int hotspotY;
};

const ToolInfo& toolInfo(ToolId id);
const QList<ToolId>& allToolIds();
const QList<ToolId>& implementedToolIds();
bool toolImplemented(ToolId id);
// The painting brushes (the B group, the S stamps, the Y history brushes, the
// Eraser and Background Eraser, and the retouch tools): they share the size ring, the `[` / `]` keys, and the paint
// cursor policy.
bool isBrushTool(ToolId id);
QString selectionModeString(SelectionMode mode);

// Asset base name for a tool ("move", "quickselection"), used for the
// `tool.<name>` icon and cursor ids.
QString toolIdName(ToolId id);

// Cursor asset id for a tool under the given modifiers. The marquee tools swap
// to the `.add` / `.remove` variants for Shift / Alt; every other tool keeps
// its plain `tool.<name>` asset.
QString toolCursorId(ToolId id, Qt::KeyboardModifiers mods);

// The distinct slot letters in catalogue order, and the group a letter maps to
// (0 when no tool carries it). Every letter maps to exactly one group.
QList<QChar> toolShortcutKeys();
int toolGroupForKey(QChar key);

// One context hint for the bottom status bar. `key` is the keycap label; when
// `commandId` is set the label is taken from that command's registered shortcut
// instead of the literal `key`.
struct ToolHint {
    QString key;
    QString text;
    const char* commandId = nullptr;
};

// The active tool's keycap hints, contextual per tool; empty when the tool has
// none (the status bar then falls back to the tool's text hint).
QList<ToolHint> toolHintEntries(ToolId id);

// True when the single active layer a tool edit may target carries the `PIXELS`
// lock (`LockFlags::PIXELS` = 0x02). Shared by the paint press path and the
// cursor branch; defined in tools_marquee.cpp with the other cursor helpers.
bool activePixelLocked(PictureView* view);

// Routes canvas pointer events to the active tool. Implemented tools dispatch
// through the handler registry; the legacy switch covers the rest.
class ToolController : public QObject, public ToolContext {
    Q_OBJECT

public:
    explicit ToolController(QObject* parent = nullptr);

    ToolId activeTool() const { return active_; }
    void setActiveTool(ToolId id);

    // Re-apply the active tool's cursor using the live keyboard modifiers
    // (Shift / Alt select the marquee add / remove cursor variants).
    void refreshCursor();
    // Resolve the cursor from an explicit modifier state. The live path calls
    // the no-arg overload; the offscreen self-test drives this one to check the
    // transient Alt eyedropper without a platform keyboard state.
    void refreshCursor(Qt::KeyboardModifiers mods);
    QString cursorIdForModifiersForTest(ToolId id, int mods) const;

    SelectionMode combineMode() const { return mode_; }
    void setCombineMode(SelectionMode mode);

    MarqueeStyle marqueeStyle() const override { return marqueeStyle_; }
    void setMarqueeStyle(MarqueeStyle style);

    double feather() const override { return feather_; }
    void setFeather(double feather);

    double fixedRatioWidth() const override { return fixedRatioW_; }
    double fixedRatioHeight() const override { return fixedRatioH_; }
    void setFixedRatio(double width, double height);

    int fixedSizeWidth() const override { return fixedSizeW_; }
    int fixedSizeHeight() const override { return fixedSizeH_; }
    void setFixedSize(int width, int height);

    double cropRatio() const override { return cropRatio_; }
    void setCropRatio(double ratio);
    bool cropDeletePixels() const override { return cropDeletePixels_; }
    void setCropDeletePixels(bool on)
    {
        cropDeletePixels_ = on;
        notifyCropOptionsChanged();
    }
    double cropAngle() const override { return cropAngle_; }
    void setCropAngle(double degrees) override;
    // The pivot the straighten preview rotates about (image space), fixed when
    // the angle is set so moving the crop box leaves the content in place.
    QPointF cropPivot() const override { return cropPivot_; }
    // A modal tool session's history changed; the frame re-evaluates the
    // Undo/Redo command enabled state.
    void notifyToolSessionChanged() override;
    // Classic (default) vs Modern crop drag semantics; persisted.
    bool cropClassicMode() const override { return cropClassicMode_; }
    void setCropClassicMode(bool on) override;
    int cropGridOverlay() const override { return cropGridOverlay_; }
    void setCropGridOverlay(int index) override;
    bool cropStraightenMode() const override { return cropStraightenMode_; }
    void setCropStraightenMode(bool on) override;
    void notifyCropOptionsChanged() override;
    // Restore persisted crop options without re-emitting the save signal (used
    // during startup, before the workspace store exists).
    void applyCropOptions(bool classic, bool deletePixels, double ratio, int grid);
    // The crop cursor zone under an image point (test hook); -1 when not Crop.
    int cropCursorKindForTest(const QPointF& imagePos, int mods);
    // The crop box centre (image space; test hook); empty without a box.
    QPointF cropCenterForTest();
    bool resizeCrop(double width, double height) override;
    double cropWidth() const override;
    double cropHeight() const override;
    // Esc / the options bar's Cancel: put the crop box back to the canvas.
    void cancelCrop();
    // True while the Crop tool holds an active (not preview) box; the options
    // bar hides Cancel/Apply/Reset otherwise.
    bool cropActive() const override;

    // Move tool options: Auto-Select 0 Group / 1 Layer, Show Transform
    // Controls, and Align To 0 Selection / 1 Canvas.
    int moveAutoSelect() const override { return moveAutoSelect_; }
    void setMoveAutoSelect(int which);
    bool moveShowTransformControls() const override { return moveShowTransform_; }
    void setMoveShowTransformControls(bool on);
    int moveAlignTo() const override { return moveAlignTo_; }
    void setMoveAlignTo(int which);

    // Eyedropper options.
    EyedropperOptions eyedropperOptions() const override { return eyedropper_; }
    void setEyedropperOptions(const EyedropperOptions& options);

    int tolerance() const override { return tolerance_; }
    void setTolerance(int tolerance);

    // Magnetic Lasso options (docs/03-tools/lasso-selection.md): Width 1-256 px,
    // Contrast 1-100 %, Frequency 0-100; CS6 defaults 10 / 10 / 57.
    int magneticWidth() const override { return magneticWidth_; }
    void setMagneticWidth(int width);
    int magneticContrast() const override { return magneticContrast_; }
    void setMagneticContrast(int contrast);
    int magneticFrequency() const override { return magneticFrequency_; }
    void setMagneticFrequency(int frequency);

    bool contiguous() const override { return contiguous_; }
    void setContiguous(bool on);
    bool antiAlias() const { return antiAlias_; }
    bool sampleAllLayers() const { return sampleAllLayers_; }

    int brushSize() const override;
    void setBrushSize(int size);
    int brushHardness() const override;
    void setBrushHardness(int h);
    int brushOpacity() const override;
    void setBrushOpacity(int o);
    int brushFlow() const override;
    void setBrushFlow(int f);
    QString brushMode() const override;
    void setBrushMode(const QString& mode);
    int brushRoundness() const override { return brushRoundness_; }
    void setBrushRoundness(int roundness);
    // For elliptical tips a single flip mirrors the angle; both flips cancel.
    int brushAngle() const override
    {
        return brushFlipX_ != brushFlipY_ ? -brushTipAngle_ : brushTipAngle_;
    }
    int brushTipAngle() const { return brushTipAngle_; }
    void setBrushTipAngle(int angle);
    int brushSpacing() const override { return brushSpacing_; }
    void setBrushSpacing(int spacing);
    bool brushFlipX() const { return brushFlipX_; }
    bool brushFlipY() const { return brushFlipY_; }
    BrushDynamics brushDynamics() const override { return brushDynamics_; }
    void setBrushDynamics(const BrushDynamics& dynamics);
    void setBrushFlip(bool x, bool y);
    bool autoErase() const override;
    void setAutoErase(bool on);
    // Spot Healing Brush Type (0/1/2) and Healing Brush Aligned.
    int spotHealingType() const override { return spotHealingType_; }
    void setSpotHealingType(int type);
    bool healingAligned() const override { return healingAligned_; }
    void setHealingAligned(bool on) { healingAligned_ = on; }
    // Patch mode (Normal / Content-Aware), Source / Destination, Transparent.
    bool patchContentAware() const override { return patchContentAware_; }
    void setPatchContentAware(bool on) { patchContentAware_ = on; }
    bool patchDestination() const override { return patchDestination_; }
    void setPatchDestination(bool on) { patchDestination_ = on; }
    bool patchTransparent() const override { return patchTransparent_; }
    void setPatchTransparent(bool on) { patchTransparent_ = on; }
    // Content-Aware Move Mode (Move / Extend) and Adaptation (0..4).
    bool contentAwareMoveExtend() const override { return contentAwareMoveExtend_; }
    void setContentAwareMoveExtend(bool on) { contentAwareMoveExtend_ = on; }
    int contentAwareAdaptation() const override { return contentAwareAdaptation_; }
    void setContentAwareAdaptation(int level);
    int redEyePupil() const override { return redEyePupil_; }
    void setRedEyePupil(int pupil) { redEyePupil_ = std::clamp(pupil, 0, 100); }
    int redEyeDarken() const override { return redEyeDarken_; }
    void setRedEyeDarken(int darken) { redEyeDarken_ = std::clamp(darken, 0, 100); }
    ColorReplaceOptions colorReplaceOptions() const override { return colorReplace_; }
    void setColorReplaceOptions(const ColorReplaceOptions& options) { colorReplace_ = options; }
    MixerOptions mixerOptions() const override { return mixer_; }
    void setMixerOptions(const MixerOptions& options) { mixer_ = options; }
    QColor mixerReservoir() const override { return mixerReservoir_; }
    StampOptions stampOptions() const override { return stamp_; }
    CloneSource cloneSource() const override { return cloneSources_[cloneSlot_]; }
    void setCloneSource(const CloneSource& source) override;
    // The Clone Source panel's five slots; each keeps its own source.
    int cloneSourceSlot() const { return cloneSlot_; }
    void setCloneSourceSlot(int slot);
    void setStampOptions(const StampOptions& options) { stamp_ = options; }
    EraserOptions eraserOptions() const override { return eraser_; }
    void setEraserOptions(const EraserOptions& options) { eraser_ = options; }
    BackgroundEraseOptions backgroundEraseOptions() const override { return backgroundErase_; }
    void setBackgroundEraseOptions(const BackgroundEraseOptions& options)
    {
        backgroundErase_ = options;
    }
    MagicEraseOptions magicEraseOptions() const override { return magicErase_; }
    void setMagicEraseOptions(const MagicEraseOptions& options) { magicErase_ = options; }
    ArtHistoryOptions artHistoryOptions() const override { return artHistory_; }
    void setArtHistoryOptions(const ArtHistoryOptions& options) { artHistory_ = options; }
    RetouchOptions retouchOptions(ToolId id) const override;
    void setRetouchOptions(ToolId id, const RetouchOptions& options);
    ToneOptions toneOptions(ToolId id) const override;
    void setToneOptions(ToolId id, const ToneOptions& options);
    GradientOptions gradientOptions() const override { return gradient_; }
    void setGradientOptions(const GradientOptions& options) { gradient_ = options; }
    BucketOptions bucketOptions() const override { return bucket_; }
    void setBucketOptions(const BucketOptions& options) { bucket_ = options; }
    PenOptions penOptions() const override { return pen_; }
    void setPenOptions(const PenOptions& options);
    ShapeOptions shapeOptions() const override { return shape_; }
    void setShapeOptions(const ShapeOptions& options) override;
    double viewRotation() const;
    void setViewRotation(double degrees) override;
    TypeOptions typeOptions() const override { return type_; }
    void setTypeOptions(const TypeOptions& options) override;
    // The active Type tool's Commit / Cancel; false when no text is being typed.
    bool commitText();
    bool cancelText();
    bool textActive() const;
    // While set, a left press on the canvas samples for a dialog (Color Range)
    // instead of reaching the active tool, under the eyedropper cursor; clear
    // it with an empty function.
    void setCanvasSampler(std::function<void(const QPointF&)> sampler);
    bool canvasSamplerActive() const { return bool(canvasSampler_); }
    // Insert `text` into the open type edit; false when none is open.
    bool insertText(const QString& text);
    void setMixerReservoir(const QColor& color) override;
    QColor foreground() const override;
    void setForeground(const QColor& color);
    QColor background() const override;
    void setBackground(const QColor& color);
    void adjustBrushSize(int delta);
    void adjustBrushHardness(int delta);
    // Apply the `[`/`]` brush shortcut by Qt key value or native scan code
    // (evdev 34/35), so it works on EU/Scandinavian layouts. Guarded to the
    // Brush/Pencil tools; returns false when the key is not a brush bracket.
    bool applyBrushShortcut(int key, quint32 nativeScanCode, bool shift);

    void bindCanvas(ImageView* canvas);
    void unbindCanvas();
    ImageView* canvas() const override { return canvas_; }

    // ToolContext: the shared services a handler receives.
    PictureView* view() const override;
    void sampledForeground(const QColor& color) override;
    bool dragging() const override { return dragging_; }
    void setDragging(bool dragging) override { dragging_ = dragging; }
    bool dragCommitted() const override { return dragCommitted_; }
    void setDragCommitted(bool committed) override { dragCommitted_ = committed; }
    SelectionMode dragMode() const override { return dragMode_; }
    void setDragMode(SelectionMode mode) override { dragMode_ = mode; }
    SelectionMode resolveSelectionMode(Qt::KeyboardModifiers mods,
                                       bool hasExistingSelection) const override;
    void refused(const QString& message) override;
    void notifyLayerCreated(const QString& path) override { emit layerCreated(path); }
    void notifyTextEditing(bool active) override { emit textEditingChanged(active); }
    void emitSelectionCommitted() override;
    void refreshAnnotations() override;
    int currentNote() const override { return currentNote_; }
    void setCurrentNote(int index) override;
    void notifyRulerChanged() override { emit rulerChanged(); }
    void notifyCountChanged() override { emit countChanged(); }
    // The options bar's Clear for the active Color Sampler, Note, or Ruler tool.
    bool clearAnnotations();
    void beginContentMove(PictureView* v, const QPointF& imagePos, bool duplicate) override;

    void setViewProvider(std::function<PictureView*()> provider);

    bool hasPendingCrop() const
    {
        ToolHandler* h = registry_.forTool(ToolId::Crop);
        return h && h->hasPendingCrop();
    }
    QRect pendingCropRect() const
    {
        ToolHandler* h = registry_.forTool(ToolId::Crop);
        return h ? h->pendingCropRect() : QRect();
    }
    bool commitCrop();
    // The active tool's modal session undo/redo, consulted by Edit Undo/Redo
    // before the document history.
    bool toolUndo();
    bool toolRedo();
    bool canToolUndo() const;
    bool canToolRedo() const;
    static SelectionMode selectionModeForModifiers(SelectionMode base, Qt::KeyboardModifiers mods,
                                                   bool hasExistingSelection);
    QRect marqueeRectForTest(const QPointF& a, const QPointF& b, int mods, bool mirror) const;
    int dragModeForTest() const { return static_cast<int>(dragMode_); }
    int dragModsForTest() const
    {
        ToolHandler* h = registry_.forTool(active_);
        return h ? int(h->dragMods()) : 0;
    }
    bool dragMirrorForTest() const
    {
        ToolHandler* h = registry_.forTool(active_);
        return h && h->marqueeMirror();
    }
    // The cursor id a selection-tool drag shows, derived from the mode captured
    // at press (`dragMode_`) rather than the live keyboard state.
    QString dragCursorId() const;
    // The canvas cursor id a selection-tool hover shows under the given live
    // modifiers: the move-selection cursor when neither Shift nor Alt is held
    // and the pointer is over the selection (or Ctrl previews it); otherwise the
    // tool's modifier cursor asset (Shift -> .add, Alt -> .remove). Static so
    // the gate is testable without live keyboard state.
    static QString hoverCursorId(ToolId id, Qt::KeyboardModifiers mods, bool overSelection,
                                 bool ctrlPreview);
    bool contentMoveActiveForTest() const { return contentMove_; }

    // Polygonal Lasso interaction state. `commitPolygonLasso` closes the
    // in-progress path (Enter); `cancelPolygonLasso` discards it (Esc) and
    // returns whether anything was discarded.
    bool commitPolygonLasso();
    bool cancelPolygonLasso();
    // Delete while a click-driven lasso is open: drop its last point.
    bool removeLassoPoint();

    // Free Transform session. `beginFreeTransform` starts a session on `path`
    // and shows its overlay; commit/cancel end it. While a session is active
    // normal tool input is suspended and routed to the session.
    bool beginFreeTransform(const QString& path);
    // Begin a Skew / Distort / Perspective session on `path` (`mode` is
    // "skew", "distort", or "perspective"), showing the same overlay.
    bool beginTransformMode(const QString& path, const QString& mode);
    void commitFreeTransform();
    void cancelFreeTransform();
    bool transformSessionActive() const;
    // Whether the active session is the Move tool's `Show Transform Controls`
    // box (an auto-managed, per-drag-commit session), not a modal Ctrl+T one.
    bool transformControlsSession() const { return transformControls_; }
    // Show or hide the controls box for the current tool, checkbox, selection,
    // and active layer; rebuild it when its target or content rect went stale.
    void syncTransformControls();
    // The transform options bar's setters: they write through to the session
    // and refresh the canvas overlay and the bar.
    bool setTransformPivot(double x, double y);
    bool setTransformPivotPreset(int anchor);
    void setTransformShowPivot(bool on);
    bool setTransformWidth(double pct, bool link);
    bool setTransformHeight(double pct, bool link);
    bool setTransformAngle(double degrees);
    bool setTransformSkew(double hDegrees, double vDegrees);
    bool setTransformInterpolation(const QString& id);
    bool setTransformWarp(const QString& styleId, double bend, double dx, double dy, bool vertical);
    void clearTransformWarp();

signals:
    void activeToolChanged(ToolId id);
    void brushSizeChanged(int size);
    void magneticWidthChanged(int width);
    void foregroundSampled(const QColor& color);
    void selectionCommitted();
    // A pixel edit was refused because the target layer's pixels are locked.
    void pixelEditRefused(const QString& message);
    // The selection mask moved during a move-from-inside drag; the view changed
    // without a `changed` emission, so the overlay must be refreshed directly.
    void selectionPreviewChanged();
    // The Ruler's measuring line changed (drawn, edited, cleared, or another
    // document's line is now shown); the options bar re-reads its readout.
    void rulerChanged();
    // The Note tool placed or picked note `index` (-1: none); the Notes panel
    // shows it.
    void noteActivated(int index);
    // A Count group or mark changed; the Count options bar re-reads its state.
    void countChanged();
    // The Mixer Brush's paint changed (a stroke, Load, Clean, or Alt-click).
    void mixerReservoirChanged(const QColor& color);
    // Hardness or a Brush Tip Shape field changed (size has brushSizeChanged).
    void brushTipChanged();
    // The active Clone Source slot or its contents changed.
    void cloneSourceChanged();
    // The foreground or background colour changed.
    void colorsChanged();
    // A tool created the layer at `path` (a committed type layer).
    void layerCreated(const QString& path);
    // Text started or stopped being typed; the Type options bar shows Commit
    // and Cancel only while it is.
    void textEditingChanged(bool active);
    // The Type options changed (the bar, or a reopened type layer's settings).
    void typeOptionsChanged();
    // The shape options changed (the bar, or the active shape layer mirrored).
    void shapeOptionsChanged();
    // Rotate View turned the canvas (a drag, the options bar, or Reset View).
    void viewRotationChanged(double degrees);
    // A modal tool session (e.g. Crop) recorded, walked, or discarded a step.
    void toolSessionChanged();
    // A crop option (mode, grid overlay, ratio, delete pixels) changed; the
    // frame persists the session.
    void cropOptionsChanged();
    // A Move option (Auto-Select, Show Transform Controls, Align To) changed;
    // the frame re-evaluates the Align/Distribute enabled state.
    void moveOptionsChanged();
    // A Free Transform session began, ended, or changed (a canvas gesture, a
    // pivot press, or an options-bar setter); the options bar re-reads it.
    void transformChanged();

private:
    void applyToolPolicy();
    // Push the canvas backdrop (background colour when a Background layer
    // exists, else the transparency tile) to the active canvas.
    void updateCanvasBackdrop();
    static bool isSelectionTool(ToolId id);
    bool maybeBeginSelectionMove(PictureView* v, const QPointF& imagePos);
    void cancelSelectionMove();
    void updateSelectionHover(const QPointF& imagePos);
    void dragSelectionMove(const QPointF& imagePos);
    void releaseSelectionMove(const QPointF& imagePos);
    void handlePressed(const QPointF& imagePos, int button, int modifiers);
    void handleMoved(const QPointF& imagePos);
    void handleModifiers(int modifiers);
    void handleReleased(const QPointF& imagePos);
    void updateBrushOutline(const QPointF& imagePos);
    void updateSamplingRing(const QPointF& imagePos);
    void updateTransformOverlay(PictureView* v);
    void setTransformCursor(const QPointF& imagePos);
    // Shared begin: resolve the view, start `mode` (empty = Free), then set up
    // the overlay and pan policy. `controls` marks the Move tool's auto-managed
    // box, which the user-command take-over must not cancel.
    bool beginTransformImpl(const QString& path, const QString& mode, bool controls = false);

    ToolRegistry registry_;
    ImageView* canvas_ = nullptr;
    std::function<PictureView*()> viewProvider_;
    ToolId active_ = ToolId::Move;
    SelectionMode mode_ = SelectionMode::New;
    SelectionMode dragMode_ = SelectionMode::New;
    MarqueeStyle marqueeStyle_ = MarqueeStyle::Normal;
    double feather_ = 0.0;
    double fixedRatioW_ = 1.0;
    double fixedRatioH_ = 1.0;
    int fixedSizeW_ = 100;
    int fixedSizeH_ = 100;
    int tolerance_ = 32;
    double cropRatio_ = 0.0;
    // docs/03-tools/crop-tool.md: Delete Cropped Pixels is on by default.
    bool cropDeletePixels_ = true;
    // The Crop tool's straighten angle in degrees.
    double cropAngle_ = 0.0;
    // The straighten pivot (image space) for the current angle.
    QPointF cropPivot_;
    // Classic (default) vs Modern crop drag semantics; the grid overlay index.
    bool cropClassicMode_ = true;
    int cropGridOverlay_ = 0;
    // The Crop straighten line tool armed state.
    bool cropStraightenMode_ = false;
    // Move options: Auto-Select 0 Group / 1 Layer, Show Transform Controls,
    // Align To 0 Selection / 1 Canvas.
    int moveAutoSelect_ = 0;
    bool moveShowTransform_ = false;
    int moveAlignTo_ = 0;
    // Eyedropper: Sample Size (odd pixels, 1..101), Sample scope, and ring.
    EyedropperOptions eyedropper_;
    int magneticWidth_ = 10;
    int magneticContrast_ = 10;
    int magneticFrequency_ = 57;
    bool contiguous_ = true;
    bool antiAlias_ = true;
    bool sampleAllLayers_ = true;

    int brushSize_ = 12;
    int brushHardness_ = 100;
    int brushOpacity_ = 100;
    int brushFlow_ = 100;
    QString brushMode_ = QStringLiteral("normal");
    bool autoErase_ = false;
    int spotHealingType_ = 0;
    bool healingAligned_ = true;
    bool patchContentAware_ = false;
    bool patchDestination_ = false;
    bool patchTransparent_ = false;
    bool contentAwareMoveExtend_ = false;
    int contentAwareAdaptation_ = 2;
    int redEyePupil_ = 50;
    int redEyeDarken_ = 50;
    ColorReplaceOptions colorReplace_;
    MixerOptions mixer_;
    QColor mixerReservoir_{Qt::black};
    StampOptions stamp_;
    EraserOptions eraser_;
    BackgroundEraseOptions backgroundErase_;
    MagicEraseOptions magicErase_;
    ArtHistoryOptions artHistory_;
    GradientOptions gradient_;
    // Blur, Sharpen, Smudge.
    std::array<RetouchOptions, 3> retouch_{};
    // Dodge, Burn, Sponge.
    std::array<ToneOptions, 3> tone_{};
    BucketOptions bucket_;
    PenOptions pen_;
    ShapeOptions shape_;
    TypeOptions type_;
    std::function<void(const QPointF&)> canvasSampler_;
    std::array<CloneSource, 5> cloneSources_{};
    int cloneSlot_ = 0;
    int brushRoundness_ = 100;
    int brushTipAngle_ = 0;
    int brushSpacing_ = 25;
    bool brushFlipX_ = false;
    bool brushFlipY_ = false;
    BrushDynamics brushDynamics_;
    QColor foreground_{Qt::black};
    QColor background_{Qt::white};

    bool dragging_ = false;
    bool dragCommitted_ = false;
    bool movingSelection_ = false;
    bool contentMove_ = false;
    bool contentDuplicate_ = false;
    // True while a duplicate content move is showing the pre-cloned pixel
    // preview on the canvas; cleared on release/cancel/tool switch.
    bool contentPreviewActive_ = false;
    bool cursorOverSelection_ = false;
    // The active handler's cursor suffix last applied by refreshCursor.
    QString cursorVariant_;
    // The press point for a selection/content move, owned by the controller
    // because the routing is cross-cutting; a tool handler's own drag anchor is
    // private to that handler.
    QPointF selectionMoveAnchor_;

    bool transformDragging_ = false;
    int transformHandle_ = -1;
    // True while the active session is the Move tool's controls box.
    bool transformControls_ = false;
    int currentNote_ = -1;
};

} // namespace pictura

Q_DECLARE_METATYPE(pictura::ToolId)
