#include "tool_handler.h"

#include "file_drop_router.h"
#include "image_view.h"
#include "panels/layers_panel_internal.h"
#include "smart_guides.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layer_target_bridge.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/auto_select.cxxqt.h"

#include <QtCore/QDebug>
#include <QtCore/QElapsedTimer>
#include <QtCore/QObject>
#include <QtCore/QStringList>
#include <QtGui/QAction>
#include <QtGui/QDrag>
#include <QtGui/QImage>
#include <QtWidgets/QMenu>

#include <algorithm>
#include <memory>

namespace pictura {

namespace {

// The box round `layer`'s non-transparent pixels in its own coordinates; the
// whole image when it has no alpha (or is not a 32-bit format).
QRectF contentBounds(const QImage& layer)
{
    if (layer.isNull()) {
        return QRectF();
    }
    QRect box = layer.rect();
    if (layer.hasAlphaChannel() && layer.depth() == 32) {
        int left = layer.width();
        int right = -1;
        int top = layer.height();
        int bottom = -1;
        for (int y = 0; y < layer.height(); ++y) {
            const auto* row = reinterpret_cast<const QRgb*>(layer.constScanLine(y));
            for (int x = 0; x < layer.width(); ++x) {
                if (qAlpha(row[x]) != 0) {
                    left = std::min(left, x);
                    right = std::max(right, x);
                    top = std::min(top, y);
                    bottom = std::max(bottom, y);
                }
            }
        }
        if (right < 0) {
            return QRectF();
        }
        box = QRect(QPoint(left, top), QPoint(right, bottom));
    }
    return QRectF(box);
}

// Move: whole-layer translate and, when a selection exists, the content move.
// It owns the warm-preview cache so the first press after activation skips the
// expensive region build.
class MoveToolHandler : public ToolHandler {
public:
    explicit MoveToolHandler(std::shared_ptr<MoveGroup> move)
        : move_(std::move(move))
    {
        addOptions(move_.get());
    }

    void onActivate(ToolServices& ctx) override { warm(ctx); }

    void onDeactivate(ToolServices& ctx) override
    {
        if (ImageView* canvas = ctx.canvas()) {
            canvas->clearSmartGuides();
        }
        warmView_ = nullptr;
        warmBase_ = QImage();
        warmLayer_ = QImage();
        warmValid_ = false;
        pendingDuplicate_ = false;
        duplicated_ = false;
        pressed_ = false;
    }

