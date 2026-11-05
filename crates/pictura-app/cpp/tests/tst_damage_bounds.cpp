// Structural-edit damage bounds: a bounded delete/reorder/group emits one
// region update, an unbounded edit rebuilds the canvas (#970). A shape layer's
// Move and Path Selection drag refresh only the shape's reach, the drag on
// its release (#1185). A painted mark on a canvas-sized layer moves at the
// mark's size, and a Move or Free Transform that cannot fit in memory is
// refused (#1207).

#include <QtTest/QtTest>
#include <QtTest/QSignalSpy>

#include <QtCore/QCoreApplication>
#include <QtCore/QStringList>

#include <memory>

#include "api/layers_api.h"
#include "frame.h"

#include "api/layers_bridge.h"
#include "api/paint_bridge.h"
#include "api/transform_bridge.h"

#include "qt_test_support.h"

class DamageBoundsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();

    void structuralDeleteEmitsOneRegion();
    void structuralReorderEmitsOneRegion();
    void structuralGroupEmitsOneRegion();
    void unboundedStructuralRebuilds();
    void maskFeatherPreviewEmitsOneRegion();
    void shapeMoveIsBoundedToTheShape();
    void shapePathDragRefreshesItsReach();
    void paintedMoveIsBoundedToTheMark();
    void moveAndTransformRefuseWhatMemoryCannotHold();

private:
    bool newDoc(int width, int height);
    QString addMark();
    QString addShape(int x0, int y0, int x1, int y1);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PictureView* view_ = nullptr;
};

void DamageBoundsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->resize(800, 600);
    window_->show();
    QCoreApplication::processEvents();
}

