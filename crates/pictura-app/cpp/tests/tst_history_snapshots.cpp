#include <QtTest/QtTest>

#include "frame.h"

#include "api/document_bridge.h"
#include "api/history_api.h"
#include "api/paint_bridge.h"

#include "qt_test_support.h"

// Named snapshots share their tiles with the document (#1215): taking one,
// editing on, and restoring it must bring back the snapshot's pixels exactly,
// record one state, and leave the edit undoable.
class HistorySnapshotsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void snapshotEditRestoreRoundTrip();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void HistorySnapshotsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void HistorySnapshotsTest::snapshotEditRestoreRoundTrip()
{
    namespace history = pictura::history_api;
    pictura::PicturaMainWindow& frame = *window_;
    // 600² spans three 256² tiles a side, so the fills cross tile edges.
    const bool created = frame.newDocument(QStringLiteral("SnapshotCtl"), 600, 600,
                                           QStringLiteral("rgb"), 8, QStringLiteral("white"));
    pictura::PictureView* view = frame.activeView();
    QVERIFY2(created && view, "snapshot fixture");
    const int doc = frame.activeDocumentIndex();
    const auto fill = [view](int x, int y, int w, int h, quint32 argb) {
        view->select_rect(x, y, w, h, QStringLiteral("new"), 0.0);
        const bool filled = pictura::edit_fill(*view, argb, -1, QStringLiteral("normal"), 100, false);
        view->deselect();
        return filled;
    };

    QVERIFY(fill(200, 200, 200, 200, 0xff2244aau));
    const quint32 inside = view->sample_argb(300, 300);
    const quint32 outside = view->sample_argb(10, 10);
    QVERIFY(inside != outside);
    QVERIFY(history::addSnapshot(*view, QStringLiteral("Snap")));
    QCOMPARE(history::snapshotCount(*view), 1);

    const quint32 green = 0xff00aa00u;
    QVERIFY(fill(0, 0, 300, 300, green));
    QCOMPARE(view->sample_argb(10, 10), green);
    QCOMPARE(view->sample_argb(250, 250), green);
    QCOMPARE(view->sample_argb(350, 350), inside);

    const int states = history::count(*view);
    QVERIFY(history::restoreSnapshot(*view, 0));
    QCOMPARE(history::count(*view), states + 1);
    QCOMPARE(history::label(*view, states), QStringLiteral("Snap"));
    QCOMPARE(view->sample_argb(10, 10), outside);
    QCOMPARE(view->sample_argb(250, 250), inside);
    QCOMPARE(view->sample_argb(300, 300), inside);
    QCOMPARE(view->sample_argb(599, 599), outside);

    QVERIFY(history::undo(*view));
    QCOMPARE(view->sample_argb(10, 10), green);
    QCOMPARE(view->sample_argb(250, 250), green);
    QVERIFY(history::redo(*view));
    QCOMPARE(view->sample_argb(250, 250), inside);
    QCOMPARE(history::snapshotCount(*view), 1);

    frame.closeDocument(doc, false);
}

QTEST_MAIN(HistorySnapshotsTest)
#include "tst_history_snapshots.moc"
