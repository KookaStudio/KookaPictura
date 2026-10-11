// Select tools Alt two-stage (#252): the first Alt subtracts, a second Alt
// during the same drag mirrors the rectangle about the anchor.

#include <QtTest/QtTest>

#include "selftest_paint_fixture.h"

#include "qt_test_support.h"

namespace {
using paint_fixture::Fixture;
} // namespace

class SelectionAltTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void firstAltSubtractsAndDoesNotMirror();
    void secondAltMirrorsAboutTheAnchor();
    void altRepressWithoutMovementMirrors();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void SelectionAltTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void SelectionAltTest::firstAltSubtractsAndDoesNotMirror()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(64, 64, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_sel_alt_first"));
    QVERIFY2(f.ok(), "selection fixture");
    frame.setActiveTool(pictura::ToolId::Marquee);
    f.tools->setCombineMode(pictura::SelectionMode::New);
    f.tools->setMarqueeStyle(pictura::MarqueeStyle::Normal);

    QVERIFY(f.view->select_rect(10, 10, 40, 40, QStringLiteral("new"), 0.0));
    const int base = f.view->history_count();

    // Alt begins the drag with an existing selection: subtract, no mirror.
    f.canvas->mousePressed(QPointF(30, 30), Qt::LeftButton, int(Qt::AltModifier));
    QCOMPARE(f.tools->dragModeForTest(), int(pictura::SelectionMode::Subtract));
    QVERIFY(!f.tools->dragMirrorForTest());
    f.canvas->mouseMoved(QPointF(40, 40));
    QVERIFY2(!f.tools->dragMirrorForTest(), "the first Alt never mirrors");
    f.canvas->mouseReleased(QPointF(40, 40));

    QCOMPARE(f.view->history_count(), base + 1);
    // The subtracted rect is the ordinary (30,30)-(40,40) drag, not a
    // from-centre mirror: (35,35) is cleared, (25,25) is untouched.
    QCOMPARE(f.view->selection_coverage(35, 35), 0);
    QCOMPARE(f.view->selection_coverage(25, 25), 255);
}

void SelectionAltTest::secondAltMirrorsAboutTheAnchor()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(64, 64, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_sel_alt_second"));
    QVERIFY2(f.ok(), "selection fixture");
    frame.setActiveTool(pictura::ToolId::Marquee);
    f.tools->setCombineMode(pictura::SelectionMode::New);
    f.tools->setMarqueeStyle(pictura::MarqueeStyle::Normal);

    QVERIFY(f.view->select_rect(10, 10, 40, 40, QStringLiteral("new"), 0.0));
    const int base = f.view->history_count();

    // First Alt (held at press) subtracts; the move clears the tracked Alt.
    f.canvas->mousePressed(QPointF(30, 30), Qt::LeftButton, int(Qt::AltModifier));
    QCOMPARE(f.tools->dragModeForTest(), int(pictura::SelectionMode::Subtract));
    f.canvas->mouseMoved(QPointF(31, 31));
    QVERIFY(!f.tools->dragMirrorForTest());

    // A second Alt press during the same drag flips the pivot-mirror.
    QTest::keyPress(&frame, Qt::Key_Alt, Qt::AltModifier);
    f.canvas->mouseMoved(QPointF(40, 40));
    QVERIFY2(f.tools->dragMirrorForTest(), "the second Alt mirrors");
    f.canvas->mouseReleased(QPointF(40, 40));
    QTest::keyRelease(&frame, Qt::Key_Alt, Qt::NoModifier);

    QCOMPARE(f.view->history_count(), base + 1);
    // The mirrored rect spans (20,20)-(40,40): (25,25) is now cleared, while
    // the ordinary (30,30)-(40,40) drag would have left it untouched.
    QCOMPARE(f.view->selection_coverage(25, 25), 0);
    QCOMPARE(f.view->selection_coverage(45, 45), 255);
}

void SelectionAltTest::altRepressWithoutMovementMirrors()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(64, 64, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_sel_alt_repress"));
    QVERIFY2(f.ok(), "selection fixture");
    frame.setActiveTool(pictura::ToolId::Marquee);
    f.tools->setCombineMode(pictura::SelectionMode::New);
    f.tools->setMarqueeStyle(pictura::MarqueeStyle::Normal);
    QVERIFY(f.view->select_rect(10, 10, 40, 40, QStringLiteral("new"), 0.0));

    // Alt is already held at press, so the drag subtracts.
    f.canvas->mousePressed(QPointF(30, 30), Qt::LeftButton, int(Qt::AltModifier));
    QCOMPARE(f.tools->dragModeForTest(), int(pictura::SelectionMode::Subtract));
    QVERIFY(!f.tools->dragMirrorForTest());

    // Release and press Alt again with no pointer movement: the key transition
    // alone arms the mirror.
    QTest::keyRelease(f.canvas, Qt::Key_Alt, Qt::NoModifier);
    QTest::keyPress(f.canvas, Qt::Key_Alt, Qt::AltModifier);
    QVERIFY2(f.tools->dragMirrorForTest(), "a second Alt press without movement mirrors");

    f.canvas->mouseReleased(QPointF(30, 30));
}

QTEST_MAIN(SelectionAltTest)
#include "tst_selection_alt.moc"