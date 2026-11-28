#include "frame_includes.h"
#include "panels/brush_panel.h"
#include "panels/character_panel.h"
#include "panels/color_panel.h"
#include "panels/clone_source_panel.h"
#include "panels/glyphs_panel.h"
#include "panels/info_panel.h"
#include "panels/layers_panel.h"
#include "panels/notes_panel.h"
#include "panels/paragraph_panel.h"
#include "panels/paths_panel.h"
#include "panels/properties_panel.h"
#include "panels/tool_presets_panel.h"
#include "panels/panel_column.h"
#include "menu_registrar.h"
#include "api/layers_bridge.h"

#include "api/document_api.h"
#include "color_picker_dialog.h"
#include "fill_layer_editor.h"
#include "command_ids.h"
#include "notification_status_bar.h"

#include "api/paint_bridge.h"
#include "api/engine_bridge.h"

#include "fonts.h"
#include "panels/actions_engine_bridge.h"
#include "panels/actions_panel.h"

#include <QtWidgets/QSlider>

#include <QtWidgets/QProgressBar>
#include "panels/brush_preset_picker.h"
#include "panels/numeric_field.h"
#include "text_input.h"

#include <QtCore/QDebug>
#include <QtCore/QEvent>
#include <QtCore/QJsonDocument>
#include <QtCore/QTimer>
#include <QtGui/QFont>
#include <QtGui/QMouseEvent>
#include <QtWidgets/QApplication>
#include <QtWidgets/QFrame>
#include <QtWidgets/QStackedWidget>

