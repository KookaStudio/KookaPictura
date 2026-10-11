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
// only the registry.
std::unique_ptr<ToolHandler> makeHandToolHandler();
std::unique_ptr<ToolHandler> makeZoomToolHandler();
std::unique_ptr<ToolHandler> makeEyedropperToolHandler();
std::unique_ptr<ToolHandler> makeBrushToolHandler(bool aliased);
std::unique_ptr<ToolHandler> makeMagicWandToolHandler();
std::unique_ptr<ToolHandler> makeQuickSelectionToolHandler();
std::unique_ptr<ToolHandler> makeMoveToolHandler();
std::unique_ptr<ToolHandler> makeCropToolHandler();
std::unique_ptr<ToolHandler> makePerspectiveCropToolHandler();
std::unique_ptr<ToolHandler> makeSliceToolHandler();
std::unique_ptr<ToolHandler> makeSliceSelectToolHandler();
std::unique_ptr<ToolHandler> makeMarqueeToolHandler();
std::unique_ptr<ToolHandler> makeEllipticalMarqueeToolHandler();
std::unique_ptr<ToolHandler> makeLassoToolHandler();
std::unique_ptr<ToolHandler> makePolygonalLassoToolHandler();
std::unique_ptr<ToolHandler> makeMagneticLassoToolHandler();
std::unique_ptr<ToolHandler> makeColorSamplerToolHandler();
std::unique_ptr<ToolHandler> makeRulerToolHandler();
std::unique_ptr<ToolHandler> makeNoteToolHandler();
std::unique_ptr<ToolHandler> makeCountToolHandler();
std::unique_ptr<ToolHandler> makeSpotHealingToolHandler();
std::unique_ptr<ToolHandler> makeHealingToolHandler();
std::unique_ptr<ToolHandler> makePatchToolHandler();
std::unique_ptr<ToolHandler> makeContentAwareMoveToolHandler();
std::unique_ptr<ToolHandler> makeRedEyeToolHandler();
std::unique_ptr<ToolHandler> makeColorReplacementToolHandler();
std::unique_ptr<ToolHandler> makeMixerBrushToolHandler();
std::unique_ptr<ToolHandler> makeCloneStampToolHandler();
std::unique_ptr<ToolHandler> makePatternStampToolHandler();
std::unique_ptr<ToolHandler> makeHistoryBrushToolHandler();
std::unique_ptr<ToolHandler> makeArtHistoryBrushToolHandler();
std::unique_ptr<ToolHandler> makeEraserToolHandler();
std::unique_ptr<ToolHandler> makeBackgroundEraserToolHandler();
std::unique_ptr<ToolHandler> makeMagicEraserToolHandler();
std::unique_ptr<ToolHandler> makeGradientToolHandler();
std::unique_ptr<ToolHandler> makePaintBucketToolHandler();
std::unique_ptr<ToolHandler> makeRetouchToolHandler(ToolId id);
std::unique_ptr<ToolHandler> makePenToolHandler(ToolId id);
std::unique_ptr<ToolHandler> makePathSelectionToolHandler(ToolId id);
std::unique_ptr<ToolHandler> makeShapeToolHandler(ToolId id);
std::unique_ptr<ToolHandler> makeRotateViewToolHandler();
std::unique_ptr<ToolHandler> makeTypeToolHandler(ToolId id);

