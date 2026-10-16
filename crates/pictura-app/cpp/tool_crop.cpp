#include "tool_handler.h"
#include "tool_sessions.h"

#include "api/transform_api.h"
#include "crop_grip.h"
#include "icons.h"
#include "image_view.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/crop_group.cxxqt.h"

#include <QtCore/QElapsedTimer>
#include <QtCore/QPointF>
#include <QtCore/QRect>
#include <QtCore/QStringList>
#include <QtGui/QAction>
#include <QtWidgets/QApplication>
#include <QtWidgets/QMenu>

#include <cmath>
#include <memory>
#include <vector>

namespace pictura {

namespace {

// Snap `value` to the nearest candidate within `tol`, else return it.
double snapValue(double value, const std::vector<double>& candidates, double tol)
{
    double best = value;
    double best_delta = tol;
    for (double c : candidates) {
        const double d = std::abs(value - c);
        if (d < best_delta) {
            best_delta = d;
            best = c;
        }
    }
    return best;
}

// The translation that snaps the nearer of `lo`/`hi` to a candidate within `tol`.
double snapDelta(double lo, double hi, const std::vector<double>& candidates, double tol)
{
    double best = 0.0;
    double best_delta = tol;
    for (double c : candidates) {
        for (double edge : {lo, hi}) {
            const double d = c - edge;
            if (std::abs(d) < best_delta) {
                best_delta = std::abs(d);
                best = d;
            }
        }
    }
    return best;
}

// Crop (CS6's redesigned crop): choosing the tool shows a canvas-sized preview
// box with its eight handles in either mode; a handle drag resizes it, and a
// drag inside draws a new active box with a shield over what will be cut and
// rule-of-thirds guides. Drag a handle to resize (ratio-locked when an aspect ratio is set),
// drag inside to move (Classic) or pan the composite under the fixed box
// (Modern), drag outside to rotate. Enter, a double-click inside, or the options
// bar's Apply commits; Escape or Cancel resets to no box. Ported from
// photorust's CanvasView resetCrop/dragCrop/applyCropRatio.
class CropToolHandler : public ToolHandler, public CropCommitSession, public CropBoxSession {
public:
    explicit CropToolHandler(std::shared_ptr<CropOptions> crop)
        : crop_(std::move(crop))
    {
        addOptions(crop_.get());
    }

    // The Crop canvas menu: common crop options, shown only while a crop box is
    // present.
    bool hasCanvasContextMenu() const override { return true; }
    QMenu* buildContextMenu(ToolServices& ctx, const QPointF& imagePos) override
    {
        (void)imagePos;
        ImageView* canvas = ctx.canvas();
        if (!canvas || !cropActive()) {
            return nullptr;
        }
        auto* menu = new QMenu(canvas);
        QAction* del = menu->addAction(QObject::tr("Delete Cropped Pixels"));
        del->setCheckable(true);
        del->setChecked(crop_->deletePixels());
        QObject::connect(del, &QAction::triggered, menu,
                [this](bool on) { crop_->setDeletePixels(on); });
        QAction* reset = menu->addAction(QObject::tr("Reset Crop"));
        QObject::connect(reset, &QAction::triggered, menu, [this]() { resetCropBox(); });
        return menu;
    }

    // The crop states: no box (after Escape), the preview box the tool starts
    // with (not yet a pending crop), or an active box.
    enum class State { None, Preview, Active };

    void onActivate(ToolServices& ctx) override
    {
        ctx_ = &ctx;
        active_ = true;
        reset(ctx);
    }

    void onDeactivate(ToolServices& ctx) override
    {
        active_ = false;
        grip_ = BoxGrip::None;
        if (rotating_) {
            endRotation(ctx);
        }
        drawingNew_ = false;
        previewPress_ = false;
        straightenArmed_ = false;
        straightenDragging_ = false;
        ctrlArmed_ = false;
        box_ = QRectF();
        state_ = State::None;
        contentOffset_ = QPointF();
        clearSession(ctx);
        if (ImageView* canvas = ctx.canvas()) {
            canvas->clearCropBox();
        }
    }

    // A commit, undo, or tab switch changes the canvas under the box; start
    // over from the new canvas.
    void onDocumentRefreshed(ToolServices& ctx) override
    {
        if (active_ && canvasRect(ctx) != canvas_) {
            reset(ctx);
        }
    }

