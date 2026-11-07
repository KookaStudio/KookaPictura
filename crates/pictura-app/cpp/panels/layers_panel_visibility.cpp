// Visibility edits present through the region path, so they emit no `changed`.
// The panel patches the affected rows from the dedicated signal instead of
// rebuilding its snapshot and thumbnails.

#include "panels/layers_panel.h"

#include "panels/layers_panel_model.h"
#include "api/layers_bridge.h"

namespace pictura {

void LayersPanel::connectVisibility(PictureView* view)
{
    if (visibilityConnection_) {
        QObject::disconnect(visibilityConnection_);
    }
    if (!view) {
        return;
    }
    visibilityConnection_ = connect(
        view, &PictureView::layerVisibilityChanged, this,
        [this](const QStringList& paths, bool visible) {
            bool patched = false;
            for (const QString& path : paths) {
                if (!model_->indexForPath(path).isValid()) {
                    continue;
                }
                model_->setVisible(path, visible);
                patched = true;
            }
            visibilityPatched_ = patched;
        });
}

bool LayersPanel::consumeVisibilityPatch()
{
    const bool patched = visibilityPatched_;
    visibilityPatched_ = false;
    return patched;
}

} // namespace pictura
