#pragma once

#include "tool_services.h"

#include <QtCore/QPointF>
#include <QtCore/QRect>
#include <QtCore/QString>
#include <QtCore/Qt>
#include <QtGui/QCursor>

#include <vector>

namespace pictura {

// One tool's pointer behavior. Returning true from `onPress` consumes the
// event; the other hooks are fire-and-forget. The default no-ops let a handler
// override only what it needs.
class ToolHandler {
public:
    virtual ~ToolHandler() = default;

    // The family option groups this handler owns or shares, in registration
    // order. `ToolController::optionsFor<T>` resolves one typed; the group's
    // setters clamp and notify.
    const std::vector<ToolOptions*>& optionGroups() const { return optionGroups_; }

    virtual bool onPress(ToolServices& ctx, const QPointF& imagePos,
                         Qt::KeyboardModifiers mods)
    {
        return false;
    }
    virtual void onMove(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) {}
    // A modifier key changed during a drag without pointer movement, so an
    // Alt/Shift edge is observed even when the pointer is still.
    virtual void onModifiers(ToolServices& ctx, Qt::KeyboardModifiers mods)
    {
        (void)ctx;
        (void)mods;
    }
    virtual void onRelease(ToolServices& ctx, const QPointF& imagePos,
                           Qt::KeyboardModifiers mods)
    {
    }

    // Lifecycle: the controller activates the new handler and deactivates the
    // old one on tool switch and canvas unbind, so handlers hold no stale state.
    virtual void onActivate(ToolServices& ctx) {}
    virtual void onDeactivate(ToolServices& ctx) {}
    // The frame refreshed (a document edit, undo, or tab switch) with this tool
    // active; re-read any document state the tool mirrors on the canvas.
    virtual void onDocumentRefreshed(ToolServices& ctx) {}
    // A tool option changed (e.g. the crop aspect ratio); re-apply it.
    virtual void onOptionsChanged(ToolServices& ctx) {}

    // Tool-specific commands the controller forwards by active tool.
    virtual bool commitPolygonLasso() { return false; }
    virtual bool cancelPolygonLasso() { return false; }
    // Delete: drop the last point of a click-driven lasso outline.
    virtual bool removeLassoPoint() { return false; }
    // True while a click-driven outline is open, so presses inside a live
    // selection extend it instead of starting a selection move.
    virtual bool lassoInProgress() const { return false; }
    virtual bool commitCrop() { return false; }
    // True while an active (not preview, not init) crop box exists; the options
    // bar shows Cancel/Apply/Reset only then.
    virtual bool cropActive() const { return false; }
    // The Crop straighten line tool: arm/disarm the draw-a-horizon gesture.
    virtual void setStraightenMode(bool on) { (void)on; }
    // The options bar's W/H fields: resize the crop box to `width`×`height`
    // (image px), honoring an active ratio. False when there is no crop box.
    virtual bool resizeCrop(double width, double height)
    {
        (void)width;
        (void)height;
        return false;
    }
    virtual double cropWidth() const { return 0.0; }
    virtual double cropHeight() const { return 0.0; }
    // Reset the active crop box to the whole canvas and clear the angle.
    virtual bool resetCropBox() { return false; }
    // The crop box centre (image space); the straighten pivot. Empty without a
    // box.
    virtual QPointF cropCenter() const { return QPointF(); }
    // Arrow-key nudge for the active tool, in image pixels (`dx`/`dy` already
    // scaled by the caller). True when the tool consumed the key.
    virtual bool nudge(int dx, int dy)
    {
        (void)dx;
        (void)dy;
        return false;
    }

    // Pointer-driven cursors: when true, the controller re-resolves the cursor
    // through `hoverCursor` on every press/move/release and on any framework
    // refresh, so a state cursor (resize/rotate/new-crop) is never left stale.
    virtual bool pointerCursor() const { return false; }
    // The cursor for `imagePos` under the live modifiers. Returns false when the
    // tool leaves the cursor to the framework default.
    virtual bool hoverCursor(const ToolServices& ctx, const QPointF& imagePos,
                             Qt::KeyboardModifiers mods, QCursor& out) const
    {
        (void)ctx;
        (void)imagePos;
        (void)mods;
        (void)out;
        return false;
    }
    // A stable id for the pointer's cursor zone (for tests), or -1.
    virtual int pointerCursorKind(const ToolServices& ctx, const QPointF& imagePos,
                                  Qt::KeyboardModifiers mods) const
    {
        (void)ctx;
        (void)imagePos;
        (void)mods;
        return -1;
    }
    // The modal tool session's transient undo/redo. While a tool holds session
    // steps the global Edit Undo/Redo act on the session before the document
    // history; the default no-session returns false.
    virtual bool toolUndo() { return false; }
    virtual bool toolRedo() { return false; }
    virtual bool canToolUndo() const { return false; }
    virtual bool canToolRedo() const { return false; }
    // The Type tools' Commit (Ctrl+Enter) and Cancel (Esc), and whether text is
    // being typed.
    virtual bool commitText() { return false; }
    virtual bool cancelText() { return false; }
    virtual bool textActive() const { return false; }
    // Insert `text` at the open type edit's caret (the Glyphs panel); false
    // when no edit is open.
    virtual bool insertText(const QString& text) { return false; }
    // The options bar's Clear: drop the tool's color samplers, notes, or ruler.
    virtual bool clearAnnotations() { return false; }

    // A suffix for the tool's cursor id while the pointer hovers something the
    // next press acts on specially (the Pen's ".close" over the first anchor);
    // the controller re-applies the cursor when it changes.
    virtual QString cursorVariant() const { return QString(); }

    // The modifiers captured at press for a selection drag; only the marquee
    // handlers keep them (the self-test reads them through the controller).
    virtual Qt::KeyboardModifiers dragMods() const { return Qt::NoModifier; }
    // A marquee drag's pivot-mirror flag after a second Alt edge (tests).
    virtual bool marqueeMirror() const { return false; }
    // The Crop handler's staged rectangle; the controller exposes it.
    virtual bool hasPendingCrop() const { return false; }
    virtual QRect pendingCropRect() const { return QRect(); }
    // Perspective Crop: the forced output size in pixels (0 = auto from the
    // quad) and whether the quad is fully staged (for the Cancel/Apply buttons).
    virtual void setPerspectiveOutput(int width, int height)
    {
        (void)width;
        (void)height;
    }
    virtual bool perspectiveQuadComplete() const { return false; }
    // Perspective Crop: show the inch grid overlay.
    virtual void setPerspectiveShowGrid(bool on) { (void)on; }

protected:
    // Register a family option group for `optionsFor<T>`; call it from the
    // handler's constructor once its group pointers are set.
    void addOptions(ToolOptions* group) { optionGroups_.push_back(group); }

private:
    std::vector<ToolOptions*> optionGroups_;
};

} // namespace pictura