    void onOptionsChanged(ToolServices& ctx) override
    {
        if (box_.isNull()) {
            return;
        }
        if (state_ == State::Preview) {
            box_ = fitRatioCentered(QRectF(canvas_), crop_->ratio());
        } else {
            box_ = fitRatio(box_, crop_->ratio(), BoxGrip::BottomRight);
        }
        show(ctx);
    }

    // The spirit-level toggle: while armed, a canvas drag draws a horizon line
    // whose inclination sets the straighten angle on release.
    void setStraightenMode(bool on) override
    {
        straightenArmed_ = on;
        if (!on) {
            straightenDragging_ = false;
            ctrlArmed_ = false;
            if (ctx_ && ctx_->canvas()) {
                ctx_->canvas()->clearCropStraightenLine();
                ctx_->canvas()->clearDragSizeHint();
            }
        }
    }

    // Holding Ctrl (Cmd) arms the straighten line until the key is released,
    // as CS6 does; a line already being drawn finishes first.
    void onModifiers(ToolServices& ctx, Qt::KeyboardModifiers mods) override
    {
        if (!active_) {
            return;
        }
        ctx_ = &ctx;
        const bool ctrl = mods.testFlag(Qt::ControlModifier);
        const bool busy = grip_ != BoxGrip::None || rotating_ || drawingNew_ || previewPress_;
        if (ctrl && !straightenArmed_ && !busy) {
            crop_->setStraightenMode(true);
            ctrlArmed_ = true;
        } else if (!ctrl && ctrlArmed_ && !straightenDragging_) {
            crop_->setStraightenMode(false);
        }
    }

    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        PictureView* v = ctx.view();
        if (!v || !v->has_document()) {
            return true;
        }
        ctx_ = &ctx;
        if (straightenArmed_ || mods.testFlag(Qt::ControlModifier)) {
            // A straighten drag from none/preview activates a box first so the
            // pivot exists and the rotation can preview.
            activateForStraighten(ctx);
            straightenDragging_ = true;
            straightenStart_ = imagePos;
            straightenBase_ = crop_->angle();
            return true;
        }
        grip_ = boxGripAt(box_, imagePos, zoom(ctx), kCropRotateMarginPx);
        drawingNew_ = false;

        if (state_ == State::Active) {
            // A double-click is a second press on the same spot inside the box,
            // never on a handle.
            const QPointF d = (imagePos - lastPress_) * zoom(ctx);
            const bool doubleClick = grip_ == BoxGrip::Move && clock_.isValid()
                && clock_.elapsed() <= QApplication::doubleClickInterval()
                && std::hypot(d.x(), d.y()) <= 4.0;
            lastPress_ = imagePos;
            if (doubleClick) {
                clock_.invalidate();
                grip_ = BoxGrip::None;
                commitCrop();
                return true;
            }
            clock_.restart();
            if (grip_ == BoxGrip::None && cropRotateZone(box_, imagePos, zoom(ctx))) {
                startRotation(ctx, imagePos);
                return true;
            }
            // Far outside the box the cursor is the default arrow and a press
            // applies the crop, as CS6 does.
            if (grip_ == BoxGrip::None && cropFarZone(box_, imagePos, zoom(ctx))) {
                commitCrop();
                return true;
            }
            start_ = imagePos;
            startBox_ = box_;
            offsetStart_ = contentOffset_;
            return true;
        }

        if (state_ == State::Preview) {
            if (grip_ != BoxGrip::None && grip_ != BoxGrip::Move) {
                // A handle press resizes the preview, which becomes the crop.
                state_ = State::Active;
                show(ctx);
                notifyStateChanged(ctx);
                lastPress_ = imagePos;
                clock_.restart();
                start_ = imagePos;
                startBox_ = box_;
                offsetStart_ = contentOffset_;
                return true;
            }
            if (grip_ == BoxGrip::None && cropRotateZone(box_, imagePos, zoom(ctx))) {
                // A rotate press outside activates the preview and rotates.
                state_ = State::Active;
                show(ctx);
                notifyStateChanged(ctx);
                startRotation(ctx, imagePos);
                return true;
            }
            // Far outside a preview nothing happens; there is no active crop yet
            // to apply.
            if (grip_ == BoxGrip::None && cropFarZone(box_, imagePos, zoom(ctx))) {
                return true;
            }
            // A press inside the preview draws a new box on drag, or adopts the
            // preview if released without moving.
            previewPress_ = true;
            start_ = imagePos;
            drawAnchor_ = imagePos;
            lastPress_ = imagePos;
            clock_.restart();
            return true;
        }

