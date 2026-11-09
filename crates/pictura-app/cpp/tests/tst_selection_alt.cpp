// Select tools Alt: Alt at press subtracts with an existing selection (or draws
// from the centre without one); an Alt press after the drag begins mirrors.

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
    void altPressedDuringDragMirrors();
    void altWithNoSelectionDrawsFromTheCentre();

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
    f.tools->optionsFor<pictura::SelectionOptions>(pictura::ToolId::Marquee)->setCombineMode(pictura::SelectionMode::New);
    f.tools->optionsFor<pictura::SelectionOptions>(pictura::ToolId::Marquee)->setMarqueeStyle(pictura::MarqueeStyle::Normal);

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
    f.tools->optionsFor<pictura::SelectionOptions>(pictura::ToolId::Marquee)->setCombineMode(pictura::SelectionMode::New);
    f.tools->optionsFor<pictura::SelectionOptions>(pictura::ToolId::Marquee)->setMarqueeStyle(pictura::MarqueeStyle::Normal);

    QVERIFY(f.view->select_rect(10, 10, 40, 40, QStringLiteral("new"), 0.0));
    const int base = f.view->history_count();

    // First Alt (held at press) subtracts; the move clears the tracked Alt.
    f.canvas->mousePressed(QPointF(30, 30), Qt::LeftButton, int(Qt::AltModifier));
    QCOMPARE(f.tools->dragModeForTest(), int(pictura::SelectionMode::Subtract));
    f.canvas->mouseMoved(QPointF(31, 31));
    QVERIFY(!f.tools->dragMirrorForTest());

    // Release and press Alt again during the same drag: the pivot-mirror.
    QTest::keyRelease(f.canvas, Qt::Key_Alt, Qt::NoModifier);
    QTest::keyPress(f.canvas, Qt::Key_Alt, Qt::AltModifier);
    f.canvas->mouseMoved(QPointF(40, 40));
    QVERIFY2(f.tools->dragMirrorForTest(), "the second Alt mirrors");
    f.canvas->mouseReleased(QPointF(40, 40));
    QTest::keyRelease(f.canvas, Qt::Key_Alt, Qt::NoModifier);

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
    f.tools->optionsFor<pictura::SelectionOptions>(pictura::ToolId::Marquee)->setCombineMode(pictura::SelectionMode::New);
    f.tools->optionsFor<pictura::SelectionOptions>(pictura::ToolId::Marquee)->setMarqueeStyle(pictura::MarqueeStyle::Normal);
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
    QTest::keyRelease(f.canvas, Qt::Key_Alt, Qt::NoModifier);
}


void SelectionAltTest::altPressedDuringDragMirrors()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(64, 64, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_sel_alt_drag"));
    QVERIFY2(f.ok(), "selection fixture");
    frame.setActiveTool(pictura::ToolId::Marquee);
    auto* options = f.tools->optionsFor<pictura::SelectionOptions>(pictura::ToolId::Marquee);
    options->setCombineMode(pictura::SelectionMode::New);
    options->setMarqueeStyle(pictura::MarqueeStyle::Normal);
    QVERIFY(f.view->select_rect(10, 10, 40, 40, QStringLiteral("new"), 0.0));

    // Start outside the existing selection with no Alt: an ordinary New marquee.
    f.canvas->mousePressed(QPointF(5, 5), Qt::LeftButton, int(Qt::NoModifier));
    QCOMPARE(f.tools->dragModeForTest(), int(pictura::SelectionMode::New));
    f.canvas->mouseMoved(QPointF(6, 6));
    QVERIFY(!f.tools->dragMirrorForTest());

    // The first Alt press after the drag began mirrors; it does not subtract.
    QTest::keyPress(f.canvas, Qt::Key_Alt, Qt::AltModifier);
    f.canvas->mouseMoved(QPointF(15, 15));
    QVERIFY(f.tools->dragMirrorForTest());
    QCOMPARE(f.tools->dragModeForTest(), int(pictura::SelectionMode::New));
    f.canvas->mouseReleased(QPointF(15, 15));
    QTest::keyRelease(f.canvas, Qt::Key_Alt, Qt::NoModifier);

    // The mirrored rect about (5,5) spans (-5,-5)-(15,15), clipped to the
    // canvas; (5,5) is inside and (16,16) is outside.
    QCOMPARE(f.view->selection_coverage(5, 5), 255);
    QCOMPARE(f.view->selection_coverage(16, 16), 0);
}

void SelectionAltTest::altWithNoSelectionDrawsFromTheCentre()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(64, 64, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_sel_alt_centre"));
    QVERIFY2(f.ok(), "selection fixture");
    frame.setActiveTool(pictura::ToolId::Marquee);
    auto* options = f.tools->optionsFor<pictura::SelectionOptions>(pictura::ToolId::Marquee);
    options->setCombineMode(pictura::SelectionMode::New);
    options->setMarqueeStyle(pictura::MarqueeStyle::Normal);
    f.view->deselect();
    QVERIFY(!f.view->has_selection());

    // Alt at press with no selection draws from the centre and commits New.
    f.canvas->mousePressed(QPointF(30, 30), Qt::LeftButton, int(Qt::AltModifier));
    QCOMPARE(f.tools->dragModeForTest(), int(pictura::SelectionMode::New));
    QVERIFY(f.tools->dragMirrorForTest());
    f.canvas->mouseMoved(QPointF(40, 40));
    QVERIFY(f.tools->dragMirrorForTest());
    f.canvas->mouseReleased(QPointF(40, 40));

    QCOMPARE(f.view->selection_coverage(25, 25), 255);
    QCOMPARE(f.view->selection_coverage(15, 15), 0);
}

QTEST_MAIN(SelectionAltTest)
#include "tst_selection_alt.moc"