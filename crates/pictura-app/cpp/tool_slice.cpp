#include "tool_handler.h"

#include "crop_grip.h"
#include "icons.h"
#include "image_view.h"
#include "tools.h"

#include "api/transform_bridge.h"

#include <QtCore/QRectF>

#include <algorithm>
#include <cmath>
#include <memory>
#include <optional>

namespace pictura {

namespace {

// Show every resolved slice on the canvas: user slices solid blue with a
// numbered badge, auto slices dotted grey, user slice `selected` (a user index,
// -1 for none) with orange handles, plus `dragging` while one is drawn out.
void showSlices(ToolServices& ctx, int selected, const QRectF& dragging = {},
                const QRectF& preview = {})
{
    ImageView* canvas = ctx.canvas();
    PictureView* v = ctx.view();
    if (!canvas) {
        return;
    }
    if (v) {
        canvas->setDocumentSliceOverlay(*v, selected, dragging, preview);
    } else {
        canvas->clearSliceOverlay();
    }
}

int roundInt(double v) { return int(std::lround(v)); }

// Slice: drag out a user slice (one "Slice" history state); a click adds none.
// Ported from photorust's CanvasView.
class SliceToolHandler : public ToolHandler {
public:
    void onActivate(ToolServices& ctx) override { showSlices(ctx, -1); }

    void onDeactivate(ToolServices& ctx) override
    {
        dragging_ = false;
        if (ImageView* canvas = ctx.canvas()) {
            canvas->clearSliceOverlay();
        }
    }

    void onDocumentRefreshed(ToolServices& ctx) override
    {
        showSlices(ctx, -1, dragging_ ? drag_ : QRectF());
    }

    void onSessionCancelled(ToolServices& ctx) override
    {
        if (dragging_) {
            dragging_ = false;
            drag_ = {};
            showSlices(ctx, -1);
        }
    }

    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        PictureView* v = ctx.view();
        if (!v || !v->has_document()) {
            return true;
        }
        dragging_ = true;
        start_ = imagePos;
        drag_ = QRectF(imagePos, imagePos);
        showSlices(ctx, -1, drag_);
        return true;
    }

    void onMove(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        if (!dragging_) {
            return;
        }
        drag_ = QRectF(start_, imagePos).normalized();
        showSlices(ctx, -1, drag_);
    }

    void onRelease(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        if (!dragging_) {
            return;
        }
        dragging_ = false;
        const QRectF box = QRectF(start_, imagePos).normalized();
        drag_ = QRectF();
        PictureView* v = ctx.view();
        if (v && box.width() >= 1.0 && box.height() >= 1.0) {
            const int x = roundInt(box.x());
            const int y = roundInt(box.y());
            add_user_slice(*v, x, y, roundInt(box.right()) - x, roundInt(box.bottom()) - y);
        }
        showSlices(ctx, -1);
    }

private:
    bool dragging_ = false;
    QPointF start_;
    QRectF drag_;
};

// Slice Select: click a user slice to select it (orange handles); clicking an
// auto slice or empty canvas deselects. Drag inside the selected slice to move
// it, or an edge/corner handle to resize it: the overlay updates live and the
// release records one "Edit Slice" state. Delete removes the selected slice
// ("Delete Slice"); Escape deselects. Ported from photorust's CanvasView.
class SliceSelectToolHandler : public ToolHandler {
public:
    void onActivate(ToolServices& ctx) override
    {
        ctx_ = &ctx;
        showSlices(ctx, selected_);
    }

    void onDeactivate(ToolServices& ctx) override
    {
        grip_ = BoxGrip::None;
        selected_ = -1;
        preview_ = {};
        if (ImageView* canvas = ctx.canvas()) {
            canvas->clearSliceOverlay();
        }
        ctx_ = nullptr;
    }

    void onDocumentRefreshed(ToolServices& ctx) override
    {
        // Undo can remove the selected slice; drop a selection that no longer
        // exists.
        if (selected_ >= 0 && !userRect(ctx, selected_)) {
            selected_ = -1;
            grip_ = BoxGrip::None;
            preview_ = {};
        }
        showSlices(ctx, selected_, {}, preview_);
    }

    void onSessionCancelled(ToolServices& ctx) override
    {
        if (grip_ != BoxGrip::None) {
            grip_ = BoxGrip::None;
            preview_ = {};
            showSlices(ctx, selected_);
        }
    }

    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        PictureView* v = ctx.view();
        if (!v || !v->has_document()) {
            return true;
        }
        ctx_ = &ctx;
        grip_ = BoxGrip::None;
        if (selected_ >= 0) {
            if (const auto rect = userRect(ctx, selected_)) {
                grip_ = gripAt(ctx, *rect, imagePos);
                startRect_ = *rect;
            }
        }
        if (grip_ == BoxGrip::None) {
            // User slices win over the auto slices beneath them.
            selected_ = userSliceAt(ctx, imagePos);
            if (selected_ >= 0) {
                grip_ = BoxGrip::Move;
                startRect_ = *userRect(ctx, selected_);
            }
        }
        start_ = imagePos;
        preview_ = {};
        showSlices(ctx, selected_);
        return true;
    }

