// Nested-layer targeting (#252): the active-layer path resolves to the exact
// leaf, path-based kind/lock bridges reflect that leaf, and the brush paints it.

#include <QtTest/QtTest>

#include "selftest_paint_fixture.h"

#include "qt_test_support.h"

#include "pictura_app/src/cxxqt_object/layer_path.cxxqt.h"

namespace {
using paint_fixture::Fixture;
} // namespace

class NestedLayerTargetTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void pathBridgesResolveTheNestedLeaf();
    void brushPaintsTheNestedLeaf();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void NestedLayerTargetTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void NestedLayerTargetTest::pathBridgesResolveTheNestedLeaf()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(48, 48, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_nested_bridge"));
    QVERIFY2(f.ok(), "nested fixture");

    const QString group = f.view->add_group_in(QString());
    QVERIFY2(!group.isEmpty(), "group inserted");
    const QString nested = f.view->add_layer_in(group);
    QVERIFY2(nested.startsWith(group + QStringLiteral("/")), "layer nested in the group");

    QCOMPARE(pictura::layer_kind_path(*f.view, group), QStringLiteral("group"));
    QCOMPARE(pictura::layer_kind_path(*f.view, nested), QStringLiteral("pixel"));

    f.view->set_layers_lock({nested}, QStringLiteral("pixels"), true);
    QVERIFY2((pictura::layer_lock_path(*f.view, nested) & 0x02) != 0, "nested pixel lock reflects");
    QCOMPARE(pictura::layer_lock_path(*f.view, group) & 0x02, 0);
    f.view->set_layers_lock({nested}, QStringLiteral("pixels"), false);
    QCOMPARE(pictura::layer_lock_path(*f.view, nested) & 0x02, 0);
}

void NestedLayerTargetTest::brushPaintsTheNestedLeaf()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(48, 48, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_nested_paint"));
    QVERIFY2(f.ok(), "nested fixture");

    const QString group = f.view->add_group_in(QString());
    QVERIFY(!group.isEmpty());
    const QString nested = f.view->add_layer_in(group);
    QVERIFY(!nested.isEmpty());
    f.view->set_active_layer(nested);

    // A fresh transparent raster layer has no covered pixels.
    QVERIFY(f.view->select_layer_alpha(nested));
    const int before = f.view->selection_count();
    f.view->deselect();

    QVERIFY(f.view->begin_paint(0xFFFF0000u, 0xFFFFFFFFu, 8, 100, 100, 0, 100, 100, 25,
                                QStringLiteral("normal"), false, false));
    QVERIFY(f.view->paint_dab(6.0, 6.0, 1.0));
    QVERIFY(f.view->end_paint());

    // The nested leaf now carries opaque pixels; that is where the stroke went.
    QVERIFY(f.view->select_layer_alpha(nested));
    QVERIFY2(f.view->selection_count() > before, "stroke landed in the nested leaf");
}

QTEST_MAIN(NestedLayerTargetTest)
#include "tst_nested_layer_target.moc"
