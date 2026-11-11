#include "tools.h"

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

} // namespace pictura
