#include "image_view.h"

#include "canvas_range.h"
#include "preferences.h"

#include <QtCore/QTimer>
#include <QtGui/QContextMenuEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtWidgets/QStyle>

#include <cmath>

namespace pictura {

namespace {
constexpr qint64 kMotionAgeMs = 80;
constexpr qint64 kMinSampleMs = 5;
constexpr double kStartSpeed = 0.3; // Logical pixels per millisecond.
constexpr double kMaxSpeed = 3.0;
constexpr double kStopSpeed = 0.025;
constexpr double kDecayMs = 130.0;
constexpr int kFrameMs = 16;

double speed(const QPointF& velocity)
{
    return std::hypot(velocity.x(), velocity.y());
}
} // namespace

double ImageView::birdsEyeZoom() const
{
    const QRectF bounds = viewRotation().mapRect(QRectF(QPointF(), QSizeF(docSize_)));
    if (bounds.isEmpty() || size().isEmpty()) {
        return zoom_;
    }
    return 0.95 * std::min(width() / bounds.width(), height() / bounds.height());
}

bool ImageView::armBirdsEye()
{
    if (spacePan_ || panning_ || zoom_ <= birdsEyeZoom()) {
        return false;
    }
    stopFlick();
    birdKeyHeld_ = true;
    birdKeyUsed_ = false;
    return true;
}

bool ImageView::releaseBirdsEyeKey()
{
    const bool tap = birdKeyHeld_ && !birdKeyUsed_;
    birdKeyHeld_ = false;
    cancelBirdsEye();
    return tap;
}

bool ImageView::beginBirdsEye(const QMouseEvent& event)
{
    if (event.button() != Qt::LeftButton) {
        return false;
    }
    if (!birdKeyHeld_ || event.modifiers() != Qt::NoModifier || spacePan_ || panning_
        || zoom_ <= birdsEyeZoom()
        || !QRectF(QPointF(), QSizeF(docSize_)).contains(widgetToImage(event.position()))) {
        return false;
    }
    birdKeyUsed_ = true;
    birdPointerCaptured_ = true;
    birdsEye_ = BirdsEye{birdsEyeZoom(), {}, {}};
    birdsEye_->fitOffset = QPointF(width() / 2.0, height() / 2.0)
        - QPointF(docSize_.width(), docSize_.height()) * (birdsEye_->fitZoom / 2.0);
    moveBirdsEye(event.position());
    return true;
}

void ImageView::moveBirdsEye(const QPointF& pos)
{
    if (!birdsEye_) {
        return;
    }
    const QPointF inView = viewRotation().inverted().map(pos);
    const QPointF doc = (inView - birdsEye_->fitOffset) / birdsEye_->fitZoom;
    const QPointF bounded(std::clamp(doc.x(), 0.0, double(docSize_.width())),
                          std::clamp(doc.y(), 0.0, double(docSize_.height())));
    const QPointF desired = QPointF(width() / 2.0, height() / 2.0) - bounded * zoom_;
    const OffsetRange range = offsetRangeFor(QSizeF(docSize_), zoom_, QSizeF(size()), freePan_,
                                             kCanvasRevealMarginPx, snapToWorkspace_);
    birdsEye_->destination = QPointF(std::clamp(desired.x(), range.minX, range.maxX),
                                     std::clamp(desired.y(), range.minY, range.maxY));
    update();
}

bool ImageView::releaseBirdsEyePointer(const QMouseEvent& event)
{
    if (!birdPointerCaptured_ || event.button() != Qt::LeftButton) {
        return false;
    }
    birdPointerCaptured_ = false;
    if (birdsEye_) {
        moveBirdsEye(event.position());
        const QPointF destination = birdsEye_->destination;
        cancelBirdsEye();
        // Keep H armed until it is released, including across repeated relocations.
        applyPanDelta(destination - offset_);
    }
    return true;
}

void ImageView::cancelBirdsEye()
{
    if (birdsEye_) {
        birdsEye_.reset();
        update();
    }
}

void ImageView::paintBirdsEye(QPainter& painter, const BirdsEye& overview, double originalZoom)
{
    const QPointF centre(width() / 2.0, height() / 2.0);
    const double ratio = overview.fitZoom / originalZoom;
    const QPointF destination =
        viewRotation().map(overview.fitOffset + (centre - overview.destination) * ratio);
    const QSizeF footprint = QSizeF(size()) * ratio;
    const QRectF outline(destination - QPointF(footprint.width(), footprint.height()) / 2.0,
                         footprint);
    painter.resetTransform();
    painter.setPen(QPen(QColor(20, 24, 28), 3));
    painter.setBrush(Qt::NoBrush);
    painter.drawRect(outline);
    painter.setPen(QPen(QColor(110, 220, 245), 1));
    painter.drawRect(outline);
}

void ImageView::setFlickPanning(FlickPanningMode mode, bool gpuAvailable)
{
    flickMode_ = mode;
    flickGpu_ = gpuAvailable;
    if (!flickAllowed()) {
        stopFlick();
    }
}

bool ImageView::flickAllowed() const
{
    return flickGpu_ && (flickMode_ == FlickPanningMode::On
        || (flickMode_ == FlickPanningMode::Automatic
            && style()->styleHint(QStyle::SH_Widget_Animation_Duration, nullptr, this) > 0));
}

void ImageView::beginFlickSample(const QMouseEvent& event)
{
    if (!flickAllowed()) {
        return;
    }
    flickSample_.emplace();
    auto& sample = *flickSample_;
    sample.anchor = event.position();
    sample.clock.start();
    sample.eventClock = event.timestamp() != 0;
    sample.anchorTime = flickTime(event);
    sample.lastMotion = sample.anchorTime;
}

qint64 ImageView::flickTime(const QMouseEvent& event) const
{
    return flickSample_->eventClock ? qint64(event.timestamp()) : flickSample_->clock.elapsed();
}

void ImageView::sampleFlick(const QMouseEvent& event)
{
    if (!flickSample_ || event.position() == last_) {
        return;
    }
    auto& sample = *flickSample_;
    const qint64 now = flickTime(event);
    const qint64 dt = now - sample.anchorTime;
    sample.lastMotion = now;
    if (dt >= kMinSampleMs || dt < 0) {
        sample.velocity = dt >= kMinSampleMs && dt <= kMotionAgeMs
            ? (event.position() - sample.anchor) / double(dt) : QPointF();
        sample.anchorTime = now;
        sample.anchor = event.position();
    }
}

void ImageView::startFlick(const QMouseEvent& event)
{
    if (!flickSample_) {
        return;
    }
    sampleFlick(event);
    const qint64 age = flickTime(event) - flickSample_->lastMotion;
    const QPointF velocity = flickSample_->velocity;
    flickSample_.reset();
    const double magnitude = speed(velocity);
    if (!flickAllowed() || age < 0 || age > kMotionAgeMs || magnitude < kStartSpeed) {
        return;
    }
    flickVelocity_ = velocity * std::min(1.0, kMaxSpeed / magnitude);
    flickModifiers_ = event.modifiers();
    if (!flickTimer_) {
        flickTimer_ = new QTimer(this);
        flickTimer_->setInterval(kFrameMs);
        connect(flickTimer_, &QTimer::timeout, this, &ImageView::advanceFlick);
    }
    flickFrameClock_.start();
    flickTimer_->start();
}

void ImageView::advanceFlick()
{
    if (!flickAllowed() || !isVisible() || workspaceSnapLocked()) {
        stopFlick();
        return;
    }
    const double dt = std::clamp(double(flickFrameClock_.restart()), 1.0, 50.0);
    const double decay = std::exp(-dt / kDecayMs);
    const QTransform unturn = QTransform().rotate(-rotation_);
    const QPointF velocity = unturn.map(flickVelocity_);
    const QPointF delta = velocity * (kDecayMs * (1.0 - decay));
    const QPointF before = offset_;
    applyPanDelta(delta);
    if (!flickTimer_->isActive()) {
        return;
    }
    const QPointF applied = offset_ - before;
    if (!applied.isNull()) {
        emit handPanned(applied, flickModifiers_);
    }
    const QPointF remaining(std::abs(applied.x() - delta.x()) < 0.001 ? velocity.x() : 0.0,
                            std::abs(applied.y() - delta.y()) < 0.001 ? velocity.y() : 0.0);
    flickVelocity_ = unturn.inverted().map(remaining * decay);
    if (speed(flickVelocity_) < kStopSpeed) {
        stopFlick();
    }
}

void ImageView::stopFlick()
{
    flickSample_.reset();
    if (flickTimer_) {
        flickTimer_->stop();
    }
}

void ImageView::contextMenuEvent(QContextMenuEvent* event)
{
    if (!birdPointerCaptured_) emit contextMenuRequested(event->globalPos());
    event->accept();
}

void ImageView::cancelHandNavigation()
{
    if (birdHandPan_) {
        birdPointerCaptured_ = true;
        birdHandPan_ = false;
        panning_ = false;
    }
    birdKeyHeld_ = false;
    cancelBirdsEye();
    stopFlick();
}

bool ImageView::event(QEvent* event)
{
    switch (event->type()) {
    case QEvent::FocusOut:
    case QEvent::WindowDeactivate:
        cancelHandNavigation();
        break;
    case QEvent::UngrabMouse:
        if (birdsEye_ || birdHandPan_) {
            cancelHandNavigation();
        }
        break;
    default:
        break;
    }
    return QWidget::event(event);
}

} // namespace pictura
