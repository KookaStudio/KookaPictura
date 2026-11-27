#include "properties_panel.h"

#include "adjustment_controls.h"
#include "api/document_bridge.h"
#include "api/layers_bridge.h"
#include "curve_widget.h"
#include "image_view.h"
#include "tools.h"

#include <QtGui/QImage>

#include <algorithm>
#include <array>
#include <cmath>
#include <memory>

namespace pictura {

void PropertiesPanel::startCurvesCanvas(int action)
{
    if (!tools_ || !view_ || kind_ != QLatin1String("curves") || !controls_->curveWidget()) return;
    if (curvesPicking_) {
        tools_->setCanvasBrush({});
        curvesPicking_ = false;
    }
    if (tools_->canvasBrushActive() || tools_->canvasSamplerActive()) return;
    curvesPicking_ = true;
    const QString path = path_;
    PictureView* const view = view_;
    auto startY = std::make_shared<qreal>(0);
    auto input = std::make_shared<int>(-1);
    auto output = std::make_shared<int>(0);
    tools_->setCanvasBrush([this, action, path, view, startY, input, output](ToolController::CanvasBrushPhase phase, const QPointF& pos, Qt::KeyboardModifiers) {
        if (view_ != view || path_ != path || !controls_->curveWidget()) return;
        auto* curve = controls_->curveWidget();
        using Phase = ToolController::CanvasBrushPhase;
        if (phase == Phase::Press) {
            const QRgb pixel = adjustment_input_argb(*view, path, int(std::floor(pos.x())), int(std::floor(pos.y())));
            if (!qAlpha(pixel)) return;
            const std::array<int, 4> values{qGray(pixel), qRed(pixel), qGreen(pixel), qBlue(pixel)};
            const int channel = std::clamp(controls_->curveChannel(), 0, 3);
            const int value = values[channel];
            if (action == 0) {
                *startY = pos.y();
                *input = curve->selectTone(value) ? value : -1;
                *output = curve->outputAt(value);
            } else if (action == 1 || action == 3) {
                curve->setEndpoint(action == 3, value);
            } else {
                commit();
                const int gray = values[0];
                for (int c = 1; c < 4; ++c) {
                    const int x = std::clamp(values[c], 1, 254);
                    const QString points = QStringLiteral("0,0 %1,%2 255,255").arg(x).arg(gray);
                    if (!adjustment_set_curve(*view, path, c, points)) {
                        adjustment_cancel(*view);
                        refresh();
                        return;
                    }
                }
                adjustment_commit(*view, title_);
                refresh();
            }
        } else if (phase == Phase::Drag && action == 0 && *input >= 0) {
            curve->moveSelected(*input, *output + qRound(*startY - pos.y()));
        } else if (phase == Phase::Release) {
            *input = -1;
            commit();
        }
    });
}

void PropertiesPanel::updateLevelsClipping()
{
    if (!tools_ || !tools_->canvas()) return;
    auto* canvas = tools_->canvas();
    int channel = 0, black = 0, white = 255;
    if (!levelsClipping_ || !view_ || !controls_->levelsClippingParameters(channel, black, white)) {
        canvas->clearFilterPreview();
        return;
    }
    const QRect bounds(0, 0, int(view_->document_width()), int(view_->document_height()));
    const QRect area = canvas->visibleDocumentRect().toAlignedRect().intersected(bounds);
    if (area.isEmpty()) { canvas->clearFilterPreview(); return; }
    QImage map = adjustment_input_image(*view_, path_, area.x(), area.y(), area.width(), area.height())
                     .convertToFormat(QImage::Format_ARGB32);
    if (map.isNull()) { canvas->clearFilterPreview(); return; }
    for (int y = 0; y < map.height(); ++y) {
        auto* row = reinterpret_cast<QRgb*>(map.scanLine(y));
        for (int x = 0; x < map.width(); ++x) {
            const QRgb pixel = row[x];
            std::array<int, 3> rgb{qRed(pixel), qGreen(pixel), qBlue(pixel)};
            if (channel > 0) rgb.fill(rgb.at(channel - 1));
            for (int& sample : rgb) sample = sample <= black ? 0 : sample >= white ? 255 : 128;
            row[x] = qRgba(rgb[0], rgb[1], rgb[2], qAlpha(pixel));
        }
    }
    canvas->setFilterPreview(map, area, true);
}

} // namespace pictura
