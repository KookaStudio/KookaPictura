#include "tool_handler.h"

#include "crop_grip.h"
#include "icons.h"
#include "image_view.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/crop_group.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust/image_size.cxxqt.h"

#include <QtCore/QElapsedTimer>
#include <QtCore/QStringList>
#include <QtGui/QPolygonF>
#include <QtWidgets/QApplication>

#include <algorithm>
#include <array>
#include <cmath>
#include <limits>
#include <memory>
#include <vector>

namespace pictura {

namespace {

// Perspective Crop: click the four corners of something that should be
// rectangular; the quad closes on the fourth click and its corners can then be
// pulled onto the edges. Enter or a double-click inside commits (warping the
// document so the quad becomes the canvas); Escape discards it. Dragging the
// whole quad moves it; dragging outside the completed quad rotates it about its
// centre. Ported from photorust's CanvasView, reshaped to CS6's four-click
// staging.
class PerspectiveCropToolHandler : public ToolHandler {
public:
    void onActivate(ToolContext& ctx) override
    {
        ctx_ = &ctx;
        showQuad(ctx);
    }

    void onDeactivate(ToolContext& ctx) override
    {
        quad_.clear();
        gesture_ = Gesture::None;
        if (ImageView* canvas = ctx.canvas()) {
            canvas->clearPerspectiveCropQuad();
        }
        ctx_ = nullptr;
    }

    bool onPress(ToolContext& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        PictureView* v = ctx.view();
        if (!v || !v->has_document()) {
            return true;
        }
        ctx_ = &ctx;
        corner_ = cornerAt(ctx, imagePos);

        // A double-click inside a complete quad (never on a corner) commits.
        const bool doubleClick = quad_.size() == 4 && corner_ < 0 && clock_.isValid()
            && clock_.elapsed() <= QApplication::doubleClickInterval()
            && screenDistance(ctx, imagePos, lastPress_) <= 4.0
            && quad_.containsPoint(imagePos, Qt::OddEvenFill);
        lastPress_ = imagePos;
        if (doubleClick) {
            clock_.invalidate();
            commit(ctx);
            return true;
        }
        clock_.restart();

        // Staging: each click drops the next corner; on the fourth the quad is
        // oriented top-left/top-right/bottom-right/bottom-left so the crop is
        // never rotated or mirrored, whatever corner was clicked first.
        if (quad_.size() < 4) {
            const QString refusal = perspective_crop_refusal_reason(*v);
            if (quad_.isEmpty() && !refusal.isEmpty()) {
                ctx.refused(refusal);
                return true;
            }
            quad_.append(snapPoint(ctx, imagePos));
            if (quad_.size() == 4) {
                quad_ = orderQuad(quad_);
            }
            showQuad(ctx);
            notifyStateChanged(ctx);
            gesture_ = Gesture::None;
            return true;
        }

        start_ = imagePos;
        startQuad_ = quad_;
        if (corner_ >= 0) {
            gesture_ = Gesture::Corner;
        } else if (quad_.containsPoint(imagePos, Qt::OddEvenFill)) {
            gesture_ = Gesture::Move;
        } else if (cropRotateZone(quadBounds(), imagePos, zoom(ctx))) {
            gesture_ = Gesture::Rotate;
            rotateStart_ = imagePos;
            rotateCentre_ = quadCentre();
        } else {
            gesture_ = Gesture::None;
        }
        return true;
    }

    void onMove(ToolContext& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        const QPointF delta = imagePos - start_;
        switch (gesture_) {
        case Gesture::None:
            updateCursor(ctx, imagePos);
            return;
        case Gesture::Corner:
            quad_[corner_] = snapPoint(ctx, startQuad_.at(corner_) + delta);
            break;
        case Gesture::Move:
            quad_ = translateQuad(startQuad_, delta);
            break;
        case Gesture::Rotate: {
            const double a0 = std::atan2(rotateStart_.y() - rotateCentre_.y(),
                                         rotateStart_.x() - rotateCentre_.x());
            const double a1 = std::atan2(imagePos.y() - rotateCentre_.y(),
                                         imagePos.x() - rotateCentre_.x());
            const double deg = (a1 - a0) * 180.0 / M_PI;
            quad_ = rotateQuad(startQuad_, rotateCentre_, deg);
            break;
        }
        }
        showQuad(ctx);
    }

    void onRelease(ToolContext& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        if (gesture_ == Gesture::None) {
            return;
        }
        onMove(ctx, imagePos, mods);
        gesture_ = Gesture::None;
        showQuad(ctx);
        notifyStateChanged(ctx);
    }

