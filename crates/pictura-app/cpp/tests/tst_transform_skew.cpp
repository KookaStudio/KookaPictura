// Edit > Transform > Skew gestures: a corner follows its dominant adjacent edge,
// an edge slides only along itself, and Alt mirrors the opposite corner or edge.

#include <QtTest/QtTest>

#include "api/transform_api.h"
#include "api/type_api.h"
#include "frame.h"
#include "image_view.h"
#include "selftest_paint_fixture.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/clipping.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layer_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/vector_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_transform/bridge.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layer_style.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools/fills.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paths.cxxqt.h"

#include "qt_test_support.h"

#include <QtWidgets/QLabel>

namespace {

constexpr double kTolerance = 1e-6;

QPolygonF quadOf(pictura::PictureView* view)
{
    return pictura::transform_api::quad(*view).corners;
}

QRect layerRectOf(pictura::PictureView* view, const QString& path)
{
    const QStringList parts =
        pictura::transform_api::layerRect(*view, path).toString().split(QLatin1Char(' '));
    if (parts.size() != 4) {
        return QRect();
    }
    return QRect(QPoint(parts[0].toInt(), parts[1].toInt()),
                 QPoint(parts[2].toInt() - 1, parts[3].toInt() - 1));
}

bool samePoint(const QPointF& a, const QPointF& b)
{
    return std::abs(a.x() - b.x()) < kTolerance && std::abs(a.y() - b.y()) < kTolerance;
}

} // namespace

class TransformSkewTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();

    void cornerMovesAlongItsDominantEdge();
    void edgeSlidesOnlyAlongItself();
    void altMirrorsTheOppositeEdge();
    void altMirrorsTheOppositeCorner();
    void previewKeepsTheLayerColor();
    void twoCornerDragsKeepTheLayerColor();
    void cornerCommitKeepsTheLayerColor();
    void strongPerspectiveKeepsTheLayerColor();
    void pixelsShapeToolKeepsTheLayerColor();
    void vectorMaskKeepsTheLayerColor();
    void maskedLayerKeepsTheLayerColor();
    void styleKeepsTheLayerColor();
    void clippedFillKeepsTheLayerColor();
    void thinLineKeepsTheLayerColor();
    void typeKeepsTheLayerColor();
    void statusShowsTheSkewAction();

private:
    // A new document with one filled raster layer selected; returns its path.
    QString beginLayer(pictura::PicturaMainWindow& frame);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void TransformSkewTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void TransformSkewTest::cleanup()
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

QString TransformSkewTest::beginLayer(pictura::PicturaMainWindow& frame)
{
    if (!frame.newDocument(QStringLiteral("Skew"), 100, 80, QStringLiteral("rgb"), 8,
                           QStringLiteral("white"))) {
        return QString();
    }
    pictura::PictureView* view = frame.activeView();
    if (!view) {
        return QString();
    }
    const int added = view->add_layer(0);
    if (added <= 0) {
        return QString();
    }
    const QString layer = QString::number(added);
    frame.selectLayerPath(layer);
    if (!view->select_rect(10, 8, 40, 30, QStringLiteral("new"), 0.0)
        || !pictura::edit_fill(*view, qRgba(0, 0, 0, 255), -1, QStringLiteral("normal"), 100,
                               false)) {
        return QString();
    }
    view->deselect();
    return layer;
}

void TransformSkewTest::cornerMovesAlongItsDominantEdge()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools);
    QVERIFY(tools->beginTransformMode(layer, QStringLiteral("skew")));
    pictura::transform_set_show_pivot(*view, false);

    const QPolygonF before = quadOf(view);
    QCOMPARE(before.size(), 4);
    const QPointF corner = before.at(1);
    QCOMPARE(view->transform_press(corner.x(), corner.y(), 1.0, false, false), 1);
    QVERIFY(view->transform_move(corner.x() + 10.0, corner.y() + 3.0, 1.0, false, false));

    const QPolygonF after = quadOf(view);
    QVERIFY(samePoint(after.at(1), QPointF(corner.x() + 10.0, corner.y())));
    QVERIFY(samePoint(after.at(0), before.at(0)));
    QVERIFY(samePoint(after.at(2), before.at(2)));
    QVERIFY(samePoint(after.at(3), before.at(3)));

    view->transform_release();
    tools->cancelFreeTransform();
}

