// Edit > Keyboard Shortcuts (ui/keyboard-shortcuts-editor): the menu and tool
// listings; the focused key field, Accept, and CS6's inline conflict notice;
// several bindings per command; Undo / Use Default / Add / Delete; OK vs
// Cancel; and the persisted overrides.

#include <QtTest/QtTest>

#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtGui/QShortcut>
#include <QtWidgets/QApplication>
#include <QtWidgets/QLabel>
#include <QtWidgets/QPushButton>

#include <memory>

#include "commands.h"
#include "frame.h"
#include "keyboard_shortcuts_dialog.h"
#include "session.h"
#include "theme.h"
#include "tools.h"

#include "qt_test_support.h"

using pictura::KeyboardShortcutsDialog;
using Scope = KeyboardShortcutsDialog::Scope;

namespace {

const QString kNew = QString::fromLatin1(pictura::command_ids::FileNew);
const QString kOpen = QString::fromLatin1(pictura::command_ids::FileOpen);

std::unique_ptr<pictura::CommandRegistry> makeRegistry()
{
    auto registry = std::make_unique<pictura::CommandRegistry>();
    pictura::addDefaultCommands(*registry);
    return registry;
}

// Presses `key` in the selected row's key field.
void type(KeyboardShortcutsDialog& dialog, Qt::Key key, Qt::KeyboardModifiers mods = {})
{
    QWidget* field = dialog.captureFieldForTest();
    QVERIFY(field);
    QTest::keyClick(field, key, mods);
}

void click(KeyboardShortcutsDialog& dialog, const char* button)
{
    QPushButton* found = dialog.buttonForTest(QLatin1String(button));
    QVERIFY2(found, button);
    QVERIFY2(found->isEnabled(), button);
    found->click();
}

bool conflictShown(const KeyboardShortcutsDialog& dialog)
{
    return dialog.buttonForTest(QStringLiteral("shortcutsGoToConflict"))->isVisibleTo(&dialog);
}

QList<QKeySequence> keys(std::initializer_list<const char*> texts)
{
    QList<QKeySequence> list;
    for (const char* text : texts) {
        list << QKeySequence(QLatin1String(text));
    }
    return list;
}

} // namespace

class KeyboardShortcutsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup() { pictura::resetToolShortcuts(); }
    void listsMenusAndTools();
    void selectedRowTakesTypedKeys();
    void leavingRowKeepsNewShortcut();
    void conflictShownInline();
    void conflictUndoChangesKeepsBoth();
    void conflictGoToSelectsOtherCommand();
    void commandHoldsSeveralBindings();
    void undoAndUseDefault();
    void toolLetterMovesBetweenGroups();
    void toolRejectsModifiedOrReservedKey();
    void menuTakingToolCycleKeyConflicts();
    void conflictGoToSwitchesList();
    void toolTakingCommandKeyConflicts();
    void workspaceKeysAreRefused();
    void workspaceKeysMatchTheWindow();
    void staleOverrideKeepsKeyUnambiguous();
    void noticeIsDarkOnYellow();
    void okAppliesCancelDiscards();
    void overridesPersistAcrossLaunch();

private:
    pictura::test::ScopedStateHome stateHome_;
};

void KeyboardShortcutsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
}

void KeyboardShortcutsTest::listsMenusAndTools()
{
    auto registry = makeRegistry();
    KeyboardShortcutsDialog dialog(registry.get());
    QCOMPARE(dialog.rowShortcutForTest(kNew), QStringLiteral("Ctrl+N"));
    QCOMPARE(dialog.rowShortcutForTest(
                 QString::fromLatin1(pictura::command_ids::EditKeyboardShortcuts)),
             QStringLiteral("Ctrl+Alt+Shift+K"));
    dialog.setScopeForTest(Scope::Tools);
    QCOMPARE(dialog.rowShortcutForTest(QStringLiteral("move")), QStringLiteral("V"));
    QCOMPARE(dialog.rowShortcutForTest(QStringLiteral("marquee")), QStringLiteral("M"));
    QVERIFY(dialog.rowShortcutForTest(kNew).isEmpty());
}

