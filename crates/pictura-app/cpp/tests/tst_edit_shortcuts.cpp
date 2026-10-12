// Keyboard paths into Edit and View commands: Delete / Backspace clearing a
// selection (#186), Ctrl+= / Ctrl+Shift+= zooming in (#187), and Ctrl+T on an
// opened photo's Background (#188).

#include <QtTest/QtTest>

#include "api/transform_api.h"
#include "commands.h"
#include "frame.h"
#include "image_view.h"
#include "panels/layers_panel.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools/fills.cxxqt.h"

#include "qt_test_support.h"

#include <QtCore/QTemporaryDir>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>
#include <QtWidgets/QLineEdit>

class EditShortcutsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void deleteAndBackspaceClearTheSelection();
    void deleteKeyDeletesTheActiveLayerAwayFromTheCanvas();
    void deleteKeyLeavesAFocusedTextFieldAlone();
    void ctrlEqualsZoomsIn();
    void freeTransformLiftsTheSelectedPixels();
    void freeTransformBoxHugsTheLayerContent();
    void aLiftedSessionSwitchesToTheSiblingAtTheFloatingPath();
    void backgroundCopyIsUnlockedAndTheLockBadgeUnlocks();

private:
    bool openPhoto(QTemporaryDir& dir);

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void EditShortcutsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    // Menu shortcuts only route to an active window.
    window_->show();
    window_->activateWindow();
    QVERIFY(QTest::qWaitForWindowActive(window_.get()));
}

void EditShortcutsTest::deleteAndBackspaceClearTheSelection()
{
    pictura::PicturaMainWindow& frame = *window_;
    for (const Qt::Key key : {Qt::Key_Delete, Qt::Key_Backspace}) {
        QVERIFY(frame.newDocument(QStringLiteral("Clear"), 40, 30, QStringLiteral("rgb"), 8,
                                  QStringLiteral("white")));
        pictura::PictureView* view = frame.activeView();
        pictura::ImageView* canvas = frame.imageView();
        QVERIFY(view && canvas);
        const int doc = frame.activeDocumentIndex();

        QVERIFY(view->select_rect(5, 5, 20, 16, QStringLiteral("new"), 0.0));
        const int selected = view->history_count();
        const QRgb outside = view->composite_argb(30, 20);
        const QRgb inside = view->composite_argb(8, 8);
        QTest::keyClick(canvas, key);
        QCOMPARE(view->history_count(), selected + 1);
        QCOMPARE(view->history_label(view->history_index()), QStringLiteral("Clear"));
        QVERIFY(view->composite_argb(8, 8) != inside);
        QCOMPARE(view->composite_argb(30, 20), outside);
        frame.closeDocument(doc, false);
    }
}

// #304: Delete / Backspace delete the selected layer from anywhere in the
// workspace, not only with the canvas focused.
void EditShortcutsTest::deleteKeyDeletesTheActiveLayerAwayFromTheCanvas()
{
    pictura::PicturaMainWindow& frame = *window_;
    auto* panel = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel);
    for (const Qt::Key key : {Qt::Key_Delete, Qt::Key_Backspace}) {
        QVERIFY(frame.newDocument(QStringLiteral("Delete"), 40, 30, QStringLiteral("rgb"), 8,
                                  QStringLiteral("white")));
        pictura::PictureView* view = frame.activeView();
        QVERIFY(view);
        const int doc = frame.activeDocumentIndex();
        QCOMPARE(view->duplicate_layer(0), 1);
        QCOMPARE(view->layer_kind(1), QStringLiteral("pixel"));
        frame.selectLayerPath(QStringLiteral("1"));
        panel->setFocus();
        const int history = view->history_count();
        QTest::keyClick(panel, key);
        QCOMPARE(view->layer_count(), 1);
        QCOMPARE(view->history_count(), history + 1);
        QCOMPARE(view->history_label(view->history_index()), QStringLiteral("Delete Layers"));
        frame.closeDocument(doc, false);
    }
}

// A focused editable field keeps Delete for its own editing.
void EditShortcutsTest::deleteKeyLeavesAFocusedTextFieldAlone()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("Field"), 40, 30, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    const int doc = frame.activeDocumentIndex();
    QCOMPARE(view->duplicate_layer(0), 1);
    frame.selectLayerPath(QStringLiteral("1"));

    QLineEdit edit(&frame);
    edit.setText(QStringLiteral("abc"));
    edit.show();
    edit.setFocus();
    QTRY_VERIFY(QApplication::focusWidget() == &edit);
    edit.setCursorPosition(1);
    QTest::keyClick(&edit, Qt::Key_Delete);
    QCOMPARE(edit.text(), QStringLiteral("ac"));
    QTest::keyClick(&edit, Qt::Key_Backspace);
    QCOMPARE(edit.text(), QStringLiteral("c"));
    QCOMPARE(view->layer_count(), 2);
    frame.closeDocument(doc, false);
}

