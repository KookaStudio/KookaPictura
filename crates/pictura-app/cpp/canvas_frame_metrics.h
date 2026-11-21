#pragma once

#include <QtCore/QRect>
#include <QtGui/QImage>

#include <memory>

class QPainter;
class QPaintEvent;

namespace pictura {

class ImageView;
class PictureView;
struct CanvasFrameSamples;

struct CanvasFrameCrop {
    QImage image;
    std::shared_ptr<CanvasFrameSamples> samples;
};

CanvasFrameCrop canvasFrameCrop(const PictureView& view, int level, const QRect& rect);
void copyFrameSamples(std::shared_ptr<CanvasFrameSamples>& target,
                      const std::shared_ptr<CanvasFrameSamples>& source, const QRect& area);
void eraseFrameSamples(std::shared_ptr<CanvasFrameSamples>& samples, const QRect& area);

class FrameCaptureSuppression {
public:
    FrameCaptureSuppression();
    ~FrameCaptureSuppression();

    FrameCaptureSuppression(const FrameCaptureSuppression&) = delete;
    FrameCaptureSuppression& operator=(const FrameCaptureSuppression&) = delete;

private:
    bool previous_;
};

class FrameViewportTiming {
public:
    explicit FrameViewportTiming(ImageView& view);
    ~FrameViewportTiming();

    FrameViewportTiming(const FrameViewportTiming&) = delete;
    FrameViewportTiming& operator=(const FrameViewportTiming&) = delete;

    void changed();

private:
    ImageView& view_;
    quint64 generation_ = 0;
};

class FramePaintTiming {
public:
    FramePaintTiming(ImageView& view, const QPaintEvent* event);
    ~FramePaintTiming();

    FramePaintTiming(const FramePaintTiming&) = delete;
    FramePaintTiming& operator=(const FramePaintTiming&) = delete;

    qint64 beginGather() const;
    void endGather(qint64 start);
    void drawSamples(std::shared_ptr<CanvasFrameSamples>& samples,
                     const QRect& source, const QRectF& target, const QPainter& painter);

private:
    ImageView& view_;
    const QPaintEvent* event_;
    quint64 generation_ = 0;
    qint64 start_ = 0;
    quint64 tileNs_ = 0;
    quint64 inputNs_ = 0;
    quint32 inputCount_ = 0;
    qint64 gpuNs_ = -1;
};

} // namespace pictura
