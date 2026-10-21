// Edit > Transform > Perspective (#750): a corner spreads or pinches its edge
// symmetrically and may cross into a bowtie, an edge handle slides only its
// edge, the reference point keeps its place, editable type is refused, Free
// Transform's Ctrl+Alt+Shift corner drag does the same and keeps the box
// scalable, and the drag shows its angle beside the pointer.

#include <QtTest/QtTest>

#include "api/transform_api.h"
#include "commands.h"
#include "frame.h"
#include "image_view.h"
#include "selftest_paint_fixture.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_transform/bridge.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools/fills.cxxqt.h"

#include "qt_test_support.h"

#include <QtCore/QCoreApplication>
#include <QtGui/QAction>

#include <array>
#include <cmath>

namespace {

using paint_fixture::Fixture;

constexpr double kTolerance = 1e-6;
constexpr Qt::KeyboardModifiers kPerspectiveMods =
    Qt::ControlModifier | Qt::AltModifier | Qt::ShiftModifier;

QImage seed()
{
    QImage image(100, 80, QImage::Format_RGB32);
    image.fill(Qt::white);
    return image;
}

QPolygonF quadOf(pictura::PictureView* view)
{
    return pictura::transform_api::quad(*view).corners;
}

bool samePoint(const QPointF& a, const QPointF& b)
{
    return std::abs(a.x() - b.x()) < kTolerance && std::abs(a.y() - b.y()) < kTolerance;
}

QPointF mid(const QPointF& a, const QPointF& b)
{
    return (a + b) / 2.0;
}

double cross(const QPointF& a, const QPointF& b)
{
    return a.x() * b.y() - a.y() * b.x();
}

QAction* action(pictura::PicturaMainWindow& frame, const char* id)
{
    frame.registry()->refresh();
    return frame.registry()->action(QString::fromLatin1(id));
}

} // namespace

class TransformPerspectiveTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();

    void cornerSpreadsAndCommits();
    void cornersCrossIntoABowtie();
    void edgeSlidesOnlyItsEdge();
    void pivotStaysAtTheCornerAverage();
    void perspectiveIsRefusedForEditableType();
    void perspectiveCommandDisabledForEditableType();
    void freeShortcutDragsPerspective();
    void plainDragScalesThePerspectiveBox();
    void freePerspectiveCommits();
    void freeShortcutIgnoredOnType();
    void readoutShowsAndClears();
    void bowtiePreviewFillsAnHourglass();
    void lopsidedBowtiePreviewHasStraightEdges();

private:
    QString beginLayer(Fixture& fixture);
    QString beginTypeLayer(Fixture& fixture);
    void beginPerspective(Fixture& fixture, const QString& path);
    void beginFree(Fixture& fixture, const QString& path);
    // Ctrl+Alt+Shift-drag the Free box's top-right corner by `delta` through
    // the tool controller, leaving the drag pressed.
    void shortcutDrag(Fixture& fixture, const QPointF& delta);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void TransformPerspectiveTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void TransformPerspectiveTest::cleanup()
{
    if (window_) {
        if (pictura::PictureView* view = window_->activeView();
            view && view->transform_session_active()) {
            view->cancel_transform();
        }
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
}

// A 30 x 22 black box at (10, 8) on its own layer.
QString TransformPerspectiveTest::beginLayer(Fixture& fixture)
{
    pictura::PictureView* view = fixture.view;
    const int added = view->add_layer(0);
    if (added <= 0) {
        return QString();
    }
    const QString layer = QString::number(added);
    fixture.frame.selectLayerPath(layer);
    if (!view->select_rect(10, 8, 30, 22, QStringLiteral("new"), 0.0)
        || !pictura::edit_fill(*view, qRgba(0, 0, 0, 255), -1, QStringLiteral("normal"), 100,
                               false)) {
        return QString();
    }
    view->deselect();
    return layer;
}

QString TransformPerspectiveTest::beginTypeLayer(Fixture& fixture)
{
    fixture.frame.show();
    if (!QTest::qWaitForWindowExposed(&fixture.frame)) {
        return QString();
    }
    fixture.frame.activateWindow();
    QCoreApplication::processEvents();
    fixture.frame.setActiveTool(pictura::ToolId::HorizontalType);
    auto* options =
        fixture.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType);
    if (!options) {
        return QString();
    }
    const pictura::TypeOptions saved = options->get();
    pictura::TypeOptions large = saved;
    large.size = 30.0;
    options->set(large);
    fixture.drag({QPointF(20, 60)});
    QTest::keyClicks(fixture.canvas, QStringLiteral("Type"));
    QTest::keyClick(fixture.canvas, Qt::Key_Return, Qt::ControlModifier);
    const QString path = QString(fixture.view->active_layer_path());
    options->set(saved);
    fixture.frame.setActiveTool(pictura::ToolId::Move);
    return path;
}