void EditShortcutsTest::ctrlEqualsZoomsIn()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("Zoom"), 40, 30, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    QVERIFY(canvas);
    const int doc = frame.activeDocumentIndex();
    const struct {
        Qt::Key key;
        Qt::KeyboardModifiers modifiers;
    } presses[] = {
        {Qt::Key_Equal, Qt::ControlModifier},
        {Qt::Key_Plus, Qt::ControlModifier | Qt::ShiftModifier},
        {Qt::Key_Plus, Qt::ControlModifier | Qt::KeypadModifier},
        {Qt::Key_Plus, Qt::ControlModifier},
    };
    canvas->setFocus();
    for (const auto& press : presses) {
        const double before = canvas->zoom();
        QTest::keyClick(canvas, press.key, press.modifiers);
        // One step each: a press must not reach both the menu shortcut and
        // the fallback.
        QVERIFY2(canvas->zoom() > before && canvas->zoom() < before * 1.3,
                 qPrintable(QStringLiteral("key %1 mods %2")
                                .arg(press.key)
                                .arg(int(press.modifiers))));
    }
    const double zoomed = canvas->zoom();
    QTest::keyClick(canvas, Qt::Key_Minus, Qt::ControlModifier);
    QVERIFY(canvas->zoom() < zoomed);
    frame.closeDocument(doc, false);
}

// CS6: with no selection a Background cannot be free-transformed; with one,
// Ctrl+T lifts the selected pixels (the hole clears to white), Escape puts them
// back, and Return commits one "Free Transform" state that keeps the
// Background.
void EditShortcutsTest::freeTransformLiftsTheSelectedPixels()
{
    pictura::PicturaMainWindow& frame = *window_;
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString png = dir.filePath(QStringLiteral("photo.png"));
    QImage photo(40, 30, QImage::Format_RGB32);
    photo.fill(qRgb(10, 120, 200));
    QVERIFY(photo.save(png));
    QVERIFY(frame.openDocumentAtPath(png));
    QCoreApplication::processEvents();
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    QVERIFY(view && canvas);
    const int doc = frame.activeDocumentIndex();
    QCOMPARE(view->layer_kind(0), QStringLiteral("background"));
    QAction* freeTransform =
        frame.registry()->action(QLatin1String(pictura::command_ids::EditFreeTransform));
    QVERIFY(freeTransform);
    canvas->setFocus();
    const QRgb blue = qRgb(10, 120, 200);

    frame.registry()->refresh();
    QVERIFY(!freeTransform->isEnabled());
    QTest::keyClick(canvas, Qt::Key_T, Qt::ControlModifier);
    QVERIFY(!view->transform_session_active());

    QVERIFY(view->select_rect(5, 5, 20, 16, QStringLiteral("new"), 0.0));
    frame.registry()->refresh();
    QVERIFY(freeTransform->isEnabled());
    const int history = view->history_count();
    QTest::keyClick(canvas, Qt::Key_T, Qt::ControlModifier);
    QVERIFY(view->transform_session_active());
    QCOMPARE(view->layer_count(), 2);
    QTest::keyClick(canvas, Qt::Key_Escape);
    QVERIFY(!view->transform_session_active());
    QCOMPARE(view->layer_count(), 1);
    QCOMPARE(view->composite_argb(8, 8), blue);
    QCOMPARE(view->history_count(), history);

    QTest::keyClick(canvas, Qt::Key_T, Qt::ControlModifier);
    QVERIFY(view->transform_session_active());
    // Away from the reference point (the box centre) and the handles: at 2x the
    // screen-space tolerances shrink to 3 image pixels.
    QCOMPARE(view->transform_press(12.0, 10.0, 2.0, false, false), 9);
    QVERIFY(view->transform_move(22.0, 15.0, 2.0, false, false));
    view->transform_release();
    QCOMPARE(view->transform_dx(), 10.0);
    QCOMPARE(view->transform_dy(), 5.0);
    QTest::keyClick(canvas, Qt::Key_Return);
    QVERIFY(!view->transform_session_active());
    QCOMPARE(view->history_count(), history + 1);
    QCOMPARE(view->history_label(view->history_index()), QStringLiteral("Free Transform"));
    QCOMPARE(view->layer_count(), 1);
    QCOMPARE(view->layer_kind(0), QStringLiteral("background"));
    // The hole the move left is white; the moved pixels and the untouched
    // canvas are blue.
    QCOMPARE(view->composite_argb(8, 8), qRgb(255, 255, 255));
    QCOMPARE(view->composite_argb(30, 22), blue);
    QCOMPARE(view->composite_argb(2, 2), blue);
    QVERIFY(!view->has_selection());
    view->undo();
    QCOMPARE(view->composite_argb(8, 8), blue);
    frame.closeDocument(doc, false);
}