        // None: a fresh box, clamped to the canvas, anchored at the press point.
        box_ = QRectF(imagePos, imagePos);
        drawAnchor_ = imagePos;
        grip_ = BoxGrip::BottomRight;
        drawingNew_ = true;
        state_ = State::Active;
        lastPress_ = imagePos;
        clock_.restart();
        start_ = imagePos;
        startBox_ = box_;
        show(ctx);
        notifyStateChanged(ctx);
        return true;
    }

    void onMove(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        if (straightenDragging_) {
            if (ImageView* canvas = ctx.canvas()) {
                canvas->setCropStraightenLine(QLineF(straightenStart_, imagePos));
                // The image holds still under the line, as CS6 does; the readout
                // shows the rotation the release will apply.
                if (std::hypot(imagePos.x() - straightenStart_.x(),
                               imagePos.y() - straightenStart_.y()) > 1.0) {
                    canvas->setDragSizeHint(
                        angleReadout(straightenBase_ + straightenAngle(straightenStart_, imagePos)),
                        imagePos);
                }
            }
            return;
        }
        if (rotating_) {
            const QPointF c = box_.center();
            double angle = startAngle_
                + std::atan2(imagePos.y() - c.y(), imagePos.x() - c.x()) * 180.0 / M_PI
                - pressAngle_ * 180.0 / M_PI;
            if (mods.testFlag(Qt::ShiftModifier)) {
                angle = std::round(angle / 15.0) * 15.0;
            }
            crop_->setAngle(angle);
            showRotation(ctx, imagePos);
            return;
        }
        if (state_ == State::Preview) {
            if (!previewPress_) {
                return;  // Hover: the cursor is resolved by hoverCursor().
            }
            const QPointF d = (imagePos - start_) * zoom(ctx);
            if (std::hypot(d.x(), d.y()) <= 2.0) {
                return;  // Still a potential click-to-adopt.
            }
            // The drag becomes a new active box drawn from the press point.
            previewPress_ = false;
            state_ = State::Active;
            box_ = QRectF(drawAnchor_, drawAnchor_);
            startBox_ = box_;
            grip_ = BoxGrip::BottomRight;
            drawingNew_ = true;
            start_ = drawAnchor_;
            notifyStateChanged(ctx);
        }
        if (grip_ == BoxGrip::None) {
            return;
        }
        if (drawingNew_) {
            box_ = dragBox(startBox_, grip_, imagePos - start_);
            box_ = box_.intersected(QRectF(canvas_));
            snapBox(ctx, box_, grip_);
            // Anchor the ratio fit on the press corner for any drag direction.
            box_ = fitRatio(box_, crop_->ratio(), ratioGripFromDirection(drawAnchor_, imagePos));
            box_ = box_.intersected(QRectF(canvas_));
            show(ctx);
            return;
        }
        if (grip_ == BoxGrip::Move) {
            if (crop_->classicMode()) {
                box_ = dragBox(startBox_, grip_, imagePos - start_);
                snapBox(ctx, box_, grip_);
            } else {
                // Modern: pan the composite under the box fixed on screen; the
                // box overlay is drawn without the offset, commit maps box-offset.
                contentOffset_ = offsetStart_ + (imagePos - start_);
                if (ImageView* canvas = ctx.canvas()) {
                    canvas->setCropContentOffset(contentOffset_);
                }
            }
            show(ctx);
            return;
        }
        const bool alt = mods.testFlag(Qt::AltModifier);
        if (alt) {
            // Alt scales about the box centre: the dragged edge moves, the
            // opposite edge mirrors it, and the ratio fit stays centred.
            box_ = dragBox(startBox_, grip_, imagePos - start_, true);
            box_ = fitRatio(box_, crop_->ratio(), grip_);
            box_.moveCenter(startBox_.center());
            show(ctx);
            return;
        }
        box_ = dragBox(startBox_, grip_, imagePos - start_);
        snapBox(ctx, box_, grip_);
        box_ = fitRatio(box_, crop_->ratio(), grip_);
        // The ratio fit moved the snapped edge; re-snap the whole box (keeping
        // the ratio) so a locked ratio still lands on a canvas/layer edge.
        if (crop_->ratio() > 0.0) {
            snapBox(ctx, box_, BoxGrip::Move);
        }
        show(ctx);
    }

