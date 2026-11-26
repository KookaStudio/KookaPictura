#include <QtTest/QtTest>

#include <QtGui/QAction>
#include <QtGui/QColor>
#include <QtWidgets/QMenuBar>
#include <QtWidgets/QWidget>

#include "commands.h"
#include "menu_registrar.h"

class MenuRegistrarTest : public QObject {
    Q_OBJECT

private slots:
    void buildsMenuWithoutFrame();
};

// The registrar owns the command table and builds the menu bar from its
// collaborators and runtime; a stub chrome registration stands in for the
// frame-owned commands.
void MenuRegistrarTest::buildsMenuWithoutFrame()
{
    QWidget shell;
    QMenuBar bar(&shell);

    bool frameHandlerCalled = false;
    pictura::MenuRegistrar::Runtime runtime;
    runtime.requestRefresh = []() {};
    runtime.showStatus = [](const QString&, int) {};
    runtime.dialogParent = [&shell]() -> QWidget* { return &shell; };
    runtime.uiScalePercent = []() { return 100; };
    runtime.foregroundColor = []() { return QColor(Qt::black); };

    pictura::MenuRegistrar registrar(
        [&frameHandlerCalled](pictura::CommandRegistry& commands) {
            commands.setImplemented(pictura::command_ids::FileNew, true);
            commands.setHandler(pictura::command_ids::FileNew,
                                [&frameHandlerCalled]() { frameHandlerCalled = true; });
            commands.setCheckedProvider(pictura::command_ids::ViewScreenModeStandard,
                                        []() { return true; });
        },
        runtime);

    registrar.registerHandlers();
    registrar.buildMenus(&bar);

    QVERIFY(registrar.commands() != nullptr);
    const QStringList expected = {
        QStringLiteral("File"),   QStringLiteral("Edit"),   QStringLiteral("Image"),
        QStringLiteral("Layer"),  QStringLiteral("Type"),   QStringLiteral("Select"),
        QStringLiteral("Filter"), QStringLiteral("View"),   QStringLiteral("Window"),
        QStringLiteral("Help")};
    QCOMPARE(registrar.commands()->topLevelTitles(), expected);
    QVERIFY(!bar.actions().isEmpty());

    QAction* newDocument = registrar.commands()->action(
        QString::fromLatin1(pictura::command_ids::FileNew));
    QVERIFY(newDocument != nullptr);
    registrar.commands()->refresh();
    QVERIFY2(newDocument->isEnabled(), "the stub chrome command is live");
    newDocument->trigger();
    QVERIFY2(frameHandlerCalled, "the chrome registration's handler runs");

    QAction* standard = registrar.commands()->action(
        QString::fromLatin1(pictura::command_ids::ViewScreenModeStandard));
    QVERIFY(standard != nullptr);
    QVERIFY2(standard->isChecked(), "a checked provider reaches the menu action");
}

#include "qt_test_support.h"

PICTURA_QT_TEST_MAIN(MenuRegistrarTest)
#include "tst_menu_registrar.moc"
