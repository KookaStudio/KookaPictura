#include "menu_registrar.h"
#include <QtCore/QCoreApplication>

#include "menu_registrar_internal.h"

#include "panel_host.h"

#include "commands.h"
#include "document_command.h"

#include "api/layers_api.h"
#include "api/layers_bridge.h"

#include "panels/actions_panel.h"
#include "panels/channels_panel.h"
#include "panels/character_panel.h"
#include "panels/history_panel.h"
#include "panels/layers_panel.h"
#include "panels/paragraph_panel.h"
#include "panels/properties_panel.h"

namespace pictura {

void MenuRegistrar::wirePanelMenuCommands()
{
    // Enable and Disable follow the mask state, and Apply needs a layer that
    // renders from its own pixels (not a smart object, group, or adjustment).
    commands_->setEnabledProvider(command_ids::LayerMaskEnable, [this]() {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        return view && !path.isEmpty() && layer_mask_present_path(*view, path)
            && layer_mask_disabled_path(*view, path);
    });
    commands_->setEnabledProvider(command_ids::LayerMaskDisable, [this]() {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        return view && !path.isEmpty() && layer_mask_present_path(*view, path)
            && !layer_mask_disabled_path(*view, path);
    });
    commands_->setEnabledProvider(command_ids::LayerMaskApply, [this]() {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        if (!view || path.isEmpty() || !layer_mask_present(*view)) {
            return false;
        }
        const auto row = pictura::layers_api::rowForPath(*view, path);
        return row.kind != QLatin1String("group") && row.kind != QLatin1String("adjustment")
            && !row.smartObject && !view->layer_is_fill_content(path);
    });

    commands_->setHandler(command_ids::LayerFlattenImage, [this]() {
        PictureView* view = activeView();
        if (view && view->has_document()
            && runDocumentCommand(dialogParent(), *view, tr("Flatten Image"),
                                  [view] { view->flatten_image(); })) {
            commands_->recordCommand(command_ids::LayerFlattenImage, CommandParams{});
        }
    });
    commands_->setEnabledProvider(command_ids::LayerFlattenImage,
                                  [this]() { return activeView() && activeView()->has_document(); });

    // The Layers row menu's immediate create and move commands. They act on
    // the anchor the right-click wrote, not on a dialog.
    commands_->setHandler(command_ids::PanelLayersNewLayer, [this]() {
        if (panels_->layersPanel()) {
            panels_->layersPanel()->createLayerAtAnchor();
        }
    });
    commands_->setHandler(command_ids::PanelLayersNewGroup, [this]() {
        if (panels_->layersPanel()) {
            panels_->layersPanel()->createGroupAtAnchor();
        }
    });
    commands_->setHandler(command_ids::PanelChannelsDuplicateChannel, [this]() {
        if (runtime_.openDuplicateChannelDialog) {
            runtime_.openDuplicateChannelDialog();
        }
    });
    commands_->setEnabledProvider(command_ids::PanelChannelsDuplicateChannel, [this]() {
        return panels_ && panels_->channelsPanel()
            && panels_->channelsPanel()->canDuplicateCurrent();
    });
    commands_->setHandler(command_ids::PanelChannelsChannelOptions, [this]() {
        if (runtime_.openChannelOptionsDialog) {
            runtime_.openChannelOptionsDialog();
        }
    });
    commands_->setEnabledProvider(command_ids::PanelChannelsChannelOptions, [this]() {
        PictureView* view = activeView();
        if (!panels_ || !panels_->channelsPanel() || !view || !view->has_document()) {
            return false;
        }
        const int alpha =
            panels_->channelsPanel()->alphaIndexAt(panels_->channelsPanel()->currentRow());
        return alpha >= 0 && !channels_alpha_is_spot(*view, alpha);
    });
    commands_->setHandler(command_ids::PanelLayersMoveUp, [this]() {
        if (panels_->layersPanel()) {
            panels_->layersPanel()->moveAnchor(1);
        }
    });
    commands_->setHandler(command_ids::PanelLayersMoveDown, [this]() {
        if (panels_->layersPanel()) {
            panels_->layersPanel()->moveAnchor(-1);
        }
    });
    for (const char* id : {command_ids::PanelLayersNewLayer, command_ids::PanelLayersNewGroup}) {
        commands_->setEnabledProvider(id, [this]() {
            return activeView() && activeView()->has_document();
        });
    }
    commands_->setEnabledProvider(command_ids::PanelLayersMoveUp, [this]() {
        return panels_ && panels_->layersPanel() && panels_->layersPanel()->canMoveCurrent(1);
    });
    commands_->setEnabledProvider(command_ids::PanelLayersMoveDown, [this]() {
        return panels_ && panels_->layersPanel() && panels_->layersPanel()->canMoveCurrent(-1);
    });
    commands_->setHandler(command_ids::PanelLayersEditAdjustment, [this]() {
        if (panels_->layersPanel()) {
            panels_->layersPanel()->editAnchor();
        }
    });
    commands_->setEnabledProvider(command_ids::PanelLayersEditAdjustment, [this]() {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        return view && !path.isEmpty()
            && pictura::layers_api::rowForPath(*view, path).hasAdjustment;
    });

    // Mask-thumbnail menu commands that act on the clicked mask.
    const auto maskSelection = [this](const char* id, const QString& kind) {
        commands_->setHandler(id, [this, kind]() {
            PictureView* view = activeView();
            const QString path = layerAnchor();
            if (view && !path.isEmpty()) {
                view->mask_selection(path, kind);
            }
        });
        commands_->setEnabledProvider(id, [this]() {
            PictureView* view = activeView();
            return view && view->has_document() && layer_mask_present(*view);
        });
    };
    maskSelection(command_ids::PanelLayersMaskAdd, QStringLiteral("add"));
    maskSelection(command_ids::PanelLayersMaskSubtract, QStringLiteral("subtract"));
    maskSelection(command_ids::PanelLayersMaskIntersect, QStringLiteral("intersect"));
    commands_->setHandler(command_ids::PanelLayersRefineMask, [this]() {
        if (panels_->layersPanel()) {
            panels_->layersPanel()->refineMaskAtAnchor();
        }
    });
    commands_->setEnabledProvider(command_ids::PanelLayersRefineMask, [this]() {
        PictureView* view = activeView();
        return view && view->has_document() && layer_mask_present(*view);
    });
    commands_->setHandler(command_ids::PanelLayersMaskOptions, [this]() {
        if (panels_->layersPanel()) {
            panels_->layersPanel()->openMaskOptionsFromMenu();
        }
    });
    commands_->setEnabledProvider(command_ids::PanelLayersMaskOptions, [this]() {
        PictureView* view = activeView();
        return view && view->has_document() && layer_mask_present(*view);
    });

    // The row menu and the menu bar share these labels.
    const auto rowKind = [this]() {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        if (!view || path.isEmpty()) {
            return QString();
        }
        return pictura::layers_api::rowForPath(*view, path).kind;
    };
    commands_->setLabelProvider(command_ids::LayerDuplicateLayer, [this, rowKind]() {
        if (rowKind() == QLatin1String("group")) {
            return QCoreApplication::translate("PicturaCommands", "Duplicate Group…");
        }
        return selectedLayerPaths().size() > 1 ? QCoreApplication::translate("PicturaCommands", "Duplicate Layers…")
                                               : QCoreApplication::translate("PicturaCommands", "Duplicate Layer…");
    });
    commands_->setLabelProvider(command_ids::LayerDeleteLayer, [this, rowKind]() {
        if (rowKind() == QLatin1String("group")) {
            return QCoreApplication::translate("PicturaCommands", "Delete Group");
        }
        return selectedLayerPaths().size() > 1 ? QCoreApplication::translate("PicturaCommands", "Delete Layers") : QCoreApplication::translate("PicturaCommands", "Delete Layer");
    });


    const auto bindPanel = [this](const char* id, auto fn) {
        commands_->setHandler(id, fn);
    };
    bindPanel(command_ids::PanelLayersPanelOptions, [this]() {
        if (panels_->layersPanel()) {
            panels_->layersPanel()->performPanelMenuAction(QStringLiteral("panelOptions"));
        }
    });
    bindPanel(command_ids::PanelHistoryStepForward, [this]() {
        if (panels_->historyPanel()) {
            panels_->historyPanel()->performPanelMenuAction(QStringLiteral("stepForward"));
        }
    });
    bindPanel(command_ids::PanelHistoryStepBackward, [this]() {
        if (panels_->historyPanel()) {
            panels_->historyPanel()->performPanelMenuAction(QStringLiteral("stepBackward"));
        }
    });
    bindPanel(command_ids::PanelHistoryNewSnapshot, [this]() {
        if (panels_->historyPanel()) {
            panels_->historyPanel()->performPanelMenuAction(QStringLiteral("newSnapshot"));
        }
    });
    const char* const actionIds[] = {
        command_ids::ActionsButtonMode,       command_ids::ActionsNewAction,
        command_ids::ActionsNewSet,           command_ids::ActionsDuplicate,
        command_ids::ActionsDelete,           command_ids::ActionsPlay,
        command_ids::ActionsToggleRecording,  command_ids::ActionsRecordAgain,
        command_ids::ActionsInsertMenuItem,   command_ids::ActionsInsertStop,
        command_ids::ActionsInsertConditional, command_ids::ActionsInsertPath,
        command_ids::ActionsOptions,          command_ids::ActionsPlaybackOptions,
        command_ids::ActionsAllowToolRecording, command_ids::ActionsClear,
        command_ids::ActionsReset,            command_ids::ActionsLoad,
        command_ids::ActionsReplace,          command_ids::ActionsSave};
    for (const char* id : actionIds) {
        bindPanel(id, [this, id]() {
            if (panels_->actionsPanel()) {
                panels_->actionsPanel()->performPanelMenuAction(QString::fromLatin1(id));
            }
        });
    }
    const char* const characterIds[] = {
        command_ids::CharacterFauxBold,     command_ids::CharacterFauxItalic,
        command_ids::CharacterAllCaps,      command_ids::CharacterSmallCaps,
        command_ids::CharacterSuperscript,  command_ids::CharacterSubscript,
        command_ids::CharacterUnderline,    command_ids::CharacterStrikethrough,
        command_ids::CharacterFractionalWidths, command_ids::CharacterSystemLayout,
        command_ids::CharacterStandardLigatures,
        command_ids::CharacterContextualAlternates,
        command_ids::CharacterDiscretionaryLigatures, command_ids::CharacterSwash,
        command_ids::CharacterStylisticAlternates, command_ids::CharacterTitlingAlternates,
        command_ids::CharacterOrdinals, command_ids::CharacterFractions,
        command_ids::CharacterReset};
    for (const char* id : characterIds) {
        bindPanel(id, [this, id]() {
            if (panels_->characterPanel()) {
                panels_->characterPanel()->performPanelMenuAction(QString::fromLatin1(id));
            }
        });
    }
    const char* const paragraphIds[] = {
        command_ids::ParagraphHanging, command_ids::ParagraphJustification,
        command_ids::ParagraphSingleLineComposer, command_ids::ParagraphEveryLineComposer,
        command_ids::ParagraphReset};
    for (const char* id : paragraphIds) {
        bindPanel(id, [this, id]() {
            if (panels_->paragraphPanel()) {
                panels_->paragraphPanel()->performPanelMenuAction(QString::fromLatin1(id));
            }
        });
    }
    bindPanel(command_ids::PanelPropertiesSavePreset, [this]() {
        if (panels_->propertiesPanel()) {
            panels_->propertiesPanel()->performPanelMenuAction(QStringLiteral("savePreset"));
        }
    });
    bindPanel(command_ids::PanelPropertiesLoadPreset, [this]() {
        if (panels_->propertiesPanel()) {
            panels_->propertiesPanel()->performPanelMenuAction(QStringLiteral("loadPreset"));
        }
    });
    bindPanel(command_ids::PanelPropertiesClipToggle, [this]() {
        PictureView* view = activeView();
        if (view && panels_->propertiesPanel()) {
            panels_->propertiesPanel()->flushPending();
            clipping_toggle(*view, panels_->propertiesPanel()->currentPath());
            panels_->propertiesPanel()->refresh();
        }
    });
    bindPanel(command_ids::PanelPropertiesReset, [this]() {
        PictureView* view = activeView();
        if (view && panels_->propertiesPanel()) {
            panels_->propertiesPanel()->flushPending();
            adjustment_reset(*view, panels_->propertiesPanel()->currentPath());
            panels_->propertiesPanel()->refresh();
        }
    });
    bindPanel(command_ids::PanelPropertiesToggleVisibility, [this]() {
        PictureView* view = activeView();
        const QString path = panels_->propertiesPanel() ? panels_->propertiesPanel()->currentPath() : QString();
        if (view && !path.isEmpty()) {
            panels_->propertiesPanel()->flushPending();
            const auto row = pictura::layers_api::rowForPath(*view, path);
            if (row.path == path) {
                view->set_layers_visible(QStringList{path}, !row.visible);
                panels_->propertiesPanel()->refresh();
            }
        }
    });
    bindPanel(command_ids::PanelPropertiesDeleteLayer, [this]() {
        if (panels_->layersPanel() && panels_->propertiesPanel()) {
            panels_->propertiesPanel()->flushPending();
            panels_->layersPanel()->deletePathFromFooter(panels_->propertiesPanel()->currentPath());
        }
    });
}

} // namespace pictura
