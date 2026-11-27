#include <QtTest/QtTest>
#include <QtCore/QLineF>
#include <QtGui/QAction>
#include <QtGui/QTransform>

#include "commands.h"
#include "api/transform_api.h"
#include "panels/paths_panel.h"
#include "selftest_paint_fixture.h"
#include "api/layers_bridge.h"
#include "api/transform_bridge.h"
#include "qt_test_support.h"

#include <cmath>

namespace {
using paint_fixture::Fixture;

QImage seed()
{
    QImage image(100, 100, QImage::Format_RGB32);
    image.fill(Qt::white);
    return image;
}

void click(const Fixture& fixture, QPointF point, Qt::KeyboardModifiers mods = Qt::NoModifier)
{
    fixture.drag({point}, mods);
}

void drawPaths(Fixture& fixture)
{
    auto* pen = fixture.tools->optionsFor<pictura::PenGroup>(pictura::ToolId::Pen);
    pictura::PenOptions options = pen->get();
    options.mode = 1;
    pen->set(options);
    fixture.frame.setActiveTool(pictura::ToolId::Pen);
    click(fixture, {10, 10});
    click(fixture, {40, 10});
    fixture.drag({QPointF(40, 40), QPointF(45, 40), QPointF(50, 40)});
    click(fixture, {10, 40});
    click(fixture, {10, 10});
    click(fixture, {70, 70});
    click(fixture, {90, 70});
    fixture.frame.setActiveTool(pictura::ToolId::PathSelection);
    click(fixture, {25, 25});
}

QPointF anchor(const Fixture& fixture, int component, int point)
{
    const auto p = pictura::path_point(*fixture.view, component, point);
    return p.size() == 9 ? QPointF(p[0], p[1]) : QPointF(-1, -1);
}

bool dispatch(Fixture& fixture, const char* id)
{
    fixture.frame.registry()->refresh();
    return fixture.frame.registry()->dispatch(QString::fromLatin1(id));
}
} // namespace

class PathComponentTransformTest : public QObject {
    Q_OBJECT
private slots:
    void initTestCase();
    void affineControls();
    void fractionalBounds();
    void multiComponentControls();
    void projectiveWarp();
    void cancelIdentityInvalid();
    void availability();
    void repeatComponents();
    void repeatPoints();
    void repeatAlignedPointsAndLine();
    void repeatAvailability();
    void targetChangeCancels();
    void repeatDocuments();
    void targetsAndMetadata();
private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void PathComponentTransformTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
}

void PathComponentTransformTest::affineControls()
{
    Fixture f(*window_, seed(), QStringLiteral("path_transform_affine"));
    QVERIFY(f.ok());
    drawPaths(f);
    const QPointF original = anchor(f, 0, 0);
    const QPointF untouched = anchor(f, 1, 0);
    const QRgb untouchedPixel = f.view->sample_argb(5, 5);
    const int before = f.view->history_index();
    QVERIFY(dispatch(f, "edit.transform.path.scale"));
    QVERIFY(f.view->transform_session_active());
    QVERIFY(f.tools->setTransformWidth(200.0, false));
    f.tools->commitFreeTransform();
    QVERIFY(anchor(f, 0, 0) != original);
    QCOMPARE(anchor(f, 1, 0), untouched);
    QCOMPARE(f.view->sample_argb(5, 5), untouchedPixel);
    QCOMPARE(f.view->history_index(), before + 1);
    const QPointF scaled = anchor(f, 0, 0);
    QVERIFY(f.view->undo());
    QCOMPARE(anchor(f, 0, 0), original);
    QVERIFY(f.view->redo());
    QCOMPARE(anchor(f, 0, 0), scaled);
    click(f, {25, 25});
    QVERIFY(dispatch(f, "edit.transform.path.rotate"));
    QVERIFY(pictura::transform_set_pivot(*f.view, 10.0, 10.0));
    QVERIFY(f.tools->setTransformAngle(30.0));
    f.tools->commitFreeTransform();
    QVERIFY(anchor(f, 0, 0) != scaled);
    QCOMPARE(anchor(f, 1, 0), untouched);
    QVERIFY(f.view->undo());
    QCOMPARE(anchor(f, 0, 0), scaled);
    click(f, {25, 25});
    QVERIFY(dispatch(f, "edit.transform.path.skew"));
    QVERIFY(f.tools->setTransformSkew(15.0, 0.0));
    f.tools->commitFreeTransform();
    QVERIFY(anchor(f, 0, 0) != scaled);
    QCOMPARE(anchor(f, 1, 0), untouched);
    QVERIFY(f.view->undo());
    QCOMPARE(anchor(f, 0, 0), scaled);
}

