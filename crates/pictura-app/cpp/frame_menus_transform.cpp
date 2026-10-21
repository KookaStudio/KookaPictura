// Edit > Transform beyond Free Transform: constrained Scale/Rotate sessions,
// the exact rotations and flips, and Transform Again. Lifted out of
// registerHandlers() to keep frame_menus.cpp within its size budget.

#include "frame_includes.h"

namespace pictura {

void PicturaMainWindow::wireTransformMenu()
{
    const auto transformTarget = [this]() {
        return (layersPanel_ && layersPanel_->selectedPaths().size() == 1)
                   ? layersPanel_->currentPath()
                   : QString();
    };
    const auto transformTargetable = [this, transformTarget]() {
        PictureView* view = activeView();
        const QString path = transformTarget();
        return view && view->has_document() && !path.isEmpty()
            && view->layer_can_free_transform(path);
    };
    const auto modalSession = [this]() {
        return tools_ && tools_->transformPending();
    };

    const struct {
        const char* id;
        const char* mode;
    } kConstrainedModes[] = {
        {command_ids::EditTransformScale, "scale"},
        {command_ids::EditTransformRotate, "rotate"},
    };
    for (const auto& entry : kConstrainedModes) {
        const QString id = QString::fromLatin1(entry.id);
        const QString mode = QString::fromLatin1(entry.mode);
        registry_->setHandler(id, [this, mode, transformTarget]() {
            PictureView* view = activeView();
            if (!view) {
                return;
            }
            if (tools_ && tools_->transformSessionActive() && !tools_->transformControlsSession()) {
                if (tools_->setTransformMode(mode)) {
                    refresh();
                }
                return;
            }
            if (beginTransformMode(transformTarget(), mode)) {
                refresh();
            }
        });
        registry_->setEnabledProvider(
            id, [transformTargetable, modalSession]() { return modalSession() || transformTargetable(); });
    }

    const struct {
        const char* id;
        const char* op;
        const char* label;
    } kExactTransforms[] = {
        {command_ids::EditTransformRotate180, "rotate180", "Rotate 180°"},
        {command_ids::EditTransformRotate90Cw, "rotate90cw", "Rotate 90° Clockwise"},
        {command_ids::EditTransformRotate90Ccw, "rotate90ccw", "Rotate 90° Counter Clockwise"},
        {command_ids::EditTransformFlipHorizontal, "flipHorizontal", "Flip Horizontal"},
        {command_ids::EditTransformFlipVertical, "flipVertical", "Flip Vertical"},
    };
    for (const auto& entry : kExactTransforms) {
        const QString id = QString::fromLatin1(entry.id);
        const QString op = QString::fromLatin1(entry.op);
        const QString label = QString::fromUtf8(entry.label);
        registry_->setHandler(id, [this, op, label, transformTarget]() {
            PictureView* view = activeView();
            if (!view) {
                return;
            }
            const bool open = tools_ && tools_->transformSessionActive()
                && !tools_->transformControlsSession();
            if (!open && !beginTransformMode(transformTarget(), QString())) {
                return;
            }
            if (!tools_ || !tools_->applyTransformOp(op, !open, label)) {
                if (!open && tools_) {
                    tools_->cancelFreeTransform();
                }
                return;
            }
            refresh();
        });
        registry_->setEnabledProvider(
            id, [transformTargetable, modalSession]() { return modalSession() || transformTargetable(); });
    }

    registry_->setHandler(command_ids::EditTransformAgain, [this, transformTarget]() {
        PictureView* view = activeView();
        if (!view || !transform_has_repeat(*view)) {
            return;
        }
        if (!beginTransformMode(transformTarget(), QString())) {
            return;
        }
        if (!tools_ || !tools_->repeatTransform()) {
            if (tools_) {
                tools_->cancelFreeTransform();
            }
            return;
        }
        refresh();
    });
    registry_->setEnabledProvider(command_ids::EditTransformAgain,
                                  [this, transformTargetable, modalSession]() {
                                      PictureView* view = activeView();
                                      return view && !modalSession() && transform_has_repeat(*view)
                                          && transformTargetable();
                                  });
}

} // namespace pictura
