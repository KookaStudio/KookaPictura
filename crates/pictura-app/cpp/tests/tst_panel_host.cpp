// The panel host seam: panels, columns, toggles and a workspace layout can be
// driven with a stub shell and no PicturaMainWindow.

#include <QtTest/QtTest>

#include <QtCore/QJsonArray>
#include <QtCore/QJsonObject>
#include <QtWidgets/QApplication>
#include <QtWidgets/QMainWindow>
#include <QtWidgets/QWidget>

#include <memory>

#include "commands.h"
#include "document_registry.h"
#include "panel_host.h"
#include "panels/color_panel.h"
#include "panels/layers_panel.h"
#include "panels/panel_column.h"
#include "panels/panel_group.h"
#include "qt_test_support.h"
#include "session.h"

namespace {

pictura::DocumentRegistry::Hooks stubDocumentHooks(QWidget* shell)
{
    pictura::DocumentRegistry::Hooks hooks;
    hooks.shellWidget = [shell]() -> QWidget* { return shell; };
    hooks.openDocumentsAsTabs = []() { return true; };
    hooks.floatingDocumentDocking = []() { return true; };
    hooks.fullScreen = []() { return false; };
    hooks.requestRefresh = []() {};
    hooks.requestPanelRefresh = []() {};
    hooks.stopPanelRefreshTimer = []() {};
    hooks.activateShellWindow = []() {};
    hooks.raisePanelOverlays = []() {};
    hooks.retargetPanels = []() {};
    hooks.closeWindowRequested = [](pictura::DocWindow*) {};
    hooks.setupCanvas = [](pictura::DocWindow&) {};
    hooks.wireCanvas = [](pictura::DocWindow&) {};
    hooks.wireDocument = [](pictura::DocEntry&) {};
    hooks.applyViewSettings = [](pictura::PictureView*) {};
    hooks.wireOpenSignals = [](pictura::PictureView*) {};
    hooks.rememberRecent = [](const QString&) {};
    hooks.forgetToolView = [](pictura::PictureView*) {};
    hooks.forgetCanvas = [](pictura::ImageView*) {};
    hooks.openDelayForTest = []() -> quint64 { return 0; };
    hooks.loadingPageColor = []() { return QColor(Qt::white); };
    hooks.scrollAllWindows = []() { return false; };
    hooks.toolCanvas = []() -> pictura::ImageView* { return nullptr; };
    return hooks;
}

QJsonObject panelGroup(const QStringList& panels)
{
    QJsonObject group;
    group.insert(QStringLiteral("order"), QJsonArray::fromStringList(panels));
    group.insert(QStringLiteral("visible"), QJsonArray::fromStringList(panels));
    group.insert(QStringLiteral("minimized"), false);
    group.insert(QStringLiteral("collapsed"), false);
    return group;
}

} // namespace

class PanelHostTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void buildColumnsToggleAndWorkspace();

private:
    pictura::test::ScopedStateHome stateHome_;
};

void PanelHostTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
}

void PanelHostTest::buildColumnsToggleAndWorkspace()
{
    QMainWindow shell;
    auto documents = std::make_unique<pictura::DocumentRegistry>(stubDocumentHooks(&shell),
                                                                &shell);
    pictura::CommandRegistry commands(&shell);
    pictura::ColorState colors;
    pictura::PanelHost::Hooks hooks;
    hooks.saveSession = []() {};
    hooks.activeView = []() -> pictura::PictureView* { return nullptr; };
    hooks.activeCanvas = []() -> pictura::ImageView* { return nullptr; };
    pictura::PanelHost host(&shell, documents.get(), &commands, &colors, nullptr, hooks);

    // Build the panel set from a fresh session and apply the factory layout.
    pictura::SessionState state;
    const QList<pictura::PanelGroup*> staged = host.buildPanels(state);
    QVERIFY(host.panelColumn() != nullptr);
    QVERIFY(host.layersPanel() != nullptr);
    QVERIFY(host.panelObjectNames().contains(QStringLiteral("layersPanel")));
    host.applyFreshSessionDefault(state, staged);
    QVERIFY(host.columnCount() >= 2);
    QVERIFY(host.columnForPanel(QStringLiteral("layersPanel")) != nullptr);

    // Create a left column and move it beside the primary column.
    const int before = host.columnCount();
    pictura::PanelColumn* left = host.createPanelColumn(pictura::PanelSide::Left, nullptr);
    QVERIFY(left != nullptr);
    QCOMPARE(host.columnCount(), before + 1);
    QCOMPARE(host.sideOf(left), pictura::PanelSide::Left);
    QVERIFY(host.movePanelColumn(left, 1, host.panelColumn()));
    QCOMPARE(host.sideOf(left), pictura::PanelSide::Right);

    // Toggle a panel through the host, as the Window > Panels commands do.
    pictura::PanelColumn* owner = host.columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(owner != nullptr);
    QVERIFY(owner->isPanelVisible(QStringLiteral("layersPanel")));
    host.togglePanel(QStringLiteral("layersPanel"));
    QVERIFY(!owner->isPanelVisible(QStringLiteral("layersPanel")));
    host.togglePanel(QStringLiteral("layersPanel"));
    QVERIFY(owner->isPanelVisible(QStringLiteral("layersPanel")));

    // Apply a workspace-shaped layout: the named panels land in a column.
    QJsonObject column;
    column.insert(QStringLiteral("side"), QStringLiteral("right"));
    column.insert(QStringLiteral("order"), 0);
    column.insert(QStringLiteral("width"), pictura::PanelColumn::defaultNormalWidth());
    column.insert(QStringLiteral("railMode"), QStringLiteral("normal"));
    column.insert(QStringLiteral("groups"),
                  QJsonArray{panelGroup({QStringLiteral("layersPanel"),
                                         QStringLiteral("channelsPanel")}),
                             panelGroup({QStringLiteral("brushPanel")})});
    host.applyWorkspaceLayout(QJsonArray{column});
    QVERIFY(host.columnForPanel(QStringLiteral("layersPanel")) != nullptr);
    QVERIFY(host.columnForPanel(QStringLiteral("brushPanel")) != nullptr);
}

QTEST_MAIN(PanelHostTest)
#include "tst_panel_host.moc"