void TransformPerspectiveTest::beginPerspective(Fixture& fixture, const QString& path)
{
    fixture.frame.selectLayerPath(path);
    QVERIFY(fixture.tools->beginTransformMode(path, QStringLiteral("perspective")));
    pictura::transform_set_show_pivot(*fixture.view, false);
}

void TransformPerspectiveTest::beginFree(Fixture& fixture, const QString& path)
{
    fixture.frame.selectLayerPath(path);
    QVERIFY(fixture.tools->beginFreeTransform(path));
    pictura::transform_set_show_pivot(*fixture.view, false);
}

void TransformPerspectiveTest::shortcutDrag(Fixture& fixture, const QPointF& delta)
{
    const QPointF corner = quadOf(fixture.view).at(1);
    fixture.canvas->mousePressed(corner, Qt::LeftButton, int(kPerspectiveMods));
    fixture.canvas->mouseMoved(corner + delta);
}

void TransformPerspectiveTest::cornerSpreadsAndCommits()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_perspective_corner"));
    QVERIFY(f.ok());
    const QString layer = beginLayer(f);
    QVERIFY(!layer.isEmpty());
    beginPerspective(f, layer);

    const QPolygonF before = quadOf(f.view);
    const QPointF corner = before.at(1);
    QCOMPARE(f.view->transform_press(corner.x(), corner.y(), 1.0, false, false), 1);
    QVERIFY(f.view->transform_move(corner.x() + 10.0, corner.y() - 2.0, 1.0, false, false));
    const QPolygonF after = quadOf(f.view);
    QVERIFY(samePoint(after.at(1), before.at(1) + QPointF(10.0, 0.0)));
    QVERIFY(samePoint(after.at(0), before.at(0) - QPointF(10.0, 0.0)));
    QVERIFY(samePoint(after.at(2), before.at(2)));
    QVERIFY(samePoint(after.at(3), before.at(3)));

    f.view->transform_release();
    const int base = f.view->history_index();
    QVERIFY(f.view->commit_transform());
    QVERIFY(f.committedOnce(base, "Free Transform"));
}

void TransformPerspectiveTest::cornersCrossIntoABowtie()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_perspective_bowtie"));
    QVERIFY(f.ok());
    const QString layer = beginLayer(f);
    QVERIFY(!layer.isEmpty());
    beginPerspective(f, layer);

    const QPolygonF before = quadOf(f.view);
    const QPointF corner = before.at(1);
    QCOMPARE(f.view->transform_press(corner.x(), corner.y(), 1.0, false, false), 1);
    QVERIFY(f.view->transform_move(corner.x() - 25.0, corner.y(), 1.0, false, false));
    const QPolygonF after = quadOf(f.view);
    QVERIFY2(after.at(0).x() > after.at(1).x(), "the top corners crossed");
    QVERIFY(samePoint(after.at(1), before.at(1) - QPointF(25.0, 0.0)));
    QVERIFY(samePoint(after.at(0), before.at(0) + QPointF(25.0, 0.0)));

    f.view->transform_release();
    f.tools->cancelFreeTransform();
}