void TransformSkewTest::edgeSlidesOnlyAlongItself()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools);
    QVERIFY(tools->beginTransformMode(layer, QStringLiteral("skew")));
    pictura::transform_set_show_pivot(*view, false);

    const QPolygonF before = quadOf(view);
    QCOMPARE(before.size(), 4);
    const QPointF mid((before.at(0).x() + before.at(1).x()) / 2.0, before.at(0).y());
    QCOMPARE(view->transform_press(mid.x(), mid.y(), 1.0, false, false), 4);
    QVERIFY(view->transform_move(mid.x() + 2.0, mid.y() + 3.0, 1.0, false, false));

    const QPolygonF after = quadOf(view);
    QVERIFY(samePoint(after.at(0), QPointF(before.at(0).x() + 2.0, before.at(0).y())));
    QVERIFY(samePoint(after.at(1), QPointF(before.at(1).x() + 2.0, before.at(1).y())));
    QVERIFY(samePoint(after.at(2), before.at(2)));
    QVERIFY(samePoint(after.at(3), before.at(3)));

    view->transform_release();
    tools->cancelFreeTransform();
}

void TransformSkewTest::altMirrorsTheOppositeEdge()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools);
    QVERIFY(tools->beginTransformMode(layer, QStringLiteral("skew")));
    pictura::transform_set_show_pivot(*view, false);

    const QPolygonF before = quadOf(view);
    QCOMPARE(before.size(), 4);
    const QPointF mid((before.at(0).x() + before.at(1).x()) / 2.0, before.at(0).y());
    QCOMPARE(view->transform_press(mid.x(), mid.y(), 1.0, false, false), 4);
    QVERIFY(view->transform_move(mid.x() + 2.0, mid.y() + 3.0, 1.0, false, true));

    const QPolygonF after = quadOf(view);
    QVERIFY(samePoint(after.at(0), QPointF(before.at(0).x() + 2.0, before.at(0).y())));
    QVERIFY(samePoint(after.at(1), QPointF(before.at(1).x() + 2.0, before.at(1).y())));
    QVERIFY(samePoint(after.at(2), QPointF(before.at(2).x() - 2.0, before.at(2).y())));
    QVERIFY(samePoint(after.at(3), QPointF(before.at(3).x() - 2.0, before.at(3).y())));

    view->transform_release();
    tools->cancelFreeTransform();
}

void TransformSkewTest::altMirrorsTheOppositeCorner()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools);
    QVERIFY(tools->beginTransformMode(layer, QStringLiteral("skew")));
    pictura::transform_set_show_pivot(*view, false);

    const QPolygonF before = quadOf(view);
    QCOMPARE(before.size(), 4);
    const QPointF corner = before.at(1);
    QCOMPARE(view->transform_press(corner.x(), corner.y(), 1.0, false, false), 1);
    QVERIFY(view->transform_move(corner.x() + 10.0, corner.y() + 3.0, 1.0, false, true));

    const QPolygonF after = quadOf(view);
    QVERIFY(samePoint(after.at(1), QPointF(corner.x() + 10.0, corner.y())));
    QVERIFY(samePoint(after.at(0), before.at(0)));
    QVERIFY(samePoint(after.at(2), before.at(2)));
    QVERIFY(samePoint(after.at(3), QPointF(before.at(3).x() - 10.0, before.at(3).y())));

    view->transform_release();
    tools->cancelFreeTransform();
}

