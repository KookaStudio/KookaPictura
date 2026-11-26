#include "properties_panel.h"

#include "adjustment_controls.h"
#include "api/document_bridge.h"
#include "api/layers_bridge.h"
#include "image_view.h"
#include "tools.h"

#include <QtGui/QImage>

#include <array>

namespace pictura {

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
