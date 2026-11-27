// Image > Adjustments and Image > Auto Tone / Auto Contrast / Auto Color:
// destructive adjustments of the active pixel layer, within the selection.


#include "menu_registrar.h"

#include "commands.h"

#include "menu_registrar_internal.h"

#include "document_registry.h"
#include "panel_host.h"
#include "tools.h"

#include "api/layers_api.h"

#include "adjustment_catalogue.h"
#include "adjustment_dialog.h"
#include "panels/channels_panel.h"
#include "panels/properties_panel.h"
#include "api/document_api.h"
#include "api/document_bridge.h"
#include "fill_layer_editor.h"
#include "hdr_toning_dialog.h"
#include "match_color_dialog.h"
#include "replace_color_dialog.h"

#include "api/layers_bridge.h"
#include "api/filters_bridge.h"

namespace pictura {

namespace {

bool adjustmentChannelAvailable(const QString& kind, PictureView* view, PanelHost* panels)
{
    if (!adjustmentRequiresCompositeChannel(kind)) {
        return true;
    }
    auto* channels = panels ? panels->channelsPanel() : nullptr;
    return channels && channels->isCompositeChannelActive(view);
}

} // namespace

void MenuRegistrar::wireImageAdjustments()
{
    // A Smart Object takes no destructive adjustment (CS6 greys them out).
    const auto ready = [this]() {
        PictureView* view = activeView();
        return view && filter_target_ready(*view) && !filter_target_smart(*view);
    };
    const auto compositeRgb = [this]() {
        PictureView* view = activeView();
        if (!view || document_api::documentMode(*view) != QLatin1String("rgb")) {
            return true;
        }
        const ChannelsPanel* panel = panels_ ? panels_->channelsPanel() : nullptr;
        return !panel || (panel->currentRow() == 0 && panel->channelMask() == 0x7);
    };
    const QStringList compositeOnly{
        QStringLiteral("Exposure"), QStringLiteral("Vibrance"),
        QStringLiteral("Hue/Saturation"), QStringLiteral("Color Balance"),
        QStringLiteral("Black & White"), QStringLiteral("Photo Filter"),
        QStringLiteral("Channel Mixer"), QStringLiteral("Color Lookup"),
        QStringLiteral("Gradient Map"), QStringLiteral("Selective Color"),
        QStringLiteral("Desaturate")};
    for (const AdjustmentKind& entry : adjustmentCatalogue()) {
        if (entry.image.isEmpty()) {
            continue;
        }
        const QString kind = entry.kind;
        // The Auto commands live at the top of the Image menu, not under
        // Adjustments.
        const QString id = entry.image == QLatin1String("root")
                               ? commandIdForPath({QStringLiteral("Image"), entry.title})
                               : commandIdForPath({QStringLiteral("Image"),
                                                   QStringLiteral("Adjustments"), entry.title});
        commands_->setImplemented(id, true);
        const bool shadowsHighlights = kind == QLatin1String("shadows-highlights");
        const auto entryReady = [this, ready, shadowsHighlights]() {
            if (!ready()) {
                return false;
            }
            const int depth = image_depth_bits(*activeView());
            return !shadowsHighlights || depth == 8 || depth == 16;
        };
        const bool needsComposite = entry.image != QLatin1String("root")
                                    && compositeOnly.contains(entry.title);
        commands_->setEnabledProvider(id, [this, entryReady, compositeRgb, needsComposite, kind]() {
            return entryReady() && (!needsComposite || compositeRgb())
                   && adjustmentChannelAvailable(kind, activeView(), panels_);
        });
        if (entry.image != QLatin1String("dialog")) {
            commands_->setHandler(id, [this, kind]() {
                PictureView* view = activeView();
                if (view && image_adjust_direct(*view, kind)) {
                    refresh();
                } else if (view) {
                    reportFilterRefusal(view);
                }
            });
            continue;
        }
        // CS6 marks every dialog-opening entry with an ellipsis.
        const QString leaf = entry.title;
        commands_->setLabelProvider(id, [leaf]() { return commandDisplayText(leaf + QStringLiteral("…")); });
        commands_->setHandler(id, [this, kind, entryReady]() {
            PictureView* view = activeView();
            if (!view || !adjustmentChannelAvailable(kind, view, panels_)
                || (adjustmentRequiresCompositeChannel(kind) && filter_target_smart(*view))) {
                return;
            }
            if (!entryReady()) {
                reportFilterRefusal(view);
                return;
            }
            if (runAdjustmentDialog(dialogParent(), view, kind, tools_, imageView())) {
                refresh();
            }
        });
    }

    // HDR Toning is a neighborhood operator, not a pointwise adjustment, so it
    // runs through the filter preview/apply path via its own dialog.
    {
        const QString leaf = QStringLiteral("HDR Toning");
        const QString id =
            commandIdForPath({QStringLiteral("Image"), QStringLiteral("Adjustments"), leaf});
        commands_->setImplemented(id, true);
        commands_->setLabelProvider(id, [leaf]() { return commandDisplayText(leaf + QStringLiteral("…")); });
        commands_->setHandler(id, [this]() {
            PictureView* view = activeView();
            if (!view) {
                return;
            }
            if (!filter_target_ready(*view)) {
                reportFilterRefusal(view);
                return;
            }
            ImageView* canvas = imageView();
            const QRect visible = canvas ? canvas->visibleDocumentRect().toAlignedRect() : QRect();
            if (HdrToningDialog::get(dialogParent(), view, visible)) {
                refresh();
            }
        });
        commands_->setEnabledProvider(id, ready);
    }

    // Replace Color is dialog-only and non-modal, so its eyedropper can reach
    // the canvas; OK commits one "Replace Color" state.
    {
        const QString leaf = QStringLiteral("Replace Color");
        const QString id =
            commandIdForPath({QStringLiteral("Image"), QStringLiteral("Adjustments"), leaf});
        commands_->setImplemented(id, true);
        commands_->setLabelProvider(id, [leaf]() { return commandDisplayText(leaf + QStringLiteral("…")); });
        commands_->setHandler(id, [this]() {
            PictureView* view = activeView();
            if (!view || !filter_target_ready(*view)) {
                if (view) {
                    reportFilterRefusal(view);
                }
                return;
            }
            if (replaceColorDialog_) {
                replaceColorDialog_->raise();
                replaceColorDialog_->activateWindow();
                return;
            }
            auto* dialog = new ReplaceColorDialog(view, tools_, imageView(), dialogParent());
            dialog->setAttribute(Qt::WA_DeleteOnClose);
            replaceColorDialog_ = dialog;
            connect(dialog, &QDialog::finished, this, [this](int result) {
                if (result == QDialog::Accepted) {
                    refresh();
                }
            });
            dialog->show();
        });
        commands_->setEnabledProvider(id, ready);
    }

    // Match Color matches the active layer's statistics to another open
    // document, one of its layers, or the loaded statistics; it is RGB-only
    // (CS6), so Grayscale documents disable the row. Modal, like the other
    // adjustment dialogs.
    {
        const QString leaf = QStringLiteral("Match Color");
        const QString id =
            commandIdForPath({QStringLiteral("Image"), QStringLiteral("Adjustments"), leaf});
        commands_->setImplemented(id, true);
        commands_->setLabelProvider(id, [leaf]() { return commandDisplayText(leaf + QStringLiteral("…")); });
        commands_->setHandler(id, [this]() {
            PictureView* view = activeView();
            if (!view || !filter_target_ready(*view)) {
                if (view) {
                    reportFilterRefusal(view);
                }
                return;
            }
            QList<MatchColorDialog::Source> sources;
            for (int i = 0; i < documentCount(); ++i) {
                if (viewAt(i) && viewAt(i) != view) {
                    sources.append({documentName(i), viewAt(i)});
                }
            }
            MatchColorDialog dialog(view, imageView(), sources, dialogParent());
            if (runDialog(dialog, dialogParent()) == QDialog::Accepted) {
                refresh();
            }
        });
        commands_->setEnabledProvider(id, [this, compositeRgb]() {
            PictureView* view = activeView();
            return view && filter_target_ready(*view) && !filter_target_smart(*view)
                && document_api::documentMode(*view) == QStringLiteral("rgb")
                && compositeRgb();
        });
    }
}

// Layer > New Adjustment Layer: the same add_adjustment bridge the Adjustments
// panel uses, one entry per CS6 kind over the active document.
void MenuRegistrar::wireLayerAdjustments()
{
    const auto ready = [this]() {
        return activeView() && activeView()->has_document();
    };
    for (const AdjustmentKind& entry : adjustmentCatalogue()) {
        if (entry.fill || !entry.layer) {
            continue;
        }
        const QString leaf = entry.title;
        const QString id = commandIdForPath(
            {QStringLiteral("Layer"), QStringLiteral("New Adjustment Layer"), leaf});
        const QString kind = entry.kind;
        commands_->setImplemented(id, true);
        if (entry.layerEllipsis) {
            commands_->setLabelProvider(id, [leaf]() { return commandDisplayText(leaf + QStringLiteral("…")); });
        }
        commands_->setHandler(id, [this, kind]() {
            PictureView* view = activeView();
            if (view && adjustmentChannelAvailable(kind, view, panels_)
                && view->add_adjustment(kind)) {
                refresh();
            }
        });
        commands_->setEnabledProvider(id, [this, ready, kind]() {
            return ready() && adjustmentChannelAvailable(kind, activeView(), panels_);
        });
    }
}

// Layer > Layer Content Options opens the active fill/adjustment layer's
// Properties page. It is enabled only for a layer carrying an adjustment block,
// which covers both adjustment and fill layers.
void MenuRegistrar::wireLayerContentOptions()
{
    const auto currentAdjustmentPath = [this]() -> QString {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        if (!view || path.isEmpty()) {
            return QString();
        }
        const auto row = pictura::layers_api::rowForPath(*view, path);
        return row.hasAdjustment ? path : QString();
    };
    const QString id = QString::fromLatin1(command_ids::LayerContentOptions);
    commands_->setImplemented(id, true);
    commands_->setHandler(id, [this, currentAdjustmentPath]() {
        const QString path = currentAdjustmentPath();
        if (path.isEmpty()) {
            return;
        }
        PictureView* view = activeView();
        if (view && layer_is_editable_fill(*view, path)) {
            openFillLayerEditor(*view, path, dialogParent());
            return;
        }
        if (panels_->panelColumn()) {
            panels_->panelColumn()->showPanel(QStringLiteral("propertiesPanel"), true);
        }
        if (panels_->propertiesPanel()) {
            panels_->propertiesPanel()->refresh();
        }
    });
    commands_->setEnabledProvider(
        id, [currentAdjustmentPath]() { return !currentAdjustmentPath().isEmpty(); });
}

} // namespace pictura