void KeyboardShortcutsTest::selectedRowTakesTypedKeys()
{
    auto registry = makeRegistry();
    KeyboardShortcutsDialog dialog(registry.get());
    dialog.show();
    dialog.activateWindow();
    QVERIFY(QTest::qWaitForWindowActive(&dialog));
    QVERIFY(dialog.selectRowForTest(kNew));
    // The key field has focus as soon as the row is chosen: keys go to it,
    // not to the tree's type-ahead.
    QCOMPARE(QApplication::focusWidget(), dialog.captureFieldForTest());
    QTest::keyClick(QApplication::focusWidget(), Qt::Key_N,
                    Qt::ControlModifier | Qt::AltModifier);
    click(dialog, "shortcutsAccept");
    QCOMPARE(dialog.rowShortcutForTest(kNew), QStringLiteral("Ctrl+Alt+N"));
    QCOMPARE(QApplication::focusWidget(), dialog.captureFieldForTest());
    // Nothing reaches the registry before OK.
    QCOMPARE(registry->shortcuts(kNew), keys({"Ctrl+N"}));
}

void KeyboardShortcutsTest::leavingRowKeepsNewShortcut()
{
    auto registry = makeRegistry();
    KeyboardShortcutsDialog dialog(registry.get());
    QVERIFY(dialog.selectRowForTest(kNew));
    type(dialog, Qt::Key_N, Qt::ControlModifier | Qt::AltModifier);
    QVERIFY(dialog.selectRowForTest(kOpen));
    QCOMPARE(dialog.rowShortcutForTest(kNew), QStringLiteral("Ctrl+Alt+N"));
}

void KeyboardShortcutsTest::conflictShownInline()
{
    auto registry = makeRegistry();
    KeyboardShortcutsDialog dialog(registry.get());
    QVERIFY(dialog.selectRowForTest(kOpen));
    QVERIFY(!conflictShown(dialog));
    type(dialog, Qt::Key_N, Qt::ControlModifier);
    QVERIFY(dialog.infoTextForTest().contains(QStringLiteral("File > New")));
    QVERIFY(conflictShown(dialog));
    click(dialog, "shortcutsAccept");
    QCOMPARE(dialog.rowShortcutForTest(kOpen), QStringLiteral("Ctrl+N"));
    QVERIFY(dialog.rowShortcutForTest(kNew).isEmpty());
}

void KeyboardShortcutsTest::conflictUndoChangesKeepsBoth()
{
    auto registry = makeRegistry();
    KeyboardShortcutsDialog dialog(registry.get());
    QVERIFY(dialog.selectRowForTest(kOpen));
    type(dialog, Qt::Key_N, Qt::ControlModifier);
    click(dialog, "shortcutsUndoChanges");
    QCOMPARE(dialog.rowShortcutForTest(kOpen), QStringLiteral("Ctrl+O"));
    QCOMPARE(dialog.rowShortcutForTest(kNew), QStringLiteral("Ctrl+N"));

    // Leaving the row with an unaccepted steal drops it too.
    type(dialog, Qt::Key_N, Qt::ControlModifier);
    QVERIFY(dialog.selectRowForTest(kNew));
    QCOMPARE(dialog.rowShortcutForTest(kOpen), QStringLiteral("Ctrl+O"));
    QCOMPARE(dialog.rowShortcutForTest(kNew), QStringLiteral("Ctrl+N"));
}

void KeyboardShortcutsTest::conflictGoToSelectsOtherCommand()
{
    auto registry = makeRegistry();
    KeyboardShortcutsDialog dialog(registry.get());
    QVERIFY(dialog.selectRowForTest(kOpen));
    type(dialog, Qt::Key_N, Qt::ControlModifier);
    click(dialog, "shortcutsGoToConflict");
    QCOMPARE(dialog.rowShortcutForTest(kOpen), QStringLiteral("Ctrl+N"));
    QCOMPARE(dialog.selectedRowForTest(), kNew);
}

