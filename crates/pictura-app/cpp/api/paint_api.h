#pragma once

#include <cstdint>
#include <functional>
#include <optional>

#include <QtCore/QString>

namespace pictura {
class PictureView;
struct PaintTip;
}

namespace pictura::paint_api {

/// A tool-lifetime paint stroke. The constructor runs the unit's begin call
/// (`begin_brush`, `begin_paint`, or any paint_tools begin_* free function) and
/// `open()` reports whether it started. `present()` queues `flush_present` and
/// never rebuilds through `image()`; `finish()` commits once via `end_paint`;
/// `cancel()` drops once via `cancel_paint`. The destructor finishes an open
/// stroke, so an early return cannot leave one open.
class Stroke {
public:
    Stroke(PictureView& view, std::function<bool()> begin);
    ~Stroke();

    Stroke(const Stroke&) = delete;
    Stroke& operator=(const Stroke&) = delete;
    Stroke(Stroke&& other) noexcept;
    Stroke& operator=(Stroke&& other) noexcept;

    bool open() const { return open_; }
    bool dab(double x, double y, double pressure = 1.0);
    bool present();
    bool finish();
    void cancel();

private:
    PictureView* view_ = nullptr;
    bool open_ = false;
};

/// Begin a brush/pencil stroke (the typed replacement for the free
/// `begin_brush` helper), owned by the returned stroke; `nullopt` on refusal.
std::optional<Stroke> beginBrush(PictureView& view, std::uint32_t foreground,
                                 std::uint32_t background, const PaintTip& tip, int opacity,
                                 int flow, const QString& mode, bool aliased, bool autoErase);

/// Queue `flush_present` on the next event-loop turn; the migrated `frame.cpp`
/// next-turn timer. A no-op when nothing is pending, so the chain ends.
void presentDeferred(PictureView& view);

} // namespace pictura::paint_api
