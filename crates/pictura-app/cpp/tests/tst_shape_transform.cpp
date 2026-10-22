// Shape layers move and transform as vectors (#734): the Move tool and the
// Edit > Transform commands map the outline, the live preview turns the layer
// with the box, its edges are antialiased, and the Layers panel draws a shape
// layer as one thumbnail, with no vector-mask slot beside it.

#include <QtTest/QtTest>

#include "api/layers_api.h"
#include "api/transform_api.h"
#include "commands.h"
#include "selftest_paint_fixture.h"

#include "pictura_app/src/cxxqt_object/impl_transform/bridge.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools/fills.cxxqt.h"
#include "pictura_app/src/cxxqt_object/shapes.cxxqt.h"

#include <memory>

#include "qt_test_support.h"

namespace {

using paint_fixture::Fixture;

const QRgb kRed = 0xffff0000u;
const QRgb kWhite = 0xffffffffu;

QImage seed()
{
    QImage image(120, 80, QImage::Format_RGB32);
    image.fill(Qt::white);
    return image;
}

// A red rectangle shape layer over `box`, made the active layer.
QString addRectangle(Fixture& f, const QRect& box = QRect(10, 10, 20, 10))
{
    pictura::ShapeSpec spec{};
    spec.kind = 0;
    spec.boxed = true;
    spec.x0 = box.left();
    spec.y0 = box.top();
    spec.x1 = box.left() + box.width();
    spec.y1 = box.top() + box.height();
    const QString path = pictura::shape_add_layer(*f.view, spec, kRed);
    if (!path.isEmpty()) {
        f.frame.selectLayerPath(path);
    }
    return path;
}

QRect controlsBox(pictura::PictureView& view, const QString& path)
{
    const ::rust::Vec<int32_t> box = pictura::controls_box_extents(view, path);
    return box.size() == 4 ? QRect(QPoint(box[0], box[1]), QPoint(box[2] - 1, box[3] - 1))
                           : QRect();
}

} // namespace

class ShapeTransformTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void shapeRowShowsOneThumbnail();
    void moveToolMovesTheShape();
    void transformCommandsMapTheOutline();
    void rotationPreviewTurnsTheLayer();
    void rotatedShapeEdgesAreSmooth();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void ShapeTransformTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void ShapeTransformTest::shapeRowShowsOneThumbnail()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_shape_thumbnail"));
    QVERIFY2(f.ok(), "shape fixture");
    const QString path = addRectangle(f);
    QVERIFY(!path.isEmpty());

    const pictura::layers_api::Snapshot snap = pictura::layers_api::snapshot(*f.view, 32, false);
    const pictura::layers_api::LayerRow* row = nullptr;
    for (const pictura::layers_api::LayerRow& candidate : snap.rows) {
        if (candidate.path == path) {
            row = &candidate;
        }
    }
    QVERIFY(row != nullptr);
    QVERIFY(row->shape);
    QVERIFY(row->hasVectorMask);
    QVERIFY(!row->thumbnail.isNull());
    QVERIFY2(row->vectorMaskThumbnail.isNull(), "no vector-mask slot beside the shape");
    QVERIFY(row->maskThumbnail.isNull());
}

void ShapeTransformTest::moveToolMovesTheShape()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_shape_move"));
    QVERIFY2(f.ok(), "shape fixture");
    const QString path = addRectangle(f, QRect(20, 20, 60, 40));
    QVERIFY(!path.isEmpty());
    f.canvas->setSmartGuidesEnabled(false);
    window_->setActiveTool(pictura::ToolId::Move);
    QCOMPARE(controlsBox(*f.view, path), QRect(20, 20, 60, 40));

    // A press inside the shape, clear of the controls box's handles, is an
    // ordinary Move drag of the layer.
    const int base = f.view->history_index();
    f.drag({QPointF(35, 30), QPointF(45, 37), QPointF(55, 45)});
    QVERIFY2(f.committedOnce(base, "Move Layer"), "one Move Layer state");
    QCOMPARE(f.view->sample_argb(25, 25), kWhite);
    QCOMPARE(f.view->sample_argb(95, 70), kRed);
    QVERIFY(pictura::shape_is_shape(*f.view, path));
    QCOMPARE(controlsBox(*f.view, path), QRect(40, 35, 60, 40));

    QVERIFY(f.view->undo());
    QCOMPARE(f.view->sample_argb(25, 25), kRed);
    QCOMPARE(f.view->sample_argb(95, 70), kWhite);
}

