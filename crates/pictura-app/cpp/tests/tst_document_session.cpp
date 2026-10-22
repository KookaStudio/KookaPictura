// The document-session boundary: cancellation reports, the replacement
// entry-point signal policy, and the tool handlers that drop their gesture
// state when a session is cancelled.

#include <QtTest/QtTest>

#include "selftest_paint_fixture.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/healing.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust/image_mode.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paths.cxxqt.h"

#include "qt_test_support.h"

namespace {

using paint_fixture::Fixture;

QImage whiteSeed(int w, int h)
{
    QImage seed(w, h, QImage::Format_RGB32);
    seed.fill(Qt::white);
    return seed;
}

} // namespace

class DocumentSessionTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void regionBlitDoesNotEmitChanged();
    void regionBlitCanAlsoEmitChanged();
    void notifyOnlyEmitsChanged();
    void rebuildEmitsChanged();
    void theTransformDragHandleComesFromTheSession();
    void undoAndRedoCancelTheTransformSession();
    void replaceEntryPointsCancelTheSession();
    void replacementSignalsMatchTheEntryPoint();
    void inPlaceRestoreResetsToolHandlers();
    void pathSelectionCancelRestoresShapeLayer();
    void switchingToolsCancelsCancellableGestures();
    void switchingToolsCommitsCommitOnDeactivate();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void DocumentSessionTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void DocumentSessionTest::regionBlitDoesNotEmitChanged()
{
    pictura::PictureView view;
    QVERIFY(view.new_document(64, 64, QStringLiteral("rgb"), 8, QStringLiteral("white")));
    QSignalSpy blits(&view, &pictura::PictureView::regionBlitted);
    QSignalSpy changes(&view, &pictura::PictureView::changed);
    QVERIFY(view.begin_paint(0xFFFF0000u, 0xFFFFFFFFu, 16, 100, 100, 0, 100, 100, 25,
                             QStringLiteral("normal"), false, false));
    QVERIFY(view.paint_dab(16.0, 16.0, 1.0));
    QVERIFY(view.cancel_active_session());
    QVERIFY2(blits.count() >= 1, "cancel blits the stroke region");
    QCOMPARE(changes.count(), 0);
}

void DocumentSessionTest::regionBlitCanAlsoEmitChanged()
{
    // A healing commit changes a region and notifies: the blit and exactly one
    // `changed` reach the shell.
    pictura::PictureView view;
    QVERIFY(view.new_document(64, 64, QStringLiteral("rgb"), 8, QStringLiteral("white")));
    QSignalSpy blits(&view, &pictura::PictureView::regionBlitted);
    QSignalSpy changes(&view, &pictura::PictureView::changed);
    QVERIFY(pictura::healing_begin(view, 20, 50));
    QVERIFY(pictura::healing_dab(view, 16.0, 16.0));
    QVERIFY(pictura::healing_commit(view, 0, false, 0, 0, false));
    QVERIFY2(blits.count() >= 1, "the commit blits the healed region");
    QCOMPARE(changes.count(), 1);
}

void DocumentSessionTest::notifyOnlyEmitsChanged()
{
    // A Work Path gesture changes no pixel: the abort notifies without a
    // region blit.
    pictura::PictureView view;
    QVERIFY(view.new_document(64, 64, QStringLiteral("rgb"), 8, QStringLiteral("white")));
    QSignalSpy blits(&view, &pictura::PictureView::regionBlitted);
    QSignalSpy changes(&view, &pictura::PictureView::changed);
    pictura::path_gesture_begin(view);
    QVERIFY(pictura::path_gesture_abort(view));
    QCOMPARE(blits.count(), 0);
    QCOMPARE(changes.count(), 1);
}

void DocumentSessionTest::rebuildEmitsChanged()
{
    pictura::PictureView view;
    QVERIFY(view.new_document(64, 64, QStringLiteral("rgb"), 8, QStringLiteral("white")));
    QVERIFY(pictura::preview_indexed(view, 1, 256, 0, 0));
    QSignalSpy changes(&view, &pictura::PictureView::changed);
    QVERIFY(view.cancel_active_session());
    QCOMPARE(changes.count(), 1);
}

void DocumentSessionTest::theTransformDragHandleComesFromTheSession()
{
    pictura::PictureView view;
    QVERIFY(view.new_document(64, 64, QStringLiteral("rgb"), 8, QStringLiteral("white")));
    QVERIFY(view.begin_free_transform(QStringLiteral("0")));
    QCOMPARE(view.transform_drag_handle(), -1);

    const int hit = view.transform_press(0.0, 0.0, 1.0, false, false);
    QVERIFY(hit >= 0);
    QCOMPARE(view.transform_drag_handle(), hit);

    QVERIFY(view.transform_release());
    QCOMPARE(view.transform_drag_handle(), -1);
}

