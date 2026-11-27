// Edit > Transform: Free Transform, the constrained Scale/Rotate and Skew/
// Distort/Perspective sessions, the exact rotations and flips, and Transform
// Again. Lifted out of registerHandlers() to keep menu_registrar.cpp within its
// size budget.

#include "menu_registrar.h"

#include "commands.h"

#include "menu_registrar_internal.h"

#include "document_registry.h"
#include "panel_host.h"
#include "tools.h"

#include "transform_commit.h"
#include "puppet_warp_dialog.h"

#include "api/paint_bridge.h"
#include "api/transform_bridge.h"

namespace pictura {

void MenuRegistrar::wireTransformMenu()
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

    const auto puppetTargetable = [this, modalSession]() {
        PictureView* view = activeView();
        return view && !modalSession() && puppet_warp_available(*view);
    };
    commands_->setEnabledProvider(command_ids::EditPuppetWarp, puppetTargetable);
    commands_->setHandler(command_ids::EditPuppetWarp, [this, puppetTargetable]() {
        PictureView* view = activeView();
        if (view && puppetTargetable() && runPuppetWarpDialog(dialogParent(), *view)) {
            refresh();
        }
    });

    commands_->setHandler(command_ids::EditFreeTransform, [this, transformTarget]() {
        // A pending border transform is not silently replaced: Apply / Don't
        // Apply / Cancel decides, and Cancel keeps the session.
        if (tools_ && tools_->transformPending() && tools_->transformSelectionTarget()
            && !tools_->resolvePendingTransform()) {
            return;
        }
        if (beginFreeTransform(transformTarget())) {
            refresh();
        }
    });
    commands_->setEnabledProvider(command_ids::EditFreeTransform, transformTargetable);

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
        // An open session switches mode and keeps its pending transform, as
        // CS6 does (#1297), instead of starting over.
        commands_->setHandler(id, [this, mode, transformTarget]() {
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
        commands_->setEnabledProvider(id, [this, mode, transformTarget, modalSession]() {
            PictureView* view = activeView();
            if (view && modalSession()) {
                return transform_can_set_mode(*view, mode);
            }
            const QString path = transformTarget();
            return view && view->has_document() && !path.isEmpty()
                && layer_can_transform_mode(*view, path, mode);
        });
    }

    commands_->setHandler(command_ids::EditTransformWarp, [this, transformTarget]() {
        if (tools_ && tools_->beginWarp(transformTarget())) {
            refresh();
        }
    });
    commands_->setEnabledProvider(
        command_ids::EditTransformWarp,
        [transformTargetable, modalSession]() {
            // An open session keeps its own target and enters Warp in place.
            return modalSession() || transformTargetable();
        });

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
        commands_->setHandler(id, [this, mode, transformTarget]() {
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
        commands_->setEnabledProvider(id, [this, mode, transformTargetable, modalSession]() {
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
        commands_->setHandler(id, [this, id, op, label, transformTarget]() {
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
                commands_->recordCommand(id, command);
            }
        });
        commands_->setEnabledProvider(
            id, [transformTargetable, modalSession]() { return modalSession() || transformTargetable(); });
        commands_->setExecutor(id, [this, op, label, transformTarget](const CommandParams&) {
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
            const bool applied = runTransformCommit(dialogParent(), *view);
            if (!applied && view->transform_session_active()) {
                // Cancelled: this one-shot command opened the session itself.
                view->cancel_transform();
            }
            if (ImageView* canvas = imageView()) {
                if (applied) {
                    canvas->endTransformOverlay();
                } else {
                    canvas->clearTransformPreview();
                }
            }
            if (applied) {
                refresh();
            } else if (edit_refused_for_memory()) {
                showStatus(transformMemoryRefusalMessage(), 6000);
            }
            return applied;
        });
    }

    commands_->setHandler(command_ids::EditTransformAgain, [this, transformTarget]() {
        PictureView* view = activeView();
        if (!view || !transform_has_repeat(*view)) {
            return;
        }
        if (transform_last_target(*view) == QLatin1String("selection")) {
            // Repeat on the current selection border, not on a layer.
            if (!view->has_selection() || !tools_ || !tools_->beginTransformSelection()) {
                return;
            }
        } else if (!beginTransformMode(transformTarget(), QString())) {
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
    commands_->setEnabledProvider(
        command_ids::EditTransformAgain,
        [this, transformTargetable, modalSession]() {
            PictureView* view = activeView();
            if (!view || modalSession() || !transform_has_repeat(*view)) {
                return false;
            }
            if (transform_last_target(*view) == QLatin1String("selection")) {
                return view->selection_count() > 0;
            }
            return transformTargetable();
        });
}

} // namespace pictura
