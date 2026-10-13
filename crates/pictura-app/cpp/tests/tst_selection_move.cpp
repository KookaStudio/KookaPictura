// Move-selection drag (#311): the drag shifts the committed ants by the drag
// delta instead of retracing the translated mask on every move; release or
// cancel drops the offset and shows the committed contour.

#include <QtTest/QtTest>

#include "selftest_paint_fixture.h"

#include "api/selection_api.h"
#include "qt_test_support.h"

namespace {
using paint_fixture::Fixture;

bool contourHasCorner(const pictura::ImageView& canvas, const QPointF& corner)
{
    for (const QPolygonF& loop : canvas.selectionContours()) {
        if (loop.contains(corner)) {
            return true;
        }
    }
    return false;
}
} // namespace

class SelectionMoveTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void dragOffsetsTheContourAndReleaseRetraces();
    void cancelDropsTheOffset();

private:
    void startMarquee(Fixture& f);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void SelectionMoveTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void SelectionMoveTest::startMarquee(Fixture& f)
{
    window_->setActiveTool(pictura::ToolId::Marquee);
    auto* options = f.tools->optionsFor<pictura::SelectionOptions>(pictura::ToolId::Marquee);
    options->setCombineMode(pictura::SelectionMode::New);
    options->setMarqueeStyle(pictura::MarqueeStyle::Normal);
    options->setFeather(0.0);
}

void SelectionMoveTest::dragOffsetsTheContourAndReleaseRetraces()
{
    QImage seed(64, 64, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(*window_, seed, QStringLiteral("pictura_sel_move_drag"));
    QVERIFY2(f.ok(), "selection fixture");
    startMarquee(f);
    QVERIFY(f.view->select_rect(10, 10, 20, 20, QStringLiteral("new"), 0.0));
    window_->refreshSelectionOverlay();
    QVERIFY(contourHasCorner(*f.canvas, QPointF(10, 10)));
    const int base = f.view->history_count();

    f.canvas->mousePressed(QPointF(15, 15), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(QPointF(25, 22));
    // The mask moves live; the ants are the origin contour drawn offset.
    QCOMPARE(pictura::selection_api::bounds(*f.view).toString(), QStringLiteral("20 17 20 20"));
    QCOMPARE(f.canvas->selectionContourOffsetForTest(), QPointF(10, 7));
    QVERIFY(contourHasCorner(*f.canvas, QPointF(10, 10)));

    f.canvas->mouseReleased(QPointF(25, 22));
    QCOMPARE(f.view->history_count(), base + 1);
    QCOMPARE(f.canvas->selectionContourOffsetForTest(), QPointF());
    QVERIFY(contourHasCorner(*f.canvas, QPointF(20, 17)));
    QVERIFY(!contourHasCorner(*f.canvas, QPointF(10, 10)));
}

void SelectionMoveTest::cancelDropsTheOffset()
{
    QImage seed(64, 64, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(*window_, seed, QStringLiteral("pictura_sel_move_cancel"));
    QVERIFY2(f.ok(), "selection fixture");
    startMarquee(f);
    QVERIFY(f.view->select_rect(10, 10, 20, 20, QStringLiteral("new"), 0.0));
    window_->refreshSelectionOverlay();
    const int base = f.view->history_count();

    f.canvas->mousePressed(QPointF(15, 15), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(QPointF(25, 22));
    QCOMPARE(f.canvas->selectionContourOffsetForTest(), QPointF(10, 7));

    // Switching tools mid-drag cancels the move: origin mask, no offset.
    window_->setActiveTool(pictura::ToolId::EllipticalMarquee);
    QCOMPARE(f.canvas->selectionContourOffsetForTest(), QPointF());
    QCOMPARE(pictura::selection_api::bounds(*f.view).toString(), QStringLiteral("10 10 20 20"));
    QCOMPARE(f.view->history_count(), base);
    QVERIFY(contourHasCorner(*f.canvas, QPointF(10, 10)));
}

QTEST_MAIN(SelectionMoveTest)
#include "tst_selection_move.moc"
