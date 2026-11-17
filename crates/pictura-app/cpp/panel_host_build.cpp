#include "panel_host.h"

#include "api/document_api.h"
#include "api/engine_bridge.h"
#include "api/layers_bridge.h"
#include "api/paint_bridge.h"
#include "color_picker_dialog.h"
#include "commands.h"
#include "dialogs.h"
#include "document_registry.h"
#include "document_window.h"
#include "image_view.h"
#include "command_ids.h"
#include "fill_layer_editor.h"
#include "icons.h"
#include "panels/actions_engine_bridge.h"
#include "panels/actions_panel.h"
#include "panels/adjustments_panel.h"
#include "panels/brush_panel.h"
#include "panels/channels_panel.h"
#include "panels/character_panel.h"
#include "panels/character_styles_panel.h"
#include "panels/clone_source_panel.h"
#include "panels/color_panel.h"
#include "panels/glyphs_panel.h"
#include "panels/histogram_panel.h"
#include "panels/history_panel.h"
#include "panels/info_panel.h"
#include "panels/layer_comps_panel.h"
#include "panels/layers_panel.h"
#include "panels/navigator_panel.h"
#include "panels/notes_panel.h"
#include "panels/panel_group.h"
#include "panels/paragraph_panel.h"
#include "panels/paragraph_styles_panel.h"
#include "panels/paths_panel.h"
#include "panels/placeholder_panel.h"
#include "panels/properties_panel.h"
#include "panels/swatches_panel.h"
#include "session.h"

#include <QtCore/QDebug>
#include <QtCore/QJsonDocument>
#include <QtWidgets/QApplication>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>

