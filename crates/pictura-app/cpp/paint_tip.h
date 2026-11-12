#pragma once

#include "tool_options.h"

#include "api/paint_bridge.h"

namespace pictura {

// The paint family's shared brush as the paint-tool bridge's tip.
inline PaintDynamics paintDynamics(const BrushDynamics& d)
{
    return PaintDynamics{
        d.shapeEnabled,       d.sizeControl,       d.sizeFade,
        d.minDiameter,        d.angleControl,      d.angleFade,
        d.roundnessControl,   d.roundnessFade,     d.minRoundness,
        d.tiltScale,          d.brushProjection,   d.scatterEnabled,
        d.scatterControl,     d.scatterFade,       d.bothAxes,
        d.countControl,       d.countFade,         d.countJitter,
        d.colorEnabled,       d.applyPerTip,       d.fgBgJitter,
        d.fgBgControl,        d.fgBgFade,          d.hueJitter,
        d.saturationJitter,   d.brightnessJitter,  d.purity,
        d.transferEnabled,    d.opacityJitter,     d.opacityControl,
        d.opacityFade,        d.flowJitter,        d.flowControl,
        d.flowFade,           d.poseEnabled,       d.poseTiltX,
        d.poseTiltY,          d.poseRotation,      d.posePressure,
        d.overrideTiltX,      d.overrideTiltY,     d.overrideRotation,
        d.overridePressure,   d.lockShape,         d.lockScatter,
        d.lockColor,          d.lockTransfer,      d.lockPose,
        d.lockTexture,        d.lockDual};
}

inline PaintTip paintTip(const PaintOptions& paint)
{
    const BrushDynamics& d = paint.dynamics();
    return PaintTip{paint.size(), paint.hardness(), paint.roundness(), paint.angle(),
                    paint.spacing(), d.scatter, d.count, d.sizeJitter, d.angleJitter,
                    d.roundnessJitter, paint.grain(), paint.wetEdges(), paintDynamics(d),
                    paint.bristleEnabled(), paint.bristleShape(), paint.bristleDensity(),
                    paint.bristleLength(), paint.bristleThickness(), paint.bristleStiffness()};
}

} // namespace pictura
