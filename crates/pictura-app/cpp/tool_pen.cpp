// The Pen tool group: Pen, Freeform Pen, Add Anchor Point, Delete Anchor
// Point, and Convert Point, all editing the document's Work Path
// (`pictura_core::path`, bridged by `cxxqt_object/paths.rs`). The path is
// drawn on the canvas while one of these tools is active. Ported from
// photorust's CanvasView::penPress / penMove / penRelease.

#include "tool_handler.h"

#include "image_view.h"
#include "path_overlay.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paths.cxxqt.h"

#include <QtCore/QLineF>

#include <array>
#include <memory>
#include <vector>

namespace pictura {

namespace {

struct PathPointView {
    QPointF anchor;
    bool hasIn = false;
    QPointF in;
    bool hasOut = false;
    QPointF out;
};

bool readPoint(const PictureView& v, int sp, int pt, PathPointView& out)
{
    const ::rust::Vec<double> p = path_point(v, sp, pt);
    if (p.size() != 9) {
        return false;
    }
    out = {QPointF(p[0], p[1]), p[2] > 0.5, QPointF(p[3], p[4]), p[5] > 0.5,
           QPointF(p[6], p[7])};
    return true;
}

} // namespace

ImageView::PathOverlay workPathOverlay(const PictureView& v, int pointsOf)
{
    ImageView::PathOverlay overlay;
    const int subpaths = path_subpath_count(v);
    for (int sp = 0; sp < subpaths; ++sp) {
        const int count = path_point_count(v, sp);
        QList<PathPointView> points;
        for (int pt = 0; pt < count; ++pt) {
            PathPointView p;
            if (readPoint(v, sp, pt, p)) {
                points.append(p);
            }
        }
        if (points.isEmpty()) {
            continue;
        }
        if (pointsOf == -1 || pointsOf == sp) {
            for (const PathPointView& p : points) {
                overlay.anchors.append(p.anchor);
                if (p.hasIn) {
                    overlay.handles.append(QLineF(p.anchor, p.in));
                }
                if (p.hasOut) {
                    overlay.handles.append(QLineF(p.anchor, p.out));
                }
            }
        }
        overlay.curve.moveTo(points.first().anchor);
        const int segments = path_subpath_closed(v, sp) ? int(points.size())
                                                        : int(points.size()) - 1;
        for (int i = 0; i < segments; ++i) {
            const PathPointView& a = points.at(i);
            const PathPointView& b = points.at((i + 1) % points.size());
            overlay.curve.cubicTo(a.hasOut ? a.out : a.anchor, b.hasIn ? b.in : b.anchor,
                                  b.anchor);
        }
    }
    return overlay;
}

namespace {

class PenToolHandler : public ToolHandler {
public:
    PenToolHandler(ToolId id, std::shared_ptr<PenGroup> pen)
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
        PictureView* v = ctx.view();
        // A drag cut short by a tool switch still lands as one state.
        if (v && v == pressView_) {
            if (gesture_ == Gesture::PlacingHandle) {
                path_commit_anchor(*v);
            } else if (moved_ && (gesture_ == Gesture::ConvertHandle
                                  || gesture_ == Gesture::ConvertNewHandles)) {
                path_commit_convert(*v);
            }
        }
        if (PictureView* press = pressView_) {
            path_gesture_end(*press);
        }
        // Switching tools leaves the subpath being drawn open.
        if (v && v->has_document()) {
            path_finish(*v);
        }
        reset();
        overClose_ = false;
        if (ImageView* canvas = ctx.canvas()) {
            canvas->clearPathOverlay();
        }
        ctx_ = nullptr;
    }

    void onDocumentRefreshed(ToolServices& ctx) override { refreshOverlay(ctx); }
    void onOptionsChanged(ToolServices& ctx) override { refreshOverlay(ctx); }

    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        ctx_ = &ctx;
        PictureView* v = ctx.view();
        reset();
        if (!v || !v->has_document()) {
            return true;
        }
        path_set_layer_target(*v, false);
        path_gesture_begin(*v);
        const double radius = hitRadius(ctx);
        const double x = imagePos.x();
        const double y = imagePos.y();
        pressView_ = v;
        press_ = imagePos;
        switch (id_) {
        case ToolId::Pen:
            pressPen(ctx, *v, imagePos, mods, radius);
            break;
        case ToolId::FreeformPen:
            trail_ = {imagePos};
            gesture_ = Gesture::Freeform;
            break;
        case ToolId::AddAnchorPoint: {
            const ::rust::Vec<double> seg = path_hit_segment(*v, x, y, radius);
            if (seg.size() == 3) {
                path_insert_anchor(*v, int(seg[0]), int(seg[1]), seg[2]);
            }
            break;
        }
        case ToolId::DeleteAnchorPoint: {
            const ::rust::Vec<std::int32_t> anchor = path_hit_anchor(*v, x, y, radius);
            if (anchor.size() == 2) {
                path_delete_anchor(*v, anchor[0], anchor[1]);
            }
            break;
        }
        case ToolId::ConvertPoint: {
            // A handle is the smaller target, so it wins over its anchor.
            const ::rust::Vec<std::int32_t> handle = path_hit_handle(*v, x, y, radius);
            const ::rust::Vec<std::int32_t> anchor = path_hit_anchor(*v, x, y, radius);
            if (handle.size() == 3) {
                target_ = {handle[0], handle[1], handle[2]};
                gesture_ = Gesture::ConvertHandle;
            } else if (anchor.size() == 2) {
                target_ = {anchor[0], anchor[1], 0};
                gesture_ = Gesture::ConvertNewHandles;
            }
            break;
        }
        default:
            break;
        }
        // A live gesture is a session step: refresh Edit Undo's enablement.
        if (gesture_ != Gesture::None) {
            ctx.notifyToolSessionChanged();
        }
        refreshOverlay(ctx);
        return true;
    }

