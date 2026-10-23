// The document-session boundary: cancellation reports, the replacement
// entry-point signal policy, and the tool handlers that drop their gesture
// state when a session is cancelled.

#include <QtTest/QtTest>
#include <QtCore/QFile>
#include <QtCore/QStringList>
#include <QtCore/QTemporaryDir>
#include <QtGui/QAction>
#include <QtGui/QColor>

#include "commands.h"
#include "selftest_paint_fixture.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/export.cxxqt.h"
#include "pictura_app/src/cxxqt_object/healing.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layer_target_bridge.cxxqt.h"
#include "pictura_app/src/cxxqt_object/open_jobs.cxxqt.h"
#include "pictura_app/src/cxxqt_object/pdf_export.cxxqt.h"
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
    void historyNavigationMarksDirty();
    void failedRevertKeepsDocument();
    void revertIsUndoable();
    void revertUnavailableWhenUntitled();
    void snapshotRestoreMovesCursor();
    void snapshotRestoreAfterUndoDropsRedo();
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

    // Revert is an in-place entry point: one full-change notification and its
    // one `Revert` state, never a second capture.
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = dir.filePath(QStringLiteral("signals.psd"));
    pictura::PictureView reverted;
    QVERIFY(reverted.new_document(64, 64, QStringLiteral("rgb"), 8, QStringLiteral("white")));
    QVERIFY(reverted.save(path));
    QSignalSpy revertChanges(&reverted, &pictura::PictureView::changed);
    QVERIFY(pictura::revert(reverted, path));
    QCOMPARE(revertChanges.count(), 1);
    QCOMPARE(reverted.history_label(reverted.history_index()), QStringLiteral("Revert"));
}

void DocumentSessionTest::historyNavigationMarksDirty()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = dir.filePath(QStringLiteral("dirty.psd"));

    pictura::PictureView view;
    QVERIFY(view.new_document(64, 64, QStringLiteral("rgb"), 8, QStringLiteral("white")));
    QVERIFY(view.save(path));
    QVERIFY(!view.is_dirty());

    view.select_all();
    QVERIFY(view.is_dirty());
    const int edit = view.history_index();

    QVERIFY(view.undo());
    QVERIFY(!view.is_dirty());
    QVERIFY(view.redo());
    QVERIFY(view.is_dirty());

    QVERIFY(view.history_jump(0));
    QVERIFY(!view.is_dirty());
    QVERIFY(view.history_jump(edit));
    QVERIFY(view.is_dirty());

    QVERIFY(view.history_add_snapshot(QStringLiteral("Snap")));
    QVERIFY(view.history_restore_snapshot(0));
    QVERIFY(view.is_dirty());
}