    void onRelease(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        if (straightenDragging_) {
            straightenDragging_ = false;
            const double dx = imagePos.x() - straightenStart_.x();
            const double dy = imagePos.y() - straightenStart_.y();
            if (std::hypot(dx, dy) > 1.0) {
                // The drawn line becomes level (or plumb): rotate the content by
                // the negative of its inclination, then shrink the box inside
                // the rotated image so the commit has no empty corners.
                crop_->setAngle(straightenBase_ + straightenAngle(straightenStart_, imagePos));
                box_ = fitBoxInRotated(effectiveBox(), QRectF(canvas_), crop_->angle(),
                                       crop_->pivot())
                           .translated(contentOffset_);
                show(ctx);
                recordState(ctx);
            }
            crop_->setStraightenMode(false);  // disarm, clear the line, uncheck
            return;
        }
        if (rotating_) {
            endRotation(ctx);
            recordState(ctx);
            return;
        }
        if (state_ == State::Preview && previewPress_) {
            // A press and release without movement adopts the preview.
            previewPress_ = false;
            state_ = State::Active;
            show(ctx);
            notifyStateChanged(ctx);
            recordState(ctx);
            return;
        }
        if (grip_ == BoxGrip::None) {
            return;
        }
        onMove(ctx, imagePos, mods);
        grip_ = BoxGrip::None;
        drawingNew_ = false;
        // A click that drew no box puts the default box back.
        if (box_.width() < 1.0 || box_.height() < 1.0) {
            reset(ctx);
        } else {
            recordState(ctx);
        }
    }

    // Snap the dragged edge(s) to the canvas or the active layer's bounding
    // edges within a screen-pixel threshold.
    void snapBox(ToolServices& ctx, QRectF& box, BoxGrip grip)
    {
        const double tol = 6.0 / std::max(zoom(ctx), 1e-6);
        std::vector<double> xs = {0.0, double(canvas_.width())};
        std::vector<double> ys = {0.0, double(canvas_.height())};
        PictureView* v = ctx.view();
        const auto addRect = [&xs, &ys](const QStringList& p) {
            if (p.size() == 4) {
                xs.push_back(p[0].toDouble());
                xs.push_back(p[2].toDouble());
                ys.push_back(p[1].toDouble());
                ys.push_back(p[3].toDouble());
            }
        };
        const auto addLayerRect = [&xs, &ys](const transform_api::LayerRect& r) {
            if (r.valid) {
                xs.push_back(r.left);
                xs.push_back(r.right);
                ys.push_back(r.top);
                ys.push_back(r.bottom);
            }
        };
        if (v && v->has_document()) {
            // Snap to the whole composite's bounding box, then the active layer.
            // ponytail: composite_bounds stays a string crossing (deferred).
            addRect(composite_bounds(*v).split(QLatin1Char(' '), Qt::SkipEmptyParts));
            if (!v->active_layer_path().isEmpty()) {
                addLayerRect(transform_api::layerRect(*v, v->active_layer_path()));
            }
        }
        if (grip == BoxGrip::Move) {
            box.translate(snapDelta(box.left(), box.right(), xs, tol),
                          snapDelta(box.top(), box.bottom(), ys, tol));
            return;
        }
        const bool left =
            grip == BoxGrip::TopLeft || grip == BoxGrip::Left || grip == BoxGrip::BottomLeft;
        const bool right =
            grip == BoxGrip::TopRight || grip == BoxGrip::Right || grip == BoxGrip::BottomRight;
        const bool top =
            grip == BoxGrip::TopLeft || grip == BoxGrip::Top || grip == BoxGrip::TopRight;
        const bool bottom =
            grip == BoxGrip::BottomLeft || grip == BoxGrip::Bottom || grip == BoxGrip::BottomRight;
        if (left) box.setLeft(snapValue(box.left(), xs, tol));
        if (right) box.setRight(snapValue(box.right(), xs, tol));
        if (top) box.setTop(snapValue(box.top(), ys, tol));
        if (bottom) box.setBottom(snapValue(box.bottom(), ys, tol));
    }

