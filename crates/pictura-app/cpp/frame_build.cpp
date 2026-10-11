#include "frame_includes.h"

#include "fonts.h"
#include "panels/numeric_field.h"

#include <QtCore/QEvent>
#include <QtGui/QFont>
#include <QtGui/QMouseEvent>
#include <QtWidgets/QApplication>
#include <QtWidgets/QFrame>
#include <QtWidgets/QStackedWidget>

namespace pictura {

void PicturaMainWindow::buildPanels()
{
    colorState_ = new ColorState(this);

    layersPanel_ = new LayersPanel(this);
    layersPanel_->setObjectName(QStringLiteral("layersPanel"));

    historyPanel_ = new HistoryPanel(this);
    historyPanel_->setObjectName(QStringLiteral("historyPanel"));

    navigatorPanel_ = new NavigatorPanel(this);
    navigatorPanel_->setObjectName(QStringLiteral("navigatorPanel"));

    colorPanel_ = new ColorPanel(colorState_, this);
    colorPanel_->setObjectName(QStringLiteral("colorPanel"));

    swatchesPanel_ = new SwatchesPanel(colorState_, this);
    swatchesPanel_->setObjectName(QStringLiteral("swatchesPanel"));

    infoPanel_ = new InfoPanel(this);
    infoPanel_->setObjectName(QStringLiteral("infoPanel"));

    histogramPanel_ = new HistogramPanel(this);
    histogramPanel_->setObjectName(QStringLiteral("histogramPanel"));

    gradientsPanel_ = new PlaceholderPanel(QStringLiteral("Gradients"), QString(), this);
    gradientsPanel_->setObjectName(QStringLiteral("gradientsPanel"));

    patternsPanel_ = new PlaceholderPanel(QStringLiteral("Patterns"), QString(), this);
    patternsPanel_->setObjectName(QStringLiteral("patternsPanel"));

    notesPanel_ = new NotesPanel(this);
    notesPanel_->setObjectName(QStringLiteral("notesPanel"));

    propertiesPanel_ = new PropertiesPanel(this);
    propertiesPanel_->setObjectName(QStringLiteral("propertiesPanel"));
    connect(propertiesPanel_, &PropertiesPanel::modeRequested, this,
            &PicturaMainWindow::convertImageMode);
    connect(propertiesPanel_, &PropertiesPanel::depthRequested, this,
            &PicturaMainWindow::convertImageDepth);

    adjustmentsPanel_ = new PlaceholderPanel(QStringLiteral("Adjustments"), QString(), this);
    adjustmentsPanel_->setObjectName(QStringLiteral("adjustmentsPanel"));

    channelsPanel_ = new ChannelsPanel(this);
    channelsPanel_->setObjectName(QStringLiteral("channelsPanel"));
    // Channel visibility follows the active document; the panel resets it on a
    // document switch, so applying it to every canvas keeps them in step.
    connect(channelsPanel_, &ChannelsPanel::channelMaskChanged, this, [this](int mask) {
        for (int i = 0; i < documentCount(); ++i) {
            if (ImageView* canvas = canvasAt(i)) {
                canvas->setChannelMask(mask);
            }
        }
    });

    pathsPanel_ = new PathsPanel(this);
    pathsPanel_->setObjectName(QStringLiteral("pathsPanel"));

    actionsPanel_ = new PlaceholderPanel(QStringLiteral("Actions"), QString(), this);
    actionsPanel_->setObjectName(QStringLiteral("actionsPanel"));

    // `Styles` takes the former Gradients/Patterns tab slot in the default set.
    stylesPanel_ = new PlaceholderPanel(QStringLiteral("Styles"), QString(), this);
    stylesPanel_->setObjectName(QStringLiteral("stylesPanel"));

    brushPanel_ = new BrushPanel(this);
    brushPanel_->setObjectName(QStringLiteral("brushPanel"));

    cloneSourcePanel_ = new CloneSourcePanel(this);
    cloneSourcePanel_->setObjectName(QStringLiteral("cloneSourcePanel"));

    characterPanel_ = new CharacterPanel(this);
    characterPanel_->setObjectName(QStringLiteral("characterPanel"));
    paragraphPanel_ = new ParagraphPanel(this);
    paragraphPanel_->setObjectName(QStringLiteral("paragraphPanel"));

    glyphsPanel_ = new GlyphsPanel(this);
    glyphsPanel_->setObjectName(QStringLiteral("glyphsPanel"));

    paragraphStylesPanel_ = new ParagraphStylesPanel(this);
    paragraphStylesPanel_->setObjectName(QStringLiteral("paragraphStylesPanel"));

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
    registerPanel(actionsPanel_, Qt::RightDockWidgetArea);
    registerPanel(brushPanel_, Qt::RightDockWidgetArea);
    registerPanel(cloneSourcePanel_, Qt::RightDockWidgetArea);
    registerPanel(characterPanel_, Qt::RightDockWidgetArea);
    registerPanel(paragraphPanel_, Qt::RightDockWidgetArea);
    registerPanel(glyphsPanel_, Qt::RightDockWidgetArea);
    registerPanel(paragraphStylesPanel_, Qt::RightDockWidgetArea);

    // CS6 Essentials groups and order. The tab text is the panel title and the
    // tab icon reuses the panel's `window.panels.<name>` asset.
    auto addPanel = [](PanelGroup* group, QWidget* panel, const QString& title,
                       const QString& iconId) {
        group->addPanel(panel, title, pictura::icon(iconId));
    };

    auto* colorGroup = new PanelGroup(this);
    addPanel(colorGroup, colorPanel_, tr("Color"), QString::fromLatin1(command_ids::WindowPanelsColor));
    addPanel(colorGroup, swatchesPanel_, tr("Swatches"),
             QString::fromLatin1(command_ids::WindowPanelsSwatches));
    addPanel(colorGroup, stylesPanel_, tr("Styles"), QStringLiteral("window.panels.styles"));
    panelColumn_->addGroup(colorGroup);

    // Properties folds into the Adjustments group; its tab starts hidden.
    auto* adjustmentsGroup = new PanelGroup(this);
    addPanel(adjustmentsGroup, adjustmentsPanel_, tr("Adjustments"),
             QString::fromLatin1(command_ids::WindowPanelsAdjustments));
    addPanel(adjustmentsGroup, propertiesPanel_, tr("Properties"),
             QString::fromLatin1(command_ids::WindowPanelsProperties));
    panelColumn_->addGroup(adjustmentsGroup);

    auto* layersGroup = new PanelGroup(this);
    addPanel(layersGroup, layersPanel_, tr("Layers"),
             QString::fromLatin1(command_ids::WindowPanelsLayers));
    addPanel(layersGroup, channelsPanel_, tr("Channels"),
             QString::fromLatin1(command_ids::WindowPanelsChannels));
    addPanel(layersGroup, pathsPanel_, tr("Paths"),
             QString::fromLatin1(command_ids::WindowPanelsPaths));
    panelColumn_->addGroup(layersGroup);

    auto* navigatorGroup = new PanelGroup(this);
    addPanel(navigatorGroup, navigatorPanel_, tr("Navigator"),
             QString::fromLatin1(command_ids::WindowPanelsNavigator));
    addPanel(navigatorGroup, histogramPanel_, tr("Histogram"),
             QString::fromLatin1(command_ids::WindowPanelsHistogram));
    addPanel(navigatorGroup, infoPanel_, tr("Info"),
             QString::fromLatin1(command_ids::WindowPanelsInfo));
    panelColumn_->addGroup(navigatorGroup);

    // Iconic single-panel groups.
    auto* historyGroup = new PanelGroup(this);
    addPanel(historyGroup, historyPanel_, tr("History"),
             QString::fromLatin1(command_ids::WindowPanelsHistory));
    panelColumn_->addGroup(historyGroup);

    auto* actionsGroup = new PanelGroup(this);
    addPanel(actionsGroup, actionsPanel_, tr("Actions"),
             QString::fromLatin1(command_ids::WindowPanelsActions));
    panelColumn_->addGroup(actionsGroup);

    // Kept registered and Window-menu reachable, but out of the default groups.
    auto* overflowGroup = new PanelGroup(this);
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
    panelColumn_->addGroup(overflowGroup);

    // CS6 docks Character and Paragraph as one group, opened from the Window
    // or Type menu or the Type options bar's panel toggle; Glyphs (post-CS6)
    // joins them.
    auto* typeGroup = new PanelGroup(this);
    addPanel(typeGroup, characterPanel_, tr("Character"),
             QString::fromLatin1(command_ids::WindowPanelsCharacter));
    addPanel(typeGroup, paragraphPanel_, tr("Paragraph"),
             QString::fromLatin1(command_ids::WindowPanelsParagraph));
    addPanel(typeGroup, glyphsPanel_, tr("Glyphs"),
             QString::fromLatin1(command_ids::WindowPanelsGlyphs));
    addPanel(typeGroup, paragraphStylesPanel_, tr("Paragraph Styles"),
             QString::fromLatin1(command_ids::WindowPanelsParagraph));
    panelColumn_->addGroup(typeGroup);

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

    connect(layersPanel_, &LayersPanel::selectionChanged, this, [this] {
        if (propertiesPanel_) {
            propertiesPanel_->refresh();
        }
        updateAlignControls();
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
    });
}

void PicturaMainWindow::togglePanel(const QString& objectName)
{
    PanelColumn* owner = columnForPanel(objectName);
    if (!owner) {
        owner = panelColumn_;
    }
    if (owner) {
        owner->showPanel(objectName, !owner->isPanelVisible(objectName));
    }
}

void PicturaMainWindow::buildTools(int toolsColumns, bool useShiftKeyForToolSwitch)
{
    tools_ = new ToolController(this);
    tools_->setViewProvider([this]() { return activeView(); });
    pathsPanel_->setToolContext(tools_);

    // A tool commit (marquee release, wand click, lasso close) reaches the
    // picture view without a `changed` emission, so refresh the overlay directly.
    connect(tools_, &ToolController::selectionCommitted, this,
            &PicturaMainWindow::refreshSelectionOverlay);
    // A live move-selection drag also stays silent, so its translated contour
    // is refreshed from the dedicated preview signal.
    connect(tools_, &ToolController::selectionPreviewChanged, this,
            &PicturaMainWindow::refreshSelectionOverlay);
    // A locked pixel edit is refused silently at the bridge; report it so the
    // user learns why nothing was painted.
    connect(tools_, &ToolController::pixelEditRefused, this, [this](const QString& message) {
        statusBar()->showMessage(message, 4000);
    });

    if (colorState_) {
        tools_->setForeground(colorState_->foreground());
        tools_->setBackground(colorState_->background());
        connect(colorState_, &ColorState::foregroundChanged, tools_,
                &ToolController::setForeground);
        connect(colorState_, &ColorState::backgroundChanged, tools_,
                &ToolController::setBackground);
    }

    auto* toolbox = new Toolbox(tools_, colorState_, this);
    toolbox_ = toolbox;
    toolbox->setShiftKeyForToolSwitch(useShiftKeyForToolSwitch);

    // D1: the Tools panel is a tabless, atomic `PanelColumn` hosting the tool
    // content as one plain child in the central splitter, default left at index
    // 0. Its header toggle reflows the tool grid one<->two columns (D2), not the
    // normal/iconic rail mode.
    toolsColumn_ = new PanelColumn(this);
    toolsColumn_->setObjectName(QStringLiteral("toolsColumn"));
    wirePanelColumn(toolsColumn_);
    centerSplitter_->insertWidget(0, toolsColumn_);
    toolsColumn_->setToolsContent(
        toolbox, [toolbox]() { return toolbox->columns(); },
        [this]() {
            if (toolbox_) {
                toolbox_->setColumns(toolbox_->columns() == 1 ? 2 : 1);
            }
        });
    // The screen-mode button opens a menu of the same three commands `F` and
    // `Shift+F` cycle, sharing the registry actions so checks and handlers agree.
    toolbox->setScreenModeActions({
        registry_->action(QString::fromLatin1(command_ids::ViewScreenModeStandard)),
        registry_->action(QString::fromLatin1(command_ids::ViewScreenModeFullWithMenuBar)),
        registry_->action(QString::fromLatin1(command_ids::ViewScreenModeFull)),
    });
    toolbox->setActiveScreenMode(static_cast<int>(screenMode_));
    // CommandRegistry::refresh() rewrites every action's text from its spec,
    // which would drop the toolbox's display-only `F` hint. Pin the hinted text
    // with label providers so the shared action keeps it in both menus.
    for (const char* id : {command_ids::ViewScreenModeStandard,
                           command_ids::ViewScreenModeFullWithMenuBar,
                           command_ids::ViewScreenModeFull}) {
        const QString commandId = QString::fromLatin1(id);
        if (QAction* action = registry_->action(commandId)) {
            const QString label = action->text();
            registry_->setLabelProvider(commandId, [label]() { return label; });
        }
    }
    connect(toolbox, &Toolbox::columnsChanged, this, [this](int) {
        if (toolsColumn_) {
            toolsColumn_->refreshToolsWidth();
        }
        saveSession();
    });
    toolbox->setColumns(toolsColumns);
    toolsColumn_->refreshToolsWidth();
    reapplyColumnStretch();
    toolsColumn_->setVisible(!panelsHidden_);

    // One plain and one Shift shortcut per distinct slot letter; the toolbox
    // resolves the letter to its group and honours `Use Shift Key For Tool
    // Switch`. refreshSlot() no longer owns any letter.
    for (const QChar key : toolShortcutKeys()) {
        connect(new QShortcut(QKeySequence(QString(key)), this), &QShortcut::activated, this,
                [this, key]() {
                    if (toolbox_) {
                        toolbox_->handleToolKey(key, false);
                    }
                });
        connect(new QShortcut(QKeySequence(Qt::SHIFT | Qt::Key(key.toUpper().unicode())), this),
                &QShortcut::activated, this, [this, key]() {
                    if (toolbox_) {
                        toolbox_->handleToolKey(key, true);
                    }
                });
    }

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
                    tools_->setMagneticWidth(tools_->magneticWidth() - 1);
                }
            });
    connect(new QShortcut(QKeySequence(Qt::Key_BracketRight), this), &QShortcut::activated, this,
            [this, brushActive, magneticActive]() {
                if (brushActive()) {
                    tools_->adjustBrushSize(+1);
                } else if (magneticActive()) {
                    tools_->setMagneticWidth(tools_->magneticWidth() + 1);
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

    // CS6 foreground/background swap. `X` is unassigned in the tool letter
    // catalogue, so it does not collide with the tool-switch shortcuts.
    connect(new QShortcut(QKeySequence(Qt::Key_X), this), &QShortcut::activated, this,
            [this]() {
                if (toolbox_) {
                    toolbox_->swapForegroundBackground();
                }
            });

    // M43: `D` resets fg/bg to the CS6 defaults (black/white). `D` is also
    // unassigned in the tool letter catalogue (see tools.cpp `kToolTable`).
    connect(new QShortcut(QKeySequence(Qt::Key_D), this), &QShortcut::activated, this,
            [this]() {
                if (toolbox_) {
                    toolbox_->resetForegroundBackground();
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
            [this](int edge) { alignSelectedLayers(edge, true, tools_ ? tools_->moveAlignTo() : 0); });
    connect(optionsBar_, &OptionsBar::distributeRequested, this,
            &PicturaMainWindow::distributeSelectedLayers);
    connect(tools_, &ToolController::moveOptionsChanged, this,
            &PicturaMainWindow::updateAlignControls);
    connect(optionsBar_, &OptionsBar::refineEdgeRequested, this, [this]() {
        registry_->dispatch(
            commandIdForPath({QStringLiteral("Select"), QStringLiteral("Refine Edge…")}));
    });
    brushPanel_->setController(tools_);
    cloneSourcePanel_->setController(tools_);
    glyphsPanel_->setController(tools_);

    // The Info panel follows the Ruler tool: its A/L block and W/H readout come
    // from the ruler, and a new measuring line re-reads them.
    connect(tools_, &ToolController::rulerChanged, this, [this]() {
        if (infoPanel_) {
            infoPanel_->refresh();
        }
    });

    connect(tools_, &ToolController::activeToolChanged, this, [this](ToolId id) {
        if (optionsBar_) {
            optionsBar_->showTool(id);
        }
        if (infoPanel_) {
            infoPanel_->setRulerMode(id == ToolId::Ruler);
        }
        updateToolHint();
    });
    // A modal tool session (Crop) recorded or discarded a step; re-evaluate the
    // Undo/Redo commands so Ctrl+Z reaches the session.
    connect(tools_, &ToolController::toolSessionChanged, this, [this]() {
        if (registry_) {
            registry_->refresh();
        }
    });
    // Crop options (mode, grid, ratio, delete pixels) persist across restarts.
    connect(tools_, &ToolController::cropOptionsChanged, this, [this]() { saveSession(); });
    // The Note tool opens a note in the Notes panel; the panel's
    // previous/next/delete hand the current note back to the controller.
    connect(tools_, &ToolController::layerCreated, this, &PicturaMainWindow::selectLayerPath);
    connect(tools_, &ToolController::noteActivated, this, [this](int index) {
        if (!notesPanel_) {
            return;
        }
        notesPanel_->showNote(index);
        if (index >= 0 && panelColumn_) {
            panelColumn_->showPanel(QStringLiteral("notesPanel"), true);
        }
    });
    if (notesPanel_) {
        connect(notesPanel_, &NotesPanel::noteRequested, tools_, &ToolController::setCurrentNote);
    }
    connect(tools_, &ToolController::foregroundSampled, this, [this](const QColor& color) {
        foreground_ = color;
        if (colorState_) {
            colorState_->setForeground(color);
        }
        updateToolHint();
    });

    if (optionsBar_) {
        optionsBar_->showTool(tools_->activeTool());
    }
    if (infoPanel_) {
        infoPanel_->setRulerMode(tools_->activeTool() == ToolId::Ruler);
    }
}

void PicturaMainWindow::buildDocumentTabs()
{
    // The document tab strip is flat chrome (no base line above the tabs) and
    // carries the shared tab font on the bar itself, so the label weight
    // does not depend on the QSS subcontrol rule and QTabBar's elision metrics
    // match the painted label.
    tabs_->tabBar()->setDrawBase(false);
    applyTabBarFont(tabs_->tabBar());
}

void PicturaMainWindow::buildStatusBar()
{
    QStatusBar* bar = statusBar();
    bar->setSizeGripEnabled(false);
    bar->setContentsMargins(6, 0, 0, 0);
    bar->setFixedHeight(28);

    // Footer text runs 2px under the app default, matching the tab chrome. A
    // point-sized default font (pixelSize() < 0) falls back to 12px at 96 DPI.
    int baseFontPx = QApplication::font().pixelSize();
    if (baseFontPx <= 0) {
        baseFontPx = 12;
    }
    QFont footerFont = bar->font();
    footerFont.setPixelSize(baseFontPx - 2);
    bar->setFont(footerFont);

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

    // The options button sits with the resolution readout it controls, between
    // Resolution and GPU. Built in visual order so the widget tree matches.
    statusOptionsButton_ = new QToolButton(bar);
    statusOptionsButton_->setObjectName(QStringLiteral("statusOptionsButton"));
    statusOptionsButton_->setArrowType(Qt::DownArrow);
    statusOptionsButton_->setPopupMode(QToolButton::InstantPopup);
    // Flat on the footer surface: the default button chrome reads as a raised
    // surface, so drop the background and border.
    statusOptionsButton_->setStyleSheet(
        QStringLiteral("QToolButton { background: transparent; border: 0; }"));
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
    statusOptionsButton_->setMenu(optionsMenu);

    backendLabel_ = new QLabel(QStringLiteral("—"), bar);
    backendLabel_->setObjectName(QStringLiteral("statusBackendLabel"));
    hintBar_ = new ToolHintBar(bar);
    // A ~28px status strip: cap the readouts and hint so the bar trims 4px
    // without clipping. (The theme already dropped the bar's top border.)
    constexpr int kStatusItemHeight = 22;
    zoomField_->setFixedHeight(kStatusItemHeight);
    sizeLabel_->setFixedHeight(kStatusItemHeight);
    statusOptionsButton_->setFixedHeight(kStatusItemHeight);
    backendLabel_->setFixedHeight(kStatusItemHeight);
    hintBar_->setFixedHeight(kStatusItemHeight);

    // A 1px themable rule between the footer groups. Mouse-transparent so it
    // never intercepts the status bar's own clicks.
    const auto makeSeparator = [bar]() {
        auto* separator = new QFrame(bar);
        separator->setObjectName(QStringLiteral("statusSeparator"));
        separator->setFrameShape(QFrame::VLine);
        separator->setFrameShadow(QFrame::Sunken);
        separator->setFixedWidth(1);
        separator->setFixedHeight(16);
        separator->setAttribute(Qt::WA_TransparentForMouseEvents);
        return separator;
    };

    // Left to right: Zoom | Resolution [options] | GPU | Hints.
    bar->addWidget(zoomField_);
    bar->addWidget(makeSeparator());
    bar->addWidget(sizeLabel_);
    bar->addWidget(statusOptionsButton_);
    bar->addWidget(makeSeparator());
    bar->addWidget(backendLabel_);
    // The rule before the hint strip travels with the strip's visibility.
    hintSeparator_ = makeSeparator();
    bar->addWidget(hintSeparator_);
    bar->addWidget(hintBar_);

    connect(zoomField_, &NumericField::valueCommitted, this, [this](double percent) {
        if (ImageView* canvas = imageView()) {
            canvas->setZoom(percent / 100.0,
                            QPointF(canvas->width() / 2.0, canvas->height() / 2.0));
        }
    });
    updateToolHint();
}

void PicturaMainWindow::updateToolHint()
{
    if (!hintBar_) {
        return;
    }
    QString fallback = tools_ ? QString::fromLatin1(toolInfo(tools_->activeTool()).hint)
                              : QStringLiteral("Ready");
    if (foreground_.isValid()) {
        fallback += QStringLiteral("  ·  Foreground %1").arg(foreground_.name());
    }
    QList<ToolHint> hints;
    if (tools_) {
        hints = toolHintEntries(tools_->activeTool());
        for (ToolHint& hint : hints) {
            if (!hint.commandId || !registry_) {
                continue;
            }
            if (QAction* action = registry_->action(QString::fromLatin1(hint.commandId))) {
                const QString key = action->shortcut().toString(QKeySequence::NativeText);
                if (!key.isEmpty()) {
                    hint.key = key;
                }
            }
        }
    }
    hintBar_->setHints(hints, fallback);
    if (infoPanel_) {
        QStringList hintTexts;
        for (const ToolHint& hint : hints) {
            hintTexts << QStringLiteral("%1 %2").arg(hint.key, hint.text);
        }
        const QString name = tools_ ? QString::fromLatin1(toolInfo(tools_->activeTool()).name)
                                    : QString();
        infoPanel_->setToolInfo(name, hintTexts);
    }
}

void PicturaMainWindow::installWorkspaceOpenGesture()
{
    // The blank pane is the tab widget's internal stacked widget, so both are
    // filtered.
    tabs_->installEventFilter(this);
    if (auto* workspace = tabs_->findChild<QStackedWidget*>()) {
        workspace->installEventFilter(this);
    }
}

bool PicturaMainWindow::eventFilter(QObject* watched, QEvent* event)
{
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