void ShapeTransformTest::transformCommandsMapTheOutline()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_shape_transform"));
    QVERIFY2(f.ok(), "shape fixture");
    const QString path = addRectangle(f);
    QVERIFY(!path.isEmpty());
    QVERIFY(f.view->layer_can_free_transform(path));

    // A quarter turn about the outline's centre (20, 15).
    int base = f.view->history_index();
    QVERIFY(window_->registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::EditTransformRotate90Cw)));
    QCOMPARE(f.view->history_index(), base + 1);
    QVERIFY(pictura::shape_is_shape(*f.view, path));
    QCOMPARE(controlsBox(*f.view, path), QRect(QPoint(15, 5), QPoint(24, 24)));
    QCOMPARE(f.view->sample_argb(20, 7), kRed);
    QCOMPARE(f.view->sample_argb(12, 15), kWhite);

    // Free Transform's box hugs the outline, and a width change scales it.
    QVERIFY(f.tools->beginFreeTransform(path));
    const QPolygonF quad = pictura::transform_api::quad(*f.view).corners;
    QCOMPARE(quad.size(), 4);
    QCOMPARE(quad.at(0), QPointF(15, 5));
    QCOMPARE(quad.at(2), QPointF(25, 25));
    QVERIFY(f.tools->setTransformWidth(200.0, false));
    base = f.view->history_index();
    f.tools->commitFreeTransform();
    QCOMPARE(f.view->history_index(), base + 1);
    QVERIFY(pictura::shape_is_shape(*f.view, path));
    QCOMPARE(controlsBox(*f.view, path), QRect(QPoint(10, 5), QPoint(29, 24)));
    QCOMPARE(f.view->sample_argb(12, 15), kRed);
}

void ShapeTransformTest::rotationPreviewTurnsTheLayer()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_shape_preview"));
    QVERIFY2(f.ok(), "shape fixture");
    const QString shape = addRectangle(f);
    QVERIFY(!shape.isEmpty());

    // A raster layer with the same mark, to show the preview is not
    // shape-specific.
    const int added = f.view->add_layer(-1);
    QVERIFY(added > 0);
    const QString raster = QString::number(added);
    f.frame.selectLayerPath(raster);
    QVERIFY(f.view->select_rect(10, 10, 20, 10, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*f.view, qRgba(0, 0, 0, 255), -1, QStringLiteral("normal"), 100,
                               false));
    f.view->deselect();

    // A quarter turn about the box centre (20, 15) sends the top-left corner
    // (10, 10) to (25, 5) in the drawn preview, not only in the box.
    for (const QString& path : {shape, raster}) {
        f.frame.selectLayerPath(path);
        QVERIFY(f.tools->beginFreeTransform(path));
        QVERIFY(f.canvas->transformLayerMapForTest().isIdentity());
        QVERIFY(f.tools->setTransformAngle(90.0));
        const QPointF corner = f.canvas->transformLayerMapForTest().map(QPointF(10, 10));
        QVERIFY2(QLineF(corner, QPointF(25, 5)).length() < 1e-6,
                 qPrintable(QStringLiteral("%1: corner drawn at %2,%3")
                                .arg(path)
                                .arg(corner.x())
                                .arg(corner.y())));
        f.tools->cancelFreeTransform();
    }
}

void ShapeTransformTest::rotatedShapeEdgesAreSmooth()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_shape_smooth"));
    QVERIFY2(f.ok(), "shape fixture");
    const QString path = addRectangle(f, QRect(30, 20, 60, 40));
    QVERIFY(!path.isEmpty());

    QVERIFY(f.tools->beginFreeTransform(path));
    QVERIFY(f.tools->setTransformAngle(30.0));
    f.tools->commitFreeTransform();
    QVERIFY(pictura::shape_is_shape(*f.view, path));

    // Red over white: a stair-stepped edge is only pure red or pure white; an
    // antialiased one blends the pixels the slanted edge crosses.
    int blended = 0;
    for (int y = 0; y < 80; ++y) {
        for (int x = 0; x < 120; ++x) {
            const int green = qGreen(f.view->sample_argb(x, y));
            if (green > 20 && green < 235) {
                ++blended;
            }
        }
    }
    QVERIFY2(blended > 40, qPrintable(QStringLiteral("%1 blended edge pixels").arg(blended)));
}

QTEST_MAIN(ShapeTransformTest)
#include "tst_shape_transform.moc"
