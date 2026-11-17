// The rubylith overlay: the Layers panel toggles it and sets its display
// options on the view; the frame draws it on each window of the active
// document.

#include "frame_includes.h"
#include "panels/channels_panel.h"
#include "panels/layers_panel.h"

#include "api/layers_bridge.h"

#include <cstdint>
#include <vector>

namespace pictura {

namespace {

constexpr const char* kMaskOverlayKey = "picturaMaskOverlayKey";
// ponytail: one image for the whole document, so it is soft at high zoom on
// very large documents; crop it per viewport if that matters.
constexpr int kMaskOverlayMaxSide = 4096;
// The CS6 default alpha-channel overlay: red at 50 % over the masked (black)
// areas.
constexpr int kAlphaOverlayOpacity = 50;

} // namespace

void PicturaMainWindow::wireMaskOverlay()
{
    connect(panels_->layersPanel(), &LayersPanel::maskDisplayChanged, this, &PicturaMainWindow::refresh);
}

void PicturaMainWindow::syncMaskOverlay(PictureView* view, ImageView* canvas)
{
    if (!canvas) {
        return;
    }
    const QList<int> alphas = panels_ && panels_->channelsPanel()
        ? panels_->channelsPanel()->shownAlphas()
        : QList<int>();
    if (!alphas.isEmpty() && view && view->has_document()) {
        QString key = QStringLiteral("alpha:");
        for (int alpha : alphas) {
            key += QString::number(alpha) + QLatin1Char(',');
        }
        key += QString::number(view->canvas_revision());
        if (canvas->property(kMaskOverlayKey).toString() == key) {
            return;
        }
        int width = 0;
        int height = 0;
        std::vector<std::uint8_t> transparency;
        for (int alpha : alphas) {
            int planeWidth = 0;
            int planeHeight = 0;
            const ::rust::Vec<std::uint8_t> plane =
                channels_alpha_thumbnail(*view, alpha, kMaskOverlayMaxSide, planeWidth,
                                         planeHeight);
            if (planeWidth <= 0 || planeHeight <= 0
                || int(plane.size()) != planeWidth * planeHeight) {
                continue;
            }
            if (transparency.empty()) {
                width = planeWidth;
                height = planeHeight;
                transparency.assign(plane.size(), 255);
            } else if (planeWidth != width || planeHeight != height) {
                continue;
            }
            for (size_t i = 0; i < transparency.size(); ++i) {
                // Masked Areas: black coverage takes the rubylith. Each shown
                // channel multiplies the remaining transparency, so more
                // channels deepen the red.
                const int red = (int(255 - plane[i]) * kAlphaOverlayOpacity + 50) / 100;
                transparency[i] =
                    std::uint8_t((int(transparency[i]) * (255 - red) + 127) / 255);
            }
        }
        if (width > 0 && height > 0 && int(transparency.size()) == width * height) {
            canvas->setProperty(kMaskOverlayKey, key);
            QImage overlay(width, height, QImage::Format_RGBA8888);
            for (int y = 0; y < height; ++y) {
                auto* row = overlay.scanLine(y);
                for (int x = 0; x < width; ++x) {
                    auto* pixel = row + x * 4;
                    pixel[0] = 255;
                    pixel[1] = 0;
                    pixel[2] = 0;
                    pixel[3] = std::uint8_t(
                        255 - transparency[size_t(y) * size_t(width) + size_t(x)]);
                }
            }
            canvas->setMaskOverlay(
                overlay, QRect(0, 0, view->document_width(), view->document_height()));
            return;
        }
    }
    if (!view || !view->has_document() || mask_overlay_path(*view).isEmpty()) {
        canvas->setProperty(kMaskOverlayKey, QVariant());
        canvas->clearMaskOverlay();
        return;
    }
    // Re-render only when the overlay, its options, or the pixels changed.
    const QString key = QStringLiteral("%1:%2")
                            .arg(mask_overlay_revision(*view))
                            .arg(view->canvas_revision());
    if (canvas->property(kMaskOverlayKey).toString() == key) {
        return;
    }
    canvas->setProperty(kMaskOverlayKey, key);
    const QImage overlay = mask_overlay_image(*view, kMaskOverlayMaxSide);
    if (overlay.isNull()) {
        canvas->clearMaskOverlay();
        return;
    }
    canvas->setMaskOverlay(overlay,
                           QRect(0, 0, view->document_width(), view->document_height()));
}

} // namespace pictura