void KeyboardShortcutsTest::commandHoldsSeveralBindings()
{
    auto registry = makeRegistry();
    KeyboardShortcutsDialog dialog(registry.get());
    QVERIFY(dialog.selectRowForTest(kNew));
    click(dialog, "shortcutsAdd");
    QCOMPARE(dialog.rowShortcutsForTest(kNew), QStringList({QStringLiteral("Ctrl+N"), QString()}));
    type(dialog, Qt::Key_F7, Qt::ControlModifier | Qt::ShiftModifier);
    click(dialog, "shortcutsAccept");
    QCOMPARE(dialog.rowShortcutsForTest(kNew),
             QStringList({QStringLiteral("Ctrl+N"), QStringLiteral("Ctrl+Shift+F7")}));

    // Remap the first binding in place; the second stays.
    QVERIFY(dialog.selectRowForTest(kNew, 0));
    type(dialog, Qt::Key_N, Qt::ControlModifier | Qt::AltModifier);
    click(dialog, "shortcutsAccept");
    QCOMPARE(dialog.rowShortcutsForTest(kNew),
             QStringList({QStringLiteral("Ctrl+Alt+N"), QStringLiteral("Ctrl+Shift+F7")}));

    QVERIFY(dialog.selectRowForTest(kNew, 1));
    click(dialog, "shortcutsDelete");
    QCOMPARE(dialog.rowShortcutsForTest(kNew), QStringList({QStringLiteral("Ctrl+Alt+N")}));
    click(dialog, "shortcutsDelete");
    QCOMPARE(dialog.rowShortcutsForTest(kNew), QStringList({QString()}));

    click(dialog, "shortcutsAdd");
    type(dialog, Qt::Key_F7, Qt::ControlModifier | Qt::ShiftModifier);
    click(dialog, "shortcutsAdd");
    type(dialog, Qt::Key_F8, Qt::ControlModifier | Qt::ShiftModifier);
    click(dialog, "shortcutsOk");
    QCOMPARE(registry->shortcuts(kNew), keys({"Ctrl+Shift+F7", "Ctrl+Shift+F8"}));
}

void KeyboardShortcutsTest::undoAndUseDefault()
{
    auto registry = makeRegistry();
    KeyboardShortcutsDialog dialog(registry.get());
    QPushButton* undo = dialog.buttonForTest(QStringLiteral("shortcutsUndo"));
    QVERIFY(!undo->isEnabled());
    QVERIFY(dialog.selectRowForTest(kNew));
    type(dialog, Qt::Key_N, Qt::ControlModifier | Qt::AltModifier);
    // Undo first drops the chord still pending in the field...
    undo->click();
    QCOMPARE(dialog.rowShortcutForTest(kNew), QStringLiteral("Ctrl+N"));
    QVERIFY(!undo->isEnabled());
    // ...then steps back through accepted edits.
    type(dialog, Qt::Key_N, Qt::ControlModifier | Qt::AltModifier);
    click(dialog, "shortcutsAccept");
    QCOMPARE(dialog.rowShortcutForTest(kNew), QStringLiteral("Ctrl+Alt+N"));
    undo->click();
    QCOMPARE(dialog.rowShortcutForTest(kNew), QStringLiteral("Ctrl+N"));

    type(dialog, Qt::Key_N, Qt::ControlModifier | Qt::AltModifier);
    click(dialog, "shortcutsAccept");
    click(dialog, "shortcutsUseDefault");
    QCOMPARE(dialog.rowShortcutsForTest(kNew), QStringList({QStringLiteral("Ctrl+N")}));
}

void KeyboardShortcutsTest::toolLetterMovesBetweenGroups()
{
    auto registry = makeRegistry();
    KeyboardShortcutsDialog dialog(registry.get());
    dialog.setScopeForTest(Scope::Tools);
    QVERIFY(dialog.selectRowForTest(QStringLiteral("move")));
    type(dialog, Qt::Key_M);
    QVERIFY(dialog.infoTextForTest().contains(QStringLiteral("Marquee")));
    click(dialog, "shortcutsAccept");
    QCOMPARE(dialog.rowShortcutForTest(QStringLiteral("move")), QStringLiteral("M"));
    QVERIFY(dialog.rowShortcutForTest(QStringLiteral("marquee")).isEmpty());
    QVERIFY(dialog.rowShortcutForTest(QStringLiteral("ellipticalmarquee")).isEmpty());

    // The freed V goes to the marquees; a tool in the same group may share a
    // letter without a conflict.
    QVERIFY(dialog.selectRowForTest(QStringLiteral("marquee")));
    type(dialog, Qt::Key_V);
    QVERIFY(!conflictShown(dialog));
    QVERIFY(dialog.selectRowForTest(QStringLiteral("ellipticalmarquee")));
    type(dialog, Qt::Key_V);
    QVERIFY(!conflictShown(dialog));

    click(dialog, "shortcutsOk");
    QCOMPARE(pictura::toolShortcut(pictura::ToolId::Move), QChar(u'M'));
    QCOMPARE(pictura::toolGroupForKey(QChar(u'M')), pictura::toolInfo(pictura::ToolId::Move).group);
    QCOMPARE(pictura::toolGroupForKey(QChar(u'V')),
             pictura::toolInfo(pictura::ToolId::Marquee).group);
    QCOMPARE(pictura::toolShortcutOverrides().size(), 3);
}

