// Paragraph type (#682): a Type tool drag sets a box the text wraps to, an
// Alt-click asks for its size, its handles resize and reflow it, and the Type
// menu converts between point and paragraph type.

#include <QtTest/QtTest>

#include "api/transform_api.h"
#include "api/type_api.h"
#include "commands.h"
#include "selftest_paint_fixture.h"
#include "type_frame.h"

#include <QtWidgets/QDialog>
#include <QtWidgets/QDoubleSpinBox>

#include <functional>
#include <memory>

#include "qt_test_support.h"

namespace {

using paint_fixture::Fixture;

QImage seed()
{
    QImage image(320, 240, QImage::Format_RGB32);
    image.fill(Qt::white);
    return image;
}

QRect layerRect(const Fixture& f, const QString& path)
{
    const QStringList parts = pictura::transform_api::layerRect(*f.view, path).toString().split(QLatin1Char(' '));
    if (parts.size() != 4) {
        return QRect();
    }
    return QRect(QPoint(parts[0].toInt(), parts[1].toInt()),
                 QPoint(parts[2].toInt() - 1, parts[3].toInt() - 1));
}

QString typeLayer(const Fixture& f)
{
    for (int i = 0; i < f.view->layer_count(); ++i) {
        if (pictura::type_api::isType(*f.view, QString::number(i))) {
            return QString::number(i);
        }
    }
    return QString();
}

bool sameRect(const QRectF& a, const QRectF& b)
{
    return qAbs(a.left() - b.left()) < 0.01 && qAbs(a.top() - b.top()) < 0.01
        && qAbs(a.width() - b.width()) < 0.01 && qAbs(a.height() - b.height()) < 0.01;
}

bool typing(const Fixture& f)
{
    return f.tools->textSession() && f.tools->textSession()->textActive();
}

// Answers the next modal dialog (polled from its own event loop) with
// `answer` and counts it in `*seen`.
std::unique_ptr<QTimer> answerNextModal(std::function<void(QDialog*)> answer, int* seen)
{
    auto timer = std::make_unique<QTimer>();
    timer->setInterval(5);
    QTimer* raw = timer.get();
    QObject::connect(raw, &QTimer::timeout, [raw, answer, seen]() {
        if (auto* dialog = qobject_cast<QDialog*>(QApplication::activeModalWidget())) {
            raw->stop();
            ++*seen;
            answer(dialog);
        }
    });
    timer->start();
    return timer;
}

} // namespace

class ParagraphTypeTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void dragMakesParagraphType();
    void altClickAsksForTheBoxSize();
    void handlesResizeAndOverflow();
    void convertCommands();
    void frameGestureMath();
    void boxRotatesWithReadout();
    void resizeShowsSizeReadout();
    void pivotDrags();
    void ctrlScalesPointType();

private:
    // Type `text` into a 120 x 80 box dragged at (20, 30) with Horizontal Type.
    void typeIntoBox(const Fixture& f, const QString& text);
    void showFrame();

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void ParagraphTypeTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void ParagraphTypeTest::showFrame()
{
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    window_->activateWindow();
    QCoreApplication::processEvents();
}

void ParagraphTypeTest::dragMakesParagraphType()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_paragraph_drag"));
    QVERIFY2(f.ok(), "paragraph fixture");
    showFrame();
    frame.setActiveTool(pictura::ToolId::HorizontalType);
    const int base = f.view->history_index();

    // The drag opens a session in the box, with no placement point.
    f.drag({QPointF(20, 30), QPointF(80, 70), QPointF(140, 110)});
    QVERIFY(typing(f));
    const QRectF box(20, 30, 120, 80);
    QVERIFY2(sameRect(f.canvas->typeOverlayBoxForTest().boundingRect(), box), "the dragged box");
    QVERIFY(!f.canvas->typeOverlayHasPlacementForTest());

    // The preview wraps to the box width.
    QTest::keyClicks(f.canvas, QStringLiteral("the quick brown fox"));
    const QList<QLineF> lines = f.canvas->typeOverlayBaselinesForTest();
    QVERIFY2(lines.size() > 1, "wraps to more than one line");
    for (const QLineF& line : lines) {
        QVERIFY2(line.p1().x() >= 19.0 && line.p2().x() <= 141.0, "within the box width");
    }
    QCOMPARE(f.view->history_index(), base);

    // The commit is one state adding a paragraph layer that keeps the box.
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    QVERIFY2(f.committedOnce(base, "Horizontal Type"), "one Horizontal Type state");
    const QString path = typeLayer(f);
    QVERIFY2(!path.isEmpty(), "a type layer");
    QVERIFY(pictura::type_api::isParagraph(*f.view, path));
    const pictura::TypeSetting s = pictura::type_api::layerSetting(*f.view, path);
    QVERIFY(s.boxed);
    QVERIFY2(sameRect(QRectF(s.x + s.box_left, s.y + s.box_top, s.box_right - s.box_left,
                             s.box_bottom - s.box_top),
                      box),
             "the layer's box is the dragged box");
    const QRect rect = layerRect(f, path);
    QVERIFY2(rect.left() >= 20 && rect.top() >= 30 && rect.right() <= 141 && rect.bottom() <= 111,
             "the layer lies within the box");

    // Clicking it reopens the same box.
    f.drag({QPointF(60, 50)});
    QVERIFY(typing(f));
    QVERIFY2(sameRect(f.canvas->typeOverlayBoxForTest().boundingRect(), box), "reopened box");
    QTest::keyClick(f.canvas, Qt::Key_Escape);
    QCOMPARE(f.view->history_index(), base + 1);
}