void PathComponentTransformTest::multiComponentControls()
{
    Fixture f(*window_, seed(), QStringLiteral("path_transform_components"));
    QVERIFY(f.ok());
    drawPaths(f);
    click(f, {80, 70}, Qt::ShiftModifier);
    const QPointF first = anchor(f, 0, 1);
    const QPointF second = anchor(f, 1, 0);
    const int before = f.view->history_index();
    QVERIFY(dispatch(f, "edit.transform.path.scale"));
    const QPolygonF quad = pictura::transform_api::quad(*f.view).corners;
    QCOMPARE(quad.size(), 4);
    QCOMPARE(f.view->transform_press(quad[1].x(), quad[1].y(), 1.0, false, false), 1);
    QVERIFY(f.view->transform_move(quad[1].x() + 20.0, quad[1].y(), 1.0, false, false));
    f.view->transform_release();
    f.tools->commitFreeTransform();
    QVERIFY(anchor(f, 0, 1) != first);
    QVERIFY(anchor(f, 1, 0) != second);
    QCOMPARE(f.view->history_index(), before + 1);
    QVERIFY(f.view->undo());
    QCOMPARE(anchor(f, 0, 1), first);
    QCOMPARE(anchor(f, 1, 0), second);
}

void PathComponentTransformTest::fractionalBounds()
{
    Fixture f(*window_, seed(), QStringLiteral("path_transform_fractional"));
    QVERIFY(f.ok());
    auto* pen = f.tools->optionsFor<pictura::PenGroup>(pictura::ToolId::Pen);
    pictura::PenOptions options = pen->get();
    options.mode = 1;
    pen->set(options);
    f.frame.setActiveTool(pictura::ToolId::Pen);
    click(f, {10.25, 10.75});
    click(f, {30.75, 10.75});
    click(f, {30.75, 20.25});
    click(f, {10.25, 20.25});
    click(f, {10.25, 10.75});
    f.frame.setActiveTool(pictura::ToolId::PathSelection);
    click(f, {20.0, 15.0});
    QVERIFY(dispatch(f, "edit.transform.path.rotate"));
    QVERIFY(std::abs(pictura::transform_pivot_x(*f.view) - 20.5) < 1e-6);
    QVERIFY(std::abs(pictura::transform_pivot_y(*f.view) - 15.5) < 1e-6);
    QVERIFY(f.tools->setTransformAngle(90.0));
    f.tools->commitFreeTransform();
    const QPointF rotated = anchor(f, 0, 0);
    QVERIFY(std::abs(rotated.x() - 25.25) < 1e-5);
    QVERIFY(std::abs(rotated.y() - 5.25) < 1e-5);
    QVERIFY(dispatch(f, "edit.transform.path.again"));
    const QPointF twice = anchor(f, 0, 0);
    QVERIFY(std::abs(twice.x() - 30.75) < 1e-5);
    QVERIFY(std::abs(twice.y() - 20.25) < 1e-5);
}