void KeyboardShortcutsTest::toolRejectsModifiedOrReservedKey()
{
    auto registry = makeRegistry();
    KeyboardShortcutsDialog dialog(registry.get());
    dialog.setScopeForTest(Scope::Tools);
    QVERIFY(dialog.selectRowForTest(QStringLiteral("move")));
    const QString help = dialog.infoTextForTest();
    for (const auto& [key, mods] : {std::pair{Qt::Key_M, Qt::ControlModifier},
                                    std::pair{Qt::Key_F2, Qt::NoModifier},
                                    std::pair{Qt::Key_X, Qt::NoModifier}}) {
        type(dialog, key, mods);
        QVERIFY(dialog.infoTextForTest() != help);
        QVERIFY(!dialog.buttonForTest(QStringLiteral("shortcutsAccept"))->isEnabled());
        QCOMPARE(dialog.rowShortcutForTest(QStringLiteral("move")), QStringLiteral("V"));
    }
}

void KeyboardShortcutsTest::menuTakingToolCycleKeyConflicts()
{
    auto registry = makeRegistry();
    KeyboardShortcutsDialog dialog(registry.get());
    QVERIFY(dialog.selectRowForTest(kOpen));
    // Shift+L cycles the lasso tools, so it is already in use.
    type(dialog, Qt::Key_L, Qt::ShiftModifier);
    QVERIFY(conflictShown(dialog));
    QVERIFY(dialog.infoTextForTest().contains(QStringLiteral("Lasso")));
    click(dialog, "shortcutsAccept");
    QCOMPARE(dialog.rowShortcutForTest(kOpen), QStringLiteral("Shift+L"));
    dialog.setScopeForTest(Scope::Tools);
    for (const char* lasso : {"lasso", "polygonallasso", "magneticlasso"}) {
        QVERIFY2(dialog.rowShortcutForTest(QLatin1String(lasso)).isEmpty(), lasso);
    }
    click(dialog, "shortcutsOk");
    QVERIFY(!pictura::toolShortcutKeys().contains(QChar(u'L')));
    QCOMPARE(registry->shortcuts(kOpen), keys({"Shift+L"}));
}

void KeyboardShortcutsTest::conflictGoToSwitchesList()
{
    auto registry = makeRegistry();
    KeyboardShortcutsDialog dialog(registry.get());
    QVERIFY(dialog.selectRowForTest(kOpen));
    type(dialog, Qt::Key_L, Qt::ShiftModifier);
    click(dialog, "shortcutsGoToConflict");
    QCOMPARE(dialog.selectedRowForTest(), QStringLiteral("lasso"));
    QVERIFY(dialog.rowShortcutForTest(QStringLiteral("lasso")).isEmpty());
}

void KeyboardShortcutsTest::toolTakingCommandKeyConflicts()
{
    auto registry = makeRegistry();
    KeyboardShortcutsDialog dialog(registry.get());
    // Free H, give its Shift+H to File > Open, then hand H back to the tool.
    dialog.setScopeForTest(Scope::Tools);
    QVERIFY(dialog.selectRowForTest(QStringLiteral("hand")));
    click(dialog, "shortcutsDelete");
    dialog.setScopeForTest(Scope::ApplicationMenus);
    QVERIFY(dialog.selectRowForTest(kOpen));
    type(dialog, Qt::Key_H, Qt::ShiftModifier);
    QVERIFY(!conflictShown(dialog));
    click(dialog, "shortcutsAccept");
    dialog.setScopeForTest(Scope::Tools);
    QVERIFY(dialog.selectRowForTest(QStringLiteral("hand")));
    type(dialog, Qt::Key_H);
    QVERIFY(conflictShown(dialog));
    QVERIFY(dialog.infoTextForTest().contains(QStringLiteral("File > Open")));
    click(dialog, "shortcutsAccept");
    click(dialog, "shortcutsOk");
    QCOMPARE(pictura::toolShortcut(pictura::ToolId::Hand), QChar(u'H'));
    QVERIFY(registry->shortcuts(kOpen).isEmpty());
}