void TransformSkewTest::previewKeepsTheLayerColor()
{
    pictura::PicturaMainWindow& frame = *window_;
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QTest::qWait(50);

    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && canvas && tools);
    QCOMPARE(view->sample_argb(20, 15), qRgba(0, 0, 0, 255));
    canvas->setZoom(0.83, QPointF(canvas->width() / 2.0, canvas->height() / 2.0));

    QCoreApplication::processEvents();
    QTest::qWait(50);
    const QPoint plain = canvas->imageToWidget(QPointF(20.0, 15.0)).toPoint();
    QVERIFY2(canvas->grab().toImage().pixelColor(plain) == QColor(Qt::black),
             "the plain document shows the filled layer");

    QVERIFY(tools->beginTransformMode(layer, QStringLiteral("skew")));
    pictura::transform_set_show_pivot(*view, false);
    QCoreApplication::processEvents();
    QTest::qWait(50);
    const QPoint preview = canvas->imageToWidget(QPointF(20.0, 15.0)).toPoint();
    QVERIFY2(canvas->grab().toImage().pixelColor(preview) == QColor(Qt::black),
             "the live preview keeps the filled layer");

    const QPolygonF quad = quadOf(view);
    QCOMPARE(quad.size(), 4);
    const QPointF corner = quad.at(1);
    const double zoom = canvas->zoom();
    QCOMPARE(view->transform_press(corner.x(), corner.y(), zoom, false, false), 1);
    QVERIFY(view->transform_move(corner.x(), corner.y() + 22.0, zoom, false, false));
    QCoreApplication::processEvents();
    QTest::qWait(50);

    const QPoint moved = canvas->imageToWidget(QPointF(20.0, 20.0)).toPoint();
    QVERIFY2(canvas->grab().toImage().pixelColor(moved) == QColor(Qt::black),
             "the drag keeps the filled layer");
    view->transform_release();
    tools->cancelFreeTransform();
}

// Two corner drags (top-left up, top-right down) leave a trapezoid whose top
// edge slants; the live preview must keep the layer's pixels.
void TransformSkewTest::twoCornerDragsKeepTheLayerColor()
{
    pictura::PicturaMainWindow& frame = *window_;
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QTest::qWait(50);

    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && canvas && tools);
    QCOMPARE(view->sample_argb(20, 15), qRgba(0, 0, 0, 255));

    QVERIFY(tools->beginTransformMode(layer, QStringLiteral("skew")));
    pictura::transform_set_show_pivot(*view, false);
    QPolygonF quad = quadOf(view);
    QCOMPARE(quad.size(), 4);
    canvas->mousePressed(quad.at(0), Qt::LeftButton, 0);
    canvas->mouseMoved(quad.at(0) + QPointF(0.0, -6.0));
    canvas->mouseReleased(quad.at(0) + QPointF(0.0, -6.0));

    quad = quadOf(view);
    canvas->mousePressed(quad.at(1), Qt::LeftButton, 0);
    canvas->mouseMoved(quad.at(1) + QPointF(0.0, 6.0));
    canvas->mouseReleased(quad.at(1) + QPointF(0.0, 6.0));
    QCoreApplication::processEvents();
    QTest::qWait(50);

    const pictura::transform_api::Matrix matrix = pictura::transform_api::previewMatrix(*view);
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
    const QImage shot = canvas->grab().toImage();
    for (const QPointF& source : {QPointF(20.0, 15.0), QPointF(30.0, 25.0),
                                  QPointF(15.0, 25.0), QPointF(15.0, 10.0)}) {
        const QPoint mapped = canvas->imageToWidget(map(source.x(), source.y())).toPoint();
        QVERIFY2(shot.pixelColor(mapped) == QColor(Qt::black),
                 qPrintable(QStringLiteral("the skewed layer keeps its color at %1,%2 (%3)")
                                .arg(mapped.x())
                                .arg(mapped.y())
                                .arg(shot.pixelColor(mapped).name())));
    }
    tools->cancelFreeTransform();
}

// The committed corner skew must keep the layer's color where it lands.
void TransformSkewTest::cornerCommitKeepsTheLayerColor()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && tools);
    QCOMPARE(view->sample_argb(20, 15), qRgba(0, 0, 0, 255));

    QVERIFY(tools->beginTransformMode(layer, QStringLiteral("skew")));
    pictura::transform_set_show_pivot(*view, false);
    QPolygonF quad = quadOf(view);
    QCOMPARE(quad.size(), 4);
    QCOMPARE(view->transform_press(quad.at(0).x(), quad.at(0).y(), 1.0, false, false), 0);
    QVERIFY(view->transform_move(quad.at(0).x(), quad.at(0).y() - 6.0, 1.0, false, false));
    view->transform_release();
    quad = quadOf(view);
    QCOMPARE(view->transform_press(quad.at(1).x(), quad.at(1).y(), 1.0, false, false), 1);
    QVERIFY(view->transform_move(quad.at(1).x(), quad.at(1).y() + 6.0, 1.0, false, false));
    view->transform_release();
    const pictura::transform_api::Matrix matrix = pictura::transform_api::previewMatrix(*view);
    QVERIFY(matrix.valid);
    QVERIFY(tools->transformSessionActive());
    tools->commitFreeTransform();
    QVERIFY(!tools->transformSessionActive());

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
    for (const QPointF& source : {QPointF(20.0, 15.0), QPointF(30.0, 25.0),
                                  QPointF(15.0, 25.0)}) {
        const QPointF target = map(source.x(), source.y());
        QVERIFY2(view->sample_argb(qRound(target.x()), qRound(target.y())) == qRgba(0, 0, 0, 255),
                 qPrintable(QStringLiteral("the committed layer keeps its color at %1,%2")
                                .arg(target.x())
                                .arg(target.y())));
    }
}