void PathComponentTransformTest::projectiveWarp()
{
    Fixture f(*window_, seed(), QStringLiteral("path_transform_modes"));
    QVERIFY(f.ok());
    drawPaths(f);
    for (const char* id : {"edit.transform.path.distort", "edit.transform.path.perspective",
                           "edit.transform.path.warp"}) {
        const QPointF original = anchor(f, 0, 0);
        const QPointF untouched = anchor(f, 1, 0);
        const auto fields = pictura::path_point(*f.view, 0, 2);
        QVERIFY(fields.size() == 9 && fields[5] > 0.5);
        const QPointF handleBefore(fields[6], fields[7]);
        const auto bounds = pictura::path_subpath_bounds(*f.view, 0);
        QVERIFY(bounds.size() == 4);
        const int before = f.view->history_index();
        QVERIFY2(dispatch(f, id), id);
        QVERIFY(f.view->transform_session_active());
        if (QString::fromLatin1(id).endsWith(QLatin1String("warp"))) {
            QVERIFY(f.tools->setTransformWarpStyle(QStringLiteral("arc")));
            QVERIFY(f.tools->setTransformWarpValues(35.0, 0.0, 0.0));
        } else {
            QVERIFY(f.view->transform_press(10.0, 10.0, 1.0, false, false) >= 0);
            QVERIFY(f.view->transform_move(5.0, 5.0, 1.0, false, false));
        }
        QPointF expectedHandle;
        const bool projective = !QString::fromLatin1(id).endsWith(QLatin1String("warp"));
        if (projective) {
            QPolygonF source;
            source << QPointF(bounds[0], bounds[1]) << QPointF(bounds[2], bounds[1])
                   << QPointF(bounds[2], bounds[3]) << QPointF(bounds[0], bounds[3]);
            QTransform oracle;
            QVERIFY(QTransform::quadToQuad(source, pictura::transform_api::quad(*f.view).corners, oracle));
            expectedHandle = oracle.map(handleBefore);
        }
        f.tools->commitFreeTransform();
        QVERIFY(anchor(f, 0, 0) != original);
        if (projective) {
            const auto result = pictura::path_point(*f.view, 0, 2);
            QVERIFY(result.size() == 9 && result[5] > 0.5);
            QVERIFY(std::abs(result[6] - expectedHandle.x()) < 1e-4);
            QVERIFY(std::abs(result[7] - expectedHandle.y()) < 1e-4);
        }
        QCOMPARE(anchor(f, 1, 0), untouched);
        QCOMPARE(f.view->history_index(), before + 1);
        QVERIFY(f.view->undo());
        QCOMPARE(anchor(f, 0, 0), original);
        click(f, {25, 25});
    }
}

void PathComponentTransformTest::cancelIdentityInvalid()
{
    Fixture f(*window_, seed(), QStringLiteral("path_transform_cancel"));
    QVERIFY(f.ok());
    drawPaths(f);
    const QPointF original = anchor(f, 0, 0);
    const int before = f.view->history_index();
    QVERIFY(dispatch(f, "edit.transform.path.scale"));
    QVERIFY(f.tools->setTransformWidth(200.0, false));
    f.tools->cancelFreeTransform();
    QCOMPARE(anchor(f, 0, 0), original);
    QCOMPARE(f.view->history_index(), before);
    QVERIFY(dispatch(f, "edit.transform.path.rotate"));
    f.tools->commitFreeTransform();
    QCOMPARE(anchor(f, 0, 0), original);
    QCOMPARE(f.view->history_index(), before);
    QVERIFY(dispatch(f, "edit.transform.path.scale"));
    QVERIFY(!f.tools->setTransformWidth(0.0, false));
    f.tools->cancelFreeTransform();
    QCOMPARE(anchor(f, 0, 0), original);
    QCOMPARE(f.view->history_index(), before);
}

void PathComponentTransformTest::availability()
{
    Fixture f(*window_, seed(), QStringLiteral("path_transform_availability"));
    QVERIFY(f.ok());
    drawPaths(f);
    f.frame.registry()->refresh();
    QAction* scale = f.frame.registry()->action(QStringLiteral("edit.transform.path.scale"));
    QVERIFY(scale);
    QVERIFY(scale->isEnabled());
    QVERIFY(dispatch(f, "edit.transform.path.scale"));
    f.frame.registry()->refresh();
    QVERIFY(!scale->isEnabled());
    f.tools->cancelFreeTransform();
    click(f, {80, 70});
    f.frame.registry()->refresh();
    QVERIFY(!scale->isEnabled());
    click(f, {95, 5});
    f.frame.registry()->refresh();
    QVERIFY(!scale->isEnabled());

    pictura::ShapeSpec spec{};
    spec.kind = 0;
    spec.boxed = true;
    spec.x0 = 10; spec.y0 = 10; spec.x1 = 40; spec.y1 = 40;
    const QString shape = pictura::shape_add_layer(*f.view, spec, 0xffff0000u);
    QVERIFY(!shape.isEmpty());
    f.frame.selectLayerPath(shape);
    click(f, {25, 25});
    QVERIFY(f.view->set_layer_lock(shape.toInt(), QStringLiteral("position"), true));
    f.frame.registry()->refresh();
    QVERIFY(!scale->isEnabled());
}

