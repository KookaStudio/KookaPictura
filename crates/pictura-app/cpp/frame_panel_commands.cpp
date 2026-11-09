#include "frame_includes.h"

#include "commands.h"

#include "api/layers_api.h"
#include "api/layers_bridge.h"

#include "panels/actions_panel.h"
#include "panels/character_panel.h"
#include "panels/history_panel.h"
#include "panels/layers_panel.h"
#include "panels/paragraph_panel.h"
#include "panels/properties_panel.h"

namespace pictura {

void PicturaMainWindow::wirePanelMenuCommands()
{
    // Enable and Disable follow the mask state, and Apply needs a layer that
    // renders from its own pixels (not a smart object, group, or adjustment).
    registry_->setEnabledProvider(command_ids::LayerMaskEnable, [this]() {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        return view && !path.isEmpty() && layer_mask_present_path(*view, path)
            && layer_mask_disabled_path(*view, path);
    });
    registry_->setEnabledProvider(command_ids::LayerMaskDisable, [this]() {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        return view && !path.isEmpty() && layer_mask_present_path(*view, path)
            && !layer_mask_disabled_path(*view, path);
    });
    registry_->setEnabledProvider(command_ids::LayerMaskApply, [this]() {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        if (!view || path.isEmpty() || !layer_mask_present(*view)) {
            return false;
        }
        const auto row = pictura::layers_api::rowForPath(*view, path);
        return row.kind != QLatin1String("group") && row.kind != QLatin1String("adjustment")
            && !row.smartObject && !view->layer_is_fill_content(path);
    });

    registry_->setHandler(command_ids::LayerFlattenImage, [this]() {
        PictureView* view = activeView();
        if (view && view->has_document() && view->flatten_image() > 0) {
            registry_->recordCommand(command_ids::LayerFlattenImage, CommandParams{});
        }
    });
    registry_->setEnabledProvider(command_ids::LayerFlattenImage,
                                  [this]() { return activeView() && activeView()->has_document(); });

    // The Layers row menu's immediate create and move commands. They act on
    // the anchor the right-click wrote, not on a dialog.
    registry_->setHandler(command_ids::PanelLayersNewLayer, [this]() {
        if (layersPanel_) {
            layersPanel_->createLayerAtAnchor();
        }
    });
    registry_->setHandler(command_ids::PanelLayersNewGroup, [this]() {
        if (layersPanel_) {
            layersPanel_->createGroupAtAnchor();
        }
    });
    registry_->setHandler(command_ids::PanelLayersMoveUp, [this]() {
        if (layersPanel_) {
            layersPanel_->moveAnchor(1);
        }
    });
    registry_->setHandler(command_ids::PanelLayersMoveDown, [this]() {
        if (layersPanel_) {
            layersPanel_->moveAnchor(-1);
        }
    });
    for (const char* id : {command_ids::PanelLayersNewLayer, command_ids::PanelLayersNewGroup}) {
        registry_->setEnabledProvider(id, [this]() {
            return activeView() && activeView()->has_document();
        });
    }
    registry_->setEnabledProvider(command_ids::PanelLayersMoveUp, [this]() {
        return layersPanel_ && layersPanel_->canMoveCurrent(1);
    });
    registry_->setEnabledProvider(command_ids::PanelLayersMoveDown, [this]() {
        return layersPanel_ && layersPanel_->canMoveCurrent(-1);
    });
    registry_->setHandler(command_ids::PanelLayersEditAdjustment, [this]() {
        if (layersPanel_) {
            layersPanel_->editAnchor();
        }
    });
    registry_->setEnabledProvider(command_ids::PanelLayersEditAdjustment, [this]() {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        return view && !path.isEmpty()
            && pictura::layers_api::rowForPath(*view, path).hasAdjustment;
    });

    // Mask-thumbnail menu commands that act on the clicked mask.
    const auto maskSelection = [this](const char* id, const QString& kind) {
        registry_->setHandler(id, [this, kind]() {
            PictureView* view = activeView();
            const QString path = layerAnchor();
            if (view && !path.isEmpty()) {
                view->mask_selection(path, kind);
            }
        });
        registry_->setEnabledProvider(id, [this]() {
            PictureView* view = activeView();
            return view && view->has_document() && layer_mask_present(*view);
        });
    };
    maskSelection(command_ids::PanelLayersMaskAdd, QStringLiteral("add"));
    maskSelection(command_ids::PanelLayersMaskSubtract, QStringLiteral("subtract"));
    maskSelection(command_ids::PanelLayersMaskIntersect, QStringLiteral("intersect"));
    registry_->setHandler(command_ids::PanelLayersRefineMask, [this]() {
        if (layersPanel_) {
            layersPanel_->refineMaskAtAnchor();
        }
    });
    registry_->setEnabledProvider(command_ids::PanelLayersRefineMask, [this]() {
        PictureView* view = activeView();
        return view && view->has_document() && layer_mask_present(*view);
    });
    registry_->setHandler(command_ids::PanelLayersMaskOptions, [this]() {
        if (layersPanel_) {
            layersPanel_->openMaskOptionsFromMenu();
        }
    });
    registry_->setEnabledProvider(command_ids::PanelLayersMaskOptions, [this]() {
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
    registry_->setLabelProvider(command_ids::LayerDuplicateLayer, [this, rowKind]() {
        if (rowKind() == QLatin1String("group")) {
            return tr("Duplicate Group…");
        }
        return selectedLayerPaths().size() > 1 ? tr("Duplicate Layers…")
                                               : tr("Duplicate Layer…");
    });
    registry_->setLabelProvider(command_ids::LayerDeleteLayer, [this, rowKind]() {
        if (rowKind() == QLatin1String("group")) {
            return tr("Delete Group");
        }
        return selectedLayerPaths().size() > 1 ? tr("Delete Layers") : tr("Delete Layer");
    });


    const auto bindPanel = [this](const char* id, auto fn) {
        registry_->setHandler(id, fn);
    };
    bindPanel(command_ids::PanelLayersPanelOptions, [this]() {
        if (layersPanel_) {
            layersPanel_->performPanelMenuAction(QStringLiteral("panelOptions"));
        }
    });
    bindPanel(command_ids::PanelHistoryStepForward, [this]() {
        if (historyPanel_) {
            historyPanel_->performPanelMenuAction(QStringLiteral("stepForward"));
        }
    });
    bindPanel(command_ids::PanelHistoryStepBackward, [this]() {
        if (historyPanel_) {
            historyPanel_->performPanelMenuAction(QStringLiteral("stepBackward"));
        }
    });
    bindPanel(command_ids::PanelHistoryNewSnapshot, [this]() {
        if (historyPanel_) {
            historyPanel_->performPanelMenuAction(QStringLiteral("newSnapshot"));
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
            if (actionsPanel_) {
                actionsPanel_->performPanelMenuAction(QString::fromLatin1(id));
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
            if (characterPanel_) {
                characterPanel_->performPanelMenuAction(QString::fromLatin1(id));
            }
        });
    }
    const char* const paragraphIds[] = {
        command_ids::ParagraphHanging, command_ids::ParagraphJustification,
        command_ids::ParagraphSingleLineComposer, command_ids::ParagraphEveryLineComposer,
        command_ids::ParagraphReset};
    for (const char* id : paragraphIds) {
        bindPanel(id, [this, id]() {
            if (paragraphPanel_) {
                paragraphPanel_->performPanelMenuAction(QString::fromLatin1(id));
            }
        });
    }
    bindPanel(command_ids::PanelPropertiesSavePreset, [this]() {
        if (propertiesPanel_) {
            propertiesPanel_->performPanelMenuAction(QStringLiteral("savePreset"));
        }
    });
    bindPanel(command_ids::PanelPropertiesLoadPreset, [this]() {
        if (propertiesPanel_) {
            propertiesPanel_->performPanelMenuAction(QStringLiteral("loadPreset"));
        }
    });
    bindPanel(command_ids::PanelPropertiesClipToggle, [this]() {
        PictureView* view = activeView();
        if (view && propertiesPanel_) {
            propertiesPanel_->flushPending();
            clipping_toggle(*view, propertiesPanel_->currentPath());
            propertiesPanel_->refresh();
        }
    });
    bindPanel(command_ids::PanelPropertiesReset, [this]() {
        PictureView* view = activeView();
        if (view && propertiesPanel_) {
            propertiesPanel_->flushPending();
            adjustment_reset(*view, propertiesPanel_->currentPath());
            propertiesPanel_->refresh();
        }
    });
    bindPanel(command_ids::PanelPropertiesToggleVisibility, [this]() {
        PictureView* view = activeView();
        const QString path = propertiesPanel_ ? propertiesPanel_->currentPath() : QString();
        if (view && !path.isEmpty()) {
            propertiesPanel_->flushPending();
            const auto row = pictura::layers_api::rowForPath(*view, path);
            if (row.path == path) {
                view->set_layers_visible(QStringList{path}, !row.visible);
                propertiesPanel_->refresh();
            }
        }
    });
    bindPanel(command_ids::PanelPropertiesDeleteLayer, [this]() {
        if (layersPanel_ && propertiesPanel_) {
            propertiesPanel_->flushPending();
            layersPanel_->deletePathFromFooter(propertiesPanel_->currentPath());
        }
    });
}

} // namespace pictura
