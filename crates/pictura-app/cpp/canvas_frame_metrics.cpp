#include "canvas_frame_metrics.h"

#include "image_view.h"
#include "pictura_debug_timing.h"
#include "api/document_bridge.h"

#include <QtGui/QPaintEvent>
#include <QtGui/QPainter>

#include <chrono>
#include <limits>
#include <utility>

namespace pictura {

struct CanvasFrameSamples {
    ::rust::Box<FrameCropSamples> data = frame_samples_new();
};

namespace {

thread_local bool captureSuppressed = false;
quint64 nextCanvasId = 1;

FrameCropRect frameRect(const QRect& rect)
{
    return {rect.x(), rect.y(), rect.width(), rect.height()};
}

} // namespace

CanvasFrameCrop canvasFrameCrop(const PictureView& view, int level, const QRect& rect)
{
    auto samples = std::make_shared<CanvasFrameSamples>();
    QImage image = frame_samples_crop(view, level, frameRect(rect), *samples->data);
    if (image.isNull() || frame_samples_empty(*samples->data)) {
        samples.reset();
    }
    return {std::move(image), std::move(samples)};
}

void copyFrameSamples(std::shared_ptr<CanvasFrameSamples>& target,
                      const std::shared_ptr<CanvasFrameSamples>& source, const QRect& area)
{
    if (!source || area.isEmpty()) {
        return;
    }
    if (!target) {
        target = std::make_shared<CanvasFrameSamples>();
    }
    frame_samples_copy(*target->data, *source->data, frameRect(area));
    if (frame_samples_empty(*target->data)) {
        target.reset();
    }
}

void eraseFrameSamples(std::shared_ptr<CanvasFrameSamples>& samples, const QRect& area)
{
    if (samples) {
        frame_samples_erase(*samples->data, frameRect(area));
        if (frame_samples_empty(*samples->data)) {
            samples.reset();
        }
    }
}

FrameCaptureSuppression::FrameCaptureSuppression()
    : previous_(std::exchange(captureSuppressed, true))
{
}

FrameCaptureSuppression::~FrameCaptureSuppression()
{
    captureSuppressed = previous_;
}

quint64 ImageView::prepareFrameTiming()
{
    if (!hasDocument() || !metricsEnabled() || captureSuppressed) {
        return 0;
    }
    const quint64 generation = frame_capture_generation();
    if (generation == 0) {
        return 0;
    }
    if (!frameTiming_) {
        frameTiming_.emplace();
        frameTiming_->id = nextCanvasId++;
    }
    if (frameTiming_->generation != generation) {
        const quint64 id = frameTiming_->id;
        frameTiming_.emplace();
        frameTiming_->id = id;
        frameTiming_->generation = generation;
    }
    return generation;
}

qint64 ImageView::frameNow() const
{
    if (frameClockForTest_) {
        return frameClockForTest_();
    }
    const auto now = std::chrono::steady_clock::now().time_since_epoch();
    return std::chrono::duration_cast<std::chrono::nanoseconds>(now).count();
}

void ImageView::seedFrameGpuSampleForTest(const QRect& rect, quint64 ns)
{
    if (prepareFrameTiming() == 0) {
        return;
    }
    if (!presentCache_.samples) {
        presentCache_.samples = std::make_shared<CanvasFrameSamples>();
    }
    frame_samples_seed_for_test(*presentCache_.samples->data, frameRect(rect), ns);
}

FrameViewportTiming::FrameViewportTiming(ImageView& view)
    : view_(view)
    , generation_(view.prepareFrameTiming())
{
    if (generation_ != 0) {
        auto& state = *view_.frameTiming_;
        if (state.inputDepth++ == 0) {
            state.inputStart = view_.frameNow();
            state.inputChanged = false;
        }
    }
}

FrameViewportTiming::~FrameViewportTiming()
{
    if (generation_ == 0 || view_.frameTiming_->generation != generation_) {
        return;
    }
    auto& state = *view_.frameTiming_;
    if (--state.inputDepth == 0 && state.inputChanged) {
        state.inputNs += quint64(std::max(qint64(0), view_.frameNow() - state.inputStart));
        ++state.inputCount;
    }
}

void FrameViewportTiming::changed()
{
    if (generation_ != 0 && view_.frameTiming_->generation == generation_) {
        view_.frameTiming_->inputChanged = true;
    }
}

FramePaintTiming::FramePaintTiming(ImageView& view, const QPaintEvent* event)
    : view_(view)
    , event_(event)
    , generation_(view.prepareFrameTiming())
{
    if (generation_ != 0) {
        auto& state = *view_.frameTiming_;
        start_ = view_.frameNow();
        inputCount_ = std::exchange(state.inputCount, 0);
        inputNs_ = std::exchange(state.inputNs, 0);
    }
}

FramePaintTiming::~FramePaintTiming()
{
    if (generation_ == 0 || view_.frameTiming_->generation != generation_
        || frame_capture_generation() != generation_) {
        return;
    }
    const qint64 end = view_.frameNow();
    auto& state = *view_.frameTiming_;
    const qint64 interval = state.lastPaint ? std::max(qint64(0), end - *state.lastPaint) : -1;
    state.lastPaint = end;
    frame_capture_record(generation_, {state.id, inputCount_, inputNs_, tileNs_,
                                        quint64(std::max(qint64(0), end - start_)), interval,
                                        gpuNs_});
}

qint64 FramePaintTiming::beginGather() const
{
    return generation_ != 0 ? view_.frameNow() : 0;
}

void FramePaintTiming::endGather(qint64 start)
{
    if (generation_ != 0) {
        tileNs_ += quint64(std::max(qint64(0), view_.frameNow() - start));
    }
}

void FramePaintTiming::drawSamples(std::shared_ptr<CanvasFrameSamples>& samples,
                                   const QRect& source, const QRectF& target,
                                   const QPainter& painter)
{
    if (generation_ == 0 || !samples || source.isEmpty() || target.isEmpty()
        || frame_capture_generation() != generation_) {
        return;
    }
    const auto rects = frame_samples_rects(*samples->data, generation_);
    if (rects.empty()) {
        samples.reset();
        return;
    }
    const QTransform world = painter.worldTransform();
    QPainterPath clip;
    clip.addRegion(event_ ? event_->region() : QRegion(view_.rect()));
    if (painter.hasClipping()) {
        clip = clip.intersected(world.map(painter.clipPath()));
    }

    QTransform sourceToTarget;
    sourceToTarget.translate(target.x(), target.y());
    sourceToTarget.scale(target.width() / source.width(), target.height() / source.height());
    sourceToTarget.translate(-source.x(), -source.y());
    const QTransform transform = sourceToTarget * world;
    bool consumed = false;
    for (std::size_t index = 0; index < rects.size(); ++index) {
        const auto& rect = rects[index];
        const QRect span = source.intersected(QRect(rect.x, rect.y, rect.w, rect.h));
        QPainterPath supplied;
        supplied.addRect(span);
        if (!span.isEmpty() && clip.intersects(transform.map(supplied))) {
            const qint64 ns = frame_samples_take(*samples->data, index, generation_);
            if (ns >= 0) {
                consumed = true;
                gpuNs_ = std::max(qint64(0), gpuNs_);
                gpuNs_ += std::min(ns, std::numeric_limits<qint64>::max() - gpuNs_);
            }
        }
    }
    if (consumed && frame_samples_prune_empty(*samples->data, generation_)) {
        samples.reset();
    }
}

} // namespace pictura