    void onMove(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        hover_ = imagePos;
        hovering_ = true;
        PictureView* v = ctx.view();
        overClose_ = v && v != pressView_ && overFirstAnchor(ctx, *v, imagePos);
        if (!v || v != pressView_) {
            if (id_ == ToolId::Pen && pen_->get().rubberBand) {
                refreshOverlay(ctx);
            }
            return;
        }
        switch (gesture_) {
        case Gesture::PlacingHandle:
            path_update_last_handle(*v, imagePos.x(), imagePos.y(),
                                    mods.testFlag(Qt::AltModifier),
                                    mods.testFlag(Qt::ShiftModifier));
            break;
        case Gesture::ConvertHandle:
            // Convert Point always frees the dragged handle from its partner.
            moved_ = path_move_handle(*v, target_[0], target_[1], target_[2], imagePos.x(),
                                      imagePos.y(), true)
                || moved_;
            break;
        case Gesture::ConvertNewHandles:
            if (moved_ || QLineF(press_, imagePos).length() * zoom(ctx) >= 2.0) {
                moved_ = path_drag_new_handles(*v, target_[0], target_[1], imagePos.x(),
                                               imagePos.y())
                    || moved_;
            }
            break;
        case Gesture::Freeform:
            if ((imagePos - trail_.last()).manhattanLength() >= 1.0) {
                trail_.append(imagePos);
            }
            break;
        case Gesture::None:
            if (!(id_ == ToolId::Pen && pen_->get().rubberBand)) {
                return;
            }
            break;
        }
        refreshOverlay(ctx);
    }

    void onRelease(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        PictureView* v = ctx.view();
        PictureView* press = pressView_;
        const Gesture gesture = gesture_;
        const QList<QPointF> trail = trail_;
        const bool moved = moved_;
        const auto target = target_;
        const bool same = v && v == press;
        reset();
        if (press) {
            path_gesture_end(*press);
        }
        if (!same) {
            return;
        }
        switch (gesture) {
        case Gesture::PlacingHandle:
            path_commit_anchor(*v);
            break;
        case Gesture::ConvertHandle:
            if (moved) {
                path_commit_convert(*v);
            }
            break;
        case Gesture::ConvertNewHandles:
            // A click, not a drag: make the point a plain corner.
            if (moved) {
                path_commit_convert(*v);
            } else {
                path_set_corner(*v, target[0], target[1]);
            }
            break;
        case Gesture::Freeform: {
            std::vector<double> flat;
            flat.reserve(std::size_t(trail.size()) * 2);
            for (const QPointF& p : trail) {
                flat.push_back(p.x());
                flat.push_back(p.y());
            }
            // Ending back near the start closes the loop.
            const bool close = trail.size() > 2
                && QLineF(trail.first(), imagePos).length() * zoom(ctx) <= 8.0;
            path_add_freeform(*v, ::rust::Slice<const double>(flat.data(), flat.size()),
                              pen_->get().curveFit, close);
            break;
        }
        case Gesture::None:
            break;
        }
        overClose_ = v && overFirstAnchor(ctx, *v, imagePos);
        refreshOverlay(ctx);
    }

    // Over the first anchor of the subpath being drawn, the Pen shows its
    // close-path cursor (a small circle beside the nib), as CS6 does.
    QString cursorVariant() const override
    {
        return overClose_ ? QStringLiteral(".close") : QString();
    }

    // Enter and Esc end the subpath being drawn, leaving it open.
    bool commitPolygonLasso() override { return finishDrawing(); }
    bool cancelPolygonLasso() override { return finishDrawing(); }

    // A live gesture is this tool session's one step: Edit Undo cancels the
    // gesture — restoring the pre-press path — before the document history.
    bool toolUndo() override
    {
        if (!canToolUndo()) {
            return false;
        }
        path_gesture_abort(*pressView_);
        reset();
        if (ctx_) {
            refreshOverlay(*ctx_);
        }
        return true;
    }

    bool toolRedo() override { return false; }