void TransformPerspectiveTest::edgeSlidesOnlyItsEdge()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_perspective_edge"));
    QVERIFY(f.ok());
    const QString layer = beginLayer(f);
    QVERIFY(!layer.isEmpty());
    beginPerspective(f, layer);

    const QPolygonF before = quadOf(f.view);
    const QPointF top = mid(before.at(0), before.at(1));
    QCOMPARE(f.view->transform_press(top.x(), top.y(), 1.0, false, false), 4);
    QVERIFY(f.view->transform_move(top.x() + 5.0, top.y() + 4.0, 1.0, false, false));
    const QPolygonF after = quadOf(f.view);
    QVERIFY(samePoint(after.at(0), before.at(0) + QPointF(5.0, 0.0)));
    QVERIFY(samePoint(after.at(1), before.at(1) + QPointF(5.0, 0.0)));
    QVERIFY(samePoint(after.at(2), before.at(2)));
    QVERIFY(samePoint(after.at(3), before.at(3)));

    f.view->transform_release();
    f.tools->cancelFreeTransform();
}

void TransformPerspectiveTest::pivotStaysAtTheCornerAverage()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_perspective_pivot"));
    QVERIFY(f.ok());
    const QString layer = beginLayer(f);
    QVERIFY(!layer.isEmpty());
    beginPerspective(f, layer);

    const QPointF corner = quadOf(f.view).at(1);
    QCOMPARE(f.view->transform_press(corner.x(), corner.y(), 1.0, false, false), 1);
    QVERIFY(f.view->transform_move(corner.x() + 1.0, corner.y() - 6.0, 1.0, false, false));
    const QPolygonF after = quadOf(f.view);
    QPointF mean;
    for (const QPointF& p : after) {
        mean += p / 4.0;
    }
    const QPointF pivot(pictura::transform_pivot_x(*f.view), pictura::transform_pivot_y(*f.view));
    QVERIFY2(samePoint(pivot, mean),
             qPrintable(QStringLiteral("pivot %1,%2 mean %3,%4")
                            .arg(pivot.x())
                            .arg(pivot.y())
                            .arg(mean.x())
                            .arg(mean.y())));

    f.view->transform_release();
    f.tools->cancelFreeTransform();
}

void TransformPerspectiveTest::perspectiveIsRefusedForEditableType()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_perspective_type"));
    QVERIFY(f.ok());
    const QString path = beginTypeLayer(f);
    QVERIFY(!path.isEmpty());
    const int base = f.view->history_index();
    frame.selectLayerPath(path);
    QVERIFY(!f.tools->beginTransformMode(path, QStringLiteral("perspective")));
    QVERIFY(!f.view->transform_session_active());
    QCOMPARE(f.view->history_index(), base);
}

void TransformPerspectiveTest::perspectiveCommandDisabledForEditableType()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_perspective_type_menu"));
    QVERIFY(f.ok());
    const QString path = beginTypeLayer(f);
    QVERIFY(!path.isEmpty());
    frame.selectLayerPath(path);
    QAction* perspective = action(frame, pictura::command_ids::EditTransformPerspective);
    QVERIFY(perspective && !perspective->isEnabled());
    perspective->trigger();
    QVERIFY(!f.view->transform_session_active());
}

void TransformPerspectiveTest::freeShortcutDragsPerspective()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_perspective_free"));
    QVERIFY(f.ok());
    const QString layer = beginLayer(f);
    QVERIFY(!layer.isEmpty());
    beginFree(f, layer);

    const QPolygonF before = quadOf(f.view);
    shortcutDrag(f, QPointF(10.0, -2.0));
    const QPolygonF after = quadOf(f.view);
    QCOMPARE(pictura::transform_session_mode(*f.view), QStringLiteral("free"));
    QVERIFY(samePoint(after.at(1), before.at(1) + QPointF(10.0, 0.0)));
    QVERIFY(samePoint(after.at(0), before.at(0) - QPointF(10.0, 0.0)));
    QVERIFY(samePoint(after.at(2), before.at(2)));
    QVERIFY(samePoint(after.at(3), before.at(3)));

    f.canvas->mouseReleased(after.at(1));
    f.tools->cancelFreeTransform();
}

