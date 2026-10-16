// The rubylith overlay: the Layers panel toggles it and sets its display
// options on the view; the frame draws it on the active canvas.

#include "frame_includes.h"

#include "pictura_app/src/cxxqt_object/impl_layers/mask_commands.cxxqt.h"

namespace pictura {

namespace {

constexpr const char* kMaskOverlayKey = "picturaMaskOverlayKey";
// ponytail: one image for the whole document, so it is soft at high zoom on
// very large documents; crop it per viewport if that matters.
constexpr int kMaskOverlayMaxSide = 4096;

} // namespace

void PicturaMainWindow::wireMaskOverlay()
{
    connect(layersPanel_, &LayersPanel::maskDisplayChanged, this, &PicturaMainWindow::refresh);
}

void PicturaMainWindow::syncMaskOverlay(PictureView* view, ImageView* canvas)
{
    if (!canvas) {
        return;
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
