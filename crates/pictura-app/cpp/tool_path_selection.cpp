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
// ponytail: no segment drags or arrow
// nudges, path operations, alignment, or Constrain Path Dragging; the
// selection lives in the tool and is dropped on a tool switch.

#include "tool_handler.h"

#include "image_view.h"
#include "path_overlay.h"
#include "shape_dialogs.h"
#include "tools.h"

#include "api/layers_bridge.h"
#include "api/transform_bridge.h"

#include <QtCore/QLineF>
#include <QtCore/QSet>
#include <QtGui/QGuiApplication>

#include <algorithm>
#include <cmath>
#include <memory>
#include <vector>

namespace pictura {

namespace {

class PathSelectionToolHandler : public ToolHandler, public PathSelectionSession {
public:
    PathSelectionToolHandler(ToolId id, std::shared_ptr<PenGroup> pen)
        : id_(id)
        , pen_(std::move(pen))
    {
        addOptions(pen_.get());
    }

    QList<int> selectedComponents() const override
    {
        QList<int> out = selectedComponents_.values();
        std::sort(out.begin(), out.end());
        return out;
    }

    QList<int> selectedAnchors() const override
    {
        QList<QPair<int, int>> pairs = selectedAnchors_.values();
        std::sort(pairs.begin(), pairs.end());
        QList<int> out;
        for (const auto& pair : pairs) {
            out.append(pair.first);
            out.append(pair.second);
        }
        return out;
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
            path_gesture_end(*v);
        }
        reset();
        clearSelection();
        if (ImageView* canvas = ctx.canvas()) {
            canvas->clearPathOverlay();
        }
        ctx_ = nullptr;
    }

    void onSessionCancelled(ToolServices& ctx) override
    {
        if (PictureView* v = pressView_) {
            path_gesture_abort(*v);
        }
        reset();
        clearSelection();
        if (ImageView* canvas = ctx.canvas()) {
            canvas->clearPathOverlay();
        }
    }

    // Undo or another tool can renumber or drop subpaths under the selection.
    void onDocumentRefreshed(ToolServices& ctx) override
    {
        PictureView* v = ctx.view();
        if (v && v->has_document()) {
            path_set_layer_target(*v, true);
        }
        bool stale = v && selectionView_ && (selectionView_ != v || selectionKey_ != path_selection_target_key(*v));
        if (v && selectionView_ == v && !pressView_
            && !static_cast<ToolController&>(ctx).pathTransformCommitInProgress()) {
            const auto outline = path_target_outline(*v);
            // The outline encoding starts each subpath with a NaN marker.
            stale = stale || !std::equal(outline.begin(), outline.end(),
                                         selectionOutline_.begin(), selectionOutline_.end(),
                                         [](double a, double b) {
                                             return a == b || (std::isnan(a) && std::isnan(b));
                                         });
            if (v->history_index() > selectionHistory_) {
                const QString label = v->history_label(v->history_index());
                stale = stale || label == QLatin1String("New Path") || label == QLatin1String("Save Path")
                    || label == QLatin1String("Duplicate Path") || label == QLatin1String("Delete Path");
            }
        }
        if (stale && path_transform_active(*v)) {
            v->cancel_transform();
        }
        if (stale && !v->transform_session_active()) {
            static_cast<ToolController&>(ctx).resetTransformUi();
        }
        if (!v || !v->has_document() || stale || (selectionView_ && v->history_index() < selectionHistory_) || selected_ >= path_subpath_count(*v)) {
            clearSelection();
        } else if (selected_ >= 0 && selectedPoint_ >= path_point_count(*v, selected_)) {
            selectedPoint_ = -1;
        }
        if (v && selectionView_) {
            rememberSelection(*v);
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
        if (selectionView_ && (selectionView_ != v || selectionKey_ != path_selection_target_key(*v))) {
            clearSelection();
        }
        selectionView_ = v;
        selectionKey_ = path_selection_target_key(*v);
        path_gesture_begin(*v);
        pressView_ = v;
        press_ = last_ = imagePos;
        const bool alt = mods.testFlag(Qt::AltModifier);
        if (id_ == ToolId::DirectSelection && !alt) {
            pressDirect(ctx, *v, imagePos, mods);
        } else {
            const int hit = path_hit_subpath(*v, imagePos.x(), imagePos.y(), hitRadius(ctx));
            const bool shift = mods.testFlag(Qt::ShiftModifier);
            if (!shift && hit >= 0 && selectedComponents_.contains(hit)) {
                replaceOnClick_ = true;
            } else if (!shift) {
                clearSelection();
            }
            selected_ = hit;
            if (selected_ >= 0) {
                if (shift && selectedComponents_.contains(selected_)) {
                    selectedComponents_.remove(selected_);
                    selected_ = selectedComponents_.isEmpty() ? -1 : *selectedComponents_.constBegin();
                    refreshOverlay(ctx);
                    return true;
                } else {
                    selectedComponents_.insert(selected_);
                }
            }
            if (selected_ >= 0) {
                whole_ = true;
                gesture_ = Gesture::Component;
                copy_ = alt && id_ == ToolId::PathSelection;
            } else if (hit < 0 && !shift) {
                gesture_ = Gesture::Marquee;
            }
        }
        if (!selectedAnchors_.isEmpty() || !selectedComponents_.isEmpty()) {
            selectionView_ = v;
            selectionKey_ = path_selection_target_key(*v);
            rememberSelection(*v);
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
                QSet<int> copies;
                for (int component : selectedComponents()) {
                    const int copy = path_duplicate_subpath(*v, component);
                    if (copy < 0) {
                        path_gesture_abort(*v);
                        reset();
                        clearSelection();
                        refreshOverlay(ctx);
                        return;
                    }
                    copies.insert(copy);
                    if (component == selected_) {
                        selected_ = copy;
                    }
                }
                selectedComponents_ = copies;
            }
            for (int component : selectedComponents_) {
                moved_ = path_move_subpath(*v, component, imagePos.x() - last_.x(),
                                           imagePos.y() - last_.y()) || moved_;
            }
            break;
        case Gesture::Anchor:
            for (const auto& item : selectedAnchors_) {
                const auto point = path_point(*v, item.first, item.second);
                if (point.size() == 9) {
                    moved_ = path_move_anchor(*v, item.first, item.second,
                                              point[0] + imagePos.x() - last_.x(),
                                              point[1] + imagePos.y() - last_.y()) || moved_;
                }
            }
            break;
        case Gesture::Handle:
            moved_ = path_move_handle(*v, selected_, handlePoint_, handleSide_, imagePos.x(),
                                      imagePos.y(), mods.testFlag(Qt::AltModifier))
                || moved_;
            break;
        case Gesture::Marquee:
        case Gesture::None:
            break;
        }
        last_ = imagePos;
        refreshOverlay(ctx);
    }

