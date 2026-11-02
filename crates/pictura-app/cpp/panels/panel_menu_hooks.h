#pragma once

#include <QtCore/QHash>
#include <QtCore/QString>

#include <functional>

class QMenu;

namespace pictura {

// A panel's dynamic per-widget menu hook, keyed by the panel's object name so
// it follows the panel when a regroup replaces the group that carried it.
using PanelMenuHook = std::function<void(QMenu*)>;

class PanelMenuHookStore {
public:
    void setPanelMenuHook(const QString& panelName, PanelMenuHook hook)
    {
        if (hook) {
            panelMenuHooks_.insert(panelName, hook);
        } else {
            panelMenuHooks_.remove(panelName);
        }
    }

    PanelMenuHook panelMenuHook(const QString& panelName) const
    {
        return panelMenuHooks_.value(panelName);
    }

private:
    QHash<QString, PanelMenuHook> panelMenuHooks_;
};

} // namespace pictura
