// While a transform is pending (a modal Free Transform, Transform submenu, or
// Warp session not yet committed), Photoshop refuses layer edits and asks
// before a tool switch drops the transform (#750): every Layer menu command is
// disabled, the Layers panel is greyed, and switching tools asks "Apply the
// transformation?". The Move tool's controls box commits each gesture on
// release, so it never locks.

#include "frame_includes.h"

namespace pictura {

void PicturaMainWindow::installTransformLock()
{
    if (!tools_ || !registry_) {
        return;
    }
    registry_->setBlockedProvider([this](const CommandSpec& spec) {
        return transformLocked_ && !spec.path.isEmpty()
            && spec.path.first() == QLatin1String("Layer");
    });
    tools_->setTransformDecisionPrompt([this]() { return askApplyTransform(this); });
    connect(tools_, &ToolController::transformChanged, this, [this]() { syncTransformLock(); });
}

void PicturaMainWindow::syncTransformLock()
{
    const bool locked = tools_ && tools_->transformPending();
    if (locked == transformLocked_) {
        return;
    }
    transformLocked_ = locked;
    if (layersPanel_) {
        layersPanel_->setEnabled(!locked);
    }
    if (registry_) {
        registry_->refresh();
    }
}

} // namespace pictura