ToolController::ToolController(QObject* parent)
    : QObject(parent)
{
    registry_.registerTool(ToolId::Hand, makeHandToolHandler());
    registry_.registerTool(ToolId::RotateView, makeRotateViewToolHandler());
    registry_.registerTool(ToolId::Zoom, makeZoomToolHandler());
    registry_.registerTool(ToolId::Eyedropper, makeEyedropperToolHandler());
    registry_.registerTool(ToolId::Brush, makeBrushToolHandler(false));
    registry_.registerTool(ToolId::Pencil, makeBrushToolHandler(true));
    registry_.registerTool(ToolId::MagicWand, makeMagicWandToolHandler());
    registry_.registerTool(ToolId::QuickSelection, makeQuickSelectionToolHandler());
    registry_.registerTool(ToolId::Move, makeMoveToolHandler());
    registry_.registerTool(ToolId::Crop, makeCropToolHandler());
    registry_.registerTool(ToolId::PerspectiveCrop, makePerspectiveCropToolHandler());
    registry_.registerTool(ToolId::Slice, makeSliceToolHandler());
    registry_.registerTool(ToolId::SliceSelect, makeSliceSelectToolHandler());
    registry_.registerTool(ToolId::Marquee, makeMarqueeToolHandler());
    registry_.registerTool(ToolId::EllipticalMarquee, makeEllipticalMarqueeToolHandler());
    registry_.registerTool(ToolId::Lasso, makeLassoToolHandler());
    registry_.registerTool(ToolId::PolygonalLasso, makePolygonalLassoToolHandler());
    registry_.registerTool(ToolId::MagneticLasso, makeMagneticLassoToolHandler());
    registry_.registerTool(ToolId::ColorSampler, makeColorSamplerToolHandler());
    registry_.registerTool(ToolId::Ruler, makeRulerToolHandler());
    registry_.registerTool(ToolId::Note, makeNoteToolHandler());
    registry_.registerTool(ToolId::Count, makeCountToolHandler());
    registry_.registerTool(ToolId::SpotHealingBrush, makeSpotHealingToolHandler());
    registry_.registerTool(ToolId::HealingBrush, makeHealingToolHandler());
    registry_.registerTool(ToolId::Patch, makePatchToolHandler());
    registry_.registerTool(ToolId::ContentAwareMove, makeContentAwareMoveToolHandler());
    registry_.registerTool(ToolId::RedEye, makeRedEyeToolHandler());
    registry_.registerTool(ToolId::ColorReplacement, makeColorReplacementToolHandler());
    registry_.registerTool(ToolId::MixerBrush, makeMixerBrushToolHandler());
    registry_.registerTool(ToolId::CloneStamp, makeCloneStampToolHandler());
    registry_.registerTool(ToolId::PatternStamp, makePatternStampToolHandler());
    registry_.registerTool(ToolId::HistoryBrush, makeHistoryBrushToolHandler());
    registry_.registerTool(ToolId::ArtHistoryBrush, makeArtHistoryBrushToolHandler());
    registry_.registerTool(ToolId::Eraser, makeEraserToolHandler());
    registry_.registerTool(ToolId::BackgroundEraser, makeBackgroundEraserToolHandler());
    registry_.registerTool(ToolId::MagicEraser, makeMagicEraserToolHandler());
    registry_.registerTool(ToolId::Gradient, makeGradientToolHandler());
    registry_.registerTool(ToolId::PaintBucket, makePaintBucketToolHandler());
    for (ToolId id : {ToolId::Blur, ToolId::Sharpen, ToolId::Smudge, ToolId::Dodge, ToolId::Burn,
                      ToolId::Sponge}) {
        registry_.registerTool(id, makeRetouchToolHandler(id));
    }
    for (ToolId id : {ToolId::Pen, ToolId::FreeformPen, ToolId::AddAnchorPoint,
                      ToolId::DeleteAnchorPoint, ToolId::ConvertPoint}) {
        registry_.registerTool(id, makePenToolHandler(id));
    }
    for (ToolId id : {ToolId::PathSelection, ToolId::DirectSelection}) {
        registry_.registerTool(id, makePathSelectionToolHandler(id));
    }
    for (ToolId id : {ToolId::Rectangle, ToolId::RoundedRectangle, ToolId::Ellipse,
                      ToolId::Polygon, ToolId::Line, ToolId::CustomShape}) {
        registry_.registerTool(id, makeShapeToolHandler(id));
    }
    for (ToolId id : {ToolId::HorizontalType, ToolId::VerticalType, ToolId::HorizontalTypeMask,
                      ToolId::VerticalTypeMask}) {
        registry_.registerTool(id, makeTypeToolHandler(id));
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

void ToolController::setCombineMode(SelectionMode mode) { mode_ = mode; }

void ToolController::setTypeOptions(const TypeOptions& options)
{
    type_ = options;
    type_.size = std::clamp(options.size, 1.0, 1296.0);
    type_.justification = std::clamp(options.justification, 0, 2);
    emit typeOptionsChanged();
    if (ToolHandler* h = registry_.forTool(active_)) {
        h->onOptionsChanged(*this);
    }
}

bool ToolController::commitText()
{
    ToolHandler* h = registry_.forTool(active_);
    return h && h->commitText();
}

bool ToolController::cancelText()
{
    ToolHandler* h = registry_.forTool(active_);
    return h && h->cancelText();
}

bool ToolController::textActive() const
{
    ToolHandler* h = registry_.forTool(active_);
    return h && h->textActive();
}

bool ToolController::insertText(const QString& text)
{
    ToolHandler* h = registry_.forTool(active_);
    return h && h->insertText(text);
}

void ToolController::setPenOptions(const PenOptions& options)
{
    pen_ = options;
    pen_.curveFit = std::clamp(options.curveFit, 0.5, 10.0);
    if (ToolHandler* h = registry_.forTool(active_)) {
        h->onOptionsChanged(*this);
    }
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

void ToolController::setShapeOptions(const ShapeOptions& options)
{
    shape_ = options;
    shape_.mode = std::clamp(options.mode, 0, 2);
    shape_.radius = std::clamp(options.radius, 0.0, 1000.0);
    shape_.sides = std::clamp(options.sides, 3, 100);
    shape_.weight = std::clamp(options.weight, 1.0, 1000.0);
    shape_.arrowWidth = std::clamp(options.arrowWidth, 10.0, 1000.0);
    shape_.arrowLength = std::clamp(options.arrowLength, 10.0, 5000.0);
    shape_.arrowConcavity = std::clamp(options.arrowConcavity, -50.0, 50.0);
    shape_.custom = std::max(options.custom, 0);
    shape_.strokeWidth = std::clamp(options.strokeWidth, 1.0, 250.0);
    shape_.strokeAlign = std::clamp(options.strokeAlign, 0, 2);
    shape_.geometry = std::clamp(options.geometry, 0, 2);
    shape_.fixedWidth = std::max(options.fixedWidth, 0.01);
    shape_.fixedHeight = std::max(options.fixedHeight, 0.01);
    shape_.indent = std::clamp(options.indent, 1.0, 99.0);
    if (ToolHandler* h = registry_.forTool(active_)) {
        h->onOptionsChanged(*this);
    }
    emit shapeOptionsChanged();
}

void ToolController::setMarqueeStyle(MarqueeStyle style) { marqueeStyle_ = style; }

void ToolController::setFeather(double feather)
{
    feather_ = feather > 0.0 ? std::min(feather, 250.0) : 0.0;
}

void ToolController::setFixedRatio(double width, double height)
{
    fixedRatioW_ = width > 0.0 ? width : 1.0;
    fixedRatioH_ = height > 0.0 ? height : 1.0;
}

void ToolController::setFixedSize(int width, int height)
{
    fixedSizeW_ = std::max(width, 1);
    fixedSizeH_ = std::max(height, 1);
}

void ToolController::setCropRatio(double ratio)
{
    cropRatio_ = ratio > 0.0 ? ratio : 0.0;
    if (ToolHandler* h = registry_.forTool(ToolId::Crop)) {
        h->onOptionsChanged(*this);
    }
    emit cropOptionsChanged();
}

void ToolController::setCropAngle(double degrees)
{
    double a = degrees;
    while (a > 180.0) {
        a -= 360.0;
    }
    while (a < -180.0) {
        a += 360.0;
    }
    cropAngle_ = a;
    // The pivot is the crop box centre at the moment the angle is set; it stays
    // fixed afterwards so moving the box does not move the content.
    if (ToolHandler* h = registry_.forTool(ToolId::Crop)) {
        cropPivot_ = h->cropCenter();
    }
    if (canvas_) {
        canvas_->setCropStraighten(a, cropPivot_);
    }
}

void ToolController::notifyToolSessionChanged()
{
    emit toolSessionChanged();
}

void ToolController::setCropClassicMode(bool on)
{
    if (cropClassicMode_ == on) {
        return;
    }
    cropClassicMode_ = on;
    if (ToolHandler* h = registry_.forTool(ToolId::Crop)) {
        h->onOptionsChanged(*this);
    }
    emit cropOptionsChanged();
}

void ToolController::setCropGridOverlay(int index)
{
    if (cropGridOverlay_ == index) {
        return;
    }
    cropGridOverlay_ = index;
    if (ToolHandler* h = registry_.forTool(ToolId::Crop)) {
        h->onOptionsChanged(*this);
    }
    emit cropOptionsChanged();
}

void ToolController::setCropStraightenMode(bool on)
{
    cropStraightenMode_ = on;
    if (ToolHandler* h = registry_.forTool(ToolId::Crop)) {
        h->setStraightenMode(on);
    }
    refreshCursor();
    emit cropOptionsChanged();
}

void ToolController::notifyCropOptionsChanged()
{
    emit cropOptionsChanged();
}

void ToolController::applyCropOptions(bool classic, bool deletePixels, double ratio, int grid)
{
    cropClassicMode_ = classic;
    cropDeletePixels_ = deletePixels;
    cropRatio_ = ratio > 0.0 ? ratio : 0.0;
    cropGridOverlay_ = grid;
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
    if (ToolHandler* h = registry_.forTool(ToolId::Crop)) {
        return h->cropCenter();
    }
    return QPointF();
}

bool ToolController::resizeCrop(double width, double height)
{
    if (ToolHandler* h = registry_.forTool(ToolId::Crop)) {
        return h->resizeCrop(width, height);
    }
    return false;
}

double ToolController::cropWidth() const
{
    ToolHandler* h = registry_.forTool(ToolId::Crop);
    return h ? h->cropWidth() : 0.0;
}

double ToolController::cropHeight() const
{
    ToolHandler* h = registry_.forTool(ToolId::Crop);
    return h ? h->cropHeight() : 0.0;
}

void ToolController::cancelCrop()
{
    if (ToolHandler* h = registry_.forTool(ToolId::Crop)) {
        h->cancelPolygonLasso();
    }
}

bool ToolController::cropActive() const
{
    ToolHandler* h = registry_.forTool(ToolId::Crop);
    return h && h->cropActive();
}

void ToolController::setMoveAutoSelect(int which)
{
    moveAutoSelect_ = which == 1 ? 1 : 0;
    emit moveOptionsChanged();
}

void ToolController::setMoveShowTransformControls(bool on)
{
    moveShowTransform_ = on;
    emit moveOptionsChanged();
    syncTransformControls();
}

void ToolController::setMoveAlignTo(int which)
{
    moveAlignTo_ = which == 1 ? 1 : 0;
    emit moveOptionsChanged();
}

void ToolController::setEyedropperOptions(const EyedropperOptions& options)
{
    eyedropper_ = options;
    const int size = std::clamp(options.sampleSize, 1, 101);
    eyedropper_.sampleSize = size % 2 == 1 ? size : size + 1;
    eyedropper_.scope = std::clamp(options.scope, 0, 4);
    if (!eyedropper_.ring && canvas_ && active_ == ToolId::Eyedropper) {
        canvas_->clearSamplingRing();
    }
}

void ToolController::updateSamplingRing(const QPointF& imagePos)
{
    if (!canvas_) {
        return;
    }
    if (active_ == ToolId::Eyedropper && eyedropper_.ring) {
        canvas_->setSamplingRing(eyedropper_.sampleSize, imagePos);
    } else {
        canvas_->clearSamplingRing();
    }
}

void ToolController::setTolerance(int tolerance)
{
    tolerance_ = std::clamp(tolerance, 0, 255);
}

void ToolController::setMagneticWidth(int width)
{
    const int clamped = std::clamp(width, 1, 256);
    if (clamped != magneticWidth_) {
        magneticWidth_ = clamped;
        emit magneticWidthChanged(clamped);
    }
}

void ToolController::setMagneticContrast(int contrast)
{
    magneticContrast_ = std::clamp(contrast, 1, 100);
}

void ToolController::setMagneticFrequency(int frequency)
{
    magneticFrequency_ = std::clamp(frequency, 0, 100);
}

void ToolController::setContiguous(bool on) { contiguous_ = on; }

int ToolController::brushSize() const { return brushSize_; }

void ToolController::setBrushSize(int size)
{
    const int clamped = std::clamp(size, 1, 5000);
    if (clamped == brushSize_) {
        return;
    }
    brushSize_ = clamped;
    emit brushSizeChanged(brushSize_);
}

int ToolController::brushHardness() const { return brushHardness_; }

void ToolController::setBrushHardness(int h)
{
    const int clamped = std::clamp(h, 0, 100);
    if (clamped != brushHardness_) {
        brushHardness_ = clamped;
        emit brushTipChanged();
    }
}

void ToolController::setBrushRoundness(int roundness)
{
    const int clamped = std::clamp(roundness, 0, 100);
    if (clamped != brushRoundness_) {
        brushRoundness_ = clamped;
        emit brushTipChanged();
    }
}

void ToolController::setBrushTipAngle(int angle)
{
    const int clamped = std::clamp(angle, -180, 180);
    if (clamped != brushTipAngle_) {
        brushTipAngle_ = clamped;
        emit brushTipChanged();
    }
}

void ToolController::setBrushSpacing(int spacing)
{
    const int clamped = std::clamp(spacing, 1, 1000);
    if (clamped != brushSpacing_) {
        brushSpacing_ = clamped;
        emit brushTipChanged();
    }
}

void ToolController::setBrushDynamics(const BrushDynamics& dynamics)
{
    const BrushDynamics d{std::clamp(dynamics.scatter, 0, 1000), std::clamp(dynamics.count, 1, 16),
                          std::clamp(dynamics.sizeJitter, 0, 100),
                          std::clamp(dynamics.angleJitter, 0, 180),
                          std::clamp(dynamics.roundnessJitter, 0, 100)};
    const BrushDynamics& o = brushDynamics_;
    if (d.scatter != o.scatter || d.count != o.count || d.sizeJitter != o.sizeJitter
        || d.angleJitter != o.angleJitter || d.roundnessJitter != o.roundnessJitter) {
        brushDynamics_ = d;
        emit brushTipChanged();
    }
}

void ToolController::setBrushFlip(bool x, bool y)
{
    if (x != brushFlipX_ || y != brushFlipY_) {
        brushFlipX_ = x;
        brushFlipY_ = y;
        emit brushTipChanged();
    }
}

void ToolController::setCloneSource(const CloneSource& source)
{
    cloneSources_[cloneSlot_] = source;
    emit cloneSourceChanged();
}

void ToolController::setCloneSourceSlot(int slot)
{
    const int clamped = std::clamp(slot, 0, int(cloneSources_.size()) - 1);
    if (clamped != cloneSlot_) {
        cloneSlot_ = clamped;
        emit cloneSourceChanged();
    }
}

int ToolController::brushOpacity() const { return brushOpacity_; }

void ToolController::setBrushOpacity(int o) { brushOpacity_ = std::clamp(o, 0, 100); }

int ToolController::brushFlow() const { return brushFlow_; }

void ToolController::setBrushFlow(int f) { brushFlow_ = std::clamp(f, 0, 100); }

QString ToolController::brushMode() const { return brushMode_; }

void ToolController::setBrushMode(const QString& mode) { brushMode_ = mode; }

bool ToolController::autoErase() const { return autoErase_; }

void ToolController::setAutoErase(bool on) { autoErase_ = on; }

namespace {

// The retouch_ slot of the Blur, Sharpen, or Smudge tool.
size_t retouchSlot(ToolId id)
{
    return id == ToolId::Sharpen ? 1 : id == ToolId::Smudge ? 2 : 0;
}

// The tone_ slot of the Dodge, Burn, or Sponge tool.
size_t toneSlot(ToolId id)
{
    return id == ToolId::Burn ? 1 : id == ToolId::Sponge ? 2 : 0;
}

} // namespace

RetouchOptions ToolController::retouchOptions(ToolId id) const
{
    return retouch_[retouchSlot(id)];
}

void ToolController::setRetouchOptions(ToolId id, const RetouchOptions& options)
{
    retouch_[retouchSlot(id)] = options;
}

ToneOptions ToolController::toneOptions(ToolId id) const { return tone_[toneSlot(id)]; }

void ToolController::setToneOptions(ToolId id, const ToneOptions& options)
{
    tone_[toneSlot(id)] = options;
}

void ToolController::setSpotHealingType(int type) { spotHealingType_ = std::clamp(type, 0, 2); }

void ToolController::setContentAwareAdaptation(int level)
{
    contentAwareAdaptation_ = std::clamp(level, 0, 4);
}

void ToolController::setMixerReservoir(const QColor& color)
{
    if (color.rgba() == mixerReservoir_.rgba()) {
        return;
    }
    mixerReservoir_ = color;
    emit mixerReservoirChanged(color);
}

QColor ToolController::foreground() const { return foreground_; }

void ToolController::setForeground(const QColor& color)
{
    foreground_ = color;
    // Choosing a foreground colour loads the Mixer Brush (docs/03-tools/mixer-brush.md).
    setMixerReservoir(color);
    emit colorsChanged();
    // ...and is the text colour, so it restyles text being typed or a selected
    // type layer, as CS6's does with the Type tool active.
    if (color != type_.color) {
        TypeOptions o = type_;
        o.color = color;
        setTypeOptions(o);
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

void ToolController::adjustBrushSize(int delta) { setBrushSize(brushSize_ + delta); }

void ToolController::adjustBrushHardness(int delta) { setBrushHardness(brushHardness_ + delta); }

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
        setMagneticWidth(magneticWidth_ + (step > 0 ? 1 : -1));
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
    return selectionModeForModifiers(mode_, mods, hasExistingSelection);
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
    if (isSelectionTool(active_) && !(handler && handler->lassoInProgress())) {
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

bool ToolController::commitPolygonLasso()
{
    if (ToolHandler* h = registry_.forTool(active_)) {
        return h->commitPolygonLasso();
    }
    return false;
}

bool ToolController::cancelPolygonLasso()
{
    if (ToolHandler* h = registry_.forTool(active_)) {
        return h->cancelPolygonLasso();
    }
    return false;
}

bool ToolController::removeLassoPoint()
{
    if (ToolHandler* h = registry_.forTool(active_)) {
        return h->removeLassoPoint();
    }
    return false;
}

bool ToolController::commitCrop()
{
    // The staged crop survives a tool switch, so reach the Crop handler
    // directly rather than through whatever is active now; a Perspective Crop
    // quad lives only while its tool is active.
    const ToolId target = active_ == ToolId::PerspectiveCrop ? ToolId::PerspectiveCrop : ToolId::Crop;
    if (ToolHandler* h = registry_.forTool(target)) {
        return h->commitCrop();
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
