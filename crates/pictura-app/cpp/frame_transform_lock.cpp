// While a transform is pending (a modal Free Transform, Transform submenu, or
// Warp session not yet committed), Photoshop refuses document edits and asks
// before a tool switch drops the transform (#750, #853): the Layer, Image, and
// Filter menus, the Edit pixel commands, and the Layers panel are locked, and
// switching tools asks "Apply the transformation?". The Move tool's controls
// box commits each gesture on release, so it never locks.

#include "frame_includes.h"

namespace pictura {

namespace {

bool locksDocumentEdit(const CommandSpec& spec)
{
    if (spec.path.isEmpty()) {
        return false;
    }
    const QString& top = spec.path.first();
    if (top == QLatin1String("Layer") || top == QLatin1String("Image")
        || top == QLatin1String("Filter")) {
        return true;
    }
    return spec.id == QLatin1String(command_ids::EditAssignProfile)
        || spec.id == QLatin1String(command_ids::EditConvertProfile)
        || spec.id == QLatin1String(command_ids::EditCut)
        || spec.id == QLatin1String(command_ids::EditPaste)
        || spec.id == QLatin1String(command_ids::EditPasteInPlace)
        || spec.id == QLatin1String(command_ids::EditPasteInto)
        || spec.id == QLatin1String(command_ids::EditPasteOutside)
        || spec.id == QLatin1String(command_ids::EditClear)
        || spec.id == QLatin1String(command_ids::EditFill)
        || spec.id == QLatin1String(command_ids::EditStroke);
}

} // namespace

void PicturaMainWindow::installTransformLock()
{
    if (!tools_ || !registry_) {
        return;
    }
    registry_->setBlockedProvider([this](const CommandSpec& spec) {
        return transformLocked_ && locksDocumentEdit(spec);
    });
    tools_->setTransformDecisionPrompt([this]() { return askApplyTransform(this); });
    tools_->setCropDecisionPrompt([this]() { return askCropImage(this); });
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
