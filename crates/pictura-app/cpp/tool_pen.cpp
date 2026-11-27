// The Pen tool group: Pen, Freeform Pen, Add Anchor Point, Delete Anchor
// Point, and Convert Point, all editing the document's Work Path
// (`pictura_core::path`, bridged by `cxxqt_object/paths.rs`). The path is
// drawn on the canvas while one of these tools is active. Ported from
// photorust's CanvasView::penPress / penMove / penRelease.

#include "tool_handler.h"

#include "image_view.h"
#include "paint_options.h"
#include "path_overlay.h"
#include "tools.h"

#include "api/layers_bridge.h"
#include "api/paint_bridge.h"

#include <QtCore/QLineF>
#include <QtCore/QElapsedTimer>
#include <QtCore/QPointer>
#include <QtWidgets/QApplication>

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

PathPointView pointFromFields(const double* fields)
{
    return {QPointF(fields[0], fields[1]), fields[2] > 0.5, QPointF(fields[3], fields[4]),
            fields[5] > 0.5, QPointF(fields[6], fields[7])};
}

bool readPoint(const PictureView& v, int sp, int pt, PathPointView& out)
{
    const ::rust::Vec<double> p = path_point(v, sp, pt);
    if (p.size() != 9) {
        return false;
    }
    out = pointFromFields(p.data());
    return true;
}

} // namespace

