// Smart alignment guides: the pure solver plus the Move and Type tools snapping
// content to the document and other layers (issue #737).

#include <QtTest/QtTest>

#include "api/transform_api.h"
#include "api/type_api.h"
#include "commands.h"
#include "selftest_paint_fixture.h"
#include "session.h"
#include "smart_guides.h"

#include <QtGui/QImage>

#include "qt_test_support.h"

namespace {

using paint_fixture::Fixture;

QImage seed()
{
    QImage image(200, 160, QImage::Format_RGB32);
    image.fill(Qt::white);
    image.setDotsPerMeterX(2835);
    image.setDotsPerMeterY(2835);
    return image;
}

bool hasLine(const QList<pictura::SmartGuideLine>& lines, bool vertical, double position)
{
    for (const pictura::SmartGuideLine& line : lines) {
        if (line.vertical == vertical && qAbs(line.position - position) < 0.01) {
            return true;
        }
    }
    return false;
}

QRectF contentRect(const Fixture& f, const QString& path)
{
    const pictura::transform_api::LayerRect rect = pictura::transform_api::contentRect(*f.view, path);
    return rect.valid ? QRectF(rect.left, rect.top, rect.width(), rect.height()) : QRectF();
}

QAction* action(pictura::PicturaMainWindow& frame, const char* id)
{
    frame.registry()->refresh();
    return frame.registry()->action(QString::fromLatin1(id));
}

// Copy a `size` square from the Background into a new layer at `at`.
QString addSquare(Fixture& f, const QPoint& at, int size)
{
    f.view->select_rect(at.x(), at.y(), size, size, QStringLiteral("new"), 0.0);
    const QString path = f.view->layer_via_copy(QStringLiteral("0"));
    f.view->deselect();
    return path;
}

} // namespace

class SmartGuidesTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void documentCentre();
    void smartGuidesToggle();
    void layerEdge();
    void threshold();
    void typeMove();
    void moveToolTextLayers();
    void gapReadout();
    void twoLines();

private:
    void showFrame();

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void SmartGuidesTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void SmartGuidesTest::showFrame()
{
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    window_->activateWindow();
    QCoreApplication::processEvents();
}

void SmartGuidesTest::documentCentre()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_sg_centre"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    const QString moving = addSquare(f, QPoint(60, 60), 20);
    QVERIFY2(!moving.isEmpty(), "a square layer");
    frame.selectLayerPath(moving);
    frame.setActiveTool(pictura::ToolId::Move);

    // The square's centre (70, 70) lands two pixels short of the document
    // centre (100, 80): both guides snap.
    const int base = f.view->history_index();
    f.canvas->mousePressed(QPointF(150, 140), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(QPointF(178, 148));
    const QList<pictura::SmartGuideLine> lines = f.canvas->smartGuidesForTest();
    QVERIFY2(hasLine(lines, true, 100.0), "vertical guide at the document centre");
    QVERIFY2(hasLine(lines, false, 80.0), "horizontal guide at the document centre");
    const QRectF moved = f.canvas->moveOutlineForTest();
    QCOMPARE(moved.center(), QPointF(100, 80));
    f.canvas->mouseReleased(QPointF(178, 148));

    QVERIFY2(!f.canvas->hasSmartGuidesForTest(), "guides clear on release");
    QVERIFY2(f.view->history_index() == base + 1, "one move state");
    QCOMPARE(contentRect(f, moving).center(), QPointF(100, 80));
}

