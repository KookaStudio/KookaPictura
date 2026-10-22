// Layer > Matting: Remove Black Matte, Remove White Matte, and the Defringe…
// dialog, over the active pixel layer. Each edit is one undo state; the bridge
// refuses a locked or empty layer.

#include "frame_includes.h"

#include "defringe_dialog.h"

#include "pictura_app/src/cxxqt_object/impl_layers/matting.cxxqt.h"

namespace pictura {

void PicturaMainWindow::wireMattingActions()
{
    const auto ready = [this]() {
        PictureView* view = activeView();
        return view && view->has_document() && matting_target_ready(*view);
    };

    registry_->setHandler(command_ids::LayerMattingRemoveBlack, [this]() {
        PictureView* view = activeView();
        if (view) {
            layer_remove_black_matte(*view);
        }
    });
    registry_->setEnabledProvider(command_ids::LayerMattingRemoveBlack, ready);

    registry_->setHandler(command_ids::LayerMattingRemoveWhite, [this]() {
        PictureView* view = activeView();
        if (view) {
            layer_remove_white_matte(*view);
        }
    });
    registry_->setEnabledProvider(command_ids::LayerMattingRemoveWhite, ready);

    registry_->setHandler(command_ids::LayerMattingDefringe, [this]() {
        PictureView* view = activeView();
        if (!view) {
            return;
        }
        int width = 1;
        if (DefringeDialog::get(&width, this)) {
            layer_defringe(*view, width);
        }
    });
    registry_->setEnabledProvider(command_ids::LayerMattingDefringe, ready);
}

} // namespace pictura