    void onRelease(ToolServices& ctx, const QPointF& position, Qt::KeyboardModifiers) override
    {
        if (PictureView* v = ctx.view(); v && v == pressView_) {
            if (gesture_ == Gesture::Marquee && QLineF(press_, position).length() * zoom(ctx) >= 2.0) {
                selectMarquee(*v, QRectF(press_, position).normalized());
                selectionView_ = v;
                selectionKey_ = path_selection_target_key(*v);
                rememberSelection(*v);
            }
            if (replaceOnClick_ && !moved_) {
                const int component = selected_;
                const int point = selectedPoint_;
                const bool whole = whole_;
                clearSelection();
                selected_ = component;
                selectedPoint_ = point;
                whole_ = whole;
                if (whole) selectedComponents_.insert(component);
                else selectedAnchors_.insert(qMakePair(component, point));
                selectionView_ = v;
                selectionKey_ = path_selection_target_key(*v);
            }
            commitDrag(*v);
            path_gesture_end(*v);
            if (selectionView_) rememberSelection(*v);
        }
        reset();
        refreshOverlay(ctx);
    }

    // Delete / Backspace removes a component selected whole.
    bool onDeletePoint() override
    {
        PictureView* v = ctx_ ? ctx_->view() : nullptr;
        if (!v || !v->has_document() || !whole_ || gesture_ != Gesture::None) {
            return false;
        }
        const QList<int> components = selectedComponents();
        if (components.isEmpty()
            || !path_remove_subpaths(*v, ::rust::Slice<const std::int32_t>(components.constData(), components.size()))) {
            return false;
        }
        clearSelection();
        refreshOverlay(*ctx_);
        return true;
    }

private:
    enum class Gesture { None, Component, Anchor, Handle, Marquee };

