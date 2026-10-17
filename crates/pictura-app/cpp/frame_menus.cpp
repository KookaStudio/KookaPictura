#include "frame_includes.h"
#include "api/type_api.h"

#include "design_system_dialog.h"

#include "api/layers_api.h"

#include "api/document_api.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layers_surface.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layers_smart_filters.cxxqt.h"

namespace pictura {

void PicturaMainWindow::buildMenus()
{
    registry_->buildMenuBar(menuBar());
    // The History panel's iconic flyout is a separate top-level Qt::Popup.
    if (QAction* undo = registry_->action(QLatin1String(command_ids::EditUndo))) {
        undo->setShortcutContext(Qt::ApplicationShortcut);
    }
    installMenuCornerButtons();
    if (QMenu* recent = recentMenu()) {
        connect(recent, &QMenu::aboutToShow, this, [this, recent]() { refreshRecentMenu(recent); });
    }

    // Icon for every implemented command; ids without an asset are skipped.
    static const char* const kIconCommands[] = {
        command_ids::FileNew,
        command_ids::FileOpen,
        command_ids::FileSave,
        command_ids::FileSaveAs,
        command_ids::FileRevert,
        command_ids::FileClose,
        command_ids::FileCloseAll,
        command_ids::FileExit,
        command_ids::EditUndo,
        command_ids::EditRedo,
        command_ids::EditStepForward,
        command_ids::EditStepBackward,
        command_ids::ImageRotate90Cw,
        command_ids::ImageRotate90Ccw,
        command_ids::ImageRotate180,
        command_ids::ImageFlipHorizontal,
        command_ids::ImageFlipVertical,
        command_ids::ImageCrop,
        command_ids::SelectAll,
        command_ids::SelectDeselect,
        command_ids::ViewZoomIn,
        command_ids::ViewZoomOut,
        command_ids::ViewFitOnScreen,
        command_ids::ViewActualPixels,
        command_ids::ViewScreenModeStandard,
        command_ids::ViewScreenModeFullWithMenuBar,
        command_ids::ViewScreenModeFull,
        command_ids::ViewOptions,
        command_ids::WindowPanelsLayers,
        command_ids::WindowPanelsTools,
        command_ids::HelpAbout,
    };
    for (const char* id : kIconCommands) {
        const QString commandId = QString::fromLatin1(id);
        if (QAction* action = registry_->action(commandId)) {
            action->setIcon(icon(commandId));
        }
    }
    wireWorkspaceCommands();
}

bool PicturaMainWindow::saveAsWithDialog()
{
    PictureView* view = activeView();
    if (!view || !view->has_document()) {
        return false;
    }
    const QStringList& filters = saveFileFilters();
    const int sourceIndex = defaultSaveFilterIndex(view);
    QString selected = filters.at(sourceIndex);
    QString path = getSaveFileName(this, tr("Save As"), activeFilePath(), filters, &selected);
    if (path.isEmpty()) {
        return false;
    }
    const int chosen = filters.indexOf(selected);
    // The trailing "All Formats" entry names no single format, so it falls back
    // to the document's smart default for the appended extension.
    int filterIndex = chosen >= 0 && chosen < filters.size() - 1 ? chosen : sourceIndex;
    path = lowercasedImageSuffix(path);
    if (QFileInfo(path).suffix().isEmpty()) {
        path += defaultSuffixForFilter(filterIndex);
    }
    if (isPdfPath(path)) {
        return savePdfWithDialog(path);
    }
    if (saveAsNeedsFeatureWarning(path, view)) {
        const auto choice = QMessageBox::warning(
            this, tr("Save As"),
            tr("The chosen format cannot hold all of the document's features. "
               "The flattened composite will be written."),
            QMessageBox::Save | QMessageBox::Cancel);
        if (choice != QMessageBox::Save) {
            return false;
        }
    }
    return saveActiveAs(path);
}

void PicturaMainWindow::registerHandlers()
{
    registry_->setHandler(command_ids::FileNew, [this]() { showNewDocumentDialog(); });
    registry_->setHandler(command_ids::FileOpen, [this]() { showOpenDialog(); });
    registry_->setHandler(command_ids::FileOpenAsSmartObject, [this]() {
        const QString path = getOpenFileName(this, tr("Open As Smart Object"),
                                             documentDirectory(activeFilePath()),
                                             openFileFilters());
        if (!path.isEmpty()) {
            openAsSmartObjectPath(path);
        }
    });
    registry_->setHandler(command_ids::FilePlace, [this]() {
        PictureView* view = activeView();
        if (!view) {
            return;
        }
        const QString path = getOpenFileName(this, tr("Place"),
                                             documentDirectory(activeFilePath()), openFileFilters());
        if (path.isEmpty()) {
            return;
        }
        const bool psd = PicturaMainWindow::isNativeDocumentPath(path);
        const QString created =
            psd ? view->place_smart_object(path) : view->place_image(path);
        if (!created.isEmpty()) {
            beginFreeTransform(created);
            selectLayerPath(created);
            refresh();
        }
    });

    registry_->setHandler(command_ids::FileInfo, [this]() { showFileInfo(); });

    registry_->setHandler(command_ids::FileSave, [this]() { saveActive(); });
    registry_->setHandler(command_ids::FileSaveAs, [this]() { saveAsWithDialog(); });
    wirePdfCommands();
    registry_->setHandler(command_ids::FileExportAs,
                          [this]() { exportAsFromView(this, activeView()); });
    // File > Print… and Print One Copy (WF-013).
    for (const char* leaf : {"Print…", "Print One Copy"}) {
        const QString id = commandIdForPath({QStringLiteral("File"), QString::fromUtf8(leaf)});
        const bool once = QByteArray(leaf) == "Print One Copy";
        registry_->setImplemented(id, true);
        registry_->setHandler(id, [this, once]() {
            once ? printOneCopyFromView(this, activeView()) : printFromView(this, activeView());
        });
        registry_->setEnabledProvider(
            id, [this]() { return activeView() && activeView()->has_document(); });
    }
    registry_->setHandler(command_ids::FileSaveForWeb, [this]() {
        saveForWebFromView(this, activeView(), documentName(activeDocumentIndex()));
    });
    registry_->setHandler(command_ids::FileQuickExportPng,
                          [this]() { quickExportPngFromView(this, activeView()); });
    registry_->setHandler(command_ids::FileRevert, [this]() {
        const int index = activeDocumentIndex();
        if (index < 0 || documentPath(index).isEmpty()) {
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
    registry_->setHandler(command_ids::FileClose, [this]() { closeActiveDocument(true); });
    registry_->setHandler(command_ids::FileCloseAll, [this]() {
        for (int i = docs_.size() - 1; i >= 0; --i) {
            if (!closeDocument(i, true)) {
                break;
            }
        }
    });
    registry_->setHandler(command_ids::FileExit, [this]() {
        for (int i = docs_.size() - 1; i >= 0; --i) {
            if (!closeDocument(i, true)) {
                break;
            }
        }
        if (documentCount() == 0) {
            qApp->quit();
        }
    });

    auto hasDocument = [this]() { return documentCount() > 0; };
    for (const char* id : {command_ids::FileSave, command_ids::FileSaveAs, command_ids::FileExportAs,
                           command_ids::FileQuickExportPng, command_ids::FileSaveForWeb,
                           command_ids::FileClose,
                           command_ids::FileCloseAll, command_ids::FilePlace,
                           command_ids::FileInfo}) {
        registry_->setEnabledProvider(id, hasDocument);
    }
    registry_->setEnabledProvider(command_ids::FileRevert,
                                  [this]() { return !activeFilePath().isEmpty(); });

    wireEditHistory();

    registry_->setHandler(command_ids::EditAssignProfile,
                          [this]() { showProfileCommand(false); });
    registry_->setHandler(command_ids::EditConvertProfile,
                          [this]() { showProfileCommand(true); });
    registry_->setHandler(command_ids::EditColorSettings,
                          [this]() { showColorSettings(); });
    for (const char* id : {command_ids::EditAssignProfile, command_ids::EditConvertProfile}) {
        registry_->setEnabledProvider(id, [this]() {
            PictureView* view = activeView();
            return view && view->has_document()
                && document_api::documentMode(*view) == QStringLiteral("rgb");
        });
    }

    registry_->setHandler(command_ids::EditFreeTransform, [this]() {
        PictureView* view = activeView();
        const QString path = (layersPanel_ && layersPanel_->selectedPaths().size() == 1)
                                 ? layersPanel_->currentPath()
                                 : QString();
        if (!view) {
            return;
        }
        if (beginFreeTransform(path)) {
            refresh();
        }
    });
    registry_->setEnabledProvider(command_ids::EditFreeTransform, [this]() {
        PictureView* view = activeView();
        const QString path = (layersPanel_ && layersPanel_->selectedPaths().size() == 1)
                                 ? layersPanel_->currentPath()
                                 : QString();
        return view && view->has_document() && !path.isEmpty()
            && view->layer_can_free_transform(path);
    });

    const struct {
        const char* id;
        const char* mode;
    } kTransformModes[] = {
        {command_ids::EditTransformSkew, "skew"},
        {command_ids::EditTransformDistort, "distort"},
        {command_ids::EditTransformPerspective, "perspective"},
    };
    for (const auto& entry : kTransformModes) {
        const QString id = QString::fromLatin1(entry.id);
        const QString mode = QString::fromLatin1(entry.mode);
        registry_->setHandler(id, [this, mode]() {
            PictureView* view = activeView();
            const QString path = (layersPanel_ && layersPanel_->selectedPaths().size() == 1)
                                     ? layersPanel_->currentPath()
                                     : QString();
            if (!view) {
                return;
            }
            if (beginTransformMode(path, mode)) {
                refresh();
            }
        });
        registry_->setEnabledProvider(id, [this]() {
            PictureView* view = activeView();
            const QString path = (layersPanel_ && layersPanel_->selectedPaths().size() == 1)
                                     ? layersPanel_->currentPath()
                                     : QString();
            return view && view->has_document() && !path.isEmpty()
                && view->layer_can_free_transform(path);
        });
    }

    registry_->setHandler(command_ids::EditTransformWarp, [this]() {
        PictureView* view = activeView();
        const QString path = (layersPanel_ && layersPanel_->selectedPaths().size() == 1)
                                 ? layersPanel_->currentPath()
                                 : QString();
        if (!view) {
            return;
        }
        WarpPresetSpec spec;
        if (!WarpPresetDialog::get(this, &spec)) {
            return;
        }
        if (view->apply_warp_preset(path, spec.style_id, spec.bend, spec.distort_x,
                                    spec.distort_y, spec.vertical)) {
            refresh();
        }
    });
    registry_->setEnabledProvider(command_ids::EditTransformWarp, [this]() {
        PictureView* view = activeView();
        const QString path = (layersPanel_ && layersPanel_->selectedPaths().size() == 1)
                                 ? layersPanel_->currentPath()
                                 : QString();
        return view && view->has_document() && !path.isEmpty()
            && view->layer_can_free_transform(path);
    });

    registry_->setHandler(command_ids::ImageRotate90Cw, [this]() {
        if (PictureView* view = activeView(); view && view->rotate_doc(1)) {
            refresh();
        }
    });
    registry_->setHandler(command_ids::ImageRotate90Ccw, [this]() {
        if (PictureView* view = activeView(); view && view->rotate_doc(3)) {
            refresh();
        }
    });
    registry_->setHandler(command_ids::ImageRotate180, [this]() {
        if (PictureView* view = activeView(); view && view->rotate_doc(2)) {
            refresh();
        }
    });
    registry_->setHandler(command_ids::ImageFlipHorizontal, [this]() {
        if (PictureView* view = activeView(); view && view->flip_doc(true)) {
            refresh();
        }
    });
    registry_->setHandler(command_ids::ImageFlipVertical, [this]() {
        if (PictureView* view = activeView(); view && view->flip_doc(false)) {
            refresh();
        }
    });
    for (const char* id : {command_ids::ImageRotate90Cw, command_ids::ImageRotate90Ccw,
                           command_ids::ImageRotate180, command_ids::ImageFlipHorizontal,
                           command_ids::ImageFlipVertical}) {
        registry_->setEnabledProvider(id,
                                      [this]() { return activeView() && activeView()->has_document(); });
    }

    wireImageMenu();

    registry_->setHandler(command_ids::SelectAll, [this]() {
        if (PictureView* view = activeView()) {
            view->select_all();
        }
    });
    registry_->setEnabledProvider(command_ids::SelectAll,
                                  [this]() { return activeView() && activeView()->has_document(); });
    registry_->setHandler(command_ids::SelectDeselect, [this]() {
        if (PictureView* view = activeView()) {
            view->deselect();
        }
    });
    registry_->setEnabledProvider(command_ids::SelectDeselect,
                                  [this]() { return activeView() && activeView()->has_document(); });

    registerSelectHandlers();
    registerEditHandlers();
    wireFilterMenu();
    wireAlignMenu();
    wireTypeMenu();
    wireLayerStyleMenu();
    wireImageAdjustments();
    wireLayerAdjustments();
    wireLayerContentOptions();
    wireLayerArrangeStamp();

    // M37: layer creation and grouping. Layer…/Group… open the modal dialog,
    // whose accept step places the node above the selection; Group from Layers…
    // wraps the panel selection. The remaining commands target the active
    // document's current layer — the Layers panel's selected row, -1 when none.
    // ponytail: one current layer; M38's multi-selection upgrades these to
    // per-selection operations.
    registry_->setHandler(command_ids::LayerNewLayer, [this]() {
        if (layersPanel_) {
            layersPanel_->openNewLayerDialog();
            refresh();
        }
    });
    registry_->setHandler(command_ids::LayerNewGroup, [this]() {
        if (layersPanel_) {
            layersPanel_->openNewGroupDialog();
            refresh();
        }
    });
    registry_->setHandler(command_ids::LayerNewGroupFromLayers, [this]() {
        if (layersPanel_) {
            layersPanel_->openGroupFromLayersDialog();
            refresh();
        }
    });
    registry_->setHandler(command_ids::LayerDuplicateLayer, [this]() {
        if (PictureView* view = activeView()) {
            view->duplicate_layer(layersPanel_ ? layersPanel_->currentLayer() : -1);
            refresh();
        }
    });
    registry_->setHandler(command_ids::LayerGroupLayers, [this]() {
        if (layersPanel_) {
            layersPanel_->groupSelection();
            refresh();
        }
    });
    registry_->setHandler(command_ids::LayerUngroupLayers, [this]() {
        if (layersPanel_) {
            layersPanel_->ungroupSelection();
            refresh();
        }
    });
    for (const char* id : {command_ids::LayerNewLayer, command_ids::LayerNewGroup,
                           command_ids::LayerDuplicateLayer, command_ids::LayerGroupLayers,
                           command_ids::LayerUngroupLayers}) {
        registry_->setEnabledProvider(id,
                                      [this]() { return activeView() && activeView()->has_document(); });
    }
    registry_->setEnabledProvider(command_ids::LayerNewGroupFromLayers, [this]() {
        return activeView() && activeView()->has_document() && layersPanel_
            && !layersPanel_->selectedPaths().isEmpty();
    });

    // Merge and flatten. Merge Down and Merge Layers share one id (design D4):
    // one selected path merges down, several merge the selection.
    registry_->setHandler(command_ids::LayerMergeLayers, [this]() {
        PictureView* view = activeView();
        const QStringList paths = layersPanel_ ? layersPanel_->selectedPaths() : QStringList{};
        if (view && !paths.isEmpty() && view->merge_layers(paths) > 0) {
            refresh();
        }
    });
    registry_->setEnabledProvider(command_ids::LayerMergeLayers, [this]() {
        PictureView* view = activeView();
        return view && view->has_document() && layersPanel_
            && view->can_merge_layers(layersPanel_->selectedPaths());
    });

    registry_->setHandler(command_ids::LayerMergeVisible, [this]() {
        PictureView* view = activeView();
        const QString path = layersPanel_ ? layersPanel_->currentPath() : QString();
        if (view && !path.isEmpty() && view->merge_visible(path) > 0) {
            refresh();
        }
    });
    registry_->setEnabledProvider(command_ids::LayerMergeVisible, [this]() {
        PictureView* view = activeView();
        const QString path = layersPanel_ ? layersPanel_->currentPath() : QString();
        if (!view || !view->has_document() || path.isEmpty()) {
            return false;
        }
        const auto row = pictura::layers_api::rowForPath(*view, path);
        return row.path == path && row.visible;
    });

    // Create / Release Clipping Mask act on every selected layer.
    const auto clippingPaths = [this]() {
        return layersPanel_ ? layersPanel_->selectedPaths() : QStringList();
    };
    registry_->setHandler(command_ids::LayerCreateClippingMask, [this, clippingPaths]() {
        PictureView* view = activeView();
        if (view && clipping_create(*view, clippingPaths()) > 0) {
            refresh();
        }
    });
    registry_->setEnabledProvider(command_ids::LayerCreateClippingMask, [this, clippingPaths]() {
        PictureView* view = activeView();
        return view && view->has_document() && clipping_can_create(*view, clippingPaths());
    });
    registry_->setHandler(command_ids::LayerReleaseClippingMask, [this, clippingPaths]() {
        PictureView* view = activeView();
        if (view && clipping_release(*view, clippingPaths()) > 0) {
            refresh();
        }
    });
    registry_->setEnabledProvider(command_ids::LayerReleaseClippingMask, [this, clippingPaths]() {
        PictureView* view = activeView();
        return view && view->has_document() && clipping_can_release(*view, clippingPaths());
    });

    registry_->setHandler(command_ids::LayerMergeClippingMask, [this]() {
        PictureView* view = activeView();
        const QString path = layersPanel_ ? layersPanel_->currentPath() : QString();
        if (view && !path.isEmpty() && view->merge_clipping_mask(path) > 0) {
            refresh();
        }
    });
    registry_->setEnabledProvider(command_ids::LayerMergeClippingMask, [this]() {
        PictureView* view = activeView();
        const QString path = layersPanel_ ? layersPanel_->currentPath() : QString();
        return view && view->has_document() && !path.isEmpty()
            && view->can_merge_clipping_mask(path);
    });

    // Layer Mask authoring; the bridge acts on the active layer.
    const auto addLayerMask = [this](const QString& kind) {
        PictureView* view = activeView();
        if (view && layer_mask_add(*view, kind)) {
            refresh();
        }
    };
    const auto enableWhenNoMask = [this](const char* id) {
        registry_->setEnabledProvider(id, [this]() {
            PictureView* view = activeView();
            return view && view->has_document() && !layer_mask_present(*view);
        });
    };
    const auto enableWhenMask = [this](const char* id) {
        registry_->setEnabledProvider(id, [this]() {
            PictureView* view = activeView();
            return view && view->has_document() && layer_mask_present(*view);
        });
    };
    registry_->setHandler(command_ids::LayerMaskRevealAll,
                          [addLayerMask]() { addLayerMask(QStringLiteral("reveal-all")); });
    registry_->setHandler(command_ids::LayerMaskHideAll,
                          [addLayerMask]() { addLayerMask(QStringLiteral("hide-all")); });
    registry_->setHandler(command_ids::LayerMaskRevealSelection,
                          [addLayerMask]() { addLayerMask(QStringLiteral("reveal-selection")); });
    registry_->setHandler(command_ids::LayerMaskHideSelection,
                          [addLayerMask]() { addLayerMask(QStringLiteral("hide-selection")); });
    registry_->setHandler(command_ids::LayerMaskFromTransparency, [addLayerMask]() {
        addLayerMask(QStringLiteral("from-transparency"));
    });
    enableWhenNoMask(command_ids::LayerMaskRevealAll);
    enableWhenNoMask(command_ids::LayerMaskHideAll);
    enableWhenNoMask(command_ids::LayerMaskFromTransparency);
    registry_->setEnabledProvider(command_ids::LayerMaskRevealSelection, [this]() {
        PictureView* view = activeView();
        return view && view->has_document() && view->has_selection()
            && !layer_mask_present(*view);
    });
    registry_->setEnabledProvider(command_ids::LayerMaskHideSelection, [this]() {
        PictureView* view = activeView();
        return view && view->has_document() && view->has_selection()
            && !layer_mask_present(*view);
    });
    const auto simpleMaskAction = [this, enableWhenMask](const char* id, auto action) {
        registry_->setHandler(id, [this, action]() {
            PictureView* view = activeView();
            if (view && action(*view)) {
                refresh();
            }
        });
        enableWhenMask(id);
    };
    simpleMaskAction(command_ids::LayerMaskDelete,
                     [](PictureView& view) { return layer_mask_delete(view); });
    simpleMaskAction(command_ids::LayerMaskApply,
                     [](PictureView& view) { return layer_mask_apply(view); });
    simpleMaskAction(command_ids::LayerMaskEnable,
                     [](PictureView& view) { return layer_mask_set_enabled(view, true); });
    simpleMaskAction(command_ids::LayerMaskDisable,
                     [](PictureView& view) { return layer_mask_set_enabled(view, false); });
    simpleMaskAction(command_ids::LayerMaskLink,
                     [](PictureView& view) { return layer_mask_set_linked(view, true); });
    simpleMaskAction(command_ids::LayerMaskUnlink,
                     [](PictureView& view) { return layer_mask_set_linked(view, false); });

    registry_->setHandler(command_ids::LayerFlattenImage, [this]() {
        PictureView* view = activeView();
        if (view && view->flatten_image() > 0) {
            refresh();
        }
    });
    registry_->setEnabledProvider(command_ids::LayerFlattenImage,
                                  [this]() { return activeView() && activeView()->has_document(); });

    // Background conversion. The applicable kind gates each command: "Layer
    // from Background…" needs the Background; "Background From Layer" needs a
    // raster pixel layer (groups and adjustment/fill layers are refused).
    auto currentLayerKind = [this]() -> QString {
        PictureView* view = activeView();
        const QString path = layersPanel_ ? layersPanel_->currentPath() : QString();
        if (!view || path.isEmpty()) {
            return QString();
        }
        return pictura::layers_api::rowForPath(*view, path).kind;
    };
    registry_->setHandler(command_ids::LayerNewLayerFromBackground, [this]() {
        PictureView* view = activeView();
        const QString path = layersPanel_ ? layersPanel_->currentPath() : QString();
        if (view && !path.isEmpty() && view->layer_from_background(path)) {
            refresh();
        }
    });
    registry_->setEnabledProvider(command_ids::LayerNewLayerFromBackground,
                                  [currentLayerKind]() {
                                      return currentLayerKind() == QStringLiteral("background");
                                  });
    registry_->setHandler(command_ids::LayerNewBackgroundFromLayer, [this]() {
        PictureView* view = activeView();
        const QString path = layersPanel_ ? layersPanel_->currentPath() : QString();
        if (view && !path.isEmpty() && view->background_from_layer(path)) {
            refresh();
        }
    });
    registry_->setEnabledProvider(command_ids::LayerNewBackgroundFromLayer, [currentLayerKind]() {
        return currentLayerKind() == QStringLiteral("pixel");
    });

    // Layer via Copy / Cut. Both need an active selection and an active layer
    // (the panel's current row, which the bridge resolves).
    const auto layerVia = [this](bool cut) {
        PictureView* view = activeView();
        const QString path = layersPanel_ ? layersPanel_->currentPath() : QString();
        if (!view || path.isEmpty()) {
            return;
        }
        const QString created =
            cut ? view->layer_via_cut(path) : view->layer_via_copy(path);
        if (!created.isEmpty()) {
            refresh();
        }
    };
    registry_->setHandler(command_ids::LayerNewLayerViaCopy, [layerVia]() { layerVia(false); });
    registry_->setHandler(command_ids::LayerNewLayerViaCut, [layerVia]() { layerVia(true); });
    for (const char* id : {command_ids::LayerNewLayerViaCopy, command_ids::LayerNewLayerViaCut}) {
        registry_->setEnabledProvider(id, [this]() {
            return activeView() && activeView()->has_document() && activeView()->has_selection()
                && layersPanel_ && !layersPanel_->currentPath().isEmpty();
        });
    }

    // Select Similar/Linked and Link/Unlink. A Select command pushes the matched
    // paths back into the panel so the result becomes the panel selection; Link
    // and Unlink only touch the transient link sets. None records history.
    registry_->setHandler(command_ids::LayerSelectSimilar, [this]() {
        PictureView* view = activeView();
        const QString path = layersPanel_ ? layersPanel_->currentPath() : QString();
        if (!view || path.isEmpty()) {
            return;
        }
        const QStringList matched = view->select_similar(path);
        if (layersPanel_) {
            layersPanel_->selectPaths(matched, path);
        }
        refresh();
    });
    registry_->setEnabledProvider(command_ids::LayerSelectSimilar, [this]() {
        return activeView() && activeView()->has_document() && layersPanel_
            && !layersPanel_->currentPath().isEmpty();
    });

    registry_->setHandler(command_ids::LayerSelectLinked, [this]() {
        PictureView* view = activeView();
        const QString path = layersPanel_ ? layersPanel_->currentPath() : QString();
        if (!view || path.isEmpty()) {
            return;
        }
        const QStringList linked = view->select_linked(path);
        if (layersPanel_) {
            layersPanel_->selectPaths(linked, path);
        }
        refresh();
    });
    registry_->setEnabledProvider(command_ids::LayerSelectLinked, [this]() {
        return activeView() && activeView()->has_document() && layersPanel_
            && !layersPanel_->currentPath().isEmpty();
    });

    registry_->setHandler(command_ids::LayerLinkLayers, [this]() {
        PictureView* view = activeView();
        const QStringList paths = layersPanel_ ? layersPanel_->selectedPaths() : QStringList{};
        if (view && !paths.isEmpty()) {
            view->link_layers(paths, true);
        }
    });
    registry_->setHandler(command_ids::LayerUnlinkLayers, [this]() {
        PictureView* view = activeView();
        const QStringList paths = layersPanel_ ? layersPanel_->selectedPaths() : QStringList{};
        if (view && !paths.isEmpty()) {
            view->link_layers(paths, false);
        }
    });
    for (const char* id : {command_ids::LayerLinkLayers, command_ids::LayerUnlinkLayers}) {
        registry_->setEnabledProvider(id, [this]() {
            return activeView() && activeView()->has_document() && layersPanel_
                && !layersPanel_->selectedPaths().isEmpty();
        });
    }

    // Delete Layer removes the Layers-panel selection, as the panel's own
    // delete does, and is unavailable when that would remove nothing (the
    // Background, locked layers, the document's last layer). Delete Hidden
    // Layers needs only a document (it records nothing when nothing is
    // hidden); Hide Layers needs a selection.
    registry_->setHandler(command_ids::LayerDeleteLayer, [this]() {
        if (layersPanel_) {
            layersPanel_->deleteSelection();
            refresh();
        }
    });
    registry_->setEnabledProvider(command_ids::LayerDeleteLayer, [this]() {
        return activeView() && activeView()->has_document() && layersPanel_
            && layers_can_delete(*activeView(), layersPanel_->selectedPaths());
    });
    registry_->setHandler(command_ids::LayerDeleteHiddenLayers, [this]() {
        PictureView* view = activeView();
        if (view && view->delete_hidden_layers() > 0) {
            refresh();
        }
    });
    registry_->setEnabledProvider(command_ids::LayerDeleteHiddenLayers,
                                  [this]() { return activeView() && activeView()->has_document(); });

    registry_->setHandler(command_ids::LayerHideLayers, [this]() {
        PictureView* view = activeView();
        const QStringList paths = layersPanel_ ? layersPanel_->selectedPaths() : QStringList{};
        if (view && !paths.isEmpty() && view->hide_layers(paths) > 0) {
            refresh();
        }
    });
    registry_->setEnabledProvider(command_ids::LayerHideLayers, [this]() {
        return activeView() && activeView()->has_document() && layersPanel_
            && !layersPanel_->selectedPaths().isEmpty();
    });

    // Solid-color fill creation. The color is the frame's foreground, which is
    // not yet plumbed from the toolbox swatch, so it resolves to opaque black.
    registry_->setHandler(command_ids::LayerNewFillSolidColor, [this]() {
        PictureView* view = activeView();
        if (!view) {
            return;
        }
        const QColor fg = foregroundColor();
        const unsigned int argb = fg.isValid() ? fg.rgba() : 0xff000000u;
        if (!view->add_solid_fill(argb).isEmpty()) {
            refresh();
        }
    });
    registry_->setEnabledProvider(command_ids::LayerNewFillSolidColor,
                                  [this]() { return activeView() && activeView()->has_document(); });

    // Gradient fill creation: a fixed black-to-white Linear fill at angle 0.
    registry_->setHandler(command_ids::LayerNewFillGradient, [this]() {
        PictureView* view = activeView();
        if (view && !view->add_gradient_fill().isEmpty()) {
            refresh();
        }
    });
    registry_->setEnabledProvider(command_ids::LayerNewFillGradient,
                                  [this]() { return activeView() && activeView()->has_document(); });

    // Rasterize. Fill Content and Type each need a current layer of that kind;
    // Layer tries type then fill; All Layers needs a document. Shape/Vector
    // Mask/Layer Style/Video/3D have no handler and stay disabled.
    const auto currentFillPath = [this]() -> QString {
        PictureView* view = activeView();
        const QString path = layersPanel_ ? layersPanel_->currentPath() : QString();
        if (!view || path.isEmpty() || !view->layer_is_fill_content(path)) {
            return QString();
        }
        return path;
    };
    const auto currentTypePath = [this]() -> QString {
        PictureView* view = activeView();
        const QString path = layersPanel_ ? layersPanel_->currentPath() : QString();
        if (!view || path.isEmpty() || !pictura::type_api::isType(*view, path)) {
            return QString();
        }
        return path;
    };
    registry_->setHandler(command_ids::LayerRasterizeFillContent, [this, currentFillPath]() {
        if (PictureView* view = activeView()) {
            const QString path = currentFillPath();
            if (!path.isEmpty() && view->rasterize_fill_content(path)) {
                refresh();
            }
        }
    });
    registry_->setEnabledProvider(command_ids::LayerRasterizeFillContent,
                                  [currentFillPath]() { return !currentFillPath().isEmpty(); });
    registry_->setHandler(command_ids::LayerRasterizeLayer, [this, currentFillPath]() {
        if (PictureView* view = activeView()) {
            const QString path = currentFillPath();
            if (!path.isEmpty() && view->rasterize_layer(path)) {
                refresh();
            }
        }
    });
    registry_->setEnabledProvider(command_ids::LayerRasterizeLayer,
                                  [currentFillPath]() { return !currentFillPath().isEmpty(); });
    registry_->setHandler(command_ids::LayerRasterizeType, [this, currentTypePath]() {
        if (PictureView* view = activeView()) {
            const QString path = currentTypePath();
            if (!path.isEmpty() && pictura::type_api::rasterize(*view, path)) {
                refresh();
            }
        }
    });
    registry_->setEnabledProvider(command_ids::LayerRasterizeType,
                                  [currentTypePath]() { return !currentTypePath().isEmpty(); });
    registry_->setHandler(command_ids::LayerRasterizeAllLayers, [this]() {
        PictureView* view = activeView();
        if (view && view->rasterize_all_layers() > 0) {
            refresh();
        }
    });
    registry_->setEnabledProvider(command_ids::LayerRasterizeAllLayers, [this]() {
        PictureView* view = activeView();
        if (!view || !view->has_document()) {
            return false;
        }
        for (const auto& row : pictura::layers_api::rows(*view)) {
            if (view->layer_is_fill_content(row.path)) {
                return true;
            }
        }
        return false;
    });

    wireShapeLayerActions();
    wireVectorMaskActions();
    wireMattingActions();
    wireAdvancedSmartObjectActions();

    // Smart Objects. Convert to Smart Object needs the current layer to be a
    // convertible raster pixel layer or Background (not a group, adjustment, or
    // already a smart object); a Background is unlocked first.
    const auto currentSmartPath = [this]() -> QString {
        PictureView* view = activeView();
        const QString path = layersPanel_ ? layersPanel_->currentPath() : QString();
        if (!view || path.isEmpty() || !view->layer_can_convert_to_smart_object(path)) {
            return QString();
        }
        return path;
    };
    registry_->setHandler(command_ids::LayerSmartObjectConvertTo, [this, currentSmartPath]() {
        if (PictureView* view = activeView()) {
            const QString path = currentSmartPath();
            if (!path.isEmpty() && view->convert_to_smart_object(path)) {
                refresh();
            }
        }
    });
    registry_->setEnabledProvider(command_ids::LayerSmartObjectConvertTo,
                                  [currentSmartPath]() { return !currentSmartPath().isEmpty(); });

    // Convert for Smart Filters shares the convertible-raster eligibility and
    // records its own state label so the Filter menu command is distinct.
    registry_->setHandler(command_ids::FilterConvertForSmartFilters,
                          [this, currentSmartPath]() {
                              if (PictureView* view = activeView()) {
                                  const QString path = currentSmartPath();
                                  if (!path.isEmpty()
                                      && convert_for_smart_filters(*view, path)) {
                                      refresh();
                                  }
                              }
                          });
    registry_->setEnabledProvider(command_ids::FilterConvertForSmartFilters,
                                  [currentSmartPath]() { return !currentSmartPath().isEmpty(); });

    wireSmartFilterActions();

    const auto currentRasterizableSmartPath = [this]() -> QString {
        PictureView* view = activeView();
        const QString path = layersPanel_ ? layersPanel_->currentPath() : QString();
        if (!view || path.isEmpty() || !view->layer_can_rasterize_smart_object(path)) {
            return QString();
        }
        return path;
    };
    registry_->setHandler(command_ids::LayerRasterizeSmartObject,
                          [this, currentRasterizableSmartPath]() {
                              if (PictureView* view = activeView()) {
                                  const QString path = currentRasterizableSmartPath();
                                  if (!path.isEmpty() && view->rasterize_smart_object(path)) {
                                      refresh();
                                  }
                              }
                          });
    registry_->setEnabledProvider(command_ids::LayerRasterizeSmartObject,
                                  [currentRasterizableSmartPath]() {
                                      return !currentRasterizableSmartPath().isEmpty();
                                  });

    // Replace Contents needs the current layer to be a replaceable smart object.
    const auto currentReplaceableSmartPath = [this]() -> QString {
        PictureView* view = activeView();
        const QString path = layersPanel_ ? layersPanel_->currentPath() : QString();
        if (!view || path.isEmpty() || !view->layer_can_replace_smart_object_contents(path)) {
            return QString();
        }
        return path;
    };
    registry_->setHandler(command_ids::LayerSmartObjectReplaceContents,
                          [this, currentReplaceableSmartPath]() {
                              const QString path = currentReplaceableSmartPath();
                              if (!path.isEmpty()) {
                                  replaceSmartObjectContents(path);
                              }
                          });
    registry_->setEnabledProvider(command_ids::LayerSmartObjectReplaceContents,
                                  [currentReplaceableSmartPath]() {
                                      return !currentReplaceableSmartPath().isEmpty();
                                  });

    // Export Contents needs the current layer to carry a non-empty embedded
    // payload. Export only reads the document, so nothing is refreshed.
    const auto currentExportableSmartPath = [this]() -> QString {
        PictureView* view = activeView();
        const QString path = layersPanel_ ? layersPanel_->currentPath() : QString();
        if (!view || path.isEmpty() || !view->layer_can_export_smart_object_contents(path)) {
            return QString();
        }
        return path;
    };
    registry_->setHandler(command_ids::LayerSmartObjectExportContents,
                          [this, currentExportableSmartPath]() {
                              const QString path = currentExportableSmartPath();
                              if (!path.isEmpty()) {
                                  exportSmartObjectContents(path);
                              }
                          });
    registry_->setEnabledProvider(command_ids::LayerSmartObjectExportContents,
                                  [currentExportableSmartPath]() {
                                      return !currentExportableSmartPath().isEmpty();
                                  });

    // Edit Contents needs the current layer to be an embedded PSD/PSB source,
    // so it can open as an ordinary document in an untitled editor tab.
    const auto currentEditableSmartPath = [this]() -> QString {
        PictureView* view = activeView();
        const QString path = layersPanel_ ? layersPanel_->currentPath() : QString();
        if (!view || path.isEmpty() || !view->layer_can_edit_smart_object_contents(path)) {
            return QString();
        }
        return path;
    };
    registry_->setHandler(command_ids::LayerSmartObjectEditContents,
                          [this, currentEditableSmartPath]() {
                              const QString path = currentEditableSmartPath();
                              if (!path.isEmpty() && editSmartObjectContents(path)) {
                                  refresh();
                              }
                          });
    registry_->setEnabledProvider(command_ids::LayerSmartObjectEditContents,
                                  [currentEditableSmartPath]() {
                                      return !currentEditableSmartPath().isEmpty();
                                  });

    // Filter > Pictura Raw…: prefill the dialog from the layer's stored
    // Pictura Raw settings (empty when none), then apply in one bridge call /
    // one history state — destructively on a plain raster layer, as a retained
    // camera-raw smart filter on an embedded smart object.
    registry_->setHandler(command_ids::FilterPicturaRaw, [this]() {
        PictureView* view = activeView();
        const QString path = layersPanel_ ? layersPanel_->currentPath() : QString();
        if (!view || path.isEmpty()) {
            return;
        }
        const QString initial = view->layer_pictura_raw_settings(path);
        QString settings;
        if (!PicturaRawDialog::get(this, view, path, initial, &settings)) {
            return;
        }
        if (view->apply_pictura_raw_filter(path, settings)) {
            refresh();
        }
    });
    registry_->setEnabledProvider(command_ids::FilterPicturaRaw, [this]() {
        PictureView* view = activeView();
        const QString path = layersPanel_ ? layersPanel_->currentPath() : QString();
        return view && view->has_document() && !path.isEmpty()
            && view->layer_can_apply_pictura_raw(path);
    });

    registry_->setHandler(command_ids::ViewZoomIn, [this]() {
        if (ImageView* canvas = imageView()) {
            canvas->zoomIn();
        }
    });
    registry_->setHandler(command_ids::ViewZoomOut, [this]() {
        if (ImageView* canvas = imageView()) {
            canvas->zoomOut();
        }
    });
    registry_->setHandler(command_ids::ViewFitOnScreen, [this]() {
        if (ImageView* canvas = imageView()) {
            canvas->fitOnScreen();
        }
    });
    registry_->setHandler(command_ids::ViewActualPixels, [this]() {
        if (ImageView* canvas = imageView()) {
            canvas->actualPixels();
        }
    });

    registry_->setHandler(command_ids::ViewScreenModeStandard,
                          [this]() { requestScreenMode(ScreenMode::Standard); });
    registry_->setHandler(command_ids::ViewScreenModeFullWithMenuBar,
                          [this]() { requestScreenMode(ScreenMode::FullWithMenuBar); });
    registry_->setHandler(command_ids::ViewScreenModeFull,
                          [this]() { requestScreenMode(ScreenMode::Full); });
    registry_->setCheckedProvider(command_ids::ViewScreenModeStandard,
                                  [this]() { return screenMode_ == ScreenMode::Standard; });
    registry_->setCheckedProvider(command_ids::ViewScreenModeFullWithMenuBar, [this]() {
        return screenMode_ == ScreenMode::FullWithMenuBar;
    });
    registry_->setCheckedProvider(command_ids::ViewScreenModeFull,
                                  [this]() { return screenMode_ == ScreenMode::Full; });

    registry_->setHandler(command_ids::ViewOptions, [this]() {
        QAction* action = registry_->action(command_ids::ViewOptions);
        if (optionsBar_ && action) {
            optionsBar_->setVisible(action->isChecked());
            layoutWorkspaceFrame();
        }
    });
    registry_->setCheckedProvider(command_ids::ViewOptions,
                                  [this]() { return optionsBar_ && optionsBar_->isVisible(); });

    registry_->setHandler(command_ids::ViewToolHints, [this]() {
        const QAction* action = registry_->action(command_ids::ViewToolHints);
        hintsVisible_ = action && action->isChecked();
        updateStatus();
    });
    registry_->setCheckedProvider(command_ids::ViewToolHints, [this]() { return hintsVisible_; });

    registry_->setHandler(command_ids::ViewFreePan, [this]() {
        freePan_ = !freePan_;
        for (const DocEntry& entry : docs_) {
            if (entry.canvas) {
                entry.canvas->setFreePan(freePan_);
            }
        }
        saveSession();
        registry_->refresh();
    });
    registry_->setCheckedProvider(command_ids::ViewFreePan, [this]() { return freePan_; });

    registry_->setHandler(command_ids::ViewSnapCanvasToWorkspace, [this]() {
        snapCanvasToWorkspace_ = !snapCanvasToWorkspace_;
        for (const DocEntry& entry : docs_) {
            if (entry.canvas) {
                entry.canvas->setSnapToWorkspace(snapCanvasToWorkspace_);
            }
        }
        saveSession();
        registry_->refresh();
    });
    registry_->setCheckedProvider(command_ids::ViewSnapCanvasToWorkspace,
                                  [this]() { return snapCanvasToWorkspace_; });

    registry_->setHandler(command_ids::ViewShowSelectionEdges, [this]() {
        const QAction* action = registry_->action(command_ids::ViewShowSelectionEdges);
        if (ImageView* canvas = imageView()) {
            canvas->setSelectionEdgesVisible(action && action->isChecked());
        }
    });
    registry_->setEnabledProvider(command_ids::ViewShowSelectionEdges,
                                  [this]() { return documentCount() > 0; });
    registry_->setCheckedProvider(command_ids::ViewShowSelectionEdges, [this]() {
        ImageView* canvas = imageView();
        return canvas ? canvas->selectionEdgesVisible() : true;
    });

    registry_->setHandler(command_ids::WindowPanelsTools, [this]() {
        QAction* action = registry_->action(command_ids::WindowPanelsTools);
        if (toolsColumn_ && action) {
            toolsColumn_->setVisible(action->isChecked());
        }
    });
    registry_->setCheckedProvider(command_ids::WindowPanelsTools,
                                  [this]() { return toolsColumn_ && toolsColumn_->isVisible(); });

    registerPreferenceHandlers();
    registerShortcutHandlers();
    registerGuideHandlers();

    // Every other panel is a content widget hosted by the PanelColumn; the
    // Window > Panels toggles drive and reflect the column's registry rather
    // than a dock's visibility.
    struct PanelToggle {
        const char* command;
        const char* panel;
    };
    static const PanelToggle kPanelToggles[] = {
        {command_ids::WindowPanelsLayers, "layersPanel"},
        {command_ids::WindowPanelsNavigator, "navigatorPanel"},
        {command_ids::WindowPanelsHistory, "historyPanel"},
        {command_ids::WindowPanelsColor, "colorPanel"},
        {command_ids::WindowPanelsSwatches, "swatchesPanel"},
        {command_ids::WindowPanelsInfo, "infoPanel"},
        {command_ids::WindowPanelsHistogram, "histogramPanel"},
        {command_ids::WindowPanelsGradients, "gradientsPanel"},
        {command_ids::WindowPanelsPatterns, "patternsPanel"},
        {command_ids::WindowPanelsNotes, "notesPanel"},
        {command_ids::WindowPanelsProperties, "propertiesPanel"},
        {command_ids::WindowPanelsAdjustments, "adjustmentsPanel"},
        {command_ids::WindowPanelsChannels, "channelsPanel"},
        {command_ids::WindowPanelsPaths, "pathsPanel"},
        {command_ids::WindowPanelsActions, "actionsPanel"},
        {command_ids::WindowPanelsBrush, "brushPanel"},
        {command_ids::WindowPanelsCloneSource, "cloneSourcePanel"},
        {command_ids::WindowPanelsCharacter, "characterPanel"},
        {command_ids::WindowPanelsParagraph, "paragraphPanel"},
        {command_ids::WindowPanelsGlyphs, "glyphsPanel"},
        {command_ids::WindowPanelsParagraphStyles, "paragraphStylesPanel"},
        {command_ids::TypePanelsCharacter, "characterPanel"},
        {command_ids::TypePanelsParagraph, "paragraphPanel"},
        {command_ids::TypePanelsParagraphStyles, "paragraphStylesPanel"},
    };
    for (const PanelToggle& toggle : kPanelToggles) {
        const QString command = QString::fromLatin1(toggle.command);
        const QString panel = QString::fromLatin1(toggle.panel);
        registry_->setHandler(command, [this, command, panel]() {
            QAction* action = registry_->action(command);
            PanelColumn* owner = columnForPanel(panel);
            if (!owner) {
                owner = panelColumn_;
            }
            if (owner && action) {
                owner->showPanel(panel, action->isChecked());
            }
        });
        registry_->setCheckedProvider(command, [this, panel]() {
            PanelColumn* owner = columnForPanel(panel);
            if (!owner) {
                owner = panelColumn_;
            }
            return owner && owner->isPanelVisible(panel);
        });
    }

    registry_->setHandler(command_ids::HelpAbout, [this]() {
        QMessageBox::about(
            this, tr("About Kooka Pictura"),
            tr("Kooka Pictura — an open-source layered image editor for Linux, written in Rust "
               "and Qt.\n\n"
               "Licensed under the GNU GPL v3.0 or later. Built with Qt 6 (LGPLv3) and "
               "open-source Rust crates; see THIRD-PARTY-LICENSES.\n\n"
                "Adobe, Photoshop, and Camera Raw are trademarks of Adobe Inc.; "
                "Kooka Pictura is not affiliated with or endorsed by Adobe."));
    });

    // Help > Developer > Design System…: the in-app storybook over the real UI.
    registry_->setHandler(command_ids::HelpDeveloperDesignSystem,
                          [this]() { showDesignSystemDialog(); });
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

bool PicturaMainWindow::beginFreeTransform(const QString& path)
{
    return tools_ && tools_->beginFreeTransform(path);
}

bool PicturaMainWindow::beginTransformMode(const QString& path, const QString& mode)
{
    return tools_ && tools_->beginTransformMode(path, mode);
}

void PicturaMainWindow::selectLayerPath(const QString& path)
{
    if (!layersPanel_ || path.isEmpty()) {
        return;
    }
    layersPanel_->setView(activeView());
    layersPanel_->refresh();
    layersPanel_->selectPaths(QStringList{path}, path);
}

} // namespace pictura
