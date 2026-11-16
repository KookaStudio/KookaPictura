// The chrome/settings commands that stay owned by the frame. MenuRegistrar
// receives this registration through its registerFrameCommands callback, so
// the menu translation units never name PicturaMainWindow.

#include "frame_includes.h"
#include "export_as_dialog.h"
#include "print_dialog.h"
#include "save_for_web_dialog.h"

#include "import_save_coordinator.h"

#include "auto_align_dialog.h"
#include "auto_blend_dialog.h"
#include "busy_job.h"
#include "design_system_dialog.h"
#include "frame_place.h"
#include "frame_update.h"
#include "gpu_probe.h"

#include "api/engine_bridge.h"
#include "api/filters_bridge.h"
#include "api/selection_bridge.h"
#include "api/transform_bridge.h"
#include "api/document_api.h"

#include "panels/layers_panel.h"

#include <QtCore/QJsonValue>
#include <QtWidgets/QMessageBox>

namespace pictura {

void PicturaMainWindow::registerFrameCommands(CommandRegistry& registry)
{
    registry.setHandler(command_ids::FileNew, [this]() { showNewDocumentDialog(); });
    registry.setHandler(command_ids::FileOpen, [this]() { showOpenDialog(); });
    registry.setHandler(command_ids::FileOpenAs, [this]() { showOpenAsDialog(); });
    registry.setHandler(command_ids::FileOpenAsSmartObject, [this]() {
        const QString path = getOpenFileName(this, tr("Open As Smart Object"),
                                             documentDirectory(activeFilePath()),
                                             openFileFilters());
        if (!path.isEmpty()) {
            startOpenAsSmartObjectPath(path);
        }
    });
    registry.setHandler(command_ids::FilePlace, [this]() {
        PictureView* view = activeView();
        if (!view || !view->has_document() || view->transform_session_active()) {
            return;
        }
        const QStringList paths = getOpenFileNames(this, tr("Place"),
                                                   documentDirectory(activeFilePath()),
                                                   openFileFilters());
        if (paths.isEmpty()) {
            return;
        }
        placeDocuments(*this, paths);
    });

    registry.setHandler(command_ids::FileInfo, [this]() { showFileInfo(); });

    registry.setHandler(command_ids::FileSave, [this]() { saveActive(); });
    registry.setHandler(command_ids::FileSaveAs, [this]() { saveAsWithDialog(); });
    wirePdfCommands();
    registry.setHandler(command_ids::FileExportAs,
                        [this]() { exportAsFromView(this, activeView()); });
    // File > Print… and Print One Copy (WF-013).
    const char* const printIds[] = {command_ids::FilePrint, command_ids::FilePrintOneCopy};
    for (int i = 0; i < 2; ++i) {
        const QString id = QString::fromLatin1(printIds[i]);
        const bool once = i == 1;
        registry.setImplemented(id, true);
        registry.setHandler(id, [this, once]() {
            once ? printOneCopyFromView(this, activeView()) : printFromView(this, activeView());
        });
        registry.setEnabledProvider(
            id, [this]() { return activeView() && activeView()->has_document(); });
    }
    registry.setHandler(command_ids::FileSaveForWeb, [this]() {
        saveForWebFromView(this, activeView(), documentName(activeDocumentIndex()));
    });
    registry.setHandler(command_ids::FileQuickExportPng,
                        [this]() { quickExportPngFromView(this, activeView()); });
    registry.setHandler(command_ids::FileRevert, [this]() {
        const int index = activeDocumentIndex();
        PictureView* view = activeView();
        if (index < 0 || !view || document_api::filePath(*view).isEmpty()) {
            return;
        }
        if (isDocumentDirty(index)) {
            switch (askUnsaved(this, documentName(index))) {
            case UnsavedChoice::Cancel:
                return;
            case UnsavedChoice::Save:
                setActiveDocumentIndex(index);
                if (!saveActive()) {
                    return;
                }
                break;
            case UnsavedChoice::Discard:
                break;
            }
        }
        revertActive();
    });
    registry.setHandler(command_ids::FileClose,
                        [this]() { closeWindow(documents_->activeWindow(), true); });
    registry.setHandler(command_ids::FileCloseAll, [this]() {
        for (int i = documents_->docs().size() - 1; i >= 0; --i) {
            if (!closeDocument(i, true)) {
                break;
            }
        }
    });
    registry.setHandler(command_ids::FileExit, [this]() {
        for (int i = documents_->docs().size() - 1; i >= 0; --i) {
            if (!closeDocument(i, true)) {
                break;
            }
        }
        if (documentCount() == 0) {
            qApp->quit();
        }
    });

    auto hasDocument = [this]() { return documentCount() > 0; };
    for (const char* id : {command_ids::FileSave, command_ids::FileSaveAs,
                           command_ids::FileExportAs, command_ids::FileQuickExportPng,
                           command_ids::FileSaveForWeb, command_ids::FileClose,
                           command_ids::FileCloseAll, command_ids::FileInfo}) {
        registry.setEnabledProvider(id, hasDocument);
    }
    registry.setEnabledProvider(command_ids::FilePlace, [this]() {
        PictureView* view = activeView();
        return view && view->has_document() && !view->transform_session_active();
    });
    registry.setEnabledProvider(command_ids::FileRevert, [this]() {
        PictureView* view = activeView();
        // A raster/smart-object import keeps the source path only on the tab;
        // its document path stays empty (Save must still ask where to write).
        return view && !document_api::filePath(*view).isEmpty();
    });

    wireColorManagementMenu();

    registry.setHandler(command_ids::ViewZoomIn, [this]() {
        if (ImageView* canvas = imageView()) {
            canvas->zoomIn();
            if (zoomResizesWindows_) {
                documents_->resizeWindowToFit(documents_->windowOfCanvas(canvas));
            }
        }
    });
    registry.setHandler(command_ids::ViewZoomOut, [this]() {
        if (ImageView* canvas = imageView()) {
            canvas->zoomOut();
            if (zoomResizesWindows_) {
                documents_->resizeWindowToFit(documents_->windowOfCanvas(canvas));
            }
        }
    });
    registry.setHandler(command_ids::ViewFitOnScreen, [this]() {
        if (ImageView* canvas = imageView()) {
            canvas->fitOnScreen();
        }
    });
    registry.setHandler(command_ids::ViewActualPixels, [this]() {
        if (ImageView* canvas = imageView()) {
            canvas->actualPixels();
        }
    });

    registry.setHandler(command_ids::ViewScreenModeStandard,
                        [this]() { requestScreenMode(ScreenMode::Standard); });
    registry.setHandler(command_ids::ViewScreenModeFullWithMenuBar,
                        [this]() { requestScreenMode(ScreenMode::FullWithMenuBar); });
    registry.setHandler(command_ids::ViewScreenModeFull,
                        [this]() { requestScreenMode(ScreenMode::Full); });
    registry.setCheckedProvider(command_ids::ViewScreenModeStandard,
                                [this]() { return screenMode_ == ScreenMode::Standard; });
    registry.setCheckedProvider(command_ids::ViewScreenModeFullWithMenuBar,
                                [this]() { return screenMode_ == ScreenMode::FullWithMenuBar; });
    registry.setCheckedProvider(command_ids::ViewScreenModeFull,
                                [this]() { return screenMode_ == ScreenMode::Full; });

    registry.setHandler(command_ids::ViewOptions, [this, &registry]() {
        QAction* action = registry.action(command_ids::ViewOptions);
        if (optionsBar_ && action) {
            optionsBar_->setVisible(action->isChecked());
            layoutWorkspaceFrame();
        }
    });
    registry.setCheckedProvider(command_ids::ViewOptions,
                                [this]() { return optionsBar_ && optionsBar_->isVisible(); });

    registry.setHandler(command_ids::ViewToolHints, [this, &registry]() {
        const QAction* action = registry.action(command_ids::ViewToolHints);
        hintsVisible_ = action && action->isChecked();
        updateStatus();
    });
    registry.setCheckedProvider(command_ids::ViewToolHints, [this]() { return hintsVisible_; });

    registry.setHandler(command_ids::ViewFreePan, [this, &registry]() {
        freePan_ = !freePan_;
        for (const DocWindow* window : documents_->windows()) {
            if (window->canvas) {
                window->canvas->setFreePan(freePan_);
            }
        }
        saveSession();
        registry.refresh();
    });
    registry.setCheckedProvider(command_ids::ViewFreePan, [this]() { return freePan_; });

    registry.setHandler(command_ids::ViewSnapCanvasToWorkspace, [this, &registry]() {
        snapCanvasToWorkspace_ = !snapCanvasToWorkspace_;
        for (const DocWindow* window : documents_->windows()) {
            if (window->canvas) {
                window->canvas->setSnapToWorkspace(snapCanvasToWorkspace_);
            }
        }
        saveSession();
        registry.refresh();
    });
    registry.setCheckedProvider(command_ids::ViewSnapCanvasToWorkspace,
                                [this]() { return snapCanvasToWorkspace_; });

    registry.setHandler(command_ids::ViewShowSelectionEdges, [this, &registry]() {
        const QAction* action = registry.action(command_ids::ViewShowSelectionEdges);
        if (ImageView* canvas = imageView()) {
            canvas->setSelectionEdgesVisible(action && action->isChecked());
        }
    });
    registry.setEnabledProvider(command_ids::ViewShowSelectionEdges,
                                [this]() { return documentCount() > 0; });
    registry.setCheckedProvider(command_ids::ViewShowSelectionEdges, [this]() {
        ImageView* canvas = imageView();
        return canvas ? canvas->selectionEdgesVisible() : true;
    });

    const auto stepUiScale = [this](int direction) {
        const QVector<int> choices = Theme::uiScaleChoices();
        int index = choices.indexOf(uiScalePercent_);
        if (index < 0) {
            index = choices.indexOf(Theme::kDefaultUiScalePercent);
        }
        setUiScalePercent(choices.at(qBound(0, index + direction, choices.size() - 1)));
    };
    registry.setHandler(command_ids::WindowUiEnlarge, [stepUiScale]() { stepUiScale(1); });
    registry.setHandler(command_ids::WindowUiDecrease, [stepUiScale]() { stepUiScale(-1); });
    registry.setHandler(command_ids::WindowUiReset,
                        [this]() { setUiScalePercent(Theme::kDefaultUiScalePercent); });
    registry.setEnabledProvider(command_ids::WindowUiEnlarge,
                                [this]() { return uiScalePercent_ < Theme::kUiScaleMaxPercent; });
    registry.setEnabledProvider(command_ids::WindowUiDecrease,
                                [this]() { return uiScalePercent_ > Theme::kUiScaleMinPercent; });
    registry.setEnabledProvider(command_ids::WindowUiReset,
                                [this]() { return uiScalePercent_ != Theme::kDefaultUiScalePercent; });
    registry.setLabelProvider(command_ids::WindowUiSize,
                              [this]() { return tr("UI Size: %1%").arg(uiScalePercent_); });

    registerPreferenceHandlers();
    registerUpdateHandlers(*this);
    registerShortcutHandlers();
    registerGuideHandlers();
    registerProofHandlers();
    registerSearchHandlers();

    registry.setHandler(command_ids::HelpSystemInfo, [this]() { showSystemInfo(); });
    registry.setHandler(command_ids::HelpSubmitIssue, [this]() { submitIssue(); });

    registry.setHandler(command_ids::HelpAbout, [this]() {
        QMessageBox::about(
            this, tr("About Kooka Pictura"),
            tr("Kooka Pictura — an open-source layered image editor for Linux, written in Rust "
               "and Qt.\n\n"
               "Licensed under the GNU GPL v3.0 or later. Built with Qt 6 (LGPLv3) and "
               "open-source Rust crates; see THIRD-PARTY-LICENSES.\n\n"
               "%1\n\n"
                "Adobe, Photoshop, and Camera Raw are trademarks of Adobe Inc.; "
                "Kooka Pictura is not affiliated with or endorsed by Adobe.")
                .arg(aboutText(activeBackendLabel(gpuCompute_)))
                + QLatin1Char('\n') + QLatin1Char('\n') + aboutVersionText());
    });

    // Help > Developer > Design System…: the in-app storybook over the real UI.
    registry.setHandler(command_ids::HelpDeveloperDesignSystem,
                        [this]() { showDesignSystemDialog(); });

    const auto saveToRecordedPath = [this](const CommandParams& params) {
        return saveActiveAsWithRememberedOptions(params.values.value(QStringLiteral("in")).toString());
    };
    registry.setExecutor(command_ids::FileSave, saveToRecordedPath);
    registry.setExecutor(command_ids::FileSaveAs, saveToRecordedPath);
}

void PicturaMainWindow::showDesignSystemDialog()
{
    if (!designSystemDialog_) {
        designSystemDialog_ = new DesignSystemDialog(this);
    }
    designSystemDialog_->show();
    designSystemDialog_->raise();
    designSystemDialog_->activateWindow();
}

bool runAutoAlign(PicturaMainWindow& window, int projection, bool vignette, bool geometric)
{
    PictureView* view = window.activeView();
    auto* panel = window.findChild<LayersPanel*>(QStringLiteral("layersPanel"));
    if (!view || !panel) {
        return false;
    }
    const QStringList paths = window.selectedLayerPaths();
    QStringList moved;
    const bool committed = runCancellableJob(
        &window, PicturaMainWindow::tr("Auto-Align Layers"),
        PicturaMainWindow::tr("Aligning layers…"),
        [&] { return auto_align_start(*view, paths, projection, vignette, geometric); },
        [&] { return auto_align_running(*view); },
        [&] { return auto_align_progress(*view); },
        [&] { auto_align_cancel(*view); },
        [&] {
            moved = auto_align_finish(*view);
            return !moved.isEmpty();
        });
    if (committed) {
        window.selectLayerPath(moved.last());
        panel->selectPaths(moved, moved.last());
    }
    if (const QString notice = auto_align_notice(*view); !notice.isEmpty()) {
        window.statusBar()->showMessage(notice, 8000);
    }
    return committed;
}

bool runAutoBlend(PicturaMainWindow& window, bool stackImages, bool seamless,
                  bool fillTransparent)
{
    PictureView* view = window.activeView();
    auto* panel = window.findChild<LayersPanel*>(QStringLiteral("layersPanel"));
    if (!view || !panel) {
        return false;
    }
    const QStringList paths = window.selectedLayerPaths();
    QStringList blended;
    const bool committed = runCancellableJob(
        &window, PicturaMainWindow::tr("Auto-Blend Layers"),
        PicturaMainWindow::tr("Blending layers…"),
        [&] { return auto_blend_start(*view, paths, stackImages, seamless, fillTransparent); },
        [&] { return auto_blend_running(*view); },
        [&] { return auto_blend_progress(*view); },
        [&] { auto_blend_cancel(*view); },
        [&] {
            blended = auto_blend_finish(*view);
            return !blended.isEmpty();
        });
    if (committed) {
        window.selectLayerPath(blended.last());
        panel->selectPaths(blended, blended.last());
    }
    if (const QString notice = auto_blend_error(*view); !notice.isEmpty()) {
        window.statusBar()->showMessage(notice, 8000);
    }
    return committed;
}

} // namespace pictura
