#include <QtTest/QtTest>

#include <QtCore/QCoreApplication>
#include <QtCore/QStringList>
#include <QtGui/QAction>
#include <QtWidgets/QMenu>

#include "commands.h"
#include "frame.h"
#include "show_extras_dialog.h"
#include "toolbox.h"
#include "tools.h"

#include <QtCore/QTimer>
#include <QtWidgets/QCheckBox>

#include "qt_test_support.h"

using pictura::PicturaMainWindow;
using pictura::Toolbox;

class EditionGatingTest : public QObject {
    Q_OBJECT

private slots:
    void editionGating();
};

void EditionGatingTest::editionGating()
{
    const QByteArray savedEdition = qgetenv("PICTURA_EDITION");
    const auto labelsForGroup = [](Toolbox* toolbox, int group) {
        QStringList labels;
        for (QAction* action : toolbox->slotMenuActionsForTest(group)) {
            labels << action->text();
        }
        return labels;
    };
    const auto hasCommand = [](PicturaMainWindow& frame, const char* id) {
        return frame.registry()->action(QString::fromLatin1(id)) != nullptr;
    };
    const auto extrasCount = [](PicturaMainWindow& frame) {
        int count = -1;
        QTimer::singleShot(0, [&frame, &count]() {
            if (auto* dialog = frame.findChild<pictura::ShowExtrasDialog*>()) {
                count = int(dialog->findChildren<QCheckBox*>().size());
                dialog->reject();
            }
        });
        frame.registry()->dispatch(
            QString::fromLatin1(pictura::command_ids::ViewShowExtraOptions));
        return count;
    };

    {
        pictura::test::ScopedStateHome extendedHome;
        QVERIFY(extendedHome.isValid());
        qputenv("PICTURA_EDITION", "extended");
        auto extended = pictura::test::makeMainWindow();
        QVERIFY(extended != nullptr);
        QCOMPARE(extrasCount(*extended), 10);
        auto* toolbox = extended->findChild<Toolbox*>(QStringLiteral("toolsPanel"));
        QVERIFY(toolbox != nullptr);
        QVERIFY(pictura::toolAvailable(pictura::ToolId::Count));
        QVERIFY2(labelsForGroup(toolbox, 6).contains(QStringLiteral("Count (Extended)")),
                 "Extended keeps the Count tool");
        for (const char* id : {"edit.preferences.3d", "file.scripts.statistics",
                               "layer.rasterize.3d", "type.extrude.to.3d",
                               "view.show.3d.axis",
                               "view.show.count", "window.panels.3d",
                               "window.panels.measurement.log",
                               "layer.smart.objects.stack.mode.none",
                               "layer.smart.objects.stack.mode.entropy",
                               "layer.smart.objects.stack.mode.kurtosis",
                               "layer.smart.objects.stack.mode.maximum",
                               "layer.smart.objects.stack.mode.mean",
                               "layer.smart.objects.stack.mode.median",
                               "layer.smart.objects.stack.mode.minimum",
                               "layer.smart.objects.stack.mode.range",
                               "layer.smart.objects.stack.mode.skewness",
                               "layer.smart.objects.stack.mode.standard.deviation",
                               "layer.smart.objects.stack.mode.summation",
                               "layer.smart.objects.stack.mode.variance"}) {
            QVERIFY2(hasCommand(*extended, id), id);
        }
    }

    {
        pictura::test::ScopedStateHome standardHome;
        QVERIFY(standardHome.isValid());
        qputenv("PICTURA_EDITION", "standard");
        auto standard = pictura::test::makeMainWindow();
        QVERIFY(standard != nullptr);
        QCOMPARE(extrasCount(*standard), 9);
        auto* toolbox = standard->findChild<Toolbox*>(QStringLiteral("toolsPanel"));
        QVERIFY(toolbox != nullptr);
        QVERIFY(!pictura::toolAvailable(pictura::ToolId::Count));
        const QStringList labels = labelsForGroup(toolbox, 6);
        QVERIFY2(!labels.contains(QStringLiteral("Count (Extended)")),
                 "Standard drops the Count tool");
        QVERIFY2(labels.contains(QStringLiteral("Note")), "the rest of the I group stays");
        standard->setActiveTool(pictura::ToolId::Count);
        QCOMPARE(standard->activeTool(), pictura::ToolId::Move);
        for (const char* id : {"edit.preferences.3d", "file.scripts.statistics",
                               "layer.rasterize.3d", "type.extrude.to.3d",
                               "view.show.3d.axis",
                               "view.show.count", "window.panels.3d",
                               "window.panels.measurement.log",
                               "layer.smart.objects.stack.mode.none",
                               "layer.smart.objects.stack.mode.entropy",
                               "layer.smart.objects.stack.mode.kurtosis",
                               "layer.smart.objects.stack.mode.maximum",
                               "layer.smart.objects.stack.mode.mean",
                               "layer.smart.objects.stack.mode.median",
                               "layer.smart.objects.stack.mode.minimum",
                               "layer.smart.objects.stack.mode.range",
                               "layer.smart.objects.stack.mode.skewness",
                               "layer.smart.objects.stack.mode.standard.deviation",
                               "layer.smart.objects.stack.mode.summation",
                               "layer.smart.objects.stack.mode.variance"}) {
            QVERIFY2(!hasCommand(*standard, id), id);
        }
        QVERIFY2(hasCommand(*standard, "window.panels.timeline"),
                 "the Timeline ships in both editions");
    }

    if (savedEdition.isNull()) {
        qunsetenv("PICTURA_EDITION");
    } else {
        qputenv("PICTURA_EDITION", savedEdition);
    }
}

QTEST_MAIN(EditionGatingTest)
#include "tst_edition_gating.moc"
