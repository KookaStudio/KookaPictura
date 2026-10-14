#pragma once

#include <QtCore/QHash>
#include <QtCore/QList>
#include <QtCore/QStringList>
#include <QtGui/QKeySequence>
#include <QtWidgets/QDialog>

class QComboBox;
class QLabel;
class QPushButton;
class QTreeWidget;
class QTreeWidgetItem;

namespace pictura {

class CommandRegistry;
class ShortcutCaptureField;

// A key the workspace itself owns (frame_build.cpp, frame.cpp): no menu
// command or tool may take it, or the two shortcuts would be ambiguous.
struct WorkspaceKey {
    QKeySequence key;
    const char* use;
};
const QList<WorkspaceKey>& workspaceKeys();

// Edit > Keyboard Shortcuts (docs/02-ui-ux/keyboard-shortcuts.md): the
// Application Menus and Tools shortcut editor. Selecting a row opens a focused
// key field in its Shortcut cell; the pressed chord is pending until Accept
// (or another row is chosen), and a conflict is reported in the info area
// with Accept and Go To Conflict / Undo Changes, as CS6 does. A command may
// hold several bindings, one row each. The dialog edits a working copy; OK
// writes it to the registry and the tool catalogue, Cancel drops it. Adapted
// from photorust's KeyboardShortcutsDialog.
// Conflicts are checked across both lists: a tool letter also owns its
// Shift+letter (the cycle key), so a menu command taking either removes the
// letter from that tool group, and the reverse.
// ponytail: one implicit user set over the defaults; Panel Menus, named sets,
// Summarize, legacy channel keys, and the Menus tab are shown disabled.
class KeyboardShortcutsDialog : public QDialog {
    Q_OBJECT

public:
    enum class Scope { ApplicationMenus, Tools };

    explicit KeyboardShortcutsDialog(CommandRegistry* registry, QWidget* parent = nullptr);

    // Test hooks. A row key is a command id (Application Menus) or a tool
    // name such as "move" (Tools); `binding` picks one of a command's rows.
    void setScopeForTest(Scope scope);
    bool selectRowForTest(const QString& key, int binding = 0);
    QString selectedRowForTest() const;
    // The key's bindings as portable text, one per row (a blank row is "").
    QStringList rowShortcutsForTest(const QString& key) const;
    QString rowShortcutForTest(const QString& key) const;
    QWidget* captureFieldForTest() const;
    QPushButton* buttonForTest(const QString& objectName) const;
    QString infoTextForTest() const;

private:
    struct Snapshot {
        QHash<QString, QList<QKeySequence>> commands;
        QHash<int, QChar> tools;
    };
    struct Pending {
        QString key;
        int binding = 0;
        QKeySequence value;
        bool dirty = false;
        // The first owner shown in the notice (a command id or a tool name)
        // and the list it is in; every owner that loses the chord.
        QString conflictKey;
        Scope conflictScope = Scope::ApplicationMenus;
        QStringList losingCommands;
        QList<int> losingTools;
    };

    void buildRows();
    void selectRow(QTreeWidgetItem* item);
    void openEditor(QTreeWidgetItem* item);
    QTreeWidgetItem* rowFor(const QString& key, int binding) const;
    QString rowKey(const QTreeWidgetItem* item) const;
    int rowBinding(const QTreeWidgetItem* item) const;
    QStringList bindingTexts(const QString& key) const;
    void refreshButtons();
    void showInfo();
    // The notice area's text, with the warning icon or the info icon.
    void setNotice(const QString& text, bool warning);
    void capture(const QKeySequence& shortcut);
    void findConflict();
    bool commitPending();
    void discardPending();
    void acceptPending(bool goToConflict);
    void reselect(const QString& key, int binding);
    void pushUndo();
    void undo();
    void useDefault();
    void addShortcut();
    void deleteShortcut();
    void applyAndAccept();

    CommandRegistry* registry_ = nullptr;
    Scope scope_ = Scope::ApplicationMenus;
    Snapshot working_;
    QList<Snapshot> undoStack_;
    Pending pending_;
    // A row added by Add Shortcut that holds no binding yet.
    QString addingKey_;

    QComboBox* scopeCombo_ = nullptr;
    QTreeWidget* tree_ = nullptr;
    ShortcutCaptureField* capture_ = nullptr;
    QLabel* infoIcon_ = nullptr;
    QLabel* infoText_ = nullptr;
    QPushButton* acceptButton_ = nullptr;
    QPushButton* undoButton_ = nullptr;
    QPushButton* defaultButton_ = nullptr;
    QPushButton* addButton_ = nullptr;
    QPushButton* deleteButton_ = nullptr;
    QPushButton* goToConflictButton_ = nullptr;
    QPushButton* undoChangesButton_ = nullptr;
};

} // namespace pictura
