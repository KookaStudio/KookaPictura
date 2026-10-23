// Crop straighten (#282): committing a crop with a straighten angle rotates the
// document about the box centre and crops to the box (rotate_document_in +
// crop_document), recording one "Crop" state. The Straighten control (#102):
// a reference line drawn with the toggle or Ctrl levels the image, and the box
// shrinks inside the rotated image.

#include <QtTest/QtTest>

#include "frame.h"
#include "image_view.h"
#include "qt_test_support.h"
#include "tools.h"

#include <QtGui/QKeyEvent>
#include <QtWidgets/QApplication>

#include <cmath>

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/crop_group.cxxqt.h"

class CropStraightenTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void straightenCommitCropsToTheBox();
    void angleZeroIsAnAxisAlignedCrop();
    void referenceLineLevelsWithoutEmptyCorners();
    void holdingCtrlArmsTheLine();
    void ctrlPressDrawsALineWithoutAKeyEvent();
    void ctrlReleaseMidLineFinishesIt();
    void secondLineAddsToTheAngle();
    void classicSecondLineIsAbsolute();

private:
    // A left-to-right reference line at `degrees` from the horizontal.
    static void drawLine(pictura::ImageView* canvas, double degrees, Qt::KeyboardModifiers mods)
    {
        const QPointF a(10, 40);
        const QPointF b(110, 40 + 100 * std::tan(degrees * M_PI / 180.0));
        canvas->mousePressed(a, Qt::LeftButton, int(mods));
        canvas->mouseMoved(b);
        canvas->mouseReleased(b);
    }

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void CropStraightenTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void CropStraightenTest::straightenCommitCropsToTheBox()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("Straighten"), 40, 30, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    const int doc = frame.activeDocumentIndex();
    const int before = view->history_count();

    QVERIFY(pictura::straighten_crop(*view, 5, 4, 20, 10, 30.0, 15.0, 9.0, true, -1,
                                     QStringLiteral("Crop: Grid")));
    QCOMPARE(view->document_width(), 20);
    QCOMPARE(view->document_height(), 10);
    QCOMPARE(view->history_count(), before + 1);
    QCOMPARE(view->history_label(view->history_count() - 1), QStringLiteral("Crop: Grid"));

    frame.closeDocument(doc, false);
}

void CropStraightenTest::angleZeroIsAnAxisAlignedCrop()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("NoAngle"), 24, 16, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    const int doc = frame.activeDocumentIndex();

    QVERIFY(pictura::straighten_crop(*view, 2, 3, 10, 8, 0.0, 7.0, 7.0, true, -1, QString()));
    QCOMPARE(view->document_width(), 10);
    QCOMPARE(view->document_height(), 8);
    QCOMPARE(view->history_label(view->history_count() - 1), QStringLiteral("Crop"));

    frame.closeDocument(doc, false);
}

// Parity criterion 6: a reference line at angle θ commits a rotation of -θ
// within 0.1°, and the box fits inside the rotated image (no empty corners).
void CropStraightenTest::referenceLineLevelsWithoutEmptyCorners()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("StraightenLine"), 120, 80, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(canvas);
    QVERIFY(tools);
    auto* crop = tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop);
    const int doc = frame.activeDocumentIndex();

    crop->setClassicMode(true);
    frame.setActiveTool(pictura::ToolId::Crop);
    crop->setStraightenMode(true);
    QCOMPARE(tools->cropCursorKindForTest(QPointF(60, 40), 0), 6);  // CursorStraighten

    const double theta = 5.0;
    const QPointF a(10, 40);
    const QPointF b(110, 40 + 100 * std::tan(theta * M_PI / 180.0));
    canvas->mousePressed(a, Qt::LeftButton, int(Qt::NoModifier));
    canvas->mouseMoved(b);
    // The readout shows the rotation to come; the image holds still until the
    // release, which rotates the preview.
    QVERIFY2(canvas->dragSizeHintForTest().contains(QStringLiteral("-5.0")),
             qPrintable(canvas->dragSizeHintForTest()));
    QCOMPARE(crop->angle(), 0.0);
    QCOMPARE(canvas->cropStraightenForTest(), 0.0);
    canvas->mouseReleased(b);
    QVERIFY(canvas->dragSizeHintForTest().isEmpty());
    QVERIFY(!crop->straightenMode());
    QVERIFY(std::abs(crop->angle() + theta) < 0.1);
    QCOMPARE(canvas->cropStraightenForTest(), crop->angle());
    QVERIFY(tools->cropBoxSession()->cropActive());

    // Every corner of the box maps back inside the unrotated 120x80 canvas
    // (one pixel of slack for the integer crop rect).
    const QRect rect = tools->pendingCropRect();
    QVERIFY(rect.width() < 120 && rect.height() < 80);
    const QPointF pivot = crop->pivot();
    const double rad = crop->angle() * M_PI / 180.0;
    for (const QPointF& p : {QPointF(rect.topLeft()), QPointF(rect.right() + 1, rect.top()),
                             QPointF(rect.right() + 1, rect.bottom() + 1),
                             QPointF(rect.left(), rect.bottom() + 1)}) {
        const QPointF u = p - pivot;
        const QPointF q = pivot
            + QPointF(std::cos(rad) * u.x() + std::sin(rad) * u.y(),
                      -std::sin(rad) * u.x() + std::cos(rad) * u.y());
        QVERIFY2(q.x() >= -1.0 && q.x() <= 121.0 && q.y() >= -1.0 && q.y() <= 81.0,
                 qPrintable(QStringLiteral("corner %1,%2 uncovered").arg(q.x()).arg(q.y())));
    }

    QVERIFY(tools->cropCommitSession()->commitCrop());
    QCOMPARE(frame.activeView()->document_width(), rect.width());
    QCOMPARE(frame.activeView()->document_height(), rect.height());

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

