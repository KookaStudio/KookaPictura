#include "tools.h"

#include "api/paint_bridge.h"

#include <algorithm>
#include <cmath>

namespace pictura {

void ToolController::setPressure(double pressure)
{
    pressure_ = std::clamp(pressure, 0.0, 1.0);
}

void ToolController::setPointerPose(double tiltX, double tiltY, double rotation, double wheel)
{
    tiltX_ = std::clamp(tiltX, -90.0, 90.0);
    tiltY_ = std::clamp(tiltY, -90.0, 90.0);
    rotation_ = std::fmod(rotation, 360.0);
    if (rotation_ < 0.0) {
        rotation_ += 360.0;
    }
    wheel_ = std::clamp(wheel, 0.0, 1.0);
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
    if (active_ == ToolId::FreeformPen && pen_->get().magnetic) {
        const int step = v->brush_shortcut_delta(key, nativeScanCode, false, true);
        if (step == 0) {
            return false;
        }
        PenOptions options = pen_->get();
        options.magneticWidth = std::clamp(options.magneticWidth + (step > 0 ? 1 : -1), 1, 256);
        pen_->set(options);
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

} // namespace pictura
