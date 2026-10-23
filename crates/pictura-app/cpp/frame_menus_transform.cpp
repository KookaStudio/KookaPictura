// Edit > Transform: Free Transform, the constrained Scale/Rotate and Skew/
// Distort/Perspective sessions, the exact rotations and flips, and Transform
// Again. Lifted out of registerHandlers() to keep frame_menus.cpp within its
// size budget.

#include "frame_includes.h"

#include "pictura_app/src/cxxqt_object/impl_transform/bridge.cxxqt.h"

namespace pictura {

void PicturaMainWindow::wireTransformMenu()
{
    const auto transformTarget = [this]() {
        return selectedLayerPaths().size() == 1 ? layerAnchor() : QString();
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

    registry_->setHandler(command_ids::EditFreeTransform, [this, transformTarget]() {
        if (beginFreeTransform(transformTarget())) {
            refresh();
        }
    });
    registry_->setEnabledProvider(command_ids::EditFreeTransform, transformTargetable);

    const struct {
        const char* id;
        const char* mode;
    } kProjectiveModes[] = {
        {command_ids::EditTransformSkew, "skew"},
        {command_ids::EditTransformDistort, "distort"},
        {command_ids::EditTransformPerspective, "perspective"},
    };
    for (const auto& entry : kProjectiveModes) {
        const QString id = QString::fromLatin1(entry.id);
        const QString mode = QString::fromLatin1(entry.mode);
        registry_->setHandler(id, [this, mode, transformTarget]() {
            if (beginTransformMode(transformTarget(), mode)) {
                refresh();
            }
        });
        registry_->setEnabledProvider(id, [this, mode, transformTarget]() {
            PictureView* view = activeView();
            const QString path = transformTarget();
            return view && view->has_document() && !path.isEmpty()
                && layer_can_transform_mode(*view, path, mode);
        });
    }

    registry_->setHandler(command_ids::EditTransformWarp, [this, transformTarget]() {
        if (tools_ && tools_->beginWarp(transformTarget())) {
            refresh();
        }
    });
    registry_->setEnabledProvider(command_ids::EditTransformWarp, transformTargetable);

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
        registry_->setEnabledProvider(id, [this, mode, transformTargetable, modalSession]() {
            PictureView* view = activeView();
            if (view && modalSession()) {
                return transform_can_set_mode(*view, mode);
            }
            return transformTargetable();
        });
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
        registry_->setHandler(id, [this, id, op, label, transformTarget]() {
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
            if (!open) {
                CommandParams command;
                if (op.startsWith(QStringLiteral("rotate"))) {
                    const int angle = op == QLatin1String("rotate180") ? 180
                                    : op == QLatin1String("rotate90cw") ? 90
                                                                        : -90;
                    command.values.insert(QStringLiteral("angle"), angle);
                } else {
                    command.values.insert(
                        QStringLiteral("axis"),
                        op == QLatin1String("flipHorizontal") ? QStringLiteral("horizontal")
                                                              : QStringLiteral("vertical"));
                }
                registry_->recordCommand(id, command);
            }
        });
        registry_->setEnabledProvider(
            id, [transformTargetable, modalSession]() { return modalSession() || transformTargetable(); });
        registry_->setExecutor(id, [this, op, label, transformTarget](const CommandParams&) {
            PictureView* view = activeView();
            const QString path = transformTarget();
            if (!view || !view->has_document() || path.isEmpty()) {
                return false;
            }
            if (!view->begin_free_transform(path)) {
                return false;
            }
            if (!transform_apply(*view, op)) {
                view->cancel_transform();
                return false;
            }
            transform_set_history_label(*view, label);
            const bool applied = view->commit_transform();
            if (applied) {
                refresh();
            }
            return applied;
        });
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