void TransformPerspectiveTest::plainDragScalesThePerspectiveBox()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_perspective_free_scale"));
    QVERIFY(f.ok());
    const QString layer = beginLayer(f);
    QVERIFY(!layer.isEmpty());
    beginFree(f, layer);
    shortcutDrag(f, QPointF(10.0, 0.0));
    f.canvas->mouseReleased(quadOf(f.view).at(1));

    const QPolygonF before = quadOf(f.view);
    const QPointF right = mid(before.at(1), before.at(2));
    QCOMPARE(f.view->transform_press(right.x(), right.y(), 1.0, false, false), 5);
    QVERIFY(f.view->transform_move(right.x() + 12.0, right.y(), 1.0, false, false));
    const QPolygonF after = quadOf(f.view);
    QVERIFY(samePoint(after.at(0), before.at(0)));
    QVERIFY(samePoint(after.at(3), before.at(3)));
    QVERIFY(after.at(1).x() > before.at(1).x() && after.at(2).x() > before.at(2).x());
    const QPointF left = after.at(3) - after.at(0);
    const QPointF side = after.at(2) - after.at(1);
    QVERIFY2(std::abs(cross(left, side)) > 1e-6, "the box kept its perspective");

    f.view->transform_release();
    f.tools->cancelFreeTransform();
}

void TransformPerspectiveTest::freePerspectiveCommits()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_perspective_free_commit"));
    QVERIFY(f.ok());
    const QString layer = beginLayer(f);
    QVERIFY(!layer.isEmpty());
    beginFree(f, layer);
    shortcutDrag(f, QPointF(8.0, 0.0));
    const QPolygonF quad = quadOf(f.view);
    f.canvas->mouseReleased(quad.at(1));

    const int base = f.view->history_index();
    QVERIFY(f.view->commit_transform());
    QVERIFY(f.committedOnce(base, "Free Transform"));
    const QRectF bounds = quad.boundingRect();
    const ::rust::Vec<int32_t> rect = pictura::layer_rect_extents(*f.view, layer);
    QCOMPARE(rect.size(), std::size_t(4));
    QCOMPARE(rect[0], int32_t(std::floor(bounds.left())));
    QCOMPARE(rect[1], int32_t(std::floor(bounds.top())));
    QCOMPARE(rect[2], int32_t(std::ceil(bounds.right())));
    QCOMPARE(rect[3], int32_t(std::ceil(bounds.bottom())));
}

void TransformPerspectiveTest::freeShortcutIgnoredOnType()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_perspective_free_type"));
    QVERIFY(f.ok());
    const QString path = beginTypeLayer(f);
    QVERIFY(!path.isEmpty());
    beginFree(f, path);

    const QPolygonF before = quadOf(f.view);
    shortcutDrag(f, QPointF(10.0, -2.0));
    const QPolygonF after = quadOf(f.view);
    QVERIFY2(std::abs(after.at(0).x() - before.at(0).x()) < kTolerance,
             "a scale keeps the left edge; a perspective drag would move it");
    QVERIFY(after.at(1).x() > before.at(1).x());

    f.canvas->mouseReleased(after.at(1));
    f.tools->cancelFreeTransform();
}

void TransformPerspectiveTest::readoutShowsAndClears()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_perspective_readout"));
    QVERIFY(f.ok());
    const QString layer = beginLayer(f);
    QVERIFY(!layer.isEmpty());
    beginPerspective(f, layer);

    // The box is 22 px tall: spreading the top edge by 22 px turns the right
    // side 45 degrees.
    const QPointF corner = quadOf(f.view).at(1);
    f.canvas->mousePressed(corner, Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(corner + QPointF(22.0, 0.0));
    QCOMPARE(f.canvas->dragSizeHintForTest(), QStringLiteral("∠: -45.0°"));
    f.canvas->mouseReleased(corner + QPointF(22.0, 0.0));
    QVERIFY(f.canvas->dragSizeHintForTest().isEmpty());

    f.tools->cancelFreeTransform();
}

