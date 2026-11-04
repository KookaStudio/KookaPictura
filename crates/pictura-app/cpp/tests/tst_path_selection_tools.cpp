// The path selection tools: Path Selection (#41) and Direct Selection (#42).

#include <QtTest/QtTest>

#include "selftest_paint_fixture.h"

#include "api/layers_bridge.h"

#include <QtGui/QKeyEvent>
#include <QtWidgets/QCheckBox>

#include "qt_test_support.h"

namespace {

using paint_fixture::Fixture;

QPointF anchor(const Fixture& f, int sp, int pt)
{
    const ::rust::Vec<double> p = pictura::path_point(*f.view, sp, pt);
    return p.size() == 9 ? QPointF(p[0], p[1]) : QPointF(-1, -1);
}

QPointF handle(const Fixture& f, int sp, int pt, bool out)
{
    const ::rust::Vec<double> p = pictura::path_point(*f.view, sp, pt);
    return p.size() == 9 ? (out ? QPointF(p[6], p[7]) : QPointF(p[3], p[4])) : QPointF(-1, -1);
}

void click(const Fixture& f, const QPointF& p, Qt::KeyboardModifiers mods = Qt::NoModifier)
{
    f.drag({p}, mods);
}

void sendKey(pictura::PicturaMainWindow& frame, int key, Qt::KeyboardModifiers mods,
             const QString& text)
{
    QKeyEvent event(QEvent::KeyPress, key, mods, text);
    QApplication::sendEvent(&frame, &event);
}

// A closed 40 x 40 square at (10, 10) drawn with the Pen; the corner at
// (50, 50) is dragged smooth, its handles at (40, 50) and (60, 50).
void drawSquare(pictura::PicturaMainWindow& frame, const Fixture& f)
{
    frame.setActiveTool(pictura::ToolId::Pen);
    click(f, QPointF(10, 10));
    click(f, QPointF(50, 10));
    f.drag({QPointF(50, 50), QPointF(55, 50), QPointF(60, 50)});
    click(f, QPointF(10, 50));
    click(f, QPointF(10, 10));
}

} // namespace

class PathSelectionToolsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void pathSelectionTool();
    void directSelectionTool();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void PathSelectionToolsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void PathSelectionToolsTest::pathSelectionTool()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(100, 100, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_path_selection_seed"));
    QVERIFY2(f.ok(), "path selection fixture");

    // A and Shift+A cycle the two tools.
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    frame.activateWindow();
    QCoreApplication::processEvents();
    frame.setActiveTool(pictura::ToolId::Move);
    sendKey(frame, Qt::Key_A, Qt::NoModifier, QStringLiteral("a"));
    QCOMPARE(frame.activeTool(), pictura::ToolId::PathSelection);
    sendKey(frame, Qt::Key_A, Qt::ShiftModifier, QStringLiteral("A"));
    QCOMPARE(frame.activeTool(), pictura::ToolId::DirectSelection);
    sendKey(frame, Qt::Key_A, Qt::ShiftModifier, QStringLiteral("A"));
    QCOMPARE(frame.activeTool(), pictura::ToolId::PathSelection);

    // The square, and an open line beside it.
    drawSquare(frame, f);
    click(f, QPointF(70, 80));
    click(f, QPointF(90, 80));
    frame.setActiveTool(pictura::ToolId::PathSelection);
    QCOMPARE(pictura::path_subpath_count(*f.view), 2);
    QCOMPARE(f.canvas->pathOverlayAnchorCountForTest(), 0);

    // A click inside the square selects it whole, recording nothing.
    const int base = f.view->history_index();
    click(f, QPointF(30, 30));
    QCOMPARE(f.view->history_index(), base);
    QCOMPARE(f.canvas->pathOverlayAnchorCountForTest(), 4);
    QVERIFY(f.canvas->pathOverlayAnchorsSolidForTest());

    // A drag moves only that component, handles and all, as one state.
    f.drag({QPointF(30, 30), QPointF(35, 32), QPointF(40, 35)});
    QVERIFY2(f.committedOnce(base, "Drag Path"), "drag");
    QCOMPARE(anchor(f, 0, 0), QPointF(20, 15));
    QCOMPARE(handle(f, 0, 2, true), QPointF(70, 55));
    QCOMPARE(anchor(f, 1, 0), QPointF(70, 80));

    // Show Bounding Box frames the selected component.
    auto* box = frame.findChild<QCheckBox*>(QStringLiteral("optionsPathShowBoundingBox"));
    QVERIFY2(box && !box->isChecked(), "path selection bar");
    QVERIFY(f.canvas->pathOverlayBoundsForTest().isNull());
    box->setChecked(true);
    const QRectF bounds = f.canvas->pathOverlayBoundsForTest();
    QCOMPARE(bounds.topLeft(), QPointF(20, 15));
    QVERIFY2(bounds.right() > 60 && bounds.right() < 70, "the bounds missed the curve");
    QCOMPARE(bounds.bottom(), 55.0);
    box->setChecked(false);

    // Alt-drag leaves the original and drags a copy.
    f.drag({QPointF(40, 35), QPointF(40, 40), QPointF(40, 45)}, Qt::AltModifier);
    QVERIFY2(f.committedOnce(base + 1, "Duplicate Path Component"), "alt-drag");
    QCOMPARE(pictura::path_subpath_count(*f.view), 3);
    QCOMPARE(anchor(f, 0, 0), QPointF(20, 15));
    QCOMPARE(anchor(f, 2, 0), QPointF(20, 25));

    // Delete removes the selected copy; the line can be picked by its stroke.
    sendKey(frame, Qt::Key_Delete, Qt::NoModifier, QString());
    QVERIFY2(f.committedOnce(base + 2, "Delete Path"), "delete");
    QCOMPARE(pictura::path_subpath_count(*f.view), 2);
    QCOMPARE(f.canvas->pathOverlayAnchorCountForTest(), 0);
    click(f, QPointF(80, 81));
    QCOMPARE(f.canvas->pathOverlayAnchorCountForTest(), 2);

    // Empty canvas deselects; Delete then leaves the path alone.
    click(f, QPointF(95, 5));
    QCOMPARE(f.canvas->pathOverlayAnchorCountForTest(), 0);
    sendKey(frame, Qt::Key_Delete, Qt::NoModifier, QString());
    QCOMPARE(pictura::path_subpath_count(*f.view), 2);

    // Undo brings the copy back.
    QVERIFY(f.view->undo());
    QCOMPARE(pictura::path_subpath_count(*f.view), 3);
}

