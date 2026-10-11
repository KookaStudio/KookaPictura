// Crop modal session (#282): each released drag records a (box, angle) step;
// Edit Undo/Redo walks the session before the document history; Cancel and a
// tool switch discard it.

#include <QtTest/QtTest>

#include "frame.h"
#include "image_view.h"
#include "tools.h"

#include "commands.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include "qt_test_support.h"

#include <QtGui/QKeyEvent>
#include <QtWidgets/QApplication>

class CropSessionTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void sessionRecordsAndWalksSteps();
    void escapeDiscardsTheSession();
    void commandUndoReachesTheSession();
    void cancelReturnsToInitMode();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;

    void drag(pictura::ImageView* canvas, const QPointF& from, const QPointF& to)
    {
        canvas->mousePressed(from, Qt::LeftButton, int(Qt::NoModifier));
        canvas->mouseMoved(to);
        canvas->mouseReleased(to);
    }

    static void sendKey(pictura::PicturaMainWindow& frame, Qt::Key key)
    {
        QKeyEvent event(QEvent::KeyPress, key, Qt::NoModifier);
        QApplication::sendEvent(&frame, &event);
    }
};

void CropSessionTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void CropSessionTest::sessionRecordsAndWalksSteps()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("Session"), 40, 40, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(canvas);
    QVERIFY(tools);
    const int doc = frame.activeDocumentIndex();

    canvas->actualPixels();
    tools->setCropClassicMode(true);  // boxless start: draw a box
    frame.setActiveTool(pictura::ToolId::Crop);
    QVERIFY2(!tools->canToolUndo(), "a fresh box is the session base");

    drag(canvas, QPointF(0, 0), QPointF(30, 30));  // draw (0,0,30,30)
    QVERIFY(tools->pendingCropRect() == QRect(0, 0, 30, 30));
    QVERIFY(tools->canToolUndo());

    drag(canvas, QPointF(30, 30), QPointF(20, 20));  // resize -> (0,0,20,20)
    QVERIFY(tools->pendingCropRect() == QRect(0, 0, 20, 20));

    QVERIFY(tools->toolUndo());
    QVERIFY(tools->pendingCropRect() == QRect(0, 0, 30, 30));
    QVERIFY(tools->toolRedo());
    QVERIFY(tools->pendingCropRect() == QRect(0, 0, 20, 20));

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

void CropSessionTest::escapeDiscardsTheSession()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("Discard"), 40, 40, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(canvas);
    QVERIFY(tools);
    const int doc = frame.activeDocumentIndex();

    canvas->actualPixels();
    frame.setActiveTool(pictura::ToolId::Crop);
    drag(canvas, QPointF(40, 40), QPointF(20, 20));
    QVERIFY(tools->canToolUndo());

    sendKey(frame, Qt::Key_Escape);
    QVERIFY2(!tools->canToolUndo(), "Escape discards the session");
    QVERIFY(!frame.hasPendingCrop());

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

void CropSessionTest::commandUndoReachesTheSession()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("CommandUndo"), 40, 40, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(canvas);
    QVERIFY(tools);
    const int doc = frame.activeDocumentIndex();

    canvas->actualPixels();
    tools->setCropClassicMode(true);  // boxless start: draw a box
    frame.setActiveTool(pictura::ToolId::Crop);
    drag(canvas, QPointF(0, 0), QPointF(30, 30));
    drag(canvas, QPointF(30, 30), QPointF(20, 20));

    // The Edit Undo command is enabled by the session, so Ctrl+Z reaches it.
    bool undoEnabled = false;
    for (const pictura::CommandInfo& info : frame.registry()->describe()) {
        if (info.id == QLatin1String(pictura::command_ids::EditUndo)) {
            undoEnabled = info.enabled;
        }
    }
    QVERIFY2(undoEnabled, "Edit Undo is enabled during a crop session");
    QVERIFY(frame.registry()->dispatch(QString::fromLatin1(pictura::command_ids::EditUndo)));
    QVERIFY(tools->pendingCropRect() == QRect(0, 0, 30, 30));
    QVERIFY(frame.registry()->dispatch(QString::fromLatin1(pictura::command_ids::EditRedo)));
    QVERIFY(tools->pendingCropRect() == QRect(0, 0, 20, 20));

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

void CropSessionTest::cancelReturnsToInitMode()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("InitMode"), 40, 40, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(canvas);
    QVERIFY(tools);
    const int doc = frame.activeDocumentIndex();

    canvas->actualPixels();
    tools->setCropClassicMode(true);
    frame.setActiveTool(pictura::ToolId::Crop);
    tools->setCropRatio(1.0);
    drag(canvas, QPointF(40, 40), QPointF(30, 20));
    QVERIFY(tools->hasPendingCrop());

    // Cancel drops the drawn box and stays in the crop tool (no box shown).
    sendKey(frame, Qt::Key_Escape);
    QVERIFY2(!tools->hasPendingCrop(), "Cancel clears the crop box");
    QVERIFY(!canvas->hasCropBoxForTest());

    // A fresh drag draws a new box, honoring the active aspect ratio.
    drag(canvas, QPointF(30, 30), QPointF(10, 10));
    const QRect box = tools->pendingCropRect();
    QVERIFY(box.width() > 0);
    QCOMPARE(box.width(), box.height());

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

QTEST_MAIN(CropSessionTest)
#include "tst_crop_session.moc"
