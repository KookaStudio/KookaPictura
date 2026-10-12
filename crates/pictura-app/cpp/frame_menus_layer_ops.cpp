// Layer > Arrange / Reverse / Merge Down / Delete Layer, the non-destructive
// Stamp Visible / Stamp Selected, and the Lock Layers / Lock All Layers In
// Group commands, over the Layers panel selection and the active layer.

#include "frame_includes.h"

#include "api/layers_api.h"

#include "pictura_app/src/cxxqt_object/layer_arrange.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layers_surface.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/vector_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/path_list.cxxqt.h"
#include "pictura_app/src/cxxqt_object/shapes.cxxqt.h"

namespace pictura {

void PicturaMainWindow::wireLayerArrangeStamp()
{
    // Layer > Arrange and Reverse: within-container ordering of the selection.
    const auto enableArrange = [this](const char* id, int op) {
        registry_->setHandler(id, [this, op]() {
            PictureView* view = activeView();
            const QString path = layersPanel_ ? layersPanel_->currentPath() : QString();
            if (view && !path.isEmpty() && arrange_layer_run(*view, path, op)) {
                refresh();
            }
        });
        registry_->setEnabledProvider(id, [this, op]() {
            PictureView* view = activeView();
            const QString path = layersPanel_ ? layersPanel_->currentPath() : QString();
            return view && view->has_document() && !path.isEmpty()
                && arrange_layer_can(*view, path, op);
        });
    };
    enableArrange(command_ids::LayerArrangeFront, 0);
    enableArrange(command_ids::LayerArrangeForward, 1);
    enableArrange(command_ids::LayerArrangeBackward, 2);
    enableArrange(command_ids::LayerArrangeBack, 3);

    registry_->setHandler(command_ids::LayerArrangeReverse, [this]() {
        PictureView* view = activeView();
        const QStringList paths = layersPanel_ ? layersPanel_->selectedPaths() : QStringList{};
        if (view && paths.size() >= 2 && reverse_layers_run(*view, paths)) {
            refresh();
        }
    });
    registry_->setEnabledProvider(command_ids::LayerArrangeReverse, [this]() {
        PictureView* view = activeView();
        return view && view->has_document() && layersPanel_
            && layersPanel_->selectedPaths().size() >= 2;
    });

    // Layer > Merge Down: the active layer and the layer directly below it.
    registry_->setHandler(command_ids::LayerMergeDown, [this]() {
        PictureView* view = activeView();
        const QString path = layersPanel_ ? layersPanel_->currentPath() : QString();
        if (view && !path.isEmpty() && merge_down_run(*view, path) > 0) {
            refresh();
        }
    });
    registry_->setEnabledProvider(command_ids::LayerMergeDown, [this]() {
        PictureView* view = activeView();
        const QString path = layersPanel_ ? layersPanel_->currentPath() : QString();
        return view && view->has_document() && !path.isEmpty()
            && view->can_merge_layers(QStringList{path});
    });

    // Layer > Delete Layer: the panel selection.
    registry_->setHandler(command_ids::LayerDeleteLayer, [this]() {
        PictureView* view = activeView();
        const QStringList paths = layersPanel_ ? layersPanel_->selectedPaths() : QStringList{};
        if (view && !paths.isEmpty() && view->delete_layers(paths) > 0) {
            refresh();
        }
    });
    registry_->setEnabledProvider(command_ids::LayerDeleteLayer, [this]() {
        PictureView* view = activeView();
        return view && view->has_document() && layersPanel_
            && !layersPanel_->selectedPaths().isEmpty();
    });

    // Stamp Visible / Stamp Selected: composite into a new layer, non-destructive.
    const auto currentLayerVisible = [this]() {
        PictureView* view = activeView();
        const QString path = layersPanel_ ? layersPanel_->currentPath() : QString();
        if (!view || !view->has_document() || path.isEmpty()) {
            return false;
        }
        const auto row = pictura::layers_api::rowForPath(*view, path);
        return row.path == path && row.visible;
    };
    registry_->setHandler(command_ids::LayerStampVisible, [this]() {
        PictureView* view = activeView();
        const QString path = layersPanel_ ? layersPanel_->currentPath() : QString();
        if (view && !path.isEmpty() && !stamp_visible_run(*view, path).isEmpty()) {
            refresh();
        }
    });
    registry_->setEnabledProvider(command_ids::LayerStampVisible, [this, currentLayerVisible]() {
        PictureView* view = activeView();
        return view && view->has_document() && currentLayerVisible();
    });
    registry_->setHandler(command_ids::LayerStampSelected, [this]() {
        PictureView* view = activeView();
        const QStringList paths = layersPanel_ ? layersPanel_->selectedPaths() : QStringList{};
        const QString active = layersPanel_ ? layersPanel_->currentPath() : QString();
        if (view && !paths.isEmpty() && !active.isEmpty()
            && !stamp_selected_run(*view, paths, active).isEmpty()) {
            refresh();
        }
    });
    registry_->setEnabledProvider(command_ids::LayerStampSelected, [this]() {
        PictureView* view = activeView();
        return view && view->has_document() && layersPanel_
            && !layersPanel_->selectedPaths().isEmpty()
            && !layersPanel_->currentPath().isEmpty();
    });

    // Layer > Lock Layers: apply one lock flag to the panel selection, one state.
    const struct {
        const char* id;
        const char* flag;
    } kLockCommands[] = {
        {command_ids::LayerLockLayersAll, "all"},
        {command_ids::LayerLockLayersTransparency, "transparency"},
        {command_ids::LayerLockLayersImage, "pixels"},
        {command_ids::LayerLockLayersPosition, "position"},
    };
    for (const auto& entry : kLockCommands) {
        const QString flag = QString::fromLatin1(entry.flag);
        registry_->setHandler(entry.id, [this, flag]() {
            PictureView* view = activeView();
            const QStringList paths = layersPanel_ ? layersPanel_->selectedPaths() : QStringList{};
            if (view && !paths.isEmpty() && view->set_layers_lock(paths, flag, true) > 0) {
                refresh();
            }
        });
        registry_->setEnabledProvider(entry.id, [this]() {
            PictureView* view = activeView();
            return view && view->has_document() && layersPanel_
                && !layersPanel_->selectedPaths().isEmpty();
        });
    }

    // Layer > Lock All Layers In Group…: the group holding the current row — the
    // row itself when it is a group, else its parent group.
    const auto groupPathForCurrent = [this]() -> QString {
        PictureView* view = activeView();
        const QString current = layersPanel_ ? layersPanel_->currentPath() : QString();
        if (!view || current.isEmpty()) {
            return {};
        }
        const auto kindOf = [view](const QString& path) {
            return pictura::layers_api::rowForPath(*view, path).kind;
        };
        if (kindOf(current) == QLatin1String("group")) {
            return current;
        }
        const int slash = current.lastIndexOf(QLatin1Char('/'));
        if (slash < 0) {
            return {};
        }
        const QString parent = current.left(slash);
        return kindOf(parent) == QLatin1String("group") ? parent : QString();
    };
    registry_->setHandler(command_ids::LayerLockAllInGroup, [this, groupPathForCurrent]() {
        PictureView* view = activeView();
        const QString group = groupPathForCurrent();
        if (view && !group.isEmpty() && lock_group_layers_run(*view, group) > 0) {
            refresh();
        }
    });
    registry_->setEnabledProvider(command_ids::LayerLockAllInGroup,
                                  [this, groupPathForCurrent]() {
        PictureView* view = activeView();
        return view && view->has_document() && !groupPathForCurrent().isEmpty();
    });
}

void PicturaMainWindow::wireShapeLayerActions()
{
    const auto currentShapePath = [this]() -> QString {
        PictureView* view = activeView();
        const QString path = layersPanel_ ? layersPanel_->currentPath() : QString();
        if (!view || path.isEmpty() || !shape_is_shape(*view, path)) {
            return QString();
        }
        return path;
    };
    registry_->setHandler(command_ids::LayerRasterizeShape, [this, currentShapePath]() {
        PictureView* view = activeView();
        if (view) {
            const QString path = currentShapePath();
            if (!path.isEmpty() && shape_rasterize(*view, path)) {
                refresh();
            }
        }
    });
    registry_->setEnabledProvider(command_ids::LayerRasterizeShape,
                                  [currentShapePath]() { return !currentShapePath().isEmpty(); });
    registry_->setHandler(command_ids::LayerCopyShapeAttributes, [this, currentShapePath]() {
        PictureView* view = activeView();
        if (view) {
            const QString path = currentShapePath();
            if (!path.isEmpty()) {
                shape_copy_attributes(*view, path);
            }
        }
    });
    registry_->setEnabledProvider(command_ids::LayerCopyShapeAttributes,
                                  [currentShapePath]() { return !currentShapePath().isEmpty(); });
    registry_->setHandler(command_ids::LayerPasteShapeAttributes, [this, currentShapePath]() {
        PictureView* view = activeView();
        if (view) {
            const QString path = currentShapePath();
            if (!path.isEmpty() && shape_paste_attributes(*view, path)) {
                refresh();
            }
        }
    });
    registry_->setEnabledProvider(command_ids::LayerPasteShapeAttributes,
                                  [this, currentShapePath]() {
                                      PictureView* view = activeView();
                                      if (!view || currentShapePath().isEmpty()) {
                                          return false;
                                      }
                                      return shape_can_paste_attributes(*view);
                                  });
}

void PicturaMainWindow::wireVectorMaskActions()
{
    const auto whenNoVectorMask = [this](const char* id) {
        registry_->setEnabledProvider(id, [this]() {
            PictureView* view = activeView();
            return view && view->has_document() && !vector_mask_present(*view);
        });
    };
    const auto whenVectorMask = [this](const char* id) {
        registry_->setEnabledProvider(id, [this]() {
            PictureView* view = activeView();
            return view && view->has_document() && vector_mask_present(*view);
        });
    };
    const auto addVectorMask = [this](const QString& kind) {
        PictureView* view = activeView();
        if (view && vector_mask_add(*view, kind)) {
            refresh();
        }
    };
    registry_->setHandler(command_ids::LayerVectorMaskRevealAll,
                          [addVectorMask]() { addVectorMask(QStringLiteral("reveal-all")); });
    registry_->setHandler(command_ids::LayerVectorMaskHideAll,
                          [addVectorMask]() { addVectorMask(QStringLiteral("hide-all")); });
    registry_->setHandler(command_ids::LayerVectorMaskCurrentPath,
                          [addVectorMask]() { addVectorMask(QStringLiteral("current-path")); });
    whenNoVectorMask(command_ids::LayerVectorMaskRevealAll);
    whenNoVectorMask(command_ids::LayerVectorMaskHideAll);
    registry_->setEnabledProvider(command_ids::LayerVectorMaskCurrentPath, [this]() {
        PictureView* view = activeView();
        return view && view->has_document() && !vector_mask_present(*view)
            && paths_has_work_path(*view);
    });

    const auto simpleVectorAction = [this, whenVectorMask](const char* id, auto action) {
        registry_->setHandler(id, [this, action]() {
            PictureView* view = activeView();
            if (view && action(*view)) {
                refresh();
            }
        });
        whenVectorMask(id);
    };
    simpleVectorAction(command_ids::LayerVectorMaskDelete,
                       [](PictureView& view) { return vector_mask_delete(view); });
    simpleVectorAction(command_ids::LayerVectorMaskEnable,
                       [](PictureView& view) { return vector_mask_set_enabled(view, true); });
    simpleVectorAction(command_ids::LayerVectorMaskDisable,
                       [](PictureView& view) { return vector_mask_set_enabled(view, false); });
    simpleVectorAction(command_ids::LayerVectorMaskLink,
                       [](PictureView& view) { return vector_mask_set_linked(view, true); });
    simpleVectorAction(command_ids::LayerVectorMaskUnlink,
                       [](PictureView& view) { return vector_mask_set_linked(view, false); });
    simpleVectorAction(command_ids::LayerRasterizeVectorMask,
                       [](PictureView& view) { return vector_mask_rasterize(view); });
}

} // namespace pictura