void SmartGuidesTest::smartGuidesToggle()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_sg_toggle"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    const QString moving = addSquare(f, QPoint(60, 60), 20);
    QVERIFY2(!moving.isEmpty(), "a square layer");
    frame.selectLayerPath(moving);
    frame.setActiveTool(pictura::ToolId::Move);

    QAction* smart = action(frame, pictura::command_ids::ViewShowSmartGuides);
    QVERIFY2(smart && smart->isCheckable(), "a checkable View > Show item");
    QCOMPARE(smart->shortcut(), QKeySequence(QStringLiteral("Ctrl+Shift+;")));
    QVERIFY2(smart->isChecked(), "on by default");
    QVERIFY(f.canvas->smartGuidesEnabled());

    smart->trigger();
    QVERIFY2(!f.canvas->smartGuidesEnabled(), "the menu turns it off");
    QVERIFY2(!pictura::loadSession().smartGuidesVisible, "the choice is saved");

    // The drag that snaps in documentCentre keeps the pointer offset while off.
    f.canvas->mousePressed(QPointF(150, 140), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(QPointF(170, 150));
    QVERIFY2(!f.canvas->hasSmartGuidesForTest(), "no guide while off");
    QCOMPARE(f.canvas->moveOutlineForTest().center(), QPointF(90, 80));
    f.canvas->mouseReleased(QPointF(170, 150));
    QCOMPARE(contentRect(f, moving).center(), QPointF(90, 80));

    smart = action(frame, pictura::command_ids::ViewShowSmartGuides);
    smart->trigger();
    QVERIFY2(f.canvas->smartGuidesEnabled(), "the menu turns it back on");
    QVERIFY(pictura::loadSession().smartGuidesVisible);

    f.canvas->mousePressed(QPointF(150, 140), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(QPointF(158, 140));
    QVERIFY2(f.canvas->hasSmartGuidesForTest(), "guides return");
    f.canvas->mouseReleased(QPointF(158, 140));
    QCOMPARE(contentRect(f, moving).center(), QPointF(100, 80));
}

void SmartGuidesTest::layerEdge()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_sg_edge"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    const QString reference = addSquare(f, QPoint(140, 40), 20);
    const QString moving = addSquare(f, QPoint(10, 10), 20);
    QVERIFY2(!reference.isEmpty() && !moving.isEmpty(), "two square layers");
    frame.selectLayerPath(moving);
    frame.setActiveTool(pictura::ToolId::Move);

    // Drag the left edge to three pixels short of the reference's left edge.
    f.canvas->mousePressed(QPointF(150, 140), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(QPointF(277, 140));
    QVERIFY2(hasLine(f.canvas->smartGuidesForTest(), true, 140.0), "a guide at the edge");
    f.canvas->mouseReleased(QPointF(277, 140));
    QCOMPARE(contentRect(f, moving).left(), contentRect(f, reference).left());
}

void SmartGuidesTest::threshold()
{
    // Exactly eight document pixels snaps; just beyond does not.
    const QList<QRectF> none;
    QVERIFY(pictura::smartGuideSnap(QRectF(50, 8, 20, 2), none, QSizeF(200, 160), 1.0).active());
    QVERIFY(!pictura::smartGuideSnap(QRectF(50, 8.1, 20, 2), none, QSizeF(200, 160), 1.0).active());

    // Far from any target the Move tool leaves the pointer's offset alone.
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_sg_threshold"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    const QString moving = addSquare(f, QPoint(60, 60), 20);
    frame.selectLayerPath(moving);
    frame.setActiveTool(pictura::ToolId::Move);
    f.canvas->mousePressed(QPointF(150, 140), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(QPointF(140, 120));
    QVERIFY2(!f.canvas->hasSmartGuidesForTest(), "no guide far from a target");
    f.canvas->mouseReleased(QPointF(140, 120));
    QCOMPARE(contentRect(f, moving).topLeft(), QPointF(50, 40));
}

void SmartGuidesTest::typeMove()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_sg_type"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    frame.setActiveTool(pictura::ToolId::HorizontalType);
    const pictura::TypeOptions saved =
        f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get();
    pictura::TypeOptions options = saved;
    options.size = 30.0;
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(options);
    f.drag({QPointF(20, 60)});
    QTest::keyClicks(f.canvas, QStringLiteral("HH"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    const QString path = QString(f.view->active_layer_path());
    QVERIFY2(!path.isEmpty(), "a type layer");
    const QRectF before = contentRect(f, path);

    // Reopen it and drag its ink centre two pixels short of the document centre.
    f.drag({before.center()});
    QVERIFY2(f.tools->textSession() && f.tools->textSession()->textActive(), "reopened");
    const QPointF delta(100.0 - before.center().x() - 2.0, 80.0 - before.center().y() - 2.0);
    f.canvas->mousePressed(QPointF(150, 140), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(QPointF(150 + delta.x(), 140 + delta.y()));
    QVERIFY2(hasLine(f.canvas->smartGuidesForTest(), true, 100.0), "the vertical guide");
    QVERIFY2(hasLine(f.canvas->smartGuidesForTest(), false, 80.0), "the horizontal guide");
    f.canvas->mouseReleased(QPointF(150 + delta.x(), 140 + delta.y()));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    QVERIFY2(!f.tools->textSession()->textActive(), "committed");
    const QPointF centre = contentRect(f, path).center();
    QVERIFY2(qAbs(centre.x() - 100.0) <= 1.0 && qAbs(centre.y() - 80.0) <= 1.0,
             qPrintable(QStringLiteral("centred at %1,%2").arg(centre.x()).arg(centre.y())));
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(saved);
}

void SmartGuidesTest::moveToolTextLayers()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage wide(400, 300, QImage::Format_RGB32);
    wide.fill(Qt::white);
    wide.setDotsPerMeterX(2835);
    wide.setDotsPerMeterY(2835);
    Fixture f(frame, wide, QStringLiteral("pictura_sg_move_text"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    frame.setActiveTool(pictura::ToolId::HorizontalType);
    const pictura::TypeOptions saved =
        f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get();
    pictura::TypeOptions options = saved;
    options.size = 48.0;
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(options);
    f.drag({QPointF(100, 120)});
    QTest::keyClicks(f.canvas, QStringLiteral("test1"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    const QString first = QString(f.view->active_layer_path());
    f.drag({QPointF(140, 180)});
    QTest::keyClicks(f.canvas, QStringLiteral("test"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    const QString second = QString(f.view->active_layer_path());
    QVERIFY2(!first.isEmpty() && !second.isEmpty() && first != second, "two type layers");
    const QRectF a = contentRect(f, first);
    const QRectF b = contentRect(f, second);
    QVERIFY2(!a.isEmpty() && !b.isEmpty(), "two ink boxes");

    frame.selectLayerPath(second);
    frame.setActiveTool(pictura::ToolId::Move);
    const QPointF delta(a.left() - b.left() - 1.0, a.top() - b.top() - 1.0);
    f.canvas->mousePressed(QPointF(360, 260), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(QPointF(360 + delta.x(), 260 + delta.y()));
    const QList<pictura::SmartGuideLine> lines = f.canvas->smartGuidesForTest();
    QVERIFY2(hasLine(lines, true, a.left()), "a vertical guide at the first layer's left");
    QVERIFY2(hasLine(lines, false, a.top()), "a horizontal guide at the first layer's top");
    f.canvas->mouseReleased(QPointF(360 + delta.x(), 260 + delta.y()));
    const QRectF moved = contentRect(f, second);
    QCOMPARE(moved.left(), a.left());
    QCOMPARE(moved.top(), a.top());
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(saved);
}

void SmartGuidesTest::gapReadout()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_sg_gap"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    // The reference sits 36 px below the moving square, left edges aligned.
    const QString reference = addSquare(f, QPoint(10, 66), 20);
    const QString moving = addSquare(f, QPoint(10, 10), 20);
    QVERIFY2(!reference.isEmpty() && !moving.isEmpty(), "two square layers");
    frame.selectLayerPath(moving);
    frame.setActiveTool(pictura::ToolId::Move);
    f.canvas->mousePressed(QPointF(150, 140), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(QPointF(152, 140));
    QVERIFY2(f.canvas->hasSmartGuidesForTest(), "a guide");
    QCOMPARE(f.canvas->smartGuideGapForTest(), QStringLiteral("0.500 in"));
    f.canvas->mouseReleased(QPointF(152, 140));
}

void SmartGuidesTest::twoLines()
{
    // A 20-wide layer against a 10-wide reference at the same left edge: the
    // left edges coincide and the layer's centre meets the reference's right.
    const QList<QRectF> references{QRectF(100, 0, 10, 20)};
    const pictura::SmartGuideResult result =
        pictura::smartGuideSnap(QRectF(100, 0, 20, 20), references, QSizeF(200, 160), 1.0);
    QVERIFY2(hasLine(result.lines, true, 100.0), "the left edge");
    QVERIFY2(hasLine(result.lines, true, 110.0), "the centre meets the reference's right");
    int vertical = 0;
    for (const pictura::SmartGuideLine& line : result.lines) {
        vertical += line.vertical ? 1 : 0;
    }
    QCOMPARE(vertical, 2);
}

QTEST_MAIN(SmartGuidesTest)
#include "tst_smart_guides.moc"