    bool canToolUndo() const override
    {
        return pressView_ != nullptr && gesture_ != Gesture::None;
    }

    bool canToolRedo() const override { return false; }

private:
    enum class Gesture { None, PlacingHandle, Freeform, ConvertHandle, ConvertNewHandles };

    void pressPen(ToolServices& ctx, PictureView& v, const QPointF& pos,
                  Qt::KeyboardModifiers mods, double radius)
    {
        // Ctrl-click away ends the subpath, open.
        if (mods.testFlag(Qt::ControlModifier)) {
            path_finish(v);
            return;
        }
        const int editing = path_editing_subpath(v);
        const ::rust::Vec<std::int32_t> anchor = path_hit_anchor(v, pos.x(), pos.y(), radius);
        if (editing >= 0) {
            if (anchor.size() == 2 && anchor[0] == editing && anchor[1] == 0
                && path_close(v)) {
                return;
            }
        } else if (anchor.size() == 2) {
            // An open subpath's endpoint resumes drawing from it.
            if (path_resume(v, anchor[0], anchor[1])) {
                return;
            }
            if (pen_->get().autoAddDelete) {
                path_delete_anchor(v, anchor[0], anchor[1]);
                return;
            }
        } else if (pen_->get().autoAddDelete) {
            const ::rust::Vec<double> seg = path_hit_segment(v, pos.x(), pos.y(), radius);
            if (seg.size() == 3) {
                path_insert_anchor(v, int(seg[0]), int(seg[1]), seg[2]);
                return;
            }
        }
        // Whether the anchor ends up a corner or smooth is the drag's call.
        if (path_append_corner(v, pos.x(), pos.y(), mods.testFlag(Qt::ShiftModifier))) {
            gesture_ = Gesture::PlacingHandle;
        }
    }

    bool overFirstAnchor(ToolServices& ctx, const PictureView& v, const QPointF& pos) const
    {
        if (id_ != ToolId::Pen || !v.has_document()) {
            return false;
        }
        const int editing = path_editing_subpath(v);
        if (editing < 0 || path_point_count(v, editing) < 2) {
            return false;
        }
        const ::rust::Vec<std::int32_t> anchor =
            path_hit_anchor(v, pos.x(), pos.y(), hitRadius(ctx));
        return anchor.size() == 2 && anchor[0] == editing && anchor[1] == 0;
    }

    bool finishDrawing()
    {
        PictureView* v = ctx_ ? ctx_->view() : nullptr;
        if (!v || !v->has_document() || path_editing_subpath(*v) < 0) {
            return false;
        }
        path_finish(*v);
        refreshOverlay(*ctx_);
        return true;
    }

    void reset()
    {
        gesture_ = Gesture::None;
        trail_.clear();
        moved_ = false;
        pressView_ = nullptr;
    }

    static double zoom(ToolServices& ctx)
    {
        return ctx.canvas() ? qMax(ctx.canvas()->zoom(), 1e-6) : 1.0;
    }

    // Eight screen pixels, so the grab area keeps its size at any zoom.
    static double hitRadius(ToolServices& ctx) { return 8.0 / zoom(ctx); }

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
        // The Pen group draws the Work Path, never a shape layer's outline.
        path_set_layer_target(*v, false);
        ImageView::PathOverlay overlay = workPathOverlay(*v, -1);
        const int editing = path_editing_subpath(*v);
        PathPointView last;
        const bool hasLast = editing >= 0
            && readPoint(*v, editing, path_point_count(*v, editing) - 1, last);
        if (hasLast) {
            int before = 0;
            for (int sp = 0; sp <= editing; ++sp) {
                before += path_point_count(*v, sp);
            }
            overlay.activeAnchor = before - 1;
        }
        if (gesture_ == Gesture::Freeform && trail_.size() > 1) {
            overlay.preview.moveTo(trail_.first());
            for (int i = 1; i < trail_.size(); ++i) {
                overlay.preview.lineTo(trail_.at(i));
            }
        } else if (id_ == ToolId::Pen && pen_->get().rubberBand && hasLast && hovering_
                   && gesture_ == Gesture::None) {
            overlay.preview.moveTo(last.anchor);
            overlay.preview.cubicTo(last.hasOut ? last.out : last.anchor, hover_, hover_);
        }
        canvas->setPathOverlay(overlay);
    }

    const ToolId id_;
    ToolServices* ctx_ = nullptr;
    Gesture gesture_ = Gesture::None;
    PictureView* pressView_ = nullptr;
    QPointF press_;
    QPointF hover_;
    bool hovering_ = false;
    bool overClose_ = false;
    bool moved_ = false;
    std::array<int, 3> target_{};
    QList<QPointF> trail_;
    std::shared_ptr<PenGroup> pen_;
};

} // namespace

std::unique_ptr<ToolHandler> makePenToolHandler(ToolId id, std::shared_ptr<PenGroup> pen)
{
    return std::make_unique<PenToolHandler>(id, std::move(pen));
}

} // namespace pictura
