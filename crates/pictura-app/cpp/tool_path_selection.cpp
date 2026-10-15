// The path selection tools, editing the active shape layer's outline or else
// the document's Work Path (`pictura_core::path`, bridged by
// `cxxqt_object/paths.rs`): Path Selection
// picks a whole subpath (a path component) and drags it, Alt-drag dragging a
// copy; Direct Selection drags one anchor or direction handle, and Alt-click
// picks the whole component. Delete removes a whole-selected component. The
// path is drawn on the canvas while one of these tools is active. Ported from
// photorust's CanvasView::pathSelectPress / pathSelectMove / pathSelectRelease.
// Dragging an anchor or handle of a live shape first asks to turn it into a
// regular path (Photoshop CC's prompt); moving the whole shape keeps it live.
//
// ponytail: no segment drags, marquee, Shift-click multi-selection, arrow
// nudges, path operations, alignment, or Constrain Path Dragging; the
// selection lives in the tool and is dropped on a tool switch.

#include "tool_handler.h"

#include "image_view.h"
#include "path_overlay.h"
#include "shape_dialogs.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paths.cxxqt.h"

#include <QtCore/QLineF>
#include <QtGui/QGuiApplication>

#include <memory>

namespace pictura {

namespace {

class PathSelectionToolHandler : public ToolHandler {
public:
    PathSelectionToolHandler(ToolId id, std::shared_ptr<PenGroup> pen)
        : id_(id)
        , pen_(std::move(pen))
    {
        addOptions(pen_.get());
    }

    void onActivate(ToolServices& ctx) override
    {
        ctx_ = &ctx;
        refreshOverlay(ctx);
    }

    void onDeactivate(ToolServices& ctx) override
    {
        // A drag cut short by a tool switch still lands as one state.
        if (PictureView* v = ctx.view(); v && v == pressView_) {
            commitDrag(*v);
        }
        reset();
        clearSelection();
        if (ImageView* canvas = ctx.canvas()) {
            canvas->clearPathOverlay();
        }
        ctx_ = nullptr;
    }

    // Undo or another tool can renumber or drop subpaths under the selection.
    void onDocumentRefreshed(ToolServices& ctx) override
    {
        PictureView* v = ctx.view();
        if (v && v->has_document()) {
            path_set_layer_target(*v, true);
        }
        if (!v || !v->has_document() || selected_ >= path_subpath_count(*v)) {
            clearSelection();
        } else if (selected_ >= 0 && selectedPoint_ >= path_point_count(*v, selected_)) {
            selectedPoint_ = -1;
        }
        refreshOverlay(ctx);
    }

    void onOptionsChanged(ToolServices& ctx) override { refreshOverlay(ctx); }

    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        ctx_ = &ctx;
        PictureView* v = ctx.view();
        reset();
        if (!v || !v->has_document()) {
            return true;
        }
        path_set_layer_target(*v, true);
        pressView_ = v;
        press_ = last_ = imagePos;
        const bool alt = mods.testFlag(Qt::AltModifier);
        if (id_ == ToolId::DirectSelection && !alt) {
            pressDirect(ctx, *v, imagePos);
        } else {
            clearSelection();
            selected_ = path_hit_subpath(*v, imagePos.x(), imagePos.y(), hitRadius(ctx));
            if (selected_ >= 0) {
                whole_ = true;
                gesture_ = Gesture::Component;
                copy_ = alt && id_ == ToolId::PathSelection;
            }
        }
        refreshOverlay(ctx);
        return true;
    }

    void onMove(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        PictureView* v = ctx.view();
        if (!v || v != pressView_ || gesture_ == Gesture::None) {
            return;
        }
        // A press that wobbles under two screen pixels stays a click.
        if (!moved_ && QLineF(press_, imagePos).length() * zoom(ctx) < 2.0) {
            return;
        }
        if (!moved_ && (gesture_ == Gesture::Anchor || gesture_ == Gesture::Handle)
            && path_target_is_live_shape(*v)) {
            // The prompt is modal: when it closes, the button may be up and
            // the release long gone, so the drag only goes on while held.
            bool asked = false;
            if (!confirmLiveShapeToPath(ctx.canvas(), &asked)) {
                reset();
                refreshOverlay(ctx);
                return;
            }
            path_convert_live_shape(*v);
            if (asked && !QGuiApplication::mouseButtons().testFlag(Qt::LeftButton)) {
                reset();
                refreshOverlay(ctx);
                return;
            }
        }
        switch (gesture_) {
        case Gesture::Component:
            if (!moved_ && copy_) {
                const int copy = path_duplicate_subpath(*v, selected_);
                if (copy < 0) {
                    return;
                }
                selected_ = copy;
            }
            moved_ = path_move_subpath(*v, selected_, imagePos.x() - last_.x(),
                                       imagePos.y() - last_.y())
                || moved_;
            break;
        case Gesture::Anchor:
            moved_ = path_move_anchor(*v, selected_, selectedPoint_, imagePos.x(), imagePos.y())
                || moved_;
            break;
        case Gesture::Handle:
            moved_ = path_move_handle(*v, selected_, handlePoint_, handleSide_, imagePos.x(),
                                      imagePos.y(), mods.testFlag(Qt::AltModifier))
                || moved_;
            break;
        case Gesture::None:
            break;
        }
        last_ = imagePos;
        refreshOverlay(ctx);
    }

