#pragma once

#include "tool_services.h"

#include <QtCore/QPoint>
#include <QtCore/QPointF>
#include <QtCore/QString>
#include <QtCore/Qt>
#include <QtGui/QCursor>
#include <QtWidgets/QMenu>

#include <vector>

namespace pictura {

// One tool's pointer behavior. `onPress` returns whether it consumes the event,
// as do the keyboard gestures `onFinishPath`/`onEscape`/`onDeletePoint`; the
// other hooks are fire-and-forget. The default no-ops let a handler override only
// what it needs.
//
// Tool-specific session commands (crop box, perspective quad, type edit) do not
// live here; a handler exposes those through the ports in `tool_sessions.h`.
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

    // Canvas context menu. `buildContextMenu` returns the menu for `imagePos`
    // (parented to `ctx.canvas()`), or null when the tool has none.
    // `onContextMenu` is the live path: it builds and pops the menu. The test
    // path calls `buildContextMenu` directly and inspects it.
    virtual QMenu* buildContextMenu(ToolServices& ctx, const QPointF& imagePos)
    {
        (void)ctx;
        (void)imagePos;
        return nullptr;
    }
    // Whether this tool offers any canvas context menu (a built menu, or a
    // shared widget menu raised from `onContextMenu`). The router consults this
    // before touching the document, so a menu-less tool's right-click has no
    // effect.
    virtual bool hasCanvasContextMenu() const { return false; }
    virtual void onContextMenu(ToolServices& ctx, const QPointF& imagePos,
                               const QPoint& globalPos)
    {
        if (QMenu* menu = buildContextMenu(ctx, imagePos)) {
            menu->popup(globalPos);
        }
    }

    // The frame dispatches the generic keyboard gestures to whatever tool is
    // active; each returns whether the tool consumed the key. They are named for
    // the gesture, not for one family: `onEscape` cancels the active gesture,
    // `onDeletePoint` removes the selected point, `onFinishPath` closes the path.
    virtual bool onFinishPath() { return false; }
    virtual bool onEscape() { return false; }
    virtual bool onDeletePoint() { return false; }
    // True while a click-driven gesture is open (a live polygonal/magnetic lasso
    // outline), so presses inside a live selection extend it instead of starting
    // a selection move.
    virtual bool gestureInProgress() const { return false; }

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

protected:
    // Register a family option group for `optionsFor<T>`; call it from the
    // handler's constructor once its group pointers are set.
    void addOptions(ToolOptions* group) { optionGroups_.push_back(group); }

private:
    std::vector<ToolOptions*> optionGroups_;
};

} // namespace pictura