void ParagraphTypeTest::altClickAsksForTheBoxSize()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_paragraph_alt"));
    QVERIFY2(f.ok(), "paragraph fixture");
    showFrame();
    frame.setActiveTool(pictura::ToolId::HorizontalType);

    int seen = 0;
    {
        auto timer = answerNextModal(
            [](QDialog* dialog) {
                QCOMPARE(dialog->objectName(), QStringLiteral("paragraphTextSizeDialog"));
                dialog->reject();
            },
            &seen);
        f.drag({QPointF(30, 40)}, Qt::AltModifier);
    }
    QCOMPARE(seen, 1);
    QVERIFY2(!typing(f), "Cancel opens nothing");

    {
        auto timer = answerNextModal(
            [](QDialog* dialog) {
                auto* width = dialog->findChild<QDoubleSpinBox*>(QStringLiteral("paragraphTextWidth"));
                auto* height = dialog->findChild<QDoubleSpinBox*>(QStringLiteral("paragraphTextHeight"));
                QVERIFY(width && height);
                width->setValue(90.0);
                height->setValue(40.0);
                dialog->accept();
            },
            &seen);
        f.drag({QPointF(30, 40)}, Qt::AltModifier);
    }
    QCOMPARE(seen, 2);
    QVERIFY(typing(f));
    QVERIFY2(sameRect(f.canvas->typeOverlayBoxForTest().boundingRect(), QRectF(30, 40, 90, 40)),
             "a 90 x 40 box at the click");
    QTest::keyClick(f.canvas, Qt::Key_Escape);
}

void ParagraphTypeTest::handlesResizeAndOverflow()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_paragraph_handles"));
    QVERIFY2(f.ok(), "paragraph fixture");
    showFrame();
    frame.setActiveTool(pictura::ToolId::HorizontalType);
    const int base = f.view->history_index();

    // A one-line box overflows.
    f.drag({QPointF(20, 30), QPointF(60, 50), QPointF(100, 70)});
    QTest::keyClicks(f.canvas, QStringLiteral("aaaa bbbb cccc dddd eeee"));
    QVERIFY2(f.canvas->typeOverlayOverflowForTest(), "overflow");
    const int before = f.canvas->typeOverlayBaselinesForTest().size();

    // Dragging the bottom-right handle out grows the box and reflows.
    f.drag({QPointF(100, 70), QPointF(200, 150), QPointF(300, 220)});
    QVERIFY(typing(f));
    QVERIFY2(sameRect(f.canvas->typeOverlayBoxForTest().boundingRect(), QRectF(20, 30, 280, 190)),
             "the box follows the handle");
    QVERIFY2(!f.canvas->typeOverlayOverflowForTest(), "the overflow clears");
    QVERIFY2(f.canvas->typeOverlayBaselinesForTest().size() != before, "the text reflows");
    QCOMPARE(f.view->history_index(), base);
    QTest::keyClick(f.canvas, Qt::Key_Escape);
}