void PathComponentTransformTest::repeatComponents()
{
    Fixture f(*window_, seed(), QStringLiteral("path_repeat_components"));
    QVERIFY(f.ok());
    drawPaths(f);
    click(f, {80, 70}, Qt::ShiftModifier);
    const QPointF original = anchor(f, 0, 1);
    const QPointF originalSecond = anchor(f, 1, 0);
    QVERIFY(dispatch(f, "edit.transform.path.rotate"));
    QVERIFY(pictura::transform_set_pivot(*f.view, 10.0, 10.0));
    QVERIFY(f.tools->setTransformAngle(90.0));
    f.tools->commitFreeTransform();
    const QPointF once = anchor(f, 0, 1);
    const QPointF onceSecond = anchor(f, 1, 0);
    QVERIFY(once != original);
    QVERIFY(onceSecond != originalSecond);
    QVERIFY(dispatch(f, "edit.transform.path.again"));
    const QPointF twice = anchor(f, 0, 1);
    const QPointF twiceSecond = anchor(f, 1, 0);
    QVERIFY(std::abs(twice.x() + 80.0) < 1e-4);
    QVERIFY(std::abs(twice.y() - 90.0) < 1e-4);
    QVERIFY(std::abs(twiceSecond.x() + 110.0) < 1e-4);
    QVERIFY(std::abs(twiceSecond.y() - 30.0) < 1e-4);
    QVERIFY(f.view->undo());
    QCOMPARE(anchor(f, 0, 1), once);
    QCOMPARE(anchor(f, 1, 0), onceSecond);
    QVERIFY(f.view->redo());
    QCOMPARE(anchor(f, 0, 1), twice);
    QCOMPARE(anchor(f, 1, 0), twiceSecond);
}

void PathComponentTransformTest::repeatPoints()
{
    Fixture f(*window_, seed(), QStringLiteral("path_repeat_points"));
    QVERIFY(f.ok());
    drawPaths(f);
    QVERIFY(dispatch(f, "edit.transform.path.rotate"));
    QVERIFY(f.tools->setTransformAngle(90.0));
    f.tools->commitFreeTransform();
    f.frame.setActiveTool(pictura::ToolId::DirectSelection);
    const QPointF selected0 = anchor(f, 0, 2);
    const auto beforeFields = pictura::path_point(*f.view, 0, 2);
    QVERIFY(beforeFields.size() == 9 && beforeFields[5] > 0.5);
    const QPointF handleBefore(beforeFields[6], beforeFields[7]);
    const QPointF selected1 = anchor(f, 1, 0);
    const QPointF untouched = anchor(f, 1, 1);
    click(f, selected0);
    click(f, selected1, Qt::ShiftModifier);
    QVERIFY(dispatch(f, "edit.transform.points.again"));
    QVERIFY(anchor(f, 0, 2) != selected0);
    QVERIFY(anchor(f, 1, 0) != selected1);
    const auto afterFields = pictura::path_point(*f.view, 0, 2);
    QVERIFY(afterFields.size() == 9 && afterFields[5] > 0.5);
    const QPointF beforeVector = handleBefore - selected0;
    const QPointF afterVector = QPointF(afterFields[6], afterFields[7]) - anchor(f, 0, 2);
    QVERIFY(std::abs(afterVector.x() + beforeVector.y()) < 1e-5);
    QVERIFY(std::abs(afterVector.y() - beforeVector.x()) < 1e-5);
    QCOMPARE(anchor(f, 1, 1), untouched);
    QVERIFY(f.view->undo());
    QCOMPARE(anchor(f, 0, 2), selected0);
    QCOMPARE(anchor(f, 1, 0), selected1);
    QVERIFY(f.view->redo());
    QCOMPARE(anchor(f, 1, 1), untouched);
}

