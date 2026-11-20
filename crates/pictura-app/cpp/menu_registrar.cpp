#include "menu_registrar.h"

#include "menu_registrar_internal.h"

#include "commands.h"
#include "document_registry.h"
#include "document_command.h"
#include "fill_layer_editor.h"
#include "icons.h"
#include "import_save_coordinator.h"
#include "memory_refusal.h"
#include "panel_host.h"
#include "pictura_raw_dialog.h"
#include "session.h"
#include "theme.h"
#include "tools.h"

#include "api/document_api.h"
#include "api/filters_bridge.h"
#include "api/layers_api.h"
#include "api/layers_bridge.h"
#include "api/type_api.h"

#include "panels/actions_panel.h"
#include "panels/layers_panel.h"

#include <QtCore/QFileInfo>
#include <QtCore/QJsonValue>
#include <QtGui/QAction>
#include <QtWidgets/QApplication>
#include <QtWidgets/QBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QMenu>
#include <QtWidgets/QMenuBar>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QWidget>
#include <QtWidgets/QWidgetAction>

#include <utility>

namespace pictura {

MenuRegistrar::MenuRegistrar(std::function<void(CommandRegistry&)> registerFrameCommands,
                             Runtime runtime, QObject* parent)
    : QObject(parent)
    , registerFrameCommands_(std::move(registerFrameCommands))
    , runtime_(std::move(runtime))
{
    commands_ = new CommandRegistry(this);
    addDefaultCommands(*commands_);
}

MenuRegistrar::~MenuRegistrar() = default;

void MenuRegistrar::setCollaborators(DocumentRegistry* documents, PanelHost* panels,
                                     ImportSaveCoordinator* importSave)
{
    documents_ = documents;
    panels_ = panels;
    importSave_ = importSave;
}

QWidget* MenuRegistrar::dialogParent() const
{
    return runtime_.dialogParent ? runtime_.dialogParent() : nullptr;
}

PictureView* MenuRegistrar::activeView() const
{
    return menu_internal::activeView(documents_);
}

ImageView* MenuRegistrar::imageView() const
{
    return menu_internal::imageView(documents_);
}

int MenuRegistrar::documentCount() const
{
    return documents_ ? documents_->count() : 0;
}

int MenuRegistrar::activeDocumentIndex() const
{
    return documents_ ? documents_->activeIndex() : -1;
}

PictureView* MenuRegistrar::viewAt(int index) const
{
    return documents_ ? documents_->viewAt(index) : nullptr;
}

QString MenuRegistrar::documentName(int index) const
{
    return documents_ ? documents_->documentName(index) : QString();
}

QString MenuRegistrar::activeDocumentName() const
{
    return documents_ ? documents_->activeDocumentName() : QString();
}

QString MenuRegistrar::activeFilePath() const
{
    return documents_ ? documents_->activeFilePath() : QString();
}

QList<ImageView*> MenuRegistrar::activeDocumentCanvases() const
{
    return documents_ ? documents_->activeDocumentCanvases() : QList<ImageView*>{};
}

QStringList MenuRegistrar::selectedLayerPaths() const
{
    return menu_internal::selectedLayerPaths(activeView());
}

QString MenuRegistrar::layerAnchor() const
{
    return menu_internal::layerAnchor(activeView());
}

void MenuRegistrar::refresh()
{
    if (runtime_.requestRefresh) {
        runtime_.requestRefresh();
    }
}

OptionsBar* MenuRegistrar::optionsBar() const
{
    return runtime_.optionsBar ? runtime_.optionsBar() : nullptr;
}

QColor MenuRegistrar::foregroundColor() const
{
    return runtime_.foregroundColor ? runtime_.foregroundColor() : QColor(Qt::black);
}

void MenuRegistrar::showStatus(const QString& message, int timeoutMs)
{
    if (runtime_.showStatus) {
        runtime_.showStatus(message, timeoutMs);
    }
}

void MenuRegistrar::registerHandlers()
{
    if (registerFrameCommands_) {
        registerFrameCommands_(*commands_);
    }

    // Image rotate / flip run as a document job; a recorded step names its
    // angle or axis.
    const auto orientHandler = [this](const char* id, int turns, const char* key,
                                      const QJsonValue& value) {
        commands_->setHandler(id, [this, id, turns, key, value]() {
            PictureView* view = activeView();
            const bool flip = turns < 0;
            const auto orient = [view, turns, flip] {
                if (flip) {
                    view->flip_doc(turns == -1);
                } else {
                    view->rotate_doc(turns);
                }
            };
            if (!view || !view->has_document()) {
                return;
            }
            if (runDocumentCommand(dialogParent(), *view,
                                   flip ? tr("Flip Canvas") : tr("Rotate Canvas"), orient)) {
                CommandParams command;
                command.values.insert(QString::fromLatin1(key), value);
                commands_->recordCommand(QLatin1String(id), command);
            } else {
                reportCommitRefusal(dialogParent(),
                                    flip ? tr("flip the canvas") : tr("rotate the canvas"));
            }
        });
    };
    orientHandler(command_ids::ImageRotate90Cw, 1, "angle", 90);
    orientHandler(command_ids::ImageRotate90Ccw, 3, "angle", -90);
    orientHandler(command_ids::ImageRotate180, 2, "angle", 180);
    orientHandler(command_ids::ImageFlipHorizontal, -1, "axis", QStringLiteral("horizontal"));
    orientHandler(command_ids::ImageFlipVertical, -2, "axis", QStringLiteral("vertical"));

    commands_->setExecutor(command_ids::ImageRotate90Cw, [this](const CommandParams&) {
        PictureView* view = activeView();
        return view && view->has_document() && view->rotate_doc(1);
    });
    commands_->setExecutor(command_ids::ImageRotate90Ccw, [this](const CommandParams&) {
        PictureView* view = activeView();
        return view && view->has_document() && view->rotate_doc(3);
    });
    commands_->setExecutor(command_ids::ImageRotate180, [this](const CommandParams&) {
        PictureView* view = activeView();
        return view && view->has_document() && view->rotate_doc(2);
    });
    commands_->setExecutor(command_ids::ImageFlipHorizontal, [this](const CommandParams&) {
        PictureView* view = activeView();
        return view && view->has_document() && view->flip_doc(true);
    });
    commands_->setExecutor(command_ids::ImageFlipVertical, [this](const CommandParams&) {
        PictureView* view = activeView();
        return view && view->has_document() && view->flip_doc(false);
    });
    for (const char* id : {command_ids::ImageRotate90Cw, command_ids::ImageRotate90Ccw,
                           command_ids::ImageRotate180, command_ids::ImageFlipHorizontal,
                           command_ids::ImageFlipVertical}) {
        commands_->setEnabledProvider(
            id, [this]() { return activeView() && activeView()->has_document(); });
    }

    wireImageMenu();

    commands_->setHandler(command_ids::SelectAll, [this]() {
        if (PictureView* view = activeView()) {
            view->select_all();
        }
    });
    commands_->setEnabledProvider(
        command_ids::SelectAll, [this]() { return activeView() && activeView()->has_document(); });
    commands_->setHandler(command_ids::SelectDeselect, [this]() {
        if (PictureView* view = activeView()) {
            view->deselect();
        }
    });
    commands_->setEnabledProvider(command_ids::SelectDeselect,
                                  [this]() { return activeView() && activeView()->has_document(); });

    registerEditCommands();
    wireTransformMenu();
    registerSelectCommands();
    wireFilterMenu();
    wireAlignMenu();
    wireTypeMenu();
    wireLayerStyleMenu();
    registerImageCommands();
    registerLayerCommands();
    registerPanelCommands();
    registerTextCommands();
}

void MenuRegistrar::registerEditCommands()
{
    wireEditHistory();
    registerEditHandlers();
}

void MenuRegistrar::registerImageCommands()
{
    wireImageAdjustments();
    wireLayerAdjustments();
    wireLayerContentOptions();
    wireLayerArrangeStamp();
    wireShapeLayerActions();
    wireVectorMaskActions();
    wireMattingActions();
    wireAdvancedSmartObjectActions();
    wireSmartFilterActions();

    // Filter > Pictura Raw…: prefill the dialog from the layer's stored
    // Pictura Raw settings (empty when none), then apply in one bridge call /
    // one history state — destructively on a plain raster layer, as a retained
    // camera-raw smart filter on an embedded smart object.
    commands_->setHandler(command_ids::FilterPicturaRaw, [this]() {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        if (!view || path.isEmpty()) {
            return;
        }
        const QString initial = view->layer_pictura_raw_settings(path);
        QString settings;
        if (!PicturaRawDialog::get(dialogParent(), view, path, initial, &settings)) {
            return;
        }
        if (view->apply_pictura_raw_filter(path, settings)) {
            refresh();
        }
    });
    commands_->setEnabledProvider(command_ids::FilterPicturaRaw, [this]() {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        return view && view->has_document() && !path.isEmpty()
            && view->layer_can_apply_pictura_raw(path);
    });
}

void MenuRegistrar::registerSelectCommands()
{
    registerSelectHandlers();
}

void MenuRegistrar::registerLayerCommands()
{
    // M37: layer creation and grouping. Layer…/Group… open the modal dialog,
    // whose accept step places the node above the selection; Group from Layers…
    // wraps the panel selection. Duplicate acts on every selected path like
    // the panel's row menu; every command reads the Rust layer target, not the
    // panel's tree current row.
    commands_->setHandler(command_ids::LayerNewLayer, [this]() {
        if (panels_ && panels_->layersPanel()) {
            panels_->layersPanel()->openNewLayerDialog();
        }
    });
    commands_->setHandler(command_ids::LayerNewGroup, [this]() {
        if (panels_ && panels_->layersPanel()) {
            panels_->layersPanel()->openNewGroupDialog();
        }
    });
    commands_->setHandler(command_ids::LayerNewGroupFromLayers, [this]() {
        if (panels_ && panels_->layersPanel()) {
            panels_->layersPanel()->openGroupFromLayersDialog();
        }
    });
    commands_->setHandler(command_ids::LayerDuplicateLayer, [this]() {
        PictureView* view = activeView();
        if (!panels_ || !panels_->layersPanel() || !view) {
            return;
        }
        const int before = view->layer_count();
        panels_->layersPanel()->duplicateSelection();
        if (view->layer_count() > before) {
            commands_->recordCommand(command_ids::LayerDuplicateLayer, CommandParams{});
        }
    });
    commands_->setHandler(command_ids::LayerGroupLayers, [this]() {
        if (panels_ && panels_->layersPanel()) {
            panels_->layersPanel()->groupSelection();
        }
    });
    commands_->setHandler(command_ids::LayerUngroupLayers, [this]() {
        if (panels_ && panels_->layersPanel()) {
            panels_->layersPanel()->ungroupSelection();
        }
    });
    for (const char* id : {command_ids::LayerNewLayer, command_ids::LayerNewGroup,
                           command_ids::LayerGroupLayers, command_ids::LayerUngroupLayers}) {
        commands_->setEnabledProvider(
            id, [this]() { return activeView() && activeView()->has_document(); });
    }
    commands_->setEnabledProvider(command_ids::LayerDuplicateLayer, [this]() {
        return activeView() && activeView()->has_document() && !selectedLayerPaths().isEmpty();
    });
    commands_->setEnabledProvider(command_ids::LayerNewGroupFromLayers, [this]() {
        return activeView() && activeView()->has_document() && panels_ && panels_->layersPanel()
            && !selectedLayerPaths().isEmpty();
    });

    // Merge Down and Merge Layers share one id (design D4): one selected layer
    // merges down, two or more merge the selection, and the label follows the
    // selection.
    commands_->setHandler(command_ids::LayerMergeLayers, [this]() {
        PictureView* view = activeView();
        const QStringList paths = selectedLayerPaths();
        if (!view || paths.isEmpty()) {
            return;
        }
        if (runDocumentCommand(dialogParent(), *view, tr("Merge Layers"),
                               [view, &paths] { view->merge_layers(paths); })) {
            if (panels_ && panels_->layersPanel()) {
                panels_->layersPanel()->refresh();
            }
            refresh();
        }
    });
    commands_->setEnabledProvider(command_ids::LayerMergeLayers, [this]() {
        PictureView* view = activeView();
        return view && view->has_document() && panels_ && panels_->layersPanel()
            && view->can_merge_layers(selectedLayerPaths());
    });
    commands_->setLabelProvider(command_ids::LayerMergeLayers, [this]() {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        const bool one = selectedLayerPaths().size() == 1;
        const bool group = view && !path.isEmpty()
            && pictura::layers_api::rowForPath(*view, path).kind == QLatin1String("group");
        const bool fill = view && !path.isEmpty() && view->layer_is_fill_content(path);
        return (one && !group && !fill) ? QStringLiteral("Merge Down")
                                        : QStringLiteral("Merge Layers");
    });

    commands_->setHandler(command_ids::LayerMergeVisible, [this]() {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        if (!view || path.isEmpty()) {
            return;
        }
        if (runDocumentCommand(dialogParent(), *view, tr("Merge Visible"),
                               [view, &path] { view->merge_visible(path); })) {
            if (panels_ && panels_->layersPanel()) {
                panels_->layersPanel()->refresh();
            }
            refresh();
        }
    });
    commands_->setEnabledProvider(command_ids::LayerMergeVisible, [this]() {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        if (!view || !view->has_document() || path.isEmpty()) {
            return false;
        }
        const auto row = pictura::layers_api::rowForPath(*view, path);
        return row.path == path && row.visible;
    });

    // Create / Release Clipping Mask act on every selected layer.
    const auto clippingPaths = [this]() { return selectedLayerPaths(); };
    commands_->setHandler(command_ids::LayerCreateClippingMask, [this, clippingPaths]() {
        PictureView* view = activeView();
        if (view) {
            clipping_create(*view, clippingPaths());
        }
    });
    commands_->setEnabledProvider(command_ids::LayerCreateClippingMask,
                                  [this, clippingPaths]() {
                                      PictureView* view = activeView();
                                      return view && view->has_document()
                                          && clipping_can_create(*view, clippingPaths());
                                  });
    commands_->setHandler(command_ids::LayerReleaseClippingMask, [this, clippingPaths]() {
        PictureView* view = activeView();
        if (view) {
            clipping_release(*view, clippingPaths());
        }
    });
    commands_->setEnabledProvider(command_ids::LayerReleaseClippingMask,
                                  [this, clippingPaths]() {
                                      PictureView* view = activeView();
                                      return view && view->has_document()
                                          && clipping_can_release(*view, clippingPaths());
                                  });

    commands_->setHandler(command_ids::LayerMergeClippingMask, [this]() {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        if (view && !path.isEmpty()) {
            runDocumentCommand(dialogParent(), *view, tr("Merge Clipping Mask"),
                               [view, &path] { view->merge_clipping_mask(path); });
        }
    });
    commands_->setEnabledProvider(command_ids::LayerMergeClippingMask, [this]() {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        return view && view->has_document() && !path.isEmpty()
            && view->can_merge_clipping_mask(path);
    });

    // Layer Mask authoring; the bridge acts on the active layer.
    const auto addLayerMask = [this](const QString& kind) {
        PictureView* view = activeView();
        if (view) {
            if (!layer_mask_add(*view, kind)) {
                reportMemoryRefusal(dialogParent(), tr("add layer mask"));
            }
        }
    };
    const auto enableWhenNoMask = [this](const char* id) {
        commands_->setEnabledProvider(id, [this]() {
            PictureView* view = activeView();
            return view && view->has_document() && !layer_mask_present(*view);
        });
    };
    const auto enableWhenMask = [this](const char* id) {
        commands_->setEnabledProvider(id, [this]() {
            PictureView* view = activeView();
            return view && view->has_document() && layer_mask_present(*view);
        });
    };
    commands_->setHandler(command_ids::LayerMaskRevealAll,
                          [addLayerMask]() { addLayerMask(QStringLiteral("reveal-all")); });
    commands_->setHandler(command_ids::LayerMaskHideAll,
                          [addLayerMask]() { addLayerMask(QStringLiteral("hide-all")); });
    commands_->setHandler(command_ids::LayerMaskRevealSelection,
                          [addLayerMask]() { addLayerMask(QStringLiteral("reveal-selection")); });
    commands_->setHandler(command_ids::LayerMaskHideSelection,
                          [addLayerMask]() { addLayerMask(QStringLiteral("hide-selection")); });
    commands_->setHandler(command_ids::LayerMaskFromTransparency, [addLayerMask]() {
        addLayerMask(QStringLiteral("from-transparency"));
    });
    enableWhenNoMask(command_ids::LayerMaskRevealAll);
    enableWhenNoMask(command_ids::LayerMaskHideAll);
    enableWhenNoMask(command_ids::LayerMaskFromTransparency);
    commands_->setEnabledProvider(command_ids::LayerMaskRevealSelection, [this]() {
        PictureView* view = activeView();
        return view && view->has_document() && view->has_selection()
            && !layer_mask_present(*view);
    });
    commands_->setEnabledProvider(command_ids::LayerMaskHideSelection, [this]() {
        PictureView* view = activeView();
        return view && view->has_document() && view->has_selection()
            && !layer_mask_present(*view);
    });
    const auto simpleMaskAction = [this, enableWhenMask](const char* id, auto action) {
        commands_->setHandler(id, [this, action]() {
            PictureView* view = activeView();
            if (view) {
                action(*view);
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

    wirePanelMenuCommands();

    // Background conversion. The applicable kind gates each command: "Layer
    // from Background…" needs the Background; "Background From Layer" needs a
    // raster pixel layer and a document with no Background yet (groups,
    // adjustment/fill layers, and a second Background are refused).
    auto currentLayerKind = [this]() -> QString {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        if (!view || path.isEmpty()) {
            return QString();
        }
        return pictura::layers_api::rowForPath(*view, path).kind;
    };
    commands_->setHandler(command_ids::LayerNewLayerFromBackground, [this]() {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        if (view && !path.isEmpty()) {
            view->layer_from_background(path);
        }
    });
    commands_->setEnabledProvider(command_ids::LayerNewLayerFromBackground,
                                  [currentLayerKind]() {
                                      return currentLayerKind() == QStringLiteral("background");
                                  });
    commands_->setHandler(command_ids::LayerNewBackgroundFromLayer, [this]() {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        if (view && !path.isEmpty()) {
            view->background_from_layer(path);
        }
    });
    commands_->setEnabledProvider(command_ids::LayerNewBackgroundFromLayer, [this]() {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        return view && !path.isEmpty() && pictura::can_background_from_layer(*view, path);
    });

    // Layer via Copy dispatches on the active layer target: a group, or a
    // selection that covers no pixels, duplicates the selected layers through
    // the duplicate command so an active Action recording keeps the step; a
    // non-group target with a non-empty selection copies the selection's
    // pixels. Cut needs a non-empty selection and a non-group target (the
    // engine refuses groups).
    commands_->setHandler(command_ids::LayerNewLayerViaCopy, [this]() {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        if (!view || path.isEmpty()) {
            return;
        }
        const bool group = pictura::layers_api::rowForPath(*view, path).kind
            == QLatin1String("group");
        if (group || view->selection_count() == 0) {
            commands_->dispatch(command_ids::LayerDuplicateLayer);
            return;
        }
        view->layer_via_copy(path);
    });
    const auto canLayerViaCut = [this]() {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        return view && view->has_document() && view->selection_count() > 0 && !path.isEmpty()
            && pictura::layers_api::rowForPath(*view, path).kind != QLatin1String("group");
    };
    commands_->setHandler(command_ids::LayerNewLayerViaCut, [this, canLayerViaCut]() {
        if (canLayerViaCut()) {
            activeView()->layer_via_cut(layerAnchor());
        }
    });
    commands_->setEnabledProvider(command_ids::LayerNewLayerViaCopy, [this]() {
        PictureView* view = activeView();
        return view && view->has_document() && !layerAnchor().isEmpty();
    });
    commands_->setEnabledProvider(command_ids::LayerNewLayerViaCut, canLayerViaCut);

    // Select Similar/Linked and Link/Unlink. The engine sets the target and
    // notifies the view; Link and Unlink only touch the transient link sets.
    // None records history.
    commands_->setHandler(command_ids::LayerSelectSimilar, [this]() {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        if (!view || path.isEmpty()) {
            return;
        }
        view->select_similar(path);
        if (panels_ && panels_->layersPanel()) {
            panels_->layersPanel()->refresh();
        }
        refresh();
    });
    commands_->setEnabledProvider(command_ids::LayerSelectSimilar, [this]() {
        return activeView() && activeView()->has_document() && panels_ && panels_->layersPanel()
            && !layerAnchor().isEmpty();
    });

    commands_->setHandler(command_ids::LayerSelectLinked, [this]() {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        if (!view || path.isEmpty()) {
            return;
        }
        view->select_linked(path);
        if (panels_ && panels_->layersPanel()) {
            panels_->layersPanel()->refresh();
        }
        refresh();
    });
    commands_->setEnabledProvider(command_ids::LayerSelectLinked, [this]() {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        return view && view->has_document() && !path.isEmpty()
            && pictura::layers_api::rowForPath(*view, path).linked;
    });

    commands_->setHandler(command_ids::LayerLinkLayers, [this]() {
        PictureView* view = activeView();
        const QStringList paths = selectedLayerPaths();
        if (view && !paths.isEmpty()) {
            view->link_layers(paths, true);
        }
    });
    commands_->setHandler(command_ids::LayerUnlinkLayers, [this]() {
        PictureView* view = activeView();
        const QStringList paths = selectedLayerPaths();
        if (view && !paths.isEmpty()) {
            view->link_layers(paths, false);
        }
    });
    for (const char* id : {command_ids::LayerLinkLayers, command_ids::LayerUnlinkLayers}) {
        commands_->setEnabledProvider(id, [this]() {
            return activeView() && activeView()->has_document() && panels_ && panels_->layersPanel()
                && !selectedLayerPaths().isEmpty();
        });
    }

    // Delete Layer removes the Layers-panel selection, as the panel's own
    // delete does, and is unavailable when that would remove nothing (the
    // Background, locked layers, the document's last layer). Delete Hidden
    // Layers needs only a document (it records nothing when nothing is
    // hidden); Hide Layers needs a selection.
    commands_->setHandler(command_ids::LayerDeleteLayer, [this]() {
        if (panels_ && panels_->layersPanel()) {
            panels_->layersPanel()->deleteSelection();
        }
    });
    commands_->setEnabledProvider(command_ids::LayerDeleteLayer, [this]() {
        return activeView() && activeView()->has_document() && panels_ && panels_->layersPanel()
            && layers_can_delete(*activeView(), selectedLayerPaths());
    });
    commands_->setHandler(command_ids::LayerDeleteHiddenLayers, [this]() {
        PictureView* view = activeView();
        if (view) {
            view->delete_hidden_layers();
        }
    });
    commands_->setEnabledProvider(command_ids::LayerDeleteHiddenLayers,
                                  [this]() { return activeView() && activeView()->has_document(); });

    commands_->setHandler(command_ids::LayerHideLayers, [this]() {
        PictureView* view = activeView();
        const QStringList paths = selectedLayerPaths();
        if (view && !paths.isEmpty() && view->hide_layers(paths) > 0) {
            commands_->recordCommand(command_ids::LayerHideLayers, CommandParams{});
        }
    });
    commands_->setEnabledProvider(command_ids::LayerHideLayers, [this]() {
        return activeView() && activeView()->has_document() && panels_ && panels_->layersPanel()
            && !selectedLayerPaths().isEmpty();
    });

    // Fill creation: New Layer dialog -> the type editor -> one creation.
    commands_->setHandler(command_ids::LayerNewFillSolidColor, [this]() {
        if (PictureView* view = activeView()) {
            createFillLayerFromDialog(*view, QStringLiteral("solid-color"), foregroundColor(),
                                      dialogParent(), true);
        }
    });
    commands_->setEnabledProvider(command_ids::LayerNewFillSolidColor,
                                  [this]() { return activeView() && activeView()->has_document(); });

    commands_->setHandler(command_ids::LayerNewFillGradient, [this]() {
        if (PictureView* view = activeView()) {
            createFillLayerFromDialog(*view, QStringLiteral("gradient"), foregroundColor(),
                                      dialogParent(), true);
        }
    });
    commands_->setEnabledProvider(command_ids::LayerNewFillGradient,
                                  [this]() { return activeView() && activeView()->has_document(); });

    commands_->setHandler(command_ids::LayerNewFillPattern, [this]() {
        if (PictureView* view = activeView()) {
            createFillLayerFromDialog(*view, QStringLiteral("pattern"), foregroundColor(),
                                      dialogParent(), true);
        }
    });
    commands_->setEnabledProvider(command_ids::LayerNewFillPattern, [this]() {
        PictureView* view = activeView();
        return view && view->has_document();
    });

    // Rasterize. Fill Content and Type each need a current layer of that kind;
    // Layer tries type then fill; All Layers needs a document. Shape/Vector
    // Mask/Video/3D have no handler and stay disabled.
    const auto currentFillPath = [this]() -> QString {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        if (!view || path.isEmpty() || !view->layer_is_fill_content(path)) {
            return QString();
        }
        return path;
    };
    const auto currentTypePath = [this]() -> QString {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        if (!view || path.isEmpty() || !pictura::type_api::readLayer(*view, path).isType) {
            return QString();
        }
        return path;
    };
    commands_->setHandler(command_ids::LayerRasterizeFillContent, [this, currentFillPath]() {
        if (PictureView* view = activeView()) {
            const QString path = currentFillPath();
            if (!path.isEmpty()) {
                view->rasterize_fill_content(path);
            }
        }
    });
    commands_->setEnabledProvider(command_ids::LayerRasterizeFillContent,
                                  [currentFillPath]() { return !currentFillPath().isEmpty(); });
    commands_->setHandler(command_ids::LayerRasterizeLayer, [this, currentFillPath]() {
        if (PictureView* view = activeView()) {
            const QString path = currentFillPath();
            if (!path.isEmpty()) {
                view->rasterize_layer(path);
            }
        }
    });
    commands_->setEnabledProvider(command_ids::LayerRasterizeLayer,
                                  [currentFillPath]() { return !currentFillPath().isEmpty(); });
    commands_->setHandler(command_ids::LayerRasterizeType, [this, currentTypePath]() {
        if (PictureView* view = activeView()) {
            const QString path = currentTypePath();
            if (!path.isEmpty()) {
                pictura::type_api::rasterize(*view, path);
            }
        }
    });
    commands_->setEnabledProvider(command_ids::LayerRasterizeType,
                                  [currentTypePath]() { return !currentTypePath().isEmpty(); });
    commands_->setHandler(command_ids::LayerRasterizeAllLayers, [this]() {
        PictureView* view = activeView();
        if (view) {
            view->rasterize_all_layers();
        }
    });
    commands_->setEnabledProvider(command_ids::LayerRasterizeAllLayers, [this]() {
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

    // Smart Objects. Convert to Smart Object needs the current layer to be a
    // convertible raster pixel layer or Background (not a group, adjustment, or
    // already a smart object); a Background is unlocked first.
    const auto currentSmartPath = [this]() -> QString {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        if (!view || path.isEmpty() || !view->layer_can_convert_to_smart_object(path)) {
            return QString();
        }
        return path;
    };
    commands_->setHandler(command_ids::LayerSmartObjectConvertTo, [this, currentSmartPath]() {
        if (PictureView* view = activeView()) {
            const QString path = currentSmartPath();
            if (!path.isEmpty() && view->convert_to_smart_object(path)) {
                if (panels_ && panels_->layersPanel()) {
                    panels_->layersPanel()->refresh();
                }
                refresh();
            }
        }
    });
    commands_->setEnabledProvider(command_ids::LayerSmartObjectConvertTo,
                                  [currentSmartPath]() { return !currentSmartPath().isEmpty(); });

    // Convert for Smart Filters shares the convertible-raster eligibility and
    // records its own state label so the Filter menu command is distinct.
    commands_->setHandler(command_ids::FilterConvertForSmartFilters,
                          [this, currentSmartPath]() {
                              if (PictureView* view = activeView()) {
                                  const QString path = currentSmartPath();
                                  if (!path.isEmpty()) {
                                      convert_for_smart_filters(*view, path);
                                  }
                              }
                          });
    commands_->setEnabledProvider(command_ids::FilterConvertForSmartFilters,
                                  [currentSmartPath]() { return !currentSmartPath().isEmpty(); });

    const auto currentRasterizableSmartPath = [this]() -> QString {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        if (!view || path.isEmpty() || !view->layer_can_rasterize_smart_object(path)) {
            return QString();
        }
        return path;
    };
    commands_->setHandler(command_ids::LayerRasterizeSmartObject,
                          [this, currentRasterizableSmartPath]() {
                              if (PictureView* view = activeView()) {
                                  const QString path = currentRasterizableSmartPath();
                                  if (!path.isEmpty()) {
                                      view->rasterize_smart_object(path);
                                  }
                              }
                          });
    commands_->setEnabledProvider(command_ids::LayerRasterizeSmartObject,
                                  [currentRasterizableSmartPath]() {
                                      return !currentRasterizableSmartPath().isEmpty();
                                  });

    // Replace Contents needs the current layer to be a replaceable smart object.
    const auto currentReplaceableSmartPath = [this]() -> QString {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        if (!view || path.isEmpty() || !view->layer_can_replace_smart_object_contents(path)) {
            return QString();
        }
        return path;
    };
    commands_->setHandler(command_ids::LayerSmartObjectReplaceContents,
                          [this, currentReplaceableSmartPath]() {
                              const QString path = currentReplaceableSmartPath();
                              if (!path.isEmpty()) {
                                  replaceSmartObjectContents(path);
                              }
                          });
    commands_->setEnabledProvider(command_ids::LayerSmartObjectReplaceContents,
                                  [currentReplaceableSmartPath]() {
                                      return !currentReplaceableSmartPath().isEmpty();
                                  });

    // Export Contents needs the current layer to carry a non-empty embedded
    // payload. Export only reads the document, so nothing is refreshed.
    const auto currentExportableSmartPath = [this]() -> QString {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        if (!view || path.isEmpty() || !view->layer_can_export_smart_object_contents(path)) {
            return QString();
        }
        return path;
    };
    commands_->setHandler(command_ids::LayerSmartObjectExportContents,
                          [this, currentExportableSmartPath]() {
                              const QString path = currentExportableSmartPath();
                              if (!path.isEmpty()) {
                                  exportSmartObjectContents(path);
                              }
                          });
    commands_->setEnabledProvider(command_ids::LayerSmartObjectExportContents,
                                  [currentExportableSmartPath]() {
                                      return !currentExportableSmartPath().isEmpty();
                                  });

    const auto currentEditableSmartPath = [this]() -> QString {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        if (!view || path.isEmpty() || !view->layer_can_edit_smart_object_contents(path)) {
            return QString();
        }
        return path;
    };
    commands_->setHandler(command_ids::LayerSmartObjectEditContents,
                          [this, currentEditableSmartPath]() {
                              const QString path = currentEditableSmartPath();
                              if (!path.isEmpty() && importSave_
                                  && importSave_->editSmartObjectContents(path)) {
                                  refresh();
                              }
                          });
    commands_->setEnabledProvider(command_ids::LayerSmartObjectEditContents,
                                  [currentEditableSmartPath]() {
                                      return !currentEditableSmartPath().isEmpty();
                                  });
}

void MenuRegistrar::registerPanelCommands()
{
    commands_->setHandler(command_ids::WindowPanelsTools, [this]() {
        QAction* action = commands_->action(command_ids::WindowPanelsTools);
        if (panels_ && panels_->toolsColumn() && action) {
            panels_->toolsColumn()->setVisible(action->isChecked());
        }
    });
    commands_->setCheckedProvider(command_ids::WindowPanelsTools, [this]() {
        return panels_ && panels_->toolsColumn() && panels_->toolsColumn()->isVisible();
    });

    // Every other panel is a content widget hosted by the PanelColumn; the
    // Window > Panels toggles drive and reflect the column's registry rather
    // than a dock's visibility.
    for (const PanelToggle& toggle : panelToggles()) {
        const QString command = QString::fromLatin1(toggle.command);
        const QString panel = QString::fromLatin1(toggle.panel);
        commands_->setHandler(command, [this, command, panel]() {
            QAction* action = commands_->action(command);
            PanelColumn* owner = panels_ ? panels_->columnForPanel(panel) : nullptr;
            if (!owner && panels_) {
                owner = panels_->panelColumn();
            }
            if (owner && action) {
                owner->showPanel(panel, action->isChecked());
            }
        });
        commands_->setCheckedProvider(command, [this, panel]() {
            PanelColumn* owner = panels_ ? panels_->columnForPanel(panel) : nullptr;
            if (!owner && panels_) {
                owner = panels_->panelColumn();
            }
            return owner && owner->isPanelVisible(panel);
        });
    }
    // The catalogue keeps this row disabled; the generic toggle owns it now.
    commands_->setImplemented(QStringLiteral("window.panels.layer.comps"), true);
}

void MenuRegistrar::registerTextCommands()
{
    // Text commands and the missing-font alert live in menu_type.cpp; the
    // gamma preference stays on the frame's colour settings.
}

void MenuRegistrar::buildMenus(QMenuBar* menuBar)
{
    menuBar_ = menuBar;
    // The old menu owns the previous document rows; drop them before the
    // command registry deletes and rebuilds the menu bar.
    if (windowMenu_) {
        for (QAction* action : windowDocumentActions_) {
            windowMenu_->removeAction(action);
            delete action;
        }
    }
    windowDocumentActions_.clear();
    commands_->buildMenuBar(menuBar);
    buildMeasurementScaleMenu();
    buildUiSizeMenu();
    // The open-document list is the Window menu's last section (#923). It is
    // built per open so labels and host moves need no signal bookkeeping.
    if (windowMenu_) {
        QMenu* window = windowMenu_;
        connect(window, &QMenu::aboutToShow, this,
                [this, window]() { refreshWindowDocumentList(window); });
    }
    // The History panel's iconic flyout is a separate top-level Qt::Popup.
    if (QAction* undo = commands_->action(QLatin1String(command_ids::EditUndo))) {
        undo->setShortcutContext(Qt::ApplicationShortcut);
    }
    if (runtime_.installMenuCornerButtons) {
        runtime_.installMenuCornerButtons();
    }
    QMenu* recent = recentMenu();
    if (recent) {
        connect(recent, &QMenu::aboutToShow, this, [this, recent]() { refreshRecentMenu(recent); });
    }
    // Icon for every implemented command; ids without an asset are skipped.
    // ponytail: file.saveCopy has no icon asset yet; the File action stays text-only.
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
        command_ids::WindowPanelsLayerComps,
        command_ids::FileScriptsLayerCompsToFiles,
        command_ids::FileScriptsLayerCompsToPdf,
        command_ids::HelpAbout,
    };
    for (const char* id : kIconCommands) {
        const QString commandId = QString::fromLatin1(id);
        if (QAction* action = commands_->action(commandId)) {
            action->setIcon(icon(commandId));
        }
    }
    wireWorkspaceCommands();
}

bool MenuRegistrar::beginFreeTransform(const QString& path)
{
    return tools_ && tools_->beginFreeTransform(path);
}

bool MenuRegistrar::beginTransformMode(const QString& path, const QString& mode)
{
    return tools_ && tools_->beginTransformMode(path, mode);
}

void MenuRegistrar::selectLayerPath(const QString& path)
{
    PictureView* view = activeView();
    if (!view || path.isEmpty()) {
        return;
    }
    select_layer_path(*view, path);
    if (panels_ && panels_->layersPanel()) {
        panels_->layersPanel()->refresh();
    }
}

void MenuRegistrar::buildUiSizeMenu()
{
    QAction* readoutAction = commands_->action(command_ids::WindowUiSize);
    if (!readoutAction) {
        return;
    }
    QMenu* windowMenu = nullptr;
    for (QObject* object : readoutAction->associatedObjects()) {
        if ((windowMenu = qobject_cast<QMenu*>(object))) {
            break;
        }
    }
    if (!windowMenu) {
        return;
    }
    windowMenu_ = windowMenu;

    auto* row = new QWidget(windowMenu);
    row->setObjectName(QStringLiteral("uiSizeControl"));
    auto* layout = new QVBoxLayout(row);
    layout->setContentsMargins(8, 2, 8, 2);
    layout->setSpacing(4);
    auto* caption = new QLabel(tr("UI size"), row);
    caption->setObjectName(QStringLiteral("uiSizeCaption"));
    layout->addWidget(caption);

    auto* stepRow = new QHBoxLayout();
    stepRow->setContentsMargins(0, 0, 0, 0);
    stepRow->setSpacing(4);
    stepRow->addStretch(1);
    layout->addLayout(stepRow);

    const auto addButton = [row](QBoxLayout* target, const QString& name,
                                 const QString& text, const QString& tip) {
        auto* button = new QToolButton(row);
        button->setObjectName(name);
        button->setText(text);
        button->setAutoRaise(true);
        button->setToolTip(tip);
        target->addWidget(button);
        return button;
    };
    auto* decrease = addButton(stepRow, QStringLiteral("uiSizeDecrease"), QStringLiteral("-"),
                               tr("Decrease the UI size by one step"));
    auto* readout = new QLabel(row);
    readout->setObjectName(QStringLiteral("uiSizeReadout"));
    readout->setAlignment(Qt::AlignCenter);
    readout->setMinimumWidth(Theme::scaled(44));
    stepRow->addWidget(readout);
    auto* increase = addButton(stepRow, QStringLiteral("uiSizeIncrease"), QStringLiteral("+"),
                               tr("Increase the UI size by one step"));
    stepRow->addStretch(1);
    auto* reset = addButton(layout, QStringLiteral("uiSizeReset"), tr("Reset"),
                            tr("Reset the UI size to 100%"));
    layout->setAlignment(reset, Qt::AlignHCenter);

    connect(decrease, &QToolButton::clicked, this,
            [this]() { commands_->dispatch(command_ids::WindowUiDecrease); });
    connect(increase, &QToolButton::clicked, this,
            [this]() { commands_->dispatch(command_ids::WindowUiEnlarge); });
    connect(reset, &QToolButton::clicked, this,
            [this]() { commands_->dispatch(command_ids::WindowUiReset); });

    auto* widgetAction = new QWidgetAction(windowMenu);
    widgetAction->setDefaultWidget(row);
    windowMenu->insertAction(readoutAction, widgetAction);
    readoutAction->setVisible(false);
    for (const char* id : {command_ids::WindowUiEnlarge, command_ids::WindowUiDecrease,
                           command_ids::WindowUiReset}) {
        if (QAction* action = commands_->action(QString::fromLatin1(id))) {
            // Keep the command on the window rather than in the menu: this
            // hides its menu row and still installs its shortcut.
            windowMenu->removeAction(action);
            if (QWidget* owner = dialogParent()) {
                owner->addAction(action);
            }
        }
    }
    connect(windowMenu, &QMenu::aboutToShow, this, [this]() {
        commands_->refresh();
        updateUiSizeMenu();
    });
    updateUiSizeMenu();
}

void MenuRegistrar::updateUiSizeMenu()
{
    QWidget* parent = dialogParent();
    if (!parent) {
        return;
    }
    if (auto* readout = parent->findChild<QLabel*>(QStringLiteral("uiSizeReadout"))) {
        const int percent = runtime_.uiScalePercent ? runtime_.uiScalePercent() : 100;
        readout->setText(tr("%1%").arg(percent));
    }
    const auto sync = [this, parent](const char* name, const char* command) {
        if (auto* button = parent->findChild<QToolButton*>(QString::fromLatin1(name))) {
            if (QAction* action = commands_->action(QString::fromLatin1(command))) {
                button->setEnabled(action->isEnabled());
            }
        }
    };
    sync("uiSizeDecrease", command_ids::WindowUiDecrease);
    sync("uiSizeIncrease", command_ids::WindowUiEnlarge);
    sync("uiSizeReset", command_ids::WindowUiReset);
    if (auto* row = parent->findChild<QWidget*>(QStringLiteral("uiSizeControl"))) {
        if (QLayout* layout = row->layout()) {
            // The scale change above resizes the row's fonts, but the layout
            // has not run yet; activate it so the menu reads a current hint.
            layout->activate();
        }
        if (auto* menu = qobject_cast<QMenu*>(row->parentWidget())) {
            if (menu->isVisible()) {
                // The popup keeps its show-time geometry; growing the row would
                // clip it, so refit the open menu to its rows.
                menu->resize(menu->sizeHint());
            }
        }
    }
}

} // namespace pictura

#include "moc_menu_registrar.cpp"