// A corner drag past the centre makes a strong perspective: the right edge
// collapses toward the bottom corner. The live preview must still draw the
// layer, not drop it when Qt's raster engine sees the extreme homography.
void TransformSkewTest::strongPerspectiveKeepsTheLayerColor()
{
    pictura::PicturaMainWindow& frame = *window_;
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QTest::qWait(50);
    QVERIFY(frame.newDocument(QStringLiteral("StrongPerspective"), 1280, 800, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && canvas && tools);
    const int added = view->add_layer(0);
    QVERIFY(added > 0);
    const QString layer = QString::number(added);
    frame.selectLayerPath(layer);
    QVERIFY(view->select_rect(354, 234, 673, 545, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*view, qRgba(0, 0, 0, 255), -1, QStringLiteral("normal"), 100,
                               false));
    view->deselect();
    QCoreApplication::processEvents();
    QTest::qWait(100);
    canvas->grab();
    QCoreApplication::processEvents();

    QVERIFY(tools->beginTransformMode(layer, QStringLiteral("skew")));
    pictura::transform_set_show_pivot(*view, false);
    const QPolygonF quad = quadOf(view);
    QCOMPARE(quad.size(), 4);
    const QPointF corner = quad.at(1);
    canvas->mousePressed(corner, Qt::LeftButton, 0);
    canvas->mouseMoved(QPointF(corner.x(), 500.0));
    QCoreApplication::processEvents();
    QTest::qWait(50);

    const pictura::transform_api::Matrix matrix = pictura::transform_api::previewMatrix(*view);
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
    const QImage shot = canvas->grab().toImage();
    for (const QPointF& source : {QPointF(500.0, 300.0), QPointF(500.0, 400.0),
                                  QPointF(400.0, 500.0)}) {
        const QPoint mapped = canvas->imageToWidget(map(source.x(), source.y())).toPoint();
        QVERIFY2(shot.pixelColor(mapped) == QColor(Qt::black),
                 qPrintable(QStringLiteral("the skewed layer keeps its color at %1,%2 (%3)")
                                .arg(mapped.x())
                                .arg(mapped.y())
                                .arg(shot.pixelColor(mapped).name())));
    }
    view->transform_release();
    tools->cancelFreeTransform();
}

// A rectangle drawn with the shape tool in Pixels mode is the raster layer the
// report used; a corner drag must keep its fill.
void TransformSkewTest::pixelsShapeToolKeepsTheLayerColor()
{
    pictura::PicturaMainWindow& frame = *window_;
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QTest::qWait(50);
    QImage seed(1280, 800, QImage::Format_RGB32);
    seed.fill(Qt::white);
    paint_fixture::Fixture f(frame, seed, QStringLiteral("pictura_pixels_shape"));
    QVERIFY2(f.ok(), "pixels shape fixture");
    frame.setActiveTool(pictura::ToolId::Rectangle);
    auto* options = f.tools->optionsFor<pictura::ShapeGroup>(pictura::ToolId::Rectangle);
    const pictura::ShapeOptions saved = options->get();
    pictura::ShapeOptions o = saved;
    o.mode = 2;
    o.fillEnabled = true;
    o.fillColor = Qt::black;
    o.strokeEnabled = false;
    options->set(o);
    const int added = f.view->add_layer(0);
    QVERIFY(added > 0);
    const QString layer = QString::number(added);
    frame.selectLayerPath(layer);
    f.drag({QPointF(224, 153), QPointF(330, 220), QPointF(433, 296)});
    options->set(saved);
    frame.setActiveTool(pictura::ToolId::Move);
    QCOMPARE(f.view->sample_argb(300, 200), qRgba(0, 0, 0, 255));

    QVERIFY(f.tools->beginTransformMode(layer, QStringLiteral("skew")));
    pictura::transform_set_show_pivot(*f.view, false);
    const QPolygonF quad = quadOf(f.view);
    QCOMPARE(quad.size(), 4);
    const QPointF corner = quad.at(1);
    f.canvas->mousePressed(corner, Qt::LeftButton, 0);
    f.canvas->mouseMoved(corner + QPointF(-60.0, 0.0));
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
    for (const QPointF& source : {QPointF(300.0, 200.0), QPointF(400.0, 270.0),
                                  QPointF(250.0, 270.0), QPointF(230.0, 160.0)}) {
        const QPoint mapped = f.canvas->imageToWidget(map(source.x(), source.y())).toPoint();
        QVERIFY2(shot.pixelColor(mapped) == QColor(Qt::black),
                 qPrintable(QStringLiteral("the pixels-mode shape keeps its color at %1,%2 (%3)")
                                .arg(mapped.x())
                                .arg(mapped.y())
                                .arg(shot.pixelColor(mapped).name())));
    }
    f.tools->cancelFreeTransform();
}

// A raster layer clipped by a vector mask is rendered through the compositor
// for the preview; a corner drag must keep the clipped pixels.
void TransformSkewTest::vectorMaskKeepsTheLayerColor()
{
    pictura::PicturaMainWindow& frame = *window_;
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QTest::qWait(50);

    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && canvas && tools);
    const QList<QPointF> corners {QPointF(10, 8), QPointF(40, 8), QPointF(40, 30),
                                  QPointF(10, 30)};
    for (const QPointF& p : corners) {
        pictura::path_append_corner(*view, p.x(), p.y(), false);
        pictura::path_commit_anchor(*view);
    }
    pictura::path_close(*view);
    QVERIFY(pictura::vector_mask_add(*view, QStringLiteral("current-path")));
    QCOMPARE(view->sample_argb(20, 15), qRgba(0, 0, 0, 255));

    QVERIFY(tools->beginTransformMode(layer, QStringLiteral("skew")));
    pictura::transform_set_show_pivot(*view, false);
    const QPolygonF quad = quadOf(view);
    QCOMPARE(quad.size(), 4);
    const QPointF corner = quad.at(1);
    canvas->mousePressed(corner, Qt::LeftButton, 0);
    canvas->mouseMoved(corner + QPointF(-15.0, 0.0));
    QCoreApplication::processEvents();
    QTest::qWait(50);

    const pictura::transform_api::Matrix matrix = pictura::transform_api::previewMatrix(*view);
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
    const QImage shot = canvas->grab().toImage();
    for (const QPointF& source : {QPointF(20.0, 15.0), QPointF(30.0, 25.0)}) {
        const QPoint mapped = canvas->imageToWidget(map(source.x(), source.y())).toPoint();
        QVERIFY2(shot.pixelColor(mapped) == QColor(Qt::black),
                 qPrintable(QStringLiteral("the vector-masked layer keeps its color at %1,%2 (%3)")
                                .arg(mapped.x())
                                .arg(mapped.y())
                                .arg(shot.pixelColor(mapped).name())));
    }
    tools->cancelFreeTransform();
}
void TransformSkewTest::maskedLayerKeepsTheLayerColor()
{
    pictura::PicturaMainWindow& frame = *window_;
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QTest::qWait(50);

    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && canvas && tools);
    QVERIFY(pictura::layer_mask_add(*view, QStringLiteral("reveal-all")));
    QCOMPARE(view->sample_argb(20, 15), qRgba(0, 0, 0, 255));

    QVERIFY(tools->beginTransformMode(layer, QStringLiteral("skew")));
    pictura::transform_set_show_pivot(*view, false);
    const QPolygonF quad = quadOf(view);
    QCOMPARE(quad.size(), 4);
    const QPointF corner = quad.at(1);
    canvas->mousePressed(corner, Qt::LeftButton, 0);
    canvas->mouseMoved(corner + QPointF(-15.0, 0.0));
    QCoreApplication::processEvents();
    QTest::qWait(50);

    const pictura::transform_api::Matrix matrix = pictura::transform_api::previewMatrix(*view);
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
    const QImage shot = canvas->grab().toImage();
    for (const QPointF& source : {QPointF(20.0, 15.0), QPointF(30.0, 25.0)}) {
        const QPoint mapped = canvas->imageToWidget(map(source.x(), source.y())).toPoint();
        QVERIFY2(shot.pixelColor(mapped) == QColor(Qt::black),
                 qPrintable(QStringLiteral("the masked layer keeps its color at %1,%2 (%3)")
                                .arg(mapped.x())
                                .arg(mapped.y())
                                .arg(shot.pixelColor(mapped).name())));
    }
    tools->cancelFreeTransform();
}