// Holding Ctrl arms the line until the key is released; it does not disarm a
// line armed from the toggle.
void CropStraightenTest::holdingCtrlArmsTheLine()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("StraightenCtrl"), 60, 40, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(canvas);
    QVERIFY(tools);
    auto* crop = tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop);
    const int doc = frame.activeDocumentIndex();
    const auto key = [canvas](QEvent::Type type) {
        QKeyEvent event(type, Qt::Key_Control, Qt::ControlModifier);
        QApplication::sendEvent(canvas, &event);
    };

    frame.setActiveTool(pictura::ToolId::Crop);
    QVERIFY(!crop->straightenMode());
    key(QEvent::KeyPress);
    QVERIFY(crop->straightenMode());
    QCOMPARE(tools->cropCursorKindForTest(QPointF(30, 20), int(Qt::ControlModifier)), 6);
    key(QEvent::KeyRelease);
    QVERIFY(!crop->straightenMode());

    crop->setStraightenMode(true);
    key(QEvent::KeyPress);
    key(QEvent::KeyRelease);
    QVERIFY(crop->straightenMode());
    crop->setStraightenMode(false);

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

// A press carrying Ctrl draws a line even when the key event went elsewhere
// (a focused panel), so the canvas never saw it.
void CropStraightenTest::ctrlPressDrawsALineWithoutAKeyEvent()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("StraightenCtrlPress"), 120, 80,
                              QStringLiteral("rgb"), 8, QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(canvas);
    QVERIFY(tools);
    auto* crop = tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop);
    const int doc = frame.activeDocumentIndex();

    crop->setClassicMode(true);
    frame.setActiveTool(pictura::ToolId::Crop);
    QVERIFY(!crop->straightenMode());
    drawLine(canvas, 5.0, Qt::ControlModifier);
    QVERIFY(std::abs(crop->angle() + 5.0) < 0.1);
    QVERIFY(!crop->straightenMode());
    QVERIFY(tools->cropBoxSession()->cropActive());

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

// Releasing Ctrl while the line is drawn does not drop it: the line finishes
// on the mouse release, then disarms.
void CropStraightenTest::ctrlReleaseMidLineFinishesIt()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("StraightenCtrlMid"), 120, 80,
                              QStringLiteral("rgb"), 8, QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(canvas);
    QVERIFY(tools);
    auto* crop = tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop);
    const int doc = frame.activeDocumentIndex();
    const auto key = [canvas](QEvent::Type type) {
        QKeyEvent event(type, Qt::Key_Control,
                        type == QEvent::KeyPress ? Qt::ControlModifier : Qt::NoModifier);
        QApplication::sendEvent(canvas, &event);
    };

    crop->setClassicMode(true);
    frame.setActiveTool(pictura::ToolId::Crop);
    key(QEvent::KeyPress);
    QVERIFY(crop->straightenMode());
    const QPointF a(10, 40);
    const QPointF b(110, 40 + 100 * std::tan(5.0 * M_PI / 180.0));
    canvas->mousePressed(a, Qt::LeftButton, int(Qt::ControlModifier));
    canvas->mouseMoved(b);
    key(QEvent::KeyRelease);
    QVERIFY(crop->straightenMode());
    canvas->mouseReleased(b);
    QVERIFY(std::abs(crop->angle() + 5.0) < 0.1);
    QVERIFY(!crop->straightenMode());

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

// Modern: a second line levels what is on screen, so its angle adds to the session's.
void CropStraightenTest::secondLineAddsToTheAngle()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("StraightenTwice"), 120, 80, QStringLiteral("rgb"),
                              8, QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(canvas);
    QVERIFY(tools);
    auto* crop = tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop);
    const int doc = frame.activeDocumentIndex();

    crop->setClassicMode(false);
    frame.setActiveTool(pictura::ToolId::Crop);
    crop->setStraightenMode(true);
    drawLine(canvas, 5.0, Qt::NoModifier);
    QVERIFY(std::abs(crop->angle() + 5.0) < 0.1);
    const QRect first = tools->pendingCropRect();

    crop->setStraightenMode(true);
    drawLine(canvas, 3.0, Qt::NoModifier);
    QVERIFY2(std::abs(crop->angle() + 8.0) < 0.1, qPrintable(QString::number(crop->angle())));
    // The box only ever shrinks to fit.
    const QRect second = tools->pendingCropRect();
    QVERIFY(second.width() <= first.width() && second.height() <= first.height());

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

// Classic: the line is drawn on the fixed image, so the box levels absolutely.
void CropStraightenTest::classicSecondLineIsAbsolute()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("StraightenAbsolute"), 120, 80,
                              QStringLiteral("rgb"), 8, QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(canvas);
    QVERIFY(tools);
    auto* crop = tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop);
    const int doc = frame.activeDocumentIndex();

    crop->setClassicMode(true);
    frame.setActiveTool(pictura::ToolId::Crop);
    crop->setStraightenMode(true);
    drawLine(canvas, 5.0, Qt::NoModifier);
    QVERIFY(std::abs(crop->angle() + 5.0) < 0.1);
    crop->setStraightenMode(true);
    drawLine(canvas, 3.0, Qt::NoModifier);
    QVERIFY2(std::abs(crop->angle() + 3.0) < 0.1, qPrintable(QString::number(crop->angle())));

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

QTEST_MAIN(CropStraightenTest)
#include "tst_crop_straighten.moc"
