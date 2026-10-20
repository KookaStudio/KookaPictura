// Edit > Transform > Distort parity (#748): the corner follows the press
// delta, stops before folding, carries the reference point, snaps to visible
// guides, refuses editable type, and the projective preview clips at the
// horizon.

#include <QtTest/QtTest>

#include "api/transform_api.h"
#include "commands.h"
#include "frame.h"
#include "image_view.h"
#include "selftest_paint_fixture.h"
#include "smart_guides.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/guides.cxxqt.h"
#include "pictura_app/src/cxxqt_object/clipping.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layer_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/vector_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_transform/bridge.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paths.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools/fills.cxxqt.h"

#include "qt_test_support.h"

#include <QtCore/QCoreApplication>

#include <array>
#include <QtGui/QAction>

namespace {

using paint_fixture::Fixture;

constexpr double kTolerance = 1e-6;

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

double turn(const QPolygonF& q, int i)
{
    const QPointF a = q.at(i);
    const QPointF b = q.at((i + 1) % q.size());
    const QPointF c = q.at((i + 2) % q.size());
    return (b.x() - a.x()) * (c.y() - b.y()) - (b.y() - a.y()) * (c.x() - b.x());
}

bool convex(const QPolygonF& q)
{
    const double first = turn(q, 0);
    if (std::abs(first) < 1e-6) {
        return false;
    }
    for (int i = 1; i < q.size(); ++i) {
        if (turn(q, i) * first <= 0.0) {
            return false;
        }
    }
    return true;
}

QAction* action(pictura::PicturaMainWindow& frame, const char* id)
{
    frame.registry()->refresh();
    return frame.registry()->action(QString::fromLatin1(id));
}

} // namespace

class TransformDistortTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();

    void cornerKeepsTheGrabOffset();
    void cornerCommitUsesTheDelta();
    void cornerStopsBeforeFolding();
    void pivotFollowsTheCorner();
    void cornerSnapsToVisibleGuides();
    void steepQuadKeepsTheWholeShape();
    void steepVectorMaskQuadKeepsTheWholeShape();
    void steepClippedFillQuadKeepsTheWholeShape();
    void steepMaskedQuadKeepsTheWholeShape();
    void distortIsRefusedForEditableType();
    void distortCommandEntersTheMode();
    void distortCommandDisabledForEditableType();
    void distortCommandDisabledForGroup();
    void projectivePreviewClipsAtTheHorizon();

private:
    QString beginLayer(Fixture& fixture);
    QString beginTypeLayer(Fixture& fixture);
    void beginDistort(Fixture& fixture, const QString& path);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void TransformDistortTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void TransformDistortTest::cleanup()
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

QString TransformDistortTest::beginLayer(Fixture& fixture)
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

QString TransformDistortTest::beginTypeLayer(Fixture& fixture)
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

void TransformDistortTest::beginDistort(Fixture& fixture, const QString& path)
{
    fixture.frame.selectLayerPath(path);
    QVERIFY(fixture.tools->beginTransformMode(path, QStringLiteral("distort")));
    pictura::transform_set_show_pivot(*fixture.view, false);
}

void TransformDistortTest::cornerKeepsTheGrabOffset()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_distort_grab"));
    QVERIFY(f.ok());
    const QString layer = beginLayer(f);
    QVERIFY(!layer.isEmpty());
    beginDistort(f, layer);

    const QPolygonF before = quadOf(f.view);
    const QPointF corner = before.at(1);
    const QPointF press = corner + QPointF(-2.0, 2.0);
    QCOMPARE(f.view->transform_press(press.x(), press.y(), 1.0, false, false), 1);
    QVERIFY(f.view->transform_move(press.x() + 10.0, press.y() + 5.0, 1.0, false, false));
    const QPolygonF after = quadOf(f.view);
    QVERIFY(samePoint(after.at(1), corner + QPointF(10.0, 5.0)));
    QVERIFY(samePoint(after.at(0), before.at(0)));
    QVERIFY(samePoint(after.at(2), before.at(2)));
    QVERIFY(samePoint(after.at(3), before.at(3)));

    f.view->transform_release();
    f.tools->cancelFreeTransform();
}

