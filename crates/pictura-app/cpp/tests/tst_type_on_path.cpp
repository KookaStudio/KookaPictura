// Type on a path (#683): a Type tool click on a Work Path flows text along it
// by arc length, a click inside a closed path makes area type, overflow marks
// the path end, and the committed layer reopens and masks on the same path.

#include <QtTest/QtTest>

#include "api/type_api.h"
#include "selftest_paint_fixture.h"

#include "pictura_app/src/cxxqt_object/paths.cxxqt.h"

#include "qt_test_support.h"

namespace {

using paint_fixture::Fixture;

QImage seed()
{
    QImage image(320, 240, QImage::Format_RGB32);
    image.fill(Qt::white);
    return image;
}

QString typeLayer(const Fixture& f)
{
    for (int i = 0; i < f.view->layer_count(); ++i) {
        if (pictura::type_api::readLayer(*f.view, QString::number(i)).isType) {
            return QString::number(i);
        }
    }
    return QString();
}

void click(const Fixture& f, const QPointF& p)
{
    f.drag({p});
}

void drawOpenPath(const Fixture& f)
{
    click(f, QPointF(20, 100));
    click(f, QPointF(300, 100));
}

void drawClosedSquare(const Fixture& f)
{
    for (const QPointF& p : {QPointF(20, 20), QPointF(140, 20), QPointF(140, 140),
                             QPointF(20, 140), QPointF(20, 20)}) {
        click(f, p);
    }
}

} // namespace

class TypeOnPathTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void clickOnOpenPathStartsOnPath();
    void overflowPlusAtPathEnd();
    void clickInsideClosedPathMakesAreaType();
    void commitAndReopenOnPath();
    void typeMaskFollowsPath();
    void saveAndReopenKeepsPath();
    void moveAndTransformOnPath();

private:
    void showFrame();

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void TypeOnPathTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void TypeOnPathTest::showFrame()
{
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    window_->activateWindow();
    QCoreApplication::processEvents();
}

void TypeOnPathTest::clickOnOpenPathStartsOnPath()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_path_click"));
    QVERIFY2(f.ok(), "path fixture");
    showFrame();
    frame.setActiveTool(pictura::ToolId::Pen);
    drawOpenPath(f);
    const int base = f.view->history_index();

    frame.setActiveTool(pictura::ToolId::HorizontalType);
    QVERIFY2(f.canvas->pathOverlayAnchorCountForTest() >= 2,
             "the path stays visible with the Type tool");
    QCOMPARE(f.tools->activeCursorKindForTest(QPointF(160, 100), 0), 5);
    click(f, QPointF(160, 100));
    QVERIFY2(f.tools->textSession() && f.tools->textSession()->textActive(), "path session");
    QCOMPARE(f.canvas->typeOverlayPathKindForTest(), 1);
    QVERIFY2(!f.canvas->typeOverlayBaselinesForTest().isEmpty(), "the path is the guide");
    QVERIFY(!f.canvas->typeOverlayHasPlacementForTest());

    QTest::keyClicks(f.canvas, QStringLiteral("abc"));
    QVERIFY(f.canvas->typeOverlayHasTextForTest());
    QCOMPARE(f.view->history_index(), base);
    QTest::keyClick(f.canvas, Qt::Key_Escape);
    QCOMPARE(f.view->history_index(), base);
}

void TypeOnPathTest::overflowPlusAtPathEnd()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_path_overflow"));
    QVERIFY2(f.ok(), "path fixture");
    showFrame();
    frame.setActiveTool(pictura::ToolId::Pen);
    click(f, QPointF(20, 100));
    click(f, QPointF(80, 100));
    const int base = f.view->history_index();

    frame.setActiveTool(pictura::ToolId::HorizontalType);
    click(f, QPointF(30, 100));
    QVERIFY2(f.tools->textSession() && f.tools->textSession()->textActive(), "path session");
    QTest::keyClicks(f.canvas, QStringLiteral("WWWWWWWWWW"));
    QVERIFY2(f.canvas->typeOverlayOverflowForTest(), "overflow");
    QVERIFY2(f.canvas->typeOverlayHasOverflowPointForTest(), "a plus at the path end");
    QVERIFY2(f.canvas->typeOverlayOverflowPointForTest().x() >= 78.0
                 && f.canvas->typeOverlayOverflowPointForTest().x() <= 82.0,
             "the marker is at the last anchor");
    QCOMPARE(f.view->history_index(), base);
    QTest::keyClick(f.canvas, Qt::Key_Escape);
}