    bool commitCrop() override
    {
        if (!ctx_ || !hasPendingCrop()) {
            return false;
        }
        PictureView* v = ctx_->view();
        const QRect rect = pendingCropRect();
        if (!v || rect.width() < 1 || rect.height() < 1) {
            return false;
        }
        // A box dragged past the canvas grows it; the added area takes the
        // background colour when the document has a Background layer.
        const int fill = int(ctx_->background().rgb() & 0xFFFFFF);
        // History names the commit by its guide overlay, e.g. "Crop: Grid".
        const QString label =
            QStringLiteral("Crop: ") + CropOptions::overlayNames().value(crop_->gridOverlay());
        const bool ok = crop_->angle() != 0.0
            ? straighten_crop(*v, rect.x(), rect.y(), rect.width(), rect.height(),
                              crop_->angle(), crop_->pivot().x(), crop_->pivot().y(),
                              crop_->deletePixels(), fill, label)
            : crop_to(*v, rect.x(), rect.y(), rect.width(), rect.height(),
                      crop_->deletePixels(), fill, label);
        if (ok) {
            ctx_->emitSelectionCommitted();
            clearSession(*ctx_);
        }
        return ok;
    }

    bool toolUndo() override
    {
        if (cursor_ <= 0) {
            return false;
        }
        --cursor_;
        applyState();
        return true;
    }

    bool toolRedo() override
    {
        if (cursor_ + 1 >= int(history_.size())) {
            return false;
        }
        ++cursor_;
        applyState();
        return true;
    }

    bool canToolUndo() const override { return cursor_ > 0; }
    bool canToolRedo() const override { return cursor_ + 1 < int(history_.size()); }

    // The options bar's W/H fields adjust the box dimensions from its top-left.
    bool resizeCrop(double width, double height) override
    {
        if (!ctx_ || state_ != State::Active || box_.isNull()) {
            return false;
        }
        QRectF b = box_;
        b.setWidth(std::max(1.0, width));
        b.setHeight(std::max(1.0, height));
        if (crop_->ratio() > 0.0) {
            b = fitRatio(b, crop_->ratio(), BoxGrip::TopLeft);
        }
        box_ = b;
        show(*ctx_);
        recordState(*ctx_);
        return true;
    }
    double cropWidth() const override { return box_.width(); }
    double cropHeight() const override { return box_.height(); }

    // Reset: make the box the whole canvas and clear the angle/offset.
    bool resetCropBox() override
    {
        if (!active_ || !ctx_ || canvas_.isEmpty()) {
            return false;
        }
        box_ = QRectF(canvas_);
        startBox_ = box_;
        contentOffset_ = QPointF();
        state_ = State::Active;
        crop_->setAngle(0.0);
        show(*ctx_);
        recordState(*ctx_);
        notifyStateChanged(*ctx_);
        return true;
    }

    // Arrow keys: move the crop box in Classic, pan the composite in Modern.
    bool nudge(int dx, int dy) override
    {
        if (!active_ || state_ != State::Active || box_.isNull() || (dx == 0 && dy == 0)) {
            return false;
        }
        if (ctx_ && crop_->classicMode()) {
            box_ = box_.translated(dx, dy);
        } else {
            contentOffset_ += QPointF(dx, dy);
        }
        if (ctx_) {
            show(*ctx_);
            recordState(*ctx_);
        }
        return true;
    }

    // Escape / Cancel: drop the box and angle, returning to no box in BOTH
    // modes, so a fresh drag draws a new box honoring the ratio.
    bool onEscape() override
    {
        grip_ = BoxGrip::None;
        if (rotating_ && ctx_) {
            endRotation(*ctx_);
        }
        rotating_ = false;
        drawingNew_ = false;
        previewPress_ = false;
        straightenArmed_ = false;
        straightenDragging_ = false;
        ctrlArmed_ = false;
        box_ = QRectF();
        startBox_ = QRectF();
        state_ = State::None;
        contentOffset_ = QPointF();
        if (ctx_) {
            clearSession(*ctx_);
            show(*ctx_);
            notifyStateChanged(*ctx_);
        }
        return true;
    }

    QPointF cropCenter() const override
    {
        return box_.isNull() ? QPointF() : effectiveBox().center();
    }

    bool cropActive() const override
    {
        return active_ && state_ == State::Active && !box_.isNull();
    }

    bool pointerCursor() const override { return true; }