namespace pictura {

QList<PanelGroup*> PanelHost::buildPanels(const SessionState& state)
{
    buildingPanels_ = true;
    layersPanel_ = new LayersPanel(colorState_, shell_);
    layersPanel_->setObjectName(QStringLiteral("layersPanel"));
    layersPanel_->setCommandRegistry(commands_);

    historyPanel_ = new HistoryPanel(shell_);
    historyPanel_->setObjectName(QStringLiteral("historyPanel"));

    navigatorPanel_ = new NavigatorPanel(shell_);
    navigatorPanel_->setObjectName(QStringLiteral("navigatorPanel"));

    colorPanel_ = new ColorPanel(colorState_, shell_);
    colorPanel_->setObjectName(QStringLiteral("colorPanel"));

    swatchesPanel_ = new SwatchesPanel(colorState_, shell_);
    swatchesPanel_->setObjectName(QStringLiteral("swatchesPanel"));
    ColorPickerDialog::setDefaultSwatchGrid(swatchesPanel_->grid());
    if (state.swatchesLoaded) {
        QList<Swatch> swatches;
        for (const QJsonValue& value : state.swatches) {
            const QJsonObject entry = value.toObject();
            const QColor color(entry.value(QStringLiteral("color")).toString());
            if (color.isValid()) {
                swatches.append({color, entry.value(QStringLiteral("name")).toString()});
            }
        }
        // An intentionally empty array clears the panel; a corrupt non-empty
        // array keeps the default palette rather than opening blank.
        if (!swatches.isEmpty() || state.swatches.isEmpty()) {
            swatchesPanel_->grid()->setSwatches(swatches);
        }
    }
    connect(swatchesPanel_->grid(), &SwatchGrid::countChanged, this,
            [this]() { requestSessionSave(); });

    infoPanel_ = new InfoPanel(shell_);
    infoPanel_->setObjectName(QStringLiteral("infoPanel"));

    histogramPanel_ = new HistogramPanel(shell_);
    histogramPanel_->setObjectName(QStringLiteral("histogramPanel"));

    gradientsPanel_ = new PlaceholderPanel(QStringLiteral("Gradients"), QString(), shell_);
    gradientsPanel_->setObjectName(QStringLiteral("gradientsPanel"));

    patternsPanel_ = new PlaceholderPanel(QStringLiteral("Patterns"), QString(), shell_);
    patternsPanel_->setObjectName(QStringLiteral("patternsPanel"));

    notesPanel_ = new NotesPanel(shell_);
    notesPanel_->setObjectName(QStringLiteral("notesPanel"));

    propertiesPanel_ = new PropertiesPanel(shell_);
    propertiesPanel_->setObjectName(QStringLiteral("propertiesPanel"));
    propertiesPanel_->setCommandRegistry(commands_);
    connect(propertiesPanel_, &PropertiesPanel::modeRequested, this,
            [this](const QString& mode) {
                if (hooks_.convertImageMode) {
                    hooks_.convertImageMode(mode);
                }
            });
    connect(propertiesPanel_, &PropertiesPanel::depthRequested, this,
            [this](int depth) {
                if (hooks_.convertImageDepth) {
                    hooks_.convertImageDepth(depth);
                }
            });
    setPanelMenuHook(QStringLiteral("propertiesPanel"),
                     [this](QMenu* menu) { propertiesPanel_->updatePanelMenu(menu); });

    adjustmentsPanel_ = new AdjustmentsPanel(shell_);
    adjustmentsPanel_->setObjectName(QStringLiteral("adjustmentsPanel"));
    // The grid is a launcher: an adjustment icon creates the layer through the
    // panel-menu path, and the engine selects the created layer so the
    // Properties panel opens its controls.
    connect(adjustmentsPanel_, &AdjustmentsPanel::adjustmentRequested, this,
            [this](const QString& kind) {
                if (!layersPanel_ || !hooks_.activeView || !hooks_.activeView()) {
                    return;
                }
                layersPanel_->performPanelMenuAction(QStringLiteral("adjustment:") + kind);
            });
    // Fill icons are the quick-add path: create with defaults, then open the
    // type editor on the new layer.
    connect(adjustmentsPanel_, &AdjustmentsPanel::fillRequested, this,
            [this](const QString& type) {
                PictureView* view = hooks_.activeView ? hooks_.activeView() : nullptr;
                if (view) {
                    createFillLayerFromDialog(*view, type, colorState_->foreground(), shell_,
                                              false);
                }
            });

    channelsPanel_ = new ChannelsPanel(shell_);
    channelsPanel_->setObjectName(QStringLiteral("channelsPanel"));
    // Channel visibility follows the active document; the panel resets it on a
    // document switch, so applying it to every window's canvas keeps them in
    // step.
    connect(channelsPanel_, &ChannelsPanel::channelMaskChanged, this, [this](int mask) {
        if (!documents_) {
            return;
        }
        for (const DocWindow* window : documents_->windows()) {
            if (window->canvas) {
                window->canvas->setChannelMask(mask);
            }
        }
    });
    connect(channelsPanel_, &ChannelsPanel::saveOptionsRequested, this, [this]() {
        if (hooks_.saveSelectionOptions) {
            hooks_.saveSelectionOptions();
        }
    });
    connect(channelsPanel_, &ChannelsPanel::alphaVisibilityChanged, this, [this]() {
        if (hooks_.alphaVisibilityChanged) {
            hooks_.alphaVisibilityChanged();
        }
    });

    pathsPanel_ = new PathsPanel(shell_);
    pathsPanel_->setObjectName(QStringLiteral("pathsPanel"));

    layerCompsPanel_ = new LayerCompsPanel(shell_);
    layerCompsPanel_->setObjectName(QStringLiteral("layerCompsPanel"));
    setPanelMenuHook(QStringLiteral("layerCompsPanel"),
                     [this](QMenu* menu) { layerCompsPanel_->updatePanelMenu(menu); });

    actionsPanel_ = new ActionsPanel(shell_);
    actionsPanel_->setObjectName(QStringLiteral("actionsPanel"));
    wireActionsPanel(state);

    // `Styles` takes the former Gradients/Patterns tab slot in the default set.
    stylesPanel_ = new PlaceholderPanel(QStringLiteral("Styles"), QString(), shell_);
    stylesPanel_->setObjectName(QStringLiteral("stylesPanel"));

    brushPanel_ = new BrushPanel(shell_);
    brushPanel_->setObjectName(QStringLiteral("brushPanel"));

    cloneSourcePanel_ = new CloneSourcePanel(shell_);
    cloneSourcePanel_->setObjectName(QStringLiteral("cloneSourcePanel"));

    characterPanel_ = new CharacterPanel(shell_);
    characterPanel_->setObjectName(QStringLiteral("characterPanel"));
    characterPanel_->setShowAsianOptions(state.typeShowAsianOptions);
    paragraphPanel_ = new ParagraphPanel(shell_);
    paragraphPanel_->setObjectName(QStringLiteral("paragraphPanel"));
    paragraphPanel_->setMiddleEastern(state.typeMiddleEastern);

    glyphsPanel_ = new GlyphsPanel(shell_);
    glyphsPanel_->setObjectName(QStringLiteral("glyphsPanel"));

    paragraphStylesPanel_ = new ParagraphStylesPanel(shell_);
    paragraphStylesPanel_->setObjectName(QStringLiteral("paragraphStylesPanel"));
    characterStylesPanel_ = new CharacterStylesPanel(shell_);
    characterStylesPanel_->setObjectName(QStringLiteral("characterStylesPanel"));

    // Content widgets, not docks: the PanelColumn hosts them. Registering the
    // objectNames keeps the Window menu and the duplicate-name guard working.
    registerPanel(layersPanel_, Qt::RightDockWidgetArea);
    registerPanel(historyPanel_, Qt::RightDockWidgetArea);
    registerPanel(navigatorPanel_, Qt::RightDockWidgetArea);
    registerPanel(colorPanel_, Qt::RightDockWidgetArea);
    registerPanel(swatchesPanel_, Qt::RightDockWidgetArea);
    registerPanel(stylesPanel_, Qt::RightDockWidgetArea);
    registerPanel(infoPanel_, Qt::RightDockWidgetArea);
    registerPanel(histogramPanel_, Qt::RightDockWidgetArea);
    registerPanel(gradientsPanel_, Qt::RightDockWidgetArea);
    registerPanel(patternsPanel_, Qt::RightDockWidgetArea);
    registerPanel(notesPanel_, Qt::RightDockWidgetArea);
    registerPanel(propertiesPanel_, Qt::RightDockWidgetArea);
    registerPanel(adjustmentsPanel_, Qt::RightDockWidgetArea);
    registerPanel(channelsPanel_, Qt::RightDockWidgetArea);
    registerPanel(pathsPanel_, Qt::RightDockWidgetArea);
    registerPanel(layerCompsPanel_, Qt::RightDockWidgetArea);
    registerPanel(actionsPanel_, Qt::RightDockWidgetArea);
    registerPanel(brushPanel_, Qt::RightDockWidgetArea);
    registerPanel(cloneSourcePanel_, Qt::RightDockWidgetArea);
    registerPanel(characterPanel_, Qt::RightDockWidgetArea);
    registerPanel(paragraphPanel_, Qt::RightDockWidgetArea);
    registerPanel(glyphsPanel_, Qt::RightDockWidgetArea);
    registerPanel(paragraphStylesPanel_, Qt::RightDockWidgetArea);
    registerPanel(characterStylesPanel_, Qt::RightDockWidgetArea);

    // CS6 Essentials groups and order. The tab text is the panel title and the
    // tab icon reuses the panel's `window.panels.<name>` asset.
    auto addPanel = [](PanelGroup* group, QWidget* panel, const QString& title,
                       const QString& iconId) {
        group->addPanel(panel, title, pictura::icon(iconId));
    };
    // A fresh session is regrouped into the Essentials layout right after
    // (applyFreshSessionDefault), so its groups are staged rather than laid out
    // in the column only to be taken apart again.
    const bool freshLayout = !sessionHasPanelLayout(state);
    QList<PanelGroup*> staged;
    auto attachGroup = [this, freshLayout, &staged](PanelGroup* group) {
        if (freshLayout) {
            staged.append(group);
        } else {
            panelColumn_->addGroup(group);
        }
    };

    auto* colorGroup = new PanelGroup(shell_);
    addPanel(colorGroup, colorPanel_, tr("Color"), QString::fromLatin1(command_ids::WindowPanelsColor));
    addPanel(colorGroup, swatchesPanel_, tr("Swatches"),
             QString::fromLatin1(command_ids::WindowPanelsSwatches));
    addPanel(colorGroup, stylesPanel_, tr("Styles"), QStringLiteral("window.panels.styles"));
    attachGroup(colorGroup);

    // Properties folds into the Adjustments group; its tab starts hidden.
    auto* adjustmentsGroup = new PanelGroup(shell_);
    addPanel(adjustmentsGroup, adjustmentsPanel_, tr("Adjustments"),
             QString::fromLatin1(command_ids::WindowPanelsAdjustments));
    addPanel(adjustmentsGroup, propertiesPanel_, tr("Properties"),
             QString::fromLatin1(command_ids::WindowPanelsProperties));
    attachGroup(adjustmentsGroup);

    auto* layersGroup = new PanelGroup(shell_);
    addPanel(layersGroup, layersPanel_, tr("Layers"),
             QString::fromLatin1(command_ids::WindowPanelsLayers));
    addPanel(layersGroup, channelsPanel_, tr("Channels"),
             QString::fromLatin1(command_ids::WindowPanelsChannels));
    addPanel(layersGroup, pathsPanel_, tr("Paths"),
             QString::fromLatin1(command_ids::WindowPanelsPaths));
    attachGroup(layersGroup);

    auto* navigatorGroup = new PanelGroup(shell_);
    addPanel(navigatorGroup, navigatorPanel_, tr("Navigator"),
             QString::fromLatin1(command_ids::WindowPanelsNavigator));
    addPanel(navigatorGroup, histogramPanel_, tr("Histogram"),
             QString::fromLatin1(command_ids::WindowPanelsHistogram));
    addPanel(navigatorGroup, infoPanel_, tr("Info"),
             QString::fromLatin1(command_ids::WindowPanelsInfo));
    attachGroup(navigatorGroup);

    // Iconic single-panel groups.
    auto* historyGroup = new PanelGroup(shell_);
    addPanel(historyGroup, historyPanel_, tr("History"),
             QString::fromLatin1(command_ids::WindowPanelsHistory));
    attachGroup(historyGroup);

    auto* actionsGroup = new PanelGroup(shell_);
    addPanel(actionsGroup, actionsPanel_, tr("Actions"),
             QString::fromLatin1(command_ids::WindowPanelsActions));
    attachGroup(actionsGroup);
    setPanelMenuHook(QStringLiteral("actionsPanel"),
                     [this](QMenu* menu) { actionsPanel_->updatePanelMenu(menu); });
    actionsPanel_->applySessionState(state);

    // Kept registered and Window-menu reachable, but out of the default groups.
    auto* overflowGroup = new PanelGroup(shell_);
    addPanel(overflowGroup, gradientsPanel_, tr("Gradients"),
             QString::fromLatin1(command_ids::WindowPanelsGradients));
    addPanel(overflowGroup, patternsPanel_, tr("Patterns"),
             QString::fromLatin1(command_ids::WindowPanelsPatterns));
    addPanel(overflowGroup, notesPanel_, tr("Notes"),
             QString::fromLatin1(command_ids::WindowPanelsNotes));
    // The painting panels open from Window > Panels or an options-bar toggle.
    addPanel(overflowGroup, brushPanel_, tr("Brush"),
             QString::fromLatin1(command_ids::WindowPanelsBrush));
    addPanel(overflowGroup, cloneSourcePanel_, tr("Clone Source"),
             QString::fromLatin1(command_ids::WindowPanelsCloneSource));
    addPanel(overflowGroup, layerCompsPanel_, tr("Layer Comps"),
             QStringLiteral("window.panels.layer.comps"));
    attachGroup(overflowGroup);

    // CS6 docks Character and Paragraph as one group, opened from the Window
    // or Type menu or the Type options bar's panel toggle; Glyphs (post-CS6)
    // joins them.
    auto* typeGroup = new PanelGroup(shell_);
    addPanel(typeGroup, characterPanel_, tr("Character"),
             QString::fromLatin1(command_ids::WindowPanelsCharacter));
    setPanelMenuHook(QStringLiteral("characterPanel"),
                     [this](QMenu* menu) { characterPanel_->updatePanelMenu(menu); });
    addPanel(typeGroup, paragraphPanel_, tr("Paragraph"),
             QString::fromLatin1(command_ids::WindowPanelsParagraph));
    setPanelMenuHook(QStringLiteral("paragraphPanel"),
                     [this](QMenu* menu) { paragraphPanel_->updatePanelMenu(menu); });
    addPanel(typeGroup, glyphsPanel_, tr("Glyphs"),
             QString::fromLatin1(command_ids::WindowPanelsGlyphs));
    addPanel(typeGroup, paragraphStylesPanel_, tr("Paragraph Styles"),
             QString::fromLatin1(command_ids::WindowPanelsParagraph));
    addPanel(typeGroup, characterStylesPanel_, tr("Character Styles"),
             QString::fromLatin1(command_ids::WindowPanelsCharacterStyles));
    attachGroup(typeGroup);

    // Default visibility matches the pre-M41 layout: Color/Swatches/Styles,
    // Adjustments, Layers/Channels/Paths and Navigator/Histogram/Info visible;
    // History, Actions and the overflow panels hidden.
    panelColumn_->showPanel(QStringLiteral("historyPanel"), false);
    panelColumn_->showPanel(QStringLiteral("actionsPanel"), false);
    panelColumn_->showPanel(QStringLiteral("propertiesPanel"), false);
    panelColumn_->showPanel(QStringLiteral("gradientsPanel"), false);
    panelColumn_->showPanel(QStringLiteral("patternsPanel"), false);
    panelColumn_->showPanel(QStringLiteral("notesPanel"), false);
    panelColumn_->showPanel(QStringLiteral("brushPanel"), false);
    panelColumn_->showPanel(QStringLiteral("cloneSourcePanel"), false);
    panelColumn_->showPanel(QStringLiteral("glyphsPanel"), false);
    panelColumn_->showPanel(QStringLiteral("characterPanel"), false);
    panelColumn_->showPanel(QStringLiteral("paragraphPanel"), false);
    panelColumn_->showPanel(QStringLiteral("paragraphStylesPanel"), false);
    panelColumn_->showPanel(QStringLiteral("characterStylesPanel"), false);
    panelColumn_->showPanel(QStringLiteral("layerCompsPanel"), false);

    wireStaticPanels();
    buildingPanels_ = false;
    return staged;
}

void PanelHost::wireActionsPanel(const SessionState& state)
{
    if (actionsEngine_) {
        set_active_actions_engine(actionsEngine_);
        actionsPanel_->setEngine(actionsEngine_);
        const bool libraryLoaded = actionsEngine_->loadLibrary();
        const QString libraryError = actionsEngine_->lastError();
        if (!libraryLoaded || !libraryError.isEmpty()) {
            const QString message =
                libraryError.isEmpty()
                    ? tr("The Actions library could not be loaded.")
                    : tr("The Actions library could not be loaded: %1").arg(libraryError);
            qWarning().noquote() << message;
            if (hooks_.showStatus) {
                hooks_.showStatus(message, 10000);
            }
        }
        connect(qApp, &QCoreApplication::aboutToQuit, this, [this] {
            if (actionsEngine_) {
                actionsEngine_->saveLibrary();
            }
        });
    }
    actionsPanel_->setRegistry(commands_);
    actionsPanel_->setPathSelectionProvider([this]() -> QString {
        if (!pathsPanel_ || !pathsPanel_->listForTest()) {
            return QString();
        }
        QListWidgetItem* item = pathsPanel_->listForTest()->currentItem();
        return item ? item->text() : QString();
    });
    actionsPanel_->setStopHandler([this](const QString& message, bool allowContinue) {
        QMessageBox box(QMessageBox::Information, tr("Stop"), message, QMessageBox::NoButton,
                        shell_);
        QPushButton* proceed =
            box.addButton(allowContinue ? tr("Continue") : tr("OK"), QMessageBox::AcceptRole);
        box.addButton(tr("Cancel"), QMessageBox::RejectRole);
        box.setDefaultButton(proceed);
        runDialog(box, shell_);
        return box.clickedButton() == proceed;
    });
    actionsPanel_->setToolStepExecutor([this](const QJsonObject& payload) {
        PictureView* view = hooks_.activeView ? hooks_.activeView() : nullptr;
        if (!view || !view->has_document()) {
            return false;
        }
        const QString json =
            QString::fromUtf8(QJsonDocument(payload).toJson(QJsonDocument::Compact));
        return pictura::replay_tool_stroke(*view, json);
    });
    actionsPanel_->setDocumentFactsProvider([this]() {
        ActionsPanel::DocumentFacts facts;
        PictureView* view = hooks_.activeView ? hooks_.activeView() : nullptr;
        if (!view || !view->has_document()) {
            return facts;
        }
        facts.present = true;
        facts.square = view->document_width() == view->document_height();
        facts.mode = document_api::documentMode(*view);
        facts.hasSelection = view->has_selection();
        return facts;
    });
    actionsPanel_->setPathStepExecutor([this](const QString& name) {
        PictureView* view = hooks_.activeView ? hooks_.activeView() : nullptr;
        if (!view || !view->has_document()) {
            return false;
        }
        const int count = paths_saved_count(*view);
        for (int i = 0; i < count; ++i) {
            if (paths_saved_name(*view, i) == name) {
                return paths_set_active(*view, i);
            }
        }
        return false;
    });
    connect(actionsPanel_, &ActionsPanel::errorOccurred, this, [this](const QString& message) {
        if (hooks_.showStatus) {
            hooks_.showStatus(message, 8000);
        }
    });
    connect(actionsPanel_, &ActionsPanel::functionKeyWarning, this,
            [this](const QString& message) {
                if (hooks_.showStatus) {
                    hooks_.showStatus(message, 8000);
                }
            });
}

void PanelHost::wireStaticPanels()
{
    // Layers row content edit: make the clicked row current, then open the fill
    // editor or the Layer Content Options command.
    connect(layersPanel_, &LayersPanel::editContentRequested, this, [this](const QString& path) {
        PictureView* view = hooks_.activeView ? hooks_.activeView() : nullptr;
        if (layersPanel_ && !path.isEmpty() && view) {
            layersPanel_->selectPaths(QStringList{path}, path);
        }
        if (view && layer_is_editable_fill(*view, path)) {
            openFillLayerEditor(*view, path, shell_);
            return;
        }
        if (commands_) {
            commands_->dispatch(QString::fromLatin1(command_ids::LayerContentOptions));
        }
    });
    // Edit Smart Filter… on a filter row reopens that filter's own dialog.
    connect(layersPanel_, &LayersPanel::smartFilterEditRequested, this,
            [this](const QString& path, int index) {
                PictureView* view = hooks_.activeView ? hooks_.activeView() : nullptr;
                if (!view || !commands_) {
                    return;
                }
                if (layersPanel_) {
                    layersPanel_->selectPaths(QStringList{path}, path);
                }
                switch (layer_smart_filter_edit_kind(*view, path, index)) {
                case 1:
                    commands_->dispatch(QString::fromLatin1(command_ids::FilterPicturaRaw));
                    break;
                case 2:
                    commands_->dispatch(QString::fromLatin1(command_ids::FilterLensCorrection));
                    break;
                case 3:
                case 4:
                    if (hooks_.editSmartFilter) {
                        hooks_.editSmartFilter(path, index);
                    }
                    break;
                default:
                    break;
                }
            });

    connect(layersPanel_, &LayersPanel::editTargetChanged, this,
            &PanelHost::layerEditTargetChanged);
    connect(layersPanel_, &LayersPanel::selectionChanged, this, [this] {
        // Panel target writes are silent even when the mask/image edit kind
        // stays unchanged; the host's structural panels refresh locally.
        if (propertiesPanel_) {
            propertiesPanel_->refresh();
        }
        if (commands_) {
            if (refreshingDock_) {
                registryRefreshPending_ = true;
            } else if (!panelsRefreshedByChanged_
                       || !layersPanel_->refreshInProgress()) {
                // A user-driven selection change must update labels and
                // enablement immediately, even right after an edit.
                commands_->refresh();
            }
        }
        // The type panels follow the active layer; selection does not bump the
        // view's `changed`, so refresh them here rather than from the timer.
        if (characterPanel_) {
            characterPanel_->refresh();
        }
        if (paragraphPanel_) {
            paragraphPanel_->refresh();
        }
        if (paragraphStylesPanel_) {
            paragraphStylesPanel_->refresh();
        }
        if (characterStylesPanel_) {
            characterStylesPanel_->refresh();
        }
        emit layerSelectionChanged();
    });
}

} // namespace pictura