void TypeOnPathTest::clickInsideClosedPathMakesAreaType()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_area"));
    QVERIFY2(f.ok(), "path fixture");
    showFrame();
    frame.setActiveTool(pictura::ToolId::Pen);
    drawClosedSquare(f);
    const int base = f.view->history_index();

    frame.setActiveTool(pictura::ToolId::HorizontalType);
    click(f, QPointF(80, 80));
    QVERIFY2(f.tools->textSession() && f.tools->textSession()->textActive(), "area session");
    QCOMPARE(f.canvas->typeOverlayPathKindForTest(), 2);
    QVERIFY2(!f.canvas->typeOverlayBoxForTest().isEmpty(), "the closed path outline");

    QTest::keyClicks(f.canvas, QStringLiteral("area type text that wraps"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    QVERIFY2(f.committedOnce(base, "Horizontal Type"), "one Horizontal Type state");
    const QString path = typeLayer(f);
    QVERIFY2(!path.isEmpty(), "a type layer");
    const pictura::TypeSetting s = pictura::type_api::readLayer(*f.view, path).spec.setting;
    QCOMPARE(s.path_kind, 2);
    QVERIFY2(s.boxed, "area type wraps in the path box");
    QVERIFY2(!s.path_points.isEmpty(), "the path is kept");
}

void TypeOnPathTest::commitAndReopenOnPath()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_path_reopen"));
    QVERIFY2(f.ok(), "path fixture");
    showFrame();
    frame.setActiveTool(pictura::ToolId::Pen);
    drawOpenPath(f);
    const int base = f.view->history_index();

    frame.setActiveTool(pictura::ToolId::HorizontalType);
    click(f, QPointF(40, 100));
    QTest::keyClicks(f.canvas, QStringLiteral("Hello"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    QVERIFY2(f.committedOnce(base, "Horizontal Type"), "one commit state");
    const QString path = typeLayer(f);
    QVERIFY2(!path.isEmpty(), "a type layer");
    const pictura::TypeSetting s = pictura::type_api::readLayer(*f.view, path).spec.setting;
    QCOMPARE(s.path_kind, 1);
    QVERIFY2(!s.path_points.isEmpty(), "the path is kept");
    QVERIFY(!pictura::type_api::isParagraph(*f.view, path));

    // Clicking the text reopens the same path and edits in place.
    click(f, QPointF(45, 100));
    QVERIFY2(f.tools->textSession() && f.tools->textSession()->textActive(), "reopened");
    QCOMPARE(f.canvas->typeOverlayPathKindForTest(), 1);
    QTest::keyClick(f.canvas, Qt::Key_End, Qt::ControlModifier);
    QTest::keyClicks(f.canvas, QStringLiteral("!"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    QVERIFY2(f.committedOnce(base + 1, "Edit Type Layer"), "one edit state");
    QCOMPARE(f.view->layer_count(), 2);
    QVERIFY2(pictura::type_api::readLayer(*f.view, path).spec.text.contains(QStringLiteral("Hello!")),
             "the text changed in place");
}

void TypeOnPathTest::typeMaskFollowsPath()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_path_mask"));
    QVERIFY2(f.ok(), "path fixture");
    showFrame();
    frame.setActiveTool(pictura::ToolId::Pen);
    drawOpenPath(f);
    const int base = f.view->history_index();

    frame.setActiveTool(pictura::ToolId::HorizontalTypeMask);
    click(f, QPointF(40, 100));
    QVERIFY2(f.tools->textSession() && f.tools->textSession()->textActive(), "mask session");
    QCOMPARE(f.canvas->typeOverlayPathKindForTest(), 1);
    QTest::keyClicks(f.canvas, QStringLiteral("MASK"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    QVERIFY2(f.committedOnce(base, "Horizontal Type Mask"), "one mask state");
    QVERIFY(f.view->has_selection());
    QVERIFY2(f.view->selection_count() > 30, "the path glyphs are selected");
    QCOMPARE(f.view->selection_coverage(5, 5), 0);
}

void TypeOnPathTest::saveAndReopenKeepsPath()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_path_psd"));
    QVERIFY2(f.ok(), "path fixture");
    showFrame();
    frame.setActiveTool(pictura::ToolId::Pen);
    drawOpenPath(f);
    const int base = f.view->history_index();

    frame.setActiveTool(pictura::ToolId::HorizontalType);
    click(f, QPointF(40, 100));
    QTest::keyClicks(f.canvas, QStringLiteral("Saved"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    QVERIFY2(f.committedOnce(base, "Horizontal Type"), "one commit state");

    const QString path = QDir::temp().filePath(QStringLiteral("pictura_type_path_psd.psd"));
    QFile::remove(path);
    QVERIFY2(frame.saveActiveAs(path), "saves the PSD");
    QVERIFY2(frame.openDocumentAtPath(path, false), "reopens the PSD");
    pictura::PictureView* reopened = frame.activeView();
    QVERIFY(reopened != nullptr);
    QString reopenedPath;
    for (int i = 0; i < reopened->layer_count(); ++i) {
        if (pictura::type_api::readLayer(*reopened, QString::number(i)).isType) {
            reopenedPath = QString::number(i);
            break;
        }
    }
    QVERIFY2(!reopenedPath.isEmpty(), "a type layer reopens");
    const pictura::TypeSetting s = pictura::type_api::readLayer(*reopened, reopenedPath).spec.setting;
    QCOMPARE(s.path_kind, 1);
    QVERIFY2(!s.path_points.isEmpty(), "the text path survives the save");
    frame.closeDocument(frame.activeDocumentIndex(), false);
    QFile::remove(path);
}

void TypeOnPathTest::moveAndTransformOnPath()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_path_move"));
    QVERIFY2(f.ok(), "path fixture");
    showFrame();
    frame.setActiveTool(pictura::ToolId::Pen);
    drawOpenPath(f);
    const int base = f.view->history_index();

    frame.setActiveTool(pictura::ToolId::HorizontalType);
    click(f, QPointF(40, 100));
    QTest::keyClicks(f.canvas, QStringLiteral("Hello"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    QVERIFY2(f.committedOnce(base, "Horizontal Type"), "one commit state");
    const QString path = typeLayer(f);
    QVERIFY2(!path.isEmpty(), "a type layer");
    const pictura::TypeSetting before = pictura::type_api::readLayer(*f.view, path).spec.setting;
    const double sizeBefore = pictura::type_api::readLayer(*f.view, path).spec.character.size;

    // Move the text object off its controls box; the stored path follows.
    frame.setActiveTool(pictura::ToolId::Move);
    f.canvas->mousePressed(QPointF(200, 180), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(QPointF(210, 190));
    f.canvas->mouseReleased(QPointF(210, 190));
    const pictura::TypeSetting moved = pictura::type_api::readLayer(*f.view, path).spec.setting;
    QCOMPARE(moved.path_kind, 1);
    QVERIFY2(moved.path_points != before.path_points, "the path moves with the layer");

    // Free Transform scales the path and the type size, keeping it type.
    QVERIFY(f.tools->beginFreeTransform(path));
    QVERIFY(f.tools->setTransformWidth(200.0, true));
    QVERIFY(f.tools->setTransformHeight(200.0, true));
    f.tools->commitFreeTransform();
    const pictura::TypeSetting after = pictura::type_api::readLayer(*f.view, path).spec.setting;
    QCOMPARE(after.path_kind, 1);
    QVERIFY2(!after.path_points.isEmpty(), "the transformed path stays type");
    const double sizeAfter = pictura::type_api::readLayer(*f.view, path).spec.character.size;
    QVERIFY2(sizeAfter > sizeBefore, "the type size scales with the path");

    // The moved and scaled path survives a save and reopen.
    const QString file = QDir::temp().filePath(QStringLiteral("pictura_type_path_moved.psd"));
    QFile::remove(file);
    QVERIFY2(frame.saveActiveAs(file), "saves the transformed path");
    QVERIFY2(frame.openDocumentAtPath(file, false), "reopens the transformed path");
    pictura::PictureView* reopened = frame.activeView();
    QVERIFY(reopened != nullptr);
    QString reopenedPath;
    for (int i = 0; i < reopened->layer_count(); ++i) {
        if (pictura::type_api::readLayer(*reopened, QString::number(i)).isType) {
            reopenedPath = QString::number(i);
            break;
        }
    }
    QVERIFY2(!reopenedPath.isEmpty(), "the transformed type layer reopens");
    const pictura::TypeSetting restored =
        pictura::type_api::readLayer(*reopened, reopenedPath).spec.setting;
    QCOMPARE(restored.path_kind, 1);
    QCOMPARE(restored.path_points, after.path_points);
    QCOMPARE(pictura::type_api::readLayer(*reopened, reopenedPath).spec.character.size, sizeAfter);
    frame.closeDocument(frame.activeDocumentIndex(), false);
    QFile::remove(file);
}

QTEST_MAIN(TypeOnPathTest)
#include "tst_type_on_path.moc"
