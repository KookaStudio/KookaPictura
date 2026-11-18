// The toning tools' shared controller behavior: the Dodge/Burn Airbrush
// toggle and the painting-mode quick keys. The document availability gate
// lives in tool_catalog.cpp with the other per-document rules. Part of
// ToolController; split from tools.cpp to keep that file under its size cap.

#include "tools.h"

namespace pictura {

bool ToolController::toggleAirbrush()
{
    if (active_ == ToolId::Brush) {
        paint_->setAirbrush(!paint_->airbrush());
        return true;
    }
    if (active_ == ToolId::Dodge || active_ == ToolId::Burn) {
        ToneGroup* group = optionsFor<ToneGroup>(active_);
        if (!group) {
            return false;
        }
        ToneOptions o = group->get();
        o.airbrush = !o.airbrush;
        group->set(o);
        emit toneOptionsChanged();
        return true;
    }
    StampGroup* group = optionsFor<StampGroup>(active_);
    if (!group) {
        return false;
    }
    StampOptions o = group->get();
    bool* flag = nullptr;
    if (active_ == ToolId::CloneStamp) {
        flag = &o.cloneAirbrush;
    } else if (active_ == ToolId::PatternStamp) {
        flag = &o.patternAirbrush;
    } else if (active_ == ToolId::HistoryBrush) {
        flag = &o.historyAirbrush;
    }
    if (!flag) {
        return false;
    }
    *flag = !*flag;
    group->set(o);
    return true;
}

bool ToolController::applyToneQuickKey(int key)
{
    ToneGroup* group = optionsFor<ToneGroup>(active_);
    if (!group) {
        return false;
    }
    ToneOptions o = group->get();
    switch (active_) {
    case ToolId::Dodge:
    case ToolId::Burn:
        switch (key) {
        case Qt::Key_S: o.range = 0; break;
        case Qt::Key_M: o.range = 1; break;
        case Qt::Key_H: o.range = 2; break;
        default: return false;
        }
        break;
    case ToolId::Sponge:
        switch (key) {
        case Qt::Key_S: o.spongeMode = 1; break;
        case Qt::Key_D: o.spongeMode = 0; break;
        default: return false;
        }
        break;
    default:
        return false;
    }
    group->set(o);
    emit toneOptionsChanged();
    return true;
}

} // namespace pictura
