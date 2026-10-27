#include "smart_guides.h"

#include "api/layers_api.h"
#include "api/transform_api.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <algorithm>
#include <array>
#include <cmath>

namespace pictura {

namespace {

constexpr double kEpsilon = 0.01;

// One axis's chosen alignment: the offset to apply, the target position, and
// the reference that supplied it (-1 for the document).
struct AxisSnap {
    double delta = 0.0;
    double target = 0.0;
    int reference = -1;
    bool found = false;
};

std::array<double, 3> edges(double low, double high)
{
    return {low, (low + high) / 2.0, high};
}

AxisSnap bestAxis(const std::array<double, 3>& moving, double documentLow, double documentHigh,
                  const QList<std::array<double, 3>>& references, double threshold)
{
    AxisSnap best;
    double bestDistance = 0.0;
    const std::array<double, 3> documentTargets{
        documentLow, (documentLow + documentHigh) / 2.0, documentHigh};
    const auto consider = [&](double movingEdge, double target, int reference) {
        const double delta = target - movingEdge;
        const double distance = std::abs(delta);
        if (distance > threshold + kEpsilon) {
            return;
        }
        // Strictly closer wins; the iteration order above settles ties.
        if (!best.found || distance < bestDistance - kEpsilon) {
            best = AxisSnap{delta, target, reference, true};
            bestDistance = distance;
        }
    };
    // References first: when a layer and the document are equally close, the
    // layer alignment is the one the user is reaching for.
    for (int i = 0; i < references.size(); ++i) {
        for (const double edge : moving) {
            for (const double target : references.at(i)) {
                consider(edge, target, i);
            }
        }
    }
    for (const double edge : moving) {
        for (const double target : documentTargets) {
            consider(edge, target, -1);
        }
    }
    return best;
}

void addLine(QList<SmartGuideLine>& lines, bool vertical, double position)
{
    const SmartGuideLine line{vertical, position};
    if (!lines.contains(line)) {
        lines.append(line);
    }
}

// Every target that coincides with a snapped moving edge becomes a line, so a
// layer the same width as its reference shows both edges.
void collectLines(QList<SmartGuideLine>& lines, const std::array<double, 3>& moving,
                  double delta, double documentLow, double documentHigh,
                  const QList<std::array<double, 3>>& references, bool vertical)
{
    const std::array<double, 3> targets{documentLow, (documentLow + documentHigh) / 2.0,
                                        documentHigh};
    const auto matches = [&](const std::array<double, 3>& candidates) {
        for (const double edge : moving) {
            for (const double target : candidates) {
                if (std::abs(target - (edge + delta)) <= kEpsilon) {
                    addLine(lines, vertical, target);
                }
            }
        }
    };
    matches(targets);
    for (const std::array<double, 3>& reference : references) {
        matches(reference);
    }
}

// The gap between two boxes along an axis: their facing edges, or 0 when they
// overlap.
double gapAlong(double movingLow, double movingHigh, double referenceLow, double referenceHigh)
{
    if (movingHigh <= referenceLow) {
        return referenceLow - movingHigh;
    }
    if (referenceHigh <= movingLow) {
        return movingLow - referenceHigh;
    }
    return 0.0;
}

} // namespace

SmartGuideResult smartGuideSnap(const QRectF& moving, const QList<QRectF>& references,
                                const QSizeF& document, double zoom, double thresholdScreenPx)
{
    SmartGuideResult result;
    if (moving.isNull() || document.isEmpty() || zoom <= 0.0) {
        return result;
    }
    const double threshold = thresholdScreenPx / zoom;
    const std::array<double, 3> movingX = edges(moving.left(), moving.right());
    const std::array<double, 3> movingY = edges(moving.top(), moving.bottom());
    QList<QRectF> valid;
    valid.reserve(references.size());
    for (const QRectF& reference : references) {
        if (!reference.isNull()) {
            valid.append(reference);
        }
    }
    QList<std::array<double, 3>> referenceX;
    QList<std::array<double, 3>> referenceY;
    referenceX.reserve(valid.size());
    referenceY.reserve(valid.size());
    for (const QRectF& reference : valid) {
        referenceX.append(edges(reference.left(), reference.right()));
        referenceY.append(edges(reference.top(), reference.bottom()));
    }
    const AxisSnap x = bestAxis(movingX, 0.0, document.width(), referenceX, threshold);
    const AxisSnap y = bestAxis(movingY, 0.0, document.height(), referenceY, threshold);
    result.snap = QPointF(x.found ? x.delta : 0.0, y.found ? y.delta : 0.0);
    if (x.found) {
        collectLines(result.lines, movingX, x.delta, 0.0, document.width(), referenceX, true);
    }
    if (y.found) {
        collectLines(result.lines, movingY, y.delta, 0.0, document.height(), referenceY, false);
    }
    const QRectF snapped = moving.translated(result.snap);
    const auto addGap = [&](const AxisSnap& axis, bool vertical) {
        if (!axis.found || axis.reference < 0 || axis.reference >= valid.size()) {
            return;
        }
        const QRectF& reference = valid.at(axis.reference);
        SmartGuideGap gap;
        gap.verticalGuide = vertical;
        if (vertical) {
            gap.distance =
                gapAlong(snapped.top(), snapped.bottom(), reference.top(), reference.bottom());
            gap.anchor = QPointF(axis.target,
                                 (std::max(snapped.top(), reference.top())
                                  + std::min(snapped.bottom(), reference.bottom()))
                                     / 2.0);
        } else {
            gap.distance =
                gapAlong(snapped.left(), snapped.right(), reference.left(), reference.right());
            gap.anchor = QPointF((std::max(snapped.left(), reference.left())
                                  + std::min(snapped.right(), reference.right()))
                                     / 2.0,
                                 axis.target);
        }
        result.gaps.append(gap);
    };
    addGap(x, true);
    addGap(y, false);
    return result;
}

QList<QRectF> smartGuideReferences(PictureView& view, const QStringList& movingPaths)
{
    QList<QRectF> out;
    for (const pictura::layers_api::LayerRow& row : pictura::layers_api::rows(view)) {
        if (!row.visible) {
            continue;
        }
        const bool moving = std::any_of(
            movingPaths.cbegin(), movingPaths.cend(), [&row](const QString& path) {
                return !path.isEmpty()
                    && (row.path == path || row.path.startsWith(path + QLatin1Char('/')));
            });
        if (moving) {
            continue;
        }
        const pictura::transform_api::LayerRect rect =
            pictura::transform_api::contentRect(view, row.path);
        if (rect.valid) {
            out.append(QRectF(rect.left, rect.top, rect.width(), rect.height()));
        }
    }
    return out;
}

} // namespace pictura