    void onRelease(ToolServices& ctx, const QPointF&, Qt::KeyboardModifiers) override
    {
        if (PictureView* v = ctx.view(); v && v == pressView_) {
            commitDrag(*v);
        }
        reset();
        refreshOverlay(ctx);
    }

    // Delete / Backspace removes a component selected whole.
    bool onDeletePoint() override
    {
        PictureView* v = ctx_ ? ctx_->view() : nullptr;
        if (!v || !v->has_document() || !whole_ || gesture_ != Gesture::None
            || !path_remove_subpath(*v, selected_)) {
            return false;
        }
        clearSelection();
        refreshOverlay(*ctx_);
        return true;
    }

private:
    enum class Gesture { None, Component, Anchor, Handle };

    // A handle on the selected component wins over an anchor (the smaller
    // target); an anchor picks its component and itself; a segment picks
    // just the component; empty canvas clears the selection.
    void pressDirect(ToolServices& ctx, PictureView& v, const QPointF& pos)
    {
        const double radius = hitRadius(ctx);
        const ::rust::Vec<std::int32_t> handle = path_hit_handle(v, pos.x(), pos.y(), radius);
        if (handle.size() == 3 && handle[0] == selected_) {
            handlePoint_ = handle[1];
            handleSide_ = handle[2];
            gesture_ = Gesture::Handle;
            return;
        }
        clearSelection();
        const ::rust::Vec<std::int32_t> anchor = path_hit_anchor(v, pos.x(), pos.y(), radius);
        if (anchor.size() == 2) {
            selected_ = anchor[0];
            selectedPoint_ = anchor[1];
            gesture_ = Gesture::Anchor;
            return;
        }
        const ::rust::Vec<double> seg = path_hit_segment(v, pos.x(), pos.y(), radius);
        if (seg.size() == 3) {
            selected_ = int(seg[0]);
        }
    }

    void commitDrag(PictureView& v)
    {
        if (!moved_) {
            return;
        }
        switch (gesture_) {
        case Gesture::Component:
            path_commit_drag(v, copy_ ? "Duplicate Path Component" : "Drag Path");
            break;
        case Gesture::Anchor:
            path_commit_drag(v, "Drag Anchor Point");
            break;
        case Gesture::Handle:
            path_commit_drag(v, "Drag Direction Point");
            break;
        case Gesture::None:
            break;
        }
        moved_ = false;
    }

    void reset()
    {
        gesture_ = Gesture::None;
        moved_ = false;
        copy_ = false;
        pressView_ = nullptr;
    }

    void clearSelection()
    {
        selected_ = -1;
        selectedPoint_ = -1;
        whole_ = false;
    }

    static double zoom(ToolServices& ctx)
    {
        return ctx.canvas() ? qMax(ctx.canvas()->zoom(), 1e-6) : 1.0;
    }

    // Eight screen pixels, so the grab area keeps its size at any zoom.
    static double hitRadius(ToolServices& ctx) { return 8.0 / zoom(ctx); }

    // The whole path's outline; the selected component's anchors (solid when
    // selected whole) and, for Direct Selection, its handles.
    void refreshOverlay(ToolServices& ctx)
    {
        ImageView* canvas = ctx.canvas();
        if (!canvas) {
            return;
        }
        PictureView* v = ctx.view();
        if (!v || !v->has_document()) {
            canvas->setPathOverlay({});
            return;
        }
        path_set_layer_target(*v, true);
        // A shape layer's outline always shows its anchors, as CC draws it.
        const int pointsOf = selected_ >= 0 ? selected_ : (path_target_is_layer(*v) ? -1 : -2);
        ImageView::PathOverlay overlay = workPathOverlay(*v, pointsOf);
        overlay.anchorsSolid = whole_;
        overlay.activeAnchor = selectedPoint_;
        if (id_ == ToolId::PathSelection) {
            overlay.handles.clear();
            const ::rust::Vec<double> b = path_subpath_bounds(*v, selected_);
            if (pen_->get().showBoundingBox && b.size() == 4) {
                overlay.bounds = QRectF(QPointF(b[0], b[1]), QPointF(b[2], b[3]));
            }
        }
        canvas->setPathOverlay(overlay);
    }

    const ToolId id_;
    ToolServices* ctx_ = nullptr;
    Gesture gesture_ = Gesture::None;
    PictureView* pressView_ = nullptr;
    QPointF press_;
    QPointF last_;
    bool moved_ = false;
    bool copy_ = false;
    int selected_ = -1;
    int selectedPoint_ = -1;
    bool whole_ = false;
    int handlePoint_ = -1;
    int handleSide_ = 0;
    std::shared_ptr<PenGroup> pen_;
};

} // namespace

std::unique_ptr<ToolHandler> makePathSelectionToolHandler(ToolId id,
                                                          std::shared_ptr<PenGroup> pen)
{
    return std::make_unique<PathSelectionToolHandler>(id, std::move(pen));
}

} // namespace pictura