    bool commitCrop() override { return ctx_ && commit(*ctx_); }

    bool hasPendingCrop() const override { return quad_.size() == 4; }
    bool perspectiveQuadComplete() const override { return quad_.size() == 4; }

    void setPerspectiveOutput(int width, int height) override
    {
        outW_ = width;
        outH_ = height;
    }

    void setPerspectiveShowGrid(bool on) override
    {
        showGrid_ = on;
        if (ctx_) {
            showQuad(*ctx_);
        }
    }

    // Escape discards the staged quad.
    bool cancelPolygonLasso() override
    {
        if (quad_.isEmpty()) {
            return false;
        }
        quad_.clear();
        gesture_ = Gesture::None;
        if (ctx_) {
            showQuad(*ctx_);
            notifyStateChanged(*ctx_);
        }
        return true;
    }

private:
    enum class Gesture { None, Corner, Move, Rotate };

    bool commit(ToolContext& ctx)
    {
        PictureView* v = ctx.view();
        if (!v || quad_.size() != 4) {
            return false;
        }
        double corners[8];
        for (int i = 0; i < 4; ++i) {
            corners[2 * i] = quad_.at(i).x();
            corners[2 * i + 1] = quad_.at(i).y();
        }
        // A degenerate quad is refused and stays up for the user to fix.
        if (!perspective_crop_commit_sized(*v, ::rust::Slice<const double>(corners, 8), outW_,
                                           outH_)) {
            ctx.refused(QStringLiteral("Could not complete the Perspective Crop: the "
                                       "corners do not form a usable quadrilateral."));
            return false;
        }
        quad_.clear();
        showQuad(ctx);
        notifyStateChanged(ctx);
        ctx.emitSelectionCommitted();
        return true;
    }

    static double zoom(const ToolContext& ctx) { return ctx.canvas() ? ctx.canvas()->zoom() : 1.0; }

    static double screenDistance(const ToolContext& ctx, const QPointF& a, const QPointF& b)
    {
        const QPointF d = (a - b) * zoom(ctx);
        return std::hypot(d.x(), d.y());
    }

    // The bounding box of the staged points; the rotate band is measured from it.
    QRectF quadBounds() const
    {
        if (quad_.isEmpty()) {
            return QRectF();
        }
        return quad_.boundingRect();
    }

    QPointF quadCentre() const
    {
        if (quad_.isEmpty()) {
            return QPointF();
        }
        QPointF sum;
        for (const QPointF& p : quad_) {
            sum += p;
        }
        return sum / quad_.size();
    }

    // Reorder four clicked corners into TL, TR, BR, BL so the warp is neither
    // rotated nor mirrored, whatever corner the user started from. The points
    // are sorted clockwise about their centroid (screen y-down), then the list
    // is rotated so the top-left-most point (smallest x + y) comes first.
    static QPolygonF orderQuad(const QPolygonF& quad)
    {
        if (quad.size() != 4) {
            return quad;
        }
        const QPointF c = (quad.at(0) + quad.at(1) + quad.at(2) + quad.at(3)) / 4.0;
        const auto angle = [&quad, c](int i) {
            return std::atan2(quad.at(i).y() - c.y(), quad.at(i).x() - c.x());
        };
        std::array<int, 4> idx{0, 1, 2, 3};
        std::sort(idx.begin(), idx.end(), [&angle](int a, int b) { return angle(a) < angle(b); });
        int start = 0;
        double best = std::numeric_limits<double>::max();
        for (int k = 0; k < 4; ++k) {
            const QPointF& p = quad.at(idx.at(k));
            const double sum = p.x() + p.y();
            if (sum < best) {
                best = sum;
                start = k;
            }
        }
        QPolygonF out;
        for (int k = 0; k < 4; ++k) {
            out << quad.at(idx.at((start + k) % 4));
        }
        return out;
    }

    static QPolygonF translateQuad(const QPolygonF& quad, const QPointF& delta)
    {
        QPolygonF out;
        for (const QPointF& p : quad) {
            out << p + delta;
        }
        return out;
    }

    static QPolygonF rotateQuad(const QPolygonF& quad, const QPointF& centre, double deg)
    {
        const double r = deg * M_PI / 180.0;
        const double cs = std::cos(r);
        const double sn = std::sin(r);
        QPolygonF out;
        for (const QPointF& p : quad) {
            const QPointF d = p - centre;
            out << QPointF(centre.x() + d.x() * cs - d.y() * sn,
                           centre.y() + d.x() * sn + d.y() * cs);
        }
        return out;
    }