void PathSelectionToolsTest::directSelectionTool()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(100, 100, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_direct_selection_seed"));
    QVERIFY2(f.ok(), "direct selection fixture");
    drawSquare(frame, f);
    frame.setActiveTool(pictura::ToolId::DirectSelection);
    QCOMPARE(f.canvas->pathOverlayAnchorCountForTest(), 0);

    // Dragging an anchor moves just it and shows its component, the anchor solid.
    const int base = f.view->history_index();
    f.drag({QPointF(10, 10), QPointF(12, 11), QPointF(15, 12)});
    QVERIFY2(f.committedOnce(base, "Drag Anchor Point"), "anchor drag");
    QCOMPARE(anchor(f, 0, 0), QPointF(15, 12));
    QCOMPARE(anchor(f, 0, 1), QPointF(50, 10));
    QCOMPARE(f.canvas->pathOverlayAnchorCountForTest(), 4);
    QCOMPARE(f.canvas->pathOverlayActiveAnchorForTest(), 0);
    QVERIFY(!f.canvas->pathOverlayAnchorsSolidForTest());

    // A smooth anchor carries its handles.
    f.drag({QPointF(50, 50), QPointF(50, 55), QPointF(50, 60)});
    QVERIFY2(f.committedOnce(base + 1, "Drag Anchor Point"), "smooth anchor drag");
    QCOMPARE(handle(f, 0, 2, false), QPointF(40, 60));
    QCOMPARE(handle(f, 0, 2, true), QPointF(60, 60));

    // Dragging one handle of a smooth point swings the other through the anchor.
    f.drag({QPointF(60, 60), QPointF(55, 65), QPointF(50, 70)});
    QVERIFY2(f.committedOnce(base + 2, "Drag Direction Point"), "handle drag");
    QCOMPARE(handle(f, 0, 2, true), QPointF(50, 70));
    QCOMPARE(handle(f, 0, 2, false), QPointF(50, 50));

    // A click records nothing; Alt-click inside picks the whole component,
    // which Delete then removes.
    click(f, QPointF(50, 10));
    QCOMPARE(f.view->history_index(), base + 3);
    click(f, QPointF(30, 30), Qt::AltModifier);
    QVERIFY(f.canvas->pathOverlayAnchorsSolidForTest());
    QCOMPARE(f.view->history_index(), base + 3);
    sendKey(frame, Qt::Key_Delete, Qt::NoModifier, QString());
    QVERIFY2(f.committedOnce(base + 3, "Delete Path"), "delete");
    QCOMPARE(pictura::path_subpath_count(*f.view), 0);
}

QTEST_MAIN(PathSelectionToolsTest)
#include "tst_path_selection_tools.moc"
