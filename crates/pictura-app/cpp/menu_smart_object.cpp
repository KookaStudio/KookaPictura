// Layer > Smart Objects: Reset Transform, Convert to Layers, and New Smart
// Object via Copy, over the active smart-object layer. Each edit is one undo
// state; the bridge refuses a layer whose embedded source does not parse.


#include "menu_registrar.h"

#include "commands.h"

#include "menu_registrar_internal.h"

#include "document_registry.h"
#include "import_save_coordinator.h"
#include "panel_host.h"
#include "tools.h"

#include "api/layers_bridge.h"
#include "dialogs.h"
#include "panels/layers_panel.h"

namespace pictura {

void MenuRegistrar::wireAdvancedSmartObjectActions()
{
    // Reset Transform / Convert to Layers need an embedded PSD/PSB source.
    const auto currentResettableSmartPath = [this]() -> QString {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        if (!view || path.isEmpty() || !layer_can_reset_smart_object_transform(*view, path)) {
            return QString();
        }
        return path;
    };
    commands_->setHandler(command_ids::LayerSmartObjectResetTransform,
                          [this, currentResettableSmartPath]() {
                              if (PictureView* view = activeView()) {
                                  const QString path = currentResettableSmartPath();
                                  if (!path.isEmpty()) {
                                      smart_object_reset_transform(*view, path);
                                  }
                              }
                          });
    commands_->setEnabledProvider(command_ids::LayerSmartObjectResetTransform,
                                  [currentResettableSmartPath]() {
                                      return !currentResettableSmartPath().isEmpty();
                                  });

    const auto currentConvertibleToLayersPath = [this]() -> QString {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        if (!view || path.isEmpty() || !layer_can_convert_smart_object_to_layers(*view, path)) {
            return QString();
        }
        return path;
    };
    commands_->setHandler(command_ids::LayerSmartObjectConvertToLayers,
                          [this, currentConvertibleToLayersPath]() {
                              if (PictureView* view = activeView()) {
                                  const QString path = currentConvertibleToLayersPath();
                                  if (!path.isEmpty()) {
                                      smart_object_convert_to_layers(*view, path);
                                  }
                              }
                          });
    commands_->setEnabledProvider(command_ids::LayerSmartObjectConvertToLayers,
                                  [currentConvertibleToLayersPath]() {
                                      return !currentConvertibleToLayersPath().isEmpty();
                                  });

    // New Smart Object via Copy needs a copyable embedded smart object; the
    // copy is selected after it is inserted.
    const auto currentCopyableSmartPath = [this]() -> QString {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        if (!view || path.isEmpty() || !layer_can_new_smart_object_via_copy(*view, path)) {
            return QString();
        }
        return path;
    };
    commands_->setHandler(command_ids::LayerSmartObjectNewViaCopy,
                          [this, currentCopyableSmartPath]() {
                              PictureView* view = activeView();
                              const QString path = currentCopyableSmartPath();
                              if (!view || path.isEmpty()) {
                                  return;
                              }
                              const QString copy = smart_object_new_via_copy(*view, path);
                              if (copy.isEmpty()) {
                                  return;
                              }
                              refresh();
                          });

    // Layer > Smart Objects > Stack Mode: 12 checkable leaves. None clears the
    // mode; the other leaves reduce the embedded stack. A mode needs an
    // embedded payload with at least two visible frames.
    struct StackModeLeaf {
        const char* command;
        int code;
    };
    static const StackModeLeaf stackModeLeaves[] = {
        {command_ids::LayerSmartObjectsStackModeNone, 0},
        {command_ids::LayerSmartObjectsStackModeEntropy, 11},
        {command_ids::LayerSmartObjectsStackModeKurtosis, 10},
        {command_ids::LayerSmartObjectsStackModeMaximum, 1},
        {command_ids::LayerSmartObjectsStackModeMean, 3},
        {command_ids::LayerSmartObjectsStackModeMedian, 4},
        {command_ids::LayerSmartObjectsStackModeMinimum, 2},
        {command_ids::LayerSmartObjectsStackModeRange, 6},
        {command_ids::LayerSmartObjectsStackModeSkewness, 9},
        {command_ids::LayerSmartObjectsStackModeStandardDeviation, 8},
        {command_ids::LayerSmartObjectsStackModeSummation, 5},
        {command_ids::LayerSmartObjectsStackModeVariance, 7},
    };
    for (const StackModeLeaf& leaf : stackModeLeaves) {
        const QString command = QString::fromLatin1(leaf.command);
        const int code = leaf.code;
        commands_->setHandler(command, [this, code]() {
            PictureView* view = activeView();
            const QString path = layerAnchor();
            if (!view || path.isEmpty()) {
                return;
            }
            if (set_smart_object_stack_mode(*view, path, code)) {
                refresh();
            }
        });
        commands_->setEnabledProvider(command, [this, code]() {
            PictureView* view = activeView();
            const QString path = layerAnchor();
            return view && !path.isEmpty()
                && layer_can_set_smart_object_stack_mode(*view, path, code);
        });
        commands_->setCheckedProvider(command, [this, code]() {
            PictureView* view = activeView();
            const QString path = layerAnchor();
            return view && !path.isEmpty()
                && layer_smart_object_stack_mode(*view, path) == code;
        });
    }
}

bool MenuRegistrar::replaceSmartObjectContents(const QString& layerPath)
{
    PictureView* view = activeView();
    if (!view || layerPath.isEmpty()
        || !view->layer_can_replace_smart_object_contents(layerPath)) {
        return false;
    }
    const QString file = getOpenFileName(dialogParent(), tr("Replace Contents"),
                                         documentDirectory(activeFilePath()),
                                         {QStringLiteral("Photoshop (*.PSD *.PSB)")});
    if (file.isEmpty() || !view->replace_smart_object_contents(layerPath, file)) {
        return false;
    }
    return true;
}

bool MenuRegistrar::exportSmartObjectContents(const QString& layerPath)
{
    PictureView* view = activeView();
    if (!view || layerPath.isEmpty()
        || !view->layer_can_export_smart_object_contents(layerPath)) {
        return false;
    }
    QString dest = getSaveFileName(dialogParent(), tr("Export Contents"), activeFilePath(),
                                   {QStringLiteral("Photoshop (*.PSD)")}, nullptr);
    if (dest.isEmpty()) {
        return false;
    }
    if (QFileInfo(dest).suffix().isEmpty()) {
        dest += QStringLiteral(".psd");
    }
    if (!view->export_smart_object_contents(layerPath, dest)) {
        QMessageBox::warning(dialogParent(), tr("Export Contents"),
                             tr("Could not export the smart object contents."));
        return false;
    }
    return true;
}

void MenuRegistrar::wireSmartObjectRowMenu()
{
    if (!panels_ || !panels_->layersPanel()) {
        return;
    }
    connect(panels_->layersPanel(), &LayersPanel::smartObjectContentsRequested, this,
            [this](const QString& action, const QString& path) {
                if (action == QLatin1String("edit")) {
                    if (importSave_) {
                        importSave_->editSmartObjectContents(path);
                    }
                } else if (action == QLatin1String("replace")) {
                    replaceSmartObjectContents(path);
                } else if (action == QLatin1String("export")) {
                    exportSmartObjectContents(path);
                }
            });
}

void MenuRegistrar::wireSmartFilterActions()
{
    // Clear Smart Filters removes every filter on the current smart-object
    // layer, recording one state. Enabled only when the current layer has one.
    const auto currentSmartFilterPath = [this]() -> QString {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        if (!view || path.isEmpty() || layer_smart_filter_count(*view, path) <= 0) {
            return QString();
        }
        return path;
    };
    commands_->setHandler(command_ids::LayerSmartFiltersClear, [this, currentSmartFilterPath]() {
        if (PictureView* view = activeView()) {
            const QString path = currentSmartFilterPath();
            if (!path.isEmpty()) {
                clear_layer_smart_filters(*view, path);
            }
        }
    });
    commands_->setEnabledProvider(command_ids::LayerSmartFiltersClear, [currentSmartFilterPath]() {
        return !currentSmartFilterPath().isEmpty();
    });
}

} // namespace pictura