void ParagraphTypeTest::convertCommands()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_paragraph_convert"));
    QVERIFY2(f.ok(), "paragraph fixture");
    showFrame();
    frame.setActiveTool(pictura::ToolId::HorizontalType);
    f.drag({QPointF(20, 60)});
    QTest::keyClicks(f.canvas, QStringLiteral("Hello type"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    frame.setActiveTool(pictura::ToolId::Move);
    const QString path = typeLayer(f);
    QVERIFY2(!path.isEmpty(), "a point type layer");
    QVERIFY(!pictura::type_api::isParagraph(*f.view, path));

    const QString toParagraph = pictura::commandIdForPath(
        {QStringLiteral("Type"), QStringLiteral("Convert To Paragraph Text")});
    const QString toPoint = pictura::commandIdForPath(
        {QStringLiteral("Type"), QStringLiteral("Convert To Point Text")});
    const auto enabled = [&frame](const QString& id) {
        frame.registry()->refresh();
        QAction* action = frame.registry()->action(id);
        return action && action->isEnabled();
    };

    QVERIFY2(enabled(toParagraph) && !enabled(toPoint), "only To Paragraph for point type");
    int base = f.view->history_index();
    QVERIFY(frame.registry()->dispatch(toParagraph));
    QVERIFY2(f.committedOnce(base, "Convert To Paragraph Text"), "one state");
    QVERIFY(pictura::type_api::isParagraph(*f.view, path));
    QCOMPARE(pictura::type_api::layerText(*f.view, path), QStringLiteral("Hello type"));

    QVERIFY2(enabled(toPoint) && !enabled(toParagraph), "only To Point for paragraph type");
    base = f.view->history_index();
    QVERIFY(frame.registry()->dispatch(toPoint));
    QVERIFY2(f.committedOnce(base, "Convert To Point Text"), "one state");
    QVERIFY(!pictura::type_api::isParagraph(*f.view, path));
    QCOMPARE(pictura::type_api::layerText(*f.view, path), QStringLiteral("Hello type"));
}

void ParagraphTypeTest::typeIntoBox(const Fixture& f, const QString& text)
{
    window_->setActiveTool(pictura::ToolId::HorizontalType);
    f.drag({QPointF(20, 30), QPointF(80, 70), QPointF(140, 110)});
    QTest::keyClicks(f.canvas, text);
}

void ParagraphTypeTest::frameGestureMath()
{
    using pictura::TypeFrame;
    const auto near = [](const QPointF& a, const QPointF& b) { return QLineF(a, b).length() < 1e-6; };

    // A quarter turn about the centre (60, 45) of a 100 x 50 box at (10, 20).
    TypeFrame f;
    f.origin = QPointF(10, 20);
    f.paragraph = true;
    f.box = {0, 0, 100, 50};
    const QPointF pivot = f.pivotPoint();
    QVERIFY(near(pivot, QPointF(60, 45)));
    f.begin(TypeFrame::Gesture::Rotate, -1, pivot + QPointF(60, 0));
    f.drag(pivot + QPointF(0, 60));
    f.end();
    QVERIFY(qAbs(f.angleDegrees() - 90.0) < 1e-6);
    QVERIFY(near(f.pivotPoint(), pivot));
    QVERIFY(near(f.map(50, 25), pivot));

    // Scale from the bottom-right corner: the top-left stays, x doubles.
    TypeFrame g;
    g.pointBox = std::array<double, 4>{0, -20, 40, 5};
    g.begin(TypeFrame::Gesture::Scale, 4, g.map(40, 5));
    g.drag(QPointF(80, 10));
    QVERIFY(near(g.map(0, -20), QPointF(0, -20)));
    QVERIFY(qAbs(g.matrix[0] - 2.0) < 1e-9 && qAbs(g.matrix[3] - 1.2) < 1e-9);
    QVERIFY(qAbs(g.width() - 80.0) < 1e-9);

    // Resize the right edge: only that edge moves.
    TypeFrame h;
    h.paragraph = true;
    h.box = {0, 0, 100, 50};
    h.begin(TypeFrame::Gesture::Resize, 3, QPointF(100, 25));
    h.drag(QPointF(130, 40));
    QCOMPARE(h.box, (std::array<double, 4>{0, 0, 130, 50}));
    QVERIFY(near(h.origin, QPointF()));

    // The reference point drags on its own.
    h.end();
    h.begin(TypeFrame::Gesture::Pivot, -1, h.pivotPoint());
    h.drag(h.pivotPoint() + QPointF(10, 5));
    QVERIFY(near(h.pivotPoint(), QPointF(75, 30)));
}

void ParagraphTypeTest::boxRotatesWithReadout()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_paragraph_rotate"));
    QVERIFY2(f.ok(), "paragraph fixture");
    showFrame();
    typeIntoBox(f, QStringLiteral("turn me"));
    const int base = f.view->history_index();

    // Just outside the top-right corner (140, 30) is the rotate band.
    const double zoom = f.canvas->zoom();
    const QPointF band = QPointF(140, 30) + QPointF(12, -12) / zoom;
    QCOMPARE(f.tools->activeCursorKindForTest(band, 0), 3);
    const QPointF pivot = f.canvas->typeOverlayPivotForTest();
    QVERIFY(QLineF(pivot, QPointF(80, 70)).length() < 1e-6);

    // Drag a quarter turn about the reference point.
    const QPointF start = band - pivot;
    const QPointF turned(-start.y(), start.x());
    f.canvas->mousePressed(band, Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(pivot + turned);
    QVERIFY2(f.canvas->dragSizeHintForTest().startsWith(QStringLiteral("∠: 90.0")),
             qPrintable(f.canvas->dragSizeHintForTest()));
    const QPointF corner = f.canvas->typeOverlayBoxForTest().at(1);
    QVERIFY2(QLineF(corner, QPointF(140, 30)).length() > 20.0, "the box turned");
    f.canvas->mouseReleased(pivot + turned);
    QVERIFY(!f.canvas->hasDragSizeHintForTest());
    QCOMPARE(f.view->history_index(), base);

    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    QVERIFY2(f.committedOnce(base, "Horizontal Type"), "one Horizontal Type state");
    const pictura::TypeSetting s = pictura::type_api::layerSetting(*f.view, typeLayer(f));
    QVERIFY2(qAbs(s.xx) < 1e-3 && qAbs(s.xy - 1.0) < 1e-3, "a quarter turn");
}

void ParagraphTypeTest::resizeShowsSizeReadout()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_paragraph_readout"));
    QVERIFY2(f.ok(), "paragraph fixture");
    showFrame();
    typeIntoBox(f, QStringLiteral("size me"));
    QCOMPARE(f.tools->activeCursorKindForTest(QPointF(140, 110), 0), 2);
    f.canvas->mousePressed(QPointF(140, 110), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(QPointF(170, 130));
    const QString hint = f.canvas->dragSizeHintForTest();
    QVERIFY2(hint.startsWith(QStringLiteral("W: ")) && hint.contains(QStringLiteral("\nH: ")),
             qPrintable(hint));
    f.canvas->mouseReleased(QPointF(170, 130));
    QVERIFY(sameRect(f.canvas->typeOverlayBoxForTest().boundingRect(), QRectF(20, 30, 150, 100)));
    QTest::keyClick(f.canvas, Qt::Key_Escape);
}

void ParagraphTypeTest::pivotDrags()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_paragraph_pivot"));
    QVERIFY2(f.ok(), "paragraph fixture");
    showFrame();
    typeIntoBox(f, QStringLiteral("pivot"));
    const QPointF pivot = f.canvas->typeOverlayPivotForTest();
    QCOMPARE(f.tools->activeCursorKindForTest(pivot, 0), 4);
    f.canvas->mousePressed(pivot, Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(pivot + QPointF(10, 6));
    QVERIFY(f.canvas->dragSizeHintForTest().startsWith(QStringLiteral("X: ")));
    f.canvas->mouseReleased(pivot + QPointF(10, 6));
    QVERIFY(QLineF(f.canvas->typeOverlayPivotForTest(), pivot + QPointF(10, 6)).length() < 1e-6);
    QTest::keyClick(f.canvas, Qt::Key_Escape);
}

void ParagraphTypeTest::ctrlScalesPointType()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_point_scale"));
    QVERIFY2(f.ok(), "paragraph fixture");
    showFrame();
    frame.setActiveTool(pictura::ToolId::HorizontalType);
    f.drag({QPointF(20, 60)});
    QTest::keyClicks(f.canvas, QStringLiteral("Scale"));
    QVERIFY2(f.canvas->typeOverlayBoxForTest().isEmpty(), "no box without Ctrl");

    emit f.canvas->modifierKeyChanged(int(Qt::ControlModifier));
    const QPolygonF box = f.canvas->typeOverlayBoxForTest();
    QVERIFY2(box.size() == 4, "Ctrl shows the box");
    const QPointF corner = box.at(2);
    f.canvas->mousePressed(corner, Qt::LeftButton, int(Qt::ControlModifier));
    f.canvas->mouseMoved(corner + QPointF(40, 20));
    f.canvas->mouseReleased(corner + QPointF(40, 20));
    emit f.canvas->modifierKeyChanged(int(Qt::NoModifier));
    QVERIFY2(f.canvas->typeOverlayBoxForTest().isEmpty(), "the box goes with Ctrl");

    const int base = f.view->history_index();
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    QVERIFY2(f.committedOnce(base, "Horizontal Type"), "one Horizontal Type state");
    const pictura::TypeSetting s = pictura::type_api::layerSetting(*f.view, typeLayer(f));
    QVERIFY2(s.xx > 1.2 && s.yy > 1.2, "scaled up");
}

QTEST_MAIN(ParagraphTypeTest)
#include "tst_paragraph_type.moc"