void PathComponentTransformTest::repeatAlignedPointsAndLine()
{
    Fixture f(*window_, seed(), QStringLiteral("path_repeat_aligned"));
    QVERIFY(f.ok());
    drawPaths(f);
    QVERIFY(dispatch(f, "edit.transform.path.rotate"));
    QVERIFY(f.tools->setTransformAngle(90.0));
    f.tools->commitFreeTransform();
    f.frame.setActiveTool(pictura::ToolId::DirectSelection);
    click(f, {70, 70});
    click(f, {90, 70}, Qt::ShiftModifier);
    QVERIFY(dispatch(f, "edit.transform.points.again"));
    QVERIFY(QLineF(anchor(f, 1, 0), QPointF(80, 60)).length() < 1e-5);
    QVERIFY(QLineF(anchor(f, 1, 1), QPointF(80, 80)).length() < 1e-5);
    QVERIFY(f.view->undo());
    f.frame.setActiveTool(pictura::ToolId::PathSelection);
    click(f, {80, 70});
    QVERIFY(dispatch(f, "edit.transform.path.again"));
    QVERIFY(QLineF(anchor(f, 1, 0), QPointF(80, 60)).length() < 1e-5);
    QVERIFY(QLineF(anchor(f, 1, 1), QPointF(80, 80)).length() < 1e-5);
    f.frame.setActiveTool(pictura::ToolId::DirectSelection);
    click(f, {80, 60});
    click(f, {80, 80}, Qt::ShiftModifier);
    QVERIFY(dispatch(f, "edit.transform.points.again"));
    QVERIFY(QLineF(anchor(f, 1, 0), QPointF(90, 70)).length() < 1e-5);
    QVERIFY(QLineF(anchor(f, 1, 1), QPointF(70, 70)).length() < 1e-5);
}

void PathComponentTransformTest::repeatAvailability()
{
    Fixture f(*window_, seed(), QStringLiteral("path_repeat_enable"));
    QVERIFY(f.ok());
    drawPaths(f);
    f.frame.registry()->refresh();
    QAction* again = f.frame.registry()->action(QStringLiteral("edit.transform.path.again"));
    QVERIFY(again);
    QVERIFY(!again->isEnabled());
    QVERIFY(dispatch(f, "edit.transform.path.rotate"));
    QVERIFY(f.tools->setTransformAngle(90.0));
    f.tools->commitFreeTransform();
    f.frame.registry()->refresh();
    QVERIFY(again->isEnabled());
    QVERIFY(dispatch(f, "edit.transform.path.scale"));
    f.frame.registry()->refresh();
    QVERIFY(!again->isEnabled());
    f.tools->cancelFreeTransform();
    f.frame.registry()->refresh();
    QVERIFY(again->isEnabled());
    QVERIFY(dispatch(f, "edit.transform.path.scale"));
    f.tools->commitFreeTransform();
    f.frame.registry()->refresh();
    QVERIFY(again->isEnabled());
    QVERIFY(dispatch(f, "edit.transform.path.warp"));
    QVERIFY(f.tools->setTransformWarpStyle(QStringLiteral("arc")));
    QVERIFY(f.tools->setTransformWarpValues(35.0, 0.0, 0.0));
    f.tools->commitFreeTransform();
    f.frame.registry()->refresh();
    QVERIFY(again->isEnabled());
    f.frame.setActiveTool(pictura::ToolId::DirectSelection);
    click(f, anchor(f, 0, 0));
    f.frame.registry()->refresh();
    QAction* pointsAgain = f.frame.registry()->action(QStringLiteral("edit.transform.points.again"));
    QVERIFY(pointsAgain);
    QVERIFY(!pointsAgain->isEnabled());
    click(f, anchor(f, 0, 2), Qt::ShiftModifier);
    f.frame.registry()->refresh();
    QVERIFY(pointsAgain->isEnabled());
}