void TransformDistortTest::cornerCommitUsesTheDelta()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_distort_commit"));
    QVERIFY(f.ok());
    const QString layer = beginLayer(f);
    QVERIFY(!layer.isEmpty());
    beginDistort(f, layer);

    const QPointF corner = quadOf(f.view).at(1);
    QCOMPARE(f.view->transform_press(corner.x(), corner.y(), 1.0, false, false), 1);
    QVERIFY(f.view->transform_move(corner.x() + 7.0, corner.y() - 5.0, 1.0, false, false));
    QVERIFY(samePoint(quadOf(f.view).at(1), corner + QPointF(7.0, -5.0)));
    f.view->transform_release();
    const int base = f.view->history_index();
    QVERIFY(f.view->commit_transform());
    QCOMPARE(f.view->history_index(), base + 1);
    QCOMPARE(f.view->history_label(base + 1), QStringLiteral("Free Transform"));
}

void TransformDistortTest::cornerStopsBeforeFolding()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_distort_convex"));
    QVERIFY(f.ok());
    const QString layer = beginLayer(f);
    QVERIFY(!layer.isEmpty());
    beginDistort(f, layer);

    const QPolygonF before = quadOf(f.view);
    const QPointF corner = before.at(0);
    QCOMPARE(f.view->transform_press(corner.x(), corner.y(), 1.0, false, false), 0);
    QVERIFY(f.view->transform_move(80.0, 70.0, 1.0, false, false));
    const QPolygonF after = quadOf(f.view);
    QVERIFY2(convex(after), "the quad stayed convex");
    QVERIFY(samePoint(after.at(1), before.at(1)));
    QVERIFY(samePoint(after.at(2), before.at(2)));
    QVERIFY(samePoint(after.at(3), before.at(3)));

    f.view->transform_release();
    f.tools->cancelFreeTransform();
}

void TransformDistortTest::pivotFollowsTheCorner()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_distort_pivot"));
    QVERIFY(f.ok());
    const QString layer = beginLayer(f);
    QVERIFY(!layer.isEmpty());
    beginDistort(f, layer);

    const QPointF before(pictura::transform_pivot_x(*f.view),
                         pictura::transform_pivot_y(*f.view));
    const QPointF corner = quadOf(f.view).at(1);
    QCOMPARE(f.view->transform_press(corner.x(), corner.y(), 1.0, false, false), 1);
    QVERIFY(f.view->transform_move(corner.x() + 15.0, corner.y() + 8.0, 1.0, false, false));
    const QPointF after(pictura::transform_pivot_x(*f.view),
                        pictura::transform_pivot_y(*f.view));
    QVERIFY2(!samePoint(before, after), "the pivot carried with the box");
    QVERIFY(std::isfinite(after.x()) && std::isfinite(after.y()));

    f.view->transform_release();
    f.tools->cancelFreeTransform();
}

void TransformDistortTest::cornerSnapsToVisibleGuides()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_distort_guides"));
    QVERIFY(f.ok());
    const QString layer = beginLayer(f);
    QVERIFY(!layer.isEmpty());
    beginDistort(f, layer);

    const QPointF corner = quadOf(f.view).at(1);
    const QPointF proposed = corner + QPointF(10.0, 5.0);
    const double guideX = proposed.x() + 3.0;
    const double guideY = proposed.y() - 2.0;
    pictura::add_guide(*f.view, true, guideX);
    pictura::add_guide(*f.view, false, guideY);
    f.tools->refreshGuides();

    f.canvas->mousePressed(corner, Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(proposed);
    const QPointF snapped = quadOf(f.view).at(1);
    QVERIFY2(samePoint(snapped, QPointF(guideX, guideY)),
             qPrintable(QStringLiteral("corner %1,%2").arg(snapped.x()).arg(snapped.y())));
    QVERIFY(f.canvas->hasSmartGuidesForTest());
    f.canvas->mouseReleased(proposed);
    QVERIFY(!f.canvas->hasSmartGuidesForTest());

    f.tools->cancelFreeTransform();
}

