#pragma once

#include <QtCore/QHash>
#include <QtCore/QJsonObject>
#include <QtCore/QList>
#include <QtCore/QObject>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtGui/QKeySequence>

#include <functional>
#include <vector>

#include "command_ids.h"

class QAction;
class QMenuBar;

namespace pictura {

// Stable command identifiers live in the generated command_ids.h; command ids
// are the dispatch key and the key for future localization and custom menu
// sets, so never derive them from labels.

// One entry of the command table. An entry with an empty `id` is a separator.
struct CommandSpec {
    QString id;
    QStringList path;                 // top-level menu first, leaf last
    QString label;
    QKeySequence shortcut;
    bool implemented = false;
    bool checkable = false;
};

// A read-only snapshot of one command, for `list_commands` and any inspection
// surface. Separators are excluded.
struct CommandInfo {
    QString id;
    QStringList path;
    QString label;
    bool implemented = false;
    bool enabled = false;
    bool checked = false;
};

// The parameter values one command executed with. A parameterless command
// carries an empty object, which is still a recordable step.
struct CommandParams {
    QJsonObject values;
};

enum class CommandMode {
    Ui,
    Replay,
    Modal,
};

struct CommandContext {
    CommandMode mode = CommandMode::Ui;
    CommandParams params;
    // Modal replay only: the handler writes the values the user confirmed and
    // sets `applied` true on OK. Leave `applied` false on Cancel.
    CommandParams* outParams = nullptr;
    bool* applied = nullptr;
};

using CommandExecutor = std::function<bool(const CommandParams&)>;
using CommandRecordingSink = std::function<void(const QString&, const CommandParams&)>;

// Declarative command table plus menu construction and dispatch. The registry
// owns no application behavior: the frame supplies handlers, providers, and
// panel-facing actions by id.
class CommandRegistry : public QObject {
    Q_OBJECT

public:
    explicit CommandRegistry(QObject* parent = nullptr);

    void add(const CommandSpec& spec);
    void add(const QString& id, const QStringList& path, const QString& label,
             const QKeySequence& shortcut = QKeySequence(), bool implemented = false);
    void addSeparator(const QStringList& path);

    void setHandler(const QString& id, std::function<void()> handler);

    // The mode-aware UI handler: it receives the dispatch mode and any recorded
    // parameters, so a modal replay can prefill the command's dialog.
    void setContextHandler(const QString& id, std::function<void(const CommandContext&)> handler);

    // Replay path: the executor runs the command with recorded values and
    // returns whether it succeeded. No UI opens unless mode is Modal.
    void setExecutor(const QString& id, CommandExecutor executor);

    // The active recorder, or empty when recording is off. recordCommand()
    // forwards a successful mutation to it; callers must not record a refused
    // or failed command.
    void setRecordingSink(CommandRecordingSink sink);
    void recordCommand(const QString& id, const CommandParams& params);

    // Execute a recorded command without its normal UI. Returns false and
    // names the command in lastError() when no executor is registered.
    bool executeRecorded(const QString& id, const CommandParams& params,
                         CommandMode mode = CommandMode::Replay);
    // Modal replay variant: a context handler may write the edited values back
    // through `params`; a cancelled dialog returns false.
    bool executeRecorded(const QString& id, CommandParams* params, CommandMode mode);
    QString lastError() const { return lastError_; }

    // Mark an already-added command as implemented (used to light up leaves
    // registered as stubs once their handler is wired).
    void setImplemented(const QString& id, bool implemented);

    // Extra enablement predicate for `id`; combined with `implemented` and the
    // presence of a handler.
    void setEnabledProvider(const QString& id, std::function<bool()> enabled);
    // A gate over every command: a command it reports blocked is disabled
    // whatever its own provider says (a pending transform locks the Layer menu).
    void setBlockedProvider(std::function<bool(const CommandSpec&)> blocked);
    // Checked-state provider for a checkable command.
    void setCheckedProvider(const QString& id, std::function<bool()> checked);
    // Dynamic label for `id`; overrides the static label when set.
    void setLabelProvider(const QString& id, std::function<QString()> label);

    QAction* action(const QString& id) const;

    // The command's current and built-in key bindings (Edit > Keyboard
    // Shortcuts may give a command several, or none). setShortcuts() also
    // updates the live action.
    QList<QKeySequence> shortcuts(const QString& id) const;
    QList<QKeySequence> defaultShortcuts(const QString& id) const;
    void setShortcuts(const QString& id, const QList<QKeySequence>& shortcuts);
    // The command holding `shortcut`, or an empty id when none does.
    QString commandForShortcut(const QKeySequence& shortcut) const;
    // Every command whose bindings differ from its defaults.
    QHash<QString, QList<QKeySequence>> shortcutOverrides() const;

    // Build (or rebuild) `menuBar` from the command table.
    void buildMenuBar(QMenuBar* menuBar);

    // Re-evaluate enabled/checked/label for every action.
    void refresh();

    // How many times refresh() has run. Test-only probe so a self-test can
    // prove the paint region path does not fan out per dab.
    int refreshCount() const { return refreshCount_; }

    // Run the handler for `id`; returns false and does nothing when none is set.
    bool dispatch(const QString& id);

    // Titles of the top-level menus in creation order.
    QStringList topLevelTitles() const;

    // Snapshot of the command table (separators excluded) with each command's
    // implemented/enabled/checked state.
    QList<CommandInfo> describe() const;

Q_SIGNALS:
    // A command's bindings changed through setShortcuts().
    void shortcutsChanged(const QString& id);

private:
    bool hasHandler(const QString& id) const;
    bool executeRecordedImpl(const QString& id, const CommandParams& params, CommandMode mode,
                             CommandParams* editedOut);

    struct Entry {
        CommandSpec spec;
        QAction* action = nullptr;
        QList<QKeySequence> shortcuts;
        QList<QKeySequence> defaults;
    };
    const Entry* entry(const QString& id) const;
    std::vector<Entry> entries_;
    QHash<QString, std::function<void()>> handlers_;
    QHash<QString, std::function<void(const CommandContext&)>> contextHandlers_;
    QHash<QString, CommandExecutor> executors_;
    CommandRecordingSink recordingSink_;
    QString lastError_;
    QHash<QString, std::function<bool()>> enabledProviders_;
    std::function<bool(const CommandSpec&)> blockedProvider_;
    QHash<QString, std::function<bool()>> checkedProviders_;
    QHash<QString, std::function<QString()>> labelProviders_;
    QMenuBar* menuBar_ = nullptr;
    int refreshCount_ = 0;
};

// Populate `registry` from the Rust command catalogue
// (`pictura_script::command_catalogue`), inserting the C++ menu-layout
// separators before the command rows they precede.
void addDefaultCommands(CommandRegistry& registry);

// The stable id derived from a command path's segments. Delegates to the Rust
// catalogue so path-derived stubs and handlers always agree with it.
QString commandIdForPath(const QStringList& path);

} // namespace pictura