    // The Move canvas menu: with Auto Select off, every layer with a pixel under
    // the pointer (topmost first) picks that layer; Isolation Mode is always
    // offered.
    bool hasCanvasContextMenu() const override { return true; }
    QMenu* buildContextMenu(ToolServices& ctx, const QPointF& imagePos) override
    {
        ImageView* canvas = ctx.canvas();
        PictureView* v = ctx.view();
        if (!canvas || !v) {
            return nullptr;
        }
        auto* menu = new QMenu(canvas);
        ToolServices* services = &ctx;
        if (!move_->get().autoSelectOn) {
            const QStringList paths =
                layers_under_layer(*v, qRound(imagePos.x()), qRound(imagePos.y()));
            for (const QString& path : paths) {
                const QString name = layer_name_for_path(*v, path);
                QAction* action = menu->addAction(name.isEmpty() ? path : name);
                QObject::connect(action, &QAction::triggered, menu,
                        [services, path]() { services->selectLayer(path); });
            }
            if (!paths.isEmpty()) {
                menu->addSeparator();
            }
        }
        QAction* isolate = menu->addAction(QObject::tr("Isolation Mode"));
        QObject::connect(isolate, &QAction::triggered, menu,
                [services]() { services->requestIsolationMode(); });
        return menu;
    }

    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        PictureView* v = ctx.view();
        if (!v) {
            return true;
        }
        if (v->has_selection()) {
            ctx.beginContentMove(v, imagePos, mods.testFlag(Qt::AltModifier));
            return true;
        }
        const MoveOptions moveOptions = move_->get();
        if (moveOptions.autoSelectOn) {
            const QString hit = auto_select_layer(
                *v, qRound(imagePos.x()), qRound(imagePos.y()), moveOptions.autoSelect == 0);
            if (!hit.isEmpty() && hit != layer_selection_anchor(*v)) {
                ctx.selectLayer(hit);
            }
        }
        QElapsedTimer pressClock;
        pressClock.start();
        const bool alt = mods.testFlag(Qt::AltModifier);
        const bool prepared = v->begin_move_preview();
        pendingDuplicate_ = alt;
        duplicated_ = false;
        // A layer the move preview cannot take (a group, text, a nested layer)
        // still leaves for another document's tab.
        pressed_ = true;
        const qint64 pressNs = pressClock.nsecsElapsed();
        if (qEnvironmentVariableIsSet("PICTURA_PRESS_TRACE") || pressNs > 8000000) {
            qWarning("[move-press] begin_move_preview hit=%d work=%.1fms",
                     (prepared && v->move_preview_cache_hit()) ? 1 : 0, pressNs / 1e6);
        }
        if (!prepared) {
            return true;
        }
        ctx.setDragging(true);
        ctx.setDragCommitted(false);
        last_ = imagePos;
        totalDelta_ = QPointF();
        rawDelta_ = QPointF();
        ImageView* canvas = ctx.canvas();
        referenceRects_ = canvas && canvas->smartGuidesEnabled()
            ? smartGuideReferences(*v, layer_selection_anchor(*v))
            : QList<QRectF>();
        if (canvas) {
            const bool reuse = warmValid_ && warmView_ == v && v->move_preview_cache_hit();
            if (!reuse) {
                warmBase_ = v->move_preview_base();
                warmLayer_ = v->move_preview_layer();
                warmView_ = v;
                warmValid_ = !warmBase_.isNull();
            }
            const QPointF origin(v->move_preview_x(), v->move_preview_y());
            canvas->beginMovePreview(warmBase_, warmLayer_, origin,
                                     v->move_preview_opacity() / 255.0);
            // The moving layer's content box as marching ants, as CS6 draws.
            // The scan is cached per layer image, so a warm press stays cheap.
            if (warmLayer_.cacheKey() != boundsKey_) {
                boundsKey_ = warmLayer_.cacheKey();
                bounds_ = contentBounds(warmLayer_);
            }
            canvas->setMoveOutline(bounds_.translated(origin));
        }
        return true;
    }

    void onMove(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        // ponytail: an Alt clone already inserted into the source stays a local
        // move; CS6 would carry it across without the clone.
        if (pressed_ && !duplicated_ && ctx.canvas()
            && FileDropRouter::otherDocumentTabAt(
                   ctx.canvas()->mapToGlobal(ctx.canvas()->imageToWidget(imagePos)).toPoint())
                >= 0) {
            dragToDocument(ctx);
            return;
        }
        if (!ctx.dragging()) {
            return;
        }
        rawDelta_ += imagePos - last_;
        last_ = imagePos;
        // Defer the Alt clone to the first real movement, so a bare Alt press
        // with no movement inserts nothing. The clone appears with the preview
        // rebuilt for the copy, then follows the pointer like the source did.
        if (pendingDuplicate_
            && (qRound(rawDelta_.x()) != 0 || qRound(rawDelta_.y()) != 0)) {
            pendingDuplicate_ = false;
            PictureView* v = ctx.view();
            if (v && v->begin_move_duplicate()) {
                warmValid_ = false;
                duplicated_ = true;
                if (ImageView* canvas = ctx.canvas()) {
                    canvas->beginMovePreview(v->move_preview_base(), v->move_preview_layer(),
                                             QPointF(v->move_preview_x(), v->move_preview_y()),
                                             v->move_preview_opacity() / 255.0);
                }
            }
        }
        PictureView* v = ctx.view();
        ImageView* canvas = ctx.canvas();
        if (v && canvas) {
            const QPointF origin(v->move_preview_x(), v->move_preview_y());
            const SmartGuideResult guides =
                canvas->smartGuidesEnabled()
                    ? smartGuideSnap(bounds_.translated(origin + rawDelta_), referenceRects_,
                                     QSizeF(v->document_width(), v->document_height()),
                                     canvas->zoom())
                    : SmartGuideResult{};
            totalDelta_ = rawDelta_ + guides.snap;
            canvas->setSmartGuides(guides.lines, guides.gaps);
            canvas->setMovePreviewDelta(totalDelta_);
            // ΔX / ΔY in the rulers' unit, by the pointer.
            canvas->setDragSizeHint(
                canvas->offsetReadout(QPointF(qRound(totalDelta_.x()), qRound(totalDelta_.y()))),
                imagePos);
        }
    }

