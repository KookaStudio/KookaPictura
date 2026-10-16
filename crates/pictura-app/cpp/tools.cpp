#include "tools.h"

#include "icons.h"
#include "image_view.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_transform/bridge.cxxqt.h"
#include "pictura_app/src/cxxqt_object/annotations.cxxqt.h"

#include <QtCore/QDebug>
#include <QtCore/QElapsedTimer>
#include <QtCore/QPoint>
#include <QtGui/QCursor>
#include <QtGui/QGuiApplication>
#include <QtGui/QPainterPath>
#include <QtWidgets/QApplication>

#include <algorithm>
#include <cmath>
#include <memory>

namespace pictura {

// Concrete handlers live in their own translation units; ToolController owns
// only the registry. The factories take the family option groups the handlers
// own (shared within a family) and read directly.
std::unique_ptr<ToolHandler> makeHandToolHandler();
std::unique_ptr<ToolHandler> makeZoomToolHandler();
std::unique_ptr<ToolHandler> makeEyedropperToolHandler(std::shared_ptr<EyedropperGroup> options);
std::unique_ptr<ToolHandler> makeBrushToolHandler(bool aliased, std::shared_ptr<PaintOptions> paint);
std::unique_ptr<ToolHandler> makeMagicWandToolHandler(std::shared_ptr<SelectionOptions> selection);
std::unique_ptr<ToolHandler> makeQuickSelectionToolHandler(
    std::shared_ptr<SelectionOptions> selection);
std::unique_ptr<ToolHandler> makeMoveToolHandler(std::shared_ptr<MoveGroup> move);
std::unique_ptr<ToolHandler> makeCropToolHandler(std::shared_ptr<CropOptions> crop);
std::unique_ptr<ToolHandler> makePerspectiveCropToolHandler(std::shared_ptr<CropOptions> crop);
std::unique_ptr<ToolHandler> makeSliceToolHandler();
std::unique_ptr<ToolHandler> makeSliceSelectToolHandler();
std::unique_ptr<ToolHandler> makeMarqueeToolHandler(std::shared_ptr<SelectionOptions> selection);
std::unique_ptr<ToolHandler> makeEllipticalMarqueeToolHandler(
    std::shared_ptr<SelectionOptions> selection);
std::unique_ptr<ToolHandler> makeLassoToolHandler(std::shared_ptr<SelectionOptions> selection);
std::unique_ptr<ToolHandler> makePolygonalLassoToolHandler(
    std::shared_ptr<SelectionOptions> selection);
std::unique_ptr<ToolHandler> makeMagneticLassoToolHandler(
    std::shared_ptr<SelectionOptions> selection);
std::unique_ptr<ToolHandler> makeColorSamplerToolHandler();
std::unique_ptr<ToolHandler> makeRulerToolHandler();
std::unique_ptr<ToolHandler> makeNoteToolHandler();
std::unique_ptr<ToolHandler> makeCountToolHandler();
std::unique_ptr<ToolHandler> makeSpotHealingToolHandler(std::shared_ptr<PaintOptions> paint,
                                                        std::shared_ptr<HealingGroup> healing);
std::unique_ptr<ToolHandler> makeHealingToolHandler(std::shared_ptr<PaintOptions> paint,
                                                    std::shared_ptr<HealingGroup> healing);
std::unique_ptr<ToolHandler> makePatchToolHandler(std::shared_ptr<PaintOptions> paint,
                                                  std::shared_ptr<PatchGroup> patch);
std::unique_ptr<ToolHandler> makeContentAwareMoveToolHandler(
    std::shared_ptr<ContentAwareMoveGroup> contentAware);
std::unique_ptr<ToolHandler> makeRedEyeToolHandler(std::shared_ptr<PaintOptions> paint,
                                                   std::shared_ptr<RedEyeGroup> redEye);
std::unique_ptr<ToolHandler> makeColorReplacementToolHandler(
    std::shared_ptr<PaintOptions> paint, std::shared_ptr<ColorReplaceGroup> colorReplace);
std::unique_ptr<ToolHandler> makeMixerBrushToolHandler(std::shared_ptr<PaintOptions> paint,
                                                       std::shared_ptr<MixerGroup> mixer);
std::unique_ptr<ToolHandler> makeCloneStampToolHandler(
    std::shared_ptr<PaintOptions> paint, std::shared_ptr<StampGroup> stamp,
    std::shared_ptr<CloneSourceOptions> clone);
std::unique_ptr<ToolHandler> makePatternStampToolHandler(std::shared_ptr<PaintOptions> paint,
                                                         std::shared_ptr<StampGroup> stamp);
std::unique_ptr<ToolHandler> makeHistoryBrushToolHandler(std::shared_ptr<PaintOptions> paint);
std::unique_ptr<ToolHandler> makeArtHistoryBrushToolHandler(
    std::shared_ptr<PaintOptions> paint, std::shared_ptr<ArtHistoryGroup> artHistory);
std::unique_ptr<ToolHandler> makeEraserToolHandler(std::shared_ptr<PaintOptions> paint,
                                                   std::shared_ptr<EraserGroup> eraser);
std::unique_ptr<ToolHandler> makeBackgroundEraserToolHandler(
    std::shared_ptr<PaintOptions> paint, std::shared_ptr<BackgroundEraseGroup> backgroundErase);
std::unique_ptr<ToolHandler> makeMagicEraserToolHandler(
    std::shared_ptr<PaintOptions> paint, std::shared_ptr<MagicEraseGroup> magicErase);
std::unique_ptr<ToolHandler> makeGradientToolHandler(std::shared_ptr<GradientGroup> gradient);
std::unique_ptr<ToolHandler> makePaintBucketToolHandler(std::shared_ptr<BucketGroup> bucket);
std::unique_ptr<ToolHandler> makeRetouchToolHandler(ToolId id, std::shared_ptr<PaintOptions> paint,
                                                    std::shared_ptr<RetouchGroup> retouch,
                                                    std::shared_ptr<ToneGroup> tone);
std::unique_ptr<ToolHandler> makePenToolHandler(ToolId id, std::shared_ptr<PenGroup> pen);
std::unique_ptr<ToolHandler> makePathSelectionToolHandler(ToolId id,
                                                          std::shared_ptr<PenGroup> pen);
std::unique_ptr<ToolHandler> makeShapeToolHandler(ToolId id, std::shared_ptr<ShapeGroup> shape);
std::unique_ptr<ToolHandler> makeRotateViewToolHandler();
std::unique_ptr<ToolHandler> makeTypeToolHandler(ToolId id, std::shared_ptr<TypeGroup> type);

ToolController::ToolController(QObject* parent)
    : QObject(parent)
{
    // The family option groups. Each handler holds the shared_ptr for its
    // family; the raw pointers below are the controller's non-owning references
    // for signal wiring and its own reads.
    auto paint = std::make_shared<PaintOptions>();
    auto selection = std::make_shared<SelectionOptions>();
    auto crop = std::make_shared<CropOptions>();
    auto clone = std::make_shared<CloneSourceOptions>();
    auto move = std::make_shared<MoveGroup>();
    auto type = std::make_shared<TypeGroup>();
    auto mixer = std::make_shared<MixerGroup>();
    auto eyedropper = std::make_shared<EyedropperGroup>();
    auto pen = std::make_shared<PenGroup>();
    auto shape = std::make_shared<ShapeGroup>();
    auto healing = std::make_shared<HealingGroup>();
    auto patch = std::make_shared<PatchGroup>();
    auto contentAware = std::make_shared<ContentAwareMoveGroup>();
    auto redEye = std::make_shared<RedEyeGroup>();
    auto colorReplace = std::make_shared<ColorReplaceGroup>();
    auto stamp = std::make_shared<StampGroup>();
    auto eraser = std::make_shared<EraserGroup>();
    auto backgroundErase = std::make_shared<BackgroundEraseGroup>();
    auto magicErase = std::make_shared<MagicEraseGroup>();
    auto artHistory = std::make_shared<ArtHistoryGroup>();
    auto gradient = std::make_shared<GradientGroup>();
    auto bucket = std::make_shared<BucketGroup>();
    std::array<std::shared_ptr<RetouchGroup>, 3> retouch{
        std::make_shared<RetouchGroup>(), std::make_shared<RetouchGroup>(),
        std::make_shared<RetouchGroup>()};
    std::array<std::shared_ptr<ToneGroup>, 3> tone{
        std::make_shared<ToneGroup>(), std::make_shared<ToneGroup>(),
        std::make_shared<ToneGroup>()};

    paint_ = paint.get();
    selection_ = selection.get();
    crop_ = crop.get();
    clone_ = clone.get();
    move_ = move.get();
    type_ = type.get();
    mixer_ = mixer.get();
    eyedropper_ = eyedropper.get();
    pen_ = pen.get();
    shape_ = shape.get();

    // Wire the groups to the controller's signals and handler hooks, so every
    // consumer keeps following an option change.
    paint->sizeChanged = [this](int size) { emit brushSizeChanged(size); };
    paint->tipChanged = [this]() { emit brushTipChanged(); };
    selection->magneticWidthChanged = [this](int width) { emit magneticWidthChanged(width); };
    crop->optionsChanged = [this]() {
        if (ToolHandler* h = registry_.forTool(ToolId::Crop)) {
            h->onOptionsChanged(*this);
        }
        emit cropOptionsChanged();
    };
    crop->angleChanged = [this]() {
        if (CropBoxSession* box = cropBoxSession()) {
            crop_->setPivot(box->cropCenter());
        }
        if (canvas_) {
            canvas_->setCropStraighten(crop_->angle(), crop_->pivot());
        }
    };
    crop->straightenChanged = [this]() {
        if (CropBoxSession* box = cropBoxSession()) {
            box->setStraightenMode(crop_->straightenMode());
        }
        refreshCursor();
        emit cropOptionsChanged();
    };
    type->changed = [this]() {
        if (ToolHandler* h = registry_.forTool(active_)) {
            h->onOptionsChanged(*this);
        }
        emit typeOptionsChanged();
    };
    pen->changed = [this]() {
        if (ToolHandler* h = registry_.forTool(active_)) {
            h->onOptionsChanged(*this);
        }
    };
    shape->changed = [this]() {
        if (ToolHandler* h = registry_.forTool(active_)) {
            h->onOptionsChanged(*this);
        }
        emit shapeOptionsChanged();
    };
    clone->changed = [this]() { emit cloneSourceChanged(); };
    move->changed = [this]() {
        emit moveOptionsChanged();
        syncTransformControls();
    };
    eyedropper->changed = [this]() {
        if (!eyedropper_->get().ring && canvas_ && active_ == ToolId::Eyedropper) {
            canvas_->clearSamplingRing();
        }
    };
    mixer->changed = [this]() { emit mixerReservoirChanged(mixer_->get().reservoir); };

    registry_.registerTool(ToolId::Hand, makeHandToolHandler());
    registry_.registerTool(ToolId::RotateView, makeRotateViewToolHandler());
    registry_.registerTool(ToolId::Zoom, makeZoomToolHandler());
    registry_.registerTool(ToolId::Eyedropper, makeEyedropperToolHandler(eyedropper));
    registry_.registerTool(ToolId::Brush, makeBrushToolHandler(false, paint));
    registry_.registerTool(ToolId::Pencil, makeBrushToolHandler(true, paint));
    registry_.registerTool(ToolId::MagicWand, makeMagicWandToolHandler(selection));
    registry_.registerTool(ToolId::QuickSelection, makeQuickSelectionToolHandler(selection));
    registry_.registerTool(ToolId::Move, makeMoveToolHandler(move));
    registry_.registerTool(ToolId::Crop, makeCropToolHandler(crop));
    registry_.registerTool(ToolId::PerspectiveCrop, makePerspectiveCropToolHandler(crop));
    registry_.registerTool(ToolId::Slice, makeSliceToolHandler());
    registry_.registerTool(ToolId::SliceSelect, makeSliceSelectToolHandler());
    registry_.registerTool(ToolId::Marquee, makeMarqueeToolHandler(selection));
    registry_.registerTool(ToolId::EllipticalMarquee, makeEllipticalMarqueeToolHandler(selection));
    registry_.registerTool(ToolId::Lasso, makeLassoToolHandler(selection));
    registry_.registerTool(ToolId::PolygonalLasso, makePolygonalLassoToolHandler(selection));
    registry_.registerTool(ToolId::MagneticLasso, makeMagneticLassoToolHandler(selection));
    registry_.registerTool(ToolId::ColorSampler, makeColorSamplerToolHandler());
    registry_.registerTool(ToolId::Ruler, makeRulerToolHandler());
    registry_.registerTool(ToolId::Note, makeNoteToolHandler());
    registry_.registerTool(ToolId::Count, makeCountToolHandler());
    registry_.registerTool(ToolId::SpotHealingBrush,
                           makeSpotHealingToolHandler(paint, healing));
    registry_.registerTool(ToolId::HealingBrush, makeHealingToolHandler(paint, healing));
    registry_.registerTool(ToolId::Patch, makePatchToolHandler(paint, patch));
    registry_.registerTool(ToolId::ContentAwareMove,
                           makeContentAwareMoveToolHandler(contentAware));
    registry_.registerTool(ToolId::RedEye, makeRedEyeToolHandler(paint, redEye));
    registry_.registerTool(ToolId::ColorReplacement,
                           makeColorReplacementToolHandler(paint, colorReplace));
    registry_.registerTool(ToolId::MixerBrush, makeMixerBrushToolHandler(paint, mixer));
    registry_.registerTool(ToolId::CloneStamp,
                           makeCloneStampToolHandler(paint, stamp, clone));
    registry_.registerTool(ToolId::PatternStamp, makePatternStampToolHandler(paint, stamp));
    registry_.registerTool(ToolId::HistoryBrush, makeHistoryBrushToolHandler(paint));
    registry_.registerTool(ToolId::ArtHistoryBrush,
                           makeArtHistoryBrushToolHandler(paint, artHistory));
    registry_.registerTool(ToolId::Eraser, makeEraserToolHandler(paint, eraser));
    registry_.registerTool(ToolId::BackgroundEraser,
                           makeBackgroundEraserToolHandler(paint, backgroundErase));
    registry_.registerTool(ToolId::MagicEraser, makeMagicEraserToolHandler(paint, magicErase));
    registry_.registerTool(ToolId::Gradient, makeGradientToolHandler(gradient));
    registry_.registerTool(ToolId::PaintBucket, makePaintBucketToolHandler(bucket));
    for (ToolId id : {ToolId::Blur, ToolId::Sharpen, ToolId::Smudge, ToolId::Dodge, ToolId::Burn,
                      ToolId::Sponge}) {
        const size_t slot = id == ToolId::Sharpen || id == ToolId::Burn
            ? 1
            : (id == ToolId::Smudge || id == ToolId::Sponge ? 2 : 0);
        registry_.registerTool(id, makeRetouchToolHandler(id, paint, retouch[slot], tone[slot]));
    }
    for (ToolId id : {ToolId::Pen, ToolId::FreeformPen, ToolId::AddAnchorPoint,
                      ToolId::DeleteAnchorPoint, ToolId::ConvertPoint}) {
        registry_.registerTool(id, makePenToolHandler(id, pen));
    }
    for (ToolId id : {ToolId::PathSelection, ToolId::DirectSelection}) {
        registry_.registerTool(id, makePathSelectionToolHandler(id, pen));
    }
    for (ToolId id : {ToolId::Rectangle, ToolId::RoundedRectangle, ToolId::Ellipse,
                      ToolId::Polygon, ToolId::Line, ToolId::CustomShape}) {
        registry_.registerTool(id, makeShapeToolHandler(id, shape));
    }
    for (ToolId id : {ToolId::HorizontalType, ToolId::VerticalType, ToolId::HorizontalTypeMask,
                      ToolId::VerticalTypeMask}) {
        registry_.registerTool(id, makeTypeToolHandler(id, type));
    }
    // A size change from the options bar or `[`/`]` moves the hover ring at
    // once. Query the pointer so a stale position is never reused after leave.
    connect(this, &ToolController::brushSizeChanged, this, [this](int size) {
        if (!canvas_ || !isBrushTool(active_)) {
            return;
        }
        const QPoint local = canvas_->mapFromGlobal(QCursor::pos());
        if (canvas_->rect().contains(local)) {
            canvas_->setBrushOutline(size, canvas_->widgetToImage(QPointF(local)));
        }
    });
}

void ToolController::setActiveTool(ToolId id)
{
    if (!toolImplemented(id)) {
        return;
    }
    if (active_ == id) {
        return;
    }
    // Drop the outgoing handler's per-tool state (in-progress polygon, warm
    // preview) before the switch, while the old tool is still active.
    if (ToolHandler* old = registry_.forTool(active_)) {
        old->onDeactivate(*this);
    }
    active_ = id;
    PictureView* v = view();
    if (v && v->is_painting()) {
        v->cancel_paint();
    }
    dragging_ = false;
    dragCommitted_ = false;
    cancelSelectionMove();
    cursorOverSelection_ = false;
    if (canvas_ && canvas_->movePreviewActive()) {
        canvas_->endMovePreview();
        if (v) {
            v->end_move_preview();
        }
    }
    if (v && v->transform_session_active()) {
        v->cancel_transform();
        transformDragging_ = false;
        transformHandle_ = -1;
        if (canvas_) {
            canvas_->clearTransformPreview();
        }
        emit transformChanged();
    }
    if (canvas_) {
        canvas_->clearOverlay();
        canvas_->clearSelectionPreview();
    }
    applyToolPolicy();
    emit activeToolChanged(id);
}

CropCommitSession* ToolController::cropCommitSession() const
{
    // Crop's staged box survives a tool switch, so commit reaches the Crop
    // handler unless Perspective Crop is active (its quad lives only then).
    const ToolId target = active_ == ToolId::PerspectiveCrop ? ToolId::PerspectiveCrop : ToolId::Crop;
    return dynamic_cast<CropCommitSession*>(registry_.forTool(target));
}

CropBoxSession* ToolController::cropBoxSession() const
{
    return dynamic_cast<CropBoxSession*>(registry_.forTool(ToolId::Crop));
}

PerspectiveCropSession* ToolController::perspectiveCropSession() const
{
    return dynamic_cast<PerspectiveCropSession*>(registry_.forTool(ToolId::PerspectiveCrop));
}

TextSession* ToolController::textSession() const
{
    return dynamic_cast<TextSession*>(registry_.forTool(active_));
}

double ToolController::viewRotation() const
{
    return canvas_ ? canvas_->rotation() : 0.0;
}

void ToolController::setViewRotation(double degrees)
{
    if (!canvas_) {
        return;
    }
    canvas_->setRotation(degrees);
    emit viewRotationChanged(canvas_->rotation());
}

void ToolController::notifyToolSessionChanged()
{
    emit toolSessionChanged();
}

void ToolController::applyCropOptions(bool classic, bool deletePixels, double ratio, int grid)
{
    crop_->apply(classic, deletePixels, ratio, grid);
    if (ToolHandler* h = registry_.forTool(ToolId::Crop)) {
        h->onOptionsChanged(*this);
    }
}

int ToolController::cropCursorKindForTest(const QPointF& imagePos, int mods)
{
    if (ToolHandler* h = registry_.forTool(ToolId::Crop)) {
        return h->pointerCursorKind(*this, imagePos, Qt::KeyboardModifiers(mods));
    }
    return -1;
}

QPointF ToolController::cropCenterForTest()
{
    if (CropBoxSession* box = cropBoxSession()) {
        return box->cropCenter();
    }
    return QPointF();
}

void ToolController::updateSamplingRing(const QPointF& imagePos)
{
    if (!canvas_) {
        return;
    }
    const EyedropperOptions options = eyedropper_->get();
    if (active_ == ToolId::Eyedropper && options.ring) {
        canvas_->setSamplingRing(options.sampleSize, imagePos);
    } else {
        canvas_->clearSamplingRing();
    }
}

QColor ToolController::foreground() const { return foreground_; }

void ToolController::setForeground(const QColor& color)
{
    foreground_ = color;
    // Choosing a foreground colour loads the Mixer Brush (docs/03-tools/mixer-brush.md).
    MixerOptions mixer = mixer_->get();
    if (mixer.reservoir.rgba() != color.rgba()) {
        mixer.reservoir = color;
        mixer_->set(mixer);
    }
    emit colorsChanged();
    // ...and is the text colour, so it restyles text being typed or a selected
    // type layer, as CS6's does with the Type tool active.
    TypeOptions options = type_->get();
    if (options.color != color) {
        options.color = color;
        type_->set(options);
    }
}

QColor ToolController::background() const { return background_; }

void ToolController::setBackground(const QColor& color)
{
    background_ = color;
    updateCanvasBackdrop();
    emit colorsChanged();
}

void ToolController::updateCanvasBackdrop()
{
    if (!canvas_) {
        return;
    }
    PictureView* v = view();
    const bool background =
        v && v->has_document() && v->layer_kind(0) == QStringLiteral("background");
    canvas_->setCanvasBackdrop(background_, background);
}

void ToolController::adjustBrushSize(int delta) { paint_->setSize(paint_->size() + delta); }

void ToolController::adjustBrushHardness(int delta)
{
    paint_->setHardness(paint_->hardness() + delta);
}

bool ToolController::applyBrushShortcut(int key, quint32 nativeScanCode, bool shift)
{
    PictureView* v = view();
    if (!v) {
        return false;
    }
    if (active_ == ToolId::MagneticLasso) {
        // `[` / `]` step the detection width by 1 px (Shift has no meaning here).
        const int step = v->brush_shortcut_delta(key, nativeScanCode, false, true);
        if (step == 0) {
            return false;
        }
        selection_->setMagneticWidth(selection_->magneticWidth() + (step > 0 ? 1 : -1));
        return true;
    }
    const bool paint = isBrushTool(active_);
    const int delta = v->brush_shortcut_delta(key, nativeScanCode, shift, paint);
    if (delta == 0) {
        return false;
    }
    // Magnitude 1 is a diameter step, magnitude 5 a hardness step.
    if (std::abs(delta) >= 5) {
        adjustBrushHardness(delta);
    } else {
        adjustBrushSize(delta);
    }
    return true;
}

void ToolController::setViewProvider(std::function<PictureView*()> provider)
{
    viewProvider_ = std::move(provider);
}

PictureView* ToolController::view() const
{
    return viewProvider_ ? viewProvider_() : nullptr;
}

void ToolController::sampledForeground(const QColor& color)
{
    emit foregroundSampled(color);
}

SelectionMode ToolController::resolveSelectionMode(Qt::KeyboardModifiers mods,
                                                   bool hasExistingSelection) const
{
    return selectionModeForModifiers(selection_->combineMode(), mods, hasExistingSelection);
}

void ToolController::refused(const QString& message) { emit pixelEditRefused(message); }

void ToolController::emitSelectionCommitted() { emit selectionCommitted(); }

void ToolController::bindCanvas(ImageView* canvas)
{
    if (canvas_ == canvas) {
        // The canvas pointer is unchanged, but the active layer's visibility or
        // identity may have changed under it; re-assert the cursor without
        // waiting for the next mouse move.
        if (!transformSessionActive()) {
            refreshCursor();
        }
        updateCanvasBackdrop();
        if (ToolHandler* h = registry_.forTool(active_)) {
            h->onDocumentRefreshed(*this);
        }
        refreshAnnotations();
        // Every refresh reaches here: keep the Move tool's controls box on the
        // active layer (or rebuild it after a content change).
        syncTransformControls();
        return;
    }
    unbindCanvas();
    canvas_ = canvas;
    // The current note belongs to the previous document.
    setCurrentNote(-1);
    if (!canvas_) {
        return;
    }
    connect(canvas_, &ImageView::mousePressed, this, &ToolController::handlePressed);
    connect(canvas_, &ImageView::mouseMoved, this, &ToolController::handleMoved);
    connect(canvas_, &ImageView::modifierKeyChanged, this, &ToolController::handleModifiers);
    connect(canvas_, &ImageView::mouseReleased, this, &ToolController::handleReleased);
    connect(canvas_, &ImageView::transformCommitRequested, this,
            &ToolController::commitFreeTransform);
    connect(canvas_, &ImageView::transformCancelRequested, this,
            &ToolController::cancelFreeTransform);
    applyToolPolicy();
    updateCanvasBackdrop();
    if (ToolHandler* h = registry_.forTool(active_)) {
        h->onDocumentRefreshed(*this);
    }
    refreshAnnotations();
}

// Color samplers, notes, and Count marks are shown with every tool, as CS6's
// Extras are.
void ToolController::refreshAnnotations()
{
    if (!canvas_) {
        return;
    }
    PictureView* v = view();
    QList<QPointF> lists[2];
    for (int kind = 0; kind < 2; ++kind) {
        const int count = v && v->has_document() ? marker_count(*v, kind) : 0;
        for (int i = 0; i < count; ++i) {
            const ::rust::Vec<std::int32_t> p = marker_at(*v, kind, i);
            if (p.size() == 2) {
                lists[kind].append(QPointF(p[0], p[1]));
            }
        }
    }
    // Undo can remove the current note.
    if (currentNote_ >= lists[1].size()) {
        setCurrentNote(-1);
    }
    canvas_->setAnnotationOverlay(lists[0], lists[1], currentNote_);

    // Count marks, flattened from the visible groups.
    QList<ImageView::CountOverlayMark> counts;
    if (v && v->has_document()) {
        const int groups = count_group_count(*v);
        for (int g = 0; g < groups; ++g) {
            if (!count_group_visible(*v, g)) {
                continue;
            }
            const QColor color(QRgb(count_group_color(*v, g)));
            const int markerSize = count_group_marker_size(*v, g);
            const int labelSize = count_group_label_size(*v, g);
            const int total = count_group_total(*v, g);
            for (int i = 0; i < total; ++i) {
                const ::rust::Vec<std::int32_t> p = count_group_mark_at(*v, g, i);
                if (p.size() == 2) {
                    counts.append({QPointF(p[0], p[1]), i + 1, color, markerSize, labelSize});
                }
            }
        }
    }
    canvas_->setCountOverlay(counts);
    refreshGuides();
}

void ToolController::setCurrentNote(int index)
{
    if (currentNote_ == index) {
        return;
    }
    currentNote_ = index;
    refreshAnnotations();
    emit noteActivated(index);
}

bool ToolController::clearAnnotations()
{
    ToolHandler* h = registry_.forTool(active_);
    return h && h->clearAnnotations();
}

void ToolController::unbindCanvas()
{
    if (!canvas_) {
        return;
    }
    disconnect(canvas_, nullptr, this, nullptr);
    // Let the active handler release anything tied to this canvas (an
    // in-progress polygon, the warm Move preview) before it goes away.
    if (ToolHandler* h = registry_.forTool(active_)) {
        h->onDeactivate(*this);
    }
    if (canvas_->movePreviewActive()) {
        canvas_->endMovePreview();
        PictureView* previewView = view();
        if (previewView) {
            previewView->end_move_preview();
        }
    }
    if (canvas_->transformPreviewActive()) {
        canvas_->clearTransformPreview();
    }
    transformDragging_ = false;
    transformHandle_ = -1;
    guideDrag_ = -1;
    guideHover_ = false;
    canvas_->clearOverlay();
    canvas_->clearSelectionPreview();
    canvas_->clearBrushOutline();
    canvas_->clearSamplingRing();
    canvas_ = nullptr;
    PictureView* v = view();
    if (v && v->is_painting()) {
        v->cancel_paint();
    }
    dragging_ = false;
    dragCommitted_ = false;
    cancelSelectionMove();
    cursorOverSelection_ = false;
}

void ToolController::applyToolPolicy()
{
    if (!canvas_) {
        return;
    }
    const bool session = transformSessionActive();
    canvas_->setPanEnabled(!session && active_ == ToolId::Hand);
    if (!isBrushTool(active_)) {
        canvas_->clearBrushOutline();
    }
    if (active_ != ToolId::Eyedropper) {
        canvas_->clearSamplingRing();
    }
    // Keep the Move tool's controls box in step with the tool and checkbox even
    // while a session is up.
    syncTransformControls();
    if (session) {
        return;
    }
    refreshCursor();
    if (ToolHandler* h = registry_.forTool(active_)) {
        h->onActivate(*this);
    }
}

void ToolController::setCanvasSampler(std::function<void(const QPointF&)> sampler)
{
    canvasSampler_ = std::move(sampler);
    refreshCursor();
}

void ToolController::handlePressed(const QPointF& imagePos, int button, int modifiers)
{
    if (button != Qt::LeftButton) {
        return;
    }
    if (canvasSampler_) {
        canvasSampler_(imagePos);
        return;
    }
    PictureView* v = view();
    const Qt::KeyboardModifiers mods = Qt::KeyboardModifiers(modifiers);
    // The idle controls box: a handle starts its session for this gesture;
    // anywhere else is an ordinary Move drag, the box hidden until release.
    if (v && controlsIdle_ && !v->transform_session_active()) {
        // The active layer can change without a refresh (a direct
        // `set_active_layer`); never grab a stale box.
        syncTransformControls();
    }
    if (v && controlsIdle_ && !v->transform_session_active()) {
        const int hit = controlsHit(imagePos);
        if (hit >= 0 && hit != 9) {
            beginControlsGesture(v);
        } else {
            clearControlsBox();
        }
    }
    if (v && v->transform_session_active()) {
        const double z = canvas_ ? canvas_->zoom() : 1.0;
        const int hit = v->transform_press(imagePos.x(), imagePos.y(), z,
                                           mods.testFlag(Qt::ShiftModifier),
                                           mods.testFlag(Qt::AltModifier));
        if (transformControls_ && (hit < 0 || hit == 9)) {
            // Inside or outside the Move tool's controls box: an ordinary Move
            // drag. Suspend the box for the drag; the release path brings it
            // back on the moved layer.
            v->transform_release();
            transformControls_ = false;
            v->cancel_transform();
            if (canvas_) {
                canvas_->clearTransformPreview();
            }
        } else {
            if (hit >= 0) {
                transformDragging_ = true;
                transformHandle_ = hit;
                updateTransformOverlay(v);
                setTransformCursor(imagePos);
                emit transformChanged();
            }
            return;
        }
    }
    if (beginGuideDrag(imagePos, mods)) {
        return;
    }
    if (active_ == ToolId::Hand && canvas_ && canvas_->panEnabled()) {
        canvas_->beginPan(canvas_->imageToWidget(imagePos));
        return;
    }
    ToolHandler* handler = registry_.forTool(active_);
    if (isSelectionTool(active_) && !(handler && handler->gestureInProgress())) {
        // A press inside a live selection moves the mask or its content instead
        // of starting a new shape; the selection handler never sees the event.
        const bool ctrl = mods.testFlag(Qt::ControlModifier);
        const bool shift = mods.testFlag(Qt::ShiftModifier);
        const bool alt = mods.testFlag(Qt::AltModifier);
        const bool inside = v && v->has_selection()
            && v->selection_coverage(qRound(imagePos.x()), qRound(imagePos.y())) > 0;
        if (ctrl && inside) {
            beginContentMove(v, imagePos, alt);
            return;
        }
        if (!shift && !alt && maybeBeginSelectionMove(v, imagePos)) {
            return;
        }
    }

    if (handler) {
        handler->onPress(*this, imagePos, mods);
        if (handler->pointerCursor()) {
            refreshCursor();
        }
    }
}

void ToolController::handleMoved(const QPointF& imagePos)
{
    if (canvasSampler_) {
        refreshCursor();
        return;
    }
    if (transformSessionActive()) {
        PictureView* v = view();
        const Qt::KeyboardModifiers mods = QGuiApplication::queryKeyboardModifiers();
        if (transformDragging_ && v && canvas_) {
            v->transform_move(imagePos.x(), imagePos.y(), canvas_->zoom(),
                              mods.testFlag(Qt::ShiftModifier), mods.testFlag(Qt::AltModifier));
            updateTransformOverlay(v);
            emit transformChanged();
        }
        setTransformCursor(imagePos);
        return;
    }
    if (guideDrag_ >= 0) {
        dragGuide(imagePos);
        return;
    }
    // Over an idle controls box's handle, the handle's cursor; elsewhere the
    // Move tool's own.
    if (controlsIdle_ && canvas_ && !dragging_) {
        const int hit = controlsHit(imagePos);
        if (hit >= 0 && hit != 9) {
            setHandleCursor(hit, imagePos, QRectF(controlsBox_).adjusted(0, 0, 1, 1).center());
            controlsHandleCursor_ = true;
            return;
        }
        if (controlsHandleCursor_) {
            controlsHandleCursor_ = false;
            refreshCursor();
        }
    }
    updateGuideHover(imagePos);
    updateSelectionHover(imagePos);
    updateBrushOutline(imagePos);
    updateSamplingRing(imagePos);
    // The selection/content move is cross-cutting: route it before the active
    // tool handler so the shape tools do not also consume the move.
    if (movingSelection_) {
        dragSelectionMove(imagePos);
        return;
    }
    if (ToolHandler* h = registry_.forTool(active_)) {
        h->onMove(*this, imagePos, QGuiApplication::queryKeyboardModifiers());
        if (h->pointerCursor() || h->cursorVariant() != cursorVariant_) {
            refreshCursor();
        }
    }
}

void ToolController::handleModifiers(int modifiers)
{
    if (canvasSampler_ || transformSessionActive() || movingSelection_) {
        return;
    }
    if (ToolHandler* h = registry_.forTool(active_)) {
        h->onModifiers(*this, Qt::KeyboardModifiers(modifiers));
    }
}

void ToolController::handleReleased(const QPointF& imagePos)
{
    if (canvasSampler_) {
        return;
    }
    if (transformSessionActive()) {
        if (transformDragging_) {
            transformDragging_ = false;
            transformHandle_ = -1;
            PictureView* v = view();
            if (v) {
                v->transform_release();
                if (transformControls_) {
                    // The controls box applies each gesture on release and stays
                    // on the resulting layer.
                    transformControls_ = false;
                    v->commit_transform();
                    if (canvas_) {
                        canvas_->clearTransformPreview();
                    }
                    syncTransformControls();
                }
            }
        }
        return;
    }
    if (guideDrag_ >= 0) {
        releaseGuide(imagePos);
        return;
    }
    if (movingSelection_) {
        dragging_ = false;
        releaseSelectionMove(imagePos);
        return;
    }
    if (ToolHandler* h = registry_.forTool(active_)) {
        h->onRelease(*this, imagePos, QGuiApplication::queryKeyboardModifiers());
        if (h->pointerCursor() || h->cursorVariant() != cursorVariant_) {
            refreshCursor();
        }
    }
    // A suspended controls box (an inside/outside Move drag) returns here, even
    // when the drag changed nothing.
    syncTransformControls();
}

bool ToolController::handleFinishPath()
{
    if (ToolHandler* h = registry_.forTool(active_)) {
        return h->onFinishPath();
    }
    return false;
}

bool ToolController::handleEscape()
{
    if (ToolHandler* h = registry_.forTool(active_)) {
        return h->onEscape();
    }
    return false;
}

bool ToolController::handleDeletePoint()
{
    if (ToolHandler* h = registry_.forTool(active_)) {
        return h->onDeletePoint();
    }
    return false;
}

bool ToolController::handleNudgeKey(int key, Qt::KeyboardModifiers mods)
{
    int dx = 0;
    int dy = 0;
    switch (key) {
    case Qt::Key_Left: dx = -1; break;
    case Qt::Key_Right: dx = 1; break;
    case Qt::Key_Up: dy = -1; break;
    case Qt::Key_Down: dy = 1; break;
    default: return false;
    }
    const int step = mods.testFlag(Qt::ShiftModifier) ? 10 : 1;
    if (ToolHandler* h = registry_.forTool(active_); h && h->nudge(dx * step, dy * step)) {
        return true;
    }
    if (active_ == ToolId::Move && view()) {
        view()->translate_layer(dx * step, dy * step);
        return true;
    }
    return false;
}

bool ToolController::toolUndo()
{
    if (ToolHandler* h = registry_.forTool(active_)) {
        return h->toolUndo();
    }
    return false;
}

bool ToolController::toolRedo()
{
    if (ToolHandler* h = registry_.forTool(active_)) {
        return h->toolRedo();
    }
    return false;
}

bool ToolController::canToolUndo() const
{
    ToolHandler* h = registry_.forTool(active_);
    return h && h->canToolUndo();
}

bool ToolController::canToolRedo() const
{
    ToolHandler* h = registry_.forTool(active_);
    return h && h->canToolRedo();
}

} // namespace pictura