namespace pictura {

QColor PicturaMainWindow::foregroundColor() const
{
    return panels_ && panels_->colorState() ? panels_->colorState()->foreground()
                                            : QColor(Qt::black);
}

QColor PicturaMainWindow::backgroundColor() const
{
    return panels_ && panels_->colorState() ? panels_->colorState()->background()
                                            : QColor(Qt::white);
}

void PicturaMainWindow::togglePanel(const QString& objectName)
{
    if (panels_) {
        panels_->togglePanel(objectName);
    }
}

void PicturaMainWindow::buildTools(int toolsColumns, bool useShiftKeyForToolSwitch,
                                   const QJsonObject& toolOptions)
{
    tools_ = new ToolController(this);
    applyToolOptionsJson(*tools_, toolOptions);
    tools_->setViewProvider([this]() { return activeView(); });
    // Every picker opened without its own sampler (the toolbox and Color panel
    // swatches, options bars, filter dialogs) samples the active document's
    // composite through this pair, as CS6's picker eyedropper does. The window
    // is held weakly, so a destroyed window leaves the functions inert.
    {
        QPointer<PicturaMainWindow> self(this);
        ColorPickerDialog::setDefaultCanvasSampler(
            [self]() -> ToolController* { return self ? self->tools_ : nullptr; },
            [self](const QPoint& pos) -> QColor {
                PictureView* view = self ? self->activeView() : nullptr;
                if (!view) {
                    return {};
                }
                const quint32 argb = view->sample_argb(pos.x(), pos.y());
                return argb != 0 ? QColor::fromRgb(argb) : QColor();
            });
    }
    connect(tools_, &ToolController::commandRequested, this, [this](const QString& id) {
        if (commandRegistry()) {
            commandRegistry()->dispatch(id);
        }
    });
    panels_->pathsPanel()->setToolServices(tools_);
    panels_->propertiesPanel()->setTools(tools_);

    // A tool commit (marquee release, wand click, lasso close) reaches the
    // picture view without a `changed` emission, so refresh the overlay directly.
    connect(tools_, &ToolController::selectionCommitted, this,
            &PicturaMainWindow::refreshSelectionOverlay);
    connect(tools_, &ToolController::selectionCommitted, panels_->propertiesPanel(),
            &PropertiesPanel::scheduleThresholdSelectionRefresh);
    connect(tools_, &ToolController::selectionMovePreviewed, panels_->propertiesPanel(),
            &PropertiesPanel::scheduleThresholdSelectionRefresh);
    // A locked pixel edit is refused silently at the bridge; report it so the
    // user learns why nothing was painted.
    connect(tools_, &ToolController::pixelEditRefused, this, [this](const QString& message) {
        statusBar()->showMessage(message, 4000);
    });

    ColorState* colorState = panels_->colorState();
    tools_->setForeground(colorState->foreground());
    tools_->setBackground(colorState->background());
    colorState->setHdrPickerContext([this] { return activeView(); }, [this] {
        auto* canvas = imageView();
        return canvas ? canvas->hdrExposure() : 0;
    });
    connect(colorState, &ColorState::foregroundChanged, tools_,
            [this](const QColor& color) {
                if (tools_->foreground() != color) {
                    tools_->setForeground(color, true);
                }
            });
    connect(colorState, &ColorState::backgroundChanged, tools_,
            [this](const QColor& color) {
                if (tools_->background() != color) {
                    tools_->setBackground(color, true);
                }
            });
    connect(colorState, &ColorState::foregroundChanged, this,
            [this](const QColor&) { updateToolHint(); });
    connect(tools_, &ToolController::foregroundSelected, colorState, &ColorState::setForeground);
    connect(tools_, &ToolController::backgroundSelected, colorState, &ColorState::setBackground);

    auto* toolbox = new Toolbox(tools_, colorState, this);
    toolbox_ = toolbox;
    wireQuickMask();
    toolbox->setShiftKeyForToolSwitch(useShiftKeyForToolSwitch);

    // D1: the Tools panel is a tabless, atomic `PanelColumn` hosting the tool
    // content as one plain child in the central splitter, default left at index
    // 0. Its header toggle reflows the tool grid one<->two columns (D2), not the
    // normal/iconic rail mode.
    PanelColumn* toolsColumn = panels_->createToolsColumn();
    toolsColumn->setToolsContent(
        toolbox, [toolbox]() { return toolbox->columns(); },
        [this]() {
            if (toolbox_) {
                toolbox_->setColumns(toolbox_->columns() == 1 ? 2 : 1);
            }
        });
    // The screen-mode button opens a menu of the same three commands `F` and
    // `Shift+F` cycle, sharing the registry actions so checks and handlers agree.
    toolbox->setScreenModeActions({
        commandRegistry()->action(QString::fromLatin1(command_ids::ViewScreenModeStandard)),
        commandRegistry()->action(QString::fromLatin1(command_ids::ViewScreenModeFullWithMenuBar)),
        commandRegistry()->action(QString::fromLatin1(command_ids::ViewScreenModeFull)),
    });
    toolbox->setActiveScreenMode(static_cast<int>(screenMode_));
    // CommandRegistry::refresh() rewrites every action's text from its spec,
    // which would drop the toolbox's display-only `F` hint. Pin the hinted text
    // with label providers so the shared action keeps it in both menus.
    for (const char* id : {command_ids::ViewScreenModeStandard,
                           command_ids::ViewScreenModeFullWithMenuBar,
                           command_ids::ViewScreenModeFull}) {
        const QString commandId = QString::fromLatin1(id);
        if (QAction* action = commandRegistry()->action(commandId)) {
            const QString label = action->text();
            commandRegistry()->setLabelProvider(commandId, [label]() { return label; });
        }
    }
    connect(toolbox, &Toolbox::columnsChanged, this, [this](int) {
        if (PanelColumn* tools = panels_->toolsColumn()) {
            tools->refreshToolsWidth();
        }
        saveSession();
    });
    toolbox->setColumns(toolsColumns);
    toolsColumn->refreshToolsWidth();
    panels_->reapplyColumnStretch();
    toolsColumn->setVisible(!panels_->panelsHidden());

    rebuildToolShortcuts();

    auto brushActive = [this]() {
        return tools_ && isBrushTool(tools_->activeTool());
    };
    // With the Magnetic Lasso, `[` / `]` step the detection width instead.
    auto magneticActive = [this]() {
        return tools_ && tools_->activeTool() == ToolId::MagneticLasso;
    };
    connect(new QShortcut(QKeySequence(Qt::Key_BracketLeft), this), &QShortcut::activated, this,
            [this, brushActive, magneticActive]() {
                if (brushActive()) {
                    tools_->adjustBrushSize(-1);
                } else if (magneticActive()) {
                    if (SelectionOptions* selection =
                            tools_->optionsFor<SelectionOptions>(ToolId::MagneticLasso)) {
                        selection->setMagneticWidth(selection->magneticWidth() - 1);
                    }
                }
            });
    connect(new QShortcut(QKeySequence(Qt::Key_BracketRight), this), &QShortcut::activated, this,
            [this, brushActive, magneticActive]() {
                if (brushActive()) {
                    tools_->adjustBrushSize(+1);
                } else if (magneticActive()) {
                    if (SelectionOptions* selection =
                            tools_->optionsFor<SelectionOptions>(ToolId::MagneticLasso)) {
                        selection->setMagneticWidth(selection->magneticWidth() + 1);
                    }
                }
            });
    connect(new QShortcut(QKeySequence(Qt::SHIFT | Qt::Key_BracketLeft), this),
            &QShortcut::activated, this, [this, brushActive]() {
                if (brushActive()) {
                    tools_->adjustBrushHardness(-5);
                }
            });
    connect(new QShortcut(QKeySequence(Qt::SHIFT | Qt::Key_BracketRight), this),
            &QShortcut::activated, this, [this, brushActive]() {
                if (brushActive()) {
                    tools_->adjustBrushHardness(+5);
                }
            });

    // CS6's Airbrush key: flip the active tool's Airbrush option. The window
    // owns it because it acts on the active tool, and workspaceKeys() lists it
    // so the shortcut editor never assigns the same key to a command.
    connect(new QShortcut(QKeySequence(Qt::SHIFT | Qt::ALT | Qt::Key_P), this),
            &QShortcut::activated, this, [this]() {
                if (tools_) {
                    tools_->toggleAirbrush();
                }
            });

    // CS6's painting-mode quick keys: Dodge/Burn Range and Sponge Mode. The
    // controller ignores a key that does not apply to the active tool.
    for (const Qt::Key key : {Qt::Key_S, Qt::Key_M, Qt::Key_H, Qt::Key_D}) {
        connect(new QShortcut(QKeySequence(Qt::SHIFT | Qt::ALT | key), this),
                &QShortcut::activated, this, [this, key]() {
                    if (tools_) {
                        tools_->applyToneQuickKey(key);
                    }
                });
    }

    // CS6 foreground/background swap. `X` is unassigned in the tool letter
    // catalogue, so it does not collide with the tool-switch shortcuts.
    connect(new QShortcut(QKeySequence(Qt::Key_X), this), &QShortcut::activated, this,
            [this]() {
                if (toolbox_) {
                    toolbox_->swapForegroundBackground();
                }
            });

    // M43: `D` resets fg/bg to the active target's CS6 defaults (black/white
    // for layer pixels, white/black for a mask). `D` is also unassigned in the
    // tool letter catalogue (the engine tool catalogue, `pictura_script::tool_catalogue`).
    connect(new QShortcut(QKeySequence(Qt::Key_D), this), &QShortcut::activated, this,
            [this]() {
                if (toolbox_) {
                    toolbox_->resetForegroundBackground();
                }
            });

    // CS6's Delete / Backspace act from anywhere in the workspace: a pixel
    // selection clears, otherwise the selected layer(s) delete. Auto-repeat is
    // off so holding the key cannot delete layer after layer; the type
    // session's shortcut override, an open dialog, and a tool that consumes
    // the key all keep it.
    QShortcut* deleteShortcuts[2] = {nullptr, nullptr};
    int deleteShortcutCount = 0;
    for (const Qt::Key key : {Qt::Key_Delete, Qt::Key_Backspace}) {
        auto* shortcut = new QShortcut(QKeySequence(key), this);
        shortcut->setAutoRepeat(false);
        shortcut->setEnabled(!isTextInput(QApplication::focusWidget()));
        connect(shortcut, &QShortcut::activated, this, [this]() { handleDeleteKey(); });
        deleteShortcuts[deleteShortcutCount++] = shortcut;
    }
    // A focused editable field owns Delete / Backspace: while one has focus the
    // shortcuts are off, so the key reaches the field instead of being
    // swallowed by the shortcut map.
    deleteShortcutsFocusConn_ = connect(qApp, &QApplication::focusChanged, this,
            [deleteShortcuts](QWidget*, QWidget* now) {
                const bool field = isTextInput(now);
                for (QShortcut* shortcut : deleteShortcuts) {
                    if (shortcut) {
                        shortcut->setEnabled(!field);
                    }
                }
            });

    // A Space pan is a key-held override; if the key-up is missed because the
    // canvas loses focus or the window deactivates, it would otherwise stick and
    // route every later press to pan. End it and restore the tool on that edge.
    spacePanFocusConn_ = connect(qApp, &QApplication::focusChanged, this,
                                 [this](QWidget* now, QWidget* old) {
        ImageView* canvas = imageView();
        if (!canvas || !canvas->spacePanForTest()) {
            return;
        }
        const bool wasCanvas = old && (old == canvas || canvas->isAncestorOf(old));
        const bool isCanvas = now && (now == canvas || canvas->isAncestorOf(now));
        if (now == nullptr || (wasCanvas && !isCanvas)) {
            canvas->setSpacePan(false);
            canvas->setPanEnabled(tools_ && tools_->activeTool() == ToolId::Hand);
            if (tools_) {
                tools_->refreshCursor();
            }
        }
    });

    registerPanel(toolbox, Qt::LeftDockWidgetArea);

    optionsBar_ = new OptionsBar(tools_, this);
    optionsBar_->setObjectName(QStringLiteral("optionsBar"));
    addToolBar(optionsBar_);
    // The options bar spans the full window width, but its content sits inside
    // the central band's 3 px frame; inset it so no control touches the line.
    // The larger left inset leaves breathing room to the right of the toolbar's
    // drag grip.
    optionsBar_->setContentsMargins(9, 0, 3, 0);
    connect(optionsBar_, &OptionsBar::panelToggleRequested, this, &PicturaMainWindow::togglePanel);
    connect(optionsBar_, &OptionsBar::alignRequested, this,
            [this](int edge) {
                const MoveGroup* move = tools_ ? tools_->optionsFor<MoveGroup>(ToolId::Move) : nullptr;
                if (menu_) {
                    menu_->alignSelectedLayers(edge, true, move ? move->get().alignTo : 0);
                }
            });
    connect(optionsBar_, &OptionsBar::distributeRequested, this, [this](int edge) {
        if (menu_) {
            menu_->distributeSelectedLayers(edge);
        }
    });
    connect(tools_, &ToolController::moveOptionsChanged, this, [this]() {
        if (menu_) {
            menu_->updateAlignControls();
        }
    });
    connect(optionsBar_, &OptionsBar::refineEdgeRequested, this, [this]() {
        commandRegistry()->dispatch(
            QString::fromLatin1(command_ids::SelectRefineEdge));
    });
    // While text is being set the Character and Paragraph panels edit that
    // session. An options bar change reaches them too: they write back every
    // field they show.
    panels_->characterPanel()->setTextSession(
        [this]() { return tools_ ? tools_->textSession() : nullptr; });
    panels_->paragraphPanel()->setTextSession(
        [this]() { return tools_ ? tools_->textSession() : nullptr; });
    connect(tools_, &ToolController::textEditingChanged, panels_->characterPanel(),
            &CharacterPanel::refresh);
    connect(tools_, &ToolController::textEditingChanged, panels_->paragraphPanel(),
            &ParagraphPanel::refresh);
    connect(tools_, &ToolController::typeOptionsChanged, this, [this]() {
        TextSession* session = tools_->textSession();
        if (session && session->textActive()) {
            panels_->characterPanel()->refresh();
            panels_->paragraphPanel()->refresh();
        }
    });
    connect(optionsBar_, &OptionsBar::warpTextRequested, this, [this]() {
        commandRegistry()->dispatch(
            QString::fromLatin1(command_ids::TypeWarpText));
    });
    connect(optionsBar_, &OptionsBar::rulerStraightenRequested, this, [this]() {
        if (menu_) menu_->straightenRuler();
    });
    panels_->brushPanel()->setController(tools_);
    panels_->cloneSourcePanel()->setController(tools_);
    panels_->glyphsPanel()->setController(tools_);
    if (panels_->toolPresetsPanel()) {
        panels_->toolPresetsPanel()->setController(tools_);
    }

    // The Info panel follows the Ruler tool: its A/L block and W/H readout come
    // from the ruler, and a new measuring line re-reads them.
    connect(tools_, &ToolController::rulerChanged, this, [this]() {
        if (panels_->infoPanel()) {
            if (imageView()) panels_->infoPanel()->setRulerCoordinates(imageView()->rulerOrigin(), imageView()->rulerUnit());
            panels_->infoPanel()->refresh();
        }
    });

    connect(tools_, &ToolController::activeToolChanged, this, [this](ToolId id) {
        if (auto* panel = panels_->layersPanel()) {
            panel->clearOpacityShortcut();
        }
        if (optionsBar_) {
            optionsBar_->showTool(id);
        }
        if (panels_->infoPanel()) {
            panels_->infoPanel()->setRulerMode(id == ToolId::Ruler);
        }
        updateToolHint();
        syncBrushTipPreview();
    });
    connect(tools_, &ToolController::brushTipChanged, this,
            [this]() { syncBrushTipPreview(); });
    // A modal tool session (Crop) recorded or discarded a step; re-evaluate the
    // Undo/Redo commands so Ctrl+Z reaches the session.
    connect(tools_, &ToolController::toolSessionChanged, this, [this]() {
        if (commandRegistry()) {
            commandRegistry()->refresh();
        }
    });
    // Crop options (mode, grid, ratio, delete pixels) persist across restarts.
    connect(tools_, &ToolController::cropOptionsChanged, this, [this]() { saveSession(); });
    // Every tool's options persist too: saved on close and quit, and on these
    // changes so a crash keeps the last tool switch or bar edit.
    for (auto signal : {&ToolController::moveOptionsChanged, &ToolController::typeOptionsChanged,
                        &ToolController::shapeOptionsChanged}) {
        connect(tools_, signal, this, [this]() { saveSession(); });
    }
    connect(tools_, &ToolController::activeToolChanged, this, [this]() { saveSession(); });
    // The Note tool opens a note in the Notes panel; the panel's
    // previous/next/delete hand the current note back to the controller.
    connect(tools_, &ToolController::layerCreated, this, &PicturaMainWindow::selectLayerPath);
    // The Move tool's Auto-Select hit asks for an existing layer's row.
    connect(tools_, &ToolController::layerSelectionRequested, this,
            &PicturaMainWindow::selectLayerPath);
    connect(tools_, &ToolController::noteActivated, this, [this](int index) {
        if (!panels_->notesPanel()) {
            return;
        }
        panels_->notesPanel()->showNote(index);
        if (index >= 0 && panels_->panelColumn()) {
            panels_->panelColumn()->showPanel(QStringLiteral("notesPanel"), true);
        }
    });
    if (panels_->notesPanel()) {
        connect(panels_->notesPanel(), &NotesPanel::noteRequested, tools_,
                &ToolController::setCurrentNote);
    }
    connect(tools_, &ToolController::foregroundSampled, this, [this](const QColor& color) {
        panels_->colorState()->setForeground(color);
    });

    // Canvas context-menu requests: the shared widget menus the controller
    // cannot reach (the toolbox Zoom preset menu, the options-bar Brush Preset
    // picker), isolation mode, and activation of the clicked document so its
    // menu acts on the right document.
    connect(tools_, &ToolController::zoomMenuRequested, this, [this](const QPoint& pos) {
        if (toolbox_ && toolbox_->zoomMenu()) {
            toolbox_->zoomMenu()->popup(pos);
        }
    });
    connect(tools_, &ToolController::toolZoomed, this,
            [this](double factor, bool allWindows, bool resizeToFit) {
                documents_->onToolZoomed(factor, allWindows, resizeToFit);
            });
    connect(tools_, &ToolController::brushPickerRequested, this, [this](const QPoint& pos) {
        if (optionsBar_) {
            optionsBar_->brushPicker()->popUpAt(pos);
        }
    });
    connect(tools_, &ToolController::isolationModeRequested, this, [this]() {
        if (panels_->layersPanel()) {
            panels_->layersPanel()->enterIsolation();
        }
    });
    tools_->setCanvasActivationHandler(
        [this](ImageView* canvas) {
            documents_->activateDocWindow(documents_->windowOfCanvas(canvas));
        });

    if (optionsBar_) {
        optionsBar_->showTool(tools_->activeTool());
    }
    if (panels_->infoPanel()) {
        panels_->infoPanel()->setRulerMode(tools_->activeTool() == ToolId::Ruler);
    }
}

void PicturaMainWindow::buildStatusBar()
{
    if (!qobject_cast<NotificationStatusBar*>(statusBar())) {
        setStatusBar(new NotificationStatusBar(this));
    }
    QStatusBar* bar = statusBar();
    bar->setSizeGripEnabled(false);
    bar->setContentsMargins(6, 0, 0, 0);
    bar->setFixedHeight(Theme::scaled(28));

    NumericFieldConfig zoomConfig;
    zoomConfig.minimum = 1;
    zoomConfig.maximum = 3200;
    zoomConfig.decimals = 0;
    zoomConfig.suffix = QStringLiteral("%");
    zoomConfig.namePrefix = QStringLiteral("statusZoom");
    zoomConfig.objectName = QStringLiteral("statusZoomField");
    zoomField_ = new NumericField(QString(), zoomConfig, bar);
    // The value hugs its content; leftover status-bar width belongs to the
    // empty message area, keeping the readouts left-aligned.
    zoomField_->setSizePolicy(QSizePolicy::Fixed, QSizePolicy::Preferred);
    sizeLabel_ = new QLabel(QStringLiteral("—"), bar);
    sizeLabel_->setObjectName(QStringLiteral("statusSizeLabel"));
    sizeLabel_->setTextFormat(Qt::PlainText);

    // The options button sits with the resolution readout it controls, between
    // Resolution and GPU. Built in visual order so the widget tree matches.
    statusOptionsButton_ = new QToolButton(bar);
    statusOptionsButton_->setObjectName(QStringLiteral("statusOptionsButton"));
    statusOptionsButton_->setArrowType(Qt::DownArrow);
    statusOptionsButton_->setPopupMode(QToolButton::InstantPopup);
    auto* optionsMenu = new QMenu(statusOptionsButton_);
    auto* group = new QActionGroup(optionsMenu);
    for (const QString& key : {QStringLiteral("sizes"), QStringLiteral("dimensions")}) {
        QAction* option = optionsMenu->addAction(
            key == QStringLiteral("sizes") ? QStringLiteral("Document Sizes")
                                           : QStringLiteral("Document Dimensions"));
        option->setCheckable(true);
        option->setData(key);
        option->setChecked(key == QStringLiteral("sizes"));
        group->addAction(option);
        connect(option, &QAction::triggered, this, [this, key]() {
            statusReadout_ = key;
            updateStatus();
        });
    }
    if (edition_is_extended()) {
        QAction* option = optionsMenu->addAction(tr("Measurement Scale"));
        option->setCheckable(true);
        group->addAction(option);
        connect(option, &QAction::triggered, this, [this]() {
            statusReadout_ = QStringLiteral("measurementScale");
            updateStatus();
        });
    }
    // Scratch Sizes and Efficiency (#636) read the governor, so a timer keeps
    // them current while one is shown.
    statusReadoutTimer_ = new QTimer(this);
    statusReadoutTimer_->setInterval(2000);
    connect(statusReadoutTimer_, &QTimer::timeout, this, [this]() { updateStatus(); });
    const std::pair<QString, QString> memoryReadouts[] = {
        {QStringLiteral("scratchSizes"), tr("Scratch Sizes")},
        {QStringLiteral("efficiency"), tr("Efficiency")},
    };
    for (const auto& [key, label] : memoryReadouts) {
        QAction* option = optionsMenu->addAction(label);
        option->setCheckable(true);
        option->setData(key);
        group->addAction(option);
    }
    connect(group, &QActionGroup::triggered, this, [this](QAction* option) {
        const QString key = option->data().toString();
        if (!statusReadoutIsLive(key)) {
            statusReadoutTimer_->stop();
            return;
        }
        statusReadout_ = key;
        statusReadoutTimer_->start();
        updateStatus();
    });
    buildHdrStatus(bar, optionsMenu, group);
    statusOptionsButton_->setMenu(optionsMenu);

    backendLabel_ = new QLabel(QStringLiteral("—"), bar);
    backendLabel_->setObjectName(QStringLiteral("statusBackendLabel"));
    actionLabel_ = new QLabel(bar);
    actionLabel_->setObjectName(QStringLiteral("statusActionLabel"));
    actionLabel_->hide();
    hintBar_ = new ToolHintBar(bar);
    // A ~28px status strip: cap the readouts and hint so the bar trims 4px
    // without clipping. (The theme already dropped the bar's top border.)
    const int kStatusItemHeight = Theme::scaled(22);
    zoomField_->setFixedHeight(kStatusItemHeight);
    sizeLabel_->setFixedHeight(kStatusItemHeight);
    statusOptionsButton_->setFixedHeight(kStatusItemHeight);
    backendLabel_->setFixedHeight(kStatusItemHeight);
    actionLabel_->setFixedHeight(kStatusItemHeight);
    hintBar_->setFixedHeight(kStatusItemHeight);

    // A 1px themable rule between the footer groups. Mouse-transparent so it
    // never intercepts the status bar's own clicks.
    const auto makeSeparator = [bar]() {
        auto* separator = new QFrame(bar);
        separator->setObjectName(QStringLiteral("statusSeparator"));
        separator->setFrameShape(QFrame::VLine);
        separator->setFrameShadow(QFrame::Sunken);
        separator->setFixedWidth(1);
        separator->setFixedHeight(Theme::scaled(16));
        separator->setAttribute(Qt::WA_TransparentForMouseEvents);
        return separator;
    };

    // Left to right: Zoom | Resolution [options] | GPU | Hints.
    bar->addWidget(zoomField_);
    bar->addWidget(makeSeparator());
    bar->addWidget(sizeLabel_);
    bar->addWidget(hdrExposureSlider_);
    backgroundSaveProgress_ = new QProgressBar(bar);
    backgroundSaveProgress_->setObjectName(QStringLiteral("backgroundSaveProgress"));
    backgroundSaveProgress_->setRange(0, 100);
    backgroundSaveProgress_->setFormat(tr("Saving %p%"));
    backgroundSaveProgress_->setAccessibleName(tr("Save progress"));
    backgroundSaveProgress_->setMaximumWidth(180);
    backgroundSaveProgress_->hide();
    bar->addWidget(backgroundSaveProgress_);
    bar->addWidget(statusOptionsButton_);
    bar->addWidget(makeSeparator());
    bar->addWidget(backendLabel_);
    // The action readout ("Skew in progress") travels with its rule.
    actionSeparator_ = makeSeparator();
    bar->addWidget(actionSeparator_);
    bar->addWidget(actionLabel_);
    // The rule before the hint strip travels with the strip's visibility.
    hintSeparator_ = makeSeparator();
    bar->addWidget(hintSeparator_);
    bar->addWidget(hintBar_);
    // Right-aligned: the Help > Search keycap, following its live binding.
    buildSearchHint(bar);
    searchHint_->setFixedHeight(kStatusItemHeight);
    // One owner for the footer font and the scaled item heights.
    applyStatusBarScale();

    connect(zoomField_, &NumericField::valueCommitted, this, [this](double percent) {
        if (ImageView* canvas = imageView()) {
            canvas->setZoom(percent / 100.0,
                            QPointF(canvas->width() / 2.0, canvas->height() / 2.0));
        }
    });
    if (tools_) {
        connect(tools_, &ToolController::transformChanged, this, [this]() {
            updateStatus();
            // A border transform hides the committed ants; restore them when
            // its session ends (commit or cancel).
            if (!tools_->transformSessionActive()) {
                refreshSelectionOverlay();
            }
        });
        installTransformLock();
    }
    // Qt hides the normal readouts while a temporary message shows, but a later
    // refresh re-shows them; the bar's message signal restores them too, so the
    // footer has one visibility owner.
    connect(bar, &QStatusBar::messageChanged, this,
            [this](const QString&) { updateStatus(); });
    updateToolHint();
}

void PicturaMainWindow::updateToolHint()
{
    if (!hintBar_) {
        return;
    }
    QString fallback = tools_ ? QCoreApplication::translate("PicturaTools", toolInfo(tools_->activeTool()).hint)
                              : tr("Ready");
    const QColor foreground = foregroundColor();
    if (foreground.isValid()) {
        fallback += tr("  ·  Foreground %1").arg(foreground.name());
    }
    QList<ToolHint> hints;
    if (tools_) {
        hints = toolHintEntries(tools_->activeTool());
        for (ToolHint& hint : hints) {
            hint.text = QCoreApplication::translate("PicturaTools", hint.text.toUtf8().constData());
            if (!hint.commandId || !commandRegistry()) {
                continue;
            }
            if (QAction* action = commandRegistry()->action(QString::fromLatin1(hint.commandId))) {
                const QString key = action->shortcut().toString(QKeySequence::NativeText);
                if (!key.isEmpty()) {
                    hint.key = key;
                }
            }
        }
    }
    hintBar_->setHints(hints, fallback);
    if (panels_->infoPanel()) {
        QStringList hintTexts;
        for (const ToolHint& hint : hints) {
            hintTexts << QStringLiteral("%1 %2").arg(hint.key, hint.text);
        }
        const QString name = tools_ ? QCoreApplication::translate("PicturaTools", toolInfo(tools_->activeTool()).label)
                                    : QString();
        panels_->infoPanel()->setToolInfo(name, hintTexts);
    }
}

void PicturaMainWindow::installWorkspaceOpenGesture()
{
    // The blank pane is the tab widget's internal stacked widget, so it, the
    // tab widget, and the document area holding them are filtered.
    documents_->documentArea()->installEventFilter(this);
    documents_->tabs()->installEventFilter(this);
    if (auto* workspace = documents_->tabs()->findChild<QStackedWidget*>()) {
        workspace->installEventFilter(this);
    }
}

bool PicturaMainWindow::eventFilter(QObject* watched, QEvent* event)
{
    if (auto* canvas = qobject_cast<ImageView*>(watched);
        canvas && documents_ && panels_ && tools_ && canvas == imageView()
        && tools_->activeTool() == ToolId::Move) {
        if (auto* panel = panels_->layersPanel();
            panel && panel->filterOpacityShortcut(canvas, event)) {
            return true;
        }
    }
    // A press on a canvas activates its window, before the tool sees it.
    if (event->type() == QEvent::MouseButtonPress) {
        if (DocWindow* window = windowOfCanvas(qobject_cast<ImageView*>(watched));
            window && window != documents_->activeWindow()) {
            documents_->activateDocWindow(window);
        }
    }
    // The empty workspace (no documents open) accepts a left double-click as
    // "Open": the tab widget and its blank stacked page are the only widgets
    // there.
    if (event->type() == QEvent::MouseButtonDblClick && workspaceOpenArmed()) {
        auto* mouse = static_cast<QMouseEvent*>(event);
        if (mouse->button() == Qt::LeftButton) {
            showOpenDialog();
            return true;
        }
    }
    return QMainWindow::eventFilter(watched, event);
}

} // namespace pictura