void PathComponentTransformTest::targetChangeCancels()
{
    Fixture f(*window_, seed(), QStringLiteral("path_transform_target_change"));
    QVERIFY(f.ok());
    drawPaths(f);
    const QPointF original = anchor(f, 0, 0);
    const int before = f.view->history_index();
    QVERIFY(dispatch(f, "edit.transform.path.scale"));
    QVERIFY(f.tools->setTransformWidth(200.0, false));
    QVERIFY(pictura::paths_set_active(*f.view, -2));
    QVERIFY(!f.view->transform_session_active());
    QVERIFY(pictura::paths_set_active(*f.view, -1));
    QCOMPARE(anchor(f, 0, 0), original);
    QCOMPARE(f.view->history_index(), before);
    click(f, {25, 25});
    QVERIFY(dispatch(f, "edit.transform.path.scale"));
    QVERIFY(f.tools->setTransformWidth(200.0, false));
    QVERIFY(f.frame.registry()->dispatch(QStringLiteral("edit.undo")));
    QVERIFY(!f.view->transform_session_active());
    QCOMPARE(anchor(f, 0, 0), original);
    QCOMPARE(f.view->history_index(), before);
    const int originalIndex = f.frame.activeDocumentIndex();
    Fixture other(*window_, seed(), QStringLiteral("path_transform_other_document"));
    QVERIFY(other.ok());
    const int otherIndex = other.frame.activeDocumentIndex();
    f.frame.setActiveDocumentIndex(originalIndex);
    f.frame.setActiveTool(pictura::ToolId::PathSelection);
    click(f, {25, 25});
    QVERIFY(dispatch(f, "edit.transform.path.scale"));
    QVERIFY(f.tools->setTransformWidth(200.0, false));
    f.frame.setActiveDocumentIndex(otherIndex);
    f.frame.setActiveDocumentIndex(originalIndex);
    QVERIFY(!f.view->transform_session_active());
    QCOMPARE(anchor(f, 0, 0), original);
    QCOMPARE(f.view->history_index(), before);
}

void PathComponentTransformTest::repeatDocuments()
{
    Fixture first(*window_, seed(), QStringLiteral("path_repeat_doc_first"));
    QVERIFY(first.ok());
    drawPaths(first);
    QVERIFY(dispatch(first, "edit.transform.path.rotate"));
    QVERIFY(first.tools->setTransformAngle(90.0));
    first.tools->commitFreeTransform();
    const int firstIndex = first.frame.activeDocumentIndex();
    Fixture second(*window_, seed(), QStringLiteral("path_repeat_doc_second"));
    QVERIFY(second.ok());
    drawPaths(second);
    second.frame.registry()->refresh();
    QAction* again = second.frame.registry()->action(QStringLiteral("edit.transform.path.again"));
    QVERIFY(again);
    QVERIFY(!again->isEnabled());
    QVERIFY(dispatch(second, "edit.transform.path.scale"));
    QVERIFY(second.tools->setTransformWidth(150.0, false));
    second.tools->commitFreeTransform();
    const int secondIndex = second.frame.activeDocumentIndex();
    second.frame.setActiveDocumentIndex(firstIndex);
    first.frame.setActiveTool(pictura::ToolId::PathSelection);
    click(first, {25, 25});
    first.frame.registry()->refresh();
    QVERIFY(again->isEnabled());
    const QPointF firstBefore = anchor(first, 0, 0);
    QVERIFY(dispatch(first, "edit.transform.path.again"));
    QVERIFY(anchor(first, 0, 0) != firstBefore);
    first.frame.setActiveDocumentIndex(secondIndex);
    second.frame.setActiveTool(pictura::ToolId::PathSelection);
    click(second, {25, 25});
    const QPointF secondBefore = anchor(second, 0, 0);
    QVERIFY(dispatch(second, "edit.transform.path.again"));
    QVERIFY(anchor(second, 0, 0).x() != secondBefore.x());
    QVERIFY(std::abs(anchor(second, 0, 0).y() - secondBefore.y()) < 1e-6);
    QVERIFY(second.frame.closeDocument(secondIndex, false));
    Fixture third(*window_, seed(), QStringLiteral("path_repeat_doc_third"));
    QVERIFY(third.ok());
    drawPaths(third);
    third.frame.registry()->refresh();
    QVERIFY(!again->isEnabled());
    QVERIFY(dispatch(third, "edit.transform.path.rotate"));
    QVERIFY(third.tools->setTransformAngle(90.0));
    third.tools->commitFreeTransform();
    QVERIFY(third.view->open_image(third.path));
    drawPaths(third);
    third.frame.registry()->refresh();
    QVERIFY(!again->isEnabled());
}