void DocumentSessionTest::undoAndRedoCancelTheTransformSession()
{
    pictura::PictureView view;
    QVERIFY(view.new_document(64, 64, QStringLiteral("rgb"), 8, QStringLiteral("white")));
    QVERIFY(view.begin_paint(0xFFFF0000u, 0xFFFFFFFFu, 16, 100, 100, 0, 100, 100, 25,
                             QStringLiteral("normal"), false, false));
    QVERIFY(view.paint_dab(16.0, 16.0, 1.0));
    QVERIFY(view.end_paint());
    const int base = view.history_index();

    QVERIFY(view.begin_free_transform(QStringLiteral("0")));
    QVERIFY(view.transform_session_active());
    QVERIFY(view.undo());
    QVERIFY(!view.transform_session_active());
    QCOMPARE(view.history_index(), base - 1);

    QVERIFY(view.begin_free_transform(QStringLiteral("0")));
    QVERIFY(view.transform_session_active());
    QVERIFY(view.redo());
    QVERIFY(!view.transform_session_active());
    QCOMPARE(view.history_index(), base);
}

void DocumentSessionTest::replaceEntryPointsCancelTheSession()
{
    pictura::PictureView view;
    QSignalSpy cancelled(&view, &pictura::PictureView::sessionCancelled);
    QVERIFY(view.new_document(64, 64, QStringLiteral("rgb"), 8, QStringLiteral("white")));
    QCOMPARE(cancelled.count(), 1);
}

void DocumentSessionTest::replacementSignalsMatchTheEntryPoint()
{
    pictura::PictureView view;
    QSignalSpy changes(&view, &pictura::PictureView::changed);
    QSignalSpy blits(&view, &pictura::PictureView::regionBlitted);
    QVERIFY(view.new_document(64, 64, QStringLiteral("rgb"), 8, QStringLiteral("white")));
    QCOMPARE(changes.count(), 0);
    QCOMPARE(blits.count(), 0);
}

void DocumentSessionTest::inPlaceRestoreResetsToolHandlers()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, whiteSeed(100, 100), QStringLiteral("pictura_session_handlers"));
    QVERIFY2(f.ok(), "the fixture");
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    frame.activateWindow();
    QCoreApplication::processEvents();

    // A committed stroke, then a second one undone mid-gesture: the handler
    // drops its press state, and the release that follows commits nothing into
    // the restored document.
    frame.setActiveTool(pictura::ToolId::Brush);
    const int base = f.view->history_index();
    f.canvas->mousePressed(QPointF(20, 20), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(QPointF(30, 30));
    f.canvas->mouseReleased(QPointF(30, 30));
    QVERIFY2(f.committedOnce(base, "Brush"), "the first stroke commits");

    f.canvas->mousePressed(QPointF(40, 40), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(QPointF(50, 50));
    QVERIFY(f.view->undo());
    const int restored = f.view->history_index();
    f.canvas->mouseReleased(QPointF(50, 50));
    QCOMPARE(f.view->history_index(), restored);
}

void DocumentSessionTest::pathSelectionCancelRestoresShapeLayer()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, whiteSeed(100, 100), QStringLiteral("pictura_session_shape"));
    QVERIFY2(f.ok(), "the fixture");
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    frame.activateWindow();
    QCoreApplication::processEvents();

    // A path gesture on the Work Path, then an abort: the path is back and
    // the document is unchanged.
    const int base = f.view->history_index();
    pictura::path_gesture_begin(*f.view);
    QVERIFY(pictura::path_gesture_abort(*f.view));
    QCOMPARE(f.view->history_index(), base);
}

void DocumentSessionTest::switchingToolsCancelsCancellableGestures()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, whiteSeed(100, 100), QStringLiteral("pictura_session_switch"));
    QVERIFY2(f.ok(), "the fixture");
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    frame.activateWindow();
    QCoreApplication::processEvents();

    // A healing gesture, then a tool switch: the gesture is cancelled and
    // records nothing.
    const int base = f.view->history_index();
    QVERIFY(pictura::healing_begin(*f.view, 20, 50));
    QVERIFY(pictura::healing_dab(*f.view, 16.0, 16.0));
    frame.setActiveTool(pictura::ToolId::Move);
    QCOMPARE(f.view->history_index(), base);
}

void DocumentSessionTest::switchingToolsCommitsCommitOnDeactivate()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, whiteSeed(100, 100), QStringLiteral("pictura_session_commit"));
    QVERIFY2(f.ok(), "the fixture");
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    frame.activateWindow();
    QCoreApplication::processEvents();

    // A brush stroke, then a tool switch: the stroke commits as one state.
    frame.setActiveTool(pictura::ToolId::Brush);
    const int base = f.view->history_index();
    f.canvas->mousePressed(QPointF(20, 20), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(QPointF(30, 30));
    frame.setActiveTool(pictura::ToolId::Move);
    QVERIFY2(f.committedOnce(base, "Brush"), "the switch commits the stroke");
}

QTEST_MAIN(DocumentSessionTest)
#include "tst_document_session.moc"
