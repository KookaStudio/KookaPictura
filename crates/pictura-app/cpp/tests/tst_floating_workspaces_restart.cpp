#include <QtTest/QtTest>

#include <QtCore/QCoreApplication>
#include <QtCore/QScopeGuard>

#include "frame.h"
#include "panels/panel_column.h"

#include "qt_test_support.h"

using namespace pictura;

namespace {

void pump(int rounds = 4)
{
    for (int i = 0; i < rounds; ++i) {
        QCoreApplication::processEvents();
    }
}

} // namespace

class FloatingWorkspacesRestartTest : public QObject {
    Q_OBJECT

private slots:
    void init() { PanelFloat::setForceChildOverlayForTest(false); }
    void cleanup() { PanelFloat::setForceChildOverlayForTest(false); }

    void restartRestoresFloatAtRightOfRestoredFrame();
};

void FloatingWorkspacesRestartTest::restartRestoresFloatAtRightOfRestoredFrame()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    PanelFloat::setForceChildOverlayForTest(true);
    auto reset = qScopeGuard([] { PanelFloat::setForceChildOverlayForTest(false); });

    QRect frameRect;
    QPoint savedOrigin;
    QRect saved;
    {
        auto window = pictura::test::makeMainWindow();
        window->setGeometry(20, 20, 835, 700);
        window->show();
        pump(8);
        frameRect = window->geometry();
        savedOrigin = window->mapToGlobal(QPoint(0, 0));
        PanelColumn* column = window->columnForPanel(QStringLiteral("colorPanel"));
        QVERIFY(column);
        QVERIFY(column->tearOffPanelForTest(QStringLiteral("colorPanel")));
        pump();
        QCOMPARE(column->floatCountForTest(), 1);
        const QPoint target(frameRect.right() - 60, frameRect.top() + 100);
        QVERIFY(column->floatClampedForTest(0, target));
        saved = column->floatGeometryForTest(0);
        QVERIFY(saved.center().x() > frameRect.center().x());
        window->saveSession();
    }
    {
        auto window = pictura::test::makeMainWindow();
        window->show();
        pump(16);
        QVERIFY(qAbs(window->width() - frameRect.width()) <= 2);
        QVERIFY(qAbs(window->height() - frameRect.height()) <= 2);
        PanelColumn* column = window->columnForPanel(QStringLiteral("colorPanel"));
        QVERIFY(column);
        QCOMPARE(column->floatCountForTest(), 1);
        const QRect restored = column->floatGeometryForTest(0);
        const QPoint restoredOrigin = window->mapToGlobal(QPoint(0, 0));
        QVERIFY2(restored.center().x() > restoredOrigin.x() + window->width() / 2,
                 "restored float must stay on the right of the restored frame");
        QVERIFY(qAbs((restored.left() - restoredOrigin.x())
                     - (saved.left() - savedOrigin.x()))
                <= 2);
        QVERIFY(restored.top() >= restoredOrigin.y());
        QVERIFY(restored.bottom() <= restoredOrigin.y() + window->height());
    }
}

QTEST_MAIN(FloatingWorkspacesRestartTest)
#include "tst_floating_workspaces_restart.moc"