    // Pointer cursor zones, shared by the live cursor and the test hook.
    enum CursorKind {
        CursorNone = 0,
        CursorBox,      // over the box body: the workspace arrow
        CursorRotate,   // in the rotate band outside the box: straighten
        CursorResize,   // on a handle
        CursorNewCrop,  // where a new box can be drawn
        CursorDefault,  // beyond the rotate band: the workspace arrow, a press applies
        CursorStraighten  // the straighten line is armed: crosshair and spirit level
    };

    int pointerCursorKind(const ToolServices& ctx, const QPointF& imagePos,
                          Qt::KeyboardModifiers mods) const override
    {
        return cursorStateAt(ctx, imagePos, mods).kind;
    }

    // The cursor for the live pointer: resize over a handle, the workspace arrow
    // over an active box, rotate outside the box, and the new-crop crosshair
    // inside a preview or where a new box can start. A captured drag keeps its
    // cursor.
    bool hoverCursor(const ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods,
                     QCursor& out) const override
    {
        const CursorState s = cursorStateAt(ctx, imagePos, mods);
        switch (s.kind) {
        case CursorBox:
        case CursorDefault:
            out = cursorOr(QStringLiteral("cursor.workspace"), 2, 2, Qt::ArrowCursor);
            return true;
        case CursorRotate: {
            // The arc bulges outward along the nearest of the eight box
            // corner/edge zones, so corners get a diagonal and edges an axis.
            const QPointF c = box_.center();
            const double a = cropRotateZoneAngle(imagePos - c) * M_PI / 180.0;
            const QCursor rc = rotateCursor(c, c + QPointF(std::cos(a), std::sin(a)));
            out = rc.pixmap().isNull() ? QCursor(Qt::CrossCursor) : rc;
            return true;
        }
        case CursorResize: out = QCursor(boxGripCursor(s.grip, Qt::ArrowCursor)); return true;
        case CursorNewCrop: out = QCursor(Qt::CrossCursor); return true;
        case CursorStraighten:
            out = cursorOr(QStringLiteral("cursor.straighten"), 8, 8, Qt::CrossCursor);
            return true;
        default: return false;
        }
    }

    // Pending only while the tool is active, the box is active, and it differs
    // from the canvas (or a straighten angle is set), so Image > Crop is not
    // captured by the default box.
    bool hasPendingCrop() const override
    {
        const double angle = ctx_ ? crop_->angle() : 0.0;
        return active_ && state_ == State::Active && !box_.isNull()
            && (pendingCropRect() != canvas_ || angle != 0.0);
    }

    QRect pendingCropRect() const override
    {
        const QRectF r = effectiveBox().normalized();
        const int x = int(std::floor(r.x()));
        const int y = int(std::floor(r.y()));
        return QRect(x, y, int(std::lround(r.right())) - x, int(std::lround(r.bottom())) - y);
    }

private:
    static double zoom(const ToolServices& ctx) { return ctx.canvas() ? ctx.canvas()->zoom() : 1.0; }

    static QRect canvasRect(const ToolServices& ctx)
    {
        PictureView* v = ctx.view();
        if (!v || !v->has_document()) {
            return QRect();
        }
        return QRect(0, 0, v->document_width(), v->document_height());
    }

    static QCursor cursorOr(const QString& id, int hotX, int hotY, Qt::CursorShape fallback)
    {
        const QCursor c = cursor(id, hotX, hotY);
        return c.pixmap().isNull() ? QCursor(fallback) : c;
    }

    // Fit `ratio` and centre the result on `rect` (Modern's initial preview).
    static QRectF fitRatioCentered(const QRectF& rect, double ratio)
    {
        if (ratio <= 0.0 || rect.width() <= 0.0 || rect.height() <= 0.0) {
            return rect;
        }
        QRectF b = fitRatio(rect, ratio, BoxGrip::BottomRight);
        b.moveCenter(rect.center());
        return b;
    }

    // The grip whose `fitRatio` anchor keeps the press point fixed, given the
    // drag direction from `anchor` to `pos`.
    static BoxGrip ratioGripFromDirection(const QPointF& anchor, const QPointF& pos)
    {
        const bool right = pos.x() >= anchor.x();
        const bool down = pos.y() >= anchor.y();
        if (right && down) return BoxGrip::BottomRight;
        if (!right && !down) return BoxGrip::TopLeft;
        if (right && !down) return BoxGrip::TopRight;
        return BoxGrip::BottomLeft;
    }