void KeyboardShortcutsTest::workspaceKeysAreRefused()
{
    auto registry = makeRegistry();
    KeyboardShortcutsDialog dialog(registry.get());
    QVERIFY(dialog.selectRowForTest(kOpen));
    for (const auto& [key, mods] : {std::pair{Qt::Key_X, Qt::NoModifier},
                                    std::pair{Qt::Key_BracketLeft, Qt::NoModifier},
                                    std::pair{Qt::Key_F1, Qt::ShiftModifier}}) {
        type(dialog, key, mods);
        QVERIFY(dialog.infoTextForTest().contains(QStringLiteral("workspace")));
        QVERIFY(!dialog.buttonForTest(QStringLiteral("shortcutsAccept"))->isEnabled());
    }
    QCOMPARE(dialog.rowShortcutForTest(kOpen), QStringLiteral("Ctrl+O"));
}

// Every key the main window binds itself is a tool letter (plain or Shift) or
// listed in workspaceKeys(), so the editor can never hand it out twice.
void KeyboardShortcutsTest::workspaceKeysMatchTheWindow()
{
    auto window = pictura::test::makeMainWindow();
    const QList<QChar> letters = pictura::toolShortcutKeys();
    QList<QKeySequence> reserved;
    for (const pictura::WorkspaceKey& key : pictura::workspaceKeys()) {
        reserved << key.key;
    }
    int checked = 0;
    for (const QShortcut* shortcut : window->findChildren<QShortcut*>()) {
        const QKeySequence key = shortcut->key();
        if (key.isEmpty() || shortcut->parent() != window.get()) {
            continue;
        }
        const QKeyCombination combo = key[0];
        const int keyCode = int(combo.key());
        if (keyCode < 0 || keyCode > 0xffff) {
            // Tab, function keys, and other non-character keys cannot be tool
            // letters; QChar only accepts BMP code points.
            ++checked;
            continue;
        }
        const QChar letter(keyCode);
        const bool toolKey = key.count() == 1 && letters.contains(letter)
                          && (combo.keyboardModifiers() == Qt::NoModifier
                              || combo.keyboardModifiers() == Qt::ShiftModifier);
        QVERIFY2(toolKey || reserved.contains(key), qPrintable(key.toString()));
        ++checked;
    }
    QVERIFY(checked > 20);
}

void KeyboardShortcutsTest::staleOverrideKeepsKeyUnambiguous()
{
    // A store from before the editor checked tool keys: File > Open on
    // Shift+L while the lassos still own L.
    pictura::SessionState state = pictura::loadSession();
    state.shortcuts = {{kOpen, {QStringLiteral("Shift+L")}}};
    state.toolShortcuts.clear();
    QVERIFY(pictura::saveSession(state));
    auto window = pictura::test::makeMainWindow();
    int owners = 0;
    for (const QShortcut* shortcut : window->findChildren<QShortcut*>()) {
        owners += shortcut->key() == QKeySequence(QStringLiteral("Shift+L")) ? 1 : 0;
    }
    for (const QAction* action : window->findChildren<QAction*>()) {
        owners += action->shortcuts().contains(QKeySequence(QStringLiteral("Shift+L"))) ? 1 : 0;
    }
    QCOMPARE(owners, 1);
    QCOMPARE(window->registry()->action(kOpen)->shortcuts(), keys({"Shift+L"}));
    // Plain L still selects the lassos.
    bool plainL = false;
    for (const QShortcut* shortcut : window->findChildren<QShortcut*>()) {
        plainL = plainL || shortcut->key() == QKeySequence(QStringLiteral("L"));
    }
    QVERIFY(plainL);
    state.shortcuts.clear();
    QVERIFY(pictura::saveSession(state));
}

void KeyboardShortcutsTest::noticeIsDarkOnYellow()
{
    pictura::Theme::apply(pictura::Theme::kDefaultLevel);
    auto registry = makeRegistry();
    KeyboardShortcutsDialog dialog(registry.get());
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    QVERIFY(dialog.selectRowForTest(kOpen));
    type(dialog, Qt::Key_N, Qt::ControlModifier);
    QVERIFY(conflictShown(dialog));
    auto* text = dialog.findChild<QLabel*>(QStringLiteral("shortcutsInfoText"));
    QVERIFY(text);
    QVERIFY(text->property("notice").toBool());
    const QImage label = text->grab().toImage();
    const QColor bg = label.pixelColor(2, 2);
    QVERIFY2(bg.hue() >= 40 && bg.hue() <= 60 && bg.lightness() > 140, qPrintable(bg.name()));
    // The darkest pixel of the rendered warning is the text, near black.
    int darkest = 255;
    for (int y = 0; y < label.height(); ++y) {
        for (int x = 0; x < label.width(); ++x) {
            darkest = qMin(darkest, label.pixelColor(x, y).lightness());
        }
    }
    QVERIFY(darkest < 64);
    // Only the text is yellow: the area around it keeps the dialog's surface.
    auto* area = dialog.findChild<QWidget*>(QStringLiteral("shortcutsInfo"));
    QVERIFY(area);
    const QColor surface = area->grab().toImage().pixelColor(area->width() - 4, area->height() - 4);
    QVERIFY2(surface.lightness() < 120, qPrintable(surface.name()));
    qApp->setStyleSheet(QString());
}

