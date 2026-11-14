// Layer > Matting: Remove Black Matte, Remove White Matte, and the Defringe…
// dialog, over the active pixel layer. Each edit is one undo state; the bridge
// refuses a locked or empty layer.


#include "menu_registrar.h"

#include "commands.h"

#include "menu_registrar_internal.h"

#include "document_registry.h"
#include "panel_host.h"
#include "tools.h"

#include "defringe_dialog.h"

#include "api/layers_bridge.h"

namespace pictura {

void MenuRegistrar::wireMattingActions()
{
    const auto ready = [this]() {
        PictureView* view = activeView();
        return view && view->has_document() && matting_target_ready(*view);
    };

    commands_->setHandler(command_ids::LayerMattingRemoveBlack, [this]() {
        PictureView* view = activeView();
        if (view) {
            layer_remove_black_matte(*view);
        }
    });
    commands_->setEnabledProvider(command_ids::LayerMattingRemoveBlack, ready);

    commands_->setHandler(command_ids::LayerMattingRemoveWhite, [this]() {
        PictureView* view = activeView();
        if (view) {
            layer_remove_white_matte(*view);
        }
    });
    commands_->setEnabledProvider(command_ids::LayerMattingRemoveWhite, ready);

    commands_->setHandler(command_ids::LayerMattingDefringe, [this]() {
        PictureView* view = activeView();
        if (!view) {
            return;
        }
        int width = 1;
        if (DefringeDialog::get(&width, dialogParent())) {
            layer_defringe(*view, width);
        }
    });
    commands_->setEnabledProvider(command_ids::LayerMattingDefringe, ready);
}

} // namespace pictura
