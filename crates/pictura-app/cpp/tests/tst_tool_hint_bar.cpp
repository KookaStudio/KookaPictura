#include <QtTest/QtTest>

#include <QtGui/QColor>
#include <QtGui/QIcon>

#include "icons.h"
#include "tool_hint_bar.h"

class ToolHintBarTest : public QObject {
    Q_OBJECT

private slots:
    void chevronsUseLucideChevronIcons();
};

void ToolHintBarTest::chevronsUseLucideChevronIcons()
{
    pictura::ToolHintBar bar;
    bar.setHints(pictura::toolHintEntries(pictura::ToolId::Move), QString());
    QCOMPARE(bar.chevronCountForTest(0), 4);
    QCOMPARE(bar.chevronDirectionsForTest(), QStringLiteral("ULDR"));

    // U L D R: up/left are the right/down assets turned 180 degrees.
    QCOMPARE(bar.chevronAssetForTest(0), QStringLiteral("layers.disclosureDown"));
    QVERIFY2(bar.chevronFlippedForTest(0), "up is a flipped chevron-down");
    QCOMPARE(bar.chevronAssetForTest(1), QStringLiteral("layers.disclosureRight"));
    QVERIFY2(bar.chevronFlippedForTest(1), "left is a flipped chevron-right");
    QCOMPARE(bar.chevronAssetForTest(2), QStringLiteral("layers.disclosureDown"));
    QVERIFY2(!bar.chevronFlippedForTest(2), "down is not flipped");
    QCOMPARE(bar.chevronAssetForTest(3), QStringLiteral("layers.disclosureRight"));
    QVERIFY2(!bar.chevronFlippedForTest(3), "right is not flipped");

    for (int i = 0; i < 4; ++i) {
        QVERIFY2(!pictura::icon(bar.chevronAssetForTest(i), QColor(255, 255, 255)).isNull(),
                 "chevron asset resolves in the resource bundle");
    }
}

QTEST_MAIN(ToolHintBarTest)
#include "tst_tool_hint_bar.moc"