void KeyboardShortcutsTest::okAppliesCancelDiscards()
{
    auto registry = makeRegistry();
    {
        KeyboardShortcutsDialog dialog(registry.get());
        QVERIFY(dialog.selectRowForTest(kNew));
        type(dialog, Qt::Key_N, Qt::ControlModifier | Qt::AltModifier);
        click(dialog, "shortcutsAccept");
        click(dialog, "shortcutsCancel");
    }
    QCOMPARE(registry->shortcuts(kNew), keys({"Ctrl+N"}));
    QVERIFY(registry->shortcutOverrides().isEmpty());
    {
        KeyboardShortcutsDialog dialog(registry.get());
        QVERIFY(dialog.selectRowForTest(kNew));
        // OK keeps a chord still pending in the field.
        type(dialog, Qt::Key_N, Qt::ControlModifier | Qt::AltModifier);
        click(dialog, "shortcutsOk");
        QCOMPARE(dialog.result(), int(QDialog::Accepted));
    }
    QCOMPARE(registry->shortcuts(kNew), keys({"Ctrl+Alt+N"}));
    QCOMPARE(registry->defaultShortcuts(kNew), keys({"Ctrl+N"}));
    QCOMPARE(registry->shortcutOverrides().size(), 1);
}

void KeyboardShortcutsTest::overridesPersistAcrossLaunch()
{
    {
        auto window = pictura::test::makeMainWindow();
        int seen = 0;
        QTimer timer;
        timer.setInterval(5);
        QObject::connect(&timer, &QTimer::timeout, [&]() {
            auto* dialog = qobject_cast<KeyboardShortcutsDialog*>(QApplication::activeModalWidget());
            if (!dialog) {
                return;
            }
            timer.stop();
            ++seen;
            QVERIFY(dialog->selectRowForTest(kNew));
            click(*dialog, "shortcutsAdd");
            type(*dialog, Qt::Key_N, Qt::ControlModifier | Qt::AltModifier);
            click(*dialog, "shortcutsAccept");
            dialog->setScopeForTest(Scope::Tools);
            QVERIFY(dialog->selectRowForTest(QStringLiteral("hand")));
            click(*dialog, "shortcutsDelete");
            click(*dialog, "shortcutsOk");
        });
        timer.start();
        QVERIFY(window->registry()->dispatch(
            QString::fromLatin1(pictura::command_ids::EditKeyboardShortcuts)));
        QCOMPARE(seen, 1);
        QAction* action = window->registry()->action(kNew);
        QVERIFY(action);
        QCOMPARE(action->shortcuts(), keys({"Ctrl+N", "Ctrl+Alt+N"}));
    }
    const pictura::SessionState state = pictura::loadSession();
    QCOMPARE(state.shortcuts.size(), 1);
    QCOMPARE(state.shortcuts.value(kNew),
             QStringList({QStringLiteral("Ctrl+N"), QStringLiteral("Ctrl+Alt+N")}));
    QVERIFY(state.toolShortcuts.contains(QStringLiteral("hand")));
    QVERIFY(state.toolShortcuts.value(QStringLiteral("hand")).isEmpty());

    pictura::resetToolShortcuts();
    auto relaunched = pictura::test::makeMainWindow();
    QCOMPARE(relaunched->registry()->action(kNew)->shortcuts(), keys({"Ctrl+N", "Ctrl+Alt+N"}));
    QVERIFY(pictura::toolShortcut(pictura::ToolId::Hand).isNull());
    QVERIFY(!pictura::toolShortcutKeys().contains(QChar(u'H')));
}

QTEST_MAIN(KeyboardShortcutsTest)
#include "tst_keyboard_shortcuts.moc"