    void onRelease(ToolServices& ctx, const QPointF&, Qt::KeyboardModifiers) override
    {
        pressed_ = false;
        if (!ctx.dragging()) {
            return;
        }
        ctx.setDragging(false);
        if (PictureView* v = ctx.view()) {
            v->end_move_preview();
            const int dx = qRound(totalDelta_.x());
            const int dy = qRound(totalDelta_.y());
            if (dx != 0 || dy != 0) {
                v->commit_move(dx, dy);
            }
        }
        pendingDuplicate_ = false;
        if (ImageView* canvas = ctx.canvas()) {
            canvas->endMovePreview();
            canvas->clearDragSizeHint();
            canvas->clearSmartGuides();
        }
    }

private:
    // The pointer reached another document's tab: drop the in-canvas move and
    // carry the layer on as a Layers-panel drag, which the frame's drop router
    // copies into whichever document takes the drop.
    void dragToDocument(ToolServices& ctx)
    {
        const bool moving = ctx.dragging();
        ctx.setDragging(false);
        pressed_ = false;
        pendingDuplicate_ = false;
        PictureView* v = ctx.view();
        ImageView* canvas = ctx.canvas();
        if (!v || !canvas) {
            return;
        }
        if (moving) {
            canvas->endMovePreview();
            canvas->clearDragSizeHint();
            canvas->clearSmartGuides();
            v->end_move_preview();
        }
        const QString path = v->active_layer_path();
        if (path.isEmpty()) {
            return;
        }
        auto* drag = new QDrag(canvas);
        drag->setMimeData(makeLayerDragMime(v, QStringList{path}));
        // Blocks until the drop; the router may switch the active document, so
        // neither `ctx` nor `canvas` is touched afterwards.
        drag->exec(Qt::CopyAction);
    }

    void warm(ToolServices& ctx)
    {
        PictureView* v = ctx.view();
        if (!v) {
            warmValid_ = false;
            return;
        }
        if (!v->prepare_move_preview()) {
            warmValid_ = false;
            return;
        }
        warmBase_ = v->move_preview_base();
        warmLayer_ = v->move_preview_layer();
        warmView_ = v;
        warmValid_ = !warmBase_.isNull();
    }

    QPointF last_;
    // The pointer's travel; `totalDelta_` is that plus the smart-guide snap.
    QPointF rawDelta_;
    QPointF totalDelta_;
    QList<QRectF> referenceRects_;
    qint64 boundsKey_ = 0;
    QRectF bounds_;
    PictureView* warmView_ = nullptr;
    QImage warmBase_;
    QImage warmLayer_;
    bool warmValid_ = false;
    // Alt is armed at press; the clone is inserted on the first non-zero move.
    bool pendingDuplicate_ = false;
    bool duplicated_ = false;
    // The button went down with no selection: a drag may leave for a tab.
    bool pressed_ = false;
    std::shared_ptr<MoveGroup> move_;
};

} // namespace

std::unique_ptr<ToolHandler> makeMoveToolHandler(std::shared_ptr<MoveGroup> move)
{
    return std::make_unique<MoveToolHandler>(std::move(move));
}

} // namespace pictura