    struct CursorState {
        int kind = CursorNone;
        BoxGrip grip = BoxGrip::None;
    };

    // The cursor zone under `p`: a captured drag keeps its cursor; otherwise a
    // handle wins, then the box body, then the rotate ring, then new-crop. A
    // preview's inside is the new-crop crosshair and its outside the rotate.
    CursorState cursorStateAt(const ToolServices& ctx, const QPointF& p,
                              Qt::KeyboardModifiers mods) const
    {
        (void)mods;
        if (!active_) {
            return {};
        }
        if (straightenArmed_ || straightenDragging_) {
            return {CursorStraighten, BoxGrip::None};
        }
        if (rotating_) {
            return {CursorRotate, BoxGrip::None};
        }
        if (state_ == State::None || box_.isNull()) {
            return {CursorNewCrop, BoxGrip::None};
        }
        const double z = zoom(ctx);
        if (state_ == State::Preview) {
            // Use the same 8 px grab as onPress so the cursor and the press
            // classify the preview rim identically (a 6 px default would show
            // the arrow over a point that still starts a draw).
            const BoxGrip hover = boxGripAt(box_, p, z, kCropRotateMarginPx);
            if (hover == BoxGrip::Move) {
                return {CursorNewCrop, BoxGrip::None};
            }
            if (hover != BoxGrip::None) {
                return {CursorResize, hover};
            }
            return cropRotateZone(box_, p, z) ? CursorState{CursorRotate, BoxGrip::None}
                                              : CursorState{CursorDefault, BoxGrip::None};
        }
        if (grip_ != BoxGrip::None && grip_ != BoxGrip::Move) {
            return {CursorResize, grip_};
        }
        const BoxGrip hover = boxGripAt(box_, p, z, kCropRotateMarginPx);
        if (hover != BoxGrip::None && hover != BoxGrip::Move) {
            return {CursorResize, hover};
        }
        if (hover == BoxGrip::Move) {
            return {CursorBox, BoxGrip::None};
        }
        // Outside the box: the rotate band rotates, beyond it the default arrow
        // (a press there applies the crop). Both extend past the canvas edge.
        return cropRotateZone(box_, p, z) ? CursorState{CursorRotate, BoxGrip::None}
                                          : CursorState{CursorDefault, BoxGrip::None};
    }

    QRectF effectiveBox() const { return box_.translated(-contentOffset_); }

    // The rotate gesture: CS6 swaps the guide overlay for a dense grid and shows
    // the angle beside the pointer until the release.
    void startRotation(ToolServices& ctx, const QPointF& imagePos)
    {
        rotating_ = true;
        startAngle_ = crop_->angle();
        const QPointF c = box_.center();
        pressAngle_ = std::atan2(imagePos.y() - c.y(), imagePos.x() - c.x());
        showRotation(ctx, imagePos);
    }

    void showRotation(ToolServices& ctx, const QPointF& imagePos)
    {
        if (ImageView* canvas = ctx.canvas()) {
            canvas->setCropRotating(true);
            canvas->setDragSizeHint(angleReadout(crop_->angle()), imagePos);
        }
    }

    static QString angleReadout(double degrees)
    {
        return QStringLiteral("\u2220: %1\u00B0").arg(degrees, 0, 'f', 1);
    }

    void endRotation(ToolServices& ctx)
    {
        rotating_ = false;
        if (ImageView* canvas = ctx.canvas()) {
            canvas->setCropRotating(false);
            canvas->clearDragSizeHint();
        }
    }

    void activateForStraighten(ToolServices& ctx)
    {
        if (state_ == State::Active) {
            return;
        }
        if (state_ == State::Preview && !box_.isNull()) {
            state_ = State::Active;
        } else {
            // No box (after Escape): use the full canvas so a pivot exists.
            box_ = QRectF(canvas_);
            state_ = State::Active;
        }
        contentOffset_ = QPointF();
        show(ctx);
        notifyStateChanged(ctx);
    }

    void reset(ToolServices& ctx)
    {
        grip_ = BoxGrip::None;
        rotating_ = false;
        drawingNew_ = false;
        previewPress_ = false;
        canvas_ = canvasRect(ctx);
        contentOffset_ = QPointF();
        crop_->setAngle(0.0);
        // Both modes start with a centered, ratio-fitted preview box, as CS6
        // frames the whole image when the tool is chosen.
        if (canvas_.isEmpty()) {
            box_ = QRectF();
            state_ = State::None;
        } else {
            box_ = fitRatioCentered(QRectF(canvas_), crop_->ratio());
            state_ = State::Preview;
        }
        history_.clear();
        cursor_ = -1;
        recordState(ctx);
        show(ctx);
        notifyStateChanged(ctx);
    }