    // A handle on the selected component wins over an anchor (the smaller
    // target); an anchor picks its component and itself; a segment picks
    // just the component; empty canvas clears the selection.
    void pressDirect(ToolServices& ctx, PictureView& v, const QPointF& pos, Qt::KeyboardModifiers mods)
    {
        const double radius = hitRadius(ctx);
        const ::rust::Vec<std::int32_t> handle = path_hit_handle(v, pos.x(), pos.y(), radius);
        if (handle.size() == 3 && handle[0] == selected_) {
            handlePoint_ = handle[1];
            handleSide_ = handle[2];
            gesture_ = Gesture::Handle;
            return;
        }
        const bool shift = mods.testFlag(Qt::ShiftModifier);
        const ::rust::Vec<std::int32_t> anchor = path_hit_anchor(v, pos.x(), pos.y(), radius);
        if (!shift && anchor.size() == 2
            && selectedAnchors_.contains(qMakePair(int(anchor[0]), int(anchor[1])))) {
            replaceOnClick_ = true;
        } else if (!shift) {
            clearSelection();
        }
        if (anchor.size() == 2) {
            selected_ = anchor[0];
            selectedPoint_ = anchor[1];
            const QPair<int, int> item(selected_, selectedPoint_);
            if (shift && selectedAnchors_.contains(item)) {
                selectedAnchors_.remove(item);
                if (selectedAnchors_.isEmpty()) {
                    selected_ = selectedPoint_ = -1;
                } else {
                    const auto remaining = *selectedAnchors_.constBegin();
                    selected_ = remaining.first;
                    selectedPoint_ = remaining.second;
                }
            } else {
                selectedAnchors_.insert(item);
                gesture_ = Gesture::Anchor;
            }
            return;
        }
        if (!shift) {
            gesture_ = Gesture::Marquee;
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
        case Gesture::Marquee:
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
        replaceOnClick_ = false;
        pressView_ = nullptr;
    }

    void clearSelection()
    {
        selected_ = -1;
        selectedPoint_ = -1;
        whole_ = false;
        selectedAnchors_.clear();
        selectedComponents_.clear();
        selectionView_ = nullptr;
        selectionKey_.clear();
        selectionOutline_.clear();
    }

    void rememberSelection(const PictureView& view)
    {
        selectionHistory_ = view.history_index();
        const auto outline = path_target_outline(view);
        selectionOutline_.assign(outline.begin(), outline.end());
    }

    void selectMarquee(const PictureView& v, const QRectF& box)
    {
        for (int sp = 0; sp < path_subpath_count(v); ++sp) {
            if (id_ == ToolId::PathSelection) {
                const auto b = path_subpath_bounds(v, sp);
                if (b.size() == 4 && box.contains(QPointF(b[0], b[1]))
                    && box.contains(QPointF(b[2], b[3]))) {
                    selectedComponents_.insert(sp);
                    selected_ = sp;
                    whole_ = true;
                }
            } else {
                for (int pt = 0; pt < path_point_count(v, sp); ++pt) {
                    const auto p = path_point(v, sp, pt);
                    if (p.size() == 9 && box.contains(QPointF(p[0], p[1]))) {
                        selectedAnchors_.insert(qMakePair(sp, pt));
                        selected_ = sp;
                        selectedPoint_ = pt;
                    }
                }
            }
        }
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
        const bool multi = !selectedAnchors_.isEmpty() || !selectedComponents_.isEmpty();
        bool spansComponents = selectedComponents_.size() > 1;
        for (const auto& item : selectedAnchors_) {
            spansComponents = spansComponents || item.first != selected_;
        }
        const int pointsOf = spansComponents ? -1 : (selected_ >= 0 ? selected_ : (path_target_is_layer(*v) ? -1 : -2));
        ImageView::PathOverlay overlay = workPathOverlay(*v, pointsOf);
        if (multi) {
            int index = 0;
            for (int sp = 0; sp < path_subpath_count(*v); ++sp) {
                if (!spansComponents && sp != pointsOf) {
                    continue;
                }
                for (int pt = 0; pt < path_point_count(*v, sp); ++pt, ++index) {
                    if (selectedComponents_.contains(sp) || selectedAnchors_.contains(qMakePair(sp, pt))) {
                        overlay.selectedAnchors.append(index);
                    }
                }
            }
            if (!spansComponents && selectedComponents_.size() == 1) {
                overlay.anchorsSolid = true;
            }
            if (!spansComponents && selectedAnchors_.size() == 1) {
                overlay.activeAnchor = selectedPoint_;
            }
        } else {
            overlay.anchorsSolid = whole_;
            overlay.activeAnchor = selectedPoint_;
        }
        if (id_ == ToolId::PathSelection) {
            overlay.handles.clear();
            if (pen_->get().showBoundingBox && !path_transform_active(*v)) {
                bool first = true;
                for (int component : selectedComponents_) {
                    const ::rust::Vec<double> b = path_subpath_bounds(*v, component);
                    if (b.size() != 4) {
                        continue;
                    }
                    const QRectF bounds(QPointF(b[0], b[1]), QPointF(b[2], b[3]));
                    overlay.bounds = first ? bounds : overlay.bounds.united(bounds);
                    first = false;
                }
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
    bool replaceOnClick_ = false;
    PictureView* selectionView_ = nullptr;
    QString selectionKey_;
    std::vector<double> selectionOutline_;
    int selectionHistory_ = -1;
    QSet<int> selectedComponents_;
    QSet<QPair<int, int>> selectedAnchors_;
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
