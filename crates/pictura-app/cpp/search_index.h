#pragma once

// The Help > Search (Shift+Space) index: every UI entry point a user can look
// up by name, with the place it lives. Not a CS6 feature (#788); post-CS6 by
// owner decision. Free of widgets so the Qt Test drives it directly.

#include <QtCore/QList>
#include <QtCore/QMetaType>
#include <QtCore/QString>
#include <QtCore/QStringList>

namespace pictura {

class CommandRegistry;

struct SearchEntry {
    enum class Kind { Command, Tool, Panel, Preference, Control };
    Kind kind = Kind::Command;
    // The target: a command id, a tool name (`toolIdName`), a panel object
    // name, a Preferences page, or a control object name.
    QString id;
    // Where the entry lives, the label last ("Edit", "Transform", "Warp").
    QStringList path;
    QStringList keywords;
    // The command behind a menu-derived entry; its menu chain is what reveal
    // opens and its binding is the shortcut shown.
    QString command;
    QString shortcut;
    // A control's panel (or "optionsBar") and the tool whose options page
    // holds it; empty when none.
    QString container;
    QString tool;
    bool implemented = true;
    bool runnable = false;

    QString label() const { return path.isEmpty() ? QString() : path.last(); }
};

struct SearchHit {
    int index = -1; // into the searched list
    int tier = 0;   // 0 exact label ... 4 path or keyword only
};

// One row of the curated control table (search_controls.cpp). `path` is a
// " > "-separated breadcrumb, the control's label last.
struct SearchControl {
    const char* container;
    const char* control;
    const char* tool;
    const char* path;
    const char* keywords;
};
const QList<SearchControl>& searchControls();

// A menu label without mnemonic ampersands or a trailing ellipsis.
QString cleanSearchLabel(const QString& label);

// Commands (with live labels and enabled state), panels, Preferences pages,
// tools, and the curated controls. Help > Search itself is left out.
QList<SearchEntry> buildSearchIndex(const CommandRegistry& registry);

// The entries matching every word of `query`, best first; at most `limit`.
QList<SearchHit> searchIndex(const QList<SearchEntry>& entries, const QString& query,
                             int limit = 50);

} // namespace pictura

Q_DECLARE_METATYPE(pictura::SearchEntry)