void TransformPerspectiveTest::bowtiePreviewFillsAnHourglass()
{
    // The top corners have swapped sides; the side edges cross at (50, 37.5).
    QImage source(100, 100, QImage::Format_ARGB32_Premultiplied);
    source.fill(Qt::white);
    // x' = (-0.6x - 0.8y + 80) / w, y' = -0.6y / w, w = 1 - 0.016y: the
    // homography onto (80,0), (20,0), (100,100), (0,100).
    const std::array<double, 9> coefficients{-0.6, 0.0, 0.0, -0.8, -0.6, -0.016, 80.0, 0.0, 1.0};
    const pictura::WarpedTransformPreview warped = pictura::warpTransformPreview(
        source, QPointF(0.0, 0.0), coefficients, QRectF(0.0, 0.0, 100.0, 100.0),
        QRectF(-100.0, -100.0, 300.0, 300.0), 1.0);
    QVERIFY(warped.crossed);
    QVERIFY(!warped.image.isNull());
    const auto alphaAt = [&warped](const QPointF& destination) {
        const QPoint pixel = (destination - warped.origin).toPoint();
        return warped.image.rect().contains(pixel) ? qAlpha(warped.image.pixel(pixel)) : 0;
    };
    QCOMPARE(alphaAt(QPointF(50.5, 10.5)), 255);
    QCOMPARE(alphaAt(QPointF(50.5, 80.5)), 255);
    QCOMPARE(alphaAt(QPointF(5.5, 5.5)), 0);
    QCOMPARE(alphaAt(QPointF(5.5, 50.5)), 0);
    QCOMPARE(alphaAt(QPointF(50.5, -20.5)), 0);
}

void TransformPerspectiveTest::lopsidedBowtiePreviewHasStraightEdges()
{
    // The homography onto (100,0), (0,20), (120,100), (0,100): the top corners
    // swapped with a tilted top edge, so the side edges cross off-centre at
    // (48, 52). The preview must fill exactly the two straight triangles, with
    // no curved waist.
    QImage source(100, 100, QImage::Format_ARGB32_Premultiplied);
    source.fill(Qt::white);
    const std::array<double, 9> coefficients{-1.0, 0.25, 0.0025, -1.0, -1.0833333333333333,
                                             -0.020833333333333332, 100.0, 0.0, 1.0};
    const pictura::WarpedTransformPreview warped = pictura::warpTransformPreview(
        source, QPointF(0.0, 0.0), coefficients, QRectF(0.0, 0.0, 100.0, 100.0),
        QRectF(-50.0, -50.0, 250.0, 250.0), 1.0);
    QVERIFY(warped.crossed);
    QVERIFY(!warped.image.isNull());
    const QPolygonF upper({QPointF(100, 0), QPointF(0, 20), QPointF(48, 52)});
    const QPolygonF lower({QPointF(48, 52), QPointF(120, 100), QPointF(0, 100)});
    // A pixel centre exactly on a triangle edge may fall either way.
    const auto onEdge = [&upper, &lower](const QPointF& p) {
        for (const QPolygonF* tri : {&upper, &lower}) {
            for (int i = 0; i < 3; ++i) {
                const QLineF edge(tri->at(i), tri->at((i + 1) % 3));
                const QPointF d = edge.p2() - edge.p1();
                if (std::abs(cross(d, p - edge.p1())) / edge.length() < 1e-6) {
                    return true;
                }
            }
        }
        return false;
    };
    int mismatches = 0;
    for (int y = 0; y < 100; ++y) {
        for (int x = 0; x < 120; ++x) {
            const QPointF p(x + 0.5, y + 0.5);
            const QPointF local = p - warped.origin;
            const QPoint pixel(int(std::floor(local.x())), int(std::floor(local.y())));
            const bool drawn = warped.image.rect().contains(pixel)
                && qAlpha(warped.image.pixel(pixel)) == 255;
            const bool want = upper.containsPoint(p, Qt::OddEvenFill)
                || lower.containsPoint(p, Qt::OddEvenFill);
            mismatches += drawn != want && !onEdge(p) ? 1 : 0;

        }
    }
    QVERIFY2(mismatches == 0, qPrintable(QStringLiteral("%1 mismatched pixels").arg(mismatches)));
    // Beside the waist, outside both triangles.
    QCOMPARE(qAlpha(warped.image.pixel((QPointF(70.5, 55.5) - warped.origin).toPoint())), 0);
}

QTEST_MAIN(TransformPerspectiveTest)
#include "tst_transform_perspective.moc"
