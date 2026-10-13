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

#include "tool_options.h"
#include "tool_registry.h"
#include "tool_services.h"

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
class ToolController : public QObject, public ToolServices {
    Q_OBJECT

public:
    explicit ToolController(QObject* parent = nullptr);

    ToolId activeTool() const { return active_; }
    void setActiveTool(ToolId id);

    // A modal tool session's history changed; the frame re-evaluates the
    // Undo/Redo command enabled state.
    void notifyToolSessionChanged() override;

    // The tool's handler; null for an unimplemented row. The options UI uses
    // this to reach the handler's family options port.
    ToolHandler* handlerFor(ToolId id) const { return registry_.forTool(id); }

    // The tool's typed option group, resolved through its handler's port;
    // null when the tool has no options. The group's setters clamp and notify.
    template <typename T> T* optionsFor(ToolId id) const
    {
        ToolHandler* handler = registry_.forTool(id);
        if (!handler) {
            return nullptr;
        }
        for (ToolOptions* group : handler->optionGroups()) {
            if (auto* typed = dynamic_cast<T*>(group)) {
                return typed;
            }
        }
        return nullptr;
    }

    // Re-apply the active tool's cursor using the live keyboard modifiers
    // (Shift / Alt select the marquee add / remove cursor variants).
    void refreshCursor();
    // Resolve the cursor from an explicit modifier state. The live path calls
    // the no-arg overload; the offscreen self-test drives this one to check the
    // transient Alt eyedropper without a platform keyboard state.
    void refreshCursor(Qt::KeyboardModifiers mods);
    QString cursorIdForModifiersForTest(ToolId id, int mods) const;

    // Restore persisted crop options without re-emitting the save signal (used
    // during startup, before the workspace store exists).
    void applyCropOptions(bool classic, bool deletePixels, double ratio, int grid);
    // The crop cursor zone under an image point (test hook); -1 when not Crop.
    int cropCursorKindForTest(const QPointF& imagePos, int mods);
    // The crop box centre (image space; test hook); empty without a box.
    QPointF cropCenterForTest();
    bool resizeCrop(double width, double height);
    double cropWidth() const;
    double cropHeight() const;
    // Esc / the options bar's Cancel: put the crop box back to the canvas.
    void cancelCrop();
    // True while the Crop tool holds an active (not preview) box; the options
    // bar hides Cancel/Apply/Reset otherwise.
    bool cropActive() const;

    // The Rotate View tool's canvas rotation in degrees clockwise.
    double viewRotation() const;
    void setViewRotation(double degrees);

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

    // ToolServices: the shared services a handler receives.
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
    void selectLayer(const QString& path) override { emit layerSelectionRequested(path); }
    void notifyTextEditing(bool active) override { emit textEditingChanged(active); }
    void emitSelectionCommitted() override;
    void refreshAnnotations() override;
    int currentNote() const override { return currentNote_; }
    void setCurrentNote(int index) override;
    void notifyRulerChanged() override { emit rulerChanged(); }
    void notifyCountChanged() override { emit countChanged(); }
    // The options bar's Clear for the active Color Sampler, Note, or Ruler tool.
    bool clearAnnotations();

    // Guides (tools_guides.cpp): show the document's guides on the canvas, and
    // View > Lock Guides, which stops a press picking one up.
    void refreshGuides();
    void setGuidesLocked(bool on) { guidesLocked_ = on; }
    bool guideDragActiveForTest() const { return guideDrag_ >= 0; }
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
    // Reset the active crop box to the whole canvas and clear its angle.
    bool resetCrop();
    // Perspective Crop: the forced output size (0 = auto) and whether its quad
    // is fully staged.
    void setPerspectiveOutput(int width, int height);
    bool perspectiveQuadComplete() const;
    void setPerspectiveShowGrid(bool on);
    // Arrow-key nudge: the active tool first (Crop nudges its box / pans), then
    // the Move tool's layer nudge. True when a key was consumed.
    bool handleNudgeKey(int key, Qt::KeyboardModifiers mods);
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
    // A tool asks the Layers panel to select the existing layer at `path`.
    void layerSelectionRequested(const QString& path);
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
    SelectionMode dragMode_ = SelectionMode::New;

    // The family option groups. The handlers own them (they hold the
    // shared_ptr the constructor handed out); these are the controller's
    // non-owning references for signal wiring and its own reads.
    PaintOptions* paint_ = nullptr;
    SelectionOptions* selection_ = nullptr;
    CropOptions* crop_ = nullptr;
    CloneSourceOptions* clone_ = nullptr;
    MoveGroup* move_ = nullptr;
    TypeGroup* type_ = nullptr;
    MixerGroup* mixer_ = nullptr;
    EyedropperGroup* eyedropper_ = nullptr;
    PenGroup* pen_ = nullptr;
    ShapeGroup* shape_ = nullptr;

    std::function<void(const QPointF&)> canvasSampler_;
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

    // The guide a press would pick up at `imagePos` (Move tool, or Ctrl with
    // any tool; guides shown and unlocked), or -1.
    int guideAt(const QPointF& imagePos, Qt::KeyboardModifiers mods) const;
    bool beginGuideDrag(const QPointF& imagePos, Qt::KeyboardModifiers mods);
    void dragGuide(const QPointF& imagePos);
    void releaseGuide(const QPointF& imagePos);
    // Show the split cursor over a guide the Move tool would pick up.
    void updateGuideHover(const QPointF& imagePos);
    int guideDrag_ = -1;
    bool guideDragVertical_ = false;
    double guideDragStart_ = 0.0;
    bool guidesLocked_ = false;
    bool guideHover_ = false;
};

} // namespace pictura

Q_DECLARE_METATYPE(pictura::ToolId)