void TransformDistortTest::steepQuadKeepsTheWholeShape()
{
    pictura::PicturaMainWindow& frame = *window_;
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QTest::qWait(50);

    QImage seed(500, 400, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_distort_steep"));
    QVERIFY(f.ok());
    const int added = f.view->add_layer(0);
    QVERIFY(added > 0);
    const QString layer = QString::number(added);
    f.frame.selectLayerPath(layer);
    QVERIFY(f.view->select_rect(125, 165, 335, 215, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*f.view, qRgba(0, 0, 0, 255), -1, QStringLiteral("normal"),
                               100, false));
    f.view->deselect();
    beginDistort(f, layer);

    const auto moveCorner = [&f](int index, const QPointF& to) {
        const QPointF from = quadOf(f.view).at(index);
        QCOMPARE(f.view->transform_press(from.x(), from.y(), 1.0, false, false), index);
        QVERIFY(f.view->transform_move(to.x(), to.y(), 1.0, false, false));
        f.view->transform_release();
    };
    moveCorner(0, QPointF(125.0, 355.0));
    moveCorner(1, QPointF(445.0, 165.0));
    moveCorner(3, QPointF(240.0, 380.0));
    QVERIFY2(convex(quadOf(f.view)), "the test quad is convex");
    QCoreApplication::processEvents();
    QTest::qWait(50);

    const pictura::transform_api::Matrix matrix = pictura::transform_api::previewMatrix(*f.view);
    QVERIFY(matrix.valid);
    const auto map = [&matrix](double x, double y) {
        const double w = matrix.coefficients[2] * x + matrix.coefficients[5] * y
                         + matrix.coefficients[8];
        return QPointF((matrix.coefficients[0] * x + matrix.coefficients[3] * y
                        + matrix.coefficients[6])
                           / w,
                       (matrix.coefficients[1] * x + matrix.coefficients[4] * y
                        + matrix.coefficients[7])
                           / w);
    };
    const QImage shot = f.canvas->grab().toImage();
    for (const QPointF& source : {QPointF(150.0, 200.0), QPointF(250.0, 200.0),
                                  QPointF(150.0, 300.0), QPointF(250.0, 300.0),
                                  QPointF(350.0, 250.0), QPointF(400.0, 300.0)}) {
        const QPoint mapped = f.canvas->imageToWidget(map(source.x(), source.y())).toPoint();
        QVERIFY2(shot.pixelColor(mapped) == QColor(Qt::black),
                 qPrintable(QStringLiteral("the steep quad keeps its color at %1,%2 (%3)")
                                .arg(mapped.x())
                                .arg(mapped.y())
                                .arg(shot.pixelColor(mapped).name())));
    }
    f.tools->cancelFreeTransform();
}

void TransformDistortTest::steepVectorMaskQuadKeepsTheWholeShape()
{
    pictura::PicturaMainWindow& frame = *window_;
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QTest::qWait(50);

    QImage seed(400, 300, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_distort_steep_mask"));
    QVERIFY(f.ok());
    const int added = f.view->add_layer(0);
    QVERIFY(added > 0);
    const QString layer = QString::number(added);
    f.frame.selectLayerPath(layer);
    QVERIFY(f.view->select_rect(50, 50, 300, 200, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*f.view, qRgba(0, 0, 0, 255), -1, QStringLiteral("normal"),
                               100, false));
    f.view->deselect();
    for (const QPointF& p : {QPointF(50, 50), QPointF(350, 50), QPointF(350, 250),
                             QPointF(50, 250)}) {
        pictura::path_append_corner(*f.view, p.x(), p.y(), false);
        pictura::path_commit_anchor(*f.view);
    }
    pictura::path_close(*f.view);
    QVERIFY(pictura::vector_mask_add(*f.view, QStringLiteral("current-path")));
    beginDistort(f, layer);

    const auto moveCorner = [&f](int index, const QPointF& to) {
        const QPointF from = quadOf(f.view).at(index);
        QCOMPARE(f.view->transform_press(from.x(), from.y(), 1.0, false, false), index);
        QVERIFY(f.view->transform_move(to.x(), to.y(), 1.0, false, false));
        f.view->transform_release();
    };
    moveCorner(0, QPointF(50.0, 230.0));
    moveCorner(1, QPointF(340.0, 50.0));
    moveCorner(3, QPointF(150.0, 250.0));
    QVERIFY2(convex(quadOf(f.view)), "the test quad is convex");
    QCoreApplication::processEvents();
    QTest::qWait(50);

    const pictura::transform_api::Matrix matrix = pictura::transform_api::previewMatrix(*f.view);
    QVERIFY(matrix.valid);
    const auto map = [&matrix](double x, double y) {
        const double w = matrix.coefficients[2] * x + matrix.coefficients[5] * y
                         + matrix.coefficients[8];
        return QPointF((matrix.coefficients[0] * x + matrix.coefficients[3] * y
                        + matrix.coefficients[6])
                           / w,
                       (matrix.coefficients[1] * x + matrix.coefficients[4] * y
                        + matrix.coefficients[7])
                           / w);
    };
    const QImage shot = f.canvas->grab().toImage();
    for (const QPointF& source : {QPointF(100.0, 100.0), QPointF(200.0, 100.0),
                                  QPointF(100.0, 200.0), QPointF(250.0, 200.0),
                                  QPointF(300.0, 100.0)}) {
        const QPoint mapped = f.canvas->imageToWidget(map(source.x(), source.y())).toPoint();
        QVERIFY2(shot.pixelColor(mapped) == QColor(Qt::black),
                 qPrintable(QStringLiteral("the steep mask keeps its color at %1,%2 (%3)")
                                .arg(mapped.x())
                                .arg(mapped.y())
                                .arg(shot.pixelColor(mapped).name())));
    }
    f.tools->cancelFreeTransform();
}

void TransformDistortTest::steepClippedFillQuadKeepsTheWholeShape()
{
    pictura::PicturaMainWindow& frame = *window_;
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QTest::qWait(50);

    QImage seed(500, 400, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_distort_steep_clip"));
    QVERIFY(f.ok());
    const int added = f.view->add_layer(0);
    QVERIFY(added > 0);
    const QString base = QString::number(added);
    f.frame.selectLayerPath(base);
    QVERIFY(f.view->select_rect(125, 165, 335, 215, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*f.view, qRgba(255, 255, 255, 255), -1, QStringLiteral("normal"),
                               100, false));
    f.view->deselect();
    const QString fill = f.view->add_solid_fill(0xff000000u);
    QVERIFY(!fill.isEmpty());
    QVERIFY(pictura::clipping_toggle(*f.view, fill));
    beginDistort(f, base);

    const auto moveCorner = [&f](int index, const QPointF& to) {
        const QPointF from = quadOf(f.view).at(index);
        QCOMPARE(f.view->transform_press(from.x(), from.y(), 1.0, false, false), index);
        QVERIFY(f.view->transform_move(to.x(), to.y(), 1.0, false, false));
        f.view->transform_release();
    };
    moveCorner(0, QPointF(125.0, 355.0));
    moveCorner(1, QPointF(445.0, 165.0));
    moveCorner(3, QPointF(240.0, 380.0));
    QVERIFY2(convex(quadOf(f.view)), "the test quad is convex");
    QCoreApplication::processEvents();
    QTest::qWait(50);

    const pictura::transform_api::Matrix matrix = pictura::transform_api::previewMatrix(*f.view);
    QVERIFY(matrix.valid);
    const auto map = [&matrix](double x, double y) {
        const double w = matrix.coefficients[2] * x + matrix.coefficients[5] * y
                         + matrix.coefficients[8];
        return QPointF((matrix.coefficients[0] * x + matrix.coefficients[3] * y
                        + matrix.coefficients[6])
                           / w,
                       (matrix.coefficients[1] * x + matrix.coefficients[4] * y
                        + matrix.coefficients[7])
                           / w);
    };
    const QImage shot = f.canvas->grab().toImage();
    for (const QPointF& source : {QPointF(150.0, 200.0), QPointF(250.0, 200.0),
                                  QPointF(150.0, 300.0), QPointF(250.0, 300.0),
                                  QPointF(350.0, 250.0), QPointF(400.0, 300.0)}) {
        const QPoint mapped = f.canvas->imageToWidget(map(source.x(), source.y())).toPoint();
        QVERIFY2(shot.pixelColor(mapped) == QColor(Qt::black),
                 qPrintable(QStringLiteral("the steep clip keeps its color at %1,%2 (%3)")
                                .arg(mapped.x())
                                .arg(mapped.y())
                                .arg(shot.pixelColor(mapped).name())));
    }
    f.tools->cancelFreeTransform();
}

void TransformDistortTest::steepMaskedQuadKeepsTheWholeShape()
{
    pictura::PicturaMainWindow& frame = *window_;
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QTest::qWait(50);

    QImage seed(500, 400, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_distort_steep_mask_layer"));
    QVERIFY(f.ok());
    const int added = f.view->add_layer(0);
    QVERIFY(added > 0);
    const QString layer = QString::number(added);
    f.frame.selectLayerPath(layer);
    QVERIFY(f.view->select_rect(125, 165, 335, 215, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*f.view, qRgba(0, 0, 0, 255), -1, QStringLiteral("normal"),
                               100, false));
    f.view->deselect();
    QVERIFY(pictura::layer_mask_add(*f.view, QStringLiteral("reveal-all")));
    beginDistort(f, layer);

    const auto moveCorner = [&f](int index, const QPointF& to) {
        const QPointF from = quadOf(f.view).at(index);
        QCOMPARE(f.view->transform_press(from.x(), from.y(), 1.0, false, false), index);
        QVERIFY(f.view->transform_move(to.x(), to.y(), 1.0, false, false));
        f.view->transform_release();
    };
    moveCorner(0, QPointF(125.0, 355.0));
    moveCorner(1, QPointF(445.0, 165.0));
    moveCorner(3, QPointF(240.0, 380.0));
    QVERIFY2(convex(quadOf(f.view)), "the test quad is convex");
    QCoreApplication::processEvents();
    QTest::qWait(50);

    const pictura::transform_api::Matrix matrix = pictura::transform_api::previewMatrix(*f.view);
    QVERIFY(matrix.valid);
    const auto map = [&matrix](double x, double y) {
        const double w = matrix.coefficients[2] * x + matrix.coefficients[5] * y
                         + matrix.coefficients[8];
        return QPointF((matrix.coefficients[0] * x + matrix.coefficients[3] * y
                        + matrix.coefficients[6])
                           / w,
                       (matrix.coefficients[1] * x + matrix.coefficients[4] * y
                        + matrix.coefficients[7])
                           / w);
    };
    const QImage shot = f.canvas->grab().toImage();
    for (const QPointF& source : {QPointF(150.0, 200.0), QPointF(250.0, 200.0),
                                  QPointF(150.0, 300.0), QPointF(250.0, 300.0),
                                  QPointF(350.0, 250.0), QPointF(400.0, 300.0)}) {
        const QPoint mapped = f.canvas->imageToWidget(map(source.x(), source.y())).toPoint();
        QVERIFY2(shot.pixelColor(mapped) == QColor(Qt::black),
                 qPrintable(QStringLiteral("the steep mask layer keeps its color at %1,%2 (%3)")
                                .arg(mapped.x())
                                .arg(mapped.y())
                                .arg(shot.pixelColor(mapped).name())));
    }
    f.tools->cancelFreeTransform();
}

void TransformDistortTest::distortIsRefusedForEditableType()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_distort_type"));
    QVERIFY(f.ok());
    const QString path = beginTypeLayer(f);
    QVERIFY(!path.isEmpty());

    QVERIFY(!pictura::layer_can_transform_mode(*f.view, path, QStringLiteral("distort")));
    QVERIFY(pictura::layer_can_transform_mode(*f.view, path, QStringLiteral("skew")));
    QVERIFY(!f.view->begin_transform_mode(path, QStringLiteral("distort")));
    QVERIFY(!f.view->transform_session_active());
    QVERIFY(f.view->begin_transform_mode(path, QStringLiteral("skew")));
    f.tools->cancelFreeTransform();
}

void TransformDistortTest::distortCommandEntersTheMode()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_distort_menu"));
    QVERIFY(f.ok());
    const QString layer = beginLayer(f);
    QVERIFY(!layer.isEmpty());
    frame.selectLayerPath(layer);
    QAction* distort = action(frame, pictura::command_ids::EditTransformDistort);
    QVERIFY(distort && distort->isEnabled());
    distort->trigger();
    QVERIFY(f.view->transform_session_active());
    QCOMPARE(pictura::transform_session_mode(*f.view), QStringLiteral("distort"));
    f.view->cancel_transform();
}

void TransformDistortTest::distortCommandDisabledForEditableType()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_distort_type_menu"));
    QVERIFY(f.ok());
    const QString path = beginTypeLayer(f);
    QVERIFY(!path.isEmpty());
    frame.selectLayerPath(path);
    QAction* distort = action(frame, pictura::command_ids::EditTransformDistort);
    QVERIFY(distort && !distort->isEnabled());
    distort->trigger();
    QVERIFY(!f.view->transform_session_active());
}

void TransformDistortTest::distortCommandDisabledForGroup()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_distort_group"));
    QVERIFY(f.ok());
    const QString layer = beginLayer(f);
    QVERIFY(!layer.isEmpty());
    const QString group = f.view->add_group_in(QString());
    QVERIFY(!group.isEmpty());
    frame.selectLayerPath(group);
    QAction* distort = action(frame, pictura::command_ids::EditTransformDistort);
    QVERIFY(distort && !distort->isEnabled());
    distort->trigger();
    QVERIFY(!f.view->transform_session_active());
}

