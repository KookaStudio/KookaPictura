// Layer > Layer Style: the Layer Style dialog on each effect, Copy / Paste /
// Clear Layer Style, Scale Effects, Global Light, Create Layers, Rasterize
// Layer Style, and Hide / Show All Effects.


#include "menu_registrar.h"

#include "commands.h"

#include "menu_registrar_internal.h"

#include "document_registry.h"
#include "panel_host.h"
#include "tools.h"

#include "dialogs.h"
#include "global_light_dialog.h"
#include "layer_style_dialog.h"

#include <QtWidgets/QInputDialog>

#include <cmath>
#include <utility>

#include "api/layers_bridge.h"

namespace pictura {

namespace {

// Command id and the dialog page it opens; "" is Blending Options.
const std::pair<const char*, const char*> kDialogLeaves[] = {
    {command_ids::LayerStyleBlendingOptions, ""},
    {command_ids::LayerStyleDropShadow, "dropShadow"},
    {command_ids::LayerStyleInnerShadow, "innerShadow"},
    {command_ids::LayerStyleOuterGlow, "outerGlow"},
    {command_ids::LayerStyleInnerGlow, "innerGlow"},
    {command_ids::LayerStyleBevelEmboss, "bevel"},
    {command_ids::LayerStyleSatin, "satin"},
    {command_ids::LayerStyleColorOverlay, "colorOverlay"},
    {command_ids::LayerStyleGradientOverlay, "gradientOverlay"},
    {command_ids::LayerStylePatternOverlay, "patternOverlay"},
    {command_ids::LayerStyleStroke, "stroke"},
};

} // namespace

void MenuRegistrar::wireLayerStyleMenu()
{
    const auto current = [this]() {
        return layerAnchor();
    };
    const auto selected = [this]() {
        return selectedLayerPaths();
    };
    const auto wire = [this](const QString& id, std::function<void()> handler,
                             std::function<bool()> enabled) {
        commands_->setImplemented(id, true);
        commands_->setHandler(id, std::move(handler));
        commands_->setEnabledProvider(id, std::move(enabled));
    };
    const auto canEdit = [this, current]() {
        PictureView* view = activeView();
        return view && view->has_document() && layer_style_can_edit(*view, current());
    };

    for (const auto& [id, effect] : kDialogLeaves) {
        const QString key = QString::fromLatin1(effect);
        wire(
            id,
            [this, current, key]() {
                PictureView* view = activeView();
                const QString path = current();
                if (view && layer_style_can_edit(*view, path)) {
                    LayerStyleDialog dialog(view, path, key, dialogParent());
                    runDialog(dialog, dialogParent());
                    refresh();
                }
            },
            canEdit);
    }

    wire(
        QString::fromLatin1(command_ids::LayerStyleGlobalLight),
        [this]() {
            PictureView* view = activeView();
            if (!view || !view->has_document()) {
                return;
            }
            GlobalLightDialog dialog(
                int(std::lround(layer_style_global_angle(*view))),
                int(std::lround(layer_style_global_altitude(*view))), dialogParent());
            if (runDialog(dialog, dialogParent()) == QDialog::Accepted && dialog.dirty()) {
                layer_style_set_global_light(*view, dialog.angle(), dialog.altitude());
            }
        },
        [this]() {
            PictureView* view = activeView();
            return view && view->has_document();
        });
    wire(
        QString::fromLatin1(command_ids::LayerRasterizeLayerStyle),
        [this, current]() {
            PictureView* view = activeView();
            const QString path = current();
            if (view && layer_style_can_rasterize(*view, path)) {
                rasterize_layer_style(*view, path);
                refresh();
            }
        },
        [this, current]() {
            PictureView* view = activeView();
            return view && view->has_document() && layer_style_can_rasterize(*view, current());
        });
    wire(
        QString::fromLatin1(command_ids::LayerStyleCreateLayers),
        [this, current]() {
            PictureView* view = activeView();
            const QString path = current();
            if (view && layer_style_can_create_layers(*view, path)) {
                layer_style_create_layers(*view, path);
                refresh();
            }
        },
        [this, current]() {
            PictureView* view = activeView();
            return view && view->has_document() && layer_style_can_create_layers(*view, current());
        });
    wire(
        QString::fromLatin1(command_ids::LayerStyleCopyLayerStyle),
        [this, current]() {
            if (PictureView* view = activeView()) {
                layer_style_copy(*view, current());
                refresh();
            }
        },
        [this, current, canEdit]() {
            return canEdit() && layer_style_has(*activeView(), current());
        });
    wire(
        QString::fromLatin1(command_ids::LayerStylePasteLayerStyle),
        [this, selected]() {
            PictureView* view = activeView();
            if (view) {
                layer_style_paste(*view, selected());
            }
        },
        [this, selected]() {
            PictureView* view = activeView();
            return view && view->has_document() && layer_style_can_paste()
                && !selected().isEmpty();
        });
    const auto anyStyled = [this, selected]() {
        PictureView* view = activeView();
        if (!view || !view->has_document()) {
            return false;
        }
        for (const QString& path : selected()) {
            if (layer_style_has(*view, path) && layer_style_can_edit(*view, path)) {
                return true;
            }
        }
        return false;
    };
    wire(
        QString::fromLatin1(command_ids::LayerStyleClearLayerStyle),
        [this, selected]() {
            PictureView* view = activeView();
            if (view) {
                layer_style_clear(*view, selected());
            }
        },
        anyStyled);
    wire(
        QString::fromLatin1(command_ids::LayerStyleScaleEffects),
        [this, selected]() {
            PictureView* view = activeView();
            bool ok = false;
            const int percent = QInputDialog::getInt(dialogParent(), tr("Scale Layer Effects"),
                                                     tr("Scale (%):"), 100, 1, 1000, 1, &ok);
            if (view && ok) {
                layer_style_scale(*view, selected(), percent);
            }
        },
        anyStyled);
    for (const bool visible : {false, true}) {
        wire(
            QString::fromLatin1(visible ? command_ids::LayerStyleShowAllEffects : command_ids::LayerStyleHideAllEffects),
            [this, visible]() {
                PictureView* view = activeView();
                if (view) {
                    layer_style_set_all_visible(*view, visible);
                }
            },
            [this, visible]() {
                // Show needs something hidden, Hide something shown.
                PictureView* view = activeView();
                return view && view->has_document() && layer_style_any_visible(*view, !visible);
            });
    }
}

} // namespace pictura