void DamageBoundsTest::cleanup()
{
    if (window_) {
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
    view_ = nullptr;
}

bool DamageBoundsTest::newDoc(int width, int height)
{
    if (!window_->newDocument(QStringLiteral("DamageBounds"), width, height,
                              QStringLiteral("rgb"), 8, QStringLiteral("white"))) {
        return false;
    }
    view_ = window_->activeView();
    QTest::qWait(0);
    QCoreApplication::processEvents();
    return view_ != nullptr;
}

QString DamageBoundsTest::addShape(int x0, int y0, int x1, int y1)
{
    pictura::ShapeSpec spec{};
    spec.kind = 0;
    spec.boxed = true;
    spec.x0 = x0;
    spec.y0 = y0;
    spec.x1 = x1;
    spec.y1 = y1;
    return pictura::shape_add_layer(*view_, spec, 0xffff0000u);
}

void DamageBoundsTest::structuralDeleteEmitsOneRegion()
{
    QVERIFY(newDoc(800, 600));
    const QString shape = addShape(40, 40, 80, 80);
    QVERIFY(!shape.isEmpty());
    QSignalSpy damaged(view_, &pictura::PictureView::regionDamaged);
    QSignalSpy changed(view_, &pictura::PictureView::changed);
    QCOMPARE(view_->delete_layers(QStringList{shape}), 1);
    QCoreApplication::processEvents();
    QVERIFY2(damaged.count() >= 1, "a bounded delete emits a region");
    QVERIFY2(changed.count() >= 1, "a bounded delete notifies the panel");
}

void DamageBoundsTest::structuralReorderEmitsOneRegion()
{
    QVERIFY(newDoc(800, 600));
    QVERIFY(!addShape(40, 40, 80, 80).isEmpty());
    QVERIFY(!addShape(120, 120, 160, 160).isEmpty());
    QSignalSpy damaged(view_, &pictura::PictureView::regionDamaged);
    QVERIFY(view_->move_layer(2, -1));
    QCoreApplication::processEvents();
    QVERIFY2(damaged.count() >= 1, "a bounded reorder emits a region");
}

void DamageBoundsTest::structuralGroupEmitsOneRegion()
{
    QVERIFY(newDoc(800, 600));
    const QString a = addShape(40, 40, 80, 80);
    const QString b = addShape(120, 120, 160, 160);
    QVERIFY(!a.isEmpty() && !b.isEmpty());
    QSignalSpy damaged(view_, &pictura::PictureView::regionDamaged);
    QVERIFY(!view_->group_layers(QStringList{a, b}).isEmpty());
    QCoreApplication::processEvents();
    QVERIFY2(damaged.count() >= 1, "a bounded group emits a region");
}

void DamageBoundsTest::unboundedStructuralRebuilds()
{
    QVERIFY(newDoc(800, 600));
    QVERIFY(view_->add_adjustment(QStringLiteral("invert")));
    QSignalSpy damaged(view_, &pictura::PictureView::regionDamaged);
    QSignalSpy changed(view_, &pictura::PictureView::changed);
    QVERIFY(view_->set_layers_visible(QStringList{QStringLiteral("1")}, false) >= 0);
    QCoreApplication::processEvents();
    QCOMPARE(damaged.count(), 0);
    QVERIFY2(changed.count() >= 1, "an unbounded visibility change rebuilds");
}

void DamageBoundsTest::maskFeatherPreviewEmitsOneRegion()
{
    QVERIFY(newDoc(800, 600));
    const QString shape = addShape(40, 40, 80, 80);
    QVERIFY(!shape.isEmpty());
    QVERIFY(pictura::layer_mask_add(*view_, QStringLiteral("reveal-all")));
    QSignalSpy damaged(view_, &pictura::PictureView::regionDamaged);
    QVERIFY(pictura::layer_mask_set_feather(*view_, 8.0, false));
    QCoreApplication::processEvents();
    QVERIFY2(damaged.count() >= 1, "a bounded feather preview emits a region");
}

namespace {

constexpr quint32 kShapeRed = 0xffff0000u;
constexpr quint32 kPaper = 0xffffffffu;

// The last regionDamaged rectangle's width and height.
QSize lastDamage(const QSignalSpy& damaged)
{
    const QList<QVariant> last = damaged.last();
    return QSize(last.at(2).toInt(), last.at(3).toInt());
}

} // namespace

void DamageBoundsTest::shapeMoveIsBoundedToTheShape()
{
    QVERIFY(newDoc(800, 600));
    QVERIFY(!addShape(40, 40, 80, 80).isEmpty());
    QVERIFY(view_->begin_move_preview());
    QCOMPARE(view_->move_preview_x(), 40);
    QCOMPARE(view_->move_preview_y(), 40);
    const QImage moving = view_->move_preview_layer();
    QVERIFY2(moving.width() <= 42 && moving.height() <= 42,
             "the moving image is the shape's box, not the canvas");

    QSignalSpy damaged(view_, &pictura::PictureView::regionDamaged);
    QVERIFY(view_->commit_move(30, 20));
    QCoreApplication::processEvents();
    QVERIFY2(damaged.count() >= 1, "the commit refreshes a region");
    const QSize reach = lastDamage(damaged);
    QVERIFY2(reach.width() <= 80 && reach.height() <= 80,
             "the commit refreshes the shape before and after, not the canvas");
    QCOMPARE(view_->sample_argb(100, 80), kShapeRed);
    QCOMPARE(view_->sample_argb(45, 45), kPaper);
}

void DamageBoundsTest::shapePathDragRefreshesItsReach()
{
    QVERIFY(newDoc(800, 600));
    QVERIFY(!addShape(40, 40, 80, 80).isEmpty());
    pictura::path_set_layer_target(*view_, true);
    QSignalSpy damaged(view_, &pictura::PictureView::regionDamaged);
    QVERIFY(pictura::path_move_subpath(*view_, 0, 20.0, 10.0));
    QVERIFY(pictura::path_move_subpath(*view_, 0, 10.0, 10.0));
    QCoreApplication::processEvents();
    QCOMPARE(damaged.count(), 0);
    QCOMPARE(view_->sample_argb(45, 45), kShapeRed);

    pictura::path_commit_drag(*view_, "Drag Path");
    QCoreApplication::processEvents();
    QCOMPARE(damaged.count(), 1);
    const QSize reach = lastDamage(damaged);
    QVERIFY2(reach.width() <= 80 && reach.height() <= 80,
             "the release refreshes the shape before and after, not the canvas");
    QCOMPARE(view_->sample_argb(100, 80), kShapeRed);
    QCOMPARE(view_->sample_argb(45, 45), kPaper);
}

// A black 40×40 mark at (40, 40) on a new canvas-sized layer; its path.
QString DamageBoundsTest::addMark()
{
    const int index = view_->add_layer(0);
    const QString path = QString::number(index);
    view_->set_active_layer(path);
    view_->select_rect(40, 40, 40, 40, QStringLiteral("new"), 0.0);
    if (!pictura::edit_fill(*view_, QColor(Qt::black).rgba(), -1, QStringLiteral("normal"), 100,
                            false)) {
        return {};
    }
    view_->deselect();
    QCoreApplication::processEvents();
    return path;
}

void DamageBoundsTest::paintedMoveIsBoundedToTheMark()
{
    QVERIFY(newDoc(800, 600));
    QVERIFY(!addMark().isEmpty());
    const QRgb black = QColor(Qt::black).rgba();
    QVERIFY(view_->begin_move_preview());
    QCOMPARE(view_->move_preview_x(), 40);
    QCOMPARE(view_->move_preview_y(), 40);
    QCOMPARE(view_->move_preview_layer().size(), QSize(40, 40));

    QSignalSpy damaged(view_, &pictura::PictureView::regionDamaged);
    view_->end_move_preview();
    QVERIFY(view_->commit_move(30, 20));
    QTRY_VERIFY(damaged.count() >= 1);
    const QSize reach = lastDamage(damaged);
    QVERIFY2(reach.width() <= 80 && reach.height() <= 80,
             "the commit refreshes the mark before and after, not the canvas");
    QTRY_COMPARE(view_->sample_argb(100, 80), black);
    QCOMPARE(view_->sample_argb(45, 45), kPaper);

    // The cached preview follows the moved layer.
    QVERIFY(view_->begin_move_preview());
    QVERIFY(view_->move_preview_cache_hit());
    QCOMPARE(view_->move_preview_x(), 70);
    QCOMPARE(view_->move_preview_y(), 60);
    view_->end_move_preview();
}

void DamageBoundsTest::moveAndTransformRefuseWhatMemoryCannotHold()
{
    QVERIFY(newDoc(800, 600));
    const QString path = addMark();
    QVERIFY(!path.isEmpty());
    const QRgb black = QColor(Qt::black).rgba();
    struct Restore {
        ~Restore() { pictura::set_available_memory_for_test(-1); }
    } restore;
    const int base = view_->history_index();

    pictura::set_available_memory_for_test(64);
    QVERIFY(!view_->begin_move_preview());
    QVERIFY(pictura::edit_refused_for_memory());

    // A press that fits, then a commit that does not: the layer stays put.
    pictura::set_available_memory_for_test(-1);
    QVERIFY(view_->begin_move_preview());
    QVERIFY(!pictura::edit_refused_for_memory());
    view_->end_move_preview();
    pictura::set_available_memory_for_test(64);
    QVERIFY(!view_->commit_move(30, 20));
    QVERIFY(pictura::edit_refused_for_memory());
    QCOMPARE(view_->history_index(), base);
    QCOMPARE(view_->sample_argb(45, 45), black);
    QCOMPARE(view_->sample_argb(100, 80), kPaper);

    // A transform commit that does not fit is dropped; the layer is as it was.
    pictura::set_available_memory_for_test(-1);
    QVERIFY(view_->begin_free_transform(path));
    QVERIFY(pictura::transform_apply(*view_, QStringLiteral("flipHorizontal")));
    pictura::set_available_memory_for_test(64);
    QVERIFY(!view_->commit_transform());
    QVERIFY(pictura::edit_refused_for_memory());
    QVERIFY(!view_->transform_session_active());
    QCOMPARE(view_->history_index(), base);
    QCOMPARE(view_->sample_argb(45, 45), black);

    pictura::set_available_memory_for_test(-1);
    QVERIFY(view_->commit_move(30, 20));
    QVERIFY(!pictura::edit_refused_for_memory());
    QTRY_COMPARE(view_->sample_argb(100, 80), black);
}

QTEST_MAIN(DamageBoundsTest)
#include "tst_damage_bounds.moc"