    void onMove(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        if (grip_ == BoxGrip::None || selected_ < 0) {
            updateCursor(ctx, imagePos);
            return;
        }
        const QRectF r = dragged(imagePos);
        if (r.width() >= 1.0 && r.height() >= 1.0) {
            preview_ = r;
            showSlices(ctx, selected_, {}, preview_);
        }
    }

    void onRelease(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        if (grip_ == BoxGrip::None) {
            return;
        }
        PictureView* v = ctx.view();
        const QRectF r = dragged(imagePos);
        grip_ = BoxGrip::None;
        preview_ = {};
        if (v && r.width() >= 1.0 && r.height() >= 1.0 && r != startRect_) {
            set_user_slice(*v, selected_, roundInt(r.x()), roundInt(r.y()), roundInt(r.width()),
                           roundInt(r.height()));
        }
        showSlices(ctx, selected_);
    }

    // Delete / Backspace (routed through the controller's delete-point hook).
    bool onDeletePoint() override
    {
        PictureView* v = ctx_ ? ctx_->view() : nullptr;
        if (!v || selected_ < 0) {
            return false;
        }
        const int index = selected_;
        selected_ = -1;
        grip_ = BoxGrip::None;
        preview_ = {};
        remove_user_slice(*v, index);
        showSlices(*ctx_, selected_);
        return true;
    }

    // Escape deselects.
    bool onEscape() override
    {
        if (selected_ < 0 || !ctx_) {
            return false;
        }
        selected_ = -1;
        grip_ = BoxGrip::None;
        preview_ = {};
        showSlices(*ctx_, selected_);
        return true;
    }

private:
    static std::optional<QRectF> userRect(ToolServices& ctx, int userIndex)
    {
        PictureView* v = ctx.view();
        const auto fields = v ? resolved_slice_fields(*v) : ::rust::Vec<std::int32_t>();
        for (size_t i = 0; i + 5 < fields.size(); i += 6) {
            if (fields[i + 5] == userIndex) {
                return QRectF(fields[i], fields[i + 1], fields[i + 2], fields[i + 3]);
            }
        }
        return std::nullopt;
    }

    static int userSliceAt(ToolServices& ctx, const QPointF& imagePos)
    {
        PictureView* v = ctx.view();
        const auto fields = v ? resolved_slice_fields(*v) : ::rust::Vec<std::int32_t>();
        for (size_t i = 0; i + 5 < fields.size(); i += 6) {
            const QRectF rect(fields[i], fields[i + 1], fields[i + 2], fields[i + 3]);
            if (fields[i + 5] >= 0 && rect.contains(imagePos)) {
                return fields[i + 5];
            }
        }
        return -1;
    }

    static BoxGrip gripAt(ToolServices& ctx, const QRectF& r, const QPointF& p)
    {
        return boxGripAt(r, p, ctx.canvas() ? ctx.canvas()->zoom() : 1.0);
    }

    QRectF dragged(const QPointF& imagePos) const
    {
        return dragBox(startRect_, grip_, imagePos - start_);
    }

    void updateCursor(ToolServices& ctx, const QPointF& imagePos)
    {
        ImageView* canvas = ctx.canvas();
        if (!canvas) {
            return;
        }
        const auto rect = selected_ >= 0 ? userRect(ctx, selected_) : std::nullopt;
        const BoxGrip grip = rect ? gripAt(ctx, *rect, imagePos) : BoxGrip::None;
        if (grip != BoxGrip::None) {
            canvas->setCursor(boxGripCursor(grip, Qt::CrossCursor));
            return;
        }
        const ToolInfo& info = toolInfo(ToolId::SliceSelect);
        const QCursor c = cursor(toolCursorId(ToolId::SliceSelect, Qt::NoModifier), info.hotspotX,
                                 info.hotspotY);
        canvas->setCursor(c.pixmap().isNull() ? QCursor(info.cursor) : c);
    }

    int selected_ = -1;
    BoxGrip grip_ = BoxGrip::None;
    QPointF start_;
    QRectF startRect_;
    QRectF preview_;
    ToolServices* ctx_ = nullptr;
};

} // namespace

bool ToolController::sliceEditingBlocked() const
{
    return slicesLocked_ && (active_ == ToolId::Slice || active_ == ToolId::SliceSelect);
}

void ToolController::setSlicesLocked(bool on)
{
    if (slicesLocked_ == on) {
        return;
    }
    slicesLocked_ = on;
    if (sliceEditingBlocked()) {
        if (ToolHandler* handler = registry_.forTool(active_)) {
            handler->onSessionCancelled(*this);
        }
        refreshCursor();
    }
}

std::unique_ptr<ToolHandler> makeSliceToolHandler()
{
    return std::make_unique<SliceToolHandler>();
}

std::unique_ptr<ToolHandler> makeSliceSelectToolHandler()
{
    return std::make_unique<SliceSelectToolHandler>();
}

} // namespace pictura