void TransformSkewTest::styleKeepsTheLayerColor()
{
    pictura::PicturaMainWindow& frame = *window_;
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QTest::qWait(50);
    QVERIFY(frame.newDocument(QStringLiteral("StyleSkew"), 100, 80, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && canvas && tools);
    const int added = view->add_layer(0);
    QVERIFY(added > 0);
    const QString layer = QString::number(added);
    frame.selectLayerPath(layer);
    QVERIFY(view->select_rect(10, 8, 40, 30, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*view, qRgba(255, 255, 255, 255), -1, QStringLiteral("normal"),
                               100, false));
    view->deselect();
    QVERIFY(pictura::layer_style_set(*view, layer, QStringLiteral("colorOverlay.on"), 1.0));
    QVERIFY(pictura::layer_style_set(*view, layer, QStringLiteral("colorOverlay.color"), 0.0));
    pictura::layer_style_commit(*view, QStringLiteral("Layer Style"));
    QCOMPARE(view->sample_argb(20, 15), qRgba(0, 0, 0, 255));

    QVERIFY(tools->beginTransformMode(layer, QStringLiteral("skew")));
    pictura::transform_set_show_pivot(*view, false);
    QCoreApplication::processEvents();
    QTest::qWait(50);
    const QPoint inside = canvas->imageToWidget(QPointF(20.0, 15.0)).toPoint();
    QVERIFY2(canvas->grab().toImage().pixelColor(inside) == QColor(Qt::black),
             "the layer style stays in the live preview");

    const QPolygonF quad = quadOf(view);
    QCOMPARE(quad.size(), 4);
    const QPointF corner = quad.at(1);
    canvas->mousePressed(corner, Qt::LeftButton, 0);
    canvas->mouseMoved(corner + QPointF(-15.0, 0.0));
    QCoreApplication::processEvents();
    QTest::qWait(50);
    const pictura::transform_api::Matrix matrix = pictura::transform_api::previewMatrix(*view);
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
    const QImage shot = canvas->grab().toImage();
    for (const QPointF& source : {QPointF(20.0, 15.0), QPointF(30.0, 25.0)}) {
        const QPoint mapped = canvas->imageToWidget(map(source.x(), source.y())).toPoint();
        QVERIFY2(shot.pixelColor(mapped) == QColor(Qt::black),
                 qPrintable(QStringLiteral("the layer style keeps its color at %1,%2 (%3)")
                                .arg(mapped.x())
                                .arg(mapped.y())
                                .arg(shot.pixelColor(mapped).name())));
    }
    tools->cancelFreeTransform();
}
void TransformSkewTest::clippedFillKeepsTheLayerColor()
{
    pictura::PicturaMainWindow& frame = *window_;
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QTest::qWait(50);
    QVERIFY(frame.newDocument(QStringLiteral("ClipSkew"), 100, 80, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && canvas && tools);
    const int added = view->add_layer(0);
    QVERIFY(added > 0);
    const QString base = QString::number(added);
    frame.selectLayerPath(base);
    QVERIFY(view->select_rect(10, 8, 40, 30, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*view, qRgba(255, 255, 255, 255), -1, QStringLiteral("normal"),
                               100, false));
    view->deselect();
    const QString fill = view->add_solid_fill(0xff000000u);
    QVERIFY(!fill.isEmpty());
    QVERIFY(pictura::clipping_toggle(*view, fill));
    QCOMPARE(view->sample_argb(20, 15), qRgba(0, 0, 0, 255));

    view->set_active_layer(base);
    QVERIFY(tools->beginTransformMode(base, QStringLiteral("skew")));
    pictura::transform_set_show_pivot(*view, false);
    QCoreApplication::processEvents();
    QTest::qWait(50);
    const QPolygonF quad = quadOf(view);
    QCOMPARE(quad.size(), 4);
    const QPointF corner = quad.at(1);
    canvas->mousePressed(corner, Qt::LeftButton, 0);
    canvas->mouseMoved(corner + QPointF(0.0, 22.0));
    QCoreApplication::processEvents();
    QTest::qWait(50);

    const pictura::transform_api::Matrix matrix = pictura::transform_api::previewMatrix(*view);
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
    const QPoint inside = canvas->imageToWidget(map(30.0, 23.0)).toPoint();
    QVERIFY2(canvas->grab().toImage().pixelColor(inside) == QColor(Qt::black),
             "the clipped fill stays in the live preview");
    tools->cancelFreeTransform();
}

void TransformSkewTest::thinLineKeepsTheLayerColor()
{
    pictura::PicturaMainWindow& frame = *window_;
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QTest::qWait(50);
    QVERIFY(frame.newDocument(QStringLiteral("LineSkew"), 100, 80, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && canvas && tools);
    const int added = view->add_layer(0);
    QVERIFY(added > 0);
    const QString layer = QString::number(added);
    frame.selectLayerPath(layer);
    // A tall, almost transparent rect keeps the transform box tall; the blue
    // line across its middle is the visible content.
    QVERIFY(view->select_rect(10, 8, 40, 40, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*view, qRgba(255, 255, 255, 1), -1, QStringLiteral("normal"),
                               100, false));
    QVERIFY(view->select_rect(10, 26, 40, 4, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*view, qRgba(0, 0, 255, 255), -1, QStringLiteral("normal"),
                               100, false));
    view->deselect();

    QVERIFY(tools->beginTransformMode(layer, QStringLiteral("skew")));
    pictura::transform_set_show_pivot(*view, false);
    QCoreApplication::processEvents();
    QTest::qWait(50);
    const QPolygonF quad = quadOf(view);
    QCOMPARE(quad.size(), 4);
    const QPointF corner = quad.at(1);
    canvas->mousePressed(corner, Qt::LeftButton, 0);
    canvas->mouseMoved(corner + QPointF(0.0, 27.0));
    QCoreApplication::processEvents();
    QTest::qWait(50);

    const pictura::transform_api::Matrix matrix = pictura::transform_api::previewMatrix(*view);
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
    const QPoint line = canvas->imageToWidget(map(30.0, 28.0)).toPoint();
    const QImage shot = canvas->grab().toImage();
    bool blue = false;
    QRect blueBounds;
    for (int y = 0; y < shot.height(); ++y) {
        for (int x = 0; x < shot.width(); ++x) {
            const QColor c = shot.pixelColor(x, y);
            if (c.blue() > 200 && c.red() < 60 && c.green() < 60) {
                blue = true;
                blueBounds = blueBounds.isNull() ? QRect(x, y, 1, 1)
                                                 : blueBounds.united(QRect(x, y, 1, 1));
            }
        }
    }
    QVERIFY2(blue && blueBounds.contains(line),
             qPrintable(QStringLiteral("the thin line stays in the live preview; mapped %1,%2 "
                                       "pixel %3 blue %4,%5 %6x%7")
                            .arg(line.x())
                            .arg(line.y())
                            .arg(shot.pixelColor(line).name())
                            .arg(blueBounds.x())
                            .arg(blueBounds.y())
                            .arg(blueBounds.width())
                            .arg(blueBounds.height())));
    tools->cancelFreeTransform();
}

void TransformSkewTest::typeKeepsTheLayerColor()
{
    pictura::PicturaMainWindow& frame = *window_;
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QTest::qWait(50);

    QImage seed(200, 160, QImage::Format_RGB32);
    seed.fill(Qt::white);
    paint_fixture::Fixture f(frame, seed, QStringLiteral("pictura_skew_type"));
    QVERIFY2(f.ok(), "type fixture");
    frame.setActiveTool(pictura::ToolId::HorizontalType);
    const pictura::TypeOptions saved =
        f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get();
    pictura::TypeOptions o = saved;
    o.size = 48.0;
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(o);
    f.drag({QPointF(10, 60)});
    QTest::keyClicks(f.canvas, QStringLiteral("WWW"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(saved);

    QString path;
    for (int i = 0; i < f.view->layer_count(); ++i) {
        if (pictura::type_api::isType(*f.view, QString::number(i))) {
            path = QString::number(i);
            break;
        }
    }
    QVERIFY2(!path.isEmpty(), "a type layer");
    const QRect rect = layerRectOf(f.view, path);
    QVERIFY(rect.isValid());

    const auto dark = [&f](int x, int y) { return qGray(f.view->sample_argb(x, y)) < 80; };
    QPoint sample;
    for (int y = rect.top() + 1; y < rect.bottom() && sample.isNull(); ++y) {
        for (int x = rect.left() + 1; x < rect.right(); ++x) {
            if (dark(x, y) && dark(x - 1, y) && dark(x + 1, y) && dark(x, y - 1)
                && dark(x, y + 1)) {
                sample = QPoint(x, y);
                break;
            }
        }
    }
    QVERIFY2(!sample.isNull(), "a solid glyph pixel");

    QVERIFY(f.tools->beginTransformMode(path, QStringLiteral("skew")));
    pictura::transform_set_show_pivot(*f.view, false);
    QCoreApplication::processEvents();
    QTest::qWait(50);

    const QPolygonF quad = quadOf(f.view);
    QCOMPARE(quad.size(), 4);
    const QPointF corner = quad.at(1);
    f.canvas->mousePressed(corner, Qt::LeftButton, 0);
    f.canvas->mouseMoved(corner + QPointF(6.0, 3.0));
    QCoreApplication::processEvents();
    QTest::qWait(50);

    const pictura::transform_api::Matrix matrix = pictura::transform_api::previewMatrix(*f.view);
    QVERIFY(matrix.valid);
    const auto map = [&matrix](double x, double y) {
        const double w = matrix.coefficients[2] * x + matrix.coefficients[5] * y
                         + matrix.coefficients[8];
        return QPointF((matrix.coefficients[0] * x + matrix.coefficients[3] * y
                        + matrix.coefficients[6]) / w,
                       (matrix.coefficients[1] * x + matrix.coefficients[4] * y
                        + matrix.coefficients[7]) / w);
    };
    const QPoint moved =
        f.canvas->imageToWidget(map(sample.x() + 0.5, sample.y() + 0.5)).toPoint();
    const QColor c = f.canvas->grab().toImage().pixelColor(moved);
    QVERIFY2(qGray(c.rgb()) < 80,
             qPrintable(QStringLiteral("the type stays in the live preview; sampled %1,%2 is %3")
                            .arg(moved.x())
                            .arg(moved.y())
                            .arg(c.name())));
    f.view->transform_release();
    f.tools->cancelFreeTransform();
}

void TransformSkewTest::statusShowsTheSkewAction()
{
    pictura::PicturaMainWindow& frame = *window_;
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QTest::qWait(50);
    const QString layer = beginLayer(frame);
    QVERIFY(!layer.isEmpty());
    auto* tools = frame.findChild<pictura::ToolController*>();
    auto* label = frame.findChild<QLabel*>(QStringLiteral("statusActionLabel"));
    QVERIFY(tools && label);
    QCoreApplication::processEvents();
    QVERIFY(!label->isVisible());

    QVERIFY(tools->beginFreeTransform(layer));
    QCoreApplication::processEvents();
    QVERIFY(label->isVisible());
    QCOMPARE(label->text(), QStringLiteral("Free Transform in progress"));

    QVERIFY(tools->beginTransformMode(layer, QStringLiteral("skew")));
    QCoreApplication::processEvents();
    QVERIFY(label->isVisible());
    QCOMPARE(label->text(), QStringLiteral("Skew in progress"));

    tools->cancelFreeTransform();
    QCoreApplication::processEvents();
    QVERIFY(!label->isVisible());
}

QTEST_MAIN(TransformSkewTest)
#include "tst_transform_skew.moc"