    // The modal tool session: box/angle/offset steps recorded on each released
    // drag, walked by Edit Undo/Redo while the crop is active.
    struct CropState {
        QRectF box;
        double angle = 0.0;
        QPointF offset;
    };

    static bool sameState(const CropState& a, const CropState& b)
    {
        return std::abs(a.box.x() - b.box.x()) < 0.01 && std::abs(a.box.y() - b.box.y()) < 0.01
            && std::abs(a.box.width() - b.box.width()) < 0.01
            && std::abs(a.box.height() - b.box.height()) < 0.01
            && std::abs(a.offset.x() - b.offset.x()) < 0.01
            && std::abs(a.offset.y() - b.offset.y()) < 0.01
            && std::abs(a.angle - b.angle) < 1e-6;
    }

    void recordState(ToolServices& ctx)
    {
        const CropState state{box_, crop_->angle(), contentOffset_};
        if (cursor_ >= 0 && cursor_ < int(history_.size()) && sameState(history_[cursor_], state)) {
            return;
        }
        history_.resize(cursor_ + 1);
        history_.push_back(state);
        cursor_ = int(history_.size()) - 1;
        ctx.notifyToolSessionChanged();
    }

    void applyState()
    {
        if (!ctx_ || cursor_ < 0 || cursor_ >= int(history_.size())) {
            return;
        }
        box_ = history_[cursor_].box;
        contentOffset_ = history_[cursor_].offset;
        if (history_[cursor_].box.isNull()) {
            state_ = State::None;
        } else if (state_ == State::None) {
            state_ = State::Active;
        }
        crop_->setAngle(history_[cursor_].angle);
        show(*ctx_);
        notifyStateChanged(*ctx_);
    }

    void clearSession(ToolServices& ctx)
    {
        history_.clear();
        cursor_ = -1;
        crop_->setAngle(0.0);
        ctx.notifyToolSessionChanged();
    }

    void notifyStateChanged(ToolServices& ctx) { ctx.notifyToolSessionChanged(); }

    void show(ToolServices& ctx)
    {
        if (ImageView* canvas = ctx.canvas()) {
            if (box_.isNull()) {
                canvas->clearCropBox();
            } else {
                canvas->setCropContentOffset(contentOffset_);
                canvas->setCropBox(box_);
                canvas->setCropPreview(state_ == State::Preview);
                canvas->setCropOverlay(crop_->gridOverlay());
            }
        }
    }

    bool active_ = false;
    State state_ = State::None;
    QRectF box_;
    QRectF startBox_;
    QRect canvas_;
    QPointF start_;
    QPointF drawAnchor_;
    QPointF contentOffset_;
    QPointF offsetStart_;
    QPointF lastPress_;
    BoxGrip grip_ = BoxGrip::None;
    QElapsedTimer clock_;
    // The straighten gesture: rotating about the box centre from `startAngle_`.
    bool rotating_ = false;
    // True while a fresh box is being dragged out; it stays clamped to the canvas.
    bool drawingNew_ = false;
    // A press inside a preview that may become a draw or a click-to-adopt.
    bool previewPress_ = false;
    // The straighten line tool: armed by the spirit-level toggle; `straightenStart_`
    // is the press point (image space) of the horizon being drawn.
    bool straightenArmed_ = false;
    bool straightenDragging_ = false;
    // Armed by holding Ctrl rather than the toggle; releasing Ctrl disarms.
    bool ctrlArmed_ = false;
    QPointF straightenStart_;
    // The angle before this line, so a second line levels what is on screen.
    double straightenBase_ = 0.0;
    double startAngle_ = 0.0;
    double pressAngle_ = 0.0;
    std::vector<CropState> history_;
    int cursor_ = -1;
    ToolServices* ctx_ = nullptr;
    std::shared_ptr<CropOptions> crop_;
};

} // namespace

std::unique_ptr<ToolHandler> makeCropToolHandler(std::shared_ptr<CropOptions> crop)
{
    return std::make_unique<CropToolHandler>(std::move(crop));
}

} // namespace pictura