void DocumentSessionTest::failedRevertKeepsDocument()
{
    pictura::PicturaMainWindow& frame = *window_;
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = dir.filePath(QStringLiteral("revert-fail.psd"));
    QVERIFY(frame.newDocument(QStringLiteral("Revert"), 64, 32, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    QVERIFY(frame.saveActiveAs(path));
    QVERIFY(view->rotate_doc(1));
    QVERIFY(view->select_rect(4, 4, 8, 8, QStringLiteral("new"), 0.0));

    QVERIFY(view->begin_paint(0xFFFF0000u, 0xFFFFFFFFu, 16, 100, 100, 0, 100, 100, 25,
                              QStringLiteral("normal"), false, false));
    QVERIFY(view->paint_dab(16.0, 16.0, 1.0));
    const int width = view->document_width();
    const int height = view->document_height();
    const int history = view->history_count();
    const bool dirty = view->is_dirty();
    const QColor pixel = QColor::fromRgba(view->sample_argb(0, 0));

    QFile file(path);
    QVERIFY(file.open(QIODevice::WriteOnly | QIODevice::Truncate));
    file.close();

    QVERIFY(!frame.revertActive());
    QCOMPARE(view->document_width(), width);
    QCOMPARE(view->document_height(), height);
    QCOMPARE(view->history_count(), history);
    QCOMPARE(view->is_dirty(), dirty);
    QCOMPARE(QColor::fromRgba(view->sample_argb(0, 0)), pixel);
    QCOMPARE(view->file_path(), path);
    QVERIFY(view->has_selection());
    QVERIFY(view->is_painting());
    view->cancel_paint();
    frame.closeDocument(frame.activeDocumentIndex(), false);
}

void DocumentSessionTest::revertIsUndoable()
{
    pictura::PicturaMainWindow& frame = *window_;
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = dir.filePath(QStringLiteral("revert-ok.psd"));
    const QString pdfPath = dir.filePath(QStringLiteral("revert-options.pdf"));
    QVERIFY(frame.newDocument(QStringLiteral("Revert"), 64, 32, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    QVERIFY(frame.saveActiveAs(path));
    QVERIFY(pictura::pdf_save(*view, pdfPath, pictura::pdf_default_options()).isEmpty());
    const QString options = pictura::pdf_view_options(*view);
    QVERIFY(!options.isEmpty());
    QVERIFY(view->save(path));
    QVERIFY(view->rotate_doc(1));
    QVERIFY(view->history_add_snapshot(QStringLiteral("Snap")));
    const int history = view->history_count();
    const int snapshots = view->history_snapshot_count();
    QCOMPARE(view->document_width(), 32);
    QCOMPARE(view->document_height(), 64);
    QVERIFY(view->is_dirty());
    QVERIFY(view->begin_paint(0xFFFF0000u, 0xFFFFFFFFu, 16, 100, 100, 0, 100, 100, 25,
                              QStringLiteral("normal"), false, false));
    QVERIFY(view->paint_dab(16.0, 16.0, 1.0));
    QVERIFY(view->is_painting());

    QVERIFY(frame.revertActive());
    QVERIFY(!view->is_painting());
    QCOMPARE(view->history_count(), history + 1);
    QCOMPARE(view->history_label(view->history_index()), QStringLiteral("Revert"));
    QCOMPARE(view->history_snapshot_count(), snapshots);
    QCOMPARE(view->document_width(), 64);
    QCOMPARE(view->document_height(), 32);
    QVERIFY(!view->is_dirty());
    QCOMPARE(view->file_path(), path);
    QCOMPARE(pictura::output_format(*view), QStringLiteral("psd"));
    QCOMPARE(pictura::pdf_view_options(*view), options);

    QVERIFY(view->undo());
    QCOMPARE(view->document_width(), 32);
    QCOMPARE(view->document_height(), 64);
    QVERIFY(view->is_dirty());
    QVERIFY(view->redo());
    QCOMPARE(view->document_width(), 64);
    QCOMPARE(view->document_height(), 32);
    QVERIFY(!view->is_dirty());
    frame.closeDocument(frame.activeDocumentIndex(), false);
}

void DocumentSessionTest::revertUnavailableWhenUntitled()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("Untitled"), 16, 16, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    frame.registry()->refresh();
    QAction* revert =
        frame.registry()->action(QString::fromLatin1(pictura::command_ids::FileRevert));
    QVERIFY(revert && !revert->isEnabled());
    QVERIFY(!frame.revertActive());
    frame.closeDocument(frame.activeDocumentIndex(), false);

    // A raster import records its source path only on the tab. It stays an
    // untitled document, so Revert must remain unavailable.
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString png = dir.filePath(QStringLiteral("import.png"));
    QImage image(8, 8, QImage::Format_RGB32);
    image.fill(Qt::white);
    QVERIFY(image.save(png));
    QVERIFY(frame.openImagePath(png));
    frame.registry()->refresh();
    revert = frame.registry()->action(QString::fromLatin1(pictura::command_ids::FileRevert));
    QVERIFY(revert && !revert->isEnabled());
    QVERIFY(!frame.revertActive());
    frame.closeDocument(frame.activeDocumentIndex(), false);
}

void DocumentSessionTest::snapshotRestoreMovesCursor()
{
    pictura::PictureView view;
    QVERIFY(view.new_document(64, 64, QStringLiteral("rgb"), 8, QStringLiteral("white")));
    const int added = view.add_layer(0);
    QVERIFY(added > 0);
    const QString top = QString::number(added);
    pictura::select_layer_path(view, QStringLiteral("0"));
    QVERIFY(view.select_rect(10, 10, 20, 20, QStringLiteral("new"), 0.0));
    QVERIFY(view.history_add_snapshot(QStringLiteral("A")));
    const QStringList snapshotTarget = pictura::selected_layer_paths(view);
    QCOMPARE(snapshotTarget, QStringList{QStringLiteral("0")});
    view.deselect();
    pictura::select_layer_path(view, top);
    QVERIFY(!view.has_selection());
    const int before = view.history_index();

    QVERIFY(view.history_restore_snapshot(0));
    QCOMPARE(view.history_index(), before + 1);
    QCOMPARE(view.history_label(view.history_index()), QStringLiteral("A"));
    QVERIFY(view.has_selection());
    QCOMPARE(pictura::selected_layer_paths(view), snapshotTarget);

    QVERIFY(view.undo());
    QCOMPARE(view.history_index(), before);
    QVERIFY(!view.has_selection());
    QVERIFY(view.redo());
    QCOMPARE(view.history_index(), before + 1);
    QVERIFY(view.has_selection());
    QCOMPARE(pictura::selected_layer_paths(view), snapshotTarget);
}

void DocumentSessionTest::snapshotRestoreAfterUndoDropsRedo()
{
    pictura::PictureView view;
    QVERIFY(view.new_document(64, 64, QStringLiteral("rgb"), 8, QStringLiteral("white")));
    QVERIFY(view.select_rect(10, 10, 20, 20, QStringLiteral("new"), 0.0));
    QVERIFY(view.history_add_snapshot(QStringLiteral("A")));
    view.deselect();

    QVERIFY(view.undo());
    QCOMPARE(view.history_index(), 1);
    QVERIFY(view.history_restore_snapshot(0));
    QCOMPARE(view.history_count(), 3);
    QCOMPARE(view.history_index(), 2);
    QCOMPARE(view.history_label(2), QStringLiteral("A"));

    QVERIFY(view.undo());
    QCOMPARE(view.history_index(), 1);
    QVERIFY(view.redo());
    QCOMPARE(view.history_index(), 2);
    QCOMPARE(view.history_label(2), QStringLiteral("A"));
    QVERIFY(!view.redo());
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
