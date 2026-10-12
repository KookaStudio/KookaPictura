#pragma once

#include "tool_options.h"

#include "pictura_app/src/cxxqt_object/paint_tools.cxxqt.h"

namespace pictura {

// The paint family's shared brush as the paint-tool bridge's tip.
inline PaintTip paintTip(const PaintOptions& paint)
{
    const BrushDynamics& d = paint.dynamics();
    return PaintTip{paint.size(), paint.hardness(), paint.roundness(), paint.angle(),
                    paint.spacing(), d.scatter, d.count, d.sizeJitter, d.angleJitter,
                    d.roundnessJitter};
}

} // namespace pictura