    // Snap a point to the canvas edges/corners and the composite bounding box
    // within 6 screen px, per axis.
    QPointF snapPoint(ToolContext& ctx, const QPointF& p) const
    {
        PictureView* v = ctx.view();
        if (!v || !v->has_document()) {
            return p;
        }
        QPointF out = p;
        const double tol = 6.0 / std::max(zoom(ctx), 1e-6);
        std::vector<double> xs = {0.0, double(v->document_width())};
        std::vector<double> ys = {0.0, double(v->document_height())};
        const QStringList comp =
            composite_bounds(*v).split(QLatin1Char(' '), Qt::SkipEmptyParts);
        if (comp.size() == 4) {
            xs.push_back(comp[0].toDouble());
            xs.push_back(comp[2].toDouble());
            ys.push_back(comp[1].toDouble());
            ys.push_back(comp[3].toDouble());
        }
        const auto snap1d = [tol](double value, const std::vector<double>& candidates) {
            double best = value;
            double bestDelta = tol;
            for (double c : candidates) {
                if (std::abs(value - c) < bestDelta) {
                    bestDelta = std::abs(value - c);
                    best = c;
                }
            }
            return best;
        };
        out.setX(snap1d(p.x(), xs));
        out.setY(snap1d(p.y(), ys));
        return out;
    }

    // Corners grab within a fixed screen radius, so they stay reachable at any
    // zoom.
    int cornerAt(ToolContext& ctx, const QPointF& imagePos) const
    {
        for (int i = 0; i < quad_.size(); ++i) {
            if (screenDistance(ctx, quad_.at(i), imagePos) <= 8.0) {
                return i;
            }
        }
        return -1;
    }

    // A precise crosshair over the canvas, the move cursor over a corner, and
    // the eight-way rotate cursor outside a completed quad.
    void updateCursor(ToolContext& ctx, const QPointF& imagePos)
    {
        ImageView* canvas = ctx.canvas();
        if (!canvas) {
            return;
        }
        if (cornerAt(ctx, imagePos) >= 0) {
            canvas->setCursor(Qt::SizeAllCursor);
            return;
        }
        if (quad_.size() == 4 && !quad_.containsPoint(imagePos, Qt::OddEvenFill)
            && cropRotateZone(quadBounds(), imagePos, zoom(ctx))) {
            const QPointF c = quadCentre();
            const double a = cropRotateZoneAngle(imagePos - c) * M_PI / 180.0;
            canvas->setCursor(rotateCursor(c, c + QPointF(std::cos(a), std::sin(a))));
            return;
        }
        const ToolInfo& info = toolInfo(ToolId::PerspectiveCrop);
        const QCursor cross = cursor(toolCursorId(ToolId::PerspectiveCrop, Qt::NoModifier),
                                     info.hotspotX, info.hotspotY);
        canvas->setCursor(cross.pixmap().isNull() ? QCursor(Qt::CrossCursor) : cross);
    }

    void showQuad(ToolContext& ctx)
    {
        if (ImageView* canvas = ctx.canvas()) {
            canvas->setPerspectiveCropQuad(quad_, gridSpacing(ctx));
        }
    }

    // The inch grid spacing in document pixels, from the resolution (72 fallback).
    double gridSpacing(const ToolContext& ctx) const
    {
        if (!showGrid_) {
            return 0.0;
        }
        PictureView* v = ctx.view();
        if (!v) {
            return 0.0;
        }
        const double ppi = document_ppi(*v);
        return ppi > 0.0 ? std::max(1.0, std::round(ppi)) : 72.0;
    }

    void notifyStateChanged(ToolContext& ctx) { ctx.notifyToolSessionChanged(); }

    QPolygonF quad_;
    QPolygonF startQuad_;
    QPointF start_;
    QPointF lastPress_;
    QPointF rotateStart_;
    QPointF rotateCentre_;
    int corner_ = -1;
    int outW_ = 0;
    int outH_ = 0;
    bool showGrid_ = true;
    Gesture gesture_ = Gesture::None;
    QElapsedTimer clock_;
    ToolContext* ctx_ = nullptr;
};

} // namespace

std::unique_ptr<ToolHandler> makePerspectiveCropToolHandler()
{
    return std::make_unique<PerspectiveCropToolHandler>();
}

} // namespace pictura