void TransformDistortTest::projectivePreviewClipsAtTheHorizon()
{
    // The source's forward divisor is 1 - 0.02x: positive at x = 0, negative
    // at x = 100. The centre is on the positive side, so the front half-plane
    // is x' > -50 in the local destination.
    QImage source(100, 100, QImage::Format_ARGB32_Premultiplied);
    source.fill(Qt::white);
    const std::array<double, 9> local{1.0, 0.0, -0.02, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0};
    const pictura::WarpedTransformPreview warped = pictura::warpTransformPreview(
        source, QPointF(0.0, 0.0), local, QRectF(0.0, 0.0, 100.0, 100.0),
        QRectF(-100.0, -100.0, 200.0, 200.0));
    QVERIFY(!warped.image.isNull());
    const QPoint front = (QPointF(0.0, 0.0) - warped.origin).toPoint();
    const QPoint far = (QPointF(-100.0, 0.0) - warped.origin).toPoint();
    QVERIFY(warped.image.rect().contains(front));
    QCOMPARE(qAlpha(warped.image.pixel(front)), 255);
    QVERIFY2(!warped.image.rect().contains(far), "the far side is not drawn");

    // An affine map has no horizon and keeps the whole source.
    const std::array<double, 9> identity{1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0};
    const pictura::WarpedTransformPreview plain = pictura::warpTransformPreview(
        source, QPointF(0.0, 0.0), identity, QRectF(0.0, 0.0, 100.0, 100.0),
        QRectF(-100.0, -100.0, 200.0, 200.0));
    QVERIFY(!plain.image.isNull());
    const QPoint middle = (QPointF(50.0, 50.0) - plain.origin).toPoint();
    QVERIFY(plain.image.rect().contains(middle));
    QCOMPARE(qAlpha(plain.image.pixel(middle)), 255);
}

QTEST_MAIN(TransformDistortTest)
#include "tst_transform_distort.moc"