bool EditShortcutsTest::openPhoto(QTemporaryDir& dir)
{
    const QString png = dir.filePath(QStringLiteral("flower.png"));
    QImage photo(40, 30, QImage::Format_RGB32);
    photo.fill(qRgb(40, 140, 60));
    if (!dir.isValid() || !photo.save(png) || !window_->openDocumentAtPath(png)) {
        return false;
    }
    QCoreApplication::processEvents();
    return window_->activeView() != nullptr;
}

// Ctrl+T on a canvas-sized layer holding a small square boxes the square, as
// Photoshop does, not the canvas.
void EditShortcutsTest::freeTransformBoxHugsTheLayerContent()
{
    pictura::PicturaMainWindow& frame = *window_;
    QTemporaryDir dir;
    QVERIFY(openPhoto(dir));
    pictura::PictureView* view = frame.activeView();
    const int doc = frame.activeDocumentIndex();
    const int added = view->add_layer(0);
    QVERIFY(added > 0);
    const QString layer = QString::number(added);
    frame.selectLayerPath(layer);
    QVERIFY(view->select_rect(10, 8, 12, 9, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*view, qRgba(0, 0, 0, 255), -1, QStringLiteral("normal"), 100,
                               false));
    view->deselect();
    const QRgb filled = view->composite_argb(12, 10);

    frame.imageView()->setFocus();
    QTest::keyClick(frame.imageView(), Qt::Key_T, Qt::ControlModifier);
    QVERIFY(view->transform_session_active());
    QCOMPARE(view->transform_session_path(), layer);
    QCOMPARE(pictura::transform_api::quad(*view).toString(),
             QStringLiteral("10.00,8.00 22.00,8.00 22.00,17.00 10.00,17.00"));
    QTest::keyClick(frame.imageView(), Qt::Key_Escape);
    QVERIFY(!view->transform_session_active());
    QCOMPARE(view->composite_argb(12, 10), filled);
    frame.closeDocument(doc, false);
}

// While a selection on the Background is lifted, the floating layer sits at
// path 1, the pre-session path of the layer above. Beginning Free Transform on
// that layer must cancel the lift and switch to it, not treat it as the
// active session.
void EditShortcutsTest::aLiftedSessionSwitchesToTheSiblingAtTheFloatingPath()
{
    pictura::PicturaMainWindow& frame = *window_;
    QTemporaryDir dir;
    QVERIFY(openPhoto(dir));
    pictura::PictureView* view = frame.activeView();
    const int doc = frame.activeDocumentIndex();
    QCOMPARE(view->add_layer(0), 1);
    frame.selectLayerPath(QStringLiteral("1"));
    QVERIFY(view->select_rect(20, 4, 8, 8, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*view, qRgba(0, 0, 0, 255), -1, QStringLiteral("normal"), 100,
                               false));
    const QRgb photo = view->composite_argb(5, 5);

    QVERIFY(view->select_rect(2, 2, 10, 10, QStringLiteral("new"), 0.0));
    QVERIFY(view->begin_free_transform(QStringLiteral("0")));
    QCOMPARE(view->layer_count(), 3);
    QCOMPARE(view->transform_session_path(), QStringLiteral("1"));

    view->deselect();
    QVERIFY(view->begin_free_transform(QStringLiteral("1")));
    QCOMPARE(view->layer_count(), 2);
    QCOMPARE(view->transform_session_path(), QStringLiteral("1"));
    QCOMPARE(view->composite_argb(5, 5), photo);
    view->cancel_transform();
    frame.closeDocument(doc, false);
}

// Duplicating the Background makes an ordinary, unlocked layer; clicking the
// Background's lock badge unlocks it in one step.
void EditShortcutsTest::backgroundCopyIsUnlockedAndTheLockBadgeUnlocks()
{
    pictura::PicturaMainWindow& frame = *window_;
    QTemporaryDir dir;
    QVERIFY(openPhoto(dir));
    pictura::PictureView* view = frame.activeView();
    const int doc = frame.activeDocumentIndex();
    QCOMPARE(view->duplicate_layer(0), 1);
    QCOMPARE(view->layer_kind(0), QStringLiteral("background"));
    QCOMPARE(view->layer_kind(1), QStringLiteral("pixel"));
    QCOMPARE(view->layer_name(1), QStringLiteral("Background copy"));
    QCOMPARE(view->layer_lock(1), 0);

    auto* panel = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel);
    panel->setView(view);
    panel->refresh();
    QVERIFY(panel->lockBadgeLeftForTest(QStringLiteral("1")) < 0);
    const int history = view->history_count();
    QVERIFY(panel->clickLockBadgeForTest(QStringLiteral("0")));
    QCOMPARE(view->layer_kind(0), QStringLiteral("pixel"));
    QCOMPARE(view->layer_lock(0), 0);
    QCOMPARE(view->history_count(), history + 1);
    frame.closeDocument(doc, false);
}

QTEST_MAIN(EditShortcutsTest)
#include "tst_edit_shortcuts.moc"