void PathComponentTransformTest::targetsAndMetadata()
{
    Fixture f(*window_, seed(), QStringLiteral("path_transform_targets"));
    QVERIFY(f.ok());
    drawPaths(f);
    auto* paths = f.frame.findChild<pictura::PathsPanel*>(QStringLiteral("pathsPanel"));
    QVERIFY(paths);
    QVERIFY(paths->saveWorkPath(QStringLiteral("Saved outline")));
    f.frame.setActiveTool(pictura::ToolId::PathSelection);
    click(f, {25, 25});
    const QPointF saved = anchor(f, 0, 0);
    QVERIFY(dispatch(f, "edit.transform.path.scale"));
    QVERIFY(f.tools->setTransformWidth(150.0, false));
    f.tools->commitFreeTransform();
    QVERIFY(anchor(f, 0, 0) != saved);
    QVERIFY(f.view->undo());
    QCOMPARE(anchor(f, 0, 0), saved);

    pictura::ShapeSpec spec{};
    spec.kind = 0;
    spec.boxed = true;
    spec.x0 = 15;
    spec.y0 = 15;
    spec.x1 = 45;
    spec.y1 = 45;
    const QString layer = pictura::shape_add_layer(*f.view, spec, 0xffff0000u);
    QVERIFY(!layer.isEmpty());
    f.frame.selectLayerPath(layer);
    f.frame.setActiveTool(pictura::ToolId::PathSelection);
    click(f, {30, 30});
    QVERIFY(pictura::path_target_is_live_shape(*f.view));
    const QPointF shapeBefore = anchor(f, 0, 0);
    const int shapeHistory = f.view->history_index();
    QVERIFY(dispatch(f, "edit.transform.path.scale"));
    QVERIFY(f.tools->setTransformWidth(150.0, false));
    f.tools->cancelFreeTransform();
    QCOMPARE(anchor(f, 0, 0), shapeBefore);
    QCOMPARE(f.view->history_index(), shapeHistory);
    QVERIFY(pictura::path_target_is_live_shape(*f.view));
    QVERIFY(dispatch(f, "edit.transform.path.scale"));
    QVERIFY(f.tools->setTransformWidth(150.0, false));
    f.tools->commitFreeTransform();
    QVERIFY(!pictura::path_target_is_live_shape(*f.view));
    QVERIFY(f.view->undo());
    QVERIFY(pictura::path_target_is_live_shape(*f.view));

    const QString raster = f.view->add_solid_fill(0xff2244aau);
    QVERIFY(!raster.isEmpty());
    QVERIFY(pictura::layer_mask_delete_path(*f.view, raster));
    QVERIFY(f.view->rasterize_fill_content(raster));
    f.frame.selectLayerPath(raster);
    QVERIFY(pictura::vector_mask_add(*f.view, QStringLiteral("hide-all")));
    QVERIFY(pictura::vector_mask_set_linked(*f.view, false));
    QVERIFY(pictura::vector_mask_set_enabled(*f.view, false));
    QVERIFY(pictura::paths_set_active(*f.view, -4));
    f.frame.setActiveTool(pictura::ToolId::DirectSelection);
    f.frame.setActiveTool(pictura::ToolId::PathSelection);
    click(f, {50, 50});
    QVERIFY(pictura::path_target_is_layer(*f.view));
    const QPointF maskAnchor = anchor(f, 0, 0);
    QVERIFY(dispatch(f, "edit.transform.path.scale"));
    QVERIFY(f.tools->setTransformWidth(80.0, false));
    f.tools->commitFreeTransform();
    QVERIFY(anchor(f, 0, 0) != maskAnchor);
    QVERIFY(pictura::vector_mask_disabled(*f.view));
    QVERIFY(!pictura::vector_mask_linked(*f.view));
    QVERIFY(f.view->undo());
    QCOMPARE(anchor(f, 0, 0), maskAnchor);
    QVERIFY(pictura::vector_mask_present_path(*f.view, raster));
    QVERIFY(pictura::vector_mask_disabled(*f.view));
    QVERIFY(!pictura::vector_mask_linked(*f.view));
    QVERIFY(f.view->redo());
    QVERIFY(pictura::vector_mask_set_enabled(*f.view, true));
    QCOMPARE(f.view->sample_argb(50, 50), 0xffffffffu);
    QCOMPARE(f.view->sample_argb(5, 5), 0xff2244aau);
    QVERIFY(!pictura::vector_mask_linked(*f.view));
}

PICTURA_QT_TEST_MAIN(PathComponentTransformTest)
#include "tst_path_component_transform.moc"