ImageView::PathOverlay workPathOverlay(const PictureView& v, int pointsOf)
{
    ImageView::PathOverlay overlay;
    const auto fields = path_overlay_points(v);
    size_t field = 0;
    for (int sp = 0; field < fields.size(); ++sp) {
        const auto count = qsizetype(fields[field++]);
        const bool closed = fields[field++] != 0.0;
        QList<PathPointView> points;
        points.reserve(count);
        for (qsizetype pt = 0; pt < count; ++pt, field += 9) {
            points.append(pointFromFields(fields.data() + field));
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
        const qsizetype segments = closed ? count : count - 1;
        for (qsizetype i = 0; i < segments; ++i) {
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
    PenToolHandler(ToolId id, std::shared_ptr<PenGroup> pen, std::shared_ptr<PaintOptions> paint)
        : id_(id)
        , pen_(std::move(pen))
        , paint_(std::move(paint))
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
        if (magneticActive_) cancelMagnetic(ctx);
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
        // Switching tools leaves the subpath being drawn open. Path mode does
        // not save here: the user may switch to an anchor tool to keep editing
        // the same Work Path, and undo must step back the last anchor.
        if (v && v->has_document()) {
            if (path_editing_subpath(*v) >= 0 && pen_->get().mode != 1) {
                finishPath(ctx, *v);
            } else {
                path_finish(*v);
            }
        }
        reset();
        overClose_ = false;
        if (ImageView* canvas = ctx.canvas()) {
            canvas->clearPathOverlay();
        }
        ctx_ = nullptr;
    }

    void onSessionCancelled(ToolServices& ctx) override
    {
        if (magneticActive_) cancelMagnetic(ctx);
        if (PictureView* press = pressView_) {
            path_gesture_abort(*press);
        }
        if (PictureView* v = ctx.view()) {
            path_gesture_end(*v);
        }
        reset();
        overClose_ = false;
        if (ImageView* canvas = ctx.canvas()) {
            canvas->clearPathOverlay();
        }
        ctx_ = nullptr;
    }

    void onDocumentRefreshed(ToolServices& ctx) override
    {
        if (magneticActive_ && magneticView_ != ctx.view()) cancelMagnetic(ctx);
        refreshOverlay(ctx);
    }
    void onOptionsChanged(ToolServices& ctx) override
    {
        if (magneticActive_ && !pen_->get().magnetic) cancelMagnetic(ctx);
        refreshOverlay(ctx);
    }

    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        ctx_ = &ctx;
        if (id_ == ToolId::FreeformPen && (magneticActive_ || pen_->get().magnetic)) {
            return pressMagnetic(ctx, imagePos, mods);
        }
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
        if (magneticActive_) {
            moveMagnetic(ctx, imagePos, mods);
            return;
        }
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
        if (magneticActive_) {
            if (magneticAltDrag_) fastenMagnetic();
            magneticPressed_ = magneticAltDrag_ = false;
            refreshOverlay(ctx);
            return;
        }
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
            const ::rust::Slice<const double> points(flat.data(), flat.size());
            if (isPenDrawingTool() && pen_->get().mode == 1) {
                path_add_freeform_and_save(*v, points, pen_->get().curveFit, close,
                                           paths_next_name(*v));
            } else if (path_add_freeform(*v, points, pen_->get().curveFit, close)) {
                landDrawnPath(ctx, *v);
            }
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
    bool onFinishPath() override
    {
        if (magneticActive_ && ctx_) { finishMagnetic(*ctx_, false, false); return true; }
        return finishDrawing();
    }
    bool onEscape() override
    {
        if (magneticActive_ && ctx_) { cancelMagnetic(*ctx_); return true; }
        return finishDrawing();
    }
    bool onDeletePoint() override
    {
        if (!magneticActive_ || !ctx_) return false;
        if (magneticFastenings_.size() <= 1) { cancelMagnetic(*ctx_); return true; }
        magneticFastenings_.removeLast();
        magneticPath_.resize(magneticFastenings_.last() + 1);
        magneticPreview_.clear();
        refreshOverlay(*ctx_);
        return true;
    }
    bool gestureInProgress() const override { return magneticActive_; }

    // A live gesture is this tool session's one step: Edit Undo cancels the
    // gesture — restoring the pre-press path — before the document history.
    bool toolUndo() override
    {
        if (!canToolUndo()) {
            return false;
        }
        if (magneticActive_ && ctx_) { cancelMagnetic(*ctx_); return true; }
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
        return magneticActive_ || (pressView_ != nullptr && gesture_ != Gesture::None);
    }

    bool canToolRedo() const override { return false; }

private:
    enum class Gesture { None, PlacingHandle, Freeform, ConvertHandle, ConvertNewHandles };

    bool pressMagnetic(ToolServices& ctx, const QPointF& pos, Qt::KeyboardModifiers mods)
    {
        PictureView* v = ctx.view();
        if (!v || !v->has_document()) return true;
        if (!magneticActive_) {
            if (!magnetic_pen_begin(*v, pen_->get().magneticContrast)) return true;
            magneticView_ = v;
            magneticActive_ = true;
            magneticPath_ = {pos};
            magneticFastenings_ = {0};
            magneticLastPress_ = pos;
            magneticClock_.start();
            magneticPressed_ = true;
            ctx.notifyToolSessionChanged();
            refreshOverlay(ctx);
            return true;
        }
        if (v != magneticView_) { cancelMagnetic(ctx); return true; }
        const bool doubleClick = magneticClock_.isValid()
            && magneticClock_.elapsed() <= QApplication::doubleClickInterval()
            && QLineF(pos, magneticLastPress_).length() <= hitRadius(ctx);
        if (doubleClick) {
            finishMagnetic(ctx, true, mods.testFlag(Qt::AltModifier));
            return true;
        }
        if (mods.testFlag(Qt::AltModifier)) {
            magneticPreview_ = {pos};
        } else {
            traceMagnetic(ctx, pos);
        }
        fastenMagnetic();
        magneticLastPress_ = pos;
        magneticClock_.restart();
        magneticPressed_ = true;
        magneticAltDrag_ = false;
        refreshOverlay(ctx);
        return true;
    }

    void traceMagnetic(ToolServices& ctx, const QPointF& target)
    {
        magneticPreview_.clear();
        if (!magneticView_ || magneticPath_.isEmpty()) return;
        const auto options = pen_->get();
        int width = options.magneticWidth;
        if (options.magneticPressure && ctx.hasTabletPressure()) {
            width = qMax(1, qRound(width * (1.0 - 0.9 * qBound(0.0, ctx.pressure(), 1.0))));
        }
        const QPoint from = magneticPath_.last().toPoint();
        const QPoint to = target.toPoint();
        const auto wire = magnetic_trace(*magneticView_, from.x(), from.y(), to.x(), to.y(), width);
        for (std::size_t i = 2; i + 1 < wire.size(); i += 2) {
            magneticPreview_.append(QPointF(wire[i], wire[i + 1]));
        }
    }

    void moveMagnetic(ToolServices& ctx, const QPointF& pos, Qt::KeyboardModifiers mods)
    {
        if (magneticPressed_ && mods.testFlag(Qt::AltModifier)) {
            magneticAltDrag_ = true;
            magneticPreview_.append(pos);
        } else {
            traceMagnetic(ctx, pos);
            const int frequency = pen_->get().magneticFrequency;
            if (frequency > 0 && magneticPreview_.size() >= qMax(8, 108 - frequency)) {
                fastenMagnetic();
            }
        }
        refreshOverlay(ctx);
    }

    void fastenMagnetic()
    {
        magneticPath_ += magneticPreview_;
        magneticPreview_.clear();
        if (!magneticPath_.isEmpty() && magneticFastenings_.last() != magneticPath_.size() - 1) {
            magneticFastenings_.append(int(magneticPath_.size()) - 1);
        }
    }

    void cancelMagnetic(ToolServices& ctx)
    {
        if (magneticView_) magnetic_end(*magneticView_);
        magneticActive_ = magneticPressed_ = magneticAltDrag_ = false;
        magneticView_ = nullptr;
        magneticPath_.clear();
        magneticPreview_.clear();
        magneticFastenings_.clear();
        magneticClock_.invalidate();
        ctx.notifyToolSessionChanged();
        refreshOverlay(ctx);
    }

    void finishMagnetic(ToolServices& ctx, bool close, bool straight)
    {
        PictureView* v = magneticView_;
        if (!v || v != ctx.view()) { cancelMagnetic(ctx); return; }
        fastenMagnetic();
        if (close && !straight && magneticPath_.size() > 1) {
            traceMagnetic(ctx, magneticPath_.first());
            magneticPath_ += magneticPreview_;
            magneticPreview_.clear();
        }
        std::vector<double> flat;
        for (const QPointF& p : magneticPath_) { flat.push_back(p.x()); flat.push_back(p.y()); }
        std::vector<std::int32_t> fastenings;
        for (int i : magneticFastenings_) fastenings.push_back(i);
        magnetic_end(*v);
        magneticView_ = nullptr;
        magneticActive_ = false;
        path_set_layer_target(*v, false);
        path_gesture_begin(*v);
        const auto mode = pen_->get().mode;
        const int active = paths_active(*v);
        const bool workTarget = active == -1 || active == -2;
        const bool added = path_add_magnetic(*v, {flat.data(), flat.size()},
            {fastenings.data(), fastenings.size()}, close, mode == 1 || !workTarget,
            paths_next_name(*v));
        bool landed = added;
        if (added && workTarget && mode == 0) {
            const QString created = shape_add_work_path_layer(*v, ctx.foreground().rgba(), "Freeform Pen");
            landed = !created.isEmpty();
            if (landed) ctx.notifyLayerCreated(created);
        } else if (added && workTarget && mode == 2) {
            landed = paths_stroke_magnetic(*v, ctx.foreground().rgba(), paint_ ? paint_->size() : 5,
                paint_ ? paint_->hardness() : 100, paint_ ? paint_->opacity() : 100,
                paint_ ? paint_->flow() : 100, paint_ ? paint_->spacing() : 25);
        }
        if (!landed) path_gesture_abort(*v);
        path_gesture_end(*v);
        cancelMagnetic(ctx);
    }

    void pressPen(ToolServices& ctx, PictureView& v, const QPointF& pos,
                  Qt::KeyboardModifiers mods, double radius)
    {
        // Ctrl-click away ends the subpath, open.
        if (mods.testFlag(Qt::ControlModifier)) {
            if (path_editing_subpath(v) >= 0) {
                finishPath(ctx, v);
            } else {
                path_finish(v);
            }
            return;
        }
        const int editing = path_editing_subpath(v);
        const ::rust::Vec<std::int32_t> anchor = path_hit_anchor(v, pos.x(), pos.y(), radius);
        if (editing >= 0) {
            if (anchor.size() == 2 && anchor[0] == editing && anchor[1] == 0) {
                const bool closed = isPenDrawingTool() && pen_->get().mode == 1
                    ? path_close_and_save(v, paths_next_name(v))
                    : path_close(v);
                if (closed) {
                    landDrawnPath(ctx, v);
                    return;
                }
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
        finishPath(*ctx_, *v);
        refreshOverlay(*ctx_);
        return true;
    }

    bool isPenDrawingTool() const { return id_ == ToolId::Pen || id_ == ToolId::FreeformPen; }

    // End the subpath being drawn and route it to the options bar's mode.
    void finishPath(ToolServices& ctx, PictureView& v)
    {
        if (isPenDrawingTool() && pen_->get().mode == 1) {
            path_finish_and_save(v, paths_next_name(v));
            return;
        }
        path_finish(v);
        landDrawnPath(ctx, v);
    }

    // Shape creates a shape layer from the Work Path; Pixels strokes it onto
    // the active layer. Path mode keeps the Work Path and saves a copy.
    void landDrawnPath(ToolServices& ctx, PictureView& v)
    {
        if (!isPenDrawingTool() || paths_active(v) != -1) {
            return;
        }
        const int mode = pen_->get().mode;
        if (mode == 0) {
            const char* label = id_ == ToolId::Pen ? "Pen Tool" : "Freeform Pen";
            const QString created =
                shape_add_work_path_layer(v, ctx.foreground().rgba(), label);
            if (!created.isEmpty()) {
                ctx.notifyLayerCreated(created);
            }
        } else if (mode == 2) {
            const int size = paint_ ? paint_->size() : 5;
            const int hardness = paint_ ? paint_->hardness() : 100;
            const int opacity = paint_ ? paint_->opacity() : 100;
            const int flow = paint_ ? paint_->flow() : 100;
            const int spacing = paint_ ? paint_->spacing() : 25;
            paths_stroke(v, ctx.foreground().rgba(), size, hardness, opacity, flow, spacing);
        }
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
        if (magneticActive_ && !magneticPath_.isEmpty()) {
            overlay.preview.moveTo(magneticPath_.first());
            for (int i = 1; i < magneticPath_.size(); ++i) overlay.preview.lineTo(magneticPath_.at(i));
            for (const QPointF& p : magneticPreview_) overlay.preview.lineTo(p);
            for (int i : magneticFastenings_) overlay.anchors.append(magneticPath_.at(i));
        } else if (gesture_ == Gesture::Freeform && trail_.size() > 1) {
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

    bool magneticActive_ = false;
    bool magneticPressed_ = false;
    bool magneticAltDrag_ = false;
    QPointer<PictureView> magneticView_;
    QList<QPointF> magneticPath_;
    QList<QPointF> magneticPreview_;
    QList<int> magneticFastenings_;
    QPointF magneticLastPress_;
    QElapsedTimer magneticClock_;
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
    std::shared_ptr<PaintOptions> paint_;
};

} // namespace

std::unique_ptr<ToolHandler> makePenToolHandler(ToolId id, std::shared_ptr<PenGroup> pen,
                                                std::shared_ptr<PaintOptions> paint)
{
    return std::make_unique<PenToolHandler>(id, std::move(pen), std::move(paint));
}

} // namespace pictura
